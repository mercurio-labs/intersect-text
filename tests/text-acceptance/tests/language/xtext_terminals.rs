use intersect_text::grammar;
use intersect_text::grammar::Datatype;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/terminal-spellings/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/terminal-spellings/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/terminal-spellings/model-behavior.rcore");
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
        uri: "terminals.kerml".into(),
        text: text.into(),
    }
}
#[test]
fn terminal_attributes_and_datatype_reference_spellings_keep_identity_and_replay() {
    let package = compile(XTEXT);
    let input = [
        source(include_str!(
            "../../../../fixtures/xtext/terminal-spellings/example.kerml"
        )),
        Source {
            uri: "more.kerml".into(),
            text: include_str!("../../../../fixtures/xtext/terminal-spellings/more.kerml").into(),
        },
    ];
    let result = evaluate(&package, &input);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 5);
    for (index, id, class, name) in [
        (0, "terminals.kerml#0", "Model", "Demo"),
        (1, "terminals.kerml#1", "Type", "@Wheel"),
        (2, "terminals.kerml#2", "Feature", "wheel"),
        (3, "more.kerml#0", "Model", "Library"),
        (4, "more.kerml#1", "Type", "@Spare"),
    ] {
        let element = &result.model.elements[index];
        assert_eq!(element.id, id);
        assert_eq!(element.class, class);
        assert_eq!(element.values["name"], name);
        assert_eq!(element.type_identity, package.schema.identity(class));
    }
    assert_eq!(
        result.model.elements[2].values["target"],
        "terminals.kerml#1"
    );
    assert_eq!(result.references.len(), 1);
    assert_eq!(
        &input[0].text[result.references[0].start..result.references[0].end],
        "@Wheel"
    );
    assert_eq!(package.sources["xtext"], XTEXT);
    let terminal = &package.grammar.as_ref().unwrap().terminals["TAG"];
    assert_eq!(terminal.identity, "mini.TerminalSpellings::TAG");
    assert_eq!(terminal.datatype, Datatype::EString);
    assert!(XTEXT[terminal.start..terminal.end].starts_with("terminal TAG"));
    assert!(XTEXT[terminal.start..terminal.end].ends_with(';'));
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(evaluate(&replay, &input).model, result.model);
    let renamed = compile(&XTEXT.replace("TAG", "SYMBOL").replace("Name", "Spelling"));
    let renamed_result = evaluate(&renamed, &input);
    assert!(renamed_result.diagnostics.is_empty());
    assert_eq!(
        renamed_result
            .model
            .elements
            .iter()
            .map(|e| &e.values)
            .collect::<Vec<_>>(),
        result
            .model
            .elements
            .iter()
            .map(|e| &e.values)
            .collect::<Vec<_>>()
    );
    let implicit = compile(&XTEXT.replace("terminal TAG returns ecore::EString:", "terminal TAG:"));
    assert!(evaluate(&implicit, &input).diagnostics.is_empty());
}
#[test]
fn committed_terminal_matching_preserves_utf16_boundaries_and_rejects_incomplete_tokens() {
    let header = XTEXT.split("terminal TAG").next().unwrap();
    for (body, text, expected) in [
        ("'@' '😀'", "@😀tail", 5),
        ("'@' ('a'..'z')+ ('0'..'9')?", "@abc2!", 5),
        ("'@' ('ab')? 'c'", "@abc!", 4),
        ("'@' ('ab')? 'c'", "@c!", 2),
        ("'@' ('a'|'b')+", "@abba!", 5),
    ] {
        let package = compile(&format!("{header}terminal TAG: {body};"));
        let terminal = &package.grammar.as_ref().unwrap().terminals["TAG"];
        assert_eq!(terminal.match_len(text).unwrap(), expected, "{body}");
    }
    let package = compile(&format!("{header}terminal TAG: '@' ('ab')? 'c';"));
    let terminal = &package.grammar.as_ref().unwrap().terminals["TAG"];
    assert!(terminal.match_len("@ac").is_err()); // entered optional body cannot silently disappear
    for text in [
        "package Demo { type @; }",
        "package Demo { type @9; }",
        "package Demo { type @/*gap*/Wheel; }",
    ] {
        let result = evaluate(&compile(XTEXT), &[source(text)]);
        assert!(result.model.elements.is_empty());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("terminal")),
            "{:?}",
            result.diagnostics
        );
        assert_eq!(
            &text[result.diagnostics[0].start..result.diagnostics[0].start + 1],
            "@"
        );
    }
}
#[test]
fn unsupported_terminal_bodies_and_lexical_conflicts_reject_explicitly() {
    let header = XTEXT.split("terminal TAG").next().unwrap();
    for body in [
        "'z'..'a'",
        "'ab'..'z'",
        "'@'?",
        "'@' (''*)",
        "'@' ('a'|'ab')",
        "'@' ('a')* 'a'",
        "'a'+",
        "'/' 'x'",
        "'@' -> ';'",
        "'@' !'x'",
        "'@' TAG",
        ".",
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &format!("{header}terminal TAG: {body};"),
                RCORE_BEHAVIOR,
                Profile::NativeV1
            )
            .is_err(),
            "{body}"
        );
    }
    for grammar in [
        XTEXT.replace("terminal TAG", "terminal fragment TAG"),
        XTEXT.replace(
            "terminal TAG returns ecore::EString",
            "terminal TAG returns ecore::EInt",
        ),
        format!("{XTEXT}\nterminal OTHER: '@' 'z';"),
        format!("{XTEXT}\nterminal ID: '@' 'z';"),
        XTEXT.replace("'type' name=TAG", "'@' name=TAG"),
        format!("{XTEXT}\nTAG returns k::Type: 'extra' name=ID;"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV1
            )
            .is_err(),
            "{grammar}"
        );
    }
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
fn new_terminals_share_existing_token_rule_and_object_budgets() {
    let package = compile(XTEXT);
    let input = [source(
        "package Demo { type @Wheel; feature wheel : @Wheel; }",
    )];
    let count = grammar::lex_borrowed_with_terminals(
        &input[0].text,
        None,
        None,
        Some(&package.grammar.as_ref().unwrap().terminals),
    )
    .unwrap()
    .len();
    let mut limits = EvaluationLimits::native();
    limits.tokens_per_resource = Some(count);
    limits.elements_per_resource = Some(3);
    assert!(
        evaluate_with_limits(&package, &input, limits)
            .diagnostics
            .is_empty()
    );
    limits.tokens_per_resource = Some(count - 1);
    let rejected = evaluate_with_limits(&package, &input, limits);
    assert!(rejected.model.elements.is_empty());
    assert!(
        rejected
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Token budget"))
    );
    limits.tokens_per_resource = None;
    limits.elements_per_resource = Some(2);
    assert!(
        evaluate_with_limits(&package, &input, limits)
            .model
            .elements
            .is_empty()
    );
    let mut maximum = XTEXT.to_owned();
    for n in 0..122 {
        maximum.push_str(&format!("\nUnused{n} returns k::Type: 'unused' name=ID;"));
    }
    assert_eq!(compile(&maximum).grammar.unwrap().rules.len(), 127);
    maximum.push_str("\nTooMany returns k::Type: 'extra' name=ID;");
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &maximum,
            RCORE_BEHAVIOR,
            Profile::NativeV1,
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("rule budget"));
    let legacy = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let serialized = serde_json::to_value(&legacy.grammar).unwrap();
    assert_eq!(serialized.get("terminals"), None);
    assert_ne!(serialized, json!(null));
}
