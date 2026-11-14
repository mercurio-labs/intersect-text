#[path = "support/mod.rs"]
mod support;
use serde_json::{Value, json};
const RCORE: &str = include_str!("../../../fixtures/tree-iterators/iterators-structure.rcore");
fn request() -> Value {
    json!({"method":"compile_rcore","rcore":RCORE,"xtext":include_str!("../../../fixtures/tree-iterators/iterators.xtext"),"rcore_behavior":include_str!("../../../fixtures/tree-iterators/iterators-behavior.rcore"),"conversions":serde_json::from_str::<Value>(include_str!("../../../fixtures/tree-iterators/conversions-rcore.json")).unwrap()})
}
fn dispatch(request: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn step(
    package: &Value,
    datatype: &Value,
    action: &str,
    cursor: Option<&Value>,
) -> Result<Value, String> {
    let mut r =
        json!({"method":"iterator/step","package":package,"datatype":datatype,"action":action});
    if let Some(cursor) = cursor {
        r["cursor"] = cursor.clone();
    }
    dispatch(&r)
}
#[test]
fn typed_cursors_traverse_prune_publish_reload_and_preserve_operation_identity() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let datatype = &input["conversions"][0]["datatype"];
    let operation = &package["schema"]["classes"]["Session"]["operations"][0];
    let target = json!({"kind":"datatype","name":"__standard_ETreeIterator"});
    assert_eq!(operation["result"]["target"], target);
    assert_eq!(operation["parameters"][0]["value"]["target"], target);
    let start = step(package, datatype, "start", None).unwrap();
    assert_eq!(start["has_next"], true);
    assert_eq!(
        step(package, datatype, "has_next", Some(&start["cursor"])).unwrap(),
        start
    );
    let mut cursor = start["cursor"].clone();
    let mut paths = vec![];
    loop {
        let result = step(package, datatype, "next", Some(&cursor)).unwrap();
        cursor = result["cursor"].clone();
        if result["item"].is_null() {
            break;
        }
        paths.push(result["item"]["path"].clone());
    }
    assert_eq!(
        paths,
        json!([[0], [0, 0], [0, 1], [0, 2], [0, 3], [0, 3, 0]])
            .as_array()
            .unwrap()
            .clone()
    );
    let first = step(package, datatype, "next", Some(&start["cursor"])).unwrap();
    let pruned = step(package, datatype, "prune", Some(&first["cursor"])).unwrap();
    assert_eq!(pruned["has_next"], false);
    assert_eq!(
        step(package, datatype, "next", Some(&pruned["cursor"])).unwrap()["cursor"],
        pruned["cursor"]
    );
    let evaluated=dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"sample.iterators","text":"session example"}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    let id = &model["elements"][0]["id"];
    assert!(model["elements"][0]["values"]["cursor"].is_null());
    let local = step(package, &input["conversions"][1]["datatype"], "start", None).unwrap();
    dispatch(&json!({"method":"script/begin","package":package,"model":model,"revision":"1"}))
        .unwrap();
    for (feature, value) in [
        ("cursor", first["cursor"].clone()),
        ("localCursor", local["cursor"].clone()),
    ] {
        dispatch(
            &json!({"method":"script/set","revision":"1","id":id,"feature":feature,"value":value}),
        )
        .unwrap();
    }
    let saved =
        dispatch(&json!({"method":"script/commit","revision":"1"})).unwrap()["model"].clone();
    for bad in [local["cursor"].clone(), {
        let mut bad = first["cursor"].clone();
        bad["value"]["position"] = json!(99);
        bad
    }] {
        dispatch(&json!({"method":"script/begin","package":package,"model":saved,"revision":"2"}))
            .unwrap();
        assert!(dispatch(&json!({"method":"script/set","revision":"2","id":id,"feature":"cursor","value":bad})).is_err());
        assert!(dispatch(&json!({"method":"script/commit","revision":"2"})).is_err());
    }
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    for (body, ok) in [
        (
            "fn same(a: i.Session,b: i.Session) -> Bool = a.cursor == b.cursor;",
            true,
        ),
        (
            "fn wrong(a: i.Session) -> Bool = a.cursor == a.localCursor;",
            false,
        ),
    ] {
        let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, RCORE);
        let mut semantic = DefinitionSource::new(
            "check-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!(
                "rcore 1; package checks version \"1\"; import model \"urn:intersect:iterators\" as i; {body}"
            ),
        );
        semantic.dependencies.push(model.id.clone());
        let result=dispatch(&json!({"method":"rcore_behavior/check","entry":semantic.id,"bundle":{"version":1,"sources":[model,semantic],"namespaces":[]},"conversions":input["conversions"]})).unwrap();
        assert_eq!(result["ok"], ok, "{result}");
        assert_eq!(result["executable"], false);
    }
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.iterators","release":"1","display_name":"Iterators","description":"Checked tree snapshots"});
    let exported = dispatch(&export).unwrap();
    assert_eq!(
        exported["manifest"]["runtime"]["conversions"],
        input["conversions"]
    );
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    dispatch(&json!({"method":"script/begin","package":loaded,"model":saved,"revision":"3"}))
        .unwrap();
    assert_eq!(
        dispatch(&json!({"method":"script/commit","revision":"3"})).unwrap()["model"],
        saved
    );
    assert_eq!(
        step(
            &loaded,
            datatype,
            "next",
            Some(&saved["elements"][0]["values"]["cursor"])
        )
        .unwrap()["item"]["path"],
        json!([0, 0])
    );
}
#[test]
fn invalid_lifecycle_provider_scope_and_cursor_inputs_reject() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    let package = &compiled["package"];
    let datatype = &input["conversions"][0]["datatype"];
    let start = step(package, datatype, "start", None).unwrap();
    let cursor = &start["cursor"];
    for action in ["prune", "remove", "unknown", "start"] {
        assert!(step(package, datatype, action, Some(cursor)).is_err());
    }
    for (pointer, value) in [
        ("/provider/version", json!("2")),
        ("/datatype/source/sha256", json!("stale")),
        ("/value/roots", json!([])),
        ("/value/position", json!(99)),
        ("/value/pruned", json!([1])),
    ] {
        let mut bad = cursor.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(step(package, datatype, "next", Some(&bad)).is_err());
    }
    for (pointer, value) in [
        ("/conversions", json!([])),
        ("/conversions/0/provider/name", json!("example.rgb")),
        ("/conversions/0/serializable", json!(true)),
        ("/conversions/0/declarations/0/path", json!([99])),
    ] {
        let mut bad = input.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert_eq!(dispatch(&bad).unwrap()["ok"], false);
    }
}

#[test]
fn source_lexical_defaults_reject_even_with_current_scope_identities() {
    let mut input = request();
    let ecore = support::with_default(RCORE, "cursor", "iterator");
    let source = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:model.rcore",
        intersect_model::definitions::bundle::SourceKind::Rcore,
        &ecore,
    )
    .id;
    input["rcore"] = json!(ecore);
    for contract in input["conversions"].as_array_mut().unwrap() {
        contract["declarations"][0]["source"] = json!(source);
        if contract["datatype"]["source"]["uri"] == "memory:model.rcore" {
            contract["datatype"]["source"] = json!(source);
        }
    }
    assert_eq!(dispatch(&input).unwrap()["ok"], false);
}
