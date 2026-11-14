use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str =
    include_str!("../../../../fixtures/xtext/relative-proof-closures/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "relative.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn relative_closures_keep_exact_caller_suffixes_and_model_contracts() {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV41,
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
            Profile::NativeV42,
        )
        .unwrap();
    for (text, value) in [
        (
            include_str!("../../../../fixtures/xtext/relative-proof-closures/example.model"),
            "right",
        ),
        ("x x x x x x p left", "left"),
        ("y y y y y y p right", "right"),
        ("y x y x y x p left", "left"),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        let root = &result.model.elements[0];
        assert_eq!(root.id, "relative.model#0");
        assert_eq!(root.values["operator"], value);
        assert_eq!((root.start, root.end), (0, text.len()));
        result.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&compiler::recompile(&package).unwrap(), &source(text)).model,
            result.model
        );
        assert_eq!(
            evaluate_with_limits(
                &package,
                &source(text),
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
            let failed = evaluate_with_limits(&package, &source(text), limits);
            assert!(!failed.diagnostics.is_empty());
            assert!(failed.model.elements.is_empty());
        }
        assert_eq!(evaluate(&package, &source(text)).model, result.model);
    }
    for text in [
        "x x x x x p left",
        "x x x x x x x p right",
        "x x x x x x left",
        "x x x x x x p right trailing",
    ] {
        let failed = evaluate(&package, &source(text));
        assert!(!failed.diagnostics.is_empty(), "{text}");
        assert!(failed.model.elements.is_empty());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn relative_closures_preserve_ambiguity_rejection() {
    let ambiguous = GRAMMAR.replace("Head0 operator='right';", "Head0 operator='left';");
    let failed =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &ambiguous,
            RCORE_BEHAVIOR,
            Profile::NativeV42,
        )
        .unwrap_err();
    assert!(
        format!("{failed:?}").contains("Ambiguous parser prediction"),
        "{failed:?}"
    );
}
