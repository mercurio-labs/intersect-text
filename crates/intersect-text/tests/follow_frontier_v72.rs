//! V72 replay of the exact imported-source witnesses, plus public compiler work controls.
//! Oracle literals are unchanged; Java is independent verification only.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate_with_limits, ordinary};
use serde_json::{Value, json};

const NS: &str = "urn:intersect:oracle:required-object-call-v71";
const ECORE: &str =
    include_str!("../../../fixtures/xtext-required-object-call-v71/required-call.ecore");
const GRAMMAR: &str =
    include_str!("../../../fixtures/xtext-required-object-call-v71/RequiredObjectCall.xtext");

fn inputs(text: &str) -> BundleCompilation {
    let model = DefinitionSource::new("memory:required-call.ecore", SourceKind::Ecore, ECORE);
    let mut grammar = DefinitionSource::new("memory:required-call.xtext", SourceKind::Xtext, text);
    grammar.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:required-call-rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package emptyRules version \"1\";",
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
        &[(model.id.clone(), "memory:required-call.rcore".into())].into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migration.reports[0].original, model);
    intersect_text::migration::remap_compilation(&migration, &inputs).unwrap()
}

fn package(text: &str) -> Package {
    let package = ordinary::compile(inputs(text), Profile::NativeV72).unwrap();
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
    package
}

fn compare(row: &Value, package: &Package) {
    let source = Source {
        uri: "required.model".into(),
        text: row["text"].as_str().unwrap().into(),
    };
    let actual = evaluate_with_limits(package, &[source], EvaluationLimits::workbench());
    assert!(
        actual.diagnostics.is_empty(),
        "{row}: {:?}",
        actual.diagnostics
    );
    assert!(actual.references.is_empty());
    assert!(actual.derived.is_empty());
    let mut expected: Vec<_> = row["objects"].as_array().unwrap().iter().map(|o| {
        let mut values = o["values_after_parse"].as_object().unwrap().clone();
        for value in values.values_mut() {
            if let Some(index) = value.get("object").and_then(Value::as_u64) {
                *value = json!(format!("required.model#{index}"));
            }
        }
        json!({"class":o["class"],"start":o["node"]["start"],"end":o["node"]["end"],"values":values})
    }).collect();
    let mut observed: Vec<_> = actual
        .model
        .elements
        .iter()
        .map(|o| {
        assert_eq!(o.type_identity, package.schema.identity(&o.class));
        json!({"class":format!("{}#//{}",package.schema.uri,o.class),"start":o.start,"end":o.end,"values":o.values})
    })
        .collect();
    // Identity/order stays literal in references; sorting only makes object rows
    // independent of how a model collection presents otherwise identical rows.
    expected.sort_by_key(Value::to_string);
    observed.sort_by_key(Value::to_string);
    assert_eq!(
        observed, expected,
        "whole independent class/value/span projection for {}",
        row["case"]
    );
}

#[test]
fn required_nullable_object_call_preserves_independent_objects_values_and_literal_spans() {
    let package = package(GRAMMAR);
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-required-object-call-v71/observations.json"
    ))
    .unwrap();
    let mut positive = 0;
    for row in rows.as_array().unwrap() {
        if row["accepted"] == true {
            compare(row, &package);
            positive += 1;
        } else {
            let actual = evaluate_with_limits(
                &package,
                &[Source {
                    uri: "required.model".into(),
                    text: row["text"].as_str().unwrap().into(),
                }],
                EvaluationLimits::workbench(),
            );
            assert!(!actual.diagnostics.is_empty(), "{row}");
            assert!(actual.model.elements.is_empty(), "whole resource rollback");
        }
    }
    assert_eq!(positive, 6);
}

#[test]
fn unrelated_guard_masking_requires_exact_partition_or_conservative_rejection() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-required-object-call-v71/masking-observations.json"
    ))
    .unwrap();
    let grammars = [
        (
            "Masking",
            include_str!(
                "../../../fixtures/xtext-required-object-call-v71/RequiredObjectCallMasking.xtext"
            ),
        ),
        (
            "Ambiguous",
            include_str!(
                "../../../fixtures/xtext-required-object-call-v71/RequiredObjectCallAmbiguous.xtext"
            ),
        ),
    ];
    let mut tested = 0;
    for (suffix, text) in grammars {
        match ordinary::compile(inputs(text), Profile::NativeV72) {
            Ok(package) => {
                for row in rows.as_array().unwrap().iter().filter(|r| {
                    r["grammar"] == format!("intersect.oracle.RequiredObjectCall{suffix}")
                        && r["accepted"] == true
                }) {
                    compare(row, &package);
                    tested += 1;
                }
            }
            Err(errors) => {
                assert!(!errors.is_empty());
                let messages = serde_json::to_string(&errors).unwrap();
                assert!(
                    messages.contains("prediction") || messages.contains("Completion"),
                    "explicit conservative prediction rejection: {messages}"
                );
                println!("V72_CONSERVATIVE_{suffix}: {messages}");
                tested += 1;
            }
        }
    }
    assert!(tested >= 2);
}

