use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/recursive-predicates/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/recursive-predicates/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/recursive-predicates/model-behavior.rcore");
fn source(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "recursive.model".into(),
        text: text.into(),
    }]
}
#[test]
fn recursive_models_match_independent_acceptance_and_preserve_profile_boundary() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV8
        )
        .unwrap_err()
        .iter()
        .any(|d| d.message.contains("Recursive first-token"))
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV9,
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
        .filter(|r| r["parser"] == "RecursiveEntryFirst")
    {
        let result = evaluate(&package, &source(row["text"].as_str().unwrap()));
        assert_eq!(
            result.diagnostics.is_empty(),
            row["accepted"].as_bool().unwrap(),
            "{row}: {:?}",
            result.diagnostics
        );
        if row["accepted"] == true {
            assert_eq!(result.model.elements[0].class, "Left");
        } else {
            assert!(result.model.elements.is_empty(), "{row}");
        }
        compared += 1;
    }
    assert_eq!(compared, 11);
    let result = evaluate(
        &package,
        &source(include_str!(
            "../../../../fixtures/xtext/recursive-predicates/example.model"
        )),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    assert_eq!(result.model.elements[0].values["number"], 12);
    assert_eq!(
        result.model.elements[0].values["child"],
        result.model.elements[1].id
    );
    assert_eq!(
        result.model.elements[1].values["child"],
        result.model.elements[2].id
    );
}

#[test]
fn recursive_execution_preserves_conversion_commit_resource_rollback_and_budgets() {
    use intersect_text::{EvaluationLimits, evaluate_with_limits};
    let grammar = XTEXT.replace(
        "Right returns r::Right: 'a' (child=Inner)? 'c';",
        "Right returns r::Right: 'a' (child=Inner)? 'b' raw=RawInt;",
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV9,
        )
        .unwrap();
    let valid = source("a a a b b b 12");
    let expected = evaluate(&package, &valid);
    assert!(expected.diagnostics.is_empty());
    let mut combined = valid.clone();
    combined.push(Source {
        uri: "bad.model".into(),
        text: "a a b b 2147483648".into(),
    });
    let rejected = evaluate(&package, &combined);
    assert!(!rejected.diagnostics.is_empty());
    let mut retained = rejected.model;
    let failed = retained.resources.remove("bad.model").unwrap();
    assert_eq!(failed.uri, "bad.model");
    assert!(failed.roots.is_empty());
    assert_eq!(failed.content_revision, 0);
    assert_eq!(
        failed.load_state,
        intersect_model::resources::ResourceLoadState::Loaded
    );
    assert!(failed.source.is_none());
    assert_eq!(failed.diagnostics.len(), 1);
    assert!(failed.diagnostics[0].contains("EInt integer out of range"));
    assert_eq!(retained, expected.model);
    let mut low = 0;
    let mut high = 100_000;
    while low < high {
        let middle = low + (high - low) / 2;
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(middle);
        if evaluate_with_limits(&package, &valid, limits)
            .diagnostics
            .is_empty()
        {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(low > 0 && low < 100_000);
    for budget in [low - 1, low] {
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(budget);
        let result = evaluate_with_limits(&package, &valid, limits);
        assert_eq!(result.diagnostics.is_empty(), budget == low);
        assert_eq!(
            result.model.elements.len(),
            if budget == low { 3 } else { 0 }
        );
    }
    for count in [2, 3] {
        let mut limits = EvaluationLimits::native();
        limits.elements_per_resource = Some(count);
        let result = evaluate_with_limits(&package, &valid, limits);
        assert_eq!(result.diagnostics.is_empty(), count == 3);
        assert_eq!(result.model.elements.len(), if count == 3 { 3 } else { 0 });
    }
}
#[test]
fn recursive_rule_alone_does_not_force_commit_and_full_predicates_still_reject() {
    let grammar = XTEXT.replace(
        "Right returns r::Right: 'a' (child=Inner)? 'c';",
        "Right returns r::Right: 'a' 'c';",
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV9,
        )
        .unwrap();
    let result = evaluate(&package, &source("a c"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[0].class, "Right");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &XTEXT.replace("->Left", "=>Left"),
            RCORE_BEHAVIOR,
            Profile::NativeV9
        )
        .is_err()
    );
    let wire = serde_json::to_value(&package).unwrap();
    assert_eq!(wire["sources"]["xtext"], grammar);
    let replay = serde_json::from_value(wire).unwrap();
    assert_eq!(
        serde_json::to_value(evaluate(&replay, &source("a c")).model).unwrap(),
        serde_json::to_value(result.model).unwrap()
    );
}
