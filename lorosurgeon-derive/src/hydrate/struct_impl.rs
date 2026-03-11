//! Hydrate derive for structs.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, DeriveInput, Fields, Ident};

use crate::attrs::{FieldAttrs, MissingStrategy};
use crate::type_util::{extract_vec_inner_type, is_option_type, is_u8_type, is_vec_non_u8};

pub fn derive_hydrate_struct(input: &DeriveInput, data: &DataStruct) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let hydrate_body = match &data.fields {
        Fields::Named(fields) => derive_named_struct(name, fields)?,
        Fields::Unnamed(fields) => derive_tuple_struct(name, fields)?,
        Fields::Unit => derive_unit_struct(name)?,
    };

    Ok(quote! {
        impl #impl_generics lorosurgeon::Hydrate for #name #ty_generics #where_clause {
            #hydrate_body
        }
    })
}

fn derive_named_struct(name: &Ident, fields: &syn::FieldsNamed) -> syn::Result<TokenStream> {
    let mut field_hydrations = Vec::new();
    let mut flatten_fields = Vec::new();

    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap();
        let attrs = FieldAttrs::from_attrs(&field.attrs)?;
        let loro_key = attrs.loro_key(&field_name.to_string());
        let field_ty = &field.ty;

        if attrs.flatten {
            // Flattened fields are handled by delegating to the inner type's hydrate_map
            // but we need to inline them. We'll generate a separate hydration.
            flatten_fields.push((field_name.clone(), field_ty.clone()));
            continue;
        }

        let hydration = if let Some(ref module) = attrs.with_module {
            let mod_path: syn::Path = syn::parse_str(module)?;
            quote! {
                #field_name: #mod_path::hydrate(map, #loro_key)?,
            }
        } else if let Some(ref func) = attrs.custom_hydrate {
            let func_path: syn::Path = syn::parse_str(func)?;
            quote! {
                #field_name: #func_path(map, #loro_key)?,
            }
        } else if attrs.json {
            match &attrs.missing {
                Some(MissingStrategy::Default) => quote! {
                    #field_name: lorosurgeon::hydrate_prop_json_or_default(map, #loro_key)?,
                },
                Some(MissingStrategy::Function(f)) => {
                    let func_path: syn::Path = syn::parse_str(f)?;
                    quote! {
                        #field_name: lorosurgeon::hydrate_prop_json_or_default(map, #loro_key)
                            .unwrap_or_else(|_| #func_path()),
                    }
                }
                None => quote! {
                    #field_name: lorosurgeon::hydrate_prop_json(map, #loro_key)?,
                },
            }
        } else if attrs.text {
            quote! {
                #field_name: lorosurgeon::hydrate_text_prop(map, #loro_key)?,
            }
        } else if attrs.movable {
            // #[loro(movable)] — hydrate Vec<T> from LoroMovableList
            let inner_ty = extract_vec_inner_type(field_ty);
            match inner_ty {
                Some(inner) => quote! {
                    #field_name: {
                        match map.get(#loro_key) {
                            Some(loro::ValueOrContainer::Container(loro::Container::MovableList(list))) => {
                                lorosurgeon::hydrate_vec_from_movable_list::<#inner>(&list)?
                            }
                            Some(_) => return Err(lorosurgeon::HydrateError::unexpected("movable_list", "other")),
                            None => Vec::new(),
                        }
                    },
                },
                None => {
                    return Err(syn::Error::new_spanned(
                        field,
                        "#[loro(movable)] can only be used on Vec<T> fields",
                    ));
                }
            }
        } else {
            // Check if it's a Vec<T> (non-u8) — use hydrate_vec_from_list
            let vec_inner = extract_vec_inner_type(field_ty);
            let is_vec_non_u8 = vec_inner.as_ref().is_some_and(|inner| !is_u8_type(inner));

            if is_vec_non_u8 {
                let inner = vec_inner.unwrap();
                match &attrs.missing {
                    Some(MissingStrategy::Default) | None => quote! {
                        #field_name: {
                            match map.get(#loro_key) {
                                Some(loro::ValueOrContainer::Container(loro::Container::List(list))) => {
                                    lorosurgeon::hydrate_vec_from_list::<#inner>(&list)?
                                }
                                Some(_) => return Err(lorosurgeon::HydrateError::unexpected("list", "other")),
                                None => Vec::new(),
                            }
                        },
                    },
                    Some(MissingStrategy::Function(f)) => {
                        let func_path: syn::Path = syn::parse_str(f)?;
                        quote! {
                            #field_name: {
                                match map.get(#loro_key) {
                                    Some(loro::ValueOrContainer::Container(loro::Container::List(list))) => {
                                        lorosurgeon::hydrate_vec_from_list::<#inner>(&list)?
                                    }
                                    Some(_) => return Err(lorosurgeon::HydrateError::unexpected("list", "other")),
                                    None => #func_path(),
                                }
                            },
                        }
                    }
                }
            } else {
                match &attrs.missing {
                    Some(MissingStrategy::Default) => quote! {
                        #field_name: lorosurgeon::hydrate_prop_or_default(map, #loro_key)?,
                    },
                    Some(MissingStrategy::Function(f)) => {
                        let func_path: syn::Path = syn::parse_str(f)?;
                        quote! {
                            #field_name: lorosurgeon::hydrate_prop_or_else(map, #loro_key, #func_path)?,
                        }
                    }
                    None => {
                        if is_option_type(field_ty) {
                            quote! {
                                #field_name: lorosurgeon::hydrate_prop_or_default(map, #loro_key)?,
                            }
                        } else {
                            quote! {
                                #field_name: lorosurgeon::hydrate_prop(map, #loro_key)?,
                            }
                        }
                    }
                }
            }
        };

        field_hydrations.push(hydration);
    }

    // Handle flattened fields
    for (field_name, field_ty) in &flatten_fields {
        field_hydrations.push(quote! {
            #field_name: <#field_ty as lorosurgeon::Hydrate>::hydrate_map(map)?,
        });
    }

    Ok(quote! {
        fn hydrate_map(map: &loro::LoroMap) -> Result<Self, lorosurgeon::HydrateError> {
            Ok(#name {
                #(#field_hydrations)*
            })
        }
    })
}

