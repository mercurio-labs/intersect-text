// Component controls over the shared graph builder and real runtime evaluator.
// This authored graph has the same optional enum-call / broad caller-FOLLOW
// shape as MemberPrefix. It is not an original KerML request or fit witness.
fn runtime_source_program() -> ParserPrediction {
    let mut prefix = call("Visibility");
    prefix.cardinality = '?';
    let mut g = grammar(
        sequence(vec![call("Prefix"), keyword("next")]),
        vec![
            ("Prefix", prefix),
            (
                "Visibility",
                choice(vec![
                    keyword("public"),
                    keyword("private"),
                    keyword("protected"),
                ]),
            ),
        ],
    );
    let template = g.rules["Entry"].clone();
    for index in 0..48 {
        let mut rule = template.clone();
        rule.body = sequence(vec![call("Prefix"), keyword(&format!("after{index}"))]);
        g.rules.insert(format!("Caller{index:02}"), rule);
    }
    g.profile = Profile::NativeV67;
    ParserPrediction::compile(&g).unwrap()
}

#[derive(serde::Serialize, Debug, PartialEq, Eq)]
enum RuntimeSourceEvent {
    Read(usize),
    Checkpoint(usize),
    Rewind(usize, usize),
}
struct RuntimeSourceInput<'a> {
    tokens: &'a [PredictionSymbol],
    position: usize,
    furthest: usize,
    events: std::cell::RefCell<Vec<RuntimeSourceEvent>>,
    fail_read: bool,
}
impl<'a> RuntimeSourceInput<'a> {
    fn new(tokens: &'a [PredictionSymbol]) -> Self {
        Self {
            tokens,
            position: 0,
            furthest: 0,
            events: std::cell::RefCell::new(Vec::new()),
            fail_read: false,
        }
    }
}
impl PredictionInput for RuntimeSourceInput<'_> {
    fn replay_position(&self) -> Option<usize> {
        Some(self.position)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        self.events
            .borrow_mut()
            .push(RuntimeSourceEvent::Checkpoint(self.position));
        Ok(self.position)
    }
    fn rewind(&mut self, position: usize) -> Result<(), String> {
        self.events
            .borrow_mut()
            .push(RuntimeSourceEvent::Rewind(self.position, position));
        if position > self.tokens.len() {
            return Err("Test rewind escaped tokens".into());
        }
        self.position = position;
        Ok(())
    }
    fn read(&mut self, remaining: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
        self.events
            .borrow_mut()
            .push(RuntimeSourceEvent::Read(self.position));
        charge(remaining)?; // the real adapter owns retained-stream read fees
        if self.fail_read {
            return Err("Authored cancellation at actual input read".into());
        }
        let value = self.tokens.get(self.position).cloned();
        if value.is_some() {
            self.position += 1;
        }
        self.furthest = self.furthest.max(self.position);
        Ok(value)
    }
}
fn runtime_source_id() -> intersect_model::definitions::bundle::SourceId {
    intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:runtime-source.input",
        intersect_model::definitions::bundle::SourceKind::Xtext,
        "public next",
    )
    .id
}
fn runtime_source_read<'program>(
    program: &'program ParserPrediction,
    source: &mut RuntimePredictionSource<'program>,
    id: &intersect_model::definitions::bundle::SourceId,
    text: &str,
    work: &mut Option<usize>,
) -> (
    Result<Option<usize>, String>,
    usize,
    Vec<RuntimeSourceEvent>,
) {
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let value = program.predict_with_runtime_source(
        source,
        id,
        &[],
        "rule/Prefix/repeat",
        &mut input,
        work,
        CompletionVerification::Legacy,
    );
    (value, input.furthest, input.events.into_inner())
}

#[test]
fn runtime_source_optional_enum_cold_warm_match_old_result_token_reads_and_furthest() {
    let program = runtime_source_program();
    let before = serde_json::to_vec(&program).unwrap();
    let id = runtime_source_id();
    for text in [
        "next",
        "public next",
        "private next",
        "protected next",
        "unrelated",
        "",
    ] {
        let tokens = symbols(text);
        let mut input = RuntimeSourceInput::new(&tokens);
        let mut old_work = Some(100_000);
        let old =
            program.predict_with_replay_input("rule/Prefix/repeat", &mut input, &mut old_work);
        let old = (old, input.furthest, input.events.into_inner());
        let mut work = Some(100_000);
        let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        let cold_start = work.unwrap();
        let cold = runtime_source_read(&program, &mut source, &id, text, &mut work);
        assert_eq!(cold, old, "cold {text:?}");
        let cold_work = cold_start - work.unwrap();
        let warm_start = work.unwrap();
        let warm = runtime_source_read(&program, &mut source, &id, text, &mut work);
        assert_eq!(warm, old, "warm {text:?}");
        let warm_work = warm_start - work.unwrap();
        assert_eq!(source.publications(), 1);
        assert_eq!(source.hits(), 1);
        assert!(
            warm_work < 100_000 - old_work.unwrap(),
            "no assumed actual KerML saving"
        );
        println!(
            "RUNTIME_SOURCE_COMPONENT {}",
            serde_json::json!({
            "text":text, "old_work":100_000-old_work.unwrap(), "cold_operation_work":cold_work,
            "warm_operation_work":warm_work, "logical_cumulative":source.logical_bytes(),
            "furthest":warm.1, "scope":"authored optional-enum graph, not actual KerML"})
        );
    }
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}

