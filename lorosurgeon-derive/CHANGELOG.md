# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-10

### Features

- `#[derive(Hydrate)]` for structs and enums
- `#[derive(Reconcile)]` for structs and enums
- Field attributes: `#[key]`, `#[loro(rename)]`, `#[loro(json)]`, `#[loro(movable)]`, `#[loro(missing)]`, `#[loro(flatten)]`, `#[loro(with/hydrate/reconcile)]`
- Newtype-over-`Vec<T>` special-cased codegen
- Enum key type generation for movable list diffing
