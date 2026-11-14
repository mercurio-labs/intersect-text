//! Original pinned First semantics, not a Full or branch-preference rewrite.
use super::*;
use intersect_text::grammar::RecursiveFrontierOrder;
use intersect_text::package::CompilationSourcePolicy;
#[test]
fn partial_token_policy_keeps_genuine_first_eleven_whole_packages_evaluations_and_documents() {
    let package = migrated(
        include_str!("../../../../fixtures/xtext/recursive-predicates/model.ecore"),
        include_str!("../../../../fixtures/xtext/recursive-predicates/model.xtext"),
        include_str!("../../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
        Profile::NativeV80,
    );
    let selected = intersect_text::ordinary::compile_with_context_frontier_selected(
        package.compilation.clone().unwrap(),
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        None,
        RecursiveFrontierOrder::BreadthFirstSharedTokenProbe,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(&selected).unwrap()
    );
    let old = DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
        .unwrap();
    let new = DocumentLanguage::prepare(&selected, Default::default(), AnalysisLimits::workbench())
        .unwrap();
    let rows: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["parser"] == "RecursiveEntryFirst")
    {
        let input = source(row["text"].as_str().unwrap());
        let a = evaluate(&package, &input);
        let b = evaluate(&selected, &input);
        let x = old
            .analyze(&input[0], None, EvaluationLimits::workbench())
            .unwrap();
        let y = new
            .analyze(&input[0], None, EvaluationLimits::workbench())
            .unwrap();
        println!(
            "PARTIAL_TOKEN_GENUINE_FIRST_ROW {}",
            json!({"oracle":row,"evaluation":b,"document":y})
        );
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&x).unwrap(),
            serde_json::to_value(&y).unwrap()
        );
        assert_eq!(b.diagnostics.is_empty(), row["accepted"].as_bool().unwrap());
        assert_eq!(y.valid_model.is_some(), row["accepted"].as_bool().unwrap());
        if row["text"] == "a b" {
            assert!(b.diagnostics.is_empty());
        }
        if row["text"] == "a c" {
            assert_eq!(row["branch"], "g");
            assert_eq!(row["syntax_errors"], 1);
            assert!(!b.diagnostics.is_empty());
        }
        count += 1;
    }
    assert_eq!(count, 11);
}
