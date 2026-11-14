//! Terminal override execution shares the composed grammar and normal evaluator.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/terminal-spellings/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/terminal-spellings/model-behavior.rcore");
const NS: &str = "urn:intersect:xtext:mini-kerml:terminal-spellings:1";
fn base() -> String {
    format!(
        r#"grammar audit.Base hidden() import "{NS}" as k Model returns k::Model: name=VALUE; terminal VALUE: '@' LETTER; terminal fragment LETTER: 'a';"#
    )
}
fn child(body: &str) -> String {
    format!(
        r#"grammar audit.Child with audit.Base hidden() import "{NS}" as k Entry returns k::Model: Model; {body}"#
    )
}
fn inputs(base_text: &str, child_text: &str) -> BundleCompilation {
    let ecore = DefinitionSource::new("model.ecore", SourceKind::Ecore, ECORE);
    let mut base = DefinitionSource::new("base.xtext", SourceKind::Xtext, base_text);
    base.dependencies.push(ecore.id.clone());
    let mut child = DefinitionSource::new("child.xtext", SourceKind::Xtext, child_text);
    child.dependencies = vec![base.id.clone(), ecore.id.clone()];
    let mut semantics = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        RCORE_BEHAVIOR,
    );
    semantics.dependencies.push(ecore.id.clone());
    let result = BundleCompilation {
        ecore: ecore.id.clone(),
        xtext: Some(child.id.clone()),
        rcore_behavior: semantics.id.clone(),
        namespace: NS.into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, base, child, semantics],
            namespaces: vec![],
        },
    };
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn inherited_calls_select_overridden_terminals_and_fragments() {
    inherited_calls_select_overridden_terminals_and_fragments_with_profile(Profile::NativeV15);
}
fn inherited_calls_select_overridden_terminals_and_fragments_with_profile(profile: Profile) {
    for (body, good) in [
        ("@Override terminal VALUE: '#' LETTER;", "#a"),
        ("@Override terminal fragment LETTER: 'b';", "@b"),
        ("terminal fragment LETTER: 'b';", "@b"),
        ("@Override terminal LETTER: 'b';", "@b"),
    ] {
        let input = inputs(&base(), &child(body));
        assert!(compiler::compile_bundle_with_profile(input.clone(), Profile::NativeV14).is_err());
        let package = compiler::compile_bundle_with_profile(input.clone(), profile).unwrap();
        let replay = compiler::recompile(&package).unwrap();
        assert_eq!(
            serde_json::to_value(&package).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        for p in [&package, &replay] {
            let result = evaluate(
                p,
                &[Source {
                    uri: "override.model".into(),
                    text: good.into(),
                }],
            );
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(result.model.elements[0].values["name"], good);
            for bad in ["@a", "#b", "@", "@bb"] {
                let result = evaluate(
                    p,
                    &[Source {
                        uri: "bad.model".into(),
                        text: bad.into(),
                    }],
                );
                assert!(!result.diagnostics.is_empty(), "{body}: {bad}");
                assert!(result.model.elements.is_empty());
            }
        }
        let grammar = package.grammar.as_ref().unwrap();
        let name = if body.contains("LETTER:") {
            "LETTER"
        } else {
            "VALUE"
        };
        assert_eq!(
            Some(&grammar.declaration_sources[name].source),
            input.xtext.as_ref()
        );
        assert_eq!(
            grammar.shadowed_declarations[name][0].source.uri,
            "base.xtext"
        );
        assert_eq!(
            grammar.terminal_order.iter().filter(|n| *n == name).count(),
            1
        );
        assert_eq!(
            grammar.terminals[name].identity,
            format!("audit.Child::{name}")
        );
    }
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        &base(),
        RCORE_BEHAVIOR,
        profile,
    )
    .unwrap();
    assert!(
        evaluate(
            &p,
            &[Source {
                uri: "base.model".into(),
                text: "@a".into()
            }]
        )
        .diagnostics
        .is_empty()
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn incompatible_or_orphan_overrides_and_shadowed_errors_reject() {
    incompatible_or_orphan_overrides_and_shadowed_errors_reject_with_profile(Profile::NativeV15);
}
fn incompatible_or_orphan_overrides_and_shadowed_errors_reject_with_profile(profile: Profile) {
    for body in [
        "@Override terminal MISSING: 'b';",
        "@Override terminal fragment VALUE: 'b';",
        "terminal VALUE returns ecore::EInt: '1';",
        "@Override VALUE returns k::Model: name=LETTER;",
    ] {
        let mut schema =
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
                .unwrap();
        let before = serde_json::to_value(&schema).unwrap();
        let child_text = child(body).replace(
            "Entry returns",
            "import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore Entry returns",
        );
        let input = inputs(&base(), &child_text);
        let failure = grammar::compile_sources(
            &input.bundle,
            input.xtext.as_ref().unwrap(),
            &mut schema,
            profile,
        )
        .unwrap_err();
        if body.contains("EInt") {
            assert!(
                failure.message.contains("Incompatible terminal override"),
                "{:?}",
                failure
            );
        }
        assert_eq!(serde_json::to_value(schema).unwrap(), before);
    }
    let input = inputs(
        &base().replace("LETTER: 'a'", "LETTER: UNKNOWN"),
        &child("terminal fragment LETTER: 'b';"),
    );
    let err = compiler::compile_bundle_with_profile(input, profile).unwrap_err();
    assert!(err.iter().any(|e| e.source == "base.xtext"));
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &base().replace("terminal VALUE", "@Override terminal VALUE"),
            RCORE_BEHAVIOR,
            profile
        )
        .is_err()
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn three_level_sources_and_aggregate_limits_remain_exact() {
    three_level_sources_and_aggregate_limits_remain_exact_with_profile(Profile::NativeV15);
}
fn three_level_sources_and_aggregate_limits_remain_exact_with_profile(profile: Profile) {
    let mut input = inputs(&base(), &child("terminal fragment LETTER: 'b';"));
    let mut top = DefinitionSource::new(
        "top.xtext",
        SourceKind::Xtext,
        child("terminal fragment LETTER: 'c';")
            .replace("audit.Child with audit.Base", "audit.Top with audit.Child")
            .replace("Entry returns", "TopEntry returns"),
    );
    top.dependencies = vec![input.ecore.clone(), input.xtext.clone().unwrap()];
    input.xtext = Some(top.id.clone());
    input.bundle.sources.push(top);
    let p = compiler::compile_bundle_with_profile(input.clone(), profile).unwrap();
    let result = evaluate(
        &p,
        &[Source {
            uri: "top.model".into(),
            text: "@c".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(
        p.grammar.as_ref().unwrap().shadowed_declarations["LETTER"]
            .iter()
            .map(|s| s.source.uri.as_str())
            .collect::<Vec<_>>(),
        ["child.xtext", "base.xtext"]
    );
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let limits = SourceLimits {
        source_bytes: 256_000,
        declarations: 7,
    };
    assert!(
        grammar::compile_sources_with_limits(
            &input.bundle,
            input.xtext.as_ref().unwrap(),
            &mut schema,
            profile,
            limits
        )
        .is_ok()
    );
    assert!(
        grammar::compile_sources_with_limits(
            &input.bundle,
            input.xtext.as_ref().unwrap(),
            &mut schema,
            profile,
            SourceLimits {
                declarations: 6,
                ..limits
            }
        )
        .is_err()
    );
    input.bundle.sources.reverse();
    assert_eq!(
        serde_json::to_value(p.grammar).unwrap(),
        serde_json::to_value(
            compiler::compile_bundle_with_profile(input, profile)
                .unwrap()
                .grammar
        )
        .unwrap()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn independent_contextual_binding_observations() {
    let observations: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/terminal-overrides/observations.json"
    ))
    .unwrap();
    assert_eq!(observations.len(), 5);
    for row in observations {
        let case = row["case"].as_str().unwrap();
        if case == "explicit-parent-call" {
            assert_eq!(row["terminal_call"], "audit.Base::LETTER");
            let source = base().replace("'@' LETTER", "'@' audit.Base.LETTER");
            assert!(
                compiler::compile_bundle_with_profile(
                    inputs(&source, &child("terminal fragment LETTER: 'b';")),
                    Profile::NativeV15
                )
                .is_err()
            );
            continue;
        }
        let mut base_text = base();
        if case == "terminal" {
            base_text = base_text.replace("terminal fragment LETTER", "terminal LETTER");
        }
        let replacement = if case == "terminal" || case == "promoted-fragment" {
            "terminal LETTER: 'b';"
        } else {
            "terminal fragment LETTER: 'b';"
        };
        for direct in [false, true] {
            let text = if direct {
                base_text.replace("name=VALUE", "name=LETTER")
            } else {
                base_text.clone()
            };
            let mut input = inputs(&text, &child(replacement));
            if case == "three-level" {
                let mut top = DefinitionSource::new(
                    "top.xtext",
                    SourceKind::Xtext,
                    child("terminal fragment LETTER: 'c';")
                        .replace("audit.Child with audit.Base", "audit.Top with audit.Child")
                        .replace("Entry returns", "TopEntry returns"),
                );
                top.dependencies = vec![input.ecore.clone(), input.xtext.clone().unwrap()];
                input.xtext = Some(top.id.clone());
                input.bundle.sources.push(top);
            }
            // Binding-only oracle ASTs are not evidence of grammar validity.
            // Direct parser calls to terminal fragments remain rejected,
            // including an invalid parent later promoted by its child.
            if direct && case != "terminal" {
                assert!(compiler::compile_bundle_with_profile(input, Profile::NativeV15).is_err());
                continue;
            }
            let package = compiler::compile_bundle_with_profile(input, Profile::NativeV15).unwrap();
            let expected_identity = row[if direct {
                "parser_call"
            } else {
                "terminal_call"
            }]
            .as_str()
            .unwrap();
            assert_eq!(
                package.grammar.as_ref().unwrap().terminals["LETTER"].identity,
                expected_identity
            );
            let literal = row["selected_literal"].as_str().unwrap();
            let model = if direct {
                literal.to_owned()
            } else {
                format!("@{literal}")
            };
            let result = evaluate(
                &package,
                &[Source {
                    uri: "oracle.model".into(),
                    text: model.clone(),
                }],
            );
            assert!(
                result.diagnostics.is_empty(),
                "{case}: {:?}",
                result.diagnostics
            );
            assert_eq!(result.model.elements[0].values["name"], model);
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_call_spellings_have_independent_scope_evidence() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/terminal-overrides/scope-observations.json"
    ))
    .unwrap();
    assert_eq!(rows.len(), 9);
    for row in rows {
        let spelling = row["spelling"].as_str().unwrap();
        let text = child(&format!("terminal fragment LETTER: {spelling};"));
        let source = DefinitionSource::new("child.xtext", SourceKind::Xtext, text.clone());
        let syntax = grammar::syntax::parse(&source, SourceLimits::default());
        let syntax = syntax.unwrap();
        let body = &syntax.rules.last().unwrap().value.body.value;
        if spelling.contains('.') {
            assert_eq!(row["body_kind"], "Group");
            let grammar::syntax::ElementKind::Sequence(items) = body else {
                panic!("Expected calls separated by wildcard operators")
            };
            assert_eq!(items.len(), 5);
            assert!(matches!(
                items[1].value,
                grammar::syntax::ElementKind::Wildcard
            ));
            assert!(matches!(
                items[3].value,
                grammar::syntax::ElementKind::Wildcard
            ));
            assert!(row["selected"].is_null());
        } else {
            assert_eq!(row["body_kind"], "RuleCall");
            let grammar::syntax::ElementKind::Call { rule, .. } = body else {
                panic!("Expected one qualified rule call")
            };
            assert_eq!(rule.value, spelling);
            assert_eq!(
                row["explicit"].as_bool().unwrap(),
                !row["selected"].is_null()
            );
        }
        // Valid syntax and independent scope selection do not imply NativeV15
        // executable support. Keep this version's rejection contract frozen.
        assert!(
            compiler::compile_bundle_with_profile(inputs(&base(), &text), Profile::NativeV15)
                .is_err(),
            "{spelling}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_ancestor_terminal_programs_execute() {
    explicit_ancestor_terminal_programs_execute_with_profile(Profile::NativeV16);
}
fn explicit_ancestor_terminal_programs_execute_with_profile(profile: Profile) {
    for spelling in [
        "super",
        "super::LETTER",
        "Base::LETTER",
        "audit::Base::LETTER",
    ] {
        let input = inputs(
            &base(),
            &child(&format!("terminal fragment LETTER: {spelling} | 'b';")),
        );
        assert!(compiler::compile_bundle_with_profile(input.clone(), Profile::NativeV15).is_err());
        let package = compiler::compile_bundle_with_profile(input.clone(), profile).unwrap();
        let replay = compiler::recompile(&package).unwrap();
        assert_eq!(
            serde_json::to_value(&package).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        for p in [&package, &replay] {
            for (text, valid) in [("@a", true), ("@b", true), ("@c", false), ("@ab", false)] {
                let result = evaluate(
                    p,
                    &[Source {
                        uri: "explicit.model".into(),
                        text: text.into(),
                    }],
                );
                assert_eq!(
                    result.diagnostics.is_empty(),
                    valid,
                    "{spelling}: {text}: {:?}",
                    result.diagnostics
                );
                if valid {
                    assert_eq!(result.model.elements[0].values["name"], text);
                } else {
                    assert!(result.model.elements.is_empty());
                }
            }
        }
        let grammar = package.grammar.as_ref().unwrap();
        assert_eq!(
            grammar.declaration_sources["@audit.Base::LETTER"]
                .source
                .uri,
            "base.xtext"
        );
        assert!(grammar.terminals["@audit.Base::LETTER"].fragment);
        assert!(!grammar.terminal_order.iter().any(|n| n.starts_with('@')));
        let mut schema =
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
                .unwrap();
        assert!(
            grammar::compile_sources_with_limits(
                &input.bundle,
                input.xtext.as_ref().unwrap(),
                &mut schema,
                profile,
                SourceLimits {
                    source_bytes: 256_000,
                    declarations: 5
                }
            )
            .is_ok()
        );
        assert!(
            grammar::compile_sources_with_limits(
                &input.bundle,
                input.xtext.as_ref().unwrap(),
                &mut schema,
                profile,
                SourceLimits {
                    source_bytes: 256_000,
                    declarations: 4
                }
            )
            .is_err()
        );
    }
    // Explicitly selecting the parent VALUE must not freeze its ordinary LETTER call.
    let input = inputs(
        &base(),
        &child("terminal VALUE: super; terminal fragment LETTER: 'b';"),
    );
    let p = compiler::compile_bundle_with_profile(input, profile).unwrap();
    for (text, valid) in [("@b", true), ("@a", false)] {
        let r = evaluate(
            &p,
            &[Source {
                uri: "context.model".into(),
                text: text.into(),
            }],
        );
        assert_eq!(r.diagnostics.is_empty(), valid, "{:?}", r.diagnostics);
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_three_level_context_and_named_super() {
    explicit_three_level_context_and_named_super_with_profile(Profile::NativeV16);
}
fn explicit_three_level_context_and_named_super_with_profile(profile: Profile) {
    let mut input = inputs(&base(), &child("terminal fragment LETTER: super | 'b';"));
    let mut top = DefinitionSource::new(
        "top.xtext",
        SourceKind::Xtext,
        child("terminal fragment LETTER: super | 'c';")
            .replace("audit.Child with audit.Base", "audit.Top with audit.Child")
            .replace("Entry returns", "TopEntry returns"),
    );
    top.dependencies = vec![input.ecore.clone(), input.xtext.clone().unwrap()];
    input.xtext = Some(top.id.clone());
    input.bundle.sources.push(top);
    let p = compiler::compile_bundle_with_profile(input, profile).unwrap();
    for text in ["@a", "@b", "@c"] {
        let r = evaluate(
            &p,
            &[Source {
                uri: "three.model".into(),
                text: text.into(),
            }],
        );
        assert!(r.diagnostics.is_empty(), "{text}: {:?}", r.diagnostics);
    }
    let base = format!("{} terminal fragment super: 's';", base());
    let p = compiler::compile_bundle_with_profile(
        inputs(&base, &child("terminal fragment LETTER: super | 'b';")),
        profile,
    )
    .unwrap();
    for (text, valid) in [("@s", true), ("@b", true), ("@a", false)] {
        let r = evaluate(
            &p,
            &[Source {
                uri: "named.model".into(),
                text: text.into(),
            }],
        );
        assert_eq!(
            r.diagnostics.is_empty(),
            valid,
            "{text}: {:?}",
            r.diagnostics
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_ancestor_failures_restore_schema() {
    explicit_ancestor_failures_restore_schema_with_profile(Profile::NativeV16);
}
fn explicit_ancestor_failures_restore_schema_with_profile(profile: Profile) {
    for spelling in [
        "Missing::LETTER",
        "Base::MISSING",
        "Child::LETTER",
        "audit::Child::LETTER",
    ] {
        let input = inputs(
            &base(),
            &child(&format!("terminal fragment LETTER: {spelling};")),
        );
        let mut schema =
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
                .unwrap();
        let before = serde_json::to_value(&schema).unwrap();
        let error = grammar::compile_sources(
            &input.bundle,
            input.xtext.as_ref().unwrap(),
            &mut schema,
            profile,
        )
        .unwrap_err();
        assert!(!error.message.is_empty());
        assert_eq!(serde_json::to_value(&schema).unwrap(), before);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    inherited_calls_select_overridden_terminals_and_fragments_with_profile(Profile::NativeV51);
    incompatible_or_orphan_overrides_and_shadowed_errors_reject_with_profile(Profile::NativeV51);
    three_level_sources_and_aggregate_limits_remain_exact_with_profile(Profile::NativeV51);
    explicit_ancestor_terminal_programs_execute_with_profile(Profile::NativeV51);
    explicit_three_level_context_and_named_super_with_profile(Profile::NativeV51);
    explicit_ancestor_failures_restore_schema_with_profile(Profile::NativeV51);
}
