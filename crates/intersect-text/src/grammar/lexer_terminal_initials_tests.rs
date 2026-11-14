//! Actual source-regenerated local graphs and canonical committed recognizer.
use super::*;
use crate::grammar;
use crate::grammar::terminals;
use crate::grammar::terminals::DocumentUnitMatch;
use crate::grammar::terminals::Expression;
use std::cell::Cell;
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
fn terminal(name: &str, body: Expression) -> Terminal {
    Terminal {
        override_declared: false,
        fragment: false,
        datatype: grammar::Datatype::EString,
        nominal_return: None,
        enumeration: None,
        identity: name.into(),
        start: 0,
        end: 1,
        body,
    }
}
fn fixture() -> Result<(BTreeMap<String, Terminal>, LexerPrediction), String> {
    let mut suffix = expr(Operation::Call {
        name: "TAIL".into(),
        body: None,
    });
    suffix.cardinality = '*';
    let id = expr(Operation::Sequence(vec![
        expr(Operation::Range(97, 122)),
        suffix,
    ]));
    let comment = expr(Operation::Sequence(vec![
        literal("/*"),
        expr(Operation::UntilOperand {
            operand: Box::new(literal("*/")),
            decision: None,
        }),
    ]));
    let mut repeated = literal("x");
    repeated.cardinality = '+';
    let eof = expr(Operation::Sequence(vec![repeated, expr(Operation::Eof)]));
    let mut map = BTreeMap::from([
        ("ID".into(), terminal("ID", id)),
        ("COMMENT".into(), terminal("COMMENT", comment)),
        ("END".into(), terminal("END", eof)),
        ("FIXED".into(), terminal("FIXED", literal("="))),
    ]);
    let mut tail = terminal(
        "TAIL",
        expr(Operation::Alternative(vec![
            literal("ab"),
            literal("ac"),
            expr(Operation::Range(97, 122)),
            expr(Operation::Range(48, 57)),
        ])),
    );
    tail.fragment = true;
    map.insert("TAIL".into(), tail);
    terminals::prepare(&mut map, grammar::Profile::NativeV67).map_err(|error| error.message)?;
    let expressions = std::iter::once(literal("if"))
        .chain(map.values().filter(|t| !t.fragment).map(|t| t.body.clone()))
        .collect::<Vec<_>>();
    let mut global = LexerPrediction::terminal_choices(&expressions)?;
    global.alternatives[0] = Alternative::Keyword("if".into());
    for (index, (name, _)) in map
        .iter()
        .filter(|(_, terminal)| !terminal.fragment)
        .enumerate()
    {
        global.alternatives[index + 1] = Alternative::Terminal(name.clone());
    }
    Ok((map, global))
}
#[allow(clippy::type_complexity)]
fn shown(
    result: Result<
        (
            Vec<grammar::Token<std::borrow::Cow<'_, str>>>,
            Vec<grammar::LexicalIssue>,
            Vec<grammar::DocumentTokenRange>,
        ),
        (usize, String),
    >,
) -> Result<serde_json::Value, (usize, String)> {
    result.map(|(tokens,issues,ranges)|serde_json::json!({"tokens":tokens.iter().map(|t|serde_json::json!({"text":t.text,"start":t.start,"end":t.end,"quoted":t.quoted,"alternative":t.lexical_alternative})).collect::<Vec<_>>(),"issues":issues,"ranges":ranges}))
}
fn matched(result: Result<DocumentUnitMatch, String>) -> String {
    match result {
        Ok(DocumentUnitMatch::Matched(end)) => format!("Matched {end}"),
        Ok(DocumentUnitMatch::Failed {
            consumed_units,
            failure,
        }) => format!("Failed {consumed_units} {failure:?}"),
        Err(error) => format!("Err {error}"),
    }
}
#[test]
fn terminal_initials_preserve_whole_tokens_unicode_priority_until_eof_and_unselected_fallback()
-> Result<(), String> {
    let (map, global) = fixture()?;
    let context = PreparedLexicalInitial::prepare(&global, &mut Some(100_000), 100_000)?
        .with_first_steps(&mut Some(100_000), 100_000)?
        .with_terminal_initials(&map, ["ID", "COMMENT", "END"], &mut Some(100_000), 100_000)?;
    assert!(context.terminal_initial_programs() >= 3);
    for text in [
        "",
        "if",
        "iff",
        "identifier123",
        "=",
        "xxx",
        "xy",
        "/*a*/",
        "/*😀*/",
        "/* **/",
        "/*unclosed",
        "😀",
        "abc=def",
    ] {
        assert_eq!(
            shown(grammar::lex_document(
                text,
                Some(100),
                &map,
                &global,
                &mut Some(100_000)
            )),
            shown(grammar::lex_document_with_prepared_initial(
                text,
                Some(100),
                &map,
                &global,
                &context,
                &mut Some(100_000)
            )),
            "source {text:?}"
        );
    }
    let registry = context.terminal_registry().ok_or("Missing registry")?;
    for name in ["ID", "COMMENT", "END", "FIXED"] {
        let terminal = map.get(name).ok_or("Missing terminal")?;
        for units in [
            vec![],
            vec![0xd83d],
            vec![0xde00],
            vec![0xd83d, 0xde00],
            "/*😀*/".encode_utf16().collect(),
            "identifier123".encode_utf16().collect(),
            "xxx".encode_utf16().collect(),
        ] {
            assert_eq!(
                matched(terminal.match_document_units(units.iter().copied(), &mut Some(100_000))),
                matched(terminal.match_document_units_with_initials(
                    units.iter().copied(),
                    registry,
                    &mut Some(100_000)
                )),
                "terminal {name}/units{units:?}"
            );
        }
    }
    let mut cold = Some(100_000);
    let mut warm = Some(100_000);
    let id = map.get("ID").ok_or("Missing ID")?;
    assert_eq!(
        matched(id.match_document_units("identifier123".encode_utf16(), &mut cold)),
        matched(id.match_document_units_with_initials(
            "identifier123".encode_utf16(),
            registry,
            &mut warm
        ))
    );
    assert!(warm > cold);
    Ok(())
}
#[test]
fn terminal_initials_paid_caps_unknown_duplicate_foreign_map_and_program_precede_input()
-> Result<(), String> {
    let (map, global) = fixture()?;
    let make = |work: &mut Option<usize>, cap| {
        PreparedLexicalInitial::prepare(&global, work, cap)?.with_terminal_initials(
            &map,
            ["ID"],
            work,
            cap,
        )
    };
    let mut work = Some(100_000);
    let context = make(&mut work, 100_000)?;
    let used = 100_000 - work.ok_or("Missing finite pool")?;
    for allowance in [0, used - 1, used] {
        assert_eq!(
            make(&mut Some(allowance), context.logical_bytes()).is_ok(),
            allowance == used
        );
    }
    assert!(make(&mut None, 100_000).is_err());
    assert!(make(&mut Some(100_000), context.logical_bytes() - 1).is_err());
    for names in [vec![], vec!["UNKNOWN"], vec!["ID", "ID"], vec!["FIXED"]] {
        assert!(
            PreparedLexicalInitial::prepare(&global, &mut Some(100_000), 100_000)?
                .with_terminal_initials(&map, names, &mut Some(100_000), 100_000)
                .is_err()
        );
    }
    let foreign = map.clone();
    assert_eq!(
        serde_json::to_value(&map).map_err(|e| e.to_string())?,
        serde_json::to_value(&foreign).map_err(|e| e.to_string())?
    );
    for text in ["", "identifier123"] {
        assert!(
            grammar::lex_document_with_prepared_initial(
                text,
                Some(100),
                &foreign,
                &global,
                &context,
                &mut Some(100_000)
            )
            .is_err()
        );
    }
    let registry = context.terminal_registry().ok_or("Missing registry")?;
    let foreign_program = registry.rows[0].program.clone();
    let reads = Cell::new(0);
    let input = std::iter::from_fn(|| {
        reads.set(reads.get() + 1);
        Some(97)
    });
    assert!(
        registry
            .predict(&foreign_program, input, &mut Some(100_000), false)
            .is_err()
    );
    assert_eq!(reads.get(), 0);
    for text in ["", "identifier123", "😀"] {
        let mut runtime = Some(100_000);
        let expected = shown(grammar::lex_document_with_prepared_initial(
            text,
            Some(100),
            &map,
            &global,
            &context,
            &mut runtime,
        ));
        let used = 100_000 - runtime.ok_or("Missing finite runtime")?;
        for allowance in [0, used - 1, used] {
            let actual = grammar::lex_document_with_prepared_initial(
                text,
                Some(100),
                &map,
                &global,
                &context,
                &mut Some(allowance),
            );
            if allowance == used {
                assert_eq!(shown(actual), expected)
            } else {
                assert!(actual.is_err())
            }
        }
    }
    Ok(())
}
