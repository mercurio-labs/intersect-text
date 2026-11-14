//! Executable language-owned class initialization policies and shared core guards.
use intersect_model::core::{
    frozen::FrozenStore,
    query::QueryBudget,
    storage::{
        self, ClassInitializer, Construction, InitialValue, InitializationContext, ModelRead,
        ModelStore,
    },
};
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    conversion::ProviderId,
    initialization::ClassInitializationContract,
    schema::Schema,
};
use serde_json::{Value, json};
use std::cell::Cell;

struct Policy {
    contract: ClassInitializationContract,
    values: Vec<Value>,
    mode: &'static str,
    calls: Cell<usize>,
}
impl ClassInitializer for Policy {
    fn contract(&self) -> &ClassInitializationContract {
        &self.contract
    }
    fn plan(
        &self,
        _: &dyn ModelRead,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<Vec<InitialValue>, String> {
        self.calls.set(self.calls.get() + 1);
        budget.charge()?;
        if self.mode == "error" {
            return Err("Constructor rejected".into());
        }
        if self.mode == "panic" {
            panic!("Constructor unwind");
        }
        let mut plan = self
            .contract
            .members
            .iter()
            .zip(&self.values)
            .map(|(feature, value)| InitialValue {
                feature: feature.clone(),
                value: value.clone(),
            })
            .collect::<Vec<_>>();
        if self.mode == "omit" {
            plan.pop();
        }
        if self.mode == "duplicate" {
            plan.push(InitialValue {
                feature: self.contract.members[0].clone(),
                value: json!("duplicate"),
            });
        }
        if self.mode == "late_invalid" {
            plan.last_mut().unwrap().value = json!({"invalid":true});
        }
        Ok(plan)
    }
}
fn fixture() -> (Schema, Vec<DefinitionSource>, Policy, FrozenStore) {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="init" nsURI="urn:init">
<eClassifiers x:type="e:EClass" name="Base" abstract="true">
<eStructuralFeatures x:type="e:EAttribute" name="label" defaultValueLiteral="ecore" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EAttribute" name="readonly" changeable="false" defaultValueLiteral="ecore" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
</eClassifiers>
<eClassifiers x:type="e:EClass" name="Item" eSuperTypes="#//Base">
<eStructuralFeatures x:type="e:EAttribute" name="nullable" defaultValueLiteral="non-null" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EAttribute" name="history" unsettable="true" defaultValueLiteral="seed" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EReference" name="owner" eType="#//Owner" eOpposite="#//Owner/items"/>
</eClassifiers>
<eClassifiers x:type="e:EClass" name="Owner"><eStructuralFeatures x:type="e:EReference" name="items" upperBound="-1" eType="#//Item" eOpposite="#//Item/owner"/></eClassifiers>
</e:EPackage>"##).unwrap();
    let source = DefinitionSource::new(
        "memory:class-initialization.json",
        SourceKind::RcoreBehavior,
        r#"["constructor","constructor",null,"seed","owner"]"#,
    );
    let values = serde_json::from_str(&source.text).unwrap();
    let contract = ClassInitializationContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class: "Item".into(),
        class_identity: schema.identity("Item"),
        provider: ProviderId {
            name: "test.class-initializer".into(),
            version: "1".into(),
        },
        sources: vec![source.id.clone()],
        members: ["label", "readonly", "nullable", "history", "owner"]
            .iter()
            .map(|name| schema.effective_feature("Item", name).unwrap().identity)
            .collect(),
    };
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "owner", "Owner", "model", 0, 0)
    })
    .unwrap();
    (
        schema,
        vec![source],
        Policy {
            contract,
            values,
            mode: "valid",
            calls: Cell::new(0),
        },
        store,
    )
}
#[test]
fn constructor_overrides_are_distinct_from_ecore_defaults_and_authored_values_win() {
    let (schema, sources, provider, mut store) = fixture();
    let context = InitializationContext {
        schema: &schema,
        expected: &provider.contract,
        sources: &sources,
        provider: Some(&provider),
        conversions: None,
    };
    let authored = [("label".to_owned(), json!("authored"))];
    let receipt = context
        .create(
            &mut store,
            Construction {
                id: "a",
                resource: "model",
                authored: &authored,
            },
            &mut QueryBudget::new(1000),
        )
        .unwrap();
    assert_eq!(receipt.contract, provider.contract);
    assert_eq!(
        store.get("a").unwrap().unwrap().initialization,
        Some(receipt.clone())
    );
    let restored = FrozenStore::from_materialized(
        &schema,
        &serde_json::from_str(&serde_json::to_string(&storage::export(&store).unwrap()).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        restored.get("a").unwrap().unwrap().initialization,
        Some(receipt.clone())
    );

    assert_eq!(
        receipt.authored_members,
        vec![schema.effective_feature("Item", "label").unwrap().identity]
    );
    assert_eq!(store.value("a", "label").unwrap(), Some(json!("authored")));
    assert_eq!(
        store.value("a", "readonly").unwrap(),
        Some(json!("constructor"))
    );
    assert_eq!(store.value("a", "nullable").unwrap(), Some(Value::Null));
    assert_eq!(store.value("owner", "items").unwrap(), Some(json!(["a"])));
    assert!(storage::is_set(&store, &schema, "a", "history").unwrap());
    assert_eq!(
        schema.features("Item").unwrap()["readonly"].default,
        json!("ecore")
    );
    let before = storage::export(&store).unwrap();
    assert!(
        storage::set_with_opposite(&mut store, &schema, "a", "readonly", json!("later")).is_err()
    );
    assert!(
        context
            .create(
                &mut store,
                Construction {
                    id: "a",
                    resource: "model",
                    authored: &[]
                },
                &mut QueryBudget::new(1000)
            )
            .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), before);
    storage::unset(&mut store, &schema, "a", "label").unwrap();
    assert_eq!(store.value("a", "label").unwrap(), Some(json!("ecore")));
    storage::unset(&mut store, &schema, "a", "history").unwrap();
    assert!(!storage::is_set(&store, &schema, "a", "history").unwrap());
    storage::validate(&store, &schema).unwrap();
    let wire = serde_json::to_string(&receipt).unwrap();
    assert_eq!(
        serde_json::from_str::<intersect_model::definitions::initialization::InitializationReceipt>(&wire)
            .unwrap(),
        receipt
    );
}
#[test]
fn rejected_initializers_and_late_authored_failures_restore_objects_and_inverse_state() {
    let (schema, sources, mut provider, mut store) = fixture();
    let original = storage::export(&store).unwrap();
    for mode in ["error", "panic", "omit", "duplicate", "late_invalid"] {
        provider.mode = mode;
        let context = InitializationContext {
            schema: &schema,
            expected: &provider.contract,
            sources: &sources,
            provider: Some(&provider),
            conversions: None,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            context.create(
                &mut store,
                Construction {
                    id: "a",
                    resource: "model",
                    authored: &[],
                },
                &mut QueryBudget::new(1000),
            )
        }));
        assert!(result.is_err() || result.unwrap().is_err(), "{mode}");
        assert_eq!(storage::export(&store).unwrap(), original, "{mode}");
        assert!(!store.is_transaction_active());
    }
    provider.mode = "valid";
    let context = InitializationContext {
        schema: &schema,
        expected: &provider.contract,
        sources: &sources,
        provider: Some(&provider),
        conversions: None,
    };
    for authored in [
        vec![
            ("label".into(), json!("changed")),
            ("readonly".into(), json!("forbidden")),
        ],
        vec![
            ("label".into(), json!("one")),
            ("label".into(), json!("two")),
        ],
        vec![("missing".into(), json!("unknown"))],
    ] {
        assert!(
            context
                .create(
                    &mut store,
                    Construction {
                        id: "a",
                        resource: "model",
                        authored: &authored
                    },
                    &mut QueryBudget::new(1000)
                )
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), original);
    }
    assert!(
        storage::atomic(&mut store, |store| {
            context.create(
                store,
                Construction {
                    id: "a",
                    resource: "model",
                    authored: &[],
                },
                &mut QueryBudget::new(1000),
            )?;
            Err::<(), String>("outer rejected".into())
        })
        .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), original);
}
#[test]
fn source_provider_member_and_budget_guards_reject_before_publication() {
    let (schema, sources, provider, mut store) = fixture();
    let original = storage::export(&store).unwrap();
    let create =
        |context: InitializationContext<'_>, store: &mut FrozenStore, budget: &mut QueryBudget| {
            context.create(
                store,
                Construction {
                    id: "a",
                    resource: "model",
                    authored: &[],
                },
                budget,
            )
        };
    let context = || InitializationContext {
        schema: &schema,
        expected: &provider.contract,
        sources: &sources,
        provider: Some(&provider),
        conversions: None,
    };
    let mut missing = context();
    missing.provider = None;
    assert!(create(missing, &mut store, &mut QueryBudget::new(1000)).is_err());
    let mut missing = context();
    missing.sources = &[];
    assert!(create(missing, &mut store, &mut QueryBudget::new(1000)).is_err());
    let mut changed_sources = sources.clone();
    changed_sources[0].text.push(' ');
    let mut tampered = context();
    tampered.sources = &changed_sources;
    assert!(create(tampered, &mut store, &mut QueryBudget::new(1000)).is_err());
    for change in 0..6 {
        let mut expected = provider.contract.clone();
        match change {
            0 => expected.schema_revision.push('x'),
            1 => expected.class_identity.push('x'),
            2 => expected.provider.version.push('x'),
            3 => expected.members.push(expected.members[0].clone()),
            4 => expected.sources.push(expected.sources[0].clone()),
            _ => expected.members[0] = schema.effective_feature("Owner", "items").unwrap().identity,
        }
        let mut invalid = context();
        invalid.expected = &expected;
        assert!(create(invalid, &mut store, &mut QueryBudget::new(1000)).is_err());
    }
    assert_eq!(provider.calls.get(), 0);
    assert_eq!(storage::export(&store).unwrap(), original);
    let mut measured = QueryBudget::new(1000);
    create(context(), &mut store, &mut measured).unwrap();
    for steps in 0..measured.reads {
        let mut fresh = FrozenStore::from_materialized(&schema, &original).unwrap();
        assert!(
            create(context(), &mut fresh, &mut QueryBudget::new(steps)).is_err(),
            "{steps}"
        );
        assert_eq!(storage::export(&fresh).unwrap(), original);
    }
    let mut fresh = FrozenStore::from_materialized(&schema, &original).unwrap();
    create(context(), &mut fresh, &mut QueryBudget::new(measured.reads)).unwrap();
}