fn runtime_source_sequence(
    program: &ParserPrediction,
    allowance: usize,
    preconsumed: usize,
) -> (Result<(), String>, usize, bool, usize, usize) {
    let id = runtime_source_id();
    let mut work = Some(allowance);
    for _ in 0..preconsumed {
        charge(&mut work).unwrap();
    }
    let mut source = match RuntimePredictionSource::new(program, &id, &[], &mut work) {
        Ok(source) => source,
        Err(error) => return (Err(error), allowance - work.unwrap(), true, 0, 0),
    };
    let result = (|| {
        runtime_source_read(program, &mut source, &id, "next", &mut work).0?;
        runtime_source_read(program, &mut source, &id, "next", &mut work).0?;
        Ok(())
    })();
    (
        result,
        allowance - work.unwrap(),
        source.is_poisoned(),
        source.publications(),
        source.hits(),
    )
}

#[test]
fn runtime_source_one_original_pool_zero_exact_short_preconsumed_poison_and_fresh_recovery() {
    let program = runtime_source_program();
    let (complete, used, poisoned, publications, hits) =
        runtime_source_sequence(&program, 100_000, 0);
    complete.unwrap();
    assert!(!poisoned);
    assert_eq!((publications, hits), (1, 1));
    for allowance in [0, used - 1, used] {
        let (result, charged, poisoned, _, _) = runtime_source_sequence(&program, allowance, 0);
        assert_eq!(charged, allowance);
        if allowance == used {
            result.unwrap();
            assert!(!poisoned);
        } else {
            assert!(result.unwrap_err().contains("budget"));
            assert!(poisoned);
        }
    }
    let (result, charged, poisoned, _, _) = runtime_source_sequence(&program, used + 7, 7);
    result.unwrap();
    assert_eq!(charged, used + 7);
    assert!(!poisoned);
    runtime_source_sequence(&program, used, 0).0.unwrap(); // separate cold invocation
    println!(
        "RUNTIME_SOURCE_CUMULATIVE exact={used} short={} preconsumed_total={}",
        used - 1,
        used + 7
    );
}

#[test]
fn runtime_source_foreign_program_source_hidden_context_and_late_input_errors_poison() {
    let program = runtime_source_program();
    let other = runtime_source_program();
    let id = runtime_source_id();
    let mut foreign = id.clone();
    foreign.uri.push_str("/foreign");
    for mismatch in 0..3 {
        let mut work = Some(100_000);
        let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        runtime_source_read(&program, &mut source, &id, "next", &mut work)
            .0
            .unwrap();
        let tokens = symbols("next");
        let mut input = RuntimeSourceInput::new(&tokens);
        let selected = if mismatch == 0 { &other } else { &program };
        let context = if mismatch == 1 { &foreign } else { &id };
        let policy = if mismatch == 2 {
            vec!["WS".into()]
        } else {
            vec![]
        };
        assert!(
            selected
                .predict_with_runtime_source(
                    &mut source,
                    context,
                    &policy,
                    "rule/Prefix/repeat",
                    &mut input,
                    &mut work,
                    CompletionVerification::Legacy
                )
                .is_err()
        );
        assert!(source.is_poisoned());
        assert_eq!(input.furthest, 0);
        assert!(
            runtime_source_read(&program, &mut source, &id, "next", &mut work)
                .0
                .is_err()
        );
    }
    let mut work = Some(100_000);
    let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
    runtime_source_read(&program, &mut source, &id, "next", &mut work)
        .0
        .unwrap();
    let tokens = symbols("next");
    let mut input = RuntimeSourceInput::new(&tokens);
    input.fail_read = true;
    assert_eq!(
        program.predict_with_runtime_source(
            &mut source,
            &id,
            &[],
            "rule/Prefix/repeat",
            &mut input,
            &mut work,
            CompletionVerification::Legacy
        ),
        Err("Authored cancellation at actual input read".into())
    );
    assert!(source.is_poisoned());
    assert_eq!(source.retained_rows(), 0);
}

#[test]
fn runtime_source_first_and_full_input_dependent_frontiers_remain_literal_and_uncached() {
    let id = runtime_source_id();
    for full_guard in [false, true] {
        let program = if full_guard {
            full_oracle_program(false)
        } else {
            let mut branch = keyword("a");
            branch.first_set_predicated = true;
            let mut g = grammar(choice(vec![branch, keyword("b")]), vec![]);
            g.profile = Profile::NativeV67;
            ParserPrediction::compile_with_first_predicates(&g, &["rule/Entry/a0".into()].into())
                .unwrap()
        };
        let path = "rule/Entry/choice";
        for text in ["a a b c", "b", "unknown"] {
            let tokens = symbols(text);
            let mut old = RuntimeSourceInput::new(&tokens);
            let expected = program.predict_with_replay_input(path, &mut old, &mut Some(100_000));
            let old_trace = (old.furthest, old.events.into_inner());
            let mut work = Some(100_000);
            let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
            let mut current = RuntimeSourceInput::new(&tokens);
            let observed = program.predict_with_runtime_source(
                &mut source,
                &id,
                &[],
                path,
                &mut current,
                &mut work,
                CompletionVerification::Legacy,
            );
            assert_eq!(observed, expected);
            assert_eq!((current.furthest, current.events.into_inner()), old_trace);
            assert_eq!(source.publications(), 0);
            assert_eq!(source.hits(), 0);
        }
    }
}

