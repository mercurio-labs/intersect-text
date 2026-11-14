// Authored actual runtime Call/return graphs. These are not original KerML
// fit, native language admission, or independent semantic-oracle evidence.
fn ports_state(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn ports_program(states: Vec<State>, entries: Vec<usize>) -> ParserPrediction {
    ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states,
        rules: BTreeMap::new(),
        decisions: BTreeMap::from([(
            "ports".into(),
            Decision {
                entries,
                exit: None,
            },
        )]),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    }
}
fn ports_dag_program(callers: usize, levels: usize) -> ParserPrediction {
    let mut states = vec![
        State {
            rule_stop: true,
            ..State::default()
        },
        ports_state(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 0)]),
        State {
            accepting: true,
            ..State::default()
        },
    ];
    let mut start = 1;
    for _ in 0..levels {
        let left = states.len();
        states.push(ports_state(vec![Edge::Epsilon(start)]));
        let right = states.len();
        states.push(ports_state(vec![Edge::Epsilon(start)]));
        let split = states.len();
        states.push(ports_state(vec![Edge::Epsilon(left), Edge::Epsilon(right)]));
        start = split;
    }
    let mut entries = Vec::new();
    for index in 0..callers {
        let follow = states.len();
        states.push(ports_state(vec![Edge::Token(
            PredictionSymbol::Keyword(format!("tail{index}")),
            2,
        )]));
        let entry = states.len();
        states.push(ports_state(vec![Edge::Call {
            target: start,
            follow,
        }]));
        states[0].follow.push(follow);
        entries.push(entry);
    }
    let mut program = ports_program(states, entries);
    program.rules.insert("Shared".into(), (start, 0));
    program
}
type PortsResult = (
    Result<Option<usize>, String>,
    usize,
    usize,
    Vec<RuntimeSourceEvent>,
);
fn ports_run<'program>(
    program: &'program ParserPrediction,
    source: &mut PortRuntimePredictionSource<'program>,
    id: &intersect_model::definitions::bundle::SourceId,
    text: &str,
    work: &mut Option<usize>,
) -> PortsResult {
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let value = program.predict_with_port_runtime_source(
        source,
        id,
        &[],
        "ports",
        &mut input,
        work,
        CompletionVerification::Legacy,
    );
    (
        value,
        input.position,
        input.furthest,
        input.events.into_inner(),
    )
}
fn ports_plain(program: &ParserPrediction, text: &str, work: &mut Option<usize>) -> PortsResult {
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let value = program.predict_with_replay_input("ports", &mut input, work);
    (
        value,
        input.position,
        input.furthest,
        input.events.into_inner(),
    )
}

