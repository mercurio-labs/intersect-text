use intersect_model::conversion::RgbConverter;
use intersect_model::core::{frozen::FrozenStore, model::Model};
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    conversion::{ConversionContract, ConversionRegistry, ConvertedValue, ProviderId},
    datatypes::Datatype,
    resources::DeclarationId,
};
use serde_json::json;

// Successful edits can restore the same content with a newer resource revision.
// Rollback and snapshot assertions continue comparing every field exactly.
fn assert_same_model_content(mut actual: Model, expected: &Model) {
    let mut expected = expected.clone();
    for record in actual.resources.values_mut() {
        record.content_revision = 0;
    }
    for record in expected.resources.values_mut() {
        record.content_revision = 0;
    }
    assert_eq!(actual, expected);
}

fn datatype(serializable: bool) -> Datatype {
    let source = DefinitionSource::new("memory:rgb.ecore", SourceKind::Ecore, "RGB declaration");
    Datatype::new(
        DeclarationId {
            source: source.id,
            path: vec![0],
        },
        "RGB".into(),
        serializable,
        Some("example.RGB".into()),
        None,
    )
    .unwrap()
}
fn provider(datatype: &Datatype, version: &str) -> RgbConverter {
    RgbConverter::new(ConversionContract {
        members: Vec::new(),
        declarations: Vec::new(),
        datatype: datatype.declaration().clone(),
        provider: ProviderId {
            name: "example.rgb".into(),
            version: version.into(),
        },
        serializable: datatype.serializable(),
    })
}

#[test]
fn real_provider_roundtrips_and_checks_payload_identity_and_version() {
    let datatype = datatype(true);
    let converter = provider(&datatype, "1");
    let mut registry = ConversionRegistry::default();
    assert!(registry.parse(datatype.declaration(), "#123ABC").is_err());
    assert!(registry.default_value(datatype.declaration()).is_err());
    registry.register(&datatype, &converter).unwrap();
    assert!(registry.register(&datatype, &converter).is_err());
    assert_eq!(
        registry.default_value(datatype.declaration()).unwrap(),
        None
    );
    let value = registry.parse(datatype.declaration(), "#123abc").unwrap();
    assert_eq!(
        registry.validate(&value).unwrap(),
        &json!({"red":18,"green":58,"blue":188})
    );
    assert_eq!(registry.format(&value).unwrap(), "#123ABC");
    let restored: ConvertedValue =
        serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(registry.format(&restored).unwrap(), "#123ABC");
    for text in ["123ABC", "#123", "#123ABG", "#１２３ＡＢＣ"] {
        assert!(registry.parse(datatype.declaration(), text).is_err());
    }
    for payload in [
        json!(null),
        json!({"red":256,"green":0,"blue":0}),
        json!({"red":1.0,"green":0,"blue":0}),
        json!({"red":1,"green":0,"blue":0,"extra":true}),
    ] {
        assert!(registry.bind(datatype.declaration(), payload).is_err());
    }
    let ConvertedValue::Converted {
        datatype: id,
        provider: identity,
        value: payload,
    } = value;
    let mut different = id.clone();
    different.source.sha256 = "0".repeat(64);
    assert!(
        registry
            .validate(&ConvertedValue::Converted {
                datatype: different,
                provider: identity.clone(),
                value: payload.clone()
            })
            .is_err()
    );
    let mut changed = identity;
    changed.version = "2".into();
    assert!(
        registry
            .validate(&ConvertedValue::Converted {
                datatype: id,
                provider: changed,
                value: payload
            })
            .is_err()
    );
}

#[test]
fn serializability_is_independent_of_present_value_validation() {
    let datatype = datatype(false);
    let converter = provider(&datatype, "1");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let value = registry
        .bind(datatype.declaration(), json!({"red":1,"green":2,"blue":3}))
        .unwrap();
    registry.validate(&value).unwrap();
    assert!(registry.parse(datatype.declaration(), "#010203").is_err());
    assert!(registry.format(&value).is_err());
    assert_eq!(
        registry.default_value(datatype.declaration()).unwrap(),
        None
    );
    let empty = provider(&datatype, "");
    assert!(
        ConversionRegistry::default()
            .register(&datatype, &empty)
            .is_err()
    );
}

#[test]
fn provider_failures_invalid_defaults_lossy_formatting_and_contract_changes_reject() {
    use intersect_model::definitions::conversion::ConversionProvider;
    use serde_json::Value;
    use std::cell::Cell;
    struct Faulty {
        contract: ConversionContract,
        changed: ConversionContract,
        mode: u8,
        switched: Cell<bool>,
    }
    impl ConversionProvider for Faulty {
        fn contract(&self) -> &ConversionContract {
            if self.switched.get() {
                &self.changed
            } else {
                &self.contract
            }
        }
        fn parse(&self, _: &str) -> Result<Value, String> {
            match self.mode {
                1 => Err("parse failed".into()),
                2 => Ok(json!(null)),
                7 => {
                    self.switched.set(true);
                    Ok(json!(1))
                }
                _ => Ok(json!(1)),
            }
        }
        fn format(&self, _: &Value) -> Result<String, String> {
            if self.mode == 3 {
                Err("format failed".into())
            } else {
                Ok("one".into())
            }
        }
        fn validate(&self, value: &Value) -> Result<(), String> {
            if self.mode == 4 || !value.is_u64() {
                return Err("invalid payload".into());
            }
            if self.mode == 8 {
                self.switched.set(true);
            }
            Ok(())
        }
        fn default_value(&self) -> Result<Option<Value>, String> {
            match self.mode {
                5 => Err("default failed".into()),
                6 => Ok(Some(json!(null))),
                _ => Ok(Some(json!(1))),
            }
        }
    }
    let datatype = datatype(true);
    let rgb = provider(&datatype, "1");
    for mode in 0..=8 {
        let contract = rgb.contract().clone();
        let mut changed = contract.clone();
        changed.provider.version = "changed".into();
        let converter = Faulty {
            contract,
            changed,
            mode,
            switched: Cell::new(false),
        };
        let mut registry = ConversionRegistry::default();
        registry.register(&datatype, &converter).unwrap();
        match mode {
            0 => {
                let value = registry.bind(datatype.declaration(), json!(2)).unwrap();
                assert!(registry.format(&value).unwrap_err().contains("loses value"));
            }
            1 | 2 | 7 => assert!(registry.parse(datatype.declaration(), "one").is_err()),
            3 => {
                let value = registry.bind(datatype.declaration(), json!(1)).unwrap();
                assert!(registry.format(&value).is_err());
            }
            4 | 8 => assert!(registry.bind(datatype.declaration(), json!(1)).is_err()),
            5 | 6 => assert!(registry.default_value(datatype.declaration()).is_err()),
            _ => unreachable!(),
        }
    }
}

