use intersect_model::core::{
    frozen::{FrozenStore, Metamodel},
    model::{Element, Model},
    storage::{self, IndexPolicy, ModelRead, ModelStore, ModelView, Query, ReferenceView},
};
use intersect_model::definitions::schema::Schema;
use serde_json::json;
use std::sync::Arc;

fn schema() -> Schema {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .expect("Library schema")
    .schema
}

#[test]
fn reference_collection_edits_preserve_slots_duplicates_and_target_incarnations() {
    let schema = schema();
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "library", "Library", "library", 0, 1)?;
        store.create_element(&schema, "member", "Member", "members", 0, 1)?;
        store.set_value("library", "members", Some(json!(["member", "member"])))
    })
    .expect("create reference collection");
    let owner = store.handle("library").expect("owner");
    let original = store.handle("member").expect("original target");
    let outer = store.begin().expect("begin changes");
    store
        .set_unresolved_reference("library", "members", "late", true)
        .expect("append unresolved item");
    store
        .inspect_handle(&owner, &mut |object| {
            assert!(matches!(
                object
                    .value("members")
                    .unwrap()
                    .collection()
                    .unwrap()
                    .get(2)
                    .unwrap()
                    .reference(),
                Some(ReferenceView::Unresolved { spelling: "late" })
            ));
            Ok(())
        })
        .expect("owner view");
    let child = store.begin().expect("begin linking");
    store
        .resolve_reference("library", "members", Some(2), "member")
        .expect("resolve exact item");
    store
        .truncate_values("library", "members", 1)
        .expect("truncate duplicate tail");
    store.rollback(&child).expect("restore unresolved tail");
    assert_eq!(
        store.value("library", "members").expect("members"),
        Some(json!(["member", "member", "?late"]))
    );
    assert_eq!(
        store
            .query(Query::Value("members", &json!("member")))
            .expect("duplicate membership"),
        vec!["library"]
    );
    store
        .resolve_reference("library", "members", Some(2), "member")
        .expect("resolve again");
    store.commit(&outer).expect("commit references");
    storage::atomic(&mut store, |store| store.remove("member")).expect("unload target");
    store
        .inspect_handle(&owner, &mut |object| {
            let collection = object.value("members").unwrap().collection().unwrap();
            assert_eq!(collection.len(), 3);
            assert!(collection.iter().all(|item| matches!(
                item.reference(),
                Some(ReferenceView::Unavailable { identity: "member" })
            )));
            Ok(())
        })
        .expect("owner view");
    assert!(store.inspect_handle(&original, &mut |_| Ok(())).is_err());
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "member", "Member", "reload", 0, 1)
    })
    .expect("reload target");
    let replacement = store.handle("member").expect("replacement");
    assert_ne!(original, replacement);
    store
        .inspect_handle(&owner, &mut |object| {
            let collection = object.value("members").unwrap().collection().unwrap();
            assert!(collection.iter().all(|item| matches!(
                item.reference(),
                Some(ReferenceView::Resolved(target)) if target == replacement
            )));
            Ok(())
        })
        .expect("owner view");
    assert_eq!(
        store
            .collection_page("library", "members", 1, 2)
            .expect("reference page"),
        vec![json!("member"), json!("member")]
    );
}

#[test]
fn exceptional_reference_items_remain_wire_data_and_typed_writes_reject_them() {
    let schema = schema();
    let metamodel = Arc::new(Metamodel::freeze(&schema).unwrap());
    let initial = json!(["unavailable", null, ["raw", null, [false, 1.0]], {"raw": [1, 2]}]);
    let mut row = Element::new(&schema, "library", "Library", "library", 0, 1).unwrap();
    row.values.insert("members".into(), initial.clone());
    let raw = Model {
        elements: vec![row],
        ..Model::default()
    };
    let wire = ModelView::new(&raw).unwrap();
    assert_eq!(
        wire.value("library", "members").unwrap(),
        Some(initial.clone())
    );
    for value in [
        json!("unavailable"),
        json!(["raw", null, [false, 1.0]]),
        json!({"raw": [1, 2]}),
    ] {
        assert_eq!(
            wire.query(Query::Value("members", &value)).unwrap(),
            ["library"]
        );
        assert!(
            wire.matches("library", Query::Value("members", &value))
                .unwrap()
        );
    }
    for policy in [
        IndexPolicy::AllValues,
        IndexPolicy::selected(std::iter::empty::<String>()),
    ] {
        let mut store = FrozenStore::with_index_policy(metamodel.clone(), policy);
        storage::atomic(&mut store, |store| {
            store.create_element(&schema, "library", "Library", "library", 0, 1)
        })
        .unwrap();
        let before = storage::export(&store).unwrap();
        assert!(
            storage::atomic(&mut store, |store| {
                store.set_value("library", "members", Some(initial.clone()))
            })
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
        let transaction = store.begin().unwrap();
        assert!(
            store
                .append_value("library", "members", json!(["nested", [null, "text"]]))
                .is_err()
        );
        store.rollback(&transaction).unwrap();
        assert_eq!(storage::export(&store).unwrap(), before);
    }
}

#[test]
fn collection_materialization_preserves_absent_default_and_null_on_rollback() {
    let schema = schema();
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |store| {
        for id in ["default", "absent", "null"] {
            store.create_element(&schema, id, "Library", "libraries", 0, 1)?;
        }
        store.set_value("absent", "members", None)?;
        store.set_value("null", "members", Some(json!(null)))
    })
    .expect("initial states");
    let transaction = store.begin().expect("begin materialization");
    store
        .append_value("default", "members", json!("target"))
        .expect("materialize default");
    store
        .append_value("absent", "members", json!("target"))
        .expect("materialize absent");
    assert!(
        store
            .append_value("null", "members", json!("target"))
            .is_err()
    );
    assert!(
        store
            .resolve_reference("default", "members", Some(2), "target")
            .is_err()
    );
    store
        .rollback(&transaction)
        .expect("rollback materialization");
    assert_eq!(
        store.value("default", "members").expect("default"),
        Some(json!([]))
    );
    assert_eq!(store.value("absent", "members").expect("absent"), None);
    assert_eq!(
        store.value("null", "members").expect("null"),
        Some(json!(null))
    );
}