#[test]
fn runtime_source_dangling_and_depth_failures_keep_actual_old_error_without_publication() {
    let id = runtime_source_id();
    for depth in [false, true] {
        let mut program = runtime_source_program();
        let decision = &program.decisions["rule/Prefix/repeat"];
        let exit = decision.entries[1];
        if depth {
            program.states[exit].rule_stop = false;
            program.states[exit].edges = vec![Edge::Call {
                target: exit,
                follow: exit,
            }];
        } else {
            program.states[exit].follow.push(usize::MAX);
        }
        let tokens = symbols("next");
        let mut old = RuntimeSourceInput::new(&tokens);
        let expected =
            program.predict_with_replay_input("rule/Prefix/repeat", &mut old, &mut Some(100_000));
        assert!(expected.is_err());
        let mut work = Some(100_000);
        let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        let mut input = RuntimeSourceInput::new(&tokens);
        let observed = program.predict_with_runtime_source(
            &mut source,
            &id,
            &[],
            "rule/Prefix/repeat",
            &mut input,
            &mut work,
            CompletionVerification::Legacy,
        );
        assert_eq!(observed, expected);
        assert_eq!(input.furthest, old.furthest);
        assert_eq!(source.publications(), 0);
        assert!(source.is_poisoned());
    }
}

#[test]
fn runtime_source_fixed_logical_policy_and_multitoken_hit_match_literal_worker() {
    assert_eq!(
        RuntimePredictionSource::logical_policy(),
        [192, 32, 64, 8, 64, 160, 64]
    );
    let mut g = grammar(
        choice(vec![
            sequence(vec![keyword("a"), keyword("b")]),
            sequence(vec![keyword("a"), keyword("c")]),
        ]),
        vec![],
    );
    g.profile = Profile::NativeV67;
    let program = ParserPrediction::compile(&g).unwrap();
    let id = runtime_source_id();
    let mut work = Some(100_000);
    let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
    for (index, text) in ["a b", "a c", "a wrong", "wrong"].into_iter().enumerate() {
        let tokens = symbols(text);
        let mut old = RuntimeSourceInput::new(&tokens);
        let expected =
            program.predict_with_replay_input("rule/Entry/choice", &mut old, &mut Some(100_000));
        let mut input = RuntimeSourceInput::new(&tokens);
        let observed = program.predict_with_runtime_source(
            &mut source,
            &id,
            &[],
            "rule/Entry/choice",
            &mut input,
            &mut work,
            CompletionVerification::Legacy,
        );
        assert_eq!(observed, expected);
        assert_eq!(
            (input.furthest, input.events.into_inner()),
            (old.furthest, old.events.into_inner())
        );
        assert_eq!(source.publications(), 1);
        assert_eq!(source.hits(), index);
    }
}

// The old graph worker treats non-entry guards as epsilon transitions. These
// controls put real First/Full edges in the broad caller FOLLOW of Prefix;
// their presence must not be confused with an actually evaluated input guard.
fn runtime_source_hidden_follow_program(full_guard: bool) -> ParserPrediction {
    let mut prefix = call("Visibility");
    prefix.cardinality = '?';
    let guarded = |name: &str| {
        if full_guard {
            full(keyword(name))
        } else {
            keyword(name)
        }
    };
    let mut g = grammar(
        sequence(vec![call("Prefix"), guarded("next"), keyword("tail")]),
        vec![
            ("Prefix", prefix),
            (
                "Visibility",
                choice(vec![keyword("public"), keyword("private")]),
            ),
        ],
    );
    let template = g.rules["Entry"].clone();
    let mut predicates = BTreeSet::new();
    predicates.insert("rule/Entry/s1".into());
    for index in 0..48 {
        let name = format!("Caller{index:02}");
        let mut rule = template.clone();
        rule.body = sequence(vec![
            call("Prefix"),
            guarded(&format!("after{index}")),
            keyword("tail"),
        ]);
        predicates.insert(format!("rule/{name}/s1"));
        g.rules.insert(name, rule);
    }
    g.profile = Profile::NativeV68;
    if full_guard {
        ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).unwrap()
    } else {
        ParserPrediction::compile_with_first_predicates(&g, &predicates).unwrap()
    }
}

#[test]
fn runtime_source_hidden_follow_first_and_full_edges_reuse_same_worker_without_input_results() {
    let id = runtime_source_id();
    for full_guard in [false, true] {
        let program = runtime_source_hidden_follow_program(full_guard);
        let bytes = serde_json::to_vec(&program).unwrap();
        let mut work = Some(100_000);
        let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        // Different real token streams share only the same immutable graph
        // source. A previous recognition result must never select the next one.
        for (text, expected) in [
            ("next", None),
            ("public next", Some(0)),
            ("unrelated", None),
            ("after47", None),
            ("private next", Some(0)),
            ("", None),
        ] {
            let tokens = symbols(text);
            let mut old_input = RuntimeSourceInput::new(&tokens);
            let mut old_work = Some(100_000);
            let old = program.predict_with_replay_input(
                "rule/Prefix/repeat",
                &mut old_input,
                &mut old_work,
            );
            assert_eq!(old, Ok(expected));
            let before = work.unwrap();
            let actual = runtime_source_read(&program, &mut source, &id, text, &mut work);
            assert_eq!(
                actual,
                (old, old_input.furthest, old_input.events.into_inner()),
                "full={full_guard} text={text:?}"
            );
            if source.hits() > 0 {
                assert!(
                    before - work.unwrap() < 100_000 - old_work.unwrap(),
                    "only this authored broad-FOLLOW graph establishes a work saving"
                );
            }
            assert!(
                actual
                    .2
                    .iter()
                    .all(|e| matches!(e, RuntimeSourceEvent::Read(_))),
                "hidden predicates perform no checkpoint/probe in the original worker"
            );
        }
        assert_eq!(source.publications(), 1);
        assert_eq!(source.hits(), 5);
        assert!(!source.is_poisoned());
        assert_eq!(serde_json::to_vec(&program).unwrap(), bytes);
    }
}

