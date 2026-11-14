use intersect_model::definitions::bundle::{
    BUNDLE_VERSION, DefinitionBundle, DefinitionSource, NamespaceBinding, SourceKind, SourceSpan,
};
use intersect_text::compiler;

const ECORE: &str = include_str!("../../../../fixtures/library/library.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/library/library.xtext");
const RCORE_BEHAVIOR: &str = include_str!("../../../../fixtures/library/library-behavior.rcore");

fn library() -> DefinitionBundle {
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let mut bundle = intersect_text::compiler::bundle_from_package(&package).unwrap();
    bundle.namespaces.push(NamespaceBinding {
        namespace: package.schema.uri,
        source: bundle.sources[0].id.clone(),
    });
    bundle
}
fn compile_fixture(bundle: DefinitionBundle) -> Result<intersect_text::package::Package, String> {
    let mut selected = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap()
    .compilation
    .unwrap();
    selected.bundle = bundle;
    compiler::compile_bundle(selected).map_err(|e| format!("{e:?}"))
}

#[test]
fn nested_bundle_preserves_existing_declaration_source_identity() {
    let start = ECORE
        .find("<eClassifiers xsi:type=\"ecore:EClass\" name=\"Named\"")
        .unwrap();
    let end = start + ECORE[start..].find("</eClassifiers>").unwrap() + "</eClassifiers>".len();
    let text = ECORE
        .replace(
            &ECORE[start..end],
            &format!(
                "<eSubpackages name=\"common\" nsURI=\"urn:common\">{}</eSubpackages>",
                &ECORE[start..end]
            ),
        )
        .replace("#//Named", "#//common/Named");
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &text,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let bundle = intersect_text::compiler::bundle_from_package(&package).unwrap();
    for declaration in package.schema.declarations.values() {
        assert_eq!(
            bundle.source(&declaration.source).unwrap().text,
            intersect_text_acceptance::migrate_model_fixture(&text).unwrap()
        );
        assert_eq!(
            bundle.source(&declaration.source).unwrap().kind,
            SourceKind::Rcore
        );
    }
    assert_eq!(
        serde_json::to_value(compiler::recompile(&package).unwrap()).unwrap(),
        serde_json::to_value(package).unwrap()
    );
}

#[test]
fn retained_library_round_trip_uses_the_same_compiler_and_exact_sources() {
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let bundle = intersect_text::compiler::bundle_from_package(&package).unwrap();
    let encoded = serde_json::to_vec(&bundle).unwrap();
    let decoded: DefinitionBundle = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, bundle);
    let compiled = compile_fixture(decoded).unwrap();
    assert_eq!(
        compiled.sources["ecore"],
        intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap()
    );
    assert_eq!(compiled.sources["xtext"], XTEXT);
    assert_eq!(compiled.sources["rcore_behavior"], RCORE_BEHAVIOR);
    assert_eq!(
        serde_json::to_value(package).unwrap(),
        serde_json::to_value(compiled).unwrap()
    );
}

#[test]
fn coexistence_does_not_implicitly_select_a_revision() {
    let mut bundle = library();
    let original = bundle.sources[0].id.clone();
    let revised = DefinitionSource::new("ecore", SourceKind::Ecore, format!("{ECORE}\n"));
    assert_ne!(original, revised.id);
    bundle.sources.push(revised.clone());
    assert_eq!(
        bundle
            .resolve_namespace(&bundle.namespaces[0].namespace)
            .unwrap()
            .id,
        original
    );
    bundle.namespaces[0].source = revised.id.clone();
    assert_eq!(
        bundle
            .resolve_namespace(&bundle.namespaces[0].namespace)
            .unwrap()
            .id,
        revised.id
    );
    assert!(compile_fixture(bundle.clone()).is_err());
    bundle.namespaces.push(NamespaceBinding {
        namespace: bundle.namespaces[0].namespace.clone(),
        source: original,
    });
    assert!(bundle.validate().unwrap_err().contains("Ambiguous"));
}

#[test]
fn dependencies_require_exact_uri_and_digest_and_duplicates_are_rejected() {
    let mut bundle = library();
    bundle.sources[1].dependencies[0].sha256 = "0".repeat(64);
    assert!(
        bundle
            .validate()
            .unwrap_err()
            .contains("Missing exact source dependency")
    );
    bundle = library();
    bundle.sources.push(bundle.sources[0].clone());
    assert!(
        bundle
            .validate()
            .unwrap_err()
            .contains("Duplicate source identity")
    );
    bundle = library();
    let dependency = bundle.sources[1].dependencies[0].clone();
    bundle.sources[1].dependencies.push(dependency);
    assert!(
        bundle
            .validate()
            .unwrap_err()
            .contains("Duplicate dependency")
    );
}

