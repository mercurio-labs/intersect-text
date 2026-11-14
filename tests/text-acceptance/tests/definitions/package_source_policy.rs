use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
fn input() -> (BundleCompilation, CompilationSourcePolicy) {
    let ecore = DefinitionSource::new(
        "memory:policy.rcore",
        SourceKind::Rcore,
        format!(
            "rcore 1; package m version \"1\"; metamodel \"urn:policy\" prefix m {{ class Item {{}} }} /*{}*/",
            "x".repeat(300_000)
        ),
    );
    let mut grammar = String::from("grammar policy.Model hidden(WS) import 'urn:policy' as m\n");
    for n in 0..281 {
        grammar.push_str(&format!("R{n} returns m::Item: 'r{n}';\n"));
    }
    grammar.push_str("terminal WS: ' '+;");
    let mut xtext = DefinitionSource::new("memory:policy.xtext", SourceKind::Xtext, grammar);
    let mut rcore_behavior = DefinitionSource::new(
        "memory:policy-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    xtext.dependencies.push(ecore.id.clone());
    rcore_behavior.dependencies.push(ecore.id.clone());
    let policy = CompilationSourcePolicy {
        structure_bytes: ecore.text.len(),
        xtext_bytes: xtext.text.len(),
        xtext_declarations: 282,
    };
    (
        BundleCompilation {
            ecore: ecore.id.clone(),
            xtext: Some(xtext.id.clone()),
            rcore_behavior: rcore_behavior.id.clone(),
            namespace: "urn:policy".into(),
            materialized_types: vec![],
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![ecore, xtext, rcore_behavior],
                namespaces: vec![],
            },
        },
        policy,
    )
}

#[test]
fn package_policy_requires_host_admission_and_exact_replay() {
    let (input, policy) = input();
    assert!(compiler::compile_bundle_with_profile(input.clone(), Profile::NativeV3).is_err());
    let package = compiler::compile_bundle_with_source_policy(
        input.clone(),
        Profile::NativeV3,
        policy,
        policy,
    )
    .unwrap();
    assert_eq!(package.source_policy, Some(policy));
    assert_eq!(package.grammar.as_ref().unwrap().rules.len(), 281);
    assert_eq!(
        serde_json::to_value(&package.compilation).unwrap(),
        serde_json::to_value(Some(&input)).unwrap()
    );
    assert!(compiler::recompile(&package).is_err());
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(compiler::recompile_with_source_policy(&package, policy).unwrap())
            .unwrap()
    );
    for lower in [
        CompilationSourcePolicy {
            structure_bytes: policy.structure_bytes - 1,
            ..policy
        },
        CompilationSourcePolicy {
            xtext_declarations: 281,
            ..policy
        },
    ] {
        assert!(
            compiler::recompile_with_source_policy(&package, lower).unwrap_err()[0]
                .message
                .contains("host admission limits")
        );
        assert!(
            compiler::compile_bundle_with_source_policy(
                input.clone(),
                Profile::NativeV3,
                lower,
                policy
            )
            .is_err()
        );
    }
    let mut forged = package.clone();
    forged.source_policy.as_mut().unwrap().structure_bytes += 1;
    assert!(compiler::recompile_with_source_policy(&forged, policy).is_err());
    let mut absent = serde_json::to_value(&package).unwrap();
    absent.as_object_mut().unwrap().remove("source_policy");
    let absent: intersect_text::package::Package = serde_json::from_value(absent).unwrap();
    assert!(absent.source_policy.is_none());
    assert!(
        !serde_json::to_value(&absent)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("source_policy")
    );
    assert!(compiler::recompile_with_source_policy(&absent, policy).is_err());
}
#[test]
fn archive_policy_is_identity_bound_and_cannot_self_grant_admission() {
    use intersect_text::distribution::ordinary::{self as distribution, PackageMetadata};
    use sha2::{Digest, Sha256};
    let (input, policy) = input();
    let metadata = PackageMetadata {
        package_id: "policy.example".into(),
        release: "1".into(),
        display_name: "Policy".into(),
        description: String::new(),
    };
    let archive = distribution::build_bundle_with_source_policy(
        input,
        metadata,
        Profile::NativeV3,
        policy,
        policy,
    )
    .unwrap();
    let mut files = archive.files.clone();
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&archive.manifest).unwrap(),
    );
    assert!(
        distribution::import_files(files.clone())
            .unwrap_err()
            .contains("host admission limits")
    );
    let replay = distribution::import_files_with_source_policy(files.clone(), policy).unwrap();
    assert_eq!(archive.manifest.revision, replay.manifest.revision);
    assert_eq!(
        serde_json::to_value(&archive.package).unwrap(),
        serde_json::to_value(replay.package).unwrap()
    );
    let mut manifest = archive.manifest.clone();
    manifest
        .runtime
        .source_policy
        .as_mut()
        .unwrap()
        .structure_bytes += 1;
    let mut changed = files.clone();
    changed.insert(
        "manifest.json".into(),
        serde_json::to_string(&manifest).unwrap(),
    );
    assert!(
        distribution::import_files_with_source_policy(changed, policy)
            .unwrap_err()
            .contains("host admission limits")
    );
    let path = &archive.manifest.compiled_package;
    let mut wire = serde_json::to_value(&archive.package).unwrap();
    wire["source_policy"]["structure_bytes"] = serde_json::json!(policy.structure_bytes + 1);
    let text = serde_json::to_string(&wire).unwrap();
    let mut manifest = archive.manifest.clone();
    let digest = manifest.files.get_mut(path).unwrap();
    digest.bytes = text.len();
    digest.sha256 = format!("{:x}", Sha256::digest(text.as_bytes()));
    files.insert(path.clone(), text);
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&manifest).unwrap(),
    );
    assert!(
        distribution::import_files_with_source_policy(files, policy)
            .unwrap_err()
            .contains("differs from freshly compiled")
    );
}

#[test]
fn stricter_host_ceiling_also_applies_to_migrated_packages_and_archives() {
    use intersect_text::distribution::ordinary::{self as distribution, PackageMetadata};
    let archive = distribution::build(
        &intersect_text_acceptance::migrate_model_fixture(include_str!(
            "../../../../fixtures/compatibility/graph.ecore"
        ))
        .unwrap(),
        include_str!("../../../../fixtures/compatibility/graph.xtext"),
        include_str!("../../../../fixtures/compatibility/graph-behavior.rcore"),
        PackageMetadata {
            package_id: "legacy.policy".into(),
            release: "1".into(),
            display_name: "Legacy".into(),
            description: String::new(),
        },
    )
    .unwrap();
    assert!(archive.package.source_policy.is_none());
    let strict = CompilationSourcePolicy {
        structure_bytes: 0,
        ..CompilationSourcePolicy::default()
    };
    assert!(compiler::recompile_with_source_policy(&archive.package, strict).is_err());
    let mut files = archive.files;
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&archive.manifest).unwrap(),
    );
    assert!(
        distribution::import_files_with_source_policy(files, strict)
            .unwrap_err()
            .contains("host admission limits")
    );
}

#[test]
fn source_policy_requires_the_canonical_structure_field() {
    let policy = CompilationSourcePolicy::default();
    let mut wire = serde_json::to_value(policy).unwrap();
    assert_eq!(wire["structure_bytes"], policy.structure_bytes);
    let value = wire
        .as_object_mut()
        .unwrap()
        .remove("structure_bytes")
        .unwrap();
    wire["ecore_bytes"] = value;
    assert!(serde_json::from_value::<CompilationSourcePolicy>(wire).is_err());
}
