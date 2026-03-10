//! Read Rust types from Loro containers.
//!
//! The [`Hydrate`] trait converts Loro values and containers into Rust types.
//! It uses a visitor-style dispatch: the default [`Hydrate::hydrate`] method
//! inspects the [`ValueOrContainer`] and calls the
//! appropriate type-specific method (`hydrate_map`, `hydrate_i64`, etc.).
//!
//! Implementations are provided for all scalar types, `Option<T>`, `Vec<T>`,
//! `HashMap<String, V>`, `Box<T>`, `Cow<T>`, and `serde_json::Value`.
//! Use `#[derive(Hydrate)]` to generate implementations for your own types.

pub mod impls;

use loro::{
    Container, LoroDoc, LoroList, LoroMap, LoroMovableList, LoroText, LoroValue, ValueOrContainer,
};

use crate::error::HydrateError;

/// Read a Rust value from a Loro container or value.
///
/// # Implementing
///
/// Override the method matching your Loro storage type:
///
/// - Scalars: `hydrate_bool`, `hydrate_i64`, `hydrate_f64`, `hydrate_string`, `hydrate_binary`
/// - Containers: `hydrate_map`, `hydrate_list`, `hydrate_movable_list`, `hydrate_text`
/// - Null: `hydrate_null` (used by `Option<T>` and `MaybeMissing<T>`)
///
/// The default [`hydrate`](Hydrate::hydrate) dispatches automatically based on the
/// Loro value type — you rarely need to override it.
pub trait Hydrate: Sized {
    /// Primary dispatch — inspects the source and calls the appropriate typed method.
    fn hydrate(source: &ValueOrContainer) -> Result<Self, HydrateError> {
        match source {
            ValueOrContainer::Value(v) => Self::hydrate_value(v),
            ValueOrContainer::Container(c) => match c {
                Container::Map(m) => Self::hydrate_map(m),
                Container::List(l) => Self::hydrate_list(l),
                Container::MovableList(l) => Self::hydrate_movable_list(l),
                Container::Text(t) => Self::hydrate_text(t),
                _ => Err(HydrateError::unexpected("known container", "unknown")),
            },
        }
    }

    /// Dispatch on a LoroValue (scalar).
    fn hydrate_value(value: &LoroValue) -> Result<Self, HydrateError> {
        match value {
            LoroValue::Null => Self::hydrate_null(),
            LoroValue::Bool(b) => Self::hydrate_bool(*b),
            LoroValue::I64(i) => Self::hydrate_i64(*i),
            LoroValue::Double(f) => Self::hydrate_f64(*f),
            LoroValue::String(s) => Self::hydrate_string(s),
            LoroValue::Binary(b) => Self::hydrate_binary(b),
            LoroValue::List(_) | LoroValue::Map(_) => {
                Err(HydrateError::unexpected("scalar", "inline collection"))
            }
            LoroValue::Container(_) => Err(HydrateError::unexpected("scalar", "container ref")),
        }
    }

    /// Read from a [`LoroMap`]. Override for structs and map-like types.
    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        let _ = map;
        Err(HydrateError::unexpected("other", "map"))
    }

    /// Read from a [`LoroList`]. Override for `Vec<T>` and tuple structs.
    fn hydrate_list(list: &LoroList) -> Result<Self, HydrateError> {
        let _ = list;
        Err(HydrateError::unexpected("other", "list"))
    }

    /// Read from a [`LoroMovableList`]. Override for `#[loro(movable)]` vecs.
    fn hydrate_movable_list(list: &LoroMovableList) -> Result<Self, HydrateError> {
        let _ = list;
        Err(HydrateError::unexpected("other", "movable_list"))
    }

    /// Read from a [`LoroText`]. Override for [`Text`](crate::Text).
    fn hydrate_text(text: &LoroText) -> Result<Self, HydrateError> {
        let _ = text;
        Err(HydrateError::unexpected("other", "text"))
    }

    /// Read from a null value. Used by `Option<T>` → `None` and `MaybeMissing<T>` → `Missing`.
    fn hydrate_null() -> Result<Self, HydrateError> {
        Err(HydrateError::unexpected("other", "null"))
    }

    /// Read from a boolean value.
    fn hydrate_bool(_b: bool) -> Result<Self, HydrateError> {
        Err(HydrateError::unexpected("other", "bool"))
    }

    /// Read from a 64-bit integer. Used by all integer types (`i8`–`i64`, `u8`–`u64`, `usize`).
    fn hydrate_i64(_i: i64) -> Result<Self, HydrateError> {
        Err(HydrateError::unexpected("other", "i64"))
    }

    /// Read from a 64-bit float. Used by `f32` and `f64`.
    fn hydrate_f64(_f: f64) -> Result<Self, HydrateError> {
        Err(HydrateError::unexpected("other", "f64"))
    }

    /// Read from a string value.
    fn hydrate_string(_s: &str) -> Result<Self, HydrateError> {
        Err(HydrateError::unexpected("other", "string"))
    }

    /// Read from binary data. Used by `Vec<u8>` and [`ByteArray<N>`](crate::ByteArray).
    fn hydrate_binary(_b: &[u8]) -> Result<Self, HydrateError> {
        Err(HydrateError::unexpected("other", "binary"))
    }
}

