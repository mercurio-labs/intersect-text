use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/explicit-lexical/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/explicit-lexical/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/explicit-lexical/model-behavior.rcore");
fn compile(text: &str) -> intersect_text::package::Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        text,
        RCORE_BEHAVIOR,
        Profile::NativeV1,
    )
    .unwrap()
}
fn source(text: &str) -> Source {
    Source {
        uri: "lexical.kerml".into(),
        text: text.into(),
    }
}
#[test]
fn pinned_terminal_slices_execute_with_explicit_hidden_policy_and_exact_spelling() {
    let package = compile(XTEXT);
    let input = [
        source(include_str!(
            "../../../../fixtures/xtext/explicit-lexical/example.kerml"
        )),
        Source {
            uri: "more.kerml".into(),
            text: include_str!("../../../../fixtures/xtext/explicit-lexical/more.kerml").into(),
        },
    ];
    let result = evaluate(&package, &input);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 5);
    assert_eq!(
        result.model.elements[0].values["documentation"],
        "/* significant documentation */"
    );
    assert_eq!(result.model.elements[1].values["name"], "'Wheel assembly'");
    assert_eq!(
        result.model.elements[1].values["description"],
        r#""A \"quoted\" wheel""#
    );
    assert_eq!(result.model.elements[2].values["target"], "lexical.kerml#1");
    assert_eq!(
        &input[0].text[result.references[0].start..result.references[0].end],
        "'Wheel assembly'"
    );
    let grammar = package.grammar.as_ref().unwrap();
    assert_eq!(
        grammar.hidden.as_deref(),
        Some(["WS".to_owned()].as_slice())
    );
    assert_eq!(grammar.terminals.len(), 5);
    for (name, terminal) in &grammar.terminals {
        assert_eq!(terminal.identity, format!("mini.ExplicitLexical::{name}"));
        assert!(XTEXT[terminal.start..terminal.end].starts_with(&format!("terminal {name}")));
    }
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(replay.sources["xtext"], XTEXT);
    assert_eq!(evaluate(&replay, &input).model, result.model);
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR
        )
        .is_err()
    );
}
#[test]
fn exact_whitespace_significant_comments_and_committed_quoted_failures() {
    let package = compile(XTEXT);
    for text in [
        "package Demo { type '😀'; }",
        " \t\r\npackage Demo /**/ { type Wheel; }",
        "package Demo /* first */ { type Wheel; }",
    ] {
        assert!(
            evaluate(&package, &[source(text)]).diagnostics.is_empty(),
            "{text}"
        );
    }
    for text in [
        "package\u{a0}Demo {}",
        "package\u{b}Demo {}",
        "package Demo { /* significant here */ }",
        "package Demo // not a declared note\n{}",
        "package Demo /* open",
        "package Demo { type 'open; }",
        r"package Demo { type '\q'; }",
        "package Demo { type package; }",
    ] {
        let result = evaluate(&package, &[source(text)]);
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty(), "{text}");
    }
    let no_hidden = compile(&XTEXT.replace("hidden(WS)", "hidden()"));
    assert!(
        !evaluate(&no_hidden, &[source("package Demo {}")])
            .diagnostics
            .is_empty()
    );
    let hidden_comments = compile(
        &XTEXT
            .replace("hidden(WS)", "hidden(WS, REGULAR_COMMENT)")
            .replace("(documentation=REGULAR_COMMENT)?", ""),
    );
    assert!(
        evaluate(
            &hidden_comments,
            &[source("package Demo { /* now explicitly hidden */ }")]
        )
        .diagnostics
        .is_empty()
    );
}
#[test]
fn acyclic_terminal_fragment_calls_share_the_same_utf16_matcher() {
    let text = XTEXT
        .replace("('a'..'z' | 'A'..'Z' | '_')", "LETTER")
        .replace(
            "('a'..'z' | 'A'..'Z' | '_' | '0'..'9')*",
            "(LETTER | '0'..'9')*",
        )
        + "\nterminal fragment LETTER: 'a'..'z' | 'A'..'Z' | '_';";
    let package = compile(&text);
    let grammar = package.grammar.as_ref().unwrap();
    assert!(grammar.terminals["LETTER"].fragment);
    assert_eq!(grammar.terminals["ID"].match_len("Abc2;"), Ok(4));
    assert!(
        evaluate(&package, &[source("package Demo { type Wheel2; }")])
            .diagnostics
            .is_empty()
    );
    assert_eq!(
        grammar.terminals["REGULAR_COMMENT"].match_len("/*😀*/tail"),
        Ok(8)
    );
    assert_eq!(
        grammar.terminals["STRING_VALUE"].match_len("\"😀\"tail"),
        Ok(6)
    );
    for value in [
        text.replace(
            "terminal fragment LETTER: 'a'..'z' | 'A'..'Z' | '_';",
            "terminal fragment LETTER: ID;",
        ),
        text.replace("name=ID", "name=LETTER"),
        text.replace("hidden(WS)", "hidden(LETTER)"),
        text.replace("LETTER | '0'..'9'", "MISSING | '0'..'9'"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &value,
                RCORE_BEHAVIOR,
                Profile::NativeV1
            )
            .is_err()
        );
    }
}
#[test]
fn unsupported_arbitration_and_incomplete_lexical_contracts_remain_explicit() {
    for text in [
        XTEXT.replace("hidden(WS)", "hidden(MISSING)"),
        XTEXT.replace("hidden(WS)", "hidden(WS, WS)"),
        XTEXT.replace(
            "hidden(WS)",
            "with org.eclipse.xtext.common.Terminals hidden(WS)",
        ),
        XTEXT.replace("name=ID", "name=INT"),
        XTEXT.replace("name=ID", "name=WS"),
        format!("{XTEXT}\nterminal ML_NOTE: '//*' -> '*/';"),
        format!("{XTEXT}\nterminal OTHER: 'a'+;"),
        format!("{XTEXT}\nterminal BAD: '@' -> '';"),
        format!("{XTEXT}\nterminal BAD: '@' !'ab';"),
        format!("{XTEXT}\nterminal BAD: '@' (''*);"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &text,
                RCORE_BEHAVIOR,
                Profile::NativeV1
            )
            .is_err(),
            "{text}"
        );
    }
}

