use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/model.ecore");
const BASE: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/base.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/model.xtext");
fn inputs() -> BundleCompilation {
    let ecore = DefinitionSource::new("model.ecore", SourceKind::Ecore, ECORE);
    let mut base = DefinitionSource::new("base.xtext", SourceKind::Xtext, BASE);
    let mut child = DefinitionSource::new("model.xtext", SourceKind::Xtext, CHILD);
    let mut rcore_behavior = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        include_str!("../../../../fixtures/xtext/contextual-overrides/model-behavior.rcore"),
    );
    base.dependencies.push(ecore.id.clone());
    child.dependencies = vec![ecore.id.clone(), base.id.clone()];
    rcore_behavior.dependencies.push(ecore.id.clone());
    let result = BundleCompilation {
        ecore: ecore.id.clone(),
        xtext: Some(child.id.clone()),
        rcore_behavior: rcore_behavior.id.clone(),
        namespace: "urn:intersect:xtext:mini-kerml:contextual-overrides:1".into(),
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
fn inherited_calls_select_the_child_replacement_without_changing_base_context() {
    let input = inputs();
    let child = compiler::compile_bundle_with_profile(input, Profile::NativeV3).unwrap();
    let base = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        BASE,
        include_str!("../../../../fixtures/xtext/contextual-overrides/model-behavior.rcore"),
        Profile::NativeV3,
    )
    .unwrap();
    let old = include_str!("../../../../fixtures/xtext/contextual-overrides/base-example.kerml");
    let new = include_str!("../../../../fixtures/xtext/contextual-overrides/example.kerml");
    for (package, positive, negative) in [(&base, old, new), (&child, new, old)] {
        let result = evaluate(
            package,
            &[Source {
                uri: "context.kerml".into(),
                text: positive.into(),
            }],
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 2);
        assert_eq!(result.model.elements[0].class, "Model");
        assert_eq!(result.model.elements[1].values["name"], "Wheel");
        let rejected = evaluate(
            package,
            &[Source {
                uri: "context.kerml".into(),
                text: negative.into(),
            }],
        );
        assert!(!rejected.diagnostics.is_empty());
        assert!(rejected.model.elements.is_empty());
    }
    let replay = compiler::recompile(&child).unwrap();
    assert_eq!(
        serde_json::to_value(&child).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
}
#[test]
fn override_archive_replays_exact_shadowed_sources_and_profile() {
    use intersect_text::distribution::ordinary::{self as distribution, PackageMetadata};
    let export = distribution::build_bundle_with_profile(
        inputs(),
        PackageMetadata {
            package_id: "example.overrides".into(),
            release: "1".into(),
            display_name: "Contextual overrides".into(),
            description: String::new(),
        },
        Profile::NativeV3,
    )
    .unwrap();
    let mut files = export.files;
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&export.manifest).unwrap(),
    );
    let imported = distribution::import_files(files).unwrap();
    assert_eq!(
        serde_json::to_value(&export.package).unwrap(),
        serde_json::to_value(&imported.package).unwrap()
    );
    assert_eq!(
        imported.package.grammar.as_ref().unwrap().profile,
        Profile::NativeV3
    );
    assert_eq!(
        imported
            .package
            .grammar
            .as_ref()
            .unwrap()
            .shadowed_declarations["Member"][0]
            .source
            .uri,
        "base.xtext"
    );
    let mut altered = imported.package;
    altered
        .compilation
        .as_mut()
        .unwrap()
        .bundle
        .sources
        .iter_mut()
        .find(|source| source.id.uri == "base.xtext")
        .unwrap()
        .text
        .push_str("\n// changed shadowed source");
    assert!(compiler::recompile(&altered).is_err());
}
#[test]
fn completely_shadowed_parent_still_requires_its_exact_ecore_dependency() {
    let mut input = inputs();
    let middle = input
        .bundle
        .sources
        .iter_mut()
        .find(|source| Some(&source.id) == input.xtext.as_ref())
        .unwrap();
    middle.dependencies.retain(|id| *id != input.ecore);
    let mut top = DefinitionSource::new(
        "top.xtext",
        SourceKind::Xtext,
        CHILD
            .replace(
                "example.OverrideChild with example.OverrideBase",
                "example.OverrideTop with example.OverrideChild",
            )
            .replace("'class'", "'struct'"),
    );
    top.dependencies = vec![input.ecore.clone(), input.xtext.clone().unwrap()];
    input.xtext = Some(top.id.clone());
    input.bundle.sources.push(top);
    let errors = compiler::compile_bundle_with_profile(input, Profile::NativeV3).unwrap_err();
    assert!(
        errors.iter().any(|error| error.source == "model.xtext"
            && error.message.contains("exact Ecore dependency")),
        "{errors:?}"
    );
}
