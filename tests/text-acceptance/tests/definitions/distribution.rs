use intersect_text::distribution::ordinary::{self as distribution, Distribution, PackageMetadata};
use sha2::{Digest, Sha256};

const ECORE: &str = include_str!("../../../../fixtures/library/library.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/library/library.xtext");
const RCORE_BEHAVIOR: &str = include_str!("../../../../fixtures/library/library-behavior.rcore");

fn metadata() -> PackageMetadata {
    PackageMetadata {
        package_id: "org.example.library".into(),
        release: "1.0.0-beta.1".into(),
        display_name: "Library".into(),
        description: "A reusable library language.".into(),
    }
}

#[test]
fn exported_package_retains_nominal_datatypes_enum_aliases_and_exact_scalar_defaults() {
    let ecore = ECORE.replace("</ecore:EPackage>", "<eClassifiers xsi:type=\"ecore:EDataType\" name=\"Counter\" instanceClassName=\"long\"/><eClassifiers xsi:type=\"ecore:EEnum\" name=\"Mode\"><eLiterals name=\"First\" value=\"7\"/><eLiterals name=\"Alias\" value=\"7\"/></eClassifiers></ecore:EPackage>")
        .replace("<eClassifiers xsi:type=\"ecore:EClass\" name=\"BookCopy\" eSuperTypes=\"#//Named\">", "<eClassifiers xsi:type=\"ecore:EClass\" name=\"BookCopy\" eSuperTypes=\"#//Named\"><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"counter\" eType=\"#//Counter\" defaultValueLiteral=\"9007199254740993\"/><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"mode\" eType=\"#//Mode\" defaultValueLiteral=\"Alias\"/><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"floating\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EDouble\" defaultValueLiteral=\"NaN\"/>");
    let ecore = ecore.replace("name=\"floating\"", "name=\"decimal\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBigDecimal\" defaultValueLiteral=\"1.00\"/><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"floating\"");
    let ecore = ecore.replace("name=\"floating\"", "name=\"bytes\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EByteArray\" defaultValueLiteral=\"ABC\"/><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"floating\"");
    let ecore = ecore.replace("name=\"floating\"", "name=\"character\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EChar\" defaultValueLiteral=\"55296\"/><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"floating\"");
    let ecore = ecore.replace("name=\"floating\"", "name=\"date\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EDate\" defaultValueLiteral=\"1969-12-31T23:59:59.999+0000\"/><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"floating\"");
    let export = distribution::build(
        &intersect_text_acceptance::migrate_model_fixture(&ecore).unwrap(),
        XTEXT,
        RCORE_BEHAVIOR,
        metadata(),
    )
    .unwrap();
    let restored: intersect_text::package::Package =
        serde_json::from_str(&export.files["package.json"]).unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&export.package).unwrap()
    );
    let inputs: intersect_text::package::BundleCompilation =
        serde_json::from_str(&export.files["definitions/bundle.json"]).unwrap();
    assert_eq!(
        inputs.bundle.source(&inputs.ecore).unwrap().text,
        intersect_text_acceptance::migrate_model_fixture(&ecore).unwrap()
    );
    for name in [
        "counter",
        "mode",
        "floating",
        "decimal",
        "bytes",
        "character",
        "date",
    ] {
        let field = &restored.schema.classes["BookCopy"].features[name];
        restored
            .schema
            .validate_attribute(field, &field.default)
            .unwrap();
    }
    let fields = &restored.schema.classes["BookCopy"].features;
    assert_eq!(
        fields["date"].default,
        serde_json::json!({"kind":"date","milliseconds":"-1"})
    );
    assert_eq!(
        fields["bytes"].default,
        serde_json::json!({"kind":"bytes","hex":"AB0C"})
    );
    assert_eq!(
        fields["character"].default,
        serde_json::json!({"kind":"char16","unit":55296})
    );
    assert_eq!(
        fields["counter"].default["value"]["value"],
        "9007199254740993"
    );
    assert_eq!(fields["floating"].default["bits"], "7ff8000000000000");
    let alias =
        serde_json::to_value(restored.schema.enumerations["Mode"].parse("Alias").unwrap()).unwrap();
    assert_eq!(fields["mode"].default, alias);
    for (path, content) in &export.files {
        assert_eq!(
            export.manifest.files[path].sha256,
            format!("{:x}", Sha256::digest(content.as_bytes()))
        );
    }
}

#[test]
fn all_definitions_contribute_to_revision_but_release_metadata_does_not() {
    let original = distribution::build(
        &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
        XTEXT,
        RCORE_BEHAVIOR,
        metadata(),
    )
    .unwrap();
    let rcore = intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap();
    for (e, x, s) in [
        (format!("{rcore}\n"), XTEXT.into(), RCORE_BEHAVIOR.into()),
        (rcore.clone(), format!("{XTEXT}\n"), RCORE_BEHAVIOR.into()),
        (rcore.clone(), XTEXT.into(), format!("{RCORE_BEHAVIOR}\n")),
    ] {
        let changed = distribution::build(&e, &x, &s, metadata()).unwrap();
        assert_ne!(original.manifest.revision, changed.manifest.revision);
    }
    let mut renamed = metadata();
    renamed.release = "2.0.0".into();
    renamed.description = "Updated description".into();
    let republished = distribution::build(
        &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
        XTEXT,
        RCORE_BEHAVIOR,
        renamed,
    )
    .unwrap();
    assert_eq!(original.manifest.revision, republished.manifest.revision);
    assert_eq!(original.files, republished.files);
    assert_eq!(
        original.manifest.identity().unwrap().to_string(),
        format!("org.example.library@{}", original.manifest.revision)
    );
}