#[test]
fn runtime_ports_first_cold_shared_dag_saves_actual_work_across_unequal_full_caller_tails() {
    let program = ports_dag_program(48, 32);
    let bytes = serde_json::to_vec(&program).unwrap();
    let id = runtime_source_id();
    // Actual Call edges carry 48 distinct suffix states; this is not duplicate
    // alternatives with a callerless or truncated return context.
    let tails = program.decisions["ports"]
        .entries
        .iter()
        .map(|&state| {
            let Edge::Call { follow, .. } = &program.states[state].edges[0] else {
                panic!("Call fixture")
            };
            *follow
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(tails.len(), 48);
    for text in ["x tail0", "x tail47", "x unrelated", "", "unrelated"] {
        let mut old_work = Some(100_000);
        let expected = ports_plain(&program, text, &mut old_work);
        let mut work = Some(100_000);
        let mut scope = PortRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        let actual = ports_run(&program, &mut scope, &id, text, &mut work);
        println!(
            "RUNTIME_PORTS_COLD {}",
            serde_json::json!({
            "text":text,"actual":actual,"old":expected,
            "old_work":100_000-old_work.unwrap(),"new_work_including_setup":100_000-work.unwrap(),
            "graph_summary_hits":scope.hits(),"graph_summary_builds":scope.publications(),
            "emitted_full_configurations":scope.emitted_configurations(),
            "logical_cumulative":scope.logical_bytes(),"original_language_award":false})
        );
        assert_eq!(actual, expected);
        assert!(scope.hits() > 0 && scope.publications() > 0);
        assert!(scope.emitted_configurations() >= 48);
        assert!(
            work.unwrap() > old_work.unwrap(),
            "paid FIRST-cold authored DAG utility required"
        );
        assert!(!scope.is_poisoned());
    }
    assert_eq!(serde_json::to_vec(&program).unwrap(), bytes);
}

fn ports_sequence(allowance: usize, preconsumed: usize) -> (Result<(), String>, usize, bool) {
    let program = ports_dag_program(8, 8);
    let id = runtime_source_id();
    let mut work = Some(allowance);
    for _ in 0..preconsumed {
        charge(&mut work).unwrap();
    }
    let mut scope = match PortRuntimePredictionSource::new(&program, &id, &[], &mut work) {
        Ok(scope) => scope,
        Err(error) => return (Err(error), allowance - work.unwrap(), true),
    };
    let result = (|| {
        assert_eq!(
            ports_run(&program, &mut scope, &id, "x tail7", &mut work).0?,
            Some(7)
        );
        assert_eq!(
            ports_run(&program, &mut scope, &id, "x tail0", &mut work).0?,
            Some(0)
        );
        Ok(())
    })();
    (result, allowance - work.unwrap(), scope.is_poisoned())
}
#[test]
fn runtime_ports_original_pool_zero_exact_short_preconsumed_poison_and_fresh_recovery() {
    let (result, used, failed) = ports_sequence(100_000, 0);
    result.unwrap();
    assert!(!failed);
    for grant in [0, 1, used - 1, used] {
        let (result, consumed, failed) = ports_sequence(grant, 0);
        assert_eq!(consumed, grant);
        if grant == used {
            result.unwrap();
            assert!(!failed);
        } else {
            assert!(result.unwrap_err().contains("budget"));
            assert!(failed);
        }
    }
    let (result, consumed, failed) = ports_sequence(used + 7, 7);
    result.unwrap();
    assert_eq!(consumed, used + 7);
    assert!(!failed);
    assert!(ports_sequence(used, 0).0.is_ok());
    let program = ports_program(
        vec![
            ports_state(vec![Edge::Epsilon(2), Edge::Epsilon(3)]),
            ports_state(vec![Edge::Token(PredictionSymbol::Keyword("z".into()), 4)]),
            ports_state(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 4)]),
            ports_state(vec![Edge::Token(PredictionSymbol::Keyword("y".into()), 4)]),
            State {
                accepting: true,
                ..State::default()
            },
        ],
        vec![0, 1],
    );
    let id = runtime_source_id();
    let mut partial_denial = false;
    for grant in 0..used {
        let mut work = Some(grant);
        if let Ok(mut scope) = PortRuntimePredictionSource::new(&program, &id, &[], &mut work) {
            let value = ports_run(&program, &mut scope, &id, "x", &mut work);
            if value.0.is_err() && scope.emitted_configurations() == 1 {
                assert!(scope.is_poisoned());
                assert_eq!(work, Some(0));
                partial_denial = true;
                break;
            }
        }
    }
    assert!(
        partial_denial,
        "denial after one of two port rows must poison the actual scope"
    );
    println!(
        "RUNTIME_PORTS_EXACT {used} short={} preconsumed={}",
        used - 1,
        used + 7
    );
}

