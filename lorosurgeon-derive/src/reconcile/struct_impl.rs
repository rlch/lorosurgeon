//! Reconcile derive for structs.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, DeriveInput, Fields, Ident};

use crate::attrs::{ContainerAttrs, FieldAttrs};
use crate::type_util::is_vec_non_u8;

pub fn derive_reconcile_struct(input: &DeriveInput, data: &DataStruct) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let container_attrs = ContainerAttrs::from_attrs(&input.attrs)?;

    let (reconcile_body, key_type, key_fn, hydrate_key_fn) = match &data.fields {
        Fields::Named(fields) => derive_named_struct(name, fields)?,
        Fields::Unnamed(fields) => derive_tuple_struct(name, fields)?,
        Fields::Unit => derive_unit_struct(name)?,
    };

    let doc_sync_impl = if let Some(root_key) = &container_attrs.root {
        quote! {
            impl #impl_generics lorosurgeon::DocSync for #name #ty_generics #where_clause {
                const ROOT_KEY: &'static str = #root_key;
            }
        }
    } else {
        TokenStream::new()
    };

    Ok(quote! {
        impl #impl_generics lorosurgeon::Reconcile for #name #ty_generics #where_clause {
            type Key = #key_type;

            #reconcile_body
            #key_fn
            #hydrate_key_fn
        }
        #doc_sync_impl
    })
}

