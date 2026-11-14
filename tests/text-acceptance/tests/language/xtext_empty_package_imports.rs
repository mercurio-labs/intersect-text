//! Shared controls for historical rejection and checked package import admission.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::resources::ResourceSet;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;

fn input(imported: &str) -> BundleCompilation {
    let ecore = DefinitionSource::new(
        "packages.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/xtext/empty-package-imports/packages.ecore"),
    );
    let mut xtext = DefinitionSource::new(
        "model.xtext",
        SourceKind::Xtext,
        format!(
            r#"grammar example.EmptyPackage hidden(WS)
import "urn:main" as p
{imported}
Model returns p::Model: 'model' name=ID;
terminal ID: ('a'..'z')+;
terminal WS: ' '+;
"#
        ),
    );
    xtext.dependencies.push(ecore.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        include_str!("../../../../fixtures/xtext/empty-package-imports/model-behavior.rcore"),
    );
    rcore_behavior.dependencies.push(ecore.id.clone());
    let result = BundleCompilation {
        ecore: ecore.id.clone(),
        xtext: Some(xtext.id.clone()),
        rcore_behavior: rcore_behavior.id.clone(),
        namespace: "urn:main".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, xtext, rcore_behavior],
            namespaces: vec![],
        },
    };
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn empty_package_identity_is_retained_but_not_admitted_as_a_grammar_import() {
    let compilation = input("import \"urn:empty\" as empty");
    assert_eq!(
        compilation.bundle.sources[1].text,
        include_str!("../../../../fixtures/xtext/empty-package-imports/model.xtext")
    );
    assert_eq!(
        compilation.bundle.sources[2].text,
        include_str!("../../../../fixtures/xtext/empty-package-imports/model-behavior.rcore")
    );
    let resources = ResourceSet::new(&compilation.bundle).unwrap();
    let packages = resources.packages(&compilation.ecore).unwrap();
    assert_eq!(packages.len(), 2);
    assert_eq!(packages["urn:empty"].source, compilation.ecore);
    assert_eq!(packages["urn:empty"].path, vec![1]);
    let schema = intersect_model::definitions::compiler::compile_structure_bundle(
        &compilation.bundle,
        &compilation.ecore,
        "urn:main",
    )
    .unwrap();
    assert!(
        schema
            .declarations
            .values()
            .all(|declaration| declaration.namespace != "urn:empty")
    );
    let errors =
        compiler::compile_bundle_with_profile(compilation, Profile::NativeV43).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("Grammar import must match")),
        "{errors:?}"
    );
    assert!(compiler::compile_bundle_with_profile(input(""), Profile::NativeV43).is_ok());
    let missing = compiler::compile_bundle_with_profile(
        input("import \"urn:missing\" as absent"),
        Profile::NativeV43,
    )
    .unwrap_err();
    assert!(
        missing
            .iter()
            .any(|error| error.message.contains("Grammar import must match"))
    );
}

