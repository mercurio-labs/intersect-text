// Qualification pins the already-existing realistic policy explicitly.
// Historical paired-control defaults and production caps remain unchanged.
fn multiway_source_compilation(
    text: &str,
) -> Result<crate::grammar::CompiledRuntimeGrammar, String> {
    deferred_source_choice_text_compilation_with_budget(
        text,
        crate::grammar::GrammarAnalysisWorkBudget::REALISTIC,
    )
}

// Handwritten controls over genuine source and the unchanged caller-aware worker.
fn live_multiway_source() -> String {
    live_choice_source().replace(
        "Body | (=> Prefix Body)",
        "Body | (-> Prefix Body) | (=> Other Body)",
    ) + " fragment Other returns t::Root: 'b';"
}

#[test]
fn input_checked_multiway_actual_source_mixed_guards_nested_tail_and_overlap() -> Result<(), String>
{
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let compilation = multiway_source_compilation(&live_multiway_source())?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No actual program")?;
    let (path, expr) = deferred_source_choice(grammar, &mut 100_000)?;
    assert_eq!(program.decisions[path].entries.len(), 3);
    assert!(
        !program.recursive_decisions.contains(path),
        "conditional obligation must not manufacture static recursive flags"
    );
    assert!(admission.decision_is_runtime(path));
    assert!(matches!(
        program.input_checked_source_choice(path, grammar, &mut 100_000)?,
        nullable_repeat::DeferredChoiceProof::RequiresRuntimeBoundary
    ));
    let (start, stop) = program.rules["Item"];
    for policy in [
        CompletionVerification::Legacy,
        CompletionVerification::BoundedGuardOverapproximation,
    ] {
        assert!(
            program
                .verify_completion_scope_with_verification(start, stop, &mut 1_000_000, policy)
                .is_err(),
            "conditional admission must not grant the prior static policy a proof"
        );
    }
    let actual_entries = &program.decisions[path].entries;
    assert!(
        actual_entries
            .iter()
            .any(|&i| matches!(program.states[i].edges.as_slice(), [Edge::First { .. }]))
    );
    assert!(
        actual_entries
            .iter()
            .any(|&i| matches!(program.states[i].edges.as_slice(), [Edge::Full { .. }]))
    );
    let calls = live_choice_calls(grammar)?;
    let frozen = serde_json::to_vec(grammar).map_err(|e| e.to_string())?;
    for (text, expected) in [
        ("{ } via send { } after done", Some(0)),
        ("{ } via send a a { } after done", Some(1)),
        ("{ } via send b { } after done", Some(2)),
        ("{ } via send { send b { } } after done", Some(0)),
        ("{ } via send b { } wrong done", None),
        ("{ } via send b { } after done trailing", None),
    ] {
        let source = DefinitionSource::new("memory:multiway/input", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 4;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        assert_eq!(
            live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)?,
            expected,
            "{text}"
        );
        if expected.is_some() {
            assert_eq!(input.inner.position, tokens.len());
        }
        assert!(work.is_some_and(|w| w > 0));
    }
    // Bounded passive adapter observations, never parser/source authority.
    struct CursorTrace<'a> {
        inner: TraceReplay<'a>,
        phase: &'static str,
        snapshots: Vec<(&'static str, usize, Option<usize>)>,
        moves: Vec<(&'static str, usize, usize)>,
        overflow: bool,
    }
    impl PredictionInput for CursorTrace<'_> {
        fn runtime_work_diagnostics(&self) -> bool {
            true
        }
        fn runtime_work_snapshot(&mut self, phase: &'static str, remaining: Option<usize>) {
            self.phase = phase;
            if matches!(
                phase,
                "outer" | "nested_decision" | "first_edge" | "cleanup" | "enter" | "exit"
            ) {
                if self.snapshots.len() < 512 {
                    self.snapshots
                        .push((phase, self.inner.inner.position, remaining));
                } else {
                    self.overflow = true;
                }
            }
        }
        fn replay_position(&self) -> Option<usize> {
            self.inner.replay_position()
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.inner.checkpoint()
        }
        fn rewind(&mut self, to: usize) -> Result<(), String> {
            let from = self.inner.inner.position;
            self.inner.rewind(to)?;
            if self.moves.len() < 512 {
                self.moves
                    .push((self.phase, from, self.inner.inner.position));
            } else {
                self.overflow = true;
            }
            Ok(())
        }
        fn read(&mut self, work: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            self.inner.read(work)
        }
    }
    for text in [
        "{ } via send a { } after done",
        "{ } via send { send a { } } after done",
    ] {
        let source = DefinitionSource::new("memory:multiway/overlap", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = CursorTrace {
            inner: TraceReplay::new(&tokens),
            phase: "created",
            snapshots: Vec::new(),
            moves: Vec::new(),
            overflow: false,
        };
        input.inner.inner.position = 4;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work);
        println!(
            "MULTIWAY_FRONTIER_CURSOR input={text:?} result={result:?} moves={:?} snapshots={:?}",
            input.moves, input.snapshots
        );
        let message = result.expect_err("actual overlap must fail");
        assert!(
            !input.overflow,
            "bounded cursor observations must be complete"
        );
        if text == "{ } via send a { } after done" {
            let nested=input.moves.iter().position(|&(phase,from,to)|phase=="nested_decision"&&from==5&&to==4)
                .ok_or("Nested optional probe never restored logical unconsumed a at4 from First-prefetched physical5")?;
            assert!(
                input.moves[nested + 1..]
                    .iter()
                    .any(|&(phase, _, to)| phase == "cleanup" && to == 5),
                "nested probe must restore parent's physical prefetch cursor5"
            );
        }
        assert!(message.contains("Ambiguous"), "{message}");
        assert!(!message.contains("budget"));
        assert!(live.is_poisoned());
    }
    assert_eq!(
        serde_json::to_vec(grammar).map_err(|e| e.to_string())?,
        frozen
    );
    // Every reachable owner, including the caller suffix beyond Item, retains
    // its original hidden policy; no extra unused declaration is authority.
    let switched = live_multiway_source().replace("Envelope 'done';", "Envelope Tail 'done';")
        + " fragment Tail returns t::Root hidden(WS): 'tail'; terminal WS: ' ';";
    let error = multiway_source_compilation(&switched)
        .err()
        .ok_or("foreign hidden policy admitted")?;
    assert!(error.contains("switches hidden policy: Tail"), "{error}");
    Ok(())
}