#[test]
fn schema_conversion_context_checks_destination_and_never_overrides_scalars() {
    let mut schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
    )
    .unwrap();
    let datatype = datatype(true);
    let converter = provider(&datatype, "1");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    schema.datatypes.insert("RGB".into(), datatype.clone());
    schema
        .conversion_contracts
        .push(registry.require(&datatype).unwrap().clone());
    let mut field = schema.classes["BookCopy"].features["title"].clone();
    field.target = "RGB".into();
    field.datatype = Some(datatype.declaration().clone());
    let value =
        serde_json::to_value(registry.parse(datatype.declaration(), "#123456").unwrap()).unwrap();
    schema
        .validate_attribute_with_conversions(&field, &value, &registry)
        .unwrap();
    assert!(schema.validate_attribute(&field, &value).is_err());
    assert!(
        schema
            .validate_attribute_with_conversions(&field, &value, &ConversionRegistry::default())
            .is_err()
    );
    assert!(
        schema
            .validate_attribute_with_conversions(
                &field,
                &json!({"red":18,"green":52,"blue":86}),
                &registry
            )
            .is_err()
    );
    assert!(
        schema
            .validate_attribute_with_conversions(&field, &json!(null), &registry)
            .is_err()
    );
    let mut wrong = value.clone();
    wrong["datatype"]["path"] = json!([1]);
    assert!(
        schema
            .validate_attribute_with_conversions(&field, &wrong, &registry)
            .is_err()
    );
    let string = &schema.classes["BookCopy"].features["title"];
    assert!(
        schema
            .validate_attribute_with_conversions(string, &value, &registry)
            .is_err()
    );
    schema
        .validate_attribute_with_conversions(string, &json!("title"), &registry)
        .unwrap();
}

#[test]
fn model_conversion_edits_share_validation_uniqueness_and_atomic_rollback() {
    use intersect_model::core::model::Model;

    let mut schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
    )
    .unwrap();
    let datatype = datatype(true);
    let converter = provider(&datatype, "1");
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
    field.upper = -1;
    field.default = json!([]);
    let mut model = Model::default();
    model
        .create(&schema, "copy", "BookCopy", "api", 0, 0)
        .unwrap();
    let rgb =
        serde_json::to_value(registry.parse(datatype.declaration(), "#010203").unwrap()).unwrap();
    model
        .edit_with_conversions(
            &schema,
            &[
                ("copy".into(), "name".into(), json!("copy")),
                ("copy".into(), "title".into(), json!([rgb])),
            ],
            &registry,
        )
        .unwrap();
    model.validate_with_conversions(&schema, &registry).unwrap();
    assert!(model.validate(&schema).is_err());
    let mut empty = model.clone();
    empty.elements[0].values.insert("title".into(), json!([]));
    empty.validate_with_conversions(&schema, &registry).unwrap();
    assert!(empty.validate(&schema).is_err());
    assert!(
        empty
            .validate_with_conversions(&schema, &ConversionRegistry::default())
            .is_err()
    );
    let before = model.clone();
    let mut wrong = rgb.clone();
    wrong["provider"]["version"] = json!("2");
    for invalid in [
        json!([rgb, rgb]),
        json!([null]),
        json!([wrong]),
        json!([{"red":1,"green":2,"blue":3}]),
    ] {
        assert!(
            model
                .edit_with_conversions(
                    &schema,
                    &[
                        ("copy".into(), "name".into(), json!("must roll back")),
                        ("copy".into(), "title".into(), invalid),
                    ],
                    &registry
                )
                .is_err()
        );
        assert_eq!(model, before);
    }
    assert!(
        model
            .edit_with_conversions(
                &schema,
                &[("copy".into(), "name".into(), json!("missing provider"))],
                &ConversionRegistry::default()
            )
            .is_err()
    );
    assert_eq!(model, before);
    let restored: Model = serde_json::from_value(serde_json::to_value(&model).unwrap()).unwrap();
    restored
        .validate_with_conversions(&schema, &registry)
        .unwrap();
    assert_eq!(restored, model);
}

