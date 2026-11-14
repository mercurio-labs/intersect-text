// Authored source/graph controls, not original Member admission or utility.
fn live_component_grammar(callers: usize) -> Grammar {
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
    for index in 0..callers {
        let mut rule = template.clone();
        rule.body = sequence(vec![call("Prefix"), keyword(&format!("tail{index}"))]);
        g.rules.insert(format!("Unrelated{index}"), rule);
    }
    live_compile(&mut g);
    g
}
fn live_compile(g: &mut Grammar) {
    g.profile = Profile::NativeV67;
    let mut predicates = BTreeSet::new();
    for (name, rule) in &mut g.rules {
        super::super::bind_prediction_paths(
            &mut rule.body,
            &format!("rule/{name}"),
            &mut predicates,
            true,
        );
    }
    g.parser_prediction =
        Some(ParserPrediction::compile_with_first_predicates(g, &predicates).unwrap());
}
fn live_component_call(g: &Grammar) -> &Expr {
    let Kind::Sequence(items) = &g.rules["Entry"].body.kind else {
        panic!("source sequence");
    };
    &items[0]
}
fn live_component_run(
    g: &Grammar,
    text: &str,
    grant: usize,
    preconsumed: usize,
) -> (PortsResult, usize, bool) {
    let source = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:live.input",
        intersect_model::definitions::bundle::SourceKind::Xtext,
        text,
    );
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let mut work = Some(grant);
    for _ in 0..preconsumed {
        charge(&mut work).unwrap();
    }
    let mut scope = match LiveCallerPrediction::new(g, &source.id, &mut work) {
        Ok(scope) => scope,
        Err(error) => return ((Err(error), 0, 0, vec![]), grant - work.unwrap(), true),
    };
    let result = scope.predict(
        &source.id,
        &[live_component_call(g)],
        &g.rules["Prefix"].body,
        "repeat",
        &mut input,
        &mut work,
    );
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
fn live_callers_first_cold_many_unrelated_follow_paths_keep_same_complete_prediction() {
    let g = live_component_grammar(48);
    let reference = live_component_grammar(0);
    let program = g.parser_prediction.as_ref().unwrap();
    let bytes = serde_json::to_vec(&g).unwrap();
    for text in [
        "public next",
        "private next",
        "protected next",
        "next",
        "tail0",
        "tail47",
        "unknown",
        "",
    ] {
        let tokens = symbols(text);
        let mut old_input = RuntimeSourceInput::new(&tokens);
        let mut old_work = Some(100_000);
        let old =
            program.predict_with_replay_input("rule/Prefix/repeat", &mut old_input, &mut old_work);
        let expected = (
            old,
            old_input.position,
            old_input.furthest,
            old_input.events.into_inner(),
        );
        let mut reference_input = RuntimeSourceInput::new(&tokens);
        let mut reference_work = Some(100_000);
        let selected = reference
            .parser_prediction
            .as_ref()
            .unwrap()
            .predict_with_replay_input(
                "rule/Prefix/repeat",
                &mut reference_input,
                &mut reference_work,
            );
        let reference_result = (
            selected,
            reference_input.position,
            reference_input.furthest,
            reference_input.events.into_inner(),
        );
        let (actual, used, poisoned) = live_component_run(&g, text, 100_000, 0);
        println!(
            "LIVE_CALLER_COLD {}",
            serde_json::json!({"text":text,"old_global_follow":expected,"single_actual_caller_reference":reference_result,"actual":actual,"old_work":100000-old_work.unwrap(),"new_work_including_all_binding_and_return_copies":used,"original_language_award":false})
        );
        // The independent root language is Visibility? 'next'. Unreachable
        // caller tails cannot justify probing an invalid root input farther.
        // Keep the old global observation, but use its single-real-caller
        // specialization as the full result/events/error-provenance reference.
        assert_eq!(actual, reference_result);
        if matches!(
            text,
            "public next" | "private next" | "protected next" | "next"
        ) {
            assert_eq!(actual, expected);
        }
        assert!(!poisoned);
        assert!(
            used < 100_000 - old_work.unwrap(),
            "authored full cold cost must improve"
        );
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), bytes);
}

#[test]
fn live_callers_real_continuation_matches_independent_bounded_recognition_language() {
    let g = live_component_grammar(4);
    // Independent finite language is Visibility? 'next', not a second general
    // parser. This checks every length<=2 word over the stated small alphabet.
    let alphabet = ["public", "private", "protected", "next", "tail0", "bad"];
    let mut cases = vec![String::new()];
    for a in alphabet {
        cases.push(a.to_owned());
        for b in alphabet {
            cases.push(format!("{a} {b}"));
        }
    }
    for text in cases {
        let words: Vec<_> = text.split_whitespace().collect();
        let is_visibility = |word: &str| ["public", "private", "protected"].contains(&word);
        let reference = matches!(words.as_slice(), ["next"])
            || matches!(words.as_slice(), [first,"next"] if is_visibility(first));
        let (actual, _, poisoned) = live_component_run(&g, &text, 100_000, 0);
        let selection = actual.0.unwrap();
        let consumed = usize::from(selection.is_some());
        let recognized = (selection.is_none()
            || words.first().is_some_and(|word| is_visibility(word)))
            && words.get(consumed) == Some(&"next")
            && words.len() == consumed + 1;
        assert_eq!(recognized, reference, "{text}");
        assert!(!poisoned);
    }
}

