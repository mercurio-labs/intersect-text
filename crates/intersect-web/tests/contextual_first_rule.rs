//! Explicit contextual source admission; original V60 requests remain immutable.
use intersect_model::definitions::bundle::SourceId;
use intersect_text::grammar;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/contextual-first-rule/native/native-cases.json"
    ))
    .unwrap()
}
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap_or_else(|error| json!({"error":error}))
}
fn case(id: &str) -> Value {
    fixture()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
        .clone()
}
fn compile(c: &Value, profile: Profile) -> Value {
    call(json!({"method":"compile_bundle","compilation":c["compilation"],"xtext_profile":profile}))
}
#[test]
fn all_twenty_authored_graphs_retain_contextual_and_independent_failures() {
    let valid = [
        "terminal_library_model_child",
        "datatype_library_model_child",
        "enum_library_model_child",
        "same_name_class_first",
        "valid_inherited_operand_override",
        "valid_explicit_parent_operand",
    ];
    let first_parser = [
        "terminal_before_local_model",
        "terminal_child_over_model_parent",
        "enum_before_local_model",
        "inherited_operand_override",
        "explicit_parent_operand",
    ];
    let first_datatype = [
        "datatype_before_local_model",
        "datatype_child_over_model_parent",
        "same_name_datatype_first",
    ];
    let mut accepted = 0;
    for c in fixture()["cases"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let result = compile(c, Profile::NativeV61);
        assert_eq!(result["ok"] == true, valid.contains(&id), "{id}: {result}");
        if result["ok"] == true {
            accepted += 1;
            assert_eq!(result["package"]["compilation"], c["compilation"]);
            continue;
        }
        let diagnostics = result["diagnostics"].as_array().unwrap();
        assert!(!diagnostics.is_empty(), "{id}");
        let msg = diagnostics[0]["message"].as_str().unwrap();
        if first_parser.contains(&id) {
            assert_eq!(msg, "The first rule must be a parser rule.", "{id}");
        }
        if first_datatype.contains(&id) {
            assert_eq!(
                msg, "The first rule must be a parser rule, but is a data type rule.",
                "{id}"
            );
        }
        if id == "fragment_class_first" {
            assert_eq!(msg, "The first rule must not be a fragment");
        }
        let original = c["compilation"]["bundle"]["sources"].as_array().unwrap();
        assert!(
            original
                .iter()
                .any(|s| s["id"]["uri"] == diagnostics[0]["source"]),
            "{id}: {diagnostics:?}"
        );
    }
    assert_eq!(accepted, 6);
}
#[test]
fn complete_frozen_v60_requests_and_errors_remain_unchanged() {
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/contextual-first-rule/native/before-v60.json"
    ))
    .unwrap();
    fn walk(v: &Value, n: &mut usize) {
        if let (Some(request), Some(expected)) = (v.get("request"), v.get("result")) {
            assert_eq!(&call(request.clone()), expected, "{request}");
            *n += 1;
            return;
        }
        match v {
            Value::Array(a) => {
                for x in a {
                    walk(x, n)
                }
            }
            Value::Object(o) => {
                for x in o.values() {
                    walk(x, n)
                }
            }
            _ => (),
        }
    }
    let mut count = 0;
    walk(&before, &mut count);
    assert_eq!(count, 240);
}
#[test]
fn shared_branch_priority_preserves_scalar_libraries_and_enum_rule_kind() {
    use grammar::first_rule::{Category as C, Issue, check};
    assert_eq!(
        check(
            Some(C::Parser {
                datatype: true,
                fragment: true
            }),
            false
        ),
        Ok(())
    );
    assert_eq!(
        check(
            Some(C::Parser {
                datatype: true,
                fragment: true
            }),
            true
        ),
        Err(Issue::DatatypeFirst)
    );
    assert_eq!(check(Some(C::NonParser), false), Ok(()));
    assert_eq!(check(Some(C::NonParser), true), Err(Issue::ParserRequired));
    assert_eq!(
        check(
            Some(C::Parser {
                datatype: false,
                fragment: true
            }),
            true
        ),
        Err(Issue::FragmentFirst)
    );
}
fn parameter(id: &str) -> Value {
    let f: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/contextual-first-rule/native/parameter-cases.json"
    ))
    .unwrap();
    f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
        .clone()
}
#[test]
fn authored_parameter_admission_precedes_specialization() {
    let good = parameter("canonical_parameter");
    let package = compile(&good, Profile::NativeV61);
    assert_eq!(package["ok"], true, "{package}");
    assert_eq!(
        package["package"]["grammar"]["profile"],
        "intersect.xtext.native.v61"
    );
    assert_eq!(package["package"]["compilation"], good["compilation"]);
    let result = call(
        json!({"method":"evaluate","package":package["package"],"sources":[{"uri":"memory:parameter.model","text":"a"}]}),
    );
    assert_eq!(result["diagnostics"], json!([]));
    assert_eq!(result["model"]["elements"][0]["values"]["name"], "a");
    let bad = parameter("terminal_first_parameter");
    assert_eq!(compile(&bad, Profile::NativeV60)["ok"], true);
    let result = compile(&bad, Profile::NativeV61);
    assert_eq!(result["ok"], false);
    assert_eq!(
        result["diagnostics"][0]["message"],
        "The first rule must be a parser rule."
    );
    assert_eq!(
        result["diagnostics"][0]["source"],
        bad["compilation"]["xtext"]["uri"]
    );
    let gap = parameter("identity_parameter_before");
    for profile in [Profile::NativeV60, Profile::NativeV61] {
        let failed = compile(&gap, profile);
        assert_eq!(failed["ok"], false);
        assert_eq!(
            failed["diagnostics"][0]["message"],
            "Grammar assignment conflicts with the imported feature"
        );
    }
}
#[test]
fn same_named_class_uses_real_model_values_and_archive_source_rebuild() {
    let c = case("same_name_class_first");
    let result = compile(&c, Profile::NativeV61);
    assert_eq!(result["ok"], true);
    let package = &result["package"];
    let evaluation = call(
        json!({"method":"evaluate","package":package,"sources":[{"uri":"memory:class.model","text":"a"}]}),
    );
    assert_eq!(evaluation["diagnostics"], json!([]));
    assert_eq!(evaluation["model"]["elements"][0]["values"]["name"], "a");
    let class = evaluation["model"]["elements"][0]["class"]
        .as_str()
        .unwrap();
    assert_eq!(package["schema"]["declarations"][class]["name"], "EString");
    assert_eq!(
        package["schema"]["declarations"][class]["namespace"],
        "urn:intersect:xtext:first-rule:identity:1"
    );
    assert_eq!(
        package["schema"]["declarations"][class]["source"],
        c["compilation"]["ecore"]
    );
    let distribution = call(
        json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV61,"metadata":{"package_id":"first-rule.class","release":"1","display_name":"Class EString","description":""}}),
    );
    let mut files = distribution["files"].clone();
    files["manifest.json"] = json!(distribution["manifest"].to_string());
    let loaded = call(json!({"method":"package/import","files":files}));
    assert_eq!(&loaded["package"], package);
    let rebuilt = intersect_text::ordinary::compile_with_optional_source_policy(
        serde_json::from_value::<BundleCompilation>(c["compilation"].clone()).unwrap(),
        Profile::NativeV61,
        &[],
        None,
        None,
        Default::default(),
    )
    .unwrap();
    assert_eq!(json!(rebuilt), *package);
}

