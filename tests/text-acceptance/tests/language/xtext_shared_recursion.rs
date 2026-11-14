use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/shared-recursion/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "shared.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn shared_recursion_preserves_models_and_earlier_budget_boundary() {
    for profile in [Profile::NativeV38, Profile::NativeV39] {
        shared_recursion_preserves_models_and_earlier_budget_boundary_for(profile);
    }
}
fn shared_recursion_preserves_models_and_earlier_budget_boundary_for(profile: Profile) {
    // Historical capacity witness: request the historical 10k grant explicitly
    // (the default grant has been REALISTIC since 1e5cd75b).
    let old =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile_and_budget(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV37,
            intersect_text::grammar::GrammarAnalysisWorkBudget::LEGACY_10K,
        )
        .unwrap_err();
    assert!(format!("{old:?}").contains("Grammar prediction budget exceeded"));
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            profile,
        )
        .unwrap();
    let prefix = "p ".repeat(2500);
    let simple = include_str!("../../../../fixtures/xtext/shared-recursion/example.model");
    let nested = format!("{prefix}a({prefix}a) left");
    for (text, count) in [(simple, 1), (nested.as_str(), 2)] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), count);
        for (index, element) in result.model.elements.iter().enumerate() {
            assert_eq!(element.id, format!("shared.model#{index}"));
            assert_eq!(element.values["operator"], "a");
        }
        assert_eq!(result.model.elements[0].start, 0);
        assert_eq!(result.model.elements[0].end, text.len() - " left".len());
        if count == 2 {
            assert_eq!(
                result.model.elements[0].values["operand"],
                serde_json::json!(["shared.model#1"])
            );
            let child = &result.model.elements[1];
            assert_eq!(&text[child.start..child.end], format!("{prefix}a"));
        }
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
            let rejected = evaluate_with_limits(&package, &source(text), limits);
            assert!(!rejected.diagnostics.is_empty());
            assert!(rejected.model.elements.is_empty());
        }
        assert_eq!(evaluate(&package, &source(text)).model, result.model);
    }
    for text in [
        format!("{prefix}a right"),
        format!("{prefix}a( left"),
        format!("{simple} trailing"),
        "p a left".into(),
    ] {
        let rejected = evaluate(&package, &source(&text));
        assert!(!rejected.diagnostics.is_empty());
        assert!(rejected.model.elements.is_empty());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn short_shared_recursion_matches_existing_predicate_policy() {
    for profile in [Profile::NativeV38, Profile::NativeV39] {
        short_shared_recursion_matches_existing_predicate_policy_for(profile);
    }
}
fn short_shared_recursion_matches_existing_predicate_policy_for(profile: Profile) {
    let grammar = GRAMMAR.replace(&"'p' ".repeat(2500), "'p' ");
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        &grammar,
        RCORE_BEHAVIOR,
        Profile::NativeV37,
    )
    .unwrap();
    let current =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            profile,
        )
        .unwrap();
    for text in [
        "p a left",
        "p a(p a) left",
        "p a right",
        "p a(p a) right",
        "p a( left",
        "p a left trailing",
    ] {
        let expected = evaluate(&old, &source(text));
        let actual = evaluate(&current, &source(text));
        assert_eq!(
            actual.diagnostics.is_empty(),
            expected.diagnostics.is_empty(),
            "{text}"
        );
        assert_eq!(actual.model, expected.model, "{text}");
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn retained_independent_recursive_oracle_remains_satisfied() {
    for profile in [Profile::NativeV38, Profile::NativeV39] {
        retained_independent_recursive_oracle_remains_satisfied_for(profile);
    }
}
fn retained_independent_recursive_oracle_remains_satisfied_for(profile: Profile) {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            include_str!("../../../../fixtures/xtext/recursive-predicates/model.ecore"),
            include_str!("../../../../fixtures/xtext/recursive-predicates/model.xtext"),
            include_str!("../../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
            profile,
        )
        .unwrap();
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut compared = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["parser"] == "RecursiveEntryFirst")
    {
        let result = evaluate(&package, &source(row["text"].as_str().unwrap()));
        assert_eq!(
            result.diagnostics.is_empty(),
            row["accepted"].as_bool().unwrap(),
            "{row}"
        );
        if row["accepted"] == true {
            assert_eq!(result.model.elements[0].class, "Left");
        } else {
            assert!(result.model.elements.is_empty());
        }
        compared += 1;
    }
    assert_eq!(compared, 11);
}
