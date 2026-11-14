use intersect_text::compiler::recompile;

const ECORE: &str = r#"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="books" nsURI="urn:books" nsPrefix="books">
<eClassifiers xsi:type="ecore:EClass" name="Book"><eStructuralFeatures xsi:type="ecore:EAttribute" name="title" lowerBound="1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
</ecore:EPackage>"#;

#[test]
fn structural_package_retains_exact_sources_without_inventing_syntax() {
    let mut package =
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(ECORE, "")
            .expect("compile structural definitions");
    assert!(package.grammar.is_none());
    assert_eq!(
        package.sources["ecore"],
        intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap()
    );
    assert!(
        package
            .compilation
            .as_ref()
            .unwrap()
            .bundle
            .sources
            .iter()
            .all(|source| source.kind != intersect_model::definitions::bundle::SourceKind::Ecore)
    );
    assert_eq!(package.sources["rcore_behavior"], package.sources["ecore"]);
    assert!(!package.sources.contains_key("xtext"));
    assert!(package.semantics.derived.is_empty());
    // The adapter boundary regenerates from retained sources, not serialized IR.
    package.schema.classes.clear();
    let rebuilt = recompile(&package).expect("recompile exact source");
    assert!(rebuilt.schema.classes.contains_key("Book"));
    assert_eq!(
        rebuilt.schema.revision,
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(ECORE, "")
            .expect("compile")
            .schema
            .revision
    );
}

#[test]
fn structural_mode_does_not_claim_missing_providers_or_unsupported_types() {
    let derived = ECORE.replace("name=\"title\"", "name=\"title\" derived=\"true\"");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
            &derived, ""
        )
        .expect_err("missing provider")[0]
            .message
            .contains("semantic provider")
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
            &ECORE.replace("#//EString", "#//EDouble"),
            ""
        )
        .is_ok()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
            &ECORE.replace("#//EString", "#//UnknownScalar"),
            ""
        )
        .is_err()
    );
    let library = include_str!("../../../../fixtures/library/library.ecore");
    let semantics = include_str!("../../../../fixtures/library/library-behavior.rcore");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
            library, semantics
        )
        .is_ok()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
            library, ""
        )
        .is_err()
    );
}
