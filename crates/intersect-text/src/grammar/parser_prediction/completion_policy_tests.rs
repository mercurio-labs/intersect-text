// Explicit proof policy; every actual guard still uses the one replay evaluator.
fn opaque_caller_repeat_program() -> ParserPrediction {
    let mut inside = call("Item");
    inside.cardinality = '*';
    let mut optional = sequence(vec![keyword("("), inside, keyword(")")]);
    optional.cardinality = '?';
    let mut guarded = full(call("Item"));
    guarded.cardinality = '*';
    let mut g = grammar(
        sequence(vec![keyword("{"), guarded, keyword("}")]),
        vec![("Item", sequence(vec![keyword("x"), optional]))],
    );
    g.profile = super::super::Profile::NativeV74;
    ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap()
}

#[test]
fn explicit_completion_policy_preserves_legacy_refusal_and_qualifies_the_same_nested_repeat() {
    let program = opaque_caller_repeat_program();
    let before = serde_json::to_vec(&program).unwrap();
    let tokens = symbols("}");
    let mut input = ReplayTokens::new(&tokens);
    assert_eq!(program.predict_with_replay_input("rule/Entry/s1/repeat", &mut input, &mut Some(100_000)),
        Err("Completion static verification failed at rule/Item/s1/repeat: Full-predicate shared-prefix proof requires explicit overapproximation".into()));
    for (text, expected) in [("}", None), ("x ( x ) x }", Some(0))] {
        let tokens = symbols(text);
        let mut input = ReplayTokens::new(&tokens);
        assert_eq!(
            program
                .predict_with_replay_input_and_completion_verification(
                    "rule/Entry/s1/repeat",
                    &mut input,
                    &mut Some(100_000),
                    CompletionVerification::GuardOverapproximation
                )
                .unwrap(),
            expected
        );
    }
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
    assert_eq!(std::mem::size_of::<CompletionVerification>(), 1);
}

#[test]
fn explicit_completion_policy_keeps_one_shared_work_pool_and_prepaid_cleanup() {
    let program = opaque_caller_repeat_program();
    let tokens = symbols("x ( x ) x }");
    let mut input = TraceReplay::new(&tokens);
    let mut work = Some(100_000);
    let result = program.predict_with_replay_input_and_completion_verification(
        "rule/Entry/s1/repeat",
        &mut input,
        &mut work,
        CompletionVerification::GuardOverapproximation,
    );
    assert_eq!(result.unwrap(), Some(0));
    let used = 100_000 - work.unwrap();
    assert_eq!(used, 235);
    println!("V74_COMPLETION_PROOF used={used} trace={:?}", input.events);
    for allowance in [0, used - 1, used] {
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(allowance);
        let result = program.predict_with_replay_input_and_completion_verification(
            "rule/Entry/s1/repeat",
            &mut input,
            &mut work,
            CompletionVerification::GuardOverapproximation,
        );
        if allowance == used {
            assert_eq!(result.unwrap(), Some(0));
        } else {
            assert!(result.unwrap_err().contains("budget"));
        }
        assert_eq!(work, Some(0));
    }
    let mut witnessed = false;
    for allowance in 1..used {
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(allowance);
        let result = program.predict_with_replay_input_and_completion_verification(
            "rule/Entry/s1/repeat",
            &mut input,
            &mut work,
            CompletionVerification::GuardOverapproximation,
        );
        if input
            .events
            .iter()
            .any(|event| matches!(event,TraceEvent::Read(position,Some(0)) if *position>0))
        {
            assert_eq!(allowance, 153);
            assert!(result.unwrap_err().contains("budget"));
            assert_eq!(work, Some(0));
            assert_eq!(input.inner.position, 0);
            assert!(matches!(input.events.last(),Some(TraceEvent::Rewind(from,0)) if *from>0));
            println!(
                "V74_COMPLETION_DENIAL allowance={allowance} trace={:?}",
                input.events
            );
            witnessed = true;
            break;
        }
    }
    assert!(witnessed, "an exhausted consumed replay was not observed");
    let mut input = ReplayTokens::new(&tokens);
    assert!(
        program
            .predict_with_replay_input_and_completion_verification(
                "rule/Entry/s1/repeat",
                &mut input,
                &mut None,
                CompletionVerification::GuardOverapproximation
            )
            .unwrap_err()
            .contains("finite")
    );
}

