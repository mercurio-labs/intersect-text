#[path = "../../../tests/text-acceptance/tests/language/common/runtime_operation_providers.rs"]
mod common;
use serde_json::{Value, json};
fn dispatch(request: &Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn rejected(request: &Value) {
    if let Ok(result) = intersect_web::dispatch(&request.to_string()) {
        assert_eq!(result["ok"], false, "{result}");
    }
}
#[test]
fn raw_method_only_provider_selections_survive_host_replay_without_schema_pollution() {
    let f = common::fixture();
    let applications:Vec<_>=f.operations.iter().zip(&f.arguments).map(|(operation,argument)|json!({"receiver":f.receiver,"owner":f.receiver.expression(),"operation":operation,"arguments":[argument]})).collect();
    let request = json!({"method":"ecore/materialize","bundle":f.input.bundle,"entry":f.input.ecore,"namespace":f.input.namespace,"types":f.input.materialized_types,"instances":[{"id":"holder","type":f.receiver}],"edits":[["holder","name","Example"]],"conversions":f.conversions,"constructors":f.constructors,"runtime_operation_applications":applications});
    let result = dispatch(&request);
    assert_eq!(result["ok"], true, "{result}");
    for field in ["conversion_contracts", "constructor_contracts"] {
        assert!(
            result["schema"][field]
                .as_array()
                .is_none_or(|a| a.is_empty())
        );
    }
    assert_eq!(
        result["operation_dependencies"]["conversions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        result["operation_dependencies"]["constructors"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(result["model"]["elements"][0]["values"]["name"], "Example");
    let replay = dispatch(&result["operation_replay"]);
    assert_eq!(replay["ok"], true, "{replay}");
    assert_eq!(result["schema"], replay["schema"]);
    assert_eq!(
        result["runtime_operation_applications"],
        replay["runtime_operation_applications"]
    );
    for field in ["conversions", "constructors"] {
        let mut missing = result["operation_replay"].clone();
        missing[field] = json!([]);
        rejected(&missing);
        let mut stale = result["operation_replay"].clone();
        stale[field][0]["provider"]["version"] = json!("2");
        rejected(&stale);
    }
    let mut stale = result["operation_replay"].clone();
    stale["runtime_operation_applications"][0]["owner"]["declaration"]["source"]["sha256"] =
        json!("0".repeat(64));
    rejected(&stale);
    let mut unused = result["operation_replay"].clone();
    unused["runtime_operation_applications"] = json!([applications[0]]);
    rejected(&unused);
    // A normal source operation check shares the same selected policy environment.
    for application in &applications {
        let mut standalone = application.clone();
        standalone["method"] = json!("ecore/runtime-operation-types");
        standalone["bundle"] = request["bundle"].clone();
        standalone["conversions"] = request["conversions"].clone();
        standalone["constructors"] = request["constructors"].clone();
        assert_eq!(dispatch(&standalone)["ok"], true);
    }
}
