# Changelog

All notable changes to this project will be documented in this file.
# Changelog


### Documentation

- add initial changelogs for release-plz
- add crates.io, docs.rs, CI, and license badges to README
- comprehensive crate docs, clean public API surface
# Changelog


### Documentation

- add initial changelogs for release-plz
- add crates.io, docs.rs, CI, and license badges to README
- comprehensive crate docs, clean public API surface

### Miscellaneous

- release v0.1.1 ([#1](https://github.com/rlch/lorosurgeon/pull/1))
# Changelog


### Miscellaneous

- release v0.1.1 ([#2](https://github.com/rlch/lorosurgeon/pull/2))

### Refactor

- rename #[loro(missing)] attribute to #[loro(default)]
- replace Text newtype with #[loro(text)] field attribute

## [0.1.0] - 2026-03-10

### Features

- Derive macros for `Hydrate` and `Reconcile` traits
- Field-level CRDT serialization between Rust types and Loro containers
- LCS diffing for `Vec<T>` via Myers algorithm
- Keyed movable list reconciliation with `mov()`/`set()`
- `#[loro(root)]` for `DocSync` (to_doc/from_doc)
- `#[loro(json)]`, `#[loro(rename)]`, `#[loro(default)]`, `#[loro(flatten)]`
- `#[loro(with/hydrate/reconcile)]` custom function attributes
- `Text` wrapper for character-level LoroText diffing
- `MaybeMissing<T>` for absent-vs-present tracking
- `VersionGuard` for stale heads detection
- `ByteArray<N>` for fixed-size binary
- Support for `Box<T>`, `Cow<T>`, `Option<T>`, `HashMap`, `BTreeMap`
