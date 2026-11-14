//! Unified authored Rcore behavior companion; existing four frozen cases stay intact.
use intersect_model::definitions::resources::ResourceSet;
use intersect_model::definitions::schema::MemberIdentity;
use intersect_text::grammar::SourceLimits;
use intersect_text::inference::{
    InferenceRequest,
    providers::{ProviderSelection, compile_selected},
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-provider-selection/unified.json"
    ))
    .unwrap()
}
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
#[test]
fn unified_source_selects_exact_authored_behavior_replays_and_executes_declared_unset() {
    let fixture = fixture();
    let input: InferenceRequest = serde_json::from_value(fixture["inference"].clone()).unwrap();
    let selection: ProviderSelection =
        serde_json::from_value(fixture["provider_selection"].clone()).unwrap();
    assert_eq!(selection.behavior, selection.computed_settings[0].source);
    assert_eq!(
        input
            .bundle
            .sources
            .iter()
            .find(|source| source.id == selection.behavior)
            .unwrap()
            .kind,
        intersect_model::definitions::bundle::SourceKind::Rcore
    );
    let selected = compile_selected(&input, &selection, SourceLimits::default(), None).unwrap();
    let expected = ResourceSet::new(&input.bundle)
        .unwrap()
        .resolve(&selection.behavior, "#//Item/computed")
        .unwrap();
    assert_eq!(
        selected.computed_settings()[0].member,
        MemberIdentity::Source {
            declaration: expected
        }
    );
    let result=call(json!({"method":"inferred/selectProviders","inference":input,"provider_selection":selection})).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(
        result["inference_context"]["behavior"],
        fixture["provider_selection"]["behavior"]
    );
    assert_eq!(result["package"], json!(selected.language().package()));
    let original = json!({"method":"evaluate","package":result["package"],"sources":[{"uri":"memory:unified.model","text":fixture["model_text"]}]});
    let evaluated = call(original.clone()).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let item = evaluated["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|element| {
            result["package"]["schema"]["declarations"][element["class"].as_str().unwrap()]["name"]
                == "Item"
        })
        .unwrap();
    assert_eq!(item["values"]["backing"], json!("alpha"));
    assert_eq!(
        evaluated["derived"][item["id"].as_str().unwrap()]["computed"],
        json!("alpha")
    );
    let replay=call(json!({"method":"compile_inferred_with_context","inference":input,"inference_context":result["inference_context"]})).unwrap();
    assert_eq!(replay["ok"], true);
    assert_eq!(replay["package"], result["package"]);
    assert_eq!(call(original).unwrap(), evaluated);
    let revision = "unified-provider";
    call(json!({"method":"script/begin","package":replay["package"],"model":evaluated["model"],"revision":revision})).unwrap();
    call(json!({"method":"script/unset","revision":revision,"id":item["id"],"feature":"computed"}))
        .unwrap();
    let unset = call(json!({"method":"script/inspect","revision":revision})).unwrap();
    let id = item["id"].as_str().unwrap();
    let item = unset["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap();
    assert_eq!(item["values"]["backing"], Value::Null);
    assert_eq!(unset["derived"][id]["computed"], Value::Null);
    let mut only_unset = selection.clone();
    only_unset.computed_settings[0].operations =
        vec![intersect_model::definitions::setting::SettingOperation::Unset];
    assert!(compile_selected(&input, &only_unset, SourceLimits::default(), None).is_err());
    let mut only_set = selection;
    only_set.computed_settings[0].operations =
        vec![intersect_model::definitions::setting::SettingOperation::Set];
    let limited=call(json!({"method":"inferred/selectProviders","inference":input,"provider_selection":only_set})).unwrap();
    assert_eq!(limited["ok"], true);
    let limited_model=call(json!({"method":"evaluate","package":limited["package"],"sources":[{"uri":"memory:unified.model","text":fixture["model_text"]}]})).unwrap();
    call(json!({"method":"script/begin","package":limited["package"],"model":limited_model["model"],"revision":revision})).unwrap();
    assert!(
        call(json!({"method":"script/unset","revision":revision,"id":id,"feature":"computed"}))
            .unwrap_err()
            .contains("operation is not declared")
    );
}