#[test]
fn source_compilation_materializes_custom_defaults_and_pins_provider_revision() {
    use intersect_model::core::model::Model;
    use intersect_model::definitions::compiler::{
        compile_structure_bundle_with_conversions, import_datatype_bundle,
    };
    use intersect_model::definitions::{bundle::DefinitionBundle, resources::ResourceSet};
    for operation in [
        "",
        "<eOperations name='identity' eType='#//RGB'><eParameters name='x' eType='#//RGB'/></eOperations>",
    ] {
        let text = format!(
            "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='paint' nsURI='urn:paint'><eClassifiers xsi:type='e:EDataType' name='RGB' instanceClassName='example.RGB'/><eClassifiers xsi:type='e:EClass' name='Paint'><eStructuralFeatures xsi:type='e:EAttribute' name='color' eType='#//RGB' defaultValueLiteral='#123abc'/><eStructuralFeatures xsi:type='e:EAttribute' name='optional' eType='#//RGB'/>{operation}</eClassifiers></e:EPackage>"
        );
        let rcore = intersect_text_acceptance::migrate_model_fixture(&text).unwrap();
        let source = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, rcore.clone());
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![source.clone()],
            namespaces: vec![],
        };
        let id = ResourceSet::new(&bundle)
            .unwrap()
            .resolve(&source.id, "#//RGB")
            .unwrap();
        let datatype = import_datatype_bundle(&bundle, &id).unwrap();
        let converter = provider(&datatype, "1");
        let mut registry = ConversionRegistry::default();
        registry.register(&datatype, &converter).unwrap();
        let schema =
            compile_structure_bundle_with_conversions(&bundle, &source.id, "urn:paint", &registry)
                .unwrap();
        let xtext =
            "grammar example.Paint import 'urn:paint' as p Paint returns p::Paint: 'paint';";
        let rcore_behavior = r#"rcore 1; package emptyRules version "1";"#;
        let package = intersect_text::compiler::compile_rcore_with_conversions_and_profile(
            &rcore,
            Some(xtext),
            Some(rcore_behavior),
            &registry,
            Default::default(),
        )
        .unwrap();
        let sources = [intersect_text::Source {
            uri: "paint.model".into(),
            text: "paint".into(),
        }];
        let evaluation = intersect_text::evaluate_with_conversions(
            &package,
            &sources,
            intersect_text::EvaluationLimits::native(),
            &registry,
        );
        assert!(
            evaluation.diagnostics.is_empty(),
            "{:?}",
            evaluation.diagnostics
        );
        assert_eq!(evaluation.model.elements.len(), 1);
        assert!(
            !intersect_text::evaluate(&package, &sources)
                .diagnostics
                .is_empty()
        );
        let mut store = FrozenStore::for_schema(&package.schema).unwrap();
        let persisted = intersect_text::evaluate_into_with_conversions(
            &package,
            &sources,
            intersect_text::EvaluationLimits::native(),
            &mut store,
            &registry,
        )
        .unwrap();
        assert!(persisted.diagnostics.is_empty());
        assert_eq!(
            intersect_model::core::storage::export(&store).unwrap(),
            evaluation.model
        );

        assert_eq!(
            schema.conversion_contracts,
            [registry.require(&datatype).unwrap().clone()]
        );
        assert!(
            schema.classes["Paint"].features["optional"]
                .default
                .is_null()
        );
        let value = serde_json::to_value(registry.parse(&id, "#123ABC").unwrap()).unwrap();
        assert_eq!(schema.classes["Paint"].features["color"].default, value);
        let mut model = Model::default();
        model
            .create(&schema, "paint", "Paint", "api", 0, 0)
            .unwrap();
        model.validate_with_conversions(&schema, &registry).unwrap();
        let context = intersect_model::core::transaction::TransactionContext {
            id: "custom-tx".into(),
            language_revision: "1".into(),
            base_generation: 0,
        };
        assert!(model.begin_transaction(&schema, context.clone()).is_err());
        let mut tx = model
            .begin_transaction_with_conversions(&schema, context.clone(), &registry)
            .unwrap();
        let changed = serde_json::to_value(registry.parse(&id, "#ABCDEF").unwrap()).unwrap();
        tx.stage(|staged| {
            staged.set_with_opposite_and_conversions(
                &schema,
                "paint",
                "color",
                changed.clone(),
                &registry,
            )
        })
        .unwrap();
        tx.prepare_with_conversions(&schema, |_| Ok(()), &registry)
            .unwrap();
        let mut published = model.clone();
        tx.commit(&mut published, "1", 0).unwrap();
        assert_eq!(published.elements[0].values["color"], changed);
        let mut rejected = model
            .begin_transaction_with_conversions(&schema, context, &registry)
            .unwrap();
        rejected
            .stage(|staged| {
                staged.elements[0]
                    .values
                    .insert("color".into(), json!("invalid"));
                Ok(())
            })
            .unwrap();
        assert!(
            rejected
                .prepare_with_conversions(&schema, |_| Ok(()), &registry)
                .is_err()
        );
        assert_eq!(
            rejected.status(),
            intersect_model::core::transaction::TransactionStatus::RolledBack
        );

        assert!(
            compile_structure_bundle_with_conversions(
                &bundle,
                &source.id,
                "urn:paint",
                &ConversionRegistry::default()
            )
            .is_err()
        );
        let v2 = provider(&datatype, "2");
        let mut other = ConversionRegistry::default();
        other.register(&datatype, &v2).unwrap();
        let changed =
            compile_structure_bundle_with_conversions(&bundle, &source.id, "urn:paint", &other)
                .unwrap();
        assert_ne!(changed.revision, schema.revision);
        assert!(model.validate_with_conversions(&schema, &other).is_err());
        let mut empty = model.clone();
        for value in empty.elements[0].values.values_mut() {
            *value = json!(null);
        }
        assert!(empty.validate_with_conversions(&schema, &other).is_err());
        let restored: intersect_model::definitions::schema::Schema =
            serde_json::from_value(serde_json::to_value(&schema).unwrap()).unwrap();
        model
            .validate_with_conversions(&restored, &registry)
            .unwrap();
    }
}

#[test]
fn collection_member_contracts_preserve_nested_identity_and_old_wire() {
    use intersect_model::definitions::conversion::{
        ConversionProvider, MemberContract, MemberType,
    };
    let data = datatype(true);
    let old = provider(&data, "1").contract().clone();
    let wire = serde_json::to_value(&old).unwrap();
    assert!(wire.get("members").is_none());
    assert_eq!(
        serde_json::from_value::<ConversionContract>(wire).unwrap(),
        old
    );
    let mut outer = old.clone();
    outer.members.push(MemberContract {
        nullable: false,
        value: MemberType::Converted {
            contract: Box::new(old.clone()),
        },
    });
    let wire = serde_json::to_value(&outer).unwrap();
    assert_eq!(
        serde_json::from_value::<ConversionContract>(wire.clone()).unwrap(),
        outer
    );
    let mut changed = wire.clone();
    changed["members"][0]["value"]["contract"]["provider"]["version"] = json!("2");
    assert_ne!(
        serde_json::from_value::<ConversionContract>(changed).unwrap(),
        outer
    );
    let mut malformed = wire;
    malformed["members"][0]["implicit_provider"] = json!(true);
    assert!(serde_json::from_value::<ConversionContract>(malformed).is_err());
    let bundle = intersect_model::definitions::bundle::DefinitionBundle {
        version: 1,
        sources: vec![],
        namespaces: vec![],
    };
    let result = intersect_model::conversion::with_catalog(&bundle, &[outer], |_| Ok(()));
    assert!(
        result
            .unwrap_err()
            .contains("does not consume member contracts")
    );
}

