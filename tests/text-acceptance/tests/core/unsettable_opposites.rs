use super::Store;
use intersect_model::core::storage::{self, ModelRead, ModelStore};
use intersect_model::definitions::{dynamic::SchemaBuilder, schema::Schema};
use serde_json::{Value, json};
fn schema(case: &Value) -> Schema {
    let mut builder = SchemaBuilder::new("inverse", "urn:inverse");
    let a = builder.class("A").unwrap();
    let b = builder.class("B").unwrap();
    let out = a
        .reference("out", &b)
        .bounds(0, if case["out_many"] == true { -1 } else { 1 })
        .unsettable(case["out_unsettable"] == true);
    let out = if case["containment"] == true {
        out.containment().unwrap()
    } else {
        out.build().unwrap()
    };
    let back = b
        .reference("back", &a)
        .bounds(0, if case["back_many"] == true { -1 } else { 1 })
        .unsettable(case["back_unsettable"] == true)
        .build()
        .unwrap();
    builder.opposites(&out, &back).unwrap();
    builder.build().unwrap().as_schema().clone()
}
fn setup(schema: &Schema) -> Store {
    let mut store = crate::stores::store::<Store>(schema).unwrap();
    storage::atomic(&mut store, |s| {
        for id in ["a1", "a2", "b1", "b2"] {
            s.create_element(
                schema,
                id,
                if id.starts_with('a') { "A" } else { "B" },
                "model",
                0,
                0,
            )?;
        }
        Ok(())
    })
    .unwrap();
    store
}
fn assign(
    store: &mut Store,
    schema: &Schema,
    id: &str,
    feature: &str,
    target: Option<&str>,
) -> Result<(), String> {
    let class = store.class(id)?.ok_or("Missing class")?;
    let fields = schema.features(&class)?;
    let value = if fields[feature].upper == 1 {
        target.map_or(Value::Null, |s| json!(s))
    } else {
        json!(target.into_iter().collect::<Vec<_>>())
    };
    storage::edit(store, schema, &[(id.into(), feature.into(), value)])
}
fn observe(store: &Store, schema: &Schema) -> Value {
    let model = storage::export(store).unwrap();
    let mut objects = serde_json::Map::new();
    for object in &model.elements {
        let name = if object.class == "A" { "out" } else { "back" };
        let value = &object.values[name];
        let is_set = storage::is_set(store, schema, &object.id, name).unwrap();
        let container = model
            .elements
            .iter()
            .find(|owner| {
                schema
                    .features(&owner.class)
                    .unwrap()
                    .iter()
                    .any(|(name, f)| {
                        f.containment
                            && intersect_model::core::model::contains(
                                owner.values.get(name),
                                &object.id,
                            )
                    })
            })
            .map(|o| o.id.clone());
        objects.insert(
            object.id.clone(),
            json!({"value":value,"is_set":is_set,"container":container}),
        );
    }
    Value::Object(objects)
}
store_test! {
pub fn inverse_state_matches_pinned_emf_in_all_twenty_four_flag_and_shape_cases() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-feature-flags/emf-opposites-observations.json"
    ))
    .unwrap();
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    let mut checked = 0;
    for case in cases {
        let schema = schema(case);
        let mut store = setup(&schema);
        for step in case["steps"].as_array().unwrap() {
            let operation = step["operation"].as_str().unwrap();
            match operation {
                "fresh" => (),
                "explicit-empty" => assign(&mut store, &schema, "a1", "out", None).unwrap(),
                "bind-first" | "bind-again" | "bind-forward" => {
                    assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap()
                }
                "replace-target" => assign(&mut store, &schema, "a1", "out", Some("b2")).unwrap(),
                "unset-forward" => storage::unset(&mut store, &schema, "a1", "out").unwrap(),
                "bind-reverse" => assign(&mut store, &schema, "b1", "back", Some("a1")).unwrap(),
                "unset-reverse" => storage::unset(&mut store, &schema, "b1", "back").unwrap(),
                "move-owner" => assign(&mut store, &schema, "a2", "out", Some("b1")).unwrap(),
                "empty-forward" => assign(&mut store, &schema, "a2", "out", None).unwrap(),
                "unset-forward-again" => storage::unset(&mut store, &schema, "a2", "out").unwrap(),
                other => panic!("Unknown oracle operation {other}"),
            }
            storage::validate(&store, &schema).unwrap();
            assert_eq!(
                observe(&store, &schema),
                step["objects"],
                "{operation} {case}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 288);
}
}
store_test! {
pub fn inverse_default_flags_and_values_rollback_together_after_late_failure() {
    let case = json!({"out_many":true,"back_many":false,"out_unsettable":true,"back_unsettable":true,"containment":true});
    let schema = schema(&case);
    let mut store = setup(&schema);
    assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap();
    let before = storage::export(&store).unwrap();
    let outer = store.begin().unwrap();
    storage::unset(&mut store, &schema, "a1", "out").unwrap();
    assert!(!storage::is_explicitly_set(&store, &schema, "b1", "back").unwrap());
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), before);
    assert!(
        storage::edit(
            &mut store,
            &schema,
            &[
                ("a1".into(), "out".into(), json!(["b2"])),
                ("a2".into(), "out".into(), json!(["missing"]))
            ]
        )
        .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), before);
}
}

