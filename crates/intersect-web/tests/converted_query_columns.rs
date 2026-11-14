use serde_json::{Value, json};
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn cases() -> Vec<(Value, Value)> {
    let plan: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-host-contexts/plan.json"
    ))
    .unwrap();
    let contexts: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-host-contexts/contexts.json"
    ))
    .unwrap();
    plan["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c.clone(),
                contexts
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|s| s["id"] == c["id"])
                    .unwrap()["inference_context"]
                    .clone(),
            )
        })
        .collect()
}
#[test]
fn explicit_host_columns_rebuild_original_source_and_preserve_legacy_query_boundary() {
    for (case, context) in cases() {
        let compiled=call(json!({"method":"compile_inferred_with_context","inference":case["inference"],"inference_context":context})).unwrap();
        assert_eq!(compiled["ok"], true);
        let package = &compiled["package"];
        for entry in [Value::Null, json!("Paint")] {
            let evaluation=call(json!({"method":"evaluate","package":package,"entry_rule":entry,"sources":[{"uri":"memory:nominal-query.model","text":"paint"}]})).unwrap();
            assert_eq!(evaluation["diagnostics"], json!([]));
            let paint = evaluation["model"]["elements"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| {
                    package["schema"]["declarations"][e["class"].as_str().unwrap()]["name"]
                        == "Paint"
                })
                .unwrap();
            let query = json!({"schema_uri":package["schema"]["uri"],"schema_revision":package["schema"]["revision"],"from":paint["class"],"select":[{"name":"color","expression":{"op":"field","path":"color"}}],"limit":10});
            let request = json!({"method":"query/execute_with_conversions","package":package,"model":evaluation["model"],"query":query});
            assert_eq!(
                call(request.clone()).unwrap()["rows"],
                json!([[paint["values"]["color"]]])
            );
            let mut old = request.clone();
            old["method"] = json!("query/execute");
            assert!(
                call(old)
                    .unwrap_err()
                    .contains("Unsupported query datatype")
            );
            let mut cached = request.clone();
            cached["package"]["schema"]["classes"] = Value::Null;
            cached["package"]["grammar"] = json!({"forged":true});
            cached["package"]["semantics"] = Value::Null;
            assert_eq!(call(cached).unwrap(), call(request.clone()).unwrap());
            let mut invalid = request.clone();
            let model = invalid["model"]["elements"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|e| e["id"] == paint["id"])
                .unwrap();
            model["values"]["color"]["value"]["red"] = json!(256);
            assert!(call(invalid).is_err());
            let mut ordered = request.clone();
            ordered["query"]["order_by"] = json!([{"expression":{"op":"field","path":"color"}}]);
            assert!(call(ordered).unwrap_err().contains("ordering"));
            assert_eq!(
                call(request).unwrap()["rows"],
                json!([[paint["values"]["color"]]])
            );
        }
    }
}
