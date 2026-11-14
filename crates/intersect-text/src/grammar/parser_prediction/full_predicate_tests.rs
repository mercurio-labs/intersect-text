// Full guards use the same Decision/Completion evaluator and one operand graph.
fn full(value: Expr) -> Expr {
    expr(Kind::SyntacticPredicate(Box::new(value)))
}
fn full_oracle_program(nested: bool) -> ParserPrediction {
    let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
    inner.cardinality = '?';
    let decision = choice(vec![
        full(sequence(vec![keyword("a"), call("Inner"), keyword("b")])),
        sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
    ]);
    let mut g = if nested {
        grammar(
            call("Probe"),
            vec![
                ("Probe", call("Nested")),
                ("Nested", decision),
                ("Inner", inner),
            ],
        )
    } else {
        grammar(decision, vec![("Inner", inner)])
    };
    g.profile = super::super::Profile::NativeV68;
    ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap()
}

#[test]
fn authored_full_guard_decisions_match_all_original_recursive_full_oracle_rows() {
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
        let text = row["text"].as_str().unwrap();
        let tokens = symbols(text);
        let mut input = ReplayTokens::new(&tokens);
        assert_eq!(
            program
                .predict_with_replay_input("rule/Entry/choice", &mut input, &mut Some(100_000))
                .unwrap(),
            match row["branch"].as_str().unwrap() {
                "g" => Some(0),
                "f" => Some(1),
                "" => None,
                branch => panic!("unknown oracle branch {branch}"),
            },
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 11);
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

#[test]
fn nested_full_guards_preserve_recursive_fallback_and_complete_operand_truth() {
    let program = full_oracle_program(true);
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
        let text = row["text"].as_str().unwrap();
        let (matched, end) = completed_prefix(&program, "Probe", text, &mut Some(100_000)).unwrap();
        assert_eq!(
            matched && end + 1 == symbols(text).len(),
            row["accepted"].as_bool().unwrap(),
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 11);
}

#[derive(Debug)]
enum TraceEvent {
    Read(usize, Option<usize>),
    Rewind(usize, usize),
}
struct TraceReplay<'a> {
    inner: ReplayTokens<'a>,
    events: Vec<TraceEvent>,
}
impl<'a> TraceReplay<'a> {
    fn new(tokens: &'a [PredictionSymbol]) -> Self {
        Self {
            inner: ReplayTokens::new(tokens),
            events: vec![],
        }
    }
}
impl PredictionInput for TraceReplay<'_> {
    fn replay_position(&self) -> Option<usize> {
        Some(self.inner.position)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        self.inner.checkpoint()
    }
    fn rewind(&mut self, position: usize) -> Result<(), String> {
        self.events
            .push(TraceEvent::Rewind(self.inner.position, position));
        self.inner.rewind(position)
    }
    fn read(&mut self, work: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
        self.events
            .push(TraceEvent::Read(self.inner.position, *work));
        self.inner.read(work)
    }
}
#[test]
fn full_probe_rewinds_failed_and_exhausted_reads_without_refunding_shared_work() {
    let program = full_oracle_program(false);
    let tokens = symbols("a a b c");
    let mut input = TraceReplay::new(&tokens);
    let mut work = Some(100_000);
    assert_eq!(
        program
            .predict_with_replay_input("rule/Entry/choice", &mut input, &mut work)
            .unwrap(),
        Some(1)
    );
    // The guard restores zero; ordinary Decision then reads its one normal
    // distinguishing token. Its final position is not the probe's restoration.
    assert_eq!(input.inner.position, 1);
    assert!(input.events.windows(2).any(
        |events| matches!(events,[TraceEvent::Rewind(from,0), TraceEvent::Read(0,_)] if *from>0)
    ));
    let used = 100_000 - work.unwrap();
    println!(
        "V68_FULL_TRACE complete used={used} events={:?}",
        input.events
    );
    for allowance in [used, used - 1] {
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(allowance);
        let result = program.predict_with_replay_input("rule/Entry/choice", &mut input, &mut work);
        if allowance == used {
            assert_eq!(result.unwrap(), Some(1));
        } else {
            assert!(result.unwrap_err().contains("budget"));
        }
        assert_eq!(work, Some(0));
        println!(
            "V68_FULL_TRACE boundary allowance={allowance} remaining={work:?} position={} events={:?}",
            input.inner.position, input.events
        );
    }
    // Find an actually denied replayed read after positive cursor progress.
    // This is an independent literal-work search, not an invented leaf counter.
    let mut witnessed = false;
    for allowance in 1..used {
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(allowance);
        let result = program.predict_with_replay_input("rule/Entry/choice", &mut input, &mut work);
        if input
            .events
            .iter()
            .any(|event| matches!(event, TraceEvent::Read(position,Some(0)) if *position>0))
        {
            assert!(result.unwrap_err().contains("budget"));
            assert_eq!(work, Some(0));
            assert_eq!(input.inner.position, 0);
            assert!(matches!(input.events.last(),Some(TraceEvent::Rewind(from,0)) if *from>0));
            println!(
                "V68_FULL_TRACE exhausted_read allowance={allowance} remaining={work:?} events={:?}",
                input.events
            );
            witnessed = true;
            break;
        }
    }
    assert!(witnessed, "no exhausted consumed probe was tested");
    let mut input = ReplayTokens::new(&tokens);
    assert!(
        program
            .predict_with_replay_input("rule/Entry/choice", &mut input, &mut None)
            .unwrap_err()
            .contains("finite")
    );
    assert_eq!(input.position, 0);
    assert!(
        program
            .predict("rule/Entry/choice", || Ok(None), &mut Some(100_000))
            .unwrap_err()
            .contains("replayable")
    );
}

