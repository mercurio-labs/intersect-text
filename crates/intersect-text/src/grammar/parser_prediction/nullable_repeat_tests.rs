// Source-only successor controls: execution requires root review.
fn nullable_repeat_grammar() -> Grammar {
    let mut optional = expr(Kind::Assign {
        feature: "tag".into(),
        many: false,
        value: Box::new(keyword("x")),
    });
    optional.cardinality = '?';
    let target = sequence(vec![expr(Kind::Action("Target".into())), optional]);
    let mut repeated = full(expr(Kind::Assign {
        feature: "children".into(),
        many: true,
        value: Box::new(call("Target")),
    }));
    repeated.cardinality = '*';
    let mut g = grammar(
        sequence(vec![
            expr(Kind::Action("Root".into())),
            repeated,
            keyword("x"),
        ]),
        vec![("Target", target)],
    );
    // Direct low-level capability controls use the old graph builder; source
    // admission/activation is tested separately through original common sources.
    g.profile = Profile::NativeV76;
    g
}
fn nullable_repeat_program() -> ParserPrediction {
    ParserPrediction::compile_with_recursive_first_predicates(
        &nullable_repeat_grammar(),
        &BTreeSet::new(),
    )
    .unwrap()
}
fn nullable_required_source_fixture() -> Grammar {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    fn retain(expr: &mut Expr) {
        expr.original_range = Some(1..99);
        match &mut expr.kind {
            Kind::Sequence(items) | Kind::Alternative(items) => {
                for item in items {
                    retain(item);
                }
            }
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => retain(value),
            _ => {}
        }
    }
    let mut grammar = nullable_repeat_grammar();
    grammar.profile = Profile::NativeV77;
    let source = DefinitionSource::new(
        "memory:authored-structural-nullable-control.xtext",
        SourceKind::Xtext,
        " ".repeat(100),
    );
    for (name, rule) in &mut grammar.rules {
        retain(&mut rule.body);
        rule.class = if name == "Target" { "Target" } else { "Root" }.into();
        grammar.declaration_sources.insert(
            name.clone(),
            SourceSpan {
                source: source.id.clone(),
                start: 0,
                end: 100,
            },
        );
    }
    grammar
}
#[test]
fn productive_nullable_full_operand_requires_real_match_and_always_restores_input() {
    let program = nullable_repeat_program();
    let before = serde_json::to_vec(&program).unwrap();
    for (text, expected) in [("", None), ("x", Some(0)), ("x x", Some(0)), ("y", None)] {
        let tokens = symbols(text);
        let mut input = TraceReplay::new(&tokens);
        let mut remaining = Some(10000);
        let result = program.predict_productive_nullable_full_repeat_with_replay_input(
            "rule/Entry/s1/repeat",
            "Target",
            "x",
            &mut input,
            &mut remaining,
            CompletionVerification::GuardOverapproximation,
        );
        assert_eq!(result.unwrap(), expected);
        assert_eq!(input.inner.position, 0);
        assert!(matches!(
            input.events.last(),
            Some(TraceEvent::Rewind(_, 0))
        ));
        println!(
            "V77_PRODUCTIVE text={text:?} consumed={} trace={:?}",
            10000 - remaining.unwrap(),
            input.events
        );
    }
    let tokens = symbols("x");
    let mut input = TraceReplay::new(&tokens);
    let mut unbounded = None;
    assert_eq!(
        program
            .predict_productive_nullable_full_repeat_with_replay_input(
                "rule/Entry/s1/repeat",
                "Target",
                "x",
                &mut input,
                &mut unbounded,
                CompletionVerification::GuardOverapproximation
            )
            .unwrap_err(),
        "Productive nullable repeat requires a finite shared work budget"
    );
    assert_eq!(unbounded, None);
    assert!(input.events.is_empty());
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}
#[test]
fn productive_nullable_repeat_shared_zero_exact_short_and_cleanup_fees() {
    let program = nullable_repeat_program();
    let tokens = symbols("x x");
    let mut input = TraceReplay::new(&tokens);
    let mut remaining = Some(10000);
    assert_eq!(
        program
            .predict_productive_nullable_full_repeat_with_replay_input(
                "rule/Entry/s1/repeat",
                "Target",
                "x",
                &mut input,
                &mut remaining,
                CompletionVerification::GuardOverapproximation
            )
            .unwrap(),
        Some(0)
    );
    let used = 10000 - remaining.unwrap();
    assert!(used > 8);
    // Initial focused execution must print this actual fee before a later
    // literal pin; no asserted capacity/saving is inferred from the measurement.
    println!("V77_PRODUCTIVE_EXACT_FEE used={used}");
    for allowance in 0..=used {
        let mut input = TraceReplay::new(&tokens);
        let mut remaining = Some(allowance);
        let result = program.predict_productive_nullable_full_repeat_with_replay_input(
            "rule/Entry/s1/repeat",
            "Target",
            "x",
            &mut input,
            &mut remaining,
            CompletionVerification::GuardOverapproximation,
        );
        if allowance == used {
            assert_eq!(result.unwrap(), Some(0));
        } else {
            assert!(result.unwrap_err().contains("budget"));
        }
        assert_eq!(remaining, Some(0));
        assert_eq!(input.inner.position, 0);
        if input
            .events
            .iter()
            .any(|event| matches!(event, TraceEvent::Read(_, _)))
        {
            assert!(matches!(
                input.events.last(),
                Some(TraceEvent::Rewind(_, 0))
            ));
        }
    }
    assert!(
        program
            .predict_productive_nullable_full_repeat_with_replay_input(
                "rule/Entry/s1/repeat",
                "Target",
                "x",
                &mut ReplayTokens::new(&tokens),
                &mut None,
                CompletionVerification::GuardOverapproximation
            )
            .unwrap_err()
            .contains("finite")
    );
}
#[test]
fn nullable_keyword_actual_decision_is_local_and_greedy_for_other_caller_follows_too() {
    let program = nullable_repeat_program();
    let before = serde_json::to_vec(&program).unwrap();
    for (text, expected) in [
        ("", None),
        ("x", Some(0)),
        ("x x", Some(0)),
        ("x y", Some(0)),
        ("y", None),
    ] {
        let tokens = symbols(text);
        let mut remaining = Some(10000);
        assert_eq!(
            program
                .predict_rule_bounded_keyword_optional_with_replay_input(
                    "rule/Target/s1/repeat",
                    "Target",
                    "x",
                    &mut ReplayTokens::new(&tokens),
                    &mut remaining,
                    CompletionVerification::GuardOverapproximation
                )
                .unwrap(),
            expected
        );
    }
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
    // This is an explicit low-level local-choice control. It does not claim an
    // independent whole-parser oracle for a new surrounding caller grammar.
}
#[test]
fn productive_binding_rejects_wrong_callee_stop_fanout_and_hidden_guard_before_reads() {
    let original = nullable_repeat_program();
    for kind in 0..5 {
        let mut program = original.clone();
        let decision = program.decisions["rule/Entry/s1/repeat"].clone();
        let body = decision.entries[0];
        match kind {
            0 => program
                .decisions
                .get_mut("rule/Entry/s1/repeat")
                .unwrap()
                .entries
                .swap(0, 1),
            1 => {
                if let Edge::Full { stop, .. } = &mut program.states[body].edges[0] {
                    *stop = body;
                }
            }
            2 => {
                let stop = program.rules["Target"].1;
                program.states[stop].follow.clear();
            }
            3 => {
                program.states[body].edges = vec![Edge::First {
                    path: "unbound".into(),
                    target: body,
                }]
            }
            _ => {
                let stop = program.rules["Target"].1;
                program.states[stop].rule_stop = false;
            }
        }
        let tokens = symbols("x");
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(10000);
        assert!(
            program
                .predict_productive_nullable_full_repeat_with_replay_input(
                    "rule/Entry/s1/repeat",
                    "Target",
                    "x",
                    &mut input,
                    &mut work,
                    CompletionVerification::GuardOverapproximation
                )
                .is_err()
        );
        assert!(input.events.is_empty());
        assert!(work.unwrap() < 10000);
    }
    for (owner, keyword) in [("Entry", "x"), ("Target", "wrong"), ("Missing", "x")] {
        let tokens = symbols("x");
        let mut input = TraceReplay::new(&tokens);
        assert!(
            original
                .predict_rule_bounded_keyword_optional_with_replay_input(
                    "rule/Target/s1/repeat",
                    owner,
                    keyword,
                    &mut input,
                    &mut Some(10000),
                    CompletionVerification::GuardOverapproximation
                )
                .is_err()
        );
        assert!(input.events.is_empty());
    }
}
#[test]
fn productive_completion_does_not_count_eof_or_nested_speculative_read_as_progress() {
    let mut program = nullable_repeat_program();
    let body = program.decisions["rule/Target/s1/repeat"].entries[0];
    let target = match &program.states[body].edges[0] {
        Edge::Token(_, target) => *target,
        _ => panic!("Expected exact original token body"),
    };
    program.states[body].edges[0] = Edge::Token(PredictionSymbol::End, target);
    let (start, stop) = program.rules["Target"];
    // Keep the observed refusal: an optional End is ambiguous at this whole
    // rule scope. It cannot establish the productivity worker's End behavior.
    match program.verify_completion_scope_with_verification(
        start,
        stop,
        &mut 10000,
        CompletionVerification::GuardOverapproximation,
    ) {
        Err(message) => assert_eq!(
            message,
            "Completion static verification failed at rule/Target/s1/repeat: Ambiguous parser prediction requires predicate resolution"
        ),
        Ok(_) => panic!("Unbound optional End unexpectedly became a qualified scope"),
    }
    // The same builder supplies one mandatory token edge and its actual rule
    // stop. Changing that token to End introduces no choice or fake scope.
    let mut program = ParserPrediction::compile(&grammar(keyword("x"), vec![])).unwrap();
    let (start, stop) = program.rules["Entry"];
    assert!(
        matches!(program.states[start].edges.as_slice(), [Edge::Token(PredictionSymbol::Keyword(value), target)] if value=="x" && *target==stop)
    );
    program.states[start].edges[0] = Edge::Token(PredictionSymbol::End, stop);
    let tokens = symbols("");
    for (productivity, expected) in [
        (CompletionProductivity::Any, true),
        (CompletionProductivity::MustConsumeSourceToken, false),
    ] {
        let mut work = Some(10000);
        let scope = program
            .verify_completion_scope_with_verification(
                start,
                stop,
                work.as_mut().unwrap(),
                CompletionVerification::GuardOverapproximation,
            )
            .unwrap();
        let mut input = ReplayTokens::new(&tokens);
        assert!(
            matches!(program.run_prediction_at_depth_with_completion_verification(PredictionGoal::Completion{start,stop,scope:&scope,productivity},&mut input,&mut work,0,CompletionVerification::GuardOverapproximation).unwrap(),PredictionOutcome::Completed(actual) if actual==expected)
        );
    }
    // Existing x-probe on y reads a nonmatching visible token in its nested
    // optional decision, rewinds, and must still complete unproductively.
    let program = nullable_repeat_program();
    let tokens = symbols("y");
    let mut input = TraceReplay::new(&tokens);
    assert_eq!(
        program
            .predict_productive_nullable_full_repeat_with_replay_input(
                "rule/Entry/s1/repeat",
                "Target",
                "x",
                &mut input,
                &mut Some(10000),
                CompletionVerification::GuardOverapproximation
            )
            .unwrap(),
        None
    );
    assert!(
        input
            .events
            .iter()
            .any(|event| matches!(event, TraceEvent::Read(_, _)))
    );
    assert_eq!(input.inner.position, 0);
}
#[test]
fn productive_completion_keeps_cumulative_runtime_call128() {
    for mutual in [false, true] {
        for short in [false, true] {
            let callers = if mutual { 119 } else { 121 } + usize::from(short);
            let mut nested = full(call(if mutual { "Other" } else { "Inner" }));
            nested.cardinality = '?';
            let mut owned = vec![
                (
                    "Inner".to_owned(),
                    sequence(vec![keyword("a"), nested, keyword("b")]),
                ),
                (
                    "Gate".to_owned(),
                    choice(vec![full(call("Inner")), keyword("c")]),
                ),
            ];
            if mutual {
                owned.push(("Other".into(), call("Inner")));
            }
            for index in 0..callers {
                owned.push((
                    format!("Caller{index}"),
                    if index + 1 == callers {
                        call("Gate")
                    } else {
                        call(&format!("Caller{}", index + 1))
                    },
                ));
            }
            let mut g = grammar(
                call("Caller0"),
                owned
                    .iter()
                    .map(|(name, body)| (name.as_str(), body.clone()))
                    .collect(),
            );
            g.profile = Profile::NativeV74;
            let program =
                ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new())
                    .unwrap();
            let (start, stop) = program.rules["Entry"];
            let mut work = Some(100000);
            let scope = program
                .verify_completion_scope_with_verification(
                    start,
                    stop,
                    work.as_mut().unwrap(),
                    CompletionVerification::GuardOverapproximation,
                )
                .unwrap();
            let tokens = symbols("a a b b");
            let mut input = ReplayTokens::new(&tokens);
            let result = program.run_prediction_at_depth_with_completion_verification(
                PredictionGoal::Completion {
                    start,
                    stop,
                    scope: &scope,
                    productivity: CompletionProductivity::MustConsumeSourceToken,
                },
                &mut input,
                &mut work,
                0,
                CompletionVerification::GuardOverapproximation,
            );
            if short {
                assert!(result.unwrap_err().contains("depth"));
            } else {
                assert!(matches!(
                    result.unwrap(),
                    PredictionOutcome::Completed(true)
                ));
            }
            assert_eq!(
                100000 - work.unwrap(),
                match (mutual, short) {
                    (false, false) => 2689,
                    (false, true) => 1733,
                    (true, false) => 2735,
                    (true, true) => 1741,
                }
            );
        }
    }
}

