//! Attribute parsing for `#[loro(...)]` and `#[key]`.

use syn::{Attribute, Lit};

/// Container-level attributes from `#[loro(...)]` on the struct/enum.
#[derive(Debug, Default)]
pub struct ContainerAttrs {
    /// Root key for DocSync: `#[loro(root = "key")]`
    pub root: Option<String>,
}

/// Field-level attributes from `#[loro(...)]` and `#[key]`.
#[derive(Debug, Default)]
pub struct FieldAttrs {
    /// This field is the identity key: `#[key]`
    pub is_key: bool,
    /// Rename the Loro key: `#[loro(rename = "name")]`
    pub rename: Option<String>,
    /// Use serde_json for serialization: `#[loro(json)]`
    pub json: bool,
    /// Use LoroMovableList instead of LoroList: `#[loro(movable)]`
    pub movable: bool,
    /// Default when missing: `#[loro(default)]` or `#[loro(default = "fn_name")]`
    pub missing: Option<MissingStrategy>,
    /// Custom module with hydrate/reconcile fns: `#[loro(with = "module")]`
    pub with_module: Option<String>,
    /// Custom hydrate function: `#[loro(hydrate = "fn")]`
    pub custom_hydrate: Option<String>,
    /// Custom reconcile function: `#[loro(reconcile = "fn")]`
    pub custom_reconcile: Option<String>,
    /// Flatten nested struct fields into parent map: `#[loro(flatten)]`
    pub flatten: bool,
}

#[derive(Debug)]
pub enum MissingStrategy {
    /// Use `Default::default()`
    Default,
    /// Use a custom function
    Function(String),
}

impl ContainerAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("loro") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("root") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.root = Some(s.value());
                    }
                }
                Ok(())
            })?;
        }

        Ok(result)
    }
}

impl FieldAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            if attr.path().is_ident("key") {
                result.is_key = true;
                continue;
            }

            if !attr.path().is_ident("loro") {
                continue;
            }

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.rename = Some(s.value());
                    }
                } else if meta.path.is_ident("json") {
                    result.json = true;
                } else if meta.path.is_ident("movable") {
                    result.movable = true;
                } else if meta.path.is_ident("default") {
                    // Check if there's a value: `default = "fn_name"`
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            result.missing = Some(MissingStrategy::Function(s.value()));
                        }
                    } else {
                        result.missing = Some(MissingStrategy::Default);
                    }
                } else if meta.path.is_ident("with") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.with_module = Some(s.value());
                    }
                } else if meta.path.is_ident("hydrate") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.custom_hydrate = Some(s.value());
                    }
                } else if meta.path.is_ident("reconcile") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.custom_reconcile = Some(s.value());
                    }
                } else if meta.path.is_ident("flatten") {
                    result.flatten = true;
                }
                Ok(())
            })?;
        }

        Ok(result)
    }

    /// Get the Loro key name for this field.
    pub fn loro_key(&self, field_name: &str) -> String {
        self.rename
            .clone()
            .unwrap_or_else(|| field_name.to_string())
    }
}
