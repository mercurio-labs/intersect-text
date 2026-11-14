// Authored controls over the sole canonical builder/worker; no original admission claim.
struct NeutralMeter {
    left: std::cell::Cell<usize>,
    retained: std::cell::Cell<usize>,
    cancelled: std::cell::Cell<bool>,
    failed: std::cell::Cell<bool>,
    cancel_on_retain: std::cell::Cell<bool>,
    deny_retain: std::cell::Cell<bool>,
}
impl NeutralMeter {
    fn new(left: usize) -> std::rc::Rc<Self> {
        std::rc::Rc::new(Self {
            left: std::cell::Cell::new(left),
            retained: std::cell::Cell::new(0),
            cancelled: std::cell::Cell::new(false),
            failed: std::cell::Cell::new(false),
            cancel_on_retain: std::cell::Cell::new(false),
            deny_retain: std::cell::Cell::new(false),
        })
    }
    fn context(
        self: &std::rc::Rc<Self>,
    ) -> intersect_model::definitions::conversion::InvocationContext<'static> {
        intersect_model::definitions::conversion::InvocationContext::new(self.clone())
    }
}
impl intersect_model::definitions::conversion::InvocationObserver for NeutralMeter {
    fn before_invoke(&self, _: &intersect_model::definitions::conversion::ProviderId) -> Result<(), String> {
        Err("No semantic preparation invocation".into())
    }
    fn checkpoint(&self) -> Result<(), String> {
        if self.cancelled.get() || self.failed.get() {
            Err("Neutral preparation cancelled or failed".into())
        } else {
            Ok(())
        }
    }
    fn failed_result(&self, _: &str) {
        self.failed.set(true);
    }
    fn work(&self, units: usize) -> Result<(), String> {
        self.checkpoint()?;
        for _ in 0..units {
            let next = self
                .left
                .get()
                .checked_sub(1)
                .ok_or("Original neutral work exhausted")?;
            self.left.set(next);
        }
        Ok(())
    }
    fn retain_storage(&self, bytes: usize) -> Result<(), String> {
        self.checkpoint()?;
        if self.deny_retain.get() {
            return Err("Original neutral storage denied".into());
        }
        self.retained.set(
            self.retained
                .get()
                .checked_add(bytes)
                .ok_or("Neutral retained overflow")?,
        );
        if self.cancel_on_retain.get() {
            self.cancelled.set(true);
        }
        Ok(())
    }
}
fn neutral_fixture() -> Grammar {
    let mut g = shared_return_graph();
    let body = g.rules["Entry"].body.clone();
    let mut wrapper = g.rules["Entry"].clone();
    wrapper.body = body;
    g.rules.insert("Wrapper".into(), wrapper);
    g.rules.get_mut("Entry").unwrap().body = sequence(vec![call("Wrapper"), keyword("done")]);
    live_compile(&mut g);
    g
}
fn neutral_prepare<'g>(
    g: &'g Grammar,
    path: &str,
) -> Result<PreparedLiveRuntimePredictionContext<'g>, String> {
    let meter = NeutralMeter::new(100000);
    PreparedLiveRuntimePredictionContext::prepare_selected_observed(
        g,
        g.hidden.as_deref().unwrap_or(&[]),
        CompletionVerification::Legacy,
        [path],
        &mut Some(100000),
        1048576,
        &meter.context(),
    )
}
#[allow(clippy::too_many_arguments)] // Independent actual source/caller/input/budget/reference controls.
fn neutral_run<'g>(
    g: &'g Grammar,
    facts: Option<&PreparedLiveRuntimePredictionContext<'g>>,
    callers: &[&Expr],
    expression: &Expr,
    suffix: &str,
    text: &str,
    grant: usize,
    prepaid: usize,
    arena: bool,
    cancel: bool,
) -> (PortsResult, usize, bool) {
    let source = runtime_source_id();
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    input.fail_read = cancel;
    let mut work = Some(grant);
    for _ in 0..prepaid {
        charge(&mut work).unwrap();
    }
    let mut scope = match LiveCallerPrediction::new(g, &source, &mut work) {
        Ok(value) => value,
        Err(error) => return ((Err(error), 0, 0, vec![]), grant - work.unwrap(), true),
    };
    let result = match (facts, arena) {
        (Some(facts), true) => scope.predict_arena_returns_with_prepared(
            facts,
            g.hidden.as_deref().unwrap_or(&[]),
            &source,
            callers,
            expression,
            suffix,
            &mut input,
            &mut work,
        ),
        (Some(facts), false) => scope.predict_shared_returns_with_prepared(
            facts,
            g.hidden.as_deref().unwrap_or(&[]),
            &source,
            callers,
            expression,
            suffix,
            &mut input,
            &mut work,
        ),
        (None, true) => {
            scope.predict_arena_returns(&source, callers, expression, suffix, &mut input, &mut work)
        }
        (None, false) => scope
            .predict_shared_returns(&source, callers, expression, suffix, &mut input, &mut work),
    };
    (
        (
            result,
            input.position,
            input.furthest,
            input.events.into_inner(),
        ),
        grant - work.unwrap(),
        scope.is_poisoned(),
    )
}
#[test]
fn caller_neutral_preparation_real_work_storage_and_default_historical_body() -> Result<(), String>
{
    let g = neutral_fixture();
    let before = serde_json::to_vec(&g).map_err(|e| e.to_string())?;
    let path = "rule/Wrapper/choice";
    let meter = NeutralMeter::new(100000);
    let mut work = Some(100000);
    let old = PreparedRuntimePredictionContext::prepare_selected(
        &g,
        &[],
        CompletionVerification::Legacy,
        [path],
        &mut Some(100000),
        1048576,
    )?;
    let facts = PreparedLiveRuntimePredictionContext::prepare_selected_observed(
        &g,
        &[],
        CompletionVerification::Legacy,
        [path],
        &mut work,
        1048576,
        &meter.context(),
    )?;
    assert_eq!(
        (
            facts.selected_decisions(),
            facts.prepared_decisions(),
            facts.ineligible_decisions()
        ),
        (1, 1, 0)
    );
    assert_eq!(old.prepared_decisions(), facts.prepared_decisions());
    assert!(facts.prefix_depth() > 0 && facts.prefix_depth() <= 128);
    assert_eq!(100000 - work.unwrap(), 100000 - meter.left.get());
    assert_eq!(facts.logical_bytes(), meter.retained.get());
    assert_eq!(facts.logical_bytes(), old.logical_bytes() + 16);
    assert_eq!(serde_json::to_vec(&g).map_err(|e| e.to_string())?, before);
    println!(
        "CALLER_NEUTRAL_PREPARATION_RAW {}",
        serde_json::json!({"canonical_observed_work":100000-work.unwrap(),"logical_bytes":facts.logical_bytes(),"prefix_depth":facts.prefix_depth(),"prepared":1,"original_requests":0,"working_RSS_bound":false})
    );
    Ok(())
}
#[test]
fn caller_neutral_live_shared_arena_whole_results_callbacks_and_actual_tail() -> Result<(), String>
{
    let g = neutral_fixture();
    let facts = neutral_prepare(&g, "rule/Wrapper/choice")?;
    let Kind::Sequence(root) = &g.rules["Entry"].body.kind else {
        return Err("Source entry sequence".into());
    };
    for arena in [false, true] {
        for (text, cancel) in [
            ("word0 left", false),
            ("word79 right", false),
            ("word0 bad", false),
            ("", false),
            ("word0 left", true),
        ] {
            let old = neutral_run(
                &g,
                None,
                &[&root[0]],
                &g.rules["Wrapper"].body,
                "choice",
                text,
                100000,
                0,
                arena,
                cancel,
            );
            let actual = neutral_run(
                &g,
                Some(&facts),
                &[&root[0]],
                &g.rules["Wrapper"].body,
                "choice",
                text,
                100000,
                0,
                arena,
                cancel,
            );
            assert_eq!(actual.0, old.0, "{text} arena{arena}");
            assert_eq!(actual.2, old.2);
            println!(
                "CALLER_NEUTRAL_LIVE_WHOLE {}",
                serde_json::json!({"text":text,"arena":arena,"cold":old.0,"actual":actual.0,"cold_work":old.1,"prepared_runtime_work":actual.1,"preparation_separate":true,"nonempty_actual_caller_tail":true,"original_admission":false})
            );
            if text == "word0 left" && !cancel {
                let exact = neutral_run(
                    &g,
                    Some(&facts),
                    &[&root[0]],
                    &g.rules["Wrapper"].body,
                    "choice",
                    text,
                    actual.1,
                    0,
                    arena,
                    false,
                );
                assert_eq!(exact.0, actual.0);
                let short = neutral_run(
                    &g,
                    Some(&facts),
                    &[&root[0]],
                    &g.rules["Wrapper"].body,
                    "choice",
                    text,
                    actual.1 - 1,
                    0,
                    arena,
                    false,
                );
                assert!(short.0.0.is_err() && short.2);
                let prepaid = neutral_run(
                    &g,
                    Some(&facts),
                    &[&root[0]],
                    &g.rules["Wrapper"].body,
                    "choice",
                    text,
                    actual.1 + 7,
                    7,
                    arena,
                    false,
                );
                assert_eq!(prepaid.0, actual.0);
                assert_eq!(prepaid.1, actual.1 + 7);
            }
        }
    }
    Ok(())
}
#[test]
fn caller_neutral_unsafe_nullable_guards_actions_use_literal_cold_fallback() -> Result<(), String> {
    let mut cases = vec![(
        live_component_grammar(3),
        "rule/Prefix/repeat",
        "public next",
        true,
    )];
    for action in [false, true] {
        let mut full = expr(Kind::SyntacticPredicate(Box::new(sequence(vec![
            keyword("x"),
            keyword("y"),
        ]))));
        full.first_set_predicated = true;
        let left = if action {
            sequence(vec![expr(Kind::Action("A".into())), full])
        } else {
            full
        };
        let mut g = grammar(
            choice(vec![left, sequence(vec![keyword("x"), keyword("z")])]),
            vec![],
        );
        live_compile(&mut g);
        cases.push((g, "rule/Entry/choice", "x z", false));
    }
    for (g, path, text, nested) in cases {
        let facts = neutral_prepare(&g, path)?;
        assert_eq!(facts.prepared_decisions(), 0);
        assert_eq!(facts.ineligible_decisions(), 1);
        let (callers, expression, suffix) = if nested {
            (
                vec![live_component_call(&g)],
                &g.rules["Prefix"].body,
                "repeat",
            )
        } else {
            (vec![], &g.rules["Entry"].body, "choice")
        };
        for arena in [false, true] {
            let old = neutral_run(
                &g, None, &callers, expression, suffix, text, 100000, 0, arena, false,
            );
            let actual = neutral_run(
                &g,
                Some(&facts),
                &callers,
                expression,
                suffix,
                text,
                100000,
                0,
                arena,
                false,
            );
            assert_eq!(actual.0, old.0);
            assert_eq!(actual.2, old.2);
        }
    }
    Ok(())
}
#[test]
fn caller_neutral_observer_exact_short_cancel_storage_and_sticky_failure() -> Result<(), String> {
    let g = neutral_fixture();
    let path = "rule/Wrapper/choice";
    let meter = NeutralMeter::new(100000);
    let mut work = Some(100000);
    let facts = PreparedLiveRuntimePredictionContext::prepare_selected_observed(
        &g,
        &[],
        CompletionVerification::Legacy,
        [path],
        &mut work,
        1048576,
        &meter.context(),
    )?;
    let needed = 100000 - work.unwrap();
    let bytes = facts.logical_bytes();
    for allowance in [0, needed - 1, needed] {
        let meter = NeutralMeter::new(100000);
        let original = meter.context();
        let mut work = Some(allowance);
        let result = PreparedLiveRuntimePredictionContext::prepare_selected_observed(
            &g,
            &[],
            CompletionVerification::Legacy,
            [path],
            &mut work,
            bytes,
            &original,
        );
        assert_eq!(result.is_ok(), allowance == needed);
        assert_eq!(allowance - work.unwrap(), 100000 - meter.left.get());
        if result.is_err() {
            let prior = meter.left.get();
            assert!(
                PreparedLiveRuntimePredictionContext::prepare_selected_observed(
                    &g,
                    &[],
                    CompletionVerification::Legacy,
                    [path],
                    &mut Some(100000),
                    bytes,
                    &original
                )
                .is_err()
            );
            assert_eq!(meter.left.get(), prior);
        }
    }
    for (available, limit, cancel) in [
        (needed - 1, bytes, false),
        (100000, bytes - 1, false),
        (100000, bytes, true),
    ] {
        let meter = NeutralMeter::new(available);
        meter.cancelled.set(cancel);
        let original = meter.context();
        let mut work = Some(100000);
        assert!(
            PreparedLiveRuntimePredictionContext::prepare_selected_observed(
                &g,
                &[],
                CompletionVerification::Legacy,
                [path],
                &mut work,
                limit,
                &original
            )
            .is_err()
        );
        assert!(meter.failed.get());
        assert_eq!(100000 - work.unwrap(), available - meter.left.get());
    }
    for mut work in [None, Some(100001)] {
        let meter = NeutralMeter::new(100000);
        assert!(
            PreparedLiveRuntimePredictionContext::prepare_selected_observed(
                &g,
                &[],
                CompletionVerification::Legacy,
                [path],
                &mut work,
                bytes,
                &meter.context()
            )
            .is_err()
        );
        assert!(meter.failed.get());
        assert_eq!(meter.left.get(), 100000);
    }
    assert!(
        PreparedLiveRuntimePredictionContext::prepare_selected_observed(
            &g,
            &[],
            CompletionVerification::Legacy,
            [path],
            &mut Some(100000),
            bytes,
            &intersect_model::definitions::conversion::InvocationContext::default()
        )
        .is_err()
    );
    let denied = NeutralMeter::new(100000);
    denied.deny_retain.set(true);
    let error = PreparedLiveRuntimePredictionContext::prepare_selected_observed(
        &g,
        &[],
        CompletionVerification::Legacy,
        [path],
        &mut Some(100000),
        bytes,
        &denied.context(),
    )
    .err()
    .ok_or("Actual retention denial must fail")?;
    assert!(error.contains("Original neutral storage denied"));
    assert!(denied.failed.get());
    assert_eq!(denied.retained.get(), 0);
    for requested in [vec!["rule/unknown/choice"], vec![path, path], vec![]] {
        let meter = NeutralMeter::new(100000);
        assert!(
            PreparedLiveRuntimePredictionContext::prepare_selected_observed(
                &g,
                &[],
                CompletionVerification::Legacy,
                requested,
                &mut Some(100000),
                1048576,
                &meter.context()
            )
            .is_err()
        );
        assert!(meter.failed.get());
    }
    let cancel_retain = NeutralMeter::new(100000);
    cancel_retain.cancel_on_retain.set(true);
    let original = cancel_retain.context();
    assert!(
        PreparedLiveRuntimePredictionContext::prepare_selected_observed(
            &g,
            &[],
            CompletionVerification::Legacy,
            [path],
            &mut Some(100000),
            bytes,
            &original,
        )
        .is_err()
    );
    assert!(cancel_retain.cancelled.get() && cancel_retain.failed.get());
    assert!(cancel_retain.retained.get() > 0);
    let before_retry = cancel_retain.left.get();
    assert!(
        PreparedLiveRuntimePredictionContext::prepare_selected_observed(
            &g,
            &[],
            CompletionVerification::Legacy,
            [path],
            &mut Some(100000),
            bytes,
            &original,
        )
        .is_err()
    );
    assert_eq!(cancel_retain.left.get(), before_retry);
    let meter = NeutralMeter::new(100000);
    let original = meter.context();
    original.work(7)?;
    let mut work = Some(100000);
    charge(&mut work)?;
    let result = PreparedLiveRuntimePredictionContext::prepare_selected_observed(
        &g,
        &[],
        CompletionVerification::Legacy,
        [path],
        &mut work,
        bytes,
        &original,
    )?;
    assert_eq!(result.logical_bytes(), bytes);
    assert_eq!(meter.left.get(), 100000 - 7 - needed);
    assert_eq!(work, Some(100000 - 1 - needed));
    Ok(())
}
#[test]
fn caller_neutral_foreign_graph_source_expression_hidden_and_depth_are_not_authority()
-> Result<(), String> {
    let mut g = neutral_fixture();
    g.hidden = Some(vec!["WS".into()]);
    let hidden = g.hidden.as_deref().ok_or("Actual hidden policy")?;
    let facts = neutral_prepare(&g, "rule/Wrapper/choice")?;
    let foreign = g.clone();
    let source = runtime_source_id();
    let copied = g.rules["Wrapper"].body.clone();
    let foreign_hidden = hidden.to_vec();
    let wrong_meter = NeutralMeter::new(100000);
    let wrong_verification = PreparedLiveRuntimePredictionContext::prepare_selected_observed(
        &g,
        hidden,
        CompletionVerification::InputCheckedCompletion,
        ["rule/Wrapper/choice"],
        &mut Some(100000),
        1048576,
        &wrong_meter.context(),
    )?;
    for case in 0..5 {
        let active = if case == 0 { &foreign } else { &g };
        let Kind::Sequence(root) = &active.rules["Entry"].body.kind else {
            return Err("Actual entry".into());
        };
        let tokens = symbols("word0 left");
        let mut input = RuntimeSourceInput::new(&tokens);
        let mut work = Some(100000);
        let mut scope = LiveCallerPrediction::new(active, &source, &mut work)?;
        let other = intersect_model::definitions::bundle::SourceId::for_text("memory:foreign-input", "x");
        let result = scope.predict_arena_returns_with_prepared(
            if case == 4 {
                &wrong_verification
            } else {
                &facts
            },
            if case == 3 { &foreign_hidden } else { hidden },
            if case == 1 { &other } else { &source },
            &[&root[0]],
            if case == 2 {
                &copied
            } else {
                &active.rules["Wrapper"].body
            },
            "choice",
            &mut input,
            &mut work,
        );
        assert!(result.is_err() && scope.is_poisoned());
        assert!(input.events.borrow().is_empty());
    }
    let mut recursive = grammar(
        sequence(vec![call("A"), keyword("stop")]),
        vec![
            (
                "A",
                choice(vec![sequence(vec![keyword("r"), call("A")]), call("Leaf")]),
            ),
            ("Leaf", choice(vec![keyword("x"), keyword("y")])),
        ],
    );
    live_compile(&mut recursive);
    let facts = neutral_prepare(&recursive, "rule/A/choice")?;
    assert!(facts.prefix_depth() > 0);
    let Kind::Sequence(entry) = &recursive.rules["Entry"].body.kind else {
        return Err("Entry source".into());
    };
    let Kind::Alternative(alts) = &recursive.rules["A"].body.kind else {
        return Err("A source".into());
    };
    let Kind::Sequence(recur) = &alts[0].kind else {
        return Err("Recursive source".into());
    };
    for depth in [127, 128, 129] {
        let mut calls = vec![&entry[0]];
        calls.extend(std::iter::repeat_n(&recur[1], depth - 1));
        let old = neutral_run(
            &recursive,
            None,
            &calls,
            &recursive.rules["A"].body,
            "choice",
            "x stop",
            100000,
            0,
            true,
            false,
        );
        let actual = neutral_run(
            &recursive,
            Some(&facts),
            &calls,
            &recursive.rules["A"].body,
            "choice",
            "x stop",
            100000,
            0,
            true,
            false,
        );
        assert_eq!(actual.0, old.0);
        assert_eq!(actual.2, old.2);
        assert_eq!(actual.0.0.is_err(), depth >= 128);
    }
    Ok(())
}

