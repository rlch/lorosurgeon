//! Integration tests for lorosurgeon derive macros and core traits.
#![allow(
    clippy::bool_assert_comparison,   // assert_eq!(x, true) is clearer in tests
    clippy::useless_conversion,       // .into() on same type in loro import_batch
    clippy::box_collection,           // Box<String> intentionally tested
    clippy::owned_cow,                // Cow<'_, String> intentionally tested
)]

use std::borrow::Cow;
use std::collections::HashMap;

use loro::LoroDoc;
use lorosurgeon::{
    ByteArray, DocSync, Hydrate, HydrateResultExt, MapReconciler, Reconcile, RootReconciler,
};

// ── Phase 1: Scalar round-trips ─────────────────────────────────────────

#[test]
fn test_scalar_roundtrip_bool() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("v", true).unwrap();
    doc.commit();

    let result: bool = lorosurgeon::hydrate_prop(&map, "v").unwrap();
    assert_eq!(result, true);
}

#[test]
fn test_scalar_roundtrip_i64() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("v", 42i64).unwrap();
    doc.commit();

    let result: i64 = lorosurgeon::hydrate_prop(&map, "v").unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_scalar_roundtrip_f64() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("v", 2.72f64).unwrap();
    doc.commit();

    let result: f64 = lorosurgeon::hydrate_prop(&map, "v").unwrap();
    assert!((result - 2.72).abs() < f64::EPSILON);
}

#[test]
fn test_scalar_roundtrip_string() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("v", "hello").unwrap();
    doc.commit();

    let result: String = lorosurgeon::hydrate_prop(&map, "v").unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_scalar_roundtrip_option_none() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");
    doc.commit();

    let result: Option<String> = lorosurgeon::hydrate_prop_or_default(&map, "v").unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_scalar_roundtrip_option_some() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("v", "world").unwrap();
    doc.commit();

    let result: Option<String> = lorosurgeon::hydrate_prop(&map, "v").unwrap();
    assert_eq!(result, Some("world".to_string()));
}

#[test]
fn test_integer_overflow() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("v", 300i64).unwrap();
    doc.commit();

    let result = lorosurgeon::hydrate_prop::<u8>(&map, "v");
    assert!(result.is_err());
}

#[test]
fn test_i64_as_f64() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("v", 42i64).unwrap();
    doc.commit();

    let result: f64 = lorosurgeon::hydrate_prop(&map, "v").unwrap();
    assert_eq!(result, 42.0);
}

// ── Phase 2: Reconcile scalars into map ─────────────────────────────────

#[test]
fn test_reconcile_scalars_into_map() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    let mut mr = MapReconciler { map: map.clone() };
    mr.entry("name", &"Alice".to_string()).unwrap();
    mr.entry("age", &30i64).unwrap();
    mr.entry("active", &true).unwrap();
    mr.entry("score", &95.5f64).unwrap();
    doc.commit();

    assert_eq!(
        lorosurgeon::hydrate_prop::<String>(&map, "name").unwrap(),
        "Alice"
    );
    assert_eq!(lorosurgeon::hydrate_prop::<i64>(&map, "age").unwrap(), 30);
    assert_eq!(
        lorosurgeon::hydrate_prop::<bool>(&map, "active").unwrap(),
        true
    );
    assert_eq!(
        lorosurgeon::hydrate_prop::<f64>(&map, "score").unwrap(),
        95.5
    );
}

// ── Phase 3: Derive macros — structs ────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Position {
    x: f64,
    y: f64,
}

#[test]
fn test_derive_simple_struct() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");
    let pos = Position { x: 10.0, y: 20.0 };

    let reconciler = RootReconciler::new(map.clone());
    pos.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = Position::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, pos);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct ResourceMeta {
    #[loro(default)]
    id: String,
    name: String,
    description: Option<String>,
    #[loro(default)]
    forkable: bool,
}

#[test]
fn test_derive_struct_with_options_and_missing() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let meta = ResourceMeta {
        id: "abc".to_string(),
        name: "Test".to_string(),
        description: Some("A test resource".to_string()),
        forkable: true,
    };

    let reconciler = RootReconciler::new(map.clone());
    meta.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ResourceMeta::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, meta);
}

#[test]
fn test_derive_struct_missing_fields_use_defaults() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    // Only write the required field
    map.insert("name", "Test").unwrap();
    doc.commit();

    let hydrated = ResourceMeta::hydrate_map(&map).unwrap();
    assert_eq!(hydrated.id, ""); // Default for String
    assert_eq!(hydrated.name, "Test");
    assert_eq!(hydrated.description, None); // Option defaults to None
    assert_eq!(hydrated.forkable, false); // Default for bool
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Nested {
    pos: Position,
    label: String,
}

#[test]
fn test_derive_nested_struct() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let nested = Nested {
        pos: Position { x: 1.0, y: 2.0 },
        label: "origin".to_string(),
    };

    let reconciler = RootReconciler::new(map.clone());
    nested.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = Nested::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, nested);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Wrapper(String);

#[test]
fn test_derive_newtype_struct() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = Wrapper("hello".to_string());
    map.insert("w", "hello").unwrap();
    doc.commit();

    let hydrated: Wrapper = lorosurgeon::hydrate_prop(&map, "w").unwrap();
    assert_eq!(hydrated, val);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
#[loro(root = "config")]
struct Config {
    version: i64,
    name: String,
}