#[test]
fn class_initialization_uses_exact_conversion_context_even_for_null_values() {
    use intersect_model::conversion::RgbConverter;
    use intersect_model::definitions::compiler::import_datatype_bundle;
    use intersect_model::definitions::{
        bundle::DefinitionBundle,
        conversion::{ConversionContract, ConversionRegistry},
        resources::ResourceSet,
    };
    let text = include_str!("../../../../fixtures/ecore-unsettable/provider.ecore");
    let ecore = DefinitionSource::new("memory:ecore", SourceKind::Ecore, text);
    let (ecore, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(ecore);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![ecore.clone()],
        namespaces: vec![],
    };
    let id = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&ecore.id, "#//RGB")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &id).unwrap();
    let converter = |version: &str| {
        RgbConverter::new(ConversionContract {
            datatype: id.clone(),
            provider: ProviderId {
                name: "example.rgb".into(),
                version: version.into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![],
        })
    };
    let current = converter("1");
    let stale = converter("2");
    let mut registry = ConversionRegistry::default();
    registry.register(&datatype, &current).unwrap();
    let mut stale_registry = ConversionRegistry::default();
    stale_registry.register(&datatype, &stale).unwrap();
    let schema = intersect_model::definitions::rcore::compile_structure_with_conversions(
        &bundle, &ecore.id, None, &registry,
    )
    .unwrap()
    .schema;
    let value = serde_json::to_value(registry.parse(&id, "#010203").unwrap()).unwrap();
    let source = DefinitionSource::new(
        "memory:converted-initialization.json",
        SourceKind::RcoreBehavior,
        serde_json::to_string(&json!([value, null])).unwrap(),
    );
    let contract = ClassInitializationContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class: "Paint".into(),
        class_identity: schema.identity("Paint"),
        provider: ProviderId {
            name: "test.converted-initializer".into(),
            version: "1".into(),
        },
        sources: vec![source.id.clone()],
        members: ["color", "optional"]
            .iter()
            .map(|name| schema.effective_feature("Paint", name).unwrap().identity)
            .collect(),
    };
    let provider = Policy {
        values: serde_json::from_str(&source.text).unwrap(),
        contract,
        mode: "valid",
        calls: Cell::new(0),
    };
    for conversions in [None, Some(&stale_registry)] {
        let mut store = FrozenStore::for_schema(&schema).unwrap();
        let before = storage::export(&store).unwrap();
        assert!(
            InitializationContext {
                schema: &schema,
                expected: &provider.contract,
                sources: std::slice::from_ref(&source),
                provider: Some(&provider),
                conversions
            }
            .create(
                &mut store,
                Construction {
                    id: "a",
                    resource: "model",
                    authored: &[]
                },
                &mut QueryBudget::new(1000)
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    InitializationContext {
        schema: &schema,
        expected: &provider.contract,
        sources: std::slice::from_ref(&source),
        provider: Some(&provider),
        conversions: Some(&registry),
    }
    .create(
        &mut store,
        Construction {
            id: "a",
            resource: "model",
            authored: &[],
        },
        &mut QueryBudget::new(1000),
    )
    .unwrap();
    assert_eq!(store.value("a", "color").unwrap(), Some(value));
    assert!(storage::is_set(&store, &schema, "a", "optional").unwrap());
    assert_eq!(store.value("a", "optional").unwrap(), Some(Value::Null));
    storage::validate_with_conversions(&store, &schema, &registry).unwrap();

    use intersect_model::core::{
        api::{BasicModel, ModelEdit, TransactionalModel, VersionedModel},
        storage::InitializationBinding,
    };
    let basic = BasicModel::from_store_with_conversions(
        schema.clone(),
        FrozenStore::for_schema(&schema).unwrap(),
        &registry,
    )
    .unwrap();
    let mut model = TransactionalModel::new(basic);
    model
        .transaction_with_conversions(&registry, |edit| {
            // Omitting a per-call registry inherits the transaction's authority.
            edit.create_initialized(
                Construction {
                    id: "session",
                    resource: "model",
                    authored: &[],
                },
                InitializationBinding {
                    expected: &provider.contract,
                    sources: std::slice::from_ref(&source),
                    provider: Some(&provider),
                    conversions: None,
                },
                &mut QueryBudget::new(1000),
            )?;
            let before = storage::export(edit)?;
            assert!(
                edit.create_initialized(
                    Construction {
                        id: "stale",
                        resource: "model",
                        authored: &[]
                    },
                    InitializationBinding {
                        expected: &provider.contract,
                        sources: std::slice::from_ref(&source),
                        provider: Some(&provider),
                        conversions: Some(&stale_registry),
                    },
                    &mut QueryBudget::new(1000),
                )
                .is_err()
            );
            assert_eq!(storage::export(edit)?, before);
            Ok(())
        })
        .unwrap();
    let mut versioned =
        VersionedModel::new_with_conversions(model.into_basic(), &registry).unwrap();
    let pinned = versioned.snapshot().unwrap();
    let mut fork = versioned.fork(&pinned).unwrap();
    fork.create_initialized(
        Construction {
            id: "fork",
            resource: "model",
            authored: &[],
        },
        InitializationBinding {
            expected: &provider.contract,
            sources: std::slice::from_ref(&source),
            provider: Some(&provider),
            conversions: Some(&registry),
        },
        &mut QueryBudget::new(1000),
    )
    .unwrap();
    versioned.commit_with_conversions(fork, &registry).unwrap();
    assert!(pinned.get("fork").unwrap().is_none());
    assert!(versioned.snapshot().unwrap().get("fork").unwrap().is_some());
}

#[test]
fn initialized_construction_is_shared_by_basic_sessions_and_versioned_forks() {
    use intersect_model::core::{
        api::{BasicModel, ModelEdit, TransactionalModel, VersionedModel},
        storage::InitializationBinding,
    };
    let (schema, sources, provider, store) = fixture();
    let binding = || InitializationBinding {
        expected: &provider.contract,
        sources: &sources,
        provider: Some(&provider),
        conversions: None,
    };
    let construct = |edit: &mut dyn ModelEdit, id: &str| {
        edit.create_initialized(
            Construction {
                id,
                resource: "model",
                authored: &[],
            },
            binding(),
            &mut QueryBudget::new(1000),
        )
    };
    let mut basic = BasicModel::from_store(schema.clone(), store).unwrap();
    let receipt = construct(&mut basic, "basic").unwrap();
    assert_eq!(receipt.object, "basic");
    assert_eq!(
        basic.get("basic").unwrap().unwrap().initialization,
        Some(receipt.clone())
    );

    assert_eq!(
        basic.value("basic", "readonly").unwrap(),
        Some(json!("constructor"))
    );
    assert!(basic.is_explicitly_set("basic", "history").unwrap());
    let before = storage::export(&basic).unwrap();
    assert!(construct(&mut basic, "basic").is_err());
    assert_eq!(storage::export(&basic).unwrap(), before);
    let mut stale = provider.contract.clone();
    stale.schema_revision = "other-schema".into();
    assert!(
        basic
            .create_initialized(
                Construction {
                    id: "stale",
                    resource: "model",
                    authored: &[]
                },
                InitializationBinding {
                    expected: &stale,
                    ..binding()
                },
                &mut QueryBudget::new(1000),
            )
            .is_err()
    );
    assert_eq!(storage::export(&basic).unwrap(), before);

    let mut transactional = TransactionalModel::new(basic);
    assert!(
        transactional
            .transaction(|edit| {
                construct(edit, "rolled-back")?;
                assert_eq!(
                    edit.value("owner", "items")?,
                    Some(json!(["basic", "rolled-back"]))
                );
                Err::<(), String>("outer validation rejected".into())
            })
            .is_err()
    );
    assert_eq!(storage::export(&transactional).unwrap(), before);
    transactional
        .transaction(|edit| {
            construct(edit, "session")?;
            let current = storage::export(edit)?;
            assert!(
                edit.create_initialized(
                    Construction {
                        id: "bad",
                        resource: "model",
                        authored: &[("readonly".into(), json!("forbidden"))]
                    },
                    binding(),
                    &mut QueryBudget::new(1000),
                )
                .is_err()
            );
            assert_eq!(storage::export(edit)?, current);
            Ok(())
        })
        .unwrap();

    let mut versioned = VersionedModel::new(transactional.into_basic()).unwrap();
    let pinned = versioned.snapshot().unwrap();
    let pinned_values = storage::export(&pinned).unwrap();
    let mut fork = versioned.fork(&pinned).unwrap();
    let mut stale_fork = versioned.fork(&pinned).unwrap();
    construct(&mut stale_fork, "stale-fork").unwrap();
    construct(&mut fork, "fork").unwrap();
    let fork_values = storage::export(&fork).unwrap();
    assert!(
        fork.create_initialized(
            Construction {
                id: "failed-fork",
                resource: "model",
                authored: &[("readonly".into(), json!("forbidden"))]
            },
            binding(),
            &mut QueryBudget::new(1000),
        )
        .is_err()
    );
    assert_eq!(storage::export(&fork).unwrap(), fork_values);
    assert_eq!(storage::export(&pinned).unwrap(), pinned_values);
    versioned.commit(fork).unwrap();
    assert!(versioned.commit(stale_fork).is_err());
    let head = versioned.snapshot().unwrap();
    assert_eq!(
        head.get("fork")
            .unwrap()
            .unwrap()
            .initialization
            .unwrap()
            .object,
        "fork"
    );

    assert_eq!(
        head.value("owner", "items").unwrap(),
        Some(json!(["basic", "session", "fork"]))
    );
    assert!(head.get("stale-fork").unwrap().is_none());
    assert!(head.get("failed-fork").unwrap().is_none());
    assert_eq!(storage::export(&pinned).unwrap(), pinned_values);
}

#[test]
fn package_declarations_select_exact_initializers_and_reject_silent_fallback() {
    use intersect_model::core::api::BasicModel;
    use intersect_model::initialization::PackageInitializers;
    use intersect_text::{Source, evaluate};
    const ECORE: &str = include_str!("../../../../fixtures/ecore-settings/settings.ecore");
    const XTEXT: &str = include_str!("../../../../fixtures/ecore-settings/settings.xtext");
    let mut semantics: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-settings/settings-behavior.rules.json"
    ))
    .unwrap();
    semantics["initializers"] = json!([{
        "class":"Item", "provider":{"name":"test.package-constructor","version":"1"},
        "members":["title"]
    }]);
    let compile_semantics = |s: &Value| {
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            XTEXT,
            &s.to_string(),
        )
    };
    let package = compile_semantics(&semantics).unwrap();
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    let prepared = PackageInitializers::new(&package).unwrap();
    let provider = Policy {
        contract: prepared.contract("Item").unwrap().clone(),
        values: vec![json!("constructor")],
        mode: "valid",
        calls: Cell::new(0),
    };
    let mut model = BasicModel::new(package.schema.clone()).unwrap();
    let before = storage::export(&model).unwrap();
    assert!(
        prepared
            .create(
                &mut model,
                "Item",
                Construction {
                    id: "a",
                    resource: "model",
                    authored: &[]
                },
                None,
                None,
                &mut QueryBudget::new(1000)
            )
            .is_err()
    );
    assert_eq!(storage::export(&model).unwrap(), before);
    let receipt = prepared
        .create(
            &mut model,
            "Item",
            Construction {
                id: "a",
                resource: "model",
                authored: &[],
            },
            Some(&provider),
            None,
            &mut QueryBudget::new(1000),
        )
        .unwrap();
    assert_eq!(receipt.contract, provider.contract);
    assert_eq!(
        model.value("a", "title").unwrap(),
        Some(json!("constructor"))
    );
    let mut altered = semantics.clone();
    altered["initializers"][0]["provider"]["version"] = json!("2");
    let changed = compile_semantics(&altered).unwrap();
    let prepared_changed = PackageInitializers::new(&changed).unwrap();
    let mut changed_model = BasicModel::new(changed.schema.clone()).unwrap();
    assert!(
        prepared_changed
            .create(
                &mut changed_model,
                "Item",
                Construction {
                    id: "a",
                    resource: "model",
                    authored: &[]
                },
                Some(&provider),
                None,
                &mut QueryBudget::new(1000)
            )
            .is_err()
    );
    assert!(changed_model.get("a").unwrap().is_none());
    for invalid in [
        json!({"class":"Missing","provider":{"name":"p","version":"1"},"members":[]}),
        json!({"class":"Item","provider":{"name":"","version":"1"},"members":[]}),
        json!({"class":"Item","provider":{"name":"p","version":"1"},"members":["title","title"]}),
        json!({"class":"Item","provider":{"name":"p","version":"1"},"members":["missing"]}),
        json!({"class":"Item","provider":{"name":"p","version":"1"},"members":["aliasTitle"]}),
    ] {
        let mut bad = semantics.clone();
        bad["initializers"] = json!([invalid]);
        assert!(compile_semantics(&bad).is_err());
    }
    let mut duplicate = semantics.clone();
    duplicate["initializers"] = json!([semantics["initializers"][0], semantics["initializers"][0]]);
    assert!(compile_semantics(&duplicate).is_err());
    let mut tampered = package.clone();
    tampered.semantics.initializers.clear();
    assert!(intersect_text::compiler::bundle_from_package(&tampered).is_err());
    let evaluation = evaluate(
        &package,
        &[Source {
            uri: "model.settings".into(),
            text: "model m { item a }".into(),
        }],
    );
    assert!(
        evaluation
            .diagnostics
            .iter()
            .any(|d| d.message.contains("requires initializer provider dispatch"))
    );
}

