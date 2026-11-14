// Authored source/graph controls for actual physical return sharing. No claim
// that the original Member request fits before a separately qualified run.
#[allow(clippy::too_many_arguments)] // Independent grammar, input, depth and budget controls.
fn shared_return_call(
    g: &Grammar,
    callers: &[&Expr],
    expression: &Expr,
    suffix: &str,
    text: &str,
    grant: usize,
    preconsumed: usize,
    shared: bool,
) -> (PortsResult, usize, bool) {
    let source = runtime_source_id();
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let mut work = Some(grant);
    for _ in 0..preconsumed {
        charge(&mut work).unwrap();
    }
    let mut scope = match LiveCallerPrediction::new(g, &source, &mut work) {
        Ok(scope) => scope,
        Err(error) => return ((Err(error), 0, 0, vec![]), grant - work.unwrap(), true),
    };
    let result = if shared {
        scope.predict_shared_returns(&source, callers, expression, suffix, &mut input, &mut work)
    } else {
        scope.predict(&source, callers, expression, suffix, &mut input, &mut work)
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
fn shared_return_graph() -> Grammar {
    let mut rules = Vec::new();
    for level in 0..7 {
        rules.push((format!("Deep{level}"), call(&format!("Deep{}", level + 1))));
    }
    rules.push((
        "Deep7".into(),
        choice((0..80).map(|n| keyword(&format!("word{n}"))).collect()),
    ));
    let mut g = grammar(
        choice(vec![
            sequence(vec![call("Deep0"), keyword("left")]),
            sequence(vec![call("Deep0"), keyword("right")]),
        ]),
        vec![],
    );
    let template = g.rules["Entry"].clone();
    for (name, body) in rules {
        let mut rule = template.clone();
        rule.body = body;
        g.rules.insert(name, rule);
    }
    live_compile(&mut g);
    g
}
#[test]
fn shared_returns_first_cold_root_and_nested_context_remove_actual_vec_copies() {
    let g = shared_return_graph();
    let unchanged = serde_json::to_vec(&g).unwrap();
    for text in ["word0 left", "word79 right", "word0 bad", ""] {
        let old = shared_return_call(
            &g,
            &[],
            &g.rules["Entry"].body,
            "choice",
            text,
            100000,
            0,
            false,
        );
        let new = shared_return_call(
            &g,
            &[],
            &g.rules["Entry"].body,
            "choice",
            text,
            100000,
            0,
            true,
        );
        println!(
            "SHARED_RETURNS_COLD {}",
            serde_json::json!({"context":"entry-no-outer-calls","text":text,"old":old.0,"actual":new.0,"eager_work":old.1,"shared_work":new.1,"original_admission":false})
        );
        assert_eq!(new.0, old.0);
        assert_eq!(new.2, old.2);
        assert!(
            new.1 < old.1,
            "actual first-cold whole cost must improve on this graph"
        );
    }
    let nested = live_component_grammar(8);
    for text in ["public next", "next", "bad", ""] {
        let old = shared_return_call(
            &nested,
            &[live_component_call(&nested)],
            &nested.rules["Prefix"].body,
            "repeat",
            text,
            100000,
            0,
            false,
        );
        let new = shared_return_call(
            &nested,
            &[live_component_call(&nested)],
            &nested.rules["Prefix"].body,
            "repeat",
            text,
            100000,
            0,
            true,
        );
        assert_eq!(new.0, old.0);
        assert_eq!(new.2, old.2);
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), unchanged);
}
#[test]
fn shared_returns_full_ordered_keys_and_prefix_pop_match_literal_vec_values() {
    use super::shared_returns::{Returns, SharedReturns};
    let mut work = Some(100000);
    let mut keys = BTreeSet::new();
    let mut expected = BTreeSet::new();
    for values in [
        vec![],
        vec![0],
        vec![0, 2],
        vec![0, 1],
        vec![1],
        vec![1, 0],
        vec![127, 0, 5],
    ] {
        let mut cache = None;
        let mut actual = SharedReturns::seed(&values, &mut cache, &mut work).unwrap();
        let mut literal = values.clone();
        let shared = actual.clone();
        if let (
            SharedReturns::Prefix { storage: a, .. },
            SharedReturns::Prefix { storage: b, .. },
        ) = (&actual, &shared)
        {
            assert!(
                std::rc::Rc::ptr_eq(a, b),
                "configuration clone must share real storage"
            );
        }
        keys.insert(actual.clone());
        expected.insert(literal.clone());
        actual.push_return(77, &mut work).unwrap();
        literal.push(77);
        assert!(
            actual.has_shared_parent(&shared),
            "actual Call must share its immutable parent instead of copying the full prefix"
        );
        assert_eq!(
            actual.materialized(&mut work).unwrap().as_ref(),
            literal.as_slice()
        );
        loop {
            match (actual.popped(&mut work, true).unwrap(), literal.pop()) {
                (Some((a, rest)), Some(b)) => {
                    assert_eq!(a, b);
                    if let (
                        SharedReturns::Prefix { storage: a, .. },
                        SharedReturns::Prefix { storage: b, .. },
                    ) = (&actual, &rest)
                    {
                        assert!(
                            std::rc::Rc::ptr_eq(a, b),
                            "return pop must share the same retained prefix"
                        );
                    }
                    actual = rest;
                    assert_eq!(
                        actual.materialized(&mut work).unwrap().as_ref(),
                        literal.as_slice()
                    );
                }
                (None, None) => break,
                _ => panic!("full return prefix diverged"),
            }
        }
    }
    assert_eq!(
        keys.iter()
            .map(|x| x.materialized(&mut work).unwrap().into_owned())
            .collect::<Vec<_>>(),
        expected.into_iter().collect::<Vec<_>>()
    );
}
#[test]
fn shared_returns_zero_exact_short_preconsumed_and_fresh_recovery_keep_original_pool() {
    let g = shared_return_graph();
    let expression = &g.rules["Entry"].body;
    let (whole, used, failed) =
        shared_return_call(&g, &[], expression, "choice", "word0 left", 100000, 0, true);
    assert!(whole.0.is_ok());
    assert!(!failed);
    for grant in [0, 1, used - 1, used] {
        let (result, charged, poisoned) =
            shared_return_call(&g, &[], expression, "choice", "word0 left", grant, 0, true);
        assert_eq!(charged, grant);
        if grant == used {
            assert_eq!(result, whole);
            assert!(!poisoned);
        } else {
            assert!(result.0.is_err());
            assert!(poisoned);
        }
    }
    let next = shared_return_call(
        &g,
        &[],
        expression,
        "choice",
        "word0 left",
        used + 7,
        7,
        true,
    );
    assert_eq!(next.0, whole);
    assert_eq!(next.1, used + 7);
    assert!(!next.2);
    assert!(
        shared_return_call(
            &g,
            &[],
            expression,
            "choice",
            "word0 right",
            100000,
            0,
            true
        )
        .0
        .0
        .is_ok()
    );
}
#[test]
fn shared_returns_foreign_source_expr_chain_and_input_failure_poison_without_reuse() {
    let g = live_component_grammar(4);
    let source = runtime_source_id();
    let foreign = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:foreign",
        intersect_model::definitions::bundle::SourceKind::Xtext,
        "foreign",
    )
    .id;
    let copied = g.rules["Prefix"].body.clone();
    for mode in 0..4 {
        let mut work = Some(100000);
        let mut scope = LiveCallerPrediction::new(&g, &source, &mut work).unwrap();
        let tokens = symbols("public next");
        let mut input = RuntimeSourceInput::new(&tokens);
        input.fail_read = mode == 3;
        let callers = if mode == 2 {
            vec![]
        } else {
            vec![live_component_call(&g)]
        };
        let actual = scope.predict_shared_returns(
            if mode == 0 { &foreign } else { &source },
            &callers,
            if mode == 1 {
                &copied
            } else {
                &g.rules["Prefix"].body
            },
            "repeat",
            &mut input,
            &mut work,
        );
        assert!(actual.is_err());
        assert!(scope.is_poisoned());
        let events = input.events.borrow().len();
        assert!(
            scope
                .predict_shared_returns(
                    &source,
                    &[live_component_call(&g)],
                    &g.rules["Prefix"].body,
                    "repeat",
                    &mut input,
                    &mut work
                )
                .is_err()
        );
        assert_eq!(input.events.borrow().len(), events);
    }
}
#[test]
fn shared_returns_genuine_full_first_action_and_partial_rewind_match_eager_traces() {
    for action in [false, true] {
        let mut guarded = expr(Kind::SyntacticPredicate(Box::new(sequence(vec![
            keyword("x"),
            keyword("y"),
        ]))));
        guarded.first_set_predicated = true;
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
        for text in ["x y", "x z", "x q", "x", ""] {
            let old = shared_return_call(
                &g,
                &[],
                &g.rules["Entry"].body,
                "choice",
                text,
                100000,
                0,
                false,
            );
            let new = shared_return_call(
                &g,
                &[],
                &g.rules["Entry"].body,
                "choice",
                text,
                100000,
                0,
                true,
            );
            assert_eq!(new.0, old.0, "{text} action{action}");
            assert_eq!(new.2, old.2);
        }
    }
}
#[test]
fn shared_returns_actual_cardinality_returns_keep_overlapping_caller_continuations() {
    for cardinality in ['1', '?', '*', '+'] {
        let mut called = call("Body");
        called.cardinality = cardinality;
        let mut optional = keyword("a");
        optional.cardinality = '?';
        let mut g = grammar(
            sequence(vec![called, keyword("stop")]),
            vec![("Body", sequence(vec![keyword("a"), optional]))],
        );
        live_compile(&mut g);
        let Kind::Sequence(root) = &g.rules["Entry"].body.kind else {
            panic!("entry")
        };
        let Kind::Sequence(body) = &g.rules["Body"].body.kind else {
            panic!("body")
        };
        for text in ["a stop", "a a stop", "stop", "a bad", ""] {
            let old =
                shared_return_call(&g, &[&root[0]], &body[1], "repeat", text, 100000, 0, false);
            let new =
                shared_return_call(&g, &[&root[0]], &body[1], "repeat", text, 100000, 0, true);
            assert_eq!(new.0, old.0, "{cardinality} {text}");
            assert_eq!(new.2, old.2);
        }
    }
}
#[test]
fn shared_returns_depth128_and_recursive_same_occurrence_keep_full_operand_bound() {
    let full = expr(Kind::SyntacticPredicate(Box::new(keyword("x"))));
    let mut g = grammar(
        sequence(vec![call("A"), keyword("stop")]),
        vec![(
            "A",
            choice(vec![sequence(vec![keyword("r"), call("A")]), full]),
        )],
    );
    live_compile(&mut g);
    let Kind::Sequence(entry) = &g.rules["Entry"].body.kind else {
        panic!("entry")
    };
    let Kind::Alternative(alternatives) = &g.rules["A"].body.kind else {
        panic!("choice")
    };
    let Kind::Sequence(recursive) = &alternatives[0].kind else {
        panic!("recursive")
    };
    for depth in [1, 127, 128] {
        let mut calls = vec![&entry[0]];
        calls.extend(std::iter::repeat_n(&recursive[1], depth - 1));
        let old = shared_return_call(
            &g,
            &calls,
            &g.rules["A"].body,
            "choice",
            "x stop",
            100000,
            0,
            false,
        );
        let new = shared_return_call(
            &g,
            &calls,
            &g.rules["A"].body,
            "choice",
            "x stop",
            100000,
            0,
            true,
        );
        assert_eq!(new.0, old.0, "depth{depth}");
        assert_eq!(new.2, old.2);
        if depth == 128 {
            assert!(new.0.0.unwrap_err().contains("depth"));
        }
    }
}
#[test]
fn shared_returns_dangling_graph_and_unproductive_recursion_remain_honest_errors() {
    let g = shared_return_graph();
    for mode in 0..2 {
        let mut changed = g.clone();
        if mode == 0 {
            let p = changed.parser_prediction.as_mut().unwrap();
            let decision = p.decisions["rule/Entry/choice"].entries[0];
            p.states[decision].edges = vec![Edge::Epsilon(p.states.len() + 1)];
        } else {
            let p = changed.parser_prediction.as_mut().unwrap();
            let decision = p.decisions["rule/Entry/choice"].entries[0];
            p.states[decision].edges = vec![Edge::Call {
                target: decision,
                follow: p.rules["Entry"].1,
            }];
        }
        let old = shared_return_call(
            &changed,
            &[],
            &changed.rules["Entry"].body,
            "choice",
            "bad",
            100000,
            0,
            false,
        );
        let new = shared_return_call(
            &changed,
            &[],
            &changed.rules["Entry"].body,
            "choice",
            "bad",
            100000,
            0,
            true,
        );
        assert_eq!(new.0, old.0);
        assert!(new.0.0.is_err());
        assert!(new.2);
    }
}