#[test]
fn productive_completion_keeps_static_call64_and_explicit_caller_proof_allowance() {
    for count in [64, 65] {
        let owned: Vec<_> = (0..count)
            .map(|i| {
                (
                    format!("A{i}"),
                    if i + 1 == count {
                        keyword("a")
                    } else {
                        call(&format!("A{}", i + 1))
                    },
                )
            })
            .collect();
        let mut g = grammar(
            choice(vec![sequence(vec![call("A0"), keyword("x")]), keyword("z")]),
            owned
                .iter()
                .map(|(name, body)| (name.as_str(), body.clone()))
                .collect(),
        );
        g.profile = Profile::NativeV74;
        let program = ParserPrediction::compile(&g).unwrap();
        let (start, stop) = program.rules["Entry"];
        let mut work = 100000;
        let scope = program.verify_completion_scope_with_verification(
            start,
            stop,
            &mut work,
            CompletionVerification::GuardOverapproximation,
        );
        if count == 64 {
            assert_eq!(100000 - work, 622);
            let scope = scope.unwrap();
            let tokens = symbols("a x");
            assert!(matches!(
                program
                    .run_prediction_at_depth_with_completion_verification(
                        PredictionGoal::Completion {
                            start,
                            stop,
                            scope: &scope,
                            productivity: CompletionProductivity::MustConsumeSourceToken
                        },
                        &mut ReplayTokens::new(&tokens),
                        &mut Some(work),
                        0,
                        CompletionVerification::GuardOverapproximation
                    )
                    .unwrap(),
                PredictionOutcome::Completed(true)
            ));
        } else {
            assert!(scope.err().unwrap().contains("budget"));
            assert_eq!(100000 - work, 616);
            assert!(work > 90000);
        }
    }
    // A valid linear graph crosses 10k work without a malformed fanout.
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
    let mut work = super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units();
    let error = program
        .verify_completion_scope_with_verification(
            0,
            6000,
            &mut work,
            CompletionVerification::GuardOverapproximation,
        )
        .err()
        .unwrap();
    assert_eq!(error, "Grammar prediction budget exceeded");
    assert_eq!(work, 0);
    let mut work = 100000;
    let scope = program
        .verify_completion_scope_with_verification(
            0,
            6000,
            &mut work,
            CompletionVerification::GuardOverapproximation,
        )
        .unwrap();
    assert!(work > 0 && work < 90000);
    // Scope eligibility cannot turn an empty path into productive completion.
    assert!(matches!(
        program
            .run_prediction_at_depth_with_completion_verification(
                PredictionGoal::Completion {
                    start: 0,
                    stop: 6000,
                    scope: &scope,
                    productivity: CompletionProductivity::MustConsumeSourceToken,
                },
                &mut ReplayTokens::new(&[]),
                &mut Some(work),
                0,
                CompletionVerification::GuardOverapproximation,
            )
            .unwrap(),
        PredictionOutcome::Completed(false)
    ));
}

