use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/current-fragments/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "fragment.kerml".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn fragments_preserve_captured_and_explicit_current_objects() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV28
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV29,
        )
        .unwrap();
    for (text, count, operator, index, operands) in [
        (
            include_str!("../../../../fixtures/xtext/current-fragments/example.kerml").trim(),
            5,
            "+",
            2,
            3,
        ),
        ("Demo = prefix all(4,5);", 4, "all", 1, 2),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), count);
        let current = &result.model.elements[index];
        assert_eq!(current.values["operator"], operator);
        assert_eq!(
            current.values["operand"].as_array().unwrap().len(),
            operands
        );
        assert_eq!(&text[current.start..current.end], &text[7..text.len() - 1]);
        let expected: Vec<_> = if index == 2 {
            vec![1, 3, 4]
        } else {
            vec![2, 3]
        }
        .into_iter()
        .map(|i| serde_json::Value::String(format!("fragment.kerml#{i}")))
        .collect();
        assert_eq!(
            current.values["operand"],
            serde_json::Value::Array(expected)
        );
        assert_eq!(result.model.elements[0].values["value"], current.id);
        for (i, e) in result.model.elements.iter().enumerate() {
            assert_eq!(e.id, format!("fragment.kerml#{i}"));
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
        let rejected = evaluate_with_limits(
            &package,
            &source(text),
            EvaluationLimits {
                elements_per_resource: Some(count - 1),
                ..EvaluationLimits::native()
            },
        );
        assert!(!rejected.diagnostics.is_empty());
        assert!(rejected.model.elements.is_empty());
    }
    for text in [
        "Demo = 1 + ;",
        "Demo = prefix all(4,);",
        "Demo = 1 + 2 trailing;",
    ] {
        let rejected = evaluate(&package, &source(text));
        assert!(!rejected.diagnostics.is_empty());
        assert!(rejected.model.elements.is_empty());
        assert!(
            evaluate(&package, &source("Demo = 1 + 2;"))
                .diagnostics
                .is_empty()
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn fragments_require_compatible_existing_owners_and_writable_members() {
    for grammar in [
        GRAMMAR.replace(
            "fragment Suffix returns k::OperatorExpression",
            "fragment Suffix returns k::Literal",
        ),
        GRAMMAR.replace("{k::OperatorExpression} 'prefix' Prefix", "'prefix' Prefix"),
        GRAMMAR.replace("operator='+'", "absent='+'"),
        GRAMMAR.replace("Suffix)*", "operand+=Suffix)*"),
        GRAMMAR.replace("operator='+' Operands", "operator='+' Literal"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV29
            )
            .is_err(),
            "{grammar}"
        );
    }
    let readonly = ECORE.replace(
        "name=\"operator\"",
        "name=\"operator\" changeable=\"false\"",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &readonly,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV29
        )
        .is_err()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn fragments_check_actual_action_class_feature_contracts() {
    let ecore=ECORE.replace("</ecore:EPackage>",r##"<eClassifiers xsi:type="ecore:EClass" name="NarrowOperator" eSuperTypes="#//OperatorExpression"/></ecore:EPackage>"##);
    let grammar = GRAMMAR.replace("{k::OperatorExpression", "{k::NarrowOperator");
    let valid =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &ecore,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV29,
        )
        .unwrap();
    assert!(
        evaluate(&valid, &source("Demo = prefix all(1,2);"))
            .diagnostics
            .is_empty()
    );
    // Synthetic invalid-schema control, not imported Ecore redefinition support.
    // A more restrictive actual caller must not bypass the inline checks through
    // a fragment whose declared superclass still has a writable member.
    let mut schema = valid.schema.clone();
    let mut field = schema.features("NarrowOperator").unwrap()["operator"].clone();
    field.changeable = false;
    let class = schema.classes.get_mut("NarrowOperator").unwrap();
    class.features.insert("operator".into(), field);
    class.feature_order.push("operator".into());
    let inline = grammar.replace(" Suffix)", " operator='+' Operands)");
    let direct =
        intersect_text::grammar::compile_with_profile(&inline, &schema, Profile::NativeV29)
            .unwrap_err();
    let delegated =
        intersect_text::grammar::compile_with_profile(&grammar, &schema, Profile::NativeV29)
            .unwrap_err();
    assert_eq!(delegated.message, direct.message);
    assert_eq!(delegated.source, "xtext");
}
