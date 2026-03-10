//! Shared type analysis utilities for derive macros.

/// Check if a type is `Vec<T>` where T is not `u8`.
pub fn is_vec_non_u8(ty: &syn::Type) -> bool {
    extract_vec_inner_type(ty).is_some_and(|inner| !is_u8_type(inner))
}

/// Extract the inner type from `Vec<T>`, returning `Some(T)` or `None`.
pub fn extract_vec_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

/// Check if a type is `u8`.
pub fn is_u8_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "u8" && segment.arguments.is_none();
        }
    }
    false
}

/// Check if a type is `Option<...>`.
pub fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}