#[test]
fn installed_constants_execute_during_text_construction_and_preserve_defaults() {
    use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
    const ECORE: &str = include_str!("../../../../fixtures/ecore-settings/settings.ecore");
    const XTEXT: &str = include_str!("../../../../fixtures/ecore-settings/settings.xtext");
    let mut semantics: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-settings/settings-behavior.rules.json"
    ))
    .unwrap();
    semantics["initializers"] = json!([{
        "class":"Item", "provider":{"name":"intersect.constant-attributes","version":"1"},
        "members":["title","tags"], "values":{"title":"constructor","tags":["initial"]}
    }]);
    let xtext = XTEXT.replace("'item' name=ID;", "'item' name=ID ('title' title=STRING)?;");
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        &xtext,
        &semantics.to_string(),
    )
    .unwrap();
    let sources = [Source {
        uri: "example.settings".into(),
        text: "model M { item a title \"authored\" item b }".into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let a = result
        .model
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&json!("a")))
        .unwrap();
    let b = result
        .model
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&json!("b")))
        .unwrap();
    assert_eq!(a.values["title"], json!("authored"));
    assert_eq!(b.values["title"], json!("constructor"));
    assert_eq!(b.values["tags"], json!(["initial"]));
    assert_eq!(b.initialization.as_ref().unwrap().object, b.id);

    assert_eq!(
        package.schema.features("Item").unwrap()["title"].default,
        json!("seed")
    );
    let mut store = FrozenStore::from_materialized(&package.schema, &result.model).unwrap();
    assert!(storage::is_set(&store, &package.schema, &b.id, "title").unwrap());
    storage::unset(&mut store, &package.schema, &b.id, "title").unwrap();
    assert_eq!(store.value(&b.id, "title").unwrap(), Some(json!("seed")));
    let mut limits = EvaluationLimits::native();
    limits.steps_per_resource = Some(10);
    assert!(
        !evaluate_with_limits(&package, &sources, limits)
            .diagnostics
            .is_empty()
    );
    let mut invalid = semantics.clone();
    invalid["initializers"][0]["values"]["tags"] = json!(["duplicate", "duplicate"]);
    let bad_package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        &xtext,
        &invalid.to_string(),
    )
    .unwrap();
    assert!(!evaluate(&bad_package, &sources).diagnostics.is_empty());
    for (field, value) in [
        ("members", json!(["peer"])),
        ("values", json!({"title":"missing tags"})),
        (
            "provider",
            json!({"name":"intersect.constant-attributes","version":"2"}),
        ),
    ] {
        let mut bad = semantics.clone();
        bad["initializers"][0][field] = value;
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
                ECORE,
                &xtext,
                &bad.to_string()
            )
            .is_err()
        );
    }
}