#[test]
fn runtime_source_visible_first_and_full_stay_literal_across_changed_inputs() {
    let id = runtime_source_id();
    for full_guard in [false, true] {
        let program = if full_guard {
            full_oracle_program(false)
        } else {
            let mut branch = keyword("a");
            branch.first_set_predicated = true;
            let mut g = grammar(choice(vec![branch, keyword("b")]), vec![]);
            g.profile = Profile::NativeV67;
            ParserPrediction::compile_with_first_predicates(&g, &["rule/Entry/a0".into()].into())
                .unwrap()
        };
        let mut work = Some(100_000);
        let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        for text in ["a b", "a c", "a a b c", "b", "unknown", "a b"] {
            let tokens = symbols(text);
            let mut old = RuntimeSourceInput::new(&tokens);
            let expected = program.predict_with_replay_input(
                "rule/Entry/choice",
                &mut old,
                &mut Some(100_000),
            );
            assert!(expected.is_ok());
            let mut actual = RuntimeSourceInput::new(&tokens);
            let observed = program.predict_with_runtime_source(
                &mut source,
                &id,
                &[],
                "rule/Entry/choice",
                &mut actual,
                &mut work,
                CompletionVerification::Legacy,
            );
            assert_eq!(observed, expected);
            assert_eq!(
                (actual.furthest, actual.events.into_inner()),
                (old.furthest, old.events.into_inner())
            );
            assert_eq!(source.publications(), 0);
            assert_eq!(source.hits(), 0);
            assert!(!source.is_poisoned());
        }
    }
}

#[test]
fn runtime_source_hidden_guard_reuse_keeps_exact_short_preconsumed_poison_and_input_failure() {
    for full_guard in [false, true] {
        let program = runtime_source_hidden_follow_program(full_guard);
        let (result, used, poisoned, publications, hits) =
            runtime_source_sequence(&program, 100_000, 0);
        assert!(result.is_ok());
        assert!(!poisoned);
        assert_eq!((publications, hits), (1, 1));
        for allowance in [0, 1, used - 1] {
            let (result, charged, poisoned, _, _) = runtime_source_sequence(&program, allowance, 0);
            assert!(result.is_err());
            assert_eq!(charged, allowance);
            assert!(poisoned);
        }
        assert!(runtime_source_sequence(&program, used, 0).0.is_ok());
        assert!(runtime_source_sequence(&program, used + 7, 7).0.is_ok());
        let id = runtime_source_id();
        let mut work = Some(100_000);
        let mut source = RuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        runtime_source_read(&program, &mut source, &id, "next", &mut work)
            .0
            .unwrap();
        let tokens = symbols("next");
        let mut input = RuntimeSourceInput::new(&tokens);
        input.fail_read = true;
        let error = program
            .predict_with_runtime_source(
                &mut source,
                &id,
                &[],
                "rule/Prefix/repeat",
                &mut input,
                &mut work,
                CompletionVerification::Legacy,
            )
            .unwrap_err();
        assert!(error.contains("Authored cancellation"));
        assert!(source.is_poisoned());
        assert_eq!(source.retained_rows(), 0);
        assert!(
            input
                .events
                .into_inner()
                .iter()
                .any(|e| matches!(e, RuntimeSourceEvent::Read(0)))
        );
        assert!(runtime_source_sequence(&program, used, 0).0.is_ok()); // fresh scope recovers
    }
}

// The new sibling is deliberately not installed in any host or ParseContext.
// These controls use actual canonical programs and the same finite pool/input.
type RepeatedRuntimeRead = (
    Result<Option<usize>, String>,
    usize,
    Vec<RuntimeSourceEvent>,
);
fn runtime_repeated_read<'program>(
    program: &'program ParserPrediction,
    source: &mut RepeatedRuntimePredictionSource<'program>,
    id: &intersect_model::definitions::bundle::SourceId,
    text: &str,
    remaining: &mut Option<usize>,
) -> RepeatedRuntimeRead {
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let result = program.predict_with_repeated_runtime_source(
        source,
        id,
        &[],
        "rule/Prefix/repeat",
        &mut input,
        remaining,
        CompletionVerification::Legacy,
    );
    (result, input.furthest, input.events.into_inner())
}

