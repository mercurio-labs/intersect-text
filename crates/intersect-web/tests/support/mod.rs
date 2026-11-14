//! Explicit conversion for independent Ecore fixtures used by host controls.
#![allow(dead_code)] // Each independent integration target uses a subset of these helpers.
pub fn migrate_text(ecore: &str) -> String {
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    let report = intersect_model::migration::migrate_ecore(
        DefinitionSource::new("memory:oracle.ecore", SourceKind::Ecore, ecore),
        "memory:model.rcore",
    )
    .unwrap();
    report
        .converted
        .unwrap_or_else(|| {
            panic!(
                "Fixture migration requires manual work: {:?}",
                report.issues
            )
        })
        .text
}

pub fn declaration(
    source: &intersect_model::definitions::bundle::DefinitionSource,
    name: &str,
) -> intersect_model::definitions::resources::DeclarationId {
    use intersect_model::definitions::{bundle::DefinitionBundle, resources::ResourceSet};
    ResourceSet::new(&DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    })
    .unwrap()
    .resolve(&source.id, &format!("#//{name}"))
    .unwrap()
}

pub fn with_default(source: &str, feature: &str, value: &str) -> String {
    let needle = format!("attr `{feature}`:");
    let line = source
        .lines()
        .find(|line| line.trim_start().starts_with(&needle))
        .unwrap();
    let replacement = format!(
        "{} default {};",
        line.strip_suffix(';').unwrap(),
        serde_json::to_string(value).unwrap()
    );
    source.replace(line, &replacement)
}