#[test]
fn persistent_shared_returns_node_push_materialization_and_ceiling_bound_are_prepaid() {
    use super::shared_returns::{Returns, SharedReturns};
    // Portable descriptor/row bounds, not equal physical storage or RSS.
    const DESCRIPTOR_BOUND: usize = 32;
    const COPY_BLOCK_BYTES: usize = 64;
    let shared_bytes = std::mem::size_of::<SharedReturns>();
    let flat_bytes =
        std::mem::size_of::<super::shared_returns::flat_reference::FlatSharedReturns>();
    let vec_bytes = std::mem::size_of::<Vec<usize>>();
    let shared_row_bytes = std::mem::size_of::<super::PredictionConfiguration<SharedReturns>>();
    let flat_row_bytes = std::mem::size_of::<
        super::PredictionConfiguration<super::shared_returns::flat_reference::FlatSharedReturns>,
    >();
    let vec_row_bytes = std::mem::size_of::<super::PredictionConfiguration<Vec<usize>>>();
    assert!(shared_bytes <= DESCRIPTOR_BOUND);
    assert!(flat_bytes <= DESCRIPTOR_BOUND);
    assert!(shared_bytes <= COPY_BLOCK_BYTES);
    assert!(flat_bytes <= COPY_BLOCK_BYTES);
    assert!(shared_row_bytes <= vec_row_bytes);
    println!(
        "PERSISTENT_SHARED_LAYOUT_RAW {}",
        serde_json::json!({"persistent_descriptor_bytes":shared_bytes,
            "flat_descriptor_bytes":flat_bytes,"vec_descriptor_bytes":vec_bytes,
            "persistent_configuration_bytes":shared_row_bytes,
            "flat_configuration_bytes":flat_row_bytes,"vec_configuration_bytes":vec_row_bytes,
            "descriptor_bound_bytes":DESCRIPTOR_BOUND,"one_copy_block_bytes":COPY_BLOCK_BYTES,
            "logical_push_node_bytes":128,"equal_physical_storage_claim":false,"RSS_claim":false})
    );
    let mut work = Some(100000);
    let mut cache = None;
    let mut stack = SharedReturns::seed(&[4, 7], &mut cache, &mut work).unwrap();
    let parent = stack.clone();
    let before = work.unwrap();
    stack.push_return(usize::MAX, &mut work).unwrap();
    assert_eq!(before - work.unwrap(), 6); // alloc1+128bytes2+link1+value1+length1
    assert!(stack.has_shared_parent(&parent));
    let preserved = stack.clone();
    let mut measure = Some(10000);
    let values = stack.materialized(&mut measure).unwrap().into_owned();
    assert_eq!(values, vec![4, 7, usize::MAX]);
    let used = 10000 - measure.unwrap();
    for grant in [0, used - 1, used] {
        let mut bounded = Some(grant);
        let result = stack.materialized(&mut bounded);
        assert_eq!(result.is_ok(), grant == used);
        assert_eq!(bounded, Some(0));
        assert_eq!(stack, preserved);
        if let Ok(value) = result {
            assert_eq!(value.as_ref(), values);
        }
    }
    for grant in 0..=6 {
        let mut actual = parent.clone();
        let mut bounded = Some(grant);
        let result = actual.push_return(9, &mut bounded);
        assert_eq!(result.is_ok(), grant == 6);
        assert_eq!(bounded, Some(0));
        if grant < 6 {
            assert_eq!(actual, parent);
        } else {
            assert!(actual.has_shared_parent(&parent));
        }
    }
    let mut deep = SharedReturns::empty();
    let mut work = Some(10_000_000);
    for n in 0..crate::grammar::MAX_CALL_DEPTH {
        deep.push_return(n, &mut work).unwrap();
    }
    let checkpoint = work;
    let copy = deep.clone();
    assert_eq!(deep.len(), crate::grammar::MAX_CALL_DEPTH);
    assert!(
        deep.push_return(crate::grammar::MAX_CALL_DEPTH, &mut work)
            .unwrap_err()
            .contains("depth")
    );
    assert_eq!(deep, copy);
    assert_eq!(work, checkpoint);
    let mut cache = None;
    assert!(
        SharedReturns::seed(&[0; crate::grammar::MAX_CALL_DEPTH + 1], &mut cache, &mut work)
            .unwrap_err()
            .contains("depth")
    );
    assert!(cache.is_none());
}

