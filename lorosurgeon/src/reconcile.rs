//! Write Rust types into Loro containers.
//!
//! The [`Reconcile`] trait writes Rust values into Loro via a [`Reconciler`],
//! which abstracts over the target location (map key, list index, document root).
//! The reconciler handles no-op detection — identical scalar values produce zero
//! CRDT operations.
//!
//! Implementations are provided for all scalar types, `Option<T>`, `Vec<T>`,
//! `HashMap<String, V>`, `Box<T>`, `Cow<T>`, and `serde_json::Value`.
//! Use `#[derive(Reconcile)]` to generate implementations for your own types.
//!
//! # Sub-reconcilers
//!
//! When writing composite types, the reconciler returns typed sub-reconcilers:
//!
//! - [`MapReconciler`] — write fields into a [`LoroMap`]
//! - [`ListReconciler`] — insert/delete items in a [`LoroList`]
//! - [`MovableListReconciler`] — insert/delete/move items in a [`LoroMovableList`]
//! - [`TextReconciler`] — update text content in a [`LoroText`]

pub(crate) mod impls;
pub mod list;
pub mod map;
pub mod movable_list;

use loro::{
    Container, ContainerTrait, LoroList, LoroMap, LoroMovableList, LoroText, LoroValue,
    ValueOrContainer,
};

use std::hash::Hash;

use crate::error::ReconcileError;

// ── Key types ───────────────────────────────────────────────────────────

/// Sentinel type for [`Reconcile::Key`] when a type has no identity key.
///
/// This is the default — types without `#[key]` use positional diffing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoKey;

/// Result of extracting an identity key from a Loro value or Rust value.
///
/// Used by movable list reconciliation to match old and new items by key.
///
/// - `NoKey` — type doesn't use keys (positional diffing)
/// - `KeyNotFound` — type uses keys but this value's key couldn't be extracted
/// - `Found(K)` — successfully extracted key
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadKey<K> {
    /// Type doesn't use identity keys.
    NoKey,
    /// Type uses keys but extraction failed for this value.
    KeyNotFound,
    /// Successfully extracted key.
    Found(K),
}

impl<K> LoadKey<K> {
    pub fn into_found(self) -> Option<K> {
        match self {
            LoadKey::Found(k) => Some(k),
            _ => None,
        }
    }
}

// ── Reconcile trait ─────────────────────────────────────────────────────

/// Write a Rust value into a Loro location via a [`Reconciler`].
///
/// # Implementing
///
/// Call the appropriate method on the reconciler to write your value:
///
/// - Scalars: `r.boolean(v)`, `r.i64(v)`, `r.f64(v)`, `r.str(v)`, `r.bytes(v)`, `r.null()`
/// - Map (structs): `r.map()` → [`MapReconciler`] → `entry(key, &value)`
/// - List: `r.list()` → [`ListReconciler`] → `insert`/`delete`
/// - Text: `r.text()` → [`TextReconciler`] → `update(text)`
///
/// # Identity Keys
///
/// Types with `#[key]` fields participate in identity-based list diffing on
/// [`LoroMovableList`]. Override [`key()`](Reconcile::key) and
/// [`hydrate_key()`](Reconcile::hydrate_key) to enable this.
pub trait Reconcile {
    /// The identity key type for list diffing. [`NoKey`] means positional diffing.
    type Key: PartialEq + Eq + Hash;

    /// Write this value into the given reconciler location.
    fn reconcile<R: Reconciler>(&self, reconciler: R) -> Result<(), ReconcileError>;

    /// Extract the identity key from this Rust value.
    fn key(&self) -> LoadKey<Self::Key> {
        LoadKey::NoKey
    }

    /// Extract the identity key from a Loro source without hydrating the full value.
    /// Used by movable list reconciliation to match items by key before diffing.
    fn hydrate_key(_source: &ValueOrContainer) -> Result<LoadKey<Self::Key>, ReconcileError> {
        Ok(LoadKey::NoKey)
    }
}

// ── Reconciler trait ────────────────────────────────────────────────────

/// Location abstraction for writing into Loro containers.
///
/// Implementations include [`PropReconciler`] (writes to a map key or list index)
/// and [`RootReconciler`] (writes to a document root map).
pub trait Reconciler {
    /// Write a null value.
    fn null(self) -> Result<(), ReconcileError>;
    /// Write a boolean.
    fn boolean(self, v: bool) -> Result<(), ReconcileError>;
    /// Write a 64-bit integer.
    fn i64(self, v: i64) -> Result<(), ReconcileError>;
    /// Write a 64-bit float.
    fn f64(self, v: f64) -> Result<(), ReconcileError>;
    /// Write a string.
    fn str(self, v: &str) -> Result<(), ReconcileError>;
    /// Write binary data.
    fn bytes(self, v: &[u8]) -> Result<(), ReconcileError>;

