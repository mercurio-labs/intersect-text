#![cfg(not(target_arch = "wasm32"))]
#[path = "../language/common/runtime_lower_text.rs"]
mod common;
use intersect_model::core::storage;
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate_into};
use serde_json::json;

#[test]
fn runtime_lower_domains_survive_three_opens_and_rejected_imports() {
    let path = std::env::temp_dir().join(format!(
        "intersect-runtime-lower-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let package_path = path.with_extension("package.json");
    let expected = {
        let package = compile_bundle(common::fixture()).unwrap();
        let mut store = SqliteStore::open(&path).unwrap();
        let report = evaluate_into(
            &package,
            &[common::source()],
            EvaluationLimits::native(),
            &mut store,
        )
        .unwrap();
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        storage::validate(&store, &package.schema).unwrap();
        std::fs::write(&package_path, serde_json::to_vec(&package).unwrap()).unwrap();
        storage::export(&store).unwrap()
    };
    let retained: Package = serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
    let rebuilt = compile_bundle(retained.compilation.clone().unwrap()).unwrap();
    assert_eq!(json!(rebuilt), json!(retained));
    {
        let mut store = SqliteStore::open(&path).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
        storage::validate(&store, &rebuilt.schema).unwrap();
        for (text, spelling, message) in [
            ("model { good G raw R Missing }", "Missing", "Unresolved"),
            ("model { narrow N raw R N }", "N", "Unresolved"),
            ("model { bad B closed C B }", "B", "Unresolved"),
        ] {
            let report = evaluate_into(
                &rebuilt,
                &[Source {
                    uri: "memory:rejected".into(),
                    text: text.into(),
                }],
                EvaluationLimits::native(),
                &mut store,
            )
            .unwrap();
            let error = report
                .diagnostics
                .iter()
                .find(|error| error.message.contains(message))
                .unwrap();
            let start = text.rfind(spelling).unwrap();
            assert_eq!((error.start, error.end), (start, start + spelling.len()));
            assert_eq!(storage::export(&store).unwrap(), expected);
            storage::validate(&store, &rebuilt.schema).unwrap();
        }
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
        storage::validate(&store, &rebuilt.schema).unwrap();
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(package_path).unwrap();
}