#[test]
fn persistent_shared_returns_full_lex_order_ignores_ancestry_and_pointer_identity() {
    use super::shared_returns::{Returns, SharedReturns};
    let mut work = Some(100000);
    let mut cases = Vec::new();
    for values in [
        vec![],
        vec![0],
        vec![0, 0],
        vec![0, 1],
        vec![1],
        vec![1, 0],
        vec![1, 0, 0],
        vec![1, usize::MAX],
        vec![usize::MAX],
        vec![usize::MAX, 0],
    ] {
        for split in 0..=values.len() {
            let mut cached = None;
            let mut actual = SharedReturns::seed(&values[..split], &mut cached, &mut work).unwrap();
            for value in &values[split..] {
                actual.push_return(*value, &mut work).unwrap();
            }
            cases.push((actual.clone(), values.clone()));
            let mut remaining = values.clone();
            while let Some(value) = remaining.pop() {
                let (last, rest) = actual.popped(&mut work, true).unwrap().unwrap();
                assert_eq!(last, value);
                cases.push((rest.clone(), remaining.clone()));
                actual = rest;
            }
        }
    }
    for (a, av) in &cases {
        for (b, bv) in &cases {
            assert_eq!(a.cmp(b), av.cmp(bv));
            assert_eq!(a == b, av == bv);
        }
    }
    let long = (0..128).collect::<Vec<_>>();
    let mut cache = None;
    let flat = SharedReturns::seed(&long, &mut cache, &mut work).unwrap();
    let mut linked = SharedReturns::empty();
    for value in &long {
        linked.push_return(*value, &mut work).unwrap();
    }
    assert_eq!(linked.cmp(&flat), std::cmp::Ordering::Equal);
    let (_, short) = linked.popped(&mut work, true).unwrap().unwrap();
    assert_eq!(short.cmp(&flat), std::cmp::Ordering::Less);
    let actual = cases
        .iter()
        .map(|(a, _)| a.clone())
        .collect::<BTreeSet<_>>();
    let literal = cases
        .iter()
        .map(|(_, a)| a.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual
            .iter()
            .map(|a| a.materialized(&mut work).unwrap().into_owned())
            .collect::<Vec<_>>(),
        literal.into_iter().collect::<Vec<_>>()
    );
}

fn persistent_shared_dag() -> Grammar {
    let mut g = grammar(
        choice(vec![
            sequence(vec![call("Dense0"), keyword("left")]),
            sequence(vec![call("Dense0"), keyword("right")]),
        ]),
        vec![],
    );
    let template = g.rules["Entry"].clone();
    for level in 0..7 {
        let mut rule = template.clone();
        rule.body = choice(vec![
            sequence(vec![
                call(&format!("Dense{}", level + 1)),
                keyword(&format!("a{level}")),
            ]),
            sequence(vec![
                call(&format!("Dense{}", level + 1)),
                keyword(&format!("b{level}")),
            ]),
        ]);
        g.rules.insert(format!("Dense{level}"), rule);
    }
    let mut leaf = template;
    leaf.body = keyword("x");
    g.rules.insert("Dense7".into(), leaf);
    live_compile(&mut g);
    g
}

fn persistent_shared_worker<R: super::shared_returns::Returns>(
    g: &Grammar,
    text: &str,
    grant: usize,
    fail_read: bool,
) -> (PortsResult, usize) {
    let program = g.parser_prediction.as_ref().unwrap();
    let root_stop = program.rules["Entry"].1;
    let eof = program.states[root_stop]
        .follow
        .iter()
        .copied()
        .find(|n| {
            matches!(
                program.states[*n].edges.as_slice(),
                [Edge::Token(PredictionSymbol::End, _)]
            )
        })
        .unwrap();
    let seed = super::live_callers::LiveSeed {
        scope: None,
        scopes: None,
        returns: &[],
        root_stop,
        eof,
    };
    let decision = &program.decisions["rule/Entry/choice"];
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    input.fail_read = fail_read;
    let mut work = Some(grant);
    let result = program
        .run_prediction_with_return_storage::<R, _>(
            PredictionGoal::Decision {
                decision,
                recursive_first: program.recursive_decisions.contains("rule/Entry/choice"),
                boundary: None,
            },
            &mut input,
            &mut work,
            0,
            CompletionVerification::Legacy,
            None,
            &mut super::runtime_source::NoInitialSource,
            Some(&seed),
        )
        .and_then(|value| match value {
            PredictionOutcome::Selected(value) => Ok(value),
            _ => Err("Unexpected completion in authored differential control".into()),
        });
    (
        (
            result,
            input.position,
            input.furthest,
            input.events.into_inner(),
        ),
        grant - work.unwrap(),
    )
}

#[test]
fn persistent_shared_returns_same_canonical_worker_preserves_whole_result_and_reduces_actual_copy_work()
 {
    use super::shared_returns::{SharedReturns, flat_reference::FlatSharedReturns};
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    for (text, fail) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x b6 b5 b4 b3 b2 b1 b0 right", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let old = persistent_shared_worker::<FlatSharedReturns>(&g, text, 100000, fail);
        let actual = persistent_shared_worker::<SharedReturns>(&g, text, 100000, fail);
        println!(
            "PERSISTENT_SHARED_SAME_WORKER {}",
            serde_json::json!({
            "text":text,"cancelled":fail,"flat_whole":old.0,"actual_whole":actual.0,
            "flat_actual_work":old.1,"persistent_actual_work":actual.1,
            "grammar_unchanged":true,"original_project_gain":false})
        );
        assert_eq!(actual.0, old.0);
        assert!(
            actual.1 < old.1,
            "cold actual worker cost includes every new node fee"
        );
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
}

