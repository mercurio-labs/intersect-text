#![cfg(not(target_arch = "wasm32"))]
use intersect_text::package::Package;
#[path = "../language/common/raw_runtime.rs"]
mod common;
use intersect_model::core::{
    model::Element,
    storage::{self, ModelStore},
};
use intersect_model::definitions::generic_types::{RuntimeType, Type};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle;
use serde_json::json;
#[test]
fn raw_identity_and_erased_reference_constraints_survive_database_reopen() {
    let mut input = common::input();
    let mut other = input.materialized_types[1].expression().clone();
    if let Type::Classifier { declaration, .. } = &mut other {
        declaration.path = vec![2];
    }
    input
        .materialized_types
        .push(RuntimeType::new(other).unwrap());
    let package = compile_bundle(input.clone()).unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-raw-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let package_path = path.with_extension("package.json");
    let expected = {
        let mut store = SqliteStore::open(&path).unwrap();
        let checkpoint = store.begin().unwrap();
        for (index, id) in ["r", "v", "o"].iter().enumerate() {
            storage::insert(
                &mut store,
                &package.schema,
                Element::new(
                    &package.schema,
                    id,
                    package
                        .schema
                        .class_key_for_type(&input.materialized_types[index])
                        .unwrap(),
                    "memory:model",
                    0,
                    0,
                )
                .unwrap(),
            )
            .unwrap();
        }
        store.commit(&checkpoint).unwrap();
        storage::edit(
            &mut store,
            &package.schema,
            &[("r".into(), "value".into(), json!("v"))],
        )
        .unwrap();
        storage::validate(&store, &package.schema).unwrap();
        std::fs::write(&package_path, serde_json::to_vec(&package).unwrap()).unwrap();
        storage::export(&store).unwrap()
    };
    let saved: Package = serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
    let rebuilt = compile_bundle(saved.compilation.clone().unwrap()).unwrap();
    assert_eq!(rebuilt.schema, saved.schema);
    {
        let mut store = SqliteStore::open(&path).unwrap();
        storage::validate(&store, &rebuilt.schema).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
        assert!(
            storage::edit(
                &mut store,
                &rebuilt.schema,
                &[
                    ("v".into(), "name".into(), json!("rollback")),
                    ("r".into(), "value".into(), json!("o"))
                ]
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(package_path).unwrap();
}
