//! Verification-only bridge: native import/export for the pinned EMF unsettable cases.
use intersect_model::core::interchange::{ModelFormat, Resource};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use serde_json::Value;
use std::path::PathBuf;
fn schema() -> Schema {
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let ecore = DefinitionSource::new(
        "file:///schema.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/ecore-unsettable/emf-xmi/schema.ecore"),
    );
    let mut xtext = DefinitionSource::new(
        "file:///model.xtext",
        SourceKind::Xtext,
        "grammar state.Models with org.eclipse.xtext.common.Terminals\nimport \"urn:intersect:e09:xmi\" as s\nItem returns s::Item: 'item';",
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
        namespace: "urn:intersect:e09:xmi".into(),
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
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecore-unsettable/emf-xmi");
    let schema = schema();
    let cases: Value = serde_json::from_slice(&std::fs::read(root.join("observations.json"))?)?;
    for case in cases["cases"].as_array().ok_or("Missing cases")? {
        let name = case["name"].as_str().ok_or("Missing name")?;
        let file = case["file"].as_str().ok_or("Missing file")?;
        let model = XmiFormat.decode(
            &schema,
            &[Resource {
                uri: file.into(),
                bytes: std::fs::read(root.join(file))?,
            }],
        )?;
        let directory = output.join(name);
        std::fs::create_dir_all(&directory)?;
        for resource in XmiFormat.encode(&schema, &model)? {
            std::fs::write(directory.join(resource.uri), resource.bytes)?;
        }
    }
    Ok(())
}