fn materialized(mut input: BundleCompilation, selected: bool) -> BundleCompilation {
    if selected {
        input.materialized_types.push(
            intersect_model::definitions::generic_types::RuntimeType::new(
                intersect_model::definitions::generic_types::Type::Classifier {
                    declaration: intersect_model::definitions::resources::DeclarationId {
                        source: input.ecore.clone(),
                        path: vec![0],
                    },
                    arguments: vec![],
                    raw: false,
                },
            )
            .unwrap(),
        );
    }
    input
}
fn external_input(reachable: bool) -> BundleCompilation {
    let mut input = input("import \"urn:external\" as unused");
    let empty = DefinitionSource::new(
        "empty.rcore",
        SourceKind::Rcore,
        r#"rcore 1; package externalDefinitions version "1"; metamodel "urn:external" prefix empty {}"#,
    );
    if reachable {
        input.bundle.sources[0].dependencies.push(empty.id.clone());
    }
    input.bundle.sources[1].dependencies.push(empty.id.clone());
    input.bundle.sources.push(empty);
    input
}
fn duplicate_input(bound: bool) -> BundleCompilation {
    let mut input = external_input(true);
    let second = DefinitionSource::new(
        "second-empty.rcore",
        SourceKind::Rcore,
        input.bundle.sources[3].text.clone(),
    );
    input.bundle.sources[0].dependencies.push(second.id.clone());
    input.bundle.sources[1].dependencies.push(second.id.clone());
    if bound {
        input
            .bundle
            .namespaces
            .push(intersect_model::definitions::bundle::NamespaceBinding {
                namespace: "urn:external".into(),
                source: second.id.clone(),
            });
    }
    input.bundle.sources.push(second);
    input
}
fn common_terminals_input() -> BundleCompilation {
    let mut input = input("");
    let old = &input.bundle.sources[1];
    let text = old.text.split("terminal ID:").next().unwrap().replace(
        "grammar example.EmptyPackage hidden(WS)",
        "grammar example.EmptyPackage with org.eclipse.xtext.common.Terminals",
    );
    let mut grammar = DefinitionSource::new(&old.id.uri, SourceKind::Xtext, text);
    grammar.dependencies = old.dependencies.clone();
    input.xtext = Some(grammar.id.clone());
    input.bundle.sources[1] = grammar;
    input
}
fn inherited_input() -> BundleCompilation {
    let mut input = input("import \"urn:empty\" as unused");
    let mut child = DefinitionSource::new(
        "child.xtext",
        SourceKind::Xtext,
        r#"grammar example.Child with example.EmptyPackage
import "urn:main" as p
import "urn:empty" as empty
@Override terminal WS: ' '+;
"#,
    );
    child.dependencies = vec![input.ecore.clone(), input.xtext.clone().unwrap()];
    input.xtext = Some(child.id.clone());
    input.bundle.sources.push(child);
    input
}
fn missing_classifier_input() -> BundleCompilation {
    let mut input = input("import \"urn:empty\" as empty");
    let source = &input.bundle.sources[1];
    let mut xtext = DefinitionSource::new(
        &source.id.uri,
        SourceKind::Xtext,
        source.text.replace("p::Model", "empty::Missing"),
    );
    xtext.dependencies = source.dependencies.clone();
    input.xtext = Some(xtext.id.clone());
    input.bundle.sources[1] = xtext;
    input
}

