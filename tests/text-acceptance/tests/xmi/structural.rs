use super::Store;
use intersect_model::core::{
    api::{BasicModel, ModelEdit, TransactionalModel},
    interchange::ModelFormat,
    model::Model,
    query::QueryBudget,
    storage::{
        self, ContainmentDestination, DeleteRequest, DeleteSelection, IncomingReferences,
        ModelRead, Query, ReparentRequest,
    },
};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use serde_json::json;

fn fixture() -> (Schema, BasicModel<Store>) {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        include_str!("../../../../fixtures/ecore-structure/deletion.ecore"),
    )
    .unwrap();
    let mut model = Model {
        schema_revision: schema.revision.clone(),
        resources: Default::default(),
        elements: vec![],
    };
    for (id, resource) in [
        ("a", "one.xmi"),
        ("b", "one.xmi"),
        ("c", "one.xmi"),
        ("d", "two.xmi"),
    ] {
        model.create(&schema, id, "Node", resource, 10, 20).unwrap();
    }
    model
        .set_with_opposite(&schema, "a", "children", json!(["b"]))
        .unwrap();
    model
        .set_with_opposite(&schema, "b", "children", json!(["c"]))
        .unwrap();
    model
        .set_with_opposite(&schema, "d", "link", json!("c"))
        .unwrap();
    // Retained no-op construction evidence is metadata, not execution authority.
    let source = intersect_model::definitions::bundle::DefinitionSource::new(
        "test:retained-construction",
        intersect_model::definitions::bundle::SourceKind::RcoreBehavior,
        "no-op",
    );
    for element in &mut model.elements {
        element.initialization = Some(
            intersect_model::definitions::initialization::InitializationReceipt {
                object: element.id.clone(),
                authored_members: vec![],
                contract:
                    intersect_model::definitions::initialization::ClassInitializationContract {
                        version: 1,
                        schema_revision: schema.revision.clone(),
                        class: "Node".into(),
                        class_identity: schema.identity("Node"),
                        provider: intersect_model::definitions::conversion::ProviderId {
                            name: "retained-test".into(),
                            version: "1".into(),
                        },
                        sources: vec![source.id.clone()],
                        members: vec![],
                    },
            },
        );
    }
    let store = crate::stores::from_model::<Store>(&schema, &model).unwrap();
    let basic = BasicModel::from_store(schema.clone(), store).unwrap();
    (schema, basic)
}
fn roundtrip(schema: &Schema, model: &Model) {
    let json_model: Model = serde_json::from_str(&serde_json::to_string(model).unwrap()).unwrap();
    assert_eq!(&json_model, model);
    let resources = XmiFormat.encode(schema, model).unwrap();
    let restored = XmiFormat.decode(schema, &resources).unwrap();
    assert_eq!(&restored, model);
    let store = crate::stores::from_model::<Store>(schema, &restored).unwrap();
    // Independent enumeration checks persisted class/resource/value indexes.
    for element in &restored.elements {
        let expected = restored
            .elements
            .iter()
            .filter(|e| e.resource == element.resource)
            .map(|e| e.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            store.query(Query::Resource(&element.resource)).unwrap(),
            expected
        );
        for (name, value) in &element.values {
            if value.is_array() || value.is_null() {
                continue;
            }
            let expected = restored
                .elements
                .iter()
                .filter(|e| e.values.get(name) == Some(value))
                .map(|e| e.id.clone())
                .collect::<Vec<_>>();
            assert_eq!(store.query(Query::Value(name, value)).unwrap(), expected);
        }
    }
    assert_eq!(
        store.query(Query::Class("Node")).unwrap(),
        restored
            .elements
            .iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>()
    );
}
store_test! {
pub fn structural_moves_and_deletes_roundtrip_history_provenance_and_indexes() {
    let (schema, mut basic) = fixture();
    basic
        .reparent(
            ReparentRequest {
                object: "c",
                destination: Some(ContainmentDestination {
                    owner: "a",
                    feature: "children",
                    position: Some(0),
                }),
            },
            None,
            &mut QueryBudget::new(10_000),
        )
        .unwrap();
    assert_eq!(
        basic.value("a", "children").unwrap(),
        Some(json!(["c", "b"]))
    );
    roundtrip(&schema, &storage::export(&basic).unwrap());
    let before = storage::export(&basic).unwrap();
    let mut managed = TransactionalModel::new(basic);
    let delete = DeleteRequest {
        roots: &["c"],
        selection: DeleteSelection::Objects,
        incoming: IncomingReferences::Detach,
    };
    assert!(
        managed
            .transaction(|edit| {
                edit.delete_objects(delete, None, &mut QueryBudget::new(10_000))?;
                Err::<(), String>("rollback".into())
            })
            .is_err()
    );
    assert_eq!(storage::export(&managed).unwrap(), before);
    roundtrip(&schema, &before);
    managed
        .transaction(|edit| edit.delete_objects(delete, None, &mut QueryBudget::new(10_000)))
        .unwrap();
    let after = storage::export(&managed).unwrap();
    assert!(managed.get("c").unwrap().is_none());
    assert!(!managed.into_basic().is_set("d", "link").unwrap());
    roundtrip(&schema, &after);
}
}
store_test! {
pub fn cross_resource_move_retains_json_identity_and_explicit_xmi_boundary() {
    let (schema, mut basic) = fixture();
    let before = storage::export(&basic).unwrap();
    basic
        .reparent(
            ReparentRequest {
                object: "b",
                destination: Some(ContainmentDestination {
                    owner: "d",
                    feature: "children",
                    position: None,
                }),
            },
            None,
            &mut QueryBudget::new(10_000),
        )
        .unwrap();
    let moved = storage::export(&basic).unwrap();
    for original in &before.elements {
        let current = moved.elements.iter().find(|e| e.id == original.id).unwrap();
        assert_eq!(
            (
                &current.resource,
                current.start,
                current.end,
                &current.type_identity
            ),
            (
                &original.resource,
                original.start,
                original.end,
                &original.type_identity
            )
        );
    }
    let json_model: Model = serde_json::from_str(&serde_json::to_string(&moved).unwrap()).unwrap();
    assert_eq!(json_model, moved);
    assert!(
        XmiFormat
            .encode(&schema, &moved)
            .unwrap_err()
            .contains("Cross-resource containment")
    );
    assert_eq!(storage::export(&basic).unwrap(), moved);
    basic
        .reparent(
            ReparentRequest {
                object: "b",
                destination: None,
            },
            None,
            &mut QueryBudget::new(10_000),
        )
        .unwrap();
    roundtrip(&schema, &storage::export(&basic).unwrap());
}
}

