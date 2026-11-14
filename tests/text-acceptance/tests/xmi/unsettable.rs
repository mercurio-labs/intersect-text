use intersect_model::core::{
    interchange::{ModelFormat, Resource},
    model::Model,
};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
use std::path::Path;
fn schema() -> Schema {
    compile_schema(
        "file:///schema.ecore",
        include_str!("../../../../fixtures/ecore-unsettable/emf-xmi/schema.ecore"),
        "urn:intersect:e09:xmi",
        "Item",
    )
}
fn compile_schema(uri: &str, source: &str, namespace: &str, class: &str) -> Schema {
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let ecore = DefinitionSource::new(uri, SourceKind::Ecore, source);
    let (ecore, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(ecore);
    let mut xtext = DefinitionSource::new(
        "file:///model.xtext",
        SourceKind::Xtext,
        format!(
            "grammar state.Models with org.eclipse.xtext.common.Terminals\nimport \"{namespace}\" as s\nRoot returns s::{class}: 'item';"
        ),
    );
    let mut rcore_behavior = DefinitionSource::new(
        "file:///model-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    xtext.dependencies.push(ecore.id.clone());
    rcore_behavior.dependencies.push(ecore.id.clone());
    compile_bundle(BundleCompilation {
        materialized_types: vec![],
        ecore: ecore.id.clone(),
        xtext: Some(xtext.id.clone()),
        rcore_behavior: rcore_behavior.id.clone(),
        namespace: namespace.into(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, xtext, rcore_behavior],
            namespaces: vec![],
        },
    })
    .unwrap()
    .schema
}

fn resource(uri: &str, text: &str) -> Resource {
    Resource {
        uri: uri.into(),
        bytes: text.as_bytes().to_vec(),
    }
}
fn observe(model: &Model, schema: &Schema, subject: &str) -> Value {
    let element = model
        .elements
        .iter()
        .find(|e| e.id.ends_with(subject))
        .unwrap();
    let mut out = serde_json::Map::new();
    for (name, feature) in schema.features("Item").unwrap() {
        let mut value = element.values[&name].clone();
        if feature.reference {
            let normalize = |v: &Value| {
                if v.is_null() {
                    Value::Null
                } else {
                    json!(v.as_str().unwrap().rsplit('#').next().unwrap())
                }
            };
            value = if let Some(items) = value.as_array() {
                Value::Array(items.iter().map(normalize).collect())
            } else {
                normalize(&value)
            };
        }
        out.insert(name.clone(),json!({"value":value,"is_set":element.assignment_state.as_ref().unwrap().explicitly_set(&name)}));
    }
    Value::Object(out)
}
#[test]
fn unchanged_emf_files_preserve_all_seventeen_assignment_states_and_native_roundtrip() {
    let schema = schema();
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecore-unsettable/emf-xmi");
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-unsettable/emf-xmi/observations.json"
    ))
    .unwrap();
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 17);
    for case in cases {
        let file = case["file"].as_str().unwrap();
        let resources = [Resource {
            uri: file.into(),
            bytes: std::fs::read(root.join(file)).unwrap(),
        }];
        let model = XmiFormat.decode(&schema, &resources).unwrap();
        assert_eq!(
            observe(&model, &schema, "#subject"),
            case["after"],
            "{file}"
        );
        let encoded = XmiFormat.encode(&schema, &model).unwrap();
        let reloaded = XmiFormat.decode(&schema, &encoded).unwrap();
        assert_eq!(reloaded, model, "{file}");
    }
}
#[test]
fn empty_collection_empty_item_null_and_unset_have_distinct_wire_representations() {
    let schema = schema();
    let header = r#"<s:Item xmlns:s="urn:intersect:e09:xmi" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#;
    for (xml, expected) in [
        (format!("{header}/>"), vec![]),
        (format!("{header} tags=\"\"/>"), vec!["tags"]),
        (format!("{header}><tags></tags></s:Item>"), vec!["tags"]),
        (
            format!("{header}><nullable xsi:nil=\"true\"/></s:Item>"),
            vec!["nullable"],
        ),
    ] {
        let model = XmiFormat
            .decode(&schema, &[resource("model", &xml)])
            .unwrap();
        assert_eq!(
            model.elements[0]
                .assignment_state
                .as_ref()
                .unwrap()
                .set_features()
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            XmiFormat
                .decode(&schema, &XmiFormat.encode(&schema, &model).unwrap())
                .unwrap(),
            model
        );
    }
    for xml in [
        format!("{header} tags=\"\"><tags>x</tags></s:Item>"),
        format!("{header}><tags xsi:nil=\"true\"/></s:Item>"),
        format!("{header}><link xsi:nil=\"true\" href=\"#x\"/></s:Item>"),
        format!("{header}><child xsi:nil=\"true\"><children/></child></s:Item>"),
        format!("{header}><nullable xsi:nil=\"false\"/></s:Item>"),
    ] {
        assert!(
            XmiFormat.decode(&schema, &[resource("bad", &xml)]).is_err(),
            "{xml}"
        );
    }
}

