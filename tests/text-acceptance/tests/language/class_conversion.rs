use intersect_model::class_conversion::ClassConverter;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{ConversionContract, ConversionProvider, ConversionRegistry, ProviderId},
    datatypes::Datatype,
    resources::DeclarationId,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn contract() -> ConversionContract {
    ConversionContract {
        members: Vec::new(),
        declarations: Vec::new(),
        datatype: DeclarationId {
            source: DefinitionSource::new("memory:ecore", SourceKind::Ecore, "class").id,
            path: vec![0],
        },
        provider: ProviderId {
            name: "test.class".into(),
            version: "1".into(),
        },
        serializable: true,
    }
}
fn names() -> BTreeMap<String, ProviderId> {
    ["java.lang.String", "example.Outer$Inner", "example.Δοκιμή"]
        .into_iter()
        .map(|name| {
            (
                name.into(),
                ProviderId {
                    name: "test.types".into(),
                    version: "1".into(),
                },
            )
        })
        .collect()
}
fn converter() -> ClassConverter {
    ClassConverter::new(contract(), names()).unwrap()
}

#[test]
fn primitive_named_and_array_descriptors_roundtrip_without_loading_classes() {
    let converter = converter();
    assert_eq!(converter.default_value().unwrap(), None);
    for (name, code) in [
        ("boolean", "Z"),
        ("byte", "B"),
        ("char", "C"),
        ("double", "D"),
        ("float", "F"),
        ("int", "I"),
        ("long", "J"),
        ("short", "S"),
    ] {
        for rank in [0, 1, 2, 255] {
            let text = if rank == 0 {
                name.into()
            } else {
                format!("{}{code}", "[".repeat(rank))
            };
            let value = converter.parse(&text).unwrap();
            assert_eq!(
                value,
                json!({"component":{"kind":"primitive","name":name},"dimensions":rank})
            );
            converter.validate(&value).unwrap();
            assert_eq!(converter.format(&value).unwrap(), text);
        }
    }
    assert_eq!(
        converter.format(&converter.parse("void").unwrap()).unwrap(),
        "void"
    );
    for (name, identity) in names() {
        for rank in [0, 1, 255] {
            let text = if rank == 0 {
                name.clone()
            } else {
                format!("{}L{name};", "[".repeat(rank))
            };
            let value = converter.parse(&text).unwrap();
            assert_eq!(
                value,
                json!({"component":{"kind":"named","name":name,"identity":identity},"dimensions":rank})
            );
            assert_eq!(converter.format(&value).unwrap(), text);
        }
    }
    for invalid in [
        "",
        "[V",
        "[[V",
        "[",
        "[int",
        "int[]",
        "[Ljava.lang.String",
        "[Ljava/lang/String;",
        "[L;",
        " int",
        "I",
        "missing.Type",
        "[Lmissing.Type;",
        "[Ljava.lang.String;;",
    ] {
        assert!(converter.parse(invalid).is_err(), "{invalid}");
    }
    assert!(converter.parse(&format!("{}I", "[".repeat(256))).is_err());
}

#[test]
fn wire_values_and_name_registrations_are_checked_against_exact_type_versions() {
    let converter = converter();
    let good = converter.parse("java.lang.String").unwrap();
    for (pointer, value) in [
        ("/component/identity/version", json!("2")),
        ("/component/identity/name", json!("unrelated")),
        ("/component/name", json!("unresolved")),
        ("/component/kind", json!("primitive")),
        ("/dimensions", json!(-1)),
        ("/dimensions", json!(256)),
        ("/dimensions", json!(1.0)),
        ("/dimensions", json!("1")),
    ] {
        let mut invalid = good.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(converter.validate(&invalid).is_err(), "{invalid}");
        assert!(converter.format(&invalid).is_err());
    }
    for invalid in [
        Value::Null,
        json!({}),
        json!({"component":{"kind":"primitive","name":"void"},"dimensions":1}),
        json!({"component":{"kind":"primitive","name":"unknown"},"dimensions":0}),
        json!({"component":{"kind":"primitive","name":"int","extra":0},"dimensions":0}),
        json!({"component":{"kind":"primitive","name":"int"},"dimensions":0,"extra":0}),
    ] {
        assert!(converter.validate(&invalid).is_err());
    }
    for name in [
        "",
        ".Type",
        "pkg..Type",
        "pkg.",
        "int",
        "void",
        "pkg/Type",
        "pkg.Type[]",
        "[I",
        "pkg.Type;",
        "pkg. Type",
        "pkg.\u{0}Type",
    ] {
        let entries = BTreeMap::from([(
            name.into(),
            ProviderId {
                name: "types".into(),
                version: "1".into(),
            },
        )]);
        assert!(ClassConverter::new(contract(), entries).is_err(), "{name}");
    }
    let mut entries = names();
    entries.get_mut("java.lang.String").unwrap().version = " ".into();
    assert!(ClassConverter::new(contract(), entries).is_err());
    let mut entries = names();
    entries.get_mut("java.lang.String").unwrap().version = "2".into();
    assert!(
        ClassConverter::new(contract(), entries)
            .unwrap()
            .validate(&good)
            .is_err()
    );
}