#[test]
fn provider_aware_reset_restores_defaults_and_rejects_missing_or_stale_bindings_atomically() {
    use intersect_model::core::{
        model::Model,
        storage::{self, ModelStore},
    };
    use intersect_model::definitions::compiler::import_datatype_bundle;
    use intersect_model::definitions::{bundle::DefinitionBundle, resources::ResourceSet};
    let text = include_str!("../../../../fixtures/ecore-unsettable/provider.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, text);
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let id = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&source.id, "#//RGB")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &id).unwrap();
    let converter = provider(&datatype, "1");
    let stale = provider(&datatype, "2");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let mut stale_registry = ConversionRegistry::default();
    stale_registry.register(&datatype, &stale).unwrap();
    let schema = intersect_model::definitions::rcore::compile_structure_with_conversions(
        &bundle, &source.id, None, &registry,
    )
    .unwrap()
    .schema;
    let mut model = Model::default();
    model.create(&schema, "a", "Paint", "model", 0, 0).unwrap();
    let changed =
        serde_json::to_value(registry.parse(datatype.declaration(), "#010203").unwrap()).unwrap();
    model
        .edit_with_conversions(
            &schema,
            &[
                ("a".into(), "color".into(), changed.clone()),
                ("a".into(), "optional".into(), changed.clone()),
                ("a".into(), "colors".into(), json!([changed])),
            ],
            &registry,
        )
        .unwrap();
    for name in ["color", "optional", "colors"] {
        let before = model.clone();
        assert!(model.unset(&schema, "a", name).is_err());
        assert_eq!(model, before);
        for invalid in [&ConversionRegistry::default(), &stale_registry] {
            assert!(
                model
                    .unset_with_conversions(&schema, "a", name, invalid)
                    .is_err()
            );
            assert_eq!(model, before);
        }
        let mut store = FrozenStore::from_materialized(&schema, &model).unwrap();
        let outer = store.begin().unwrap();
        storage::unset_with_conversions(&mut store, &schema, "a", name, &registry).unwrap();
        assert!(!storage::is_set(&store, &schema, "a", name).unwrap());
        store.rollback(&outer).unwrap();
        assert_eq!(storage::export(&store).unwrap(), before);
        model
            .unset_with_conversions(&schema, "a", name, &registry)
            .unwrap();
        assert!(!model.is_set(&schema, "a", name).unwrap());
        assert_eq!(
            model.elements[0].values[name],
            schema.features("Paint").unwrap()[name].default
        );
        model.validate_with_conversions(&schema, &registry).unwrap();
    }
}

#[test]
fn computed_converted_state_validates_current_registry_even_for_null_and_empty_results() {
    use intersect_model::core::{
        model::Model,
        query::{DerivedValues, QueryBudget},
        storage::{self, ModelRead},
    };
    use intersect_model::definitions::compiler::import_datatype_bundle;
    use intersect_model::definitions::{
        bundle::DefinitionBundle, resources::ResourceSet, schema::Schema,
    };
    use std::cell::Cell;
    let text = include_str!("../../../../fixtures/ecore-unsettable/provider.ecore")
        .replace("unsettable=\"true\"", "derived=\"true\"");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, &text);
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let id = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&source.id, "#//RGB")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &id).unwrap();
    let converter = provider(&datatype, "1");
    let stale = provider(&datatype, "2");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let mut stale_registry = ConversionRegistry::default();
    stale_registry.register(&datatype, &stale).unwrap();
    let schema = intersect_model::definitions::rcore::compile_structure_with_conversions(
        &bundle, &source.id, None, &registry,
    )
    .unwrap()
    .schema;
    let mut model = Model::default();
    model.create(&schema, "a", "Paint", "model", 0, 0).unwrap();
    let view = storage::ModelView::new(&model).unwrap();
    let original = model.clone();
    let changed =
        serde_json::to_value(registry.parse(datatype.declaration(), "#010203").unwrap()).unwrap();
    let stale_value = serde_json::to_value(
        stale_registry
            .parse(datatype.declaration(), "#010203")
            .unwrap(),
    )
    .unwrap();
    struct Getter<'a> {
        schema: &'a Schema,
        value: serde_json::Value,
        calls: Cell<usize>,
    }
    impl DerivedValues for Getter<'_> {
        fn schema_revision(&self) -> Option<&str> {
            Some(&self.schema.revision)
        }
        fn supports(&self, class: &str, _: &str) -> bool {
            class == "Paint"
        }
        fn read(
            &self,
            _: &dyn ModelRead,
            _: &str,
            _: &str,
            budget: &mut QueryBudget,
        ) -> Result<serde_json::Value, String> {
            budget.charge()?;
            self.calls.set(self.calls.get() + 1);
            Ok(self.value.clone())
        }
    }
    for (feature, value, expected) in [
        (
            "color",
            schema.features("Paint").unwrap()["color"].default.clone(),
            false,
        ),
        ("color", changed.clone(), true),
        ("color", json!(null), true),
        ("optional", json!(null), false),
        ("colors", json!([]), false),
        ("colors", json!([changed]), true),
    ] {
        let getter = Getter {
            schema: &schema,
            value,
            calls: Cell::new(0),
        };
        assert!(
            storage::is_set_with_derived(
                &view,
                &schema,
                "a",
                feature,
                &getter,
                &mut QueryBudget::new(100)
            )
            .is_err()
        );
        for invalid in [&ConversionRegistry::default(), &stale_registry] {
            assert!(
                storage::is_set_with_derived_and_conversions(
                    &view,
                    &schema,
                    "a",
                    feature,
                    &getter,
                    &mut QueryBudget::new(100),
                    invalid
                )
                .is_err()
            );
        }
        assert_eq!(getter.calls.get(), 0);
        assert_eq!(
            storage::is_set_with_derived_and_conversions(
                &view,
                &schema,
                "a",
                feature,
                &getter,
                &mut QueryBudget::new(100),
                &registry
            )
            .unwrap(),
            expected
        );
        assert_eq!(getter.calls.get(), 1);
    }
    for value in [json!("#010203"), stale_value] {
        let getter = Getter {
            schema: &schema,
            value,
            calls: Cell::new(0),
        };
        assert!(
            storage::is_set_with_derived_and_conversions(
                &view,
                &schema,
                "a",
                "color",
                &getter,
                &mut QueryBudget::new(100),
                &registry
            )
            .is_err()
        );
        assert_eq!(getter.calls.get(), 1);
    }
    assert_eq!(model, original);
}