    /// Get or create a [`LoroMap`] at this location.
    fn map(self) -> Result<MapReconciler, ReconcileError>;
    /// Get or create a [`LoroList`] at this location.
    fn list(self) -> Result<ListReconciler, ReconcileError>;
    /// Get or create a [`LoroMovableList`] at this location.
    fn movable_list(self) -> Result<MovableListReconciler, ReconcileError>;
    /// Get or create a [`LoroText`] at this location.
    fn text(self) -> Result<TextReconciler, ReconcileError>;
}

// ── PropReconciler ──────────────────────────────────────────────────────

/// Concrete [`Reconciler`] that writes to a specific location (map key, list index, etc).
///
/// Created via factory methods like [`PropReconciler::map_put()`] or
/// [`PropReconciler::list_insert()`]. Includes no-op detection for scalars —
/// if the existing value is identical, no CRDT operation is emitted.
pub struct PropReconciler {
    action: PropAction,
}

enum PropAction {
    MapPut { map: LoroMap, key: String },
    ListInsert { list: LoroList, index: usize },
    MovableListInsert { list: LoroMovableList, index: usize },
    MovableListSet { list: LoroMovableList, index: usize },
}

impl PropReconciler {
    pub fn map_put(map: LoroMap, key: String) -> Self {
        Self {
            action: PropAction::MapPut { map, key },
        }
    }

    pub fn list_insert(list: LoroList, index: usize) -> Self {
        Self {
            action: PropAction::ListInsert { list, index },
        }
    }

    pub fn movable_list_insert(list: LoroMovableList, index: usize) -> Self {
        Self {
            action: PropAction::MovableListInsert { list, index },
        }
    }

    pub fn movable_list_set(list: LoroMovableList, index: usize) -> Self {
        Self {
            action: PropAction::MovableListSet { list, index },
        }
    }

    fn put_value(self, value: impl Into<LoroValue>) -> Result<(), ReconcileError> {
        match self.action {
            PropAction::MapPut { map, key } => {
                let new_value = value.into();
                // No-op detection: skip write if existing value is identical.
                if let Some(ValueOrContainer::Value(existing)) = map.get(&key) {
                    if existing == new_value {
                        return Ok(());
                    }
                }
                map.insert(&key, new_value)?;
            }
            PropAction::ListInsert { list, index } => {
                list.insert(index, value)?;
            }
            PropAction::MovableListInsert { list, index } => {
                list.insert(index, value)?;
            }
            PropAction::MovableListSet { list, index } => {
                let new_value = value.into();
                // No-op detection: skip write if existing value is identical.
                if let Some(ValueOrContainer::Value(existing)) = list.get(index) {
                    if existing == new_value {
                        return Ok(());
                    }
                }
                list.set(index, new_value)?;
            }
        }
        Ok(())
    }

    fn get_or_create_container<C: ContainerTrait>(self, detached: C) -> Result<C, ReconcileError> {
        let container = match self.action {
            PropAction::MapPut { map, key } => map.get_or_create_container(&key, detached)?,
            PropAction::ListInsert { list, index } => list.insert_container(index, detached)?,
            PropAction::MovableListInsert { list, index } => {
                list.insert_container(index, detached)?
            }
            PropAction::MovableListSet { list, index } => list.set_container(index, detached)?,
        };
        Ok(container)
    }