#[test]
fn enlarged_completion_languages_still_reject_overlap_hidden_guards_and_invalid_boundaries() {
    let ambiguous =
        ParserPrediction::compile(&grammar(choice(vec![keyword("a"), keyword("a")]), vec![]))
            .unwrap();
    let &(start, stop) = ambiguous.rules.get("Entry").unwrap();
    assert!(
        ambiguous
            .verify_completion_scope_with_verification(
                start,
                stop,
                &mut 10_000,
                CompletionVerification::GuardOverapproximation
            )
            .is_err()
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
    let &(start, stop) = hidden.rules.get("Nested").unwrap();
    assert_eq!(
        hidden
            .verify_completion_scope_with_verification(
                start,
                stop,
                &mut 10_000,
                CompletionVerification::GuardOverapproximation
            )
            .err()
            .unwrap(),
        "Completion guard visibility remains unsupported at rule/Nested/a0/s1"
    );
    let mut unbound = hidden.clone();
    unbound.first_guards.clear();
    assert!(
        unbound
            .verify_completion_scope_with_verification(
                start,
                stop,
                &mut 10_000,
                CompletionVerification::GuardOverapproximation
            )
            .err()
            .unwrap()
            .contains("Unbound completion first-token guard")
    );
    let program = opaque_caller_repeat_program();
    let &(start, stop) = program.rules.get("Entry").unwrap();
    assert_eq!(
        program
            .verify_completion_scope_with_verification(
                usize::MAX,
                stop,
                &mut 10_000,
                CompletionVerification::GuardOverapproximation
            )
            .err()
            .unwrap(),
        "Dangling completion state"
    );
    let mut bad = program.clone();
    let mut changed = false;
    for state in &mut bad.states {
        for edge in &mut state.edges {
            if let Edge::Full { stop, .. } = edge {
                *stop = usize::MAX;
                changed = true;
            }
        }
    }
    assert!(changed);
    assert_eq!(
        bad.verify_completion_scope_with_verification(
            start,
            stop,
            &mut 10_000,
            CompletionVerification::GuardOverapproximation
        )
        .err()
        .unwrap(),
        "Dangling full-predicate operand boundary"
    );
    let mut bad = program;
    for state in &mut bad.states {
        for edge in &mut state.edges {
            if let Edge::Call { follow, .. } = edge {
                *follow = usize::MAX;
            }
        }
    }
    assert_eq!(
        bad.verify_completion_scope_with_verification(
            start,
            stop,
            &mut 10_000,
            CompletionVerification::GuardOverapproximation
        )
        .err()
        .unwrap(),
        "Dangling completion state"
    );
}

#[test]
fn explicit_completion_policy_preserves_original_full_oracle_rows_and_legacy_traces() {
    let program = full_oracle_program(false);
    let before = serde_json::to_vec(&program).unwrap();
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../assets/fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["parser"] == "RecursiveEntryFull")
    {
        let tokens = symbols(row["text"].as_str().unwrap());
        let mut old = TraceReplay::new(&tokens);
        let mut old_work = Some(100_000);
        let old_result =
            program.predict_with_replay_input("rule/Entry/choice", &mut old, &mut old_work);
        let mut explicit = TraceReplay::new(&tokens);
        let mut explicit_work = Some(100_000);
        let explicit_result = program.predict_with_replay_input_and_completion_verification(
            "rule/Entry/choice",
            &mut explicit,
            &mut explicit_work,
            CompletionVerification::Legacy,
        );
        assert_eq!(old_result, explicit_result);
        assert_eq!(old_work, explicit_work);
        assert_eq!(
            format!("{:?}", old.events),
            format!("{:?}", explicit.events)
        );
        let mut input = ReplayTokens::new(&tokens);
        assert_eq!(
            program
                .predict_with_replay_input_and_completion_verification(
                    "rule/Entry/choice",
                    &mut input,
                    &mut Some(100_000),
                    CompletionVerification::GuardOverapproximation
                )
                .unwrap(),
            match row["branch"].as_str().unwrap() {
                "g" => Some(0),
                "f" => Some(1),
                "" => None,
                branch => panic!("unknown branch {branch}"),
            }
        );
        count += 1;
    }
    assert_eq!(count, 11);
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn explicit_completion_policy_retains_static_call64_and_work10000_inside_larger_allowance() {
    for count in [64, 65] {
        let owned: Vec<_> = (0..count)
            .map(|index| {
                (
                    format!("A{index}"),
                    if index + 1 == count {
                        keyword("a")
                    } else {
                        call(&format!("A{}", index + 1))
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
        g.profile = super::super::Profile::NativeV74;
        let program = ParserPrediction::compile(&g).unwrap();
        let &(start, stop) = program.rules.get("Entry").unwrap();
        let mut work = 100_000;
        let result = program.verify_completion_scope_with_verification(
            start,
            stop,
            &mut work,
            CompletionVerification::GuardOverapproximation,
        );
        if count == 64 {
            assert!(result.is_ok());
            assert_eq!(100_000 - work, 622);
        } else {
            assert!(result.err().unwrap().contains("budget"));
            assert_eq!(100_000 - work, 616);
            assert!(
                work > 90_000,
                "depth failure was not distinguished from the static work cap"
            );
        }
        println!(
            "V74_COMPLETION_STATIC calls={count} consumed={}",
            100_000 - work
        );
    }
    let program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states: vec![
            State {
                edges: vec![Edge::Epsilon(1); 10_000],
                ..State::default()
            },
            State {
                rule_stop: true,
                ..State::default()
            },
        ],
        rules: [("Operand".into(), (0, 1))].into(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    // The caller selects the retained legacy limit explicitly.
    let mut work = super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units();
    assert_eq!(
        program
            .verify_completion_scope_with_verification(
                0,
                1,
                &mut work,
                CompletionVerification::GuardOverapproximation
            )
            .err()
            .unwrap(),
        "Grammar prediction budget exceeded"
    );
    assert_eq!(work, 0);
    // A larger caller pool reaches the distinct invalid-graph boundary.
    let mut work = 100_000;
    assert_eq!(
        program
            .verify_completion_scope_with_verification(
                0,
                1,
                &mut work,
                CompletionVerification::GuardOverapproximation,
            )
            .err()
            .unwrap(),
        "Completion contains an unbound branch state"
    );
    assert!(work > 0 && 100_000 - work > 10_000);
}

#[test]
fn explicit_completion_policy_productive_nested_calls_keep_cumulative_runtime128() {
    for mutual in [false, true] {
        for one_short in [false, true] {
            let callers = if mutual { 119 } else { 121 } + usize::from(one_short);
            let mut nested = full(call(if mutual { "Other" } else { "Inner" }));
            nested.cardinality = '?';
            let mut owned = vec![
                (
                    "Inner".to_string(),
                    sequence(vec![keyword("a"), nested, keyword("b")]),
                ),
                (
                    "Gate".to_string(),
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
            g.profile = super::super::Profile::NativeV74;
            let program =
                ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new())
                    .unwrap();
            let &(start, stop) = program.rules.get("Entry").unwrap();
            let tokens = symbols("a a b b");
            let mut work = Some(100_000);
            let scope = program
                .verify_completion_scope_with_verification(
                    start,
                    stop,
                    work.as_mut().unwrap(),
                    CompletionVerification::GuardOverapproximation,
                )
                .unwrap();
            let mut input = ReplayTokens::new(&tokens);
            let result = program.run_prediction_at_depth_with_completion_verification(
                PredictionGoal::Completion {
                    start,
                    stop,
                    scope: &scope,
                    productivity: CompletionProductivity::Any,
                },
                &mut input,
                &mut work,
                0,
                CompletionVerification::GuardOverapproximation,
            );
            if one_short {
                assert!(result.unwrap_err().contains("depth"));
            } else {
                assert!(matches!(
                    result.unwrap(),
                    PredictionOutcome::Completed(true)
                ));
                assert_eq!(input.position + 1, tokens.len());
            }
            assert_eq!(
                100_000 - work.unwrap(),
                match (mutual, one_short) {
                    (false, false) => 2689,
                    (false, true) => 1733,
                    (true, false) => 2735,
                    (true, true) => 1741,
                }
            );
            println!(
                "V74_COMPLETION_DEPTH mutual={mutual} callers={callers} one_short={one_short} consumed={}",
                100_000 - work.unwrap()
            );
        }
    }
}

// Same frozen depth/work fixtures; the old selector and its exact work
// assertions above remain byte-literal. This new strategy has no larger pool.
#[test]
fn bounded_completion_policy_retains_static_call64_and_work10000_inside_larger_allowance() {
    for count in [64, 65] {
        let owned: Vec<_> = (0..count)
            .map(|index| {
                (
                    format!("A{index}"),
                    if index + 1 == count {
                        keyword("a")
                    } else {
                        call(&format!("A{}", index + 1))
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
        g.profile = super::super::Profile::NativeV74;
        let program = ParserPrediction::compile(&g).unwrap();
        let &(start, stop) = program.rules.get("Entry").unwrap();
        let mut work = 100_000;
        let result = program.verify_completion_scope_with_verification(
            start,
            stop,
            &mut work,
            CompletionVerification::BoundedGuardOverapproximation,
        );
        if count == 64 {
            assert!(result.is_ok());
        } else {
            assert!(result.err().unwrap().contains("budget"));
            assert!(
                work > 90_000,
                "depth failure was not distinguished from the static work cap"
            );
        }
        println!(
            "BOUNDED_COMPLETION_STATIC calls={count} consumed={}",
            100_000 - work
        );
    }
    let program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states: vec![
            State {
                edges: vec![Edge::Epsilon(1); 10_000],
                ..State::default()
            },
            State {
                rule_stop: true,
                ..State::default()
            },
        ],
        rules: [("Operand".into(), (0, 1))].into(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    // The caller selects the retained legacy limit explicitly.
    let mut work = super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units();
    assert_eq!(
        program
            .verify_completion_scope_with_verification(
                0,
                1,
                &mut work,
                CompletionVerification::BoundedGuardOverapproximation
            )
            .err()
            .unwrap(),
        "Grammar prediction budget exceeded"
    );
    assert_eq!(work, 0);
    // A larger caller pool reaches the distinct invalid-graph boundary.
    let mut work = 100_000;
    assert_eq!(
        program
            .verify_completion_scope_with_verification(
                0,
                1,
                &mut work,
                CompletionVerification::BoundedGuardOverapproximation,
            )
            .err()
            .unwrap(),
        "Completion contains an unbound branch state"
    );
    assert!(work > 0 && 100_000 - work > 10_000);
}

// These controls invoke the existing low-level scope/Decision worker over its
// canonical graph builder. They do not publish a native typed compilation;
// actual ambiguous grammars still fail the unchanged deferred compiler proof.
fn input_checked_ordinary_choice_program(ambiguous: bool) -> ParserPrediction {
    let alternatives = if ambiguous {
        choice(vec![keyword("x"), keyword("x")])
    } else {
        choice(vec![
            sequence(vec![keyword("x"), keyword("q")]),
            sequence(vec![keyword("x"), keyword("r")]),
        ])
    };
    ParserPrediction::compile(&grammar(
        call("Wrap"),
        vec![
            (
                "Wrap",
                sequence(vec![keyword("<"), call("Name"), keyword(">")]),
            ),
            ("Name", alternatives),
        ],
    ))
    .unwrap()
}
type OrdinaryChoiceProbe = (Result<bool, String>, usize, usize, Vec<(u8, usize, usize)>);

fn input_checked_ordinary_choice_probe(
    program: &ParserPrediction,
    text: &str,
    allowance: usize,
    verification: CompletionVerification,
) -> OrdinaryChoiceProbe {
    let (start, stop) = program.rules["Entry"];
    let tokens = symbols(text);
    let mut input = TraceReplay::new(&tokens);
    let mut work = Some(allowance);
    let result = (|| {
        let scope = program.verify_completion_scope_with_verification(
            start,
            stop,
            work.as_mut().unwrap(),
            verification,
        )?;
        match program.run_prediction_at_depth_with_completion_verification(
            PredictionGoal::Completion {
                start,
                stop,
                scope: &scope,
                productivity: CompletionProductivity::Any,
            },
            &mut input,
            &mut work,
            0,
            verification,
        )? {
            PredictionOutcome::Completed(matched) => Ok(matched),
            _ => Err("Unexpected ordinary choice outcome".into()),
        }
    })();
    let events = input
        .events
        .iter()
        .map(|event| match event {
            TraceEvent::Read(position, _) => (0, *position, 0),
            TraceEvent::Rewind(from, to) => (1, *from, *to),
        })
        .collect();
    (
        result,
        allowance - work.unwrap(),
        input.inner.position,
        events,
    )
}

#[test]
fn input_checked_ordinary_choice_shape_keeps_guards_recursive_commit_and_finite_pool() {
    let program = input_checked_ordinary_choice_program(false);
    let decision = &program.decisions["rule/Name/choice"];
    let mut work = 10000;
    assert!(
        program
            .input_checked_ordinary_choice("rule/Name/choice", decision, &mut work)
            .unwrap()
    );
    let used = 10000 - work;
    for allowance in [0, used - 1, used] {
        let mut work = allowance;
        let result = program.input_checked_ordinary_choice("rule/Name/choice", decision, &mut work);
        if allowance == used {
            assert!(result.unwrap());
        } else {
            assert!(result.unwrap_err().contains("budget"));
        }
        assert_eq!(work, 0);
    }
    let mut changed = decision.clone();
    changed.exit = Some(0);
    assert!(
        !program
            .input_checked_ordinary_choice("rule/Name/choice", &changed, &mut 10000)
            .unwrap()
    );
    assert!(
        !program
            .input_checked_ordinary_choice("rule/Name/repeat", decision, &mut 10000)
            .unwrap()
    );
    changed = decision.clone();
    changed.entries.truncate(1);
    assert!(
        !program
            .input_checked_ordinary_choice("rule/Name/choice", &changed, &mut 10000)
            .unwrap()
    );
    changed = decision.clone();
    changed.entries[0] = program.states.len();
    assert!(
        program
            .input_checked_ordinary_choice("rule/Name/choice", &changed, &mut 10000)
            .unwrap_err()
            .contains("Dangling")
    );
    for full in [false, true] {
        let mut guarded = program.clone();
        let entry = decision.entries[0];
        guarded.states[entry].edges = if full {
            vec![Edge::Full {
                target: entry,
                stop: entry,
            }]
        } else {
            vec![Edge::First {
                path: "test/visible".into(),
                target: entry,
            }]
        };
        assert!(
            !guarded
                .input_checked_ordinary_choice("rule/Name/choice", decision, &mut 10000)
                .unwrap()
        );
    }
    let mut recursive = program.clone();
    recursive
        .recursive_decisions
        .insert("rule/Name/choice".into());
    let (start, stop) = recursive.rules["Entry"];
    assert!(
        recursive
            .verify_completion_scope_with_verification(
                start,
                stop,
                &mut 10000,
                CompletionVerification::BoundedGuardOverapproximation
            )
            .err()
            .unwrap()
            .contains("Unqualified completion commitment")
    );
    // Generic input-checked eligibility preserves the actual recursive flag;
    // it does not supply a source-bound runtime companion or an entry guard.
    let scope = recursive
        .verify_completion_scope_with_verification(
            start,
            stop,
            &mut 10000,
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap();
    assert!(scope.guards.is_empty());
    assert!(
        scope
            .decisions
            .values()
            .any(|bound| bound.path == "rule/Name/choice" && bound.recursive_first)
    );
    assert!(
        !recursive
            .is_input_checked_live_choice("rule/Name/choice", &mut Some(10000))
            .unwrap()
    );
}

#[test]
fn input_checked_ordinary_choice_actual_caller_returns_invalid_suffix_and_exact_short() {
    let program = input_checked_ordinary_choice_program(false);
    let before = serde_json::to_vec(&program).unwrap();
    for (text, expected) in [
        ("< x q >", true),
        ("< x r >", true),
        ("< x q ! >", false),
        ("< x q", false),
        ("< x s >", false),
    ] {
        let old = input_checked_ordinary_choice_probe(
            &program,
            text,
            100000,
            CompletionVerification::BoundedGuardOverapproximation,
        );
        let new = input_checked_ordinary_choice_probe(
            &program,
            text,
            100000,
            CompletionVerification::InputCheckedCompletion,
        );
        assert_eq!(old.0, Ok(expected));
        assert_eq!(new.0, old.0);
        assert_eq!(new.2, old.2);
        assert_eq!(new.3, old.3);
        println!(
            "INPUT_CHECKED_ORDINARY_CHOICE_PARITY text={text:?} expected={expected} old_work={} new_work={} position={} events={:?}",
            old.1, new.1, new.2, new.3
        );
        for allowance in [0, new.1 - 1, new.1] {
            let actual = input_checked_ordinary_choice_probe(
                &program,
                text,
                allowance,
                CompletionVerification::InputCheckedCompletion,
            );
            if allowance == new.1 {
                assert_eq!(actual.0, new.0);
            } else {
                assert!(actual.0.unwrap_err().contains("budget"));
            }
            assert_eq!(actual.1, allowance);
        }
    }
    // Absence of recursive-FIRST metadata is not a call-graph acyclicity claim.
    // Productive recursive calls retain the same canonical depth/work bounds.
    let recursive = ParserPrediction::compile(&grammar(
        call("Wrap"),
        vec![
            (
                "Wrap",
                sequence(vec![keyword("<"), call("Name"), keyword(">")]),
            ),
            (
                "Name",
                choice(vec![
                    sequence(vec![keyword("x"), call("Name")]),
                    keyword("y"),
                ]),
            ),
        ],
    ))
    .unwrap();
    for text in ["< y >", "< x x y >"] {
        let old = input_checked_ordinary_choice_probe(
            &recursive,
            text,
            100000,
            CompletionVerification::BoundedGuardOverapproximation,
        );
        let new = input_checked_ordinary_choice_probe(
            &recursive,
            text,
            100000,
            CompletionVerification::InputCheckedCompletion,
        );
        assert_eq!(old.0, Ok(true));
        assert_eq!(new.0, old.0);
        assert_eq!(new.2, old.2);
        assert_eq!(new.3, old.3);
    }
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn input_checked_ordinary_choice_dynamic_ambiguity_is_hard_not_native_admission() {
    let program = input_checked_ordinary_choice_program(true);
    let before = serde_json::to_vec(&program).unwrap();
    let old = input_checked_ordinary_choice_probe(
        &program,
        "< x >",
        100000,
        CompletionVerification::BoundedGuardOverapproximation,
    );
    assert!(old.0.unwrap_err().contains("Ambiguous"));
    let new = input_checked_ordinary_choice_probe(
        &program,
        "< x >",
        100000,
        CompletionVerification::InputCheckedCompletion,
    );
    println!(
        "INPUT_CHECKED_ORDINARY_DYNAMIC_AMBIGUITY result={:?} charged={} position={} events={:?} native_admission=false",
        new.0, new.1, new.2, new.3
    );
    assert!(
        new.0
            .unwrap_err()
            .contains("Ambiguous parser prediction requires predicate resolution")
    );
    assert!(new.3.iter().any(|event| event.0 == 0));
    assert!(new.3.iter().any(|event| event.0 == 1));
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

fn input_checked_one_shot_optional_program(
    card: char,
    keyword_body: bool,
    nullable: bool,
) -> ParserPrediction {
    let mut optional = if keyword_body {
        keyword("x")
    } else {
        call("Maybe")
    };
    optional.cardinality = card;
    let mut maybe = sequence(vec![keyword("x"), keyword("y")]);
    if nullable {
        maybe = keyword("x");
        maybe.cardinality = '?';
    }
    ParserPrediction::compile(&grammar(
        call("Wrap"),
        vec![
            ("Wrap", sequence(vec![keyword("<"), optional, keyword(">")])),
            ("Maybe", maybe),
        ],
    ))
    .unwrap()
}
fn input_checked_one_shot_optional_parts(program: &ParserPrediction) -> (&Decision, Vec<usize>) {
    let decision = &program.decisions["rule/Wrap/s1/repeat"];
    let [body, end] = decision.entries.as_slice() else {
        panic!("Invalid canonical optional");
    };
    let points = program
        .states
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            matches!(s.edges.as_slice(),[Edge::Epsilon(a),Edge::Epsilon(b)] if a==body && b==end)
                .then_some(i)
        })
        .collect();
    (decision, points)
}

#[test]
fn input_checked_one_shot_optional_shape_call_keyword_exact_short_and_loops_distinct() {
    for keyword_body in [false, true] {
        let program = input_checked_one_shot_optional_program('?', keyword_body, false);
        let before = serde_json::to_vec(&program).unwrap();
        let (decision, points) = input_checked_one_shot_optional_parts(&program);
        let mut work = 10000;
        assert!(
            program
                .input_checked_one_shot_optional(
                    "rule/Wrap/s1/repeat",
                    decision,
                    &points,
                    &mut work
                )
                .unwrap()
        );
        let used = 10000 - work;
        for allowance in [0, used - 1, used] {
            let mut work = allowance;
            let actual = program.input_checked_one_shot_optional(
                "rule/Wrap/s1/repeat",
                decision,
                &points,
                &mut work,
            );
            if allowance == used {
                assert!(actual.unwrap());
            } else {
                assert!(actual.unwrap_err().contains("budget"));
            }
            assert_eq!(work, 0);
        }
        assert_eq!(serde_json::to_vec(&program).unwrap(), before);
        for card in ['*', '+'] {
            let repeated = input_checked_one_shot_optional_program(card, keyword_body, false);
            let (d, p) = input_checked_one_shot_optional_parts(&repeated);
            assert!(
                !repeated
                    .input_checked_one_shot_optional("rule/Wrap/s1/repeat", d, &p, &mut 10000)
                    .unwrap()
            );
        }
    }
}

#[test]
fn input_checked_one_shot_optional_actual_exit_falsebody_caller_returns_and_nullable() {
    let program = input_checked_one_shot_optional_program('?', false, false);
    for (text, expected) in [
        ("< >", true),
        ("< x y >", true),
        ("< x >", false),
        ("< x y ! >", false),
        ("< x y", false),
    ] {
        let old = input_checked_ordinary_choice_probe(
            &program,
            text,
            100000,
            CompletionVerification::BoundedGuardOverapproximation,
        );
        let new = input_checked_ordinary_choice_probe(
            &program,
            text,
            100000,
            CompletionVerification::InputCheckedCompletion,
        );
        println!(
            "INPUT_CHECKED_ONE_SHOT_OPTIONAL text={text:?} old={:?} new={:?} old_work={} new_work={} position={} events={:?}",
            old.0, new.0, old.1, new.1, new.2, new.3
        );
        assert_eq!(old.0, Ok(expected));
        assert_eq!(new.0, old.0);
        assert_eq!(new.2, old.2);
        assert_eq!(new.3, old.3);
        for allowance in [0, new.1 - 1, new.1] {
            let actual = input_checked_ordinary_choice_probe(
                &program,
                text,
                allowance,
                CompletionVerification::InputCheckedCompletion,
            );
            if allowance == new.1 {
                assert_eq!(actual.0, new.0);
            } else {
                assert!(actual.0.unwrap_err().contains("budget"));
            }
            assert_eq!(actual.1, allowance);
        }
    }
    // Nullable does not make a finite optional structurally ineligible. The
    // SAME canonical per-input ambiguity/exit rules remain authoritative.
    let nullable = input_checked_one_shot_optional_program('?', false, true);
    let (decision, points) = input_checked_one_shot_optional_parts(&nullable);
    assert!(
        nullable
            .input_checked_one_shot_optional("rule/Wrap/s1/repeat", decision, &points, &mut 10000)
            .unwrap()
    );
    let entered = input_checked_ordinary_choice_probe(
        &nullable,
        "< x >",
        100000,
        CompletionVerification::InputCheckedCompletion,
    );
    assert_eq!(entered.0, Ok(true));
    let empty = input_checked_ordinary_choice_probe(
        &nullable,
        "< >",
        100000,
        CompletionVerification::InputCheckedCompletion,
    );
    println!(
        "INPUT_CHECKED_NULLABLE_OPTIONAL actual={:?} events={:?} native_admission=false",
        empty.0, empty.3
    );
    assert!(
        empty
            .0
            .unwrap_err()
            .contains("Ambiguous parser prediction requires predicate resolution")
    );
}

#[test]
fn input_checked_one_shot_optional_keeps_guard_boundary_and_malformed_failures() {
    let program = input_checked_one_shot_optional_program('?', false, false);
    let (decision, points) = input_checked_one_shot_optional_parts(&program);
    let body = decision.entries[0];
    let end = decision.entries[1];
    let [Edge::Call { follow, .. }] = program.states[body].edges.as_slice() else {
        panic!("Missing actual Call");
    };
    let done = *follow;
    for mode in 0..8 {
        let mut changed = program.clone();
        match mode {
            0 => changed.states[done].edges = vec![Edge::Epsilon(points[0])],
            1 => changed.states[done].edges.push(Edge::Epsilon(end)),
            2 => changed.states[done].accepting = true,
            3 => changed.states[done].rule_stop = true,
            4 => {
                changed.states[body].edges = vec![Edge::First {
                    path: "visible".into(),
                    target: done,
                }]
            }
            5 => {
                changed.states[body].edges = vec![Edge::Full {
                    target: done,
                    stop: end,
                }]
            }
            6 => {
                if let [Edge::Call { follow, .. }] = changed.states[body].edges.as_mut_slice() {
                    *follow = end;
                }
            }
            _ => {
                changed.states[body].edges =
                    vec![Edge::Token(PredictionSymbol::Keyword(String::new()), done)]
            }
        }
        assert!(
            !changed
                .input_checked_one_shot_optional(
                    "rule/Wrap/s1/repeat",
                    decision,
                    &points,
                    &mut 10000
                )
                .unwrap(),
            "mode={mode}"
        );
    }
    for site in [0, 1] {
        let mut changed = program.clone();
        let dangling = changed.states.len();
        if let [Edge::Call { target, follow }] = changed.states[body].edges.as_mut_slice() {
            if site == 0 {
                *target = dangling;
            } else {
                *follow = dangling;
            }
        }
        assert!(
            changed
                .input_checked_one_shot_optional(
                    "rule/Wrap/s1/repeat",
                    decision,
                    &points,
                    &mut 10000
                )
                .unwrap_err()
                .contains("Dangling")
        );
    }
    let mut wrong = decision.clone();
    wrong.exit = Some(0);
    assert!(
        !program
            .input_checked_one_shot_optional("rule/Wrap/s1/repeat", &wrong, &points, &mut 10000)
            .unwrap()
    );
    assert!(
        !program
            .input_checked_one_shot_optional("rule/Wrap/s1/repeat", decision, &[end], &mut 10000)
            .unwrap()
    );
    assert!(
        !program
            .input_checked_one_shot_optional("rule/Wrap/s1/choice", decision, &points, &mut 10000)
            .unwrap()
    );
}

// Low-level canonical-builder controls. The recursive metadata is deliberately
// conservative, as in the original SendNode static proof. This fixture is not a
// native package/admission authority and uses no language-name allowlist.
fn asymmetric_guarded_choice_program(nullable_prefix: bool) -> ParserPrediction {
    let prefix = if nullable_prefix {
        expr(Kind::Action("Object".into()))
    } else {
        keyword("a")
    };
    let g = grammar(
        sequence(vec![keyword("<"), call("Name"), keyword(">")]),
        vec![
            (
                "Name",
                choice(vec![
                    call("Body"),
                    sequence(vec![full(call("Prefix")), call("Body")]),
                ]),
            ),
            ("Body", sequence(vec![keyword("a"), keyword("b")])),
            ("Prefix", prefix),
        ],
    );
    let mut program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        states: vec![],
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        allow_empty_choices: true,
        runtime_requirement: None,
    };
    for name in g.rules.keys() {
        let start = program.state().unwrap();
        let stop = program.state().unwrap();
        program.states[stop].rule_stop = true;
        program.rules.insert(name.clone(), (start, stop));
    }
    for (name, rule) in &g.rules {
        let (start, stop) = program.rules[name];
        program
            .expression(
                &rule.body,
                start,
                stop,
                &format!("rule/{name}"),
                (0, None, None),
                None,
            )
            .unwrap();
    }
    program
        .recursive_decisions
        .insert("rule/Name/choice".into());
    program
}

#[test]
fn input_checked_asymmetric_guarded_choice_consumes_actual_input_and_exact_caller_tail() {
    let program = asymmetric_guarded_choice_program(false);
    let before = serde_json::to_vec(&program).unwrap();
    let (start, stop) = program.rules["Entry"];
    for policy in [
        CompletionVerification::Legacy,
        CompletionVerification::BoundedGuardOverapproximation,
    ] {
        assert!(
            program
                .verify_completion_scope_with_verification(start, stop, &mut 100000, policy)
                .err()
                .unwrap()
                .contains("Unqualified completion commitment")
        );
    }
    for (text, expected) in [
        ("< a b >", true),
        ("< a a b >", true),
        ("< a c >", false),
        ("< a b ! >", false),
        ("< a a b", false),
    ] {
        let actual = input_checked_ordinary_choice_probe(
            &program,
            text,
            100000,
            CompletionVerification::InputCheckedCompletion,
        );
        println!(
            "ASYMMETRIC_GUARDED_CHOICE text={text:?} result={:?} work={} position={} events={:?}",
            actual.0, actual.1, actual.2, actual.3
        );
        assert_eq!(actual.0, Ok(expected));
        assert!(actual.3.iter().any(|event| event.0 == 1));
        // A real second distinguishing token is consumed; no final-guard
        // presence grants commitment at the shared first token.
        assert!(actual.3.iter().any(|event| event.0 == 0 && event.1 >= 2));
        for grant in [0, actual.1 - 1, actual.1] {
            let bound = input_checked_ordinary_choice_probe(
                &program,
                text,
                grant,
                CompletionVerification::InputCheckedCompletion,
            );
            if grant == actual.1 {
                assert_eq!(bound.0, actual.0);
            } else {
                assert!(bound.0.unwrap_err().contains("budget"));
            }
            assert_eq!(bound.1, grant);
        }
    }
    // The public/no-boundary Decision route retains its prior refusal and
    // callback stream. No outer caller or stop is fabricated for this API.
    let mut unbound = Vec::new();
    for verification in [
        CompletionVerification::Legacy,
        CompletionVerification::InputCheckedCompletion,
    ] {
        let tokens = symbols("a a b >");
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(100000);
        let result = program.predict_with_replay_input_and_completion_verification(
            "rule/Name/choice",
            &mut input,
            &mut work,
            verification,
        );
        assert!(
            result
                .as_ref()
                .unwrap_err()
                .contains("visible predicate resolution")
        );
        unbound.push((
            result,
            work,
            input.inner.position,
            format!("{:?}", input.events),
        ));
    }
    assert_eq!(unbound[0], unbound[1]);
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn input_checked_asymmetric_guarded_choice_keeps_ambiguity_hidden_and_forged_boundaries_hard() {
    let program = asymmetric_guarded_choice_program(false);
    let decision = &program.decisions["rule/Name/choice"];
    let plain = decision.entries[0];
    let guarded = decision.entries[1];
    let full = program.states[guarded].edges[0].clone();
    let Edge::Full {
        target: _,
        stop: suffix,
    } = full
    else {
        panic!("Full entry missing");
    };
    let (start, stop) = program.rules["Entry"];
    let (_, reached) = program
        .verify_completion_scope_with_reach(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
            None,
        )
        .unwrap();
    assert!(
        program
            .input_checked_asymmetric_guarded_choice(
                "rule/Name/choice",
                &program.decisions["rule/Name/choice"],
                &reached,
                &mut 100000,
            )
            .unwrap()
    );
    let mut negatives = Vec::new();
    let mut wrong = program.clone();
    wrong
        .decisions
        .get_mut("rule/Name/choice")
        .unwrap()
        .entries
        .reverse();
    negatives.push(wrong);
    let mut hidden = program.clone();
    let moved = hidden.state().unwrap();
    hidden.states[moved].edges = hidden.states[guarded].edges.clone();
    hidden.states[guarded].edges = vec![Edge::Action(moved)];
    negatives.push(hidden);
    let mut unguarded = program.clone();
    unguarded.states[guarded].edges = unguarded.states[plain].edges.clone();
    negatives.push(unguarded);
    let mut foreign_stop = program.clone();
    if let Edge::Full { stop, .. } = &mut foreign_stop.states[guarded].edges[0] {
        *stop = program.rules["Body"].1;
    }
    negatives.push(foreign_stop);
    let mut dangling = program.clone();
    if let Edge::Full { stop, .. } = &mut dangling.states[guarded].edges[0] {
        *stop = program.states.len();
    }
    negatives.push(dangling);
    let mut wrong_join = program.clone();
    if let Edge::Call { follow, .. } = &mut wrong_join.states[suffix].edges[0] {
        *follow = plain;
    }
    negatives.push(wrong_join);
    let mut wrong_stop = program.clone();
    wrong_stop.states[program.rules["Body"].1].rule_stop = false;
    negatives.push(wrong_stop);
    for (mode, changed) in negatives.into_iter().enumerate() {
        let before = serde_json::to_vec(&changed).unwrap();
        let (start, stop) = changed.rules["Entry"];
        let scope = changed.verify_completion_scope_with_reach(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
            None,
        );
        if mode == 4 {
            assert!(
                matches!(&scope, Err(message) if message == "Dangling full-predicate operand boundary")
            );
        }
        // General conditional choices may have another valid graph shape.
        // They must never acquire this independent exact asymmetric-pair proof.
        assert!(!matches!(&scope, Err(message) if message.contains("budget")));
        if let Ok((_, reached)) = scope {
            let witness = changed.input_checked_asymmetric_guarded_choice(
                "rule/Name/choice",
                &changed.decisions["rule/Name/choice"],
                &reached,
                &mut 100000,
            );
            assert!(!matches!(&witness, Err(message) if message.contains("budget")));
            assert!(!matches!(witness, Ok(true)), "mutation {mode}");
        }
        assert_eq!(serde_json::to_vec(&changed).unwrap(), before);
    }
    // A nullable prefix makes these two real languages equal. Retained Full
    // replay succeeds, but the SAME Decision worker must still reject ambiguity.
    let ambiguous = asymmetric_guarded_choice_program(true);
    let result = input_checked_ordinary_choice_probe(
        &ambiguous,
        "< a b >",
        100000,
        CompletionVerification::InputCheckedCompletion,
    );
    assert!(result.0.unwrap_err().contains("Ambiguous"));
}

#[test]
fn input_checked_asymmetric_guarded_choice_propagates_cancel_and_restores_speculation() {
    struct Cancel<'a> {
        inner: TraceReplay<'a>,
        reads: usize,
    }
    impl PredictionInput for Cancel<'_> {
        fn replay_position(&self) -> Option<usize> {
            self.inner.replay_position()
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.inner.checkpoint()
        }
        fn rewind(&mut self, p: usize) -> Result<(), String> {
            self.inner.rewind(p)
        }
        fn read(&mut self, work: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            self.reads += 1;
            if self.reads == 3 {
                return Err("asymmetric input cancelled".into());
            }
            self.inner.read(work)
        }
    }
    let program = asymmetric_guarded_choice_program(false);
    let (start, stop) = program.rules["Entry"];
    let mut work = Some(100000);
    let scope = program
        .verify_completion_scope_with_verification(
            start,
            stop,
            work.as_mut().unwrap(),
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap();
    let tokens = symbols("< a a b >");
    let mut input = Cancel {
        inner: TraceReplay::new(&tokens),
        reads: 0,
    };
    let result = program.run_prediction_at_depth_with_completion_verification(
        PredictionGoal::Completion {
            start,
            stop,
            scope: &scope,
            productivity: CompletionProductivity::Any,
        },
        &mut input,
        &mut work,
        0,
        CompletionVerification::InputCheckedCompletion,
    );
    assert!(result.unwrap_err().contains("asymmetric input cancelled"));
    assert!(work.unwrap() < 100000);
    assert!(
        input
            .inner
            .events
            .iter()
            .any(|event| matches!(event, TraceEvent::Rewind(..)))
    );
}

#[test]
fn input_checked_asymmetric_guarded_choice_real_source_emission_and_native_seal() {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    use crate::grammar::backend_test_support::inspect_ecore_schema;
    use crate::grammar;
    use crate::grammar::GrammarAnalysisWorkBudget;
    use crate::grammar::RuntimeGuardedPolicy;
    let schema = inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
    let source = DefinitionSource::new(
        "memory:asymmetric-guarded-native.xtext",
        SourceKind::Xtext,
        "grammar asym hidden() import \"urn:t\" as t Root returns t::Root: {t::Root} '{' (=> Item)* '}'; fragment Item returns t::Root: 'send' (Body | (=> Prefix Body)); fragment Body returns t::Root: 'a' 'b'; fragment Prefix returns t::Root: 'a';",
    );
    let (mut g, _, spans, _) = grammar::parse_declarations(
        &source.text,
        &schema,
        grammar::Profile::NativeV80,
        false,
        None,
        128,
        std::rc::Rc::default(),
    )
    .unwrap();
    g.declaration_sources = spans
        .into_iter()
        .map(|(name, range)| {
            (
                name,
                SourceSpan {
                    source: source.id.clone(),
                    start: range.start,
                    end: range.end,
                },
            )
        })
        .collect();
    grammar::terminals::prepare(&mut g.terminals, g.profile)
        .map_err(|e| e.message)
        .unwrap();
    let mut predicates = BTreeSet::new();
    for (name, rule) in &mut g.rules {
        grammar::bind_prediction_paths(
            &mut rule.body,
            &format!("rule/{name}"),
            &mut predicates,
            true,
        );
    }
    let original = serde_json::to_vec(&g).unwrap();
    let mut admission = RuntimeGuardedAdmission::new_with_policy_and_budget(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        GrammarAnalysisWorkBudget::default(),
    );
    let program = ParserPrediction::compile_predicates_impl_prediction_policy::<true>(
        &g,
        &predicates,
        true,
        None,
        None,
        RecursiveFrontierOrder::Historical,
        None,
        Some(&mut admission),
    )
    .unwrap();
    assert!(program.runtime_requirement.is_some());
    assert!(admission.decision_count() > 0);
    assert!(program.input_checked_completion_obligation_count() > 0);
    assert_eq!(serde_json::to_vec(&g).unwrap(), original);
    let (start, stop) = program.rules["Body"];
    let scope = program
        .verify_completion_scope_with_verification(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap();
    assert!(scope.decisions.is_empty());
    // Native sealing is a real compiler prerequisite. Publishing a runtime
    // owner/package and running an original grammar remain distinct gates.
}

// Same canonical prefix/suffix witness with the OTHER existing guard kind.
// The path is the real second-alternative first operand, not a class/rule key.
fn first_guarded_prefix_choice_program() -> ParserPrediction {
    let mut program = asymmetric_guarded_choice_program(false);
    let entry = program.decisions["rule/Name/choice"].entries[1];
    let Edge::Full { target, .. } = program.states[entry].edges[0] else {
        panic!("Missing Full fixture entry");
    };
    let path = "rule/Name/a1/s0".to_string();
    program
        .first_guards
        .insert(path.clone(), [PredictionSymbol::Keyword("a".into())].into());
    program.states[entry].edges = vec![Edge::First { path, target }];
    program
}

#[test]
fn input_checked_guarded_prefix_choice_first_actual_tail_ambiguity_and_exact_shared_work() {
    let program = first_guarded_prefix_choice_program();
    let before = serde_json::to_vec(&program).unwrap();
    let (start, stop) = program.rules["Entry"];
    for verification in [
        CompletionVerification::Legacy,
        CompletionVerification::BoundedGuardOverapproximation,
    ] {
        assert!(
            program
                .verify_completion_scope_with_verification(start, stop, &mut 100000, verification)
                .err()
                .unwrap()
                .contains("Unqualified completion commitment")
        );
    }
    for (text, expected) in [
        ("< a b >", true),
        ("< a a b >", true),
        ("< a c >", false),
        ("< x a b >", false),
        ("< a b ! >", false),
        ("< a a b", false),
    ] {
        let actual = input_checked_ordinary_choice_probe(
            &program,
            text,
            100000,
            CompletionVerification::InputCheckedCompletion,
        );
        println!(
            "INPUT_CHECKED_FIRST_PREFIX text={text:?} result={:?} work={} position={} events={:?}",
            actual.0, actual.1, actual.2, actual.3
        );
        assert_eq!(actual.0, Ok(expected));
        for grant in [0, actual.1 - 1, actual.1] {
            let bounded = input_checked_ordinary_choice_probe(
                &program,
                text,
                grant,
                CompletionVerification::InputCheckedCompletion,
            );
            if grant == actual.1 {
                assert_eq!(bounded.0, actual.0);
            } else {
                assert!(bounded.0.unwrap_err().contains("budget"));
            }
            assert_eq!(bounded.1, grant);
        }
    }
    // First is only a first-token guard. Build a genuine productive recursive
    // Body = 'a' Body? with the SAME canonical expression emitter; Prefix='a'
    // remains nonnullable and its stored FIRST={'a'} remains accurate. On 'aa'
    // both complete alternatives match, so no source-prefix preference is made.
    let mut ambiguous = program.clone();
    let (body_start, body_stop) = ambiguous.rules["Body"];
    ambiguous.states[body_start].edges.clear();
    let mut again = call("Body");
    again.cardinality = '?';
    let body = sequence(vec![keyword("a"), again]);
    ambiguous
        .expression(
            &body,
            body_start,
            body_stop,
            "rule/Body",
            (0, None, None),
            None,
        )
        .unwrap();
    let outcome = input_checked_ordinary_choice_probe(
        &ambiguous,
        "< a a >",
        100000,
        CompletionVerification::InputCheckedCompletion,
    );
    assert!(outcome.0.unwrap_err().contains("Ambiguous"));
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn input_checked_guarded_prefix_choice_first_foreign_marker_hidden_operand_and_return_reject() {
    let program = first_guarded_prefix_choice_program();
    let entry = program.decisions["rule/Name/choice"].entries[1];
    let Edge::First { ref path, target } = program.states[entry].edges[0] else {
        panic!("Missing First entry");
    };
    let path = path.clone();
    let (start, stop) = program.rules["Entry"];
    let (_, reached) = program
        .verify_completion_scope_with_reach(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
            None,
        )
        .unwrap();
    assert!(
        program
            .input_checked_asymmetric_guarded_choice(
                "rule/Name/choice",
                &program.decisions["rule/Name/choice"],
                &reached,
                &mut 100000,
            )
            .unwrap()
    );
    let mut negatives = Vec::new();
    let mut missing = program.clone();
    missing.first_guards.remove(&path);
    negatives.push(missing);
    let mut empty = program.clone();
    empty.first_guards.get_mut(&path).unwrap().clear();
    negatives.push(empty);
    let mut foreign = program.clone();
    let wrong = "rule/Name/a0/s0".to_string();
    foreign
        .first_guards
        .insert(wrong.clone(), foreign.first_guards[&path].clone());
    if let Edge::First { path, .. } = &mut foreign.states[entry].edges[0] {
        *path = wrong;
    }
    negatives.push(foreign);
    let mut hidden = program.clone();
    let moved = hidden.state().unwrap();
    hidden.states[moved].edges = hidden.states[target].edges.clone();
    hidden.states[target].edges = vec![Edge::Action(moved)];
    negatives.push(hidden);
    let mut foreign_call = program.clone();
    if let Edge::Call { target: callee, .. } = &mut foreign_call.states[target].edges[0] {
        *callee = program.rules["Entry"].1;
    }
    negatives.push(foreign_call);
    let mut returned = program.clone();
    if let Edge::Call { follow, .. } = &mut returned.states[target].edges[0] {
        *follow = program.rules["Body"].1;
    }
    negatives.push(returned);
    let mut dangling = program.clone();
    if let Edge::First { target, .. } = &mut dangling.states[entry].edges[0] {
        *target = program.states.len();
    }
    negatives.push(dangling);
    for (mode, changed) in negatives.into_iter().enumerate() {
        let before = serde_json::to_vec(&changed).unwrap();
        let (start, stop) = changed.rules["Entry"];
        let scope = changed.verify_completion_scope_with_reach(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
            None,
        );
        if matches!(mode, 0 | 1) {
            assert!(
                matches!(&scope, Err(message) if message == &format!("Unbound completion first-token guard at {path}"))
            );
        } else if mode == 6 {
            assert!(matches!(&scope, Err(message) if message == "Dangling completion state"));
        }
        assert!(!matches!(&scope, Err(message) if message.contains("budget")));
        if let Ok((_, reached)) = scope {
            let witness = changed.input_checked_asymmetric_guarded_choice(
                "rule/Name/choice",
                &changed.decisions["rule/Name/choice"],
                &reached,
                &mut 100000,
            );
            assert!(!matches!(&witness, Err(message) if message.contains("budget")));
            assert!(!matches!(witness, Ok(true)), "mutation {mode}");
        }
        assert_eq!(serde_json::to_vec(&changed).unwrap(), before);
    }
}

#[test]
fn input_checked_guarded_prefix_choice_first_real_source_emission_and_native_seal() {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    use crate::grammar::backend_test_support::inspect_ecore_schema;
    use crate::grammar;
    use crate::grammar::GrammarAnalysisWorkBudget;
    use crate::grammar::RuntimeGuardedPolicy;
    let schema = inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
    let source = DefinitionSource::new(
        "memory:first-guarded-prefix-native.xtext",
        SourceKind::Xtext,
        "grammar firstPrefix hidden() import \"urn:t\" as t Root returns t::Root: {t::Root} '{' (=> Item)* '}'; fragment Item returns t::Root: 'terminate' (Body | (-> Prefix Body)); fragment Body returns t::Root: 'a' 'b'; fragment Prefix returns t::Root: 'a';",
    );
    let (mut g, _, spans, _) = grammar::parse_declarations(
        &source.text,
        &schema,
        grammar::Profile::NativeV80,
        false,
        None,
        128,
        std::rc::Rc::default(),
    )
    .unwrap();
    g.declaration_sources = spans
        .into_iter()
        .map(|(name, range)| {
            (
                name,
                SourceSpan {
                    source: source.id.clone(),
                    start: range.start,
                    end: range.end,
                },
            )
        })
        .collect();
    grammar::terminals::prepare(&mut g.terminals, g.profile)
        .map_err(|e| e.message)
        .unwrap();
    let mut predicates = BTreeSet::new();
    for (name, rule) in &mut g.rules {
        grammar::bind_prediction_paths(
            &mut rule.body,
            &format!("rule/{name}"),
            &mut predicates,
            true,
        );
    }
    assert_eq!(predicates.len(), 1);
    let original = serde_json::to_vec(&g).unwrap();
    let mut admission = RuntimeGuardedAdmission::new_with_policy_and_budget(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        GrammarAnalysisWorkBudget::default(),
    );
    let program = ParserPrediction::compile_predicates_impl_prediction_policy::<true>(
        &g,
        &predicates,
        true,
        None,
        None,
        RecursiveFrontierOrder::Historical,
        None,
        Some(&mut admission),
    )
    .unwrap();
    assert!(program.runtime_requirement.is_some());
    assert!(admission.decision_count() > 0);
    assert_eq!(
        program.input_checked_completion_obligation_count(),
        admission.decision_count()
    );
    assert_eq!(serde_json::to_vec(&g).unwrap(), original);
    // This is the same parsed-source -> emitter -> collector -> final static
    // flags -> native seal chain, not a serialized graph or an original award.
}

// The last alternative is guarded, so the existing priority shape is false.
// It nevertheless has a successful canonical static disjointness proof.
fn qualified_guard_visibility_program(overlap: bool, hidden: bool) -> ParserPrediction {
    let guarded = full(call("Prefix"));
    let guarded = if hidden {
        sequence(vec![expr(Kind::Action("Object".into())), guarded])
    } else {
        guarded
    };
    let g = grammar(
        sequence(vec![keyword("<"), call("Name"), keyword(">")]),
        vec![
            ("Name", choice(vec![keyword("a"), keyword("b"), guarded])),
            (
                "Prefix",
                if overlap {
                    keyword("a")
                } else {
                    sequence(vec![keyword("c"), keyword("d")])
                },
            ),
        ],
    );
    let mut program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        states: vec![],
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        allow_empty_choices: true,
        runtime_requirement: None,
    };
    for name in g.rules.keys() {
        let start = program.state().unwrap();
        let stop = program.state().unwrap();
        program.states[stop].rule_stop = true;
        program.rules.insert(name.clone(), (start, stop));
    }
    for (name, rule) in &g.rules {
        let (start, stop) = program.rules[name];
        program
            .expression(
                &rule.body,
                start,
                stop,
                &format!("rule/{name}"),
                (0, None, None),
                None,
            )
            .unwrap();
    }
    // Match the canonical compiler's real root FOLLOW. Without this suffix,
    // overlap discovery stops at an unknown caller instead of an accepting
    // conflict; that is not an ambiguity fixture or a resource failure.
    let eof = program.state().unwrap();
    let accepted = program.state().unwrap();
    program.states[accepted].accepting = true;
    let root_stop = program.rules["Entry"].1;
    program.states[root_stop].follow.push(eof);
    program.states[eof]
        .edges
        .push(Edge::Token(PredictionSymbol::End, accepted));
    program
}

#[test]
fn input_checked_qualified_guard_visibility_static_proof_actual_input_tail_and_exact_work() {
    let program = qualified_guard_visibility_program(false, false);
    let before = serde_json::to_vec(&program).unwrap();
    program
        .verify_completion_decision_with_verification(
            "rule/Name/choice",
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap();
    let (start, stop) = program.rules["Entry"];
    for verification in [
        CompletionVerification::Legacy,
        CompletionVerification::BoundedGuardOverapproximation,
    ] {
        assert!(
            program
                .verify_completion_scope_with_verification(start, stop, &mut 100000, verification)
                .is_err()
        );
    }
    let scope = program
        .verify_completion_scope_with_verification(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap();
    let guard = program.decisions["rule/Name/choice"].entries[2];
    assert_eq!(scope.guards.get(&guard), Some(&CompletionGuard::Full));
    assert_eq!(
        scope
            .decisions
            .values()
            .filter(|bound| bound.path == "rule/Name/choice")
            .count(),
        1
    );
    for (text, expected) in [
        ("< a >", true),
        ("< b >", true),
        ("< c d >", true),
        ("< c e >", false),
        ("< c d ! >", false),
        ("< c d", false),
        ("< a d >", false),
    ] {
        let actual = input_checked_ordinary_choice_probe(
            &program,
            text,
            100000,
            CompletionVerification::InputCheckedCompletion,
        );
        println!("INPUT_CHECKED_QUALIFIED_GUARD text={text:?} whole={actual:?}");
        assert_eq!(actual.0, Ok(expected));
        if text == "< c d >" {
            // The same nested Decision rewinds to its own start after '<', not
            // the root start, before running the existing canonical Full probe.
            assert!(actual.3.iter().any(|&(kind, _, to)| kind == 1 && to == 1));
        }
        for grant in [0, actual.1 - 1, actual.1] {
            let bounded = input_checked_ordinary_choice_probe(
                &program,
                text,
                grant,
                CompletionVerification::InputCheckedCompletion,
            );
            if grant == actual.1 {
                assert_eq!(bounded, actual);
            } else {
                assert!(bounded.0.unwrap_err().contains("budget"));
            }
            assert_eq!(bounded.1, grant);
        }
    }
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn input_checked_qualified_guard_visibility_rejects_overlap_hidden_and_forged_boundaries() {
    let ambiguous = qualified_guard_visibility_program(true, false);
    let (start, stop) = ambiguous.rules["Entry"];
    let mut overlap_remaining = 100000;
    // A generic conditional scope need not prove disjointness. The existing
    // static worker still reports overlap, and the actual-input decision must
    // report ambiguity at the complete live tail.
    let error = ambiguous
        .verify_completion_decision_with_verification(
            "rule/Name/choice",
            &mut overlap_remaining,
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap_err();
    ambiguous
        .verify_completion_scope_with_verification(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap();
    let actual = input_checked_ordinary_choice_probe(
        &ambiguous,
        "< a >",
        100000,
        CompletionVerification::InputCheckedCompletion,
    );
    assert!(actual.0.unwrap_err().contains("Ambiguous"));
    println!("QUALIFIED_GUARD_OVERLAP error={error:?} remaining={overlap_remaining}");
    assert!(
        overlap_remaining > 90000,
        "ambiguity fixture unexpectedly exhausted work: {error}"
    );
    assert!(
        !error.contains("budget"),
        "resource failure is not an ambiguity proof: {error}"
    );
    assert!(error.contains("Ambiguous"), "wrong static failure: {error}");
    let hidden = qualified_guard_visibility_program(false, true);
    let (start, stop) = hidden.rules["Entry"];
    assert!(
        hidden
            .verify_completion_scope_with_verification(
                start,
                stop,
                &mut 100000,
                CompletionVerification::InputCheckedCompletion
            )
            .err()
            .unwrap()
            .contains("guard visibility")
    );
    let program = qualified_guard_visibility_program(false, false);
    let entry = program.decisions["rule/Name/choice"].entries[2];
    for change_target in [false, true] {
        let mut forged = program.clone();
        let dangling = forged.states.len();
        match &mut forged.states[entry].edges[0] {
            Edge::Full { target, stop } => {
                if change_target {
                    *target = dangling;
                } else {
                    *stop = dangling;
                }
            }
            _ => panic!("Missing Full fixture entry"),
        }
        let before = serde_json::to_vec(&forged).unwrap();
        let (start, stop) = forged.rules["Entry"];
        assert!(
            forged
                .verify_completion_scope_with_verification(
                    start,
                    stop,
                    &mut 100000,
                    CompletionVerification::InputCheckedCompletion
                )
                .err()
                .unwrap()
                .contains("Dangling")
        );
        assert_eq!(serde_json::to_vec(&forged).unwrap(), before);
    }
    let mut recursive = program;
    recursive
        .recursive_decisions
        .insert("rule/Name/choice".into());
    let (start, stop) = recursive.rules["Entry"];
    assert!(
        recursive
            .verify_completion_scope_with_verification(
                start,
                stop,
                &mut 100000,
                CompletionVerification::BoundedGuardOverapproximation
            )
            .err()
            .unwrap()
            .contains("Unqualified completion commitment")
    );
}

#[test]
fn input_checked_qualified_guard_visibility_cancel_and_finite_pool_preserve_replay() {
    struct Cancel<'a> {
        inner: TraceReplay<'a>,
        reads: usize,
    }
    impl PredictionInput for Cancel<'_> {
        fn replay_position(&self) -> Option<usize> {
            self.inner.replay_position()
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.inner.checkpoint()
        }
        fn rewind(&mut self, p: usize) -> Result<(), String> {
            self.inner.rewind(p)
        }
        fn read(&mut self, work: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            self.reads += 1;
            if self.reads == 3 {
                return Err("qualified Full input cancelled".into());
            }
            self.inner.read(work)
        }
    }
    let program = qualified_guard_visibility_program(false, false);
    let (start, stop) = program.rules["Entry"];
    let scope = program
        .verify_completion_scope_with_verification(
            start,
            stop,
            &mut 100000,
            CompletionVerification::InputCheckedCompletion,
        )
        .unwrap();
    let tokens = symbols("< c d >");
    let mut input = Cancel {
        inner: TraceReplay::new(&tokens),
        reads: 0,
    };
    let mut remaining = Some(100000);
    let result = program.run_prediction_at_depth_with_completion_verification(
        PredictionGoal::Completion {
            start,
            stop,
            scope: &scope,
            productivity: CompletionProductivity::Any,
        },
        &mut input,
        &mut remaining,
        0,
        CompletionVerification::InputCheckedCompletion,
    );
    assert!(
        result
            .unwrap_err()
            .contains("qualified Full input cancelled")
    );
    assert!(remaining.unwrap() < 100000);
    assert!(
        input
            .inner
            .events
            .iter()
            .any(|event| matches!(event, TraceEvent::Rewind(2, 1)))
    );
    let mut input = TraceReplay::new(&tokens);
    assert!(
        program
            .run_prediction_at_depth_with_completion_verification(
                PredictionGoal::Completion {
                    start,
                    stop,
                    scope: &scope,
                    productivity: CompletionProductivity::Any
                },
                &mut input,
                &mut None,
                0,
                CompletionVerification::InputCheckedCompletion,
            )
            .unwrap_err()
            .contains("finite")
    );
}

#[test]
fn input_checked_qualified_guard_visibility_real_parsed_native_pipeline() {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    use crate::grammar::backend_test_support::inspect_ecore_schema;
    use crate::grammar;
    use crate::grammar::GrammarAnalysisWorkBudget;
    use crate::grammar::RuntimeGuardedPolicy;
    let schema = inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
    let source = DefinitionSource::new(
        "memory:qualified-guard-visibility-native.xtext",
        SourceKind::Xtext,
        "grammar qualifiedGuard hidden() import \"urn:t\" as t Root returns t::Root: {t::Root} '{' (=> Item)* '}'; fragment Item returns t::Root: 'a' | 'b' | (=> Prefix); fragment Prefix returns t::Root: 'c' 'd';",
    );
    let (mut g, _, spans, _) = grammar::parse_declarations(
        &source.text,
        &schema,
        grammar::Profile::NativeV80,
        false,
        None,
        128,
        std::rc::Rc::default(),
    )
    .unwrap();
    g.declaration_sources = spans
        .into_iter()
        .map(|(name, range)| {
            (
                name,
                SourceSpan {
                    source: source.id.clone(),
                    start: range.start,
                    end: range.end,
                },
            )
        })
        .collect();
    grammar::terminals::prepare(&mut g.terminals, g.profile)
        .map_err(|e| e.message)
        .unwrap();
    let mut predicates = BTreeSet::new();
    for (name, rule) in &mut g.rules {
        grammar::bind_prediction_paths(
            &mut rule.body,
            &format!("rule/{name}"),
            &mut predicates,
            true,
        );
    }
    assert!(predicates.is_empty());
    let original = serde_json::to_vec(&g).unwrap();
    let mut admission = RuntimeGuardedAdmission::new_with_policy_and_budget(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        GrammarAnalysisWorkBudget::default(),
    );
    let program = ParserPrediction::compile_predicates_impl_prediction_policy::<true>(
        &g,
        &predicates,
        true,
        None,
        None,
        RecursiveFrontierOrder::Historical,
        None,
        Some(&mut admission),
    )
    .unwrap();
    assert!(program.runtime_requirement.is_some());
    assert!(admission.decision_count() > 0);
    assert_eq!(
        program.input_checked_completion_obligation_count(),
        admission.decision_count()
    );
    assert_eq!(serde_json::to_vec(&g).unwrap(), original);
    // This is the same parsed-source -> emitter -> collector -> final static
    // flags -> native seal chain, not a serialized graph or an original award.
}

// Whole native compilation below mints the real immutable runtime companion.
// The target choice is deliberately OUTSIDE the only admitted Tag repeat.
fn deferred_source_choice_compilation(
    ambiguous: bool,
) -> Result<crate::grammar::CompiledRuntimeGrammar, String> {
    let text = "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} '{' (=> Tag)* '}' Item; fragment Tag returns t::Root: 'x'; fragment Item returns t::Root: 'send' (Body | (=> Prefix Body)); fragment Body returns t::Root: 'a' 'b'; fragment Prefix returns t::Root: 'a';";
    let text = if ambiguous {
        text.replace(
            "fragment Body returns t::Root: 'a' 'b';",
            "fragment Body returns t::Root: 'a' ('b' | 'a' 'b');",
        )
    } else {
        text.into()
    };
    deferred_source_choice_text_compilation(&text)
}

fn deferred_source_choice_text_compilation(
    text: &str,
) -> Result<crate::grammar::CompiledRuntimeGrammar, String> {
    // Historical paired controls retain their SAME default-budget semantics.
    deferred_source_choice_text_compilation_with_budget(
        text,
        crate::grammar::GrammarAnalysisWorkBudget::default(),
    )
}

fn deferred_source_choice_text_compilation_with_budget(
    text: &str,
    work_budget: crate::grammar::GrammarAnalysisWorkBudget,
) -> Result<crate::grammar::CompiledRuntimeGrammar, String> {
    deferred_source_choice_text_compilation_with_model(
        text,
        work_budget,
        r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="deferred" nsURI="urn:deferred-choice"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#,
    )
}

fn deferred_source_choice_text_compilation_with_model(
    text: &str,
    work_budget: crate::grammar::GrammarAnalysisWorkBudget,
    ecore_text: &str,
) -> Result<crate::grammar::CompiledRuntimeGrammar, String> {
    use intersect_model::definitions::bundle::DefinitionBundle;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::model_definition::ModelCompilation;
    use intersect_model::definitions::schema::Diagnostic;
    let ecore = DefinitionSource::new(
        "memory:deferred-choice/model.ecore",
        SourceKind::Ecore,
        ecore_text,
    );
    let mut syntax = DefinitionSource::new(
        "memory:deferred-choice/grammar.xtext",
        SourceKind::Xtext,
        text,
    );
    syntax.dependencies.push(ecore.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:deferred-choice/empty.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package deferredRules version \"1\";",
    );
    behavior.dependencies.push(ecore.id.clone());
    let expected = syntax.id.clone();
    let input = ModelCompilation {
        materialized_types: vec![],
        structure: ecore.id.clone(),
        namespace: "urn:deferred-choice".into(),
        behavior: behavior.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), syntax, behavior],
            namespaces: vec![],
        },
    };
    let migration = intersect_model::migration::migrate_ecore_bundle(
        input.bundle.clone(),
        &[(ecore.id, "memory:deferred-choice/model.rcore".into())].into(),
    )
    .map_err(|e| e.to_string())?;
    let converted = migration.remap_model_compilation(&input)?;
    // This trusted authored-source test adapter admits only the exact supplied
    // identity. The same native compiler still parses all actual source text.
    let validator =
        |bundle: &DefinitionBundle, id: &intersect_model::definitions::bundle::SourceId, _limits| {
            if id != &expected {
                return Err(Diagnostic::new(&id.uri, 0, 1, "Foreign authored source"));
            }
            bundle
                .source(id)
                .map(|_| ())
                .map_err(|e| Diagnostic::new(&id.uri, 0, 1, e))
        };
    crate::grammar::backend_test_support::runtime(&converted, &expected, work_budget, &validator)
}

fn deferred_source_choice<'a>(
    grammar: &'a Grammar,
    work: &mut usize,
) -> Result<(&'a str, &'a Expr), String> {
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No actual native prediction")?;
    deferred_source_choice_in_program(grammar, program, work)
}

fn deferred_source_choice_in_program<'a>(
    grammar: &'a Grammar,
    program: &'a ParserPrediction,
    work: &mut usize,
) -> Result<(&'a str, &'a Expr), String> {
    let mut selected = None;
    for path in program
        .decisions
        .keys()
        .filter(|p| p.starts_with("rule/Item/") && p.ends_with("/choice"))
    {
        let base = path
            .strip_suffix("/choice")
            .ok_or("No actual source choice suffix")?;
        let Some((owner, _, expression)) =
            super::super::nullable_repetition::prefix_source_occurrence_with_work(
                grammar, base, work,
            )?
        else {
            continue;
        };
        if owner == "Item" && matches!(expression.kind, Kind::Alternative(_)) {
            if selected.is_some() {
                return Err("More than one actual Item choice".into());
            }
            selected = Some((path.as_str(), expression));
        }
    }
    selected.ok_or("Missing actual Item choice".into())
}

#[test]
fn input_checked_deferred_choice_native_owner_and_ordinary_live_outside_repeat()
-> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let compilation = deferred_source_choice_compilation(false)?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No native program")?;
    let (path, expression) = deferred_source_choice(grammar, &mut 100000)?;
    // The native compiler now binds this guarded pair as a conditional live
    // choice even though its final classifier remains nonrecursive.
    assert!(admission.decision_is_runtime(path));
    assert!(
        admission
            .decisions()
            .all(|p| p == path || (p.starts_with("rule/Root/") && p.ends_with("/repeat")))
    );
    assert!(
        !program.recursive_decisions.contains(path),
        "The ordinary live case must have actual final nonrecursive metadata"
    );
    // Final native admission additionally requires exact original source and
    // ordered graph endpoints. Generic graph eligibility grants no such owner.
    assert_eq!(
        admission.live_choice_boundary(grammar, program, path, "Item", &mut Some(100_000))?,
        Some(program.rules["Item"].1),
    );
    let mut reordered = program.clone();
    reordered
        .decisions
        .get_mut(path)
        .ok_or("No native choice")?
        .entries
        .reverse();
    let message = admission
        .live_choice_boundary(grammar, &reordered, path, "Item", &mut Some(100_000))
        .expect_err("original ordered source entries must bind");
    assert!(
        message.contains("endpoints") || message.contains("fanout"),
        "{message}"
    );
    let mut forged_guard = program.clone();
    let guarded = forged_guard.decisions[path].entries[1];
    let [Edge::Full { stop, .. }] = forged_guard.states[guarded].edges.as_mut_slice() else {
        return Err("No actual native Full entry".into());
    };
    *stop = program.rules["Body"].1;
    let message = admission
        .live_choice_boundary(grammar, &forged_guard, path, "Item", &mut Some(100_000))
        .expect_err("original Full operand endpoints must bind");
    assert!(
        message.contains("Full operand graph endpoints"),
        "{message}"
    );
    let mut foreign = grammar.clone();
    let source = foreign
        .declaration_sources
        .get_mut("Item")
        .ok_or("No native Item source")?;
    source.end = source.start + 1;
    let message = admission
        .live_choice_boundary(&foreign, program, path, "Item", &mut Some(100_000))
        .expect_err("original source interval must bind");
    assert!(message.contains("interval"), "{message}");
    let original = serde_json::to_vec(grammar).map_err(|e| e.to_string())?;
    assert!(matches!(
        program.input_checked_source_choice(path, grammar, &mut 100000)?,
        nullable_repeat::DeferredChoiceProof::Qualified
    ));
    let Kind::Sequence(root) = &grammar.rules["Root"].body.kind else {
        return Err("No actual root sequence".into());
    };
    let call = root
        .iter()
        .find(|e| matches!(&e.kind,Kind::Call(name)if name=="Item"))
        .ok_or("No actual outside-repeat caller")?;
    for (text, expected) in [
        ("{ } send a b", Some(0)),
        ("{ } send a a b", Some(1)),
        ("{ } send a c", None),
    ] {
        let source = DefinitionSource::new("memory:deferred-choice/input", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = TraceReplay::new(&tokens);
        // The real ordinary parser is already past '{' '}' 'send' here.
        input.inner.position = 3;
        let mut work = Some(100000);
        let mut live = super::LiveCallerPrediction::new_runtime_guarded(
            grammar, &source.id, admission, &mut work,
        )?;
        let result = live.predict(
            &source.id,
            &[call],
            expression,
            "choice",
            &mut input,
            &mut work,
        );
        println!(
            "INPUT_CHECKED_DEFERRED_ORDINARY_LIVE text={text:?} result={result:?} remaining={work:?} events={:?}",
            input.events
        );
        assert_eq!(result?, expected);
    }
    assert_eq!(
        serde_json::to_vec(grammar).map_err(|e| e.to_string())?,
        original
    );
    // A second actual source has overlapping plain/prefix languages. The
    // ordinary, nonrecursive live path must report ambiguity on actual input.
    let overlap = deferred_source_choice_compilation(true)?;
    let grammar = overlap.grammar();
    let admission = overlap.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No overlapping native program")?;
    let (path, expression) = deferred_source_choice(grammar, &mut 100000)?;
    assert!(!program.recursive_decisions.contains(path));
    // The native compiler now binds this guarded pair as a conditional live
    // choice even though its final classifier remains nonrecursive.
    assert!(admission.decision_is_runtime(path));
    assert!(matches!(
        program.input_checked_source_choice(path, grammar, &mut 100000)?,
        nullable_repeat::DeferredChoiceProof::Qualified
    ));
    let Kind::Sequence(root) = &grammar.rules["Root"].body.kind else {
        return Err("No real overlap caller".into());
    };
    let call = root
        .iter()
        .find(|e| matches!(&e.kind,Kind::Call(name) if name=="Item"))
        .ok_or("No overlap Item call")?;
    let source = DefinitionSource::new(
        "memory:deferred-choice/overlap",
        SourceKind::Xtext,
        "{ } send a a b",
    );
    let tokens = symbols(&source.text);
    let mut input = TraceReplay::new(&tokens);
    input.inner.position = 3;
    let mut work = Some(100000);
    let mut live = super::LiveCallerPrediction::new_runtime_guarded(
        grammar, &source.id, admission, &mut work,
    )?;
    let result = live.predict(
        &source.id,
        &[call],
        expression,
        "choice",
        &mut input,
        &mut work,
    );
    assert!(result.unwrap_err().contains("Ambiguous"));
    Ok(())
}

#[test]
fn input_checked_deferred_choice_source_boundaries_and_recursive_boundary_refusal()
-> Result<(), String> {
    let compilation = deferred_source_choice_compilation(false)?;
    let grammar = compilation.grammar();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No actual program")?;
    let (path, _) = deferred_source_choice(grammar, &mut 100000)?;
    let before = serde_json::to_vec(program).map_err(|e| e.to_string())?;
    let mut recursive = program.clone();
    recursive.recursive_decisions.insert(path.into());
    assert!(matches!(
        recursive.input_checked_source_choice(path, grammar, &mut 100000)?,
        nullable_repeat::DeferredChoiceProof::RequiresRuntimeBoundary
    ));
    // A declined new route preserves a previously valid old static proof and
    // its actual remaining pool. It grants no boundary-dependent replay.
    let mut expected_work = 100000;
    let eligibility = recursive.input_checked_source_choice(path, grammar, &mut expected_work)?;
    assert!(matches!(
        eligibility,
        nullable_repeat::DeferredChoiceProof::RequiresRuntimeBoundary
    ));
    let old = recursive.verify_nullable_deferred_decision_with_remaining(
        path,
        grammar,
        &mut expected_work,
    );
    let mut actual_work = 100000;
    let actual = recursive.verify_input_checked_deferred_decision_with_remaining(
        path,
        grammar,
        &mut actual_work,
    );
    assert_eq!(actual_work, expected_work);
    match (old, actual) {
        (Ok(()), Ok(())) => {}
        (Err(old), Err(actual)) => {
            assert!(actual.starts_with(&old));
            assert!(actual.contains(
                "final recursive-first metadata requires an exact runtime completion boundary"
            ));
        }
        outcomes => panic!("Changed conservative fallback result: {outcomes:?}"),
    }
    let mut missing = grammar.clone();
    missing.declaration_sources.remove("Item");
    assert!(
        program
            .input_checked_source_choice(path, &missing, &mut 100000)
            .unwrap_err()
            .contains("original owner source")
    );
    let mut wrong = grammar.clone();
    wrong
        .declaration_sources
        .get_mut("Item")
        .ok_or("No actual Item source")?
        .end = 0;
    assert!(
        program
            .input_checked_source_choice(path, &wrong, &mut 100000)
            .unwrap_err()
            .contains("source interval differs")
    );
    assert!(matches!(
        program.input_checked_source_choice("rule/Item/repeat", grammar, &mut 100000)?,
        nullable_repeat::DeferredChoiceProof::Unsupported
    ));
    let mut forged = program.clone();
    forged
        .decisions
        .get_mut(path)
        .ok_or("No actual decision")?
        .entries
        .reverse();
    assert!(matches!(
        forged.input_checked_source_choice(path, grammar, &mut 100000)?,
        nullable_repeat::DeferredChoiceProof::Unsupported
    ));
    assert_eq!(
        serde_json::to_vec(program).map_err(|e| e.to_string())?,
        before
    );
    Ok(())
}

#[test]
fn input_checked_deferred_choice_one_pool_exact_short_and_old_proof_unchanged() -> Result<(), String>
{
    let compilation = deferred_source_choice_compilation(false)?;
    let grammar = compilation.grammar();
    let program = grammar.parser_prediction.as_ref().ok_or("No program")?;
    let (path, _) = deferred_source_choice(grammar, &mut 100000)?;
    let mut remaining = 100000;
    assert!(matches!(
        program.input_checked_source_choice(path, grammar, &mut remaining)?,
        nullable_repeat::DeferredChoiceProof::Qualified
    ));
    let used = 100000 - remaining;
    assert!(used > 1);
    for grant in [0, used - 1, used] {
        let mut work = grant;
        let result = program.input_checked_source_choice(path, grammar, &mut work);
        if grant == used {
            assert!(matches!(
                result?,
                nullable_repeat::DeferredChoiceProof::Qualified
            ));
        } else {
            assert!(result.unwrap_err().contains("budget"));
        }
        assert_eq!(work, 0);
    }
    let mut shared = 2 * used - 1;
    assert!(matches!(
        program.input_checked_source_choice(path, grammar, &mut shared)?,
        nullable_repeat::DeferredChoiceProof::Qualified
    ));
    assert!(
        program
            .input_checked_source_choice(path, grammar, &mut shared)
            .unwrap_err()
            .contains("budget")
    );
    assert_eq!(shared, 0);
    // Historical compatibility facade still selects the literal old worker.
    let legacy = program.verify_nullable_deferred_decision(path, grammar);
    let mut legacy_work = 10_000;
    assert_eq!(
        legacy,
        program.verify_nullable_deferred_decision_with_remaining(path, grammar, &mut legacy_work)
    );
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let (_, expression) = deferred_source_choice(grammar, &mut 100000)?;
    let Kind::Sequence(root) = &grammar.rules["Root"].body.kind else {
        return Err("No real finite caller".into());
    };
    let call = root
        .iter()
        .find(|e| matches!(&e.kind,Kind::Call(name)if name=="Item"))
        .ok_or("No finite Item call")?;
    let source = DefinitionSource::new(
        "memory:deferred-choice/cancel",
        SourceKind::Xtext,
        "{ } send a a b",
    );
    let admission = compilation.admission();
    let mut unlimited = None;
    assert!(
        super::LiveCallerPrediction::new_runtime_guarded(
            grammar,
            &source.id,
            admission,
            &mut unlimited
        )
        .is_err()
    );
    struct Cancel<'a> {
        inner: TraceReplay<'a>,
        reads: usize,
    }
    impl PredictionInput for Cancel<'_> {
        fn replay_position(&self) -> Option<usize> {
            self.inner.replay_position()
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.inner.checkpoint()
        }
        fn rewind(&mut self, p: usize) -> Result<(), String> {
            self.inner.rewind(p)
        }
        fn read(&mut self, w: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            self.reads += 1;
            if self.reads == 2 {
                return Err("deferred-choice input cancelled".into());
            }
            self.inner.read(w)
        }
    }
    let tokens = symbols(&source.text);
    let mut input = Cancel {
        inner: TraceReplay::new(&tokens),
        reads: 0,
    };
    input.inner.inner.position = 3;
    let mut work = Some(100000);
    let mut live = super::LiveCallerPrediction::new_runtime_guarded(
        grammar, &source.id, admission, &mut work,
    )?;
    let error = live
        .predict(
            &source.id,
            &[call],
            expression,
            "choice",
            &mut input,
            &mut work,
        )
        .unwrap_err();
    assert!(error.contains("deferred-choice input cancelled"));
    assert_eq!(input.reads, 2);
    assert!(
        live.predict(
            &source.id,
            &[call],
            expression,
            "choice",
            &mut input,
            &mut work
        )
        .is_err()
    );
    assert_eq!(input.reads, 2);
    Ok(())
}

include!("live_choice_boundary_tests.rs");

fn required_call_text_compilation(
    text: &str,
) -> Result<crate::grammar::CompiledRuntimeGrammar, String> {
    use intersect_model::definitions::bundle::DefinitionBundle;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::model_definition::ModelCompilation;
    use intersect_model::definitions::schema::Diagnostic;
    let ecore = DefinitionSource::new(
        "memory:deferred-choice/model.ecore",
        SourceKind::Ecore,
        r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="deferred" nsURI="urn:deferred-choice"><eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="children" upperBound="-1" containment="true" eType="#//Root"/></eClassifiers></e:EPackage>"##,
    );
    let mut syntax = DefinitionSource::new(
        "memory:deferred-choice/grammar.xtext",
        SourceKind::Xtext,
        text,
    );
    syntax.dependencies.push(ecore.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:deferred-choice/empty.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package deferredRules version \"1\";",
    );
    behavior.dependencies.push(ecore.id.clone());
    let expected = syntax.id.clone();
    let input = ModelCompilation {
        materialized_types: vec![],
        structure: ecore.id.clone(),
        namespace: "urn:deferred-choice".into(),
        behavior: behavior.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), syntax, behavior],
            namespaces: vec![],
        },
    };
    let migration = intersect_model::migration::migrate_ecore_bundle(
        input.bundle.clone(),
        &[(ecore.id, "memory:deferred-choice/model.rcore".into())].into(),
    )
    .map_err(|e| e.to_string())?;
    let converted = migration.remap_model_compilation(&input)?;
    // This trusted authored-source test adapter admits only the exact supplied
    // identity. The same native compiler still parses all actual source text.
    let validator =
        |bundle: &DefinitionBundle, id: &intersect_model::definitions::bundle::SourceId, _limits| {
            if id != &expected {
                return Err(Diagnostic::new(&id.uri, 0, 1, "Foreign authored source"));
            }
            bundle
                .source(id)
                .map(|_| ())
                .map_err(|e| Diagnostic::new(&id.uri, 0, 1, e))
        };
    crate::grammar::backend_test_support::runtime(
        &converted,
        &expected,
        crate::grammar::GrammarAnalysisWorkBudget::default(),
        &validator,
    )
}

// SAME native bundle/migration/compiler as the retained source-choice controls;
// only this genuine fixture schema adds the actual containment assignment.
fn required_call_source() -> String {
    live_choice_source().replace(
        "fragment Envelope returns t::Root: 'via' Item 'after';",
        "fragment Envelope returns t::Root: 'via' children+=Holder 'a' Item 'after'; Holder returns t::Root: {t::Root} (=> Body '.')?;",
    )
}

pub(super) type RequiredCallStage = (
    Grammar,
    ParserPrediction,
    RuntimeGuardedAdmission,
    Vec<super::super::follow::RequiredObjectCall>,
    usize,
);

pub(super) fn required_call_reserved_stage() -> Result<RequiredCallStage, String> {
    required_call_reserved_stage_with_text(&required_call_source())
}
fn required_call_reserved_stage_with_text(text: &str) -> Result<RequiredCallStage, String> {
    let compiled = required_call_text_compilation(text)?;
    let mut grammar = compiled.grammar().clone();
    grammar.parser_prediction = None;
    // Replay the SAME canonical compiler stages to inspect the transient
    // reserved companion before immutable publication. No records are forged,
    // cloned, deserialized or installed by this component fixture.
    let mut admission = RuntimeGuardedAdmission::new_with_policy_and_budget(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        super::super::GrammarAnalysisWorkBudget::default(),
    );
    let mut deferred = BTreeSet::new();
    // SAME order as analyze(): syntactic prediction checks contribute deferred
    // choices before FOLLOW contributes repeat/required-call obligations.
    // FOLLOW alone cannot reconstruct the native compiler's choice set.
    for (name, rule) in &grammar.rules {
        super::super::check_prediction(
            &rule.body,
            &grammar,
            &format!("rule/{name}"),
            &mut deferred,
            super::super::GrammarAnalysisWorkBudget::default(),
        )?;
    }
    let (required, remaining, mut certificates) =
        super::super::follow::check_required_calls_certified_runtime(
            &grammar,
            &mut deferred,
            None,
            None,
            Some(&mut admission),
        )?;
    let mut predicates = BTreeSet::new();
    for (name, rule) in &mut grammar.rules {
        super::super::bind_prediction_paths(
            &mut rule.body,
            &format!("rule/{name}"),
            &mut predicates,
            true,
        );
    }
    let mut program = ParserPrediction::compile_runtime_guarded_linear_repeats(
        &grammar,
        &predicates,
        None,
        Some(&mut certificates),
        &mut admission,
    )?;
    let mut shared_deferred = super::super::GrammarAnalysisWorkBudget::default().units();
    for path in deferred {
        if admission.contains(&path) {
            admission.validate_decision_with_remaining(
                &grammar,
                &program,
                &path,
                &mut shared_deferred,
            )?;
        } else {
            admission.verify_deferred_with_live_choices(
                &grammar,
                &mut program,
                &path,
                &mut shared_deferred,
            )?;
        }
    }
    let actual_program = compiled
        .grammar()
        .parser_prediction
        .as_ref()
        .ok_or("No genuine native program for stage comparison")?;
    assert_eq!(
        serde_json::to_value(&program).map_err(|error| error.to_string())?,
        serde_json::to_value(actual_program).map_err(|error| error.to_string())?,
        "Reserved-stage reconstruction must preserve the entire native program",
    );
    assert_eq!(
        admission.decisions().collect::<BTreeSet<_>>(),
        compiled.admission().decisions().collect::<BTreeSet<_>>(),
        "All native runtime obligations, including overlapping choices, must remain",
    );
    Ok((grammar, program, admission, required, remaining))
}

#[test]
fn input_checked_required_call_native_companion_preserves_full_exit_and_live_tail()
-> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let compiled = required_call_text_compilation(&required_call_source())?;
    let grammar = compiled.grammar();
    let admission = compiled.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No genuine program")?;
    let (path, expression) = deferred_source_choice(grammar, &mut 100_000)?;
    assert!(program.recursive_decisions.contains(path));
    assert!(admission.decision_is_runtime(path));
    assert!(!grammar.rules["Holder"].fragment);
    let (grammar_stage, program_stage, mut admission_stage, required, mut work) =
        required_call_reserved_stage()?;
    let call = required
        .iter()
        .find(|call| call.owner == "Envelope" && call.callee == "Holder")
        .ok_or("Actual nullable containment Call did not produce the required FOLLOW obligation")?;
    assert_eq!(call.path, "rule/Envelope/s1/value");
    let legacy = program_stage
        .verify_required_object_call(&call.path, &call.callee, &mut 10_000_000)
        .expect_err("The retained Legacy verifier must not acquire input-checked authority");
    assert!(
        legacy.contains("shared-prefix") || legacy.contains("Unqualified completion commitment"),
        "{legacy}"
    );
    assert!(!legacy.contains("budget"), "{legacy}");
    admission_stage.verify_required_input_checked_call(
        &grammar_stage,
        &program_stage,
        &call.owner,
        &call.path,
        &call.callee,
        &mut work,
    )?;
    assert!(work > 0);
    let calls = live_choice_calls(grammar)?;
    for (text, expected) in [
        ("{ } via a send { } after done", Some(0)),
        ("{ } via a send a a { } after done", Some(1)),
        ("{ } via a send { send a a { } } after done", Some(0)),
        ("{ } via a send { } wrong done", None),
        ("{ } via a send { } after done trailing", None),
    ] {
        let source = DefinitionSource::new("memory:required-call/input", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 5;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        assert_eq!(
            live.predict(
                &source.id, &calls, expression, "choice", &mut input, &mut work
            )?,
            expected
        );
        if expected.is_some() {
            assert_eq!(input.inner.position, tokens.len());
        }
        assert!(work.is_some_and(|n| n > 0));
    }
    // Nested First guards must not grant the competing outer alternatives
    // priority. Actual complete overlapping input remains an ambiguity.
    let text = "{ } via a send a { } after done";
    let source = DefinitionSource::new("memory:required-call/overlap", SourceKind::Xtext, text);
    let tokens = symbols(text);
    let mut input = TraceReplay::new(&tokens);
    input.inner.position = 5;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    let error = live
        .predict(
            &source.id, &calls, expression, "choice", &mut input, &mut work,
        )
        .expect_err("Real overlapping recursive guarded input must remain ambiguous");
    assert!(error.contains("Ambiguous"), "{error}");
    assert!(!error.contains("budget"), "{error}");
    assert!(work.is_some_and(|n| n > 0));
    struct RequiredCancel<'a> {
        inner: TraceReplay<'a>,
        reads: usize,
    }
    impl PredictionInput for RequiredCancel<'_> {
        fn replay_position(&self) -> Option<usize> {
            self.inner.replay_position()
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.inner.checkpoint()
        }
        fn rewind(&mut self, p: usize) -> Result<(), String> {
            self.inner.rewind(p)
        }
        fn read(&mut self, work: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            self.reads += 1;
            if self.reads == 2 {
                return Err("Required Call input cancelled".into());
            }
            self.inner.read(work)
        }
    }
    let tokens = symbols("{ } via a send a a { } after done");
    let source = DefinitionSource::new(
        "memory:required-call/cancel",
        SourceKind::Xtext,
        "{ } via a send a a { } after done",
    );
    let mut input = RequiredCancel {
        inner: TraceReplay::new(&tokens),
        reads: 0,
    };
    input.inner.inner.position = 5;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    let error = live
        .predict(
            &source.id, &calls, expression, "choice", &mut input, &mut work,
        )
        .expect_err("Cancellation remains hard");
    assert!(error.contains("Required Call input cancelled"), "{error}");
    assert_eq!(input.reads, 2);
    assert!(
        live.predict(
            &source.id, &calls, expression, "choice", &mut input, &mut work
        )
        .is_err()
    );
    assert_eq!(input.reads, 2);
    Ok(())
}

#[test]
fn input_checked_required_call_shared_pool_exact_short_and_sticky_failure() -> Result<(), String> {
    let (grammar, program, mut admission, required, _) = required_call_reserved_stage()?;
    let call = required
        .iter()
        .find(|call| call.owner == "Envelope" && call.callee == "Holder")
        .ok_or("No actual required Call")?;
    let mut work = 10_000_000;
    admission.verify_required_input_checked_call(
        &grammar,
        &program,
        &call.owner,
        &call.path,
        &call.callee,
        &mut work,
    )?;
    let used = 10_000_000 - work;
    assert!(used > 0);
    let (grammar, program, mut admission, required, _) = required_call_reserved_stage()?;
    let call = required
        .iter()
        .find(|call| call.owner == "Envelope" && call.callee == "Holder")
        .ok_or("No actual required Call")?;
    let mut shared = used
        .checked_mul(2)
        .and_then(|n| n.checked_add(17))
        .ok_or("Test work overflow")?;
    shared -= 17; // The proof must respect a preconsumed shared pool.
    for _ in 0..2 {
        admission.verify_required_input_checked_call(
            &grammar,
            &program,
            &call.owner,
            &call.path,
            &call.callee,
            &mut shared,
        )?;
    }
    assert_eq!(shared, 0, "No per-call reset or hidden fresh grant");
    for initial in [0, used - 1, used] {
        let (grammar, program, mut admission, required, _) = required_call_reserved_stage()?;
        let call = required
            .iter()
            .find(|call| call.owner == "Envelope" && call.callee == "Holder")
            .ok_or("No actual required Call")?;
        let mut remaining = initial;
        let result = admission.verify_required_input_checked_call(
            &grammar,
            &program,
            &call.owner,
            &call.path,
            &call.callee,
            &mut remaining,
        );
        assert_eq!(remaining, 0);
        if initial == used {
            result?;
        } else {
            let error = result.expect_err("Short/zero proof cannot grant authority");
            assert!(error.contains("budget"), "{error}");
            let error = admission
                .verify_required_input_checked_call(
                    &grammar,
                    &program,
                    &call.owner,
                    &call.path,
                    &call.callee,
                    &mut 10_000_000,
                )
                .expect_err("A caught denial must poison the same companion");
            assert!(error.contains("actual input-checked companion"), "{error}");
        }
    }
    Ok(())
}

#[test]
fn input_checked_required_call_rejects_missing_foreign_source_return_and_hidden_obligations()
-> Result<(), String> {
    for corruption in 0..9 {
        let (mut grammar, mut program, mut admission, required, _) =
            required_call_reserved_stage()?;
        let call = required
            .iter()
            .find(|call| call.owner == "Envelope" && call.callee == "Holder")
            .ok_or("No actual required Call")?;
        let mut owner = call.owner.as_str();
        let mut callee = call.callee.as_str();
        match corruption {
            0 => {
                admission = RuntimeGuardedAdmission::new_with_policy(
                    RuntimeGuardedPolicy::InputCheckedCompletionScopes,
                )
            }
            1 => program = program.clone(), // A cloned marker is not the reserved owner.
            2 => owner = "Root",
            3 => callee = "Body",
            4 => {
                let (_, stop) = program.rules["Holder"];
                program.states[stop].follow.clear();
            }
            5 => {
                grammar
                    .declaration_sources
                    .get_mut("Envelope")
                    .ok_or("No real source")?
                    .end = 0
            }
            6 => {
                let Some(runtime_guarded::Requirement::InputCheckedCompletionScopes { decisions }) =
                    program.runtime_requirement.as_deref_mut()
                else {
                    return Err("No genuine marker".into());
                };
                let key = decisions
                    .iter()
                    .find(|key| key.starts_with("rule/Item/"))
                    .cloned()
                    .ok_or("No actual choice key")?;
                decisions.remove(&key);
            }
            7 => {
                grammar
                    .declaration_sources
                    .get_mut("Item")
                    .ok_or("No nested source")?
                    .source
                    .sha256 = "foreign-same-text-context".into();
            }
            _ => {
                for decision in program.decisions.values_mut() {
                    decision.exit = None;
                }
            }
        }
        let mut remaining = 10_000_000;
        let error = admission
            .verify_required_input_checked_call(
                &grammar,
                &program,
                owner,
                &call.path,
                callee,
                &mut remaining,
            )
            .expect_err("Missing/foreign/source/return authority must reject");
        assert!(!error.contains("budget"), "{corruption}: {error}");
        assert!(remaining > 0);
        assert!(
            admission
                .verify_required_input_checked_call(
                    &grammar,
                    &program,
                    &call.owner,
                    &call.path,
                    &call.callee,
                    &mut remaining
                )
                .is_err(),
            "caught error must poison"
        );
    }
    let text = required_call_source().replace("Envelope 'done';", "Envelope Tail 'done';")
        + " fragment Tail returns t::Root hidden(WS): 'tail'; terminal WS: ' ';";
    let error = required_call_text_compilation(&text)
        .err()
        .ok_or("Reachable hidden override compiled")?;
    assert!(
        error.contains("reachable rule switches hidden policy: Tail"),
        "{error}"
    );
    assert!(!error.contains("budget"), "{error}");
    Ok(())
}

#[test]
fn input_checked_required_entry_priority_native_guarded_first_is_not_missing_runtime_authority()
-> Result<(), String> {
    let text = required_call_source().replace("Body | (=> Prefix Body)", "(-> Prefix Body) | Body");
    let (grammar, program, mut admission, required, mut remaining) =
        required_call_reserved_stage_with_text(&text)?;
    assert!(
        grammar.parser_prediction.is_none(),
        "Preserve actual reserved stage"
    );
    let (path, expression) = deferred_source_choice_in_program(&grammar, &program, &mut 100_000)?;
    assert!(
        program.recursive_decisions.contains(path),
        "Must exercise actual final recursive metadata"
    );
    assert!(
        !admission.decision_is_runtime(path),
        "Canonical check_prediction deliberately does not defer a prior visible entry guard"
    );
    let decision = program
        .decisions
        .get(path)
        .ok_or("No actual guarded-first decision")?;
    assert!(program.completion_entry_priority(decision, &mut remaining)?);
    let call = required
        .iter()
        .find(|call| call.owner == "Envelope" && call.callee == "Holder")
        .ok_or("No actual nullable required Call")?;
    let (start, end) =
        program.required_object_call_binding(&call.path, &call.callee, &mut remaining)?;
    // These old policies already accepted exactly this guarded recursive scope.
    // No new static disjointness, nullable or ordinary-live grant is needed.
    for policy in [
        CompletionVerification::Legacy,
        CompletionVerification::BoundedGuardOverapproximation,
    ] {
        let scope = program.verify_completion_scope_with_verification(
            start,
            end,
            &mut remaining,
            policy,
        )?;
        program.verify_required_full_exit(&scope, &mut remaining)?;
    }
    program.verify_required_object_call(&call.path, &call.callee, &mut remaining)?;
    let before = serde_json::to_vec(&program).map_err(|e| e.to_string())?;
    let proof_entry = remaining;
    admission.verify_required_input_checked_call(
        &grammar,
        &program,
        &call.owner,
        &call.path,
        &call.callee,
        &mut remaining,
    )?;
    assert_eq!(
        serde_json::to_vec(&program).map_err(|e| e.to_string())?,
        before
    );
    assert!(
        !admission.decision_is_runtime(path),
        "No obligation is minted by required-call verification"
    );
    assert!(remaining > 0);
    let used = proof_entry - remaining;
    for initial in [0, used - 1, used] {
        let (grammar, program, mut admission, required, _) =
            required_call_reserved_stage_with_text(&text)?;
        let call = required
            .iter()
            .find(|call| call.owner == "Envelope" && call.callee == "Holder")
            .ok_or("No required Call")?;
        let mut work = initial;
        let result = admission.verify_required_input_checked_call(
            &grammar,
            &program,
            &call.owner,
            &call.path,
            &call.callee,
            &mut work,
        );
        assert_eq!(work, 0);
        if initial == used {
            result?;
        } else {
            let error =
                result.expect_err("Short source/entry priority proof cannot grant authority");
            assert!(error.contains("budget"), "{error}");
            assert!(
                admission
                    .verify_required_input_checked_call(
                        &grammar,
                        &program,
                        &call.owner,
                        &call.path,
                        &call.callee,
                        &mut 10_000_000
                    )
                    .is_err()
            );
        }
    }
    // SAME canonical guarded-priority worker under its exact owner stop. The
    // genuinely overlapping guarded language keeps its existing source order.
    let (_, stop) = program.rules["Item"];
    let tokens = symbols("a { }");
    for policy in [
        CompletionVerification::Legacy,
        CompletionVerification::InputCheckedCompletion,
    ] {
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(1_000_000);
        let outcome = program.run_prediction_at_depth_with_completion_verification(
            PredictionGoal::Decision {
                decision,
                recursive_first: true,
                boundary: Some(DecisionBoundary {
                    live: false,
                    stop,
                    returns: &[],
                }),
            },
            &mut input,
            &mut work,
            0,
            policy,
        )?;
        assert!(matches!(outcome, PredictionOutcome::Selected(Some(0))));
        assert!(work.is_some_and(|left| left > 0));
    }
    assert!(matches!(expression.kind, Kind::Alternative(_)));
    Ok(())
}

#[test]
fn input_checked_required_entry_priority_source_guard_and_full_boundary_forgery_reject()
-> Result<(), String> {
    for first in [true, false] {
        let text = required_call_source().replace(
            "Body | (=> Prefix Body)",
            if first {
                "(-> Prefix Body) | Body"
            } else {
                "(=> Prefix Body) | Body"
            },
        );
        let (grammar, program, mut admission, required, mut remaining) =
            required_call_reserved_stage_with_text(&text)?;
        let call = required
            .iter()
            .find(|call| call.owner == "Envelope" && call.callee == "Holder")
            .ok_or("No actual required Call")?;
        // The real Full/First source is eligible without a per-input record.
        admission.verify_required_input_checked_call(
            &grammar,
            &program,
            &call.owner,
            &call.path,
            &call.callee,
            &mut remaining,
        )?;
        for corrupt_source in [true, false] {
            let (mut grammar, mut program, mut admission, required, _) =
                required_call_reserved_stage_with_text(&text)?;
            let call = required
                .iter()
                .find(|call| call.owner == "Envelope" && call.callee == "Holder")
                .ok_or("No actual required Call")?;
            assert!(
                grammar.parser_prediction.is_none(),
                "Preserve actual reserved stage"
            );
            let (path, _) = deferred_source_choice_in_program(&grammar, &program, &mut 100_000)?;
            let path = path.to_owned();
            let entry = program.decisions[&path].entries[0];
            if corrupt_source {
                let owner = grammar.rules.get_mut("Item").ok_or("No real Item source")?;
                fn remove_guard(expr: &mut Expr) {
                    expr.first_set_predicated = false;
                    match &mut expr.kind {
                        Kind::SyntacticPredicate(value) => {
                            expr.kind = value.kind.clone();
                        }
                        Kind::Sequence(items) | Kind::Alternative(items) => {
                            for item in items {
                                remove_guard(item)
                            }
                        }
                        Kind::Assign { value, .. } | Kind::BooleanAssign { value, .. } => {
                            remove_guard(value)
                        }
                        _ => {}
                    }
                }
                remove_guard(&mut owner.body);
            } else {
                match &mut program.states[entry].edges[0] {
                    Edge::First { path, .. } => *path = "rule/Item/foreign".into(),
                    Edge::Full { stop, .. } => *stop = entry,
                    _ => return Err("Expected real visible guard".into()),
                }
            }
            let mut work = 10_000_000;
            let error = admission
                .verify_required_input_checked_call(
                    &grammar,
                    &program,
                    &call.owner,
                    &call.path,
                    &call.callee,
                    &mut work,
                )
                .expect_err("Source/First/Full forgery must not qualify priority");
            assert!(!error.contains("budget"), "{error}");
            assert!(work > 0);
            assert!(
                admission
                    .verify_required_input_checked_call(
                        &grammar,
                        &program,
                        &call.owner,
                        &call.path,
                        &call.callee,
                        &mut work
                    )
                    .is_err()
            );
        }
    }
    Ok(())
}

include!("live_multiway_choice_tests.rs");