#[test]
fn runtime_repeated_admission_first_second_warm_whole_worker_inputs_and_oneoff_utility() {
    let program = runtime_source_program();
    let bytes = serde_json::to_vec(&program).unwrap();
    let id = runtime_source_id();
    let mut work = Some(100_000);
    let mut source = RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
    let new_setup = 100_000 - work.unwrap();
    let mut immediate_work = Some(100_000);
    let mut immediate =
        RuntimePredictionSource::new(&program, &id, &[], &mut immediate_work).unwrap();
    assert_eq!(new_setup, 100_000 - immediate_work.unwrap() + 2);
    let start = immediate_work.unwrap();
    runtime_source_read(&program, &mut immediate, &id, "next", &mut immediate_work)
        .0
        .unwrap();
    let immediate_cold = start - immediate_work.unwrap();
    let mut fees = Vec::new();
    for (index, text) in [
        "next",
        "public next",
        "private next",
        "unrelated",
        "",
        "protected next",
    ]
    .into_iter()
    .enumerate()
    {
        let tokens = symbols(text);
        let mut old_input = RuntimeSourceInput::new(&tokens);
        let mut old_work = Some(100_000);
        let expected =
            program.predict_with_replay_input("rule/Prefix/repeat", &mut old_input, &mut old_work);
        let before = work.unwrap();
        let actual = runtime_repeated_read(&program, &mut source, &id, text, &mut work);
        let used = before - work.unwrap();
        assert_eq!(
            actual,
            (expected, old_input.furthest, old_input.events.into_inner())
        );
        assert_eq!(source.retained_keys(), 1);
        assert_eq!(source.publications(), usize::from(index > 0));
        assert_eq!(source.hits(), index.saturating_sub(1));
        if index == 0 {
            assert_eq!(used, 100_000 - old_work.unwrap() + 9);
            assert_eq!(source.retained_rows(), 0);
            assert!(
                used < immediate_cold,
                "utility is required only for this authored broad-FOLLOW graph"
            );
            assert!(source.logical_bytes() < immediate.logical_bytes());
        }
        fees.push(used);
    }
    assert!(!source.is_poisoned());
    assert_eq!(serde_json::to_vec(&program).unwrap(), bytes);
    println!(
        "REPEATED_ADMISSION_COMPONENT {}",
        serde_json::json!({
        "setup":new_setup,"per_call":fees,"immediate_cold":immediate_cold,
        "logical_cumulative":source.logical_bytes(),"publications":source.publications(),"hits":source.hits(),
        "scope":"authored broad-FOLLOW graph only; no original K01 saving assumed"})
    );
}

fn runtime_repeated_sequence(
    program: &ParserPrediction,
    allowance: usize,
    preconsumed: usize,
) -> (Result<(), String>, usize, bool, usize, usize) {
    let id = runtime_source_id();
    let mut work = Some(allowance);
    for _ in 0..preconsumed {
        charge(&mut work).unwrap();
    }
    let mut source = match RepeatedRuntimePredictionSource::new(program, &id, &[], &mut work) {
        Ok(value) => value,
        Err(error) => return (Err(error), allowance - work.unwrap(), true, 0, 0),
    };
    let result = (|| {
        runtime_repeated_read(program, &mut source, &id, "next", &mut work).0?;
        runtime_repeated_read(program, &mut source, &id, "next", &mut work).0?;
        runtime_repeated_read(program, &mut source, &id, "next", &mut work).0?;
        Ok(())
    })();
    (
        result,
        allowance - work.unwrap(),
        source.is_poisoned(),
        source.publications(),
        source.hits(),
    )
}

#[test]
fn runtime_repeated_admission_same_pool_zero_exact_short_preconsumed_and_fresh_recovery() {
    let program = runtime_source_program();
    let (result, used, poisoned, publications, hits) =
        runtime_repeated_sequence(&program, 100_000, 0);
    result.unwrap();
    assert!(!poisoned);
    assert_eq!((publications, hits), (1, 1));
    for allowance in [0, 1, used - 1, used] {
        let (actual, charged, poisoned, _, _) = runtime_repeated_sequence(&program, allowance, 0);
        assert_eq!(charged, allowance);
        if allowance == used {
            actual.unwrap();
            assert!(!poisoned);
        } else {
            assert!(actual.unwrap_err().contains("budget"));
            assert!(poisoned);
        }
    }
    let (actual, charged, poisoned, _, _) = runtime_repeated_sequence(&program, used + 7, 7);
    actual.unwrap();
    assert_eq!(charged, used + 7);
    assert!(!poisoned);
    runtime_repeated_sequence(&program, used, 0).0.unwrap();
    println!(
        "REPEATED_ADMISSION_EXACT {used} short={} preconsumed={}",
        used - 1,
        used + 7
    );
}

#[test]
fn runtime_repeated_admission_foreign_authorities_and_actual_read_cancellation_clear_both_tables() {
    let program = runtime_source_program();
    let other = runtime_source_program();
    let id = runtime_source_id();
    let mut foreign = id.clone();
    foreign.sha256.push('0');
    for mismatch in 0..5 {
        let mut work = Some(100_000);
        let mut source =
            RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        for _ in 0..2 {
            runtime_repeated_read(&program, &mut source, &id, "next", &mut work)
                .0
                .unwrap();
        }
        assert_eq!(source.retained_rows(), 1);
        let tokens = symbols("next");
        let mut input = RuntimeSourceInput::new(&tokens);
        input.fail_read = mismatch == 4;
        let selected = if mismatch == 0 { &other } else { &program };
        let actual_id = if mismatch == 1 { &foreign } else { &id };
        let hidden = if mismatch == 2 {
            vec!["WS".into()]
        } else {
            vec![]
        };
        let path = if mismatch == 3 {
            "rule/not-a-decision"
        } else {
            "rule/Prefix/repeat"
        };
        let error = selected
            .predict_with_repeated_runtime_source(
                &mut source,
                actual_id,
                &hidden,
                path,
                &mut input,
                &mut work,
                CompletionVerification::Legacy,
            )
            .unwrap_err();
        let expected = match mismatch {
            0 => "Runtime prediction source belongs to another exact program",
            1 | 2 => "Runtime prediction source/hidden-policy context differs",
            3 => "Unknown parser prediction decision",
            _ => "Authored cancellation at actual input read",
        };
        assert_eq!(error, expected);
        if mismatch != 4 {
            assert!(input.events.borrow().is_empty());
        } else {
            assert!(input.events.borrow().contains(&RuntimeSourceEvent::Read(0)));
        }
        assert!(source.is_poisoned());
        assert_eq!((source.retained_keys(), source.retained_rows()), (0, 0));
        assert!(
            runtime_repeated_read(&program, &mut source, &id, "next", &mut work)
                .0
                .unwrap_err()
                .contains("poisoned")
        );
    }
}

