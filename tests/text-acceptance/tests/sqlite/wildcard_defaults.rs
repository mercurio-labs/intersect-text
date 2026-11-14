#![cfg(not(target_arch = "wasm32"))]
use intersect_text::package::{BundleCompilation, Package};
#[path = "../language/common/wildcard_defaults.rs"]
mod defaults;
#[allow(dead_code)]
#[path = "../language/common/constructor_wildcards.rs"]
mod wildcards;
use intersect_model::conversion::with_catalog_policies;
use intersect_model::core::storage;
use intersect_model::definitions::conversion::{AppliedConversionContract, ConstructorContract};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
use serde_json::{Value, json};
fn reopen(
    input: BundleCompilation,
    applications: Vec<AppliedConversionContract>,
    constructors: Vec<ConstructorContract>,
    text: &str,
    edit: Option<Value>,
) {
    let path = std::env::temp_dir().join(format!(
        "intersect-wildcard-defaults-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let package_path = path.with_extension("package.json");
    let expected = with_catalog_policies(
        &input.bundle,
        &[],
        &applications,
        &constructors,
        |registry| {
            let package = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            let mut store = SqliteStore::open(&path)?;
            let outcome = evaluate_into_with_conversions(
                &package,
                &[Source {
                    uri: "memory:reopen".into(),
                    text: text.into(),
                }],
                EvaluationLimits::native(),
                &mut store,
                registry,
            )?;
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            if let Some(value) = edit {
                storage::edit_with_conversions(
                    &mut store,
                    &package.schema,
                    &[("memory:reopen#0".into(), "values".into(), value)],
                    registry,
                )?;
            }
            storage::validate_with_conversions(&store, &package.schema, registry)?;
            std::fs::write(&package_path, serde_json::to_vec(&package).unwrap()).unwrap();
            storage::export(&store)
        },
    )
    .unwrap();
    let retained: Package = serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
    let input = retained.compilation.clone().unwrap();
    with_catalog_policies(
        &input.bundle,
        &retained.schema.conversion_contracts,
        &retained.schema.applied_conversion_contracts,
        &retained.schema.constructor_contracts,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(retained), json!(rebuilt));
            let mut store = SqliteStore::open(&path)?;
            assert_eq!(storage::export(&store)?, expected);
            storage::validate_with_conversions(&store, &rebuilt.schema, registry)?;
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &rebuilt.schema,
                    &[
                        ("memory:reopen#0".into(), "name".into(), json!("rollback")),
                        (
                            "memory:reopen#0".into(),
                            "values".into(),
                            json!("wrong application")
                        )
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
#[test]
fn populated_alias_and_diamond_defaults_survive_sqlite_reopen() {
    for diamond in [false, true] {
        let f = defaults::fixture(diamond);
        let _ = f.selected;
        reopen(f.input, f.applications, f.constructors, "root Demo", None);
    }
}
#[test]
fn installed_lower_wildcard_values_survive_sqlite_reopen() {
    let bound = r##"<eBounds eClassifier="#//Collection"><eTypeArguments><eLowerBound eClassifier="#//EList"><eTypeArguments eClassifier="#//Text"/></eLowerBound></eTypeArguments></eBounds>"##;
    let f = wildcards::fixture(
        bound,
        wildcards::app(4, vec![wildcards::app(3, vec![wildcards::scalar(0)])]),
        &[2, 3, 4, 5],
    );
    reopen(
        f.input,
        f.applications,
        f.constructors,
        "holder Demo",
        Some(f.value),
    );
}