#[test]
fn legacy_source_and_follow_rejections_are_whole_exact_and_v72_small_program_changes_only_profile()
{
    let old = ordinary::compile(inputs(GRAMMAR), Profile::NativeV70).unwrap_err();
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        json!([{
            "source":"memory:required-call.xtext", "start":0,"end":1,
            "message":"Fragment caller follow conflict requires unsupported Xtext prediction"
        }])
    );
    for profile in [Profile::NativeV68, Profile::NativeV69] {
        assert_eq!(
            ordinary::compile(inputs(GRAMMAR), profile).unwrap_err(),
            old
        );
    }
    for profile in [Profile::NativeV65, Profile::NativeV66, Profile::NativeV67] {
        assert_eq!(
            serde_json::to_value(ordinary::compile(inputs(GRAMMAR), profile).unwrap_err()).unwrap(),
            json!([{
                "source":"memory:required-call.xtext", "start":325,"end":326,
                "message":"Unsupported Xtext construct '='"
            }])
        );
    }
    let simple = GRAMMAR.replace(
        "(=> binding=Binding '.')? empty=Empty",
        "'target' empty=Empty",
    );
    let mut old =
        serde_json::to_value(ordinary::compile(inputs(&simple), Profile::NativeV70).unwrap())
            .unwrap();
    let mut new =
        serde_json::to_value(ordinary::compile(inputs(&simple), Profile::NativeV72).unwrap())
            .unwrap();
    old["grammar"]["profile"] = Value::Null;
    new["grammar"]["profile"] = Value::Null;
    assert_eq!(old, new);
}

#[test]
fn many_consuming_fragments_release_only_the_follow_walk_work() {
    // Independent authored compiler-capacity witness, not the original language.
    // Root127 calls plus127 thirty-eight-keyword fragments: N5081 and K0.
    // Exactly128 declarations fit the unchanged default source limit128.
    let text = include_str!("../../../fixtures/xtext-follow-frontier-v72/ManyConsuming.xtext");
    let old = ordinary::compile(inputs(text), Profile::NativeV71).unwrap_err();
    assert_eq!(
        serde_json::to_value(old).unwrap(),
        json!([{
            "source":"memory:required-call.xtext","start":0,"end":1,
            "message":"Grammar prediction budget exceeded"
        }])
    );
    let package = ordinary::compile(inputs(text), Profile::NativeV72).unwrap();
    let grammar = package.grammar.as_ref().unwrap();
    assert_eq!(grammar.profile, Profile::NativeV72);
    assert_eq!(grammar.rules.len(), 128);
    assert_eq!(grammar.entry, "Root");
    assert_eq!(grammar.name, "intersect.follow.Many");
    for index in 0..127 {
        let rule = &grammar.rules[&format!("F{index:03}")];
        assert!(rule.fragment);
        assert_eq!(rule.class, "Root");
        assert_eq!(rule.body.cardinality, '1');
        let intersect_text::grammar::Kind::Sequence(items) = &rule.body.kind else {
            panic!("authored thirty-eight-keyword sequence")
        };
        assert_eq!(items.len(), 38);
        assert!(items.iter().all(|e| e.cardinality == '1'
            && matches!(&e.kind,intersect_text::grammar::Kind::Literal(s) if s=="a")));
    }
    let program = serde_json::to_value(grammar.parser_prediction.as_ref().unwrap()).unwrap();
    assert_eq!(program["rules"].as_object().unwrap().len(), 128);
    assert!(program["states"].as_array().unwrap().len() < 10_000);
    // The existing DTO omits this field when its set is empty.
    assert!(program.get("recursive_decisions").is_none());
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
}

#[test]
fn whole_small_package_program_and_evaluation_match_v71_except_explicit_profile() {
    let old = ordinary::compile(inputs(GRAMMAR), Profile::NativeV71).unwrap();
    let new = ordinary::compile(inputs(GRAMMAR), Profile::NativeV72).unwrap();
    let mut a = serde_json::to_value(&old).unwrap();
    let mut b = serde_json::to_value(&new).unwrap();
    assert_eq!(a["grammar"]["profile"], json!("intersect.xtext.native.v71"));
    assert_eq!(b["grammar"]["profile"], json!("intersect.xtext.native.v72"));
    a["grammar"]["profile"] = Value::Null;
    b["grammar"]["profile"] = Value::Null;
    assert_eq!(a, b);
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-required-object-call-v71/observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let sources = [Source {
            uri: "required.model".into(),
            text: row["text"].as_str().unwrap().into(),
        }];
        assert_eq!(
            serde_json::to_value(evaluate_with_limits(
                &old,
                &sources,
                EvaluationLimits::workbench()
            ))
            .unwrap(),
            serde_json::to_value(evaluate_with_limits(
                &new,
                &sources,
                EvaluationLimits::workbench()
            ))
            .unwrap()
        );
    }
}
