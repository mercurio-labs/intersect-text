#![cfg(not(target_arch = "wasm32"))]
use intersect_model::core::{
    model::{AssignmentState, Element},
    storage::{self, ModelRead, ModelStore},
};
use intersect_model::sqlite::SqliteStore;
use serde_json::json;

fn record() -> Element {
    let mut element: Element = serde_json::from_value(
        json!({"id":"object","class":"Item","type_identity":"urn:raw#Item",
        "resource":"memory:state","start":0,"end":1,"values":{"value":"seed","items":["a"]}}),
    )
    .unwrap();
    element.assignment_state = Some(AssignmentState::new(["value", "items"]).unwrap());
    element
}

#[test]
fn fresh_assignment_storage_and_nested_rollback_preserve_state() {
    let mut store = SqliteStore::in_memory().unwrap();
    storage::atomic(&mut store, |s| s.put(record())).unwrap();
    let original = storage::export(&store).unwrap();
    let outer = store.begin().unwrap();
    store.append_value("object", "items", json!("b")).unwrap();
    let inner = store.begin().unwrap();
    let mut replacement = store.get("object").unwrap().unwrap();
    replacement.assignment_state = Some(AssignmentState::new(Vec::<String>::new()).unwrap());
    store.put(replacement).unwrap();
    store.commit(&inner).unwrap();
    assert_eq!(
        store
            .get("object")
            .unwrap()
            .unwrap()
            .assignment_state
            .unwrap()
            .set_features()
            .len(),
        0
    );
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), original);
    assert_eq!(store.get("object").unwrap().unwrap(), record());
}

#[test]
fn reopened_format_five_retains_state_and_old_headers_reject() {
    let path =
        std::env::temp_dir().join(format!("intersect-e09-state-{}.sqlite", std::process::id()));
    assert!(!path.exists(), "test path already exists");
    let mut store = SqliteStore::open(&path).unwrap();
    storage::atomic(&mut store, |s| s.put(record())).unwrap();
    drop(store);
    let store = SqliteStore::open(&path).unwrap();
    assert_eq!(store.get("object").unwrap().unwrap(), record());
    drop(store);
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
    connection.execute_batch("PRAGMA user_version=2;").unwrap();
    drop(connection);
    assert!(
        SqliteStore::open(&path)
            .err()
            .unwrap()
            .contains("expected format 5")
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn raw_assignment_update_obeys_transaction_and_rollback_contracts() {
    let mut store = SqliteStore::in_memory().unwrap();
    let mut legacy = record();
    legacy.assignment_state = None;
    storage::atomic(&mut store, |s| s.put(legacy.clone())).unwrap();
    assert!(
        store
            .set_assignment_state("object", None)
            .unwrap_err()
            .contains("transaction")
    );
    let outer = store.begin().unwrap();
    assert!(
        store
            .set_assignment_state("missing", None)
            .unwrap_err()
            .contains("Missing element")
    );
    store
        .set_assignment_state("object", record().assignment_state)
        .unwrap();
    let inner = store.begin().unwrap();
    store.set_assignment_state("object", None).unwrap();
    store.rollback(&inner).unwrap();
    assert_eq!(store.get("object").unwrap().unwrap(), record());
    let inner = store.begin().unwrap();
    store
        .set_assignment_state(
            "object",
            Some(AssignmentState::new(Vec::<String>::new()).unwrap()),
        )
        .unwrap();
    store.commit(&inner).unwrap();
    store.rollback(&outer).unwrap();
    assert_eq!(store.get("object").unwrap().unwrap(), legacy);
    storage::atomic(&mut store, |s| {
        s.set_assignment_state("object", record().assignment_state)
    })
    .unwrap();
    assert_eq!(store.get("object").unwrap().unwrap(), record());
}

#[test]
fn schema_unsettable_construction_edit_and_reset_use_fresh_format_four() {
    use intersect_model::definitions::dynamic::{ScalarType, SchemaBuilder};
    let mut builder = SchemaBuilder::new("p", "urn:p");
    builder
        .class("Item")
        .unwrap()
        .attribute("value", ScalarType::String)
        .unsettable(true)
        .default("seed")
        .unwrap();
    let dynamic = builder.build().unwrap();
    let schema = dynamic.as_schema();
    let mut store = SqliteStore::in_memory().unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(schema, "object", "Item", "model", 0, 0)
    })
    .unwrap();
    storage::validate(&store, schema).unwrap();
    assert!(!storage::is_explicitly_set(&store, schema, "object", "value").unwrap());
    storage::edit(
        &mut store,
        schema,
        &[("object".into(), "value".into(), json!("seed"))],
    )
    .unwrap();
    assert!(storage::is_explicitly_set(&store, schema, "object", "value").unwrap());
    let checkpoint = store.begin().unwrap();
    storage::unset(&mut store, schema, "object", "value").unwrap();
    assert!(!storage::is_explicitly_set(&store, schema, "object", "value").unwrap());
    store.rollback(&checkpoint).unwrap();
    assert!(storage::is_explicitly_set(&store, schema, "object", "value").unwrap());
    storage::unset(&mut store, schema, "object", "value").unwrap();
    storage::validate(&store, schema).unwrap();
}

