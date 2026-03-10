//! Derive macros for bidirectional serialization between Rust types and
//! [Loro](https://loro.dev) CRDT containers — the Loro equivalent of
//! [autosurgeon](https://github.com/automerge/autosurgeon) for Automerge.
//!
//! `#[derive(Hydrate, Reconcile)]` generates field-level mapping between Rust
//! structs/enums and Loro containers. Only modified fields produce CRDT
//! operations, enabling efficient collaborative editing.
//!
//! # Quick Start
//!
//! ```rust
//! use loro::LoroDoc;
//! use lorosurgeon::{Hydrate, Reconcile, DocSync};
//!
//! #[derive(Debug, PartialEq, Hydrate, Reconcile)]
//! #[loro(root = "config")]
//! struct Config {
//!     name: String,
//!     version: i64,
//!     position: Position,
//! }
//!
//! #[derive(Debug, PartialEq, Hydrate, Reconcile)]
//! struct Position { x: f64, y: f64 }
//!
//! let doc = LoroDoc::new();
//! let config = Config {
//!     name: "hello".into(),
//!     version: 1,
//!     position: Position { x: 10.0, y: 20.0 },
//! };
//!
//! config.to_doc(&doc).unwrap();
//! doc.commit();
//!
//! let loaded = Config::from_doc(&doc).unwrap();
//! assert_eq!(loaded, config);
//! ```
//!
//! # Core Traits
//!
//! | Trait | Direction | Purpose |
//! |-------|-----------|---------|
//! | [`Hydrate`] | Loro → Rust | Read Rust types from Loro containers |
//! | [`Reconcile`] | Rust → Loro | Write Rust types into Loro containers |
//! | [`DocSync`] | Both | Root-level `to_doc()`/`from_doc()` via `#[loro(root)]` |
//!
//! Most users only need `#[derive(Hydrate, Reconcile)]` — the traits are
//! implemented automatically. Manual impls are covered in the trait docs.
//!
//! # Type Mappings
//!
//! ## Structs and Enums
//!
//! | Rust type | Loro storage |
//! |-----------|-------------|
//! | Named struct | [`LoroMap`](loro::LoroMap) — fields become keys |
//! | Newtype (`Foo(T)`) | Transparent — delegates to inner type |
//! | Newtype (`Foo(Vec<T>)`) | [`LoroList`](loro::LoroList) — special-cased |
//! | Tuple struct (`Foo(A, B)`) | [`LoroList`](loro::LoroList) — positional |
//! | Unit enum variant | `String` — variant name |
//! | Data enum variant | [`LoroMap`](loro::LoroMap) — `{ "Variant": data }` |
//!
//! ## Scalars
//!
//! | Rust | Loro |
//! |------|------|
//! | `bool` | `Bool` |
//! | `i8`–`i64`, `u8`–`u64`, `usize` | `I64` (overflow checked on hydration) |
//! | `f32`, `f64` | `Double` |
//! | `String` | `String` |
//! | `Vec<u8>` | `Binary` |
//! | `Option<T>` | `Null` or `T` |
//! | `Box<T>`, `Cow<T>` | Transparent |
//! | `serde_json::Value` | deep conversion via `LoroValue` |
//!
//! ## Collections
//!
//! | Rust | Loro | Strategy |
//! |------|------|----------|
//! | `Vec<T>` | [`LoroList`](loro::LoroList) | Myers LCS diffing (requires `T: Hydrate + PartialEq`) |
//! | `Vec<T>` + `#[loro(movable)]` | [`LoroMovableList`](loro::LoroMovableList) | Key-based diffing with `mov()`/`set()` |
//! | `HashMap<String, V>` | [`LoroMap`](loro::LoroMap) | Put entries, delete stale keys |
//!
//! # Special Types
//!
//! - [`Text`] — plain text backed by [`LoroText`](loro::LoroText) with character-level LCS
//! - [`ByteArray<N>`](ByteArray) — fixed-size byte array, length-checked on hydration
//! - [`MaybeMissing<T>`](MaybeMissing) — distinguishes "key absent" from "key present" (unlike `Option`)
//! - [`VersionGuard`] — captures document version to detect stale-heads before write-back
//!
//! # Attributes
//!
//! ## Container-level
//!
//! - `#[loro(root = "key")]` — generate [`DocSync`] impl for root-level `to_doc()`/`from_doc()`
//!
//! ## Field-level
//!
//! - `#[key]` — identity key for [`LoroMovableList`](loro::LoroMovableList) diffing
//! - `#[loro(rename = "name")]` — use a different key name in Loro
//! - `#[loro(json)]` — store as JSON string via serde (coarse-grained fallback)
//! - `#[loro(movable)]` — use [`LoroMovableList`](loro::LoroMovableList) instead of [`LoroList`](loro::LoroList)
//! - `#[loro(missing)]` — use `Default::default()` when key is absent
//! - `#[loro(missing = "fn_name")]` — call a custom function when key is absent
//! - `#[loro(flatten)]` — inline nested struct fields into parent map
//! - `#[loro(with = "module")]` — custom hydrate + reconcile via `module::hydrate` / `module::reconcile`
//! - `#[loro(hydrate = "fn")]` — custom hydrate function only
//! - `#[loro(reconcile = "fn")]` — custom reconcile function only
//!
//! # Keyed List Diffing
//!
//! `#[loro(movable)]` + `#[key]` enables identity-preserving list reconciliation:
//!
//! ```rust
//! use lorosurgeon::{Hydrate, Reconcile};
//!
//! #[derive(Hydrate, Reconcile)]
//! struct Item {
//!     #[key]
//!     id: String,
//!     value: i64,
//! }
//!
//! #[derive(Hydrate, Reconcile)]
//! struct Doc {
//!     #[loro(movable)]
//!     items: Vec<Item>,
//! }
//! ```
//!
//! Matched items use `set()` in-place (preserving CRDT element identity),
//! so two peers editing different fields of the same item merge correctly.
//!
//! # Concurrent Editing
//!
//! Because each struct field maps to its own Loro container key, two peers
//! can edit different fields of the same struct and merge without conflict:
//!
//! ```rust
//! use loro::LoroDoc;
//! use lorosurgeon::{Hydrate, Reconcile, DocSync};
//!
//! #[derive(Debug, PartialEq, Hydrate, Reconcile)]
//! #[loro(root = "doc")]
//! struct Document {
//!     title: String,
//!     version: i64,
//! }
//!
//! // Peer A
//! let doc_a = LoroDoc::new();
//! let state = Document { title: "Hello".into(), version: 1 };
//! state.to_doc(&doc_a).unwrap();
//! doc_a.commit();
//!
//! // Peer B starts from A's state
//! let doc_b = LoroDoc::new();
//! doc_b.import(&doc_a.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
//!
//! // A changes title, B changes version — concurrently
//! let mut a = Document::from_doc(&doc_a).unwrap();
//! a.title = "World".into();
//! a.to_doc(&doc_a).unwrap();
//! doc_a.commit();
//!
//! let mut b = Document::from_doc(&doc_b).unwrap();
//! b.version = 2;
//! b.to_doc(&doc_b).unwrap();
//! doc_b.commit();
//!
//! // Merge — both changes preserved
//! doc_a.import(&doc_b.export(loro::ExportMode::updates(&doc_a.oplog_vv())).unwrap()).unwrap();
//! let merged = Document::from_doc(&doc_a).unwrap();
//! assert_eq!(merged, Document { title: "World".into(), version: 2 });
//! ```
//!
//! # Custom Serialization
//!
//! For fields that need custom logic, use `#[loro(with = "module")]`:
//!
//! ```rust
//! use lorosurgeon::{Hydrate, Reconcile};
//!
//! mod uppercase {
//!     use loro::LoroMap;
//!     use lorosurgeon::{HydrateError, ReconcileError, MapReconciler};
//!
//!     pub fn hydrate(map: &LoroMap, key: &str) -> Result<String, HydrateError> {
//!         lorosurgeon::hydrate_prop::<String>(map, key)
//!             .map(|s| s.to_uppercase())
//!     }
//!
//!     pub fn reconcile(
//!         value: &String,
//!         m: &mut MapReconciler,
//!         key: &str,
//!     ) -> Result<(), ReconcileError> {
//!         m.entry(key, &value.to_lowercase())
//!     }
//! }
//!
//! #[derive(Hydrate, Reconcile)]
//! struct Config {
//!     #[loro(with = "uppercase")]
//!     name: String,
//! }
//! ```
//!
//! # Flatten
//!
//! `#[loro(flatten)]` inlines a nested struct's fields directly into the parent map:
//!
//! ```rust
//! use lorosurgeon::{Hydrate, Reconcile};
//!
//! #[derive(Hydrate, Reconcile)]
//! struct Position { x: f64, y: f64 }
//!
//! #[derive(Hydrate, Reconcile)]
//! struct Element {
//!     id: String,
//!     #[loro(flatten)]
//!     pos: Position,  // x, y written directly to Element's LoroMap
//! }
//! ```
//!
//! # Stale Heads Detection
//!
//! [`VersionGuard`] prevents write-back after concurrent modifications:
//!
//! ```rust
//! use loro::LoroDoc;
//! use lorosurgeon::{Hydrate, Reconcile, DocSync, VersionGuard};
//!
//! #[derive(Debug, PartialEq, Hydrate, Reconcile)]
//! #[loro(root = "data")]
//! struct Data { value: i64 }
//!
//! let doc = LoroDoc::new();
//! Data { value: 1 }.to_doc(&doc).unwrap();
//! doc.commit();
//!
//! let guard = VersionGuard::capture(&doc);
//! let mut state = Data::from_doc(&doc).unwrap();
//! state.value = 42;
//!
//! // If another thread modified doc here, check() would fail:
//! guard.check(&doc).unwrap();
//! state.to_doc(&doc).unwrap();
//! doc.commit();
//! ```
//!
//! # Optimizations
//!
//! - **No-op detection** — writing identical scalar values produces zero CRDT operations
//! - **LCS diffing** — `Vec<T>` uses Myers diff via [`similar`](https://docs.rs/similar)
//!   for minimal insert/delete ops
//! - **Stale heads** — [`VersionGuard`] detects concurrent modifications before write-back
//!
//! # Feature Flags
//!
//! | Feature | Effect |
//! |---------|--------|
//! | `uuid` | [`Hydrate`]/[`Reconcile`] impls for [`uuid::Uuid`](https://docs.rs/uuid) (stored as 16-byte binary) |

