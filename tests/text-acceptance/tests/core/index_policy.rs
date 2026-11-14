use intersect_model::core::{
    frozen::{FrozenStore, Metamodel},
    model::Element,
    storage::{self, IndexPolicy, ModelRead, ModelStore, Query, ReferenceView},
};
use intersect_model::definitions::{
    dynamic::{ScalarType, SchemaBuilder},
    schema::Schema,
};
use serde_json::{Value, json};
use std::sync::Arc;

#[test]
fn selected_postings_follow_bound_class_changes_removal_and_nested_rollback() {
    let mut builder = SchemaBuilder::new("index", "urn:frozen:index");
    let node = builder.class("Node").unwrap();
    node.attribute("name", ScalarType::String)
        .default("")
        .unwrap();
    node.reference("links", &node).many().build().unwrap();
    builder.class("Leaf").unwrap();
    let schema = builder.build().unwrap().as_schema().clone();
    let metamodel = Arc::new(Metamodel::freeze(&schema).unwrap());
    let policy = IndexPolicy::selected(["name"]);
    let mut store = FrozenStore::with_index_policy(metamodel, policy);
    storage::atomic(&mut store, |store| {
        for id in ["z", "a", "m"] {
            store.create_element(&schema, id, "Node", "source", 0, 0)?;
        }
        store.set_value("z", "name", Some(json!(["hit", "hit", ["nested"], null])))?;
        store.set_value("m", "name", Some(json!("hit")))?;
        store.set_value("z", "links", Some(json!(["a"])))
    })
    .unwrap();
    let hit = json!("hit");
    let query = Query::Value("name", &hit);
    let first = store.query_candidates(query, None, 1).unwrap();
    assert!(first.exact && !first.exhausted);
    assert_eq!(
        first
            .rows
            .iter()
            .map(|(_, id)| id.as_str())
            .collect::<Vec<_>>(),
        ["z"]
    );
    let last = store.query_candidates(query, first.after, 1).unwrap();
    assert!(last.exact && last.exhausted);
    assert_eq!(
        last.rows
            .iter()
            .map(|(_, id)| id.as_str())
            .collect::<Vec<_>>(),
        ["m"]
    );
    assert_eq!(
        store
            .query(Query::Value("name", &json!(["nested"])))
            .unwrap(),
        ["z"]
    );
    assert!(
        store
            .query(Query::Value("name", &Value::Null))
            .unwrap()
            .is_empty()
    );
    let link = json!("a");
    let unindexed = Query::Value("links", &link);
    let page = store.query_candidates(unindexed, None, 2).unwrap();
    assert!(!page.exact && !page.exhausted);
    assert_eq!(
        page.rows
            .iter()
            .map(|(_, id)| id.as_str())
            .collect::<Vec<_>>(),
        ["z", "a"]
    );
    assert_eq!(store.query(unindexed).unwrap(), ["z"]);
    assert!(store.query_candidates(query, None, 0).is_err());

    let outer = store.begin().unwrap();
    let inner = store.begin().unwrap();
    store.remove("z").unwrap();
    store.set_value("a", "name", Some(hit.clone())).unwrap();
    store
        .put(Element::new(&schema, "m", "Leaf", "destination", 0, 0).unwrap())
        .unwrap();
    store.commit(&inner).unwrap();
    assert_eq!(store.query(query).unwrap(), ["a"]);
    assert_eq!(store.query(Query::Class("Leaf")).unwrap(), ["m"]);
    assert_eq!(store.query(Query::Resource("destination")).unwrap(), ["m"]);
    store.rollback(&outer).unwrap();
    assert_eq!(store.query(query).unwrap(), ["z", "m"]);
    assert_eq!(store.query(Query::Class("Node")).unwrap(), ["z", "a", "m"]);
    assert!(
        store
            .query(Query::Resource("destination"))
            .unwrap()
            .is_empty()
    );
}

fn stores(schema: &Schema) -> Vec<FrozenStore> {
    let metamodel = Arc::new(Metamodel::freeze(schema).unwrap());
    [
        IndexPolicy::AllValues,
        IndexPolicy::selected(["strings", "flag", "edition"]),
        IndexPolicy::selected(std::iter::empty::<&str>()),
    ]
    .into_iter()
    .map(|policy| FrozenStore::with_index_policy(metamodel.clone(), policy))
    .collect()
}