#[test]
fn unchanged_emf_inverse_xmi_matches_twenty_four_configurations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ecore-unsettable/emf-inverse-xmi");
    let oracle: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("observations.json")).unwrap())
            .unwrap();
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 24);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let schema = compile_schema(
            &format!("memory:/{name}.ecore"),
            &std::fs::read_to_string(root.join(format!("{name}.ecore"))).unwrap(),
            case["namespace"].as_str().unwrap(),
            "A",
        );
        let resources = [Resource {
            uri: format!("memory:/{name}.xmi"),
            bytes: std::fs::read(root.join(format!("{name}.xmi"))).unwrap(),
        }];
        let model = XmiFormat
            .decode(&schema, &resources)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let normalize = |v: &Value| {
            if v.is_null() {
                Value::Null
            } else {
                json!(v.as_str().unwrap().rsplit('#').next().unwrap())
            }
        };
        let mut observed = serde_json::Map::new();
        for element in &model.elements {
            let feature = if element.class == "A" { "out" } else { "back" };
            let field = &schema.features(&element.class).unwrap()[feature];
            let value = &element.values[feature];
            let is_set = if field.unsettable {
                element
                    .assignment_state
                    .as_ref()
                    .unwrap()
                    .explicitly_set(feature)
            } else {
                !value.is_null() && value.as_array().is_none_or(|v| !v.is_empty())
            };
            let value = if let Some(items) = value.as_array() {
                Value::Array(items.iter().map(normalize).collect())
            } else {
                normalize(value)
            };
            let container = model
                .elements
                .iter()
                .find(|owner| {
                    schema
                        .features(&owner.class)
                        .unwrap()
                        .iter()
                        .any(|(key, f)| {
                            f.containment
                                && intersect_model::core::model::contains(
                                    owner.values.get(key),
                                    &element.id,
                                )
                        })
                })
                .map(|owner| normalize(&json!(owner.id)));
            observed.insert(
                element.id.rsplit('#').next().unwrap().into(),
                json!({"class":element.class,"value":value,"is_set":is_set,"container":container}),
            );
        }
        assert_eq!(Value::Object(observed), case["after"], "{name}");
        let encoded = XmiFormat.encode(&schema, &model).unwrap();
        assert_eq!(
            XmiFormat.decode(&schema, &encoded).unwrap(),
            model,
            "{name}"
        );
    }
}

#[test]
fn readonly_feature_defaults_load_but_authored_assignments_reject() {
    let mut schema = schema();
    schema
        .classes
        .get_mut("Item")
        .unwrap()
        .features
        .get_mut("nullable")
        .unwrap()
        .changeable = false;
    let header = r#"<s:Item xmlns:s="urn:intersect:e09:xmi" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#;
    assert!(
        XmiFormat
            .decode(&schema, &[resource("model", &format!("{header}/>"))])
            .is_ok()
    );
    for xml in [
        format!("{header} nullable=\"value\"/>"),
        format!("{header}><nullable xsi:nil=\"true\"/></s:Item>"),
    ] {
        assert!(
            XmiFormat
                .decode(&schema, &[resource("model", &xml)])
                .unwrap_err()
                .contains("non-changeable")
        );
    }
}

