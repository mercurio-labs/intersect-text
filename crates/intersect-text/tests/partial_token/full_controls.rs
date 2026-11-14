//! Existing imported-source genuine Xtext rows, new explicit scheduling only.
use super::*;
use intersect_text::grammar::RecursiveFrontierOrder;
use intersect_text::package::CompilationSourcePolicy;
fn selected(
    package: &Package,
    order: RecursiveFrontierOrder,
) -> Result<Package, Vec<intersect_model::definitions::schema::Diagnostic>> {
    ordinary::compile_with_context_frontier_selected(
        package.compilation.clone().unwrap(),
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        None,
        order,
    )
}
#[test]
fn partial_token_policy_keeps_genuine_full_packages_models_spans_and_whole_documents() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext-full-predicates-v68/observations.json"
    ))
    .unwrap();
    let mut count = 0;
    for (name, text) in [
        (
            "ImportedFullPrefix",
            include_str!("../../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"),
        ),
        (
            "ImportedFullBoth",
            include_str!("../../../../fixtures/xtext-full-predicates-v68/ImportedFullBoth.xtext"),
        ),
    ] {
        let old = compile_profile(name, text, Profile::NativeV80);
        let historical = selected(&old, RecursiveFrontierOrder::Historical).unwrap();
        let bfs = selected(&old, RecursiveFrontierOrder::BreadthFirstSharedTokenProbe).unwrap();
        assert_eq!(
            serde_json::to_value(&historical).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&bfs).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        let a = DocumentLanguage::prepare(&old, Default::default(), AnalysisLimits::workbench())
            .unwrap();
        let b = DocumentLanguage::prepare(&bfs, Default::default(), AnalysisLimits::workbench())
            .unwrap();
        for row in rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["grammar"] == format!("intersect.oracle.{name}"))
        {
            let source = Source {
                uri: "memory:full-prefix.model".into(),
                text: row["text"].as_str().unwrap().into(),
            };
            let x = evaluate_with_limits(
                &old,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            );
            let y = evaluate_with_limits(
                &bfs,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            );
            println!(
                "PARTIAL_TOKEN_GENUINE_FULL_ROW {}",
                json!({"oracle":row,"evaluation":y})
            );
            assert_eq!(
                serde_json::to_value(&x).unwrap(),
                serde_json::to_value(&y).unwrap()
            );
            assert_eq!(y.diagnostics.is_empty(), row["accepted"].as_bool().unwrap());
            if row["accepted"] == true {
                assert_eq!(y.model.elements.len(), 1);
                let e = &y.model.elements[0];
                assert_eq!(
                    json!({"class":format!("{}#//{}",bfs.schema.uri,e.class),"values":e.values}),
                    row["model"]
                );
                assert_eq!(e.start, row["root_start"]);
                assert_eq!(e.end, row["root_end"]);
                if source.text == "a b d" {
                    assert_eq!(e.values["branch"], "d");
                }
            } else {
                assert!(y.model.elements.is_empty());
            }
            let x = a
                .analyze(&source, None, EvaluationLimits::workbench())
                .unwrap();
            let y = b
                .analyze(&source, None, EvaluationLimits::workbench())
                .unwrap();
            assert_eq!(
                json!({"snapshot":x.snapshot,"diagnostics":x.diagnostics,"valid":x.valid_model,"recovered":x.recovered}),
                json!({"snapshot":y.snapshot,"diagnostics":y.diagnostics,"valid":y.valid_model,"recovered":y.recovered})
            );
            count += 1;
        }
    }
    assert_eq!(count, 14);
}
#[test]
fn partial_token_policy_rejects_foreign_profile_and_keeps_authored_source_errors() {
    let text =
        include_str!("../../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext");
    let old = compile_profile("ImportedFullPrefix", text, Profile::NativeV80);
    let result = ordinary::compile_with_context_frontier_selected(
        old.compilation.clone().unwrap(),
        Profile::NativeV79,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        None,
        RecursiveFrontierOrder::BreadthFirstSharedTokenProbe,
    );
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message,
        "Recursive frontier selection requires NativeV80"
    );
    let mut changed = old.compilation.clone().unwrap();
    let entry = changed.xtext.clone().unwrap();
    changed
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == entry)
        .unwrap()
        .text
        .push_str(" invalid bytes");
    let baseline = ordinary::compile(changed.clone(), Profile::NativeV80).unwrap_err();
    let selected = ordinary::compile_with_context_frontier_selected(
        changed,
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        None,
        RecursiveFrontierOrder::BreadthFirstSharedTokenProbe,
    )
    .unwrap_err();
    assert_eq!(
        serde_json::to_value(selected).unwrap(),
        serde_json::to_value(baseline).unwrap()
    );
}