#[test]
fn nullable_required_call_composes_real_full_loop_and_exact_local_choice_without_old_waiver() {
    let program = nullable_repeat_program();
    let grammar = nullable_required_source_fixture();
    let before = serde_json::to_vec(&program).unwrap();
    let mut work = 10000;
    program
        .verify_required_object_call_with_nullable_full_source(
            &grammar,
            "Entry",
            "rule/Entry/s1/value/value",
            "Target",
            &mut work,
        )
        .unwrap();
    let used = 10000 - work;
    println!("V77_REQUIRED_FULL_COMPOSITION_FEE used={used}");
    assert!(used > 20);
    for amount in 0..=used {
        let mut work = amount;
        let result = program.verify_required_object_call_with_nullable_full_source(
            &grammar,
            "Entry",
            "rule/Entry/s1/value/value",
            "Target",
            &mut work,
        );
        if amount == used {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err(), "Grammar prediction budget exceeded");
        }
        assert_eq!(work, 0);
    }
    // The old ordinary proof still performs its original stronger guarded-exit
    // prerequisite; the new proof neither creates that guard nor edits metadata.
    let old =
        program.verify_required_object_call("rule/Entry/s1/value/value", "Target", &mut 10000);
    assert!(old.is_err());
    println!("V77_OLD_REQUIRED_UNCHANGED_ERROR result={old:?}");
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
    for (owner, path, callee) in [
        ("Wrong", "rule/Entry/s1/value/value", "Target"),
        ("Entry", "rule/Entry/s1/value/value", "Entry"),
        ("Entry", "rule/Entry/s0/value/value", "Target"),
    ] {
        assert!(
            program
                .verify_required_object_call_with_nullable_full_source(
                    &grammar, owner, path, callee, &mut 10000
                )
                .is_err()
        );
    }
}
#[test]
fn nullable_composed_call_rejects_foreign_occurrence_return_keyword_and_unbound_scope() {
    let original = nullable_repeat_program();
    let grammar = nullable_required_source_fixture();
    for mode in 0..7 {
        let mut program = original.clone();
        let call = program.decisions["rule/Entry/s1/value/value/recovery"].entries[0];
        let optional = program.decisions["rule/Target/s1/repeat"].entries[0];
        match mode {
            0 => program
                .decisions
                .get_mut("rule/Entry/s1/value/value/recovery")
                .unwrap()
                .entries
                .swap(0, 1),
            1 => {
                if let Edge::Call { follow, .. } = &mut program.states[call].edges[0] {
                    *follow = call;
                }
            }
            2 => {
                let stop = program.rules["Target"].1;
                program.states[stop].follow.clear();
            }
            3 => {
                if let Edge::Token(PredictionSymbol::Keyword(keyword), _) =
                    &mut program.states[optional].edges[0]
                {
                    *keyword = "foreign".into();
                }
            }
            4 => {
                let start = program.rules["Target"].0;
                program.states[start].edges = vec![Edge::Epsilon(optional)];
            }
            5 => {
                let decision = program.decisions.get_mut("rule/Target/s1/repeat").unwrap();
                decision.entries.swap(0, 1);
            }
            _ => {
                program
                    .recursive_decisions
                    .insert("rule/Target/s1/repeat".into());
            }
        }
        let mut work = 10000;
        assert!(
            program
                .verify_required_object_call_with_nullable_full_source(
                    &grammar,
                    "Entry",
                    "rule/Entry/s1/value/value",
                    "Target",
                    &mut work
                )
                .is_err(),
            "mode {mode}"
        );
        assert!(work < 10000);
    }
    let mut foreign = grammar.clone();
    foreign.declaration_sources.remove("Target");
    assert!(
        original
            .verify_required_object_call_with_nullable_full_source(
                &foreign,
                "Entry",
                "rule/Entry/s1/value/value",
                "Target",
                &mut 10000
            )
            .is_err()
    );
}
#[test]
fn productive_local_choice_preserves_real_outer_call_depth128_and_prepaid_restore() {
    let original = nullable_repeat_program();
    for callers in [0, 1, 127, 128] {
        let mut program = original.clone();
        let body = program.decisions["rule/Entry/s1/repeat"].entries[0];
        let (mut start, mut stop) = match program.states[body].edges.as_slice() {
            [Edge::Full { target, stop }] => (*target, *stop),
            _ => panic!("Expected full operand"),
        };
        program.states[stop].rule_stop = true;
        program.states[stop].edges.clear();
        for _ in 0..callers {
            let entry = program.states.len();
            let end = entry + 1;
            program.states.push(State {
                edges: vec![Edge::Call {
                    target: start,
                    follow: end,
                }],
                ..State::default()
            });
            program.states.push(State {
                rule_stop: true,
                ..State::default()
            });
            start = entry;
            stop = end;
        }
        program
            .verify_nullable_keyword_optional_binding(
                "rule/Target/s1/repeat",
                "Target",
                "x",
                &mut 10000,
            )
            .unwrap();
        let local = RuleLocalCompletionDecision {
            path: "rule/Target/s1/repeat",
            decision: &program.decisions["rule/Target/s1/repeat"],
            entry: program.decisions["rule/Target/s1/recovery"].entries[0],
            stop: program.rules["Target"].1,
        };
        let scope = program
            .verify_completion_scope_with_local_decision(
                start,
                stop,
                &mut 10000,
                CompletionVerification::GuardOverapproximation,
                Some(local),
            )
            .unwrap();
        let tokens = symbols("x");
        let mut input = TraceReplay::new(&tokens);
        let mut remaining = Some(100000);
        let result = program.run_prediction_at_depth_with_local_decision(
            PredictionGoal::Completion {
                start,
                stop,
                scope: &scope,
                productivity: CompletionProductivity::MustConsumeSourceToken,
            },
            &mut input,
            &mut remaining,
            0,
            CompletionVerification::GuardOverapproximation,
            Some(local),
        );
        if callers < 128 {
            assert!(matches!(
                result.unwrap(),
                PredictionOutcome::Completed(true)
            ));
        } else {
            assert_eq!(result.unwrap_err(), "Parser prediction call depth exceeded");
        }
        assert!(
            remaining.unwrap() > 90000,
            "test-only large pool isolates unchanged call128"
        );
        // Outer input is advanced only by its real successful frontier match;
        // nested local speculation has its own paid rewind in the trace.
        if callers < 128 {
            assert_eq!(input.inner.position, 1);
            assert!(
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, TraceEvent::Rewind(_, 0)))
            );
        }
    }
}

