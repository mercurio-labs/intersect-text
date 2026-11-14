use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/branching-recursion/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "branching.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn depth_first_analysis_preserves_recursive_models_and_limits() {
    // Historical capacity witness: request the historical 10k grant explicitly
    // (the default grant has been REALISTIC since 1e5cd75b).
    let old =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile_and_budget(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV38,
            intersect_text::grammar::GrammarAnalysisWorkBudget::LEGACY_10K,
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
            Profile::NativeV39,
        )
        .unwrap();
    let simple = include_str!("../../../../fixtures/xtext/branching-recursion/example.model");
    let nested = format!("{}a({}a) left", "a x ".repeat(10), "a y ".repeat(10));
    for (text, count) in [
        (simple.to_owned(), 11),
        (simple.replace('x', "y"), 11),
        (nested, 22),
    ] {
        let result = evaluate(&package, &source(&text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), count);
        assert_eq!(result.model.elements[0].start, 0);
        assert_eq!(result.model.elements[0].end, text.len() - 5);
        for (index, element) in result.model.elements.iter().enumerate() {
            assert_eq!(element.id, format!("branching.model#{index}"));
            assert_eq!(element.values["operator"], "a");
            if index + 1 < count {
                assert_eq!(
                    element.values["operand"],
                    serde_json::json!([format!("branching.model#{}", index + 1)])
                );
                let child = &result.model.elements[index + 1];
                assert!(element.start <= child.start && child.end <= element.end);
            }
        }
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
                    elements_per_resource: Some(count),
                    ..EvaluationLimits::native()
                }
            )
            .model,
            result.model
        );
        for limits in [
            EvaluationLimits {
                elements_per_resource: Some(count - 1),
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
        simple.replace("left", "right"),
        simple.replacen("a x ", "", 1),
        format!("{simple} trailing"),
        "a x left".into(),
    ] {
        let failed = evaluate(&package, &source(&text));
        assert!(!failed.diagnostics.is_empty(), "{text}");
        assert!(failed.model.elements.is_empty());
    }
}