store_test! {
pub fn transient_volatile_containment_projects_without_changing_deletion_selection() {
    let source = include_str!("../../../../fixtures/ecore-structure/deletion.ecore").replace(
        "name=\"children\"",
        "name=\"children\" transient=\"true\" volatile=\"true\"",
    );
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&source).unwrap();
    let mut basic = crate::stores::basic::<Store>(schema.clone()).unwrap();
    for id in ["a", "b", "c"] {
        basic.create(id, "Node", "model.xmi").unwrap();
    }
    basic.set("a", "children", json!(["b"])).unwrap();
    basic.set("b", "children", json!(["c"])).unwrap();
    let before = storage::export(&basic).unwrap();
    let projected = XmiFormat
        .decode(&schema, &XmiFormat.encode(&schema, &before).unwrap())
        .unwrap();
    assert_eq!(
        projected
            .elements
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a"]
    );
    assert_eq!(storage::export(&basic).unwrap(), before);
    basic
        .reparent(
            ReparentRequest {
                object: "b",
                destination: None,
            },
            None,
            &mut QueryBudget::new(10_000),
        )
        .unwrap();
    let detached = storage::export(&basic).unwrap();
    let projected = XmiFormat
        .decode(&schema, &XmiFormat.encode(&schema, &detached).unwrap())
        .unwrap();
    assert_eq!(
        projected
            .elements
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    let removed = basic
        .delete_objects(
            DeleteRequest {
                roots: &["b"],
                selection: DeleteSelection::Subtrees,
                incoming: IncomingReferences::Reject,
            },
            None,
            &mut QueryBudget::new(10_000),
        )
        .unwrap();
    assert_eq!(removed, vec!["b", "c"]);
    assert!(basic.get("c").unwrap().is_none());
    let final_model = storage::export(&basic).unwrap();
    let final_projection = XmiFormat
        .decode(&schema, &XmiFormat.encode(&schema, &final_model).unwrap())
        .unwrap();
    assert_eq!(final_projection.elements.len(), 1);
    assert_eq!(final_projection.elements[0].id, "a");
}
}
