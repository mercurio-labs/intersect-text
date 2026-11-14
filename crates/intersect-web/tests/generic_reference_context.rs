#[allow(dead_code)]
#[path = "../../../tests/text-acceptance/tests/language/common/generic_reference_context.rs"]
mod common;
use serde_json::{Value, json};
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
#[test]
fn host_collection_references_preserve_runtime_targets_and_embedded_context() {
    let fixture = common::fixture();
    let compiled = call(
        json!({"method":"compile_bundle", "compilation":fixture.input,
        "conversions":fixture.ordinary, "applied_conversions":fixture.applications}),
    )
    .unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let selection =
        json!({"package":package,"application":fixture.applications[0].application,"member":0});
    let bind = |selection: &Value, element: &str| {
        let mut request = selection.clone();
        request["method"] = json!("collection/reference-bind");
        request["element"] = json!(element);
        call(request)
    };
    let read = |selection: &Value, reference: &Value| {
        let mut request = selection.clone();
        request["method"] = json!("collection/reference-read");
        request["reference"] = reference.clone();
        call(request)
    };
    let value = bind(&selection, "value").unwrap();
    let subtype = bind(&selection, "sub").unwrap();
    assert_eq!(value["context"], fixture.context.digest().unwrap());
    let expected = fixture.context.model["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|element| element["id"] == "value")
        .unwrap();
    assert_eq!(read(&selection, &value).unwrap(), *expected);
    assert_eq!(read(&selection, &subtype).unwrap()["id"], "sub");
    for invalid in ["other", "raw", "plain", "missing"] {
        assert!(bind(&selection, invalid).is_err());
    }
    let raw_selection =
        json!({"package":package,"datatype":fixture.ordinary[0].datatype,"member":0});
    let raw = bind(&raw_selection, "raw").unwrap();
    assert_eq!(bind(&raw_selection, "value").unwrap(), value);
    assert!(read(&selection, &raw).is_err());
    let plain_selection =
        json!({"package":package,"datatype":fixture.ordinary[1].datatype,"member":0});
    let plain = bind(&plain_selection, "plain").unwrap();
    assert_eq!(read(&plain_selection, &plain).unwrap()["id"], "plain");
    assert!(bind(&plain_selection, "value").is_err());
    for member in [json!(1), json!(-1), Value::Null] {
        let mut invalid = selection.clone();
        invalid["member"] = member;
        assert!(bind(&invalid, "value").is_err());
    }
    let mut invalid = selection.clone();
    invalid["datatype"] = json!(fixture.ordinary[0].datatype);
    assert!(bind(&invalid, "value").is_err());
    let mut forged = value.clone();
    forged["context"] = json!("stale");
    assert!(read(&selection, &forged).is_err());
    let step = |index: usize, operation: Value, collection: Option<&Value>| {
        let mut request = json!({"method":"collection/step","package":package,
            "application":fixture.applications[index].application,"operation":operation});
        if let Some(collection) = collection {
            request["collection"] = collection.clone();
        }
        call(request)
    };
    let empty = step(0, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    let list = step(
        0,
        json!({"kind":"insert","index":0,"item":value}),
        Some(&empty),
    )
    .unwrap()["collection"]
        .clone();
    assert!(
        step(
            0,
            json!({"kind":"insert","index":1,"item":raw}),
            Some(&list)
        )
        .is_err()
    );
    assert_eq!(
        step(0, json!({"kind":"remove_at","index":0}), Some(&list)).unwrap()["result"]["value"],
        value
    );
    let map = step(1, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    let map = step(
        1,
        json!({"kind":"put","key":value,"item":subtype}),
        Some(&map),
    )
    .unwrap()["collection"]
        .clone();
    assert_eq!(
        step(1, json!({"kind":"get","key":value}), Some(&map)).unwrap()["result"]["value"],
        subtype
    );
    let retained: Value = serde_json::from_str(&serde_json::to_string(package).unwrap()).unwrap();
    let mut replay = selection.clone();
    replay["package"] = retained;
    assert_eq!(read(&replay, &value).unwrap(), *expected);
}

#[test]
fn selected_materialization_preserves_reference_context_revision_and_replay() {
    let fixture = common::fixture();
    let context = &fixture.context;
    let request = json!({"method":"ecore/materialize","bundle":context.bundle,
        "entry":context.entry,"namespace":context.namespace,"types":context.materialized_types,
        "conversions":context.conversions,"applied_conversions":context.applied_conversions,
        "constructors":context.constructors,"operation_applications":[]});
    let result = call(request.clone()).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    let expected = intersect_model::conversion::with_catalog_policies(
        &context.bundle,
        &context.conversions,
        &context.applied_conversions,
        &context.constructors,
        |registry| {
            intersect_model::definitions::compiler::materialize_runtime_bundle(
                &context.bundle,
                &context.entry,
                &context.namespace,
                &context.materialized_types,
                Some(registry),
            )
            .map_err(|error| error.to_string())
        },
    )
    .unwrap();
    assert_eq!(result["schema"], json!(expected));
    assert_eq!(call(result["operation_replay"].clone()).unwrap(), result);
    for missing in ["entry", "namespace"] {
        let mut invalid = request.clone();
        invalid.as_object_mut().unwrap().remove(missing);
        assert!(call(invalid).is_err());
    }
    let mut stale = request.clone();
    stale["entry"]["sha256"] = json!("0".repeat(64));
    assert_eq!(call(stale).unwrap()["ok"], false);
    let mut wrong = request;
    wrong["namespace"] = json!("urn:missing");
    assert_eq!(call(wrong).unwrap()["ok"], false);
}
