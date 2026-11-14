//! Authored same-worker full-pair storage controls; no original admission award.
use super::adjacent::ClosureQueue;
use super::analysis_calls::{Calls, SharedCalls, VecCalls};
use super::transparent_ports::{BorrowedComponentPortQueue, SharedCallPortQueue};
use super::*;

fn shared_calls_node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
#[allow(clippy::needless_range_loop)] // Graph indices are authored call targets.
fn shared_calls_graph(depth: usize, width: usize) -> ParserPrediction {
    let mut states = vec![State::default(); depth + width + 6];
    let end_a = states.len() - 2;
    let end_b = states.len() - 1;
    states[0] = shared_calls_node(vec![Edge::Call {
        target: 2,
        follow: end_a,
    }]);
    states[1] = shared_calls_node(vec![Edge::Call {
        target: 2,
        follow: end_b,
    }]);
    for i in 2..depth + 2 {
        states[i] = shared_calls_node(vec![Edge::Call {
            target: i + 1,
            follow: end_a,
        }]);
    }
    let fan = depth + 2;
    states[fan] = shared_calls_node((0..width).map(|n| Edge::Epsilon(fan + 1 + n)).collect());
    for n in 0..width {
        states[fan + 1 + n] = shared_calls_node(vec![Edge::Token(
            PredictionSymbol::Keyword(format!("word{n}")),
            end_a,
        )]);
    }
    states[end_a].rule_stop = true;
    states[end_b].rule_stop = true;
    ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states,
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    }
}
fn shared_calls_run<'g, C: Calls, Q: ClosureQueue<'g, C>>(
    p: &'g ParserPrediction,
    work: &mut usize,
) -> (Result<bool, String>, BTreeMap<(usize, usize), bool>) {
    let initial = [
        AnalysisConfiguration {
            state: 0,
            alternative: 0,
            calls: C::empty(),
        },
        AnalysisConfiguration {
            state: 1,
            alternative: 1,
            calls: C::empty(),
        },
    ]
    .into();
    let mut witnesses = BTreeMap::new();
    let result=p.multiple_recursive_configurations_with_call_storage::<C,Q,frontier::AdoptFrontiers,true,partial_token_probe::NoProbe>(
        initial,work,true,&mut witnesses,false,(true,None,None,None),&mut partial_token_probe::NoProbe,None);
    (result, witnesses)
}
#[test]
fn shared_compiler_calls_first_cold_full_tail_work_and_outcomes_match_vector_worker() {
    let p = shared_calls_graph(6, 32);
    let before = serde_json::to_vec(&p).unwrap();
    let (mut a, mut b) = (10000, 10000);
    let old = shared_calls_run::<VecCalls, BorrowedComponentPortQueue<'_>>(&p, &mut a);
    let new = shared_calls_run::<SharedCalls, SharedCallPortQueue<'_>>(&p, &mut b);
    println!(
        "SHARED_COMPILER_CALLS_COLD {:?}",
        (&old, &new, 10000 - a, 10000 - b)
    );
    assert_eq!(new, old);
    assert_eq!(new.0, Ok(false));
    assert!(
        b > a,
        "Complete first-cold scope/header/Call storage/snapshots/port rows/return/frontier work must improve on this actual graph"
    );
    assert_eq!(serde_json::to_vec(&p).unwrap(), before);
}
#[test]
fn shared_compiler_calls_exact_order_pairs_real_sharing_and_move_pop_are_preserved()
-> Result<(), String> {
    let mut work = 10000;
    let mut scope = SharedCalls::scope(&mut work)?;
    let mut actual = BTreeSet::new();
    let mut expected = BTreeSet::new();
    for values in [
        vec![],
        vec![(0, 1)],
        vec![(0, 2)],
        vec![(0, 1), (9, 7)],
        vec![(0, 1), (9, 8)],
        vec![(1, 0)],
    ] {
        let mut calls = SharedCalls::empty();
        for pair in &values {
            calls.pushed(*pair, &mut scope, &mut work)?;
        }
        calls.snapshot_copy_fee(&mut scope, &mut work)?;
        let duplicate = calls.clone();
        if let (SharedCalls::Prefix { storage: a, .. }, SharedCalls::Prefix { storage: b, .. }) =
            (&calls, &duplicate)
        {
            assert!(std::rc::Rc::ptr_eq(a, b));
        }
        actual.insert(duplicate);
        expected.insert(values.clone());
        let mut literal = values;
        while let Some(pair) = literal.pop() {
            let owned = match &calls {
                SharedCalls::Prefix { storage, .. } => Some((
                    std::rc::Rc::as_ptr(storage),
                    std::rc::Rc::strong_count(storage),
                )),
                SharedCalls::Empty => None,
            };
            let (got, rest) = calls
                .popped(&mut scope, &mut work)?
                .ok_or("Missing actual paired frame")?;
            assert_eq!(got, pair);
            assert!(calls.is_empty());
            if let SharedCalls::Prefix { storage, .. } = &rest {
                let (pointer, count) = owned.ok_or("Missing real Rc allocation")?;
                assert_eq!(std::rc::Rc::as_ptr(storage), pointer);
                assert_eq!(std::rc::Rc::strong_count(storage), count);
            }
            calls = rest;
            assert_eq!(&*calls, literal.as_slice());
        }
    }
    assert_eq!(
        actual.into_iter().map(|r| r.to_vec()).collect::<Vec<_>>(),
        expected.into_iter().collect::<Vec<_>>()
    );
    Ok(())
}
#[test]
fn shared_compiler_calls_original_zero_exact_short_preconsumed_and_fresh_scope_are_real() {
    let p = shared_calls_graph(4, 12);
    let mut full = 10000;
    let expected = shared_calls_run::<SharedCalls, SharedCallPortQueue<'_>>(&p, &mut full);
    assert!(expected.0.is_ok());
    let used = 10000 - full;
    for grant in [0, 1, used - 1, used] {
        let mut work = grant;
        let actual = shared_calls_run::<SharedCalls, SharedCallPortQueue<'_>>(&p, &mut work);
        assert_eq!(work, 0);
        if grant == used {
            assert_eq!(actual, expected);
        } else {
            assert_eq!(actual.0, Err("Grammar prediction budget exceeded".into()));
        }
    }
    let mut work = used + 7;
    for _ in 0..7 {
        analysis_charge(&mut work).unwrap();
    }
    assert_eq!(
        shared_calls_run::<SharedCalls, SharedCallPortQueue<'_>>(&p, &mut work),
        expected
    );
    assert_eq!(work, 0);
    let mut work = 10000;
    let mut scope = SharedCalls::scope(&mut work).unwrap();
    let mut calls = SharedCalls::empty();
    let mut zero = 0;
    assert_eq!(
        calls.pushed((1, 2), &mut scope, &mut zero),
        Err("Grammar prediction budget exceeded".into())
    );
    assert!(calls.is_empty());
    assert_eq!(
        calls.pushed((1, 2), &mut scope, &mut work),
        Err("Shared compiler call storage is poisoned".into())
    );
    let mut fresh = SharedCalls::scope(&mut work).unwrap();
    calls.pushed((1, 2), &mut fresh, &mut work).unwrap();
    assert_eq!(&*calls, &[(1, 2)]);
}
#[test]
fn shared_compiler_calls_same_site_recursion_depth128_and_dangling_graph_stay_canonical() {
    for depth in [127, 128, 129] {
        let p = shared_calls_graph(depth - 1, 2);
        let (mut a, mut b) = (100000, 100000);
        let x = shared_calls_run::<VecCalls, BorrowedComponentPortQueue<'_>>(&p, &mut a);
        let y = shared_calls_run::<SharedCalls, SharedCallPortQueue<'_>>(&p, &mut b);
        assert_eq!(x, y);
        assert_eq!(
            y.0,
            if depth <= 128 {
                Ok(false)
            } else {
                Err("Parser prediction call depth exceeded".into())
            }
        );
    }
    for mode in [0, 1] {
        let mut p = shared_calls_graph(2, 3);
        p.states[2].edges = if mode == 0 {
            vec![Edge::Epsilon(p.states.len() + 1)]
        } else {
            vec![Edge::Call {
                target: 2,
                follow: p.states.len() - 1,
            }]
        };
        let (mut a, mut b) = (10000, 10000);
        let x = shared_calls_run::<VecCalls, BorrowedComponentPortQueue<'_>>(&p, &mut a);
        let y = shared_calls_run::<SharedCalls, SharedCallPortQueue<'_>>(&p, &mut b);
        assert_eq!(x, y);
        if mode == 0 {
            assert_eq!(y.0, Err("Dangling parser prediction state".into()));
        }
    }
}
#[test]
#[allow(clippy::let_unit_value)] // Compare both storage types through the same scope API.
fn shared_compiler_calls_default_layout_and_selected_retention_denial_are_explicit()
-> Result<(), String> {
    // Default Vec operations keep their historical literal zero-fee route.
    let mut literal = vec![(1, 2), (3, 4)];
    let mut zero = 0;
    let mut literal_scope = VecCalls::scope(&mut zero)?;
    literal.snapshot_copy_fee(&mut literal_scope, &mut zero)?;
    assert_eq!(
        literal.popped(&mut literal_scope, &mut zero)?,
        Some(((3, 4), vec![(1, 2)]))
    );
    assert_eq!(literal, vec![(1, 2), (3, 4)]);
    literal.pushed((5, 6), &mut literal_scope, &mut zero)?;
    assert_eq!(literal, vec![(1, 2), (3, 4), (5, 6)]);
    assert_eq!(zero, 0);
    assert_eq!(
        std::mem::size_of::<AnalysisConfiguration>(),
        std::mem::size_of::<AnalysisConfiguration<VecCalls>>()
    );
    assert!(std::mem::size_of::<AnalysisConfiguration<SharedCalls>>() <= 48);
    let mut work = 10000;
    let mut scope = SharedCalls::scope(&mut work)?;
    scope.at_retention_boundary_for_test();
    let mut calls = SharedCalls::empty();
    assert_eq!(
        calls.pushed((1, 2), &mut scope, &mut work),
        Err("Shared compiler call storage retention bound exceeded".into())
    );
    assert!(scope.failed_for_test() && calls.is_empty());
    Ok(())
}