    /// For MovableListSet, try to get the existing container at this index
    /// to reconcile into it (preserving CRDT identity for sub-fields).
    fn try_get_existing_map(&self) -> Option<LoroMap> {
        match &self.action {
            PropAction::MovableListSet { list, index } => {
                if let Some(ValueOrContainer::Container(Container::Map(m))) = list.get(*index) {
                    Some(m)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Reconciler for PropReconciler {
    fn null(self) -> Result<(), ReconcileError> {
        self.put_value(LoroValue::Null)
    }

    fn boolean(self, v: bool) -> Result<(), ReconcileError> {
        self.put_value(v)
    }

    fn i64(self, v: i64) -> Result<(), ReconcileError> {
        self.put_value(v)
    }

    fn f64(self, v: f64) -> Result<(), ReconcileError> {
        self.put_value(v)
    }

    fn str(self, v: &str) -> Result<(), ReconcileError> {
        self.put_value(v)
    }

    fn bytes(self, v: &[u8]) -> Result<(), ReconcileError> {
        self.put_value(LoroValue::Binary(v.to_vec().into()))
    }

    fn map(self) -> Result<MapReconciler, ReconcileError> {
        // For MovableListSet, reuse the existing map container to preserve
        // CRDT identity — enables field-level concurrent merges within list items.
        if let Some(existing) = self.try_get_existing_map() {
            return Ok(MapReconciler { map: existing });
        }
        let m = self.get_or_create_container(LoroMap::new())?;
        Ok(MapReconciler { map: m })
    }

    fn list(self) -> Result<ListReconciler, ReconcileError> {
        let l = self.get_or_create_container(LoroList::new())?;
        Ok(ListReconciler { list: l })
    }

    fn movable_list(self) -> Result<MovableListReconciler, ReconcileError> {
        let l = self.get_or_create_container(LoroMovableList::new())?;
        Ok(MovableListReconciler { list: l })
    }

    fn text(self) -> Result<TextReconciler, ReconcileError> {
        let t = self.get_or_create_container(LoroText::new())?;
        Ok(TextReconciler { text: t })
    }
}

// ── RootReconciler ──────────────────────────────────────────────────────

/// Reconciler for writing directly into a root LoroMap (no insert needed).
pub struct RootReconciler {
    map: LoroMap,
}

impl RootReconciler {
    pub fn new(map: LoroMap) -> Self {
        Self { map }
    }
}

impl Reconciler for RootReconciler {
    fn null(self) -> Result<(), ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "null",
        })
    }

    fn boolean(self, _v: bool) -> Result<(), ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "bool",
        })
    }

    fn i64(self, _v: i64) -> Result<(), ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "i64",
        })
    }

    fn f64(self, _v: f64) -> Result<(), ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "f64",
        })
    }

    fn str(self, _v: &str) -> Result<(), ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "string",
        })
    }

    fn bytes(self, _v: &[u8]) -> Result<(), ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "binary",
        })
    }

    fn map(self) -> Result<MapReconciler, ReconcileError> {
        Ok(MapReconciler { map: self.map })
    }

    fn list(self) -> Result<ListReconciler, ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "list",
        })
    }

    fn movable_list(self) -> Result<MovableListReconciler, ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "movable_list",
        })
    }

    fn text(self) -> Result<TextReconciler, ReconcileError> {
        Err(ReconcileError::TypeMismatch {
            expected: "map",
            found: "text",
        })
    }
}

// ── Sub-Reconcilers ─────────────────────────────────────────────────────

/// Reconciler for writing fields into a [`LoroMap`].
///
/// Obtained via [`Reconciler::map()`]. Use [`entry()`](MapReconciler::entry) to
/// write fields and [`delete()`](MapReconciler::delete) to remove stale keys.
pub struct MapReconciler {
    pub map: LoroMap,
}

/// Reconciler for a [`LoroList`].
///
/// Obtained via [`Reconciler::list()`]. Supports `insert()`, `delete()`,
/// and positional access. Used internally by `Vec<T>` LCS diffing.
pub struct ListReconciler {
    pub(crate) list: LoroList,
}

/// Reconciler for a [`LoroMovableList`].
///
/// Obtained via [`Reconciler::movable_list()`]. Supports `insert()`, `delete()`,
/// `set()` (in-place update), and `mov()` (reorder). Used by `#[loro(movable)]` vecs.
pub struct MovableListReconciler {
    pub(crate) list: LoroMovableList,
}

/// Reconciler for a [`LoroText`].
///
/// Obtained via [`Reconciler::text()`]. Uses Loro's built-in `update()`
/// which performs LCS diffing to produce minimal insert/delete operations.
pub struct TextReconciler {
    pub(crate) text: LoroText,
}

impl TextReconciler {
    /// Update the text content using Loro's built-in LCS diff.
    pub fn update(&mut self, new_text: &str) -> Result<(), ReconcileError> {
        self.text
            .update(new_text, loro::UpdateOptions::default())
            .map_err(|_e| ReconcileError::TypeMismatch {
                expected: "text update",
                found: "timeout",
            })?;
        Ok(())
    }

    /// Get current text content.
    pub fn get(&self) -> String {
        self.text.to_string()
    }
}

/// Reconcile a `String` into a [`LoroText`] container at a map key.
///
/// Used by `#[loro(text)]` codegen.
pub fn reconcile_text_prop(
    value: &str,
    map: &LoroMap,
    key: &str,
) -> Result<(), ReconcileError> {
    let reconciler = PropReconciler::map_put(map.clone(), key.to_string());
    let mut t = reconciler.text()?;
    t.update(value)?;
    Ok(())
}
