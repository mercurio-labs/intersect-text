//! Public same-source parity controls; the source remains authored and intact.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::grammar::admission_observation::AnalysisObservers;
use intersect_text::grammar::admission_observation::Coverage;
use intersect_text::grammar::admission_observation::PredictionObservation;
use intersect_text::grammar::admission_observation::RecursiveAnalysisCounters;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate_with_limits, ordinary};
use serde_json::{Value, json};

const TEXT: &str =
    include_str!("../../../fixtures/xtext-nonconflicting-repeat-v74/Qualified.xtext");
fn input(text: &str) -> BundleCompilation {
    let model = DefinitionSource::new(
        "memory:v74/model.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/xtext-nonconflicting-repeat-v74/model.ecore"),
    );
    let mut authored = DefinitionSource::new("memory:v74/grammar.xtext", SourceKind::Xtext, text);
    authored.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:v74/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package fixtureProviders version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:intersect:repeat:74".into(),
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
        &[(model.id.clone(), "memory:v74/model.rcore".into())].into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migration.reports[0].original, model);
    intersect_text::migration::remap_compilation(&migration, &input).unwrap()
}
fn package(text: &str, profile: Profile) -> Package {
    ordinary::compile(input(text), profile).unwrap()
}
fn profile_only_equal(a: &Package, b: &Package) {
    let mut a = serde_json::to_value(a).unwrap();
    let mut b = serde_json::to_value(b).unwrap();
    a["grammar"]["profile"] = Value::Null;
    b["grammar"]["profile"] = Value::Null;
    assert_eq!(a, b);
}
fn legacy_static_refusal() -> Value {
    json!({"model":{"schema_revision":"","resources":{},"elements":[]},"diagnostics":[{"source":"model","start":0,"end":0,"message":"Completion static verification failed at rule/Item/s1/repeat: Full-predicate shared-prefix proof requires explicit overapproximation"}],"references":[],"derived":{}})
}
fn legacy_lexical_refusal() -> Value {
    json!({
        "model": {
            "schema_revision": "70e8d74de0f41fae90b87518035b64343e711699da775737e0b997abd528def3",
            "resources": {"repeat.model": {
                "uri": "repeat.model", "roots": [], "content_revision": "0",
                "load_state": "Loaded", "diagnostics": ["6..7: No viable lexical alternative"],
                "source": null
            }},
            "elements": []
        },
        "diagnostics": [{"source": "repeat.model", "start": 6, "end": 7,
            "message": "No viable lexical alternative"}],
        "references": [], "derived": {}
    })
}
#[test]
fn qualified_repeat_preserves_whole_package_runtime_construction_spans_and_errors() {
    let old = package(TEXT, Profile::NativeV73);
    let new = package(TEXT, Profile::NativeV74);
    profile_only_equal(&old, &new);
    for text in ["{}", "{ x ( x ) x }", "{ x } trailing", "{ ( }"] {
        let source = [Source {
            uri: "repeat.model".into(),
            text: text.into(),
        }];
        let a = evaluate_with_limits(&old, &source, EvaluationLimits::workbench());
        let b = evaluate_with_limits(&new, &source, EvaluationLimits::workbench());
        let legacy_dto = serde_json::to_value(&a).unwrap();
        let dto = serde_json::to_value(&b).unwrap();
        if text == "{ x } trailing" {
            // This unchanged malformed input fails lexing before prediction.
            assert_eq!(legacy_dto, legacy_lexical_refusal());
            assert_eq!(dto, legacy_dto);
            continue;
        }
        assert_eq!(legacy_dto, legacy_static_refusal());
        if text == "{}" || text == "{ x ( x ) x }" {
            assert!(
                b.diagnostics.is_empty(),
                "authored input {text:?}; exact NativeV73 baseline {}",
                serde_json::to_string(&a).unwrap()
            );
            assert_eq!(dto["model"]["elements"].as_array().unwrap().len(), 1);
            assert_eq!(dto["model"]["elements"][0]["class"], "Root");
            assert_eq!(dto["model"]["elements"][0]["start"], 0);
            assert_eq!(dto["model"]["elements"][0]["end"], text.len());
            assert_eq!(
                b.model.elements[0].type_identity,
                new.schema.identity("Root")
            );
            assert_eq!(b.model.elements[0].id, "repeat.model#0");
            assert_eq!(b.model.elements[0].resource, "repeat.model");
            assert_eq!(dto["model"]["elements"][0]["values"], json!({}));
            assert!(b.references.is_empty() && b.derived.is_empty());
        } else {
            assert!(!b.diagnostics.is_empty());
            assert!(dto["model"]["elements"].as_array().unwrap().is_empty());
        }
    }
}
#[test]
fn qualified_repeat_reuses_the_same_runtime_work_and_rollback_at_every_small_limit() {
    let old = package(TEXT, Profile::NativeV73);
    let new = package(TEXT, Profile::NativeV74);
    let sources = [Source {
        uri: "repeat.model".into(),
        text: "{ x ( x ) x }".into(),
    }];
    for steps in 0..=120 {
        let limits = EvaluationLimits {
            steps_per_resource: Some(steps),
            ..EvaluationLimits::workbench()
        };
        assert_eq!(
            serde_json::to_value(evaluate_with_limits(&old, &sources, limits)).unwrap(),
            serde_json::to_value(evaluate_with_limits(&new, &sources, limits)).unwrap()
        );
    }
}
#[test]
fn hidden_source_policy_uses_the_shared_recognizer_without_certificate_bonus() {
    let text = TEXT.replace(
        "fragment Item returns model::Root:",
        "fragment Item returns model::Root hidden():",
    );
    let old = package(&text, Profile::NativeV73);
    let new = package(&text, Profile::NativeV74);
    profile_only_equal(&old, &new);
    for text in ["{x}", "{ x }"] {
        let sources = [Source {
            uri: "repeat.model".into(),
            text: text.into(),
        }];
        assert_eq!(
            serde_json::to_value(evaluate_with_limits(
                &old,
                &sources,
                EvaluationLimits::workbench()
            ))
            .unwrap(),
            legacy_static_refusal()
        );
        let actual = evaluate_with_limits(&new, &sources, EvaluationLimits::workbench());
        assert!(
            actual.diagnostics.is_empty(),
            "{text}: {:?}",
            actual.diagnostics
        );
        assert_eq!(actual.model.elements.len(), 1);
        assert_eq!(actual.model.elements[0].class, "Root");
        assert_eq!(
            (actual.model.elements[0].start, actual.model.elements[0].end),
            (0, text.len())
        );
    }
}
#[test]
fn new_profile_phase_and_seven_category_coverage_are_truthful_and_whole_result_equal() {
    let phase = PredictionObservation::new();
    let counters = RecursiveAnalysisCounters::new();
    let policy = Default::default();
    let plain = ordinary::compile(input(TEXT), Profile::NativeV74).unwrap();
    let observed = ordinary::compile_with_context_measured(
        input(TEXT),
        Profile::NativeV74,
        &[],
        None,
        policy,
        policy,
        AnalysisObservers {
            phases: &phase,
            recursive_work: Some(&counters),
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(plain).unwrap(),
        serde_json::to_value(observed).unwrap()
    );
    assert!(phase.first_failure().is_none());
    assert_eq!(
        counters.snapshot().coverage,
        Coverage::RepeatCertificateAnalysisUninstrumented
    );
    assert_eq!(counters.snapshot().stages[2].program_invocations, 0);
}