#[test]
fn caller_neutral_ambiguous_actual_runtime_and_guard_probe_errors_are_not_cached_results()
-> Result<(), String> {
    let mut g = neutral_fixture();
    let program = g.parser_prediction.as_mut().ok_or("Actual program")?;
    let mut changed = 0;
    for state in &mut program.states {
        for edge in &mut state.edges {
            if let Edge::Token(PredictionSymbol::Keyword(text), _) = edge
                && text == "right" {
                    *text = "left".into();
                    changed += 1;
                }
        }
    }
    assert!(changed > 0);
    // This raw graph deliberately overlaps after its first token. It is a
    // cache/worker parity control, not evidence that native static admission
    // accepts an ambiguous grammar.
    let facts = neutral_prepare(&g, "rule/Wrapper/choice")?;
    let Kind::Sequence(root) = &g.rules["Entry"].body.kind else {
        return Err("Actual entry".into());
    };
    for arena in [false, true] {
        let old = neutral_run(
            &g,
            None,
            &[&root[0]],
            &g.rules["Wrapper"].body,
            "choice",
            "word0 left done",
            100000,
            0,
            arena,
            false,
        );
        let actual = neutral_run(
            &g,
            Some(&facts),
            &[&root[0]],
            &g.rules["Wrapper"].body,
            "choice",
            "word0 left done",
            100000,
            0,
            arena,
            false,
        );
        assert_eq!(actual.0, old.0);
        assert_eq!(actual.2, old.2);
        assert!(
            actual
                .0
                .0
                .as_ref()
                .is_err_and(|error| error.contains("Ambiguous")),
            "{:?}",
            actual.0
        );
    }
    let mut full = expr(Kind::SyntacticPredicate(Box::new(sequence(vec![
        keyword("x"),
        keyword("y"),
    ]))));
    full.first_set_predicated = true;
    let mut g = grammar(
        choice(vec![full, sequence(vec![keyword("x"), keyword("z")])]),
        vec![],
    );
    live_compile(&mut g);
    let facts = neutral_prepare(&g, "rule/Entry/choice")?;
    assert_eq!(facts.ineligible_decisions(), 1);
    for arena in [false, true] {
        for (text, cancel) in [
            ("x y", false),
            ("x z", false),
            ("x q", false),
            ("x", false),
            ("", false),
            ("x y", true),
        ] {
            let old = neutral_run(
                &g,
                None,
                &[],
                &g.rules["Entry"].body,
                "choice",
                text,
                100000,
                0,
                arena,
                cancel,
            );
            let actual = neutral_run(
                &g,
                Some(&facts),
                &[],
                &g.rules["Entry"].body,
                "choice",
                text,
                100000,
                0,
                arena,
                cancel,
            );
            assert_eq!(actual.0, old.0);
            assert_eq!(actual.2, old.2);
            if cancel {
                assert!(actual.0.0.is_err() && actual.2);
            }
        }
    }
    Ok(())
}