#[test]
fn full_guard_stops_at_operand_before_unrelated_suffix_and_keeps_cardinality_outside() {
    let mut guarded = full(sequence(vec![keyword("a"), keyword("b")]));
    guarded.cardinality = '*';
    let mut g = grammar(sequence(vec![guarded, keyword("c")]), vec![]);
    g.profile = super::super::Profile::NativeV68;
    let program =
        ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap();
    for (text, expected) in [("a b c", Some(0)), ("a c", None), ("c", None)] {
        let tokens = symbols(text);
        let mut input = ReplayTokens::new(&tokens);
        assert_eq!(
            program
                .predict_with_replay_input("rule/Entry/s0/repeat", &mut input, &mut Some(100_000))
                .unwrap(),
            expected,
            "{text}"
        );
    }
}

#[test]
fn full_probe_depth_is_cumulative_with_outer_logical_calls_at_original_ceiling() {
    let mut g = grammar(choice(vec![full(keyword("a")), keyword("b")]), vec![]);
    g.profile = super::super::Profile::NativeV68;
    let program =
        ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap();
    let decision = &program.decisions["rule/Entry/choice"];
    let tokens = symbols("a");
    for (depth, accepted) in [(127, true), (128, false)] {
        let mut input = ReplayTokens::new(&tokens);
        let result = program.run_prediction_at_depth(
            PredictionGoal::Decision {
                decision,
                recursive_first: false,
                boundary: None,
            },
            &mut input,
            &mut Some(100_000),
            depth,
        );
        if accepted {
            assert!(matches!(
                result.unwrap(),
                PredictionOutcome::Selected(Some(0))
            ));
        } else {
            assert!(result.unwrap_err().contains("depth"));
            assert_eq!(input.position, 0);
        }
    }
}

#[test]
fn nested_full_direct_and_mutual_calls_share_the_original_depth_ceiling() {
    for mutual in [false, true] {
        // Actual caller return frames consume the same ceiling as nested probes.
        // Only two nested operands are needed; deep repeated lookahead otherwise
        // reaches its honest work ceiling before it can isolate the depth boundary.
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
                    call(if index + 1 == callers {
                        "Gate"
                    } else {
                        "Unused"
                    }),
                ));
            }
            for index in 0..callers.saturating_sub(1) {
                owned
                    .iter_mut()
                    .find(|(name, _)| name == &format!("Caller{index}"))
                    .unwrap()
                    .1 = call(&format!("Caller{}", index + 1));
            }
            let pairs = owned
                .iter()
                .map(|(name, body)| (name.as_str(), body.clone()))
                .collect();
            let mut g = grammar(call("Caller0"), pairs);
            g.profile = super::super::Profile::NativeV68;
            let program =
                ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new())
                    .unwrap();
            let tokens = symbols("a a b b");
            let mut work = Some(100_000);
            let result = program.probe_rule_completion("Entry", &tokens, &mut work);
            if one_short {
                assert!(result.unwrap_err().contains("depth"));
            } else {
                assert!(result.unwrap());
                let used = 100_000 - work.unwrap();
                assert!(
                    program
                        .probe_rule_completion("Entry", &tokens, &mut Some(used))
                        .unwrap()
                );
                assert!(
                    program
                        .probe_rule_completion("Entry", &tokens, &mut Some(used - 1))
                        .unwrap_err()
                        .contains("budget")
                );
                println!(
                    "V68_FULL_DEPTH mutual={mutual} caller_frames={callers} nested_operands=2 exact_work={used}"
                );
            }
        }
    }
}

