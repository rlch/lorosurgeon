# Changelog

All notable changes to this project will be documented in this file.
# Changelog


### Documentation

- add initial changelogs for release-plz
- comprehensive crate docs, clean public API surface
# Changelog


### Documentation

- add initial changelogs for release-plz
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

- `#[derive(Hydrate)]` for structs and enums
- `#[derive(Reconcile)]` for structs and enums
- Field attributes: `#[key]`, `#[loro(rename)]`, `#[loro(json)]`, `#[loro(movable)]`, `#[loro(default)]`, `#[loro(flatten)]`, `#[loro(with/hydrate/reconcile)]`
- Newtype-over-`Vec<T>` special-cased codegen
- Enum key type generation for movable list diffing
