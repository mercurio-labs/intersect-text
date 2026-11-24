use intersect_text::{
    compiler,
    distribution::{
        self, Distribution, ExecutionDistribution, ExecutionLanguageManifest, ExecutionManifest,
        ExecutionProfile, FileDigest, LanguageArchive, LanguageManifest, PackageMetadata,
    },
    grammar::Profile,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const PATH: &str = "execution/thermostat.rcore";
const SOURCE: &str = "// π\r\nrcore 1; package thermostat version \"1\";\r\n";

fn base() -> Distribution {
    let package = compiler::compile_rcore_project(
        r#"
        rcore 1;
        package example version "1";
        metamodel "urn:example" prefix example {
            class Item { attr name: String [1,1]; }
        }
        "#,
        r#"
        grammar example.Items with org.eclipse.xtext.common.Terminals
        import "urn:example" as example
        Item returns example::Item: 'item' name=ID ';';
        "#,
    )
    .unwrap();
    distribution::build_bundle_with_profile(
        package.compilation.unwrap(),
        PackageMetadata {
            package_id: "example.items".into(),
            release: "1".into(),
            display_name: "Items".into(),
            description: String::new(),
        },
        Profile::default(),
    )
    .unwrap()
}

fn declarations() -> ExecutionManifest {
    ExecutionManifest {
        default: Some("thermostat".into()),
        profiles: BTreeMap::from([
            (
                "thermostat".into(),
                ExecutionProfile {
                    engine: "intersect.coupled-ode@1".into(),
                    source: PATH.into(),
                },
            ),
            (
                "alternate".into(),
                ExecutionProfile {
                    engine: "intersect.coupled-ode@2".into(),
                    source: PATH.into(),
                },
            ),
        ]),
    }
}

fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([(PATH.into(), SOURCE.into())])
}

fn archive(manifest: &impl Serialize, mut files: DistributionFiles) -> DistributionFiles {
    files.insert(
        "manifest.json".into(),
        serde_json::to_string_pretty(manifest).unwrap(),
    );
    files
}

type DistributionFiles = BTreeMap<String, String>;

fn import_execution(files: DistributionFiles) -> ExecutionDistribution {
    let LanguageArchive::Execution(distribution) = distribution::import_archive(files).unwrap()
    else {
        panic!("expected a format-3 distribution");
    };
    distribution
}

#[test]
fn execution_archive_round_trips_exact_sources_and_preserves_ordinary_archive() {
    let base = base();
    assert_eq!(base.manifest.format_version, 2);
    // Exercise the format-2 manifest struct literal used by downstream Rust consumers.
    let legacy = &base.manifest;
    let original = archive(
        &LanguageManifest {
            format_version: legacy.format_version,
            metadata: legacy.metadata.clone(),
            revision: legacy.revision.clone(),
            ecore_namespace: legacy.ecore_namespace.clone(),
            schema_revision: legacy.schema_revision.clone(),
            compiler: legacy.compiler.clone(),
            runtime: legacy.runtime.clone(),
            definitions: legacy.definitions.clone(),
            compiled_package: legacy.compiled_package.clone(),
            files: legacy.files.clone(),
            python: legacy.python.clone(),
        },
        base.files.clone(),
    );
    assert!(!original["manifest.json"].contains("\"execution\""));
    let replay = distribution::import_files(original.clone()).unwrap();
    assert_eq!(archive(&replay.manifest, replay.files), original);
    let LanguageArchive::Source(replay) = distribution::import_archive(original.clone()).unwrap()
    else {
        panic!("expected a format-2 distribution");
    };
    assert_eq!(archive(&replay.manifest, replay.files), original);
    let extended = base
        .clone()
        .with_execution(declarations(), sources())
        .unwrap();
    assert_eq!(extended.manifest.language.format_version, 3);
    assert_eq!(extended.manifest.execution, declarations());
    assert_eq!(
        extended.manifest.language.schema_revision,
        base.manifest.schema_revision
    );
    assert_eq!(extended.files["package.json"], base.files["package.json"]);
    assert_ne!(extended.manifest.language.revision, base.manifest.revision);
    assert_eq!(
        extended.manifest.language.files[PATH],
        FileDigest::of(SOURCE.as_bytes())
    );

    let files = archive(&extended.manifest, extended.files.clone());
    let wire: serde_json::Value = serde_json::from_str(&files["manifest.json"]).unwrap();
    assert_eq!(wire["format_version"], 3);
    assert_eq!(wire["execution"]["default"], "thermostat");
    assert!(wire.get("language").is_none());
    assert!(distribution::import_files(files.clone()).is_err());
    let loaded = import_execution(files.clone());
    assert_eq!(loaded.files[PATH], SOURCE);
    assert_eq!(archive(&loaded.manifest, loaded.files), files);
}

