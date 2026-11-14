use intersect_text::package::BundleCompilation;
use intersect_web::dispatch;
use serde_json::json;

#[test]
fn module_host_and_bundle_compiler_use_the_same_exact_resolution_context() {
    let compilation: intersect_text::package::BundleCompilation = serde_json::from_str(
        include_str!("../../../fixtures/library/rcore-behavior-modules.json"),
    )
    .unwrap();
    let compilation =
        intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(compilation).unwrap();
    let result = dispatch(
        &json!({"method":"rcore_behavior/bind", "bundle":compilation.bundle, "entry":compilation.rcore_behavior})
            .to_string(),
    )
    .unwrap();
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["executable"], false);
    assert_eq!(result["stage"], "resolution");
    let expected = intersect_model::definitions::rcore_behavior::bindings::bind(
        &compilation.bundle,
        &compilation.rcore_behavior,
    )
    .unwrap();
    assert_eq!(result["resolved"], serde_json::to_value(expected).unwrap());
    let compiled =
        dispatch(&json!({"method":"compile_bundle", "compilation":compilation}).to_string())
            .unwrap();
    assert!(
        compiled["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Derivation result must match feature type")
    );
    let mut bad = compilation;
    let entry = bad
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == bad.rcore_behavior)
        .unwrap();
    entry.dependencies.retain(|id| id == &bad.ecore);
    let compiled =
        dispatch(&json!({"method":"compile_bundle", "compilation":bad}).to_string()).unwrap();
    assert!(
        compiled["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("No exact dependency")
    );
    assert_eq!(
        compiled["diagnostics"][0]["source"],
        "file:///definitions/library-modular-behavior.rcore"
    );
}

#[test]
fn syntax_host_matches_shared_parser_and_never_claims_execution() {
    let text = include_str!("../../../fixtures/library/library-v2-behavior.rcore");
    let uri = "file:///library-behavior.rcore";
    let result =
        dispatch(&json!({"method":"rcore_behavior/parse", "uri":uri, "text":text}).to_string())
            .unwrap();
    let native = intersect_model::definitions::rcore_behavior::parse(
        intersect_model::definitions::bundle::DefinitionSource::new(
            uri,
            intersect_model::definitions::bundle::SourceKind::RcoreBehavior,
            text,
        ),
    );
    assert_eq!(result["document"], serde_json::to_value(native).unwrap());
    assert_eq!(result["ok"], true);
    assert_eq!(result["executable"], false);
    assert_eq!(result["stage"], "syntax");
    let oversized = " ".repeat(256_001);
    assert!(
        dispatch(
            &json!({"method":"rcore_behavior/parse", "uri":uri, "text":oversized}).to_string()
        )
        .unwrap_err()
        .contains("budget")
    );
    let boundary = dispatch(
        &json!({"method":"rcore_behavior/parse", "uri":uri, "text":" ".repeat(256_000)})
            .to_string(),
    )
    .unwrap();
    assert_eq!(boundary["ok"], false);
}

#[test]
fn package_compiler_retains_typed_functions_and_reports_syntax_errors() {
    let compile = |text: &str| {
        dispatch(&json!({"method":"compile_rcore", "rcore":r#"rcore 1; package fixture version "1"; metamodel "urn:fixture" prefix f { class Item {} }"#, "rcore_behavior":text}).to_string()).unwrap()
    };
    let valid = compile("rcore 1; package valid version \"1\"; fn value() -> Int = 1;");
    assert_eq!(valid["ok"], true, "{valid}");
    assert_eq!(valid["package"]["semantics"]["derived"], json!([]));
    assert_eq!(
        valid["package"]["compilation"]["bundle"]["sources"][1]["text"],
        "rcore 1; package valid version \"1\"; fn value() -> Int = 1;"
    );
    let invalid = compile("rcore 1; package m version \"1\"; fn broken() -> Int = ;");
    assert_eq!(invalid["ok"], false);
    assert!(invalid["diagnostics"][0]["start"].as_u64().unwrap() > 30);
    assert_eq!(invalid["diagnostics"][0]["source"], "memory:behavior.rcore");
}

#[test]
fn typing_host_matches_shared_checker_and_compilation_preserves_external_type_errors() {
    use intersect_model::definitions::{bundle::DefinitionSource, rcore_behavior::checking::check};
    let compilation: BundleCompilation = serde_json::from_str(include_str!(
        "../../../fixtures/rcore-behavior-typing/bundle.json"
    ))
    .unwrap();
    for (source, text) in compilation.bundle.sources.iter().zip([
        include_str!("../../../fixtures/rcore-behavior-typing/model.ecore"),
        include_str!("../../../fixtures/rcore-behavior-typing/model.xtext"),
        include_str!("../../../fixtures/rcore-behavior-typing/main-behavior.rcore"),
        include_str!("../../../fixtures/rcore-behavior-typing/helpers-behavior.rcore"),
    ]) {
        assert_eq!(source.text, text.replace("\r\n", "\n"));
        source.validate().unwrap();
    }
    let compilation =
        intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(compilation).unwrap();
    let result = dispatch(
        &json!({"method":"rcore_behavior/check", "bundle":compilation.bundle, "entry":compilation.rcore_behavior})
            .to_string(),
    )
    .unwrap();
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["stage"], "typing");
    assert_eq!(result["executable"], false);
    assert_eq!(
        result["typed"],
        serde_json::to_value(check(&compilation.bundle, &compilation.rcore_behavior).unwrap())
            .unwrap()
    );
    let compiled =
        dispatch(&json!({"method":"compile_bundle", "compilation":compilation}).to_string())
            .unwrap();
    assert!(
        compiled["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("type checked successfully"),
        "{compiled}"
    );
    let mut bad = compilation.clone();
    let helper = &bad.bundle.sources[3];
    let changed = DefinitionSource::new(
        &helper.id.uri,
        helper.kind,
        helper.text.replace("= value;", "= 1;"),
    );
    bad.bundle.sources[2].dependencies = vec![bad.ecore.clone(), changed.id.clone()];
    bad.bundle.sources[3] = changed.clone();
    let compiled =
        dispatch(&json!({"method":"compile_bundle", "compilation":bad}).to_string()).unwrap();
    assert!(
        compiled["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["source"] == changed.id.uri
                && d["message"].as_str().unwrap().contains("Type mismatch")),
        "{compiled}"
    );
    let mut oversize = compilation.bundle;
    oversize.sources[2].text = " ".repeat(256_001);
    assert!(
        dispatch(
            &json!({"method":"rcore_behavior/check", "bundle":oversize, "entry":compilation.rcore_behavior})
                .to_string()
        )
        .unwrap_err()
        .contains("budget")
    );
}