#[test]
fn input_checked_multiway_shared_short_foreign_and_cancel_refuse() -> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let compilation = multiway_source_compilation(&live_multiway_source())?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let (_, expr) = deferred_source_choice(grammar, &mut 100_000)?;
    let calls = live_choice_calls(grammar)?;
    let text = "{ } via send b { } after done";
    let source = DefinitionSource::new("memory:multiway/budget", SourceKind::Xtext, text);
    let tokens = symbols(text);
    let run = |limit, prepaid| -> Result<_, String> {
        let mut work = Some(limit);
        for _ in 0..prepaid {
            charge(&mut work)?;
        }
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 4;
        let outcome = (|| {
            let mut live = LiveCallerPrediction::new_runtime_guarded(
                grammar, &source.id, admission, &mut work,
            )?;
            live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)
        })();
        Ok((
            outcome,
            limit - work.ok_or("Lost finite pool")?,
            input.inner.position,
        ))
    };
    let full = run(1_000_000, 0)?;
    assert_eq!(full.0, Ok(Some(2)));
    let used = full.1;
    assert_eq!(run(used, 0)?, full);
    let prepaid = run(used + 9, 9)?;
    assert_eq!(prepaid.0, full.0);
    assert_eq!(prepaid.1, used + 9);
    for limit in [0, used - 1] {
        let short = run(limit, 0)?;
        assert!(short.0.is_err());
        assert_eq!(short.1, limit);
    }
    let mut input = TraceReplay::new(&tokens);
    input.inner.position = 4;
    let foreign = DefinitionSource::new("memory:foreign/multiway", SourceKind::Xtext, text);
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    assert!(
        live.predict(&foreign.id, &calls, expr, "choice", &mut input, &mut work)
            .expect_err("foreign input")
            .contains("source differs")
    );
    assert!(live.is_poisoned());
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
        .expect_err("foreign caller")
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
        fn rewind(&mut self, p: usize) -> Result<(), String> {
            self.inner.rewind(p)
        }
        fn read(&mut self, w: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            if self.left == 0 {
                return Err("cancelled actual multiway input".into());
            }
            self.left -= 1;
            self.inner.read(w)
        }
    }
    let mut input = Cancel {
        inner: TraceReplay::new(&tokens),
        left: 1,
    };
    input.inner.inner.position = 4;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    assert!(
        live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)
            .expect_err("cancellation")
            .contains("cancelled actual multiway")
    );
    assert!(live.is_poisoned());
    assert!(work.is_some_and(|w| w > 0));
    let reads = input.inner.events.len();
    assert!(
        live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)
            .is_err()
    );
    assert_eq!(input.inner.events.len(), reads);
    Ok(())
}