store_test! {
pub fn identical_scalar_assignment_keeps_existing_inverse_order() {
    let case = json!({"out_many":false,"back_many":true,"out_unsettable":true,"back_unsettable":true,"containment":false});
    let schema = schema(&case);
    let mut store = setup(&schema);
    assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap();
    assign(&mut store, &schema, "a2", "out", Some("b1")).unwrap();
    let before = storage::export(&store).unwrap();
    assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap();
    assert_eq!(storage::export(&store).unwrap(), before);
    assert_eq!(
        store.value("b1", "back").unwrap(),
        Some(json!(["a1", "a2"]))
    );
}
}

store_test! {
pub fn assigning_null_container_remains_unset_but_ordinary_reference_becomes_set() {
    for containment in [false, true] {
        let schema = schema(
            &json!({"containment":containment,"out_many":true,"back_many":false,"out_unsettable":true,"back_unsettable":true}),
        );
        let mut store = setup(&schema);
        assign(&mut store, &schema, "b1", "back", None).unwrap();
        assert_eq!(
            storage::is_explicitly_set(&store, &schema, "b1", "back").unwrap(),
            !containment
        );
        if containment {
            storage::atomic(&mut store, |s| {
                s.set_assignment_state(
                    "b1",
                    Some(intersect_model::core::model::AssignmentState::new([
                        "back".to_owned(),
                    ])?),
                )
            })
            .unwrap();
            assert!(
                storage::is_explicitly_set(&store, &schema, "b1", "back")
                    .unwrap_err()
                    .contains("Container assignment state")
            );
        }
    }
}
}

store_test! {
pub fn collection_edits_keep_retained_inverse_order_and_rollback_bad_targets() {
    use storage::CollectionEdit;
    let schema = schema(
        &json!({"out_many":true,"back_many":true,"out_unsettable":true,"back_unsettable":true,"containment":false}),
    );
    let mut store = setup(&schema);
    assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap();
    assign(&mut store, &schema, "a2", "out", Some("b1")).unwrap();
    storage::edit_collection(
        &mut store,
        &schema,
        "a1",
        "out",
        CollectionEdit::Add(json!("b2")),
    )
    .unwrap();
    assert_eq!(
        store.value("b1", "back").unwrap(),
        Some(json!(["a1", "a2"]))
    );
    let before = storage::export(&store).unwrap();
    assert!(
        storage::edit_collection(
            &mut store,
            &schema,
            "a1",
            "out",
            CollectionEdit::Add(json!("missing"))
        )
        .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), before);
    storage::edit_collection(
        &mut store,
        &schema,
        "a1",
        "out",
        CollectionEdit::Remove(json!("b2")),
    )
    .unwrap();
    assert_eq!(
        store.value("b1", "back").unwrap(),
        Some(json!(["a1", "a2"]))
    );
    assert_eq!(store.value("b2", "back").unwrap(), Some(json!([])));
    assert!(storage::is_explicitly_set(&store, &schema, "b2", "back").unwrap());
}
}

