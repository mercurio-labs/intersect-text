//! Language-owned setting policy exercises core's checked plan execution.
use intersect_model::core::{
    frozen::FrozenStore,
    model::Model,
    query::{DerivedValues, QueryBudget},
    storage::{
        self, ComputedEdit, ComputedSettings, ModelRead, ModelStore, SettingEdit, SettingValues,
    },
};
use intersect_model::definitions::schema::Schema;
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
use std::cell::Cell;

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

fn fixture() -> (Schema, FrozenStore) {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="p" nsURI="urn:settings">
<eClassifiers x:type="e:EClass" name="Item">
<eStructuralFeatures x:type="e:EAttribute" name="backing" unsettable="true" defaultValueLiteral="seed" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EAttribute" name="computed" derived="true" volatile="true" defaultValueLiteral="seed" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EAttribute" name="locked" derived="true" changeable="false" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
</eClassifiers></e:EPackage>"##).unwrap();
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "a", "Item", "model", 0, 0)
    })
    .unwrap();
    (schema, store)
}
struct Policy<'a> {
    revision: &'a str,
    mode: &'a str,
    calls: Cell<usize>,
}
impl DerivedValues for Policy<'_> {
    fn schema_revision(&self) -> Option<&str> {
        Some(self.revision)
    }
    fn explicit_set_state(
        &self,
        store: &dyn ModelRead,
        id: &str,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<bool, String> {
        budget.charge()?;
        let object = store.get(id)?.ok_or("Missing object")?;
        let assigned = object
            .assignment_state
            .ok_or("Missing backing state")?
            .explicitly_set("backing");
        match self.mode {
            "missing_state" => Err("No setting-state provider".into()),
            "always_set" => Ok(true),
            "never_set" => Ok(false),
            "state_error_after" if assigned => Err("Setting-state failure".into()),
            _ => Ok(assigned),
        }
    }
    fn supports(&self, class: &str, feature: &str) -> bool {
        class == "Item" && feature == "computed"
    }
    fn read(
        &self,
        store: &dyn ModelRead,
        id: &str,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<Value, String> {
        budget.charge()?;
        match self.mode {
            "bad_result" => Ok(json!(42)),
            "panic" => panic!("provider unwind"),
            _ => store.value(id, "backing")?.ok_or("Missing backing".into()),
        }
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
        self.calls.set(self.calls.get() + 1);
        budget.charge()?;
        if self.mode == "plan_error" {
            return Err("Rejected by language policy".into());
        }
        let mut edits = vec![match edit {
            ComputedEdit::Set(value) => SettingEdit::Set {
                id: id.into(),
                feature: "backing".into(),
                value: value.clone(),
            },
            ComputedEdit::Append(_) => {
                return Err("Append unsupported by this test provider".into());
            }
            ComputedEdit::Unset => SettingEdit::Unset {
                id: id.into(),
                feature: "backing".into(),
            },
        }];
        if self.mode == "late_error" {
            edits.push(SettingEdit::Set {
                id: id.into(),
                feature: "backing".into(),
                value: json!(42),
            });
        }
        if self.mode == "recursive" {
            edits.push(SettingEdit::Set {
                id: id.into(),
                feature: "computed".into(),
                value: json!("recursive"),
            });
        }
        Ok(edits)
    }
}
fn policy<'a>(schema: &'a Schema, mode: &'a str) -> Policy<'a> {
    Policy {
        revision: &schema.revision,
        mode,
        calls: Cell::new(0),
    }
}
#[test]
fn settings_use_live_backing_state_and_shared_unsettable_history() {
    let (schema, mut store) = fixture();
    let original = storage::export(&store).unwrap();
    let provider = policy(&schema, "valid");
    let service = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    };
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("updated")),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert_eq!(store.value("a", "backing").unwrap(), Some(json!("updated")));
    assert!(storage::is_set(&store, &schema, "a", "backing").unwrap());
    assert!(
        storage::is_set_with_derived(
            &store,
            &schema,
            "a",
            "computed",
            &provider,
            &mut QueryBudget::new(100)
        )
        .unwrap()
    );
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("seed")),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert!(storage::is_set(&store, &schema, "a", "backing").unwrap());
    assert!(
        !storage::is_set_with_derived(
            &store,
            &schema,
            "a",
            "computed",
            &provider,
            &mut QueryBudget::new(100)
        )
        .unwrap()
    );
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
    storage::set_with_opposite(&mut store, &schema, "a", "backing", json!("direct")).unwrap();
    assert_eq!(
        provider
            .read(&store, "a", "computed", &mut QueryBudget::new(100))
            .unwrap(),
        json!("direct")
    );
}
#[test]
fn rejected_plans_results_and_budgets_restore_the_whole_model() {
    let (schema, mut store) = fixture();
    let original = storage::export(&store).unwrap();
    for mode in [
        "plan_error",
        "late_error",
        "recursive",
        "bad_result",
        "panic",
    ] {
        let provider = policy(&schema, mode);
        let service = ComputedSettings {
            schema: &schema,
            provider: &provider,
            conversions: None,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            service.apply(
                &mut store,
                "a",
                "computed",
                ComputedEdit::Set(json!("changed")),
                &mut QueryBudget::new(100),
            )
        }));
        assert!(result.is_err() || result.unwrap().is_err(), "{mode}");
        assert_eq!(storage::export(&store).unwrap(), original, "{mode}");
        assert!(!store.is_transaction_active());
    }
    let provider = policy(&schema, "valid");
    let service = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    };
    let mut measured = QueryBudget::new(100);
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("changed")),
            &mut measured,
        )
        .unwrap();
    let steps = measured.reads;
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Unset,
            &mut QueryBudget::new(100),
        )
        .unwrap();
    let after_reset = storage::export(&store).unwrap();
    assert_same_model_content(after_reset.clone(), &original);
    for budget in 0..steps {
        assert!(
            service
                .apply(
                    &mut store,
                    "a",
                    "computed",
                    ComputedEdit::Set(json!("changed")),
                    &mut QueryBudget::new(budget)
                )
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), after_reset);
    }
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("changed")),
            &mut QueryBudget::new(steps),
        )
        .unwrap();
}
#[test]
fn invalid_requests_and_provider_bindings_reject_before_planning() {
    let (schema, mut store) = fixture();
    let original = storage::export(&store).unwrap();
    let provider = policy(&schema, "valid");
    let service = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    };
    for (id, feature, value) in [
        ("missing", "computed", json!("x")),
        ("a", "unknown", json!("x")),
        ("a", "locked", json!("x")),
        ("a", "backing", json!("x")),
        ("a", "computed", json!(42)),
    ] {
        assert!(
            service
                .apply(
                    &mut store,
                    id,
                    feature,
                    ComputedEdit::Set(value),
                    &mut QueryBudget::new(100)
                )
                .is_err()
        );
    }
    assert_eq!(provider.calls.get(), 0);
    let stale = Policy {
        revision: "stale",
        mode: "valid",
        calls: Cell::new(0),
    };
    assert!(
        ComputedSettings {
            schema: &schema,
            provider: &stale,
            conversions: None
        }
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Unset,
            &mut QueryBudget::new(100)
        )
        .is_err()
    );
    assert_eq!(stale.calls.get(), 0);
    assert_eq!(storage::export(&store).unwrap(), original);
}

