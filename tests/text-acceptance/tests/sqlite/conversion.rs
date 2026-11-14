#![cfg(not(target_arch = "wasm32"))]
use super::Store;
use intersect_model::conversion::RgbConverter;
use intersect_model::core::{
    model::Element,
    storage::{self, ModelRead, Query},
};
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    conversion::{ConversionContract, ConversionRegistry, ProviderId},
    datatypes::Datatype,
    resources::DeclarationId,
};
use intersect_model::sqlite::SqliteStore;
use serde_json::json;

control_test! {
pub fn custom_values_reopen_query_and_rollback_through_shared_storage_services() {
    let mut schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
    )
    .unwrap();
    let source = DefinitionSource::new("memory:rgb.ecore", SourceKind::Ecore, "RGB declaration");
    let datatype = Datatype::new(
        DeclarationId {
            source: source.id,
            path: vec![0],
        },
        "RGB".into(),
        true,
        Some("example.RGB".into()),
        None,
    )
    .unwrap();
    let converter = RgbConverter::new(ConversionContract {
        members: Vec::new(),
        declarations: Vec::new(),
        datatype: datatype.declaration().clone(),
        provider: ProviderId {
            name: "example.rgb".into(),
            version: "1".into(),
        },
        serializable: true,
    });
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    schema.datatypes.insert("RGB".into(), datatype.clone());
    schema
        .conversion_contracts
        .push(registry.require(&datatype).unwrap().clone());
    let field = schema
        .classes
        .get_mut("BookCopy")
        .unwrap()
        .features
        .get_mut("title")
        .unwrap();
    field.target = "RGB".into();
    field.datatype = Some(datatype.declaration().clone());
    let value =
        serde_json::to_value(registry.parse(datatype.declaration(), "#123ABC").unwrap()).unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-custom-conversion-{}.sqlite",
        std::process::id()
    ));
    let expected;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        storage::atomic(&mut store, |store| {
            storage::insert(
                store,
                &schema,
                Element::new(&schema, "copy", "BookCopy", "api", 0, 0)?,
            )
        })
        .unwrap();
        storage::edit_with_conversions(
            &mut store,
            &schema,
            &[
                ("copy".into(), "name".into(), json!("copy")),
                ("copy".into(), "title".into(), value.clone()),
            ],
            &registry,
        )
        .unwrap();
        expected = storage::export(&store).unwrap();
        let mut wrong = value.clone();
        wrong["provider"]["version"] = json!("2");
        assert!(
            storage::edit_with_conversions(
                &mut store,
                &schema,
                &[
                    ("copy".into(), "name".into(), json!("rollback")),
                    ("copy".into(), "title".into(), wrong),
                ],
                &registry
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
        assert!(
            store
                .query(Query::Value("name", &json!("rollback")))
                .unwrap()
                .is_empty()
        );
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        storage::validate_with_conversions(&store, &schema, &registry).unwrap();
        assert!(storage::validate(&store, &schema).is_err());
        assert!(
            storage::validate_with_conversions(&store, &schema, &ConversionRegistry::default())
                .is_err()
        );
        assert_eq!(
            store.query(Query::Value("title", &value)).unwrap(),
            ["copy"]
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    std::fs::remove_file(path).unwrap();
}
}

store_test! {
pub fn pinned_class_descriptors_persist_exact_identity_and_failed_list_edits_restore_indexes() {
    use intersect_model::class_conversion::ClassConverter;
    use intersect_model::core::storage::{ModelStore};
    use intersect_model::definitions::compiler::standard_datatype;
    use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
    let datatype = standard_datatype("EJavaClass").unwrap();
    let contract = ConversionContract {
        members: Vec::new(),
        declarations: Vec::new(),
        datatype: datatype.declaration().clone(),
        provider: ProviderId {
            name: "ecore.class".into(),
            version: "1".into(),
        },
        serializable: true,
    };
    let converter = ClassConverter::portable_core(contract.clone()).unwrap();
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let ecore = r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="types" nsURI="urn:types">
      <eClassifiers xsi:type="e:EClass" name="Item">
        <eStructuralFeatures xsi:type="e:EAttribute" name="type" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EJavaClass" defaultValueLiteral="[Ljava.lang.String;"/>
        <eStructuralFeatures xsi:type="e:EAttribute" name="types" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EJavaClass" upperBound="-1"/>
      </eClassifiers>
    </e:EPackage>"#;
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
            ecore,
            "grammar example.Types import 'urn:types' as t Item returns t::Item: 'item';",
            r#"rcore 1; package emptyRules version "1";"#,
            &registry,
        )
        .unwrap();
    let sources = [Source {
        uri: "types.model".into(),
        text: "item".into(),
    }];
    let value =
        |text| serde_json::to_value(registry.parse(datatype.declaration(), text).unwrap()).unwrap();
    let named = value("[Ljava.lang.String;");
    let primitive = value("[[I");
    let path = std::env::temp_dir().join(format!(
        "intersect-class-descriptors-{}-{}.sqlite",
        std::process::id(), <Store as crate::stores::StoreFixture>::NAME
    ));
    let expected;
    {
        let mut memory = crate::stores::store::<Store>(&package.schema).unwrap();
        let mut sqlite = SqliteStore::open(&path).unwrap();
        let a = evaluate_into_with_conversions(
            &package,
            &sources,
            EvaluationLimits::native(),
            &mut memory,
            &registry,
        )
        .unwrap();
        let b = evaluate_into_with_conversions(
            &package,
            &sources,
            EvaluationLimits::native(),
            &mut sqlite,
            &registry,
        )
        .unwrap();
        assert!(a.diagnostics.is_empty());
        assert!(b.diagnostics.is_empty());
        for store in [
            &mut memory as &mut dyn ModelStore,
            &mut sqlite as &mut dyn ModelStore,
        ] {
            let model = storage::export(store).unwrap();
            let id = model.elements[0].id.clone();
            assert_eq!(model.elements[0].values["type"], named);
            storage::edit_with_conversions(
                store,
                &package.schema,
                &[(id.clone(), "types".into(), json!([named, primitive]))],
                &registry,
            )
            .unwrap();
            let before = storage::export(store).unwrap();
            let mut stale = named.clone();
            stale["value"]["component"]["identity"]["version"] = json!("2");
            let mut bad_rank = primitive.clone();
            bad_rank["value"]["dimensions"] = json!(256);
            for invalid in [
                json!([stale]),
                json!([bad_rank]),
                json!([primitive, primitive]),
                json!([null]),
            ] {
                assert!(
                    storage::edit_with_conversions(
                        store,
                        &package.schema,
                        &[
                            (id.clone(), "type".into(), primitive.clone()),
                            (id.clone(), "types".into(), invalid)
                        ],
                        &registry
                    )
                    .is_err()
                );
                assert_eq!(storage::export(store).unwrap(), before);
                assert_eq!(
                    store.query(Query::Value("type", &named)).unwrap(),
                    std::slice::from_ref(&id)
                );
                assert!(
                    store
                        .query(Query::Value("type", &primitive))
                        .unwrap()
                        .is_empty()
                );
                assert_eq!(
                    store.query(Query::Value("types", &primitive)).unwrap(),
                    std::slice::from_ref(&id)
                );
            }
        }
        let memory_expected = storage::export(&memory).unwrap();
        expected = storage::export(&sqlite).unwrap();
        assert_eq!(
            semantic_model(expected.clone()),
            semantic_model(memory_expected)
        );
    }
    {
        let sqlite = SqliteStore::open(&path).unwrap();
        storage::validate_with_conversions(&sqlite, &package.schema, &registry).unwrap();
        assert_eq!(storage::export(&sqlite).unwrap(), expected);
        let id = expected.elements[0].id.clone();
        assert_eq!(
            sqlite.query(Query::Value("type", &named)).unwrap(),
            std::slice::from_ref(&id)
        );
        assert_eq!(
            sqlite.query(Query::Value("types", &primitive)).unwrap(),
            [id]
        );
        assert!(
            storage::validate_with_conversions(
                &sqlite,
                &package.schema,
                &ConversionRegistry::default()
            )
            .is_err()
        );
        let changed = ClassConverter::new(
            contract,
            std::collections::BTreeMap::from([(
                "java.lang.String".into(),
                ProviderId {
                    name: "intersect.portable-java-core".into(),
                    version: "2".into(),
                },
            )]),
        )
        .unwrap();
        let mut changed_registry = ConversionRegistry::default();
        changed_registry.register(&datatype, &changed).unwrap();
        assert!(
            storage::validate_with_conversions(&sqlite, &package.schema, &changed_registry)
                .is_err()
        );
        assert_eq!(storage::export(&sqlite).unwrap(), expected);
    }
    std::fs::remove_file(path).unwrap();
}
}

control_test! {
pub fn standard_object_source_ingestion_reopens_and_preserves_provider_identity() {
    use intersect_model::definitions::compiler::standard_datatype;
    use intersect_text::{EvaluationLimits, Source, evaluate_into_with_conversions};
    let datatype = standard_datatype("EJavaObject").unwrap();
    let converter = RgbConverter::new(ConversionContract {
        members: Vec::new(),
        declarations: Vec::new(),
        datatype: datatype.declaration().clone(),
        provider: ProviderId {
            name: "example.rgb".into(),
            version: "1".into(),
        },
        serializable: true,
    });
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let lexical = registry.parse(datatype.declaration(), "#ABCDEF").unwrap();
    assert_eq!(registry.format(&lexical).unwrap(), "#ABCDEF");
    assert_eq!(
        registry.default_value(datatype.declaration()).unwrap(),
        None
    );
    let value = serde_json::to_value(lexical).unwrap();
    let ecore = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='objects' nsURI='urn:objects'><eClassifiers xsi:type='e:EClass' name='Item'><eStructuralFeatures xsi:type='e:EAttribute' name='object' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EJavaObject'/><eOperations name='accept' eType='http://www.eclipse.org/emf/2002/Ecore#//EJavaObject'><eParameters name='object' eType='http://www.eclipse.org/emf/2002/Ecore#//EJavaObject'/></eOperations></eClassifiers></e:EPackage>";
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
            ecore,
            "grammar example.Objects import 'urn:objects' as o Item returns o::Item: 'item';",
            r#"rcore 1; package emptyRules version "1";"#,
            &registry,
        )
        .unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-standard-object-{}.sqlite",
        std::process::id()
    ));
    let expected;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        let result = evaluate_into_with_conversions(
            &package,
            &[Source {
                uri: "objects.model".into(),
                text: "item".into(),
            }],
            EvaluationLimits::native(),
            &mut store,
            &registry,
        )
        .unwrap();
        assert!(result.diagnostics.is_empty());
        let model = storage::export(&store).unwrap();
        let id = model.elements[0].id.clone();
        assert!(model.elements[0].values["object"].is_null());
        storage::edit_with_conversions(
            &mut store,
            &package.schema,
            &[(id.clone(), "object".into(), value.clone())],
            &registry,
        )
        .unwrap();
        expected = storage::export(&store).unwrap();
        let mut stale = value.clone();
        stale["provider"]["version"] = json!("2");
        let mut wrong_domain = value.clone();
        wrong_domain["datatype"]["path"] = json!([38]);
        for bad in [
            stale,
            wrong_domain,
            json!({"javaSerialized":"opaque"}),
            json!({"red":1,"green":2,"blue":3}),
        ] {
            assert!(
                storage::edit_with_conversions(
                    &mut store,
                    &package.schema,
                    &[
                        (id.clone(), "object".into(), json!(null)),
                        (id.clone(), "object".into(), bad)
                    ],
                    &registry
                )
                .is_err()
            );
            assert_eq!(storage::export(&store).unwrap(), expected);
            assert_eq!(
                store.query(Query::Value("object", &value)).unwrap(),
                std::slice::from_ref(&id)
            );
        }
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        storage::validate_with_conversions(&store, &package.schema, &registry).unwrap();
        assert!(storage::validate(&store, &package.schema).is_err());
        assert_eq!(storage::export(&store).unwrap(), expected);
        assert_eq!(
            store.query(Query::Value("object", &value)).unwrap(),
            [expected.elements[0].id.clone()]
        );
    }
    std::fs::remove_file(path).unwrap();
}
}

// Resource content revisions are adapter-local deterministic tokens, not a
// portable edit counter. Preserve every other field in semantic comparisons.
fn semantic_model(
    mut model: intersect_model::core::model::Model,
) -> intersect_model::core::model::Model {
    for record in model.resources.values_mut() {
        record.content_revision = 0;
    }
    model
}