// ── Public modules ────────────────────────────────────────────────────

pub mod error;
pub mod hydrate;
pub mod reconcile;

// Modules with re-exported types — keep private to avoid duplicate paths in docs.
mod byte_array;
mod doc_sync;
mod maybe_missing;
mod text;

// ── Derive macros ─────────────────────────────────────────────────────

pub use lorosurgeon_derive::{Hydrate, Reconcile};

// ── Core traits ───────────────────────────────────────────────────────

pub use crate::doc_sync::{DocSync, VersionGuard};
pub use crate::hydrate::Hydrate;
pub use crate::reconcile::{LoadKey, MapReconciler, NoKey, Reconcile, Reconciler};

// ── Error types ───────────────────────────────────────────────────────

pub use crate::error::{HydrateError, ReconcileError};

// ── Special types ─────────────────────────────────────────────────────

pub use crate::byte_array::ByteArray;
pub use crate::maybe_missing::MaybeMissing;
pub use crate::text::Text;

// ── Helpers for manual use ────────────────────────────────────────────

pub use crate::hydrate::{hydrate, hydrate_map, hydrate_prop};

// ── Derive macro codegen support ──────────────────────────────────────
//
// These are used by the generated code from `#[derive(Hydrate, Reconcile)]`.
// They are not part of the public API and may change without notice.

#[doc(hidden)]
pub use crate::error::HydrateResultExt;
#[doc(hidden)]
pub use crate::hydrate::impls::{
    hydrate_keyed_map, hydrate_vec_from_list, hydrate_vec_from_movable_list,
};
#[doc(hidden)]
pub use crate::hydrate::{
    hydrate_list_item, hydrate_prop_json, hydrate_prop_json_or_default, hydrate_prop_or,
    hydrate_prop_or_default, hydrate_prop_or_else,
};
#[doc(hidden)]
pub use crate::reconcile::list::{reconcile_vec, reconcile_vec_movable, reconcile_vec_simple};
#[doc(hidden)]
pub use crate::reconcile::map::reconcile_keyed_map;
#[doc(hidden)]
pub use crate::reconcile::{
    ListReconciler, MovableListReconciler, PropReconciler, RootReconciler, TextReconciler,
};
