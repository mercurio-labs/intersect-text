//! Authored source assertions, unexecuted until the frozen packet is reviewed.
use super::adjacent::{AdjacentQueue, ClosureQueue};
use super::*;

type AnalysisResult = (Result<bool, String>, usize, BTreeMap<(usize, usize), bool>);

fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn row(state: usize, alternative: usize) -> AnalysisConfiguration {
    AnalysisConfiguration {
        state,
        alternative,
        calls: vec![(40 + alternative, 80 + alternative)],
    }
}
fn place<'graph, Q: ClosureQueue<'graph>>(
    queue: &mut Q,
    scheduled: &mut BTreeSet<AnalysisConfiguration>,
    remaining: &mut usize,
    value: AnalysisConfiguration,
) -> Result<(), String> {
    Q::schedule(
        value,
        queue,
        scheduled,
        remaining,
        (true, ScheduleSite::Seed, None),
    )
}
fn drain<'graph, Q: ClosureQueue<'graph>>(queue: &mut Q) -> Vec<AnalysisConfiguration> {
    let mut rows = Vec::new();
    while let Some(row) = queue.pop_front() {
        rows.push(row);
    }
    rows
}
fn program(states: Vec<State>) -> ParserPrediction {
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
fn analyze<'graph, Q: ClosureQueue<'graph>>(
    p: &'graph ParserPrediction,
    entries: &[usize],
    budget: usize,
    shared: bool,
) -> AnalysisResult {
    let mut remaining = budget;
    let mut witnesses = BTreeMap::new();
    let result = p.multiple_recursive_alternatives_with_queue::<Q>(
        &Decision {
            entries: entries.to_vec(),
            exit: None,
        },
        &mut remaining,
        shared,
        &mut witnesses,
        true,
        (true, None, None),
    );
    (result, remaining, witnesses)
}

#[test]
fn adjacent_queue_unfolds_exact_fifo_and_entire_contexts() -> Result<(), String> {
    let graph = vec![State::default(); 5];
    let mut queue = AdjacentQueue::empty(&graph);
    let mut closed = BTreeSet::new();
    let mut work = 20;
    let expected = vec![
        row(0, 0),
        row(0, 1),
        row(2, 2),
        row(0, 3),
        row(0, 4),
        row(3, 5),
    ];
    for value in expected.clone() {
        place(&mut queue, &mut closed, &mut work, value)?;
    }
    assert_eq!(work, 12); // Six exact keys, two paid overflow promotions.
    assert_eq!(drain(&mut queue), expected);
    assert_eq!(closed.len(), 6);
    Ok(())
}

#[test]
fn transparent_single_edge_uses_literal_fees_and_moves_call_vectors() -> Result<(), String> {
    for edge in [
        Edge::Epsilon(1),
        Edge::Action(1),
        Edge::First {
            path: "unbound-classifier-metadata".into(),
            target: 1,
        },
        Edge::Full { target: 1, stop: 2 },
    ] {
        let graph = vec![node(vec![edge]), State::default(), State::default()];
        let before = serde_json::to_value(&graph).map_err(|e| e.to_string())?;
        let mut queue = AdjacentQueue::empty(&graph);
        let mut closed = BTreeSet::new();
        let mut work = 11;
        let input: Vec<_> = (0..4).map(|a| row(0, a)).collect();
        let pointers: Vec<_> = input.iter().map(|r| r.calls.as_ptr() as usize).collect();
        for value in input {
            place(&mut queue, &mut closed, &mut work, value)?;
        }
        assert_eq!(work, 6);
        let first = queue.pop_front().ok_or("Missing paid first context")?;
        assert!(
            queue
                .share_transparent(first, &mut closed, &mut work, None)?
                .is_none()
        );
        assert_eq!(work, 0);
        assert_eq!(closed.len(), 8);
        let output = drain(&mut queue);
        assert_eq!(output, (0..4).map(|a| row(1, a)).collect::<Vec<_>>());
        assert_eq!(
            output
                .iter()
                .map(|r| r.calls.as_ptr() as usize)
                .collect::<Vec<_>>(),
            pointers
        );
        assert_eq!(
            serde_json::to_value(&graph).map_err(|e| e.to_string())?,
            before
        );
    }
    Ok(())
}

#[test]
fn shape_and_output_denials_keep_literal_partial_keys_without_refunds() -> Result<(), String> {
    let graph = vec![node(vec![Edge::Epsilon(1)]), State::default()];
    // Seed4keys+one overflow fee=5. Shared output needs shape1+4keys+overflow1=6.
    for (total, keys) in [(5, 4), (6, 4), (7, 5), (8, 5), (9, 6), (10, 7), (11, 8)] {
        let mut queue = AdjacentQueue::empty(&graph);
        let mut closed = BTreeSet::new();
        let mut work = total;
        for a in 0..4 {
            place(&mut queue, &mut closed, &mut work, row(0, a))?;
        }
        let first = queue.pop_front().ok_or("Missing fee-test first context")?;
        let result = queue.share_transparent(first, &mut closed, &mut work, None);
        if total == 11 {
            assert_eq!(result, Ok(None));
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
        }
        assert_eq!(work, 0);
        assert_eq!(closed.len(), keys);
        if total == 8 {
            assert!(closed.contains(&row(1, 0)));
            assert!(!closed.contains(&row(1, 1)));
        }
    }
    Ok(())
}

#[test]
fn key_and_overflow_reservations_precede_key_publication() -> Result<(), String> {
    let graph = vec![State::default()];
    let mut queue = AdjacentQueue::empty(&graph);
    let mut closed = BTreeSet::new();
    let mut work = 1;
    place(&mut queue, &mut closed, &mut work, row(0, 0))?;
    assert_eq!(work, 0);
    assert_eq!(
        place(&mut queue, &mut closed, &mut work, row(0, 1)),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(closed.len(), 1);
    // A separate ordinary invocation proves the next unit is an overflow fee,
    // using one unchanged finite pool from initial admission through denial.
    let mut second = AdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut allowance = 2;
    place(&mut second, &mut keys, &mut allowance, row(0, 0))?;
    assert_eq!(
        place(&mut second, &mut keys, &mut allowance, row(0, 1)),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(allowance, 0);
    assert_eq!(keys, BTreeSet::from([row(0, 0)]));
    assert_eq!(drain(&mut second), vec![row(0, 0)]);
    Ok(())
}

#[test]
fn nontransparent_shapes_fallback_once_and_keep_later_tail_appends() -> Result<(), String> {
    let shapes = vec![
        node(vec![Edge::Token(PredictionSymbol::End, 1)]),
        node(vec![Edge::Call {
            target: 1,
            follow: 2,
        }]),
        node(vec![Edge::Epsilon(1), Edge::Epsilon(2)]),
        State {
            rule_stop: true,
            ..State::default()
        },
        State {
            accepting: true,
            edges: vec![Edge::Epsilon(1)],
            ..State::default()
        },
        State::default(),
    ];
    for shape in shapes {
        let graph = vec![shape, State::default(), State::default()];
        let mut queue = AdjacentQueue::empty(&graph);
        let mut keys = BTreeSet::new();
        let mut work = 30;
        for a in 0..3 {
            place(&mut queue, &mut keys, &mut work, row(0, a))?;
        }
        let first = queue.pop_front().ok_or("Missing fallback first context")?;
        let before = work;
        assert_eq!(
            queue.share_transparent(first.clone(), &mut keys, &mut work, None)?,
            Some(first)
        );
        assert_eq!(work, before - 1);
        place(&mut queue, &mut keys, &mut work, row(0, 3))?;
        let after_append = work;
        for a in 1..4 {
            let next = queue.pop_front().ok_or("Missing fallback sibling")?;
            assert_eq!(next, row(0, a));
            assert_eq!(
                queue.share_transparent(next.clone(), &mut keys, &mut work, None)?,
                Some(next)
            );
        }
        assert_eq!(work, after_append);
        assert!(queue.pop_front().is_none());
    }
    Ok(())
}

#[test]
fn exact_sibling_count_excludes_already_queued_target_and_new_successors() -> Result<(), String> {
    let graph = vec![
        node(vec![Edge::Epsilon(1)]),
        State::default(),
        State::default(),
    ];
    let mut queue = AdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut work = 40;
    for value in [row(0, 0), row(0, 1), row(2, 8), row(1, 9)] {
        place(&mut queue, &mut keys, &mut work, value)?;
    }
    let first = queue.pop_front().ok_or("Missing prefix first context")?;
    assert!(
        queue
            .share_transparent(first, &mut keys, &mut work, None)?
            .is_none()
    );
    assert_eq!(
        drain(&mut queue),
        vec![row(2, 8), row(1, 9), row(1, 0), row(1, 1)]
    );
    Ok(())
}

#[test]
fn transparent_self_loop_only_consumes_original_siblings_and_dedups() -> Result<(), String> {
    let graph = vec![node(vec![Edge::Epsilon(0)])];
    let mut queue = AdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut work = 6;
    for a in 0..4 {
        place(&mut queue, &mut keys, &mut work, row(0, a))?;
    }
    let first = queue.pop_front().ok_or("Missing loop first context")?;
    assert!(
        queue
            .share_transparent(first, &mut keys, &mut work, None)?
            .is_none()
    );
    assert_eq!(work, 0);
    assert_eq!(keys.len(), 4);
    assert!(queue.pop_front().is_none());
    Ok(())
}

#[test]
fn dangling_state_and_unreserved_internal_mode_fail_explicitly() -> Result<(), String> {
    let graph = vec![State::default()];
    let mut queue = AdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut work = 4;
    for a in 0..2 {
        place(&mut queue, &mut keys, &mut work, row(9, a))?;
    }
    let first = queue.pop_front().ok_or("Missing dangling first context")?;
    assert_eq!(
        queue.share_transparent(first, &mut keys, &mut work, None),
        Err("Dangling parser prediction state".into())
    );
    assert_eq!(work, 0);
    assert!(
        matches!(AdjacentQueue::unreserved(BTreeSet::<AnalysisConfiguration>::new(),&graph),Err(message) if message=="Adjacent closure batching requires exact-key reservations")
    );
    Ok(())
}

#[test]
fn shared_classifier_utility_counts_all_growth_shape_and_empty_fallback_fees() {
    let p = program(vec![
        node(vec![Edge::Epsilon(1)]),
        node(vec![Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Action(4)]),
        node(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 5)]),
        State::default(),
    ]);
    let old = analyze::<VecDeque<AnalysisConfiguration>>(&p, &[0, 0, 0, 0], 46, false);
    let new = analyze::<AdjacentQueue<'_>>(&p, &[0, 0, 0, 0], 46, false);
    assert_eq!(old, (Ok(false), 0, BTreeMap::new()));
    assert_eq!(new, (Ok(false), 4, BTreeMap::new()));
    assert_eq!(
        analyze::<AdjacentQueue<'_>>(&p, &[0, 0, 0, 0], 42, false),
        (Ok(false), 0, BTreeMap::new())
    );
    assert_eq!(
        analyze::<AdjacentQueue<'_>>(&p, &[0, 0, 0, 0], 41, false).0,
        Err("Grammar prediction budget exceeded".into())
    );
}

#[test]
fn calls_returns_shared_witness_and_early_errors_keep_original_results() {
    let cases = vec![
        program(vec![
            node(vec![Edge::Call {
                target: 2,
                follow: 4,
            }]),
            node(vec![Edge::Call {
                target: 2,
                follow: 5,
            }]),
            node(vec![Edge::Call {
                target: 2,
                follow: 3,
            }]),
            State {
                rule_stop: true,
                ..State::default()
            },
            State::default(),
            State::default(),
        ]),
        program(vec![
            node(vec![Edge::Call {
                target: 2,
                follow: 4,
            }]),
            node(vec![Edge::Call {
                target: 2,
                follow: 5,
            }]),
            node(vec![Edge::Token(PredictionSymbol::Keyword("a".into()), 3)]),
            State {
                rule_stop: true,
                ..State::default()
            },
            node(vec![Edge::Token(PredictionSymbol::Keyword("b".into()), 6)]),
            node(vec![Edge::Token(PredictionSymbol::Keyword("c".into()), 6)]),
            State::default(),
        ]),
        program(vec![
            node(vec![Edge::Epsilon(999)]),
            node(vec![Edge::Epsilon(999)]),
        ]),
    ];
    for p in cases {
        for shared in [false, true] {
            let old = analyze::<VecDeque<AnalysisConfiguration>>(&p, &[0, 1], 10000, shared);
            let new = analyze::<AdjacentQueue<'_>>(&p, &[0, 1], 10000, shared);
            assert_eq!(old.0, new.0);
            assert_eq!(old.2, new.2);
        }
    }
}

#[test]
fn repeated_context_call_128_and_129_keep_original_guard_before_reuse() {
    for count in [128, 129] {
        let mut states = vec![State::default(); count + 3];
        for (at, state) in states.iter_mut().enumerate().take(count) {
            state.edges.push(Edge::Call {
                target: at + 1,
                follow: count + 1,
            });
        }
        states[count].edges.push(Edge::Token(
            PredictionSymbol::Keyword("x".into()),
            count + 2,
        ));
        let p = program(states);
        let old = analyze::<VecDeque<AnalysisConfiguration>>(&p, &[0, 0], 10000, false);
        let new = analyze::<AdjacentQueue<'_>>(&p, &[0, 0], 10000, false);
        assert_eq!(old.0, new.0);
        assert_eq!(old.2, new.2);
        if count == 129 {
            assert_eq!(new.0, Err("Parser prediction call depth exceeded".into()));
        } else {
            assert_eq!(new.0, Ok(false));
        }
    }
}

#[test]
fn early_shared_witness_returns_before_a_later_actual_dangling_context() {
    // The first two distinct incoming Calls prove recursion; the queued third
    // alternative is deliberately dangling. Batching must not inspect it early.
    let p = program(vec![
        node(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        node(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        node(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        State::default(),
    ]);
    let old = analyze::<VecDeque<AnalysisConfiguration>>(&p, &[0, 1, 999], 100, true);
    let new = analyze::<AdjacentQueue<'_>>(&p, &[0, 1, 999], 100, true);
    assert_eq!(old.0, Ok(true));
    assert_eq!(new.0, Ok(true));
    assert_eq!(new.2, old.2);
    assert_eq!(new.2, BTreeMap::from([((2, 128), true)]));
    // These three singleton source states cannot batch, so even the literal
    // successful and one-short work/partial witness outcomes remain equal.
    let exact = 100 - old.1;
    assert_eq!(
        analyze::<AdjacentQueue<'_>>(&p, &[0, 1, 999], exact, true),
        analyze::<VecDeque<AnalysisConfiguration>>(&p, &[0, 1, 999], exact, true)
    );
    assert_eq!(
        analyze::<AdjacentQueue<'_>>(&p, &[0, 1, 999], exact - 1, true),
        analyze::<VecDeque<AnalysisConfiguration>>(&p, &[0, 1, 999], exact - 1, true)
    );
    assert_eq!(
        analyze::<AdjacentQueue<'_>>(&p, &[0, 1, 999], exact - 1, true).0,
        Err("Grammar prediction budget exceeded".into())
    );
}

#[test]
fn zero_denial_and_duplicate_at_zero_preserve_original_key_order() -> Result<(), String> {
    let graph = vec![State::default()];
    let mut queue = AdjacentQueue::empty(&graph);
    let mut closed = BTreeSet::new();
    let mut work = 0;
    assert_eq!(
        place(&mut queue, &mut closed, &mut work, row(0, 0)),
        Err("Grammar prediction budget exceeded".into())
    );
    assert!(closed.is_empty());
    assert!(queue.pop_front().is_none());
    let mut one = 1;
    place(&mut queue, &mut closed, &mut one, row(0, 0))?;
    place(&mut queue, &mut closed, &mut one, row(0, 0))?;
    assert_eq!(one, 0);
    assert_eq!(closed.len(), 1);
    assert_eq!(drain(&mut queue), vec![row(0, 0)]);
    Ok(())
}

// New source-only controls. These observe the SAME existing queue/classifier.
fn observed_source() -> intersect_model::definitions::bundle::SourceId {
    intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:adjacent-work-control.xtext",
        intersect_model::definitions::bundle::SourceKind::Xtext,
        "Root: ->'a';",
    )
    .id
}
#[test]
fn adjacent_work_queue_literal_eleven_zero_short_and_recovery() -> Result<(), String> {
    use super::super::admission_observation::RunStage;
    let graph = vec![
        node(vec![Edge::Epsilon(1)]),
        node(vec![Edge::Token(PredictionSymbol::Keyword("a".into()), 2)]),
        State::default(),
    ];
    let run = |grant,
               exact: Option<&AdjacentWorkObservation>|
     -> (Result<Vec<AnalysisConfiguration>, String>, usize) {
        let mut remaining = grant;
        let result = (|| {
            let mut queue = AdjacentQueue::empty(&graph);
            let mut keys = BTreeSet::new();
            for alternative in 0..4 {
                AdjacentQueue::schedule_observed(
                    row(0, alternative),
                    &mut queue,
                    &mut keys,
                    &mut remaining,
                    (true, ScheduleSite::Seed, None),
                    exact,
                )?;
            }
            let first = queue.pop_front().ok_or("Missing original first context")?;
            assert!(
                queue
                    .share_transparent_observed(first, &mut keys, &mut remaining, None, exact)?
                    .is_none()
            );
            Ok(drain(&mut queue))
        })();
        (result, remaining)
    };
    for grant in [0, 10, 11, 11] {
        let observed = AdjacentWorkObservation::new();
        observed.begin(RunStage::Executable, &observed_source(), grant);
        let actual = run(grant, Some(&observed));
        observed.end(actual.1, actual.0.is_ok());
        assert_eq!(actual, run(grant, None));
        let snapshot = observed.snapshot();
        let last = snapshot.last.ok_or("Missing observed program")?;
        assert!(last.conserved && !snapshot.inconsistent && !snapshot.overflowed);
        assert_eq!(last.granted.iter().sum::<u64>(), (grant - actual.1) as u64);
        if grant == 11 {
            let expected: Vec<_> = (0..4).map(|alternative| row(1, alternative)).collect();
            assert_eq!(actual, (Ok(expected), 0));
            assert_eq!(last.granted[AdjacentCharge::KeySeed as usize], 4);
            assert_eq!(last.granted[AdjacentCharge::KeySuccessor as usize], 4);
            assert_eq!(last.granted[AdjacentCharge::OverflowGrowth as usize], 2);
            assert_eq!(last.granted[AdjacentCharge::ShapeInspection as usize], 1);
            assert_eq!(
                last.denied,
                [0; super::super::admission_observation::ADJACENT_CHARGE_COUNT]
            );
            assert_eq!(
                (snapshot.transparent_shapes, snapshot.fallback_shapes),
                (1, 0)
            );
        } else {
            assert_eq!(last.denied.iter().sum::<u64>(), 1);
            assert_eq!(
                last.denied[if grant == 0 {
                    AdjacentCharge::KeySeed
                } else {
                    AdjacentCharge::KeySuccessor
                } as usize],
                1
            );
        }
    }
    Ok(())
}
#[test]
fn adjacent_work_same_full_results_remaining_witnesses_and_old_counters() -> Result<(), String> {
    use super::super::admission_observation::RunStage;
    let p = program(vec![
        node(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        node(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        node(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        State::default(),
    ]);
    let before = serde_json::to_value(&p).map_err(|e| e.to_string())?;
    for grant in 0..=20 {
        let run = |exact: Option<&AdjacentWorkObservation>, old: &RecursiveAnalysisCounters| {
            let mut remaining = grant;
            let mut witnesses = BTreeMap::new();
            old.begin_program(RunStage::Executable, grant);
            let result = p
                .multiple_recursive_alternatives_with_queue_observed::<AdjacentQueue<'_>>(
                    &Decision {
                        entries: vec![0, 1],
                        exit: None,
                    },
                    &mut remaining,
                    true,
                    &mut witnesses,
                    true,
                    (true, Some(old), None, exact),
                );
            old.end_program(remaining, result.is_ok());
            (result, remaining, witnesses, old.snapshot())
        };
        let exact = AdjacentWorkObservation::new();
        exact.begin(RunStage::Executable, &observed_source(), grant);
        let a = run(Some(&exact), &RecursiveAnalysisCounters::new());
        exact.end(a.1, a.0.is_ok());
        let b = run(None, &RecursiveAnalysisCounters::new());
        assert_eq!(a, b);
        let s = exact.snapshot();
        assert!(s.last.ok_or("Missing pool result")?.conserved && !s.inconsistent);
        assert_eq!(serde_json::to_value(&p).map_err(|e| e.to_string())?, before);
    }
    // Existing public observer and queue/context/batch holders do not retain a sidecar.
    assert_eq!(
        std::mem::size_of::<super::super::admission_observation::AnalysisObservers<'_>>(),
        2 * std::mem::size_of::<usize>()
    );
    Ok(())
}
#[test]
fn adjacent_work_denied_shape_has_no_guessed_edge_kind() -> Result<(), String> {
    use super::super::admission_observation::RunStage;
    for edge in [
        Edge::Token(PredictionSymbol::Keyword("a".into()), 1),
        Edge::Call {
            target: 1,
            follow: 1,
        },
    ] {
        let graph = vec![node(vec![edge]), State::default()];
        let exact = AdjacentWorkObservation::new();
        exact.begin(RunStage::Executable, &observed_source(), 3);
        let mut remaining = 3;
        let mut queue = AdjacentQueue::empty(&graph);
        let mut keys = BTreeSet::new();
        for alternative in 0..2 {
            AdjacentQueue::schedule_observed(
                row(0, alternative),
                &mut queue,
                &mut keys,
                &mut remaining,
                (true, ScheduleSite::Seed, None),
                Some(&exact),
            )?;
        }
        assert_eq!(remaining, 0);
        let first = queue.pop_front().ok_or("Missing context")?;
        assert_eq!(
            queue.share_transparent_observed(first, &mut keys, &mut remaining, None, Some(&exact)),
            Err("Grammar prediction budget exceeded".into())
        );
        exact.end(remaining, false);
        let s = exact.snapshot();
        assert_eq!(s.shape_edges, [0; 6]);
        assert_eq!((s.transparent_shapes, s.fallback_shapes), (0, 0));
        assert_eq!(
            s.last.ok_or("Missing pool")?.denied[AdjacentCharge::ShapeInspection as usize],
            1
        );
        assert!(s.last.ok_or("Missing pool")?.conserved);
    }
    Ok(())
}