#[test]
fn test_derive_doc_sync() {
    let doc = LoroDoc::new();

    let config = Config {
        version: 1,
        name: "test-doc".to_string(),
    };

    config.to_doc(&doc).unwrap();
    doc.commit();

    let hydrated = Config::from_doc(&doc).unwrap();
    assert_eq!(hydrated, config);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct WithRename {
    #[loro(rename = "pos_x")]
    x: f64,
    #[loro(rename = "pos_y")]
    y: f64,
}

#[test]
fn test_derive_rename() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = WithRename { x: 1.0, y: 2.0 };
    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    // Verify the actual keys in Loro
    let x: f64 = lorosurgeon::hydrate_prop(&map, "pos_x").unwrap();
    let y: f64 = lorosurgeon::hydrate_prop(&map, "pos_y").unwrap();
    assert_eq!(x, 1.0);
    assert_eq!(y, 2.0);

    let hydrated = WithRename::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
struct Theme {
    primary: String,
    font_size: i32,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct WithJson {
    name: String,
    #[loro(json, default)]
    theme: Theme,
}

#[test]
fn test_derive_json_attribute() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = WithJson {
        name: "test".to_string(),
        theme: Theme {
            primary: "blue".to_string(),
            font_size: 14,
        },
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = WithJson::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

// ── Phase 4: Derive macros — enums ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn test_derive_unit_enum() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    map.insert("color", "Green").unwrap();
    doc.commit();

    let hydrated: Color = lorosurgeon::hydrate_prop(&map, "color").unwrap();
    assert_eq!(hydrated, Color::Green);

    // Round-trip via reconcile
    let mut mr = MapReconciler { map: map.clone() };
    mr.entry("color2", &Color::Blue).unwrap();
    doc.commit();

    let hydrated2: Color = lorosurgeon::hydrate_prop(&map, "color2").unwrap();
    assert_eq!(hydrated2, Color::Blue);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Point,
}

#[test]
fn test_derive_mixed_enum_named_variant() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let shape = Shape::Circle { radius: 5.0 };
    let reconciler = RootReconciler::new(map.clone());
    shape.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = Shape::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, shape);
}

#[test]
fn test_derive_mixed_enum_unit_variant() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let shape = Shape::Point;
    let reconciler = RootReconciler::new(map.clone());
    shape.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = Shape::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, shape);
}

#[test]
fn test_derive_mixed_enum_switch_variant() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    // Write Circle first
    let shape1 = Shape::Circle { radius: 5.0 };
    let reconciler = RootReconciler::new(map.clone());
    shape1.reconcile(reconciler).unwrap();
    doc.commit();

    // Overwrite with Rectangle — should clean up Circle key
    let shape2 = Shape::Rectangle {
        width: 10.0,
        height: 20.0,
    };
    let reconciler = RootReconciler::new(map.clone());
    shape2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = Shape::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, shape2);

    // Verify Circle key is gone
    assert!(map.get("Circle").is_none());
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum Value {
    Int(i64),
    Text(String),
    Pair(i64, String),
}

#[test]
fn test_derive_tuple_enum_single() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = Value::Int(42);
    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = Value::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

#[test]
fn test_derive_tuple_enum_multi() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = Value::Pair(7, "lucky".to_string());
    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = Value::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

// ── Phase 5: HashMap round-trip ─────────────────────────────────────────

#[test]
fn test_hashmap_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let mut data: HashMap<String, i64> = HashMap::new();
    data.insert("a".to_string(), 1);
    data.insert("b".to_string(), 2);
    data.insert("c".to_string(), 3);

    let mut mr = MapReconciler { map: map.clone() };
    mr.entry("data", &data).unwrap();
    doc.commit();

    let hydrated: HashMap<String, i64> = lorosurgeon::hydrate_prop(&map, "data").unwrap();
    assert_eq!(hydrated, data);
}

#[test]
fn test_hashmap_update_removes_old_keys() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    // First write
    let mut data1: HashMap<String, i64> = HashMap::new();
    data1.insert("a".to_string(), 1);
    data1.insert("b".to_string(), 2);
    let mut mr = MapReconciler { map: map.clone() };
    mr.entry("data", &data1).unwrap();
    doc.commit();

    // Second write — removes "b", adds "c"
    let mut data2: HashMap<String, i64> = HashMap::new();
    data2.insert("a".to_string(), 10);
    data2.insert("c".to_string(), 3);
    let mut mr = MapReconciler { map: map.clone() };
    mr.entry("data", &data2).unwrap();
    doc.commit();

    let hydrated: HashMap<String, i64> = lorosurgeon::hydrate_prop(&map, "data").unwrap();
    assert_eq!(hydrated, data2);
}

// ── Phase 6: Text type ──────────────────────────────────────────────────

#[test]
fn test_text_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let text = lorosurgeon::Text::new("Hello, world!");
    let mut mr = MapReconciler { map: map.clone() };
    mr.entry("content", &text).unwrap();
    doc.commit();

    let hydrated: lorosurgeon::Text = lorosurgeon::hydrate_prop(&map, "content").unwrap();
    assert_eq!(hydrated, text);
}

// ── Phase 7: MaybeMissing ───────────────────────────────────────────────

#[test]
fn test_maybe_missing_absent() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let result: lorosurgeon::MaybeMissing<String> =
        lorosurgeon::hydrate_prop_or_default(&map, "nonexistent").unwrap();
    assert!(result.is_missing());
}