store_test! {
pub fn indexed_reference_moves_leave_inverse_order_and_replacement_detaches_only_old_target() {
    use storage::CollectionEdit;
    let schema = schema(
        &json!({"out_many":true,"back_many":true,"out_unsettable":true,"back_unsettable":true,"containment":false}),
    );
    let mut store = setup(&schema);
    assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap();
    assign(&mut store, &schema, "a2", "out", Some("b1")).unwrap();
    storage::edit_collection(
        &mut store,
        &schema,
        "a1",
        "out",
        CollectionEdit::Insert {
            index: 1,
            value: json!("b2"),
        },
    )
    .unwrap();
    storage::edit_collection(
        &mut store,
        &schema,
        "a1",
        "out",
        CollectionEdit::Move { from: 1, to: 0 },
    )
    .unwrap();
    assert_eq!(store.value("a1", "out").unwrap(), Some(json!(["b2", "b1"])));
    assert_eq!(
        store.value("b1", "back").unwrap(),
        Some(json!(["a1", "a2"]))
    );
    storage::edit_collection(
        &mut store,
        &schema,
        "a1",
        "out",
        CollectionEdit::RemoveAt(0),
    )
    .unwrap();
    storage::edit_collection(
        &mut store,
        &schema,
        "a1",
        "out",
        CollectionEdit::Replace {
            index: 0,
            value: json!("b2"),
        },
    )
    .unwrap();
    assert_eq!(store.value("b1", "back").unwrap(), Some(json!(["a2"])));
    assert_eq!(store.value("b2", "back").unwrap(), Some(json!(["a1"])));
}
}