fn derive_named_struct(
    _name: &Ident,
    fields: &syn::FieldsNamed,
) -> syn::Result<(TokenStream, TokenStream, TokenStream, TokenStream)> {
    let mut field_reconciliations = Vec::new();
    let mut key_field: Option<(Ident, syn::Type)> = None;

    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap();
        let attrs = FieldAttrs::from_attrs(&field.attrs)?;
        let loro_key = attrs.loro_key(&field_name.to_string());

        if attrs.is_key {
            key_field = Some((field_name.clone(), field.ty.clone()));
        }

        if attrs.flatten {
            // Flatten: delegate to inner type's reconcile, which writes directly to the map
            field_reconciliations.push(quote! {
                // Flatten: reconcile inner fields directly into this map
                {
                    let inner_reconciler = lorosurgeon::RootReconciler::new(m.map.clone());
                    lorosurgeon::Reconcile::reconcile(&self.#field_name, inner_reconciler)?;
                }
            });
            continue;
        }

        let reconciliation = if let Some(ref module) = attrs.with_module {
            let mod_path: syn::Path = syn::parse_str(module)?;
            quote! {
                #mod_path::reconcile(&self.#field_name, &mut m, #loro_key)?;
            }
        } else if let Some(ref func) = attrs.custom_reconcile {
            let func_path: syn::Path = syn::parse_str(func)?;
            quote! {
                #func_path(&self.#field_name, &mut m, #loro_key)?;
            }
        } else if attrs.json {
            quote! {
                {
                    let json_str: String = serde_json::to_string(&self.#field_name)
                        .map_err(lorosurgeon::ReconcileError::from)?;
                    m.entry(#loro_key, &json_str)?;
                }
            }
        } else if attrs.movable {
            // Use movable list reconciliation
            quote! {
                {
                    let reconciler = lorosurgeon::PropReconciler::map_put(m.map.clone(), #loro_key.to_string());
                    lorosurgeon::reconcile_vec_movable(&self.#field_name, reconciler)?;
                }
            }
        } else if is_vec_non_u8(&field.ty) {
            // Vec<T> (non-u8) — use reconcile_vec (LoroList clear+rewrite)
            quote! {
                {
                    let reconciler = lorosurgeon::PropReconciler::map_put(m.map.clone(), #loro_key.to_string());
                    lorosurgeon::reconcile_vec(&self.#field_name, reconciler)?;
                }
            }
        } else {
            quote! {
                m.entry(#loro_key, &self.#field_name)?;
            }
        };

        field_reconciliations.push(reconciliation);
    }

    let reconcile_body = quote! {
        fn reconcile<R: lorosurgeon::Reconciler>(&self, r: R) -> Result<(), lorosurgeon::ReconcileError> {
            let mut m = lorosurgeon::Reconciler::map(r)?;
            #(#field_reconciliations)*
            Ok(())
        }
    };

    let (key_type, key_fn, hydrate_key_fn) = if let Some((key_name, key_ty)) = key_field {
        let key_str = key_name.to_string();
        (
            // The key type IS the field type itself (String, i64, etc.)
            quote! { #key_ty },
            quote! {
                fn key(&self) -> lorosurgeon::LoadKey<Self::Key> {
                    lorosurgeon::LoadKey::Found(self.#key_name.clone())
                }
            },
            quote! {
                fn hydrate_key(source: &loro::ValueOrContainer) -> Result<lorosurgeon::LoadKey<Self::Key>, lorosurgeon::ReconcileError> {
                    match source {
                        loro::ValueOrContainer::Container(loro::Container::Map(map)) => {
                            match map.get(#key_str) {
                                Some(voc) => {
                                    Ok(lorosurgeon::LoadKey::Found(
                                        <#key_ty as lorosurgeon::Hydrate>::hydrate(&voc)
                                            .map_err(|_| lorosurgeon::ReconcileError::TypeMismatch {
                                                expected: "key value",
                                                found: "incompatible type",
                                            })?
                                    ))
                                }
                                None => Ok(lorosurgeon::LoadKey::KeyNotFound),
                            }
                        }
                        _ => Ok(lorosurgeon::LoadKey::KeyNotFound),
                    }
                }
            },
        )
    } else {
        (
            quote! { lorosurgeon::NoKey },
            TokenStream::new(),
            TokenStream::new(),
        )
    };

    Ok((reconcile_body, key_type, key_fn, hydrate_key_fn))
}

fn derive_tuple_struct(
    _name: &Ident,
    fields: &syn::FieldsUnnamed,
) -> syn::Result<(TokenStream, TokenStream, TokenStream, TokenStream)> {
    if fields.unnamed.len() == 1 {
        let inner_ty = &fields.unnamed[0].ty;

        // Newtype over Vec<T> (non-u8) — use list reconciliation
        if is_vec_non_u8(inner_ty) {
            let reconcile_body = quote! {
                fn reconcile<R: lorosurgeon::Reconciler>(&self, r: R) -> Result<(), lorosurgeon::ReconcileError> {
                    lorosurgeon::reconcile_vec_simple(&self.0, r)
                }
            };
            return Ok((
                reconcile_body,
                quote! { lorosurgeon::NoKey },
                TokenStream::new(),
                TokenStream::new(),
            ));
        }

        // Newtype — transparent delegation
        let reconcile_body = quote! {
            fn reconcile<R: lorosurgeon::Reconciler>(&self, r: R) -> Result<(), lorosurgeon::ReconcileError> {
                self.0.reconcile(r)
            }
        };
        Ok((
            reconcile_body,
            quote! { <#inner_ty as lorosurgeon::Reconcile>::Key },
            quote! {
                fn key(&self) -> lorosurgeon::LoadKey<Self::Key> {
                    self.0.key()
                }
            },
            TokenStream::new(),
        ))
    } else {
        // Tuple struct with 2+ fields — LoroList positionally
        let field_indices: Vec<_> = (0..fields.unnamed.len()).map(syn::Index::from).collect();

        let reconcile_body = quote! {
            fn reconcile<R: lorosurgeon::Reconciler>(&self, r: R) -> Result<(), lorosurgeon::ReconcileError> {
                let mut l = r.list().map_err(Into::into)?;
                #(
                    l.insert(#field_indices, &self.#field_indices)?;
                )*
                Ok(())
            }
        };

        Ok((
            reconcile_body,
            quote! { lorosurgeon::NoKey },
            TokenStream::new(),
            TokenStream::new(),
        ))
    }
}

fn derive_unit_struct(
    _name: &Ident,
) -> syn::Result<(TokenStream, TokenStream, TokenStream, TokenStream)> {
    let reconcile_body = quote! {
        fn reconcile<R: lorosurgeon::Reconciler>(&self, r: R) -> Result<(), lorosurgeon::ReconcileError> {
            r.null().map_err(Into::into)
        }
    };

    Ok((
        reconcile_body,
        quote! { lorosurgeon::NoKey },
        TokenStream::new(),
        TokenStream::new(),
    ))
}
