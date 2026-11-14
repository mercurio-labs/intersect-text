//! Exact-source span controls. Java is independent verification only.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate_with_limits, ordinary};
use serde_json::{Value, json};

const NS: &str = "urn:intersect:oracle:semantic-object-spans-v71";
const ECORE: &str =
    include_str!("../../../fixtures/xtext-owned-rule-spans-v71/semantic-spans.ecore");
const MAIN: &str =
    include_str!("../../../fixtures/xtext-owned-rule-spans-v71/SemanticObjectSpans.xtext");
const FRAGMENT: &str =
    include_str!("../../../fixtures/xtext-owned-rule-spans-v71/BoundedFragmentSemanticSpans.xtext");

fn inputs(text: &str) -> BundleCompilation {
    inputs_with_rules(text, "rcore 1; package emptyRules version \"1\";")
}

fn inputs_with_rules(text: &str, behavior: &str) -> BundleCompilation {
    let model = DefinitionSource::new("memory:semantic-spans.ecore", SourceKind::Ecore, ECORE);
    let mut grammar = DefinitionSource::new("memory:semantic-spans.xtext", SourceKind::Xtext, text);
    grammar.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:semantic-spans-rules.rcore",
        SourceKind::RcoreBehavior,
        behavior,
    );
    rules.dependencies.push(model.id.clone());
    let inputs = BundleCompilation {
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), grammar.clone(), rules.clone()],
            namespaces: vec![],
        },
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: NS.into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: rules.id,
    };
    let migration = intersect_model::migration::migrate_ecore_bundle_with_limits(
        inputs.bundle.clone(),
        &[(model.id.clone(), "memory:semantic-spans.rcore".into())].into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migration.reports.len(), 1);
    assert_eq!(migration.reports[0].original, model);
    intersect_text::migration::remap_compilation(&migration, &inputs).unwrap()
}

fn rows() -> Vec<Value> {
    let mut rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-owned-rule-spans-v71/main-observations.json"
    ))
    .unwrap();
    rows.extend(
        serde_json::from_str::<Vec<Value>>(include_str!(
            "../../../fixtures/xtext-owned-rule-spans-v71/fragment-observations.json"
        ))
        .unwrap(),
    );
    rows.extend(
        serde_json::from_str::<Vec<Value>>(include_str!(
            "../../../fixtures/xtext-owned-rule-spans-v71/astral-observations.json"
        ))
        .unwrap(),
    );
    assert_eq!(rows.len(), 31);
    rows
}

fn evaluate(package: &Package, row: &Value) -> Value {
    let output = evaluate_with_limits(
        package,
        &[Source {
            uri: "span.model".into(),
            text: row["text"].as_str().unwrap().into(),
        }],
        EvaluationLimits::workbench(),
    );
    serde_json::to_value(output).unwrap()
}

fn complete_record(profile: Profile, name: &str, text: &str, rows: &[Value]) -> Value {
    let selected: Vec<_> = rows
        .iter()
        .filter(|row| row["grammar"] == format!("intersect.oracle.{name}"))
        .collect();
    match ordinary::compile(inputs(text), profile) {
        Ok(package) => {
            assert_eq!(package.schema.uri, NS);
            assert_eq!(package.grammar.as_ref().unwrap().entry, "Root");
            assert!(
                package
                    .compilation
                    .as_ref()
                    .unwrap()
                    .bundle
                    .sources
                    .iter()
                    .any(|s| s.kind == SourceKind::Xtext && s.text == text)
            );
            let results: Vec<_> = selected.into_iter().map(|row|json!({"case":row["case"],"text":row["text"],"output":evaluate(&package,row)})).collect();
            println!(
                "SPAN_BEFORE {} {} compile=true cases={}",
                profile.as_str(),
                name,
                results.len()
            );
            json!({"profile":profile,"grammar":name,"ok":true,"package":package,"cases":results})
        }
        Err(errors) => {
            println!(
                "SPAN_BEFORE {} {} compile=false {}",
                profile.as_str(),
                name,
                serde_json::to_string(&errors).unwrap()
            );
            json!({"profile":profile,"grammar":name,"ok":false,"errors":errors})
        }
    }
}

