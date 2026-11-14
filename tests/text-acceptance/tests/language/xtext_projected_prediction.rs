//! Independently authored shared-prefix qualification; not a KerML substitute.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::GrammarAnalysisWorkBudget;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/projected-prediction/model.xtext");
const NARY: &str = include_str!("../../../../fixtures/xtext/projected-prediction/nary.xtext");

fn input(grammar: &str, materialized: bool) -> BundleCompilation {
    let ecore = DefinitionSource::new(
        "model.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/xtext/current-actions/model.ecore"),
    );
    let mut xtext = DefinitionSource::new("projected.xtext", SourceKind::Xtext, grammar);
    xtext.dependencies.push(ecore.id.clone());
    let mut behavior = DefinitionSource::new(
        "behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package projected version \"1\";",
    );
    behavior.dependencies.push(ecore.id.clone());
    let mut compilation = BundleCompilation {
        ecore: ecore.id.clone(),
        xtext: Some(xtext.id.clone()),
        rcore_behavior: behavior.id.clone(),
        namespace: "urn:intersect:xtext:current-actions:1".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, xtext, behavior],
            namespaces: vec![],
        },
    };
    if materialized {
        for index in 0..4 {
            compilation.materialized_types.push(
                intersect_model::definitions::generic_types::RuntimeType::new(
                    intersect_model::definitions::generic_types::Type::Classifier {
                        declaration: intersect_model::definitions::resources::DeclarationId {
                            source: compilation.ecore.clone(),
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
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(compilation).unwrap()
}
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "projected.model".into(),
        text: text.into(),
    }]
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn projected_proof_constructs_models_with_exact_sources_replay_and_recovery() {
    check_models(
        GRAMMAR,
        Profile::NativeV52,
        Profile::NativeV53,
        &[
            ("left", "left"),
            ("right", "right"),
            ("x x x x x x x x x x :: n left", "left"),
            ("y x y x y x y x y x $ :: m right", "right"),
            ("y y y y y y y y y y :: n right", "right"),
        ],
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn nary_certificates_construct_all_branches_and_preserve_replay_and_recovery() {
    use sha2::{Digest, Sha256};
    assert_eq!(NARY.len(), 2166);
    assert_eq!(
        format!("{:x}", Sha256::digest(NARY.as_bytes())),
        "e1d3d0f9f3acc97cdab352310c49bdae0cce41dc0160efec01b9948c32600938"
    );
    check_models(
        NARY,
        Profile::NativeV53,
        Profile::NativeV54,
        &[
            ("branch0", "branch0"),
            ("branch1", "branch1"),
            ("branch2", "branch2"),
            ("branch3", "branch3"),
            ("branch4", "branch4"),
            ("branch5", "branch5"),
            ("branch6", "branch6"),
            ("branch7", "branch7"),
            ("shared branch8", "branch8"),
            ("shared branch9", "branch9"),
            ("x x x x x x x x x x :: n branch0", "branch0"),
            ("y x y x y x y x y x $ :: m shared branch9", "branch9"),
        ],
    );
}

fn check_models(grammar: &str, baseline: Profile, profile: Profile, cases: &[(&str, &str)]) {
    check_models_with_trailing(grammar, baseline, profile, cases, 0);
}
fn check_models_with_trailing(
    grammar: &str,
    baseline: Profile,
    profile: Profile,
    cases: &[(&str, &str)],
    trailing: usize,
) {
    let mut ordinary = None;
    for materialized in [false, true] {
        let compilation = input(grammar, materialized);
        let exact = compilation.bundle.sources.clone();
        let failed =
            compiler::compile_bundle_with_profile(compilation.clone(), baseline).unwrap_err();
        assert!(
            format!("{failed:?}").contains("Grammar prediction budget exceeded"),
            "{failed:?}"
        );
        let package = compiler::compile_bundle_with_profile(compilation, profile).unwrap();
        assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
        assert_eq!(package.grammar.as_ref().unwrap().profile, profile);
        assert_eq!(
            package.grammar.as_ref().unwrap().declaration_sources["Head0"].source,
            exact[1].id
        );
        let replay = compiler::recompile(&package).unwrap();
        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(&package).unwrap()
        );
        let mut outputs = Vec::new();
        for &(text, expected) in cases {
            let result = evaluate(&package, &source(text));
            assert!(
                result.diagnostics.is_empty(),
                "{text}: {:?}",
                result.diagnostics
            );
            assert_eq!(result.model.elements.len(), 1);
            let root = &result.model.elements[0];
            assert_eq!(root.id, "projected.model#0");
            assert_eq!(root.resource, "projected.model");
            let declaration = format!("{}@{}#3", exact[0].id.uri, exact[0].id.sha256);
            assert_eq!(root.type_identity, declaration);
            assert_eq!(package.schema.identity(&root.class), declaration);
            assert_eq!(result.model.schema_revision, package.schema.revision);
            assert_eq!(root.values["operator"], expected);
            assert_eq!(root.values["operand"], serde_json::json!([]));
            assert_eq!(root.values.len(), 2);
            assert_eq!((root.start, root.end), (0, text.len() - trailing));
            result.model.validate(&package.schema).unwrap();
            assert_eq!(evaluate(&replay, &source(text)).model, result.model);
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
                let failed = evaluate_with_limits(&package, &source(text), limits);
                assert!(!failed.diagnostics.is_empty());
                assert!(failed.model.elements.is_empty());
            }
            assert_eq!(evaluate(&package, &source(text)).model, result.model);
            // Runtime materialization intentionally has its own schema revision
            // and concrete class key. Compare every semantic field plus the
            // original exact declaration, after validating each representation.
            outputs.push((
                root.id.clone(),
                root.resource.clone(),
                root.start,
                root.end,
                root.type_identity.clone(),
                root.values.clone(),
            ));
        }
        if let Some(expected) = &ordinary {
            assert_eq!(expected, &outputs);
        } else {
            ordinary = Some(outputs);
        }
        for text in [
            format!("x x x x x x x x x :: n {}", cases[0].0),
            format!("x x x x x x x x x x {}", cases[0].0),
            format!("x x x x x x x x x x :: bad {}", cases[0].0),
            format!("{} trailing", cases[0].0),
        ] {
            let failed = evaluate(&package, &source(&text));
            assert!(!failed.diagnostics.is_empty(), "{text}");
            assert!(failed.model.elements.is_empty());
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn nary_certificates_reject_ambiguity_and_left_recursion_then_recover() {
    for materialized in [false, true] {
        for (grammar, diagnostic) in [
            (
                NARY.replace("operator='branch9'", "operator='branch8'"),
                "Ambiguous parser prediction",
            ),
            (
                NARY.replace(
                    "Branch0 returns k::OperatorExpression: Head0?",
                    "Branch0 returns k::OperatorExpression: Branch0",
                ),
                "Left recursion",
            ),
        ] {
            let failed = compiler::compile_bundle_with_profile(
                input(&grammar, materialized),
                Profile::NativeV54,
            )
            .unwrap_err();
            assert!(format!("{failed:?}").contains(diagnostic), "{failed:?}");
        }
        assert!(
            compiler::compile_bundle_with_profile(input(NARY, materialized), Profile::NativeV54)
                .is_ok()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn projected_proof_keeps_ambiguity_and_left_recursion_explicit() {
    for materialized in [false, true] {
        let ambiguous = GRAMMAR.replace("Head0? operator='right'", "Head0? operator='left'");
        let failed = compiler::compile_bundle_with_profile(
            input(&ambiguous, materialized),
            Profile::NativeV53,
        )
        .unwrap_err();
        assert!(
            format!("{failed:?}").contains("Ambiguous parser prediction"),
            "{failed:?}"
        );
        let recursive = GRAMMAR.replace(
            "Left returns k::OperatorExpression: Head0?",
            "Left returns k::OperatorExpression: Left",
        );
        let failed = compiler::compile_bundle_with_profile(
            input(&recursive, materialized),
            Profile::NativeV53,
        )
        .unwrap_err();
        assert!(
            format!("{failed:?}").contains("Left recursion"),
            "{failed:?}"
        );
        assert!(
            compiler::compile_bundle_with_profile(input(GRAMMAR, materialized), Profile::NativeV53)
                .is_ok()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn projected_profile_keeps_v52_string_conversion_and_scalar_boundaries() {
    let grammar = r#"grammar strings hidden(WS) import "urn:intersect:xtext:current-actions:1" as k
Root returns k::OperatorExpression: operator=STRING;
terminal STRING: '"' ('\\' . | !('\\'|'"'))* '"';
terminal WS: ' '+;
"#;
    for materialized in [false, true] {
        for profile in [
            Profile::NativeV53,
            Profile::NativeV54,
            Profile::NativeV55,
            Profile::NativeV56,
        ] {
            let package =
                compiler::compile_bundle_with_profile(input(grammar, materialized), profile)
                    .unwrap();
            for (text, expected) in [
                (r#""line\nnext""#, "line\nnext"),
                (r#""\uD83D\uDE80""#, "🚀"),
            ] {
                let result = evaluate(&package, &source(text));
                assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
                assert_eq!(result.model.elements[0].values["operator"], expected);
            }
            for text in [r#""\q""#, r#""\uD800""#] {
                let failed = evaluate(&package, &source(text));
                assert!(!failed.diagnostics.is_empty());
                assert!(failed.model.elements.is_empty());
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn projected_token_masks_above_64_bits_preserve_existing_models() {
    let keywords = (0..90)
        .map(|index| format!("'k{index}'"))
        .collect::<Vec<_>>()
        .join(" | ");
    let grammar = format!(
        "grammar wide hidden(WS) import \"urn:intersect:xtext:current-actions:1\" as k\n\
        Root returns k::Expression: Left | Right;\n\
        Left returns k::OperatorExpression: Shared? operator='left';\n\
        Right returns k::OperatorExpression: Shared? operator='right';\n\
        fragment Shared returns k::OperatorExpression: ({keywords});\n\
        terminal WS: ' '+;"
    );
    for materialized in [false, true] {
        for profile in [
            Profile::NativeV53,
            Profile::NativeV54,
            Profile::NativeV55,
            Profile::NativeV56,
        ] {
            let old = compiler::compile_bundle_with_profile(
                input(&grammar, materialized),
                Profile::NativeV52,
            )
            .unwrap();
            let new = compiler::compile_bundle_with_profile(input(&grammar, materialized), profile)
                .unwrap();
            for (text, expected) in [
                ("k65 left", "left"),
                ("k89 right", "right"),
                ("left", "left"),
            ] {
                let actual = evaluate(&new, &source(text));
                assert!(actual.diagnostics.is_empty(), "{:?}", actual.diagnostics);
                assert_eq!(actual.model.elements[0].values["operator"], expected);
                assert_eq!(actual.model, evaluate(&old, &source(text)).model);
            }
        }
    }
}

const FILTERED: &str =
    include_str!("../../../../fixtures/xtext/projected-prediction/filtered.xtext");
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn filtered_repeat_constructs_literal_models_and_preserves_replay_recovery() {
    use sha2::Digest as _;
    assert_eq!(FILTERED.len(), 1621);
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(FILTERED.as_bytes())),
        "f5aaa853d7c2021e3307ee3cf2bca71065f9cc0a91e1ef72b7e2099bdf309cf2"
    );
    check_models_with_trailing(
        FILTERED,
        Profile::NativeV54,
        Profile::NativeV55,
        &[
            ("( left )", "left"),
            ("( right )", "right"),
            ("( left left right )", "right"),
            ("( x x x x x x x x x x :: n left )", "left"),
            ("( y x y x y x y x y x $ :: m right )", "right"),
            ("( y y y y y y y y y y :: n left right )", "right"),
        ],
        2,
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn filtered_repeat_rejects_ambiguity_and_recursion_then_recovers() {
    for materialized in [false, true] {
        for (grammar, diagnostic) in [
            (
                FILTERED.replace("operator='right'", "operator='left'"),
                "prediction",
            ),
            (
                FILTERED.replace(
                    "Root returns k::Expression: Body ')';",
                    "Root returns k::Expression: Root;",
                ),
                "Grammar prediction budget exceeded",
            ),
        ] {
            let failed = compiler::compile_bundle_with_profile(
                input(&grammar, materialized),
                Profile::NativeV55,
            )
            .unwrap_err();
            assert!(format!("{failed:?}").contains(diagnostic), "{failed:?}");
        }
        let package = compiler::compile_bundle_with_profile(
            input(FILTERED, materialized),
            Profile::NativeV55,
        )
        .unwrap();
        let empty = evaluate(&package, &source("( )"));
        assert!(empty.diagnostics.is_empty(), "{:?}", empty.diagnostics);
        assert_eq!(empty.model.elements.len(), 1);
        assert_eq!(
            empty.model.elements[0].values["operator"],
            serde_json::Value::Null
        );
        assert_eq!(
            (empty.model.elements[0].start, empty.model.elements[0].end),
            (0, 1)
        );
        for text in [
            "( left",
            "left )",
            "( right left )",
            "( x x :: n left )",
            "( x x x x x x x x x x :: bad right )",
        ] {
            let failed = evaluate(&package, &source(text));
            assert!(!failed.diagnostics.is_empty(), "{text}");
            assert!(failed.model.elements.is_empty());
        }
        assert!(
            evaluate(&package, &source("( left right )"))
                .diagnostics
                .is_empty()
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn filtered_profile_preserves_prior_projected_and_nary_models() {
    for profile in [Profile::NativeV55, Profile::NativeV56] {
        check_models(
            GRAMMAR,
            Profile::NativeV52,
            profile,
            &[
                ("left", "left"),
                ("right", "right"),
                ("x x x x x x x x x x :: n right", "right"),
            ],
        );
        check_models(
            NARY,
            Profile::NativeV53,
            profile,
            &[
                ("branch0", "branch0"),
                ("shared branch9", "branch9"),
                ("y x y x y x y x y x $ :: m shared branch9", "branch9"),
            ],
        );
    }
}

const DISCOVERY: &str =
    include_str!("../../../../fixtures/xtext/projected-prediction/discovery.xtext");
fn segmented_prefix(sections: usize, width: usize) -> String {
    let mut grammar = String::from(
        "grammar discoveryPrediction hidden(WS)\nimport \"urn:intersect:xtext:current-actions:1\" as k\nRoot returns k::Expression: Left | Right;\nLeft returns k::OperatorExpression: Head0 operator='left';\nRight returns k::OperatorExpression: Head0 operator='right';\nfragment Head0 returns k::OperatorExpression:",
    );
    for index in 0..sections {
        grammar.push_str(&format!(" Segment{index}"));
    }
    grammar.push_str(";\n");
    for index in 0..sections {
        grammar.push_str(&format!(
            "fragment Segment{index} returns k::OperatorExpression: ("
        ));
        for token in 0..width {
            if token > 0 {
                grammar.push_str(" | ");
            }
            grammar.push_str(&format!("'g{index}v{token}'"));
        }
        grammar.push_str(")?;\n");
    }
    grammar.push_str("terminal WS: ' '+;\n");
    grammar
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn bounded_discovery_executes_literal_models_with_exact_sources_and_recovery() {
    use sha2::Digest as _;
    assert_eq!(DISCOVERY.len(), 4215);
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(DISCOVERY.as_bytes())),
        "3b042d8e4868cc2baac21f5eb876d1592dd9397fbf70143e10d18edc28719666"
    );
    assert_eq!(DISCOVERY, segmented_prefix(48, 2));
    let complete = format!(
        "{} left",
        (0..48)
            .map(|i| format!("g{i}v{}", i % 2))
            .collect::<Vec<_>>()
            .join(" ")
    );
    check_models(
        DISCOVERY,
        Profile::NativeV55,
        Profile::NativeV56,
        &[
            ("left", "left"),
            ("right", "right"),
            ("g0v0 g47v1 right", "right"),
            ("g47v0 left", "left"),
            (&complete, "left"),
        ],
    );
    check_models_with_trailing(
        FILTERED,
        Profile::NativeV54,
        Profile::NativeV56,
        &[
            ("( left )", "left"),
            ("( left right )", "right"),
            ("( x x x x x x x x x x :: n left )", "left"),
        ],
        2,
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn bounded_discovery_keeps_nullable_family_and_rejects_unfenced_overlaps() {
    let mut retained = 0;
    for sections in [4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 56] {
        for width in [2, 4] {
            let grammar = segmented_prefix(sections, width);
            let compilation = input(&grammar, false);
            let exact = compilation.bundle.sources.clone();
            let package =
                compiler::compile_bundle_with_profile(compilation.clone(), Profile::NativeV56)
                    .unwrap();
            assert_eq!(package.compilation.as_ref().unwrap().bundle.sources, exact);
            let text = format!("g0v0 g{}v{} right", sections - 1, width - 1);
            let actual = evaluate(&package, &source(&text));
            assert!(
                actual.diagnostics.is_empty(),
                "{sections}/{width}: {:?}",
                actual.diagnostics
            );
            assert_eq!(actual.model.elements.len(), 1);
            assert_eq!(actual.model.elements[0].values["operator"], "right");
            match compiler::compile_bundle_with_profile(compilation, Profile::NativeV55) {
                Ok(previous) => {
                    assert_eq!(evaluate(&previous, &source(&text)).model, actual.model);
                    retained += 1;
                }
                Err(errors) => {
                    assert!(format!("{errors:?}").contains("Grammar prediction budget exceeded"))
                }
            }
        }
    }
    assert_eq!(retained, 16);
    for materialized in [false, true] {
        // The 'g0v0' -> 'left' overlap is a historical capacity rejection: it
        // requests the historical 10k grant explicitly (the default grant has
        // been REALISTIC since 1e5cd75b). The other two reject on their own.
        for (grammar, budget) in [
            (
                DISCOVERY.replace("operator='right'", "operator='left'"),
                None,
            ),
            (
                DISCOVERY.replace("'g0v0'", "'left'"),
                Some(GrammarAnalysisWorkBudget::LEGACY_10K),
            ),
            (
                DISCOVERY.replace(
                    "Left returns k::OperatorExpression: Head0",
                    "Left returns k::OperatorExpression: Left",
                ),
                None,
            ),
        ] {
            let compilation = input(&grammar, materialized);
            let errors = match budget {
                None => compiler::compile_bundle_with_profile(compilation, Profile::NativeV56),
                Some(budget) => {
                    let policy = Default::default();
                    compiler::compile_bundle_with_context_and_budget(
                        compilation,
                        Profile::NativeV56,
                        &[],
                        None,
                        policy,
                        policy,
                        None,
                        None,
                        budget,
                    )
                }
            }
            .unwrap_err();
            let message = format!("{errors:?}");
            assert!(
                message.contains("prediction") || message.contains("Left recursion"),
                "{message}"
            );
        }
        let package = compiler::compile_bundle_with_profile(
            input(DISCOVERY, materialized),
            Profile::NativeV56,
        )
        .unwrap();
        for text in [
            "g1v0 g0v0 left",
            "g0v0 g0v1 right",
            "g48v0 left",
            "g0v2 left",
            "g47v1",
            "left trailing",
        ] {
            let failed = evaluate(&package, &source(text));
            assert!(!failed.diagnostics.is_empty(), "{text}");
            assert!(failed.model.elements.is_empty());
        }
        assert!(
            evaluate(&package, &source("g0v0 g47v1 right"))
                .diagnostics
                .is_empty()
        );
    }
}