#[test]
fn distribution_retains_sources_and_verifiable_compiled_artifact() {
    let bundle = distribution::build(
        &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
        XTEXT,
        RCORE_BEHAVIOR,
        metadata(),
    )
    .unwrap();
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let restored: Distribution = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
    let inputs: intersect_text::package::BundleCompilation =
        serde_json::from_str(&restored.files["definitions/bundle.json"]).unwrap();
    assert_eq!(
        inputs.bundle.source(&inputs.ecore).unwrap().text,
        intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap()
    );
    assert_eq!(
        inputs
            .bundle
            .source(inputs.xtext.as_ref().unwrap())
            .unwrap()
            .text,
        XTEXT
    );
    assert_eq!(
        inputs.bundle.source(&inputs.rcore_behavior).unwrap().text,
        RCORE_BEHAVIOR
    );
    for (path, content) in &restored.files {
        let digest = &restored.manifest.files[path];
        assert_eq!(digest.bytes, content.len());
        assert_eq!(
            digest.sha256,
            format!("{:x}", Sha256::digest(content.as_bytes()))
        );
    }
    let compiled: serde_json::Value =
        serde_json::from_str(&restored.files["package.json"]).unwrap();
    assert_eq!(compiled, serde_json::to_value(restored.package).unwrap());
    assert_eq!(
        restored.manifest.ecore_namespace,
        "urn:intersect:example:library:1"
    );
    assert!(restored.manifest.python.is_none());
    assert_eq!(restored.manifest.format_version, 2);
}

#[test]
fn metadata_and_invalid_definitions_are_rejected() {
    for id in ["", "../library", "a/b", " library", "é", ".hidden"] {
        let mut data = metadata();
        data.package_id = id.into();
        assert!(
            distribution::build(
                &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
                XTEXT,
                RCORE_BEHAVIOR,
                data
            )
            .is_err()
        );
    }
    let mut data = metadata();
    data.release = "../1.0".into();
    assert!(data.validate().is_err());
    data = metadata();
    data.display_name = "\n".into();
    assert!(data.validate().is_err());
    data = metadata();
    data.description = "x".repeat(2001);
    assert!(data.validate().is_err());
    assert!(distribution::build("<bad/>", XTEXT, RCORE_BEHAVIOR, metadata()).is_err());
    assert!(
        distribution::build(
            &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
            XTEXT,
            "{}",
            metadata()
        )
        .is_err()
    );
}

#[test]
fn python_binding_versions_replay_supported_assets_and_reject_unknown_contracts() {
    use distribution::{FileDigest, PYTHON_BINDING_VERSION, PythonBinding};
    let mut bundle = distribution::build(
        &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
        XTEXT,
        RCORE_BEHAVIOR,
        metadata(),
    )
    .unwrap();
    for (path, content) in [
        ("python/intersect_api.py", "# generated test asset\n"),
        ("python/intersect_api.pyi", "# generated test stub\n"),
    ] {
        bundle.files.insert(path.into(), content.into());
        bundle.manifest.files.insert(
            path.into(),
            FileDigest {
                sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
                bytes: content.len(),
            },
        );
    }
    let binding = PythonBinding {
        generator: "intersect-python-api".into(),
        version: PYTHON_BINDING_VERSION,
        revision: bundle.manifest.revision.clone(),
        host_api: "intersect.script.v1".into(),
        source: "python/intersect_api.py".into(),
        stub: "python/intersect_api.pyi".into(),
    };
    let load = |python: PythonBinding| {
        let mut manifest = bundle.manifest.clone();
        manifest.python = Some(python);
        let mut files = bundle.files.clone();
        files.insert(
            "manifest.json".into(),
            serde_json::to_string(&manifest).unwrap(),
        );
        distribution::import_files(files)
    };
    {
        let version = PYTHON_BINDING_VERSION;
        let imported = load(PythonBinding {
            version,
            ..binding.clone()
        })
        .unwrap();
        assert_eq!(imported.manifest.python.unwrap().version, version);
        assert_eq!(imported.package.sources, bundle.package.sources);
    }
    for version in [
        0,
        1,
        PYTHON_BINDING_VERSION - 1,
        PYTHON_BINDING_VERSION + 1,
        u32::MAX,
    ] {
        assert!(
            load(PythonBinding {
                version,
                ..binding.clone()
            })
            .unwrap_err()
            .contains("Python binding metadata")
        );
    }
    for invalid in [
        PythonBinding {
            host_api: "unknown-host".into(),
            ..binding.clone()
        },
        PythonBinding {
            generator: "unknown-generator".into(),
            ..binding.clone()
        },
        PythonBinding {
            revision: "stale".into(),
            ..binding.clone()
        },
    ] {
        assert!(load(invalid).is_err());
    }
}

#[test]
fn package_import_version_preserves_the_python_wire_contract() {
    // Version 7 is the retained wire contract, independent of a browser checkout.
    // The live generator/importer agreement is exercised by the examples repo's
    // toolkit/tests/compatibility.node.js against its selected WASM runtime.
    assert_eq!(
        distribution::PYTHON_BINDING_VERSION,
        7,
        "Changing the Python binding wire version requires a compatibility decision"
    );
}
