use intersect_text::compiler;
use intersect_text::distribution::ordinary::{self as distribution, PackageMetadata};
use intersect_text::grammar::Grammar;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use serde_json::json;
use sha2::{Digest, Sha256};
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/compatibility/graph.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/compatibility/graph-behavior.rcore");
fn boolean() -> String {
    XTEXT.replace("'node' name=ID ('count' count=INT)? ('label' labels+=STRING)* ('peer' peer=[g::Node|ID])? ('link' links+=[g::Node|ID])* ';'", "'node' name=ID active?='active'")
}
fn metadata() -> PackageMetadata {
    PackageMetadata {
        package_id: "profiles.graph".into(),
        release: "1".into(),
        display_name: "Graph".into(),
        description: String::new(),
    }
}
#[test]
fn unchanged_default_rejects_same_input_accepted_by_explicit_native_profile() {
    for xtext in [boolean(), boolean().replace("?='active'", "?='active'?")] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
                ECORE,
                &xtext,
                RCORE_BEHAVIOR
            )
            .is_err()
        );
        let native =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &xtext,
                RCORE_BEHAVIOR,
                Profile::NativeV1,
            )
            .unwrap();
        assert_eq!(native.sources["xtext"], xtext);
        assert_eq!(native.grammar.as_ref().unwrap().profile, Profile::NativeV1);
        assert_eq!(
            serde_json::to_value(compiler::recompile(&native).unwrap()).unwrap(),
            serde_json::to_value(&native).unwrap()
        );
        let bundle = intersect_text::compiler::bundle_from_package(&native).unwrap();
        let compilation = BundleCompilation {
            materialized_types: vec![],
            ecore: bundle.sources[0].id.clone(),
            namespace: native.schema.uri.clone(),
            xtext: Some(bundle.sources[1].id.clone()),
            rcore_behavior: bundle.sources[2].id.clone(),
            bundle,
        };
        assert!(compiler::compile_bundle(compilation.clone()).is_err());
        let bundled =
            compiler::compile_bundle_with_profile(compilation, Profile::NativeV1).unwrap();
        assert_eq!(
            serde_json::to_value(compiler::recompile(&bundled).unwrap()).unwrap(),
            serde_json::to_value(&bundled).unwrap()
        );
        let mut downgraded = native.clone();
        downgraded.grammar.as_mut().unwrap().profile = Profile::WorkbenchV1;
        assert!(compiler::recompile(&downgraded).is_err());
        let mut forged = native.clone();
        forged.grammar.as_mut().unwrap().entry = "forged".into();
        assert!(intersect_text::compiler::bundle_from_package(&forged).is_err());
        assert_eq!(
            compiler::recompile(&forged).unwrap().grammar.unwrap().entry,
            native.grammar.unwrap().entry
        );
    }
}
#[test]
fn explicit_profiles_participate_in_retained_input_identity() {
    let legacy = distribution::build(
        &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
        XTEXT,
        RCORE_BEHAVIOR,
        metadata(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&legacy.package.grammar).unwrap()["profile"],
        Profile::WorkbenchV1.as_str()
    );
    assert_eq!(legacy.manifest.format_version, 2);
    let native = distribution::build_with_profile(
        &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
        XTEXT,
        RCORE_BEHAVIOR,
        metadata(),
        Profile::NativeV1,
    )
    .unwrap();
    assert_eq!(native.package.sources, legacy.package.sources);
    assert_ne!(native.manifest.revision, legacy.manifest.revision);
    assert_eq!(native.manifest.runtime.profile, Profile::NativeV1.as_str());
    let legacy_bundle = intersect_text::compiler::bundle_from_package(&legacy.package).unwrap();
    let native_bundle = intersect_text::compiler::bundle_from_package(&native.package).unwrap();
    assert_eq!(
        serde_json::to_value(legacy_bundle).unwrap(),
        serde_json::to_value(native_bundle).unwrap()
    );
    let mut grammar = serde_json::to_value(native.package.grammar.unwrap()).unwrap();
    for profile in [json!("unknown"), json!(null), json!(42)] {
        grammar["profile"] = profile;
        assert!(serde_json::from_value::<Grammar>(grammar.clone()).is_err());
    }
}
#[test]
fn archive_replay_checks_profile_and_retained_ir_in_addition_to_file_hashes() {
    let native = distribution::build_with_profile(
        &intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
        &boolean(),
        RCORE_BEHAVIOR,
        metadata(),
        Profile::NativeV1,
    )
    .unwrap();
    let mut files = native.files.clone();
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&native.manifest).unwrap(),
    );
    let rebuilt = distribution::import_files(files.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(rebuilt.package).unwrap(),
        serde_json::to_value(&native.package).unwrap()
    );
    for profile in ["unknown", Profile::WorkbenchV1.as_str()] {
        let mut changed = files.clone();
        let mut manifest = native.manifest.clone();
        manifest.runtime.profile = profile.into();
        changed.insert(
            "manifest.json".into(),
            serde_json::to_string(&manifest).unwrap(),
        );
        assert!(distribution::import_files(changed).is_err());
    }
    let mut manifest = native.manifest.clone();
    let mut wire = serde_json::to_value(&native.package).unwrap();
    wire["grammar"].as_object_mut().unwrap().remove("profile");
    let text = serde_json::to_string(&wire).unwrap();
    let digest = manifest.files.get_mut("package.json").unwrap();
    digest.bytes = text.len();
    digest.sha256 = format!("{:x}", Sha256::digest(text.as_bytes()));
    files.insert("package.json".into(), text);
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&manifest).unwrap(),
    );
    assert!(
        distribution::import_files(files)
            .unwrap_err()
            .contains("Archived compiled package differs")
    );
}
