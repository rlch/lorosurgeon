//! lorosurgeon — Autosurgeon-style derive macros for Loro CRDT containers.
//!
//! Two derive macros (`Hydrate`, `Reconcile`) generate idiomatic Loro
//! serialization/deserialization for Rust types.

pub mod error;
pub mod hydrate;
pub mod reconcile;

pub mod byte_array;
pub mod doc_sync;
pub mod maybe_missing;
pub mod text;

// Re-export derive macros
pub use lorosurgeon_derive::{Hydrate, Reconcile};

// Re-export core traits
pub use crate::doc_sync::{DocSync, VersionGuard};
pub use crate::hydrate::Hydrate;
pub use crate::reconcile::{
    ListReconciler, LoadKey, MapReconciler, MovableListReconciler, NoKey, PropReconciler,
    Reconcile, Reconciler, RootReconciler, TextReconciler,
};

// Re-export error types
pub use crate::error::{HydrateError, HydrateResultExt, ReconcileError};

// Re-export special types
pub use crate::byte_array::ByteArray;
pub use crate::maybe_missing::MaybeMissing;
pub use crate::text::Text;

// Re-export top-level helper functions
pub use crate::hydrate::{
    hydrate, hydrate_list_item, hydrate_map, hydrate_prop, hydrate_prop_json,
    hydrate_prop_json_or_default, hydrate_prop_or, hydrate_prop_or_default, hydrate_prop_or_else,
};

// Re-export vec helpers for derive macro codegen
pub use crate::hydrate::impls::{
    hydrate_keyed_map, hydrate_vec_from_list, hydrate_vec_from_movable_list,
};

// Re-export list reconcile helpers for derive macro codegen
pub use crate::reconcile::list::{reconcile_vec, reconcile_vec_movable, reconcile_vec_simple};
pub use crate::reconcile::map::reconcile_keyed_map;