#[test]
fn direct_text_and_exact_source_admission_share_the_resolved_predicate() {
    let c = case("same_name_class_first");
    let native: intersect_text::package::Package =
        serde_json::from_value(compile(&c, Profile::NativeV61)["package"].clone()).unwrap();
    let text = c["compilation"]["bundle"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["kind"] == "xtext")
        .unwrap()["text"]
        .as_str()
        .unwrap();
    let good = grammar::compile_with_profile(text, &native.schema, Profile::NativeV61).unwrap();
    assert_eq!(good.rules["Model"].class, "EString");
    let (header, rules) = text.split_once("Model returns").unwrap();
    let (model, terminal) = rules.split_once("terminal VALUE").unwrap();
    let invalid = format!("{header}terminal VALUE{terminal}Model returns{model}");
    assert!(grammar::compile_with_profile(&invalid, &native.schema, Profile::NativeV60).is_ok());
    let error =
        grammar::compile_with_profile(&invalid, &native.schema, Profile::NativeV61).unwrap_err();
    assert_eq!(error.message, "The first rule must be a parser rule.");
    let selected = SourceId::for_text("memory:direct-first-rule.xtext", &invalid);
    let mut source = intersect_model::definitions::bundle::DefinitionSource::new(
        &selected.uri,
        intersect_model::definitions::bundle::SourceKind::Xtext,
        invalid,
    );
    let mut input: BundleCompilation = serde_json::from_value(c["compilation"].clone()).unwrap();
    source.dependencies = vec![input.ecore.clone()];
    input.xtext = Some(source.id.clone());
    input
        .bundle
        .sources
        .retain(|s| s.kind != intersect_model::definitions::bundle::SourceKind::Xtext);
    input.bundle.sources.push(source);
    let errors = intersect_text::ordinary::compile(input, Profile::NativeV61).unwrap_err();
    assert_eq!(errors[0].message, error.message);
    let origin = errors[0].origin.as_ref().unwrap();
    assert!(format!("{origin:?}").contains(&selected.sha256));
}
#[test]
fn source_verified_archive_rejects_rehashed_invalid_source_and_cached_entry() {
    let c = case("same_name_class_first");
    let ex = call(
        json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV61,"metadata":{"package_id":"first-rule.tamper","release":"1","display_name":"First rule","description":""}}),
    );
    let bad = case("enum_before_local_model");
    let mut manifest = ex["manifest"].clone();
    let mut files = ex["files"].clone();
    let path = manifest["definitions"]["bundle"]
        .as_str()
        .unwrap()
        .to_owned();
    let text = bad["compilation"].to_string();
    files[&path] = json!(text);
    manifest["files"][&path] =
        json!({"bytes":text.len(),"sha256":SourceId::for_text("digest", &text).sha256});
    files["manifest.json"] = json!(manifest.to_string());
    let rejected = call(json!({"method":"package/import","files":files}));
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .contains("first rule must be a parser rule"),
        "{rejected}"
    );
    let mut manifest = ex["manifest"].clone();
    let mut files = ex["files"].clone();
    let path = manifest["compiled_package"].as_str().unwrap().to_owned();
    let mut package: Value = serde_json::from_str(files[&path].as_str().unwrap()).unwrap();
    package["grammar"]["entry"] = json!("forgedEntry");
    let text = package.to_string();
    files[&path] = json!(text);
    manifest["files"][&path] =
        json!({"bytes":text.len(),"sha256":SourceId::for_text("digest", &text).sha256});
    files["manifest.json"] = json!(manifest.to_string());
    assert!(call(json!({"method":"package/import","files":files}))["error"].is_string());
    let active = compile(&c, Profile::NativeV61);
    let live = call(
        json!({"method":"evaluate","package":active["package"],"sources":[{"uri":"memory:still-good.model","text":"a"}]}),
    );
    assert_eq!(live["model"]["elements"][0]["values"]["name"], "a");
}

