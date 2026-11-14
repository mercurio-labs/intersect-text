#[path = "support/mod.rs"]
mod support;
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    compiler::standard_datatype,
};
use serde_json::{Value, json};
const RCORE: &str = include_str!("../../../fixtures/invocation-errors/errors-structure.rcore");
const XTEXT: &str = include_str!("../../../fixtures/invocation-errors/errors.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../fixtures/invocation-errors/errors-behavior.rcore");
fn dispatch(request: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn request() -> Value {
    let source = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, RCORE);
    let standard = standard_datatype("EInvocationTargetException").unwrap();
    let contract = |id: Value| json!({"datatype":id,"provider":{"name":"ecore.invocation-error","version":"1"},"serializable":false});
    json!({"method":"compile_rcore","rcore":RCORE,"xtext":XTEXT,"rcore_behavior":RCORE_BEHAVIOR,"conversions":[contract(json!(standard.declaration())),contract(json!(support::declaration(&source,"InvocationError")))]})
}
fn wire(contract: &Value, message: Value) -> Value {
    json!({"kind":"converted","datatype":contract["datatype"],"provider":contract["provider"],"value":{"message":message,"target":{"domain":{"name":"example.validation","version":"2"},"code":"invalid","message":"constraint failed"}}})
}
#[test]
fn source_defaults_model_rollback_export_and_nominal_rcore_behavior_use_same_provider() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let fixture_contracts: Value = serde_json::from_str(include_str!(
        "../../../fixtures/invocation-errors/conversions-rcore.json"
    ))
    .unwrap();
    assert_eq!(input["conversions"], fixture_contracts);
    let operation = &package["schema"]["classes"]["Attempt"]["operations"][0];
    let target = json!({"kind":"datatype","name":"__standard_EInvocationTargetException"});
    assert_eq!(operation["result"]["target"], target);
    assert_eq!(operation["parameters"][0]["value"]["target"], target);
    assert_eq!(
        package["schema"]["datatypes"]["__standard_EInvocationTargetException"]["declaration"],
        input["conversions"][0]["datatype"]
    );

    let evaluated=dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"sample.errors","text":"attempt example"}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    let id = &model["elements"][0]["id"];
    assert_eq!(model["elements"][0]["values"]["failure"], Value::Null);
    assert_eq!(model["elements"][0]["values"]["history"], json!([]));
    let first = wire(&input["conversions"][0], json!(null));
    let second = wire(&input["conversions"][0], json!(""));
    let local = wire(&input["conversions"][1], json!("local"));
    dispatch(&json!({"method":"script/begin","package":package,"model":model,"revision":"1"}))
        .unwrap();
    for (feature, value) in [
        ("failure", first.clone()),
        ("history", json!([first, second])),
        ("localFailure", local.clone()),
    ] {
        dispatch(
            &json!({"method":"script/set","revision":"1","id":id,"feature":feature,"value":value}),
        )
        .unwrap();
    }
    let committed = dispatch(&json!({"method":"script/commit","revision":"1"})).unwrap();
    let saved = &committed["model"];
    for (feature, value) in [
        ("failure", local),
        ("history", json!([first, first])),
        (
            "failure",
            json!({"kind":"converted","datatype":first["datatype"],"provider":first["provider"],"value":{"message":null,"target":{"javaObject":true}}}),
        ),
    ] {
        dispatch(&json!({"method":"script/begin","package":package,"model":saved,"revision":"2"}))
            .unwrap();
        let staged = dispatch(
            &json!({"method":"script/set","revision":"2","id":id,"feature":feature,"value":value}),
        );
        assert_eq!(
            staged.is_ok(),
            feature == "history",
            "{feature}: {staged:?}"
        );
        assert!(dispatch(&json!({"method":"script/commit","revision":"2"})).is_err());
    }
    // JSON snapshots re-enter the checked registry; they are not lexical serialization.
    let saved: Value = serde_json::from_str(&saved.to_string()).unwrap();
    dispatch(&json!({"method":"script/begin","package":package,"model":saved,"revision":"3"}))
        .unwrap();
    assert_eq!(
        dispatch(&json!({"method":"script/commit","revision":"3"})).unwrap()["model"],
        saved
    );
    for (body, ok) in [
        (
            "fn same(a: e.Attempt,b: e.Attempt) -> Bool = a.failure == b.failure;",
            true,
        ),
        ("fn wrong(a: e.Attempt) -> String? = a.failure;", false),
        (
            "fn wrong(a: e.Attempt) -> Bool = a.failure == a.localFailure;",
            false,
        ),
    ] {
        let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, RCORE);
        let mut semantic = DefinitionSource::new(
            "errors-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!(
                "rcore 1; package checks version \"1\"; import model \"urn:intersect:errors\" as e; {body}"
            ),
        );
        semantic.dependencies.push(model.id.clone());
        let check = json!({"method":"rcore_behavior/check","entry":semantic.id,"bundle":{"version":1,"sources":[model,semantic],"namespaces":[]},"conversions":input["conversions"]});
        let result = dispatch(&check).unwrap();
        assert_eq!(result["ok"], ok, "{result}");
        assert_eq!(result["executable"], false);
        let mut missing = check;
        missing["conversions"] = json!([]);
        assert_eq!(dispatch(&missing).unwrap()["ok"], false);
    }
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.errors","release":"1","display_name":"Errors","description":"Portable error descriptors"});
    let exported = dispatch(&export).unwrap();
    assert_eq!(
        exported["manifest"]["runtime"]["conversions"],
        input["conversions"]
    );
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    dispatch(&json!({"method":"script/begin","package":loaded,"model":saved,"revision":"4"}))
        .unwrap();
    assert_eq!(
        dispatch(&json!({"method":"script/commit","revision":"4"})).unwrap()["model"],
        saved
    );
}
#[test]
fn missing_mismatched_and_lexical_contracts_reject() {
    let input = request();
    for (pointer, value) in [
        ("/conversions", json!([])),
        ("/conversions/0/provider/name", json!("example.rgb")),
        ("/conversions/0/provider/version", json!("2")),
        ("/conversions/0/serializable", json!(true)),
        ("/conversions/0/datatype/source/sha256", json!("stale")),
    ] {
        let mut invalid = input.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
    }
    for ecore in [
        support::with_default(RCORE, "failure", "opaque"),
        RCORE.replace(
            "java.lang.reflect.InvocationTargetException",
            "java.lang.Object",
        ),
        RCORE.replace(" nonserializable", ""),
    ] {
        let mut invalid = input.clone();
        invalid["rcore"] = json!(ecore);
        // Keep declared source identity current so the metadata/default check is exercised.
        invalid["conversions"][1]["datatype"]["source"] =
            json!(DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, &ecore).id);
        assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
    }
}
