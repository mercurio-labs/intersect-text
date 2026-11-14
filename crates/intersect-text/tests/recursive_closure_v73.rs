//! Public compiler and parser controls for V73 scheduling; no second algorithm.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate_with_limits, ordinary};
use serde_json::{Value, json};

fn inputs(ecore: &str, namespace: &str, text: &str) -> BundleCompilation {
    let model = DefinitionSource::new("memory:v73/model.ecore", SourceKind::Ecore, ecore);
    let mut authored = DefinitionSource::new("memory:v73/grammar.xtext", SourceKind::Xtext, text);
    authored.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:v73/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package fixtureProviders version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: namespace.into(),
        xtext: Some(authored.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), authored, rules],
            namespaces: vec![],
        },
    };
    let migration = intersect_model::migration::migrate_ecore_bundle_with_limits(
        input.bundle.clone(),
        &[(model.id.clone(), "memory:v73/model.rcore".into())].into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migration.reports[0].original, model);
    intersect_text::migration::remap_compilation(&migration, &input).unwrap()
}
fn package(text: &str, profile: Profile) -> Package {
    ordinary::compile(
        inputs(
            include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
            "urn:intersect:xtext:recursive:1",
            text,
        ),
        profile,
    )
    .unwrap()
}
fn profile_only_equal(before: &Package, after: &Package) {
    let mut before = serde_json::to_value(before).unwrap();
    let mut after = serde_json::to_value(after).unwrap();
    before["grammar"]["profile"] = Value::Null;
    after["grammar"]["profile"] = Value::Null;
    assert_eq!(before, after);
}
#[test]
fn v74_fallback_preserves_original_recursive_commitment_and_literal_oracle_rows() {
    let text = include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext");
    let old = package(text, Profile::NativeV73);
    let new = package(text, Profile::NativeV74);
    profile_only_equal(&old, &new);
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["parser"] == "RecursiveEntryFirst")
    {
        let source = [Source {
            uri: "original.model".into(),
            text: row["text"].as_str().unwrap().into(),
        }];
        let before = evaluate_with_limits(&old, &source, EvaluationLimits::workbench());
        let after = evaluate_with_limits(&new, &source, EvaluationLimits::workbench());
        assert_eq!(
            after.diagnostics.is_empty(),
            row["accepted"].as_bool().unwrap(),
            "{row}"
        );
        assert_eq!(
            serde_json::to_value(before).unwrap(),
            serde_json::to_value(after).unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 11);
}
#[test]
fn original_guarded_recursion_has_whole_program_runtime_and_literal_oracle_parity() {
    let text = include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext");
    let old = package(text, Profile::NativeV72);
    let new = package(text, Profile::NativeV73);
    profile_only_equal(&old, &new);
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut tested = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["parser"] == "RecursiveEntryFirst")
    {
        let sources = [Source {
            uri: "shared.model".into(),
            text: row["text"].as_str().unwrap().into(),
        }];
        let a = evaluate_with_limits(&old, &sources, EvaluationLimits::workbench());
        let b = evaluate_with_limits(&new, &sources, EvaluationLimits::workbench());
        assert_eq!(
            a.diagnostics.is_empty(),
            row["accepted"].as_bool().unwrap(),
            "{row}"
        );
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
        tested += 1;
    }
    assert_eq!(tested, 11);
    let sources = [Source {
        uri: "shared.model".into(),
        text: include_str!("../../../fixtures/xtext/recursive-predicates/example.model").into(),
    }];
    let actual = evaluate_with_limits(&new, &sources, EvaluationLimits::workbench());
    assert!(actual.diagnostics.is_empty());
    assert_eq!(actual.model.elements.len(), 2);
    assert_eq!(actual.model.elements[0].class, "Left");
    assert_eq!(actual.model.elements[0].values["number"], 12);
    assert_eq!(actual.model.elements[1].class, "Inner");
    assert_eq!(actual.model.elements[1].values["child"], Value::Null);
    assert_eq!(
        serde_json::to_value(&actual).unwrap(),
        serde_json::to_value(evaluate_with_limits(
            &old,
            &sources,
            EvaluationLimits::workbench()
        ))
        .unwrap()
    );
}
#[test]
fn authored_convergent_choice_work_ceiling_stays_explicit_in_both_profiles() {
    let text = include_str!("../../../fixtures/xtext-recursive-closure-v73/Convergent.xtext");
    // Historical capacity witness: request the historical 10k grant explicitly
    // (the default grant has been REALISTIC since 1e5cd75b).
    let compile = |profile| {
        let policy = Default::default();
        ordinary::compile_with_context_and_budget(
            inputs(
                include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
                "urn:intersect:xtext:recursive:1",
                text,
            ),
            profile,
            &[],
            None,
            policy,
            policy,
            intersect_text::grammar::GrammarAnalysisWorkBudget::LEGACY_10K,
        )
    };
    let before = compile(Profile::NativeV72);
    let after = compile(Profile::NativeV73);
    // Native capacity-negative witness. This is not a saving/admission award.
    let expected = json!([{"source":"memory:v73/grammar.xtext","start":0,"end":1,"message":"Grammar prediction budget exceeded"}]);
    assert_eq!(serde_json::to_value(before.unwrap_err()).unwrap(), expected);
    assert_eq!(serde_json::to_value(after.unwrap_err()).unwrap(), expected);
}
#[test]
fn small_convergent_program_replays_complete_native_v72_dtos_without_semantic_oracle_claim() {
    let text = "grammar example.Small hidden(WS)\nimport \"urn:intersect:xtext:recursive:1\" as r\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as Ecore\nEntry returns r::Entry: -> Left | Right;\nLeft returns r::Left: {r::Left} F F 'b';\nRight returns r::Right: {r::Right} F F 'c';\nfragment F returns r::Entry: -> ('a' 'q'?) | 'a' 'q'?;\nterminal WS: (' '| '\\t' | '\\r' | '\\n')+;\n";
    let before = package(text, Profile::NativeV72);
    let after = package(text, Profile::NativeV73);
    profile_only_equal(&before, &after);
    // Exact historical native behavior, not a new independent Xtext oracle.
    // Nonrecursive First choice currently predicts the distinguishing suffix;
    // genuine recursive First oracle assertions above remain unchanged.
    let baseline: Vec<_> = ["a a b", "a a c", "a b", ""]
        .into_iter()
        .map(|text| {
            let sources = [Source {
                uri: "small.model".into(),
                text: text.into(),
            }];
            (
                text,
                evaluate_with_limits(&before, &sources, EvaluationLimits::workbench()),
            )
        })
        .collect();
    println!(
        "V73_NATIVE_SMALL_BEFORE: {}",
        serde_json::to_string(&baseline).unwrap()
    );
    for (text, a) in baseline {
        let sources = [Source {
            uri: "small.model".into(),
            text: text.into(),
        }];
        let b = evaluate_with_limits(&after, &sources, EvaluationLimits::workbench());
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
        if text == "a a b" || text == "a a c" {
            assert!(a.diagnostics.is_empty());
            assert_eq!(a.model.elements.len(), 1);
            assert_eq!(
                a.model.elements[0].class,
                if text == "a a b" { "Left" } else { "Right" }
            );
            assert_eq!((a.model.elements[0].start, a.model.elements[0].end), (0, 5));
            a.model.validate(&before.schema).unwrap();
        } else {
            assert!(!a.diagnostics.is_empty());
            assert!(a.model.elements.is_empty());
        }
    }
    assert_eq!(
        serde_json::to_value(Profile::NativeV72).unwrap(),
        json!("intersect.xtext.native.v72")
    );
    assert_eq!(
        serde_json::to_value(Profile::NativeV73).unwrap(),
        json!("intersect.xtext.native.v73")
    );
}

