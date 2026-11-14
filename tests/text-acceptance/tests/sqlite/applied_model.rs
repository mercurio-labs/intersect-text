#![cfg(not(target_arch = "wasm32"))]
#[path = "../language/common/applied_model.rs"]
mod common;
use intersect_model::conversion::with_catalog_context;
use intersect_model::core::{
    model::Element,
    storage::{self, ModelStore},
};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle_with_conversions;
use serde_json::json;
#[test]
fn applied_attributes_reopen_with_reconstructed_package_and_atomic_assignment_checks() {
    let (input, contracts) = common::input();
    let path = std::env::temp_dir().join(format!(
        "intersect-applied-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let package_path = path.with_extension("package.json");
    let expected = with_catalog_context(&input.bundle, &[], &contracts, |registry| {
        let package = compile_bundle_with_conversions(input.clone(), registry)
            .map_err(|e| format!("{e:?}"))?;
        let schema = &package.schema;
        let mut store = SqliteStore::open(&path)?;
        let checkpoint = store.begin()?;
        storage::insert(
            &mut store,
            schema,
            Element::new(
                schema,
                "h",
                schema.language_class_key("Holder")?,
                "memory:model",
                0,
                0,
            )?,
        )?;
        store.commit(&checkpoint)?;
        let text = registry.applied().bind(
            &contracts[0].application,
            json!({"items":[common::nominal(&input,0,json!("saved"))]}),
        )?;
        let nested = registry
            .applied()
            .bind(&contracts[2].application, json!({"items":[text.clone()]}))?;
        storage::edit_with_conversions(
            &mut store,
            schema,
            &[
                ("h".into(), "texts".into(), json!(text)),
                ("h".into(), "nested".into(), json!(nested)),
            ],
            registry,
        )?;
        let expected = storage::export(&store)?;
        std::fs::write(&package_path, serde_json::to_vec(&package).unwrap()).unwrap();
        Ok(expected)
    })
    .unwrap();
    let retained: intersect_text::package::Package =
        serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
    let replay = retained.compilation.clone().unwrap();
    with_catalog_context(
        &replay.bundle,
        &retained.schema.conversion_contracts,
        &retained.schema.applied_conversion_contracts,
        |registry| {
            let package = compile_bundle_with_conversions(replay.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                serde_json::to_value(&package).unwrap(),
                serde_json::to_value(&retained).unwrap()
            );
            let mut store = SqliteStore::open(&path)?;
            storage::validate_with_conversions(&store, &package.schema, registry)?;
            assert_eq!(storage::export(&store)?, expected);
            let wrong = registry.applied().bind(
                &contracts[1].application,
                json!({"items":[common::nominal(&input,1,json!(42))]}),
            )?;
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &package.schema,
                    &[
                        ("h".into(), "name".into(), json!("must roll back")),
                        ("h".into(), "texts".into(), json!(wrong))
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
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(package_path).unwrap();
}
