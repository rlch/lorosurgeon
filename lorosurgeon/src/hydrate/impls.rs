//! Hydrate implementations for built-in types.

use std::collections::{BTreeMap, HashMap};

use loro::{LoroList, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};

use crate::error::HydrateError;
use crate::hydrate::Hydrate;

// ── Boolean ─────────────────────────────────────────────────────────────

impl Hydrate for bool {
    fn hydrate_bool(b: bool) -> Result<Self, HydrateError> {
        Ok(b)
    }
}

// ── Signed integers ─────────────────────────────────────────────────────

macro_rules! impl_hydrate_signed {
    ($($t:ty),*) => {
        $(
            impl Hydrate for $t {
                fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
                    <$t>::try_from(i).map_err(|_| HydrateError::Overflow {
                        value: i,
                        target_type: stringify!($t),
                    })
                }
            }
        )*
    };
}

impl_hydrate_signed!(i8, i16, i32);

impl Hydrate for i64 {
    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        Ok(i)
    }
}

// ── Unsigned integers ───────────────────────────────────────────────────

macro_rules! impl_hydrate_unsigned {
    ($($t:ty),*) => {
        $(
            impl Hydrate for $t {
                fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
                    <$t>::try_from(i).map_err(|_| HydrateError::Overflow {
                        value: i,
                        target_type: stringify!($t),
                    })
                }
            }
        )*
    };
}

impl_hydrate_unsigned!(u8, u16, u32, u64);

impl Hydrate for usize {
    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        usize::try_from(i).map_err(|_| HydrateError::Overflow {
            value: i,
            target_type: "usize",
        })
    }
}

// ── Floating point ──────────────────────────────────────────────────────

impl Hydrate for f64 {
    fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
        Ok(f)
    }

    // Accept integers as floats.
    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        Ok(i as f64)
    }
}

impl Hydrate for f32 {
    fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
        Ok(f as f32)
    }

    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        Ok(i as f32)
    }
}

// ── String ──────────────────────────────────────────────────────────────

impl Hydrate for String {
    fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
        Ok(s.to_string())
    }
}

// ── Vec<u8> (Binary) ────────────────────────────────────────────────────

// Note: Vec<u8> is special-cased as binary. For Vec<T> where T != u8,
// we impl via LoroList (see below).

// ── Option<T> ───────────────────────────────────────────────────────────

impl<T: Hydrate> Hydrate for Option<T> {
    fn hydrate(source: &ValueOrContainer) -> Result<Self, HydrateError> {
        match source {
            ValueOrContainer::Value(LoroValue::Null) => Ok(None),
            other => T::hydrate(other).map(Some),
        }
    }

    fn hydrate_null() -> Result<Self, HydrateError> {
        Ok(None)
    }

    fn hydrate_bool(b: bool) -> Result<Self, HydrateError> {
        T::hydrate_bool(b).map(Some)
    }

    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        T::hydrate_i64(i).map(Some)
    }

    fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
        T::hydrate_f64(f).map(Some)
    }

    fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
        T::hydrate_string(s).map(Some)
    }

    fn hydrate_binary(b: &[u8]) -> Result<Self, HydrateError> {
        T::hydrate_binary(b).map(Some)
    }

    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        T::hydrate_map(map).map(Some)
    }

    fn hydrate_list(list: &LoroList) -> Result<Self, HydrateError> {
        T::hydrate_list(list).map(Some)
    }

    fn hydrate_movable_list(list: &LoroMovableList) -> Result<Self, HydrateError> {
        T::hydrate_movable_list(list).map(Some)
    }

    fn hydrate_text(text: &loro::LoroText) -> Result<Self, HydrateError> {
        T::hydrate_text(text).map(Some)
    }
}

// ── Vec<T> (from LoroList) ──────────────────────────────────────────────

impl Hydrate for Vec<u8> {
    fn hydrate_binary(b: &[u8]) -> Result<Self, HydrateError> {
        Ok(b.to_vec())
    }

