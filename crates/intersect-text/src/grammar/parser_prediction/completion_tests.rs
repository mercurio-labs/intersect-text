// Focused prerequisites only: these controls do not admit source `=>` syntax.
// Included in the existing graph test module to reuse its IR construction helpers.

#[test]
fn shared_goal_decisions_preserve_original_reads_and_literal_work_stream() {
    let program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states: vec![
            State {
                edges: vec![Edge::Token(PredictionSymbol::Keyword("a".into()), 2)],
                ..State::default()
            },
            State {
                edges: vec![Edge::Token(PredictionSymbol::Keyword("b".into()), 3)],
                ..State::default()
            },
            State::default(),
            State::default(),
        ],
        rules: BTreeMap::new(),
        decisions: [(
            "choice".into(),
            Decision {
                entries: vec![0, 1],
                exit: None,
            },
        )]
        .into(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    let before = serde_json::to_vec(&program).unwrap();
    // Before the refactor this exact graph spends eight graph charges and reads
    // one symbol. An input adapter may charge its own read on the same allowance.
    for input_fee in [false, true] {
        let used = 8 + usize::from(input_fee);
        let mut reads = 0;
        let mut remaining = Some(used);
        assert_eq!(
            program
                .predict_with_input_work(
                    "choice",
                    |work| {
                        if input_fee {
                            charge(work)?;
                        }
                        reads += 1;
                        assert_eq!(reads, 1, "decision read an unrelated suffix");
                        Ok(Some(PredictionSymbol::Keyword("a".into())))
                    },
                    &mut remaining
                )
                .unwrap(),
            Some(0)
        );
        assert_eq!(remaining, Some(0));
        assert_eq!(reads, 1);
        assert!(
            program
                .predict_with_input_work(
                    "choice",
                    |work| {
                        if input_fee {
                            charge(work)?;
                        }
                        Ok(Some(PredictionSymbol::Keyword("a".into())))
                    },
                    &mut Some(used - 1)
                )
                .unwrap_err()
                .contains("budget")
        );
    }
    assert_eq!(
        program
            .predict(
                "choice",
                || Ok(Some(PredictionSymbol::Keyword("a".into()))),
                &mut None
            )
            .unwrap(),
        Some(0)
    );
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn borrowed_syntax_replay_charges_reads_and_never_refunds_after_rewind() {
    let tokens = symbols("a b");
    let original = tokens.clone();
    let mut input = ReplayTokens::new(&tokens);
    let mut remaining = Some(4);
    assert_eq!(
        input.read(&mut remaining).unwrap(),
        Some(PredictionSymbol::Keyword("a".into()))
    );
    let checkpoint = input.checkpoint().unwrap();
    assert_eq!(
        input.read(&mut remaining).unwrap(),
        Some(PredictionSymbol::Keyword("b".into()))
    );
    assert_eq!(remaining, Some(2));
    input.rewind(checkpoint).unwrap();
    assert_eq!(remaining, Some(2));
    assert_eq!(
        input.read(&mut remaining).unwrap(),
        Some(PredictionSymbol::Keyword("b".into()))
    );
    assert_eq!(remaining, Some(1));
    assert_eq!(
        input.read(&mut remaining).unwrap(),
        Some(PredictionSymbol::End)
    );
    assert!(input.read(&mut remaining).unwrap_err().contains("budget"));
    input.rewind(checkpoint).unwrap();
    assert_eq!(remaining, Some(0));
    assert!(input.read(&mut remaining).unwrap_err().contains("budget"));
    assert_eq!(tokens, original);
}

fn completion_oracle_program() -> ParserPrediction {
    let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
    inner.cardinality = '?';
    let g = grammar(
        choice(vec![call("Guarded"), call("Fallback")]),
        vec![
            (
                "Guarded",
                sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
            ),
            (
                "Fallback",
                sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
            ),
            ("Inner", inner),
        ],
    );
    ParserPrediction::compile(&g).unwrap()
}

#[test]
fn completion_goal_consumes_the_whole_operand_but_never_its_caller_follow() {
    let g = grammar(
        call("Operand"),
        vec![("Operand", sequence(vec![keyword("a"), keyword("b")]))],
    );
    let program = ParserPrediction::compile(&g).unwrap();
    assert!(
        !program
            .probe_rule_completion("Operand", &symbols("a c"), &mut Some(10_000))
            .unwrap()
    );
    assert!(
        program
            .probe_rule_completion("Operand", &symbols("a b unrelated"), &mut Some(10_000))
            .unwrap()
    );
    let &(start, stop) = program.rules.get("Operand").unwrap();
    let mut remaining = Some(10_000);
    let scope = program
        .verify_completion_scope(start, stop, remaining.as_mut().unwrap())
        .unwrap();
    let mut tokens = symbols("a b").into_iter();
    let mut reads = 0;
    let outcome = program
        .run_prediction(
            PredictionGoal::Completion {
                start,
                stop,
                scope: &scope,
                productivity: CompletionProductivity::Any,
            },
            &mut ForwardInput(|work: &mut Option<usize>| {
                charge(work)?;
                reads += 1;
                assert!(reads <= 2, "completion escaped into caller FOLLOW/EOF");
                Ok(tokens.next())
            }),
            &mut remaining,
        )
        .unwrap();
    assert!(matches!(outcome, PredictionOutcome::Completed(true)));
    assert_eq!(reads, 2);
}

#[test]
fn completion_probe_matches_all_pinned_full_guard_truth_observations() {
    let program = completion_oracle_program();
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
        .filter(|row| row["parser"] == "RecursiveEntryFull")
    {
        let text = row["text"].as_str().unwrap();
        let matched = program
            .probe_rule_completion("Guarded", &symbols(text), &mut Some(100_000))
            .unwrap();
        // This compares the independently observed guard truth only. It does
        // not infer model acceptance from a successful prefix guard.
        assert_eq!(matched, row["branch"] == "g", "{row}");
        compared += 1;
    }
    assert_eq!(compared, 11);
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn full_completion_and_first_token_commitment_preserve_the_oracle_distinction() {
    let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
    inner.cardinality = '?';
    let g = grammar(
        choice(vec![
            sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
            sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
        ]),
        vec![("Inner", inner)],
    );
    let first = ParserPrediction::compile_with_recursive_first_predicates(
        &g,
        &["rule/Entry/a0".into()].into(),
    )
    .unwrap();
    let full = completion_oracle_program();
    for text in ["a c", "a a b c"] {
        assert_eq!(
            selected(&first, "rule/Entry/choice", text, &mut Some(10_000)).unwrap(),
            Some(0)
        );
        assert!(
            !full
                .probe_rule_completion("Guarded", &symbols(text), &mut Some(100_000))
                .unwrap()
        );
    }
}

#[test]
fn completion_obeys_nested_commitment_instead_of_accepting_nfa_fallback() {
    let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
    inner.cardinality = '?';
    let g = grammar(
        call("Probe"),
        vec![
            ("Probe", call("Nested")),
            (
                "Nested",
                choice(vec![
                    sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
                    sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
                ]),
            ),
            ("Inner", inner),
        ],
    );
    let program = ParserPrediction::compile_with_recursive_first_predicates(
        &g,
        &["rule/Nested/a0".into()].into(),
    )
    .unwrap();
    // The independent RecursiveEntryFirst oracle selects its guarded branch
    // then rejects `a c`. Existential graph membership could instead accept c.
    assert_eq!(
        selected(&program, "rule/Nested/choice", "a c", &mut Some(10_000)).unwrap(),
        Some(0)
    );
    assert!(
        !program
            .probe_rule_completion("Probe", &symbols("a c"), &mut Some(100_000))
            .unwrap()
    );
}

#[test]
fn completion_requires_conservative_static_proof_and_a_finite_shared_allowance() {
    let ambiguous =
        ParserPrediction::compile(&grammar(choice(vec![keyword("a"), keyword("a")]), vec![]))
            .unwrap();
    let message = ambiguous
        .probe_rule_completion("Entry", &symbols("a"), &mut Some(100_000))
        .unwrap_err();
    assert!(
        message.contains("Completion static verification failed at rule/Entry/choice"),
        "{message}"
    );
    let program = completion_oracle_program();
    assert!(
        program
            .probe_rule_completion("Guarded", &symbols("a b"), &mut None)
            .unwrap_err()
            .contains("finite shared")
    );
    let mut remaining = Some(100_000);
    assert!(
        program
            .probe_rule_completion("Guarded", &symbols("a b"), &mut remaining)
            .unwrap()
    );
    let used = 100_000 - remaining.unwrap();
    assert!(
        used > 2,
        "static proof and traversal must share charged work"
    );
    assert!(
        program
            .probe_rule_completion("Guarded", &symbols("a b"), &mut Some(used))
            .unwrap()
    );
    assert!(
        program
            .probe_rule_completion("Guarded", &symbols("a b"), &mut Some(used - 1))
            .unwrap_err()
            .contains("budget")
    );
}

#[test]
fn completion_static_proof_uses_the_explicit_caller_work_allowance() {
    // A real, bound linear graph crosses the old 10k work allowance without
    // introducing an unrelated unbound fanout or exceeding the state ceiling.
    let mut states = vec![State::default(); 6001];
    for (index, state) in states[..6000].iter_mut().enumerate() {
        state.edges = vec![Edge::Epsilon(index + 1)];
    }
    states[6000].rule_stop = true;
    let program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states,
        rules: [("Operand".into(), (0, 6000))].into(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    let before = serde_json::to_vec(&program).unwrap();
    let mut legacy = Some(super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units());
    let message = program
        .probe_rule_completion("Operand", &[], &mut legacy)
        .unwrap_err();
    assert!(message.contains("budget"), "{message}");
    assert_eq!(legacy, Some(0));
    let mut larger = Some(100_000);
    assert!(
        program
            .probe_rule_completion("Operand", &[], &mut larger)
            .unwrap()
    );
    assert!(larger.is_some_and(|remaining| remaining > 0 && remaining < 90_000));
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

// Draft only: append to the existing cfg(test) completion_tests.rs if the
// genuine full-source leaf points to recursive analysis. No production change.
#[test]
fn observed_recursive_component_legacy_pool_preserves_error_and_default_compile_parity() {
    use super::super::admission_observation::{Phase, PredictionObservation, RunStage};
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    let count = 2500;
    let recursive_body = |name: &str, exit: &str| {
        let mut prefix: Vec<_> = (0..count).map(|_| keyword("a")).collect();
        prefix.push(call(name));
        choice(vec![sequence(prefix), keyword(exit)])
    };
    let mut g = grammar(
        choice(vec![
            sequence(vec![call("A"), keyword("x")]),
            sequence(vec![call("B"), keyword("y")]),
        ]),
        vec![
            ("A", recursive_body("A", "b")),
            ("B", recursive_body("B", "c")),
        ],
    );
    g.profile = Profile::NativeV66;
    let entry = "Entry: -> A 'x' | B 'y';\n";
    let prefix = vec!["'a'"; count].join(" ");
    let a = format!("A: {prefix} A | 'b';\n");
    let b = format!("B: {prefix} B | 'c';\n");
    let source = DefinitionSource::new(
        "memory:leaf-recursive-work.xtext",
        SourceKind::Xtext,
        format!("{entry}{a}{b}"),
    );
    for (name, start, end) in [
        ("Entry", 0, entry.len()),
        ("A", entry.len(), entry.len() + a.len()),
        ("B", entry.len() + a.len(), source.text.len()),
    ] {
        g.declaration_sources.insert(
            name.into(),
            SourceSpan {
                source: source.id.clone(),
                start,
                end,
            },
        );
    }
    let predicates = BTreeSet::from(["rule/Entry/a0/s0".into()]);
    let ordinary =
        ParserPrediction::compile_with_recursive_first_predicates(&g, &predicates).unwrap();
    let default_observation = PredictionObservation::new();
    let observed = ParserPrediction::compile_observed(
        &g,
        &predicates,
        true,
        super::super::AnalysisObservation {
            observation: &default_observation,
            recursive_work: None,
            adjacent_work: None,
            entry: &source.id,
            stage: RunStage::Executable,
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&ordinary).unwrap(),
        serde_json::to_vec(&observed).unwrap()
    );
    assert!(default_observation.first_failure().is_none());
    // Component-only: explicitly select the historical pool for the same
    // recursive worker, then exercise the existing phase-observation adapter.
    // This is not a failure of the current default whole-compiler allowance.
    let allowance = super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units();
    let (plain, plain_remaining) =
        bounded_recursive_decision(&ordinary, "rule/Entry/choice", allowance, true, true);
    assert_eq!(plain, Err("Grammar prediction budget exceeded".into()));
    assert_eq!(plain_remaining, 0);
    let observation = PredictionObservation::new();
    let (result, remaining) =
        bounded_recursive_decision(&ordinary, "rule/Entry/choice", allowance, true, true);
    let observed_error = observe_parser_phase(
        result,
        &g,
        Some(super::super::AnalysisObservation {
            observation: &observation,
            recursive_work: None,
            adjacent_work: None,
            entry: &source.id,
            stage: RunStage::Executable,
        }),
        Phase::ParserRecursiveAnalysis,
        (Some("Entry"), Some("rule/Entry/choice")),
        (Some(allowance - remaining), Some(ordinary.states.len())),
    )
    .unwrap_err();
    assert_eq!(observed_error, plain.unwrap_err());
    assert_eq!(remaining, plain_remaining);
    let failure = observation.first_failure().unwrap();
    assert_eq!(failure.phase, Phase::ParserRecursiveAnalysis);
    assert_eq!(failure.charged_work, Some(10000));
    assert!(
        failure
            .state_count
            .is_some_and(|states| states > 5000 && states < 10000)
    );
    assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "Entry");
    assert_eq!(failure.declaration_span, Some((0, entry.len())));
    assert_eq!(
        failure.decision.unwrap().as_str().unwrap(),
        "rule/Entry/choice"
    );
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        source.id.sha256
    );
    assert_eq!(failure.syntax_depth, None);
    assert_eq!(failure.call_depth, None);
    assert_eq!(failure.repeated_state_count, None);
    // Without the guard there is no recursive-guard analysis; the same sized
    // operand graphs fit the unchanged graph-state ceiling.
    let no_guard =
        ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap();
    assert!(no_guard.states.len() < 10000);
}