#[test]
fn input_checked_multiway_third_entry_and_source_boundary_are_rebound() -> Result<(), String> {
    let compilation = multiway_source_compilation(&live_multiway_source())?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar.parser_prediction.as_ref().ok_or("No program")?;
    let (path, _) = deferred_source_choice(grammar, &mut 100_000)?;
    let mut work = Some(1_000_000);
    assert_eq!(
        admission.live_choice_boundary(grammar, program, path, "Item", &mut work)?,
        Some(program.rules["Item"].1)
    );
    let used = 1_000_000 - work.ok_or("Lost finite pool")?;
    let mut exact = Some(used);
    assert!(
        admission
            .live_choice_boundary(grammar, program, path, "Item", &mut exact)?
            .is_some()
    );
    assert_eq!(exact, Some(0));
    let mut short = Some(used - 1);
    assert!(
        admission
            .live_choice_boundary(grammar, program, path, "Item", &mut short)
            .is_err()
    );
    assert_eq!(short, Some(0));
    let mut changed = program.clone();
    let entries = &mut changed
        .decisions
        .get_mut(path)
        .ok_or("No actual choice")?
        .entries;
    entries[2] = entries[0];
    let message = admission
        .live_choice_boundary(grammar, &changed, path, "Item", &mut Some(1_000_000))
        .expect_err("third ordered entry must bind");
    assert!(
        message.contains("repeats") || message.contains("endpoints") || message.contains("fanout"),
        "{message}"
    );
    let mut foreign = grammar.clone();
    let source = foreign
        .declaration_sources
        .get_mut("Item")
        .ok_or("No actual owner")?;
    source.end = source.start + 1;
    let message = admission
        .live_choice_boundary(&foreign, program, path, "Item", &mut Some(1_000_000))
        .expect_err("foreign source range must fail");
    assert!(message.contains("interval"), "{message}");
    Ok(())
}

#[test]
fn input_checked_multiway_distinct_callees_without_unrelated_full_repeat() -> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let text = "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} Envelope 'done'; fragment Envelope returns t::Root: 'via' Item 'after'; fragment Item returns t::Root: 'send' (A | (-> Prefix B) | (=> Other C)); fragment A returns t::Root: 'a'; fragment B returns t::Root: 'b'; fragment C returns t::Root: 'c'; fragment Prefix returns t::Root: 'x'; fragment Other returns t::Root: 'y';";
    let compilation = multiway_source_compilation(text)?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar.parser_prediction.as_ref().ok_or("No program")?;
    let (path, expr) = deferred_source_choice(grammar, &mut 100_000)?;
    assert_eq!(program.decisions[path].entries.len(), 3);
    assert!(!program.recursive_decisions.contains(path));
    assert!(admission.decision_is_runtime(path));
    assert!(
        admission.decisions().all(|p| p.ends_with("/choice")),
        "no unrelated Full-repeat authority"
    );
    let calls = live_choice_calls(grammar)?;
    for (text, expected) in [
        ("via send a after done", Some(0)),
        ("via send x b after done", Some(1)),
        ("via send y c after done", Some(2)),
        ("via send y c wrong done", None),
    ] {
        let source = DefinitionSource::new("memory:multiway/distinct", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 2;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        assert_eq!(
            live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)?,
            expected,
            "{text}"
        );
        if expected.is_some() {
            assert_eq!(input.inner.position, tokens.len());
        }
    }
    Ok(())
}

