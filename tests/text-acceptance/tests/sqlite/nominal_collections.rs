#![cfg(not(target_arch = "wasm32"))]
use intersect_model::conversion::with_catalog;
use intersect_model::core::storage::{self, ModelRead, Query};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::ConversionContract,
};
use intersect_model::sqlite::SqliteStore;
use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
use serde_json::json;
#[test]
fn persisted_nested_collections_validate_and_failed_edits_roll_back() {
    let ecore = include_str!("../../../../fixtures/collections/collections.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, ecore);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let contracts: Vec<ConversionContract> = serde_json::from_str(include_str!(
        "../../../../fixtures/collections/conversions.json"
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
                include_str!("../../../../fixtures/collections/collections.xtext"),
                include_str!("../../../../fixtures/collections/collections-behavior.rcore"),
                registry,
            )
            .unwrap();
        let list = registry.bind(&contracts[0].datatype, json!({"items":[1,2,1]}))?;
        let cursor = serde_json::to_value(registry.bind(
            &contracts[1].datatype,
            json!({"entries":[{"key":"a","value":list}]}),
        )?)
        .unwrap();
        let path = std::env::temp_dir().join(format!(
            "intersect-collections-{}.sqlite",
            std::process::id()
        ));
        let expected;
        {
            let mut store = SqliteStore::open(&path)?;
            let result = evaluate_into_with_conversions(
                &package,
                &[Source {
                    uri: "sample.collections".into(),
                    text: "catalog example".into(),
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
                &[(owner.clone(), "map".into(), cursor.clone())],
                registry,
            )?;
            expected = storage::export(&store)?;
            let mut bad = cursor.clone();
            bad["value"]["entries"][0]["value"]["value"]["items"] = json!(["wrong"]);
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &package.schema,
                    &[
                        (owner.clone(), "map".into(), json!(null)),
                        (owner, "map".into(), bad)
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
                store.query(Query::Value("map", &cursor))?,
                [expected.elements[0].id.clone()]
            );
        }
        std::fs::remove_file(path).unwrap();
        Ok(())
    })
    .unwrap();
}
