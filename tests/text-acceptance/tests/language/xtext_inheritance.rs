use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/inherited-grammar/model.ecore");
const BASE: &str = include_str!("../../../../fixtures/xtext/inherited-grammar/base.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/inherited-grammar/model.xtext");
fn inputs() -> BundleCompilation {
    let ecore = DefinitionSource::new("model.ecore", SourceKind::Ecore, ECORE);
    let mut base = DefinitionSource::new("base.xtext", SourceKind::Xtext, BASE);
    let mut child = DefinitionSource::new("model.xtext", SourceKind::Xtext, CHILD);
    let mut rcore_behavior = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        include_str!("../../../../fixtures/xtext/inherited-grammar/model-behavior.rcore"),
    );
    base.dependencies.push(ecore.id.clone());
    child.dependencies = vec![ecore.id.clone(), base.id.clone()];
    rcore_behavior.dependencies.push(ecore.id.clone());
    let result = BundleCompilation {
        ecore: ecore.id.clone(),
        xtext: Some(child.id.clone()),
        rcore_behavior: rcore_behavior.id.clone(),
        namespace: "urn:intersect:xtext:mini-kerml:inherited-grammar:1".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, child, rcore_behavior, base],
            namespaces: vec![],
        },
    };
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
#[test]
fn inherited_source_package_executes_and_replays_exact_inputs() {
    let input = inputs();
    let package = compiler::compile_bundle_with_profile(input.clone(), Profile::NativeV2).unwrap();
    let sources = vec![Source {
        uri: "inherited.kerml".into(),
        text: include_str!("../../../../fixtures/xtext/inherited-grammar/example.kerml").into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 2);
    assert_eq!(result.model.elements[0].class, "Model");
    assert_eq!(result.model.elements[0].values["name"], "p");
    assert_eq!(result.model.elements[1].class, "Type");
    assert_eq!(result.model.elements[1].values["name"], "Wheel");
    assert_eq!(
        serde_json::to_value(&package.compilation).unwrap(),
        serde_json::to_value(Some(input)).unwrap()
    );
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(evaluate(&replay, &sources)).unwrap()
    );
    for text in [
        "package p { type ; }",
        "package p { type Wheel;",
        "package p { type Wheel; } garbage",
    ] {
        let rejected = evaluate(
            &package,
            &[Source {
                uri: "bad.kerml".into(),
                text: text.into(),
            }],
        );
        assert!(!rejected.diagnostics.is_empty());
    }
}
#[test]
fn inherited_package_replay_checks_parent_digest() {
    let mut package = compiler::compile_bundle_with_profile(inputs(), Profile::NativeV2).unwrap();
    let bundle = &mut package.compilation.as_mut().unwrap().bundle;
    bundle
        .sources
        .iter_mut()
        .find(|source| source.id.uri == "base.xtext")
        .unwrap()
        .text
        .push_str("\n// tampered");
    assert!(
        compiler::recompile(&package)
            .unwrap_err()
            .iter()
            .any(|error| error.message.contains("digest mismatch"))
    );
}

#[test]
fn inherited_archive_preserves_all_sources_and_rebuilds_ir() {
    use intersect_text::distribution::ordinary::{self as distribution, PackageMetadata};
    let input = inputs();
    let metadata = PackageMetadata {
        package_id: "example.inherited".into(),
        release: "1".into(),
        display_name: "Inherited grammar".into(),
        description: "Two exact grammar sources".into(),
    };
    let export =
        distribution::build_bundle_with_profile(input.clone(), metadata.clone(), Profile::NativeV2)
            .unwrap();
    assert_eq!(export.manifest.format_version, 2);
    assert_eq!(export.manifest.runtime.package_format, 2);
    let bundle_path = export.manifest.definitions["bundle"].clone();
    assert_eq!(
        serde_json::from_str::<BundleCompilation>(&export.files[&bundle_path])
            .unwrap()
            .bundle
            .sources
            .len(),
        4
    );
    let mut files = export.files.clone();
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&export.manifest).unwrap(),
    );
    let imported = distribution::import_files(files.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&export.package).unwrap(),
        serde_json::to_value(imported.package).unwrap()
    );
    files.get_mut(&bundle_path).unwrap().push(' ');
    assert!(
        distribution::import_files(files)
            .unwrap_err()
            .contains("digest mismatch")
    );
    let mut changed = input;
    let old = changed
        .bundle
        .sources
        .iter()
        .find(|source| source.id.uri == "base.xtext")
        .unwrap()
        .clone();
    let mut replacement = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        format!("{}\n// retained revision", old.text),
    );
    replacement.dependencies = old.dependencies.clone();
    for source in &mut changed.bundle.sources {
        for dependency in &mut source.dependencies {
            if *dependency == old.id {
                *dependency = replacement.id.clone();
            }
        }
        if source.id == old.id {
            *source = replacement.clone();
        }
    }
    let revised =
        distribution::build_bundle_with_profile(changed, metadata, Profile::NativeV2).unwrap();
    assert_ne!(export.manifest.revision, revised.manifest.revision);
}

#[test]
fn child_terminal_precedence_beats_an_identical_parent_terminal() {
    let mut input = inputs();
    let old_base = input
        .bundle
        .sources
        .iter()
        .find(|source| source.id.uri == "base.xtext")
        .unwrap()
        .clone();
    let old_child = input
        .bundle
        .sources
        .iter()
        .find(|source| source.id.uri == "model.xtext")
        .unwrap()
        .clone();
    let mut base = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        format!("{BASE}\nterminal PARENT: '@';"),
    );
    base.dependencies = old_base.dependencies;
    let mut child = DefinitionSource::new(
        "model.xtext",
        SourceKind::Xtext,
        format!(
            "{}\nterminal CHILD: '@';",
            CHILD.replace("name=Name", "name=CHILD")
        ),
    );
    child.dependencies = vec![input.ecore.clone(), base.id.clone()];
    input.xtext = Some(child.id.clone());
    for source in &mut input.bundle.sources {
        if source.id.uri == old_base.id.uri {
            *source = base.clone();
        } else if source.id.uri == old_child.id.uri {
            *source = child.clone();
        }
    }
    let package = compiler::compile_bundle_with_profile(input, Profile::NativeV2).unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "priority.kerml".into(),
            text: "package @ { type Wheel; }".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[0].values["name"], "@");
}

#[test]
fn every_consumed_grammar_must_pin_the_selected_ecore_dependency() {
    let mut input = inputs();
    input
        .bundle
        .sources
        .iter_mut()
        .find(|source| source.id.uri == "base.xtext")
        .unwrap()
        .dependencies
        .clear();
    let errors = compiler::compile_bundle_with_profile(input, Profile::NativeV2).unwrap_err();
    assert!(
        errors.iter().any(|error| error.source == "base.xtext"
            && error.message.contains("exact Ecore dependency"))
    );
}
