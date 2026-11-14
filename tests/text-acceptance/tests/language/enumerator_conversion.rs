use intersect_model::definitions::compiler::{import_enum_bundle, standard_datatype};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{ConversionContract, ConversionProvider, ConversionRegistry, ProviderId},
    enums::{EnumValue, Enumeration},
};
use intersect_model::enumerator_conversion::EnumeratorConverter;
use serde_json::json;

fn model() -> (DefinitionBundle, Enumeration, Enumeration) {
    let source = DefinitionSource::new(
        "memory:ecore",
        SourceKind::Ecore,
        r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="values" nsURI="urn:values">
      <eClassifiers xsi:type="e:EEnum" name="Mode"><eLiterals name="First" value="7" literal="first"/><eLiterals name="Alias" value="7" literal="alias"/></eClassifiers>
      <eClassifiers xsi:type="e:EEnum" name="Other"><eLiterals name="First" value="7" literal="first"/></eClassifiers>
      <eClassifiers xsi:type="e:EClass" name="Item">
        <eStructuralFeatures xsi:type="e:EAttribute" name="value" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EEnumerator"/>
        <eStructuralFeatures xsi:type="e:EAttribute" name="values" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EEnumerator" upperBound="-1"/>
        <eOperations name="identity" eType="http://www.eclipse.org/emf/2002/Ecore#//EEnumerator"><eParameters name="value" eType="http://www.eclipse.org/emf/2002/Ecore#//EEnumerator"/></eOperations>
      </eClassifiers>
    </e:EPackage>"##,
    );
    let (mut source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    // Match the canonical string compiler source identity for exact enum values.
    source = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, source.text);
    let declaration = |path| {
        let mut id = declarations[&vec![path]].clone();
        id.source = source.id.clone();
        id
    };
    let first_id = declaration(0);
    let second_id = declaration(1);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let first = import_enum_bundle(&bundle, &first_id).unwrap();
    let second = import_enum_bundle(&bundle, &second_id).unwrap();
    (bundle, first, second)
}
fn contract() -> ConversionContract {
    let datatype = standard_datatype("EEnumerator").unwrap();
    assert_eq!(
        datatype.instance_class_name(),
        Some("org.eclipse.emf.common.util.Enumerator")
    );
    assert!(!datatype.serializable());
    ConversionContract {
        members: Vec::new(),
        declarations: Vec::new(),
        datatype: datatype.declaration().clone(),
        provider: ProviderId {
            name: "test.enumerator".into(),
            version: "1".into(),
        },
        serializable: false,
    }
}

#[test]
fn adapter_retains_aliases_and_checks_enum_literal_ownership_and_revision() {
    let (_, first, second) = model();
    let converter = EnumeratorConverter::new(contract(), [first.clone(), second.clone()]).unwrap();
    let a = first.parse("first").unwrap();
    let alias = first.parse("alias").unwrap();
    assert_ne!(a, alias);
    assert_eq!(
        converter.literal(&a).unwrap().value,
        converter.literal(&alias).unwrap().value
    );
    assert_ne!(
        converter.literal(&a).unwrap().declaration,
        converter.literal(&alias).unwrap().declaration
    );
    for value in [&a, &alias, &second.parse("first").unwrap()] {
        converter
            .validate(&serde_json::to_value(value).unwrap())
            .unwrap();
    }
    let wrong = EnumValue::Enum {
        enumeration: first.declaration().clone(),
        literal: second.literals()[0].declaration.clone(),
    };
    assert!(converter.literal(&wrong).is_err());
    let good = serde_json::to_value(&alias).unwrap();
    for pointer in ["/enumeration/source/sha256", "/literal/source/sha256"] {
        let mut stale = good.clone();
        *stale.pointer_mut(pointer).unwrap() = json!("changed");
        assert!(converter.validate(&stale).is_err());
    }
    for invalid in [
        json!(null),
        json!(7),
        json!("alias"),
        json!({"name":"Alias","value":7}),
        json!({"kind":"enum","enumeration":first.declaration(),"literal":first.declaration()}),
    ] {
        assert!(converter.validate(&invalid).is_err());
    }
    let mut extra = good.clone();
    extra["extra"] = json!(true);
    assert!(converter.validate(&extra).is_err());
    assert!(EnumeratorConverter::new(contract(), [first.clone(), first.clone()]).is_err());
    assert!(
        EnumeratorConverter::new(contract(), [])
            .unwrap()
            .validate(&good)
            .is_err()
    );
    let empty =
        Enumeration::new(first.declaration().clone(), "Empty".into(), false, vec![]).unwrap();
    assert!(
        EnumeratorConverter::new(contract(), [empty])
            .unwrap()
            .validate(&good)
            .is_err()
    );
    let mut changed = first.declaration().clone();
    changed.source.sha256 = "changed".into();
    let same_name = Enumeration::new(changed, first.name().into(), true, vec![]).unwrap();
    assert!(
        EnumeratorConverter::new(contract(), [same_name])
            .unwrap()
            .validate(&good)
            .is_err()
    );
}