#[test]
fn execution_profile_content_and_selection_contribute_to_revision() {
    let base = base();
    let mut changed_source = sources();
    changed_source.get_mut(PATH).unwrap().push('\n');
    let mut changed_engine = declarations();
    changed_engine
        .profiles
        .get_mut("thermostat")
        .unwrap()
        .engine = "intersect.coupled-ode@3".into();
    let mut changed_default = declarations();
    changed_default.default = None;
    let mut revisions = std::collections::BTreeSet::new();
    for (execution, sources) in [
        (declarations(), sources()),
        (declarations(), changed_source),
        (changed_engine, sources()),
        (changed_default, sources()),
    ] {
        let extended = base.clone().with_execution(execution, sources).unwrap();
        assert_eq!(
            extended.manifest.language.schema_revision,
            base.manifest.schema_revision
        );
        assert!(revisions.insert(extended.manifest.language.revision));
    }
}

#[test]
fn execution_authoring_rejects_invalid_declarations_and_incomplete_inventory() {
    let base = base();
    let mut unknown_default = declarations();
    unknown_default.default = Some("unknown".into());
    let mut empty_profiles = declarations();
    empty_profiles.default = None;
    empty_profiles.profiles.clear();
    for invalid in [unknown_default, empty_profiles] {
        assert!(base.clone().with_execution(invalid, sources()).is_err());
    }
    for path in [
        "/absolute.rcore",
        "execution/../bad.rcore",
        "execution\\bad.rcore",
        "execution/bad.toml",
        "file:execution.rcore",
    ] {
        let mut invalid = declarations();
        invalid.profiles.get_mut("thermostat").unwrap().source = path.into();
        assert!(invalid.source_paths().is_err(), "{path}");
    }
    for engine in ["unversioned", "engine@0", "engine@+1", "engine@2.0"] {
        let mut invalid = declarations();
        invalid.profiles.get_mut("thermostat").unwrap().engine = engine.into();
        assert!(invalid.source_paths().is_err(), "{engine}");
    }
    assert!(
        base.clone()
            .with_execution(declarations(), BTreeMap::new())
            .is_err()
    );
    let mut redundant = sources();
    redundant.insert("execution/extra.rcore".into(), SOURCE.into());
    assert!(
        base.clone()
            .with_execution(declarations(), redundant)
            .is_err()
    );
    let mut collision = base;
    collision.files.extend(sources());
    assert!(collision.with_execution(declarations(), sources()).is_err());
}

#[test]
fn execution_archive_rejects_tampering_even_if_payload_digest_is_repaired() {
    let extended = base().with_execution(declarations(), sources()).unwrap();
    let mut changed = extended.files.clone();
    changed.get_mut(PATH).unwrap().push('\n');
    assert!(
        distribution::import_archive(archive(&extended.manifest, changed.clone()))
            .unwrap_err()
            .contains("digest mismatch")
    );

    let mut manifest = extended.manifest.clone();
    manifest
        .language
        .files
        .insert(PATH.into(), FileDigest::of(changed[PATH].as_bytes()));
    assert!(
        distribution::import_archive(archive(&manifest, changed))
            .unwrap_err()
            .contains("definition identity mismatch")
    );
    manifest = extended.manifest.clone();
    manifest.execution.default = Some("alternate".into());
    assert!(
        distribution::import_archive(archive(&manifest, extended.files.clone()))
            .unwrap_err()
            .contains("definition identity mismatch")
    );
    let mut missing = extended.files.clone();
    missing.remove(PATH);
    manifest = extended.manifest.clone();
    manifest.language.files.remove(PATH);
    assert!(
        distribution::import_archive(archive(&manifest, missing))
            .unwrap_err()
            .contains("Missing execution profile source")
    );
    manifest = extended.manifest.clone();
    manifest.language.format_version = 2;
    assert!(distribution::import_archive(archive(&manifest, extended.files.clone())).is_err());
    assert!(
        distribution::import_archive(archive(&extended.manifest.language, extended.files)).is_err()
    );
}