#[test]
fn basic_facade_conversion_operations_validate_defaults_and_rollback_whole_batches() {
    use intersect_model::core::{
        api::{BasicModel, ModelEdit},
        storage::{self, CollectionEdit, ModelRead},
    };
    use intersect_model::definitions::compiler::import_datatype_bundle;
    use intersect_model::definitions::{bundle::DefinitionBundle, resources::ResourceSet};
    let text = include_str!("../../../../fixtures/ecore-unsettable/provider.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, text);
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let id = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&source.id, "#//RGB")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &id).unwrap();
    let converter = provider(&datatype, "1");
    let stale = provider(&datatype, "2");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let mut stale_registry = ConversionRegistry::default();
    stale_registry.register(&datatype, &stale).unwrap();
    let schema = intersect_model::definitions::rcore::compile_structure_with_conversions(
        &bundle, &source.id, None, &registry,
    )
    .unwrap()
    .schema;
    let changed =
        serde_json::to_value(registry.parse(datatype.declaration(), "#010203").unwrap()).unwrap();
    let mut model = BasicModel::new_with_conversions(schema.clone(), &registry).unwrap();
    let empty = storage::export(&model).unwrap();
    assert!(model.create("a", "Paint", "model").is_err());
    for invalid in [&ConversionRegistry::default(), &stale_registry] {
        assert!(
            model
                .create_with_conversions("a", "Paint", "model", invalid)
                .is_err()
        );
        assert_eq!(storage::export(&model).unwrap(), empty);
    }
    model
        .create_with_conversions("a", "Paint", "model", &registry)
        .unwrap();
    assert_eq!(
        model.value("a", "color").unwrap().unwrap(),
        schema.features("Paint").unwrap()["color"].default
    );
    assert!(!model.is_set("a", "color").unwrap());
    let original = storage::export(&model).unwrap();
    assert!(
        model
            .set_many_with_conversions(
                &[
                    ("a", "color", changed.clone()),
                    ("a", "optional", json!("invalid"))
                ],
                &registry
            )
            .is_err()
    );
    assert_eq!(storage::export(&model).unwrap(), original);
    for invalid in [&ConversionRegistry::default(), &stale_registry] {
        assert!(
            model
                .set_with_conversions("a", "color", changed.clone(), invalid)
                .is_err()
        );
        assert!(
            model
                .unset_with_conversions("a", "optional", invalid)
                .is_err()
        );
        assert!(
            model
                .edit_collection_with_conversions("a", "colors", CollectionEdit::Clear, invalid)
                .is_err()
        );
        assert_eq!(storage::export(&model).unwrap(), original);
    }
    model
        .set_many_with_conversions(
            &[
                ("a", "color", changed.clone()),
                ("a", "optional", changed.clone()),
            ],
            &registry,
        )
        .unwrap();
    assert!(model.is_set("a", "color").unwrap());
    assert!(
        model
            .edit_collection_with_conversions(
                "a",
                "colors",
                CollectionEdit::Add(changed.clone()),
                &registry
            )
            .unwrap()
    );
    assert!(
        !model
            .edit_collection_with_conversions(
                "a",
                "colors",
                CollectionEdit::Add(changed),
                &registry
            )
            .unwrap()
    );
    assert!(
        model
            .edit_collection_with_conversions("a", "colors", CollectionEdit::Clear, &registry)
            .unwrap()
    );
    assert!(model.is_set("a", "colors").unwrap());
    for field in ["color", "optional", "colors"] {
        model.unset_with_conversions("a", field, &registry).unwrap();
    }
    assert_same_model_content(storage::export(&model).unwrap(), &original);
    model.validate_with_conversions(&registry).unwrap();
    assert!(model.validate().is_err());
    for invalid in [&ConversionRegistry::default(), &stale_registry] {
        assert!(
            BasicModel::from_store_with_conversions(
                schema.clone(),
                FrozenStore::from_materialized(&schema, &original).unwrap(),
                invalid
            )
            .is_err()
        );
    }
    let store = FrozenStore::from_materialized(&schema, &original).unwrap();
    let restored = BasicModel::from_store_with_conversions(schema, store, &registry).unwrap();
    assert_eq!(storage::export(&restored).unwrap(), original);
}