#[test]
fn until_search_and_terminal_call_expansion_remain_bounded() {
    let text = format!("{XTEXT}\nterminal LONG: '@' -> '{}b';", "a".repeat(1000));
    let package = compile(&text);
    let terminal = &package.grammar.as_ref().unwrap().terminals["LONG"];
    let input = format!("@{}b!", "a".repeat(100_000));
    assert_eq!(terminal.match_len(&input), Ok(100_002));
    assert!(terminal.match_len(&input[..100_001]).is_err());
    let mut chain = XTEXT.to_owned();
    for index in 0..66 {
        chain.push_str(&format!("\nterminal fragment F{index}: F{};", index + 1));
    }
    chain.push_str("\nterminal fragment F66: '@';");
    let errors =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &chain,
            RCORE_BEHAVIOR,
            Profile::NativeV1,
        )
        .unwrap_err();
    assert!(format!("{errors:?}").contains("nesting budget"));
}

#[test]
fn cached_pinned_body_controls_match_the_shared_recognizer() {
    let package = compile(XTEXT);
    let grammar = package.grammar.as_ref().unwrap();
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/explicit-lexical/terminal-body-controls.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 735);
    for name in [
        "ID",
        "UNRESTRICTED_NAME",
        "STRING_VALUE",
        "REGULAR_COMMENT",
        "WS",
    ] {
        assert_eq!(
            cases.iter().filter(|case| case["rule"] == name).count(),
            147,
            "{name}"
        );
    }
    for case in cases {
        let name = case["rule"].as_str().unwrap();
        let source = case["source"].as_str().unwrap();
        let actual = grammar.terminals[name]
            .match_len(source)
            .ok()
            .map(|bytes| source[..bytes].encode_utf16().count());
        let expected = case["length_utf16"].as_u64().map(|n| n as usize);
        assert_eq!(actual, expected, "{name}: {source:?}");
    }
}
