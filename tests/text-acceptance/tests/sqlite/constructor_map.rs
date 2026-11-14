#![cfg(not(target_arch = "wasm32"))]
#[path = "../language/common/constructor_map.rs"]
mod common;
use intersect_model::conversion::with_catalog_policies;
use intersect_model::core::{
    model::Element,
    storage::{self, ModelStore},
};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::package::Package;
use serde_json::json;
#[test]
fn map_constructor_dependencies_and_values_survive_sqlite_reopen_and_failed_edits() {
    for wrapped in [false, true] {
        let f = common::fixture(wrapped);
        let path = std::env::temp_dir().join(format!(
            "intersect-constructor-map-{}-{}-{wrapped}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let package_path = path.with_extension("package.json");
        let expected = with_catalog_policies(
            &f.input.bundle,
            &f.ordinary,
            &f.applications,
            &f.constructors,
            |registry| {
                let package = compile_bundle_with_conversions(f.input.clone(), registry)
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(package.schema.constructor_contracts.len(), 2);
                let schema = &package.schema;
                let mut store = SqliteStore::open(&path)?;
                let checkpoint = store.begin()?;
                storage::insert(
                    &mut store,
                    schema,
                    Element::new(
                        schema,
                        "h",
                        schema.class_key_for_type(&f.input.materialized_types[0])?,
                        "memory:model",
                        0,
                        0,
                    )?,
                )?;
                store.commit(&checkpoint)?;
                let mut edits = vec![("h".into(), f.field.into(), f.value.clone())];
                if !wrapped {
                    edits.push(("h".into(), "mapping".into(), f.map.clone()));
                }
                storage::edit_with_conversions(&mut store, schema, &edits, registry)?;
                storage::validate_with_conversions(&store, schema, registry)?;
                std::fs::write(&package_path, serde_json::to_vec(&package).unwrap()).unwrap();
                storage::export(&store)
            },
        )
        .unwrap();
        let retained: Package =
            serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
        let replay = retained.compilation.clone().unwrap();
        with_catalog_policies(
            &replay.bundle,
            &retained.schema.conversion_contracts,
            &retained.schema.applied_conversion_contracts,
            &retained.schema.constructor_contracts,
            |registry| {
                let rebuilt = compile_bundle_with_conversions(replay.clone(), registry)
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(
                    serde_json::to_value(&rebuilt).unwrap(),
                    serde_json::to_value(&retained).unwrap()
                );
                let mut store = SqliteStore::open(&path)?;
                storage::validate_with_conversions(&store, &rebuilt.schema, registry)?;
                assert_eq!(storage::export(&store)?, expected);
                let mut wrong = f.value.clone();
                let bounded = if wrapped {
                    &mut wrong["value"]["items"][0]
                } else {
                    &mut wrong
                };
                bounded["converted"]["value"]["items"][0]["application"]["declaration"]["path"] =
                    json!([2]);
                bounded["converted"]["value"]["items"][0]["converted"]["datatype"]["path"] =
                    json!([2]);
                assert!(
                    storage::edit_with_conversions(
                        &mut store,
                        &rebuilt.schema,
                        &[
                            ("h".into(), "name".into(), json!("must roll back")),
                            ("h".into(), f.field.into(), wrong)
                        ],
                        registry
                    )
                    .is_err()
                );
                assert_eq!(storage::export(&store)?, expected);
                Ok(())
            },
        )
        .unwrap();
        assert!(
            with_catalog_policies(
                &replay.bundle,
                &retained.schema.conversion_contracts,
                &retained.schema.applied_conversion_contracts,
                &retained.schema.constructor_contracts[..1],
                |_| Ok(())
            )
            .is_err()
        );
        let mut stale = retained.schema.constructor_contracts.clone();
        stale[0].provider.version = "stale".into();
        assert!(
            with_catalog_policies(
                &replay.bundle,
                &retained.schema.conversion_contracts,
                &retained.schema.applied_conversion_contracts,
                &stale,
                |_| Ok(())
            )
            .is_err()
        );
        {
            let store = SqliteStore::open(&path).unwrap();
            assert_eq!(storage::export(&store).unwrap(), expected);
        }
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(package_path).unwrap();
    }
}