#[test]
fn versioned_conversion_contexts_preserve_snapshots_and_validate_publication() {
    use intersect_model::core::{
        api::{BasicModel, ModelEdit, VersionedModel},
        frozen::FrozenStore,
        storage::{self, CollectionEdit},
    };
    use intersect_model::definitions::compiler::import_datatype_bundle;
    use intersect_model::definitions::{bundle::DefinitionBundle, resources::ResourceSet};
    let text = include_str!("../../../../fixtures/ecore-unsettable/provider.ecore");
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, text);
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let id = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&source.id, "#//RGB")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &id).unwrap();
    let converter = provider(&datatype, "1");
    let stale = provider(&datatype, "2");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let mut stale_registry = ConversionRegistry::default();
    stale_registry.register(&datatype, &stale).unwrap();
    let schema = intersect_model::definitions::rcore::compile_structure_with_conversions(
        &bundle, &source.id, None, &registry,
    )
    .unwrap()
    .schema;
    let changed =
        serde_json::to_value(registry.parse(datatype.declaration(), "#010203").unwrap()).unwrap();

    let mut basic = BasicModel::new_with_conversions(schema.clone(), &registry).unwrap();
    basic
        .create_with_conversions("a", "Paint", "model", &registry)
        .unwrap();
    let original = storage::export(&basic).unwrap();
    let adopt = || {
        BasicModel::from_store_with_conversions(
            schema.clone(),
            FrozenStore::from_materialized(&schema, &original).unwrap(),
            &registry,
        )
        .unwrap()
    };
    assert!(VersionedModel::new(adopt()).is_err());
    for invalid in [&ConversionRegistry::default(), &stale_registry] {
        assert!(VersionedModel::new_with_conversions(adopt(), invalid).is_err());
    }
    let mut model = VersionedModel::new_with_conversions(basic, &registry).unwrap();
    let pinned = model.snapshot().unwrap();
    let mut fork = model.fork(&pinned).unwrap();
    let sibling = model.fork(&pinned).unwrap();
    fork.set_with_conversions("a", "color", changed.clone(), &registry)
        .unwrap();
    let prior = storage::export(&fork).unwrap();
    assert!(
        fork.set_many_with_conversions(
            &[
                (
                    "a",
                    "color",
                    schema.features("Paint").unwrap()["color"].default.clone()
                ),
                ("a", "optional", json!("invalid"))
            ],
            &registry
        )
        .is_err()
    );
    assert_eq!(storage::export(&fork).unwrap(), prior);
    for invalid in [&ConversionRegistry::default(), &stale_registry] {
        assert!(
            fork.create_with_conversions("b", "Paint", "model", invalid)
                .is_err()
        );
        assert!(
            fork.unset_with_conversions("a", "optional", invalid)
                .is_err()
        );
        assert!(
            fork.edit_collection_with_conversions("a", "colors", CollectionEdit::Clear, invalid)
                .is_err()
        );
        assert_eq!(storage::export(&fork).unwrap(), prior);
        let called = std::cell::Cell::new(false);
        assert!(
            model
                .commit_with_conversions_and_validation(
                    model.fork(&pinned).unwrap(),
                    invalid,
                    |_| {
                        called.set(true);
                        Ok(())
                    }
                )
                .is_err()
        );
        assert!(!called.get());
        assert_eq!(model.snapshot().unwrap().version(), pinned.version());
    }
    assert!(model.commit(model.fork(&pinned).unwrap()).is_err());
    let mut rejected = model.fork(&pinned).unwrap();
    rejected
        .create_with_conversions("rejected", "Paint", "model", &registry)
        .unwrap();
    assert!(
        model
            .commit_with_conversions_and_validation(rejected, &registry, |_| Err(
                "domain rejected".into()
            ))
            .is_err()
    );
    assert_eq!(model.snapshot().unwrap().version(), pinned.version());
    let unwinding = model.fork(&pinned).unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = model.commit_with_conversions_and_validation(unwinding, &registry, |_| {
                panic!("domain unwind")
            });
        }))
        .is_err()
    );
    assert_eq!(model.snapshot().unwrap().version(), pinned.version());
    fork.create_with_conversions("b", "Paint", "model", &registry)
        .unwrap();
    assert!(
        fork.edit_collection_with_conversions(
            "a",
            "colors",
            CollectionEdit::Add(changed.clone()),
            &registry
        )
        .unwrap()
    );
    assert!(
        !fork
            .edit_collection_with_conversions(
                "a",
                "colors",
                CollectionEdit::Add(changed),
                &registry
            )
            .unwrap()
    );
    assert!(fork.is_set("a", "colors").unwrap());
    fork.validate_with_conversions(&registry).unwrap();
    assert!(fork.validate().is_err());
    let expected = storage::export(&fork).unwrap();
    let version = model.commit_with_conversions(fork, &registry).unwrap();
    assert_ne!(version, pinned.version());
    assert_eq!(
        storage::export(&model.snapshot().unwrap()).unwrap(),
        expected
    );
    assert_eq!(storage::export(&pinned).unwrap(), original);
    assert_eq!(storage::export(&sibling).unwrap(), original);
    let called = std::cell::Cell::new(false);
    assert!(
        model
            .commit_with_conversions_and_validation(sibling, &registry, |_| {
                called.set(true);
                Ok(())
            })
            .is_err()
    );
    assert!(!called.get());
    let other = VersionedModel::new_with_conversions(adopt(), &registry).unwrap();
    assert!(
        model
            .commit_with_conversions(other.fork(&other.snapshot().unwrap()).unwrap(), &registry)
            .is_err()
    );
    let published = model.snapshot().unwrap();
    let mut reset = model.fork(&published).unwrap();
    for field in ["color", "optional", "colors"] {
        reset.unset_with_conversions("a", field, &registry).unwrap();
        assert!(!reset.is_set("a", field).unwrap());
    }
    model.commit_with_conversions(reset, &registry).unwrap();
    assert_eq!(storage::export(&published).unwrap(), expected);
    assert_eq!(
        storage::export(&model.snapshot().unwrap())
            .unwrap()
            .elements
            .into_iter()
            .find(|element| element.id == "a")
            .unwrap(),
        original
            .elements
            .into_iter()
            .find(|element| element.id == "a")
            .unwrap()
    );
}

