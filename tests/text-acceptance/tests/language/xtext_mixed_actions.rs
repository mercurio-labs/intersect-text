use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str = r#"{"version":1,"scope":"unique_names","derived":[],"validations":[],"initializers":[{"class":"OperatorExpression","provider":{"name":"intersect.constant-attributes","version":"1"},"members":["operator"],"values":{"operator":"initialized"}}]}"#;
fn grammar() -> String {
    include_str!("../../../../fixtures/xtext/current-actions/model.xtext").replace(
        "operand+=Literal)*;",
        "operand+=Literal)* | {k::OperatorExpression} 'prefix' operand+=Literal;",
    )
}
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "mixed.kerml".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn mixed_branches_share_construction_and_initializers() {
    let grammar = grammar();
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV25
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV26,
        )
        .unwrap();
    for (text, count, root) in [
        ("Demo = 1;", 2, 1),
        ("Demo = 1 + 2;", 4, 2),
        ("Demo = prefix 7;", 3, 1),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(
            result.diagnostics.is_empty(),
            "{text}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.model.elements.len(), count);
        assert_eq!(
            result.model.elements[0].values["value"],
            format!("mixed.kerml#{root}")
        );
        for (index, element) in result.model.elements.iter().enumerate() {
            assert_eq!(element.id, format!("mixed.kerml#{index}"));
        }
        if count > 2 {
            let operator = &result.model.elements[root];
            assert_eq!(operator.class, "OperatorExpression");
            assert_eq!(
                operator.initialization.as_ref().unwrap().object,
                operator.id
            );
            assert_eq!(
                operator.values["operator"],
                if text.contains("prefix") {
                    "initialized"
                } else {
                    "+"
                }
            );
            assert_eq!(
                operator.values["operand"],
                if text.contains("prefix") {
                    json!(["mixed.kerml#2"])
                } else {
                    json!(["mixed.kerml#1", "mixed.kerml#3"])
                }
            );
            assert_eq!(
                &text[operator.start..operator.end],
                if text.contains("prefix") {
                    "prefix 7"
                } else {
                    "1 + 2"
                }
            );
        }
        result.model.validate(&package.schema).unwrap();
        let replay = compiler::recompile(&package).unwrap();
        assert_eq!(evaluate(&replay, &source(text)).model, result.model);
        let exact = EvaluationLimits {
            elements_per_resource: Some(count),
            ..EvaluationLimits::native()
        };
        assert_eq!(
            evaluate_with_limits(&package, &source(text), exact).model,
            result.model
        );
        let failed = evaluate_with_limits(
            &package,
            &source(text),
            EvaluationLimits {
                elements_per_resource: Some(count - 1),
                ..exact
            },
        );
        assert!(!failed.diagnostics.is_empty());
        assert!(failed.model.elements.is_empty());
    }
    for text in [
        "Demo = prefix;",
        "Demo = prefix 2147483648;",
        "Demo = prefix 7 +;",
        "Demo = 1 + 2 -;",
        "Demo = prefix 7; trailing",
    ] {
        let failed = evaluate(&package, &source(text));
        assert!(!failed.diagnostics.is_empty(), "{text}");
        assert!(failed.model.elements.is_empty());
    }
    let failed = evaluate_with_limits(
        &package,
        &source("Demo = prefix 7;"),
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..EvaluationLimits::native()
        },
    );
    assert!(!failed.diagnostics.is_empty());
    assert!(failed.model.elements.is_empty());
    assert!(
        evaluate(&package, &source("Demo = prefix 7;"))
            .diagnostics
            .is_empty()
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn mixed_action_flow_rejects_invalid_creation_paths() {
    let grammar = grammar();
    for action in [
        "{k::Expression}",
        "{k::Model}",
        "Literal {k::OperatorExpression}",
        "({k::OperatorExpression})?",
        "({k::OperatorExpression})+",
        "{k::OperatorExpression} {k::OperatorExpression}",
    ] {
        let invalid = grammar.replace(
            "{k::OperatorExpression} 'prefix'",
            &format!("{action} 'prefix'"),
        );
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &invalid,
                RCORE_BEHAVIOR,
                Profile::NativeV26
            )
            .is_err(),
            "{action}"
        );
    }
    let invalid = ECORE.replace(
        "name=\"OperatorExpression\"",
        "name=\"OperatorExpression\" interface=\"true\"",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &invalid,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV26
        )
        .is_err()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn simple_action_alternative_without_capture_uses_same_flow() {
    let grammar=grammar().replace("Literal ({k::OperatorExpression.operand += current} operator=AdditiveOperator operand+=Literal)*", "Literal");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV25
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV26,
        )
        .unwrap();
    for (text, count) in [("Demo = 1;", 2), ("Demo = prefix 7;", 3)] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), count);
        assert_eq!(result.model.elements[0].values["value"], "mixed.kerml#1");
        if count == 3 {
            let operator = &result.model.elements[1];
            assert_eq!(operator.class, "OperatorExpression");
            assert_eq!(operator.values["operator"], "initialized");
            assert_eq!(
                operator.initialization.as_ref().unwrap().object,
                operator.id
            );
            assert_eq!(operator.values["operand"], json!(["mixed.kerml#2"]));
        }
    }
    let invalid = grammar.replace("Literal |", "'empty' |");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &invalid,
            RCORE_BEHAVIOR,
            Profile::NativeV26
        )
        .is_err()
    );
}