#[test]
fn test_maybe_missing_present() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    map.insert("name", "Alice").unwrap();
    doc.commit();

    let result: lorosurgeon::MaybeMissing<String> =
        lorosurgeon::hydrate_prop(&map, "name").unwrap();
    assert_eq!(
        result,
        lorosurgeon::MaybeMissing::Present("Alice".to_string())
    );
}

// ── Phase 8: Concurrent edits (CRDT verification) ───────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
#[loro(root = "data")]
struct CrdtStruct {
    name: String,
    value: i64,
    active: bool,
}

#[test]
fn test_concurrent_field_edits_merge() {
    let doc1 = LoroDoc::new();
    let initial = CrdtStruct {
        name: "initial".to_string(),
        value: 0,
        active: false,
    };
    initial.to_doc(&doc1).unwrap();
    doc1.commit();

    // Fork
    let bytes = doc1.export(loro::ExportMode::snapshot()).unwrap();
    let doc2 = LoroDoc::new();
    doc2.import_batch(&[bytes.into()]).unwrap();

    // doc1: update name
    {
        let updated = CrdtStruct {
            name: "updated-by-1".to_string(),
            value: 0,
            active: false,
        };
        updated.to_doc(&doc1).unwrap();
        doc1.commit();
    }

    // doc2: update value and active
    {
        let updated = CrdtStruct {
            name: "initial".to_string(),
            value: 42,
            active: true,
        };
        updated.to_doc(&doc2).unwrap();
        doc2.commit();
    }

    // Merge
    let update1 = doc1.export(loro::ExportMode::all_updates()).unwrap();
    let update2 = doc2.export(loro::ExportMode::all_updates()).unwrap();

    let merged = LoroDoc::new();
    merged
        .import_batch(&[update1.into(), update2.into()])
        .unwrap();

    let result = CrdtStruct::from_doc(&merged).unwrap();
    // Both changes should be present (last-writer-wins per field)
    assert_eq!(result.value, 42);
    assert_eq!(result.active, true);
    // name is LWW — one of the two values wins
    assert!(
        result.name == "updated-by-1" || result.name == "initial",
        "name should be one of the two values"
    );
}

// ── Phase 9: Flatten ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct CanvasPosition {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Element {
    id: String,
    #[loro(flatten)]
    canvas: CanvasPosition,
}

#[test]
fn test_derive_flatten() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let elem = Element {
        id: "e1".to_string(),
        canvas: CanvasPosition {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        },
    };

    let reconciler = RootReconciler::new(map.clone());
    elem.reconcile(reconciler).unwrap();
    doc.commit();

    // Verify flattened keys exist directly on the map
    let x: f64 = lorosurgeon::hydrate_prop(&map, "x").unwrap();
    assert_eq!(x, 10.0);

    let hydrated = Element::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, elem);
}

// ── MovableList LCS reconciliation ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct KeyedItem {
    #[key]
    id: String,
    value: i64,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct PositionalMovable {
    #[loro(movable)]
    items: Vec<i64>,
}

#[test]
fn test_movable_list_positional_basic() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = PositionalMovable {
        items: vec![10, 20, 30],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = PositionalMovable::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v1);

    // Update: set overlap, delete extras, append new
    let v2 = PositionalMovable {
        items: vec![10, 25, 30, 40],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = PositionalMovable::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_movable_list_positional_shrink() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = PositionalMovable {
        items: vec![1, 2, 3, 4, 5],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Shrink to [1, 2]
    let v2 = PositionalMovable { items: vec![1, 2] };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = PositionalMovable::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct MovableContainer {
    #[loro(movable)]
    items: Vec<KeyedItem>,
}

#[test]
fn test_movable_list_keyed_insert_delete() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
            KeyedItem {
                id: "c".into(),
                value: 3,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MovableContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v1);

    // Remove "b", add "d"
    let v2 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
            KeyedItem {
                id: "c".into(),
                value: 3,
            },
            KeyedItem {
                id: "d".into(),
                value: 4,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MovableContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_movable_list_keyed_reorder() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
            KeyedItem {
                id: "c".into(),
                value: 3,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Reverse order
    let v2 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "c".into(),
                value: 3,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MovableContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_movable_list_keyed_update_in_place() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Update values but keep same keys — should use set() (identity-preserving)
    let v2 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 10,
            },
            KeyedItem {
                id: "b".into(),
                value: 20,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MovableContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_movable_list_keyed_concurrent_reorder_merge() {
    // Two peers reorder items concurrently — CRDT should merge
    let doc1 = LoroDoc::new();
    let map1 = doc1.get_map("root");

    let initial = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
            KeyedItem {
                id: "c".into(),
                value: 3,
            },
        ],
    };

    let reconciler = RootReconciler::new(map1.clone());
    initial.reconcile(reconciler).unwrap();
    doc1.commit();

    // Fork
    let bytes = doc1.export(loro::ExportMode::snapshot()).unwrap();
    let doc2 = LoroDoc::new();
    doc2.import_batch(&[bytes.into()]).unwrap();
    let map2 = doc2.get_map("root");

    // Peer 1: update "a" value
    let v1 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 100,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
            KeyedItem {
                id: "c".into(),
                value: 3,
            },
        ],
    };
    let reconciler = RootReconciler::new(map1.clone());
    v1.reconcile(reconciler).unwrap();
    doc1.commit();

    // Peer 2: update "c" value
    let v2 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
            KeyedItem {
                id: "c".into(),
                value: 300,
            },
        ],
    };
    let reconciler = RootReconciler::new(map2.clone());
    v2.reconcile(reconciler).unwrap();
    doc2.commit();

    // Merge
    let update1 = doc1.export(loro::ExportMode::all_updates()).unwrap();
    let update2 = doc2.export(loro::ExportMode::all_updates()).unwrap();
    let merged = LoroDoc::new();
    merged
        .import_batch(&[update1.into(), update2.into()])
        .unwrap();

    let result = MovableContainer::hydrate_map(&merged.get_map("root")).unwrap();
    assert_eq!(result.items.len(), 3);

    // Both field-level updates should be preserved (set() preserves CRDT identity)
    let a = result.items.iter().find(|i| i.id == "a").unwrap();
    let c = result.items.iter().find(|i| i.id == "c").unwrap();
    assert_eq!(a.value, 100);
    assert_eq!(c.value, 300);
}