// ── Top-level helpers ───────────────────────────────────────────────────

/// Hydrate a value from a LoroDoc root map key.
pub fn hydrate<T: Hydrate>(doc: &LoroDoc, root_key: &str) -> Result<T, HydrateError> {
    let map = doc.get_map(root_key);
    T::hydrate_map(&map)
}

/// Hydrate a value from a LoroMap (the map IS the value).
pub fn hydrate_map<T: Hydrate>(map: &LoroMap) -> Result<T, HydrateError> {
    T::hydrate_map(map)
}

/// Hydrate a single property from a LoroMap.
pub fn hydrate_prop<T: Hydrate>(map: &LoroMap, key: &str) -> Result<T, HydrateError> {
    match map.get(key) {
        Some(voc) => T::hydrate(&voc),
        None => T::hydrate_null().map_err(|_| HydrateError::missing(key)),
    }
}

/// Hydrate a property with a default when missing.
pub fn hydrate_prop_or_default<T: Hydrate + Default>(
    map: &LoroMap,
    key: &str,
) -> Result<T, HydrateError> {
    match map.get(key) {
        Some(voc) => T::hydrate(&voc),
        None => Ok(T::default()),
    }
}

/// Hydrate a property with a fallback value when missing.
pub fn hydrate_prop_or<T: Hydrate>(
    map: &LoroMap,
    key: &str,
    default: T,
) -> Result<T, HydrateError> {
    match map.get(key) {
        Some(voc) => T::hydrate(&voc),
        None => Ok(default),
    }
}

/// Hydrate a property with a fallback closure when missing.
pub fn hydrate_prop_or_else<T: Hydrate>(
    map: &LoroMap,
    key: &str,
    default: impl FnOnce() -> T,
) -> Result<T, HydrateError> {
    match map.get(key) {
        Some(voc) => T::hydrate(&voc),
        None => Ok(default()),
    }
}

/// Hydrate a property from JSON string stored in a LoroMap.
pub fn hydrate_prop_json<T: serde::de::DeserializeOwned>(
    map: &LoroMap,
    key: &str,
) -> Result<T, HydrateError> {
    match map.get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => {
            serde_json::from_str(&s).map_err(|e| HydrateError::Json {
                key: key.to_string(),
                source: e,
            })
        }
        Some(_) => Err(HydrateError::unexpected("string (json)", "other")),
        None => Err(HydrateError::missing(key)),
    }
}

/// Hydrate a property from JSON string with a default when missing.
pub fn hydrate_prop_json_or_default<T: serde::de::DeserializeOwned + Default>(
    map: &LoroMap,
    key: &str,
) -> Result<T, HydrateError> {
    match map.get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => {
            serde_json::from_str(&s).map_err(|e| HydrateError::Json {
                key: key.to_string(),
                source: e,
            })
        }
        Some(_) => Err(HydrateError::unexpected("string (json)", "other")),
        None => Ok(T::default()),
    }
}

/// Hydrate an item from a LoroList by index.
pub fn hydrate_list_item<T: Hydrate>(list: &LoroList, index: usize) -> Result<T, HydrateError> {
    match list.get(index) {
        Some(voc) => T::hydrate(&voc),
        None => Err(HydrateError::missing(format!("[{index}]"))),
    }
}
