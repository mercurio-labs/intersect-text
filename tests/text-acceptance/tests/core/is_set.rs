use super::Store;
use intersect_model::core::{
    api::ModelEdit,
    model::Element,
    storage::{self, CollectionEdit, ModelRead, ModelStore},
};
use intersect_model::definitions::{
    dynamic::{ScalarType, SchemaBuilder},
    schema::Schema,
};
use serde_json::{Value, json};
fn schema(case: &Value) -> Schema {
    let mut builder = SchemaBuilder::new("p", "urn:p");
    let c = builder.class("Item").unwrap();
    let feature = c
        .attribute("value", ScalarType::String)
        .bounds(0, if case["many"] == true { -1 } else { 1 })
        .volatile(case["volatile"] == true)
        .transient(case["transient"] == true)
        .changeable(case["changeable"] == true)
        .unsettable(case["unsettable"] == true);
    if case["many"] == true {
        feature.build().unwrap();
    } else {
        feature.default("seed").unwrap();
    }
    builder.build().unwrap().as_schema().clone()
}
store_test! {
pub fn semantic_set_state_matches_all_supported_dynamic_scalar_and_many_transitions() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-feature-flags/emf-dynamic-observations.json"
    ))
    .unwrap();
    let mut checked = 0;
    for case in oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["derived"] == false)
    {
        let schema = schema(case);
        let mut model = crate::stores::basic::<Store>(schema).unwrap();
        model.create("a", "Item", "model").unwrap();
        for step in case["steps"].as_array().unwrap() {
            let many = case["many"] == true;
            let accepted = match step["operation"].as_str().unwrap() {
                "fresh" => true,
                "unset" | "unset-again" | "unset-default" => model.unset("a", "value").is_ok(),
                "clear-list" => model
                    .edit_collection("a", "value", CollectionEdit::Clear)
                    .is_ok(),
                "add-list" => model
                    .edit_collection("a", "value", CollectionEdit::Add(json!("next")))
                    .is_ok(),
                "remove-last" => model
                    .edit_collection("a", "value", CollectionEdit::Remove(json!("next")))
                    .is_ok(),
                "explicit-default" => model
                    .set("a", "value", if many { json!([]) } else { json!("seed") })
                    .is_ok(),
                "explicit-value" => model
                    .set(
                        "a",
                        "value",
                        if many {
                            json!(["custom"])
                        } else {
                            json!("custom")
                        },
                    )
                    .is_ok(),
                "explicit-null" => model.set("a", "value", Value::Null).is_ok(),
                _ => panic!("Unknown operation"),
            };
            assert_eq!(json!(accepted), step["accepted"]);
            assert_eq!(model.value("a", "value").unwrap().unwrap(), step["value"]);
            assert_eq!(
                json!(model.is_set("a", "value").unwrap()),
                step["is_set"],
                "{} {}",
                case["mask"],
                step["operation"]
            );
            model
                .inspect("a", &mut |object| {
                    assert!(object.is_set("value"));
                    Ok(())
                })
                .unwrap();
            checked += 1;
        }
    }
    assert_eq!(checked, 256);
}
}
store_test! {
pub fn missing_stale_foreign_and_computed_state_are_explicit_errors() {
    let schema = schema(&json!({"changeable":true}));
    let mut store = crate::stores::store::<Store>(&schema).unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(&schema, "a", "Item", "model", 0, 0)
    })
    .unwrap();
    assert!(storage::is_set(&store, &schema, "missing", "value").is_err());
    assert!(storage::is_set(&store, &schema, "a", "missing").is_err());
    let mut stale = schema.clone();
    stale.revision.push('x');
    assert!(
        storage::is_set(&store, &stale, "a", "value")
            .unwrap_err()
            .contains("revision")
    );
    let mut object = store.get("a").unwrap().unwrap();
    object.type_identity = "foreign".into();
    let mut forged = storage::export(&store).unwrap();
    forged.elements[0] = object.clone();
    let error = storage::atomic(&mut store, |s| s.put(object)).unwrap_err();
    assert!(error.contains("identity"));
    let raw = storage::ModelView::new(&forged).unwrap();
    assert!(
        storage::is_set(&raw, &schema, "a", "value")
            .unwrap_err()
            .contains("identity")
    );
    let mut object = Element::new(&schema, "a", "Item", "model", 0, 0).unwrap();
    object.values.remove("value");
    storage::atomic(&mut store, |s| s.put(object)).unwrap();
    assert!(
        storage::is_set(&store, &schema, "a", "value")
            .unwrap_err()
            .contains("effective value")
    );
    let mut derived = schema.clone();
    derived
        .classes
        .get_mut("Item")
        .unwrap()
        .features
        .get_mut("value")
        .unwrap()
        .derived = true;
    assert!(
        storage::is_set(&store, &derived, "a", "value")
            .unwrap_err()
            .contains("semantic provider")
    );
    assert!(
        storage::is_explicitly_set(&store, &schema, "a", "value")
            .unwrap_err()
            .contains("not unsettable")
    );
}
}