#[test]
fn test_movable_list_empty_to_nonempty() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = MovableContainer { items: vec![] };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MovableContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated.items.len(), 0);

    // Add items
    let v2 = MovableContainer {
        items: vec![KeyedItem {
            id: "x".into(),
            value: 42,
        }],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MovableContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_movable_list_nonempty_to_empty() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = MovableContainer {
        items: vec![
            KeyedItem {
                id: "a".into(),
                value: 1,
            },
            KeyedItem {
                id: "b".into(),
                value: 2,
            },
        ],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let v2 = MovableContainer { items: vec![] };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MovableContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated.items.len(), 0);
}

// ── No-op detection ─────────────────────────────────────────────────────

#[test]
fn test_noop_detection_no_history_bloat() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let state = Position { x: 10.0, y: 20.0 };

    // First write
    let reconciler = RootReconciler::new(map.clone());
    state.reconcile(reconciler).unwrap();
    doc.commit();

    let version_after_first = doc.oplog_vv();

    // Second write — identical values, should be no-op
    let reconciler = RootReconciler::new(map.clone());
    state.reconcile(reconciler).unwrap();
    doc.commit();

    let version_after_second = doc.oplog_vv();

    // Version vectors should be identical (no new operations created)
    assert_eq!(
        version_after_first, version_after_second,
        "Reconciling identical values should not create new CRDT operations"
    );
}

#[test]
fn test_noop_detection_partial_change() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let state1 = Position { x: 10.0, y: 20.0 };
    let reconciler = RootReconciler::new(map.clone());
    state1.reconcile(reconciler).unwrap();
    doc.commit();

    let version_after_first = doc.oplog_vv();

    // Only change y — x should be skipped
    let state2 = Position { x: 10.0, y: 30.0 };
    let reconciler = RootReconciler::new(map.clone());
    state2.reconcile(reconciler).unwrap();
    doc.commit();

    let version_after_second = doc.oplog_vv();

    // Should have new ops (y changed), but fewer than if both were written
    assert_ne!(
        version_after_first, version_after_second,
        "Changed values should create new CRDT operations"
    );

    // Verify correct values
    let hydrated = Position::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, state2);
}

// ── Stale heads detection ───────────────────────────────────────────────

#[test]
fn test_version_guard_no_change() {
    let doc = LoroDoc::new();
    let guard = lorosurgeon::VersionGuard::capture(&doc);
    // No changes — check should pass
    assert!(guard.check(&doc).is_ok());
}

#[test]
fn test_version_guard_detects_concurrent_commit() {
    let doc = LoroDoc::new();

    let config = Config {
        version: 1,
        name: "initial".to_string(),
    };
    config.to_doc(&doc).unwrap();
    doc.commit();

    // Capture version after initial commit
    let guard = lorosurgeon::VersionGuard::capture(&doc);

    // Simulate concurrent modification
    let map = doc.get_map("config");
    map.insert("name", "concurrent-change").unwrap();
    doc.commit();

    // Guard should detect the change
    let result = guard.check(&doc);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        lorosurgeon::ReconcileError::StaleHeads
    ));
}

#[test]
fn test_version_guard_workflow() {
    let doc = LoroDoc::new();

    let initial = Config {
        version: 1,
        name: "test".to_string(),
    };
    initial.to_doc(&doc).unwrap();
    doc.commit();

    // Read-modify-write with version guard
    let guard = lorosurgeon::VersionGuard::capture(&doc);
    let mut state = Config::from_doc(&doc).unwrap();
    state.version = 2;
    state.name = "updated".to_string();

    // No concurrent modification — guard passes
    guard.check(&doc).unwrap();
    state.to_doc(&doc).unwrap();
    doc.commit();

    let result = Config::from_doc(&doc).unwrap();
    assert_eq!(result.version, 2);
    assert_eq!(result.name, "updated");
}

// ── Phase 10: Struct with all features combined ─────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Default)]
struct StyleConfig {
    color: String,
    opacity: f64,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
#[loro(root = "whiteboard")]
struct WhiteboardState {
    #[loro(default)]
    id: String,
    name: String,
    #[loro(rename = "desc")]
    description: Option<String>,
    #[loro(json, default)]
    style: StyleConfig,
    #[loro(flatten)]
    position: Position,
}

#[test]
fn test_full_featured_struct() {
    let doc = LoroDoc::new();

    let state = WhiteboardState {
        id: "wb-1".to_string(),
        name: "My Whiteboard".to_string(),
        description: Some("A test whiteboard".to_string()),
        style: StyleConfig {
            color: "red".to_string(),
            opacity: 0.8,
        },
        position: Position { x: 100.0, y: 200.0 },
    };

    state.to_doc(&doc).unwrap();
    doc.commit();

    let hydrated = WhiteboardState::from_doc(&doc).unwrap();
    assert_eq!(hydrated, state);
}

// ── LoroList LCS diffing ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct ListContainer {
    items: Vec<i64>,
}