#[test]
fn persistent_shared_returns_actual_caller_exact_short_preconsumed_and_cancellation_keep_scope() {
    let g = live_component_grammar(4);
    let expression = &g.rules["Prefix"].body;
    let calls = [live_component_call(&g)];
    let (whole, used, poisoned) = shared_return_call(
        &g,
        &calls,
        expression,
        "repeat",
        "public next",
        100000,
        0,
        true,
    );
    assert!(whole.0.is_ok() && !poisoned);
    for grant in [0, used - 1, used] {
        let (actual, charged, poisoned) = shared_return_call(
            &g,
            &calls,
            expression,
            "repeat",
            "public next",
            grant,
            0,
            true,
        );
        assert_eq!(charged, grant);
        if grant == used {
            assert_eq!(actual, whole);
            assert!(!poisoned);
        } else {
            assert!(actual.0.is_err());
            assert!(poisoned);
        }
    }
    let next = shared_return_call(
        &g,
        &calls,
        expression,
        "repeat",
        "public next",
        used + 11,
        11,
        true,
    );
    assert_eq!(next.0, whole);
    assert_eq!(next.1, used + 11);
    assert!(!next.2);
    let source = runtime_source_id();
    let mut work = Some(100000);
    let mut scope = LiveCallerPrediction::new(&g, &source, &mut work).unwrap();
    let tokens = symbols("public next");
    let mut input = RuntimeSourceInput::new(&tokens);
    input.fail_read = true;
    let error =
        scope.predict_shared_returns(&source, &calls, expression, "repeat", &mut input, &mut work);
    assert!(
        error
            .unwrap_err()
            .contains("Authored cancellation at actual input read")
    );
    assert!(scope.is_poisoned());
    let count = input.events.borrow().len();
    assert!(
        scope
            .predict_shared_returns(&source, &calls, expression, "repeat", &mut input, &mut work)
            .is_err()
    );
    assert_eq!(input.events.borrow().len(), count);
}

