# lorosurgeon

[![Crates.io](https://img.shields.io/crates/v/lorosurgeon.svg)](https://crates.io/crates/lorosurgeon)
[![docs.rs](https://docs.rs/lorosurgeon/badge.svg)](https://docs.rs/lorosurgeon)
[![CI](https://github.com/rlch/lorosurgeon/actions/workflows/ci.yml/badge.svg)](https://github.com/rlch/lorosurgeon/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Derive macros for bidirectional serialization between Rust types and [Loro](https://loro.dev) CRDT containers — the Loro equivalent of [autosurgeon](https://github.com/automerge/autosurgeon) for Automerge.

`#[derive(Hydrate, Reconcile)]` generates field-level mapping between Rust types and Loro containers. Only modified fields produce CRDT operations.

## Quick Start

```rust
use loro::LoroDoc;
use lorosurgeon::{Hydrate, Reconcile, DocSync};

#[derive(Debug, PartialEq, Hydrate, Reconcile)]
#[loro(root = "config")]
struct Config {
    name: String,
    version: i64,
    position: Position,
}

#[derive(Debug, PartialEq, Hydrate, Reconcile)]
struct Position { x: f64, y: f64 }

let doc = LoroDoc::new();
let config = Config {
    name: "hello".into(),
    version: 1,
    position: Position { x: 10.0, y: 20.0 },
};

config.to_doc(&doc).unwrap();  // Rust → Loro
doc.commit();

let loaded = Config::from_doc(&doc).unwrap();  // Loro → Rust
assert_eq!(loaded, config);
```

## Documentation

**[Full API documentation on docs.rs →](https://docs.rs/lorosurgeon)**

The crate docs include type mapping tables, attribute reference, examples for concurrent editing, custom serialization, flatten, keyed list diffing, and more.

## Features

- **Structs** → `LoroMap` (fields become keys)
- **Enums** → `LoroMap` with variant discriminator, unit variants as strings
- **`Vec<T>`** → `LoroList` with Myers LCS diffing
- **`#[loro(movable)]`** → `LoroMovableList` with identity-preserving `mov()`/`set()`
- **`HashMap<String, V>`** → `LoroMap` with stale-key cleanup
- **`Text`** → `LoroText` with character-level diffing
- **No-op detection** — identical values produce zero CRDT operations
- **Concurrent merge** — field-level granularity means independent edits compose

## Attributes

```rust
// Container-level
#[loro(root = "key")]          // DocSync: to_doc() / from_doc()

// Field-level
#[key]                         // Identity key for movable list diffing
#[loro(rename = "name")]       // Different key in Loro
#[loro(json)]                  // serde_json round-trip
#[loro(movable)]               // LoroMovableList instead of LoroList
#[loro(missing)]               // Default::default() when absent
#[loro(missing = "fn")]        // Custom default function
#[loro(flatten)]               // Inline nested struct fields
#[loro(with = "module")]       // Custom hydrate + reconcile
#[loro(hydrate = "fn")]        // Custom hydrate only
#[loro(reconcile = "fn")]      // Custom reconcile only
```

## License

MIT