#[test]
fn nested_full_work_exhaustion_stays_explicit_before_the_depth_ceiling() {
    let mut nested = full(call("Inner"));
    nested.cardinality = '?';
    let mut g = grammar(
        choice(vec![full(call("Inner")), keyword("c")]),
        vec![("Inner", sequence(vec![keyword("a"), nested, keyword("b")]))],
    );
    g.profile = super::super::Profile::NativeV68;
    let program =
        ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap();
    let text = format!("{} {}", vec!["a"; 14].join(" "), vec!["b"; 14].join(" "));
    let tokens = symbols(&text);
    let mut input = TraceReplay::new(&tokens);
    let mut work = Some(1_000_000);
    assert!(
        program
            .predict_with_replay_input("rule/Entry/choice", &mut input, &mut work)
            .unwrap_err()
            .contains("budget")
    );
    assert_eq!(work, Some(0));
    assert_eq!(input.inner.position, 0);
}

#[test]
fn legacy_first_hidden_scope_retains_the_entire_public_error_string() {
    for profile in [Profile::NativeV66, Profile::NativeV67] {
        let mut g = grammar(
            call("Probe"),
            vec![(
                "Probe",
                sequence(vec![expr(Kind::Action("Object".into())), keyword("a")]),
            )],
        );
        g.profile = profile;
        let program =
            ParserPrediction::compile_with_first_predicates(&g, &["rule/Probe/s1".into()].into())
                .unwrap();
        assert_eq!(
            program
                .probe_rule_completion("Probe", &symbols("a"), &mut Some(10_000))
                .unwrap_err(),
            "Completion guard visibility remains unsupported at rule/Probe/s1"
        );
    }
}

// SysML `send x.y;` (SendNode: `'send' ( ActionBody | => ( NodeParameterMember
// SenderReceiverPart? | ... ) ActionBody )`). Inside the predicate probe, the
// expression's BaseExpression-like choice ends at the predicate: after `id`
// the plain reference already reaches that stop (the optional suffixes may be
// empty) while the reference, metadata and chained-invocation readings all
// still match `.`. The stop is the probe's, not the parse's, so the longer
// match is read on (greedy bounded acceptance) instead of being reported as
// an ambiguous accepting continuation.
#[test]
fn bounded_acceptance_reads_on_to_the_longer_match_inside_a_full_probe() {
    let optional = |mut value: Expr| {
        value.cardinality = '?';
        value
    };
    let mut g = grammar(
        choice(vec![
            keyword("k"),
            sequence(vec![full(call("Group")), keyword(";")]),
        ]),
        vec![
            (
                "Group",
                sequence(vec![
                    call("Expr"),
                    optional(sequence(vec![keyword("via"), call("Expr")])),
                ]),
            ),
            (
                "Expr",
                sequence(vec![
                    call("Base"),
                    optional(sequence(vec![keyword("."), keyword("id")])),
                ]),
            ),
            (
                "Base",
                choice(vec![
                    keyword("id"),
                    sequence(vec![keyword("id"), keyword("."), keyword("m")]),
                    sequence(vec![
                        keyword("id"),
                        keyword("."),
                        keyword("id"),
                        keyword("("),
                        keyword(")"),
                    ]),
                ]),
            ),
        ],
    );
    g.profile = super::super::Profile::NativeV68;
    let program =
        ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap();
    for (text, expected) in [
        ("k", Some(0)),
        ("id ;", Some(1)),
        ("id . id ;", Some(1)),
        ("id . m ;", Some(1)),
        ("id . id ( ) ;", Some(1)),
        ("id via id . id ;", Some(1)),
        ("id . id via id . m ;", Some(1)),
    ] {
        let tokens = symbols(text);
        let mut input = ReplayTokens::new(&tokens);
        assert_eq!(
            program
                .predict_with_replay_input("rule/Entry/choice", &mut input, &mut Some(100_000))
                .map_err(|error| format!("{text}: {error}"))
                .unwrap(),
            expected,
            "{text}"
        );
    }
}
