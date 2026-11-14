#![cfg(not(target_arch = "wasm32"))]
use intersect_model::conversion::with_catalog;
use intersect_model::core::storage::{self, ModelRead, Query};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::standard_datatype,
    conversion::ConversionContract,
};
use intersect_model::sqlite::SqliteStore;
use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
use serde_json::json;
#[test]
fn invocation_error_snapshots_reopen_query_and_rollback_through_same_provider() {
    let ecore = include_str!("../../../../fixtures/invocation-errors/errors.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, ecore);
    let make = |id| {
        serde_json::from_value::<ConversionContract>(json!({"datatype":id,"provider":{"name":"ecore.invocation-error","version":"1"},"serializable":false})).unwrap()
    };
    let contracts = [
        make(json!(
            standard_datatype("EInvocationTargetException")
                .unwrap()
                .declaration()
        )),
        make(json!({"source":source.id,"path":[0]})),
    ];
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let (bundle, contracts) =
        intersect_text_acceptance::migration_fixture::migrate_provider_fixture(
            bundle.sources[0].clone(),
            &contracts,
        );
    with_catalog(&bundle,&contracts,|registry| {
        let package=intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(ecore,include_str!("../../../../fixtures/invocation-errors/errors.xtext"),include_str!("../../../../fixtures/invocation-errors/errors-behavior.rcore"),registry).unwrap();
        let first=registry.bind(&contracts[0].datatype,json!({"message":null,"target":null}))?;
        let second=registry.bind(&contracts[0].datatype,json!({"message":"","target":{"domain":{"name":"model.check","version":"1"},"code":"invalid","message":"details"}}))?;
        assert!(registry.format(&first).is_err()); assert!(registry.format(&second).is_err());
        let first=serde_json::to_value(first).unwrap();let second=serde_json::to_value(second).unwrap();
        let path=std::env::temp_dir().join(format!("intersect-invocation-error-{}.sqlite",std::process::id()));
        let expected;
        {
            let mut store=SqliteStore::open(&path)?;
            let result=evaluate_into_with_conversions(&package,&[Source{uri:"sample.errors".into(),text:"attempt example".into()}],EvaluationLimits::native(),&mut store,registry)?;
            assert!(result.diagnostics.is_empty());
            let id=storage::export(&store)?.elements[0].id.clone();
            storage::edit_with_conversions(&mut store,&package.schema,&[(id.clone(),"failure".into(),second.clone()),(id.clone(),"history".into(),json!([first,second]))],registry)?;
            expected=storage::export(&store)?;
            let mut bad=second.clone();bad["value"]["target"]["domain"]["version"]=json!("");
            assert!(storage::edit_with_conversions(&mut store,&package.schema,&[(id.clone(),"failure".into(),first.clone()),(id,"history".into(),json!([bad]))],registry).is_err());
            assert_eq!(storage::export(&store)?,expected);
        }
        {
            let store=SqliteStore::open(&path)?;
            storage::validate_with_conversions(&store,&package.schema,registry)?;
            assert!(storage::validate(&store,&package.schema).is_err());
            assert_eq!(storage::export(&store)?,expected);
            for value in [&first,&second] {assert_eq!(store.query(Query::Value("history",value))?,[expected.elements[0].id.clone()]);}
        }
        std::fs::remove_file(path).unwrap(); Ok(())
    }).unwrap();
}
