//! Verification-only bridge: native import/export for the pinned EMF unsettable cases.
use intersect_model::core::interchange::{ModelFormat, Resource};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use serde_json::Value;
use std::path::PathBuf;
fn schema(uri: &str, source: &str, namespace: &str) -> Schema {
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let ecore = DefinitionSource::new(uri, SourceKind::Ecore, source);
    let mut xtext = DefinitionSource::new(
        "file:///model.xtext",
        SourceKind::Xtext,
        format!(
            "grammar state.Models with org.eclipse.xtext.common.Terminals\nimport \"{namespace}\" as s\nRoot returns s::A: 'item';"
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(std::env::args().nth(1).ok_or("Output directory required")?);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ecore-unsettable/emf-inverse-xmi");
    let cases: Value = serde_json::from_slice(&std::fs::read(root.join("observations.json"))?)?;
    for case in cases["cases"].as_array().ok_or("Missing cases")? {
        let name = case["name"].as_str().ok_or("Missing name")?;
        let file = format!("{name}.xmi");
        let schema = schema(
            &format!("memory:/{name}.ecore"),
            &std::fs::read_to_string(root.join(format!("{name}.ecore")))?,
            case["namespace"].as_str().ok_or("Missing namespace")?,
        );
        let model = XmiFormat.decode(
            &schema,
            &[Resource {
                uri: format!("memory:/{file}"),
                bytes: std::fs::read(root.join(&file))?,
            }],
        )?;
        let directory = output.join(name);
        std::fs::create_dir_all(&directory)?;
        // Identity mapping only: no values or assignment markers go to the oracle.
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
        for resource in XmiFormat.encode(&schema, &model)? {
            std::fs::write(directory.join(resource.uri), resource.bytes)?;
        }
    }
    Ok(())
}