#[test]
fn test_list_lcs_basic_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer {
        items: vec![1, 2, 3],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v1);
}

#[test]
fn test_list_lcs_append() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer {
        items: vec![1, 2, 3],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Append — LCS should skip [1,2,3] and only insert [4,5]
    let v2 = ListContainer {
        items: vec![1, 2, 3, 4, 5],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_list_lcs_prepend() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer {
        items: vec![3, 4, 5],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Prepend — LCS should insert [1,2] and skip [3,4,5]
    let v2 = ListContainer {
        items: vec![1, 2, 3, 4, 5],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_list_lcs_middle_insert() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer {
        items: vec![1, 2, 5, 6],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Insert in middle — LCS should skip [1,2], insert [3,4], skip [5,6]
    let v2 = ListContainer {
        items: vec![1, 2, 3, 4, 5, 6],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_list_lcs_delete() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer {
        items: vec![1, 2, 3, 4, 5],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Remove middle — LCS should skip [1,2], delete [3], skip [4,5]
    let v2 = ListContainer {
        items: vec![1, 2, 4, 5],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_list_lcs_no_change_no_ops() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer {
        items: vec![1, 2, 3],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let version_before = doc.oplog_vv();

    // Reconcile identical list — should produce zero ops
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let version_after = doc.oplog_vv();
    assert_eq!(
        version_before, version_after,
        "Identical list should produce zero ops"
    );
}

#[test]
fn test_list_lcs_empty_to_nonempty() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer { items: vec![] };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let v2 = ListContainer {
        items: vec![1, 2, 3],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_list_lcs_nonempty_to_empty() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = ListContainer {
        items: vec![1, 2, 3],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let v2 = ListContainer { items: vec![] };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = ListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct NestedListContainer {
    items: Vec<Position>,
}

#[test]
fn test_list_lcs_nested_structs() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = NestedListContainer {
        items: vec![
            Position { x: 1.0, y: 2.0 },
            Position { x: 3.0, y: 4.0 },
            Position { x: 5.0, y: 6.0 },
        ],
    };
    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Append one, remove first
    let v2 = NestedListContainer {
        items: vec![
            Position { x: 3.0, y: 4.0 },
            Position { x: 5.0, y: 6.0 },
            Position { x: 7.0, y: 8.0 },
        ],
    };
    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = NestedListContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

// ── Gap 1: Box<T> + Cow<'a, T> round-trips ──────────────────────────────

#[test]
fn test_box_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    // Reconcile a Box<Position>
    let boxed = Box::new(Position { x: 1.0, y: 2.0 });
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "pos".into());
    boxed.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: Box<Position> = lorosurgeon::hydrate_prop(&map, "pos").unwrap();
    assert_eq!(*hydrated, Position { x: 1.0, y: 2.0 });
}

#[test]
fn test_cow_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    let cow: Cow<'_, String> = Cow::Owned("hello".to_string());
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "s".into());
    cow.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: Cow<'_, String> = lorosurgeon::hydrate_prop(&map, "s").unwrap();
    assert_eq!(&*hydrated, "hello");
}

// ── Gap 2: &[T] Reconcile ────────────────────────────────────────────────

#[test]
fn test_slice_reconcile() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    let items: &[i64] = &[1, 2, 3];
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "list".into());
    items.reconcile(reconciler).unwrap();
    doc.commit();

    // Hydrate back as Vec
    let list = match map.get("list").unwrap() {
        loro::ValueOrContainer::Container(loro::Container::List(l)) => l,
        _ => panic!("expected list"),
    };
    let result: Vec<i64> = lorosurgeon::hydrate::impls::hydrate_vec_from_list(&list).unwrap();
    assert_eq!(result, vec![1, 2, 3]);
}

// ── Gap 5: MapReconciler::entries() ──────────────────────────────────────

#[test]
fn test_map_reconciler_entries() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    map.insert("a", 1i64).unwrap();
    map.insert("b", 2i64).unwrap();
    doc.commit();

    let mr = MapReconciler { map: map.clone() };
    let entries: HashMap<String, i64> = mr
        .entries()
        .map(|(k, voc)| {
            let v: i64 = lorosurgeon::Hydrate::hydrate(&voc).unwrap();
            (k, v)
        })
        .collect();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries["a"], 1);
    assert_eq!(entries["b"], 2);
}

// ── Gap 6: HydrateResultExt::strip_unexpected() ─────────────────────────

#[test]
fn test_strip_unexpected_ok() {
    let result: Result<i64, lorosurgeon::HydrateError> = Ok(42);
    assert_eq!(result.strip_unexpected().unwrap(), Some(42));
}

#[test]
fn test_strip_unexpected_converts_unexpected() {
    let result: Result<i64, lorosurgeon::HydrateError> =
        Err(lorosurgeon::HydrateError::unexpected("int", "string"));
    assert_eq!(result.strip_unexpected().unwrap(), None);
}

#[test]
fn test_strip_unexpected_propagates_other_errors() {
    let result: Result<i64, lorosurgeon::HydrateError> =
        Err(lorosurgeon::HydrateError::missing("field"));
    assert!(result.strip_unexpected().is_err());
}

// ── Gap 7: ByteArray<N> round-trip ───────────────────────────────────────

#[test]
fn test_byte_array_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    let arr = ByteArray::new([0xDE, 0xAD, 0xBE, 0xEF]);
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "hash".into());
    arr.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: ByteArray<4> = lorosurgeon::hydrate_prop(&map, "hash").unwrap();
    assert_eq!(hydrated, arr);
}

