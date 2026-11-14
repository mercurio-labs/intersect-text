//! The unchanged parser constructing into the frozen engine through
//! `FrozenStore`, compared with the same parse through the owned facade.
use intersect_library as library;
use intersect_model::core::{
    frozen::{FrozenStore, Metamodel},
    storage,
};
use intersect_text::{EvaluationLimits, evaluate_into, evaluate_stored};
use std::sync::Arc;

fn sorted(mut model: intersect_model::core::model::Model) -> intersect_model::core::model::Model {
    model.elements.sort_by(|a, b| a.id.cmp(&b.id));
    model
}

#[test]
fn parser_constructs_the_same_library_graph_through_both_entries() -> Result<(), String> {
    let language = library::language()?;
    let package = language.package();
    let sources = library::sources();

    let owned = evaluate_stored(package, &sources, EvaluationLimits::workbench())?;
    let mm = Arc::new(Metamodel::freeze(&package.schema).map_err(|e| e.to_string())?);
    let mut frozen = FrozenStore::new(mm);
    let in_frozen = evaluate_into(
        package,
        &sources,
        EvaluationLimits::workbench(),
        &mut frozen,
    )?;

    assert_eq!(
        format!("{:?}", in_frozen.diagnostics),
        format!("{:?}", owned.evaluation.diagnostics)
    );
    let expected = storage::export(&owned.store)?;
    assert!(!expected.elements.is_empty());
    let actual = storage::export(&frozen)?;
    assert_eq!(sorted(actual).elements, sorted(expected.clone()).elements);
    assert_eq!(storage::export(&frozen)?.resources, expected.resources);

    // The constructed arena becomes a validated frozen model with the same graph.
    let model = frozen
        .into_builder()
        .map_err(|e| e.to_string())?
        .link(&intersect_model::core::frozen::ById)
        .map_err(|e| e.to_string())?
        .validate()
        .map_err(|e| e.to_string())?;
    assert_eq!(
        sorted(model.to_model().map_err(|e| e.to_string())?).elements,
        sorted(expected).elements
    );
    Ok(())
}
