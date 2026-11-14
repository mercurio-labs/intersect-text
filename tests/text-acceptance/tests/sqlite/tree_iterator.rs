#![cfg(not(target_arch = "wasm32"))]
use intersect_model::core::storage::{self, ModelRead, Query};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::ConversionContract,
};
use intersect_model::sqlite::SqliteStore;
use intersect_model::{conversion::with_catalog, tree_iterator_conversion::TreeIteratorConverter};
use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
use serde_json::json;
#[test]
fn persisted_iterator_resumes_at_exact_node_and_failed_edits_roll_back() {
    let ecore = include_str!("../../../../fixtures/tree-iterators/iterators.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, ecore);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let contracts: Vec<ConversionContract> = serde_json::from_str(include_str!(
        "../../../../fixtures/tree-iterators/conversions.json"
    ))
    .unwrap();
    let (bundle, contracts) =
        intersect_text_acceptance::migration_fixture::migrate_provider_fixture(
            bundle.sources[0].clone(),
            &contracts,
        );
    with_catalog(&bundle, &contracts, |registry| {
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
                ecore,
                include_str!("../../../../fixtures/tree-iterators/iterators.xtext"),
                include_str!("../../../../fixtures/tree-iterators/iterators-behavior.rcore"),
                registry,
            )
            .unwrap();
        let adapter = TreeIteratorConverter::new(contracts[0].clone(), &bundle)?;
        let mut snapshot = adapter.start();
        adapter.next(&mut snapshot)?;
        let cursor = serde_json::to_value(registry.bind(
            &contracts[0].datatype,
            serde_json::to_value(&snapshot).unwrap(),
        )?)
        .unwrap();
        let path = std::env::temp_dir().join(format!(
            "intersect-tree-iterator-{}.sqlite",
            std::process::id()
        ));
        let expected;
        {
            let mut store = SqliteStore::open(&path)?;
            let result = evaluate_into_with_conversions(
                &package,
                &[Source {
                    uri: "sample.iterators".into(),
                    text: "session example".into(),
                }],
                EvaluationLimits::native(),
                &mut store,
                registry,
            )?;
            assert!(result.diagnostics.is_empty());
            let owner = storage::export(&store)?.elements[0].id.clone();
            storage::edit_with_conversions(
                &mut store,
                &package.schema,
                &[(owner.clone(), "cursor".into(), cursor.clone())],
                registry,
            )?;
            expected = storage::export(&store)?;
            let mut bad = cursor.clone();
            bad["value"]["position"] = json!(999);
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &package.schema,
                    &[
                        (owner.clone(), "cursor".into(), json!(null)),
                        (owner, "cursor".into(), bad)
                    ],
                    registry
                )
                .is_err()
            );
            assert_eq!(storage::export(&store)?, expected);
        }
        {
            let store = SqliteStore::open(&path)?;
            storage::validate_with_conversions(&store, &package.schema, registry)?;
            assert_eq!(storage::export(&store)?, expected);
            assert_eq!(
                store.query(Query::Value("cursor", &cursor))?,
                [expected.elements[0].id.clone()]
            );
            let mut resumed = adapter
                .snapshot(&storage::export(&store)?.elements[0].values["cursor"]["value"])?;
            let expected_feature =
                intersect_model::definitions::resources::ResourceSet::new_diagnostic(&bundle)
                    .unwrap()
                    .resolve(&bundle.sources[0].id, "#//Session/name")
                    .unwrap();
            assert_eq!(adapter.next(&mut resumed)?.unwrap(), expected_feature);
        }
        std::fs::remove_file(path).unwrap();
        Ok(())
    })
    .unwrap();
}
