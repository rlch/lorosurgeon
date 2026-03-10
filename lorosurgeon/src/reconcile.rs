//! Reconcile trait — write Rust types into Loro containers.

pub(crate) mod impls;
pub mod list;
pub(crate) mod map;
pub(crate) mod movable_list;

use loro::{
    Container, ContainerTrait, LoroList, LoroMap, LoroMovableList, LoroText, LoroValue,
    ValueOrContainer,
};

use crate::error::ReconcileError;

// ── Key types ───────────────────────────────────────────────────────────

/// Sentinel for types with no identity key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoKey;

/// Result of extracting a key from a Loro value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadKey<K> {
    NoKey,
    KeyNotFound,
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

/// Write a Rust value into a Loro location via a Reconciler.
pub trait Reconcile {
    /// The identity key type for list diffing. Defaults to NoKey (positional).
    /// Must be owned — keys are stored for comparison during LCS diffing.
    type Key: PartialEq;

    /// Write this value into the given reconciler location.
    fn reconcile<R: Reconciler>(&self, reconciler: R) -> Result<(), ReconcileError>;

    /// Extract the identity key from this value.
    fn key(&self) -> LoadKey<Self::Key> {
        LoadKey::NoKey
    }

    /// Extract the identity key from a Loro source (for pre-diffing).
    /// Only hydrates the key, not the full value — used by LCS list reconciliation.
    fn hydrate_key(
        _source: &ValueOrContainer,
    ) -> Result<LoadKey<Self::Key>, ReconcileError> {
        Ok(LoadKey::NoKey)
    }
}

// ── Reconciler trait ────────────────────────────────────────────────────

/// Location abstraction for writing into Loro containers.
pub trait Reconciler {
    // Scalars
    fn null(self) -> Result<(), ReconcileError>;
    fn boolean(self, v: bool) -> Result<(), ReconcileError>;
    fn i64(self, v: i64) -> Result<(), ReconcileError>;
    fn f64(self, v: f64) -> Result<(), ReconcileError>;
    fn str(self, v: &str) -> Result<(), ReconcileError>;
    fn bytes(self, v: &[u8]) -> Result<(), ReconcileError>;

    // Containers — return typed sub-reconcilers
    fn map(self) -> Result<MapReconciler, ReconcileError>;
    fn list(self) -> Result<ListReconciler, ReconcileError>;
    fn movable_list(self) -> Result<MovableListReconciler, ReconcileError>;
    fn text(self) -> Result<TextReconciler, ReconcileError>;
}

// ── PropReconciler ──────────────────────────────────────────────────────

/// Concrete reconciler that wraps a specific location (map key, list index, etc).
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

    fn get_or_create_container<C: ContainerTrait>(
        self,
        detached: C,
    ) -> Result<C, ReconcileError> {
        let container = match self.action {
            PropAction::MapPut { map, key } => {
                map.get_or_create_container(&key, detached)?
            }
            PropAction::ListInsert { list, index } => {
                list.insert_container(index, detached)?
            }
            PropAction::MovableListInsert { list, index } => {
                list.insert_container(index, detached)?
            }
            PropAction::MovableListSet { list, index } => {
                list.set_container(index, detached)?
            }
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

pub struct MapReconciler {
    pub map: LoroMap,
}

pub struct ListReconciler {
    pub(crate) list: LoroList,
}

pub struct MovableListReconciler {
    pub(crate) list: LoroMovableList,
}

pub struct TextReconciler {
    pub(crate) text: LoroText,
}
