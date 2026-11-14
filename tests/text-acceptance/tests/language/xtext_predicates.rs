use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/first-token-predicates/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/first-token-predicates/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/first-token-predicates/model-behavior.rcore");
fn source(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "prediction.model".into(),
        text: text.into(),
    }]
}
#[test]
fn visible_guard_selects_once_and_conversion_failure_cannot_retry() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV6,
        )
        .unwrap();
    let result = evaluate(&package, &source("model { a b 12; }"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 2);
    assert_eq!(result.model.elements[1].class, "Left");
    assert_eq!(result.model.elements[1].values["number"], 12);
    for text in [
        "model { a b 2147483648; }",
        "model { a b ; }",
        "model { a b 12; a b ; }",
    ] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty(), "{text}");
    }
}
#[test]
fn lookahead_distinguishes_shared_prefix_without_constructing_discarded_branch() {
    let grammar = XTEXT.replace("'a' 'b' raw", "'a' 'c' raw");
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV6,
        )
        .unwrap();
    let result = evaluate(&package, &source("model { a c 2147483648; a b 12; }"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    assert_eq!(result.model.elements[1].class, "Right");
    assert_eq!(result.model.elements[1].values["raw"], "2147483648");
    assert_eq!(result.model.elements[2].class, "Left");
    for (i, element) in result.model.elements.iter().enumerate() {
        assert_eq!(element.id, format!("prediction.model#{i}"));
    }
}
#[test]
fn predicate_admission_is_explicit_and_legacy_profile_unchanged() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV5
        )
        .is_err()
    );
    for prefix in ["", "=>", "- >", "-> ->"] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &XTEXT.replace("->Left", &format!("{prefix}Left")),
                RCORE_BEHAVIOR,
                Profile::NativeV6
            )
            .is_err(),
            "{prefix}"
        );
    }
}
#[test]
fn prediction_work_is_charged_and_resource_failures_roll_back() {
    use intersect_text::{EvaluationLimits, evaluate_with_limits};
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV6,
        )
        .unwrap();
    let input = source("model { a b 12; }");
    let mut limits = EvaluationLimits::native();
    limits.elements_per_resource = Some(2);
    assert!(
        evaluate_with_limits(&package, &input, limits)
            .diagnostics
            .is_empty()
    );
    limits.elements_per_resource = Some(1);
    let result = evaluate_with_limits(&package, &input, limits);
    assert!(!result.diagnostics.is_empty());
    assert!(result.model.elements.is_empty());
    let mut low = 0;
    let mut high = 100_000;
    while low < high {
        let mid = low + (high - low) / 2;
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(mid);
        if evaluate_with_limits(&package, &input, limits)
            .diagnostics
            .is_empty()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(low > 0 && low < 100_000);
    for budget in [low - 1, low] {
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(budget);
        let result = evaluate_with_limits(&package, &input, limits);
        assert_eq!(result.diagnostics.is_empty(), budget == low);
        assert_eq!(
            result.model.elements.len(),
            if budget == low { 2 } else { 0 }
        );
    }
}
#[test]
fn new_prediction_path_preserves_assigned_current_graph() {
    let ecore = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
    let grammar = include_str!("../../../../fixtures/xtext/current-actions/model.xtext");
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ecore,
        grammar,
        RCORE_BEHAVIOR,
        Profile::NativeV5,
    )
    .unwrap();
    let new = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ecore,
        grammar,
        RCORE_BEHAVIOR,
        Profile::NativeV6,
    )
    .unwrap();
    for text in ["Demo = 1;", "Demo = 1 + 2 - 3;"] {
        let old_result = evaluate(&old, &source(text));
        let new_result = evaluate(&new, &source(text));
        assert!(
            new_result.diagnostics.is_empty(),
            "{:?}",
            new_result.diagnostics
        );
        assert_eq!(
            serde_json::to_value(old_result.model).unwrap(),
            serde_json::to_value(new_result.model).unwrap()
        );
    }
    for text in ["Demo = 1 +;", "Demo = 1 + 2 -;", "Demo = + 1;"] {
        let result = evaluate(&new, &source(text));
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty());
    }
}
#[test]
fn guarded_specialization_shape_executes_through_typed_fragment() {
    let grammar = XTEXT
        .replace("'model' '{' entries+=Entry* '}'", "'model' '{' Parts '}'")
        .replace(
            "Entry returns p::Entry: ->Left | Right;",
            "fragment Parts returns p::Model: (-> entries+=Left)+ entries+=Right? entries+=Left*;",
        )
        .replace("'a' 'b' raw", "'multiplicity' raw");
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV6,
        )
        .unwrap();
    for (text, classes) in [
        ("model { a b 12; }", vec!["Model", "Left"]),
        ("model { a b 12; a b 13; }", vec!["Model", "Left", "Left"]),
        (
            "model { a b 12; multiplicity 2; a b 13; }",
            vec!["Model", "Left", "Right", "Left"],
        ),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(
            result.diagnostics.is_empty(),
            "{text}: {:?}",
            result.diagnostics
        );
        assert_eq!(
            result
                .model
                .elements
                .iter()
                .map(|element| element.class.as_str())
                .collect::<Vec<_>>(),
            classes
        );
    }
    for text in [
        "model { }",
        "model { multiplicity 2; }",
        "model { a b 12; multiplicity ; }",
    ] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty(), "{text}");
    }
}