#[test]
fn registry_enforces_serialization_datatype_and_converter_identity() {
    for serializable in [true, false] {
        let mut contract = contract();
        contract.serializable = serializable;
        let datatype = Datatype::new(
            contract.datatype.clone(),
            "Class".into(),
            serializable,
            Some("java.lang.Class".into()),
            None,
        )
        .unwrap();
        let converter = ClassConverter::new(contract.clone(), names()).unwrap();
        let mut registry = ConversionRegistry::default();
        registry.register(&datatype, &converter).unwrap();
        let wire = registry
            .bind(
                datatype.declaration(),
                converter.parse("[Ljava.lang.String;").unwrap(),
            )
            .unwrap();
        assert_eq!(
            registry.parse(datatype.declaration(), "int").is_ok(),
            serializable
        );
        assert_eq!(registry.format(&wire).is_ok(), serializable);
        assert!(registry.validate_for(&datatype, &wire).is_ok());
        assert!(registry.bind(datatype.declaration(), Value::Null).is_err());
        assert_eq!(
            registry.default_value(datatype.declaration()).unwrap(),
            None
        );
        for pointer in ["/provider/version", "/datatype/source/sha256"] {
            let mut value = serde_json::to_value(&wire).unwrap();
            *value.pointer_mut(pointer).unwrap() = json!("different");
            assert!(
                registry
                    .validate(&serde_json::from_value(value).unwrap())
                    .is_err()
            );
        }
    }
}

#[test]
fn nominal_class_defaults_and_atomic_model_edits_use_the_shared_provider() {
    use intersect_model::core::model::Model;
    use intersect_model::definitions::compiler::import_datatype_bundle;
    let ecore = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="types" nsURI="urn:types">
      <eClassifiers xsi:type="e:EDataType" name="Class" instanceClassName="java.lang.Class"/>
      <eClassifiers xsi:type="e:EClass" name="Item">
        <eStructuralFeatures xsi:type="e:EAttribute" name="type" eType="#//Class" defaultValueLiteral="[Ljava.lang.String;"/>
        <eStructuralFeatures xsi:type="e:EAttribute" name="optional" eType="#//Class"/>
      </eClassifiers>
    </e:EPackage>"##;
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, ecore);
    let migration =
        intersect_model::migration::migrate_ecore(source, "memory:model.rcore").unwrap();
    let mut contract = contract();
    contract.datatype = migration
        .correspondence
        .iter()
        .find(|row| row.original.path == vec![0])
        .unwrap()
        .converted
        .clone();
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![migration.converted.unwrap()],
        namespaces: vec![],
    };
    let datatype = import_datatype_bundle(&bundle, &contract.datatype).unwrap();
    let converter = ClassConverter::new(contract, names()).unwrap();
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
            ecore,
            "grammar example.Types import \"urn:types\" as t Item returns t::Item: 'item';",
            r#"rcore 1; package emptyRules version "1";"#,
            &registry,
        )
        .unwrap();
    let evaluation = intersect_text::evaluate_with_conversions(
        &package,
        &[intersect_text::Source {
            uri: "memory:model".into(),
            text: "item".into(),
        }],
        intersect_text::EvaluationLimits::native(),
        &registry,
    );
    assert!(
        evaluation.diagnostics.is_empty(),
        "{:?}",
        evaluation.diagnostics
    );
    let mut model = evaluation.model;
    assert_eq!(model.elements[0].values["optional"], Value::Null);
    assert_eq!(
        model.elements[0].values["type"]["value"],
        converter.parse("[Ljava.lang.String;").unwrap()
    );
    let id = model.elements[0].id.clone();
    let value =
        serde_json::to_value(registry.parse(datatype.declaration(), "[[I").unwrap()).unwrap();
    model
        .edit_with_conversions(
            &package.schema,
            &[(id.clone(), "type".into(), value.clone())],
            &registry,
        )
        .unwrap();
    let before = model.clone();
    let mut invalid = value.clone();
    invalid["value"]["component"]["name"] = json!("void");
    assert!(
        model
            .edit_with_conversions(
                &package.schema,
                &[
                    (id.clone(), "optional".into(), value),
                    (id, "type".into(), invalid)
                ],
                &registry
            )
            .is_err()
    );
    assert_eq!(model, before);
    let restored: Model = serde_json::from_value(serde_json::to_value(&model).unwrap()).unwrap();
    restored
        .validate_with_conversions(&package.schema, &registry)
        .unwrap();
    assert_eq!(model, restored);
}