fn raw(schema: &Schema, id: &str) -> Element {
    Element {
        implicit: false,
        initialization: None,
        assignment_state: None,
        id: id.into(),
        class: "Raw".into(),
        type_identity: schema.identity("Raw"),
        resource: "source:α".into(),
        start: 7,
        end: 42,
        values: serde_json::from_value(json!({
            "null": null, "flag": true, "integer": 1, "float": 1.0,
            "zero": -0.0, "text": "λ\n\"", "strings": ["a", "b", "a"],
            "empty": [], "mixed": [null, true, 42, ["nested"], {"a": 1}],
            "object": {"z": [1, 2]}, "class": "Raw", "resource": "source:α",
        }))
        .unwrap(),
    }
}

fn assert_query(stores: &[FrozenStore], oracle: &impl ModelRead, query: Query<'_>) {
    let expected = oracle.query(query).unwrap();
    for store in stores {
        assert_eq!(store.query(query).unwrap(), expected);
        for (_, id) in store.id_page(None, usize::MAX).unwrap() {
            assert_eq!(
                store.matches(&id, query).unwrap(),
                expected.contains(&id),
                "membership for {id}"
            );
        }
        assert!(!store.matches("missing-object", query).unwrap());
    }
}

// The compatibility model has its own serialized-key index. It checks the
// indexed and scanning stores against a separate implementation, including
// integer/float distinctions and the rule that null atoms are not indexed.
fn assert_equivalent(stores: &[FrozenStore]) {
    let model = storage::export(&stores[0]).unwrap();
    for store in stores {
        assert_eq!(storage::export(store).unwrap(), model);
    }
    let oracle = storage::ModelView::new(&model).unwrap();
    for class in ["Raw", "Changed", "Library", "Member", "Loan", "missing"] {
        assert_query(stores, &oracle, Query::Class(class));
    }
    for resource in ["source:α", "changed", "model", "missing"] {
        assert_query(stores, &oracle, Query::Resource(resource));
    }
    for (field, value) in [
        ("null", Value::Null),
        ("flag", json!(true)),
        ("integer", json!(1)),
        ("integer", json!(1.0)),
        ("float", json!(1)),
        ("float", json!(1.0)),
        ("zero", json!(0.0)),
        ("zero", json!(-0.0)),
        ("text", json!("λ\n\"")),
        ("text", json!("changed")),
        ("strings", json!("a")),
        ("strings", json!("b")),
        ("strings", json!(["a", "b", "a"])),
        ("empty", json!([])),
        ("mixed", Value::Null),
        ("mixed", json!(true)),
        ("mixed", json!(42)),
        ("mixed", json!(["nested"])),
        ("mixed", json!({"a": 1})),
        ("object", json!({"z": [1, 2]})),
        ("class", json!("Raw")),
        ("resource", json!("source:α")),
        ("absent", json!("a")),
        ("name", json!("")),
        ("edition", json!(1)),
        ("edition", Value::Null),
        ("members", json!("target")),
        ("member", json!("target")),
        ("member", json!("?future")),
        ("member", json!("future")),
    ] {
        assert_query(stores, &oracle, Query::Value(field, &value));
    }
}

#[test]
fn index_selection_preserves_raw_queries_duplicates_order_and_nested_rollback() {
    let mut builder = SchemaBuilder::new("index", "urn:raw");
    let row = builder.class("Raw").unwrap();
    for field in [
        "null", "flag", "integer", "float", "zero", "text", "object", "class", "resource",
    ] {
        row.attribute(field, ScalarType::String).optional().unwrap();
    }
    for field in ["strings", "empty", "mixed"] {
        row.attribute(field, ScalarType::String)
            .many()
            .unique(false)
            .build()
            .unwrap();
    }
    builder.class("Changed").unwrap().extends(&row).unwrap();
    let schema = builder.build().unwrap();
    let schema = schema.as_schema();
    let mut stores = stores(schema);
    for store in &mut stores {
        storage::atomic(store, |s| {
            s.put(raw(schema, "z-first"))?;
            s.put(raw(schema, "a-second"))
        })
        .unwrap();
    }
    assert_equivalent(&stores);
    let original = storage::export(&stores[0]).unwrap();
    let outer: Vec<_> = stores.iter_mut().map(|s| s.begin().unwrap()).collect();
    for store in &mut stores {
        store
            .set_value("z-first", "strings", Some(json!(["a", "a"])))
            .unwrap();
        store
            .append_value("z-first", "strings", json!("a"))
            .unwrap();
        store.truncate_values("z-first", "strings", 1).unwrap();
        let inner = store.begin().unwrap();
        let mut changed = store.get("a-second").unwrap().unwrap();
        changed.class = "Changed".into();
        changed.type_identity = schema.identity("Changed");
        changed.resource = "changed".into();
        changed.values.insert("text".into(), json!("changed"));
        store.put(changed).unwrap();
        store.commit(&inner).unwrap();
    }
    assert_equivalent(&stores);
    assert_eq!(
        stores[0]
            .query(Query::Value("strings", &json!("a")))
            .unwrap(),
        ["z-first", "a-second"]
    );

    let child: Vec<_> = stores.iter_mut().map(|s| s.begin().unwrap()).collect();
    for store in &mut stores {
        store.remove("z-first").unwrap();
        store.put(raw(schema, "z-first")).unwrap();
    }
    assert_equivalent(&stores);
    assert_eq!(
        stores[0]
            .query(Query::Value("strings", &json!("a")))
            .unwrap(),
        ["a-second", "z-first"]
    );
    for (store, child) in stores.iter_mut().zip(child) {
        store.rollback(&child).unwrap();
    }
    assert_equivalent(&stores);
    for (store, outer) in stores.iter_mut().zip(outer) {
        store.rollback(&outer).unwrap();
    }
    assert_equivalent(&stores);
    assert_eq!(storage::export(&stores[0]).unwrap(), original);

    for store in &mut stores {
        storage::atomic(store, |s| {
            s.remove("z-first")?;
            s.put(raw(schema, "z-first"))?;
            s.truncate_values("a-second", "strings", 0)
        })
        .unwrap();
    }
    assert_equivalent(&stores);
    assert_eq!(
        stores[0].query(Query::Class("Raw")).unwrap(),
        ["a-second", "z-first"]
    );
    assert_eq!(
        stores[0]
            .query(Query::Value("strings", &json!("a")))
            .unwrap(),
        ["z-first"]
    );
}

