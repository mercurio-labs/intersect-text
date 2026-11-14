use intersect_text::grammar;
use intersect_text::grammar::Profile;
use intersect_text::grammar::PunctuationKeywords;
use std::borrow::Cow;
#[test]
fn model_lexer_matches_whole_keywords_and_keeps_source_backing_and_exact_spans() {
    let input = "é :: :>> ** \"::\" /*gap*/ ;";
    let matcher = PunctuationKeywords::new(["::", ":>", ":>>", "*", "**", ";"]);
    let native = grammar::lex_borrowed_with_keywords(input, Some(6), Some(&matcher)).unwrap();
    assert_eq!(
        native.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
        vec!["é", "::", ":>>", "**", "::", ";"]
    );
    for (token, (start, end, quoted)) in native.iter().zip([
        (0, 2, false),
        (3, 5, false),
        (6, 9, false),
        (10, 12, false),
        (13, 17, true),
        (26, 27, false),
    ]) {
        assert_eq!((token.start, token.end, token.quoted), (start, end, quoted));
        assert!(matches!(token.text, Cow::Borrowed(_)));
        if !quoted {
            assert_eq!(token.text, &input[start..end]);
        }
    }
    let error = grammar::lex_borrowed_with_keywords(input, Some(5), Some(&matcher)).unwrap_err();
    assert_eq!(error.0, 26);
    assert!(error.1.contains("Token budget"));
    let legacy = grammar::lex_borrowed(input, None).unwrap();
    assert_eq!(
        legacy.iter().map(|t| t.text.as_ref()).collect::<Vec<_>>(),
        vec!["é", ":", ":", ":", ">", ">", "*", "*", "::", ";"]
    );
    let explicit = grammar::lex_borrowed_with_keywords(input, None, None).unwrap();
    assert_eq!(
        legacy
            .iter()
            .map(|t| (&t.text, t.start, t.end, t.quoted))
            .collect::<Vec<_>>(),
        explicit
            .iter()
            .map(|t| (&t.text, t.start, t.end, t.quoted))
            .collect::<Vec<_>>()
    );
}
#[test]
fn native_keyword_admission_is_independent_of_boolean_and_dispatch_capabilities() {
    let ecore = include_str!("../../../../fixtures/library/library.ecore");
    let xtext = include_str!("../../../../fixtures/library/library.xtext");
    let rcore_behavior = include_str!("../../../../fixtures/library/library-behavior.rcore");
    for keyword in ["::", "**", ":>>", "/="] {
        let changed = xtext.replace("'library'", &format!("'library' '{keyword}'"));
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ecore,
                &changed,
                rcore_behavior,
                Profile::NativeV1
            )
            .is_ok(),
            "{keyword}"
        );
        let errors = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ecore,
            &changed,
            rcore_behavior,
        )
        .unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Keywords must")),
            "{errors:?}"
        );
    }
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ecore,
            &xtext.replace("'library'", "'library' ':'"),
            rcore_behavior
        )
        .is_ok()
    );
}