#[test]
fn text_assignment_history_matches_memory_and_failed_import_rolls_back_sqlite() {
    use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_into};
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/ecore-unsettable/model.ecore"),
        include_str!("../../../../fixtures/ecore-unsettable/model.xtext"),
        include_str!("../../../../fixtures/ecore-unsettable/model-behavior.rcore"),
    )
    .unwrap();
    let sources = vec![Source {
        uri: "model".into(),
        text: r#"root r { item a value "seed" tags "x" link b; item b; }"#.into(),
    }];
    let native = evaluate(&package, &sources);
    assert!(native.diagnostics.is_empty());
    let mut store = SqliteStore::in_memory().unwrap();
    assert!(
        evaluate_into(&package, &sources, EvaluationLimits::native(), &mut store)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    assert_eq!(storage::export(&store).unwrap(), native.model);
    let bad = vec![Source {
        uri: "bad".into(),
        text: r#"root r { item a value "seed" link missing; }"#.into(),
    }];
    assert!(
        !evaluate_into(&package, &bad, EvaluationLimits::native(), &mut store)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    assert_eq!(storage::export(&store).unwrap(), native.model);
    assert!(storage::is_explicitly_set(&store, &package.schema, "model#1", "value").unwrap());
    assert!(!storage::is_explicitly_set(&store, &package.schema, "model#2", "value").unwrap());
}

#[test]
fn inverse_state_transitions_and_rollback_share_sqlite_authority() {
    use intersect_model::definitions::dynamic::SchemaBuilder;
    let mut builder = SchemaBuilder::new("inverse", "urn:inverse");
    let a = builder.class("A").unwrap();
    let b = builder.class("B").unwrap();
    let out = a
        .reference("out", &b)
        .many()
        .unsettable(true)
        .containment()
        .unwrap();
    let back = b.reference("back", &a).unsettable(true).optional().unwrap();
    builder.opposites(&out, &back).unwrap();
    let dynamic = builder.build().unwrap();
    let schema = dynamic.as_schema();
    let mut store = SqliteStore::in_memory().unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(schema, "a", "A", "model", 0, 0)?;
        s.create_element(schema, "b", "B", "model", 0, 0)
    })
    .unwrap();
    storage::edit(
        &mut store,
        schema,
        &[("a".into(), "out".into(), json!(["b"]))],
    )
    .unwrap();
    assert!(storage::is_explicitly_set(&store, schema, "b", "back").unwrap());
    let before = storage::export(&store).unwrap();
    let checkpoint = store.begin().unwrap();
    storage::unset(&mut store, schema, "b", "back").unwrap();
    assert!(!storage::is_explicitly_set(&store, schema, "b", "back").unwrap());
    assert!(storage::is_explicitly_set(&store, schema, "a", "out").unwrap());
    assert_eq!(store.value("a", "out").unwrap(), Some(json!([])));
    store.rollback(&checkpoint).unwrap();
    assert_eq!(storage::export(&store).unwrap(), before);
    storage::unset(&mut store, schema, "a", "out").unwrap();
    assert!(!storage::is_explicitly_set(&store, schema, "a", "out").unwrap());
    assert!(!storage::is_explicitly_set(&store, schema, "b", "back").unwrap());
    storage::validate(&store, schema).unwrap();
}

