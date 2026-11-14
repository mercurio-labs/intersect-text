use super::Store;
use intersect_model::core::{
    model::Element,
    storage::{self, ModelRead, ModelStore},
};
use intersect_model::definitions::dynamic::{ScalarType, SchemaBuilder};
use serde_json::json;

control_test! {
pub fn readonly_defaults_and_inherited_flags_agree_in_source_and_builder() {
    let source = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="p" nsURI="urn:p"><eClassifiers x:type="e:EClass" name="Base"><eStructuralFeatures x:type="e:EAttribute" name="value" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString" changeable="false" unsettable="true" defaultValueLiteral="seed"/></eClassifiers><eClassifiers x:type="e:EClass" name="Item" eSuperTypes="#//Base"/></e:EPackage>"##;
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(source).unwrap();
    let mut builder = SchemaBuilder::new("p", "urn:p");
    let base = builder.class("Base").unwrap();
    base.attribute("value", ScalarType::String)
        .changeable(false)
        .unsettable(true)
        .default("seed")
        .unwrap();
    builder.class("Item").unwrap().extends(&base).unwrap();
    assert_eq!(
        schema.features("Item").unwrap(),
        builder
            .build()
            .unwrap()
            .as_schema()
            .features("Item")
            .unwrap()
    );
    assert_eq!(
        Element::new(&schema, "a", "Item", "model", 0, 0)
            .unwrap()
            .values["value"],
        json!("seed")
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
            &source.replace("changeable=\"false\"", "changeable=\"invalid\"")
        )
        .is_err()
    );
    let normal = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        &source.replace("changeable=\"false\"", ""),
    )
    .unwrap();
    let field = normal.features("Item").unwrap()["value"].clone();
    let wire = serde_json::to_value(&field).unwrap();
    assert!(wire.get("changeable").is_none());
    assert!(
        serde_json::from_value::<intersect_model::definitions::schema::Feature>(wire)
            .unwrap()
            .changeable
    );
    assert!(
        !serde_json::from_value::<intersect_model::definitions::schema::Feature>(
            serde_json::to_value(&schema.features("Item").unwrap()["value"]).unwrap()
        )
        .unwrap()
        .changeable
    );
}
}

store_test! {
pub fn reflective_scalar_many_and_reference_writes_reset_and_late_failure_are_atomic() {
    for many in [false, true] {
        for reference in [false, true] {
            let mut builder = SchemaBuilder::new("p", "urn:p");
            let class = builder.class("Item").unwrap();
            class
                .attribute("editable", ScalarType::String)
                .unsettable(true)
                .build()
                .unwrap();
            let field = if reference {
                class.reference("value", &class)
            } else {
                class.attribute("value", ScalarType::String)
            };
            field
                .bounds(0, if many { -1 } else { 1 })
                .changeable(false)
                .unsettable(true)
                .build()
                .unwrap();
            let schema = builder.build().unwrap().as_schema().clone();
            let mut store = crate::stores::store::<Store>(&schema).unwrap();
            storage::atomic(&mut store, |s| {
                s.create_element(&schema, "a", "Item", "model", 0, 0)?;
                s.create_element(&schema, "b", "Item", "model", 0, 0)
            })
            .unwrap();
            let before = storage::export(&store).unwrap();
            let atom = if reference {
                json!("b")
            } else {
                json!("changed")
            };
            let value = if many { json!([atom]) } else { atom };
            let error = storage::edit(
                &mut store,
                &schema,
                &[
                    ("a".into(), "editable".into(), json!("first")),
                    ("a".into(), "value".into(), value),
                ],
            )
            .unwrap_err();
            assert!(error.contains("non-changeable"));
            assert_eq!(storage::export(&store).unwrap(), before);
            assert!(
                storage::unset(&mut store, &schema, "a", "value")
                    .unwrap_err()
                    .contains("non-changeable")
            );
            assert_eq!(storage::export(&store).unwrap(), before);
            assert!(!storage::is_explicitly_set(&store, &schema, "a", "value").unwrap());
            assert_eq!(store.value("a", "editable").unwrap(), Some(json!(null)));
        }
    }
}
}

store_test! {
pub fn reflective_rejection_replays_independent_emf_prefix_before_collection_view_edits() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-feature-flags/emf-dynamic-observations.json"
    ))
    .unwrap();
    let mut checked = 0;
    for case in oracle["cases"].as_array().unwrap().iter().filter(|c| {
        c["changeable"] == false
            && c["derived"] == false
            && c["volatile"] == false
            && c["transient"] == false
    }) {
        let many = case["many"] == true;
        let unsettable = case["unsettable"] == true;
        let mut builder = SchemaBuilder::new("p", "urn:p");
        let class = builder.class("Item").unwrap();
        let feature = class
            .attribute("value", ScalarType::String)
            .bounds(0, if many { -1 } else { 1 })
            .changeable(false)
            .unsettable(unsettable);
        if many {
            feature.build().unwrap();
        } else {
            feature.default("seed").unwrap();
        }
        let schema = builder.build().unwrap().as_schema().clone();
        let mut store = crate::stores::store::<Store>(&schema).unwrap();
        storage::atomic(&mut store, |s| {
            s.create_element(&schema, "a", "Item", "model", 0, 0)
        })
        .unwrap();
        // The later eGet-list mutations belong to the separate collection-view contract.
        for step in case["steps"].as_array().unwrap().iter().take(4) {
            let accepted = match step["operation"].as_str().unwrap() {
                "fresh" => true,
                "unset-default" => storage::unset(&mut store, &schema, "a", "value").is_ok(),
                operation => {
                    let value = if operation == "explicit-value" {
                        if many {
                            json!(["custom"])
                        } else {
                            json!("custom")
                        }
                    } else {
                        if many { json!([]) } else { json!("seed") }
                    };
                    storage::edit(&mut store, &schema, &[("a".into(), "value".into(), value)])
                        .is_ok()
                }
            };
            assert_eq!(json!(accepted), step["accepted"]);
            assert_eq!(store.value("a", "value").unwrap().unwrap(), step["value"]);
            if unsettable {
                assert_eq!(
                    json!(storage::is_explicitly_set(&store, &schema, "a", "value").unwrap()),
                    step["is_set"]
                );
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 16);
}
}