#[test]
fn computed_setters_check_conversion_contracts_before_planning_and_after_edits() {
    use intersect_model::core::{
        query::{DerivedValues, QueryBudget},
        storage::{
            self, ComputedEdit, ComputedSettings, ModelRead, ModelStore, SettingEdit, SettingValues,
        },
    };
    use intersect_model::definitions::compiler::import_datatype_bundle;
    use intersect_model::definitions::{bundle::DefinitionBundle, resources::ResourceSet};
    use serde_json::Value;
    struct Policy<'a> {
        revision: &'a str,
        calls: std::cell::Cell<usize>,
    }
    impl DerivedValues for Policy<'_> {
        fn schema_revision(&self) -> Option<&str> {
            Some(self.revision)
        }
        fn supports(&self, class: &str, field: &str) -> bool {
            class == "Paint" && field == "computed"
        }
        fn read(
            &self,
            store: &dyn ModelRead,
            id: &str,
            _: &str,
            budget: &mut QueryBudget,
        ) -> Result<Value, String> {
            budget.charge()?;
            store.value(id, "optional")?.ok_or("Missing backing".into())
        }
    }
    impl SettingValues for Policy<'_> {
        fn plan(
            &self,
            _: &dyn ModelRead,
            id: &str,
            _: &str,
            edit: &ComputedEdit,
            budget: &mut QueryBudget,
        ) -> Result<Vec<SettingEdit>, String> {
            budget.charge()?;
            self.calls.set(self.calls.get() + 1);
            Ok(vec![match edit {
                ComputedEdit::Set(value) => SettingEdit::Set {
                    id: id.into(),
                    feature: "optional".into(),
                    value: value.clone(),
                },
                ComputedEdit::Append(_) => {
                    return Err("Append unsupported by this test provider".into());
                }
                ComputedEdit::Unset => SettingEdit::Unset {
                    id: id.into(),
                    feature: "optional".into(),
                },
            }])
        }
    }
    let text = include_str!("../../../../fixtures/ecore-unsettable/provider.ecore");
    let extended = text.replace("  </eClassifiers>", r##"    <eStructuralFeatures xsi:type="e:EAttribute" name="computed" derived="true" eType="#//RGB"/>
  </eClassifiers>"##);
    let text = extended.as_str();
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, text);
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let id = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&source.id, "#//RGB")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &id).unwrap();
    let converter = provider(&datatype, "1");
    let stale = provider(&datatype, "2");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &converter).unwrap();
    let mut stale_registry = ConversionRegistry::default();
    stale_registry.register(&datatype, &stale).unwrap();
    let schema = intersect_model::definitions::rcore::compile_structure_with_conversions(
        &bundle, &source.id, None, &registry,
    )
    .unwrap()
    .schema;
    let changed =
        serde_json::to_value(registry.parse(datatype.declaration(), "#010203").unwrap()).unwrap();

    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "a", "Paint", "model", 0, 0)
    })
    .unwrap();
    let original = storage::export(&store).unwrap();
    let policy = Policy {
        revision: &schema.revision,
        calls: std::cell::Cell::new(0),
    };
    for conversions in [None, Some(&stale_registry)] {
        let service = ComputedSettings {
            schema: &schema,
            provider: &policy,
            conversions,
        };
        for edit in [
            ComputedEdit::Set(Value::Null),
            ComputedEdit::Unset,
            ComputedEdit::Set(changed.clone()),
        ] {
            assert!(
                service
                    .apply(
                        &mut store,
                        "a",
                        "computed",
                        edit,
                        &mut QueryBudget::new(100)
                    )
                    .is_err()
            );
            assert_eq!(storage::export(&store).unwrap(), original);
        }
    }
    assert_eq!(policy.calls.get(), 0);
    let service = ComputedSettings {
        schema: &schema,
        provider: &policy,
        conversions: Some(&registry),
    };
    assert!(
        service
            .apply(
                &mut store,
                "a",
                "computed",
                ComputedEdit::Set(json!("#010203")),
                &mut QueryBudget::new(100)
            )
            .is_err()
    );
    assert_eq!(policy.calls.get(), 0);
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(changed.clone()),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert_eq!(store.value("a", "optional").unwrap(), Some(changed));
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(Value::Null),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert!(storage::is_set(&store, &schema, "a", "optional").unwrap());
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Unset,
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert_same_model_content(storage::export(&store).unwrap(), &original);
    use intersect_model::core::{
        api::{BasicModel, ModelEdit, TransactionalModel, VersionedModel},
        storage::SettingContext,
    };
    let mut basic =
        BasicModel::from_store_with_conversions(schema.clone(), store, &registry).unwrap();
    let value =
        serde_json::to_value(registry.parse(datatype.declaration(), "#010203").unwrap()).unwrap();
    let explicit = SettingContext {
        provider: &policy,
        conversions: Some(&registry),
    };
    basic
        .edit_computed(
            "a",
            "computed",
            ComputedEdit::Set(value.clone()),
            explicit,
            &mut QueryBudget::new(100),
        )
        .unwrap();
    basic
        .edit_computed(
            "a",
            "computed",
            ComputedEdit::Unset,
            explicit,
            &mut QueryBudget::new(100),
        )
        .unwrap();
    let mut transactional = TransactionalModel::new(basic);
    let inherited = SettingContext {
        provider: &policy,
        conversions: None,
    };
    transactional
        .transaction_with_conversions(&registry, |edit| {
            edit.edit_computed(
                "a",
                "computed",
                ComputedEdit::Set(value.clone()),
                inherited,
                &mut QueryBudget::new(100),
            )?;
            let prior = storage::export(edit)?;
            assert!(
                edit.edit_computed(
                    "a",
                    "computed",
                    ComputedEdit::Unset,
                    SettingContext {
                        provider: &policy,
                        conversions: Some(&stale_registry)
                    },
                    &mut QueryBudget::new(100)
                )
                .is_err()
            );
            assert_eq!(storage::export(edit)?, prior);
            edit.edit_computed(
                "a",
                "computed",
                ComputedEdit::Unset,
                inherited,
                &mut QueryBudget::new(100),
            )
        })
        .unwrap();
    assert_same_model_content(storage::export(&transactional).unwrap(), &original);
    let mut versioned =
        VersionedModel::new_with_conversions(transactional.into_basic(), &registry).unwrap();
    let pinned = versioned.snapshot().unwrap();
    let pinned_before = storage::export(&pinned).unwrap();
    let mut fork = versioned.fork(&pinned).unwrap();
    assert!(
        fork.edit_computed(
            "a",
            "computed",
            ComputedEdit::Set(value.clone()),
            inherited,
            &mut QueryBudget::new(100)
        )
        .is_err()
    );
    fork.edit_computed(
        "a",
        "computed",
        ComputedEdit::Set(value.clone()),
        explicit,
        &mut QueryBudget::new(100),
    )
    .unwrap();
    versioned.commit_with_conversions(fork, &registry).unwrap();
    assert_eq!(
        versioned
            .snapshot()
            .unwrap()
            .value("a", "optional")
            .unwrap(),
        Some(value)
    );
    assert_eq!(storage::export(&pinned).unwrap(), pinned_before);
}

