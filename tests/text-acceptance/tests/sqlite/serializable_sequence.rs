#![cfg(not(target_arch = "wasm32"))]
#[path = "../language/common/serializable_sequence.rs"]
mod common;
use intersect_model::conversion::with_catalog_context;
use intersect_model::core::storage::{self, ModelStore};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
use serde_json::json;

#[test]
fn lexical_generic_values_survive_actual_sqlite_reopen_and_rejected_text_import() {
    let (input, contracts) = common::input();
    let path = std::env::temp_dir().join(format!(
        "intersect-sequence-{}-{}.sqlite",
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
        let mut store = SqliteStore::open(&path)?;
        let report = evaluate_into_with_conversions(
            &package,
            &[common::source()],
            EvaluationLimits::native(),
            &mut store,
            registry,
        )?;
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        storage::validate_with_conversions(&store, &package.schema, registry)?;
        std::fs::write(&package_path, serde_json::to_vec(&package).unwrap()).unwrap();
        storage::export(&store)
    })
    .unwrap();
    let package: Package = serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
    let input = package.compilation.clone().unwrap();
    with_catalog_context(
        &input.bundle,
        &package.schema.conversion_contracts,
        &package.schema.applied_conversion_contracts,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(rebuilt), json!(package));
            let mut store = SqliteStore::open(&path)?;
            assert_eq!(storage::export(&store)?, expected);
            storage::validate_with_conversions(&store, &rebuilt.schema, registry)?;
            let bad = evaluate_into_with_conversions(
                &rebuilt,
                &[Source {
                    uri: "memory:bad".into(),
                    text: "holder Bad texts 'invalid'".into(),
                }],
                EvaluationLimits::native(),
                &mut store,
                registry,
            )?;
            assert!(!bad.diagnostics.is_empty());
            assert_eq!(storage::export(&store)?, expected);
            let wrong = registry
                .applied()
                .parse(&contracts[1].application, r#"["4"]"#)?;
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &rebuilt.schema,
                    &[
                        (
                            "memory:sequence.model#0".into(),
                            "name".into(),
                            json!("rollback")
                        ),
                        (
                            "memory:sequence.model#0".into(),
                            "texts".into(),
                            json!(wrong)
                        )
                    ],
                    registry
                )
                .is_err()
            );
            assert_eq!(storage::export(&store)?, expected);
            // Explicit transaction roundtrip confirms no hidden open failed-import transaction.
            let checkpoint = store.begin()?;
            store.commit(&checkpoint)?;
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
