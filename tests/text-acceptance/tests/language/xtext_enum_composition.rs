use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/enum-composition/model.ecore");
const BASE: &str = include_str!("../../../../fixtures/xtext/enum-composition/base.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/enum-composition/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/enum-composition/model-behavior.rcore");
fn input(ecore: &str, base: &str, child: &str, rcore_behavior: &str) -> BundleCompilation {
    let namespace =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ecore)
            .unwrap()
            .uri;
    let e = DefinitionSource::new("model.ecore", SourceKind::Ecore, ecore);
    let mut b = DefinitionSource::new("base.xtext", SourceKind::Xtext, base);
    b.dependencies.push(e.id.clone());
    let mut c = DefinitionSource::new("child.xtext", SourceKind::Xtext, child);
    c.dependencies = vec![e.id.clone(), b.id.clone()];
    let mut s = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        rcore_behavior,
    );
    s.dependencies.push(e.id.clone());
    let result = BundleCompilation {
        ecore: e.id.clone(),
        xtext: Some(c.id.clone()),
        rcore_behavior: s.id.clone(),
        namespace,
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![e, b, c, s],
            namespaces: vec![],
        },
    };
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
fn sources(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "typed.model".into(),
        text: text.into(),
    }]
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_override_preserves_nominal_values_and_source_identity() {
    for child in [CHILD.to_owned(), CHILD.replace("@Override ", "")] {
        let input = input(ECORE, BASE, &child, RCORE_BEHAVIOR);
        assert!(compiler::compile_bundle_with_profile(input.clone(), Profile::NativeV42).is_err());
        let base_id = input.bundle.sources[1].id.clone();
        let child_id = input.bundle.sources[2].id.clone();
        let package = compiler::compile_bundle_with_profile(input, Profile::NativeV43).unwrap();
        let grammar = package.grammar.as_ref().unwrap();
        assert_eq!(grammar.declaration_sources["Choice"].source, child_id);
        assert_eq!(grammar.shadowed_declarations["Choice"][0].source, base_id);
        for (text, literal) in [("color warm;", "Red"), ("color cool;", "Blue")] {
            let result = evaluate(&package, &sources(text));
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(result.model.elements.len(), 1);
            assert_eq!(result.model.elements[0].id, "typed.model#0");
            assert_eq!(
                (result.model.elements[0].start, result.model.elements[0].end),
                (0, text.len())
            );
            assert_eq!(
                evaluate_with_limits(
                    &package,
                    &sources(text),
                    EvaluationLimits {
                        elements_per_resource: Some(1),
                        ..EvaluationLimits::native()
                    }
                )
                .model,
                result.model
            );
            for limits in [
                EvaluationLimits {
                    elements_per_resource: Some(0),
                    ..EvaluationLimits::native()
                },
                EvaluationLimits {
                    steps_per_resource: Some(1),
                    ..EvaluationLimits::native()
                },
            ] {
                assert!(
                    !evaluate_with_limits(&package, &sources(text), limits)
                        .diagnostics
                        .is_empty()
                );
            }
            let expected = package.schema.enumerations["Shade"]
                .by_name(literal)
                .unwrap();
            let value = package.schema.enumerations["Shade"]
                .value_by_id(&expected.declaration)
                .unwrap();
            assert_eq!(
                result.model.elements[0].values["shade"],
                serde_json::to_value(value).unwrap()
            );
            result.model.validate(&package.schema).unwrap();
            assert_eq!(
                evaluate(&compiler::recompile(&package).unwrap(), &sources(text)).model,
                result.model
            );
        }
        for text in ["color red;", "color blue;", "color missing;"] {
            assert!(!evaluate(&package, &sources(text)).diagnostics.is_empty());
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_override_rejects_nominal_mismatch_final_and_orphan() {
    for (base, child) in [
        (BASE.to_owned(), CHILD.replace("p::Shade", "p::OtherShade")),
        (
            BASE.replace("enum Choice", "@Final enum Choice"),
            CHILD.to_owned(),
        ),
        (
            BASE.to_owned(),
            CHILD.replace("enum Choice", "enum Missing"),
        ),
    ] {
        assert!(
            compiler::compile_bundle_with_profile(
                input(ECORE, &base, &child, RCORE_BEHAVIOR),
                Profile::NativeV43
            )
            .is_err()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_parent_enum_retains_its_spelling_and_literal_identity() {
    let child = format!("{CHILD} @Override Model returns p::Model: 'old' shade=super::Choice ';';");
    let package = compiler::compile_bundle_with_profile(
        input(ECORE, BASE, &child, RCORE_BEHAVIOR),
        Profile::NativeV43,
    )
    .unwrap();
    for text in ["old red;", "old blue;"] {
        let result = evaluate(&package, &sources(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        result.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&compiler::recompile(&package).unwrap(), &sources(text)).model,
            result.model
        );
    }
    assert!(
        !evaluate(&package, &sources("old warm;"))
            .diagnostics
            .is_empty()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn declaration_only_children_inherit_entry_but_standalone_grammars_reject() {
    let header = CHILD.split("@Override").next().unwrap();
    let child = format!("{header} @Override terminal WS: (' '|'\\t')+;");
    for child in [child.clone(), child.replace("@Override ", "")] {
        let package = compiler::compile_bundle_with_profile(
            input(ECORE, BASE, &child, RCORE_BEHAVIOR),
            Profile::NativeV43,
        )
        .unwrap();
        assert_eq!(package.grammar.as_ref().unwrap().entry, "Model");
        let result = evaluate(&package, &sources("color red;"));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            evaluate(
                &compiler::recompile(&package).unwrap(),
                &sources("color red;")
            )
            .model,
            result.model
        );
    }
    let empty_error = compiler::compile_bundle_with_profile(
        input(ECORE, BASE, header, RCORE_BEHAVIOR),
        Profile::NativeV43,
    )
    .unwrap_err();
    assert!(format!("{empty_error:?}").contains("Grammar requires a rule"));
    let standalone = CHILD
        .replace(" with example.Colors", " hidden()")
        .replace("@Override ", "");
    let error = compiler::compile_bundle_with_profile(
        input(ECORE, BASE, &standalone, RCORE_BEHAVIOR),
        Profile::NativeV43,
    )
    .unwrap_err();
    assert!(
        format!("{error:?}").contains("Grammar requires a parser rule"),
        "{error:?}"
    );
}