#[test]
#[cfg_attr(not(feature = "milestone-tests"), ignore = "milestone")]
fn structural_provider_errors_and_unwinds_restore_every_validation_boundary() {
    use intersect_model::core::{
        model::Model,
        query::QueryBudget,
        storage::{
            self, ContainmentDestination, DeleteRequest, DeleteSelection, IncomingReferences,
            ModelRead, Query, ReparentRequest, StructuralContext,
        },
    };
    use intersect_model::definitions::conversion::{ConversionProvider, InvocationContext};
    use serde_json::Value;
    use std::cell::Cell;
    struct Probe {
        inner: RgbConverter,
        calls: Cell<usize>,
        fail_at: Cell<usize>,
        unwind: Cell<bool>,
    }
    impl ConversionProvider for Probe {
        fn contract(&self) -> &ConversionContract {
            self.inner.contract()
        }
        fn parse(&self, text: &str) -> Result<Value, String> {
            self.inner.parse(text)
        }
        fn format(&self, value: &Value) -> Result<String, String> {
            self.inner.format(value)
        }
        fn default_value(&self) -> Result<Option<Value>, String> {
            Ok(None)
        }
        fn validate(&self, value: &Value) -> Result<(), String> {
            let call = self.calls.get() + 1;
            self.calls.set(call);
            if call == self.fail_at.get() {
                assert!(!self.unwind.get(), "injected structural provider unwind");
                return Err("injected structural provider failure".into());
            }
            self.inner.validate(value)
        }
        fn validate_observed(
            &self,
            value: &Value,
            context: &InvocationContext,
        ) -> Result<(), String> {
            context.work(3)?;
            self.validate(value)
        }
    }
    let datatype = datatype(true);
    let probe = Probe {
        inner: provider(&datatype, "1"),
        calls: Cell::new(0),
        fail_at: Cell::new(usize::MAX),
        unwind: Cell::new(false),
    };
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &probe).unwrap();
    let text=include_str!("../../../../fixtures/ecore-structure/deletion.ecore").replace("</eClassifiers>",r#"<eStructuralFeatures x:type="e:EAttribute" name="color" unsettable="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>"#);
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&text)
            .unwrap();
    schema.datatypes.insert("RGB".into(), datatype.clone());
    schema.conversion_contracts.push(probe.contract().clone());
    let color = schema
        .classes
        .get_mut("Node")
        .unwrap()
        .features
        .get_mut("color")
        .unwrap();
    color.target = "RGB".into();
    color.datatype = Some(datatype.declaration().clone());
    let payload =
        serde_json::to_value(registry.parse(datatype.declaration(), "#123456").unwrap()).unwrap();
    let mut model = Model {
        schema_revision: schema.revision.clone(),
        resources: Default::default(),
        elements: vec![],
    };
    for id in ["a", "b", "c", "d"] {
        model.create(&schema, id, "Node", "model", 10, 20).unwrap();
        model
            .set_with_opposite_and_conversions(&schema, id, "color", payload.clone(), &registry)
            .unwrap();
    }
    model
        .set_with_opposite_and_conversions(&schema, "a", "children", json!(["b"]), &registry)
        .unwrap();
    model
        .set_with_opposite_and_conversions(&schema, "b", "children", json!(["c"]), &registry)
        .unwrap();
    model
        .set_with_opposite_and_conversions(&schema, "d", "link", json!("c"), &registry)
        .unwrap();
    model.elements[0].initialization = Some(
        intersect_model::definitions::initialization::InitializationReceipt {
            object: "a".into(),
            authored_members: vec![],
            contract: intersect_model::definitions::initialization::ClassInitializationContract {
                version: 1,
                schema_revision: schema.revision.clone(),
                class: "Node".into(),
                class_identity: schema.identity("Node"),
                provider: ProviderId {
                    name: "retained-noop".into(),
                    version: "1".into(),
                },
                sources: vec![datatype.declaration().source.clone()],
                members: vec![],
            },
        },
    );
    let context = StructuralContext {
        schema: &schema,
        conversions: Some(&registry),
    };
    let run =
        |store: &mut FrozenStore, delete: bool, budget: &mut QueryBudget| -> Result<(), String> {
            if delete {
                context.delete(
                    store,
                    DeleteRequest {
                        roots: &["b"],
                        selection: DeleteSelection::Subtrees,
                        incoming: IncomingReferences::Detach,
                    },
                    budget,
                )?;
            } else {
                context.reparent(
                    store,
                    ReparentRequest {
                        object: "b",
                        destination: Some(ContainmentDestination {
                            owner: "d",
                            feature: "children",
                            position: None,
                        }),
                    },
                    budget,
                )?;
            }
            Ok(())
        };
    for delete in [false, true] {
        let mut success = FrozenStore::from_materialized(&schema, &model).unwrap();
        probe.calls.set(0);
        let mut budget = QueryBudget::new(100_000);
        run(&mut success, delete, &mut budget).unwrap();
        let calls = probe.calls.get();
        assert!(calls > 4, "must include post-edit validation");
        let expected = storage::export(&success).unwrap();
        for unwind in [false, true] {
            for fail_at in 1..=calls {
                let mut store = FrozenStore::from_materialized(&schema, &model).unwrap();
                probe.calls.set(0);
                probe.fail_at.set(fail_at);
                probe.unwind.set(unwind);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(&mut store, delete, &mut QueryBudget::new(100_000))
                }));
                if unwind {
                    assert!(result.is_err());
                } else {
                    assert!(
                        result
                            .unwrap()
                            .unwrap_err()
                            .contains("injected structural provider failure")
                    );
                }
                assert_eq!(
                    storage::export(&store).unwrap(),
                    model,
                    "delete={delete} call={fail_at} unwind={unwind}"
                );
                assert_eq!(
                    store.query(Query::Value("parent", &json!("a"))).unwrap(),
                    vec!["b"]
                );
                assert_eq!(
                    store.query(Query::Value("link", &json!("c"))).unwrap(),
                    vec!["d"]
                );
                probe.fail_at.set(usize::MAX);
                probe.unwind.set(false);
                probe.calls.set(0);
                run(&mut store, delete, &mut QueryBudget::new(100_000)).unwrap();
                assert_eq!(storage::export(&store).unwrap(), expected);
            }
        }
        for limit in 0..budget.reads {
            let mut store = FrozenStore::from_materialized(&schema, &model).unwrap();
            probe.calls.set(0);
            assert!(
                run(&mut store, delete, &mut QueryBudget::new(limit)).is_err(),
                "limit={limit}"
            );
            assert_eq!(
                storage::export(&store).unwrap(),
                model,
                "delete={delete} budget={limit}"
            );
        }
        let mut exact = FrozenStore::from_materialized(&schema, &model).unwrap();
        run(&mut exact, delete, &mut QueryBudget::new(budget.reads)).unwrap();
        assert_eq!(storage::export(&exact).unwrap(), expected);
        let mut store = FrozenStore::from_materialized(&schema, &model).unwrap();
        let missing = StructuralContext {
            schema: &schema,
            conversions: None,
        };
        assert!(
            missing
                .delete(
                    &mut store,
                    DeleteRequest {
                        roots: &["b"],
                        selection: DeleteSelection::Subtrees,
                        incoming: IncomingReferences::Detach
                    },
                    &mut QueryBudget::new(100_000)
                )
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), model);
    }
}