#[test]
fn test_byte_array_wrong_length() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    // Store 4 bytes
    let arr = ByteArray::new([1, 2, 3, 4]);
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "data".into());
    arr.reconcile(reconciler).unwrap();
    doc.commit();

    // Try to hydrate as 8-byte array — should fail
    let result: Result<ByteArray<8>, _> = lorosurgeon::hydrate_prop(&map, "data");
    assert!(result.is_err());
}

// ── Gap 8: &str Reconcile ────────────────────────────────────────────────

#[test]
fn test_str_ref_reconcile() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");

    let s: &str = "hello world";
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "msg".into());
    s.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: String = lorosurgeon::hydrate_prop(&map, "msg").unwrap();
    assert_eq!(hydrated, "hello world");
}

// ── Gap 3: Enum key type generation ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum KeyedShape {
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
    Point,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct KeyedShapeContainer {
    #[loro(movable)]
    shapes: Vec<KeyedShape>,
}

#[test]
fn test_enum_key_basic_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = KeyedShapeContainer {
        shapes: vec![
            KeyedShape::Circle {
                id: "c1".into(),
                radius: 5.0,
            },
            KeyedShape::Rectangle {
                id: "r1".into(),
                width: 10.0,
                height: 20.0,
            },
            KeyedShape::Point,
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = KeyedShapeContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v1);
}

