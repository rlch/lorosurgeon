//! Reconcile derive for enums.

use proc_macro2::TokenStream;
use quote::quote;
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

    Ok(quote! {
        impl #impl_generics lorosurgeon::Reconcile for #name #ty_generics #where_clause {
            type Key = lorosurgeon::NoKey;

            fn reconcile<R: lorosurgeon::Reconciler>(&self, r: R) -> Result<(), lorosurgeon::ReconcileError> {
                match self {
                    #(#match_arms)*
                }
            }
        }
    })
}
