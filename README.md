# lorosurgeon

Derive macros for [Loro](https://loro.dev) CRDT containers. The Loro equivalent of [autosurgeon](https://github.com/automerge/autosurgeon) for Automerge.

`#[derive(Hydrate, Reconcile)]` generates field-level serialization between Rust types and Loro containers — only modified fields produce CRDT operations.

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

## Derives

**`Hydrate`** reads Rust values from Loro containers. **`Reconcile`** writes Rust values into Loro containers with minimal operations.

| Rust Type | Loro Storage |
|-----------|-------------|
| Named struct | `LoroMap` (fields → keys) |
| Newtype struct | Transparent (delegates to inner) |
| Tuple struct | `LoroList` (positional) |
| Unit enum | `String` (variant name) |
| Data enum | `LoroMap` (`{ "Variant": data }`) |

## Scalars

| Rust | Loro | Notes |
|------|------|-------|
| `bool` | `Bool` | |
| `i8`–`i64`, `u8`–`u64`, `usize` | `I64` | Overflow checked on hydration |
| `f32`, `f64` | `Double` | |
| `String`, `&str` | `String` | |
| `Vec<u8>` | `Binary` | Special-cased |
| `Option<T>` | `Null` / T | |
| `Box<T>`, `Cow<'a, T>`, `&T` | Transparent | Delegates to inner |
| `serde_json::Value` | `String` | JSON-serialized |

## Collections

| Rust | Loro | Strategy |
|------|------|----------|
| `Vec<T>` | `LoroList` | Clear + rewrite |
| `Vec<T>` + `#[loro(movable)]` | `LoroMovableList` | Keyed LCS diffing with `mov()`/`set()` |
| `HashMap<String, V>` | `LoroMap` | Put entries, delete stale keys |
| `BTreeMap<String, V>` | `LoroMap` | Same |

## Special Types

| Type | Loro | Behavior |
|------|------|----------|
| `Text` | `LoroText` | LCS-diffed via Loro's built-in `update()` |
| `ByteArray<N>` | `Binary` | Fixed-size, length checked on hydration |
| `MaybeMissing<T>` | Null / T | Distinguishes "absent key" from "present" (unlike `Option`) |

## Attributes

### Container

| Attribute | Effect |
|-----------|--------|
| `#[loro(root = "key")]` | Generate `DocSync` impl — `to_doc()`/`from_doc()` at named root |

### Field

| Attribute | Effect |
|-----------|--------|
| `#[key]` | Identity key for list diffing |
| `#[loro(rename = "name")]` | Use different key name in Loro |
| `#[loro(json)]` | serde_json round-trip (coarse-grained fallback) |
| `#[loro(movable)]` | Use `LoroMovableList` instead of `LoroList` |
| `#[loro(missing)]` | `Default::default()` when key absent |
| `#[loro(missing = "fn")]` | Custom function when key absent |
| `#[loro(flatten)]` | Inline nested struct fields into parent map |
| `#[loro(with = "module")]` | Custom hydrate + reconcile module |
| `#[loro(hydrate = "fn")]` | Custom hydrate function |
| `#[loro(reconcile = "fn")]` | Custom reconcile function |

## Keyed List Diffing

`#[loro(movable)]` + `#[key]` enables identity-preserving list reconciliation on `LoroMovableList`:

- **Matched items** → `set()` in-place (preserves CRDT element identity)
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

Two peers editing different fields of the same item merge correctly — `set()` preserves the container identity so field-level changes compose.

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

## Optimizations

**No-op detection** — reconciling identical values produces zero CRDT operations. The reconciler reads existing values before writing and skips unchanged fields.

**Stale heads detection** — `VersionGuard` captures a document's version vector and detects concurrent modifications before write-back:

```rust
let guard = VersionGuard::capture(&doc);
let mut state = MyState::from_doc(&doc)?;
state.name = "updated".into();
guard.check(&doc)?;  // Err(StaleHeads) if doc was modified
state.to_doc(&doc)?;
doc.commit();
```

## Cargo Features

| Feature | Effect |
|---------|--------|
| `uuid` | `Hydrate`/`Reconcile` for `uuid::Uuid` (16-byte binary) |

## Crate Structure

```
lorosurgeon/          # Core traits, type impls, special types
lorosurgeon-derive/   # Proc macros
```

## License

MIT OR Apache-2.0