#[test]
fn inherited_required_call_certificates_and_owned_spans_remain_exact() {
    const NS: &str = "urn:intersect:oracle:required-object-call-v71";
    const ECORE: &str =
        include_str!("../../../fixtures/xtext-required-object-call-v71/required-call.ecore");
    let text =
        include_str!("../../../fixtures/xtext-required-object-call-v71/RequiredObjectCall.xtext");
    let a = ordinary::compile(inputs(ECORE, NS, text), Profile::NativeV72).unwrap();
    let b = ordinary::compile(inputs(ECORE, NS, text), Profile::NativeV73).unwrap();
    profile_only_equal(&a, &b);
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-required-object-call-v71/observations.json"
    ))
    .unwrap();
    let mut positive = 0;
    for row in rows.as_array().unwrap() {
        let sources = [Source {
            uri: "required.model".into(),
            text: row["text"].as_str().unwrap().into(),
        }];
        let actual = evaluate_with_limits(&b, &sources, EvaluationLimits::workbench());
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(evaluate_with_limits(
                &a,
                &sources,
                EvaluationLimits::workbench()
            ))
            .unwrap()
        );
        if row["accepted"] == true {
            assert!(actual.diagnostics.is_empty());
            positive += 1;
            let mut expected:Vec<_>=row["objects"].as_array().unwrap().iter().map(|o|{
                let mut values=o["values_after_parse"].as_object().unwrap().clone();for value in values.values_mut(){if let Some(index)=value.get("object").and_then(Value::as_u64){*value=json!(format!("required.model#{index}"));}}
                json!({"class":o["class"],"start":o["node"]["start"],"end":o["node"]["end"],"values":values})
            }).collect();
            let mut observed:Vec<_>=actual.model.elements.iter().map(|o|{assert_eq!(o.type_identity,b.schema.identity(&o.class));json!({"class":format!("{}#//{}",b.schema.uri,o.class),"start":o.start,"end":o.end,"values":o.values})}).collect();
            expected.sort_by_key(Value::to_string);
            observed.sort_by_key(Value::to_string);
            assert_eq!(observed, expected);
        } else {
            assert!(!actual.diagnostics.is_empty());
            assert!(actual.model.elements.is_empty());
        }
    }
    assert_eq!(positive, 6);
    for text in [
        include_str!(
            "../../../fixtures/xtext-required-object-call-v71/RequiredObjectCallMasking.xtext"
        ),
        include_str!(
            "../../../fixtures/xtext-required-object-call-v71/RequiredObjectCallAmbiguous.xtext"
        ),
    ] {
        let old = ordinary::compile(inputs(ECORE, NS, text), Profile::NativeV72).unwrap_err();
        let new = ordinary::compile(inputs(ECORE, NS, text), Profile::NativeV73).unwrap_err();
        assert!(!old.is_empty());
        assert_eq!(old, new);
    }
}

#[test]
fn v75_call_free_fallback_preserves_original_recursive_commitment_and_literal_oracle_rows() {
    let text = include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext");
    let old = package(text, Profile::NativeV74);
    let new = package(text, Profile::NativeV75);
    profile_only_equal(&old, &new);
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["parser"] == "RecursiveEntryFirst")
    {
        let source = [Source {
            uri: "original.model".into(),
            text: row["text"].as_str().unwrap().into(),
        }];
        let before = evaluate_with_limits(&old, &source, EvaluationLimits::workbench());
        let after = evaluate_with_limits(&new, &source, EvaluationLimits::workbench());
        assert_eq!(
            after.diagnostics.is_empty(),
            row["accepted"].as_bool().unwrap(),
            "{row}"
        );
        assert_eq!(
            serde_json::to_value(before).unwrap(),
            serde_json::to_value(after).unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 11);
}
