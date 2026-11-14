use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/current-actions/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
fn source(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "current.kerml".into(),
        text: text.into(),
    }]
}
#[test]
fn assigned_current_constructs_left_associated_objects_and_exact_spans() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV5,
        )
        .unwrap();
    let result = evaluate(&package, &source("Demo = 1 + 2 - 3;"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let elements = &result.model.elements;
    assert_eq!(elements.len(), 6);
    for (i, class, start, end) in [
        (0, "Model", 0, 17),
        (1, "Literal", 7, 8),
        (2, "OperatorExpression", 7, 12),
        (3, "Literal", 11, 12),
        (4, "OperatorExpression", 7, 16),
        (5, "Literal", 15, 16),
    ] {
        assert_eq!(elements[i].class, class);
        assert_eq!(elements[i].id, format!("current.kerml#{i}"));
        assert_eq!((elements[i].start, elements[i].end), (start, end));
    }
    assert_eq!(elements[0].values["value"], "current.kerml#4");
    assert_eq!(
        elements[2].values["operand"],
        json!(["current.kerml#1", "current.kerml#3"])
    );
    assert_eq!(
        elements[4].values["operand"],
        json!(["current.kerml#2", "current.kerml#5"])
    );
    assert_eq!(elements[2].values["operator"], "+");
    assert_eq!(elements[4].values["operator"], "-");
    for (i, value) in [(1, 1), (3, 2), (5, 3)] {
        assert_eq!(elements[i].values["value"], value);
    }
}
#[test]
fn current_repetition_rolls_back_tentative_wrappers_and_preserves_exact_limits() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV5,
        )
        .unwrap();
    for (text, count) in [("Demo = 1;", 2), ("Demo = 1 + 2 - 3;", 6)] {
        let mut limits = EvaluationLimits::native();
        limits.elements_per_resource = Some(count);
        let result = evaluate_with_limits(&package, &source(text), limits);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), count);
        limits.elements_per_resource = Some(count - 1);
        let result = evaluate_with_limits(&package, &source(text), limits);
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty());
    }
    for text in ["Demo = 1 +;", "Demo = 1 + 2 -;", "Demo = + 1;"] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty());
    }
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV4
        )
        .is_err()
    );
}

#[test]
fn scalar_current_capture_uses_the_same_shared_constructor() {
    let ecore=ECORE.replace("upperBound=\"-1\"", "upperBound=\"1\"")
  .replace("name=\"operand\" containment", "name=\"right\" containment")
  .replace("  </eClassifiers>\n</ecore:EPackage>", "    <eStructuralFeatures xsi:type=\"ecore:EReference\" name=\"operand\" containment=\"true\" eType=\"#//Expression\"/>\n  </eClassifiers>\n</ecore:EPackage>");
    let xtext = XTEXT
        .replace(".operand += current", ".operand = current")
        .replace("operand+=Literal", "right=Literal");
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &ecore,
            &xtext,
            RCORE_BEHAVIOR,
            Profile::NativeV5,
        )
        .unwrap();
    let result = evaluate(&package, &source("Demo = 1 + 2 - 3;"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 6);
    assert_eq!(
        result.model.elements[4].values["operand"],
        "current.kerml#2"
    );
    assert_eq!(result.model.elements[4].values["right"], "current.kerml#5");
    assert_eq!(
        result.model.elements[2].values["operand"],
        "current.kerml#1"
    );
}
#[test]
fn capture_rejects_missing_current_and_invalid_feature_contracts() {
    for (ecore, xtext) in [
        (
            ECORE.to_owned(),
            XTEXT.replace("Expression: Literal (", "Expression: ("),
        ),
        (
            ECORE.to_owned(),
            XTEXT.replace(".operand += current", ".missing += current"),
        ),
        (
            ECORE.to_owned(),
            XTEXT.replace(".operand += current", ".operator = current"),
        ),
        (
            ECORE.to_owned(),
            XTEXT.replace(".operand += current", ".operand = current"),
        ),
        (
            ECORE.replace(
                "name=\"operand\"",
                "name=\"operand\" derived=\"true\" transient=\"true\" volatile=\"true\"",
            ),
            XTEXT.to_owned(),
        ),
        (
            ECORE.replace("name=\"operand\"", "name=\"operand\" changeable=\"false\""),
            XTEXT.to_owned(),
        ),
        (
            ECORE.replace(
                "name=\"operand\" containment=\"true\"",
                "name=\"operand\" containment=\"false\"",
            ),
            XTEXT.to_owned(),
        ),
        (
            ECORE.replace(
                "name=\"OperatorExpression\"",
                "name=\"OperatorExpression\" abstract=\"true\"",
            ),
            XTEXT.to_owned(),
        ),
    ] {
        assert!(
            ecore != ECORE || xtext != XTEXT,
            "Rejection must mutate the fixture"
        );
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &ecore,
                &xtext,
                RCORE_BEHAVIOR,
                Profile::NativeV5
            )
            .is_err(),
            "Accepted invalid capture: {ecore} {xtext}"
        );
    }
}

#[test]
fn current_wrappers_share_initializer_dispatch_and_failed_resources_stay_atomic() {
    let ecore=ECORE.replace("<eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"operator\"", "<eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"origin\" unsettable=\"true\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString\"/><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"operator\"");
    let mut rcore_behavior: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/current-actions/model-behavior.rules.json"
    ))
    .unwrap();
    rcore_behavior["initializers"] = json!([{"class":"OperatorExpression","provider":{"name":"intersect.constant-attributes","version":"1"},"members":["origin"],"values":{"origin":"constructor"}}]);
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &ecore,
            XTEXT,
            &rcore_behavior.to_string(),
            Profile::NativeV5,
        )
        .unwrap();
    let result = evaluate(&package, &source("Demo = 1 + 2 - 3;"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 6);
    for element in &result.model.elements {
        assert_eq!(
            element.initialization.is_some(),
            element.class == "OperatorExpression"
        );
        if element.class == "OperatorExpression" {
            assert_eq!(element.values["origin"], "constructor");
        }
    }
    let failed = evaluate(&package, &source("Demo = 1 + 2 -;"));
    assert!(!failed.diagnostics.is_empty());
    assert!(failed.model.elements.is_empty());
}

#[test]
fn current_actions_preserve_token_and_rule_depth_boundaries() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV5,
        )
        .unwrap();
    let sources = source("Demo = 1 + 2 - 3;");
    let mut limits = EvaluationLimits::native();
    limits.tokens_per_resource = Some(8);
    limits.nesting = 3;
    assert!(
        evaluate_with_limits(&package, &sources, limits)
            .diagnostics
            .is_empty()
    );
    limits.tokens_per_resource = Some(7);
    let rejected = evaluate_with_limits(&package, &sources, limits);
    assert!(!rejected.diagnostics.is_empty());
    assert!(rejected.model.elements.is_empty());
    limits.tokens_per_resource = Some(8);
    limits.nesting = 2;
    let rejected = evaluate_with_limits(&package, &sources, limits);
    assert!(!rejected.diagnostics.is_empty());
    assert!(rejected.model.elements.is_empty());
}