#[test]
fn input_checked_multiway_distinct_callee_preserves_real_nested_priority() -> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let text = live_choice_source().replace("Body | (=> Prefix Body)", "A | B | (=> Other C)")
        + " fragment A returns t::Root: (-> Prefix Body 'y') | (Body 'z'); fragment B returns t::Root: Body 'z'; fragment Other returns t::Root: 'c'; fragment C returns t::Root: 'd';";
    let compilation = multiway_source_compilation(&text)?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No actual native program")?;
    let (outer_path, outer) = deferred_source_choice(grammar, &mut 100_000)?;
    let nested_path = program
        .decisions
        .keys()
        .find(|p| p.starts_with("rule/A/") && p.ends_with("/choice"))
        .ok_or("No real A choice")?;
    let nested_decision = &program.decisions[nested_path];
    println!(
        "MULTIWAY_NESTED_CANONICAL flags={:?} nested={nested_path}",
        program.recursive_decisions
    );
    assert!(
        program.recursive_decisions.contains(nested_path),
        "fixture must reach genuine recursive commitment metadata"
    );
    assert!(
        program.completion_entry_priority(nested_decision, &mut 100_000)?,
        "fixture must prove existing actual entry priority"
    );
    assert!(
        !admission.decision_is_runtime(nested_path),
        "A retains its existing source-proved priority, not conditional multiway authority"
    );
    let base = nested_path
        .strip_suffix("/choice")
        .ok_or("No A occurrence suffix")?;
    let (_, _, nested) = super::super::nullable_repetition::prefix_source_occurrence_with_work(
        grammar,
        base,
        &mut 100_000,
    )?
    .ok_or("No actual A occurrence")?;
    let Kind::Alternative(outer_branches) = &outer.kind else {
        return Err("No outer Alternative".into());
    };
    let a_call = outer_branches.first().ok_or("No actual A branch")?;
    assert!(matches!(&a_call.kind,Kind::Call(name) if name=="A"));
    let calls = live_choice_calls(grammar)?;
    let nested_calls = [calls[0], calls[1], a_call];
    for (text, want_a, want_b, expected) in [
        ("{ } via send a { } z after done", false, true, 1),
        ("{ } via send a a { } y after done", true, false, 0),
    ] {
        let source =
            DefinitionSource::new("memory:multiway/nested-canonical", SourceKind::Xtext, text);
        let tokens = symbols(text);
        // Establish canonical nested selection using the real source call
        // chain BEFORE projecting any viability result onto the outer choice.
        let mut direct = TraceReplay::new(&tokens);
        direct.inner.position = 4;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        assert_eq!(
            live.predict(
                &source.id,
                &nested_calls,
                nested,
                "choice",
                &mut direct,
                &mut work
            )?,
            Some(0),
            "actual canonical A commitment"
        );
        for (owner, expected_body) in [("A", want_a), ("B", want_b)] {
            let (start, stop) = program.rules[owner];
            let scope = program.verify_completion_scope_with_verification(
                start,
                stop,
                &mut 1_000_000,
                CompletionVerification::InputCheckedCompletion,
            )?;
            let mut input = TraceReplay::new(&tokens);
            input.inner.position = 4;
            let mut work = Some(1_000_000);
            let outcome = program.run_prediction_at_depth_with_local_decision(
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
                None,
            )?;
            assert!(
                matches!(outcome,PredictionOutcome::Completed(value) if value==expected_body),
                "canonical {owner} viability {text}: {outcome:?}"
            );
            if expected_body {
                assert_eq!(
                    tokens.get(input.inner.position),
                    Some(&PredictionSymbol::Keyword("after".into()))
                );
            }
        }
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 4;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = live.predict(&source.id, &calls, outer, "choice", &mut input, &mut work);
        println!(
            "MULTIWAY_NESTED_OUTER {text:?} path={outer_path} result={result:?} remaining={work:?}"
        );
        assert_eq!(
            result?,
            Some(expected),
            "outer viability must honor actual nested commitment"
        );
        assert_eq!(
            input.inner.position,
            tokens.len(),
            "actual caller suffix and EOF must execute"
        );
        assert!(work.is_some_and(|n| n > 0));
    }
    Ok(())
}