fn derive_tuple_struct(name: &Ident, fields: &syn::FieldsUnnamed) -> syn::Result<TokenStream> {
    if fields.unnamed.len() == 1 {
        let inner_ty = &fields.unnamed[0].ty;

        // Newtype over Vec<T> (non-u8) — hydrate from LoroList
        if is_vec_non_u8(inner_ty) {
            let elem_ty = extract_vec_inner_type(inner_ty).unwrap();
            return Ok(quote! {
                fn hydrate_list(list: &loro::LoroList) -> Result<Self, lorosurgeon::HydrateError> {
                    lorosurgeon::hydrate_vec_from_list::<#elem_ty>(list).map(#name)
                }
            });
        }

        // Newtype — transparent delegation
        Ok(quote! {
            fn hydrate(source: &loro::ValueOrContainer) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate(source).map(#name)
            }

            fn hydrate_map(map: &loro::LoroMap) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_map(map).map(#name)
            }

            fn hydrate_value(value: &loro::LoroValue) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_value(value).map(#name)
            }

            fn hydrate_list(list: &loro::LoroList) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_list(list).map(#name)
            }

            fn hydrate_null() -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_null().map(#name)
            }

            fn hydrate_bool(b: bool) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_bool(b).map(#name)
            }

            fn hydrate_i64(i: i64) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_i64(i).map(#name)
            }

            fn hydrate_f64(f: f64) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_f64(f).map(#name)
            }

            fn hydrate_string(s: &str) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_string(s).map(#name)
            }

            fn hydrate_binary(b: &[u8]) -> Result<Self, lorosurgeon::HydrateError> {
                <#inner_ty as lorosurgeon::Hydrate>::hydrate_binary(b).map(#name)
            }
        })
    } else {
        // Tuple struct with 2+ fields — use LoroList positionally
        let field_count = fields.unnamed.len();
        let field_hydrations: Vec<_> = fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let ty = &f.ty;
                quote! {
                    lorosurgeon::hydrate_list_item::<#ty>(list, #i)?
                }
            })
            .collect();

        Ok(quote! {
            fn hydrate_list(list: &loro::LoroList) -> Result<Self, lorosurgeon::HydrateError> {
                if list.len() != #field_count {
                    return Err(lorosurgeon::HydrateError::unexpected(
                        concat!("list of length ", stringify!(#field_count)),
                        "list of different length",
                    ));
                }
                Ok(#name(#(#field_hydrations),*))
            }
        })
    }
}

fn derive_unit_struct(name: &Ident) -> syn::Result<TokenStream> {
    Ok(quote! {
        fn hydrate_null() -> Result<Self, lorosurgeon::HydrateError> {
            Ok(#name)
        }
    })
}