#[test]
fn test_enum_key_reorder_preserves_identity() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = KeyedShapeContainer {
        shapes: vec![
            KeyedShape::Circle {
                id: "c1".into(),
                radius: 5.0,
            },
            KeyedShape::Rectangle {
                id: "r1".into(),
                width: 10.0,
                height: 20.0,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Reverse order and update values
    let v2 = KeyedShapeContainer {
        shapes: vec![
            KeyedShape::Rectangle {
                id: "r1".into(),
                width: 30.0,
                height: 40.0,
            },
            KeyedShape::Circle {
                id: "c1".into(),
                radius: 15.0,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = KeyedShapeContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

#[test]
fn test_enum_key_insert_delete() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = KeyedShapeContainer {
        shapes: vec![
            KeyedShape::Circle {
                id: "c1".into(),
                radius: 5.0,
            },
            KeyedShape::Circle {
                id: "c2".into(),
                radius: 10.0,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Remove c1, add r1
    let v2 = KeyedShapeContainer {
        shapes: vec![
            KeyedShape::Circle {
                id: "c2".into(),
                radius: 10.0,
            },
            KeyedShape::Rectangle {
                id: "r1".into(),
                width: 5.0,
                height: 5.0,
            },
        ],
    };

    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = KeyedShapeContainer::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

// ── Gap 4: Flexible HashMap keys ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MyId(String);

impl From<String> for MyId {
    fn from(s: String) -> Self {
        MyId(s)
    }
}

impl std::fmt::Display for MyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[test]
fn test_flexible_hashmap_keys() {
    let doc = LoroDoc::new();
    let map = doc.get_map("test");
    let inner = map
        .get_or_create_container("data", loro::LoroMap::new())
        .unwrap();

    inner.insert("key1", "value1").unwrap();
    inner.insert("key2", "value2").unwrap();
    doc.commit();

    // Hydrate with custom key type
    let result: HashMap<MyId, String> = lorosurgeon::hydrate_keyed_map(&inner).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[&MyId("key1".into())], "value1");
    assert_eq!(result[&MyId("key2".into())], "value2");

    // Reconcile back using reconcile_keyed_map
    let new_map: HashMap<MyId, String> = [(MyId("key1".into()), "updated".into())].into();
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "data2".into());
    lorosurgeon::reconcile_keyed_map(&new_map, reconciler).unwrap();
    doc.commit();

    let inner2 = match map.get("data2").unwrap() {
        loro::ValueOrContainer::Container(loro::Container::Map(m)) => m,
        _ => panic!("expected map"),
    };
    let result2: HashMap<MyId, String> = lorosurgeon::hydrate_keyed_map(&inner2).unwrap();
    assert_eq!(result2.len(), 1);
    assert_eq!(result2[&MyId("key1".into())], "updated");
}

// ── Custom function attributes ──────────────────────────────────────────

#[allow(clippy::ptr_arg)] // Signatures must match derive macro codegen: &self.field → &String
mod custom_fns {
    use loro::{LoroMap, ValueOrContainer};
    use lorosurgeon::{HydrateError, MapReconciler};

    /// Custom hydrate function: reads a string and uppercases it.
    pub fn hydrate_upper(map: &LoroMap, key: &str) -> Result<String, HydrateError> {
        match map.get(key) {
            Some(ValueOrContainer::Value(loro::LoroValue::String(s))) => Ok(s.to_uppercase()),
            Some(_) => Err(HydrateError::unexpected("string", "other")),
            None => Err(HydrateError::missing(key)),
        }
    }

    /// Custom reconcile function: lowercases before writing.
    pub fn reconcile_lower(
        value: &String,
        m: &mut MapReconciler,
        key: &str,
    ) -> Result<(), lorosurgeon::ReconcileError> {
        m.entry(key, &value.to_lowercase())
    }
}

#[allow(clippy::ptr_arg)]
mod custom_module {
    use loro::LoroMap;
    use lorosurgeon::{HydrateError, MapReconciler};

    /// Module-style: hydrate reverses the string.
    pub fn hydrate(map: &LoroMap, key: &str) -> Result<String, HydrateError> {
        match map.get(key) {
            Some(loro::ValueOrContainer::Value(loro::LoroValue::String(s))) => {
                Ok(s.chars().rev().collect())
            }
            Some(_) => Err(HydrateError::unexpected("string", "other")),
            None => Err(HydrateError::missing(key)),
        }
    }

    /// Module-style: reconcile reverses the string before writing.
    pub fn reconcile(
        value: &String,
        m: &mut MapReconciler,
        key: &str,
    ) -> Result<(), lorosurgeon::ReconcileError> {
        let reversed: String = value.chars().rev().collect();
        m.entry(key, &reversed)
    }
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct CustomHydrateOnly {
    #[loro(hydrate = "custom_fns::hydrate_upper")]
    name: String,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct CustomReconcileOnly {
    #[loro(reconcile = "custom_fns::reconcile_lower")]
    name: String,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct CustomBoth {
    #[loro(
        hydrate = "custom_fns::hydrate_upper",
        reconcile = "custom_fns::reconcile_lower"
    )]
    name: String,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct CustomWithModule {
    #[loro(with = "custom_module")]
    label: String,
}

#[test]
fn test_custom_hydrate_fn() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");
    map.insert("name", "hello").unwrap();
    doc.commit();

    let result = CustomHydrateOnly::hydrate_map(&map).unwrap();
    assert_eq!(result.name, "HELLO");
}

#[test]
fn test_custom_reconcile_fn() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = CustomReconcileOnly {
        name: "HELLO".to_string(),
    };
    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    // Raw value should be lowercased
    let raw: String = lorosurgeon::hydrate_prop(&map, "name").unwrap();
    assert_eq!(raw, "hello");
}

#[test]
fn test_custom_hydrate_and_reconcile() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = CustomBoth {
        name: "Hello".to_string(),
    };
    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    // Reconcile lowercases: stored as "hello"
    let raw: String = lorosurgeon::hydrate_prop(&map, "name").unwrap();
    assert_eq!(raw, "hello");

    // Hydrate uppercases: reads as "HELLO"
    let hydrated = CustomBoth::hydrate_map(&map).unwrap();
    assert_eq!(hydrated.name, "HELLO");
}

#[test]
fn test_custom_with_module() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = CustomWithModule {
        label: "abc".to_string(),
    };
    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    // Module reconcile reverses: stored as "cba"
    let raw: String = lorosurgeon::hydrate_prop(&map, "label").unwrap();
    assert_eq!(raw, "cba");

    // Module hydrate reverses back: reads as "abc"
    let hydrated = CustomWithModule::hydrate_map(&map).unwrap();
    assert_eq!(hydrated.label, "abc");
}

// ── Flatten edge cases ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Dimensions {
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct Origin {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct MultiFlattened {
    name: String,
    #[loro(flatten)]
    origin: Origin,
    #[loro(flatten)]
    size: Dimensions,
}

#[test]
fn test_flatten_multiple() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = MultiFlattened {
        name: "rect".to_string(),
        origin: Origin { x: 5.0, y: 10.0 },
        size: Dimensions {
            width: 100.0,
            height: 50.0,
        },
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    // All fields should be directly on the map
    let x: f64 = lorosurgeon::hydrate_prop(&map, "x").unwrap();
    let y: f64 = lorosurgeon::hydrate_prop(&map, "y").unwrap();
    let w: f64 = lorosurgeon::hydrate_prop(&map, "width").unwrap();
    let h: f64 = lorosurgeon::hydrate_prop(&map, "height").unwrap();
    assert_eq!(x, 5.0);
    assert_eq!(y, 10.0);
    assert_eq!(w, 100.0);
    assert_eq!(h, 50.0);

    let hydrated = MultiFlattened::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct FlattenWithMissing {
    name: String,
    #[loro(flatten)]
    pos: PositionWithDefaults,
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct PositionWithDefaults {
    #[loro(default)]
    x: f64,
    #[loro(default)]
    y: f64,
    #[loro(default)]
    rotation: f64,
}

#[test]
fn test_flatten_with_missing_defaults() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    // Only set name — flattened fields should use defaults
    map.insert("name", "test").unwrap();
    doc.commit();

    let hydrated = FlattenWithMissing::hydrate_map(&map).unwrap();
    assert_eq!(hydrated.name, "test");
    assert_eq!(hydrated.pos.x, 0.0);
    assert_eq!(hydrated.pos.y, 0.0);
    assert_eq!(hydrated.pos.rotation, 0.0);
}

#[test]
fn test_flatten_roundtrip_update() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let v1 = MultiFlattened {
        name: "rect".to_string(),
        origin: Origin { x: 0.0, y: 0.0 },
        size: Dimensions {
            width: 50.0,
            height: 25.0,
        },
    };

    let reconciler = RootReconciler::new(map.clone());
    v1.reconcile(reconciler).unwrap();
    doc.commit();

    // Update only some fields
    let v2 = MultiFlattened {
        name: "rect".to_string(),
        origin: Origin { x: 10.0, y: 20.0 },
        size: Dimensions {
            width: 50.0,
            height: 25.0,
        },
    };

    let reconciler = RootReconciler::new(map.clone());
    v2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = MultiFlattened::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, v2);
}

// ── Derive edge cases: generics ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct GenericWrapper<
    T: lorosurgeon::Hydrate + lorosurgeon::Reconcile + std::fmt::Debug + Clone + PartialEq,
> {
    inner: T,
    label: String,
}

#[test]
fn test_generic_struct_with_scalar() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = GenericWrapper {
        inner: 42i64,
        label: "test".to_string(),
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = GenericWrapper::<i64>::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

#[test]
fn test_generic_struct_nested() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = GenericWrapper {
        inner: Position { x: 1.0, y: 2.0 },
        label: "pos".to_string(),
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = GenericWrapper::<Position>::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

// ── Derive edge cases: enum + json ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum JsonTheme {
    Light,
    Dark,
    Custom { primary: String, secondary: String },
}

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct DocWithJsonEnum {
    name: String,
    #[loro(json)]
    theme: JsonTheme,
}

#[test]
fn test_enum_json_unit_variant() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = DocWithJsonEnum {
        name: "my-doc".to_string(),
        theme: JsonTheme::Light,
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = DocWithJsonEnum::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

#[test]
fn test_enum_json_data_variant() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = DocWithJsonEnum {
        name: "my-doc".to_string(),
        theme: JsonTheme::Custom {
            primary: "#ff0000".to_string(),
            secondary: "#00ff00".to_string(),
        },
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = DocWithJsonEnum::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

// ── Derive edge cases: enum with scalar-only variants ───────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum Content {
    Text { body: String },
    Number { value: i64 },
    Empty,
}

#[test]
fn test_enum_data_variant_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = Content::Text {
        body: "hello".to_string(),
    };
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "v".to_string());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: Content = lorosurgeon::Hydrate::hydrate(&map.get("v").unwrap()).unwrap();
    assert_eq!(hydrated, val);
}

#[test]
fn test_enum_unit_variant_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = Content::Empty;
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "v".to_string());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: Content = lorosurgeon::Hydrate::hydrate(&map.get("v").unwrap()).unwrap();
    assert_eq!(hydrated, val);
}

#[test]
fn test_enum_switch_variant() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    // Write Text variant
    let val1 = Content::Text {
        body: "hello".to_string(),
    };
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "v".to_string());
    val1.reconcile(reconciler).unwrap();
    doc.commit();

    // Overwrite with Number variant
    let val2 = Content::Number { value: 42 };
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "v".to_string());
    val2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: Content = lorosurgeon::Hydrate::hydrate(&map.get("v").unwrap()).unwrap();
    assert_eq!(hydrated, val2);
}