#[test]
#[ignore = "verification-only host capture; explicit outside-source output path required"]
fn capture_old_runtime_complete_packages_and_outputs_before_span_patch() {
    let output = std::path::PathBuf::from(std::env::var("INTERSECT_PRIVATE_SPAN_BEFORE").unwrap());
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest.parent().unwrap().parent().unwrap();
    assert!(output.is_absolute() && !output.starts_with(source_root));
    let rows = rows();
    let mut records = Vec::new();
    for profile in [
        Profile::NativeV65,
        Profile::NativeV66,
        Profile::NativeV67,
        Profile::NativeV68,
        Profile::NativeV69,
        Profile::NativeV70,
        Profile::NativeV71,
    ] {
        for (name, text) in [
            ("SemanticObjectSpans", MAIN),
            ("BoundedFragmentSemanticSpans", FRAGMENT),
        ] {
            records.push(complete_record(profile, name, text, &rows));
        }
    }
    let bytes=serde_json::to_vec_pretty(&json!({"state":"old_runtime_before_span_patch","records":records,"scope":"Whole native package and evaluate output DTO baseline; NativeV71 before-state is an unqualified historical candidate, not independent semantics"})).unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap()
        .write_all(&bytes)
        .unwrap();
}

#[test]
#[ignore = "verification-only host capture; new local grammar BEFORE only"]
fn capture_old_runtime_local_capture_before_span_patch() {
    let output = std::path::PathBuf::from(std::env::var("INTERSECT_PRIVATE_SPAN_BEFORE").unwrap());
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest.parent().unwrap().parent().unwrap();
    assert!(output.is_absolute() && !output.starts_with(source_root));
    let text = include_str!("../../../fixtures/xtext-owned-rule-spans-v71/LocalCaptureSpans.xtext");
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-owned-rule-spans-v71/local-capture-observations.json"
    ))
    .unwrap();
    assert_eq!(rows.len(), 11);
    let records: Vec<_> = [
        Profile::NativeV65,
        Profile::NativeV66,
        Profile::NativeV67,
        Profile::NativeV68,
        Profile::NativeV69,
        Profile::NativeV70,
        Profile::NativeV71,
    ]
    .into_iter()
    .map(|profile| complete_record(profile, "LocalCaptureSpans", text, &rows))
    .collect();
    assert!(records.iter().all(|r| r["ok"] == true));
    let bytes=serde_json::to_vec_pretty(&json!({"state":"old_runtime_before_span_patch_new_local_capture_fixture_only","records":records,"scope":"Fresh independent grammar fixture, not a repair/replacement of original blocked main23. Whole package/output baseline; NativeV71 before is historical and unqualified."})).unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap()
        .write_all(&bytes)
        .unwrap();
}

