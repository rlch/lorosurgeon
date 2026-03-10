# lorosurgeon

Derive macros for [Loro](https://loro.dev) CRDT containers — the Loro equivalent of [autosurgeon](https://github.com/automerge/autosurgeon) for Automerge.

`#[derive(Hydrate, Reconcile)]` generates field-level serialization between Rust types and Loro containers. Only modified fields produce CRDT operations.

```rust
use loro::LoroDoc;
use lorosurgeon::{Hydrate, Reconcile, DocSync};

#[derive(Hydrate, Reconcile)]
#[loro(root = "config")]
struct Config {
    name: String,
    version: i64,
    position: Position,
}

#[derive(Hydrate, Reconcile)]
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

## Type Mappings

### Structs and Enums

| Rust type | Loro storage |
|---|---|
| Named struct | `LoroMap` — fields become keys |
| Newtype struct (`Foo(T)`) | Transparent — delegates to inner type |
| Newtype struct (`Foo(Vec<T>)`) | `LoroList` — special-cased for non-`u8` element types |
| Tuple struct (`Foo(A, B)`) | `LoroList` — positional |
| Unit enum | `String` — variant name |
| Data enum | `LoroMap` — `{ "Variant": data }` |

### Scalars

| Rust | Loro | Notes |
|---|---|---|
| `bool` | `Bool` | |
| `i8`–`i64`, `u8`–`u64`, `usize` | `I64` | Overflow checked on hydration |
| `f32`, `f64` | `Double` | |
| `String`, `&str` | `String` | |
| `Vec<u8>` | `Binary` | Special-cased for efficiency |
| `Option<T>` | `Null` / `T` | |
| `Box<T>`, `Cow<'a, T>`, `&T` | Transparent | Delegates to inner |
| `serde_json::Value` | `String` | JSON-serialized |

### Collections

| Rust | Loro | Strategy |
|---|---|---|
| `Vec<T>` | `LoroList` | LCS diffing (requires `T: Hydrate + PartialEq`), falls back to clear+rewrite |
| `Vec<T>` + `#[loro(movable)]` | `LoroMovableList` | Keyed LCS diffing with `mov()`/`set()` |
| `HashMap<String, V>` | `LoroMap` | Put entries, delete stale keys |
| `BTreeMap<String, V>` | `LoroMap` | Same |

### Special Types

| Type | Loro | Behavior |
|---|---|---|
| `Text` | `LoroText` | Character-level LCS via Loro's built-in `update()` |
| `ByteArray<N>` | `Binary` | Fixed-size byte array, length-checked on hydration |
| `MaybeMissing<T>` | `Null` / `T` | Distinguishes "absent key" from "present" (unlike `Option`) |

## Attributes

### Container-level

```rust
#[loro(root = "key")]  // Generate DocSync impl — to_doc()/from_doc() at named root
```

### Field-level

```rust
#[key]                       // Identity key for movable list diffing
#[loro(rename = "name")]     // Use different key name in Loro
#[loro(json)]                // serde_json round-trip (coarse-grained fallback)
#[loro(movable)]             // Use LoroMovableList instead of LoroList
#[loro(missing)]             // Default::default() when key absent
#[loro(missing = "fn")]      // Custom function when key absent
#[loro(flatten)]             // Inline nested struct fields into parent map
#[loro(with = "module")]     // Custom hydrate + reconcile via module::hydrate / module::reconcile
#[loro(hydrate = "fn")]      // Custom hydrate function
#[loro(reconcile = "fn")]    // Custom reconcile function
```

## Keyed List Diffing

`#[loro(movable)]` + `#[key]` enables identity-preserving list reconciliation on `LoroMovableList`:

- **Matched items** → `set()` in-place, preserving CRDT element identity
- **New items** → `insert()`
- **Removed items** → `delete()`
- **Reordered items** → `mov()`

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

Two peers editing different fields of the same item merge correctly — `set()` preserves container identity so field-level changes compose.

### Enum Keys

Enums with `#[key]` on any variant generate a companion key type automatically:

```rust
#[derive(Hydrate, Reconcile)]
enum Shape {
    Circle { #[key] id: String, radius: f64 },
    Rectangle { #[key] id: String, width: f64, height: f64 },
    Point,  // Matches by variant name
}
```

## Custom Hydration and Reconciliation

For fields that need custom serialization logic:

```rust
mod hex_color {
    use loro::LoroMap;
    use lorosurgeon::{HydrateError, MapReconciler};

    pub fn hydrate(map: &LoroMap, key: &str) -> Result<[u8; 3], HydrateError> { /* ... */ }
    pub fn reconcile(value: &[u8; 3], m: &mut MapReconciler, key: &str) -> Result<(), lorosurgeon::ReconcileError> { /* ... */ }
}

#[derive(Hydrate, Reconcile)]
struct Theme {
    #[loro(with = "hex_color")]
    primary: [u8; 3],
}
```

`#[loro(hydrate = "fn")]` and `#[loro(reconcile = "fn")]` can also be used independently when you only need to customize one direction.

## Flatten

`#[loro(flatten)]` inlines a nested struct's fields directly into the parent map:

```rust
#[derive(Hydrate, Reconcile)]
struct Position { x: f64, y: f64, rotation: f64 }

#[derive(Hydrate, Reconcile)]
struct Element {
    id: String,
    #[loro(flatten)]
    position: Position,  // x, y, rotation written directly to Element's map
}
```

Multiple `#[loro(flatten)]` fields are supported. The inner struct's attributes (`#[loro(missing)]`, etc.) are respected.

## Optimizations

**No-op detection** — Reconciling identical values produces zero CRDT operations. The reconciler reads existing values before writing and skips unchanged fields.

**LCS diffing** — `Vec<T>` reconciliation uses Myers LCS (via [`similar`](https://docs.rs/similar)) to compute minimal insert/delete operations. Types implementing `Hydrate + PartialEq` get the optimized path; others fall back to clear+rewrite.

**Stale heads detection** — `VersionGuard` captures a document's version vector and detects concurrent modifications before write-back:

```rust
let guard = VersionGuard::capture(&doc);
let mut state = MyState::from_doc(&doc)?;
state.name = "updated".into();
guard.check(&doc)?;  // Err(StaleHeads) if doc was modified concurrently
state.to_doc(&doc)?;
doc.commit();
```

## Cargo Features

| Feature | Effect |
|---|---|
| `uuid` | `Hydrate`/`Reconcile` for `uuid::Uuid` (stored as 16-byte binary) |

## Crate Structure

```
lorosurgeon/          # Core traits, type impls, special types
lorosurgeon-derive/   # Proc macros (#[derive(Hydrate, Reconcile)])
```

## License

MIT