#[test]
fn nonserializable_native_values_do_not_gain_lexical_conversion() {
    let (_, first, _) = model();
    let converter = EnumeratorConverter::new(contract(), [first.clone()]).unwrap();
    let datatype = standard_datatype("EEnumerator").unwrap();
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let payload = serde_json::to_value(first.parse("alias").unwrap()).unwrap();
    let value = registry
        .bind(datatype.declaration(), payload.clone())
        .unwrap();
    assert_eq!(registry.validate_for(&datatype, &value).unwrap(), &payload);
    assert!(registry.parse(datatype.declaration(), "alias").is_err());
    assert!(registry.format(&value).is_err());
    assert!(converter.parse("alias").is_err());
    assert!(converter.format(&payload).is_err());
    assert_eq!(
        registry.default_value(datatype.declaration()).unwrap(),
        None
    );
    let nonserializable_enum = Enumeration::new(
        first.declaration().clone(),
        first.name().into(),
        false,
        first.literals().to_vec(),
    )
    .unwrap();
    EnumeratorConverter::new(contract(), [nonserializable_enum])
        .unwrap()
        .validate(&payload)
        .unwrap();
    let mut invalid = contract();
    invalid.serializable = true;
    assert!(EnumeratorConverter::new(invalid, [first]).is_err());
    let mut stale = serde_json::to_value(value).unwrap();
    stale["provider"]["version"] = json!("2");
    assert!(
        registry
            .validate(&serde_json::from_value(stale).unwrap())
            .is_err()
    );
}

#[test]
fn source_compilation_model_lists_and_rcore_behavior_reads_use_exact_enum_adapters() {
    let (mut bundle, first, second) = model();
    let datatype = standard_datatype("EEnumerator").unwrap();
    let converter = EnumeratorConverter::new(contract(), [first.clone(), second]).unwrap();
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let rcore = bundle.sources[0].text.clone();
    let xtext = "grammar example.Values import 'urn:values' as v Item returns v::Item: 'item';";
    let rcore_behavior = r#"rcore 1; package emptyRules version "1";"#;
    let package = intersect_text::compiler::compile_rcore_with_conversions_and_profile(
        &rcore,
        Some(xtext),
        Some(rcore_behavior),
        &registry,
        Default::default(),
    )
    .unwrap();
    assert!(
        intersect_text::compiler::compile_rcore_with_conversions_and_profile(
            &rcore,
            Some(xtext),
            Some(rcore_behavior),
            &ConversionRegistry::default(),
            Default::default()
        )
        .is_err()
    );
    let sources = [intersect_text::Source {
        uri: "values.model".into(),
        text: "item".into(),
    }];
    let evaluation = intersect_text::evaluate_with_conversions(
        &package,
        &sources,
        intersect_text::EvaluationLimits::native(),
        &registry,
    );
    assert!(evaluation.diagnostics.is_empty());
    let mut model = evaluation.model;
    assert!(model.elements[0].values["value"].is_null());
    assert_eq!(model.elements[0].values["values"], json!([]));
    let id = model.elements[0].id.clone();
    let bind = |name| {
        serde_json::to_value(
            registry
                .bind(
                    datatype.declaration(),
                    serde_json::to_value(first.parse(name).unwrap()).unwrap(),
                )
                .unwrap(),
        )
        .unwrap()
    };
    let a = bind("first");
    let alias = bind("alias");
    model
        .edit_with_conversions(
            &package.schema,
            &[(id.clone(), "values".into(), json!([a, alias]))],
            &registry,
        )
        .unwrap();
    let before = model.clone();
    assert!(
        model
            .edit_with_conversions(
                &package.schema,
                &[
                    (id.clone(), "value".into(), a.clone()),
                    (id.clone(), "values".into(), json!([a, a]))
                ],
                &registry
            )
            .is_err()
    );
    assert_eq!(model, before);
    let restored: intersect_model::core::model::Model =
        serde_json::from_value(serde_json::to_value(&model).unwrap()).unwrap();
    restored
        .validate_with_conversions(&package.schema, &registry)
        .unwrap();
    let mut semantic = DefinitionSource::new(
        "values-behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package checks version \"1\"; import model \"urn:values\" as v; fn same(a: v.Item, b: v.Item) -> Bool = a.value == b.value;",
    );
    semantic.dependencies.push(bundle.sources[0].id.clone());
    let entry = semantic.id.clone();
    bundle.sources.push(semantic);
    intersect_model::definitions::rcore_behavior::checking::check_with_conversions(
        &bundle, &entry, &registry,
    )
    .unwrap();
    assert!(
        intersect_model::definitions::rcore_behavior::checking::check(&bundle, &entry).is_err()
    );
    let mut selected = contract();
    selected.provider.name = "example.rgb".into();
    assert!(intersect_model::conversion::with_catalog(&bundle, &[selected], |_| Ok(())).is_err());
}
