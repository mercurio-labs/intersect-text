//! Actual canonical transition/closure parity, complete outcomes and denial.
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
fn lexical_first_steps_all_utf16_transitions_priority_until_and_conditional_eof()
-> Result<(), String> {
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
        let context = PreparedLexicalInitial::prepare(program, &mut Some(100_000), 100_000)?
            .with_first_steps(&mut Some(100_000), 100_000)?;
        assert!(context.first_step_intervals() > 1);
        // Every possible unit, including lone surrogate units, maps to exactly
        // the old advance -> epsilon closure -> owner/accepting facts.
        for unit in 0..=u16::MAX {
            let targets = program.advance(&context.initial, unit, &mut Some(100_000))?;
            let closed = program.closure(targets, &mut Some(100_000))?;
            let (owners, accepting) = program.frontier_owners(&closed, &mut Some(100_000))?;
            let (actual, actual_owners, actual_accepting) = context
                .first_step(unit, &mut Some(100))?
                .ok_or("Missing complete partition")?;
            assert_eq!(actual, &closed, "unit {unit}");
            assert_eq!(actual_owners, &owners, "unit {unit}");
            assert_eq!(actual_accepting, &accepting, "unit {unit}");
        }
        for units in [
            vec![],
            vec![0xd83d],
            vec![0xde00],
            vec![0xd83d, 0xde00],
            vec![0xd83d, 33],
        ]
        .into_iter()
        .chain(
            ["if", "iff", "a", "ab", "ac", "!a", "!a!", "😀", "😀!", "?"]
                .into_iter()
                .map(|text| text.encode_utf16().collect()),
        ) {
            assert_eq!(
                shown(program.predict_document_units(units.iter().copied(), &mut Some(100_000))),
                shown(program.predict_document_units_with_prepared_initial(
                    units.iter().copied(),
                    &context,
                    &mut Some(100_000)
                )),
                "units {units:?}",
            );
        }
    }
    assert_eq!(keyword.predict("if", &mut Some(100_000))?, 0);
    assert_eq!(keyword.predict("iff", &mut Some(100_000))?, 1);
    assert_eq!(eof.predict("a", &mut Some(100_000))?, 0);
    assert_eq!(eof.predict("ab", &mut Some(100_000))?, 1);
    Ok(())
}
#[test]
fn lexical_first_steps_paid_combined_cap_exact_runtime_and_foreign_binding() -> Result<(), String> {
    let program = LexerPrediction::terminal_choices(&[literal("a"), literal("ab"), literal("😀")])?;
    let make = |work: &mut Option<usize>, cap| {
        PreparedLexicalInitial::prepare(&program, work, cap)?.with_first_steps(work, cap)
    };
    let mut work = Some(100_000);
    let context = make(&mut work, 100_000)?;
    let paid = 100_000 - work.ok_or("Missing finite pool")?;
    for allowance in [0, paid - 1, paid] {
        let result = make(&mut Some(allowance), context.logical_bytes());
        assert_eq!(result.is_ok(), allowance == paid);
    }
    assert!(make(&mut None, 100_000).is_err());
    assert!(make(&mut Some(100_000), context.logical_bytes() - 1).is_err());
    assert!(make(&mut Some(100_000), 0).is_err());
    assert!(
        make(&mut Some(100_000), 100_000)?
            .with_first_steps(&mut Some(100_000), 100_000)
            .is_err()
    );
    let old = PreparedLexicalInitial::prepare(&program, &mut Some(100_000), 100_000)?;
    assert_eq!(old.first_step_intervals(), 0);
    let foreign = program.clone();
    for text in ["", "a", "ab", "😀", "?"] {
        assert!(
            foreign
                .predict_document_units_with_prepared_initial(
                    text.encode_utf16(),
                    &context,
                    &mut Some(100_000)
                )
                .is_err()
        );
        let mut pool = Some(100_000);
        let expected = shown(program.predict_document_units_with_prepared_initial(
            text.encode_utf16(),
            &context,
            &mut pool,
        ));
        let used = 100_000 - pool.ok_or("Missing finite runtime")?;
        for allowance in [0, used - 1, used] {
            let actual = program.predict_document_units_with_prepared_initial(
                text.encode_utf16(),
                &context,
                &mut Some(allowance),
            );
            if allowance == used {
                assert_eq!(shown(actual), expected)
            } else {
                assert!(actual.is_err())
            }
        }
        assert_eq!(
            expected,
            shown(program.predict_document_units(text.encode_utf16(), &mut Some(100_000)))
        );
    }
    Ok(())
}