#[test]
fn runtime_repeated_admission_visible_guards_stay_literal_and_hidden_guards_replay_changed_inputs()
{
    let id = runtime_source_id();
    for full_guard in [false, true] {
        let hidden_program = runtime_source_hidden_follow_program(full_guard);
        let mut hidden_work = Some(100_000);
        let mut hidden_source =
            RepeatedRuntimePredictionSource::new(&hidden_program, &id, &[], &mut hidden_work)
                .unwrap();
        for (index, text) in [
            "next",
            "public next",
            "after47",
            "private next",
            "unrelated",
            "",
        ]
        .into_iter()
        .enumerate()
        {
            let tokens = symbols(text);
            let mut expected_input = RuntimeSourceInput::new(&tokens);
            let expected = hidden_program.predict_with_replay_input(
                "rule/Prefix/repeat",
                &mut expected_input,
                &mut Some(100_000),
            );
            let actual = runtime_repeated_read(
                &hidden_program,
                &mut hidden_source,
                &id,
                text,
                &mut hidden_work,
            );
            assert_eq!(
                actual,
                (
                    expected,
                    expected_input.furthest,
                    expected_input.events.into_inner()
                )
            );
            assert_eq!(hidden_source.publications(), usize::from(index > 0));
            assert_eq!(hidden_source.hits(), index.saturating_sub(1));
        }
        let visible_program = if full_guard {
            full_oracle_program(false)
        } else {
            let mut first = keyword("a");
            first.first_set_predicated = true;
            let mut g = grammar(choice(vec![first, keyword("b")]), vec![]);
            g.profile = Profile::NativeV67;
            ParserPrediction::compile_with_first_predicates(&g, &["rule/Entry/a0".into()].into())
                .unwrap()
        };
        let mut work = Some(100_000);
        let mut source =
            RepeatedRuntimePredictionSource::new(&visible_program, &id, &[], &mut work).unwrap();
        for text in ["a b", "a c", "a a b c", "b", "unknown", "a b"] {
            let tokens = symbols(text);
            let mut expected_input = RuntimeSourceInput::new(&tokens);
            let expected = visible_program.predict_with_replay_input(
                "rule/Entry/choice",
                &mut expected_input,
                &mut Some(100_000),
            );
            let mut actual_input = RuntimeSourceInput::new(&tokens);
            let actual = visible_program.predict_with_repeated_runtime_source(
                &mut source,
                &id,
                &[],
                "rule/Entry/choice",
                &mut actual_input,
                &mut work,
                CompletionVerification::Legacy,
            );
            assert_eq!(actual, expected);
            assert_eq!(
                (actual_input.furthest, actual_input.events.into_inner()),
                (expected_input.furthest, expected_input.events.into_inner())
            );
            assert_eq!((source.publications(), source.hits()), (0, 0));
            assert!(!source.is_poisoned());
        }
    }
}

#[test]
fn runtime_repeated_admission_real_depth_errors_and_verification_keys_are_not_waived() {
    let id = runtime_source_id();
    for depth in [false, true] {
        let mut program = runtime_source_program();
        let exit = program.decisions["rule/Prefix/repeat"].entries[1];
        if depth {
            program.states[exit].rule_stop = false;
            program.states[exit].edges = vec![Edge::Call {
                target: exit,
                follow: exit,
            }];
        } else {
            program.states[exit].follow.push(usize::MAX);
        }
        let tokens = symbols("next");
        let mut old = RuntimeSourceInput::new(&tokens);
        let expected =
            program.predict_with_replay_input("rule/Prefix/repeat", &mut old, &mut Some(100_000));
        assert!(expected.is_err());
        let mut work = Some(100_000);
        let mut source =
            RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        let actual = runtime_repeated_read(&program, &mut source, &id, "next", &mut work);
        assert_eq!(actual, (expected, old.furthest, old.events.into_inner()));
        assert!(source.is_poisoned());
        assert_eq!(
            (source.publications(), source.hits(), source.retained_keys()),
            (0, 0, 0)
        );
    }
    let program = runtime_source_program();
    let mut work = Some(100_000);
    let mut source = RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
    for (index, verification) in [
        CompletionVerification::Legacy,
        CompletionVerification::GuardOverapproximation,
        CompletionVerification::BoundedGuardOverapproximation,
        CompletionVerification::InputCheckedCompletion,
    ]
    .into_iter()
    .enumerate()
    {
        for visit in 0..3 {
            let tokens = symbols("next");
            let mut input = RuntimeSourceInput::new(&tokens);
            let value = program.predict_with_repeated_runtime_source(
                &mut source,
                &id,
                &[],
                "rule/Prefix/repeat",
                &mut input,
                &mut work,
                verification,
            );
            assert_eq!(value, Ok(None));
            assert_eq!(source.retained_keys(), index + 1);
            assert_eq!(source.publications(), index + usize::from(visit > 0));
            assert_eq!(source.hits(), index + usize::from(visit == 2));
        }
    }
}

