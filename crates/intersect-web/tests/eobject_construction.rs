#[path = "../../../tests/text-acceptance/tests/language/common/eobject_construction.rs"]
mod eobject;
#[path = "../../../tests/text-acceptance/tests/language/common/operation_policy_qualification.rs"]
mod policy;
use serde_json::{Value, json};
fn dispatch(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn rejected(request: Value) {
    if let Ok(result) = intersect_web::dispatch(&request.to_string()) {
        assert_eq!(result["ok"], false, "{result}");
    }
}
#[test]
fn eobject_host_constructs_real_instances_and_replays_all_signatures() {
    let f = eobject::fixture();
    let receiver = &f.materialized_types[0];
    let resolver =
        intersect_model::definitions::generic_resolution::Resolver::new(&f.bundle).unwrap();
    let members = resolver.runtime_members(receiver).unwrap();
    let applications: Vec<_> = members
        .members
        .iter()
        .map(|m| json!({"receiver":receiver,"operation":m.declaration,"arguments":[]}))
        .collect();
    let names = [
        "object", "raw", "applied", "root", "child", "other", "local",
    ];
    let instances: Vec<_> = names
        .iter()
        .zip(&f.materialized_types)
        .map(|(id, ty)| json!({"id":id,"type":ty}))
        .collect();
    let request = json!({"method":"ecore/materialize","bundle":f.bundle,"entry":f.ecore,"namespace":f.namespace,"types":f.materialized_types,"instances":instances,"edits":[["root","objects",["object","raw","applied","child","other","local"]],["root","any","object"],["root","local","local"]],"runtime_operation_applications":applications});
    let result = dispatch(request.clone());
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["model"]["elements"].as_array().unwrap().len(), 7);
    assert_eq!(
        result["runtime_operation_applications"]
            .as_array()
            .unwrap()
            .len(),
        15
    );
    assert!(
        result["runtime_operation_applications"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["executable"] == false)
    );
    let replay = dispatch(result["operation_replay"].clone());
    assert_eq!(replay["schema"], result["schema"]);
    assert_eq!(
        replay["runtime_operation_applications"],
        result["runtime_operation_applications"]
    );
    let compiled = dispatch(json!({"method":"compile_bundle","compilation":f}));
    assert_eq!(compiled["ok"], true, "{compiled}");
    let outcome = dispatch(
        json!({"method":"evaluate","package":compiled["package"],"sources":[eobject::source()]}),
    );
    assert!(outcome["diagnostics"].as_array().unwrap().is_empty());
    let mut invalid = request.clone();
    invalid["edits"] = json!([["root", "local", "object"]]);
    rejected(invalid);
    let mut invalid = request;
    invalid["types"][0]["declaration"]["source"]["sha256"] = json!("0".repeat(64));
    rejected(invalid);
}
#[test]
fn selected_operation_policy_replays_native_groups_and_checks_source_identity() {
    for generic in [false, true] {
        let f = policy::fixture(generic);
        let request = json!({"method":"ecore/materialize","bundle":f.input.bundle,"entry":f.input.ecore,"namespace":f.input.namespace,"types":f.input.materialized_types,"conversions":f.conversions,"instances":[{"id":"child","type":f.receiver}],"edits":[["child","name","Saved"]],"runtime_operation_applications":[{"receiver":f.receiver,"operation":f.child,"arguments":[]}]});
        let result = dispatch(request);
        assert_eq!(result["ok"], true, "{result}");
        let sets = result["schema"]["declared_operation_selections"]
            .as_object()
            .unwrap();
        let set = sets
            .values()
            .find(|s| s["receiver"] == json!(f.receiver))
            .unwrap();
        assert_eq!(set["groups"].as_array().unwrap().len(), 1);
        assert_eq!(set["groups"][0]["selected"], json!(f.child));
        assert!(
            set["groups"][0]["operations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["operation"]["declaration"] == json!(f.base))
        );
        let replay = dispatch(result["operation_replay"].clone());
        assert_eq!(replay["schema"], result["schema"]);
        let mut stale = result["operation_replay"].clone();
        stale["conversions"][0]["provider"]["version"] = json!("stale");
        rejected(stale);
        let mut stale = result["operation_replay"].clone();
        stale["runtime_operation_applications"][0]["operation"]["source"]["sha256"] =
            json!("0".repeat(64));
        rejected(stale);
    }
}