#[test]
fn live_callers_zero_exact_short_preconsumed_denial_poison_and_fresh_recovery() {
    let g = live_component_grammar(8);
    let (ok, used, failed) = live_component_run(&g, "public next", 100_000, 0);
    assert_eq!(ok.0.unwrap(), Some(0));
    assert!(!failed);
    for allowance in [0, 1, used - 1, used] {
        let (actual, charged, failed) = live_component_run(&g, "public next", allowance, 0);
        assert_eq!(charged, allowance);
        if allowance == used {
            assert_eq!(actual.0.unwrap(), Some(0));
            assert!(!failed);
        } else {
            assert!(actual.0.unwrap_err().contains("budget"));
            assert!(failed);
        }
    }
    let (actual, charged, failed) = live_component_run(&g, "public next", used + 7, 7);
    assert_eq!(actual.0.unwrap(), Some(0));
    assert_eq!(charged, used + 7);
    assert!(!failed);
    assert!(live_component_run(&g, "next", 100_000, 0).0.0.is_ok());
}

#[test]
fn live_callers_exact_source_occurrence_chain_foreign_and_late_input_failures_are_sticky() {
    let g = live_component_grammar(4);
    let source = runtime_source_id();
    let other = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:foreign",
        intersect_model::definitions::bundle::SourceKind::Xtext,
        "x",
    )
    .id;
    let foreign = g.rules["Prefix"].body.clone();
    for mode in 0..4 {
        let mut work = Some(100_000);
        let mut scope = LiveCallerPrediction::new(&g, &source, &mut work).unwrap();
        let tokens = symbols("public next");
        let mut input = RuntimeSourceInput::new(&tokens);
        let calls = if mode == 2 {
            vec![]
        } else {
            vec![live_component_call(&g)]
        };
        if mode == 3 {
            input.fail_read = true;
        }
        let value = scope.predict(
            if mode == 0 { &other } else { &source },
            &calls,
            if mode == 1 {
                &foreign
            } else {
                &g.rules["Prefix"].body
            },
            "repeat",
            &mut input,
            &mut work,
        );
        assert!(value.is_err());
        assert!(scope.is_poisoned());
        if mode != 3 {
            assert_eq!(input.position, 0);
            assert!(input.events.borrow().is_empty());
        }
        let before = input.events.borrow().len();
        assert!(
            scope
                .predict(
                    &source,
                    &[live_component_call(&g)],
                    &g.rules["Prefix"].body,
                    "repeat",
                    &mut input,
                    &mut work
                )
                .unwrap_err()
                .contains("poisoned")
        );
        assert_eq!(input.events.borrow().len(), before);
    }
}

#[test]
fn live_callers_full_predicate_partial_match_rewind_and_actions_use_same_worker() {
    for action in [false, true] {
        let guarded = expr(Kind::SyntacticPredicate(Box::new(sequence(vec![
            keyword("x"),
            keyword("y"),
        ]))));
        let left = if action {
            sequence(vec![expr(Kind::Action("A".into())), guarded])
        } else {
            guarded
        };
        let mut g = grammar(
            choice(vec![left, sequence(vec![keyword("x"), keyword("z")])]),
            vec![],
        );
        live_compile(&mut g);
        let id = runtime_source_id();
        for text in ["x y", "x z", "x q", "x", ""] {
            let tokens = symbols(text);
            let mut old = RuntimeSourceInput::new(&tokens);
            let mut old_work = Some(100_000);
            let expected = g
                .parser_prediction
                .as_ref()
                .unwrap()
                .predict_with_replay_input("rule/Entry/choice", &mut old, &mut old_work);
            let mut work = Some(100_000);
            let mut scope = LiveCallerPrediction::new(&g, &id, &mut work).unwrap();
            let mut input = RuntimeSourceInput::new(&tokens);
            let actual = scope.predict(
                &id,
                &[],
                &g.rules["Entry"].body,
                "choice",
                &mut input,
                &mut work,
            );
            assert_eq!(
                (
                    actual,
                    input.position,
                    input.furthest,
                    input.events.into_inner()
                ),
                (
                    expected,
                    old.position,
                    old.furthest,
                    old.events.into_inner()
                ),
                "{text} action{action}"
            );
        }
    }
}

