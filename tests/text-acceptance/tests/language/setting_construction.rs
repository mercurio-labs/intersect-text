//! Authored language policy over the shared setting transaction; no KerML claim.
use intersect_model::core::{
    frozen::FrozenStore,
    model::Model,
    query::{DerivedValues, QueryBudget},
    storage::{
        self, ComputedEdit, ComputedSettings, ModelRead, ModelStore, SettingEdit, SettingValues,
    },
};
use intersect_model::definitions::schema::Schema;
use serde_json::{Value, json};
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
    fixture_with_bounds("")
}
fn fixture_with_bounds(bounds: &str) -> (Schema, FrozenStore) {
    let schema=intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="construction" nsURI="urn:test:setting-construction">
 <eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EAttribute" name="computed" derived="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/><eStructuralFeatures x:type="e:EReference" name="slots" upperBound="-1" containment="true" eType="#//Slot"/><eStructuralFeatures x:type="e:EReference" name="link" eType="#//Value"/></eClassifiers>
 <eClassifiers x:type="e:EClass" name="Slot"><eStructuralFeatures x:type="e:EReference" name="value" containment="true" eType="#//Value"/></eClassifiers>
 <eClassifiers x:type="e:EClass" name="Value"><eStructuralFeatures x:type="e:EAttribute" name="text" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
 </e:EPackage>"##.replace("name=\"computed\" derived", &format!("name=\"computed\" {bounds} derived"))).unwrap();
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(&schema, "root", "Root", "model", 0, 0)
    })
    .unwrap();
    (schema, store)
}
struct Policy<'a> {
    revision: &'a str,
    mode: &'a str,
}
impl DerivedValues for Policy<'_> {
    fn schema_revision(&self) -> Option<&str> {
        Some(self.revision)
    }
    fn supports(&self, class: &str, feature: &str) -> bool {
        class == "Root" && feature == "computed"
    }
    fn read(
        &self,
        store: &dyn ModelRead,
        _: &str,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<Value, String> {
        budget.charge()?;
        if matches!(self.mode, "invalid_readback" | "attach_invalid_readback") {
            return Ok(json!(17));
        }
        if self.mode == "write_only" {
            return Ok(json!([]));
        }
        store
            .value("value", "text")?
            .ok_or("Missing constructed value".into())
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
        let (ComputedEdit::Set(value) | ComputedEdit::Append(value)) = edit else {
            return Err("Unset unsupported by this policy".into());
        };
        if self.mode.starts_with("attach_") {
            return Ok(vec![
                SettingEdit::Set {
                    id: "value".into(),
                    feature: "text".into(),
                    value: value.clone(),
                },
                SettingEdit::Set {
                    id: "slot".into(),
                    feature: "value".into(),
                    value: json!("value"),
                },
                SettingEdit::Set {
                    id: id.into(),
                    feature: "slots".into(),
                    value: json!(["slot"]),
                },
            ]);
        }
        let mut plan = vec![
            SettingEdit::Create {
                id: if self.mode == "duplicate" { id } else { "slot" }.into(),
                class: if self.mode == "class" {
                    "Unknown"
                } else {
                    "Slot"
                }
                .into(),
                resource: "model".into(),
                start: 2,
                end: 7,
            },
            SettingEdit::Create {
                id: "value".into(),
                class: "Value".into(),
                resource: "model".into(),
                start: 3,
                end: 6,
            },
            SettingEdit::Set {
                id: "value".into(),
                feature: "text".into(),
                value: value.clone(),
            },
            SettingEdit::Set {
                id: "slot".into(),
                feature: "value".into(),
                value: json!("value"),
            },
            SettingEdit::Set {
                id: id.into(),
                feature: "slots".into(),
                value: json!(["slot"]),
            },
        ];
        if self.mode == "late" {
            plan.push(SettingEdit::Set {
                id: "slot".into(),
                feature: "missing".into(),
                value: json!("failure"),
            });
        }
        if self.mode == "containment" {
            plan.extend([
                SettingEdit::Create {
                    id: "second".into(),
                    class: "Slot".into(),
                    resource: "model".into(),
                    start: 2,
                    end: 7,
                },
                SettingEdit::Set {
                    id: "second".into(),
                    feature: "value".into(),
                    value: json!("value"),
                },
                SettingEdit::Set {
                    id: id.into(),
                    feature: "slots".into(),
                    value: json!(["slot", "second"]),
                },
            ]);
        }
        Ok(plan)
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn construction_plan_matches_direct_checked_graph_and_preserves_spans() {
    let (schema, mut store) = fixture();
    let provider = Policy {
        revision: &schema.revision,
        mode: "valid",
    };
    ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    }
    .apply(
        &mut store,
        "root",
        "computed",
        ComputedEdit::Set(json!("hello")),
        &mut QueryBudget::new(100),
    )
    .unwrap();
    let (_, mut direct) = fixture();
    storage::atomic(&mut direct, |s| {
        s.create_element(&schema, "slot", "Slot", "model", 2, 7)?;
        s.create_element(&schema, "value", "Value", "model", 3, 6)?;
        storage::set_with_opposite(s, &schema, "value", "text", json!("hello"))?;
        storage::set_with_opposite(s, &schema, "slot", "value", json!("value"))?;
        storage::set_with_opposite(s, &schema, "root", "slots", json!(["slot"]))?;
        storage::validate_draft(s, &schema)
    })
    .unwrap();
    assert_same_model_content(
        storage::export(&store).unwrap(),
        &storage::export(&direct).unwrap(),
    );
    assert_eq!(store.get("value").unwrap().unwrap().start, 3);
    assert_eq!(store.get("slot").unwrap().unwrap().end, 7);
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn construction_failure_and_every_work_boundary_roll_back_all_objects() {
    let (schema, mut store) = fixture();
    let before = storage::export(&store).unwrap();
    for mode in [
        "duplicate",
        "class",
        "late",
        "containment",
        "invalid_readback",
    ] {
        let provider = Policy {
            revision: &schema.revision,
            mode,
        };
        assert!(
            ComputedSettings {
                schema: &schema,
                provider: &provider,
                conversions: None
            }
            .apply(
                &mut store,
                "root",
                "computed",
                ComputedEdit::Set(json!("hello")),
                &mut QueryBudget::new(100)
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    let provider = Policy {
        revision: &schema.revision,
        mode: "valid",
    };
    let service = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    };
    let mut budget = QueryBudget::new(100);
    let outer = store.begin().unwrap();
    service
        .apply(
            &mut store,
            "root",
            "computed",
            ComputedEdit::Set(json!("hello")),
            &mut budget,
        )
        .unwrap();
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), before);
    for limit in 0..budget.reads {
        let error = service
            .apply(
                &mut store,
                "root",
                "computed",
                ComputedEdit::Set(json!("hello")),
                &mut QueryBudget::new(limit),
            )
            .unwrap_err();
        assert!(error.contains("budget"), "{limit}: {error}");
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    service
        .apply(
            &mut store,
            "root",
            "computed",
            ComputedEdit::Set(json!("hello")),
            &mut QueryBudget::new(budget.reads),
        )
        .unwrap();
    assert_eq!(store.value("value", "text").unwrap(), Some(json!("hello")));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn append_uses_existing_package_projection_and_checks_final_collection() {
    let package=intersect_text_acceptance::migration_fixture::compile_migrated_fixture(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="append" nsURI="urn:test:append">
 <eClassifiers x:type="e:EClass" name="Root">
 <eStructuralFeatures x:type="e:EAttribute" name="backing" lowerBound="2" upperBound="3" unsettable="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
 <eStructuralFeatures x:type="e:EAttribute" name="computed" derived="true" lowerBound="2" upperBound="3" unsettable="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
 </eClassifiers></e:EPackage>"##,
 "grammar append.Test import \"urn:test:append\" as a Root returns a::Root: 'root';",
 r#"{"version":1,"scope":"unique_names","derived":[{"class":"Root","feature":"computed","expression":{"op":"stored","feature":"backing","writable":true}}],"validations":[]}"#).unwrap();
    let schema = &package.schema;
    let mut store = FrozenStore::for_schema(schema).unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(schema, "root", "Root", "r", 0, 0)?;
        storage::set_with_opposite(s, schema, "root", "backing", json!(["a", "b"]))
    })
    .unwrap();
    let provider = intersect_model::query::PackageDerived(&package);
    let service = ComputedSettings {
        schema,
        provider: &provider,
        conversions: None,
    };
    // Incoming singleton does not need to satisfy the collection lower bound of two.
    service
        .apply(
            &mut store,
            "root",
            "computed",
            ComputedEdit::Append(json!("c")),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert_eq!(
        store.value("root", "backing").unwrap(),
        Some(json!(["a", "b", "c"]))
    );
    assert!(storage::is_explicitly_set(&store, schema, "root", "backing").unwrap());
    let before = storage::export(&store).unwrap();
    for value in [json!("d"), json!("c"), json!(17), json!(["nested"])] {
        assert!(
            service
                .apply(
                    &mut store,
                    "root",
                    "computed",
                    ComputedEdit::Append(value),
                    &mut QueryBudget::new(100)
                )
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    let (schema, mut scalar) = fixture();
    let provider = Policy {
        revision: &schema.revision,
        mode: "valid",
    };
    let error = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    }
    .apply(
        &mut scalar,
        "root",
        "computed",
        ComputedEdit::Append(json!("x")),
        &mut QueryBudget::new(100),
    )
    .unwrap_err();
    assert!(error.contains("many-valued"));
    assert!(scalar.get("slot").unwrap().is_none());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn write_only_append_constructs_a_graph_without_storing_the_computed_feature() {
    let (schema, mut store) = fixture_with_bounds("upperBound=\"-1\"");
    let provider = Policy {
        revision: &schema.revision,
        mode: "write_only",
    };
    ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    }
    .apply(
        &mut store,
        "root",
        "computed",
        ComputedEdit::Append(json!("operand")),
        &mut QueryBudget::new(100),
    )
    .unwrap();
    assert_eq!(
        store.value("value", "text").unwrap(),
        Some(json!("operand"))
    );
    assert_eq!(store.value("root", "slots").unwrap(), Some(json!(["slot"])));
    assert_eq!(
        provider
            .read(&store, "root", "computed", &mut QueryBudget::new(10))
            .unwrap(),
        json!([])
    );
    assert!(
        !store
            .get("root")
            .unwrap()
            .unwrap()
            .values
            .contains_key("computed")
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn construction_is_shared_by_basic_transactions_and_versioned_forks() {
    use intersect_model::core::{
        api::{BasicModel, ModelEdit, TransactionalModel, VersionedModel},
        storage::SettingContext,
    };
    fn construct(edit: &mut impl ModelEdit, schema: &Schema, mode: &str) -> Result<(), String> {
        let provider = Policy {
            revision: &schema.revision,
            mode,
        };
        edit.edit_computed(
            "root",
            "computed",
            ComputedEdit::Append(json!("operand")),
            SettingContext {
                provider: &provider,
                conversions: None,
            },
            &mut QueryBudget::new(100),
        )
    }
    let (schema, store) = fixture_with_bounds("upperBound=\"-1\"");
    let original = storage::export(&store).unwrap();
    let mut basic = BasicModel::from_store(schema.clone(), store).unwrap();
    construct(&mut basic, &schema, "write_only").unwrap();
    let expected = storage::export(&basic).unwrap();
    assert_eq!(
        basic.value("value", "text").unwrap(),
        Some(json!("operand"))
    );

    let (_, store) = fixture_with_bounds("upperBound=\"-1\"");
    let mut transactional =
        TransactionalModel::new(BasicModel::from_store(schema.clone(), store).unwrap());
    assert!(
        transactional
            .transaction(|edit| {
                construct(edit, &schema, "write_only")?;
                assert_same_model_content(storage::export(edit)?, &expected);
                Err::<(), String>("abort outer transaction".into())
            })
            .is_err()
    );
    assert_eq!(storage::export(&transactional).unwrap(), original);
    assert!(
        transactional
            .transaction(|edit| construct(edit, &schema, "late"))
            .is_err()
    );
    assert_eq!(storage::export(&transactional).unwrap(), original);
    transactional
        .transaction(|edit| construct(edit, &schema, "write_only"))
        .unwrap();
    assert_same_model_content(storage::export(&transactional).unwrap(), &expected);

    let (_, store) = fixture_with_bounds("upperBound=\"-1\"");
    let mut versioned =
        VersionedModel::new(BasicModel::from_store(schema.clone(), store).unwrap()).unwrap();
    let pinned = versioned.snapshot().unwrap();
    let mut fork = versioned.fork(&pinned).unwrap();
    assert!(construct(&mut fork, &schema, "late").is_err());
    assert_eq!(storage::export(&fork).unwrap(), original);
    construct(&mut fork, &schema, "write_only").unwrap();
    assert_same_model_content(storage::export(&fork).unwrap(), &expected);
    assert_eq!(storage::export(&pinned).unwrap(), original);
    versioned.commit(fork).unwrap();
    assert_same_model_content(
        storage::export(&versioned.snapshot().unwrap()).unwrap(),
        &expected,
    );
    assert_eq!(storage::export(&pinned).unwrap(), original);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn construction_during_linking_preserves_strict_reference_boundaries() {
    use intersect_model::core::storage::DraftReferencePolicy;
    let (schema, mut store) = fixture_with_bounds("upperBound=\"-1\"");
    storage::atomic(&mut store, |s| {
        s.set_unresolved_reference("root", "link", "future", false)
    })
    .unwrap();
    let before = storage::export(&store).unwrap();
    let provider = Policy {
        revision: &schema.revision,
        mode: "write_only",
    };
    let service = ComputedSettings {
        schema: &schema,
        provider: &provider,
        conversions: None,
    };
    assert!(
        service
            .apply(
                &mut store,
                "root",
                "computed",
                ComputedEdit::Append(json!("operand")),
                &mut QueryBudget::new(100)
            )
            .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), before);
    for mode in ["late", "invalid_readback"] {
        let invalid_provider = Policy {
            revision: &schema.revision,
            mode,
        };
        assert!(
            ComputedSettings {
                schema: &schema,
                provider: &invalid_provider,
                conversions: None
            }
            .apply_with_reference_policy(
                &mut store,
                "root",
                "computed",
                ComputedEdit::Append(json!("operand")),
                &mut QueryBudget::new(100),
                DraftReferencePolicy::PendingNonContainment
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    let outer = store.begin().unwrap();
    service
        .apply_with_reference_policy(
            &mut store,
            "root",
            "computed",
            ComputedEdit::Append(json!("operand")),
            &mut QueryBudget::new(100),
            DraftReferencePolicy::PendingNonContainment,
        )
        .unwrap();
    assert!(storage::validate_draft(&store, &schema).is_err());
    storage::set_with_opposite(&mut store, &schema, "root", "link", json!("value")).unwrap();
    storage::validate_draft(&store, &schema).unwrap();
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), before);

    // Provisional mode does not accept a missing resolved identity or pending containment.
    for pending_containment in [false, true] {
        let (_, mut invalid) = fixture_with_bounds("upperBound=\"-1\"");
        storage::atomic(&mut invalid, |s| {
            if pending_containment {
                s.set_unresolved_reference("root", "slots", "future", true)
            } else {
                s.set_value("root", "link", Some(json!("missing")))
            }
        })
        .unwrap();
        let before = storage::export(&invalid).unwrap();
        // Empty plan keeps the malformed field intact for shared draft validation.
        struct Noop<'a>(&'a Policy<'a>);
        impl DerivedValues for Noop<'_> {
            fn schema_revision(&self) -> Option<&str> {
                self.0.schema_revision()
            }
            fn supports(&self, class: &str, feature: &str) -> bool {
                self.0.supports(class, feature)
            }
            fn read(
                &self,
                _: &dyn ModelRead,
                _: &str,
                _: &str,
                budget: &mut QueryBudget,
            ) -> Result<Value, String> {
                budget.charge()?;
                Ok(json!([]))
            }
        }
        impl SettingValues for Noop<'_> {
            fn plan(
                &self,
                _: &dyn ModelRead,
                _: &str,
                _: &str,
                _: &ComputedEdit,
                budget: &mut QueryBudget,
            ) -> Result<Vec<SettingEdit>, String> {
                budget.charge()?;
                Ok(vec![])
            }
        }
        let noop = Noop(&provider);
        assert!(
            ComputedSettings {
                schema: &schema,
                provider: &noop,
                conversions: None
            }
            .apply_with_reference_policy(
                &mut invalid,
                "root",
                "computed",
                ComputedEdit::Append(json!("operand")),
                &mut QueryBudget::new(100),
                DraftReferencePolicy::PendingNonContainment
            )
            .is_err()
        );
        assert_eq!(storage::export(&invalid).unwrap(), before);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn set_only_containment_plan_binds_canonical_model_and_restores_failed_roots() {
    let (schema, mut store) = fixture();
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "slot", "Slot", "script.py", 2, 7)?;
        store.create_element(&schema, "value", "Value", "script.py", 3, 6)
    })
    .unwrap();
    let before = storage::export(&store).unwrap();
    assert_eq!(
        before.resources["script.py"].roots.as_slice(),
        ["slot", "value"]
    );
    let invalid_provider = Policy {
        revision: &schema.revision,
        mode: "attach_invalid_readback",
    };
    let invalid = ComputedSettings {
        schema: &schema,
        provider: &invalid_provider,
        conversions: None,
    };
    let mut model = before.clone();
    assert!(
        model
            .edit_computed(
                &invalid,
                "root",
                "computed",
                ComputedEdit::Set(json!("hello")),
                &mut QueryBudget::new(100),
            )
            .is_err()
    );
    assert_eq!(model, before);

    let mut retry = FrozenStore::from_materialized(&schema, &before).unwrap();
    assert_eq!(retry.metamodel().schema(), &schema);
    assert!(
        invalid
            .apply(
                &mut retry,
                "root",
                "computed",
                ComputedEdit::Set(json!("hello")),
                &mut QueryBudget::new(100),
            )
            .is_err()
    );
    assert_eq!(retry.metamodel().schema(), &schema);
    assert_eq!(storage::export(&retry).unwrap(), before);

    let provider = Policy {
        revision: &schema.revision,
        mode: "attach_existing",
    };
    model
        .edit_computed(
            &ComputedSettings {
                schema: &schema,
                provider: &provider,
                conversions: None,
            },
            "root",
            "computed",
            ComputedEdit::Set(json!("hello")),
            &mut QueryBudget::new(100),
        )
        .unwrap();
    assert!(model.resources["script.py"].roots.is_empty());
    assert_eq!(model.resources["model"].roots.as_slice(), ["root"]);
    assert_eq!(model.elements.len(), before.elements.len());
    for id in ["slot", "value"] {
        let element = model
            .elements
            .iter()
            .find(|element| element.id == id)
            .unwrap();
        assert_eq!(element.resource, "script.py");
        let original = before
            .elements
            .iter()
            .find(|element| element.id == id)
            .unwrap();
        assert_eq!(element.type_identity, original.type_identity);
        assert_eq!((element.start, element.end), (original.start, original.end));
    }
    model.validate(&schema).unwrap();
}
