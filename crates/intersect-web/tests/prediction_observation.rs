//! Supplemental observation must preserve the entire original compiler result.
//! These controls are a review draft until the guarded packet is built/executed.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::grammar::admission_observation::Coverage;
use intersect_text::grammar::admission_observation::Phase;
use intersect_text::grammar::admission_observation::PredictionObservation;
use intersect_text::grammar::syntax;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use serde_json::{Value, json};

fn compilation(body: &str, uri: &str) -> BundleCompilation {
    let frozen: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-assignment-v66/native-before-v65.json"
    ))
    .unwrap();
    let mut result: BundleCompilation =
        serde_json::from_value(frozen["cases"][2]["selection"]["compilation"].clone()).unwrap();
    let old = result.xtext.take().unwrap();
    let original = result.bundle.source(&old).unwrap();
    let mut source = DefinitionSource::new(
        uri,
        original.kind,
        format!(
            "grammar observation.Sample hidden() import {} as u import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore {body}",
            serde_json::to_string(&result.namespace).unwrap()
        ),
    );
    source.dependencies = original.dependencies.clone();
    let new = source.id.clone();
    *result
        .bundle
        .sources
        .iter_mut()
        .find(|source| source.id == old)
        .unwrap() = source;
    for source in &mut result.bundle.sources {
        for dependency in &mut source.dependencies {
            if *dependency == old {
                *dependency = new.clone();
            }
        }
    }
    result.xtext = Some(new);
    result
}

fn run(
    compilation: BundleCompilation,
    profile: Profile,
    observation: Option<&PredictionObservation>,
) -> Value {
    let policy = CompilationSourcePolicy::default();
    let result = if let Some(observation) = observation {
        intersect_text::ordinary::compile_with_context_observed(
            compilation,
            profile,
            &[],
            None,
            policy,
            policy,
            observation,
        )
    } else {
        intersect_text::ordinary::compile_with_context(
            compilation,
            profile,
            &[],
            None,
            policy,
            policy,
        )
    };
    match result {
        Ok(package) => json!({"ok":true,"package":package}),
        Err(diagnostics) => json!({"ok":false,"diagnostics":diagnostics}),
    }
}

#[test]
fn observation_preserves_the_whole_successful_package_and_prediction_programs() {
    let source = compilation("Root returns u::Model: 'x';", "memory:phase/good.xtext");
    let observation = PredictionObservation::new();
    let ordinary = run(source.clone(), Profile::NativeV66, None);
    let observed = run(source, Profile::NativeV66, Some(&observation));
    assert_eq!(ordinary["ok"], true, "{ordinary}");
    assert_eq!(ordinary, observed);
    assert_eq!(observation.coverage(), Coverage::CoreSourceAnalysis);
    assert_eq!(observation.first_failure(), None);
}

#[test]
fn follow_conflict_observation_preserves_whole_ordinary_diagnostics_and_real_owner() {
    let source = compilation(
        "Root returns u::Model: F 'x'; fragment F returns u::Model: nodes+=B; B returns u::Node: name='x'?;",
        "memory:phase/follow-conflict.xtext",
    );
    let entry = source.xtext.as_ref().unwrap();
    let syntax = syntax::parse(
        source.bundle.source(entry).unwrap(),
        SourceLimits::default(),
    )
    .unwrap();
    let fragment = syntax
        .rules
        .iter()
        .find(|rule| rule.value.name.value == "F")
        .unwrap();
    let observation = PredictionObservation::new();
    let ordinary = run(source.clone(), Profile::NativeV70, None);
    let observed = run(source.clone(), Profile::NativeV70, Some(&observation));
    assert_eq!(ordinary["ok"], false, "{ordinary}");
    assert_eq!(ordinary, observed);
    assert_eq!(
        ordinary["diagnostics"][0]["message"],
        "Fragment caller follow conflict requires unsupported Xtext prediction"
    );
    let failure = observation.first_failure().unwrap();
    assert_eq!(failure.phase, Phase::Follow);
    assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "F");
    assert_eq!(failure.source.unwrap().uri.as_str().unwrap(), entry.uri);
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        entry.sha256
    );
    assert_eq!(
        failure.declaration_span,
        Some((fragment.start, fragment.end))
    );
    assert_eq!(failure.decision.unwrap().as_str().unwrap(), "rule/F/value");
    assert!(failure.charged_work.is_some());
    assert_eq!(failure.syntax_depth, None);
    assert_eq!(failure.call_depth, None);
    assert_eq!(failure.state_count, None);
    assert_eq!(failure.repeated_state_count, None);
}

