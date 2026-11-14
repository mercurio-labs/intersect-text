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
fn persisted_reference_maps_preserve_context_and_object_readback() {
    let ecore = include_str!("../../../../fixtures/feature-references/references.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, ecore);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let contracts: Vec<ConversionContract> = serde_json::from_str(include_str!(
        "../../../../fixtures/feature-references/conversions.json"
    ))
    .unwrap();
    let (bundle, contracts) =
        intersect_text_acceptance::migration_fixture::migrate_provider_fixture(
            bundle.sources[0].clone(),
            &contracts,
        );
    with_catalog(&bundle, &contracts, |registry| {
        let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
            ecore,
            include_str!("../../../../fixtures/feature-references/references.xtext"),
            include_str!("../../../../fixtures/feature-references/references-behavior.rcore"),
            registry,
        )
        .unwrap();
        let feature = &contracts[0].declarations[0];
        let adapter = intersect_model::conversion::feature_map_provider(&bundle, &contracts[1])?;
        let reference = adapter.bind_reference(feature, "special")?;
        let cursor = serde_json::to_value(registry.bind(
            &contracts[1].datatype,
            json!({"entries":[{"feature":feature,"value":reference},{"feature":feature,"value":reference}]}),
        )?)
        .unwrap();
        let entry = serde_json::to_value(registry.bind(
            &contracts[0].datatype, json!({"feature":feature,"value":reference}),
        )?).unwrap();
        let path = std::env::temp_dir().join(format!(
            "intersect-feature-references-{}.sqlite",
            std::process::id()
        ));
        let expected;
        {
            let mut store = SqliteStore::open(&path)?;
            let result = evaluate_into_with_conversions(
                &package,
                &[Source {
                    uri: "sample.library".into(),
                    text: include_str!("../../../../fixtures/library/catalog.library").into(),
                }],
                EvaluationLimits::native(),
                &mut store,
                registry,
            )?;
            assert!(result.diagnostics.is_empty());
            let owner = storage::export(&store)?.elements.iter().find(|e| e.values.get("name") == Some(&json!("earthsea"))).unwrap().id.clone();
            storage::edit_with_conversions(
                &mut store,
                &package.schema,
                &[(owner.clone(), "featureMap".into(), cursor.clone()), (owner.clone(), "entry".into(), entry.clone())],
                registry,
            )?;
            expected = storage::export(&store)?;
            let mut bad = cursor.clone();
            bad["value"]["entries"][0]["value"] = json!(99);
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &package.schema,
                    &[
                        (owner.clone(), "featureMap".into(), json!(null)),
                        (owner, "featureMap".into(), bad)
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
            let reloaded = storage::export(&store)?;
            assert_eq!(reloaded, expected);
            let owner = reloaded.elements.iter().find(|e| e.values.get("name") == Some(&json!("earthsea"))).unwrap();
            for reference in [&owner.values["entry"]["value"]["value"], &owner.values["featureMap"]["value"]["entries"][0]["value"]] {
                assert_eq!(adapter.resolve_reference(feature, reference)?.class, "SpecialCopy");
            }
            assert_eq!(
                store.query(Query::Value("featureMap", &cursor))?,
                [expected.elements.iter().find(|e| e.values.get("name") == Some(&json!("earthsea"))).unwrap().id.clone()]
            );
        }
        let owner = expected.elements.iter().find(|e| e.values.get("name") == Some(&json!("earthsea"))).unwrap();
        let loaded = &owner.values["featureMap"]["value"]["entries"][0]["value"];
        assert_eq!(adapter.resolve_reference(feature, loaded)?.class, "SpecialCopy");
        std::fs::remove_file(path).unwrap();
        Ok(())
    })
    .unwrap();
}