#[test]
fn readonly_collection_edits_preserve_sqlite_state_and_nested_rollback() {
    use intersect_model::definitions::dynamic::{ScalarType, SchemaBuilder};
    use storage::CollectionEdit;
    let mut builder = SchemaBuilder::new("lists", "urn:lists");
    let c = builder.class("Item").unwrap();
    c.attribute("value", ScalarType::String)
        .many()
        .unsettable(true)
        .changeable(false)
        .build()
        .unwrap();
    let schema = builder.build().unwrap().as_schema().clone();
    let mut store = SqliteStore::in_memory().unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(&schema, "a", "Item", "model", 0, 0)
    })
    .unwrap();
    let original = storage::export(&store).unwrap();
    let outer = store.begin().unwrap();
    assert!(
        !storage::edit_collection(
            &mut store,
            &schema,
            "a",
            "value",
            CollectionEdit::Remove(json!("missing"))
        )
        .unwrap()
    );
    assert!(!storage::is_explicitly_set(&store, &schema, "a", "value").unwrap());
    storage::edit_collection(&mut store, &schema, "a", "value", CollectionEdit::Clear).unwrap();
    assert!(storage::is_explicitly_set(&store, &schema, "a", "value").unwrap());
    storage::edit_collection(
        &mut store,
        &schema,
        "a",
        "value",
        CollectionEdit::Add(json!("x")),
    )
    .unwrap();
    for edit in [
        CollectionEdit::Insert {
            index: 0,
            value: json!("y"),
        },
        CollectionEdit::Move { from: 1, to: 0 },
        CollectionEdit::Replace {
            index: 0,
            value: json!("z"),
        },
        CollectionEdit::RemoveAt(1),
    ] {
        storage::edit_collection(&mut store, &schema, "a", "value", edit).unwrap();
    }
    assert_eq!(store.value("a", "value").unwrap(), Some(json!(["z"])));
    let before = storage::export(&store).unwrap();
    assert!(
        storage::edit_collection(
            &mut store,
            &schema,
            "a",
            "value",
            CollectionEdit::Add(json!(42))
        )
        .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), before);
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), original);
}

#[test]
fn sqlite_semantic_set_query_distinguishes_defaults_history_and_rollback() {
    use intersect_model::definitions::dynamic::{ScalarType, SchemaBuilder};
    let mut builder = SchemaBuilder::new("p", "urn:p");
    let c = builder.class("Item").unwrap();
    c.attribute("ordinary", ScalarType::String)
        .default("seed")
        .unwrap();
    c.attribute("history", ScalarType::String)
        .unsettable(true)
        .default("seed")
        .unwrap();
    let schema = builder.build().unwrap().as_schema().clone();
    let mut store = SqliteStore::in_memory().unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(&schema, "a", "Item", "model", 0, 0)
    })
    .unwrap();
    for name in ["ordinary", "history"] {
        assert!(!storage::is_set(&store, &schema, "a", name).unwrap());
    }
    let outer = store.begin().unwrap();
    storage::edit(
        &mut store,
        &schema,
        &[
            ("a".into(), "ordinary".into(), json!("seed")),
            ("a".into(), "history".into(), json!("seed")),
        ],
    )
    .unwrap();
    assert!(!storage::is_set(&store, &schema, "a", "ordinary").unwrap());
    assert!(storage::is_set(&store, &schema, "a", "history").unwrap());
    store.rollback(&outer).unwrap();
    assert!(!storage::is_set(&store, &schema, "a", "history").unwrap());
}