#[test]
fn computed_setting_checkpoints_compose_with_outer_transactions() {
    let (schema, mut store) = fixture();
    let original = storage::export(&store).unwrap();
    let provider = policy(&schema, "valid");
    let service = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    };
    assert!(
        storage::atomic(&mut store, |store| {
            service.apply(
                store,
                "a",
                "computed",
                ComputedEdit::Set(json!("outer")),
                &mut QueryBudget::new(100),
            )?;
            let prior = storage::export(store)?;
            let invalid = policy(&schema, "late_error");
            assert!(
                ComputedSettings {
                    schema: &schema,
                    provider: &invalid,
                    conversions: None
                }
                .apply(
                    store,
                    "a",
                    "computed",
                    ComputedEdit::Set(json!("failed")),
                    &mut QueryBudget::new(100)
                )
                .is_err()
            );
            assert_eq!(storage::export(store)?, prior);
            Err::<(), String>("outer rejected".into())
        })
        .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), original);
}

#[test]
fn one_checked_consumer_runs_on_basic_sessions_and_forks() {
    use intersect_model::core::{
        api::{BasicModel, ModelEdit, TransactionalModel, VersionedModel},
        storage::SettingContext,
    };
    fn exercise(edit: &mut impl ModelEdit, schema: &Schema) -> Result<(), String> {
        let original = storage::export(edit)?;
        let provider = policy(schema, "valid");
        let context = SettingContext {
            provider: &provider,
            conversions: None,
        };
        edit.edit_computed(
            "a",
            "computed",
            ComputedEdit::Set(json!("changed")),
            context,
            &mut QueryBudget::new(100),
        )?;
        assert_eq!(edit.value("a", "backing")?, Some(json!("changed")));
        assert!(edit.is_set("a", "backing")?);
        let changed = storage::export(edit)?;
        let invalid = policy(schema, "late_error");
        assert!(
            edit.edit_computed(
                "a",
                "computed",
                ComputedEdit::Set(json!("failed")),
                SettingContext {
                    provider: &invalid,
                    conversions: None
                },
                &mut QueryBudget::new(100)
            )
            .is_err()
        );
        assert_eq!(storage::export(edit)?, changed);
        edit.edit_computed(
            "a",
            "computed",
            ComputedEdit::Unset,
            context,
            &mut QueryBudget::new(100),
        )?;
        assert_same_model_content(storage::export(edit)?, &original);
        Ok(())
    }
    let (schema, store) = fixture();
    let mut basic = BasicModel::from_store(schema.clone(), store).unwrap();
    exercise(&mut basic, &schema).unwrap();
    let original = storage::export(&basic).unwrap();
    let mut transactional = TransactionalModel::new(basic);
    transactional
        .transaction(|edit| exercise(edit, &schema))
        .unwrap();
    let before_failure = storage::export(&transactional).unwrap();
    assert_same_model_content(before_failure.clone(), &original);
    let provider = policy(&schema, "valid");
    let context = SettingContext {
        provider: &provider,
        conversions: None,
    };
    assert!(
        transactional
            .transaction(|edit| {
                edit.edit_computed(
                    "a",
                    "computed",
                    ComputedEdit::Set(json!("outer")),
                    context,
                    &mut QueryBudget::new(100),
                )?;
                Err::<(), String>("rollback".into())
            })
            .is_err()
    );
    assert_eq!(storage::export(&transactional).unwrap(), before_failure);
    let mut versioned = VersionedModel::new(transactional.into_basic()).unwrap();
    let pinned = versioned.snapshot().unwrap();
    let pinned_before = storage::export(&pinned).unwrap();
    let mut fork = versioned.fork(&pinned).unwrap();
    exercise(&mut fork, &schema).unwrap();
    fork.edit_computed(
        "a",
        "computed",
        ComputedEdit::Set(json!("published")),
        context,
        &mut QueryBudget::new(100),
    )
    .unwrap();
    versioned.commit(fork).unwrap();
    assert_eq!(
        versioned.snapshot().unwrap().value("a", "backing").unwrap(),
        Some(json!("published"))
    );
    assert_eq!(storage::export(&pinned).unwrap(), pinned_before);
}

