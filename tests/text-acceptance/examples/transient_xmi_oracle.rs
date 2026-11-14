//! Verification-only bridge: native import/export for the pinned EMF unsettable cases.
use intersect_model::core::{
    interchange::{ModelFormat, Resource},
    model::{Element, Model},
};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
use std::path::PathBuf;
fn schema(uri: &str, source: &str, namespace: &str, class: &str) -> Schema {
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let ecore = DefinitionSource::new(uri, SourceKind::Ecore, source);
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

fn write_export(
    output: &std::path::Path,
    name: &str,
    schema: &Schema,
    model: &Model,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = output.join(name);
    std::fs::create_dir_all(&directory)?;
    let ids = model
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            format!(
                "e{index}\t{}\n",
                element.id.rsplit('#').next().unwrap_or(&element.id)
            )
        })
        .collect::<String>();
    std::fs::write(directory.join("ids.tsv"), ids)?;
    for resource in XmiFormat.encode(schema, model)? {
        std::fs::write(directory.join(resource.uri), resource.bytes)?;
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(std::env::args().nth(1).ok_or("Output directory required")?);
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let root = fixtures.join("ecore-transient/emf-xmi");
    let oracle: Value = serde_json::from_slice(&std::fs::read(root.join("observations.json"))?)?;
    for case in oracle["cases"].as_array().ok_or("Missing cases")? {
        let name = case["name"].as_str().ok_or("Missing name")?;
        let uri = format!("memory:/{name}.xmi");
        let schema = schema(
            &format!("memory:/{name}.ecore"),
            &std::fs::read_to_string(root.join(format!("{name}.ecore")))?,
            case["namespace"].as_str().ok_or("Missing namespace")?,
            "A",
        );
        let mut model = Model {
            schema_revision: schema.revision.clone(),
            resources: Default::default(),
            elements: vec![],
        };
        for id in case["order"].as_array().ok_or("Missing object order")? {
            let id = id.as_str().ok_or("Invalid ID")?;
            let state = &case["before"][id];
            let class = state["class"].as_str().ok_or("Missing class")?;
            let feature = if class == "A" { "out" } else { "back" };
            let mut element = Element::new(&schema, &format!("{uri}#{id}"), class, &uri, 0, 0)?;
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
                element.assignment_state = Some(flags.with_assignment(
                    feature,
                    state["is_set"].as_bool().ok_or("Missing set state")?,
                )?);
            }
            model.elements.push(element);
        }
        model.resources =
            intersect_model::core::resources::records_for_elements(&schema, &model.elements)?;
        model.validate(&schema)?;
        write_export(&output.join("inverse"), name, &schema, &model)?;
    }
    let root = fixtures.join("ecore-transient/emf-values");
    let baseline_root = fixtures.join("ecore-unsettable/emf-xmi");
    let baseline = schema(
        "file:///schema.ecore",
        &std::fs::read_to_string(baseline_root.join("schema.ecore"))?,
        "urn:intersect:e09:xmi",
        "Item",
    );
    let schema = schema(
        "file:///schema.ecore",
        &std::fs::read_to_string(root.join("schema.ecore"))?,
        "urn:intersect:e09:xmi",
        "Item",
    );
    let oracle: Value = serde_json::from_slice(&std::fs::read(root.join("observations.json"))?)?;
    for case in oracle["cases"].as_array().ok_or("Missing cases")? {
        let name = case["name"].as_str().ok_or("Missing name")?;
        let file = case["file"].as_str().ok_or("Missing file")?;
        let mut model = XmiFormat.decode(
            &baseline,
            &[Resource {
                uri: file.into(),
                bytes: std::fs::read(baseline_root.join(file))?,
            }],
        )?;
        model.schema_revision = schema.revision.clone();
        for element in &mut model.elements {
            element.type_identity = schema.identity(&element.class);
        }
        model.resources =
            intersect_model::core::resources::records_for_elements(&schema, &model.elements)?;
        model.validate(&schema)?;
        write_export(&output.join("values"), name, &schema, &model)?;
    }
    Ok(())
}
