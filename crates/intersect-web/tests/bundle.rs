use intersect_web::dispatch;
use serde_json::{Value, json};
const INPUT: &str = include_str!("../../../fixtures/library/multi-resource-canonical.json");

#[test]
fn bundle_host_rebuilds_from_retained_inputs_and_rejects_stale_definitions() {
    let input: Value = serde_json::from_str(INPUT).unwrap();
    let compiled =
        dispatch(&json!({"method":"compile_bundle", "compilation":input}).to_string()).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let mut package = compiled["package"].clone();
    package["schema"]["classes"] = json!({});
    package["sources"]["ecore"] = json!("<untrusted/>");
    let sources = json!([{"uri":"catalog.library", "text": include_str!("../../../fixtures/library/catalog.library")}, {"uri":"loans.library", "text": include_str!("../../../fixtures/library/loans.library")}]);
    let request = json!({"method":"evaluate", "package":package, "sources":sources});
    let evaluated = dispatch(&request.to_string()).unwrap();
    assert_eq!(evaluated["model"]["elements"].as_array().unwrap().len(), 6);
    assert_eq!(evaluated["diagnostics"], json!([]));
    package["compilation"]["bundle"]["sources"][1]["text"] = json!("<stale/>");
    assert!(
        dispatch(&json!({"method":"evaluate", "package":package, "sources":sources}).to_string())
            .unwrap_err()
            .contains("digest")
    );
}
