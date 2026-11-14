use intersect_web::dispatch;
use serde_json::json;

#[test]
fn browser_adapter_keeps_workbench_input_budget() {
    let compiled = dispatch(
        &json!({
            "method": "compile_rcore",
            "rcore": include_str!("../../../fixtures/library/library-structure.rcore"),
            "xtext": include_str!("../../../fixtures/library/library.xtext"),
            "rcore_behavior": include_str!("../../../fixtures/library/library-behavior.rcore"),
        })
        .to_string(),
    )
    .unwrap();
    let result = dispatch(&json!({
        "method": "evaluate", "package": compiled["package"],
        "sources": [{"uri": "large", "text": format!("library Small {{}}{}", " ".repeat(256_001))}],
    }).to_string()).unwrap();
    assert_eq!(
        result["diagnostics"][0]["message"],
        "Model resource budget exceeded"
    );
    assert_eq!(result["model"]["elements"], json!([]));
}