#[test]
fn content_versions_and_namespace_bindings_are_validated_at_use() {
    let mut bundle = library();
    bundle.sources[0].text.push('\n');
    assert!(
        bundle
            .resolve_namespace(&bundle.namespaces[0].namespace)
            .unwrap_err()
            .contains("digest mismatch")
    );
    bundle = library();
    bundle.version += 1;
    assert!(
        compile_fixture(bundle.clone())
            .unwrap_err()
            .contains("version")
    );
    bundle = library();
    bundle.namespaces[0].source = bundle.sources[1].id.clone();
    assert!(bundle.validate().unwrap_err().contains("Ecore source"));
    bundle = library();
    bundle.namespaces[0].namespace = "urn:wrong".into();
    assert!(
        compile_fixture(bundle.clone())
            .unwrap_err()
            .to_lowercase()
            .contains("namespace")
    );
    assert!(bundle.resolve_namespace("urn:absent").is_err());
}

#[test]
fn compiled_metadata_tampering_and_unrecognized_source_are_rejected() {
    let mut package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    package.grammar.as_mut().unwrap().name = "different.Grammar".into();
    assert!(
        intersect_text::compiler::bundle_from_package(&package)
            .unwrap_err()
            .contains("metadata differs")
    );
    let mut package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    package
        .sources
        .insert("other".into(), "retained unknown".into());
    assert!(intersect_text::compiler::bundle_from_package(&package).is_err());
}

#[test]
fn missing_member_inventory_and_changed_provenance_are_rejected() {
    let original = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let mut legacy = original.clone();
    for class in legacy.schema.classes.values_mut() {
        class.feature_order.clear();
        class.feature_declarations.clear();
    }
    assert!(intersect_text::compiler::bundle_from_package(&legacy).is_err());
    legacy
        .schema
        .classes
        .get_mut("Named")
        .unwrap()
        .features
        .get_mut("name")
        .unwrap()
        .lower = 0;
    assert!(intersect_text::compiler::bundle_from_package(&legacy).is_err());
    let mut changed = original.clone();
    changed
        .schema
        .classes
        .get_mut("Named")
        .unwrap()
        .feature_declarations
        .get_mut("name")
        .unwrap()
        .path
        .push(99);
    assert!(intersect_text::compiler::bundle_from_package(&changed).is_err());
    let mut changed = original;
    changed
        .schema
        .classes
        .get_mut("Library")
        .unwrap()
        .feature_order
        .reverse();
    assert!(intersect_text::compiler::bundle_from_package(&changed).is_err());
}

#[test]
fn source_spans_preserve_non_bmp_crlf_and_eof_boundaries() {
    let source = DefinitionSource::new("memory:unicode", SourceKind::Xtext, "a😀\r\né");
    let span = SourceSpan::from_utf16(&source, 1, 3).unwrap();
    assert_eq!((span.start, span.end), (1, 5));
    let bundle = DefinitionBundle {
        version: BUNDLE_VERSION,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    assert_eq!(span.text(&bundle).unwrap(), "😀");
    assert_eq!(
        SourceSpan::from_utf16(&source, 3, 5)
            .unwrap()
            .text(&bundle)
            .unwrap(),
        "\r\n"
    );
    assert_eq!(
        SourceSpan::from_utf16(&source, 6, 6)
            .unwrap()
            .text(&bundle)
            .unwrap(),
        ""
    );
    assert!(SourceSpan::from_utf16(&source, 2, 3).is_err());
    assert!(SourceSpan::from_utf16(&source, 0, 7).is_err());
    assert!(SourceSpan::from_utf16(&source, 5, 4).is_err());
    let invalid = SourceSpan {
        source: source.id,
        start: 2,
        end: 5,
    };
    assert!(invalid.text(&bundle).is_err());
}

#[test]
fn spans_cannot_float_to_a_changed_source_revision() {
    let source = DefinitionSource::new("ecore", SourceKind::Ecore, "one");
    let span = SourceSpan::from_utf16(&source, 0, 3).unwrap();
    let bundle = DefinitionBundle {
        version: BUNDLE_VERSION,
        sources: vec![DefinitionSource::new("ecore", SourceKind::Ecore, "two")],
        namespaces: vec![],
    };
    assert!(
        span.text(&bundle)
            .unwrap_err()
            .contains("Missing exact definition source")
    );
}

#[test]
fn unknown_wire_fields_are_not_silently_discarded() {
    let mut encoded = serde_json::to_value(library()).unwrap();
    encoded["implied_namespace_resolution"] = true.into();
    assert!(serde_json::from_value::<DefinitionBundle>(encoded).is_err());
}
