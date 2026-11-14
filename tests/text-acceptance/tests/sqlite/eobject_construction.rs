#![cfg(not(target_arch = "wasm32"))]
#[path = "../language/common/eobject_construction.rs"]
mod common;
use intersect_model::core::storage;
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, evaluate_into};
use serde_json::json;
#[test]
fn constructed_pinned_eobject_and_inherited_operations_survive_three_database_opens() {
    let input = common::fixture();
    let package = compile_bundle(input.clone()).unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-eobject-construction-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let saved = path.with_extension("json");
    let expected = {
        let mut store = SqliteStore::open(&path).unwrap();
        let outcome = evaluate_into(
            &package,
            &[common::source()],
            EvaluationLimits::native(),
            &mut store,
        )
        .unwrap();
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        let model = storage::export(&store).unwrap();
        let pinned = package
            .schema
            .class_key_for_type(&input.materialized_types[0])
            .unwrap();
        let object = model.elements.iter().find(|e| e.class == pinned).unwrap();
        storage::edit(
            &mut store,
            &package.schema,
            &[(model.elements[0].id.clone(), "any".into(), json!(object.id))],
        )
        .unwrap();
        storage::validate(&store, &package.schema).unwrap();
        std::fs::write(&saved, serde_json::to_vec(&package).unwrap()).unwrap();
        storage::export(&store).unwrap()
    };
    let retained: Package = serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();
    let rebuilt = compile_bundle(retained.compilation.clone().unwrap()).unwrap();
    assert_eq!(json!(rebuilt), json!(retained));
    let pinned = rebuilt
        .schema
        .class_key_for_type(&input.materialized_types[0])
        .unwrap();
    assert_eq!(
        rebuilt.schema.effective_operations(pinned).unwrap().len(),
        15
    );
    {
        let mut store = SqliteStore::open(&path).unwrap();
        storage::validate(&store, &rebuilt.schema).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
        let root = &expected.elements[0];
        let object = expected
            .elements
            .iter()
            .find(|e| e.class == pinned)
            .unwrap();
        let other = expected
            .elements
            .iter()
            .find(|e| e.values.get("name") == Some(&json!("O")))
            .unwrap();
        for invalid in [json!(object.id), json!("other-model#0")] {
            assert!(
                storage::edit(
                    &mut store,
                    &rebuilt.schema,
                    &[
                        (root.id.clone(), "any".into(), json!(other.id)),
                        (root.id.clone(), "local".into(), invalid)
                    ]
                )
                .is_err()
            );
            assert_eq!(storage::export(&store).unwrap(), expected);
        }
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        storage::validate(&store, &rebuilt.schema).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(saved).unwrap();
}