fn all_rows() -> Vec<Value> {
    let mut result = rows();
    result.extend(
        serde_json::from_str::<Vec<Value>>(include_str!(
            "../../../fixtures/xtext-owned-rule-spans-v71/local-capture-observations.json"
        ))
        .unwrap(),
    );
    assert_eq!(result.len(), 42);
    result
}
fn before_records() -> Vec<Value> {
    let mut records = Vec::new();
    for source in [
        include_str!("../../../fixtures/xtext-owned-rule-spans-v71/native-before.json"),
        include_str!("../../../fixtures/xtext-owned-rule-spans-v71/native-before-local.json"),
    ] {
        records.extend(
            serde_json::from_str::<Value>(source).unwrap()["records"]
                .as_array()
                .unwrap()
                .iter()
                .cloned(),
        );
    }
    assert_eq!(records.len(), 21);
    records
}
fn grammar_text(name: &str) -> &'static str {
    match name {
        "SemanticObjectSpans" => MAIN,
        "BoundedFragmentSemanticSpans" => FRAGMENT,
        "LocalCaptureSpans" => {
            include_str!("../../../fixtures/xtext-owned-rule-spans-v71/LocalCaptureSpans.xtext")
        }
        _ => panic!("unregistered exact fixture {name}"),
    }
}
fn independent_objects(row: &Value, package: &Package) {
    use std::collections::{BTreeMap, BTreeSet};
    let output = evaluate(package, row);
    assert_eq!(output["diagnostics"], json!([]), "{}", row["case"]);
    assert_eq!(output["references"], json!([]));
    assert_eq!(output["derived"], json!({}));
    let native = output["model"]["elements"].as_array().unwrap();
    let expected = row["objects"].as_array().unwrap();
    let ids: BTreeMap<_, _> = native
        .iter()
        .enumerate()
        .map(|(index, element)| {
            assert_eq!(
                element["id"],
                format!("span.model#{index}"),
                "native allocation identity"
            );
            assert_eq!(element["resource"], "span.model");
            assert_eq!(
                element["type_identity"],
                json!(package.schema.identity(element["class"].as_str().unwrap()))
            );
            (element["id"].as_str().unwrap(), index)
        })
        .collect();
    let roots = output["model"]["resources"]["span.model"]["roots"]
        .as_array()
        .unwrap();
    assert_eq!(roots.len(), 1);
    let mut pending = vec![(0usize, ids[roots[0].as_str().unwrap()], "root".to_string())];
    let mut visited = BTreeSet::new();
    let mut observed = BTreeSet::new();
    while let Some((oracle_index, native_index, path)) = pending.pop() {
        assert!(
            visited.insert(oracle_index),
            "oracle containment must be acyclic"
        );
        assert!(
            observed.insert(native_index),
            "native containment must preserve object identity"
        );
        let oracle = &expected[oracle_index];
        let element = &native[native_index];
        assert_eq!(
            format!(
                "{}#//{}",
                package.schema.uri,
                element["class"].as_str().unwrap()
            ),
            oracle["class"],
            "{} {path}",
            row["case"]
        );
        assert_eq!(
            element["start"], oracle["node"]["utf8_start"],
            "{} {path} literal UTF8 start",
            row["case"]
        );
        assert_eq!(
            element["end"], oracle["node"]["utf8_end"],
            "{} {path} literal UTF8 end",
            row["case"]
        );
        let values = oracle["values_after_parse"].as_object().unwrap();
        let actual = element["values"].as_object().unwrap();
        assert_eq!(actual.len(), values.len(), "{path} whole feature set");
        for (feature, value) in values {
            if let Some(child) = value.get("object").and_then(Value::as_u64) {
                let id = actual[feature].as_str().unwrap();
                pending.push((child as usize, ids[id], format!("{path}/{feature}")));
            } else {
                assert_eq!(&actual[feature], value, "{} {path}/{feature}", row["case"]);
            }
        }
    }
    assert_eq!(visited.len(), expected.len());
    assert_eq!(observed.len(), native.len());
}

#[test]
fn legacy_complete_packages_errors_and_outputs_match_sealed_before_without_projection() {
    let rows = all_rows();
    let mut tested = 0;
    for old in before_records() {
        let profile: Profile = serde_json::from_value(old["profile"].clone()).unwrap();
        if profile == Profile::NativeV71 {
            continue;
        }
        let name = old["grammar"].as_str().unwrap();
        assert_eq!(
            complete_record(profile, name, grammar_text(name), &rows),
            old,
            "whole sealed old-profile DTO"
        );
        tested += 1;
    }
    assert_eq!(tested, 18);
}

#[test]
fn original_main23_nullable_dispatch_remains_exactly_blocked() {
    let old = before_records()
        .into_iter()
        .find(|r| {
            r["grammar"] == "SemanticObjectSpans" && r["profile"] == "intersect.xtext.native.v71"
        })
        .unwrap();
    assert_eq!(
        complete_record(Profile::NativeV71, "SemanticObjectSpans", MAIN, &all_rows()),
        old
    );
}

