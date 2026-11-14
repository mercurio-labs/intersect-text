use intersect_text::grammar::lex;
use intersect_text::grammar::lex_borrowed;
use std::borrow::Cow;

#[test]
fn borrowed_lexemes_preserve_unicode_decoding_and_source_offsets() {
    let input = "/* α */ member café; 'plain' \"a\\nb\\tλ\\\"z\" // tail\n42";
    let tokens = lex_borrowed(input, None).unwrap();
    let text: Vec<_> = tokens.iter().map(|t| t.text.as_ref()).collect();
    assert_eq!(text, ["member", "café", ";", "plain", "a\nb\tλ\"z", "42"]);
    assert!(
        tokens
            .iter()
            .enumerate()
            .all(|(i, t)| matches!(t.text, Cow::Owned(_)) == (i == 4))
    );
    assert_eq!(&input[tokens[1].start..tokens[1].end], "café");
    assert_eq!(&input[tokens[3].start..tokens[3].end], "'plain'");
    for (borrowed, owned) in tokens.iter().zip(lex(input).unwrap()) {
        assert_eq!(borrowed.text, owned.text);
        assert_eq!(
            (borrowed.start, borrowed.end, borrowed.quoted),
            (owned.start, owned.end, owned.quoted)
        );
    }
    assert_eq!(lex_borrowed("'' \"\"", Some(2)).unwrap().len(), 2);
    assert!(lex_borrowed(input, Some(5)).is_err());
    for malformed in ["'unclosed", "/* open", "'\\q'", "'\\"] {
        assert_eq!(
            lex(malformed).unwrap_err(),
            lex_borrowed(malformed, None).unwrap_err()
        );
    }
}