#[test]
fn derived_unsettable_state_requires_a_consistent_live_provider_and_atomic_postconditions() {
    let (mut schema, _) = fixture();
    schema
        .classes
        .get_mut("Item")
        .unwrap()
        .features
        .get_mut("computed")
        .unwrap()
        .unsettable = true;
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "a", "Item", "model", 0, 0)
    })
    .unwrap();
    let original = storage::export(&store).unwrap();
    let provider = policy(&schema, "valid");
    let service = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    };
    let state = |store: &FrozenStore, provider: &Policy<'_>| {
        storage::is_set_with_derived(
            store,
            &schema,
            "a",
            "computed",
            provider,
            &mut QueryBudget::new(100),
        )
    };
    assert!(!state(&store, &provider).unwrap());
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("seed")),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert!(state(&store, &provider).unwrap());
    assert_eq!(
        store
            .get("a")
            .unwrap()
            .unwrap()
            .assignment_state
            .unwrap()
            .set_features()
            .collect::<Vec<_>>(),
        vec!["backing"]
    );
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Unset,
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert!(!state(&store, &provider).unwrap());
    let after_reset = storage::export(&store).unwrap();
    assert_same_model_content(after_reset.clone(), &original);
    for mode in ["missing_state", "never_set", "state_error_after"] {
        let invalid = policy(&schema, mode);
        assert!(
            ComputedSettings {
                schema: &schema,
                provider: &invalid,
                conversions: None
            }
            .apply(
                &mut store,
                "a",
                "computed",
                ComputedEdit::Set(json!("seed")),
                &mut QueryBudget::new(100)
            )
            .is_err(),
            "{mode}"
        );
        assert_eq!(storage::export(&store).unwrap(), after_reset);
    }
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("changed")),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    let changed = storage::export(&store).unwrap();
    assert!(
        state(&store, &policy(&schema, "never_set"))
            .unwrap_err()
            .contains("differs from its default")
    );
    assert!(
        ComputedSettings {
            schema: &schema,
            provider: &policy(&schema, "always_set"),
            conversions: None
        }
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Unset,
            &mut QueryBudget::new(100)
        )
        .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), changed);
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Unset,
            &mut QueryBudget::new(100),
        )
        .unwrap();
    let mut measured = QueryBudget::new(100);
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("seed")),
            &mut measured,
        )
        .unwrap();
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Unset,
            &mut QueryBudget::new(100),
        )
        .unwrap();
    let before_budget_failures = storage::export(&store).unwrap();
    assert_same_model_content(before_budget_failures.clone(), &original);
    for steps in 0..measured.reads {
        assert!(
            service
                .apply(
                    &mut store,
                    "a",
                    "computed",
                    ComputedEdit::Set(json!("seed")),
                    &mut QueryBudget::new(steps)
                )
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before_budget_failures);
    }
    service
        .apply(
            &mut store,
            "a",
            "computed",
            ComputedEdit::Set(json!("seed")),
            &mut QueryBudget::new(measured.reads),
        )
        .unwrap();
}