#[test]
fn input_checked_multiway_productive_loop_bounds_actual_worker_invocations() -> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let syntax = "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} ('open' (A | B | (=> Other C)) 'close')* 'done'; fragment A returns t::Root: 'a'; fragment B returns t::Root: 'b'; fragment Other returns t::Root: 'c'; fragment C returns t::Root: 'd';";
    let compilation = multiway_source_compilation(syntax)?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar.parser_prediction.as_ref().ok_or("No program")?;
    let path = program
        .decisions
        .iter()
        .find(|(p, d)| {
            p.starts_with("rule/Root/") && p.ends_with("/choice") && d.entries.len() == 3
        })
        .map(|(p, _)| p.as_str())
        .ok_or("No actual loop-owned threeway Alternative")?;
    assert!(admission.decision_is_runtime(path));
    assert!(
        !program.recursive_decisions.contains(path),
        "no fabricated language-call/priority depth"
    );
    let base = path
        .strip_suffix("/choice")
        .ok_or("No actual choice suffix")?;
    let (owner, _, expr) = super::super::nullable_repetition::prefix_source_occurrence_with_work(
        grammar,
        base,
        &mut 100_000,
    )?
    .ok_or("No source loop choice")?;
    assert_eq!(owner, "Root");
    assert_eq!(expr.cardinality, '1');
    struct DepthInput<'a> {
        inner: TraceReplay<'a>,
        max_base: usize,
        max_returns: usize,
        entries: usize,
    }
    impl PredictionInput for DepthInput<'_> {
        fn runtime_work_diagnostics(&self) -> bool {
            true
        }
        fn runtime_work_geometry(&mut self, event: RuntimeWorkGeometry) {
            self.max_base = self.max_base.max(event.base_depth);
            self.max_returns = self.max_returns.max(event.return_depth);
            if event.kind == "decision_enter" || event.kind == "completion_enter" {
                self.entries += 1;
            }
        }
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
            self.inner.read(w)
        }
    }
    // Same caller-owned loop, no recursive grammar Call between iterations.
    // All items and all speculation share ONE original finite caller pool.
    for (text, want_success) in [
        (
            "open a close open b close open c d close done".to_string(),
            true,
        ),
        (format!("{}done", "open a close ".repeat(160)), false),
    ] {
        let source =
            DefinitionSource::new("memory:multiway/productive-loop", SourceKind::Xtext, &text);
        let tokens = symbols(&text);
        let mut input = DepthInput {
            inner: TraceReplay::new(&tokens),
            max_base: 0,
            max_returns: 0,
            entries: 0,
        };
        input.inner.inner.position = 1; // the actual first 'open' has been consumed
        let original = 20_000_000;
        let mut work = Some(original);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = live.predict(&source.id, &[], expr, "choice", &mut input, &mut work);
        println!(
            "MULTIWAY_LOOP success={want_success} result={result:?} max_base={} max_returns={} entries={} original={original} remaining={work:?}",
            input.max_base, input.max_returns, input.entries
        );
        if want_success {
            assert_eq!(result?, Some(0));
            assert_eq!(input.inner.inner.position, tokens.len());
        } else {
            let message = result.expect_err(
                "long sibling loop must not grow Rust invocation stack without a bound",
            );
            assert!(message.contains("depth"), "{message}");
            assert!(
                !message.contains("budget"),
                "fixture must reach the actual explicit depth bound, not exhaust work first: {message}"
            );
            assert!(
                input.max_base > 32,
                "actual invocation depth must grow across sibling iterations"
            );
            assert!(
                input.max_base <= 128,
                "existing depth envelope must remain unchanged"
            );
            assert!(
                input.max_returns <= 2,
                "grammar call stack must remain shallow while real invocation depth grows"
            );
            assert!(live.is_poisoned());
            assert!(
                input
                    .inner
                    .events
                    .iter()
                    .any(|e| matches!(e, TraceEvent::Rewind(_, _))),
                "nested speculative input must restore through actual adapter"
            );
            let events = input.inner.events.len();
            assert!(
                live.predict(&source.id, &[], expr, "choice", &mut input, &mut work)
                    .is_err()
            );
            assert_eq!(
                input.inner.events.len(),
                events,
                "poisoned retry cannot read or rewind input"
            );
        }
        assert!(
            work.is_some_and(|w| w > 0 && w < original),
            "one original caller grant must remain finite and consumed"
        );
    }
    Ok(())
}

