#![cfg(not(target_arch = "wasm32"))]
use intersect_model::core::{
    model::Element,
    storage::{self, ModelStore},
};
use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use intersect_model::sqlite::SqliteStore;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::{BundleCompilation, Package};
use serde_json::json;

#[test]
fn reference_top_domain_survives_database_reopen_and_rejected_edits() {
    let ecore = DefinitionSource::new(
        "memory:raw-unbounded",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/raw-unbounded.ecore"),
    );
    let mut grammar = DefinitionSource::new(
        "memory:raw-unbounded.xtext",
        SourceKind::Xtext,
        "grammar example.Raw import \"urn:raw-unbounded\" as u Root returns u::Box: 'raw';",
    );
    grammar.dependencies.push(ecore.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "memory:raw-unbounded-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior.dependencies.push(ecore.id.clone());
    let mut input: BundleCompilation = serde_json::from_value(json!({
        "bundle":{"version":1,"sources":[ecore,grammar,rcore_behavior],"namespaces":[]},
        "ecore":ecore.id,"namespace":"urn:raw-unbounded","xtext":grammar.id,"rcore_behavior":rcore_behavior.id,
        "materialized_types":[
            {"kind":"classifier","declaration":{"source":ecore.id,"path":[2]},"arguments":[],"raw":true},
            {"kind":"classifier","declaration":{"source":ecore.id,"path":[1]},"arguments":[],"raw":false}
        ]
    })).unwrap();

    input.materialized_types.push(serde_json::from_value(json!({"kind":"classifier","declaration":{"source":input.ecore,"path":[0]},"arguments":[],"raw":false})).unwrap());
    let input =
        intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(input).unwrap();
    let package = compile_bundle(input.clone()).unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-raw-top-{}-{}.sqlite",
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
        for (index, id) in ["r", "o", "v"].iter().enumerate() {
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
            &[("r".into(), "value".into(), json!("o"))],
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
        for invalid in [json!("missing"), json!(12)] {
            assert!(
                storage::edit(
                    &mut store,
                    &rebuilt.schema,
                    &[
                        ("v".into(), "name".into(), json!("rollback")),
                        ("r".into(), "value".into(), invalid),
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
    std::fs::remove_file(package_path).unwrap();
}
