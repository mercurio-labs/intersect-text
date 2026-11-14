//! Authored X04 controls, not imported KerML or upstream observations.
use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/terminal-spellings/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/terminal-spellings/model-behavior.rcore");
fn grammar(body: &str) -> String {
    format!(
        "grammar audit.Terminals hidden() import \"urn:intersect:xtext:mini-kerml:terminal-spellings:1\" as k Model returns k::Model: name=VALUE; terminal VALUE: {body};"
    )
}
fn check(body: &str, positives: &[&str], negatives: &[&str]) {
    let text = grammar(body);
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &text,
            RCORE_BEHAVIOR,
            Profile::NativeV13,
        )
        .unwrap();
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(replay.sources["xtext"], text);
    let terminal = &package.grammar.as_ref().unwrap().terminals["VALUE"];
    assert_eq!(terminal.identity, "audit.Terminals::VALUE");
    assert!(text[terminal.start..terminal.end].starts_with("terminal VALUE:"));
    for input in positives {
        assert_eq!(terminal.match_len(input).unwrap(), input.len());
        let sources = [Source {
            uri: "audit.xtext-model".into(),
            text: (*input).into(),
        }];
        let result = evaluate(&package, &sources);
        assert!(
            result.diagnostics.is_empty(),
            "{input}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.model.elements.len(), 1);
        assert_eq!(result.model.elements[0].values["name"], *input);
        assert_eq!(result.model, evaluate(&replay, &sources).model);
    }
    for input in negatives {
        let result = evaluate(
            &package,
            &[Source {
                uri: "audit.xtext-model".into(),
                text: (*input).into(),
            }],
        );
        assert!(!result.diagnostics.is_empty(), "accepted {input}");
        assert!(
            result.model.elements.is_empty(),
            "partial model for {input}"
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn shared_prefixes_nested_groups_and_range_boundaries() {
    check(
        "'@' ('ab' | 'ac') ('0'..'9')?",
        &["@ab", "@ac", "@ab0", "@ac9"],
        &["@a", "@ad", "@ab/", "@ac:"],
    );
    check(
        "'@' (('\u{03b1}b' | '\u{03b1}c') | ('ab' | 'ac'))+",
        &["@\u{03b1}b", "@\u{03b1}cabac"],
        &["@", "@\u{03b1}", "@\u{03b1}d"],
    );
    check(
        "'@' ('a' | 'ab')+ 'b'",
        &["@ab", "@abb", "@abab"],
        &["@a", "@aba"],
    );
    check(
        "'@' PART 'b'; terminal fragment PART: ('a' | 'abx')",
        &["@ab", "@abxb"],
        &["@abx"],
    );
    check("'@' ('ab' | 'abc')", &["@ab", "@abc"], &["@a", "@abd"]);
    check(
        "'@' ('\u{1f600}x' | '\u{1f600}y')",
        &["@\u{1f600}x", "@\u{1f600}y"],
        &["@\u{1f600}", "@\u{1f600}z"],
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn nullable_branch_and_committed_failure_controls() {
    check("'@' (('ab')? | 'c')", &["@", "@ab", "@c"], &["@a", "@ac"]);
    check("'@' ('a' | 'ab') 'b'", &["@ab", "@abb"], &["@a", "@aba"]);
    check("'@' ('a' | 'abx') 'b'", &["@ab", "@abxb"], &["@abx"]);
    check("'@' ('abc' | 'abd')", &["@abc", "@abd"], &["@ab", "@abe"]);
    // Incomplete selected suffixes cannot fall back to a shorter success.
    check("'@' ('a' | 'ab' 'c')", &["@a", "@abc"], &["@ab", "@abx"]);
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn old_profiles_and_unsupported_nullable_repetition_stay_rejected() {
    let text = grammar("'@' ('ab' | 'ac')");
    for profile in [Profile::NativeV1, Profile::NativeV11, Profile::NativeV12] {
        let errors =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &text,
                RCORE_BEHAVIOR,
                profile,
            )
            .unwrap_err();
        assert!(
            errors.iter().any(|e| e
                .message
                .contains("Overlapping or nullable terminal alternatives")),
            "{errors:?}"
        );
    }
    for body in [
        "('' | 'ab')",
        "'@' ('' | 'ab')*",
        "'@' ('a')* 'a'",
        "'@' ('z'..'a')",
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar(body),
                RCORE_BEHAVIOR,
                Profile::NativeV13
            )
            .is_err(),
            "{body}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn wildcard_and_eof_preserve_boundaries_and_reject_trailing_input() {
    check("'@' . EOF", &["@a", "@\n"], &["@", "@ab"]);
    check("'@' .+ EOF", &["@abc", "@\u{1f600}", "@a\nb"], &["@"]);
    check("'@' ('a' EOF | 'ab')", &["@a", "@ab"], &["@ac", "@abc"]);
    for body in ["'@' .", "'@' EOF"] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar(body),
                RCORE_BEHAVIOR,
                Profile::NativeV12
            )
            .is_err()
        );
    }
    for body in ["EOF", "EOF*", "'@' EOF+", "'@' !EOF"] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar(body),
                RCORE_BEHAVIOR,
                Profile::NativeV13
            )
            .is_err()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn every_internal_prediction_budget_cutoff_is_reported() {
    let text = grammar("'@' ('a' | 'abx') 'b'");
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &text,
            RCORE_BEHAVIOR,
            Profile::NativeV13,
        )
        .unwrap();
    let terminal = &package.grammar.as_ref().unwrap().terminals["VALUE"];
    let mut budget = Some(10000);
    assert_eq!(
        terminal.match_len_with_work("@abxb", &mut budget).unwrap(),
        5
    );
    let used = 10000 - budget.unwrap();
    assert!(used > 0);
    for amount in 0..used {
        let error = terminal
            .match_len_with_work("@abxb", &mut Some(amount))
            .unwrap_err();
        assert!(error.contains("step budget exceeded"), "{amount}: {error}");
    }
    assert_eq!(
        terminal
            .match_len_with_work("@abxb", &mut Some(used))
            .unwrap(),
        5
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn cached_independent_lexer_observations_match() {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/terminal-decisions/observations.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let text = grammar(row["body"].as_str().unwrap());
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &text,
                RCORE_BEHAVIOR,
                Profile::NativeV13,
            )
            .unwrap();
        let input = row["text"].as_str().unwrap();
        let result = evaluate(
            &package,
            &[Source {
                uri: "oracle.model".into(),
                text: input.into(),
            }],
        );
        if row["body"] == "'@' . EOF" && input == "@" {
            // ANTLR 3 matchAny accepts EOF without consuming a character.
            // Preserve the observed disagreement; native wildcard requires a
            // UTF-16 unit. This blocks general X05 compatibility qualification.
            assert_eq!(row["accepted"], true);
            assert!(!result.diagnostics.is_empty());
            assert!(result.model.elements.is_empty());
        } else {
            assert_eq!(
                result.diagnostics.is_empty(),
                row["accepted"].as_bool().unwrap(),
                "{row}: {:?}",
                result.diagnostics
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn saved_example_and_language_limits_use_the_shared_path() {
    use intersect_text::{EvaluationLimits, evaluate_with_limits};
    let text = include_str!("../../../../fixtures/xtext/terminal-decisions/model.xtext");
    let input = include_str!("../../../../fixtures/xtext/terminal-decisions/example.model");
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            text,
            RCORE_BEHAVIOR,
            Profile::NativeV13,
        )
        .unwrap();
    let sources = [Source {
        uri: "example.model".into(),
        text: input.into(),
    }];
    let limits = EvaluationLimits {
        resources: Some(1),
        input_bytes: Some(input.len()),
        elements_per_resource: Some(1),
        ..EvaluationLimits::native()
    };
    let result = evaluate_with_limits(&package, &sources, limits);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[0].values["name"], "@abxb");
    assert_eq!(result.model, evaluate(&package, &sources).model);
    for restricted in [
        EvaluationLimits {
            input_bytes: Some(input.len() - 1),
            ..limits
        },
        EvaluationLimits {
            elements_per_resource: Some(0),
            ..limits
        },
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..limits
        },
    ] {
        let rejected = evaluate_with_limits(&package, &sources, restricted);
        assert!(!rejected.diagnostics.is_empty());
        assert!(rejected.model.elements.is_empty());
    }
}
