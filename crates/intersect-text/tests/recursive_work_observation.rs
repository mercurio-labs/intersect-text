//! Separate diagnostic counters over the same public ordinary compiler path.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::grammar::admission_observation::AnalysisObservers;
use intersect_text::grammar::admission_observation::Coverage;
use intersect_text::grammar::admission_observation::PredictionObservation;
use intersect_text::grammar::admission_observation::RecursiveAnalysisCounters;
use intersect_text::ordinary;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};

fn input(text: &str) -> (BundleCompilation, DefinitionSource) {
    let model = DefinitionSource::new(
        "memory:work/model.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
    );
    let mut source = DefinitionSource::new("memory:work/grammar.xtext", SourceKind::Xtext, text);
    source.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:work/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package fixtureProviders version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:intersect:xtext:recursive:1".into(),
        xtext: Some(source.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), source.clone(), rules],
            namespaces: vec![],
        },
    };
    let migration = intersect_model::migration::migrate_ecore_bundle_with_limits(
        input.bundle.clone(),
        &[(model.id.clone(), "memory:work/model.rcore".into())].into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migration.reports[0].original, model);
    (
        intersect_text::migration::remap_compilation(&migration, &input).unwrap(),
        source,
    )
}
#[test]
fn measured_ordinary_success_preserves_whole_package_old_observation_and_real_origins() {
    let (input, source) = input(include_str!(
        "../../../fixtures/xtext/recursive-predicates/model.xtext"
    ));
    let policy = CompilationSourcePolicy::default();
    let old = PredictionObservation::new();
    let phase = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    let plain = ordinary::compile_with_context(
        input.clone(),
        Profile::NativeV73,
        &[],
        None,
        policy,
        policy,
    )
    .unwrap();
    let observed = ordinary::compile_with_context_observed(
        input.clone(),
        Profile::NativeV73,
        &[],
        None,
        policy,
        policy,
        &old,
    )
    .unwrap();
    let measured = ordinary::compile_with_context_measured(
        input,
        Profile::NativeV73,
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
        serde_json::to_value(&plain).unwrap(),
        serde_json::to_value(observed).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&plain).unwrap(),
        serde_json::to_value(measured).unwrap()
    );
    assert_eq!(old.first_failure(), phase.first_failure());
    assert_eq!(old.coverage(), phase.coverage());
    let counts = work.snapshot();
    assert_eq!(counts.coverage, Coverage::CoreSourceAnalysis);
    assert!(!counts.overflowed);
    assert!(!counts.inconsistent);
    assert!(counts.last_program.unwrap().conserved);
    assert!(counts.last_program.unwrap().completed);
    assert!(counts.stages[2].decisions_started > 0);
    assert!(counts.current.is_none());
    for context in counts.heaviest_completed.iter().flatten() {
        assert!(!context.capture_unavailable);
        let owner = context.owner_rule.unwrap();
        let owner = owner.as_str().unwrap();
        let expected = &plain.grammar.as_ref().unwrap().declaration_sources[owner];
        let identity = context.source.unwrap();
        assert_eq!(identity.uri.as_str().unwrap(), source.id.uri);
        assert_eq!(identity.sha256.as_str().unwrap(), source.id.sha256);
        assert_eq!(
            context.declaration_span,
            Some((expected.start, expected.end))
        );
        assert_eq!(expected.source, source.id);
    }
}
#[test]
fn measured_bound_negative_preserves_whole_error_old_phase_and_conserves_one_bucket() {
    let (input, source) = input(include_str!(
        "../../../fixtures/xtext-recursive-closure-v73/Convergent.xtext"
    ));
    let policy = CompilationSourcePolicy::default();
    let old = PredictionObservation::new();
    let phase = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    // Historical capacity witness: request the historical 10k grant explicitly
    // (the default grant has been REALISTIC since 1e5cd75b).
    let legacy = intersect_text::grammar::GrammarAnalysisWorkBudget::LEGACY_10K;
    let plain = ordinary::compile_with_context_and_budget(
        input.clone(),
        Profile::NativeV73,
        &[],
        None,
        policy,
        policy,
        legacy,
    )
    .unwrap_err();
    let observed = ordinary::compile_with_context_work_observed_and_budget(
        input.clone(),
        Profile::NativeV73,
        &[],
        None,
        policy,
        policy,
        AnalysisObservers::phases(&old).into(),
        legacy,
    )
    .unwrap_err();
    let measured = ordinary::compile_with_context_work_observed_and_budget(
        input,
        Profile::NativeV73,
        &[],
        None,
        policy,
        policy,
        AnalysisObservers {
            phases: &phase,
            recursive_work: Some(&work),
        }
        .into(),
        legacy,
    )
    .unwrap_err();
    assert_eq!(plain, observed);
    assert_eq!(plain, measured);
    assert_eq!(old.first_failure(), phase.first_failure());
    assert_eq!(old.coverage(), phase.coverage());
    let counts = work.snapshot();
    let program = counts.last_program.unwrap();
    assert_eq!(program.initial, 10000);
    assert_eq!(program.remaining, 0);
    assert!(program.conserved);
    assert!(!program.completed);
    assert_eq!(program.granted.iter().sum::<u64>(), 10000);
    assert_eq!(program.denied.iter().sum::<u64>(), 1);
    assert!(!counts.overflowed);
    assert!(!counts.inconsistent);
    let current = counts.current.unwrap();
    assert!(current.failed);
    assert!(!current.capture_unavailable);
    let identity = current.source.unwrap();
    assert_eq!(identity.uri.as_str().unwrap(), source.id.uri);
    assert_eq!(identity.sha256.as_str().unwrap(), source.id.sha256);
    let owner = current.owner_rule.unwrap();
    let parsed = intersect_text::grammar::syntax::parse(&source, SourceLimits::default()).unwrap();
    let declaration = parsed
        .rules
        .iter()
        .find(|r| r.value.name.value == owner.as_str().unwrap())
        .unwrap();
    assert_eq!(
        current.declaration_span,
        Some((declaration.start, declaration.end))
    );
}
#[test]
fn measured_source_preflight_resets_and_never_changes_existing_failure() {
    let (input, _) = input(include_str!(
        "../../../fixtures/xtext/recursive-predicates/model.xtext"
    ));
    let policy = CompilationSourcePolicy::default();
    let work = RecursiveAnalysisCounters::new();
    let phase = PredictionObservation::new();
    ordinary::compile_with_context_measured(
        input.clone(),
        Profile::NativeV73,
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
    assert!(work.snapshot().stages[2].program_invocations > 0);
    let requested = CompilationSourcePolicy {
        xtext_bytes: policy.xtext_bytes + 1,
        ..policy
    };
    let plain = ordinary::compile_with_context(
        input.clone(),
        Profile::NativeV73,
        &[],
        None,
        requested,
        policy,
    )
    .unwrap_err();
    let measured = ordinary::compile_with_context_measured(
        input,
        Profile::NativeV73,
        &[],
        None,
        requested,
        policy,
        AnalysisObservers {
            phases: &phase,
            recursive_work: Some(&work),
        },
    )
    .unwrap_err();
    assert_eq!(plain, measured);
    assert_eq!(work.snapshot(), Default::default());
    assert_eq!(phase.coverage(), Coverage::NotReached);
}

#[test]
fn native80_partial_counters_preserve_whole_package_phase_and_v76_exact_rejection() {
    use intersect_text::grammar::admission_observation::AdjacentWorkObservation;
    use intersect_text::grammar::admission_observation::AnalysisWorkObservers;
    let (input, _) = input(include_str!(
        "../../../fixtures/xtext/recursive-predicates/model.xtext"
    ));
    let policy = CompilationSourcePolicy::default();
    let old_phase = PredictionObservation::new();
    let phase = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    let plain = ordinary::compile_with_context(
        input.clone(),
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
    )
    .unwrap();
    let observed = ordinary::compile_with_context_observed(
        input.clone(),
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
        &old_phase,
    )
    .unwrap();
    let measured = ordinary::compile_with_context_measured(
        input.clone(),
        Profile::NativeV80,
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
        serde_json::to_value(&plain).unwrap(),
        serde_json::to_value(observed).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&plain).unwrap(),
        serde_json::to_value(measured).unwrap()
    );
    assert_eq!(old_phase.first_failure(), phase.first_failure());
    assert_eq!(old_phase.coverage(), phase.coverage());
    let partial = work.snapshot();
    assert_eq!(
        partial.coverage,
        Coverage::AdjacentClosureSourceUninstrumented
    );
    assert!(!partial.overflowed && !partial.inconsistent);
    assert!(partial.stages[2].program_invocations > 0);
    assert!(partial.stages[2].decisions_started > 0);
    let program = partial.last_program.unwrap();
    assert!(program.completed);
    let granted = program.granted.iter().sum::<u64>();
    assert!(granted > 0);
    assert!(granted <= u64::try_from(program.initial - program.remaining).unwrap());
    // The old seven categories are a subset, not a claim that newly paid
    // source/template/frontier/growth/range operations cost zero.
    let exact = AdjacentWorkObservation::new();
    let result = ordinary::compile_with_context_work_observed(
        input,
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
        AnalysisWorkObservers {
            legacy: AnalysisObservers {
                phases: &phase,
                recursive_work: Some(&work),
            },
            adjacent_work: Some(&exact),
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(plain).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    assert_eq!(work.snapshot(), partial);
    let unavailable = exact.snapshot();
    assert!(unavailable.unsupported_profile && !unavailable.covered_v76);
    assert_eq!(unavailable.programs, 0);
    assert_eq!(
        std::mem::size_of::<AnalysisObservers<'_>>(),
        2 * std::mem::size_of::<usize>()
    );
    println!("NATIVE80_PARTIAL_OBSERVATION {:?}", partial);
}

#[test]
fn native80_partial_reset_preserves_preflight_error_and_older_coverage() {
    let (input, _) = input(include_str!(
        "../../../fixtures/xtext/recursive-predicates/model.xtext"
    ));
    let policy = CompilationSourcePolicy::default();
    let phase = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    for profile in [Profile::NativeV78, Profile::NativeV79] {
        let plain =
            ordinary::compile_with_context(input.clone(), profile, &[], None, policy, policy);
        let measured = ordinary::compile_with_context_measured(
            input.clone(),
            profile,
            &[],
            None,
            policy,
            policy,
            AnalysisObservers {
                phases: &phase,
                recursive_work: Some(&work),
            },
        );
        assert_eq!(
            serde_json::to_value(plain).unwrap(),
            serde_json::to_value(measured).unwrap()
        );
        assert_eq!(
            work.snapshot().coverage,
            Coverage::AdjacentClosureSourceUninstrumented
        );
        assert_eq!(work.snapshot().stages[2].program_invocations, 0);
    }
    ordinary::compile_with_context_measured(
        input.clone(),
        Profile::NativeV80,
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
    assert!(work.snapshot().stages[2].program_invocations > 0);
    let refused = CompilationSourcePolicy {
        xtext_bytes: policy.xtext_bytes + 1,
        ..policy
    };
    let plain = ordinary::compile_with_context(
        input.clone(),
        Profile::NativeV80,
        &[],
        None,
        refused,
        policy,
    )
    .unwrap_err();
    let measured = ordinary::compile_with_context_measured(
        input,
        Profile::NativeV80,
        &[],
        None,
        refused,
        policy,
        AnalysisObservers {
            phases: &phase,
            recursive_work: Some(&work),
        },
    )
    .unwrap_err();
    assert_eq!(plain, measured);
    assert_eq!(work.snapshot(), Default::default());
    assert_eq!(phase.coverage(), Coverage::NotReached);
}