#[test]
fn execution_archive_rejects_duplicate_profile_names_before_revision_check() {
    let extended = base().with_execution(declarations(), sources()).unwrap();
    let mut files = archive(&extended.manifest, extended.files);
    let original = &files["manifest.json"];
    let changed = original.replacen(
        "\"profiles\": {",
        r#""profiles": {"thermostat": {"engine":"intersect.coupled-ode@99","source":"execution/thermostat.rcore"},"#,
        1,
    );
    assert_ne!(&changed, original);
    // The validated entry is still last and the formerly accepted revision is identical.
    files.insert("manifest.json".into(), changed);
    let error = distribution::import_archive(files).unwrap_err();
    assert!(
        error.contains("Duplicate execution profile name"),
        "{error}"
    );
}

#[test]
fn execution_archive_rejects_unknown_and_duplicate_top_level_fields() {
    let extended = base().with_execution(declarations(), sources()).unwrap();
    let duplicate_execution = format!(
        "\"execution\":{},",
        serde_json::to_string(&extended.manifest.execution).unwrap()
    );
    let files = archive(&extended.manifest, extended.files);
    for field in [
        r#""unrecognized":true,"#,
        r#""format_version":3,"#,
        &duplicate_execution,
    ] {
        let mut changed = files.clone();
        changed.insert(
            "manifest.json".into(),
            files["manifest.json"].replacen('{', &format!("{{{field}"), 1),
        );
        assert!(distribution::import_archive(changed).is_err(), "{field}");
    }
}

#[test]
fn execution_archive_rejects_file_directory_collisions_when_building_or_importing() {
    let base = base();
    let definition = &base.manifest.definitions["bundle"];
    let collisions: [(String, String, String); 3] = [
        (
            format!("{definition}/profile.rcore"),
            PATH.into(),
            format!("{definition}-extra.rcore"),
        ),
        (
            "manifest.json/profile.rcore".into(),
            PATH.into(),
            "manifest.json-extra.rcore".into(),
        ),
        (
            "execution/parent.rcore".into(),
            "execution/parent.rcore/child.rcore".into(),
            "execution/parent.rcore-extra.rcore".into(),
        ),
    ];
    for (first, second, interleaved) in collisions {
        let mut execution = declarations();
        execution.profiles.get_mut("thermostat").unwrap().source = first.clone();
        execution.profiles.get_mut("alternate").unwrap().source = second.clone();
        // A similarly named sibling can sort between the parent and its child.
        execution.profiles.insert(
            "extra".into(),
            ExecutionProfile {
                engine: "intersect.coupled-ode@1".into(),
                source: interleaved.clone(),
            },
        );
        let sources = BTreeMap::from([
            (first, SOURCE.into()),
            (second, SOURCE.into()),
            (interleaved, SOURCE.into()),
        ]);
        let authoring_error = base
            .clone()
            .with_execution(execution.clone(), sources.clone())
            .unwrap_err();
        assert!(
            authoring_error.contains("Language archive path collides"),
            "{authoring_error}"
        );

        // Assemble the previously accepted wire fixture with a valid revision and digests.
        let digests = sources
            .iter()
            .map(|(path, source)| (path.clone(), FileDigest::of(source.as_bytes())))
            .collect::<BTreeMap<_, _>>();
        let serialized_identity = serde_json::to_vec(&(
            "intersect.language.execution-inputs.v1",
            &base.manifest.revision,
            &execution,
            &digests,
        ))
        .unwrap();
        let mut language = base.manifest.clone();
        language.format_version = 3;
        language.revision = format!("sha256:{:x}", Sha256::digest(serialized_identity));
        language.files.extend(digests);
        let mut files = base.files.clone();
        files.extend(sources);
        let manifest = ExecutionLanguageManifest {
            language,
            execution,
        };
        let importing_error = distribution::import_archive(archive(&manifest, files)).unwrap_err();
        assert!(
            importing_error.contains("Language archive path collides"),
            "{importing_error}"
        );
    }
}