#[test]
fn index_selection_preserves_defaults_reference_states_and_identity_rebinding() {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap()
    .schema;
    let mut stores = stores(&schema);
    for store in &mut stores {
        storage::atomic(store, |s| {
            s.create_element(&schema, "root", "Library", "model", 0, 1)?;
            s.create_element(&schema, "copy", "BookCopy", "model", 1, 2)?;
            s.create_element(&schema, "null", "BookCopy", "model", 2, 3)?;
            s.set_value("null", "edition", Some(Value::Null))?;
            s.create_element(&schema, "absent", "BookCopy", "model", 3, 4)?;
            s.set_value("absent", "edition", None)?;
            s.create_element(&schema, "target", "Member", "model", 4, 5)?;
            s.create_element(&schema, "loan", "Loan", "model", 5, 6)?;
            s.append_value("root", "members", json!("target"))?;
            s.append_value("root", "members", json!("target"))?;
            s.set_value("loan", "member", Some(json!("target")))
        })
        .unwrap();
    }
    assert_equivalent(&stores);
    assert_eq!(
        stores[0].query(Query::Value("edition", &json!(1))).unwrap(),
        ["copy"]
    );
    let original: Vec<_> = stores.iter().map(|s| s.handle("target").unwrap()).collect();
    let outer: Vec<_> = stores.iter_mut().map(|s| s.begin().unwrap()).collect();
    for store in &mut stores {
        store.remove("target").unwrap();
        let loan = store.handle("loan").unwrap();
        store
            .inspect_handle(&loan, &mut |object| {
                assert!(matches!(
                    object.value("member").unwrap().reference(),
                    Some(ReferenceView::Unavailable { identity: "target" })
                ));
                Ok(())
            })
            .unwrap();
    }
    assert_equivalent(&stores);
    for (store, original) in stores.iter_mut().zip(&original) {
        let child = store.begin().unwrap();
        store
            .create_element(&schema, "target", "Member", "model", 7, 8)
            .unwrap();
        store.commit(&child).unwrap();
        assert!(store.inspect_handle(original, &mut |_| Ok(())).is_err());
        let rebound = store.handle("target").unwrap();
        let loan = store.handle("loan").unwrap();
        store
            .inspect_handle(&loan, &mut |object| {
                assert!(matches!(
                    object.value("member").unwrap().reference(),
                    Some(ReferenceView::Resolved(target)) if target == rebound
                ));
                Ok(())
            })
            .unwrap();
    }
    assert_equivalent(&stores);
    for ((store, outer), original) in stores.iter_mut().zip(outer).zip(original) {
        store.rollback(&outer).unwrap();
        assert!(store.inspect_handle(&original, &mut |_| Ok(())).is_ok());
    }
    assert_equivalent(&stores);
    for store in &mut stores {
        storage::atomic(store, |s| {
            s.set_unresolved_reference("loan", "member", "future", false)
        })
        .unwrap();
    }
    assert_equivalent(&stores);
    for store in &mut stores {
        storage::atomic(store, |s| {
            s.create_element(&schema, "future", "Member", "model", 8, 9)?;
            s.resolve_reference("loan", "member", None, "future")
        })
        .unwrap();
    }
    assert_equivalent(&stores);
}
