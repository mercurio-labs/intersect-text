#![cfg(not(target_arch = "wasm32"))]
#[path = "../language/common/generic_reference_context.rs"]
mod common;
use intersect_model::conversion::{
    applied_collection_provider_with_conversions, with_catalog_policies,
};
use intersect_model::core::{
    model::Element,
    storage::{self, ModelStore},
};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::package::Package;
use serde_json::json;
#[test]
fn generic_references_and_embedded_policies_survive_sqlite_reopen() {
    let f = common::fixture();
    assert!(!f.context.constructors.is_empty());
    assert_ne!(f.target, f.raw_target);
    let path = std::env::temp_dir().join(format!(
        "intersect-reference-context-{}-{}.sqlite",
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
        &[],
        |registry| {
            let package = compile_bundle_with_conversions(f.input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            let adapter = applied_collection_provider_with_conversions(
                &f.input.bundle,
                &f.applications[0],
                registry,
            )?;
            let reference = adapter.bind_reference(0, "sub")?;
            let value = registry
                .applied()
                .bind(&f.applications[0].application, json!({"items":[reference]}))?;
            let mut store = SqliteStore::open(&path)?;
            let checkpoint = store.begin()?;
            storage::insert(
                &mut store,
                &package.schema,
                Element::new(
                    &package.schema,
                    "owner",
                    package.schema.language_class_key("Holder")?,
                    "memory:model",
                    0,
                    0,
                )?,
            )?;
            store.commit(&checkpoint)?;
            storage::edit_with_conversions(
                &mut store,
                &package.schema,
                &[("owner".into(), "refs".into(), json!(value))],
                registry,
            )?;
            storage::validate_with_conversions(&store, &package.schema, registry)?;
            std::fs::write(&package_path, serde_json::to_vec(&package).unwrap()).unwrap();
            storage::export(&store)
        },
    )
    .unwrap();
    let package: Package = serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
    let input = package.compilation.clone().unwrap();
    with_catalog_policies(
        &input.bundle,
        &package.schema.conversion_contracts,
        &package.schema.applied_conversion_contracts,
        &package.schema.constructor_contracts,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(rebuilt), json!(package));
            let mut store = SqliteStore::open(&path)?;
            storage::validate_with_conversions(&store, &rebuilt.schema, registry)?;
            assert_eq!(storage::export(&store)?, expected);
            let adapter = applied_collection_provider_with_conversions(
                &input.bundle,
                &f.applications[0],
                registry,
            )?;
            let mut reference = adapter.bind_reference(0, "sub")?;
            reference["context"] = json!("stale");
            let mut wrong = json!(
                registry
                    .applied()
                    .bind(&f.applications[0].application, json!({"items":[]}))?
            );
            wrong["converted"]["value"]["items"] = json!([reference]);
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &rebuilt.schema,
                    &[
                        ("owner".into(), "name".into(), json!("rollback")),
                        ("owner".into(), "refs".into(), wrong)
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
    let store = SqliteStore::open(&path).unwrap();
    assert_eq!(storage::export(&store).unwrap(), expected);
    drop(store);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(package_path).unwrap();
}
