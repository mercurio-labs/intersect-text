#[path = "support/mod.rs"]
mod support;
use serde_json::{Value, json};
const RCORE: &str = include_str!("../../../fixtures/diagnostic-chains/diagnostics-structure.rcore");
fn request() -> Value {
    json!({"method":"compile_rcore","rcore":RCORE,"xtext":include_str!("../../../fixtures/diagnostic-chains/diagnostics.xtext"),"rcore_behavior":include_str!("../../../fixtures/diagnostic-chains/diagnostics-behavior.rcore"),"conversions":serde_json::from_str::<Value>(include_str!("../../../fixtures/diagnostic-chains/conversions-rcore.json")).unwrap()})
}
fn dispatch(request: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn leaf() -> Value {
    json!({"severity":"warning","source":"example.checks","code":12,"message":null,"data":[{"kind":"scalar","datatype":"EBigInteger","value":{"kind":"bigint","value":"922337203685477580800"}}],"children":[]})
}
fn chain(contract: &Value) -> Value {
    json!({"kind":"converted","datatype":contract["datatype"],"provider":contract["provider"],"value":{"diagnostics":[]}})
}
#[test]
fn checked_accumulation_source_identity_model_edits_typing_and_export_rebuild() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let op = &package["schema"]["classes"]["Report"]["operations"][0];
    let target = json!({"kind":"datatype","name":"__standard_EDiagnosticChain"});
    assert_eq!(op["result"]["target"], target);
    assert_eq!(op["parameters"][0]["value"]["target"], target);
    let empty = chain(&input["conversions"][0]);
    let local = chain(&input["conversions"][1]);
    let leaf = leaf();
    let mut parent = leaf.clone();
    parent["children"] = json!([leaf, leaf]);
    let mut accumulated = empty.clone();
    for (operation, diagnostic, count) in [
        ("add", parent.clone(), 1),
        ("add_all", parent.clone(), 3),
        ("merge", parent, 5),
        ("add_all", leaf.clone(), 5),
        ("merge", leaf.clone(), 6),
    ] {
        let result=dispatch(&json!({"method":"diagnostic/collect","package":package,"chain":accumulated,"operation":operation,"diagnostic":diagnostic})).unwrap();
        assert_eq!(result["severity"], "warning");
        accumulated = result["chain"].clone();
        assert_eq!(
            accumulated["value"]["diagnostics"]
                .as_array()
                .unwrap()
                .len(),
            count
        );
    }
    let local_added=dispatch(&json!({"method":"diagnostic/collect","package":package,"chain":local,"operation":"add","diagnostic":leaf})).unwrap()["chain"].clone();
    assert_eq!(local_added["datatype"], local["datatype"]);
    let evaluated=dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"sample.diagnostics","text":"report example"}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    let id = &model["elements"][0]["id"];
    assert!(model["elements"][0]["values"]["diagnostics"].is_null());
    dispatch(&json!({"method":"script/begin","package":package,"model":model,"revision":"1"}))
        .unwrap();
    for (feature, value) in [
        ("diagnostics", accumulated.clone()),
        ("history", json!([empty, accumulated])),
        ("localDiagnostics", local_added.clone()),
    ] {
        dispatch(
            &json!({"method":"script/set","revision":"1","id":id,"feature":feature,"value":value}),
        )
        .unwrap();
    }
    let committed = dispatch(&json!({"method":"script/commit","revision":"1"})).unwrap();
    let saved = &committed["model"];
    for (feature, value, staged) in [
        ("diagnostics", local_added, false),
        ("history", json!([empty, empty]), true),
    ] {
        dispatch(&json!({"method":"script/begin","package":package,"model":saved,"revision":"2"}))
            .unwrap();
        assert_eq!(dispatch(&json!({"method":"script/set","revision":"2","id":id,"feature":feature,"value":value})).is_ok(),staged);
        assert!(dispatch(&json!({"method":"script/commit","revision":"2"})).is_err());
    }
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    for (body, ok) in [
        (
            "fn same(a: d.Report,b: d.Report) -> Bool = a.diagnostics == b.diagnostics;",
            true,
        ),
        ("fn wrong(a: d.Report) -> String? = a.diagnostics;", false),
        (
            "fn wrong(a: d.Report) -> Bool = a.diagnostics == a.localDiagnostics;",
            false,
        ),
    ] {
        let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, RCORE);
        let mut semantic = DefinitionSource::new(
            "diagnostics-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!(
                "rcore 1; package checks version \"1\"; import model \"urn:intersect:diagnostics\" as d; {body}"
            ),
        );
        semantic.dependencies.push(model.id.clone());
        let check = json!({"method":"rcore_behavior/check","entry":semantic.id,"bundle":{"version":1,"sources":[model,semantic],"namespaces":[]},"conversions":input["conversions"]});
        let checked = dispatch(&check).unwrap();
        assert_eq!(checked["ok"], ok, "{checked}");
        assert_eq!(checked["executable"], false);
        let mut missing = check;
        missing["conversions"] = json!([]);
        assert_eq!(dispatch(&missing).unwrap()["ok"], false);
    }
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.diagnostics","release":"1","display_name":"Diagnostics","description":"Explicit chains"});
    let exported = dispatch(&export).unwrap();
    assert_eq!(
        exported["manifest"]["runtime"]["conversions"],
        input["conversions"]
    );
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    let restored: Value = serde_json::from_str(&saved.to_string()).unwrap();
    dispatch(&json!({"method":"script/begin","package":loaded,"model":restored,"revision":"3"}))
        .unwrap();
    assert_eq!(
        dispatch(&json!({"method":"script/commit","revision":"3"})).unwrap()["model"],
        restored
    );
}
#[test]
fn accumulation_rejects_bad_snapshots_contracts_operations_and_unregistered_types() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    let package = &compiled["package"];
    let base = json!({"method":"diagnostic/collect","package":package,"chain":chain(&input["conversions"][0]),"diagnostic":leaf(),"operation":"merge"});
    for (pointer, value) in [
        ("/operation", json!("unknown")),
        ("/chain/provider/name", json!("example.rgb")),
        ("/chain/provider/version", json!("2")),
        ("/chain/datatype/source/sha256", json!("stale")),
        ("/chain/value", json!(null)),
        ("/diagnostic/severity", json!("fatal")),
        ("/diagnostic/data", json!([{"javaObject":1}])),
        ("/package/schema/conversion_contracts", json!([])),
    ] {
        let mut invalid = base.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(dispatch(&invalid).is_err(), "{invalid}");
    }
    for (pointer, value) in [
        ("/conversions", json!([])),
        ("/conversions/0/provider/name", json!("example.rgb")),
        ("/conversions/0/serializable", json!(true)),
        ("/conversions/0/provider/version", json!("2")),
    ] {
        let mut invalid = input.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
    }
    let ecore = support::with_default(RCORE, "diagnostics", "[]");
    let mut invalid = input.clone();
    invalid["rcore"] = json!(ecore);
    invalid["conversions"][1]["datatype"]["source"] = json!(
        intersect_model::definitions::bundle::DefinitionSource::new(
            "memory:model.rcore",
            intersect_model::definitions::bundle::SourceKind::Rcore,
            &ecore
        )
        .id
    );
    assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
}