#[test]
fn runtime_ports_foreign_program_source_hidden_path_and_input_cancellation_poison_actual_scope() {
    let program = ports_dag_program(4, 3);
    let foreign_program = program.clone();
    let id = runtime_source_id();
    for failure in 0..5 {
        let mut work = Some(100_000);
        let mut scope = PortRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        ports_run(&program, &mut scope, &id, "x tail0", &mut work)
            .0
            .unwrap();
        let mut foreign_id = id.clone();
        foreign_id.sha256.push('0');
        let tokens = symbols("x tail0");
        let mut input = RuntimeSourceInput::new(&tokens);
        input.fail_read = failure == 4;
        let selected = if failure == 0 {
            &foreign_program
        } else {
            &program
        };
        let hidden = if failure == 2 {
            vec!["WS".into()]
        } else {
            vec![]
        };
        let error = selected
            .predict_with_port_runtime_source(
                &mut scope,
                if failure == 1 { &foreign_id } else { &id },
                &hidden,
                if failure == 3 { "unknown" } else { "ports" },
                &mut input,
                &mut work,
                CompletionVerification::Legacy,
            )
            .unwrap_err();
        assert_eq!(
            error,
            match failure {
                0 => "Runtime prediction source belongs to another exact program",
                1 | 2 => "Runtime prediction source/hidden-policy context differs",
                3 => "Unknown parser prediction decision",
                _ => "Authored cancellation at actual input read",
            }
        );
        assert_eq!(input.events.borrow().is_empty(), failure != 4);
        assert!(scope.is_poisoned());
        assert!(
            ports_run(&program, &mut scope, &id, "x tail0", &mut work)
                .0
                .unwrap_err()
                .contains("poisoned")
        );
        let mut fresh = PortRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        assert_eq!(
            ports_run(&program, &mut fresh, &id, "x tail0", &mut work).0,
            Ok(Some(0))
        );
    }
}

#[test]
fn runtime_ports_cycles_effect_and_mixed_roots_and_actual_call128_errors_stay_literal() {
    let token = |text: &str| Edge::Token(PredictionSymbol::Keyword(text.into()), 3);
    let cases = [
        ports_program(
            vec![
                ports_state(vec![Edge::Epsilon(0), Edge::Epsilon(1)]),
                ports_state(vec![token("x")]),
                ports_state(vec![token("y")]),
                State {
                    accepting: true,
                    ..State::default()
                },
            ],
            vec![0, 2],
        ),
        ports_program(
            vec![
                ports_state(vec![Edge::Epsilon(1), token("x")]),
                ports_state(vec![token("x")]),
                ports_state(vec![token("y")]),
                State {
                    accepting: true,
                    ..State::default()
                },
            ],
            vec![0, 2],
        ),
        ports_program(
            vec![
                ports_state(vec![Edge::Call {
                    target: 0,
                    follow: 3,
                }]),
                ports_state(vec![]),
                ports_state(vec![token("x")]),
                State {
                    accepting: true,
                    ..State::default()
                },
            ],
            vec![0, 2],
        ),
    ];
    let id = runtime_source_id();
    for (index, program) in cases.iter().enumerate() {
        let old = ports_plain(program, "x", &mut Some(100_000));
        let mut work = Some(100_000);
        let mut scope = PortRuntimePredictionSource::new(program, &id, &[], &mut work).unwrap();
        let actual = ports_run(program, &mut scope, &id, "x", &mut work);
        assert_eq!(actual, old);
        if index == 2 {
            assert_eq!(
                actual.0,
                Err("Parser prediction call depth exceeded".into())
            );
            assert!(scope.is_poisoned());
        } else {
            assert_eq!(actual.0, Ok(Some(0)));
        }
    }
}

#[test]
fn runtime_ports_visible_and_hidden_first_full_boundaries_keep_actual_full_results_and_traces() {
    let id = runtime_source_id();
    for full_guard in [false, true] {
        let mut first = keyword("a");
        first.first_set_predicated = true;
        let mut first_grammar = grammar(choice(vec![first, keyword("b")]), vec![]);
        first_grammar.profile = Profile::NativeV67;
        let visible = if full_guard {
            full_oracle_program(false)
        } else {
            ParserPrediction::compile_with_first_predicates(
                &first_grammar,
                &["rule/Entry/a0".into()].into(),
            )
            .unwrap()
        };
        let hidden = runtime_source_hidden_follow_program(full_guard);
        for (program, path, texts) in [
            (
                &visible,
                "rule/Entry/choice",
                vec!["a b", "a c", "b", "unknown", ""],
            ),
            (
                &hidden,
                "rule/Prefix/repeat",
                vec![
                    "next",
                    "public next",
                    "after47",
                    "private next",
                    "unrelated",
                    "",
                ],
            ),
        ] {
            for text in texts {
                let tokens = symbols(text);
                let mut old_input = RuntimeSourceInput::new(&tokens);
                let expected =
                    program.predict_with_replay_input(path, &mut old_input, &mut Some(100_000));
                let mut work = Some(100_000);
                let mut scope =
                    PortRuntimePredictionSource::new(program, &id, &[], &mut work).unwrap();
                let mut input = RuntimeSourceInput::new(&tokens);
                let actual = program.predict_with_port_runtime_source(
                    &mut scope,
                    &id,
                    &[],
                    path,
                    &mut input,
                    &mut work,
                    CompletionVerification::Legacy,
                );
                println!(
                    "RUNTIME_PORTS_GUARD {}",
                    serde_json::json!({"full":full_guard,"path":path,"text":text,
                    "actual":actual,"expected":expected,"position":input.position,"furthest":input.furthest,
                    "trace":&*input.events.borrow(),"old_trace":&*old_input.events.borrow()})
                );
                assert_eq!(actual, expected);
                assert_eq!(
                    (input.position, input.furthest, input.events.into_inner()),
                    (
                        old_input.position,
                        old_input.furthest,
                        old_input.events.into_inner()
                    )
                );
                assert_eq!(scope.is_poisoned(), actual.is_err());
            }
        }
    }
}