// Characterization, not qualification: NativeV44 does not implement the local
// package scope used by XtextScopeProvider for explicit unqualified TypeRefs.
// Keep these historical observations when a later profile fixes the scope.
#[test]
fn native_v44_unqualified_type_scope_gap_is_explicit() {
    fn changed(
        mut compilation: BundleCompilation,
        rcore: String,
        grammar: String,
    ) -> BundleCompilation {
        let previous = compilation.ecore.clone();
        let source = DefinitionSource::new(&previous.uri, SourceKind::Rcore, rcore);
        compilation.ecore = source.id.clone();
        compilation.bundle.sources[0] = source;
        for source in &mut compilation.bundle.sources {
            for dependency in &mut source.dependencies {
                if *dependency == previous {
                    *dependency = compilation.ecore.clone();
                }
            }
        }
        let mut source = DefinitionSource::new("model.xtext", SourceKind::Xtext, grammar);
        source.dependencies = compilation.bundle.sources[1].dependencies.clone();
        compilation.xtext = Some(source.id.clone());
        compilation.bundle.sources[1] = source;
        compilation
    }
    let original = input("import \"urn:empty\" as other");
    let grammar = original.bundle.sources[1].text.replace("p::Model", "Model");
    // The migrated Rcore quotes names only where needed since eeb500c4.
    let rcore = original.bundle.sources[0].text.replace(
        "package empty namespace \"urn:empty\" prefix empty {\n}",
        "package empty namespace \"urn:empty\" prefix empty { class Model {} }",
    );
    assert_ne!(rcore, original.bundle.sources[0].text);
    // Upstream would reject the two local Model candidates; V44 searches only
    // the unaliased package and overlooks the second candidate.
    let ambiguous = compiler::compile_bundle_with_profile(
        changed(
            original.clone(),
            rcore,
            grammar.replace("import \"urn:main\" as p", "import \"urn:main\""),
        ),
        Profile::NativeV44,
    );
    assert!(ambiguous.is_ok(), "{ambiguous:?}");

    // Missing local imports already reject; preserve this control.
    let no_imports = grammar
        .replace("import \"urn:main\" as p\n", "")
        .replace("import \"urn:empty\" as other\n", "");
    assert!(
        compiler::compile_bundle_with_profile(
            changed(
                original.clone(),
                original.bundle.sources[0].text.clone(),
                no_imports
            ),
            Profile::NativeV44,
        )
        .is_err()
    );

    // With only the aliased secondary package defining Other, upstream finds
    // it by unqualified name. V44 incorrectly searches only the empty default
    // alias scope, ignoring all the aliased imports.
    let rcore = original.bundle.sources[0].text.replace(
        "package empty namespace \"urn:empty\" prefix empty {\n}",
        "package empty namespace \"urn:empty\" prefix empty { class Other { attr name: String [0,1]; } }",
    );
    assert_ne!(rcore, original.bundle.sources[0].text);
    let errors = compiler::compile_bundle_with_profile(
        changed(
            original,
            rcore,
            grammar.replace("returns Model", "returns Other"),
        ),
        Profile::NativeV44,
    )
    .unwrap_err();
    assert!(
        errors.iter().any(|error| error.message.contains("Other")),
        "{errors:?}"
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn checked_empty_imports_execute_and_replay_in_both_compiler_paths() {
    checked_empty_imports_execute_and_replay_in_both_compiler_paths_with_profile(
        Profile::NativeV44,
    );
}
fn checked_empty_imports_execute_and_replay_in_both_compiler_paths_with_profile(profile: Profile) {
    use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
    for selected in [false, true] {
        for input in [
            input("import \"urn:empty\" as empty"),
            input("import \"urn:empty\""),
            external_input(true),
            inherited_input(),
            duplicate_input(true),
        ] {
            let input = materialized(input, selected);
            let exact_sources = input.bundle.sources.clone();
            let package = compiler::compile_bundle_with_profile(input, profile).unwrap();
            assert_eq!(
                package.compilation.as_ref().unwrap().bundle.sources,
                exact_sources
            );
            let sources = [Source {
                uri: "empty.model".into(),
                text: include_str!(
                    "../../../../fixtures/xtext/empty-package-imports/example.model"
                )
                .into(),
            }];
            let result = evaluate(&package, &sources);
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(result.model.elements.len(), 1);
            assert_eq!(result.model.elements[0].id, "empty.model#0");
            assert_eq!(result.model.elements[0].values["name"], "valid");
            assert_eq!(
                (result.model.elements[0].start, result.model.elements[0].end),
                (0, 11)
            );
            result.model.validate(&package.schema).unwrap();
            let replay = compiler::recompile(&package).unwrap();
            assert_eq!(
                serde_json::to_value(&replay).unwrap(),
                serde_json::to_value(&package).unwrap()
            );
            assert_eq!(evaluate(&replay, &sources).model, result.model);
            assert!(
                !evaluate_with_limits(
                    &package,
                    &sources,
                    EvaluationLimits {
                        elements_per_resource: Some(0),
                        ..EvaluationLimits::native()
                    }
                )
                .diagnostics
                .is_empty()
            );
            assert!(
                !evaluate_with_limits(
                    &package,
                    &sources,
                    EvaluationLimits {
                        steps_per_resource: Some(1),
                        ..EvaluationLimits::native()
                    }
                )
                .diagnostics
                .is_empty()
            );
            assert!(
                !evaluate(
                    &package,
                    &[Source {
                        uri: "empty.model".into(),
                        text: "model".into()
                    }]
                )
                .diagnostics
                .is_empty()
            );
            assert_eq!(evaluate(&package, &sources).model, result.model);
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn empty_imports_do_not_authorize_missing_duplicate_unreachable_or_stale_sources() {
    empty_imports_do_not_authorize_missing_duplicate_unreachable_or_stale_sources_with_profile(
        Profile::NativeV44,
    );
}
fn empty_imports_do_not_authorize_missing_duplicate_unreachable_or_stale_sources_with_profile(
    profile: Profile,
) {
    for profile in [Profile::NativeV43, Profile::NativeV44] {
        let errors =
            compiler::compile_bundle_with_profile(common_terminals_input(), profile).unwrap_err();
        assert!(
            errors.iter().any(|error| error
                .message
                .contains("Predictive lexical profile requires an explicit hidden policy")),
            "{errors:?}"
        );
    }

    for selected in [false, true] {
        for input in [
            input("import \"urn:missing\" as absent"),
            input("import \"urn:empty\" as a import \"urn:empty\" as b"),
            external_input(false),
            missing_classifier_input(),
            duplicate_input(false),
        ] {
            assert!(
                compiler::compile_bundle_with_profile(materialized(input, selected), profile)
                    .is_err()
            );
        }
        let mut stale = external_input(true);
        stale.bundle.sources.last_mut().unwrap().text.push(' ');
        assert!(
            compiler::compile_bundle_with_profile(materialized(stale, selected), profile).is_err()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    checked_empty_imports_execute_and_replay_in_both_compiler_paths_with_profile(
        Profile::NativeV51,
    );
    empty_imports_do_not_authorize_missing_duplicate_unreachable_or_stale_sources_with_profile(
        Profile::NativeV51,
    );
}
