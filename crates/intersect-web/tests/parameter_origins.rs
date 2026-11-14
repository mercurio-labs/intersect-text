//! Authored source-correspondence controls, not additional JVM semantic evidence.
use serde_json::{Value, json};
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
#[test]
fn generated_parameter_failures_keep_legacy_diagnostics_and_narrow_authored_scopes() {
    let plan: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/parameter-origins/cases.json"
    ))
    .unwrap();
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/parameter-origins/before.json"
    ))
    .unwrap();
    let expected: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/parameter-origins/expectations.json"
    ))
    .unwrap();
    for case in plan["cases"].as_array().unwrap() {
        let previous = before
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == case["id"])
            .unwrap();
        let request = json!({"method":"compile_inferred_with_origins", "inference":previous["inference"], "diagnostic_origins":[{"kind":"source","source":{"uri":"forged","sha256":"forged"}}]});
        let result = call(request);
        assert_eq!(result["ok"], false);
        assert_eq!(
            result["diagnostics"], previous["result"]["diagnostics"],
            "{}",
            case["id"]
        );
        assert_eq!(
            result["diagnostic_origins"][0]["generated"],
            previous["result"]["diagnostic_origins"][0]["generated"]
        );
        let authored = case["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["id"] == case["expected_source"])
            .unwrap();
        let span = json!({"source":{"uri":authored["uri"],"sha256":authored["sha256"]},"start":expected["scopes"][case["id"].as_str().unwrap()]["start"],"end":expected["scopes"][case["id"].as_str().unwrap()]["end"]});
        assert_ne!(
            previous["result"]["diagnostic_origins"][0]["originals"],
            json!([span.clone()])
        );
        assert_eq!(
            result["diagnostic_origins"][0]["originals"],
            json!([span]),
            "{}: {result}",
            case["id"]
        );
        let legacy = call(json!({"method":"compile_inferred","inference":previous["inference"]}));
        assert_eq!(
            legacy,
            json!({"ok":false,"diagnostics":previous["result"]["diagnostics"]})
        );
    }
}
