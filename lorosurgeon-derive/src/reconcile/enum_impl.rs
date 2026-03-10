//! Reconcile derive for enums.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, DeriveInput, Fields, Ident};

use crate::attrs::FieldAttrs;

pub fn derive_reconcile_enum(
    input: &DeriveInput,
    data: &DataEnum,
) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let has_data_variants = data.variants.iter().any(|v| !matches!(v.fields, Fields::Unit));
    let all_unit = !has_data_variants;

    // Check if any variant has a #[key] field
    let has_keys = data.variants.iter().any(|v| {
        match &v.fields {
            Fields::Named(fields) => fields.named.iter().any(|f| {
                FieldAttrs::from_attrs(&f.attrs).is_ok_and(|a| a.is_key)
            }),
            _ => false,
        }
    });

    let match_arms: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.ident;
            let variant_str = variant_name.to_string();

            match &v.fields {
                Fields::Unit => {
                    if all_unit {
                        // Pure unit enum: serialize as string
                        quote! {
                            #name::#variant_name => lorosurgeon::Reconciler::str(r, #variant_str),
                        }
                    } else {
                        // Mixed enum: serialize as LoroMap { "Variant": "Variant" }
                        quote! {
                            #name::#variant_name => {
                                let mut m = lorosurgeon::Reconciler::map(r)?;
                                m.retain(|k| k == #variant_str)?;
                                m.entry(#variant_str, &String::from(#variant_str))?;
                                Ok(())
                            }
                        }
                    }
                }
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    // Single tuple: LoroMap { "Variant": value }
                    quote! {
                        #name::#variant_name(inner) => {
                            let mut m = lorosurgeon::Reconciler::map(r)?;
                            m.retain(|k| k == #variant_str)?;
                            m.entry(#variant_str, inner)?;
                            Ok(())
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    // Multi-element tuple: LoroMap { "Variant": LoroList [a, b, ...] }
                    let bindings: Vec<_> = (0..fields.unnamed.len())
                        .map(|i| {
                            Ident::new(&format!("f{i}"), proc_macro2::Span::call_site())
                        })
                        .collect();

                    let pattern = quote! { #(#bindings),* };

                    let list_entries: Vec<_> = bindings
                        .iter()
                        .enumerate()
                        .map(|(i, binding)| {
                            quote! {
                                l.insert(#i, #binding)?;
                            }
                        })
                        .collect();

                    quote! {
                        #name::#variant_name(#pattern) => {
                            let mut m = lorosurgeon::Reconciler::map(r)?;
                            m.retain(|k| k == #variant_str)?;
                            let prop_r = lorosurgeon::PropReconciler::map_put(m.map.clone(), #variant_str.to_string());
                            let mut l = lorosurgeon::Reconciler::list(prop_r)?;
                            // Clear existing
                            while l.len() > 0 {
                                l.delete(0)?;
                            }
                            #(#list_entries)*
                            Ok(())
                        }
                    }
                }
                Fields::Named(fields) => {
                    // Named fields: LoroMap { "Variant": LoroMap { field: value, ... } }
                    let field_names: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();

                    let field_entries: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| {
                            let field_name = f.ident.as_ref().unwrap();
                            let attrs = FieldAttrs::from_attrs(&f.attrs).unwrap_or_default();
                            let loro_key = attrs.loro_key(&field_name.to_string());
                            quote! {
                                inner_map.entry(#loro_key, #field_name)?;
                            }
                        })
                        .collect();

                    let pattern = quote! { #(#field_names),* };

                    quote! {
                        #name::#variant_name { #pattern } => {
                            let mut m = lorosurgeon::Reconciler::map(r)?;
                            m.retain(|k| k == #variant_str)?;
                            let prop_r = lorosurgeon::PropReconciler::map_put(m.map.clone(), #variant_str.to_string());
                            let mut inner_map = lorosurgeon::Reconciler::map(prop_r)?;
                            #(#field_entries)*
                            Ok(())
                        }
                    }
                }
            }
        })
        .collect();

    // Generate key type and impls for enums with #[key] fields
    let (key_items, key_type, key_fn, hydrate_key_fn) = if has_keys {
        generate_enum_key(name, data)?
    } else {
        (
            TokenStream::new(),
            quote! { lorosurgeon::NoKey },
            TokenStream::new(),
            TokenStream::new(),
        )
    };

    Ok(quote! {
        #key_items

        impl #impl_generics lorosurgeon::Reconcile for #name #ty_generics #where_clause {
            type Key = #key_type;

            fn reconcile<R: lorosurgeon::Reconciler>(&self, r: R) -> Result<(), lorosurgeon::ReconcileError> {
                match self {
                    #(#match_arms)*
                }
            }

            #key_fn
            #hydrate_key_fn
        }
    })
}

/// Generate a companion key enum for keyed list diffing.
///
/// For each variant:
/// - Named variant with `#[key]` field → key variant carries that field's type
/// - Named variant without `#[key]` → key variant is unit (matches by variant name only)
/// - Unit variant → key variant is unit
/// - Tuple variant → key variant is unit (no key field possible)
fn generate_enum_key(
    name: &Ident,
    data: &DataEnum,
) -> syn::Result<(TokenStream, TokenStream, TokenStream, TokenStream)> {
    let key_name = format_ident!("__{}Key", name);

    let mut key_variants = Vec::new();
    let mut key_extract_arms = Vec::new();
    let mut hydrate_key_arms = Vec::new();

    for variant in &data.variants {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();

        match &variant.fields {
            Fields::Named(fields) => {
                // Find #[key] field in this variant
                let key_field = fields.named.iter().find(|f| {
                    FieldAttrs::from_attrs(&f.attrs).is_ok_and(|a| a.is_key)
                });

                if let Some(kf) = key_field {
                    let key_field_name = kf.ident.as_ref().unwrap();
                    let key_field_ty = &kf.ty;
                    let attrs = FieldAttrs::from_attrs(&kf.attrs).unwrap_or_default();
                    let loro_key = attrs.loro_key(&key_field_name.to_string());

                    // Key variant carries the key field type
                    key_variants.push(quote! { #variant_name(#key_field_ty) });

                    // Extract key from enum value
                    let other_fields: Vec<_> = fields.named.iter()
                        .filter(|f| f.ident.as_ref().unwrap() != key_field_name)
                        .map(|f| {
                            let n = f.ident.as_ref().unwrap();
                            quote! { #n: _ }
                        })
                        .collect();

                    key_extract_arms.push(quote! {
                        #name::#variant_name { #key_field_name, #(#other_fields),* } => {
                            lorosurgeon::LoadKey::Found(#key_name::#variant_name(#key_field_name.clone()))
                        }
                    });

                    // Hydrate key from Loro: look for variant key in map, then extract key field
                    hydrate_key_arms.push(quote! {
                        if let Some(loro::ValueOrContainer::Container(loro::Container::Map(inner))) = map.get(#variant_str) {
                            if let Some(voc) = inner.get(#loro_key) {
                                if let Ok(k) = <#key_field_ty as lorosurgeon::Hydrate>::hydrate(&voc) {
                                    return Ok(lorosurgeon::LoadKey::Found(#key_name::#variant_name(k)));
                                }
                            }
                        }
                    });
                } else {
                    // No key field — unit key variant
                    key_variants.push(quote! { #variant_name });

                    let other_fields: Vec<_> = fields.named.iter()
                        .map(|f| {
                            let n = f.ident.as_ref().unwrap();
                            quote! { #n: _ }
                        })
                        .collect();

                    key_extract_arms.push(quote! {
                        #name::#variant_name { #(#other_fields),* } => {
                            lorosurgeon::LoadKey::Found(#key_name::#variant_name)
                        }
                    });

                    hydrate_key_arms.push(quote! {
                        if map.get(#variant_str).is_some() {
                            return Ok(lorosurgeon::LoadKey::Found(#key_name::#variant_name));
                        }
                    });
                }
            }
            Fields::Unnamed(_) => {
                // Tuple variants — unit key variant (match by variant name)
                key_variants.push(quote! { #variant_name });

                key_extract_arms.push(quote! {
                    #name::#variant_name(..) => {
                        lorosurgeon::LoadKey::Found(#key_name::#variant_name)
                    }
                });

                hydrate_key_arms.push(quote! {
                    if map.get(#variant_str).is_some() {
                        return Ok(lorosurgeon::LoadKey::Found(#key_name::#variant_name));
                    }
                });
            }
            Fields::Unit => {
                key_variants.push(quote! { #variant_name });

                key_extract_arms.push(quote! {
                    #name::#variant_name => {
                        lorosurgeon::LoadKey::Found(#key_name::#variant_name)
                    }
                });

                // For unit variants in mixed enums (map storage)
                hydrate_key_arms.push(quote! {
                    if map.get(#variant_str).is_some() {
                        return Ok(lorosurgeon::LoadKey::Found(#key_name::#variant_name));
                    }
                });
            }
        }
    }

    let key_items = quote! {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum #key_name {
            #(#key_variants),*
        }
    };

    let key_fn = quote! {
        fn key(&self) -> lorosurgeon::LoadKey<Self::Key> {
            match self {
                #(#key_extract_arms)*
            }
        }
    };

    // Generate string match arms for unit variants stored as strings
    let string_key_arms: Vec<_> = data.variants.iter()
        .filter(|v| matches!(v.fields, Fields::Unit))
        .map(|v| {
            let variant_name = &v.ident;
            let variant_str = variant_name.to_string();
            quote! {
                #variant_str => return Ok(lorosurgeon::LoadKey::Found(#key_name::#variant_name)),
            }
        })
        .collect();

    let string_match = if string_key_arms.is_empty() {
        TokenStream::new()
    } else {
        quote! {
            loro::ValueOrContainer::Value(loro::LoroValue::String(s)) => {
                match s.as_ref() {
                    #(#string_key_arms)*
                    _ => {}
                }
                Ok(lorosurgeon::LoadKey::KeyNotFound)
            }
        }
    };

    let hydrate_key_fn = quote! {
        fn hydrate_key(source: &loro::ValueOrContainer) -> Result<lorosurgeon::LoadKey<Self::Key>, lorosurgeon::ReconcileError> {
            match source {
                loro::ValueOrContainer::Container(loro::Container::Map(map)) => {
                    #(#hydrate_key_arms)*
                    Ok(lorosurgeon::LoadKey::KeyNotFound)
                }
                #string_match
                _ => Ok(lorosurgeon::LoadKey::KeyNotFound),
            }
        }
    };

    Ok((key_items, quote! { #key_name }, key_fn, hydrate_key_fn))
}
