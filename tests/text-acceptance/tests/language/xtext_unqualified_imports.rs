//! Explicit TypeRef scope; shared native/WASM controls, not full X02 closure.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};

const ECORE: &str = include_str!("../../../../fixtures/xtext/unqualified-imports/model.ecore");
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/unqualified-imports/model.xtext");
fn input(ecore: &str, grammar: &str, materialized: bool, child: Option<&str>) -> BundleCompilation {
    let ecore = DefinitionSource::new("unqualified.ecore", SourceKind::Ecore, ecore);
    let mut xtext = DefinitionSource::new("unqualified.xtext", SourceKind::Xtext, grammar);
    xtext.dependencies.push(ecore.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "unqualified-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior.dependencies.push(ecore.id.clone());
    let mut result = BundleCompilation {
        ecore: ecore.id.clone(),
        xtext: Some(xtext.id.clone()),
        rcore_behavior: rcore_behavior.id.clone(),
        namespace: "urn:unqualified:main".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, xtext, rcore_behavior],
            namespaces: vec![],
        },
    };
    if materialized {
        result.materialized_types.push(
            intersect_model::definitions::generic_types::RuntimeType::new(
                intersect_model::definitions::generic_types::Type::Classifier {
                    declaration: intersect_model::definitions::resources::DeclarationId {
                        source: result.ecore.clone(),
                        path: if grammar.contains("returns Other") {
                            vec![2, 0]
                        } else {
                            vec![0]
                        },
                    },
                    arguments: vec![],
                    raw: false,
                },
            )
            .unwrap(),
        );
    }
    if let Some(text) = child {
        let mut source = DefinitionSource::new("child.xtext", SourceKind::Xtext, text);
        source.dependencies = vec![result.ecore.clone(), result.xtext.clone().unwrap()];
        result.xtext = Some(source.id.clone());
        result.bundle.sources.push(source);
    }
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
fn verify(profile: Profile, input: BundleCompilation, text: &str) {
    let expected_path = if input.bundle.sources[1].text.contains("returns Other") {
        vec![2, 0]
    } else {
        vec![0]
    };
    let expected_source = input.ecore.clone();
    let exact = input.bundle.sources.clone();
    let package = compiler::compile_bundle_with_profile(input, profile).unwrap();
    assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
    let sources = [Source {
        uri: "unqualified.model".into(),
        text: text.into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    assert_eq!(result.model.elements[0].id, "unqualified.model#0");
    let declaration = &package.schema.declarations[&result.model.elements[0].class];
    assert_eq!(declaration.source, expected_source);
    assert_eq!(declaration.path, expected_path);
    assert_eq!(result.model.elements[0].values["name"], "valid");
    if text.contains(" red") {
        let enumeration = package
            .schema
            .enumerations
            .values()
            .find(|enumeration| enumeration.name() == "Shade")
            .unwrap();
        assert_eq!(
            enumeration.declaration().source,
            package.compilation.as_ref().unwrap().ecore
        );
        assert_eq!(enumeration.declaration().path, vec![1]);
        let literal = enumeration.by_name("Red").unwrap();
        assert_eq!(
            result.model.elements[0].values["shade"],
            serde_json::to_value(enumeration.value_by_id(&literal.declaration).unwrap()).unwrap()
        );
    }
    if text.contains(" ref ") {
        assert_eq!(result.references.len(), 1);
        assert_eq!(result.references[0].target, "unqualified.model#0");
    }
    assert_eq!(
        (result.model.elements[0].start, result.model.elements[0].end),
        (0, text.len())
    );
    result.model.validate(&package.schema).unwrap();
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    assert_eq!(evaluate(&replay, &sources).model, result.model);
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
            !evaluate_with_limits(&package, &sources, limits)
                .diagnostics
                .is_empty()
        );
    }
    assert!(
        !evaluate(
            &package,
            &[Source {
                uri: "bad.model".into(),
                text: "model ?".into()
            }]
        )
        .diagnostics
        .is_empty()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_types_resolve_across_local_aliased_imports() {
    explicit_types_resolve_across_local_aliased_imports_with_profile(Profile::NativeV45);
}
fn explicit_types_resolve_across_local_aliased_imports_with_profile(profile: Profile) {
    let text =
        include_str!("../../../../fixtures/xtext/unqualified-imports/example.model").trim_end();
    for materialized in [false, true] {
        verify(profile, input(ECORE, GRAMMAR, materialized, None), text);
        let qualified = GRAMMAR
            .replace("returns Model", "returns p::Model")
            .replace("{Model}", "{p::Model}")
            .replace("[Model|", "[p::Model|")
            .replace("returns Shade", "returns p::Shade")
            .replace("returns EString", "returns ecore::EString");
        verify(profile, input(ECORE, &qualified, materialized, None), text);
        verify(
            profile,
            input(
                &ECORE.replace("name=\"Other\"", "name=\"Model\""),
                &qualified,
                materialized,
                None,
            ),
            text,
        );
        verify(
            profile,
            input(
                ECORE,
                &GRAMMAR.replace("Model returns Model:", "Model:"),
                materialized,
                None,
            ),
            text,
        );
    }
    let other = GRAMMAR.replace(
        "Model returns Model: {Model} 'model' name=Name shade=Choice ('ref' target=[Model|Name])?;",
        "Model returns Other: 'model' name=Name;",
    );
    verify(profile, input(ECORE, &other, false, None), "model valid");
    let other_only = other
        .replace("import \"urn:unqualified:main\" as p\n", "")
        .replace("enum Choice returns Shade: Red='red' | Blue='blue';\n", "");
    for materialized in [false, true] {
        verify(
            profile,
            input(ECORE, &other_only, materialized, None),
            "model valid",
        );
    }
    assert!(
        compiler::compile_bundle_with_profile(
            input(ECORE, GRAMMAR, false, None),
            Profile::NativeV44
        )
        .is_err()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn ambiguous_and_missing_unqualified_types_reject() {
    ambiguous_and_missing_unqualified_types_reject_with_profile(Profile::NativeV45);
}
fn ambiguous_and_missing_unqualified_types_reject_with_profile(profile: Profile) {
    for materialized in [false, true] {
        for (ecore, grammar, message) in [
            (
                ECORE.to_owned(),
                GRAMMAR
                    .replace("import \"urn:unqualified:main\" as p\n", "")
                    .replace("Model returns Model:", "Model:"),
                "Inferred returns require an explicit selected EPackage import",
            ),
            (
                ECORE.replace("name=\"Other\"", "name=\"Model\""),
                GRAMMAR.to_owned(),
                "Ambiguous unqualified classifier Model",
            ),
            (
                ECORE.replace("name=\"Other\"", "name=\"Model\""),
                GRAMMAR.replace(
                    "import \"urn:unqualified:main\" as p",
                    "import \"urn:unqualified:main\"",
                ),
                "Ambiguous unqualified classifier Model",
            ),
            (
                ECORE.to_owned(),
                GRAMMAR.replace("returns Model", "returns Missing"),
                "Unknown unqualified classifier Missing",
            ),
            (
                ECORE.to_owned(),
                GRAMMAR.replace("import \"urn:unqualified:main\" as p\n", ""),
                "Unknown unqualified classifier Model",
            ),
            (
                ECORE.to_owned(),
                GRAMMAR.replace("returns Shade", "returns Missing"),
                "Unknown unqualified classifier Missing",
            ),
            (
                ECORE.replace("name=\"Other\"", "name=\"Model\""),
                GRAMMAR
                    .replace("as p", "as shared")
                    .replace("as q", "as shared")
                    .replace("returns Model", "returns shared::Model"),
                "Ambiguous classifier shared::Model",
            ),
            (
                ECORE.to_owned(),
                GRAMMAR.replace("{Model}", "{Missing}"),
                "Unknown unqualified classifier Missing",
            ),
            (
                ECORE.to_owned(),
                GRAMMAR.replace("[Model|", "[Missing|"),
                "Unknown unqualified classifier Missing",
            ),
        ] {
            let errors = compiler::compile_bundle_with_profile(
                input(&ecore, &grammar, materialized, None),
                profile,
            )
            .unwrap_err();
            assert!(
                errors.iter().any(|error| error.message.contains(message)),
                "{errors:?}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn inherited_rules_keep_scope_but_child_types_require_local_imports() {
    inherited_rules_keep_scope_but_child_types_require_local_imports_with_profile(
        Profile::NativeV45,
    );
}
fn inherited_rules_keep_scope_but_child_types_require_local_imports_with_profile(profile: Profile) {
    let child = "grammar example.Child with example.Unqualified\nimport \"urn:unqualified:main\" as local\n@Override Model returns Model: 'child' name=Name shade=Choice;\n";
    for materialized in [false, true] {
        verify(
            profile,
            input(ECORE, GRAMMAR, materialized, Some(child)),
            "child valid red",
        );
        verify(
            profile,
            input(
                ECORE,
                GRAMMAR,
                materialized,
                Some(
                    "grammar example.Child with example.Unqualified\n@Override terminal WS: ' '+;\n",
                ),
            ),
            "model valid red ref valid",
        );
        for bad in [
            child.replace("import \"urn:unqualified:main\" as local\n", ""),
            child.replace("returns Model", "returns p::Model"),
        ] {
            let errors = compiler::compile_bundle_with_profile(
                input(ECORE, GRAMMAR, materialized, Some(&bad)),
                profile,
            )
            .unwrap_err();
            assert!(
                errors.iter().any(|error| error
                    .message
                    .contains("Unknown unqualified classifier Model")
                    || error.message.contains("Unknown metamodel alias p")),
                "{errors:?}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn multiple_parent_syntax_is_retained_but_native_admission_rejects() {
    let child = "grammar example.Child with example.Unqualified, example.Second\nimport \"urn:unqualified:main\" as local\n@Override terminal WS: ' '+;\n";
    for materialized in [false, true] {
        let compilation = input(ECORE, GRAMMAR, materialized, Some(child));
        let source = compilation
            .bundle
            .source(compilation.xtext.as_ref().unwrap())
            .unwrap();
        let syntax = intersect_text::grammar::syntax::parse(source, Default::default()).unwrap();
        assert_eq!(syntax.source, source.id);
        assert_eq!(syntax.parents.len(), 2);
        for (parent, expected) in syntax
            .parents
            .iter()
            .zip(["example.Unqualified", "example.Second"])
        {
            assert_eq!(parent.value, expected);
            assert_eq!(&source.text[parent.start..parent.end], expected);
        }
        for profile in [Profile::NativeV44, Profile::NativeV45, Profile::NativeV51] {
            let errors =
                compiler::compile_bundle_with_profile(compilation.clone(), profile).unwrap_err();
            assert!(
                errors.iter().any(|error| error
                    .message
                    .contains("Multiple grammar inheritance is unsupported")),
                "{errors:?}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    explicit_types_resolve_across_local_aliased_imports_with_profile(Profile::NativeV51);
    ambiguous_and_missing_unqualified_types_reject_with_profile(Profile::NativeV51);
    inherited_rules_keep_scope_but_child_types_require_local_imports_with_profile(
        Profile::NativeV51,
    );
}