#[test]
fn observed_rule_failure_uses_its_real_retained_source_and_span() {
    let source = compilation(
        "Root returns u::Model: 'x' ('y' | 'y');",
        "memory:phase/ambiguous.xtext",
    );
    let entry = source.xtext.as_ref().unwrap();
    let syntax = syntax::parse(
        source.bundle.source(entry).unwrap(),
        SourceLimits::default(),
    )
    .unwrap();
    let root = syntax
        .rules
        .iter()
        .find(|rule| rule.value.name.value == "Root")
        .unwrap();
    let observation = PredictionObservation::new();
    let ordinary = run(source.clone(), Profile::NativeV66, None);
    let observed = run(source.clone(), Profile::NativeV66, Some(&observation));
    assert_eq!(ordinary["ok"], false, "{ordinary}");
    assert_eq!(ordinary, observed);
    let failure = observation.first_failure().unwrap();
    assert!(matches!(
        failure.phase,
        Phase::PredictionCheck | Phase::DeferredProof
    ));
    let captured = failure.source.unwrap();
    assert_eq!(captured.uri.as_str().unwrap(), entry.uri);
    assert_eq!(captured.sha256.as_str().unwrap(), entry.sha256);
    assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "Root");
    assert_eq!(failure.declaration_span, Some((root.start, root.end)));
    assert!(!failure.capture_unavailable);
    // Since 1e5cd75b a deferred-proof failure carries the work charged to
    // the shared deferred-proof pool; no other counter is measured here.
    assert_eq!(
        failure.charged_work.is_some(),
        failure.phase == Phase::DeferredProof,
        "{failure:?}"
    );
    assert!(
        [
            failure.syntax_depth,
            failure.call_depth,
            failure.state_count,
            failure.repeated_state_count,
        ]
        .iter()
        .all(Option::is_none),
        "{failure:?}"
    );
}

#[test]
fn global_state_exhaustion_is_localized_to_phase_without_inventing_an_owner() {
    let body = format!(
        "Root returns u::Model: 'x'; terminal WIDE returns ecore::EString: '{}';",
        "a".repeat(10_001)
    );
    let source = compilation(&body, "memory:phase/wide.xtext");
    let observation = PredictionObservation::new();
    let ordinary = run(source.clone(), Profile::NativeV66, None);
    let observed = run(source.clone(), Profile::NativeV66, Some(&observation));
    assert_eq!(ordinary["ok"], false, "{ordinary}");
    assert_eq!(ordinary, observed);
    assert_eq!(
        ordinary["diagnostics"][0]["message"],
        "Grammar prediction budget exceeded"
    );
    let failure = observation.first_failure().unwrap();
    assert_eq!(failure.phase, Phase::LexicalProgram);
    assert_eq!(failure.owner_rule, None);
    assert_eq!(failure.declaration_span, None);
    assert_eq!(failure.decision, None);
    assert_eq!(
        failure.source.unwrap().uri.as_str().unwrap(),
        source.xtext.unwrap().uri
    );
    assert_eq!(failure.charged_work, None);
    assert_eq!(failure.state_count, None);
}

#[test]
fn bounded_capture_never_truncates_identity_or_changes_the_compilation_result() {
    let body = format!(
        "Root returns u::Model: 'x'; terminal WIDE returns ecore::EString: '{}';",
        "a".repeat(10_001)
    );
    let source = compilation(&body, &format!("memory:phase/{}", "u".repeat(600)));
    let observation = PredictionObservation::new();
    let ordinary = run(source.clone(), Profile::NativeV66, None);
    let observed = run(source, Profile::NativeV66, Some(&observation));
    assert_eq!(ordinary["ok"], false);
    assert_eq!(ordinary, observed);
    let failure = observation.first_failure().unwrap();
    assert!(failure.capture_unavailable);
    assert_eq!(failure.source, None);
}

#[test]
fn unsupported_observation_coverage_preserves_legacy_results_and_reset_fences_jobs() {
    let bad = compilation(
        "Root returns u::Model: 'x' ('y' | 'y');",
        "memory:phase/old.xtext",
    );
    let good = compilation("Root returns u::Model: 'x';", "memory:phase/next.xtext");
    let observation = PredictionObservation::new();
    assert_eq!(
        run(bad, Profile::NativeV66, Some(&observation))["ok"],
        false
    );
    assert!(observation.first_failure().is_some());
    let ordinary = run(good.clone(), Profile::NativeV64, None);
    let observed = run(good, Profile::NativeV64, Some(&observation));
    assert_eq!(ordinary, observed);
    assert_eq!(observation.coverage(), Coverage::LegacyFrontend);
    assert_eq!(observation.first_failure(), None);
}

#[test]
fn no_xtext_coverage_preserves_the_entire_structure_only_result() {
    let mut source = compilation(
        "Root returns u::Model: 'x';",
        "memory:phase/structure.xtext",
    );
    source.xtext = None;
    let observation = PredictionObservation::new();
    let ordinary = run(source.clone(), Profile::NativeV66, None);
    let observed = run(source, Profile::NativeV66, Some(&observation));
    assert_eq!(ordinary, observed);
    assert_eq!(observation.coverage(), Coverage::NoXtext);
    assert_eq!(observation.first_failure(), None);
}

#[test]
fn source_program_coverage_is_explicitly_absent_without_changing_its_result() {
    let source = compilation(
        "Root returns u::Model: Item<true>; Item<Flag> returns u::Model: <Flag> 'x' | <!Flag> 'y';",
        "memory:phase/parameters.xtext",
    );
    let observation = PredictionObservation::new();
    let ordinary = run(source.clone(), Profile::NativeV66, None);
    let observed = run(source, Profile::NativeV66, Some(&observation));
    assert_eq!(ordinary, observed);
    assert_eq!(observation.coverage(), Coverage::SourceProgramFrontend);
    assert_eq!(observation.first_failure(), None);
}
