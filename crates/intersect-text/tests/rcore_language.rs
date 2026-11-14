use intersect_text::{Source, compiler, evaluate};

#[test]
fn direct_rcore_project_replays_structure_and_domain_behavior() {
    let text = include_str!("../assets/fixtures/library/library-project.rcore");
    let package = compiler::compile_rcore_project(
        text,
        include_str!("../assets/fixtures/library/library.xtext"),
    )
    .unwrap();
    assert!(package.source_policy.is_none());
    let compilation = package.compilation.as_ref().unwrap();
    assert_eq!(compilation.ecore.uri, "memory:model.rcore");
    assert_eq!(compilation.bundle.sources.len(), 2);
    assert_eq!(compilation.ecore, compilation.rcore_behavior);
    assert!(
        !compilation
            .bundle
            .sources
            .iter()
            .any(|s| s.kind == intersect_model::definitions::bundle::SourceKind::Ecore)
    );
    let sources = [Source {
        uri: "catalog.library".into(),
        text: "library Catalog { copy dune \"Dune\"; member ada; loan first copy dune to ada; }"
            .into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let copy = result
        .model
        .elements
        .iter()
        .find(|e| e.class == "BookCopy")
        .unwrap();
    assert_eq!(
        result.derived[&copy.id]["available"],
        serde_json::json!(false)
    );
    let rebuilt = compiler::recompile(&package).unwrap();
    assert_eq!(rebuilt.schema, package.schema);
    assert!(rebuilt.source_policy.is_none());
    assert_eq!(
        serde_json::to_value(&rebuilt).unwrap(),
        serde_json::to_value(&package).unwrap()
    );
    let replay = evaluate(&rebuilt, &sources);
    assert_eq!(replay.model, result.model);
    assert_eq!(replay.derived, result.derived);
}