#[test]
fn package_stored_projections_preserve_specialized_generic_enum_domains() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        generic_types::{RuntimeType, Type},
    };
    use intersect_model::query::PackageDerived;
    use intersect_text::compiler::compile_bundle;
    let text = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="g" nsURI="urn:generic-settings">
<eClassifiers x:type="e:EEnum" name="Color"><eLiterals name="A" literal="a" value="1"/></eClassifiers>
<eClassifiers x:type="e:EEnum" name="Other"><eLiterals name="A" literal="a" value="1"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Box"><eTypeParameters name="T"/>
<eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EAttribute" name="backing" unsettable="true"><eGenericType eTypeParameter="#//Box/T"/></eStructuralFeatures>
<eStructuralFeatures x:type="e:EAttribute" name="computed" unsettable="true" derived="true" volatile="true"><eGenericType eTypeParameter="#//Box/T"/></eStructuralFeatures>
<eStructuralFeatures x:type="e:EAttribute" name="items" unsettable="true" upperBound="2"><eGenericType eTypeParameter="#//Box/T"/></eStructuralFeatures>
<eStructuralFeatures x:type="e:EAttribute" name="computedItems" unsettable="true" derived="true" upperBound="2"><eGenericType eTypeParameter="#//Box/T"/></eStructuralFeatures>
</eClassifiers>
<eClassifiers x:type="e:EClass" name="ColorBox"><eGenericSuperTypes eClassifier="#//Box"><eTypeArguments eClassifier="#//Color"/></eGenericSuperTypes></eClassifiers>
<eClassifiers x:type="e:EClass" name="OtherBox"><eGenericSuperTypes eClassifier="#//Box"><eTypeArguments eClassifier="#//Other"/></eGenericSuperTypes></eClassifiers>
</e:EPackage>"##;
    let ecore = DefinitionSource::new("memory:generic-settings.ecore", SourceKind::Ecore, text);
    let (ecore, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(ecore);
    let mut grammar = DefinitionSource::new(
        "memory:generic-settings.xtext",
        SourceKind::Xtext,
        "grammar example.Settings import \"urn:generic-settings\" as g ColorBox returns g::ColorBox: 'box' name=ID;",
    );
    grammar.dependencies.push(ecore.id.clone());
    let requested = [3, 4]
        .into_iter()
        .map(|index| {
            RuntimeType::new(Type::Classifier {
                declaration: declarations[&vec![index]].clone(),
                arguments: vec![],
                raw: false,
            })
            .unwrap()
        })
        .collect::<Vec<_>>();
    let projected = intersect_model::definitions::compiler::materialize_runtime_types(
        &DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone()],
            namespaces: vec![],
        },
        &requested,
        None,
    )
    .unwrap();
    let keys = requested
        .iter()
        .map(|ty| projected.class_key_for_type(ty).unwrap().to_owned())
        .collect::<Vec<_>>();
    let owners = keys
        .iter()
        .map(|class| {
            projected
                .feature_owner(class, "computed")
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    let mut semantics = DefinitionSource::new(
        "memory:generic-settings-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package settings version "1";
import model "urn:generic-settings" as m;
projection m.Box.computed from backing writable;
projection m.Box.computedItems from items writable;"#,
    );
    semantics.dependencies.push(ecore.id.clone());
    let input = BundleCompilation {
        ecore: ecore.id.clone(),
        namespace: "urn:generic-settings".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: [3, 4]
            .into_iter()
            .map(|index| {
                RuntimeType::new(Type::Classifier {
                    declaration: declarations[&vec![index]].clone(),
                    arguments: vec![],
                    raw: false,
                })
                .unwrap()
            })
            .collect(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, grammar, semantics],
            namespaces: vec![],
        },
    };
    let mut aliased = input.clone();
    aliased.materialized_types.truncate(1);
    let declaration = format!(
        "{}\ninitialize m.Box using \"required\" version \"1\" {{ backing; }}",
        aliased.bundle.sources[2].text
    );
    let mut source = DefinitionSource::new(
        "memory:generic-settings-behavior.rcore",
        SourceKind::RcoreBehavior,
        declaration,
    );
    source.dependencies.push(aliased.ecore.clone());
    aliased.rcore_behavior = source.id.clone();
    aliased.bundle.sources[2] = source;
    let bound = compile_bundle(aliased).unwrap();
    assert_eq!(bound.semantics.initializers[0].class, owners[0]);
    let package = compile_bundle(input).unwrap();
    let schema = &package.schema;
    let mut store = FrozenStore::for_schema(schema).unwrap();
    storage::atomic(&mut store, |store| {
        store.create_element(schema, "a", &keys[0], "model", 0, 0)
    })
    .unwrap();
    let provider = PackageDerived(&package);
    let service = ComputedSettings {
        schema,
        provider: &provider,
        conversions: None,
    };
    let valid = schema.features(&keys[0]).unwrap()["backing"]
        .default
        .clone();
    let foreign = schema.features(&keys[1]).unwrap()["backing"]
        .default
        .clone();
    assert_ne!(valid, foreign);
    for (name, value, wrong, backing) in [
        ("computed", valid.clone(), foreign.clone(), "backing"),
        ("computedItems", json!([valid]), json!([foreign]), "items"),
    ] {
        assert!(
            !storage::is_set_with_derived(
                &store,
                schema,
                "a",
                name,
                &provider,
                &mut QueryBudget::new(100)
            )
            .unwrap()
        );
        service
            .apply(
                &mut store,
                "a",
                name,
                ComputedEdit::Set(value.clone()),
                &mut QueryBudget::new(100),
            )
            .unwrap();
        assert_eq!(store.value("a", backing).unwrap(), Some(value));
        assert!(
            storage::is_set_with_derived(
                &store,
                schema,
                "a",
                name,
                &provider,
                &mut QueryBudget::new(100)
            )
            .unwrap()
        );
        let before = storage::export(&store).unwrap();
        assert!(
            service
                .apply(
                    &mut store,
                    "a",
                    name,
                    ComputedEdit::Set(wrong),
                    &mut QueryBudget::new(100)
                )
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
        service
            .apply(
                &mut store,
                "a",
                name,
                ComputedEdit::Unset,
                &mut QueryBudget::new(100),
            )
            .unwrap();
        assert!(!storage::is_set(&store, schema, "a", backing).unwrap());
    }
}
