//! Compatibility facade must preserve whole model-rule outcomes and text metadata.
#[path = "../../../fixtures/rcore-model-rule-service/controls.rs"]
mod controls;
use intersect_model::{model_rules, storage::ModelView};
use intersect_text::{Reference, StoredEvaluation, apply_stored_semantics_with_provider};

#[test]
fn xtext_facade_matches_shared_literal_success_and_rejection_without_touching_references() {
    let (package, model) = controls::fixture();
    let before = serde_json::to_value(&model).unwrap();
    let view = ModelView::new(&model).unwrap();
    for rejected in [false, true] {
        let (values, diagnostics) = controls::seed();
        let mut metadata = StoredEvaluation {
            derived: values.clone(),
            diagnostics: diagnostics.clone(),
            references: vec![Reference {
                uri: "memory:authored".into(),
                start: 31,
                end: 37,
                target: "preserved-target".into(),
            }],
        };
        let references = serde_json::to_value(&metadata.references).unwrap();
        let facade_provider = controls::Recording::new(&package, rejected);
        let outcome =
            apply_stored_semantics_with_provider(&package, &view, &mut metadata, &facade_provider);
        let facade = controls::snapshot(
            outcome,
            &metadata.derived,
            &metadata.diagnostics,
            &facade_provider,
        );
        let mut values = values;
        let mut diagnostics = diagnostics;
        let shared_provider = controls::Recording::new(&package, rejected);
        let outcome = model_rules::apply_with_provider(
            &package,
            &view,
            &mut values,
            &mut diagnostics,
            &shared_provider,
        );
        let shared = controls::snapshot(outcome, &values, &diagnostics, &shared_provider);
        assert_eq!(facade, controls::expected(rejected));
        assert_eq!(shared, controls::expected(rejected));
        assert_eq!(
            serde_json::to_value(&metadata.references).unwrap(),
            references
        );
    }
    assert_eq!(serde_json::to_value(&model).unwrap(), before);
}
