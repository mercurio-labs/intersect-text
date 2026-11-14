//! Declaration-library consumption without inventing an executable parent entry.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ENUM: &str = include_str!("../../../../fixtures/xtext/grammar-libraries/enum.xtext");
const DATATYPE: &str = include_str!("../../../../fixtures/xtext/grammar-libraries/datatype.xtext");
const FRAGMENT: &str = include_str!("../../../../fixtures/xtext/grammar-libraries/fragment.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/grammar-libraries/model.xtext");
fn input(parent: &str, child: &str, materialized: bool) -> BundleCompilation {
    let e = DefinitionSource::new(
        "colors.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/xtext/enum-composition/model.ecore"),
    );
    let mut p = DefinitionSource::new("library.xtext", SourceKind::Xtext, parent);
    p.dependencies.push(e.id.clone());
    let mut c = DefinitionSource::new("child.xtext", SourceKind::Xtext, child);
    c.dependencies = vec![e.id.clone(), p.id.clone()];
    let mut s = DefinitionSource::new(
        "colors-behavior.rcore",
        SourceKind::RcoreBehavior,
        include_str!("../../../../fixtures/xtext/enum-composition/model-behavior.rcore"),
    );
    s.dependencies.push(e.id.clone());
    let mut result = BundleCompilation {
        ecore: e.id.clone(),
        xtext: Some(c.id.clone()),
        rcore_behavior: s.id.clone(),
        namespace: "urn:intersect:xtext:enum-composition:1".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![e, p, c, s],
            namespaces: vec![],
        },
    };
    if materialized {
        result.materialized_types.push(
            intersect_model::definitions::generic_types::RuntimeType::new(
                intersect_model::definitions::generic_types::Type::Classifier {
                    declaration: intersect_model::definitions::resources::DeclarationId {
                        source: result.ecore.clone(),
                        path: vec![0],
                    },
                    arguments: vec![],
                    raw: false,
                },
            )
            .unwrap(),
        );
    }
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
fn sources(text: &str) -> [Source; 1] {
    [Source {
        uri: "library.model".into(),
        text: text.into(),
    }]
}
fn verify(compilation: BundleCompilation) {
    let exact = compilation.bundle.sources.clone();
    let package = compiler::compile_bundle_with_profile(compilation, Profile::NativeV51).unwrap();
    assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
    let grammar = package.grammar.as_ref().unwrap();
    assert_eq!(grammar.entry, "Model");
    assert_eq!(grammar.declaration_sources["Choice"].source, exact[1].id);
    let text = "color red;";
    let result = evaluate(&package, &sources(text));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    assert_eq!(result.model.elements[0].id, "library.model#0");
    assert_eq!(
        (result.model.elements[0].start, result.model.elements[0].end),
        (0, text.len())
    );
    let enumeration = package
        .schema
        .enumerations
        .values()
        .find(|e| e.name() == "Shade")
        .unwrap();
    let expected = enumeration
        .value_by_id(&enumeration.by_name("Red").unwrap().declaration)
        .unwrap();
    assert_eq!(
        result.model.elements[0].values["shade"],
        serde_json::to_value(expected).unwrap()
    );
    result.model.validate(&package.schema).unwrap();
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    assert_eq!(evaluate(&replay, &sources(text)).model, result.model);
    for text in ["color missing;", "color ?;", "color red"] {
        assert!(!evaluate(&package, &sources(text)).diagnostics.is_empty());
    }
    for limits in [
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            elements_per_resource: Some(0),
            ..EvaluationLimits::native()
        },
    ] {
        assert!(
            !evaluate_with_limits(&package, &sources(text), limits)
                .diagnostics
                .is_empty()
        );
    }
    assert_eq!(evaluate(&package, &sources(text)).model, result.model);
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_datatype_and_fragment_libraries_execute_in_child_context() {
    for parent in [ENUM, DATATYPE, FRAGMENT] {
        let child = if parent == FRAGMENT {
            CHILD.replace("shade=Choice ';'", "Tail")
        } else {
            CHILD.into()
        };
        for materialized in [false, true] {
            let compilation = input(parent, &child, materialized);
            assert!(
                compiler::compile_bundle_with_profile(compilation.clone(), Profile::NativeV50)
                    .is_err()
            );
            verify(compilation);
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn libraries_do_not_gain_standalone_executable_entries() {
    for parent in [ENUM, DATATYPE, FRAGMENT] {
        for materialized in [false, true] {
            let mut compilation = input(parent, CHILD, materialized);
            compilation.xtext = Some(compilation.bundle.sources[1].id.clone());
            let errors =
                compiler::compile_bundle_with_profile(compilation, Profile::NativeV51).unwrap_err();
            assert!(
                errors
                    .iter()
                    .any(|e| e.message.contains("requires a parser entry")
                        || e.message
                            .contains("Scalar parser entry execution is unsupported")),
                "{errors:?}"
            );
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn invalid_shadowed_library_declarations_still_reject() {
    for (parent, replacement, expected) in [
        (
            DATATYPE.replace("'red' | 'blue'", "('red')?"),
            "@Override Choice returns p::Shade: 'red';",
            "Nullable datatype",
        ),
        (
            ENUM.replace("Red='red'", "Missing='red'"),
            "@Override enum Choice returns p::Shade: Red='red';",
            "Missing",
        ),
        (
            FRAGMENT.replace("shade=Choice ';'", "Tail"),
            "@Override fragment Tail returns p::Model: shade=Choice ';';",
            "Left recursion through Tail",
        ),
    ] {
        for materialized in [false, true] {
            let child = format!("{CHILD}\n{replacement}");
            let errors = compiler::compile_bundle_with_profile(
                input(&parent, &child, materialized),
                Profile::NativeV51,
            )
            .unwrap_err();
            assert!(
                errors.iter().any(|e| e.source == "library.xtext"
                    && e.message.to_lowercase().contains(&expected.to_lowercase())),
                "{errors:?}"
            );
        }
    }
}