#[test]
fn live_callers_quantified_call_uses_loop_return_and_keeps_genuine_overlapping_tail() {
    for cardinality in ['1', '?', '*', '+'] {
        let build = |extra: bool| {
            let mut called = call("Body");
            called.cardinality = cardinality;
            let mut tail = keyword("a");
            tail.cardinality = '?';
            let mut g = grammar(
                sequence(vec![called, keyword("stop")]),
                vec![("Body", sequence(vec![keyword("a"), tail]))],
            );
            if extra {
                let mut other = g.rules["Entry"].clone();
                other.body = sequence(vec![call("Body"), keyword("foreign")]);
                g.rules.insert("Unrelated".into(), other);
            }
            live_compile(&mut g);
            g
        };
        let g = build(true);
        let reference = build(false);
        let id = runtime_source_id();
        let Kind::Sequence(root) = &g.rules["Entry"].body.kind else {
            panic!("entry")
        };
        let Kind::Sequence(body) = &g.rules["Body"].body.kind else {
            panic!("body")
        };
        for text in ["a stop", "a a stop", "stop", "foreign", ""] {
            let tokens = symbols(text);
            let mut old = RuntimeSourceInput::new(&tokens);
            let mut old_work = Some(100_000);
            let expected = reference
                .parser_prediction
                .as_ref()
                .unwrap()
                .predict_with_replay_input("rule/Body/s1/repeat", &mut old, &mut old_work);
            let mut work = Some(100_000);
            let mut scope = LiveCallerPrediction::new(&g, &id, &mut work).unwrap();
            let mut input = RuntimeSourceInput::new(&tokens);
            let actual = scope.predict(&id, &[&root[0]], &body[1], "repeat", &mut input, &mut work);
            assert_eq!(
                (
                    actual,
                    input.position,
                    input.furthest,
                    input.events.into_inner()
                ),
                (
                    expected,
                    old.position,
                    old.furthest,
                    old.events.into_inner()
                ),
                "{cardinality} {text}"
            );
        }
    }
}

#[test]
fn live_callers_visible_first_and_true_entry_eof_keep_original_predicate_effects() {
    let mut left = keyword("x");
    left.first_set_predicated = true;
    let mut g = grammar(choice(vec![left, keyword("y")]), vec![]);
    live_compile(&mut g);
    let id = runtime_source_id();
    for text in ["x", "y", "z", ""] {
        let tokens = symbols(text);
        let mut old = RuntimeSourceInput::new(&tokens);
        let mut old_work = Some(100_000);
        let expected = g
            .parser_prediction
            .as_ref()
            .unwrap()
            .predict_with_replay_input("rule/Entry/choice", &mut old, &mut old_work);
        let mut work = Some(100_000);
        let mut scope = LiveCallerPrediction::new(&g, &id, &mut work).unwrap();
        let mut input = RuntimeSourceInput::new(&tokens);
        let actual = scope.predict(
            &id,
            &[],
            &g.rules["Entry"].body,
            "choice",
            &mut input,
            &mut work,
        );
        assert_eq!(
            (
                actual,
                input.position,
                input.furthest,
                input.events.into_inner()
            ),
            (
                expected,
                old.position,
                old.furthest,
                old.events.into_inner()
            )
        );
    }
    let mut optional = keyword("x");
    optional.cardinality = '?';
    let mut eof = grammar(optional, vec![]);
    live_compile(&mut eof);
    let mut work = Some(100_000);
    let mut scope = LiveCallerPrediction::new(&eof, &id, &mut work).unwrap();
    let tokens = symbols("");
    let mut input = RuntimeSourceInput::new(&tokens);
    assert_eq!(
        scope
            .predict(
                &id,
                &[],
                &eof.rules["Entry"].body,
                "repeat",
                &mut input,
                &mut work
            )
            .unwrap(),
        None
    );
}

#[test]
fn live_callers_full_repeat_keeps_actual_follow_overlap_instead_of_probe_only_priority() {
    let build = |extra: bool| {
        let mut repeated = expr(Kind::SyntacticPredicate(Box::new(keyword("x"))));
        repeated.cardinality = '*';
        let mut g = grammar(
            sequence(vec![call("Body"), keyword("x")]),
            vec![("Body", repeated)],
        );
        if extra {
            let mut other = g.rules["Entry"].clone();
            other.body = sequence(vec![call("Body"), keyword("foreign")]);
            g.rules.insert("Unrelated".into(), other);
        }
        live_compile(&mut g);
        g
    };
    let g = build(true);
    let reference = build(false);
    let id = runtime_source_id();
    for text in ["x", "x x", "x foreign", "foreign", ""] {
        let tokens = symbols(text);
        let mut old = RuntimeSourceInput::new(&tokens);
        let mut old_work = Some(100_000);
        let expected = reference
            .parser_prediction
            .as_ref()
            .unwrap()
            .predict_with_replay_input("rule/Body/repeat", &mut old, &mut old_work);
        let mut work = Some(100_000);
        let mut scope = LiveCallerPrediction::new(&g, &id, &mut work).unwrap();
        let mut input = RuntimeSourceInput::new(&tokens);
        let actual = scope.predict(
            &id,
            &[live_component_call(&g)],
            &g.rules["Body"].body,
            "repeat",
            &mut input,
            &mut work,
        );
        assert_eq!(
            (
                actual,
                input.position,
                input.furthest,
                input.events.into_inner()
            ),
            (
                expected,
                old.position,
                old.furthest,
                old.events.into_inner()
            ),
            "{text}"
        );
    }
}