    // Also support reading from a LoroList of integers.
    fn hydrate_list(list: &LoroList) -> Result<Self, HydrateError> {
        let mut result = Vec::with_capacity(list.len());
        for i in 0..list.len() {
            match list.get(i) {
                Some(voc) => result.push(u8::hydrate(&voc)?),
                None => return Err(HydrateError::missing(format!("[{i}]"))),
            }
        }
        Ok(result)
    }
}

// We can't blanket-impl Vec<T> because of the Vec<u8> specialization.
// Instead, provide a helper and use it in the derive macro.

/// Hydrate a `Vec<T>` from a LoroList. Used by derive macros.
pub fn hydrate_vec_from_list<T: Hydrate>(list: &LoroList) -> Result<Vec<T>, HydrateError> {
    let mut result = Vec::with_capacity(list.len());
    for i in 0..list.len() {
        match list.get(i) {
            Some(voc) => result.push(T::hydrate(&voc)?),
            None => return Err(HydrateError::missing(format!("[{i}]"))),
        }
    }
    Ok(result)
}

/// Hydrate a `Vec<T>` from a LoroMovableList. Used by derive macros.
pub fn hydrate_vec_from_movable_list<T: Hydrate>(
    list: &LoroMovableList,
) -> Result<Vec<T>, HydrateError> {
    let mut result = Vec::with_capacity(list.len());
    for i in 0..list.len() {
        match list.get(i) {
            Some(voc) => result.push(T::hydrate(&voc)?),
            None => return Err(HydrateError::missing(format!("[{i}]"))),
        }
    }
    Ok(result)
}

// ── HashMap<String, V> ─────────────────────────────────────────────────

impl<V: Hydrate> Hydrate for HashMap<String, V> {
    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        hydrate_string_map(map)
    }
}

impl<V: Hydrate> Hydrate for BTreeMap<String, V> {
    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        let hash_map: HashMap<String, V> = hydrate_string_map(map)?;
        Ok(hash_map.into_iter().collect())
    }
}

fn hydrate_string_map<V: Hydrate, M: FromIterator<(String, V)>>(
    map: &LoroMap,
) -> Result<M, HydrateError> {
    let mut pairs = Vec::new();
    map.for_each(|key, voc| {
        pairs.push((key.to_string(), voc));
    });
    pairs
        .into_iter()
        .map(|(k, voc)| V::hydrate(&voc).map(|v| (k, v)))
        .collect()
}

// ── HashMap<K, V> with K: From<String> ────────────────────────────────

/// Hydrate a HashMap with non-String keys that implement `From<String>`.
pub fn hydrate_keyed_map<K, V>(map: &LoroMap) -> Result<HashMap<K, V>, HydrateError>
where
    K: From<String> + Eq + std::hash::Hash,
    V: Hydrate,
{
    let mut pairs = Vec::new();
    map.for_each(|key, voc| {
        pairs.push((key.to_string(), voc));
    });
    pairs
        .into_iter()
        .map(|(k, voc)| V::hydrate(&voc).map(|v| (K::from(k), v)))
        .collect()
}

// ── Box<T> ────────────────────────────────────────────────────────────

impl<T: Hydrate> Hydrate for Box<T> {
    fn hydrate(source: &ValueOrContainer) -> Result<Self, HydrateError> {
        T::hydrate(source).map(Box::new)
    }

    fn hydrate_value(value: &LoroValue) -> Result<Self, HydrateError> {
        T::hydrate_value(value).map(Box::new)
    }

    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        T::hydrate_map(map).map(Box::new)
    }

    fn hydrate_list(list: &LoroList) -> Result<Self, HydrateError> {
        T::hydrate_list(list).map(Box::new)
    }

    fn hydrate_movable_list(list: &LoroMovableList) -> Result<Self, HydrateError> {
        T::hydrate_movable_list(list).map(Box::new)
    }

    fn hydrate_text(text: &loro::LoroText) -> Result<Self, HydrateError> {
        T::hydrate_text(text).map(Box::new)
    }

    fn hydrate_null() -> Result<Self, HydrateError> {
        T::hydrate_null().map(Box::new)
    }

    fn hydrate_bool(b: bool) -> Result<Self, HydrateError> {
        T::hydrate_bool(b).map(Box::new)
    }

    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        T::hydrate_i64(i).map(Box::new)
    }

    fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
        T::hydrate_f64(f).map(Box::new)
    }

    fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
        T::hydrate_string(s).map(Box::new)
    }

    fn hydrate_binary(b: &[u8]) -> Result<Self, HydrateError> {
        T::hydrate_binary(b).map(Box::new)
    }
}

