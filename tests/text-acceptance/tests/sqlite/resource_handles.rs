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
fn scoped_resource_and_set_snapshots_reopen_with_atomic_rejection() {
    let ecore = include_str!("../../../../fixtures/resource-handles/handles.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, ecore);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let contracts: Vec<ConversionContract> = serde_json::from_str(include_str!(
        "../../../../fixtures/resource-handles/conversions.json"
    ))
    .unwrap();
    let (bundle, contracts) =
        intersect_text_acceptance::migration_fixture::migrate_provider_fixture(
            bundle.sources[0].clone(),
            &contracts,
        );
    let id = bundle.sources[0].id.clone();
    with_catalog(&bundle, &contracts, |registry| {
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
                ecore,
                include_str!("../../../../fixtures/resource-handles/handles.xtext"),
                include_str!("../../../../fixtures/resource-handles/handles-behavior.rcore"),
                registry,
            )
            .unwrap();
        let path = std::env::temp_dir().join(format!(
            "intersect-resource-handles-{}.sqlite",
            std::process::id()
        ));
        let expected;
        let values: Vec<_> = contracts
            .iter()
            .enumerate()
            .map(|(index, c)| {
                serde_json::to_value(
                    registry
                        .bind(
                            &c.datatype,
                            if index < 2 {
                                json!({"resource":id})
                            } else {
                                json!({"resources":[id]})
                            },
                        )
                        .unwrap(),
                )
                .unwrap()
            })
            .collect();
        {
            let mut store = SqliteStore::open(&path)?;
            let result = evaluate_into_with_conversions(
                &package,
                &[Source {
                    uri: "sample.handles".into(),
                    text: "catalog example".into(),
                }],
                EvaluationLimits::native(),
                &mut store,
                registry,
            )?;
            assert!(result.diagnostics.is_empty());
            let owner = storage::export(&store)?.elements[0].id.clone();
            let edits: Vec<_> = ["resource", "localResource", "resources", "localSet"]
                .iter()
                .zip(&values)
                .map(|(feature, value)| (owner.clone(), (*feature).into(), value.clone()))
                .collect();
            storage::edit_with_conversions(&mut store, &package.schema, &edits, registry)?;
            expected = storage::export(&store)?;
            let mut invalid = values[2].clone();
            invalid["value"]["resources"] = json!([id, id]);
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &package.schema,
                    &[
                        (owner.clone(), "resource".into(), json!(null)),
                        (owner, "resources".into(), invalid)
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
            assert!(storage::validate(&store, &package.schema).is_err());
            assert_eq!(storage::export(&store)?, expected);
            for (field, value) in ["resource", "localResource", "resources", "localSet"]
                .iter()
                .zip(&values)
            {
                assert_eq!(
                    store.query(Query::Value(field, value))?,
                    [expected.elements[0].id.clone()]
                );
            }
        }
        std::fs::remove_file(path).unwrap();
        Ok(())
    })
    .unwrap();
}