// Explicit borrowed replay is an authored core algorithm witness, not a K01
// fit or a claim that seven actual warm hits finish the original100000 request.
fn runtime_borrowed_read<'program>(
    program: &'program ParserPrediction,
    source: &mut RepeatedRuntimePredictionSource<'program>,
    id: &intersect_model::definitions::bundle::SourceId,
    text: &str,
    work: &mut Option<usize>,
) -> (
    Result<Option<usize>, String>,
    usize,
    Vec<RuntimeSourceEvent>,
) {
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let value = program.predict_with_borrowed_repeated_runtime_source(
        source,
        id,
        &[],
        "rule/Prefix/repeat",
        &mut input,
        work,
        CompletionVerification::Legacy,
    );
    (value, input.furthest, input.events.into_inner())
}

#[test]
fn runtime_borrowed_warm_replay_matches_whole_result_reads_and_furthest_without_full_copy() {
    let program = runtime_source_program();
    let unchanged = serde_json::to_vec(&program).unwrap();
    let id = runtime_source_id();
    for text in [
        "next",
        "public next",
        "private next",
        "protected next",
        "unrelated",
        "",
    ] {
        let mut owned_work = Some(100_000);
        let mut borrowed_work = Some(100_000);
        let mut owned =
            RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut owned_work).unwrap();
        let mut borrowed =
            RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut borrowed_work).unwrap();
        for visit in 0..3 {
            let a = owned_work.unwrap();
            let b = borrowed_work.unwrap();
            let old_logical = owned.logical_bytes();
            let new_logical = borrowed.logical_bytes();
            let x = runtime_repeated_read(&program, &mut owned, &id, text, &mut owned_work);
            let y = runtime_borrowed_read(&program, &mut borrowed, &id, text, &mut borrowed_work);
            println!(
                "BORROWED_RUNTIME_REPLAY_COMPONENT {}",
                serde_json::json!({
                "text":text,"visit":visit,"owned":x,"borrowed":y,
                "owned_work":a-owned_work.unwrap(),"borrowed_work":b-borrowed_work.unwrap(),
                "owned_logical_delta":owned.logical_bytes()-old_logical,
                "borrowed_logical_delta":borrowed.logical_bytes()-new_logical,
                "scope":"authored48caller graph; actual K01 utility unknown"})
            );
            assert_eq!(x, y);
            assert_eq!(
                (owned.publications(), owned.hits()),
                (borrowed.publications(), borrowed.hits())
            );
            if visit < 2 {
                assert_eq!(owned_work, borrowed_work);
                assert_eq!(owned.logical_bytes(), borrowed.logical_bytes());
            } else {
                assert!(b - borrowed_work.unwrap() < a - owned_work.unwrap());
                assert!(
                    borrowed.logical_bytes() - new_logical < owned.logical_bytes() - old_logical
                );
            }
        }
    }
    assert_eq!(serde_json::to_vec(&program).unwrap(), unchanged);
}

fn runtime_borrowed_sequence(
    program: &ParserPrediction,
    allowance: usize,
    prepaid: usize,
) -> (Result<(), String>, usize, bool, usize) {
    let mut work = Some(allowance);
    for _ in 0..prepaid {
        charge(&mut work).unwrap();
    }
    let id = runtime_source_id();
    let mut source = match RepeatedRuntimePredictionSource::new(program, &id, &[], &mut work) {
        Ok(source) => source,
        Err(error) => return (Err(error), allowance - work.unwrap(), true, 0),
    };
    let result = (|| {
        for _ in 0..3 {
            assert_eq!(
                runtime_borrowed_read(program, &mut source, &id, "next", &mut work).0?,
                None
            );
        }
        Ok(())
    })();
    (
        result,
        allowance - work.unwrap(),
        source.is_poisoned(),
        source.retained_rows(),
    )
}

