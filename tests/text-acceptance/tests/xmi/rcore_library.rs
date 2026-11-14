use intersect_model::core::interchange::{ModelFormat, Resource};
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    rcore,
};
use intersect_model::xmi::XmiFormat;
use intersect_text::{Source, evaluate};

#[test]
fn typed_library_xmi_omits_derived_int64_and_preserves_enum_values() {
    let lowered = rcore::lower_structure(
        DefinitionSource::new(
            "memory:library.rcore",
            SourceKind::Rcore,
            include_str!("../../../../fixtures/library/library-project.rcore"),
        ),
        "memory:library.ecore",
    )
    .unwrap();
    let behavior = lowered.behavior.unwrap();
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &lowered.generated.text,
        include_str!("../../../../fixtures/library/library.xtext"),
        &behavior.source.text,
    )
    .unwrap();
    let evaluated = evaluate(
        &package,
        &[Source {
            uri: "catalog.library".into(),
            text:
                "library Catalog { copy dune \"Dune\"; member ada; loan first copy dune to ada; }"
                    .into(),
        }],
    );
    assert!(evaluated.diagnostics.is_empty());
    let resources = XmiFormat.encode(&package.schema, &evaluated.model).unwrap();
    assert!(resources.iter().all(|r| {
        let text = std::str::from_utf8(&r.bytes).unwrap();
        !text.contains("copyCount=") && !text.contains("available=")
    }));
    let restored = XmiFormat.decode(&package.schema, &resources).unwrap();
    assert_eq!(restored.elements.len(), evaluated.model.elements.len());
    let copy = restored
        .elements
        .iter()
        .find(|e| e.class == "BookCopy")
        .unwrap();
    let original = evaluated
        .model
        .elements
        .iter()
        .find(|e| e.class == "BookCopy")
        .unwrap();
    assert_eq!(copy.values["condition"], original.values["condition"]);
    let mut invalid = resources.clone();
    invalid[0].bytes = String::from_utf8(invalid[0].bytes.clone())
        .unwrap()
        .replace("name=\"Catalog\"", "name=\"Catalog\" copyCount=\"99\"")
        .into_bytes();
    assert!(
        XmiFormat
            .decode(&package.schema, &invalid)
            .unwrap_err()
            .contains("Unsupported serialized feature Library.copyCount")
    );
    let xml = lowered
        .generated
        .text
        .replace("derived=\"true\"", "derived=\"false\"");
    let stored =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&xml)
            .unwrap();
    let empty: Vec<Resource> = vec![];
    assert!(
        XmiFormat
            .decode(&stored, &empty)
            .unwrap_err()
            .contains("Unsupported XMI feature: copyCount")
    );
}