#[test]
fn admitted_local_capture_and_fragment_objects_match_genuine_oracle_intervals_and_graphs() {
    let rows = all_rows();
    let mut positive = 0;
    let mut negative = 0;
    for name in ["LocalCaptureSpans", "BoundedFragmentSemanticSpans"] {
        let text = grammar_text(name);
        let package = ordinary::compile(inputs(text), Profile::NativeV71).unwrap();
        assert_eq!(package.grammar.as_ref().unwrap().entry, "Root");
        assert!(
            package
                .compilation
                .as_ref()
                .unwrap()
                .bundle
                .sources
                .iter()
                .any(|s| s.kind == SourceKind::Xtext && s.text == text)
        );
        let old = before_records()
            .into_iter()
            .find(|r| r["grammar"] == name && r["profile"] == "intersect.xtext.native.v71")
            .unwrap();
        assert_eq!(
            serde_json::to_value(&package).unwrap(),
            old["package"],
            "runtime fix does not change retained programs or sources"
        );
        for row in rows
            .iter()
            .filter(|r| r["grammar"] == format!("intersect.oracle.{name}"))
        {
            if row["accepted"] == true {
                independent_objects(row, &package);
                positive += 1;
            } else {
                let output = evaluate(&package, row);
                assert!(!output["diagnostics"].as_array().unwrap().is_empty());
                assert_eq!(
                    output["model"]["elements"],
                    json!([]),
                    "whole resource rollback"
                );
                let old = old["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|case| case["case"] == row["case"])
                    .unwrap();
                assert_eq!(output, old["output"], "whole rejection DTO remains exact");
                negative += 1;
            }
        }
    }
    assert_eq!((positive, negative), (15, 4));
}

#[test]
fn bounded_span_helpers_preserve_whole_resource_rollback_at_execution_limit() {
    let text = grammar_text("LocalCaptureSpans");
    let package = ordinary::compile(inputs(text), Profile::NativeV71).unwrap();
    let source = Source {
        uri: "span.model".into(),
        text: "two x := y;".into(),
    };
    let mut limits = EvaluationLimits::workbench();
    // Discover the exact boundary within the unchanged workbench policy. The
    // prior fixture's arbitrary512 search was insufficient; no production
    // resource policy or parser/lexer budget is increased by this control.
    let mut low = 0;
    let mut high = limits.steps_per_resource.unwrap();
    assert!(
        evaluate_with_limits(&package, std::slice::from_ref(&source), limits)
            .diagnostics
            .is_empty()
    );
    while low < high {
        let middle = low + (high - low) / 2;
        limits.steps_per_resource = Some(middle);
        let result = evaluate_with_limits(&package, std::slice::from_ref(&source), limits);
        if result.diagnostics.is_empty() {
            high = middle;
        } else {
            assert!(
                result.model.elements.is_empty(),
                "failed bounded resource must rollback"
            );
            low = middle + 1;
        }
    }
    let limit = high;
    limits.steps_per_resource = Some(limit);
    let result = evaluate_with_limits(&package, std::slice::from_ref(&source), limits);
    assert!(result.diagnostics.is_empty());
    assert!(limit > 2);
    limits.steps_per_resource = Some(limit - 1);
    let denied = evaluate_with_limits(&package, std::slice::from_ref(&source), limits);
    assert!(denied.model.elements.is_empty());
    assert_eq!(denied.diagnostics.len(), 1);
    assert!(denied.diagnostics[0].message.contains("budget"));
    limits.steps_per_resource = Some(limit);
    assert_eq!(
        serde_json::to_value(evaluate_with_limits(&package, &[source], limits)).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    println!(
        "V71_CONSECUTIVE_EXECUTION_EXACT {limit} one_short {}",
        limit - 1
    );
}

#[test]
fn source_compiled_initializer_executes_at_each_capture_and_authored_assignment_still_wins() {
    // New explicit native integration contract, not an independent Java oracle
    // claim. Grammar/provider bytes remain the genuine local-capture fixture.
    let behavior = r#"rcore 1; package spanInitialization version "1";
import model "urn:intersect:oracle:semantic-object-spans-v71" as m;
initialize m.Root using "intersect.constant-attributes" version "1" { name = "seed"; }
"#;
    for profile in [Profile::NativeV70, Profile::NativeV71] {
        let package = ordinary::compile(
            inputs_with_rules(grammar_text("LocalCaptureSpans"), behavior),
            profile,
        )
        .unwrap();
        assert_eq!(package.semantics.initializers.len(), 1);
        assert!(
            package
                .compilation
                .as_ref()
                .unwrap()
                .bundle
                .sources
                .iter()
                .any(|s| s.kind == SourceKind::RcoreBehavior && s.text == behavior)
        );
        let output = evaluate(&package, &json!({"text":"two x := y;"}));
        assert_eq!(output["diagnostics"], json!([]));
        assert_eq!(output["model"]["elements"].as_array().unwrap().len(), 3);
        let elements = &output["model"]["elements"];
        assert_eq!(
            elements[1]["values"]["name"], "seed",
            "inner capture initializer runs"
        );
        assert_eq!(
            elements[2]["values"]["name"], "x",
            "authored text assignment wins"
        );
        assert_eq!(elements[2]["values"]["previous"], "span.model#1");
        if profile == Profile::NativeV71 {
            assert_eq!(
                (elements[0]["start"].as_u64(), elements[0]["end"].as_u64()),
                (Some(0), Some(3))
            );
            assert_eq!(
                (elements[1]["start"].as_u64(), elements[1]["end"].as_u64()),
                (Some(0), Some(3))
            );
        }
    }
}
