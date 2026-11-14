use intersect_model::core::storage::{ModelRead, ReferenceView};
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate_stored};
use serde_json::json;

fn package() -> Package {
    let grammar = include_str!("../../../../fixtures/library/library.xtext").replace(
        "copy=[lib::BookCopy|ID]",
        "copy=[lib::BookCopy|ID] 'instead' copy=[lib::BookCopy|ID]",
    );
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        &grammar,
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap()
}

#[test]
fn final_unresolved_scalar_assignment_cannot_be_overwritten_by_a_superseded_reference() {
    let package = package();
    let result = evaluate_stored(&package, &[Source {
        uri: "test".into(),
        text: "library L { copy known \"Known\"; member reader; loan lending copy known instead missing to reader; }".into(),
    }], EvaluationLimits::native()).unwrap();
    assert_eq!(result.evaluation.diagnostics.len(), 1);
    assert!(
        result.evaluation.diagnostics[0]
            .message
            .contains("Unresolved reference 'missing'")
    );
    assert!(
        result
            .store
            .inspect("test#3", &mut |owner| {
                assert!(matches!(
                    owner.value("copy").unwrap().reference(),
                    Some(ReferenceView::Unresolved {
                        spelling: "missing"
                    })
                ));
                Ok(())
            })
            .unwrap()
    );
    assert_eq!(
        result.store.value("test#3", "copy").unwrap(),
        Some(json!("?missing"))
    );
    assert_eq!(result.evaluation.references.len(), 1);
    assert_eq!(result.evaluation.references[0].target, "test#2");
}

#[test]
fn superseded_unresolved_scalar_assignment_does_not_reject_the_current_reference() {
    let package = package();
    let result = evaluate_stored(&package, &[Source {
        uri: "test".into(),
        text: "library L { copy known \"Known\"; member reader; loan lending copy missing instead known to reader; }".into(),
    }], EvaluationLimits::native()).unwrap();
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_eq!(
        result.store.value("test#3", "copy").unwrap(),
        Some(json!("test#1"))
    );
    assert_eq!(result.evaluation.references.len(), 2);
    assert_eq!(result.evaluation.references[0].target, "test#1");
    let source = "library L { copy known \"Known\"; member reader; loan lending copy missing instead known to reader; }";
    let reference = &result.evaluation.references[0];
    assert_eq!(&source[reference.start..reference.end], "known");
    assert!(reference.start > source.find("instead").unwrap());
}

#[test]
fn successful_scalar_override_emits_only_the_current_reference_metadata() {
    let package = package();
    let result = evaluate_stored(&package, &[Source {
        uri: "test".into(),
        text: "library L { copy first \"First\"; copy second \"Second\"; member reader; loan lending copy first instead second to reader; }".into(),
    }], EvaluationLimits::native()).unwrap();
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_eq!(
        result.store.value("test#4", "copy").unwrap(),
        Some(json!("test#2"))
    );
    assert_eq!(result.evaluation.references.len(), 2);
    assert_eq!(result.evaluation.references[0].target, "test#2");
    assert_eq!(result.evaluation.references[1].target, "test#3");
}