// Same persistent carrier and every old fee, with only ownership transfer off.
// This is a differential adapter over the same worker, not a second predictor.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CloneSingletonShared(super::shared_returns::SharedReturns);
impl super::shared_returns::Returns for CloneSingletonShared {
    type Visited = std::collections::BTreeSet<crate::grammar::parser_prediction::PredictionConfiguration<Self>>;
    const SHARED: bool = true;
    fn empty() -> Self {
        Self(super::shared_returns::SharedReturns::empty())
    }
    fn from_owned(value: Vec<usize>) -> Result<Self, String> {
        super::shared_returns::SharedReturns::from_owned(value).map(Self)
    }
    fn seed(
        value: &[usize],
        cached: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        let mut inner = cached.take().map(|value| value.0);
        let result = super::shared_returns::SharedReturns::seed(value, &mut inner, work).map(Self);
        *cached = inner.map(Self);
        result
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn materialized(
        &self,
        work: &mut Option<usize>,
    ) -> Result<std::borrow::Cow<'_, [usize]>, String> {
        self.0.materialized(work)
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String> {
        self.0.copy_fee(work)
    }
    fn popped(
        &self,
        work: &mut Option<usize>,
        metered: bool,
    ) -> Result<Option<(usize, Self)>, String> {
        self.0
            .popped(work, metered)
            .map(|value| value.map(|(state, rest)| (state, Self(rest))))
    }
    fn push_return(&mut self, value: usize, work: &mut Option<usize>) -> Result<(), String> {
        self.0.push_return(value, work)
    }
}

#[test]
fn shared_returns_singleton_move_same_worker_whole_callbacks_and_real_clone_work() {
    use super::shared_returns::SharedReturns;
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    let program = g.parser_prediction.as_ref().unwrap();
    assert!(
        program
            .states
            .iter()
            .any(|state| matches!(state.edges.as_slice(), [Edge::Call { .. }]))
    );
    assert!(program.states.iter().any(|state| state.edges.len() > 1));
    for (text, cancel) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x b6 b5 b4 b3 b2 b1 b0 right", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let old = persistent_shared_worker::<CloneSingletonShared>(&g, text, 100000, cancel);
        let actual = persistent_shared_worker::<SharedReturns>(&g, text, 100000, cancel);
        let flat = persistent_shared_worker::<
            super::shared_returns::flat_reference::FlatSharedReturns,
        >(&g, text, 100000, cancel);
        println!(
            "SINGLETON_SHARED_MOVE_WHOLE {}",
            serde_json::json!({
            "text":text,"cancelled":cancel,"old_whole":old.0,"actual_whole":actual.0,
            "flat_whole":flat.0,"old_persistent_work":old.1,"actual_work":actual.1,
            "flat_work":flat.1,"original_project_gain":false})
        );
        assert_eq!(actual.0, old.0);
        assert_eq!(actual.0, flat.0);
        assert!(
            actual.1 < old.1,
            "actual singleton descriptor clones removed"
        );
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
}

#[test]
fn shared_returns_singleton_move_first_full_action_and_multiedge_keep_exact_outcomes() {
    use super::shared_returns::SharedReturns;
    for (full, action) in [(false, false), (false, true), (true, false)] {
        let mut guarded = expr(Kind::SyntacticPredicate(Box::new(sequence(vec![
            keyword("x"),
            keyword("y"),
        ]))));
        guarded.first_set_predicated = !full;
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
        let before = serde_json::to_vec(&g).unwrap();
        for text in ["x y", "x z", "x q", "x", ""] {
            let old = persistent_shared_worker::<CloneSingletonShared>(&g, text, 100000, false);
            let actual = persistent_shared_worker::<SharedReturns>(&g, text, 100000, false);
            assert_eq!(actual.0, old.0, "full={full} action={action} {text}");
            assert!(actual.1 <= old.1);
        }
        assert_eq!(serde_json::to_vec(&g).unwrap(), before);
    }
}

#[test]
fn shared_returns_singleton_move_exact_short_preconsumed_and_cancel_keep_original_pool() {
    use super::shared_returns::SharedReturns;
    let g = persistent_shared_dag();
    let text = "x a6 a5 a4 a3 a2 a1 a0 left";
    let (whole, used) = persistent_shared_worker::<SharedReturns>(&g, text, 100000, false);
    assert!(whole.0.is_ok());
    for grant in [0, used - 1, used] {
        let (actual, charged) = persistent_shared_worker::<SharedReturns>(&g, text, grant, false);
        assert_eq!(charged, grant);
        if grant == used {
            assert_eq!(actual, whole);
        } else {
            assert!(actual.0.unwrap_err().contains("budget"));
        }
    }
    // Exercise prior debit in the actual source/owner-bound live adapter.
    let live = shared_return_graph();
    let expression = &live.rules["Entry"].body;
    let (live_whole, live_used, poisoned) = shared_return_call(
        &live,
        &[],
        expression,
        "choice",
        "word0 left",
        100000,
        0,
        true,
    );
    assert!(!poisoned && live_whole.0.is_ok());
    let (actual, charged, poisoned) = shared_return_call(
        &live,
        &[],
        expression,
        "choice",
        "word0 left",
        live_used + 11,
        11,
        true,
    );
    assert_eq!(actual, live_whole);
    assert_eq!(charged, live_used + 11);
    assert!(!poisoned);
    let old = persistent_shared_worker::<CloneSingletonShared>(&g, text, 100000, true);
    let actual = persistent_shared_worker::<SharedReturns>(&g, text, 100000, true);
    assert_eq!(actual.0, old.0);
    assert!(
        actual
            .0
            .0
            .unwrap_err()
            .contains("Authored cancellation at actual input read")
    );
}

// Exact qualified79b singleton-only policy over the same persistent carrier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SingletonOnlyShared(CloneSingletonShared);
impl super::shared_returns::Returns for SingletonOnlyShared {
    type Visited = std::collections::BTreeSet<crate::grammar::parser_prediction::PredictionConfiguration<Self>>;
    const SHARED: bool = true;
    const MOVE_SINGLE_EDGE: bool = true;
    fn empty() -> Self {
        Self(CloneSingletonShared::empty())
    }
    fn from_owned(value: Vec<usize>) -> Result<Self, String> {
        CloneSingletonShared::from_owned(value).map(Self)
    }
    fn seed(
        value: &[usize],
        cached: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        let mut inner = cached.take().map(|value| value.0);
        let result = CloneSingletonShared::seed(value, &mut inner, work).map(Self);
        *cached = inner.map(Self);
        result
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn materialized(
        &self,
        work: &mut Option<usize>,
    ) -> Result<std::borrow::Cow<'_, [usize]>, String> {
        self.0.materialized(work)
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String> {
        self.0.copy_fee(work)
    }
    fn popped(
        &self,
        work: &mut Option<usize>,
        metered: bool,
    ) -> Result<Option<(usize, Self)>, String> {
        self.0
            .popped(work, metered)
            .map(|value| value.map(|(state, rest)| (state, Self(rest))))
    }
    fn push_return(&mut self, value: usize, work: &mut Option<usize>) -> Result<(), String> {
        self.0.push_return(value, work)
    }
}

#[test]
fn shared_returns_last_edge_same_carrier_singleton_reference_whole_and_true_work() {
    use super::shared_returns::SharedReturns;
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    for (text, cancel) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x b6 b5 b4 b3 b2 b1 b0 right", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let old = persistent_shared_worker::<SingletonOnlyShared>(&g, text, 100000, cancel);
        let actual = persistent_shared_worker::<SharedReturns>(&g, text, 100000, cancel);
        println!(
            "SHARED_LAST_EDGE_SAME_WORKER_WHOLE {}",
            serde_json::json!({
            "text":text,"cancelled":cancel,"singleton_whole":old.0,"actual_whole":actual.0,
            "singleton_work":old.1,"actual_work":actual.1,"project_gain":false})
        );
        assert_eq!(actual.0, old.0);
        assert!(
            actual.1 < old.1,
            "actual last-sibling clone omitted after earlier siblings keep their copies"
        );
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
}

#[test]
fn shared_returns_last_edge_three_siblings_order_exact_short_and_cancel() {
    use super::shared_returns::SharedReturns;
    let mut g = grammar(
        choice(vec![
            sequence(vec![call("Body"), keyword("left")]),
            sequence(vec![call("Body"), keyword("middle")]),
            sequence(vec![call("Body"), keyword("right")]),
        ]),
        vec![(
            "Body",
            choice(vec![
                sequence(vec![keyword("x"), keyword("y")]),
                sequence(vec![keyword("x"), keyword("z")]),
                sequence(vec![keyword("x"), keyword("q")]),
            ]),
        )],
    );
    live_compile(&mut g);
    assert!(
        g.parser_prediction
            .as_ref()
            .unwrap()
            .states
            .iter()
            .any(|state| state.edges.len() >= 3)
    );
    let before = serde_json::to_vec(&g).unwrap();
    for (text, selected) in [("x y left", 0), ("x z middle", 1), ("x q right", 2)] {
        let old = persistent_shared_worker::<SingletonOnlyShared>(&g, text, 100000, false);
        let actual = persistent_shared_worker::<SharedReturns>(&g, text, 100000, false);
        assert_eq!(actual.0, old.0);
        assert_eq!(actual.0.0, Ok(Some(selected)));
    }
    for (text, cancel) in [("x bad", false), ("", false), ("x q right", true)] {
        let old = persistent_shared_worker::<SingletonOnlyShared>(&g, text, 100000, cancel);
        let actual = persistent_shared_worker::<SharedReturns>(&g, text, 100000, cancel);
        assert_eq!(actual.0, old.0);
    }
    let text = "x q right";
    let (whole, used) = persistent_shared_worker::<SharedReturns>(&g, text, 100000, false);
    for grant in [0, used - 1, used] {
        let (actual, charged) = persistent_shared_worker::<SharedReturns>(&g, text, grant, false);
        assert_eq!(charged, grant);
        if grant == used {
            assert_eq!(actual, whole);
        } else {
            assert!(actual.0.unwrap_err().contains("budget"));
        }
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
}

#[test]
fn shared_returns_arena_same_worker_old_rc_flat_whole_callbacks_and_actual_cost() {
    use super::shared_returns::{
        SharedReturns, arena::ArenaReturns, flat_reference::FlatSharedReturns,
    };
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    for (text, cancel) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x b6 b5 b4 b3 b2 b1 b0 right", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let old = persistent_shared_worker::<SharedReturns>(&g, text, 100000, cancel);
        let flat = persistent_shared_worker::<FlatSharedReturns>(&g, text, 100000, cancel);
        let actual = persistent_shared_worker::<ArenaReturns>(&g, text, 100000, cancel);
        println!(
            "ARENA_SHARED_SAME_WORKER_WHOLE {}",
            serde_json::json!({
            "text":text,"cancelled":cancel,"rc_whole":old.0,"flat_whole":flat.0,"actual_whole":actual.0,
            "rc_work":old.1,"flat_work":flat.1,"arena_work":actual.1,"original_project_gain":false})
        );
        assert_eq!(actual.0, old.0);
        assert_eq!(actual.0, flat.0);
        assert!(
            actual.1 < old.1,
            "this large same-worker fixture must repay real growth/copy/header costs"
        );
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
}
#[test]
fn shared_returns_arena_first_full_action_and_finite_worker_results_match_rc() {
    use super::shared_returns::{SharedReturns, arena::ArenaReturns};
    for (full, action) in [(false, false), (false, true), (true, false)] {
        let mut guarded = expr(Kind::SyntacticPredicate(Box::new(sequence(vec![
            keyword("x"),
            keyword("y"),
        ]))));
        guarded.first_set_predicated = !full;
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
        for (text, cancel) in [
            ("x y", false),
            ("x z", false),
            ("x bad", false),
            ("", false),
            ("x y", true),
        ] {
            let old = persistent_shared_worker::<SharedReturns>(&g, text, 100000, cancel);
            let actual = persistent_shared_worker::<ArenaReturns>(&g, text, 100000, cancel);
            assert_eq!(actual.0, old.0, "full{full} action{action} {text}");
        }
    }
}
fn arena_live_call(
    g: &Grammar,
    callers: &[&Expr],
    expression: &Expr,
    text: &str,
    grant: usize,
    preconsumed: usize,
) -> (PortsResult, usize, bool) {
    let source = runtime_source_id();
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    let mut work = Some(grant);
    for _ in 0..preconsumed {
        charge(&mut work).unwrap();
    }
    let mut scope = match LiveCallerPrediction::new(g, &source, &mut work) {
        Ok(scope) => scope,
        Err(error) => return ((Err(error), 0, 0, vec![]), grant - work.unwrap(), true),
    };
    let result = scope.predict_arena_returns(
        &source, callers, expression, "repeat", &mut input, &mut work,
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
fn shared_returns_arena_actual_caller_exact_short_foreign_source_and_cancel_poison() {
    let g = live_component_grammar(4);
    let expression = &g.rules["Prefix"].body;
    let calls = [live_component_call(&g)];
    let old = shared_return_call(
        &g,
        &calls,
        expression,
        "repeat",
        "public next",
        100000,
        0,
        true,
    );
    let (whole, used, failed) = arena_live_call(&g, &calls, expression, "public next", 100000, 0);
    assert_eq!(whole, old.0);
    assert!(whole.0.is_ok() && !failed);
    for grant in [0, used - 1, used] {
        let (actual, charged, failed) =
            arena_live_call(&g, &calls, expression, "public next", grant, 0);
        assert_eq!(charged, grant);
        if grant == used {
            assert_eq!(actual, whole);
            assert!(!failed);
        } else {
            assert!(actual.0.is_err());
            assert!(failed);
        }
    }
    let (actual, charged, failed) =
        arena_live_call(&g, &calls, expression, "public next", used + 9, 9);
    assert_eq!(actual, whole);
    assert_eq!(charged, used + 9);
    assert!(!failed);
    // Reject an oversized entry pool before validation can debit it below the ceiling.
    // Scope construction has its separate setup pool; the rejected prediction never reads input.
    {
        let source = runtime_source_id();
        let mut setup = Some(100000);
        let mut scope = LiveCallerPrediction::new(&g, &source, &mut setup).unwrap();
        let mut oversized = Some(super::shared_returns::arena::WORK_LIMIT + 1);
        let tokens = symbols("public next");
        let mut input = RuntimeSourceInput::new(&tokens);
        let error = scope
            .predict_arena_returns(
                &source,
                &calls,
                expression,
                "repeat",
                &mut input,
                &mut oversized,
            )
            .unwrap_err();
        assert!(error.contains("finite at-most4000000000 entry pool"));
        assert_eq!(
            oversized,
            Some(super::shared_returns::arena::WORK_LIMIT + 1)
        );
        assert!(input.events.borrow().is_empty());
        assert!(scope.is_poisoned());
        assert!(
            scope
                .predict_arena_returns(
                    &source,
                    &calls,
                    expression,
                    "repeat",
                    &mut input,
                    &mut oversized,
                )
                .is_err()
        );
        assert_eq!(
            oversized,
            Some(super::shared_returns::arena::WORK_LIMIT + 1)
        );
        assert!(input.events.borrow().is_empty());
    }
    let source = runtime_source_id();
    for foreign in [false, true] {
        let mut work = Some(100000);
        let mut scope = LiveCallerPrediction::new(&g, &source, &mut work).unwrap();
        let tokens = symbols("public next");
        let mut input = RuntimeSourceInput::new(&tokens);
        input.fail_read = !foreign;
        let wrong = intersect_model::definitions::bundle::SourceId::for_text(
            "memory:foreign-arena-source",
            "public next",
        );
        let result = scope.predict_arena_returns(
            if foreign { &wrong } else { &source },
            &calls,
            expression,
            "repeat",
            &mut input,
            &mut work,
        );
        let error = result.unwrap_err();
        assert!(if foreign {
            error.contains("source differs")
        } else {
            error.contains("Authored cancellation at actual input read")
        });
        assert!(scope.is_poisoned());
        let count = input.events.borrow().len();
        assert!(
            scope
                .predict_arena_returns(&source, &calls, expression, "repeat", &mut input, &mut work)
                .is_err()
        );
        assert_eq!(input.events.borrow().len(), count);
    }
}

// Invocation-borrowed storage controls use the same generic worker and input
// adapter as the retained Rc-Arena oracle; no recognition implementation here.
fn borrowed_arena_worker(
    g: &Grammar,
    text: &str,
    grant: usize,
    fail_read: bool,
) -> (PortsResult, usize) {
    use super::shared_returns::arena::borrowed::{ArenaBank, BorrowedArenaReturns};
    let program = g.parser_prediction.as_ref().unwrap();
    let root_stop = program.rules["Entry"].1;
    let eof = program.states[root_stop]
        .follow
        .iter()
        .copied()
        .find(|n| {
            matches!(
                program.states[*n].edges.as_slice(),
                [Edge::Token(PredictionSymbol::End, _)]
            )
        })
        .unwrap();
    let seed = super::live_callers::LiveSeed {
        scope: None,
        scopes: None,
        returns: &[],
        root_stop,
        eof,
    };
    let decision = &program.decisions["rule/Entry/choice"];
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    input.fail_read = fail_read;
    let mut work = Some(grant);
    let bank = ArenaBank::new();
    let result = program
        .run_prediction_with_return_storage_seeded::<BorrowedArenaReturns<'_>, _>(
            PredictionGoal::Decision {
                decision,
                recursive_first: program.recursive_decisions.contains("rule/Entry/choice"),
                boundary: None,
            },
            &mut input,
            &mut work,
            0,
            CompletionVerification::Legacy,
            None,
            &mut super::runtime_source::NoInitialSource,
            Some(&seed),
            Some(BorrowedArenaReturns::unseeded(&bank)),
        )
        .and_then(|value| match value {
            PredictionOutcome::Selected(value) => Ok(value),
            _ => Err("Unexpected completion in authored borrowed control".into()),
        });
    (
        (
            result,
            input.position,
            input.furthest,
            input.events.into_inner(),
        ),
        grant - work.unwrap(),
    )
}

#[test]
fn shared_returns_borrowed_arena_same_worker_whole_callbacks_guards_and_errors() {
    use super::shared_returns::arena::ArenaReturns;
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    for (text, cancel) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x b6 b5 b4 b3 b2 b1 b0 right", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let old = persistent_shared_worker::<ArenaReturns>(&g, text, 100000, cancel);
        let flat = persistent_shared_worker::<Vec<usize>>(&g, text, 100000, cancel);
        let actual = borrowed_arena_worker(&g, text, 100000, cancel);
        println!(
            "BORROWED_ARENA_SAME_WORKER_WHOLE {}",
            serde_json::json!({"text":text,"cancelled":cancel,"old_arena_whole":old.0,"vec_whole":flat.0,"borrowed_whole":actual.0,"old_arena_work":old.1,"borrowed_work":actual.1,"original_project_gain":false})
        );
        assert_eq!(actual.0, old.0);
        assert_eq!(actual.0, flat.0);
        assert!(
            actual.1 < old.1,
            "actual large-row fixture must repay the real cold bank overhead"
        );
    }
    let (whole, used) = borrowed_arena_worker(&g, "x a6 a5 a4 a3 a2 a1 a0 left", 100000, false);
    for grant in [0, used - 1, used] {
        let (result, charged) =
            borrowed_arena_worker(&g, "x a6 a5 a4 a3 a2 a1 a0 left", grant, false);
        assert_eq!(charged, grant);
        if grant == used {
            assert_eq!(result, whole);
        } else {
            assert!(result.0.unwrap_err().contains("budget"));
        }
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
    for (full, action) in [(false, false), (false, true), (true, false)] {
        let mut guarded = expr(Kind::SyntacticPredicate(Box::new(sequence(vec![
            keyword("x"),
            keyword("y"),
        ]))));
        guarded.first_set_predicated = !full;
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
        for (text, cancel) in [
            ("x y", false),
            ("x z", false),
            ("x bad", false),
            ("", false),
            ("x y", true),
        ] {
            let old = persistent_shared_worker::<ArenaReturns>(&g, text, 100000, cancel);
            let actual = borrowed_arena_worker(&g, text, 100000, cancel);
            assert_eq!(actual.0, old.0, "full{full} action{action} {text}");
        }
    }
    for dangling in [true, false] {
        let mut bad = g.clone();
        let program = bad.parser_prediction.as_mut().unwrap();
        let entry = program.decisions["rule/Entry/choice"].entries[0];
        program.states[entry].edges = if dangling {
            vec![Edge::Epsilon(program.states.len() + 1)]
        } else {
            vec![Edge::Call {
                target: entry,
                follow: program.rules["Entry"].1,
            }]
        };
        let old = persistent_shared_worker::<ArenaReturns>(&bad, "", 100000, false);
        let actual = borrowed_arena_worker(&bad, "", 100000, false);
        assert_eq!(actual.0, old.0);
        assert!(actual.0.0.is_err());
    }
}

#[allow(clippy::too_many_arguments)] // Actual caller/source/budget and old/new carrier controls.
fn borrowed_arena_neutral_run<'g>(
    g: &'g Grammar,
    facts: &PreparedLiveRuntimePredictionContext<'g>,
    callers: &[&Expr],
    expression: &Expr,
    suffix: &str,
    text: &str,
    grant: usize,
    prepaid: usize,
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
    let result = scope.predict_borrowed_arena_returns_with_prepared(
        facts,
        g.hidden.as_deref().unwrap_or(&[]),
        &source,
        callers,
        expression,
        suffix,
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
fn shared_returns_borrowed_arena_actual_neutral_tail_exact_short_ambiguity_and_cancel()
-> Result<(), String> {
    let g = neutral_fixture();
    let before = serde_json::to_vec(&g).map_err(|e| e.to_string())?;
    let facts = neutral_prepare(&g, "rule/Wrapper/choice")?;
    let Kind::Sequence(root) = &g.rules["Entry"].body.kind else {
        return Err("Actual entry sequence".into());
    };
    for (text, cancel) in [
        ("word0 left", false),
        ("word79 right", false),
        ("word0 bad", false),
        ("", false),
        ("word0 left", true),
    ] {
        let old = neutral_run(
            &g,
            Some(&facts),
            &[&root[0]],
            &g.rules["Wrapper"].body,
            "choice",
            text,
            100000,
            0,
            true,
            cancel,
        );
        let actual = borrowed_arena_neutral_run(
            &g,
            &facts,
            &[&root[0]],
            &g.rules["Wrapper"].body,
            "choice",
            text,
            100000,
            0,
            cancel,
        );
        assert_eq!(actual.0, old.0);
        assert_eq!(actual.2, old.2);
        println!(
            "BORROWED_ARENA_NEUTRAL_TAIL_WHOLE {}",
            serde_json::json!({"text":text,"cancelled":cancel,"old_arena_whole":old.0,"borrowed_whole":actual.0,"old_arena_work":old.1,"borrowed_work":actual.1,"nonempty_actual_tail":true,"project_gain":false})
        );
        if text == "word0 left" && !cancel {
            for grant in [0, actual.1 - 1, actual.1] {
                let bounded = borrowed_arena_neutral_run(
                    &g,
                    &facts,
                    &[&root[0]],
                    &g.rules["Wrapper"].body,
                    "choice",
                    text,
                    grant,
                    0,
                    false,
                );
                assert_eq!(bounded.1, grant);
                if grant == actual.1 {
                    assert_eq!(bounded.0, actual.0);
                    assert!(!bounded.2);
                } else {
                    assert!(bounded.0.0.is_err() && bounded.2);
                }
            }
            let prepaid = borrowed_arena_neutral_run(
                &g,
                &facts,
                &[&root[0]],
                &g.rules["Wrapper"].body,
                "choice",
                text,
                actual.1 + 7,
                7,
                false,
            );
            assert_eq!(prepaid.0, actual.0);
            assert_eq!(prepaid.1, actual.1 + 7);
        }
    }
    assert_eq!(serde_json::to_vec(&g).map_err(|e| e.to_string())?, before);
    // A low-level deliberately overlapping graph; native ambiguity admission
    // is not claimed. Actual dynamic ambiguity and callback order remain exact.
    let mut ambiguous = g.clone();
    let program = ambiguous
        .parser_prediction
        .as_mut()
        .ok_or("Actual program")?;
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
    let facts = neutral_prepare(&ambiguous, "rule/Wrapper/choice")?;
    let Kind::Sequence(root) = &ambiguous.rules["Entry"].body.kind else {
        return Err("Actual ambiguous entry".into());
    };
    let old = neutral_run(
        &ambiguous,
        Some(&facts),
        &[&root[0]],
        &ambiguous.rules["Wrapper"].body,
        "choice",
        "word0 left done",
        100000,
        0,
        true,
        false,
    );
    let actual = borrowed_arena_neutral_run(
        &ambiguous,
        &facts,
        &[&root[0]],
        &ambiguous.rules["Wrapper"].body,
        "choice",
        "word0 left done",
        100000,
        0,
        false,
    );
    assert_eq!(actual.0, old.0);
    assert_eq!(actual.2, old.2);
    assert!(actual.0.0.as_ref().is_err_and(|e| e.contains("Ambiguous")));
    Ok(())
}

#[test]
fn shared_returns_borrowed_arena_source_hidden_verification_poison_and_deep_live_bounds()
-> Result<(), String> {
    let mut g = neutral_fixture();
    g.hidden = Some(vec!["WS".into()]);
    let hidden = g.hidden.as_deref().ok_or("Actual hidden policy")?;
    let facts = neutral_prepare(&g, "rule/Wrapper/choice")?;
    let foreign = g.clone();
    let source = runtime_source_id();
    let copied = g.rules["Wrapper"].body.clone();
    let foreign_hidden = hidden.to_vec();
    let meter = NeutralMeter::new(100000);
    let wrong_verification = PreparedLiveRuntimePredictionContext::prepare_selected_observed(
        &g,
        hidden,
        CompletionVerification::InputCheckedCompletion,
        ["rule/Wrapper/choice"],
        &mut Some(100000),
        1048576,
        &meter.context(),
    )?;
    for case in 0..7 {
        let active = if case == 0 { &foreign } else { &g };
        let Kind::Sequence(root) = &active.rules["Entry"].body.kind else {
            return Err("Actual entry".into());
        };
        let tokens = symbols("word0 left");
        let mut input = RuntimeSourceInput::new(&tokens);
        let mut setup = Some(100000);
        let mut scope = LiveCallerPrediction::new(active, &source, &mut setup)?;
        let wrong_source =
            intersect_model::definitions::bundle::SourceId::for_text("memory:foreign-borrowed-input", "x");
        let mut work = if case == 5 {
            Some(super::shared_returns::arena::WORK_LIMIT + 1)
        } else if case == 6 {
            None
        } else {
            Some(100000)
        };
        let before = work;
        let result = scope.predict_borrowed_arena_returns_with_prepared(
            if case == 4 {
                &wrong_verification
            } else {
                &facts
            },
            if case == 3 { &foreign_hidden } else { hidden },
            if case == 1 { &wrong_source } else { &source },
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
        if case >= 5 {
            assert_eq!(work, before, "entry clamp precedes every debit");
        }
        let retry = scope.predict_borrowed_arena_returns_with_prepared(
            &facts,
            hidden,
            &source,
            &[&root[0]],
            &active.rules["Wrapper"].body,
            "choice",
            &mut input,
            &mut work,
        );
        assert!(retry.is_err());
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
    for depth in [1, 127, 128, 129] {
        let mut calls = vec![&entry[0]];
        calls.extend(std::iter::repeat_n(&recur[1], depth - 1));
        let old = neutral_run(
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
        let actual = borrowed_arena_neutral_run(
            &recursive,
            &facts,
            &calls,
            &recursive.rules["A"].body,
            "choice",
            "x stop",
            100000,
            0,
            false,
        );
        assert_eq!(actual.0, old.0, "depth{depth}");
        assert_eq!(actual.2, old.2);
        assert_eq!(actual.0.0.is_err(), depth >= 128);
    }
    Ok(())
}

#[test]
fn shared_returns_runtime_work_policy_one_million_preserves_results_callbacks_and_debits() {
    use super::shared_returns::arena::ArenaReturns;
    assert_eq!(super::shared_returns::arena::WORK_LIMIT, 4_000_000_000);
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    for (text, cancel) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let regression = borrowed_arena_worker(&g, text, 100000, cancel);
        let configured = borrowed_arena_worker(&g, text, 1_000_000, cancel);
        let rc = persistent_shared_worker::<ArenaReturns>(&g, text, 1_000_000, cancel);
        assert_eq!(
            configured, regression,
            "larger ceiling cannot change charged work or callbacks"
        );
        assert_eq!(configured.0, rc.0);
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
    let g = live_component_grammar(4);
    let calls = [live_component_call(&g)];
    let expression = &g.rules["Prefix"].body;
    let regression = arena_live_call(&g, &calls, expression, "public next", 100000, 0);
    let configured = arena_live_call(&g, &calls, expression, "public next", 1_000_000, 0);
    assert_eq!(configured, regression);
    let used = configured.1;
    for grant in [0, used - 1, used] {
        let actual = arena_live_call(&g, &calls, expression, "public next", grant, 0);
        assert_eq!(actual.1, grant);
        if grant == used {
            assert_eq!(actual.0, configured.0);
            assert!(!actual.2);
        } else {
            assert!(actual.0.0.is_err());
            assert!(actual.2);
        }
    }
    let same = arena_live_call(&g, &calls, expression, "public next", used + 9, 9);
    assert_eq!(same.0, configured.0);
    assert_eq!(same.1, used + 9);
    assert!(!same.2);
}

#[test]
fn shared_returns_runtime_work_policy_ten_million_retains_small_grants_and_real_debits() {
    use super::shared_returns::arena::ArenaReturns;
    assert_eq!(super::shared_returns::arena::WORK_LIMIT, 4_000_000_000);
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    for (text, cancel) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let hundred_k = borrowed_arena_worker(&g, text, 100000, cancel);
        let million = borrowed_arena_worker(&g, text, 1_000_000, cancel);
        let configured = borrowed_arena_worker(&g, text, 10_000_000, cancel);
        let rc = persistent_shared_worker::<ArenaReturns>(&g, text, 10_000_000, cancel);
        assert_eq!(configured, hundred_k);
        assert_eq!(configured, million);
        assert_eq!(configured.0, rc.0);
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
    let g = live_component_grammar(4);
    let calls = [live_component_call(&g)];
    let expression = &g.rules["Prefix"].body;
    let regression = arena_live_call(&g, &calls, expression, "public next", 100000, 0);
    let configured = arena_live_call(&g, &calls, expression, "public next", 10_000_000, 0);
    assert_eq!(configured, regression);
    let used = configured.1;
    for grant in [0, used - 1, used] {
        let actual = arena_live_call(&g, &calls, expression, "public next", grant, 0);
        assert_eq!(actual.1, grant);
        if grant == used {
            assert_eq!(actual.0, configured.0);
            assert!(!actual.2);
        } else {
            assert!(actual.0.0.is_err());
            assert!(actual.2);
        }
    }
    let preconsumed = arena_live_call(&g, &calls, expression, "public next", used + 9, 9);
    assert_eq!(preconsumed.0, configured.0);
    assert_eq!(preconsumed.1, used + 9);
    assert!(!preconsumed.2);
}

#[test]
fn shared_returns_full_library_finite_ceiling_preserves_debits_callbacks_and_small_grants() {
    use super::shared_returns::arena::ArenaReturns;
    assert_eq!(super::shared_returns::arena::WORK_LIMIT, 4_000_000_000);
    let g = persistent_shared_dag();
    let before = serde_json::to_vec(&g).unwrap();
    for (text, cancel) in [
        ("x a6 a5 a4 a3 a2 a1 a0 left", false),
        ("x wrong", false),
        ("", false),
        ("x a6", true),
    ] {
        let old = borrowed_arena_worker(&g, text, 10_000_000, cancel);
        let retained_250m = borrowed_arena_worker(&g, text, 250_000_000, cancel);
        let high = borrowed_arena_worker(&g, text, 4_000_000_000, cancel);
        let shared = persistent_shared_worker::<ArenaReturns>(&g, text, 4_000_000_000, cancel);
        assert_eq!(old, retained_250m);
        assert_eq!(retained_250m, high);
        assert_eq!(high.0, shared.0);
    }
    assert_eq!(serde_json::to_vec(&g).unwrap(), before);
    let g = live_component_grammar(4);
    let calls = [live_component_call(&g)];
    let expr = &g.rules["Prefix"].body;
    let retained_250m = arena_live_call(&g, &calls, expr, "public next", 250_000_000, 0);
    let high = arena_live_call(&g, &calls, expr, "public next", 4_000_000_000, 0);
    assert_eq!(high, retained_250m);
    const {
        assert!(4_000_000_001u64 <= u32::MAX as u64);
    }
    assert_eq!(
        high,
        arena_live_call(&g, &calls, expr, "public next", 10_000_000, 0)
    );
    let used = high.1;
    for grant in [0, used - 1, used] {
        let actual = arena_live_call(&g, &calls, expr, "public next", grant, 0);
        assert_eq!(actual.1, grant);
        if grant == used {
            assert_eq!(actual.0, high.0);
            assert!(!actual.2);
        } else {
            assert!(actual.0.0.is_err());
            assert!(actual.2);
        }
    }
    let pre = arena_live_call(&g, &calls, expr, "public next", used + 9, 9);
    assert_eq!(pre.0, high.0);
    assert_eq!(pre.1, used + 9);
    assert!(!pre.2);
    // Scope construction legitimately spends work. Test the actual prediction
    // ENTRY boundary with its own untouched oversized grant, as the retained
    // arena caller-boundary control does; do not debit it below the maximum.
    let source = runtime_source_id();
    let mut setup = Some(100_000);
    let mut scope = LiveCallerPrediction::new(&g, &source, &mut setup).unwrap();
    let tokens = symbols("public next");
    let mut input = RuntimeSourceInput::new(&tokens);
    let mut above = Some(4_000_000_001);
    let error = scope
        .predict_arena_returns(&source, &calls, expr, "repeat", &mut input, &mut above)
        .unwrap_err();
    assert!(error.contains("finite at-most4000000000 entry pool"));
    assert_eq!(above, Some(4_000_000_001));
    assert!(input.events.borrow().is_empty());
    assert!(scope.is_poisoned());
}
