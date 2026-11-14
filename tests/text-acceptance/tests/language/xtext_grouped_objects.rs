use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str =
    include_str!("../../../../fixtures/xtext/grouped-object-assignments/model.xtext");
const TEXT: &str =
    include_str!("../../../../fixtures/xtext/grouped-object-assignments/example.model");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "grouped.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn scalar_and_many_grouped_assignments_preserve_containment() {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV35,
    )
    .unwrap_err();
    assert!(format!("{old:?}").contains("Assignments require"));
    let nested = GRAMMAR.replace("(Literal | Operator)", "((Literal | (Operator)))");
    for grammar in [GRAMMAR.to_owned(), nested] {
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV36,
            )
            .unwrap();
        for (text, count) in [(TEXT, 6), ("Demo = 4;", 2)] {
            let result = evaluate(&package, &source(text));
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(result.model.elements.len(), count);
            for (index, element) in result.model.elements.iter().enumerate() {
                assert_eq!(element.id, format!("grouped.model#{index}"));
            }
            assert_eq!(result.model.elements[0].values["value"], "grouped.model#1");
            assert_eq!(
                (result.model.elements[0].start, result.model.elements[0].end),
                (0, text.len())
            );
            if count == 6 {
                assert_eq!(result.model.elements[1].values["operator"], "sum");
                assert_eq!(
                    result.model.elements[1].values["operand"],
                    serde_json::json!(["grouped.model#2", "grouped.model#3"])
                );
                assert_eq!(
                    result.model.elements[3].values["operand"],
                    serde_json::json!(["grouped.model#4", "grouped.model#5"])
                );
                for (index, value) in [(2, 1), (4, 2), (5, 3)] {
                    let element = &result.model.elements[index];
                    assert_eq!(element.values["value"], value);
                    assert_eq!(&text[element.start..element.end], value.to_string());
                }
                let inner = &result.model.elements[3];
                assert_eq!(&text[inner.start..inner.end], "sum(2,3)");
            } else {
                assert_eq!(result.model.elements[1].values["value"], 4);
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
            "Demo = sum(1,);",
            "Demo = sum(1,sum(2,3);",
            "Demo = 4; trailing",
        ] {
            let rejected = evaluate(&package, &source(text));
            assert!(!rejected.diagnostics.is_empty());
            assert!(rejected.model.elements.is_empty());
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn grouped_rhs_types_cardinality_and_writability_stay_checked() {
    for rhs in [
        "(Literal | Model)",
        "(Literal | INT)",
        "(Literal | [k::Expression])",
        "(Literal?)",
        "(Literal Operator)",
        "(Literal | Choice)",
    ] {
        let grammar = GRAMMAR.replace("value=(Literal | Operator)", &format!("value={rhs}"))
            + "\nfragment Choice returns k::Expression: 'choice';";
        let error =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV36,
            )
            .unwrap_err();
        let diagnostic = if matches!(rhs, "(Literal?)" | "(Literal Operator)") {
            "Expected ')'"
        } else {
            "Grouped object assignment"
        };
        assert!(
            format!("{error:?}").contains(diagnostic),
            "{rhs}: {error:?}"
        );
    }
    let wrong_many = GRAMMAR.replace("value=(Literal | Operator)", "value+=(Literal | Operator)");
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &wrong_many,
            RCORE_BEHAVIOR,
            Profile::NativeV36,
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("Assignment cardinality differs"));
    for (schema, diagnostic) in [
        (
            ECORE.replace(
                "name=\"value\" containment=\"true\"",
                "name=\"value\" containment=\"false\"",
            ),
            "requires containment",
        ),
        (
            ECORE.replace(
                "name=\"value\" containment=\"true\"",
                "name=\"value\" changeable=\"false\" containment=\"true\"",
            ),
            "non-changeable",
        ),
    ] {
        let error =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &schema,
                GRAMMAR,
                RCORE_BEHAVIOR,
                Profile::NativeV36,
            )
            .unwrap_err();
        assert!(format!("{error:?}").contains(diagnostic), "{error:?}");
    }
}