#[test]
fn standard_class_requirements_cover_absence_operations_collisions_and_generic_boundary() {
    use intersect_model::definitions::compiler::standard_datatype;
    fn compile_converted(
        text: &str,
        registry: &ConversionRegistry<'_>,
    ) -> Result<intersect_model::definitions::schema::Schema, String> {
        use intersect_model::definitions::bundle::{
            DefinitionBundle, DefinitionSource, SourceKind,
        };
        let source = DefinitionSource::new(
            "memory:classes.rcore",
            SourceKind::Rcore,
            intersect_text_acceptance::migrate_model_fixture(text)?,
        );
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![source.clone()],
            namespaces: vec![],
        };
        intersect_model::definitions::rcore::compile_structure_with_conversions(
            &bundle, &source.id, None, registry,
        )
        .map(|structure| structure.schema)
        .map_err(|errors| format!("{errors:?}"))
    }
    let datatype = standard_datatype("EJavaClass").unwrap();
    let mut contract = contract();
    contract.datatype = datatype.declaration().clone();
    let converter = ClassConverter::portable_core(contract).unwrap();
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let wrap = |body: &str| {
        format!(
            "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='test' nsURI='urn:test'>{body}</e:EPackage>"
        )
    };
    for feature in [
        "<eStructuralFeatures xsi:type='e:EAttribute' name='type' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EJavaClass'/>",
        "<eOperations name='type' eType='http://www.eclipse.org/emf/2002/Ecore#//EJavaClass'/>",
    ] {
        let body =
            format!("<eClassifiers xsi:type='e:EClass' name='Item'>{feature}</eClassifiers>");
        compile_converted(&wrap(&body), &registry).unwrap();
        assert!(compile_converted(&wrap(&body), &ConversionRegistry::default()).is_err());
        let collision =
            format!("{body}<eClassifiers xsi:type='e:EClass' name='__standard_EJavaClass'/>");
        assert!(
            compile_converted(&wrap(&collision), &registry)
                .unwrap_err()
                .to_string()
                .contains("collision")
        );
    }
    let generic = wrap(
        "<eClassifiers xsi:type='e:EDataType' name='Class' instanceClassName='java.lang.Class'><eTypeParameters name='T'/></eClassifiers>",
    );
    assert!(
        compile_converted(&generic, &registry)
            .unwrap_err()
            .to_string()
            .contains("generic")
    );
    let wrong_kind = wrap(
        "<eClassifiers xsi:type='e:EClass' name='Item'><eStructuralFeatures xsi:type='e:EReference' name='wrong' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EJavaClass'/></eClassifiers>",
    );
    assert!(compile_converted(&wrong_kind, &registry).is_err());
}
