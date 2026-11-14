//! Ordinary enum-valued parser rules share the imported enum lexical codec.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/enum-composition/model.ecore");
const BASE: &str = include_str!("../../../../fixtures/xtext/enum-composition/base.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/enum-datatype-parsers/model.xtext");
const PARSER: &str = "Choice returns p::Shade: 'red' | 'blue' | 'missing';";
const ENUM: &str = "enum Choice returns p::Shade: Red='red' | Blue='blue';";
fn input(base: &str, child: &str, materialized: bool) -> BundleCompilation {
    let e = DefinitionSource::new("colors.ecore", SourceKind::Ecore, ECORE);
    let mut b = DefinitionSource::new("base.xtext", SourceKind::Xtext, base);
    b.dependencies.push(e.id.clone());
    let mut c = DefinitionSource::new("child.xtext", SourceKind::Xtext, child);
    c.dependencies = vec![e.id.clone(), b.id.clone()];
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
            sources: vec![e, b, c, s],
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
        uri: "color.model".into(),
        text: text.into(),
    }]
}
fn verify(compilation: BundleCompilation, text: &str, literal: &str) {
    verify_profile(compilation, text, literal, Profile::NativeV46);
}
fn verify_profile(compilation: BundleCompilation, text: &str, literal: &str, profile: Profile) {
    let exact = compilation.bundle.sources.clone();
    let base = exact[1].id.clone();
    let child = exact[2].id.clone();
    let selected = compilation.xtext.clone().unwrap();
    let package = compiler::compile_bundle_with_profile(compilation, profile).unwrap();
    assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
    let grammar = package.grammar.as_ref().unwrap();
    if selected == child {
        assert_eq!(grammar.declaration_sources["Choice"].source, child);
        assert_eq!(grammar.shadowed_declarations["Choice"][0].source, base);
    } else {
        assert_eq!(selected, base);
        assert_eq!(grammar.declaration_sources["Choice"].source, base);
        assert!(!grammar.shadowed_declarations.contains_key("Choice"));
    }
    let result = evaluate(&package, &sources(text));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    assert_eq!(result.model.elements[0].id, "color.model#0");
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
    let value = enumeration
        .value_by_id(&enumeration.by_name(literal).unwrap().declaration)
        .unwrap();
    assert_eq!(
        result.model.elements[0].values["shade"],
        serde_json::to_value(value).unwrap()
    );
    result.model.validate(&package.schema).unwrap();
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    assert_eq!(evaluate(&replay, &sources(text)).model, result.model);
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
    assert!(
        !evaluate(&package, &sources("color missing;"))
            .diagnostics
            .is_empty()
    );
    assert!(
        !evaluate(&package, &sources("color ?;"))
            .diagnostics
            .is_empty()
    );
    assert_eq!(evaluate(&package, &sources(text)).model, result.model);
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_and_parser_overrides_preserve_nominal_values() {
    for materialized in [false, true] {
        for parent_parser in [false, true] {
            for child_parser in [false, true] {
                let base = if parent_parser {
                    BASE.replace(ENUM, PARSER)
                } else {
                    BASE.into()
                };
                let child = if child_parser {
                    CHILD.into()
                } else {
                    CHILD.replace(
                        PARSER,
                        "enum Choice returns p::Shade: Red='warm' | Blue='cool';",
                    )
                };
                for (text, literal) in if child_parser {
                    [("color red;", "Red"), ("color blue;", "Blue")]
                } else {
                    [("color warm;", "Red"), ("color cool;", "Blue")]
                } {
                    verify(input(&base, &child, materialized), text, literal);
                    verify_profile(
                        input(&base, &child, materialized),
                        text,
                        literal,
                        Profile::NativeV47,
                    );
                }
            }
        }
    }
    assert!(
        compiler::compile_bundle_with_profile(input(BASE, CHILD, false), Profile::NativeV45)
            .is_err()
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_parser_uses_lexical_conversion_and_retained_parent_calls() {
    let base = BASE.replace(ENUM, PARSER);
    let child = CHILD
        .replace(
            PARSER,
            "enum Choice returns p::Shade: Red='warm' | Blue='cool';",
        )
        .replace("shade=Choice", "shade=super::Choice");
    for materialized in [false, true] {
        verify(input(&base, &child, materialized), "color red;", "Red");
    }
    let helper = CHILD.replace(
        PARSER,
        "Choice returns p::Shade: Word;\nWord returns p::Shade: 'red' | 'blue' | 'missing';",
    );
    verify(
        input(BASE, &helper, false),
        include_str!("../../../../fixtures/xtext/enum-datatype-parsers/example.model").trim_end(),
        "Red",
    );
    let invalid = CHILD.replace(PARSER, "Choice returns p::Shade: 'warm';");
    let package =
        compiler::compile_bundle_with_profile(input(BASE, &invalid, false), Profile::NativeV46)
            .unwrap();
    let result = evaluate(&package, &sources("color warm;"));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Invalid literal")),
        "{:?}",
        result.diagnostics
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_parser_category_rejections_remain_explicit() {
    for child in [
        CHILD.replace("Choice returns p::Shade", "Choice returns p::OtherShade"),
        CHILD.replace("@Override Choice", "@Override fragment Choice"),
        CHILD.replace(PARSER, "Choice returns p::Shade: 'red'?;"),
        CHILD.replace(PARSER, "Choice returns p::Shade: Choice;"),
        CHILD.replace("@Override Choice", "@Override Missing"),
    ] {
        assert!(
            compiler::compile_bundle_with_profile(input(BASE, &child, false), Profile::NativeV46)
                .is_err()
        );
    }
    let base = BASE.replace("enum Choice", "@Final enum Choice");
    assert!(
        compiler::compile_bundle_with_profile(input(&base, CHILD, false), Profile::NativeV46)
            .is_err()
    );
    let child = CHILD.replace("@Override Choice", "@Override terminal Choice");
    let errors =
        compiler::compile_bundle_with_profile(input(BASE, &child, false), Profile::NativeV46)
            .unwrap_err();
    assert!(
        errors.iter().any(|e| e
            .message
            .contains("Custom terminals currently require EString")),
        "{errors:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_terminals_preserve_nominal_override_values() {
    let terminal = "terminal Choice returns p::Shade: 'red' | 'blue' | 'missing';";
    assert_eq!(
        CHILD.replace(PARSER, terminal),
        include_str!("../../../../fixtures/xtext/enum-datatype-parsers/terminal.xtext")
    );
    let declaration_only = CHILD.replace(PARSER, terminal).replace(
        "@Override Model returns p::Model: 'color' shade=Choice ';';\n",
        "",
    );
    verify_profile(
        input(BASE, &declaration_only, false),
        "color red;",
        "Red",
        Profile::NativeV47,
    );
    for materialized in [false, true] {
        for base in [
            BASE.to_owned(),
            BASE.replace(ENUM, PARSER),
            BASE.replace(ENUM, terminal),
        ] {
            for child in [
                CHILD.replace(PARSER, terminal),
                CHILD
                    .replace(PARSER, terminal)
                    .replace("@Override terminal", "terminal"),
            ] {
                for (text, literal) in [("color red;", "Red"), ("color blue;", "Blue")] {
                    verify_profile(
                        input(&base, &child, materialized),
                        text,
                        literal,
                        Profile::NativeV47,
                    );
                }
            }
        }
    }
    assert!(
        compiler::compile_bundle_with_profile(
            input(BASE, &CHILD.replace(PARSER, terminal), false),
            Profile::NativeV46
        )
        .is_err()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_terminal_categories_and_types_reject_invalid_overrides() {
    let terminal = "terminal Choice returns p::Shade: 'red' | 'blue' | 'missing';";
    let base = BASE.replace(ENUM, terminal);
    for child in [
        CHILD.to_owned(),
        CHILD.replace(PARSER, ENUM),
        CHILD.replace(PARSER, terminal).replace(
            "terminal Choice returns p::Shade",
            "terminal fragment Choice",
        ),
    ] {
        for child in [child.clone(), child.replace("@Override ", "")] {
            let errors = compiler::compile_bundle_with_profile(
                input(&base, &child, false),
                Profile::NativeV47,
            )
            .unwrap_err();
            assert!(
                errors.iter().any(
                    |error| error.message.contains("Incompatible terminal override")
                        || error
                            .message
                            .contains("@Override has no inherited parser declaration")
                ),
                "{errors:?}"
            );
        }
    }
    for base in [BASE.to_owned(), BASE.replace(ENUM, PARSER), base] {
        let child = CHILD.replace(PARSER, &terminal.replace("p::Shade", "p::OtherShade"));
        let errors =
            compiler::compile_bundle_with_profile(input(&base, &child, false), Profile::NativeV47)
                .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("Incompatible terminal override")),
            "{errors:?}"
        );
    }
    let base = BASE.replace(ENUM, "terminal Choice returns p::Shade: 'red' | 'blue';");
    let child = CHILD
        .replace(PARSER, "terminal Other returns p::OtherShade: 'green';")
        .replace("@Override terminal Other", "terminal Other")
        .replace("shade=Choice", "shade=Other");
    let errors =
        compiler::compile_bundle_with_profile(input(&base, &child, false), Profile::NativeV47)
            .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("Enum terminal type mismatch")),
        "{errors:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_terminal_parent_calls_and_parser_calls_share_codec() {
    let terminal = "terminal Choice returns p::Shade: 'red' | 'blue' | 'missing';";
    let base = BASE.replace(ENUM, terminal);
    let child = CHILD
        .replace(PARSER, terminal)
        .replace("shade=Choice", "shade=super::Choice");
    // Historical lexer priority limitation: the child token label masks an
    // identical retained parent terminal. This is not upstream-invalid syntax.
    let overlap =
        compiler::compile_bundle_with_profile(input(&base, &child, false), Profile::NativeV47)
            .unwrap();
    assert!(
        evaluate(&overlap, &sources("color red;"))
            .diagnostics
            .iter()
            .any(|error| error.message.contains("Expected @example.Colors::Choice"))
    );
    let base = BASE.replace(ENUM, "terminal Choice returns p::Shade: 'red';");
    let child = child.replace(terminal, "terminal Choice returns p::Shade: 'blue';");
    for materialized in [false, true] {
        verify_profile(
            input(&base, &child, materialized),
            "color red;",
            "Red",
            Profile::NativeV47,
        );
    }
    let child = CHILD.replace(PARSER, "Choice returns p::Shade: ColorToken;\nterminal ColorToken returns p::Shade: 'red' | 'blue' | 'missing';");
    verify_profile(
        input(BASE, &child, false),
        "color blue;",
        "Blue",
        Profile::NativeV47,
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn retained_parent_partial_overlap_keeps_child_token_priority() {
    // Pinned RuleNames + ANTLR observations are recorded in terminal-overrides/
    // terminal-identity-observations.json. Shared red belongs to the child;
    // blue remains available to the separately named retained parent token.
    let base = BASE.replace(ENUM, "terminal Choice returns p::Shade: 'red' | 'blue';");
    let child = CHILD
        .replace(PARSER, "terminal Choice returns p::Shade: 'red';")
        .replace("shade=Choice", "shade=super::Choice");
    for materialized in [false, true] {
        verify_profile(
            input(&base, &child, materialized),
            "color blue;",
            "Blue",
            Profile::NativeV47,
        );
        let package = compiler::compile_bundle_with_profile(
            input(&base, &child, materialized),
            Profile::NativeV47,
        )
        .unwrap();
        let result = evaluate(&package, &sources("color red;"));
        assert!(
            result
                .diagnostics
                .iter()
                .any(|error| error.message.contains("Expected @example.Colors::Choice")),
            "{:?}",
            result.diagnostics
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn unreachable_retained_terminal_rejects_at_admission() {
    let terminal = "terminal Choice returns p::Shade: 'red';";
    let base = BASE.replace(ENUM, terminal);
    let child = CHILD
        .replace(PARSER, terminal)
        .replace("shade=Choice", "shade=super::Choice");
    for materialized in [false, true] {
        let compilation = input(&base, &child, materialized);
        let parent = compilation.bundle.sources[1].id.clone();
        let errors =
            compiler::compile_bundle_with_profile(compilation, Profile::NativeV48).unwrap_err();
        let error = errors
            .iter()
            .find(|error| {
                error
                    .message
                    .contains("Unreachable terminal @example.Colors::Choice")
            })
            .unwrap_or_else(|| panic!("{errors:?}"));
        assert!(error.message.contains("lexical prediction"));
        // The diagnostic must identify the original retained parent declaration.
        assert_eq!(error.source, parent.uri);
        assert_eq!(&base[error.start..error.end], terminal);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn reachable_terminal_overlap_preserves_priority_and_values() {
    for (parent, child_body, spelling, literal) in [
        ("'red'", "'blue'", "red", "Red"),
        ("'red' | 'blue'", "'red'", "blue", "Blue"),
    ] {
        let base = BASE.replace(
            ENUM,
            &format!("terminal Choice returns p::Shade: {parent};"),
        );
        let child = CHILD
            .replace(
                PARSER,
                &format!("terminal Choice returns p::Shade: {child_body};"),
            )
            .replace("shade=Choice", "shade=super::Choice");
        for materialized in [false, true] {
            verify_profile(
                input(&base, &child, materialized),
                &format!("color {spelling};"),
                literal,
                Profile::NativeV48,
            );
            if parent.contains('|') {
                let package = compiler::compile_bundle_with_profile(
                    input(&base, &child, materialized),
                    Profile::NativeV48,
                )
                .unwrap();
                assert!(
                    evaluate(&package, &sources("color red;"))
                        .diagnostics
                        .iter()
                        .any(|error| error.message.contains("Expected @example.Colors::Choice"))
                );
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn lexical_reachability_proves_union_and_reports_inconclusive_budget() {
    for materialized in [false, true] {
        // Neither earlier terminal alone shadows Both: the decision must explore their union.
        let child = format!(
            "{}\nterminal Red: 'red';\nterminal Blue: 'blue';\nterminal Both: 'red'|'blue';",
            CHILD.replace(PARSER, "Choice returns p::Shade: Red | Blue;")
        );
        let errors = compiler::compile_bundle_with_profile(
            input(BASE, &child, materialized),
            Profile::NativeV48,
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("Unreachable terminal Both")),
            "{errors:?}"
        );
        // This fits the unchanged NFA state ceiling, but exceeds bounded proof work.
        let word = "x".repeat(1700);
        let child = format!("{CHILD}\nterminal LongA: '{word}';\nterminal LongB: '{word}';");
        let errors = compiler::compile_bundle_with_profile(
            input(BASE, &child, materialized),
            Profile::NativeV48,
        )
        .unwrap_err();
        assert!(
            errors.iter().any(|error| error
                .message
                .contains("Lexical reachability analysis budget exceeded (inconclusive)")),
            "{errors:?}"
        );
        assert!(
            !errors
                .iter()
                .any(|error| error.message.contains("Unreachable terminal"))
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn absent_hidden_policy_is_an_explicit_native_v48_gap() {
    let observations: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/terminal-overrides/hidden-policy-observations.json"
    ))
    .unwrap();
    for case in ["absent-root", "inherited-absent"] {
        let row = observations
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case"] == case)
            .unwrap();
        assert_eq!(row["source_defines_hidden"], false);
        assert_eq!(row["flattened_defines_hidden"], true);
        assert_eq!(row["hidden"], serde_json::json!([]));
    }
    for materialized in [false, true] {
        let base = BASE.replace(" hidden(WS)", "");
        let errors = compiler::compile_bundle_with_profile(
            input(&base, CHILD, materialized),
            Profile::NativeV48,
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("requires an explicit hidden policy")),
            "{errors:?}"
        );
        // An explicit empty policy executes the identical token/model contract.
        let base = BASE.replace("hidden(WS)", "hidden()");
        verify_profile(
            input(&base, CHILD, materialized),
            "colorred;",
            "Red",
            Profile::NativeV48,
        );
        let package = compiler::compile_bundle_with_profile(
            input(&base, CHILD, materialized),
            Profile::NativeV48,
        )
        .unwrap();
        assert!(
            !evaluate(&package, &sources("color red;"))
                .diagnostics
                .is_empty()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn default_hidden_policy_matches_pinned_flattening() {
    default_hidden_policy_matches_pinned_flattening_with_profile(Profile::NativeV49);
}
fn default_hidden_policy_matches_pinned_flattening_with_profile(profile: Profile) {
    let observations: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/terminal-overrides/hidden-policy-observations.json"
    ))
    .unwrap();
    for row in observations.as_array().unwrap() {
        let case = row["case"].as_str().unwrap();
        let inherited = case.starts_with("inherited-") || case == "child-empty";
        let base = match case {
            "absent-root" | "inherited-absent" => BASE.replace(" hidden(WS)", ""),
            "empty-root" => BASE.replace("hidden(WS)", "hidden()"),
            _ => BASE.into(),
        };
        let child = if case == "child-empty" {
            CHILD.replace("with example.Colors", "with example.Colors hidden()")
        } else {
            CHILD.into()
        };
        let hidden = !row["hidden"].as_array().unwrap().is_empty();
        for materialized in [false, true] {
            let mut compilation = input(&base, &child, materialized);
            if !inherited {
                compilation.xtext = Some(compilation.bundle.sources[1].id.clone());
            }
            verify_profile(
                compilation.clone(),
                if hidden { "color red;" } else { "colorred;" },
                "Red",
                profile,
            );
            let package =
                compiler::compile_bundle_with_profile(compilation.clone(), profile).unwrap();
            assert_eq!(
                package.grammar.as_ref().unwrap().hidden.as_ref().unwrap(),
                &if hidden {
                    vec!["WS".to_string()]
                } else {
                    vec![]
                }
            );
            assert_eq!(
                evaluate(&package, &sources("color red;"))
                    .diagnostics
                    .is_empty(),
                hidden,
                "{case}"
            );
            assert!(
                evaluate(&package, &sources("colorred;"))
                    .diagnostics
                    .is_empty(),
                "{case}"
            );
            // Exact original bytes survive even though the effective policy is normalized.
            assert_eq!(
                package.compilation.as_ref().unwrap().bundle.sources,
                compilation.bundle.sources
            );
            assert_eq!(
                compilation.bundle.sources[1].text.contains("hidden("),
                case != "absent-root" && case != "inherited-absent"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn default_hidden_policy_preserves_missing_parent_and_invalid_token_rejections() {
    default_hidden_policy_preserves_missing_parent_and_invalid_token_rejections_with_profile(
        Profile::NativeV49,
    );
}
fn default_hidden_policy_preserves_missing_parent_and_invalid_token_rejections_with_profile(
    profile: Profile,
) {
    for materialized in [false, true] {
        for child in [
            CHILD.replace("with example.Colors", "with example.Missing"),
            CHILD.replace("with example.Colors", "with example.Colors hidden(Missing)"),
        ] {
            let errors =
                compiler::compile_bundle_with_profile(input(BASE, &child, materialized), profile)
                    .unwrap_err();
            assert!(
                errors.iter().any(|error| error
                    .message
                    .contains("Missing exact parent grammar dependency")
                    || error
                        .message
                        .contains("Unknown or fragment hidden terminal")),
                "{errors:?}"
            );
        }
    }
    let absent = BASE.replace(" hidden(WS)", "");
    let package =
        compiler::compile_bundle_with_profile(input(BASE, CHILD, false), profile).unwrap();
    let standalone =
        intersect_text::grammar::compile_with_profile(&absent, &package.schema, profile).unwrap();
    assert_eq!(standalone.hidden, Some(vec![]));
    assert!(
        intersect_text::grammar::compile_with_profile(&absent, &package.schema, Profile::NativeV48)
            .is_err()
    );
    let unavailable = absent.replace(
        "grammar example.Colors",
        "grammar example.Colors with org.eclipse.xtext.common.Terminals",
    );
    let error =
        intersect_text::grammar::compile_with_profile(&unavailable, &package.schema, profile)
            .unwrap_err();
    assert!(error.message.contains("requires an explicit hidden policy"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_hidden_policy_contracts() {
    default_hidden_policy_matches_pinned_flattening_with_profile(Profile::NativeV51);
    default_hidden_policy_preserves_missing_parent_and_invalid_token_rejections_with_profile(
        Profile::NativeV51,
    );
}
