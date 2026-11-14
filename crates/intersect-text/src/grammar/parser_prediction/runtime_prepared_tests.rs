fn prepared_context_grammar() -> Grammar {
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
    g.profile = Profile::NativeV67;
    g.parser_prediction = Some(ParserPrediction::compile(&g).unwrap());
    g
}

#[test]
fn prepared_runtime_context_real_worker_reuse_foreign_identity_and_invocation_poison() {
    let g = prepared_context_grammar();
    let program = g.parser_prediction.as_ref().unwrap();
    let mut preparation = Some(100_000);
    let context = PreparedRuntimePredictionContext::prepare_selected(
        &g,
        &[],
        CompletionVerification::Legacy,
        ["rule/Prefix/repeat"],
        &mut preparation,
        100_000,
    )
    .unwrap();
    assert!(preparation.unwrap() < 100_000 && context.prepared_decisions() > 0);
    assert_eq!(context.selected_decisions(), 1);
    assert_eq!(context.prepared_decisions(), 1);
    let context_bytes = context.logical_bytes();
    let before = serde_json::to_vec(program).unwrap();
    for text in [
        "next",
        "public next",
        "private next",
        "protected next",
        "unknown",
        "",
    ] {
        let tokens = symbols(text);
        let mut old = RuntimeSourceInput::new(&tokens);
        let expected =
            program.predict_with_replay_input("rule/Prefix/repeat", &mut old, &mut Some(100_000));
        let id = intersect_model::definitions::bundle::SourceId::for_text("memory:prepared/real", text);
        let mut work = Some(100_000);
        let mut source =
            RepeatedRuntimePredictionSource::new(program, &id, &[], &mut work).unwrap();
        let mut actual = RuntimeSourceInput::new(&tokens);
        let result = context.predict(
            &g,
            program,
            &mut source,
            &id,
            &[],
            "rule/Prefix/repeat",
            &mut actual,
            &mut work,
            CompletionVerification::Legacy,
        );
        assert_eq!(result, expected);
        assert_eq!(actual.furthest, old.furthest);
        assert_eq!(actual.events.into_inner(), old.events.into_inner());
        assert!(source.hits() > 0);
    }
    let tokens = symbols("public next");
    let id = runtime_source_id();
    let mut work = Some(100_000);
    let mut source = RepeatedRuntimePredictionSource::new(program, &id, &[], &mut work).unwrap();
    let mut denied = RuntimeSourceInput::new(&tokens);
    denied.fail_read = true;
    assert!(
        context
            .predict(
                &g,
                program,
                &mut source,
                &id,
                &[],
                "rule/Prefix/repeat",
                &mut denied,
                &mut work,
                CompletionVerification::Legacy
            )
            .is_err()
    );
    assert!(source.is_poisoned());
    // Equal serialized grammar/program cannot impersonate the bound live one.
    let foreign = g.clone();
    assert_eq!(
        serde_json::to_vec(&g).unwrap(),
        serde_json::to_vec(&foreign).unwrap()
    );
    let fp = foreign.parser_prediction.as_ref().unwrap();
    let mut foreign_source = RepeatedRuntimePredictionSource::new(fp, &id, &[], &mut work).unwrap();
    let mut unread = RuntimeSourceInput::new(&tokens);
    assert!(
        context
            .predict(
                &foreign,
                fp,
                &mut foreign_source,
                &id,
                &[],
                "rule/Prefix/repeat",
                &mut unread,
                &mut work,
                CompletionVerification::Legacy
            )
            .is_err()
    );
    assert_eq!(unread.furthest, 0);
    assert!(foreign_source.is_poisoned());
    let mut fresh = RepeatedRuntimePredictionSource::new(program, &id, &[], &mut work).unwrap();
    let mut recovery = RuntimeSourceInput::new(&tokens);
    assert_eq!(
        context
            .predict(
                &g,
                program,
                &mut fresh,
                &id,
                &[],
                "rule/Prefix/repeat",
                &mut recovery,
                &mut work,
                CompletionVerification::Legacy
            )
            .unwrap(),
        Some(0)
    );
    assert_eq!(context.logical_bytes(), context_bytes);
    assert_eq!(serde_json::to_vec(program).unwrap(), before);
}