// Original SendNode has two alternatives and a Full operand containing another
// alternative, assignment targets, an optional receiver and the final ActionBody.
// This genuine source retains that structure instead of flattening it into the
// previously qualified direct-prefix/same-callee pair.
#[test]
fn input_checked_binary_complex_guarded_prefix_sendnode_shape() -> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let text = "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} Envelope 'done'; fragment Envelope returns t::Root: 'via' Item 'after'; fragment Item returns t::Root: 'send' (ActionBody | (=> (children += NodeParameterMember SenderReceiverPart? | children += EmptyParameterMember SenderReceiverPart) ActionBody)); fragment ActionBody returns t::Root: 'p'? '{' (=> Item)* '}'; NodeParameterMember returns t::Root: 'p'; EmptyParameterMember returns t::Root: 'empty'; fragment SenderReceiverPart returns t::Root: 'via' children += NodeParameterMember | children += EmptyParameterMember 'to' children += NodeParameterMember;";
    let model = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="deferred" nsURI="urn:deferred-choice"><eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="children" eType="#//Root" upperBound="-1" containment="true"/></eClassifiers></e:EPackage>"##;
    let compilation = deferred_source_choice_text_compilation_with_model(
        text,
        crate::grammar::GrammarAnalysisWorkBudget::REALISTIC,
        model,
    )?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No actual program")?;
    let path = "rule/Item/s1/s0/choice";
    let (owner, _, expr) = super::super::nullable_repetition::prefix_source_occurrence_with_work(
        grammar,
        "rule/Item/s1/s0",
        &mut 100_000,
    )?
    .ok_or("No genuine outer Item choice")?;
    assert_eq!(owner, "Item");
    assert!(matches!(&expr.kind, Kind::Alternative(items) if items.len() == 2));
    assert!(
        program
            .decisions
            .keys()
            .any(|p| p.starts_with("rule/Item/s1/s0/a1/") && p.ends_with("/choice")),
        "Full operand retains its genuine inner alternative"
    );
    assert_eq!(program.decisions[path].entries.len(), 2);
    assert!(admission.decision_is_runtime(path));
    assert!(
        !program.input_checked_asymmetric_guarded_shape(
            path,
            &program.decisions[path],
            None,
            &mut 100_000,
        )?,
        "complex operand requires its own conditional obligation instead of the qualified direct-prefix exclusion"
    );
    let calls = live_choice_calls(grammar)?;
    let frozen = serde_json::to_vec(grammar).map_err(|e| e.to_string())?;
    for (input_text, expected) in [
        ("via send { } after done", Some(0)),
        ("via send p p { } after done", Some(1)),
        ("via send empty via p { } after done", Some(1)),
        (
            "via send p via p { send empty via p { } } after done",
            Some(1),
        ),
        ("via send p { } wrong done", None),
        ("via send p p { } after done trailing", None),
    ] {
        let source = DefinitionSource::new("memory:binary/sendnode", SourceKind::Xtext, input_text);
        let tokens = symbols(input_text);
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 2;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        assert_eq!(
            live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)?,
            expected,
            "{input_text}"
        );
        if expected.is_some() {
            assert_eq!(input.inner.position, tokens.len());
        }
    }
    let source = DefinitionSource::new(
        "memory:binary/overlap",
        SourceKind::Xtext,
        "via send p { } after done",
    );
    let tokens = symbols(&source.text);
    let mut input = TraceReplay::new(&tokens);
    input.inner.position = 2;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    let overlap = live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work);
    assert!(
        overlap.as_ref().is_err_and(|e| e.contains("Ambiguous")),
        "actual binary overlap must fail: {overlap:?}"
    );
    assert!(live.is_poisoned());

    let input_text = "via send empty via p { } after done";
    let source = DefinitionSource::new("memory:binary/guard", SourceKind::Xtext, input_text);
    let tokens = symbols(input_text);
    let foreign = DefinitionSource::new("memory:binary/foreign", SourceKind::Xtext, input_text);
    let mut input = TraceReplay::new(&tokens);
    input.inner.position = 2;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    assert!(
        live.predict(&foreign.id, &calls, expr, "choice", &mut input, &mut work)
            .is_err()
    );
    assert!(live.is_poisoned());
    struct Cancel<'a>(TraceReplay<'a>);
    impl PredictionInput for Cancel<'_> {
        fn replay_position(&self) -> Option<usize> {
            self.0.replay_position()
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.0.checkpoint()
        }
        fn rewind(&mut self, p: usize) -> Result<(), String> {
            self.0.rewind(p)
        }
        fn read(&mut self, _: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            Err("cancelled actual binary input".into())
        }
    }
    let mut input = Cancel(TraceReplay::new(&tokens));
    input.0.inner.position = 2;
    let mut work = Some(1_000_000);
    let mut live =
        LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
    assert!(
        live.predict(&source.id, &calls, expr, "choice", &mut input, &mut work)
            .expect_err("actual cancellation")
            .contains("cancelled actual binary")
    );
    assert!(live.is_poisoned());
    assert!(work.is_some_and(|w| w > 0));
    assert_eq!(
        serde_json::to_vec(grammar).map_err(|e| e.to_string())?,
        frozen
    );
    Ok(())
}

