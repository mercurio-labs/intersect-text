//! Exact terminal-only parent source composition, shared native/WASM controls.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const PARENT: &str = include_str!("../../../../fixtures/xtext/terminal-parents/Terminals.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/terminal-parents/model.xtext");
fn input(parent: &str, child: &str, materialized: bool) -> BundleCompilation {
    let e = DefinitionSource::new(
        "model.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/xtext/terminal-spellings/model.ecore"),
    );
    let mut p = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        parent,
    );
    p.dependencies.push(e.id.clone());
    let mut c = DefinitionSource::new("child.xtext", SourceKind::Xtext, child);
    c.dependencies = vec![e.id.clone(), p.id.clone()];
    let mut s = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    s.dependencies.push(e.id.clone());
    let mut result = BundleCompilation {
        ecore: e.id.clone(),
        xtext: Some(c.id.clone()),
        rcore_behavior: s.id.clone(),
        namespace: "urn:intersect:xtext:mini-kerml:terminal-spellings:1".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![e, p, c, s],
            namespaces: vec![],
        },
    };
    if materialized {
        for index in 0..4 {
            result.materialized_types.push(
                intersect_model::definitions::generic_types::RuntimeType::new(
                    intersect_model::definitions::generic_types::Type::Classifier {
                        declaration: intersect_model::definitions::resources::DeclarationId {
                            source: result.ecore.clone(),
                            path: vec![index],
                        },
                        arguments: vec![],
                        raw: false,
                    },
                )
                .unwrap(),
            );
        }
    }
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
fn sources(text: &str) -> [Source; 1] {
    [Source {
        uri: "terminal.model".into(),
        text: text.into(),
    }]
}
fn verify(profile: Profile, compilation: BundleCompilation, text: &str) {
    let exact = compilation.bundle.sources.clone();
    let package = compiler::compile_bundle_with_profile(compilation.clone(), profile).unwrap();
    assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
    let grammar = package.grammar.as_ref().unwrap();
    assert_eq!(grammar.entry, "Model");
    assert_eq!(grammar.declaration_sources["ID"].source, exact[1].id);
    assert_eq!(
        exact[1].id.sha256,
        "3e3d37770a731b0d5270b456743fc70331aa7aaf1c99afa6728896c64bbe401a"
    );
    let result = evaluate(&package, &sources(text));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    assert_eq!(result.model.elements[0].id, "terminal.model#0");
    assert_eq!(result.model.elements[0].values["name"], "Alpha");
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
    assert_eq!(evaluate(&replay, &sources(text)).model, result.model);
    for invalid in ["model 123;", "model ?;", "model Alpha", "model /* unclosed"] {
        assert!(
            !evaluate(&package, &sources(invalid)).diagnostics.is_empty(),
            "{invalid}"
        );
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
    let mut schema = package.schema.clone();
    let error = grammar::compile_sources_with_limits(
        &compilation.bundle,
        compilation.xtext.as_ref().unwrap(),
        &mut schema,
        profile,
        SourceLimits {
            declarations: 7,
            ..SourceLimits::default()
        },
    )
    .unwrap_err();
    assert!(error.message.contains("budget"), "{error:?}");
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn pinned_terminal_parent_executes_with_exact_identity() {
    pinned_terminal_parent_executes_with_exact_identity_with_profile(Profile::NativeV50);
}
fn pinned_terminal_parent_executes_with_exact_identity_with_profile(profile: Profile) {
    for materialized in [false, true] {
        let compilation = input(PARENT, CHILD, materialized);
        assert!(
            compiler::compile_bundle_with_profile(compilation.clone(), Profile::NativeV49).is_err()
        );
        verify(profile, compilation, "model /* note */ Alpha;");
        // Pinned STRINGValueConverter removes quotes; V50 currently retains them.
        // Imported/consumed STRING is not semantic converter qualification.
        let string_package = compiler::compile_bundle_with_profile(
            input(
                PARENT,
                &CHILD.replace("name=ID", "name=STRING"),
                materialized,
            ),
            profile,
        )
        .unwrap();
        let string_result = evaluate(&string_package, &sources("model \"Alpha\";"));
        assert!(string_result.diagnostics.is_empty());
        assert_eq!(string_result.model.elements[0].values["name"], "\"Alpha\"");
        assert_ne!(
            string_result.model.elements[0].values["name"], "Alpha",
            "Known X06 converter gap; do not count as compatibility"
        );
        verify(
            profile,
            input(
                PARENT,
                &format!("{CHILD}\n@Override terminal WS: (' '|'~')+;"),
                materialized,
            ),
            "model~Alpha;",
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn terminal_parent_can_pass_through_an_intermediate_grammar() {
    terminal_parent_can_pass_through_an_intermediate_grammar_with_profile(Profile::NativeV50);
}
fn terminal_parent_can_pass_through_an_intermediate_grammar_with_profile(profile: Profile) {
    for materialized in [false, true] {
        let mut compilation = input(
            PARENT,
            &CHILD.replace(
                "with org.eclipse.xtext.common.Terminals",
                "with example.Middle",
            ),
            materialized,
        );
        let mut middle = DefinitionSource::new(
            "middle.xtext",
            SourceKind::Xtext,
            "grammar example.Middle with org.eclipse.xtext.common.Terminals terminal MARK: '@';",
        );
        middle.dependencies = vec![
            compilation.ecore.clone(),
            compilation.bundle.sources[1].id.clone(),
        ];
        compilation.bundle.sources[2]
            .dependencies
            .push(middle.id.clone());
        compilation.bundle.sources.push(middle);
        verify(profile, compilation, "model Alpha;");
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn terminal_parent_does_not_hide_invalid_or_unexecutable_sources() {
    terminal_parent_does_not_hide_invalid_or_unexecutable_sources_with_profile(Profile::NativeV50);
}
fn terminal_parent_does_not_hide_invalid_or_unexecutable_sources_with_profile(profile: Profile) {
    for materialized in [false, true] {
        let mut standalone = input(PARENT, CHILD, materialized);
        standalone.xtext = Some(standalone.bundle.sources[1].id.clone());
        let package =
            compiler::compile_bundle_with_profile(input(PARENT, CHILD, materialized), profile)
                .unwrap();
        let mut schema = package.schema.clone();
        let error = grammar::compile_sources(
            &standalone.bundle,
            standalone.xtext.as_ref().unwrap(),
            &mut schema,
            profile,
        )
        .unwrap_err();
        assert!(
            error.message.contains("requires a parser entry"),
            "{error:?}"
        );
        let errors = compiler::compile_bundle_with_profile(standalone, profile).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("requires a parser entry")
                    || e.message.contains("requires a parser rule")),
            "{errors:?}"
        );
        let bad = PARENT.replace("'a'..'z'", "'z'..'a'");
        let replacement = format!("{CHILD}\n@Override terminal ID: 'Alpha';");
        let compilation = input(&bad, &replacement, materialized);
        let parent = compilation.bundle.sources[1].id.clone();
        let errors = compiler::compile_bundle_with_profile(compilation, profile).unwrap_err();
        assert!(
            errors.iter().any(|e| e.source == parent.uri
                && e.message
                    .contains("Invalid UTF-16 terminal character range")),
            "{errors:?}"
        );
        let mut missing = input(PARENT, CHILD, materialized);
        missing.bundle.sources[2]
            .dependencies
            .retain(|id| id == &missing.ecore);
        assert!(
            compiler::compile_bundle_with_profile(missing, profile)
                .unwrap_err()
                .iter()
                .any(|e| e.message.contains("Missing exact parent"))
        );
        let mut stale = input(PARENT, CHILD, materialized);
        stale.bundle.sources[1].text.push(' ');
        assert!(compiler::compile_bundle_with_profile(stale, profile).is_err());
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    pinned_terminal_parent_executes_with_exact_identity_with_profile(Profile::NativeV51);
    terminal_parent_can_pass_through_an_intermediate_grammar_with_profile(Profile::NativeV51);
    terminal_parent_does_not_hide_invalid_or_unexecutable_sources_with_profile(Profile::NativeV51);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn exact_parent_dependencies_reject_ambiguity_and_cycles_without_schema_changes() {
    for materialized in [false, true] {
        let valid = input(PARENT, CHILD, materialized);
        let package =
            compiler::compile_bundle_with_profile(valid.clone(), Profile::NativeV51).unwrap();
        let mut ambiguous = valid.clone();
        let mut other = DefinitionSource::new("other-parent.xtext", SourceKind::Xtext, PARENT);
        other.dependencies.push(valid.ecore.clone());
        ambiguous.bundle.sources[2]
            .dependencies
            .push(other.id.clone());
        ambiguous.bundle.sources.push(other);

        let mut cyclic = valid.clone();
        let mut parent = DefinitionSource::new(
            &valid.bundle.sources[1].id.uri,
            SourceKind::Xtext,
            PARENT.replace(
                "grammar org.eclipse.xtext.common.Terminals",
                "grammar org.eclipse.xtext.common.Terminals with example.TerminalChild",
            ),
        );
        parent.dependencies = vec![valid.ecore.clone(), valid.xtext.clone().unwrap()];
        cyclic.bundle.sources[2].dependencies = vec![valid.ecore.clone(), parent.id.clone()];
        cyclic.bundle.sources[1] = parent;

        for (compilation, expected) in [
            (ambiguous, "Ambiguous parent grammar identity"),
            (cyclic, "Cyclic"),
        ] {
            let mut schema = package.schema.clone();
            let before = serde_json::to_value(&schema).unwrap();
            let error = grammar::compile_sources(
                &compilation.bundle,
                compilation.xtext.as_ref().unwrap(),
                &mut schema,
                Profile::NativeV51,
            )
            .unwrap_err();
            assert!(error.message.contains(expected), "{error:?}");
            assert!(
                compilation
                    .bundle
                    .sources
                    .iter()
                    .any(|s| s.id.uri == error.source)
            );
            assert_eq!(serde_json::to_value(&schema).unwrap(), before);
            assert!(
                compiler::compile_bundle_with_profile(compilation, Profile::NativeV51).is_err()
            );
        }

        // A second revision is available, but the child's exact dependency
        // selects only the original pinned bytes, not the newest revision.
        let mut revised = valid;
        let mut unused = DefinitionSource::new(
            &revised.bundle.sources[1].id.uri,
            SourceKind::Xtext,
            format!("{PARENT}\n"),
        );
        unused.dependencies.push(revised.ecore.clone());
        revised.bundle.sources.push(unused);
        verify(Profile::NativeV51, revised, "model Alpha;");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn parent_dependency_diagnostics_select_the_exact_reference_bytes() {
    for prefix in ["", "// π😀\r\n"] {
        for spelling in [
            "org.eclipse.xtext.common.Terminals",
            "org /* parent */ . eclipse . xtext . common . Terminals",
        ] {
            let child = format!(
                "{prefix}{}",
                CHILD.replace("org.eclipse.xtext.common.Terminals", spelling)
            );
            let start = child.find(spelling).unwrap();
            for materialized in [false, true] {
                let valid = input(PARENT, &child, materialized);
                let package =
                    compiler::compile_bundle_with_profile(valid.clone(), Profile::NativeV51)
                        .unwrap();
                let mut missing = valid.clone();
                missing.bundle.sources[2]
                    .dependencies
                    .retain(|id| id == &missing.ecore);
                let mut ambiguous = valid;
                let mut duplicate =
                    DefinitionSource::new("duplicate.xtext", SourceKind::Xtext, PARENT);
                duplicate.dependencies.push(ambiguous.ecore.clone());
                ambiguous.bundle.sources[2]
                    .dependencies
                    .push(duplicate.id.clone());
                ambiguous.bundle.sources.push(duplicate);
                for (compilation, message) in [
                    (missing, "Missing exact parent"),
                    (ambiguous, "Ambiguous parent"),
                ] {
                    let mut schema = package.schema.clone();
                    let before = serde_json::to_value(&schema).unwrap();
                    let error = grammar::compile_sources(
                        &compilation.bundle,
                        compilation.xtext.as_ref().unwrap(),
                        &mut schema,
                        Profile::NativeV51,
                    )
                    .unwrap_err();
                    assert!(error.message.contains(message), "{error:?}");
                    assert_eq!(error.source, compilation.xtext.as_ref().unwrap().uri);
                    assert_eq!((error.start, error.end), (start, start + spelling.len()));
                    assert_eq!(&child[error.start..error.end], spelling);
                    assert_eq!(serde_json::to_value(&schema).unwrap(), before);
                    let errors =
                        compiler::compile_bundle_with_profile(compilation, Profile::NativeV51)
                            .unwrap_err();
                    assert!(
                        errors.iter().any(|e| e.source == error.source
                            && e.start == error.start
                            && e.end == error.end
                            && e.message == error.message),
                        "{errors:?}"
                    );
                }
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn pinned_string_converter_observations_expose_native_v51_gap() {
    let observations: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/default-converters/observations.json"
    ))
    .unwrap();
    assert_eq!(observations.as_array().unwrap().len(), 16);
    for materialized in [false, true] {
        let direct = CHILD.replace("name=ID", "name=STRING");
        let wrapped = direct.replace("name=STRING", "name=Text").replace(
            "Model returns",
            "import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nModel returns",
        ) + "\nText returns ecore::EString: STRING;";
        for child in [&direct, &wrapped] {
            let compilation = input(PARENT, child, materialized);
            let exact = compilation.bundle.sources.clone();
            let package =
                compiler::compile_bundle_with_profile(compilation, Profile::NativeV51).unwrap();
            assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
            let replay = compiler::recompile(&package).unwrap();
            assert_eq!(
                serde_json::to_value(&package).unwrap(),
                serde_json::to_value(&replay).unwrap()
            );
            for row in observations.as_array().unwrap() {
                let literal = row["literal"].as_str().unwrap();
                let text = format!("model {literal};");
                let result = evaluate(&package, &sources(&text));
                if literal == "\"" || literal == "\"unclosed" {
                    assert!(!result.diagnostics.is_empty());
                    assert!(result.model.elements.is_empty());
                    continue;
                }
                assert!(
                    result.diagnostics.is_empty(),
                    "{literal}: {:?}",
                    result.diagnostics
                );
                assert_eq!(result.model.elements.len(), 1);
                assert_eq!(result.model.elements[0].id, "terminal.model#0");
                assert_eq!(
                    (result.model.elements[0].start, result.model.elements[0].end),
                    (0, text.len())
                );
                assert_eq!(result.model.elements[0].values["name"], literal);
                result.model.validate(&package.schema).unwrap();
                assert_eq!(evaluate(&replay, &sources(&text)).model, result.model);
                if let Some(units) = row.get("value").and_then(|value| value.get("utf16")) {
                    let units: Vec<u16> = serde_json::from_value(units.clone()).unwrap();
                    assert_ne!(
                        literal.encode_utf16().collect::<Vec<_>>(),
                        units,
                        "Direct STRING semantic gap; datatype Text intentionally accumulates raw spelling"
                    );
                } else {
                    assert_eq!(row["error"], "ValueConverterWithValueException");
                    // Recognition succeeds, but direct STRING should report the
                    // upstream conversion error. A Text datatype rule instead
                    // preserves raw spelling and does not invoke STRING conversion.
                }
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn default_string_conversion_matches_oracle_with_explicit_model_boundary() {
    let observations: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/default-converters/observations.json"
    ))
    .unwrap();
    for row in observations.as_array().unwrap() {
        let literal = row["literal"].as_str().unwrap();
        let decoded = grammar::syntax::decode_string_literal(literal);
        if let Some(units) = row.get("value").and_then(|v| v.get("utf16")) {
            let units: Vec<u16> = serde_json::from_value(units.clone()).unwrap();
            assert_eq!(decoded.unwrap().0, units);
        } else {
            assert!(decoded.is_err(), "{literal}");
        }
    }
    for materialized in [false, true] {
        for (call, raw) in [
            ("STRING", false),
            ("Terminals::STRING", false),
            ("Text", true),
            ("QUOTED", true),
        ] {
            let mut child = CHILD.replace("name=ID", &format!("name={call}"));
            if call == "Text" {
                child = child.replace(
                    "Model returns",
                    "import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nModel returns",
                ) + "\nText returns ecore::EString: STRING;";
            }
            let parent = if call == "QUOTED" {
                PARENT.replace("terminal STRING:", "terminal QUOTED:")
            } else {
                PARENT.into()
            };
            let compilation = input(&parent, &child, materialized);
            let exact = compilation.bundle.sources.clone();
            let package =
                compiler::compile_bundle_with_profile(compilation, Profile::NativeV52).unwrap();
            assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
            let replay = compiler::recompile(&package).unwrap();
            assert_eq!(
                serde_json::to_value(&package).unwrap(),
                serde_json::to_value(&replay).unwrap()
            );
            for row in observations.as_array().unwrap() {
                let literal = row["literal"].as_str().unwrap();
                let text = format!("model {literal};");
                let result = evaluate(&package, &sources(&text));
                let units: Option<Vec<u16>> = row
                    .get("value")
                    .map(|v| serde_json::from_value(v["utf16"].clone()).unwrap());
                let expected = if raw && literal != "\"" && literal != "\"unclosed" {
                    Some(literal.to_owned())
                } else if raw {
                    None
                } else {
                    units.as_ref().and_then(|u| String::from_utf16(u).ok())
                };
                if let Some(expected) = expected {
                    assert!(
                        result.diagnostics.is_empty(),
                        "{call} {literal}: {:?}",
                        result.diagnostics
                    );
                    assert_eq!(result.model.elements.len(), 1);
                    assert_eq!(result.model.elements[0].id, "terminal.model#0");
                    assert_eq!(
                        (result.model.elements[0].start, result.model.elements[0].end),
                        (0, text.len())
                    );
                    assert_eq!(result.model.elements[0].values["name"], expected);
                    result.model.validate(&package.schema).unwrap();
                    assert_eq!(evaluate(&replay, &sources(&text)).model, result.model);
                } else {
                    assert!(!result.diagnostics.is_empty(), "{call} {literal}");
                    assert!(result.model.elements.is_empty());
                    if units.is_some() {
                        assert!(
                            result
                                .diagnostics
                                .iter()
                                .any(|d| d.message.contains("unpaired UTF-16 surrogate")),
                            "{:?}",
                            result.diagnostics
                        );
                    }
                }
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
                    !evaluate_with_limits(&package, &sources("model \"Alpha\";"), limits)
                        .diagnostics
                        .is_empty()
                );
            }
            assert!(
                evaluate(&package, &sources("model \"Alpha\";"))
                    .diagnostics
                    .is_empty()
            );
        }
    }
}
