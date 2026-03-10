# lorosurgeon

Derive macros for [Loro](https://loro.dev) CRDT containers. Inspired by [autosurgeon](https://github.com/automerge/autosurgeon) for Automerge.

Two derive macros — `Hydrate` and `Reconcile` — generate idiomatic serialization and deserialization between Rust types and Loro containers, with full CRDT granularity.

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
    tags: Vec<String>,
}

#[derive(Debug, PartialEq, Hydrate, Reconcile)]
struct Position {
    x: f64,
    y: f64,
}

let doc = LoroDoc::new();

let config = Config {
    name: "hello".into(),
    version: 1,
    position: Position { x: 10.0, y: 20.0 },
    tags: vec!["a".into(), "b".into()],
};

// Write to Loro document
config.to_doc(&doc).unwrap();
doc.commit();

// Read back
let loaded = Config::from_doc(&doc).unwrap();
assert_eq!(loaded, config);
```

Each struct field maps to a key in a `LoroMap`. Nested structs become nested maps. Changes are written at field granularity — only modified fields produce CRDT operations.

## Type Mappings

| Rust Type | Loro Storage | Notes |
|-----------|-------------|-------|
| `bool` | `Bool` | |
| `i8`–`i64`, `u8`–`u64` | `I64` | Overflow checked on hydration |
| `f32`, `f64` | `Double` | |
| `String`, `&str` | `String` | |
| `Vec<u8>` | `Binary` | Special-cased |
| `Vec<T>` | `LoroList` | Clear + rewrite |
| `Vec<T>` with `#[loro(movable)]` | `LoroMovableList` | Keyed LCS diffing, move-aware |
| `Option<T>` | `Null` / T | |
| `HashMap<String, V>` | `LoroMap` | Stale keys auto-deleted |
| `Box<T>`, `Cow<'a, T>` | (transparent) | Delegates to inner type |
| Named struct | `LoroMap` | Fields → keys |
| Newtype struct | (transparent) | Delegates to inner |
| Tuple struct | `LoroList` | Positional |
| Unit enum | `String` | Variant name |
| Data enum | `LoroMap` | `{ "Variant": data }` |

### Special Types

| Type | Loro Container | Behavior |
|------|---------------|----------|
| `Text` | `LoroText` | LCS-diffed text (Loro's built-in `update()`) |
| `ByteArray<N>` | `Binary` | Fixed-size binary with compile-time length check |
| `MaybeMissing<T>` | Null / T | Distinguishes "key absent" from "key present" (unlike `Option`) |

## Field Attributes

```rust
#[derive(Hydrate, Reconcile)]
struct Example {
    #[key]                          // Identity key for list diffing
    id: String,

    #[loro(rename = "pos")]         // Different key name in Loro
    position: Position,

    #[loro(json)]                   // serde_json round-trip (coarse fallback)
    theme: Theme,

    #[loro(movable)]                // LoroMovableList with keyed LCS
    items: Vec<Item>,

    #[loro(missing)]                // Default::default() when key absent
    count: i32,

    #[loro(missing = "default_name")] // Custom function when absent
    name: String,

    #[loro(flatten)]                // Inline nested struct fields into parent map
    canvas: CanvasPosition,
}
```

## Keyed List Diffing

For `LoroMovableList`, items with `#[key]` fields are matched by identity across reconciliation. Matched items are updated in-place with `set()` (preserving CRDT element identity), new items are `insert()`ed, removed items are `delete()`d, and reordered items use `mov()`.

```rust
#[derive(Hydrate, Reconcile)]
struct Item {
    #[key]
    id: String,
    value: i64,
}

#[derive(Hydrate, Reconcile)]
struct Doc {
    #[loro(movable)]
    items: Vec<Item>,
}
```

This enables concurrent field-level edits within list items to merge correctly — two peers can edit different fields of the same item and both changes are preserved.

### Enum Keys

Enums with `#[key]` fields in any variant automatically generate a companion key type for list diffing:

```rust
#[derive(Hydrate, Reconcile)]
enum Shape {
    Circle {
        #[key]
        id: String,
        radius: f64,
    },
    Rectangle {
        #[key]
        id: String,
        width: f64,
        height: f64,
    },
    Point,  // No key — matches by variant name
}
```

## DocSync — Document-Level Round-Trip

`#[loro(root = "key")]` generates a `DocSync` impl for writing to/from a `LoroDoc`:

```rust
#[derive(Hydrate, Reconcile)]
#[loro(root = "whiteboard")]
struct WhiteboardState {
    name: String,
    elements: Vec<Element>,
}

// Generated:
// impl DocSync for WhiteboardState {
//     const ROOT_KEY: &'static str = "whiteboard";
// }

let doc = LoroDoc::new();
state.to_doc(&doc)?;
let loaded = WhiteboardState::from_doc(&doc)?;
```

## Stale Heads Detection

`VersionGuard` captures a document's version before hydration and detects concurrent modifications:

```rust
let guard = VersionGuard::capture(&doc);
let mut state = MyState::from_doc(&doc)?;
state.name = "updated".into();
guard.check(&doc)?;  // Fails if doc was modified concurrently
state.to_doc(&doc)?;
doc.commit();
```

## No-Op Detection

Reconciling identical values produces zero CRDT operations — the `PropReconciler` reads the existing value before writing and skips if unchanged. This prevents unnecessary history bloat.

## Features

- `uuid` — `Hydrate`/`Reconcile` impls for `uuid::Uuid` (stored as 16-byte binary)

## Crate Structure

```
lorosurgeon/          # Core traits, type impls, special types
lorosurgeon-derive/   # Proc macros (#[derive(Hydrate, Reconcile)])
```

## License

MIT OR Apache-2.0