#[test]
fn input_checked_classified_assigned_first_pair_before_sealing() -> Result<(), String> {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let text = "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} Envelope 'done'; fragment Envelope returns t::Root: 'via' Item 'after'; fragment Item returns t::Root: 'send' (Body | (-> Prefix Body) | (=> Other Body)); fragment Body returns t::Root: '{' (=> Inner)* '}'; fragment Inner returns t::Root: 'terminate' (Body | -> children += Parameter Body); Parameter returns t::Root: 'p'; fragment Prefix returns t::Root: 'x'; fragment Other returns t::Root: 'y';";
    let model = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="deferred" nsURI="urn:deferred-choice"><eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="children" eType="#//Root" upperBound="-1" containment="true"/></eClassifiers></e:EPackage>"##;
    let compilation = deferred_source_choice_text_compilation_with_model(
        text,
        crate::grammar::GrammarAnalysisWorkBudget::REALISTIC,
        model,
    )?;
    let grammar = compilation.grammar();
    let admission = compilation.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No native program")?;
    let nested_path = "rule/Inner/s1/s0/choice";
    let pair = program
        .decisions
        .get(nested_path)
        .ok_or("No assigned First pair")?;
    assert_eq!(pair.entries.len(), 2);
    assert!(
        !program.completion_entry_priority(pair, &mut 100_000)?,
        "plain first alternative cannot borrow canonical early priority"
    );
    assert!(
        program.input_checked_asymmetric_guarded_shape(nested_path, pair, None, &mut 100_000,)?,
        "source is exactly the single-prefix pair reserved during prebinding"
    );
    assert!(
        !program.recursive_decisions.contains(nested_path),
        "disjoint actual first tokens must not acquire fabricated recursive flags"
    );
    assert!(
        !program.input_checked_runtime_source_choice(nested_path, grammar, &mut 100_000)?,
        "the old route requires its own actual positive classifier/source proof"
    );
    assert!(
        admission.decision_is_runtime(nested_path),
        "postclassification resolves the reserved pair before outer sealing"
    );
    println!(
        "CLASSIFIED_ASSIGNED_PAIR path={nested_path} final_recursive=false old_positive=false runtime_obligation=true"
    );
    let (outer_path, outer) = deferred_source_choice(grammar, &mut 100_000)?;
    assert_eq!(program.decisions[outer_path].entries.len(), 3);
    let calls = live_choice_calls(grammar)?;
    let frozen = serde_json::to_vec(grammar).map_err(|e| e.to_string())?;
    for (input_text, expected) in [
        ("via send { terminate { } } after done", Some(0)),
        ("via send { terminate p { } } after done", Some(0)),
        ("via send x { terminate p { } } after done", Some(1)),
        ("via send y { terminate { } } after done", Some(2)),
        ("via send { terminate p { } } wrong done", None),
        ("via send { terminate p { } } after done trailing", None),
    ] {
        let source = DefinitionSource::new(
            "memory:classified/assigned-first",
            SourceKind::Xtext,
            input_text,
        );
        let tokens = symbols(input_text);
        let mut input = TraceReplay::new(&tokens);
        input.inner.position = 2;
        let mut work = Some(1_000_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        assert_eq!(
            live.predict(&source.id, &calls, outer, "choice", &mut input, &mut work)?,
            expected,
            "{input_text}"
        );
        if expected.is_some() {
            assert_eq!(input.inner.position, tokens.len());
        }
        assert!(work.is_some_and(|remaining| remaining > 0));
    }
    assert_eq!(
        serde_json::to_vec(grammar).map_err(|e| e.to_string())?,
        frozen
    );
    Ok(())
}
