use intersect_web::dispatch;
use serde_json::{Value, json};
fn fixture() -> (Value, Value) {
    let compiled = dispatch(&json!({"method":"compile_rcore","rcore":include_str!("../../../fixtures/library/library-structure.rcore"),"xtext":include_str!("../../../fixtures/library/library.xtext"),"rcore_behavior":include_str!("../../../fixtures/library/library-behavior.rcore")}).to_string()).unwrap();
    let p = compiled["package"].clone();
    let result = dispatch(&json!({"method":"evaluate","package":p,"sources":[{"uri":"catalog.library","text":include_str!("../../../fixtures/library/catalog.library")},{"uri":"loans.library","text":include_str!("../../../fixtures/library/loans.library")}]}).to_string()).unwrap();
    (p, result["model"].clone())
}
#[test]
fn host_executes_shared_queries_and_rejects_bad_inputs() {
    let (package, model) = fixture();
    let mut request = json!({"method":"query/execute","package":package,"model":model,"query":{"schema_uri":package["schema"]["uri"],"schema_revision":package["schema"]["revision"],"from":"BookCopy","select":[{"name":"available","expression":{"op":"field","path":"available"}}],"limit":50}});
    assert_eq!(
        dispatch(&request.to_string()).unwrap()["rows"],
        json!([[false], [true]])
    );
    request["query"]["schema_revision"] = json!("stale");
    assert!(
        dispatch(&request.to_string())
            .unwrap_err()
            .contains("revision")
    );
    request["query"]["schema_revision"] = package["schema"]["revision"].clone();
    request["query"]["select"][0]["expression"]["path"] = json!("not_a_feature");
    assert!(
        dispatch(&request.to_string())
            .unwrap_err()
            .contains("Unknown feature")
    );
    request["model"]["elements"][1]["values"]["edition"] = json!("bad");
    assert!(dispatch(&request.to_string()).is_err());
}
