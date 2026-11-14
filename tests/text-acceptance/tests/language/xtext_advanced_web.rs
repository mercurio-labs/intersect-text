use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
#[test]
fn advanced_library_reuses_domain_semantics_and_preserves_starter() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            include_str!("../../../../fixtures/library/library.ecore"),
            include_str!("../../../../fixtures/xtext/advanced-web/library.xtext"),
            include_str!("../../../../fixtures/library/library-behavior.rcore"),
            Profile::NativeV27,
        )
        .unwrap();
    let sources = [
        Source {
            uri: "catalog.library".into(),
            text: include_str!("../../../../fixtures/xtext/advanced-web/catalog.library").into(),
        },
        Source {
            uri: "loans.library".into(),
            text: include_str!("../../../../fixtures/xtext/advanced-web/loans.library").into(),
        },
    ];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 6);
    let dune = result
        .model
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&serde_json::json!("dune")))
        .unwrap();
    assert_eq!(dune.values["edition"], 2);
    assert_eq!(dune.values["title"], "Dune");
    let spare = result
        .model
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&serde_json::json!("spare")))
        .unwrap();
    assert_eq!(spare.values["title"], "untitled");
    assert_eq!(spare.values["edition"], 1);
    assert_eq!(result.references.len(), 2);
    let mut invalid = sources.clone();
    invalid[1].text = "library Bad { loan bad copy Missing to ada; }".into();
    assert!(!evaluate(&package, &invalid).diagnostics.is_empty());
    invalid[1].text =
        "library Bad { loan first copy dune to ada; loan duplicate copy dune to ada; }".into();
    assert!(
        evaluate(&package, &invalid)
            .diagnostics
            .iter()
            .any(|d| d.message.contains("more than one active loan"))
    );
}
#[test]
fn advanced_expression_example_builds_exact_action_trees() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            include_str!("../../../../fixtures/xtext/current-actions/model.ecore"),
            include_str!("../../../../fixtures/xtext/advanced-web/expressions.xtext"),
            include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore"),
            Profile::NativeV27,
        )
        .unwrap();
    let sources = [
        Source {
            uri: "catalog.kerml".into(),
            text: include_str!("../../../../fixtures/xtext/advanced-web/catalog.kerml").into(),
        },
        Source {
            uri: "more.kerml".into(),
            text: include_str!("../../../../fixtures/xtext/advanced-web/more.kerml").into(),
        },
    ];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 9);
    let operators: Vec<_> = result
        .model
        .elements
        .iter()
        .filter(|e| e.class == "OperatorExpression")
        .map(|e| e.values["operator"].clone())
        .collect();
    assert_eq!(
        operators,
        vec![
            serde_json::json!("+"),
            serde_json::json!("-"),
            serde_json::json!("all")
        ]
    );
    let mut invalid = sources.clone();
    invalid[0].text = "Demo = (1 +);".into();
    assert!(!evaluate(&package, &invalid).diagnostics.is_empty());
}
