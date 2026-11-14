use intersect_text::distribution::ordinary::PackageMetadata;
use intersect_text::grammar::Profile;
use intersect_text::{Source, distribution};
use serde_json::{Value, json};
#[path = "../../../tests/text-acceptance/shared/xtext_imported_parameter_inputs.rs"]
mod imported_inputs;
fn metadata() -> PackageMetadata {
    PackageMetadata {
        package_id: "ordinary.host".into(),
        release: "1".into(),
        display_name: "Ordinary host".into(),
        description: String::new(),
    }
}
fn extracted(d: &distribution::Distribution) -> std::collections::BTreeMap<String, String> {
    let mut files = d.files.clone();
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&d.manifest).unwrap(),
    );
    files
}
#[test]
fn host_adoption() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ))
    .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "false_call")
        .unwrap();
    let inputs = imported_inputs::input(case);
    let call = |request: Value| intersect_web::dispatch(&request.to_string()).unwrap();
    let compiled = call(
        json!({"method":"compile_bundle", "compilation":inputs, "xtext_profile":Profile::NativeV57}),
    );
    assert_eq!(compiled["ok"], true, "{compiled}");
    assert!(compiled["package"]["source_policy"].is_null());
    let with_registry = call(
        json!({"method":"compile_bundle", "compilation":inputs, "xtext_profile":Profile::NativeV57, "conversions":[]}),
    );
    assert_eq!(with_registry["package"], compiled["package"]);
    let exported = call(
        json!({"method":"package/export", "compilation":inputs, "xtext_profile":Profile::NativeV57, "metadata":metadata()}),
    );
    assert_eq!(exported["package"], compiled["package"]);
    let with_registry = call(
        json!({"method":"package/export", "compilation":inputs, "xtext_profile":Profile::NativeV57, "metadata":metadata(), "conversions":[]}),
    );
    assert_eq!(with_registry, exported);
    let archive: distribution::Distribution = serde_json::from_value(exported).unwrap();
    let loaded = call(json!({"method":"package/import", "files":extracted(&archive)}));
    assert_eq!(loaded["package"], compiled["package"]);
    let evaluated = call(
        json!({"method":"evaluate", "package":loaded["package"], "sources":[{"uri":"memory:host.model", "text":"alpha"}]}),
    );
    assert_eq!(evaluated["diagnostics"], json!([]));
    let evaluated: intersect_text::Evaluation = serde_json::from_value(evaluated).unwrap();
    let expected = intersect_text::evaluate(
        &archive.package,
        &[Source {
            uri: "memory:host.model".into(),
            text: "alpha".into(),
        }],
    );
    assert_eq!(evaluated.model, expected.model);
    let invalid = call(
        json!({"method":"compile_bundle", "compilation":inputs, "xtext_profile":Profile::NativeV56}),
    );
    assert_eq!(invalid["ok"], false);
}