store_test! {
pub fn displaced_relationship_and_history_roll_back_after_late_failure() {
    for many in [false, true] {
        let case = json!({"out_many":many,"back_many":false,"out_unsettable":true,"back_unsettable":true,"containment":true});
        let schema = schema(&case);
        let mut store = setup(&schema);
        assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap();
        let before = storage::export(&store).unwrap();
        let moved = if many { json!(["b1"]) } else { json!("b1") };
        assert!(
            storage::edit(
                &mut store,
                &schema,
                &[
                    ("a2".into(), "out".into(), moved),
                    ("a1".into(), "unknown".into(), Value::Null),
                ]
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
        let outer = store.begin().unwrap();
        assign(&mut store, &schema, "a2", "out", Some("b1")).unwrap();
        assert_eq!(store.value("b1", "back").unwrap(), Some(json!("a2")));
        store.rollback(&outer).unwrap();
        assert_eq!(storage::export(&store).unwrap(), before);
        assign(&mut store, &schema, "a2", "out", Some("b1")).unwrap();
        storage::validate(&store, &schema).unwrap();
    }
}
}

store_test! {
pub fn displacement_preserves_remaining_inverse_order_and_rejects_required_survivor_loss() {
    let case = json!({"out_many":true,"back_many":false,"out_unsettable":true,"back_unsettable":true,"containment":false});
    let mut schema = schema(&case);
    let mut store = setup(&schema);
    storage::edit(
        &mut store,
        &schema,
        &[("a1".into(), "out".into(), json!(["b2", "b1"]))],
    )
    .unwrap();
    assign(&mut store, &schema, "a2", "out", Some("b1")).unwrap();
    assert_eq!(store.value("a1", "out").unwrap(), Some(json!(["b2"])));
    assert!(storage::is_explicitly_set(&store, &schema, "a1", "out").unwrap());
    schema
        .classes
        .get_mut("A")
        .unwrap()
        .features
        .get_mut("out")
        .unwrap()
        .lower = 1;
    storage::validate(&store, &schema).unwrap();
    let before = storage::export(&store).unwrap();
    assert!(assign(&mut store, &schema, "a2", "out", Some("b2")).is_err());
    assert_eq!(storage::export(&store).unwrap(), before);
}
}

store_test! {
pub fn nonunique_declaration_matrix_and_valid_mutations_match_independent_emf() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-structure/nonunique-opposites.json"
    ))
    .unwrap();
    let mut states = 0;
    let mut invalid = 0;
    for case in oracle["cases"].as_array().unwrap() {
        let om = if case["out_many"] == true { -1 } else { 1 };
        let bm = if case["back_many"] == true { -1 } else { 1 };
        let text = format!(
            r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="duplicates" nsURI="urn:duplicates">
<eClassifiers x:type="e:EClass" name="A"><eStructuralFeatures x:type="e:EReference" name="out" eType="#//B" eOpposite="#//B/back" upperBound="{om}" unique="{}" containment="{}" unsettable="{}"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="B"><eStructuralFeatures x:type="e:EReference" name="back" eType="#//A" eOpposite="#//A/out" upperBound="{bm}" unique="{}" unsettable="{}"/></eClassifiers></e:EPackage>"##,
            case["out_unique"],
            case["containment"],
            case["unsettable"],
            case["back_unique"],
            case["unsettable"]
        );
        let built = (|| -> Result<Schema, String> {
            let mut builder = SchemaBuilder::new("duplicates", "urn:duplicates");
            let a = builder.class("A")?;
            let b = builder.class("B")?;
            let out = a
                .reference("out", &b)
                .bounds(0, om)
                .unique(case["out_unique"] == true)
                .unsettable(case["unsettable"] == true);
            let out = if case["containment"] == true {
                out.containment()?
            } else {
                out.build()?
            };
            let back = b
                .reference("back", &a)
                .bounds(0, bm)
                .unique(case["back_unique"] == true)
                .unsettable(case["unsettable"] == true)
                .build()?;
            builder.opposites(&out, &back)?;
            Ok(builder.build()?.as_schema().clone())
        })();
        assert_eq!(
            built.is_ok(),
            case["valid"] == true,
            "builder {case}: {built:?}"
        );
        let imported =
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&text);
        assert_eq!(
            imported.is_ok(),
            case["valid"] == true,
            "{case}: {imported:?}"
        );
        if case["valid"] != true {
            invalid += 1;
            continue;
        }
        let schema = imported.unwrap();
        let mut store = setup(&schema);
        for step in case["steps"].as_array().unwrap() {
            match step["operation"].as_str().unwrap() {
                "fresh" => (),
                "add-first" | "add-duplicate" => {
                    if om == 1 {
                        assign(&mut store, &schema, "a1", "out", Some("b1")).unwrap();
                    } else {
                        storage::edit_collection(
                            &mut store,
                            &schema,
                            "a1",
                            "out",
                            storage::CollectionEdit::Add(json!("b1")),
                        )
                        .unwrap();
                    }
                }
                "add-reverse" => {
                    if bm == 1 {
                        assign(&mut store, &schema, "b1", "back", Some("a2")).unwrap();
                    } else {
                        storage::edit_collection(
                            &mut store,
                            &schema,
                            "b1",
                            "back",
                            storage::CollectionEdit::Add(json!("a2")),
                        )
                        .unwrap();
                    }
                }
                "remove-one" => {
                    if om == 1 {
                        storage::unset(&mut store, &schema, "a1", "out").unwrap();
                    } else {
                        storage::edit_collection(
                            &mut store,
                            &schema,
                            "a1",
                            "out",
                            storage::CollectionEdit::Remove(json!("b1")),
                        )
                        .unwrap();
                    }
                }
                "unset-reverse" => storage::unset(&mut store, &schema, "b1", "back").unwrap(),
                _ => panic!("unknown operation"),
            }
            storage::validate(&store, &schema).unwrap();
            let mut actual = observe(&store, &schema);
            for object in actual.as_object_mut().unwrap().values_mut() {
                object.as_object_mut().unwrap().remove("container");
            }
            assert_eq!(actual, step["objects"], "{case}: {}", step["operation"]);
            states += 1;
        }
    }
    assert_eq!(invalid, 34);
    assert_eq!(states, 180);
}
}

store_test! {
pub fn retained_schema_rejects_nonunique_ownership_but_preserves_plain_duplicate_lists() {
    let mut builder = SchemaBuilder::new("plain", "urn:plain");
    let node = builder.class("Node").unwrap();
    node.reference("links", &node)
        .many()
        .unique(false)
        .build()
        .unwrap();
    let schema = builder.build().unwrap().as_schema().clone();
    let mut model = crate::stores::basic::<Store>(schema.clone()).unwrap();
    use intersect_model::core::api::ModelEdit;
    model.create("a", "Node", "model").unwrap();
    model.create("b", "Node", "model").unwrap();
    model.set("a", "links", json!(["b", "b"])).unwrap();
    assert_eq!(model.value("a", "links").unwrap(), Some(json!(["b", "b"])));
    let mut invalid = schema;
    invalid
        .classes
        .get_mut("Node")
        .unwrap()
        .features
        .get_mut("links")
        .unwrap()
        .containment = true;
    assert!(invalid.validate().unwrap_err().contains("must be unique"));
    assert!(crate::stores::basic::<Store>(invalid).is_err());
}
}
