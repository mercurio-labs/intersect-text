//! Public authored algorithm witnesses over the shared ordinary compiler/runtime.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::grammar::admission_observation::AnalysisObservers;
use intersect_text::grammar::admission_observation::Coverage;
use intersect_text::grammar::admission_observation::Phase;
use intersect_text::grammar::admission_observation::PredictionObservation;
use intersect_text::grammar::admission_observation::RecursiveAnalysisCounters;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate_with_limits, ordinary};
use serde_json::{Value, json};

const MANY: &str = include_str!("../../../fixtures/xtext-call-free-relative-v75/ManyCallers.xtext");
const SMALL: &str = include_str!("../../../fixtures/xtext-call-free-relative-v75/Small.xtext");
fn input(text: &str) -> BundleCompilation {
    let model = DefinitionSource::new(
        "memory:v75/model.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/xtext-call-free-relative-v75/model.ecore"),
    );
    let mut grammar = DefinitionSource::new("memory:v75/grammar.xtext", SourceKind::Xtext, text);
    grammar.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:v75/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package fixtureProviders version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:intersect:reuse:75".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), grammar, rules],
            namespaces: vec![],
        },
    };
    let migration = intersect_model::migration::migrate_ecore_bundle_with_limits(
        input.bundle.clone(),
        &[(model.id.clone(), "memory:v75/model.rcore".into())].into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migration.reports[0].original, model);
    intersect_text::migration::remap_compilation(&migration, &input).unwrap()
}
fn package(text: &str, profile: Profile) -> Package {
    ordinary::compile(input(text), profile).unwrap()
}
fn model_text(count: usize, suffix: &str) -> String {
    format!("{} {suffix}", vec!["k00"; count].join(" "))
}

#[test]
fn call_free_many_actual_callers_reuse_prefixes_without_changing_the_original_pool() {
    let policy = Default::default();
    let old_observation = PredictionObservation::new();
    // V74 is the historical capacity witness: request the historical 10k grant
    // explicitly (the default grant has been REALISTIC since 1e5cd75b).
    let old = ordinary::compile_with_context_work_observed_and_budget(
        input(MANY),
        Profile::NativeV74,
        &[],
        None,
        policy,
        policy,
        AnalysisObservers::phases(&old_observation).into(),
        intersect_text::grammar::GrammarAnalysisWorkBudget::LEGACY_10K,
    )
    .unwrap_err();
    assert!(
        old.iter()
            .any(|d| d.message == "Grammar prediction budget exceeded")
    );
    assert_eq!(
        old_observation.first_failure().unwrap().phase,
        Phase::ParserRecursiveAnalysis
    );
    let new = package(MANY, Profile::NativeV75);
    for suffix in ["left", "right"] {
        let text = model_text(90, suffix);
        let result = evaluate_with_limits(
            &new,
            &[Source {
                uri: "many.model".into(),
                text: text.clone(),
            }],
            EvaluationLimits::workbench(),
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        let object = &result.model.elements[0];
        assert_eq!(object.class, "Root");
        assert_eq!(object.type_identity, new.schema.identity("Root"));
        assert_eq!((object.start, object.end), (0, text.len()));
        assert_eq!(object.id, "many.model#0");
        assert_eq!(serde_json::to_value(&object.values).unwrap(), json!({}));
        assert!(result.references.is_empty() && result.derived.is_empty());
    }
}

#[test]
fn call_free_small_whole_program_runtime_errors_and_rollback_preserve_v74() {
    let old = package(SMALL, Profile::NativeV74);
    let new = package(SMALL, Profile::NativeV75);
    let mut a = serde_json::to_value(&old).unwrap();
    let mut b = serde_json::to_value(&new).unwrap();
    a["grammar"]["profile"] = Value::Null;
    b["grammar"]["profile"] = Value::Null;
    assert_eq!(a, b);
    for text in [
        model_text(3, "left"),
        model_text(3, "right"),
        model_text(2, "left"),
        model_text(3, "unknown"),
        String::new(),
    ] {
        let sources = [Source {
            uri: "small.model".into(),
            text,
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
        for work in [0, 1, 10, 100] {
            let limits = EvaluationLimits {
                steps_per_resource: Some(work),
                ..EvaluationLimits::workbench()
            };
            assert_eq!(
                serde_json::to_value(evaluate_with_limits(&old, &sources, limits)).unwrap(),
                serde_json::to_value(evaluate_with_limits(&new, &sources, limits)).unwrap()
            );
        }
    }
}

#[test]
fn call_free_new_profile_is_explicit_and_measured_coverage_does_not_invent_fees() {
    assert_eq!(
        serde_json::to_value(Profile::default()).unwrap(),
        "intersect.workbench.v1"
    );
    for version in 1..=75 {
        let name = format!("intersect.xtext.native.v{version}");
        let profile: Profile = serde_json::from_value(json!(name)).unwrap();
        assert_eq!(profile.as_str(), name);
        assert_eq!(serde_json::to_value(profile).unwrap(), name);
    }
    let phase = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    let policy = Default::default();
    let plain = package(SMALL, Profile::NativeV75);
    let observed = ordinary::compile_with_context_measured(
        input(SMALL),
        Profile::NativeV75,
        &[],
        None,
        policy,
        policy,
        AnalysisObservers {
            phases: &phase,
            recursive_work: Some(&work),
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(plain).unwrap(),
        serde_json::to_value(observed).unwrap()
    );
    assert!(phase.first_failure().is_none());
    assert_eq!(
        work.snapshot().coverage,
        Coverage::CallFreeRelativeClosureUninstrumented
    );
    assert!(work.snapshot().current.is_none());
    assert_eq!(work.snapshot().stages[2].program_invocations, 0);
}
