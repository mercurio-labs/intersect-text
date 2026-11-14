#[path = "../../../tests/text-acceptance/tests/language/common/applied_model.rs"]
mod common;
use serde_json::{Value, json};

// Give each native request an independent ownership boundary, including empty
// commit requests, instead of reusing an inlined mutable JSON closure.
#[inline(never)]
fn script(method: &str, request: Value) -> Result<Value, String> {
    let Value::Object(mut fields) = request else {
        return Err("Script request must be an object".into());
    };
    fields.insert("method".into(), json!(method));
    fields.insert("revision".into(), json!("applied-model:1"));
    intersect_web::dispatch(&Value::Object(fields).to_string())
}

#[test]
fn applied_only_packages_reconstruct_their_context_for_host_evaluation() {
    let (compilation, contracts) = common::input();
    let call = |request: Value| intersect_web::dispatch(&request.to_string()).unwrap();
    let compiled = call(
        json!({"method":"compile_bundle","compilation":compilation,"applied_conversions":contracts}),
    );
    assert_eq!(compiled["ok"], true, "{compiled}");
    assert!(
        compiled["package"]["schema"]
            .get("conversion_contracts")
            .is_none()
    );
    let sources = json!([{"uri":"one.model","text":"holder sample"}]);
    let evaluated =
        call(json!({"method":"evaluate","package":compiled["package"],"sources":sources}));
    assert_eq!(evaluated["diagnostics"], json!([]));
    assert_eq!(
        evaluated["model"]["elements"][0]["values"]["name"],
        "sample"
    );
    let mut stale = compiled["package"].clone();
    stale["schema"]["applied_conversion_contracts"][0]["conversion"]["provider"]["version"] =
        json!("wrong");
    assert!(
        intersect_web::dispatch(
            &json!({"method":"evaluate","package":stale,"sources":sources}).to_string()
        )
        .is_err()
    );
    let bound = call(
        json!({"method":"conversion/applied","bundle":compilation.bundle,"conversions":contracts,
        "operation":{"kind":"bind","application":contracts[0].application,"payload":{"items":[common::nominal(&compilation,0,json!("sample"))]}}}),
    );
    assert_eq!(bound["ok"], true, "{bound}");

    script(
        "script/begin",
        json!({"package":compiled["package"],"model":evaluated["model"]}),
    )
    .unwrap();
    let element = &evaluated["model"]["elements"][0]["id"];
    script(
        "script/set",
        json!({"id":element,"feature":"texts","value":bound["value"]}),
    )
    .unwrap();
    assert_eq!(
        script("script/get", json!({"id":element,"feature":"texts"})).unwrap(),
        bound["value"]
    );
    let prepared = script("script/commit", json!({})).unwrap();
    assert_eq!(
        prepared["model"]["elements"][0]["values"]["texts"],
        bound["value"]
    );
    let checked = script(
        "script/check",
        json!({"package":compiled["package"],"model":prepared["model"]}),
    )
    .unwrap();
    assert_eq!(checked["model"], prepared["model"]);
    script(
        "script/begin",
        json!({"package":compiled["package"],"model":prepared["model"]}),
    )
    .unwrap();
    let wrong = call(
        json!({"method":"conversion/applied","bundle":compilation.bundle,"conversions":contracts,
        "operation":{"kind":"bind","application":contracts[1].application,"payload":{"items":[common::nominal(&compilation,1,json!(42))]}}}),
    );
    assert!(
        script(
            "script/set",
            json!({"id":element,"feature":"texts","value":wrong["value"]})
        )
        .is_err()
    );
    assert!(script("script/commit", json!({})).is_err());
    let unchanged = script(
        "script/check",
        json!({"package":compiled["package"],"model":prepared["model"]}),
    )
    .unwrap();
    assert_eq!(unchanged["model"], prepared["model"]);
    let model = call(
        json!({"method":"ecore/materialize","bundle":compilation.bundle,"types":compilation.materialized_types,
        "applied_conversions":contracts,"instances":[{"id":"h","type":compilation.materialized_types[0]}],"edits":[["h","texts",bound["value"]]]}),
    );
    assert_eq!(model["ok"], true, "{model}");
    assert_eq!(
        model["model"]["elements"][0]["values"]["texts"],
        bound["value"]
    );
}