#[test]
fn runtime_ports_explicit_macrostep_policy_exposes_earlier_dangling_error_before_guard_read() {
    // Old expanded FIFO executes the second entry's visible First before it
    // discovers the first entry's dangling pure tail. The explicit new policy
    // proves/scans that tail first. This is a REAL trace/furthest difference,
    // never hidden by a whole-error-string comparison or default-path claim.
    let mut program = ports_program(
        vec![
            ports_state(vec![Edge::Epsilon(2)]),
            ports_state(vec![Edge::First {
                path: "guard".into(),
                target: 3,
            }]),
            ports_state(vec![Edge::Epsilon(usize::MAX)]),
            ports_state(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 4)]),
            State {
                accepting: true,
                ..State::default()
            },
        ],
        vec![0, 1],
    );
    program.first_guards.insert(
        "guard".into(),
        BTreeSet::from([PredictionSymbol::Keyword("x".into())]),
    );
    let old = ports_plain(&program, "x", &mut Some(100_000));
    let mut work = Some(100_000);
    let id = runtime_source_id();
    let mut scope = PortRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
    let actual = ports_run(&program, &mut scope, &id, "x", &mut work);
    println!(
        "RUNTIME_PORTS_ORDER_DIFFERENCE {}",
        serde_json::json!({"old":old,"selected":actual})
    );
    assert_eq!(actual.0, Err("Dangling parser prediction state".into()));
    assert_eq!(old.0, actual.0);
    assert_eq!(
        (old.1, old.2, old.3),
        (1, 1, vec![RuntimeSourceEvent::Read(0)])
    );
    assert_eq!((actual.1, actual.2, actual.3), (0, 0, vec![]));
    assert!(scope.is_poisoned());
}

#[test]
fn runtime_ports_default_source_hooks_are_disabled_zero_work_and_old_holders_program_exact() {
    const {
        assert!(
            !<super::runtime_source::NoInitialSource as super::runtime_source::InitialSource<
                'static,
            >>::PORTS
        )
    };
    let program = ports_dag_program(4, 3);
    let bytes = serde_json::to_vec(&program).unwrap();
    let mut plain = Some(100_000);
    let expected = ports_plain(&program, "x tail3", &mut plain);
    let mut repeat = Some(100_000);
    let actual = ports_plain(&program, "x tail3", &mut repeat);
    assert_eq!((actual, repeat), (expected, plain));
    assert_eq!(serde_json::to_vec(&program).unwrap(), bytes);
    assert_eq!(
        std::mem::size_of::<super::runtime_source::NoInitialSource>(),
        0
    );
    let mut noop = super::runtime_source::NoInitialSource;
    let row = Configuration {
        state: usize::MAX,
        returns: vec![usize::MAX],
        alternative: 0,
        visible: true,
        guarded: false,
        full_guard: false,
    };
    let mut pending = VecDeque::new();
    let mut denied = Some(0);
    assert!(
        !super::runtime_source::InitialSource::expand_ports(
            &mut noop,
            &row,
            &mut pending,
            &mut denied
        )
        .unwrap()
    );
    assert_eq!(denied, Some(0));
    assert!(pending.is_empty());
}