fn observe_inverse_projection(model: &Model, schema: &Schema) -> Value {
    let normalize = |v: &Value| {
        if v.is_null() {
            Value::Null
        } else {
            json!(v.as_str().unwrap().rsplit('#').next().unwrap())
        }
    };
    let mut result = serde_json::Map::new();
    for element in &model.elements {
        let name = if element.class == "A" { "out" } else { "back" };
        let raw = &element.values[name];
        let value = if let Some(list) = raw.as_array() {
            Value::Array(list.iter().map(normalize).collect())
        } else {
            normalize(raw)
        };
        let container = model
            .elements
            .iter()
            .find(|owner| {
                schema
                    .features(&owner.class)
                    .unwrap()
                    .iter()
                    .any(|(key, f)| {
                        f.containment
                            && intersect_model::core::model::contains(
                                owner.values.get(key),
                                &element.id,
                            )
                    })
            })
            .map(|e| normalize(&json!(e.id)));
        result.insert(element.id.rsplit('#').next().unwrap().into(),json!({"class":element.class,"value":value,"is_set":model.is_set(schema,&element.id,name).unwrap(),"container":container}));
    }
    Value::Object(result)
}
#[test]
fn transient_projection_matches_all_ninety_six_emf_inverse_configurations() {
    use intersect_model::core::model::Element;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecore-transient/emf-xmi");
    let oracle: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("observations.json")).unwrap())
            .unwrap();
    let cases = oracle["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 96);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let uri = format!("memory:/{name}.xmi");
        let schema = compile_schema(
            &format!("memory:/{name}.ecore"),
            &std::fs::read_to_string(root.join(format!("{name}.ecore"))).unwrap(),
            case["namespace"].as_str().unwrap(),
            "A",
        );
        let expected = Value::Object(
            case["after"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );
        let imported = XmiFormat
            .decode(
                &schema,
                &[Resource {
                    uri: uri.clone(),
                    bytes: std::fs::read(root.join(format!("{name}.xmi"))).unwrap(),
                }],
            )
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            observe_inverse_projection(&imported, &schema),
            expected,
            "EMF import {name}"
        );
        let mut before = Model {
            schema_revision: schema.revision.clone(),
            resources: Default::default(),
            elements: vec![],
        };
        for id in case["order"].as_array().unwrap() {
            let id = id.as_str().unwrap();
            let state = &case["before"][id];
            let class = state["class"].as_str().unwrap();
            let feature = if class == "A" { "out" } else { "back" };
            let mut element =
                Element::new(&schema, &format!("{uri}#{id}"), class, &uri, 0, 0).unwrap();
            let qualify = |v: &Value| {
                if v.is_null() {
                    Value::Null
                } else {
                    json!(format!("{uri}#{}", v.as_str().unwrap()))
                }
            };
            let raw = &state["value"];
            element.values.insert(
                feature.into(),
                if let Some(list) = raw.as_array() {
                    Value::Array(list.iter().map(qualify).collect())
                } else {
                    qualify(raw)
                },
            );
            if let Some(flags) = &element.assignment_state {
                element.assignment_state = Some(
                    flags
                        .with_assignment(feature, state["is_set"].as_bool().unwrap())
                        .unwrap(),
                );
            }
            before.elements.push(element);
        }
        before.resources =
            intersect_model::core::resources::records_for_elements(&schema, &before.elements)
                .unwrap();
        before.validate(&schema).unwrap();
        let original = before.clone();
        let resources = XmiFormat
            .encode(&schema, &before)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let reloaded = XmiFormat
            .decode(&schema, &resources)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            observe_inverse_projection(&reloaded, &schema),
            expected,
            "Native projection {name}"
        );
        assert_eq!(before, original);
        assert_eq!(
            XmiFormat
                .decode(&schema, &XmiFormat.encode(&schema, &reloaded).unwrap())
                .unwrap(),
            reloaded,
            "Stable projection {name}"
        );
    }
}

