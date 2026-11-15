fn ordinary_return_worker<R: super::shared_returns::Returns>(
    program: &ParserPrediction,
    path: &str,
    text: &str,
    grant: usize,
    fail_read: bool,
) -> (PortsResult, usize) {
    let tokens = symbols(text);
    let mut input = RuntimeSourceInput::new(&tokens);
    input.fail_read = fail_read;
    let mut work = Some(grant);
    let result = program.run_prediction_with_return_storage::<R, _>(
        PredictionGoal::Decision {
            decision: &program.decisions[path],
            recursive_first: program.recursive_decisions.contains(path),
            boundary: None,
        },
        &mut input,
        &mut work,
        0,
        CompletionVerification::Legacy,
        None,
        &mut super::runtime_source::NoInitialSource,
        None,
    ).and_then(|outcome| match outcome {
        PredictionOutcome::Selected(selected) => Ok(selected),
        _ => Err("Unexpected authored ordinary prediction outcome".into()),
    });
    ((result, input.position, input.furthest, input.events.into_inner()), grant - work.unwrap())
}

#[test]
fn ordinary_return_prefixes_keep_values_hashes_and_fork_independence() {
    use super::ordinary_returns::OrdinaryReturns;
    use super::shared_returns::Returns;
    use std::hash::{Hash, Hasher};
    let mut work = None;
    for length in 0..=crate::grammar::MAX_CALL_DEPTH {
        let values = (0..length).collect::<Vec<_>>();
        let original = OrdinaryReturns::from_owned(values.clone()).unwrap();
        let mut current = original.clone();
        let mut literal = values.clone();
        loop {
            assert_eq!(current.materialized(&mut work).unwrap().as_ref(), literal);
            let mut actual_hash = rustc_hash::FxHasher::default();
            let mut expected_hash = rustc_hash::FxHasher::default();
            current.hash(&mut actual_hash);
            literal.hash(&mut expected_hash);
            assert_eq!(actual_hash.finish(), expected_hash.finish());
            assert_eq!(current.cmp(&original), literal.cmp(&values));
            if let Some((&last, rest)) = literal.split_last() {
                let (target, prefix) = current.popped(&mut work, false).unwrap().unwrap();
                assert_eq!(target, last);
                let mut fork = prefix.clone();
                fork.push_return(usize::MAX, &mut work).unwrap();
                assert_eq!(fork.materialized(&mut work).unwrap().as_ref(), [rest, &[usize::MAX]].concat());
                assert_eq!(prefix.materialized(&mut work).unwrap().as_ref(), rest);
                assert_eq!(original.materialized(&mut work).unwrap().as_ref(), values);
                current = prefix;
                literal.pop();
            } else {
                assert!(current.popped(&mut work, false).unwrap().is_none());
                // Unique ownership can reuse a formerly longer backing vector.
                current.push_return(7, &mut work).unwrap();
                assert_eq!(current.materialized(&mut work).unwrap().as_ref(), [7]);
                break;
            }
        }
    }
}

#[test]
fn ordinary_prediction_matches_vec_at_every_budget_input_failure_and_follow_fanout() {
    use super::ordinary_returns::OrdinaryReturns;
    let mut g = grammar(
        choice(vec![
            sequence(vec![call("Deep0"), keyword("left")]),
            sequence(vec![call("Deep0"), keyword("right")]),
        ]),
        vec![],
    );
    let template = g.rules["Entry"].clone();
    for level in 0..8 {
        let mut rule = template.clone();
        rule.body = if level == 7 { keyword("x") } else { call(&format!("Deep{}", level + 1)) };
        g.rules.insert(format!("Deep{level}"), rule);
    }
    let deep = ParserPrediction::compile(&g).unwrap();
    let fanout = runtime_source_program();
    for (program, path, texts) in [
        (&deep, "rule/Entry/choice", vec!["x left", "x right", "x wrong", "", "x"]),
        (&fanout, "rule/Prefix/repeat", vec!["public next", "private after17", "protected", "next", "wrong", ""]),
    ] {
        let unchanged = serde_json::to_vec(program).unwrap();
        for text in texts {
            for fail_read in [false, true] {
                let baseline = ordinary_return_worker::<Vec<usize>>(program, path, text, 100000, fail_read);
                assert_eq!(ordinary_return_worker::<OrdinaryReturns>(program, path, text, 100000, fail_read), baseline);
                for grant in 0..=baseline.1 + 1 {
                    assert_eq!(
                        ordinary_return_worker::<OrdinaryReturns>(program, path, text, grant, fail_read),
                        ordinary_return_worker::<Vec<usize>>(program, path, text, grant, fail_read),
                        "{path}, {text:?}, grant {grant}, cancellation {fail_read}",
                    );
                }
                // Exercise the actual ordinary dispatcher as well as the two
                // representations of the same worker above.
                let tokens = symbols(text);
                let mut input = RuntimeSourceInput::new(&tokens);
                input.fail_read = fail_read;
                let mut work = Some(100000);
                let result = program.predict_with_replay_input(path, &mut input, &mut work);
                assert_eq!((result,input.position,input.furthest,input.events.into_inner()),baseline.0);
                assert_eq!(100000-work.unwrap(),baseline.1);
            }
        }
        assert_eq!(serde_json::to_vec(program).unwrap(), unchanged);
    }
}