// ── Derive edge cases: Box and Cow fields ───────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct BoxedFields {
    #[loro(default)]
    name: Box<String>,
    value: i64,
}

#[test]
fn test_boxed_field_roundtrip() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = BoxedFields {
        name: Box::new("boxed".to_string()),
        value: 99,
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = BoxedFields::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}

// ── Vec<T> in enum variants ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
enum ListContent {
    Text { body: String },
    Items { items: Vec<String> },
    Empty,
}

#[test]
fn test_enum_with_vec_field() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = ListContent::Items {
        items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "v".to_string());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: ListContent = lorosurgeon::Hydrate::hydrate(&map.get("v").unwrap()).unwrap();
    assert_eq!(hydrated, val);
}

#[test]
fn test_enum_with_vec_field_switch() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    // Write Items variant
    let val1 = ListContent::Items {
        items: vec!["x".to_string()],
    };
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "v".to_string());
    val1.reconcile(reconciler).unwrap();
    doc.commit();

    // Switch to Text variant
    let val2 = ListContent::Text {
        body: "hello".to_string(),
    };
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "v".to_string());
    val2.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: ListContent = lorosurgeon::Hydrate::hydrate(&map.get("v").unwrap()).unwrap();
    assert_eq!(hydrated, val2);
}

// ── Newtype over Vec<T> ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct IdList(Vec<String>);

#[test]
fn test_newtype_over_vec() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = IdList(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "ids".to_string());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: IdList = lorosurgeon::Hydrate::hydrate(&map.get("ids").unwrap()).unwrap();
    assert_eq!(hydrated, val);
}

#[test]
fn test_newtype_over_vec_empty() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = IdList(vec![]);
    let reconciler = lorosurgeon::PropReconciler::map_put(map.clone(), "ids".to_string());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated: IdList = lorosurgeon::Hydrate::hydrate(&map.get("ids").unwrap()).unwrap();
    assert_eq!(hydrated, val);
}

// ── Newtype over Vec<T> as struct field ─────────────────────────────────

#[derive(Debug, Clone, PartialEq, Hydrate, Reconcile)]
struct DocWithIdList {
    name: String,
    tags: IdList,
}

#[test]
fn test_newtype_vec_as_field() {
    let doc = LoroDoc::new();
    let map = doc.get_map("root");

    let val = DocWithIdList {
        name: "test".to_string(),
        tags: IdList(vec!["rust".to_string(), "loro".to_string()]),
    };

    let reconciler = RootReconciler::new(map.clone());
    val.reconcile(reconciler).unwrap();
    doc.commit();

    let hydrated = DocWithIdList::hydrate_map(&map).unwrap();
    assert_eq!(hydrated, val);
}