#[test]
fn transient_values_reset_on_persistence_without_changing_runtime_snapshots() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecore-transient/emf-values");
    let baseline_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecore-unsettable/emf-xmi");
    let baseline = schema();
    let schema = compile_schema(
        "file:///schema.ecore",
        &std::fs::read_to_string(root.join("schema.ecore")).unwrap(),
        "urn:intersect:e09:xmi",
        "Item",
    );
    let oracle: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("observations.json")).unwrap())
            .unwrap();
    assert_eq!(oracle["cases"].as_array().unwrap().len(), 17);
    for case in oracle["cases"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let imported = XmiFormat
            .decode(
                &schema,
                &[Resource {
                    uri: file.into(),
                    bytes: std::fs::read(root.join(file)).unwrap(),
                }],
            )
            .unwrap();
        assert_eq!(observe(&imported, &schema, "#subject"), case["after"]);
        let mut before = XmiFormat
            .decode(
                &baseline,
                &[Resource {
                    uri: file.into(),
                    bytes: std::fs::read(baseline_root.join(file)).unwrap(),
                }],
            )
            .unwrap();
        before.schema_revision = schema.revision.clone();
        for element in &mut before.elements {
            element.type_identity = schema.identity(&element.class);
        }
        before.resources =
            intersect_model::core::resources::records_for_elements(&schema, &before.elements)
                .unwrap();
        before.validate(&schema).unwrap();
        assert_eq!(observe(&before, &schema, "#subject"), case["before"]);
        let snapshot = serde_json::to_value(&before).unwrap();
        let resources = XmiFormat.encode(&schema, &before).unwrap();
        assert_eq!(serde_json::to_value(&before).unwrap(), snapshot);
        let reloaded = XmiFormat.decode(&schema, &resources).unwrap();
        assert_eq!(
            observe(&reloaded, &schema, "#subject"),
            case["after"],
            "{file}"
        );
    }
}
#[test]
fn persistent_reference_to_transiently_owned_object_rejects_dangling_projection() {
    use intersect_model::definitions::dynamic::SchemaBuilder;
    let mut builder = SchemaBuilder::new("p", "urn:p");
    let parent = builder.class("Parent").unwrap();
    let child = builder.class("Child").unwrap();
    parent
        .reference("children", &child)
        .many()
        .transient(true)
        .containment()
        .unwrap();
    parent.reference("link", &child).build().unwrap();
    child
        .reference("nested", &child)
        .many()
        .containment()
        .unwrap();
    let schema = builder.build().unwrap().as_schema().clone();
    let mut model = Model::default();
    model.create(&schema, "a", "Parent", "model", 0, 0).unwrap();
    model.create(&schema, "b", "Child", "model", 0, 0).unwrap();
    model
        .edit(
            &schema,
            &[
                ("a".into(), "children".into(), json!(["b"])),
                ("a".into(), "link".into(), json!("b")),
            ],
        )
        .unwrap();
    model.create(&schema, "g", "Child", "model", 0, 0).unwrap();
    model
        .edit(&schema, &[("b".into(), "nested".into(), json!(["g"]))])
        .unwrap();
    let before = model.clone();
    assert!(
        XmiFormat
            .encode(&schema, &model)
            .unwrap_err()
            .contains("omitted from persistent XMI projection")
    );
    assert_eq!(model, before);
    model
        .edit(&schema, &[("a".into(), "link".into(), Value::Null)])
        .unwrap();
    let projected = XmiFormat
        .decode(&schema, &XmiFormat.encode(&schema, &model).unwrap())
        .unwrap();
    assert_eq!(projected.elements.len(), 1);
    assert_eq!(model.elements.len(), 3);
}

#[test]
fn derived_unsettable_projection_replays_backing_history_without_serializing_computed_state() {
    use intersect_model::core::{
        frozen::FrozenStore,
        query::QueryBudget,
        storage::{self, ComputedEdit, ComputedSettings},
    };
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/ecore-settings/settings.ecore"),
        include_str!("../../../../fixtures/ecore-settings/settings.xtext"),
        include_str!("../../../../fixtures/ecore-settings/settings-behavior.rcore"),
    )
    .unwrap();
    let evaluation = intersect_text::evaluate(
        &package,
        &[intersect_text::Source {
            uri: "example.settings".into(),
            text: "model Demo { item a }".into(),
        }],
    );
    assert!(evaluation.diagnostics.is_empty());
    let id = evaluation
        .model
        .elements
        .iter()
        .find(|element| element.class == "Item")
        .unwrap()
        .id
        .clone();
    let mut store = FrozenStore::from_materialized(&package.schema, &evaluation.model).unwrap();
    let provider = intersect_model::query::PackageDerived(&package);
    let service = ComputedSettings {
        schema: &package.schema,
        provider: &provider,
        conversions: None,
    };
    for (edit, expected) in [
        (ComputedEdit::Set(json!("seed")), true),
        (ComputedEdit::Unset, false),
    ] {
        service
            .apply(
                &mut store,
                &id,
                "aliasTitle",
                edit,
                &mut QueryBudget::new(100),
            )
            .unwrap();
        let model = storage::export(&store).unwrap();
        let resources = XmiFormat.encode(&package.schema, &model).unwrap();
        assert!(
            resources
                .iter()
                .all(|resource| !String::from_utf8_lossy(&resource.bytes).contains("aliasTitle"))
        );
        let restored = XmiFormat.decode(&package.schema, &resources).unwrap();
        assert_eq!(restored, model);
        let view = storage::ModelView::new(&restored).unwrap();
        assert_eq!(
            storage::is_set_with_derived(
                &view,
                &package.schema,
                &id,
                "aliasTitle",
                &provider,
                &mut QueryBudget::new(100)
            )
            .unwrap(),
            expected
        );
    }
}
