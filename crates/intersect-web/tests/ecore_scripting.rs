use intersect_web::dispatch;
use serde_json::{Value, json};

const ECORE: &str = r#"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="books" nsURI="urn:books" nsPrefix="books">
<eClassifiers xsi:type="ecore:EClass" name="Book"><eStructuralFeatures xsi:type="ecore:EAttribute" name="title" lowerBound="1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
</ecore:EPackage>"#;
fn call(method: &str, mut request: Value) -> Result<Value, String> {
    request["method"] = json!(method);
    request["revision"] = json!("ecore-test:1");
    dispatch(&request.to_string())
}

fn migrated_source() -> String {
    let migrated = call(
        "ecore/migrate",
        json!({"uri":"memory:books.ecore", "target_uri":"memory:books.rcore", "text":ECORE}),
    )
    .unwrap();
    assert_eq!(migrated["ok"], true);
    assert_eq!(migrated["report"]["original"]["text"], ECORE);
    migrated["report"]["converted"]["text"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn ecore_compilation_and_script_validation_share_the_native_model() {
    let rcore = migrated_source();
    let compiled = call("compile_rcore", json!({"rcore": rcore})).expect("compile");
    assert_eq!(compiled["ok"], true);
    assert!(compiled["package"]["grammar"].is_null());
    assert_eq!(compiled["keywords"], json!([]));
    let package = &compiled["package"];
    let structural = intersect_text::compiler::compile_rcore_with_profile(
        &rcore,
        None,
        None,
        Default::default(),
    )
    .expect("structural package");
    let no_syntax = intersect_text::evaluate(&structural, &[]);
    assert!(
        no_syntax
            .diagnostics
            .iter()
            .any(|d| d.message.contains("no text grammar"))
    );
    let empty = call("evaluate", json!({"package": package, "sources":[{"uri":"unrelated.text", "text":"this has no grammar"}]})).expect("empty Ecore model");
    assert_eq!(empty["model"]["elements"], json!([]));
    assert_eq!(empty["diagnostics"], json!([]));
    call(
        "script/begin",
        json!({"package":package,"model":empty["model"]}),
    )
    .expect("begin");
    call(
        "script/create",
        json!({"class":"Book","values":{"title":"Earthsea"}}),
    )
    .expect("create");
    let prepared = call("script/commit", json!({})).expect("shared validation");
    assert_eq!(
        prepared["model"]["elements"][0]["values"]["title"],
        "Earthsea"
    );
    assert_eq!(prepared["transaction"]["status"], "prepared");
    let discarded = call("script/finish", json!({"transaction":prepared["transaction"],"status":"rolled_back","reason":"Disposable test completed","changes_complete":true})).expect("discard");
    assert_eq!(discarded["status"], "rolled_back");
    // Every test can restart from the same fresh fixture without publishing it.
    call(
        "script/begin",
        json!({"package":package,"model":empty["model"]}),
    )
    .expect("fresh fixture");
    assert_eq!(
        call("script/all", json!({"class":"Book"})).expect("empty"),
        json!([])
    );
    call("script/create", json!({"class":"Book","values":{}}))
        .expect("incomplete model allowed while editing");
    assert!(
        call("script/commit", json!({}))
            .expect_err("required title")
            .contains("Multiplicity")
    );
    assert_eq!(empty["model"]["elements"], json!([]));
}

#[test]
fn ecore_queries_and_xmi_use_the_same_recompiled_package() {
    let compiled = call("compile_rcore", json!({"rcore": migrated_source()})).expect("compile");
    let package = &compiled["package"];
    let empty = call("evaluate", json!({"package":package,"sources":[]})).expect("empty");
    call(
        "script/begin",
        json!({"package":package,"model":empty["model"]}),
    )
    .expect("begin");
    call(
        "script/create",
        json!({"class":"Book","values":{"title":"Dune"}}),
    )
    .expect("create");
    let prepared = call("script/commit", json!({})).expect("prepare");
    let exported = call(
        "model/export",
        json!({"package":package,"model":prepared["model"],"format":"xmi"}),
    )
    .expect("Ecore XMI export");
    assert!(
        exported["resources"]
            .as_array()
            .is_some_and(|r| !r.is_empty())
    );
    let result = call("query/execute",json!({"package":package,"model":prepared["model"],"query":{"schema_uri":package["schema"]["uri"],"schema_revision":package["schema"]["revision"],"from":"Book","select":[{"name":"title","expression":{"op":"field","path":"title"}}],"limit":10}})).expect("Ecore query");
    assert!(result.to_string().contains("Dune"));
}
