use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/cached-proof-closures/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "closures.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn closure_reuse_preserves_decisions_models_and_limits() {
    for profile in [Profile::NativeV41, Profile::NativeV42] {
        closure_reuse_preserves_decisions_models_and_limits_for(profile);
    }
}
fn closure_reuse_preserves_decisions_models_and_limits_for(profile: Profile) {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV40,
    )
    .unwrap_err();
    assert!(
        format!("{old:?}").contains("Grammar prediction budget exceeded"),
        "{old:?}"
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            profile,
        )
        .unwrap();
    for (text, value) in [
        (
            include_str!("../../../../fixtures/xtext/cached-proof-closures/example.model")
                .to_owned(),
            "right",
        ),
        (format!("{}right", "y ".repeat(100)), "right"),
        ("x left".into(), "left"),
        (format!("{}left", "x y ".repeat(50)), "left"),
    ] {
        let result = evaluate(&package, &source(&text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        let root = &result.model.elements[0];
        assert_eq!(root.id, "closures.model#0");
        assert_eq!(root.values["operator"], value);
        assert_eq!((root.start, root.end), (0, text.len()));
        result.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&compiler::recompile(&package).unwrap(), &source(&text)).model,
            result.model
        );
        assert_eq!(
            evaluate_with_limits(
                &package,
                &source(&text),
                EvaluationLimits {
                    elements_per_resource: Some(1),
                    ..EvaluationLimits::native()
                }
            )
            .model,
            result.model
        );
        for limits in [
            EvaluationLimits {
                elements_per_resource: Some(0),
                ..EvaluationLimits::native()
            },
            EvaluationLimits {
                steps_per_resource: Some(1),
                ..EvaluationLimits::native()
            },
        ] {
            let failed = evaluate_with_limits(&package, &source(&text), limits);
            assert!(!failed.diagnostics.is_empty());
            assert!(failed.model.elements.is_empty());
        }
        assert_eq!(evaluate(&package, &source(&text)).model, result.model);
    }
    for text in [
        "x right".into(),
        "left".into(),
        format!("{}right", "x ".repeat(101)),
        format!("{}right trailing", "x ".repeat(100)),
    ] {
        let failed = evaluate(&package, &source(&text));
        assert!(!failed.diagnostics.is_empty(), "{text}");
        assert!(failed.model.elements.is_empty());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn short_forms_match_uncached_proof_execution() {
    for profile in [Profile::NativeV41, Profile::NativeV42] {
        short_forms_match_uncached_proof_execution_for(profile);
    }
}
fn short_forms_match_uncached_proof_execution_for(profile: Profile) {
    let short = GRAMMAR.replace(&"('x'|'y') ".repeat(100), &"('x'|'y') ".repeat(2));
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        &short,
        RCORE_BEHAVIOR,
        Profile::NativeV40,
    )
    .unwrap();
    let current =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &short,
            RCORE_BEHAVIOR,
            profile,
        )
        .unwrap();
    for (text, accepted) in [
        ("x y right", true),
        ("y left", true),
        ("x y left", true),
        ("x right", false),
        ("left", false),
        ("x y right trailing", false),
    ] {
        let expected = evaluate(&old, &source(text));
        let actual = evaluate(&current, &source(text));
        assert_eq!(expected.diagnostics.is_empty(), accepted, "{text}");
        assert_eq!(actual.diagnostics.is_empty(), accepted, "{text}");
        assert_eq!(actual.model, expected.model, "{text}");
    }
}
