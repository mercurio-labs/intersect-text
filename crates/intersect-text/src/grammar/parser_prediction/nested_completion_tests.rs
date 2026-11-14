// Nested completion uses the established Decision evaluator, not a second parser.

fn nested_first_program(recursive: bool) -> ParserPrediction {
    let (left, right, calls) = if recursive {
        let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
        inner.cardinality = '?';
        (
            sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
            sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
            vec![("Inner", inner)],
        )
    } else {
        (
            sequence(vec![keyword("a"), keyword("b")]),
            sequence(vec![keyword("a"), keyword("c")]),
            vec![],
        )
    };
    let mut calls = calls;
    calls.push(("Probe", call("Nested")));
    calls.push(("Nested", choice(vec![left, right])));
    let g = grammar(call("Probe"), calls);
    ParserPrediction::compile_with_recursive_first_predicates(&g, &["rule/Nested/a0".into()].into())
        .unwrap()
}

fn completed_prefix(
    program: &ParserPrediction,
    rule: &str,
    text: &str,
    remaining: &mut Option<usize>,
) -> Result<(bool, usize), String> {
    let &(start, stop) = program.rules.get(rule).unwrap();
    let scope = program.verify_completion_scope(start, stop, remaining.as_mut().unwrap())?;
    let tokens = symbols(text);
    let mut input = ReplayTokens::new(&tokens);
    let outcome = program.run_prediction(
        PredictionGoal::Completion {
            start,
            stop,
            scope: &scope,
            productivity: CompletionProductivity::Any,
        },
        &mut input,
        remaining,
    )?;
    match outcome {
        PredictionOutcome::Completed(matched) => Ok((matched, input.position)),
        PredictionOutcome::Selected(_) => panic!("completion selected a branch"),
        PredictionOutcome::InitialPrepared(_) => panic!("completion returned graph preparation"),
    }
}

#[test]
fn nested_recursive_commitment_matches_all_independent_first_parser_acceptance_rows() {
    let program = nested_first_program(true);
    let before = serde_json::to_vec(&program).unwrap();
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../assets/fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut compared = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["parser"] == "RecursiveEntryFirst")
    {
        let text = row["text"].as_str().unwrap();
        let (matched, end) = completed_prefix(&program, "Probe", text, &mut Some(100_000)).unwrap();
        let accepted = matched && end + 1 == symbols(text).len();
        assert_eq!(accepted, row["accepted"].as_bool().unwrap(), "{row}");
        compared += 1;
    }
    assert_eq!(compared, 11);
    assert!(
        !program
            .probe_rule_completion("Probe", &symbols("a c"), &mut Some(100_000))
            .unwrap()
    );
    assert!(
        !program
            .probe_rule_completion("Probe", &symbols("a a b c"), &mut Some(100_000))
            .unwrap()
    );
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn nested_regular_guard_preserves_second_token_lookahead_instead_of_forcing_commitment() {
    let program = nested_first_program(false);
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../assets/fixtures/xtext/first-token-predicates/decision-observations.json"
    ))
    .unwrap();
    let mut compared = 0;
    for row in rows["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["parser"] == "FirstChoice")
    {
        let text = row["text"].as_str().unwrap();
        let (matched, end) = completed_prefix(&program, "Probe", text, &mut Some(100_000)).unwrap();
        assert_eq!(
            matched && end + 1 == symbols(text).len(),
            row["accepted"].as_bool().unwrap(),
            "{row}"
        );
        compared += 1;
    }
    assert_eq!(compared, 11);
    assert!(
        program
            .probe_rule_completion("Probe", &symbols("a c"), &mut Some(100_000))
            .unwrap()
    );
}

#[test]
fn nested_scope_rejects_forged_entry_order_missing_guard_bindings_and_hidden_guards() {
    let mut reversed = nested_first_program(true);
    reversed
        .decisions
        .get_mut("rule/Nested/choice")
        .unwrap()
        .entries
        .reverse();
    assert!(
        reversed
            .probe_rule_completion("Probe", &symbols("a b"), &mut Some(100_000))
            .unwrap_err()
            .contains("cannot bind")
    );
    let mut missing = nested_first_program(true);
    missing.first_guards.remove("rule/Nested/a0");
    assert!(
        missing
            .probe_rule_completion("Probe", &symbols("a b"), &mut Some(100_000))
            .unwrap_err()
            .contains("Unbound completion first-token guard")
    );
    let g = grammar(
        call("Nested"),
        vec![(
            "Nested",
            choice(vec![
                sequence(vec![
                    expr(Kind::Action("Object".into())),
                    keyword("a"),
                    keyword("b"),
                ]),
                sequence(vec![keyword("a"), keyword("c")]),
            ]),
        )],
    );
    let hidden =
        ParserPrediction::compile_with_first_predicates(&g, &["rule/Nested/a0/s1".into()].into())
            .unwrap();
    let message = hidden
        .probe_rule_completion("Nested", &symbols("a c"), &mut Some(100_000))
        .unwrap_err();
    // The independent visibility fixture rejects guards hidden behind actions.
    // This is explicit native proof refusal, not compiler diagnostic equivalence.
    assert!(
        message.contains("Completion static verification failed")
            || message.contains("visibility remains unsupported"),
        "{message}"
    );
}

#[test]
fn nested_prediction_replay_keeps_exact_one_short_and_failed_work_charged() {
    let program = nested_first_program(true);
    let input = symbols("a a b b");
    let mut remaining = Some(100_000);
    assert!(
        program
            .probe_rule_completion("Probe", &input, &mut remaining)
            .unwrap()
    );
    let used = 100_000 - remaining.unwrap();
    assert!(
        program
            .probe_rule_completion("Probe", &input, &mut Some(used))
            .unwrap()
    );
    assert!(
        program
            .probe_rule_completion("Probe", &input, &mut Some(used - 1))
            .unwrap_err()
            .contains("budget")
    );
    let mut failed = Some(100_000);
    assert!(
        !program
            .probe_rule_completion("Probe", &symbols("a c"), &mut failed)
            .unwrap()
    );
    assert!(failed.unwrap() < 100_000);
    let after_first = failed.unwrap();
    assert!(
        !program
            .probe_rule_completion("Probe", &symbols("a c"), &mut failed)
            .unwrap()
    );
    assert!(failed.unwrap() < after_first);
}

#[test]
fn exact_operand_boundary_supports_greedy_optional_without_using_caller_follow() {
    let mut optional = keyword("a");
    optional.cardinality = '?';
    let g = grammar(
        sequence(vec![call("Operand"), keyword("outside")]),
        vec![("Operand", optional)],
    );
    let program = ParserPrediction::compile(&g).unwrap();
    let (matched, end) =
        completed_prefix(&program, "Operand", "a outside", &mut Some(100_000)).unwrap();
    assert!(matched);
    assert_eq!(end, 1);
    let (matched, end) =
        completed_prefix(&program, "Operand", "outside", &mut Some(100_000)).unwrap();
    assert!(matched);
    assert_eq!(end, 0);
}
