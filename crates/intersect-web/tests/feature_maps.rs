use serde_json::{Value, json};
const RCORE: &str = include_str!("../../../fixtures/feature-maps/features-structure.rcore");
fn request() -> Value {
    json!({"method":"compile_rcore","rcore":RCORE,"xtext":include_str!("../../../fixtures/feature-maps/features.xtext"),"rcore_behavior":include_str!("../../../fixtures/feature-maps/features-behavior.rcore"),"conversions":serde_json::from_str::<Value>(include_str!("../../../fixtures/feature-maps/conversions-rcore.json")).unwrap()})
}
fn dispatch(request: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
#[test]
fn feature_map_host_operations_model_rollback_export_and_static_types() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let entry_type = &input["conversions"][0]["datatype"];
    let map_type = &input["conversions"][1]["datatype"];
    let feature = &input["conversions"][0]["declarations"][0];
    let step = |datatype: &Value, operation: Value, value: Option<&Value>| {
        let mut req = json!({"method":"feature-map/step","package":package,"datatype":datatype,"operation":operation});
        if let Some(value) = value {
            req["value"] = value.clone();
        }
        dispatch(&req)
    };
    let entry = step(
        entry_type,
        json!({"kind":"entry","feature":feature,"value":"a"}),
        None,
    )
    .unwrap()["value"]
        .clone();
    let mut map = step(map_type, json!({"kind":"empty"}), None).unwrap()["value"].clone();
    for index in 0..2 {
        map = step(
            map_type,
            json!({"kind":"insert","index":index,"entry":entry["value"]}),
            Some(&map),
        )
        .unwrap()["value"]
            .clone();
    }
    assert_eq!(
        step(
            map_type,
            json!({"kind":"values","feature":feature}),
            Some(&map)
        )
        .unwrap()["result"],
        json!(["a", "a"])
    );
    let replacement = json!({"feature":feature,"value":"b"});
    let changed = step(
        map_type,
        json!({"kind":"set","index":1,"entry":replacement}),
        Some(&map),
    )
    .unwrap();
    assert_eq!(changed["result"], entry["value"]);
    let moved = step(
        map_type,
        json!({"kind":"move","new_position":0,"old_position":1}),
        Some(&changed["value"]),
    )
    .unwrap();
    assert_eq!(moved["result"], replacement);
    assert_eq!(
        step(
            map_type,
            json!({"kind":"remove","index":0}),
            Some(&moved["value"])
        )
        .unwrap()["result"],
        replacement
    );
    assert_eq!(
        step(map_type, json!({"kind":"clear"}), Some(&map)).unwrap()["value"]["value"],
        json!({"entries":[]})
    );
    for op in [
        json!({"kind":"insert","index":99,"entry":entry["value"]}),
        json!({"kind":"insert","index":0,"entry":{"feature":feature}}),
        json!({"kind":"clear","extra":1}),
        json!({"kind":"entry","feature":feature,"value":"a"}),
    ] {
        assert!(step(map_type, op, Some(&map)).is_err());
    }
    assert!(step(entry_type, json!({"kind":"empty"}), None).is_err());
    assert!(step(entry_type, json!({"kind":"entry","feature":feature}), None).is_err());
    assert!(step(map_type, json!({"kind":"clear"}), Some(&entry)).is_err());
    let evaluated=dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"sample.features","text":"holder \"example\""}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    let id = &model["elements"][0]["id"];
    assert!(model["elements"][0]["values"]["entry"].is_null());
    dispatch(
        &json!({"method":"script/begin","package":package,"model":model,"revision":"features"}),
    )
    .unwrap();
    for (field, value) in [("entry", entry.clone()), ("map", map.clone())] {
        dispatch(&json!({"method":"script/set","revision":"features","id":id,"feature":field,"value":value})).unwrap();
    }
    let saved =
        dispatch(&json!({"method":"script/commit","revision":"features"})).unwrap()["model"]
            .clone();
    let mut bad = map.clone();
    bad["value"]["entries"][0]["value"] = json!(99);
    dispatch(&json!({"method":"script/begin","package":package,"model":saved,"revision":"bad"}))
        .unwrap();
    assert!(
        dispatch(
            &json!({"method":"script/set","revision":"bad","id":id,"feature":"map","value":bad})
        )
        .is_err()
    );
    assert!(dispatch(&json!({"method":"script/commit","revision":"bad"})).is_err());
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.features","release":"1","display_name":"Features","description":"Checked entries"});
    let exported = dispatch(&export).unwrap();
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    assert_eq!(
        loaded["schema"]["conversion_contracts"],
        package["schema"]["conversion_contracts"]
    );
    dispatch(&json!({"method":"script/begin","package":loaded,"model":saved,"revision":"reload"}))
        .unwrap();
    dispatch(&json!({"method":"script/commit","revision":"reload"})).unwrap();
    let operation = &package["schema"]["classes"]["Holder"]["operations"][0];
    assert_eq!(
        operation["result"]["target"]["name"],
        "__standard_EFeatureMapEntry"
    );
    assert_eq!(
        operation["parameters"][0]["value"]["target"]["name"],
        "__standard_EFeatureMap"
    );
}
#[test]
fn feature_catalog_rejects_wrong_scope_policy_provider_and_reference_context() {
    for mutation in 0..6 {
        let mut input = request();
        match mutation {
            0 => input["conversions"][0]["members"][0]["value"]["codec"] = json!("EInt"),
            1 => {
                input["conversions"][0]["declarations"][0]["source"]["sha256"] =
                    json!("0".repeat(64))
            }
            2 => input["conversions"][0]["provider"]["name"] = json!("ecore.feature-map"),
            3 => {
                input["conversions"][0]["declarations"] = json!([{"source":input["conversions"][0]["declarations"][0]["source"],"path":[3,4]}]);
                input["conversions"][0]["members"] =
                    json!([{"nullable":true,"value":{"kind":"builtin","codec":"EString"}}]);
            }
            4 => input["conversions"][0]["members"] = json!([]),
            _ => {
                input["conversions"].as_array_mut().unwrap().remove(0);
            }
        }
        let result = dispatch(&input).unwrap();
        assert_eq!(result["ok"], false, "{result}");
    }
}

#[test]
fn rcore_behavior_feature_map_reads_preserve_nominal_type_boundaries() {
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    for (body, ok) in [
        (
            "fn same(a: f.Holder,b: f.Holder) -> Bool = a.entry == b.entry;",
            true,
        ),
        (
            "fn wrong(a: f.Holder) -> Bool = a.entry == a.localEntry;",
            false,
        ),
        ("fn wrong(a: f.Holder) -> Bool = a.entry == a.map;", false),
    ] {
        let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, RCORE);
        let mut semantic = DefinitionSource::new(
            "check-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!("rcore 1; package checks version \"1\"; import model \"urn:fm\" as f; {body}"),
        );
        semantic.dependencies.push(model.id.clone());
        let checked=dispatch(&json!({"method":"rcore_behavior/check","entry":semantic.id,"bundle":{"version":1,"sources":[model,semantic],"namespaces":[]},"conversions":request()["conversions"]})).unwrap();
        assert_eq!(checked["ok"], ok, "{checked}");
        assert_eq!(checked["executable"], false);
    }
}