#[test]
fn original_source_revision_and_requested_bounds_remain_authoritative() {
    let c = parameter("canonical_parameter");
    let input: BundleCompilation = serde_json::from_value(c["compilation"].clone()).unwrap();
    let package = intersect_text::ordinary::compile_with_optional_source_policy(
        input.clone(),
        Profile::NativeV61,
        &[],
        None,
        None,
        Default::default(),
    )
    .unwrap();
    assert_eq!(json!(package.compilation.as_ref().unwrap()), json!(input));
    assert_eq!(
        serde_json::to_value(intersect_text::recompile(&package).unwrap()).unwrap(),
        json!(package)
    );
    let selected = input.xtext.clone().unwrap();
    let mut stale = input.clone();
    stale
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == selected)
        .unwrap()
        .text
        .push(' ');
    assert!(intersect_text::ordinary::compile(stale, Profile::NativeV61).is_err());
    let size = intersect_text::package::CompilationSourcePolicy {
        xtext_bytes: input.bundle.source(&selected).unwrap().text.len() - 1,
        ..Default::default()
    };
    assert!(
        intersect_text::ordinary::compile_with_source_policy(
            input.clone(),
            Profile::NativeV61,
            size,
            Default::default()
        )
        .is_err()
    );
    let declarations = intersect_text::package::CompilationSourcePolicy {
        xtext_declarations: 1,
        ..Default::default()
    };
    assert!(
        intersect_text::ordinary::compile_with_source_policy(
            input,
            Profile::NativeV61,
            declarations,
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn v61_inherits_existing_finite_until_execution_on_all_generated_native_controls() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until/native-cases.json"
    ))
    .unwrap();
    let o: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until/native-observations.json"
    ))
    .unwrap();
    let mut graphs = 0;
    let mut runs = 0;
    for row in o["cases"].as_array().unwrap() {
        if row["upstream_generated"] != true {
            continue;
        }
        let c = f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == row["id"])
            .unwrap();
        let package = compile(c, Profile::NativeV61);
        assert_eq!(package["ok"], true, "{}: {package}", row["id"]);
        graphs += 1;
        for run in row["runtime"].as_array().unwrap() {
            let result = call(
                json!({"method":"evaluate","package":package["package"],"sources":[{"uri":"memory:inherited-until.model","text":run["text"]}]}),
            );
            assert_eq!(
                result["diagnostics"].as_array().unwrap().is_empty(),
                run["resource_accepted"].as_bool().unwrap(),
                "{} {}",
                row["id"],
                run["text"]
            );
            runs += 1;
        }
    }
    assert_eq!((graphs, runs), (20, 268));
}
