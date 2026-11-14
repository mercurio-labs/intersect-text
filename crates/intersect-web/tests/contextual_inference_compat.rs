//! Exact old inferred request/response contracts, including fixed v56/v57 choices.
use serde_json::{Value, json};
#[test]
fn complete_immutable_old_inferred_responses_and_profile_selection_remain_unchanged() {
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/contextual-first-rule/native/inferred-before-v60.json"
    ))
    .unwrap();
    let mut n = 0;
    for row in before["cases"].as_array().unwrap() {
        let response = intersect_web::dispatch(&row["request"].to_string())
            .unwrap_or_else(|error| json!({"error":error}));
        assert_eq!(response, row["result"]);
        n += 1;
    }
    assert_eq!(n, 4);
}