#[test]
fn grammar_deferred_work_pool_nullable_source_qualification_exact_and_fallback_conserve() {
    let grammar = nullable_required_source_fixture();
    let program = nullable_repeat_program();
    let path = "rule/Target/s1/repeat";
    let before = serde_json::to_vec(&program).unwrap();
    assert!(
        grammar
            .qualified_nullable_keyword_optional_with_work(path, &mut 10000)
            .unwrap()
            .is_some()
    );
    program
        .verify_nullable_deferred_decision(path, &grammar)
        .unwrap();
    let mut remaining = 100000;
    program
        .verify_nullable_deferred_decision_with_remaining(path, &grammar, &mut remaining)
        .unwrap();
    let used = 100000 - remaining;
    assert!(used > 0);
    for grant in [0, used - 1, used] {
        let mut remaining = grant;
        let actual = program.verify_nullable_deferred_decision_with_remaining(
            path,
            &grammar,
            &mut remaining,
        );
        if grant == used {
            actual.unwrap();
        } else {
            assert!(actual.unwrap_err().contains("budget"));
        }
        assert_eq!(remaining, 0);
    }
    let mut shared = used * 2;
    program
        .verify_nullable_deferred_decision_with_remaining(path, &grammar, &mut shared)
        .unwrap();
    program
        .verify_nullable_deferred_decision_with_remaining(path, &grammar, &mut shared)
        .unwrap();
    assert_eq!(shared, 0);
    // Unsupported source shape does not gain an optional-keyword lemma. It
    // takes the SAME static fallback with the SAME remaining source/proof pool.
    let mut foreign = grammar.clone();
    let Kind::Sequence(items) = &mut foreign.rules.get_mut("Target").unwrap().body.kind else {
        panic!("Missing nullable source fixture sequence");
    };
    items[1].cardinality = '1';
    let mut source_work = 100000;
    assert!(
        foreign
            .qualified_nullable_keyword_optional_with_work(path, &mut source_work)
            .unwrap()
            .is_none()
    );
    let source_used = 100000 - source_work;
    let mut expected_remaining = 100000;
    for _ in 0..source_used {
        analysis_charge(&mut expected_remaining).unwrap();
    }
    let expected =
        program.verify_disjoint_with_discovery_remaining(path, &mut expected_remaining, Some(2000));
    let mut actual_remaining = 100000;
    let actual = program.verify_nullable_deferred_decision_with_remaining(
        path,
        &foreign,
        &mut actual_remaining,
    );
    assert_eq!(actual, expected);
    assert_eq!(actual_remaining, expected_remaining);
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}
