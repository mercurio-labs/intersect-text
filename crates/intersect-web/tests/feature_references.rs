use serde_json::{Value, json};
fn request() -> Value {
    json!({"method":"compile_rcore","rcore":include_str!("../../../fixtures/feature-references/references-structure.rcore"),"xtext":include_str!("../../../fixtures/feature-references/references.xtext"),"rcore_behavior":include_str!("../../../fixtures/feature-references/references-behavior.rcore"),"conversions":serde_json::from_str::<Value>(include_str!("../../../fixtures/feature-references/conversions-rcore.json")).unwrap()})
}
fn dispatch(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
#[test]
#[cfg_attr(not(feature = "milestone-tests"), ignore = "milestone")]
fn reference_entries_roundtrip_through_host_transactions_and_export() {
    let input = request();
    let compiled = dispatch(input.clone()).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let contracts = &input["conversions"];
    let entry_type = &contracts[0]["datatype"];
    let map_type = &contracts[1]["datatype"];
    let feature = &contracts[0]["declarations"][0];
    let context = &contracts[0]["members"][0]["value"]["context"];
    // The retained snapshot is executable fixture data, not self-asserted type metadata.
    assert_eq!(context["bundle"]["sources"][0]["text"], input["rcore"]);
    let exact = context["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["values"]["name"] == "dune")
        .unwrap();
    let unrelated = context["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["values"]["name"] == "ada")
        .unwrap();
    let mut references = Vec::new();
    for element in [exact["id"].as_str().unwrap(), "special"] {
        let reference = dispatch(json!({"method":"feature-map/reference-bind","package":package,"datatype":entry_type,"feature":feature,"element":element})).unwrap();
        let object = dispatch(json!({"method":"feature-map/reference-read","package":package,"datatype":map_type,"feature":feature,"reference":reference})).unwrap();
        assert_eq!(
            object,
            *context["model"]["elements"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["id"] == element)
                .unwrap()
        );
        references.push(reference);
    }
    for element in ["missing", unrelated["id"].as_str().unwrap()] {
        assert!(dispatch(json!({"method":"feature-map/reference-bind","package":package,"datatype":entry_type,"feature":feature,"element":element})).is_err());
    }
    for mutation in 0..4 {
        let mut wire = references[1].clone();
        match mutation {
            0 => wire["context"] = json!("stale"),
            1 => wire["element"] = unrelated["id"].clone(),
            2 => wire["class"] = json!("BookCopy"),
            _ => wire = Value::Null,
        }
        assert!(dispatch(json!({"method":"feature-map/reference-read","package":package,"datatype":entry_type,"feature":feature,"reference":wire})).is_err());
    }
    let mut wrong_feature = feature.clone();
    wrong_feature["path"] = json!([4, 1]);
    assert!(dispatch(json!({"method":"feature-map/reference-bind","package":package,"datatype":entry_type,"feature":wrong_feature,"element":"special"})).is_err());
    let entry = dispatch(json!({"method":"feature-map/step","package":package,"datatype":entry_type,"operation":{"kind":"entry","feature":feature,"value":references[1]}})).unwrap()["value"].clone();
    let mut map = dispatch(json!({"method":"feature-map/step","package":package,"datatype":map_type,"operation":{"kind":"empty"}})).unwrap()["value"].clone();
    for (index, reference) in references.iter().enumerate() {
        map = dispatch(json!({"method":"feature-map/step","package":package,"datatype":map_type,"value":map,"operation":{"kind":"insert","index":index,"entry":{"feature":feature,"value":reference}}})).unwrap()["value"].clone();
    }
    let evaluated = dispatch(json!({"method":"evaluate","package":package,"sources":[{"uri":"sample.library","text":include_str!("../../../fixtures/library/catalog.library")}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    let id = model["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["values"]["name"] == "earthsea")
        .unwrap()["id"]
        .clone();
    dispatch(
        json!({"method":"script/begin","package":package,"model":model,"revision":"references"}),
    )
    .unwrap();
    for (field, value) in [("entry", &entry), ("featureMap", &map)] {
        dispatch(json!({"method":"script/set","revision":"references","id":id,"feature":field,"value":value})).unwrap();
    }
    let saved =
        dispatch(json!({"method":"script/commit","revision":"references"})).unwrap()["model"]
            .clone();
    let mut bad = map.clone();
    bad["value"]["entries"][0]["value"]["context"] = json!("stale");
    dispatch(
        json!({"method":"script/begin","package":package,"model":saved,"revision":"bad-reference"}),
    )
    .unwrap();
    dispatch(json!({"method":"script/set","revision":"bad-reference","id":id,"feature":"entry","value":null})).unwrap();
    assert!(dispatch(json!({"method":"script/set","revision":"bad-reference","id":id,"feature":"featureMap","value":bad})).is_err());
    assert!(dispatch(json!({"method":"script/commit","revision":"bad-reference"})).is_err());
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.references","release":"1","display_name":"References","description":"Checked object references"});
    let exported = dispatch(export).unwrap();
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    dispatch(json!({"method":"script/begin","package":loaded,"model":saved,"revision":"reload-reference"})).unwrap();
    assert_eq!(
        dispatch(json!({"method":"script/commit","revision":"reload-reference"})).unwrap()["model"],
        saved
    );
    assert_eq!(dispatch(json!({"method":"feature-map/reference-read","package":loaded,"datatype":map_type,"feature":feature,"reference":references[1]})).unwrap()["class"], "SpecialCopy");
}
