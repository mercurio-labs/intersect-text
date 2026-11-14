#![cfg(not(target_arch = "wasm32"))]
use intersect_text::package::Package;
#[path = "../language/common/runtime_operation_providers.rs"]
mod common;
use intersect_model::conversion::with_catalog_policies;
use intersect_model::core::storage;
use intersect_model::definitions::{
    conversion::{ConstructorContract, ConversionContract},
    generic_resolution::{Resolver, RuntimeAppliedOperation},
};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
use serde_json::json;
#[test]
fn raw_operation_method_policies_and_model_identity_survive_database_reopen() {
    let f = common::fixture();
    let path = std::env::temp_dir().join(format!(
        "intersect-runtime-methods-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let inputs = path.with_extension("json");
    let expected = with_catalog_policies(
        &f.input.bundle,
        &f.conversions,
        &[],
        &f.constructors,
        |registry| {
            let package = compile_bundle_with_conversions(f.input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            let resolver = Resolver::new(&f.input.bundle)
                .map_err(|e| e.to_string())?
                .with_conversions(registry)
                .map_err(|e| e.to_string())?;
            let reports = f
                .operations
                .iter()
                .zip(&f.arguments)
                .map(|(op, arg)| {
                    package.schema.instantiate_runtime_operation(
                        &resolver,
                        &f.receiver,
                        op,
                        Some(f.receiver.expression()),
                        std::slice::from_ref(arg),
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let mut store = SqliteStore::open(&path)?;
            let outcome = evaluate_into_with_conversions(
                &package,
                &[Source {
                    uri: "memory:methods.model".into(),
                    text: "holder Saved".into(),
                }],
                EvaluationLimits::native(),
                &mut store,
                registry,
            )?;
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            storage::validate_with_conversions(&store, &package.schema, registry)?;
            std::fs::write(
                &inputs,
                serde_json::to_vec(&(
                    package,
                    f.conversions.clone(),
                    f.constructors.clone(),
                    reports,
                ))
                .unwrap(),
            )
            .unwrap();
            storage::export(&store)
        },
    )
    .unwrap();
    let (package, conversions, constructors, reports): (
        Package,
        Vec<ConversionContract>,
        Vec<ConstructorContract>,
        Vec<RuntimeAppliedOperation>,
    ) = serde_json::from_slice(&std::fs::read(&inputs).unwrap()).unwrap();
    let input = package.compilation.clone().unwrap();
    with_catalog_policies(
        &input.bundle,
        &conversions,
        &[],
        &constructors,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(rebuilt), json!(package));
            let resolver = Resolver::new(&input.bundle)
                .map_err(|e| e.to_string())?
                .with_conversions(registry)
                .map_err(|e| e.to_string())?;
            for report in &reports {
                let projection = &report.projection;
                assert_eq!(
                    rebuilt
                        .schema
                        .instantiate_runtime_operation(
                            &resolver,
                            &projection.receiver,
                            &projection.source.declaration,
                            Some(&projection.source.owner),
                            &report.arguments
                        )
                        .map_err(|e| e.to_string())?,
                    *report
                );
                let mut owner = projection.source.owner.clone();
                if let intersect_model::definitions::generic_types::Type::Classifier {
                    declaration,
                    ..
                } = &mut owner
                {
                    declaration.source.sha256 = "0".repeat(64);
                }
                assert!(
                    rebuilt
                        .schema
                        .instantiate_runtime_operation(
                            &resolver,
                            &projection.receiver,
                            &projection.source.declaration,
                            Some(&owner),
                            &report.arguments
                        )
                        .is_err()
                );
            }
            let mut store = SqliteStore::open(&path)?;
            storage::validate_with_conversions(&store, &rebuilt.schema, registry)?;
            assert_eq!(storage::export(&store)?, expected);
            let id = expected.elements[0].id.clone();
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &rebuilt.schema,
                    &[
                        (id.clone(), "name".into(), json!("rollback")),
                        (id, "name".into(), json!(7))
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
    let mut stale = constructors.clone();
    stale[0].provider.version = "stale".into();
    assert!(with_catalog_policies(&input.bundle, &conversions, &[], &stale, |_| Ok(())).is_err());
    let mut stale = conversions.clone();
    stale[0].datatype.source.sha256 = "0".repeat(64);
    assert!(with_catalog_policies(&input.bundle, &stale, &[], &constructors, |_| Ok(())).is_err());
    {
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(inputs).unwrap();
}