#[test]
fn prepared_runtime_context_paid_preparation_failure_and_first_full_fallback() {
    let g = prepared_context_grammar();
    for paths in [
        vec!["rule/Unknown/repeat"],
        vec!["rule/Prefix/repeat", "rule/Prefix/repeat"],
        vec![],
    ] {
        assert!(
            PreparedRuntimePredictionContext::prepare_selected(
                &g,
                &[],
                CompletionVerification::Legacy,
                paths,
                &mut Some(100_000),
                100_000
            )
            .is_err()
        );
    }
    // Unlisted real decisions remain in the existing canonical local fallback.
    let selected = PreparedRuntimePredictionContext::prepare_selected(
        &g,
        &[],
        CompletionVerification::Legacy,
        ["rule/Prefix/repeat"],
        &mut Some(100_000),
        100_000,
    )
    .unwrap();
    let program = g.parser_prediction.as_ref().unwrap();
    for text in ["public", "private", "unknown"] {
        let tokens = symbols(text);
        let mut old = RuntimeSourceInput::new(&tokens);
        let expected = program.predict_with_replay_input(
            "rule/Visibility/choice",
            &mut old,
            &mut Some(100_000),
        );
        let mut work = Some(100_000);
        let mut cursor =
            RepeatedRuntimePredictionSource::new(program, &runtime_source_id(), &[], &mut work)
                .unwrap();
        let mut actual = RuntimeSourceInput::new(&tokens);
        let observed = selected.predict(
            &g,
            program,
            &mut cursor,
            &runtime_source_id(),
            &[],
            "rule/Visibility/choice",
            &mut actual,
            &mut work,
            CompletionVerification::Legacy,
        );
        assert_eq!(observed, expected);
        assert_eq!(actual.events.into_inner(), old.events.into_inner());
        assert_eq!(actual.furthest, old.furthest);
        assert_eq!(cursor.hits(), 0);
    }
    for mut work in [None, Some(0), Some(1)] {
        assert!(
            PreparedRuntimePredictionContext::prepare(
                &g,
                &[],
                CompletionVerification::Legacy,
                &mut work,
                100_000
            )
            .is_err()
        );
    }
    assert!(
        PreparedRuntimePredictionContext::prepare(
            &g,
            &[],
            CompletionVerification::Legacy,
            &mut Some(100_000),
            0
        )
        .is_err()
    );
    let mut enough = Some(100_000);
    let complete = PreparedRuntimePredictionContext::prepare(
        &g,
        &[],
        CompletionVerification::Legacy,
        &mut enough,
        100_000,
    )
    .unwrap();
    let paid = 100_000 - enough.unwrap();
    assert!(paid > 1);
    assert!(
        PreparedRuntimePredictionContext::prepare(
            &g,
            &[],
            CompletionVerification::Legacy,
            &mut Some(paid - 1),
            100_000
        )
        .is_err()
    );
    let exact = PreparedRuntimePredictionContext::prepare(
        &g,
        &[],
        CompletionVerification::Legacy,
        &mut Some(paid),
        complete.logical_bytes(),
    )
    .unwrap();
    assert_eq!(exact.prepared_decisions(), complete.prepared_decisions());
    for full_guard in [false, true] {
        let mut first = keyword("a");
        first.first_set_predicated = true;
        let body = if full_guard {
            choice(vec![
                full(sequence(vec![keyword("a"), keyword("b")])),
                keyword("b"),
            ])
        } else {
            choice(vec![first, keyword("b")])
        };
        let mut guarded = grammar(body, vec![]);
        guarded.profile = if full_guard {
            Profile::NativeV68
        } else {
            Profile::NativeV67
        };
        guarded.parser_prediction = Some(if full_guard {
            ParserPrediction::compile_with_recursive_first_predicates(&guarded, &BTreeSet::new())
                .unwrap()
        } else {
            ParserPrediction::compile_with_first_predicates(
                &guarded,
                &["rule/Entry/a0".into()].into(),
            )
            .unwrap()
        });
        let p = guarded.parser_prediction.as_ref().unwrap();
        let prepared = PreparedRuntimePredictionContext::prepare_selected(
            &guarded,
            &[],
            CompletionVerification::Legacy,
            ["rule/Entry/choice"],
            &mut Some(100_000),
            100_000,
        )
        .unwrap();
        assert_eq!(prepared.selected_decisions(), 1);
        assert_eq!(prepared.ineligible_decisions(), 1);
        assert_eq!(prepared.prepared_decisions(), 0);
        for text in ["a b", "b", "unknown"] {
            let tokens = symbols(text);
            let mut old = RuntimeSourceInput::new(&tokens);
            let expected =
                p.predict_with_replay_input("rule/Entry/choice", &mut old, &mut Some(100_000));
            let mut work = Some(100_000);
            let mut cursor =
                RepeatedRuntimePredictionSource::new(p, &runtime_source_id(), &[], &mut work)
                    .unwrap();
            let mut actual = RuntimeSourceInput::new(&tokens);
            let observed = prepared.predict(
                &guarded,
                p,
                &mut cursor,
                &runtime_source_id(),
                &[],
                "rule/Entry/choice",
                &mut actual,
                &mut work,
                CompletionVerification::Legacy,
            );
            assert_eq!(observed, expected);
            assert_eq!(actual.furthest, old.furthest);
            assert_eq!(actual.events.into_inner(), old.events.into_inner());
            assert_eq!(cursor.publications(), 0);
            assert_eq!(cursor.hits(), 0);
        }
    }
}
