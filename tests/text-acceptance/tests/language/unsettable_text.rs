use super::Store;
use intersect_model::core::storage::{self, ModelRead};
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_into};
use serde_json::json;
fn package() -> Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/ecore-unsettable/model.ecore"),
        include_str!("../../../../fixtures/ecore-unsettable/model.xtext"),
        include_str!("../../../../fixtures/ecore-unsettable/model-behavior.rcore"),
    )
    .unwrap()
}
fn sources() -> Vec<Source> {
    vec![
        Source {
            uri: "one".into(),
            text: r#"root first { item a value "seed" tags "x" "y" link b; }"#.into(),
        },
        Source {
            uri: "two".into(),
            text: "root second { item b tags; }".into(),
        },
    ]
}
store_test! {
pub fn native_text_preserves_default_assignment_containment_and_cross_resource_links() {
    let package = package();
    let result = evaluate(&package, &sources());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let mut store = crate::stores::store::<Store>(&package.schema).unwrap();
    let metadata =
        evaluate_into(&package, &sources(), EvaluationLimits::native(), &mut store).unwrap();
    assert!(metadata.diagnostics.is_empty());
    assert_eq!(storage::export(&store).unwrap(), result.model);
    for (id, feature, assigned) in [
        ("one#0", "items", true),
        ("one#0", "value", false),
        ("one#1", "value", true),
        ("one#1", "tags", true),
        ("one#1", "link", true),
        ("two#1", "value", false),
        ("two#1", "tags", false),
        ("two#1", "link", false),
    ] {
        assert_eq!(
            storage::is_explicitly_set(&store, &package.schema, id, feature).unwrap(),
            assigned,
            "{id}.{feature}"
        );
    }
    assert_eq!(store.value("one#1", "value").unwrap(), Some(json!("seed")));
    assert_eq!(store.value("one#1", "link").unwrap(), Some(json!("two#1")));
    storage::validate(&store, &package.schema).unwrap();
}
}
store_test! {
pub fn syntax_and_link_failures_restore_preexisting_assignment_history() {
    let package = package();
    let mut store = crate::stores::store::<Store>(&package.schema).unwrap();
    assert!(
        evaluate_into(&package, &sources(), EvaluationLimits::native(), &mut store)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    let original = storage::export(&store).unwrap();
    for text in [
        r#"root failed { item a value "seed" tags "x" missing }"#,
        r#"root failed { item a value "seed" link absent; }"#,
    ] {
        let output = evaluate_into(
            &package,
            &[Source {
                uri: "bad".into(),
                text: text.into(),
            }],
            EvaluationLimits::native(),
            &mut store,
        )
        .unwrap();
        assert!(!output.diagnostics.is_empty());
        assert_eq!(storage::export(&store).unwrap(), original);
    }
    assert!(
        evaluate_into(&package, &sources(), EvaluationLimits::native(), &mut store)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    assert_eq!(storage::export(&store).unwrap(), original);
}
}

control_test! {
pub fn grammar_rejects_assignment_to_inherited_nonchangeable_feature() {
    let ecore = include_str!("../../../../fixtures/ecore-unsettable/model.ecore")
        .replace("name=\"value\"", "name=\"value\" changeable=\"false\"");
    let xtext = include_str!("../../../../fixtures/ecore-unsettable/model.xtext");
    let rcore_behavior = include_str!("../../../../fixtures/ecore-unsettable/model-behavior.rcore");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            &ecore,
            xtext,
            rcore_behavior
        )
        .unwrap_err()
        .iter()
        .any(|d| d.message.contains("non-changeable"))
    );
}
}
