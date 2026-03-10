//! Derive macros for lorosurgeon.

mod attrs;
mod type_util;

mod hydrate {
    pub mod enum_impl;
    pub mod struct_impl;
}

mod reconcile {
    pub mod enum_impl;
    pub mod struct_impl;
}

use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput};

/// Derive the `Hydrate` trait for reading from Loro containers.
///
/// # Structs
/// - Named struct → hydrates from `LoroMap` (fields as keys)
/// - Newtype struct → transparent delegation to inner type
/// - Tuple struct → hydrates from `LoroList` (positional)
///
/// # Enums
/// - Unit variants → hydrates from `String` (variant name)
/// - Tuple/named variants → hydrates from `LoroMap` with variant name as key
///
/// # Attributes
/// - `#[loro(root = "key")]` — generates `DocSync` impl
/// - `#[loro(rename = "name")]` — use different Loro key
/// - `#[loro(json)]` — serde_json round-trip
/// - `#[loro(missing)]` — use `Default::default()` when absent
/// - `#[loro(missing = "fn")]` — use custom fn when absent
/// - `#[loro(with = "module")]` — custom module
/// - `#[loro(hydrate = "fn")]` — custom hydrate fn
/// - `#[loro(flatten)]` — flatten nested struct fields
#[proc_macro_derive(Hydrate, attributes(loro, key))]
pub fn derive_hydrate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let result = match &input.data {
        Data::Struct(data) => hydrate::struct_impl::derive_hydrate_struct(&input, data),
        Data::Enum(data) => hydrate::enum_impl::derive_hydrate_enum(&input, data),
        Data::Union(_) => Err(syn::Error::new_spanned(
            &input,
            "Hydrate cannot be derived for unions",
        )),
    };

    match result {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive the `Reconcile` trait for writing into Loro containers.
///
/// # Structs
/// - Named struct → reconciles as `LoroMap` (fields as entries)
/// - Newtype struct → transparent delegation to inner type
/// - Tuple struct → reconciles as `LoroList` (positional)
///
/// # Enums
/// - Pure unit enum → reconciles as `String`
/// - Mixed enum → reconciles as `LoroMap` with variant name as key
///
/// # Attributes
/// - `#[loro(root = "key")]` — generates `DocSync` impl
/// - `#[key]` — marks field as identity key for list diffing
/// - `#[loro(rename = "name")]` — use different Loro key
/// - `#[loro(json)]` — serde_json round-trip
/// - `#[loro(movable)]` — use `LoroMovableList` instead of `LoroList`
/// - `#[loro(with = "module")]` — custom module
/// - `#[loro(reconcile = "fn")]` — custom reconcile fn
/// - `#[loro(flatten)]` — flatten nested struct fields
#[proc_macro_derive(Reconcile, attributes(loro, key))]
pub fn derive_reconcile(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let result = match &input.data {
        Data::Struct(data) => reconcile::struct_impl::derive_reconcile_struct(&input, data),
        Data::Enum(data) => reconcile::enum_impl::derive_reconcile_enum(&input, data),
        Data::Union(_) => Err(syn::Error::new_spanned(
            &input,
            "Reconcile cannot be derived for unions",
        )),
    };

    match result {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
