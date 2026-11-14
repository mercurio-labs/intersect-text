//! Actual canonical lexer graph/control tests; no source/model authority.
use super::*;
use crate::grammar::terminals::Expression;
use crate::grammar::terminals::Operation;
fn expr(kind: Operation) -> Expression {
    Expression {
        kind,
        cardinality: '1',
        first: vec![],
        nullable: false,
        decision: None,
        cardinality_decision: None,
    }
}
fn literal(text: &str) -> Expression {
    expr(Operation::Literal(text.encode_utf16().collect()))
}
fn shown(result: Result<LexicalDecision, String>) -> String {
    format!("{result:?}")
}

#[test]
fn lexical_initial_keeps_priority_until_eof_and_utf16_decisions() -> Result<(), String> {
    let mut letters = expr(Operation::Range(u16::from(b'a'), u16::from(b'z')));
    letters.cardinality = '+';
    let mut keyword = LexerPrediction::terminal_choices(&[literal("if"), letters])?;
    keyword.alternatives[0] = Alternative::Keyword("if".into());
    let eof = LexerPrediction::terminal_choices(&[
        expr(Operation::Sequence(vec![
            literal("a"),
            expr(Operation::Eof),
        ])),
        literal("ab"),
    ])?;
    let until = LexerPrediction::terminal_choices(&[
        expr(Operation::Until("!".encode_utf16().collect())),
        literal("!a"),
        literal("😀"),
    ])?;
    for program in [&keyword, &eof, &until] {
        let initial = PreparedLexicalInitial::prepare(program, &mut Some(100_000), 100_000)?;
        for text in [
            "", "if", "iff", "a", "ab", "ac", "!a", "!a!", "😀", "😀!", "?",
        ] {
            let old = program.predict_document_units(text.encode_utf16(), &mut Some(100_000));
            let new = program.predict_document_units_with_prepared_initial(
                text.encode_utf16(),
                &initial,
                &mut Some(100_000),
            );
            assert_eq!(shown(old), shown(new), "source {text:?}");
        }
        // Interior UTF-16 units are real inputs, not invented Unicode scalars.
        for units in [
            vec![0xd83d],
            vec![0xde00],
            vec![0xd83d, 0xde00],
            vec![0xd83d, 33],
        ] {
            assert_eq!(
                shown(program.predict_document_units(units.iter().copied(), &mut Some(100_000))),
                shown(program.predict_document_units_with_prepared_initial(
                    units.iter().copied(),
                    &initial,
                    &mut Some(100_000)
                ))
            );
        }
    }
    assert_eq!(keyword.predict("if", &mut Some(100_000))?, 0);
    assert_eq!(keyword.predict("iff", &mut Some(100_000))?, 1);
    assert_eq!(eof.predict("a", &mut Some(100_000))?, 0);
    assert_eq!(eof.predict("ab", &mut Some(100_000))?, 1);
    assert!(eof.predict("ac", &mut Some(100_000)).is_err());
    Ok(())
}

#[test]
fn lexical_initial_paid_preparation_runtime_limits_and_foreign_equal_program() -> Result<(), String>
{
    let program = LexerPrediction::terminal_choices(&[
        expr(Operation::Sequence(vec![
            literal("a"),
            expr(Operation::Eof),
        ])),
        literal("ab"),
        literal("😀"),
    ])?;
    for mut denied in [None, Some(0), Some(1)] {
        assert!(PreparedLexicalInitial::prepare(&program, &mut denied, 100_000).is_err());
    }
    let mut pool = Some(100_000);
    let context = PreparedLexicalInitial::prepare(&program, &mut pool, 100_000)?;
    let used = 100_000 - pool.ok_or("Missing finite preparation balance")?;
    for allowance in [used - 1, used] {
        let result = PreparedLexicalInitial::prepare(
            &program,
            &mut Some(allowance),
            context.logical_bytes(),
        );
        assert_eq!(result.is_ok(), allowance == used);
    }
    assert!(
        PreparedLexicalInitial::prepare(&program, &mut Some(100_000), context.logical_bytes() - 1)
            .is_err()
    );
    let foreign = program.clone();
    assert_eq!(
        serde_json::to_value(&program).map_err(|e| e.to_string())?,
        serde_json::to_value(&foreign).map_err(|e| e.to_string())?
    );
    assert!(
        foreign
            .predict_document_units_with_prepared_initial(
                "a".encode_utf16(),
                &context,
                &mut Some(100_000)
            )
            .is_err()
    );
    for text in ["a", "ab", "😀"] {
        let mut runtime = Some(100_000);
        let expected = shown(program.predict_document_units_with_prepared_initial(
            text.encode_utf16(),
            &context,
            &mut runtime,
        ));
        let used = 100_000 - runtime.ok_or("Missing finite runtime balance")?;
        for allowance in [0, used - 1, used] {
            let actual = program.predict_document_units_with_prepared_initial(
                text.encode_utf16(),
                &context,
                &mut Some(allowance),
            );
            if allowance == used {
                assert_eq!(shown(actual), expected);
            } else {
                assert!(actual.is_err());
            }
        }
    }
    assert!(
        program
            .predict_document_units_with_prepared_initial("a".encode_utf16(), &context, &mut None)
            .is_err()
    );
    // A denied independent invocation cannot mutate immutable graph facts.
    assert_eq!(
        shown(program.predict_document_units_with_prepared_initial(
            "a".encode_utf16(),
            &context,
            &mut Some(100_000)
        )),
        shown(program.predict_document_units("a".encode_utf16(), &mut Some(100_000)))
    );
    Ok(())
}