store_test! {
pub fn transient_declarations_preserve_runtime_values_history_and_snapshot_transport() {
    let source = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="p" nsURI="urn:p"><eClassifiers x:type="e:EClass" name="Base"><eStructuralFeatures x:type="e:EAttribute" name="value" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString" transient="true" unsettable="true" defaultValueLiteral="seed"/></eClassifiers><eClassifiers x:type="e:EClass" name="Item" eSuperTypes="#//Base"/></e:EPackage>"##;
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(source).unwrap();
    let mut builder = SchemaBuilder::new("p", "urn:p");
    let base = builder.class("Base").unwrap();
    base.attribute("value", ScalarType::String)
        .transient(true)
        .unsettable(true)
        .default("seed")
        .unwrap();
    builder.class("Item").unwrap().extends(&base).unwrap();
    assert_eq!(
        schema.features("Item").unwrap(),
        builder
            .build()
            .unwrap()
            .as_schema()
            .features("Item")
            .unwrap()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
            &source.replace("transient=\"true\"", "transient=\"invalid\"")
        )
        .is_err()
    );
    let legacy = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        &source.replace("transient=\"true\"", ""),
    )
    .unwrap();
    assert!(!legacy.features("Item").unwrap()["value"].transient);
    assert!(
        serde_json::to_value(&legacy.features("Item").unwrap()["value"])
            .unwrap()
            .get("transient")
            .is_none()
    );
    let mut store = crate::stores::store::<Store>(&schema).unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(&schema, "a", "Item", "model", 0, 0)
    })
    .unwrap();
    storage::edit(
        &mut store,
        &schema,
        &[("a".into(), "value".into(), json!("runtime"))],
    )
    .unwrap();
    let original = storage::export(&store).unwrap();
    assert!(storage::is_set(&store, &schema, "a", "value").unwrap());
    let restored: intersect_model::core::model::Model =
        serde_json::from_value(serde_json::to_value(&original).unwrap()).unwrap();
    assert_eq!(restored, original);
    assert_eq!(restored.elements[0].values["value"], json!("runtime"));
    let outer = store.begin().unwrap();
    storage::unset(&mut store, &schema, "a", "value").unwrap();
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), original);
}
}

store_test! {
pub fn volatile_declarations_preserve_runtime_values_history_and_snapshot_transport() {
    let source = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="p" nsURI="urn:p"><eClassifiers x:type="e:EClass" name="Base"><eStructuralFeatures x:type="e:EAttribute" name="value" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString" volatile="true" unsettable="true" defaultValueLiteral="seed"/></eClassifiers><eClassifiers x:type="e:EClass" name="Item" eSuperTypes="#//Base"/></e:EPackage>"##;
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(source).unwrap();
    let mut builder = SchemaBuilder::new("p", "urn:p");
    let base = builder.class("Base").unwrap();
    base.attribute("value", ScalarType::String)
        .volatile(true)
        .unsettable(true)
        .default("seed")
        .unwrap();
    builder.class("Item").unwrap().extends(&base).unwrap();
    assert_eq!(
        schema.features("Item").unwrap(),
        builder
            .build()
            .unwrap()
            .as_schema()
            .features("Item")
            .unwrap()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
            &source.replace("volatile=\"true\"", "volatile=\"invalid\"")
        )
        .is_err()
    );
    let legacy = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        &source.replace("volatile=\"true\"", ""),
    )
    .unwrap();
    assert!(!legacy.features("Item").unwrap()["value"].volatile);
    assert!(
        serde_json::to_value(&legacy.features("Item").unwrap()["value"])
            .unwrap()
            .get("volatile")
            .is_none()
    );
    let mut store = crate::stores::store::<Store>(&schema).unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(&schema, "a", "Item", "model", 0, 0)
    })
    .unwrap();
    storage::edit(
        &mut store,
        &schema,
        &[("a".into(), "value".into(), json!("runtime"))],
    )
    .unwrap();
    let original = storage::export(&store).unwrap();
    assert!(storage::is_set(&store, &schema, "a", "value").unwrap());
    let restored: intersect_model::core::model::Model =
        serde_json::from_value(serde_json::to_value(&original).unwrap()).unwrap();
    assert_eq!(restored, original);
    assert_eq!(restored.elements[0].values["value"], json!("runtime"));
    let outer = store.begin().unwrap();
    storage::unset(&mut store, &schema, "a", "value").unwrap();
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), original);
}
}