#[test]
fn runtime_borrowed_original_pool_exact_short_prepaid_cancel_poison_and_fresh_recovery() {
    let program = runtime_source_program();
    let baseline = runtime_borrowed_sequence(&program, 100_000, 0);
    assert_eq!(baseline.0, Ok(()));
    let used = baseline.1;
    let exact = runtime_borrowed_sequence(&program, used, 0);
    assert_eq!(exact, baseline);
    for grant in [0, used - 1] {
        let failure = runtime_borrowed_sequence(&program, grant, 0);
        assert!(failure.0.unwrap_err().contains("budget"));
        assert_eq!(failure.1, grant);
        assert!(failure.2);
        assert_eq!(failure.3, 0);
    }
    let prepaid = runtime_borrowed_sequence(&program, used + 7, 7);
    assert_eq!(prepaid.0, Ok(()));
    assert_eq!(prepaid.1, used + 7);
    assert_eq!(runtime_borrowed_sequence(&program, 100_000, 0), baseline);
    let id = runtime_source_id();
    let mut work = Some(100_000);
    let mut source = RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
    for _ in 0..2 {
        runtime_borrowed_read(&program, &mut source, &id, "next", &mut work)
            .0
            .unwrap();
    }
    let tokens = symbols("next");
    let mut input = RuntimeSourceInput::new(&tokens);
    input.fail_read = true;
    let failure = program.predict_with_borrowed_repeated_runtime_source(
        &mut source,
        &id,
        &[],
        "rule/Prefix/repeat",
        &mut input,
        &mut work,
        CompletionVerification::Legacy,
    );
    assert_eq!(
        failure,
        Err("Authored cancellation at actual input read".into())
    );
    assert!(source.is_poisoned());
    assert_eq!((source.retained_rows(), source.retained_keys()), (0, 0));
    println!(
        "BORROWED_RUNTIME_ORIGINAL_POOL {}",
        serde_json::json!({
        "operation":used,"prepaid":7,"total":used+7,"actual_K01_fit":false})
    );
}

#[test]
fn runtime_borrowed_replay_keeps_foreign_program_source_hidden_and_real_guard_failures() {
    let program = runtime_source_program();
    let id = runtime_source_id();
    for mismatch in 0..3 {
        let mut work = Some(100_000);
        let mut source =
            RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        for _ in 0..2 {
            runtime_borrowed_read(&program, &mut source, &id, "next", &mut work)
                .0
                .unwrap();
        }
        let other = program.clone();
        let mut changed = id.clone();
        changed.sha256.push('0');
        let hidden = vec!["WS".to_owned()];
        let tokens = symbols("next");
        let mut input = RuntimeSourceInput::new(&tokens);
        let actual = if mismatch == 0 { &other } else { &program };
        let outcome = actual.predict_with_borrowed_repeated_runtime_source(
            &mut source,
            if mismatch == 1 { &changed } else { &id },
            if mismatch == 2 { &hidden } else { &[] },
            "rule/Prefix/repeat",
            &mut input,
            &mut work,
            CompletionVerification::Legacy,
        );
        assert_eq!(
            outcome,
            Err(if mismatch == 0 {
                "Runtime prediction source belongs to another exact program"
            } else {
                "Runtime prediction source/hidden-policy context differs"
            }
            .into())
        );
        assert!(input.events.borrow().is_empty());
        assert!(source.is_poisoned());
        assert_eq!((source.retained_rows(), source.retained_keys()), (0, 0));
    }
    // Exercise the actually warm borrowed route over the canonical hidden
    // First/Full control graph, not only a visible guard that cannot publish.
    for full_guard in [false, true] {
        let program = runtime_source_hidden_follow_program(full_guard);
        let unchanged = serde_json::to_vec(&program).unwrap();
        let mut work = Some(100_000);
        let mut source =
            RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        for (index, text) in [
            "next",
            "public next",
            "after47",
            "private next",
            "unrelated",
            "",
        ]
        .into_iter()
        .enumerate()
        {
            let tokens = symbols(text);
            let mut input = RuntimeSourceInput::new(&tokens);
            let expected = program.predict_with_replay_input(
                "rule/Prefix/repeat",
                &mut input,
                &mut Some(100_000),
            );
            let actual = runtime_borrowed_read(&program, &mut source, &id, text, &mut work);
            println!(
                "BORROWED_RUNTIME_HIDDEN_GUARD {}",
                serde_json::json!({
                "full":full_guard,"visit":index,"text":text,"actual":actual,
                "publications":source.publications(),"hits":source.hits()})
            );
            assert_eq!(
                actual,
                (expected, input.furthest, input.events.into_inner())
            );
            assert_eq!(source.publications(), usize::from(index > 0));
            assert_eq!(source.hits(), index.saturating_sub(1));
            assert!(!source.is_poisoned());
        }
        assert_eq!(serde_json::to_vec(&program).unwrap(), unchanged);
    }
    // Actual visible First/Full guards cannot publish/replay an input-independent
    // closure. Their real input/checkpoint/rewind trace stays literal.
    for full_guard in [false, true] {
        let program = if full_guard {
            full_oracle_program(false)
        } else {
            let mut first = keyword("a");
            first.first_set_predicated = true;
            let mut grammar = grammar(choice(vec![first, keyword("b")]), vec![]);
            grammar.profile = Profile::NativeV67;
            ParserPrediction::compile_with_first_predicates(
                &grammar,
                &["rule/Entry/a0".into()].into(),
            )
            .unwrap()
        };
        let mut work = Some(100_000);
        let mut source =
            RepeatedRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        for text in ["a b", "a c", "a a b c", "b", "unknown", "a b"] {
            let tokens = symbols(text);
            let mut a = RuntimeSourceInput::new(&tokens);
            let mut b = RuntimeSourceInput::new(&tokens);
            let x =
                program.predict_with_replay_input("rule/Entry/choice", &mut a, &mut Some(100_000));
            let y = program.predict_with_borrowed_repeated_runtime_source(
                &mut source,
                &id,
                &[],
                "rule/Entry/choice",
                &mut b,
                &mut work,
                CompletionVerification::Legacy,
            );
            assert_eq!(x, y);
            assert_eq!(
                (a.furthest, a.events.into_inner()),
                (b.furthest, b.events.into_inner())
            );
            assert_eq!((source.publications(), source.hits()), (0, 0));
        }
    }
}
