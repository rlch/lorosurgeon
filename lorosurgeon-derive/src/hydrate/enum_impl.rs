//! Hydrate derive for enums.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, DeriveInput, Fields};

use crate::attrs::FieldAttrs;
use crate::type_util::{extract_vec_inner_type, is_option_type, is_vec_non_u8};

pub fn derive_hydrate_enum(input: &DeriveInput, data: &DataEnum) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let has_unit_variants = data
        .variants
        .iter()
        .any(|v| matches!(v.fields, Fields::Unit));
    let has_data_variants = data
        .variants
        .iter()
        .any(|v| !matches!(v.fields, Fields::Unit));

    // Unit variants hydrate from strings
    let unit_string_arms: Vec<_> = data
        .variants
        .iter()
        .filter(|v| matches!(v.fields, Fields::Unit))
        .map(|v| {
            let variant_name = &v.ident;
            let variant_str = variant_name.to_string();
            quote! {
                #variant_str => Ok(#name::#variant_name),
            }
        })
        .collect();

    let hydrate_string_fn = if has_unit_variants {
        let name_str = name.to_string();
        quote! {
            fn hydrate_string(s: &str) -> Result<Self, lorosurgeon::HydrateError> {
                match s {
                    #(#unit_string_arms)*
                    other => Err(lorosurgeon::HydrateError::unexpected(
                        concat!(#name_str, " variant"),
                        "unknown variant",
                    )),
                }
            }
        }
    } else {
        TokenStream::new()
    };

    // Data variants hydrate from LoroMap with variant name as key
    let map_variant_arms: Vec<_> = data
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.ident;
            let variant_str = variant_name.to_string();

            match &v.fields {
                Fields::Unit => {
                    // Unit variant in a map: key exists with string value
                    quote! {
                        if let Some(loro::ValueOrContainer::Value(loro::LoroValue::String(s))) = map.get(#variant_str) {
                            if s.as_ref() == #variant_str {
                                return Ok(#name::#variant_name);
                            }
                        }
                    }
                }
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    // Single-element tuple: LoroMap { "Variant": value }
                    let inner_ty = &fields.unnamed[0].ty;
                    quote! {
                        if let Some(inner) = map.get(#variant_str) {
                            return <#inner_ty as lorosurgeon::Hydrate>::hydrate(&inner)
                                .map(#name::#variant_name);
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    // Multi-element tuple: LoroMap { "Variant": LoroList }
                    let _field_count = fields.unnamed.len();
                    let field_hydrations: Vec<_> = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let ty = &f.ty;
                            quote! {
                                lorosurgeon::hydrate_list_item::<#ty>(&list, #i)?
                            }
                        })
                        .collect();

                    quote! {
                        if let Some(loro::ValueOrContainer::Container(loro::Container::List(list))) = map.get(#variant_str) {
                            return Ok(#name::#variant_name(#(#field_hydrations),*));
                        }
                    }
                }
                Fields::Named(fields) => {
                    // Named fields: LoroMap { "Variant": LoroMap { field: value, ... } }
                    let field_hydrations: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| {
                            let field_name = f.ident.as_ref().unwrap();
                            let field_attrs = FieldAttrs::from_attrs(&f.attrs).unwrap_or_default();
                            let loro_key = field_attrs.loro_key(&field_name.to_string());
                            let ty = &f.ty;

                            if is_vec_non_u8(ty) {
                                let inner = extract_vec_inner_type(ty).unwrap();
                                quote! {
                                    #field_name: {
                                        match inner_map.get(#loro_key) {
                                            Some(loro::ValueOrContainer::Container(loro::Container::List(list))) => {
                                                lorosurgeon::hydrate_vec_from_list::<#inner>(&list)?
                                            }
                                            Some(_) => return Err(lorosurgeon::HydrateError::unexpected("list", "other")),
                                            None => Vec::new(),
                                        }
                                    },
                                }
                            } else if is_option_type(ty) {
                                quote! {
                                    #field_name: lorosurgeon::hydrate_prop_or_default(&inner_map, #loro_key)?,
                                }
                            } else {
                                quote! {
                                    #field_name: lorosurgeon::hydrate_prop(&inner_map, #loro_key)?,
                                }
                            }
                        })
                        .collect();

                    quote! {
                        if let Some(loro::ValueOrContainer::Container(loro::Container::Map(inner_map))) = map.get(#variant_str) {
                            return Ok(#name::#variant_name {
                                #(#field_hydrations)*
                            });
                        }
                    }
                }
            }
        })
        .collect();

    let hydrate_map_fn = if has_data_variants || has_unit_variants {
        let name_str = name.to_string();
        quote! {
            fn hydrate_map(map: &loro::LoroMap) -> Result<Self, lorosurgeon::HydrateError> {
                #(#map_variant_arms)*
                Err(lorosurgeon::HydrateError::unexpected(
                    concat!(#name_str, " variant"),
                    "unknown variant in map",
                ))
            }
        }
    } else {
        TokenStream::new()
    };

    // For purely unit enums, also support hydration from LoroValue directly
    let hydrate_value_override = if !has_data_variants && has_unit_variants {
        quote! {
            fn hydrate(source: &loro::ValueOrContainer) -> Result<Self, lorosurgeon::HydrateError> {
                match source {
                    loro::ValueOrContainer::Value(v) => Self::hydrate_value(v),
                    loro::ValueOrContainer::Container(loro::Container::Map(m)) => Self::hydrate_map(m),
                    _ => Err(lorosurgeon::HydrateError::unexpected("string or map", "other container")),
                }
            }
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        impl #impl_generics lorosurgeon::Hydrate for #name #ty_generics #where_clause {
            #hydrate_value_override
            #hydrate_string_fn
            #hydrate_map_fn
        }
    })
}
