#![cfg(not(target_arch = "wasm32"))]
use intersect_model::core::storage::{self, ModelRead, Query};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::{import_enum_bundle, standard_datatype},
    conversion::{ConversionContract, ConversionRegistry, ProviderId},
    resources::DeclarationId,
};
use intersect_model::enumerator_conversion::EnumeratorConverter;
use intersect_model::sqlite::SqliteStore;
use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
use serde_json::json;

#[test]
fn nonserializable_enumerator_snapshots_reopen_without_lexical_serialization() {
    let ecore = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='values' nsURI='urn:values'><eClassifiers xsi:type='e:EEnum' name='Mode' serializable='false'><eLiterals name='First' value='1'/><eLiterals name='Alias' value='1'/></eClassifiers><eClassifiers xsi:type='e:EClass' name='Item'><eStructuralFeatures xsi:type='e:EAttribute' name='values' upperBound='-1' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EEnumerator'/></eClassifiers></e:EPackage>";
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, ecore);
    let id = DeclarationId {
        source: source.id.clone(),
        path: vec![0],
    };
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let enumeration = import_enum_bundle(&bundle, &id).unwrap();
    let datatype = standard_datatype("EEnumerator").unwrap();
    let converter = EnumeratorConverter::new(
        ConversionContract {
            members: Vec::new(),
            declarations: Vec::new(),
            datatype: datatype.declaration().clone(),
            provider: ProviderId {
                name: "test.enum".into(),
                version: "1".into(),
            },
            serializable: false,
        },
        [enumeration.clone()],
    )
    .unwrap();
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let bind = |ordinal: usize| {
        let value = enumeration
            .value_by_id(&enumeration.literals()[ordinal].declaration)
            .unwrap();
        let value = registry
            .bind(datatype.declaration(), serde_json::to_value(value).unwrap())
            .unwrap();
        assert!(registry.format(&value).is_err());
        serde_json::to_value(value).unwrap()
    };
    let first = bind(0usize);
    let alias = bind(1usize);
    assert_ne!(first, alias);
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
            ecore,
            "grammar example.Values import 'urn:values' as v Item returns v::Item: 'item';",
            r#"rcore 1; package emptyRules version "1";"#,
            &registry,
        )
        .unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-enumerator-{}.sqlite",
        std::process::id()
    ));
    let expected;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        let result = evaluate_into_with_conversions(
            &package,
            &[Source {
                uri: "values.model".into(),
                text: "item".into(),
            }],
            EvaluationLimits::native(),
            &mut store,
            &registry,
        )
        .unwrap();
        assert!(result.diagnostics.is_empty());
        let id = storage::export(&store).unwrap().elements[0].id.clone();
        storage::edit_with_conversions(
            &mut store,
            &package.schema,
            &[(id.clone(), "values".into(), json!([first, alias]))],
            &registry,
        )
        .unwrap();
        expected = storage::export(&store).unwrap();
        let mut invalid = alias.clone();
        invalid["value"]["literal"]["path"] = json!([0, 99]);
        assert!(
            storage::edit_with_conversions(
                &mut store,
                &package.schema,
                &[
                    (id.clone(), "values".into(), json!([])),
                    (id, "values".into(), json!([invalid]))
                ],
                &registry
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        storage::validate_with_conversions(&store, &package.schema, &registry).unwrap();
        assert!(storage::validate(&store, &package.schema).is_err());
        assert_eq!(storage::export(&store).unwrap(), expected);
        for value in [&first, &alias] {
            assert_eq!(
                store.query(Query::Value("values", value)).unwrap(),
                [expected.elements[0].id.clone()]
            );
        }
    }
    std::fs::remove_file(path).unwrap();
}