// ── Cow<'a, T> ────────────────────────────────────────────────────────

impl<T: Hydrate + Clone> Hydrate for std::borrow::Cow<'_, T> {
    fn hydrate(source: &ValueOrContainer) -> Result<Self, HydrateError> {
        T::hydrate(source).map(std::borrow::Cow::Owned)
    }

    fn hydrate_value(value: &LoroValue) -> Result<Self, HydrateError> {
        T::hydrate_value(value).map(std::borrow::Cow::Owned)
    }

    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        T::hydrate_map(map).map(std::borrow::Cow::Owned)
    }

    fn hydrate_list(list: &LoroList) -> Result<Self, HydrateError> {
        T::hydrate_list(list).map(std::borrow::Cow::Owned)
    }

    fn hydrate_movable_list(list: &LoroMovableList) -> Result<Self, HydrateError> {
        T::hydrate_movable_list(list).map(std::borrow::Cow::Owned)
    }

    fn hydrate_null() -> Result<Self, HydrateError> {
        T::hydrate_null().map(std::borrow::Cow::Owned)
    }

    fn hydrate_bool(b: bool) -> Result<Self, HydrateError> {
        T::hydrate_bool(b).map(std::borrow::Cow::Owned)
    }

    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        T::hydrate_i64(i).map(std::borrow::Cow::Owned)
    }

    fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
        T::hydrate_f64(f).map(std::borrow::Cow::Owned)
    }

    fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
        T::hydrate_string(s).map(std::borrow::Cow::Owned)
    }

    fn hydrate_binary(b: &[u8]) -> Result<Self, HydrateError> {
        T::hydrate_binary(b).map(std::borrow::Cow::Owned)
    }
}

// ── serde_json::Value ───────────────────────────────────────────────────

impl Hydrate for serde_json::Value {
    fn hydrate_null() -> Result<Self, HydrateError> {
        Ok(serde_json::Value::Null)
    }

    fn hydrate_bool(b: bool) -> Result<Self, HydrateError> {
        Ok(serde_json::Value::Bool(b))
    }

    fn hydrate_i64(i: i64) -> Result<Self, HydrateError> {
        Ok(serde_json::Value::Number(i.into()))
    }

    fn hydrate_f64(f: f64) -> Result<Self, HydrateError> {
        Ok(serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null))
    }

    fn hydrate_string(s: &str) -> Result<Self, HydrateError> {
        Ok(serde_json::Value::String(s.to_string()))
    }

    fn hydrate_map(map: &LoroMap) -> Result<Self, HydrateError> {
        // Deep-convert via LoroValue
        let deep = map.get_deep_value();
        Ok(loro_value_to_json(&deep))
    }

    fn hydrate_list(list: &LoroList) -> Result<Self, HydrateError> {
        let deep = list.get_deep_value();
        Ok(loro_value_to_json(&deep))
    }
}

fn loro_value_to_json(v: &LoroValue) -> serde_json::Value {
    match v {
        LoroValue::Null => serde_json::Value::Null,
        LoroValue::Bool(b) => serde_json::Value::Bool(*b),
        LoroValue::I64(i) => serde_json::Value::Number((*i).into()),
        LoroValue::Double(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        LoroValue::String(s) => serde_json::Value::String(s.to_string()),
        LoroValue::Binary(b) => serde_json::Value::Array(
            b.iter()
                .map(|byte| serde_json::Value::Number((*byte as i64).into()))
                .collect(),
        ),
        LoroValue::List(list) => {
            serde_json::Value::Array(list.iter().map(loro_value_to_json).collect())
        }
        LoroValue::Map(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.to_string(), loro_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        LoroValue::Container(_) => serde_json::Value::Null,
    }
}
