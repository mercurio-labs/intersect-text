// Genuine native source -> final recursive flags -> private owner -> ordinary Live.
fn live_choice_compilation() -> Result<crate::grammar::CompiledRuntimeGrammar, String>
{
    deferred_source_choice_text_compilation(&live_choice_source())
}
fn live_choice_source() -> String {
    // Both source alternatives have a genuine shared FIRST and recursive body.
    // Actual input must distinguish unique cases ({} / a a{}) from the real
    // overlapping case (a{}); no metadata bit is manufactured by the test.
    let body = "'a'? '{' (=> Item)* '}'";
    format!(
        "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {{t::Root}} '{{' (=> Tag)* '}}' Envelope 'done'; fragment Tag returns t::Root: 'x'; fragment Envelope returns t::Root: 'via' Item 'after'; fragment Item returns t::Root: 'send' (Body | (=> Prefix Body)); fragment Body returns t::Root: {body}; fragment Prefix returns t::Root: 'a';"
    )
}
fn live_choice_calls(grammar: &Grammar) -> Result<[&Expr; 2], String> {
    fn call_named<'a>(expr: &'a Expr, name: &str) -> Option<&'a Expr> {
        match &expr.kind {
            Kind::Call(actual) if actual == name => Some(expr),
            Kind::Sequence(items) | Kind::Alternative(items) => {
                items.iter().find_map(|e| call_named(e, name))
            }
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => call_named(value, name),
            _ => None,
        }
    }
    Ok([
        call_named(&grammar.rules["Root"].body, "Envelope").ok_or("No real Root/Envelope call")?,
        call_named(&grammar.rules["Envelope"].body, "Item").ok_or("No real Envelope/Item call")?,
    ])
}

#[test]
fn input_checked_live_choice_native_recursive_full_tail_and_eof() -> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let compilation = live_choice_compilation()?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No native program")?;
    let (path, expr) = deferred_source_choice(grammar, &mut 100_000)?;
    assert!(program.recursive_decisions.contains(path));
    assert!(admission.decision_is_runtime(path));
    assert_eq!(
        program.input_checked_completion_obligation_count(),
        admission.decision_count()
    );
    let before = serde_json::to_vec(grammar).map_err(|e| e.to_string())?;
    let calls = live_choice_calls(grammar)?;
    for (text, expected) in [
        ("{ } via send { } after done", Some(0)),
        ("{ } via send a a { } after done", Some(1)),
        ("{ } via send { send a a { } } after done", Some(0)),
        ("{ } via send { } wrong done", None),
        ("{ } via send a a { } after wrong", None),
        ("{ } via send { } after done trailing", None),
    ] {
        let source = DefinitionSource::new("memory:live-choice/input", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 4;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work);
        println!(
            "INPUT_CHECKED_LIVE_BOUNDARY text={text:?} result={result:?} remaining={work:?} events={:?}",
            input.events
        );
        assert_eq!(result?, expected);
        if expected.is_some() {
            assert_eq!(
                input.inner.position,
                tokens.len(),
                "must read actual suffix and EOF, not owner-local completion"
            );
        }
        assert!(work.is_some_and(|left| left > 0));
    }
    let text = "{ } via send a a { send { } } after done";
    let source = DefinitionSource::new(
        "memory:live-choice/representations",
        SourceKind::Xtext,
        text,
    );
    let tokens = symbols(text);
    let mut oracle = None;
    for representation in 0..3 {
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 4;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = match representation {
            0 => live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work),
            1 => live
                .predict_shared_returns(&source.id, &calls, expr, "choice", &mut input, &mut work),
            _ => live
                .predict_arena_returns(&source.id, &calls, expr, "choice", &mut input, &mut work),
        };
        assert_eq!(result, Ok(Some(1)));
        let events = input
            .events
            .iter()
            .map(|event| match event {
                TraceEvent::Read(pos, _) => format!("read:{pos}"),
                TraceEvent::Rewind(from, to) => format!("rewind:{from}:{to}"),
            })
            .collect::<Vec<_>>();
        let whole = (result, input.inner.position, events);
        if let Some(expected) = &oracle {
            assert_eq!(&whole, expected);
        } else {
            oracle = Some(whole);
        }
        assert!(work.is_some_and(|left| left > 0));
    }
    assert_eq!(
        serde_json::to_vec(grammar).map_err(|e| e.to_string())?,
        before
    );
    // A future caller suffix is outside Item's completion scope, but the
    // root-to-EOF LIVE traversal executes it. Its real parsed hidden override
    // must therefore prevent native publication of this new obligation.
    let source = live_choice_source();
    let suffix = source.replace("Envelope 'done';", "Envelope Tail 'done';")
        + " fragment Tail returns t::Root hidden(WS): 'tail'; terminal WS: ' ';";
    let error = deferred_source_choice_text_compilation(&suffix)
        .err()
        .ok_or("Reachable hidden override unexpectedly compiled")?;
    assert!(
        error.contains("Runtime choice reachable rule switches hidden policy: Tail"),
        "{error}"
    );
    assert!(!error.contains("budget"), "{error}");
    // The same source declaration is irrelevant when no actual source call
    // reaches it. This is not a whole-grammar hidden-policy allowlist.
    let unreachable =
        source + " fragment Tail returns t::Root hidden(WS): 'tail'; terminal WS: ' ';";
    let untouched = deferred_source_choice_text_compilation(&unreachable)?;
    let grammar = untouched.grammar();
    let admission = untouched.admission();
    assert_eq!(
        grammar.rules["Tail"].hidden.as_deref(),
        Some(["WS".to_owned()].as_slice())
    );
    let (path, expr) = deferred_source_choice(grammar, &mut 100_000)?;
    assert!(admission.decision_is_runtime(path));
    let calls = live_choice_calls(grammar)?;
    let source = DefinitionSource::new(
        "memory:live-choice/unreachable-hidden",
        SourceKind::Xtext,
        "{ } via send a a { } after done",
    );
    let tokens = symbols(&source.text);
    let mut input = TraceReplay::new(&tokens);
    input.inner.position = 4;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    assert_eq!(
        live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)?,
        Some(1)
    );
    assert_eq!(input.inner.position, tokens.len());
    assert!(work.is_some_and(|left| left > 0));
    Ok(())
}