#[test]
fn retained_receipts_survive_edits_but_cannot_bypass_structural_validation() {
    let (schema, sources, provider, mut store) = fixture();
    let receipt = InitializationContext {
        schema: &schema,
        expected: &provider.contract,
        sources: &sources,
        provider: Some(&provider),
        conversions: None,
    }
    .create(
        &mut store,
        Construction {
            id: "a",
            resource: "model",
            authored: &[],
        },
        &mut QueryBudget::new(1000),
    )
    .unwrap();
    storage::set_with_opposite(&mut store, &schema, "a", "label", json!("later")).unwrap();
    storage::unset(&mut store, &schema, "a", "history").unwrap();
    assert_eq!(
        store.get("a").unwrap().unwrap().initialization,
        Some(receipt.clone())
    );
    let original = storage::export(&store).unwrap();
    for mode in [
        "object", "revision", "identity", "source", "member", "authored", "value",
    ] {
        let mut model = original.clone();
        let object = model.elements.iter_mut().find(|e| e.id == "a").unwrap();
        let saved = object.initialization.as_mut().unwrap();
        match mode {
            "object" => saved.object = "foreign".into(),
            "revision" => saved.contract.schema_revision = "stale".into(),
            "identity" => saved.contract.class_identity = "foreign".into(),
            "source" => saved
                .contract
                .sources
                .push(saved.contract.sources[0].clone()),
            "member" => saved
                .contract
                .members
                .push(saved.contract.members[0].clone()),
            "authored" => saved.authored_members = vec![saved.contract.members[0].clone(); 2],
            "value" => {
                object.values.insert("label".into(), json!(123));
            }
            _ => unreachable!(),
        }
        assert!(model.validate(&schema).is_err(), "{mode}");
    }
    // Legacy absence remains explicit and does not rerun an initializer.
    let mut legacy = original;
    legacy
        .elements
        .iter_mut()
        .for_each(|object| object.initialization = None);
    legacy.validate(&schema).unwrap();
}