#[test]
fn input_checked_live_choice_overlap_nested_guards_never_grants_outer_priority()
-> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let compilation = live_choice_compilation()?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let (path, expr) = deferred_source_choice(grammar, &mut 100_000)?;
    assert!(admission.decision_is_runtime(path));
    let calls = live_choice_calls(grammar)?;
    for text in [
        "{ } via send a { } after done",
        "{ } via send a { send { } } after done",
    ] {
        let source = DefinitionSource::new("memory:live-choice/overlap", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 4;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work);
        let message = result.expect_err("real complete overlap must reject");
        assert!(message.contains("Ambiguous"), "{message}");
        assert!(!message.contains("budget"));
        assert!(work.is_some_and(|left| left > 0));
        assert!(live.is_poisoned());
        assert!(
            input
                .events
                .iter()
                .any(|e| matches!(e, TraceEvent::Rewind(_, _))),
            "actual Full probes must have executed"
        );
    }
    Ok(())
}

#[test]
fn input_checked_live_choice_shared_pool_foreign_context_and_forged_boundary() -> Result<(), String>
{
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let compilation = live_choice_compilation()?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar.parser_prediction.as_ref().ok_or("No program")?;
    let (_, expr) = deferred_source_choice(grammar, &mut 100_000)?;
    // The old record lookup still costs exactly one; only the genuinely new
    // choices-map probe adds paid work. A short probe cannot return authority.
    let repeat = admission
        .decisions()
        .find(|p| p.ends_with("/repeat"))
        .ok_or("No old repeat obligation")?;
    let mut old = 1;
    assert!(admission.decision_is_runtime_with_remaining(repeat, &mut old)?);
    assert_eq!(old, 0);
    let (path, _) = deferred_source_choice(grammar, &mut 100_000)?;
    let mut generous = 1000;
    assert!(admission.decision_is_runtime_with_remaining(path, &mut generous)?);
    let used = 1000 - generous;
    assert!(used > 1);
    let mut exact = used;
    assert!(admission.decision_is_runtime_with_remaining(path, &mut exact)?);
    assert_eq!(exact, 0);
    let mut short = used - 1;
    assert!(
        admission
            .decision_is_runtime_with_remaining(path, &mut short)
            .is_err()
    );
    assert_eq!(short, 0);
    let mut zero = 0;
    assert!(
        admission
            .decision_is_runtime_with_remaining(path, &mut zero)
            .is_err()
    );
    let calls = live_choice_calls(grammar)?;
    let source = DefinitionSource::new(
        "memory:live-choice/budget",
        SourceKind::Xtext,
        "{ } via send a a { } after done",
    );
    let tokens = symbols(&source.text);
    let run = |limit, prepaid| -> Result<_, String> {
        let mut work = Some(limit);
        for _ in 0..prepaid {
            charge(&mut work)?;
        }
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 4;
        let result = (|| {
            let mut live = LiveCallerPrediction::new_runtime_guarded(
                grammar, &source.id, admission, &mut work,
            )?;
            live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)
        })();
        Ok((
            result,
            limit - work.ok_or("Finite pool disappeared")?,
            input.inner.position,
        ))
    };
    let full = run(1_000_000, 0)?;
    assert_eq!(full.0, Ok(Some(1)));
    let used = full.1;
    assert_eq!(run(used, 0)?, full);
    let pre = run(used + 9, 9)?;
    assert_eq!(pre.0, full.0);
    assert_eq!(pre.1, used + 9);
    assert_eq!(pre.2, full.2);
    for limit in [0, used - 1] {
        let short = run(limit, 0)?;
        assert!(short.0.is_err());
        assert_eq!(short.1, limit);
    }
    let mut work = Some(1_000_000);
    let mut input = ReplayTokens::new(&tokens);
    input.position = 4;
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    let foreign = DefinitionSource::new("memory:foreign/input", SourceKind::Xtext, &source.text);
    assert!(
        live.predict(&foreign.id, &calls, expr, "choice", &mut input, &mut work)
            .unwrap_err()
            .contains("source differs")
    );
    assert!(live.is_poisoned());
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    let copied = expr.clone();
    assert!(
        live.predict(&source.id, &calls, &copied, "choice", &mut input, &mut work)
            .unwrap_err()
            .contains("actual borrowed")
    );
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    assert!(
        live.predict(
            &source.id,
            &calls[..1],
            expr,
            "choice",
            &mut input,
            &mut work
        )
        .unwrap_err()
        .contains("owner differs")
    );
    struct Cancel<'a> {
        inner: TraceReplay<'a>,
        left: usize,
    }
    impl PredictionInput for Cancel<'_> {
        fn replay_position(&self) -> Option<usize> {
            self.inner.replay_position()
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.inner.checkpoint()
        }
        fn rewind(&mut self, position: usize) -> Result<(), String> {
            self.inner.rewind(position)
        }
        fn read(&mut self, work: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            if self.left == 0 {
                return Err("cancelled actual live choice input".into());
            }
            self.left -= 1;
            self.inner.read(work)
        }
    }
    let mut cancelled = Cancel {
        inner: TraceReplay::new(&tokens),
        left: 1,
    };
    cancelled.inner.inner.position = 4;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    let failure = live
        .predict(
            &source.id,
            &calls,
            expr,
            "choice",
            &mut cancelled,
            &mut work,
        )
        .unwrap_err();
    assert!(failure.contains("cancelled"), "{failure}");
    assert!(live.is_poisoned());
    assert!(work.is_some_and(|n| n > 0));
    let reads = cancelled.inner.events.len();
    assert!(
        live.predict(
            &source.id,
            &calls,
            expr,
            "choice",
            &mut cancelled,
            &mut work
        )
        .is_err()
    );
    assert_eq!(cancelled.inner.events.len(), reads);
    let cloned = grammar.clone();
    assert!(
        LiveCallerPrediction::new_runtime_guarded(
            &cloned,
            &source.id,
            admission,
            &mut Some(1_000_000)
        )
        .is_err()
    );
    assert!(
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut None)
            .is_err()
    );
    // The private ephemeral LIVE mode itself still rejects copied return-tail
    // descriptors and a forged EOF endpoint before reading input.
    let (_, _, expression) = super::super::nullable_repetition::prefix_source_occurrence_with_work(
        grammar,
        expr.prediction_path.as_deref().ok_or("No path")?,
        &mut 100_000,
    )?
    .ok_or("No actual choice")?;
    assert!(std::ptr::eq(expr, expression));
    let path = format!(
        "{}/choice",
        expr.prediction_path.as_deref().ok_or("No path")?
    );
    let decision = &program.decisions[&path];
    let root_stop = program.rules["Root"].1;
    let (eof, accepted) = program.states[root_stop]
        .follow
        .iter()
        .find_map(|&id| match program.states[id].edges.as_slice() {
            [Edge::Token(PredictionSymbol::End, to)] => Some((id, *to)),
            _ => None,
        })
        .ok_or("No actual EOF")?;
    let actual_tail = vec![1usize];
    let copied_tail = actual_tail.clone();
    let seed = live_callers::LiveSeed {
        scope: None,
        scopes: None,
        returns: &actual_tail,
        root_stop,
        eof,
    };
    for (bound_stop, tail) in [
        (accepted, copied_tail.as_slice()),
        (usize::MAX, actual_tail.as_slice()),
    ] {
        let goal = PredictionGoal::Decision {
            decision,
            recursive_first: true,
            boundary: Some(DecisionBoundary {
                live: true,
                stop: bound_stop,
                returns: tail,
            }),
        };
        let mut input = TraceReplay::new(&tokens);
        let mut work = Some(100_000);
        let result = program.run_prediction_with_live_seed(
            goal,
            &mut input,
            &mut work,
            0,
            CompletionVerification::InputCheckedCompletion,
            None,
            &mut runtime_source::NoInitialSource,
            Some(&seed),
        );
        assert!(result.is_err());
        assert!(input.events.is_empty());
    }
    Ok(())
}
