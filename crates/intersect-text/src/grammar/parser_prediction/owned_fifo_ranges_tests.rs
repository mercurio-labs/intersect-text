use super::super::{
    ParserPrediction, PredictionSymbol, analysis_calls::Calls, frontier, partial_token_probe,
};
use super::*;

type Literal<'g> = Queue<'g, BorrowedCompletedComponents, SharedCalls>;
type Projection = (usize, usize, Vec<(usize, usize)>);
type RowsAndKeys = (Vec<Projection>, BTreeSet<Projection>);
fn project(row: AnalysisConfiguration<SharedCalls>) -> Projection {
    (row.state, row.alternative, row.calls.to_vec())
}
fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn graph() -> Vec<State> {
    let mut graph = vec![State::default(); 96];
    graph[0] = node((2..34).map(Edge::Epsilon).collect());
    graph[1] = node((2..34).map(Edge::Action).collect());
    for state in graph.iter_mut().take(34).skip(2) {
        *state = node(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 60)]);
    }
    graph[60].rule_stop = true;
    graph
}
fn tail(reversed: bool, work: &mut usize) -> Result<SharedCalls, String> {
    let mut scope = SharedCalls::scope(work)?;
    let mut calls = SharedCalls::empty();
    for pair in if reversed {
        [(5, 71), (4, 70)]
    } else {
        [(4, 70), (5, 71)]
    } {
        calls.pushed(pair, &mut scope, work)?;
    }
    Ok(calls)
}
fn schedule<'g, Q: ClosureQueue<'g, SharedCalls>>(
    queue: &mut Q,
    keys: &mut BTreeSet<AnalysisConfiguration<SharedCalls>>,
    state: usize,
    alternative: usize,
    work: &mut usize,
) -> Result<(), String> {
    <BTreeSet<_> as ClosureKeys<SharedCalls>>::schedule(
        AnalysisConfiguration {
            state,
            alternative,
            calls: SharedCalls::empty(),
        },
        queue,
        keys,
        work,
        (true, ScheduleSite::Successor, None),
        None,
    )
}
fn interleaved<'g, Q: ClosureQueue<'g, SharedCalls>>(
    graph: &'g [State],
    work: &mut usize,
) -> Result<RowsAndKeys, String> {
    let mut queue = Q::empty(graph);
    let mut keys = BTreeSet::new();
    schedule(&mut queue, &mut keys, 80, 90, work)?;
    let a = tail(false, work)?;
    let b = tail(true, work)?;
    // One duplicate candidate must not enter the FIFO range. The equal states
    // with two unequal complete tails still remain distinct actual full keys.
    keys.insert(AnalysisConfiguration {
        state: 4,
        alternative: 7,
        calls: a.clone(),
    });
    assert!(
        queue
            .share_keys_observed(
                AnalysisConfiguration {
                    state: 0,
                    alternative: 7,
                    calls: a
                },
                &mut keys,
                work,
                None,
                None
            )?
            .is_none()
    );
    schedule(&mut queue, &mut keys, 81, 91, work)?;
    assert!(
        queue
            .share_keys_observed(
                AnalysisConfiguration {
                    state: 1,
                    alternative: 7,
                    calls: b
                },
                &mut keys,
                work,
                None,
                None
            )?
            .is_none()
    );
    let mut output = Vec::new();
    while let Some(row) = queue.pop_front_with_work(work, None)? {
        let after_first_range_row = row.state == 2 && row.calls.first() == Some(&(4, 70));
        output.push(project(row));
        if after_first_range_row {
            schedule(&mut queue, &mut keys, 82, 92, work)?;
        }
    }
    Ok((output, keys.into_iter().map(project).collect()))
}

#[test]
fn owned_range_fifo_preserves_eager_full_keys_interleaving_duplicates_and_ordered_tails_with_less_completed_work()
-> Result<(), String> {
    let graph = graph();
    let (mut a, mut b) = (100_000, 100_000);
    let old = interleaved::<Literal<'_>>(&graph, &mut a);
    let new = interleaved::<SharedCallPortQueue<'_>>(&graph, &mut b);
    println!(
        "OWNED_RANGE_FIFO_PREASSERT {:?}",
        (&old, &new, 100_000 - a, 100_000 - b)
    );
    assert_eq!(old, new);
    let (output, keys) = new?;
    assert_eq!(output.len(), 66);
    assert_eq!(keys.len(), 67);
    assert_eq!((output[0].0, output[32].0, output[65].0), (80, 81, 82));
    assert_eq!(output.iter().filter(|row| row.0 == 4).count(), 1);
    assert!(
        b > a,
        "packed owned state IDs must remove actual completed canonical work"
    );
    Ok(())
}

#[test]
fn owned_range_partial_failure_keeps_exact_eager_key_prefix_and_no_foreign_queue_rows()
-> Result<(), String> {
    let graph = graph();
    let mut exact = 100_000;
    let mut queue = SharedCallPortQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    assert!(
        queue
            .share_keys_observed(
                AnalysisConfiguration {
                    state: 0,
                    alternative: 3,
                    calls: SharedCalls::empty()
                },
                &mut keys,
                &mut exact,
                None,
                None
            )?
            .is_none()
    );
    let used = 100_000 - exact;
    assert_eq!(keys.len(), 32);
    // Repeat with the actual exact boundary, including a caller that already
    // consumed work. No fresh or replenished budget may reach the producer.
    for preconsumed in [0, 7] {
        let mut queue = SharedCallPortQueue::empty(&graph);
        let mut keys = BTreeSet::new();
        let mut work = used + preconsumed;
        for _ in 0..preconsumed {
            analysis_charge(&mut work)?;
        }
        let result = queue.share_keys_observed(
            AnalysisConfiguration {
                state: 0,
                alternative: 3,
                calls: SharedCalls::empty(),
            },
            &mut keys,
            &mut work,
            None,
            None,
        );
        assert!(result?.is_none());
        assert_eq!(work, 0);
        assert_eq!(keys.len(), 32);
    }
    for allowance in 0..used {
        let mut queue = SharedCallPortQueue::empty(&graph);
        let mut keys = BTreeSet::new();
        let mut work = allowance;
        let result = queue.share_keys_observed(
            AnalysisConfiguration {
                state: 0,
                alternative: 3,
                calls: SharedCalls::empty(),
            },
            &mut keys,
            &mut work,
            None,
            None,
        );
        assert!(result.is_err(), "allowance={allowance}");
        assert_eq!(work, 0, "allowance={allowance}");
        let expected: Vec<_> = keys.iter().cloned().map(project).collect();
        let mut observed = Vec::new();
        let mut inspection = 100_000;
        while let Some(row) = queue.pop_front_with_work(&mut inspection, None)? {
            observed.push(project(row));
        }
        assert_eq!(
            observed, expected,
            "eager accepted prefix allowance={allowance}"
        );
        assert!(queue.summaries.stats().logical_bytes <= MAX_LOGICAL_BYTES);
    }
    Ok(())
}

#[test]
fn owned_range_actual_paid_pop_denial_is_sticky_and_fresh_scope_recovers() -> Result<(), String> {
    let graph = graph();
    for successful_pops in [0, 1, 17, 31] {
        let mut queue = SharedCallPortQueue::empty(&graph);
        let mut keys = BTreeSet::new();
        let mut work = 100_000;
        queue.share_keys_observed(
            AnalysisConfiguration {
                state: 0,
                alternative: 3,
                calls: SharedCalls::empty(),
            },
            &mut keys,
            &mut work,
            None,
            None,
        )?;
        for expected in 2..2 + successful_pops {
            assert_eq!(
                queue
                    .pop_front_with_work(&mut work, None)?
                    .map(|row| row.state),
                Some(expected)
            );
        }
        let mut none = 0;
        assert!(
            queue
                .pop_front_with_work(&mut none, None)
                .unwrap_err()
                .contains("budget")
        );
        assert!(
            queue
                .pop_front_with_work(&mut work, None)
                .unwrap_err()
                .contains("poisoned")
        );
    }
    let mut work = 100_000;
    assert!(interleaved::<SharedCallPortQueue<'_>>(&graph, &mut work).is_ok());
    Ok(())
}

#[test]
fn owned_range_unpaid_facade_cannot_materialize_a_caller_or_discard_a_paid_range_as_success()
-> Result<(), String> {
    let graph = graph();
    let mut queue = SharedCallPortQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut work = 100_000;
    queue.share_keys_observed(
        AnalysisConfiguration {
            state: 0,
            alternative: 3,
            calls: SharedCalls::empty(),
        },
        &mut keys,
        &mut work,
        None,
        None,
    )?;
    assert!(queue.pop_front().is_none());
    assert!(
        queue
            .pop_front_with_work(&mut work, None)
            .unwrap_err()
            .contains("poisoned")
    );
    // The actual classifier uses the paid interface; this private legacy method
    // is never a range capability and cannot be called by a second recognizer.
    Ok(())
}

fn prediction(states: Vec<State>) -> ParserPrediction {
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
fn classify<'g, Q: ClosureQueue<'g, SharedCalls>>(
    p: &'g ParserPrediction,
    work: &mut usize,
    counters: &RecursiveAnalysisCounters,
) -> (Result<bool, String>, BTreeMap<(usize, usize), bool>) {
    counters.begin_program(
        crate::grammar::admission_observation::RunStage::Executable,
        *work,
    );
    let initial = [
        AnalysisConfiguration {
            state: 0,
            alternative: 0,
            calls: SharedCalls::empty(),
        },
        AnalysisConfiguration {
            state: 1,
            alternative: 1,
            calls: SharedCalls::empty(),
        },
    ]
    .into();
    let mut witnesses = BTreeMap::new();
    let result=p.multiple_recursive_configurations_with_call_storage::<SharedCalls,Q,frontier::AdoptFrontiers,true,partial_token_probe::NoProbe>(initial,work,true,&mut witnesses,false,(true,Some(counters),None,None),&mut partial_token_probe::NoProbe,None);
    counters.end_program(*work, result.is_ok());
    (result, witnesses)
}
#[test]
fn owned_range_real_canonical_classifier_preserves_whole_result_witnesses_and_seven_work_counters()
-> Result<(), String> {
    for recursive in [false, true] {
        let mut states = graph();
        if recursive {
            states[60] = node(vec![Edge::Call {
                target: 61,
                follow: 62,
            }]);
            states[61] = node(vec![Edge::Token(PredictionSymbol::Keyword("r".into()), 63)]);
            states[63] = node(vec![Edge::Call {
                target: 61,
                follow: 62,
            }]);
            states[62].rule_stop = true;
        }
        let p = prediction(states);
        let (mut a, mut b) = (100_000, 100_000);
        let x = RecursiveAnalysisCounters::new();
        let y = RecursiveAnalysisCounters::new();
        let old = classify::<Literal<'_>>(&p, &mut a, &x);
        let new = classify::<SharedCallPortQueue<'_>>(&p, &mut b, &y);
        println!(
            "OWNED_RANGE_CANONICAL_PREASSERT {:?}",
            (
                &old,
                &new,
                100_000 - a,
                100_000 - b,
                x.snapshot(),
                y.snapshot()
            )
        );
        assert_eq!(old, new);
        assert_eq!(new.0, Ok(recursive));
        assert!(b > a);
        // Root grouping changes only physical queue ownership. Actual canonical
        // transitions, full keys, witness outcomes and old seven debits stay equal.
        assert_eq!(x.snapshot().stages, y.snapshot().stages);
        assert!(!x.snapshot().overflowed && !x.snapshot().inconsistent);
        assert!(!y.snapshot().overflowed && !y.snapshot().inconsistent);
        assert!(x.snapshot().stages[2].work.closure_pops > 32);
    }
    Ok(())
}

#[test]
fn owned_range_cumulative_capacity_keeps_original_one_mib_bound_and_no_summary_reference_escape()
-> Result<(), String> {
    let graph = graph();
    let mut queue = SharedCallPortQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut work = 10_000_000;
    let mut stopped = false;
    for alternative in 0..2000 {
        let result = queue.share_keys_observed(
            AnalysisConfiguration {
                state: 0,
                alternative,
                calls: SharedCalls::empty(),
            },
            &mut keys,
            &mut work,
            None,
            None,
        );
        if let Err(error) = result {
            assert!(error.contains("retention bound"));
            stopped = true;
            break;
        }
    }
    assert!(stopped);
    assert!(queue.summaries.stats().logical_bytes <= MAX_LOGICAL_BYTES);
    assert!(queue.summaries.poisoned);
    // Existing queued IDs are owned, not references into the now-invalidated map.
    assert!(queue.pop_front_with_work(&mut work, None)?.is_some());
    Ok(())
}

#[test]
fn owned_range_sorted_state_action_ports_keep_real_eager_duplicate_checks() -> Result<(), String> {
    let mut graph = graph();
    graph[0] = node(
        (2..34)
            .flat_map(|state| [Edge::Action(state), Edge::Epsilon(state)])
            .collect(),
    );
    fn run<'g, Q: ClosureQueue<'g, SharedCalls>>(
        graph: &'g [State],
        work: &mut usize,
    ) -> Result<RowsAndKeys, String> {
        let mut queue = Q::empty(graph);
        let mut keys = BTreeSet::new();
        let counts = RecursiveAnalysisCounters::new();
        counts.begin_program(
            crate::grammar::admission_observation::RunStage::Executable,
            *work,
        );
        queue.share_keys_observed(
            AnalysisConfiguration {
                state: 0,
                alternative: 6,
                calls: SharedCalls::empty(),
            },
            &mut keys,
            work,
            Some(&counts),
            None,
        )?;
        let mut rows = Vec::new();
        while let Some(row) = queue.pop_front_with_work(work, None)? {
            rows.push(project(row));
        }
        counts.end_program(*work, true);
        let observed = counts.snapshot();
        assert_eq!(
            observed
                .stages
                .iter()
                .map(|stage| stage.work.scheduling[3][2])
                .sum::<u64>(),
            32
        );
        Ok((rows, keys.into_iter().map(project).collect()))
    }
    let (mut a, mut b) = (100_000, 100_000);
    let old = run::<Literal<'_>>(&graph, &mut a);
    let new = run::<SharedCallPortQueue<'_>>(&graph, &mut b);
    println!(
        "OWNED_RANGE_STATE_ACTION_PREASSERT {:?}",
        (&old, &new, 100_000 - a, 100_000 - b)
    );
    assert_eq!(old, new);
    let (rows, keys) = new?;
    assert_eq!(rows.len(), 32);
    assert_eq!(keys.len(), 32);
    assert_eq!(
        rows.iter().map(|row| row.0).collect::<Vec<_>>(),
        (2..34).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn owned_range_warm_and_none_cycle_opaque_paths_preserve_exact_fallbacks() -> Result<(), String> {
    for mode in 0..3 {
        let mut graph = graph();
        if mode == 1 {
            graph[0].edges.push(Edge::Epsilon(0));
        }
        if mode == 2 {
            graph[0] = node(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 60)]);
        }
        let mut queue = SharedCallPortQueue::empty(&graph);
        let mut old = Literal::empty(&graph);
        let (mut a, mut b) = (100_000, 100_000);
        let mut old_keys = BTreeSet::new();
        let mut new_keys = BTreeSet::new();
        for _ in 0..2 {
            let row = AnalysisConfiguration {
                state: 0,
                alternative: 5,
                calls: SharedCalls::empty(),
            };
            let x = old.share_keys_observed(row.clone(), &mut old_keys, &mut a, None, None)?;
            let y = queue.share_keys_observed(row, &mut new_keys, &mut b, None, None)?;
            assert_eq!(x, y);
            assert_eq!(old_keys, new_keys);
        }
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        while let Some(row) = old.pop_front_with_work(&mut a, None)? {
            xs.push(project(row));
        }
        while let Some(row) = queue.pop_front_with_work(&mut b, None)? {
            ys.push(project(row));
        }
        assert_eq!(xs, ys);
        if mode != 2 {
            assert!(queue.summaries.stats().hits > 0);
        } else {
            assert_eq!(queue.summaries.stats().logical_bytes, 0);
        }
    }
    Ok(())
}

#[test]
fn owned_range_reuses_original_4096_full_key_and_128_depth_guards() -> Result<(), String> {
    let mut graph = vec![State::default(); MAX_KEYS + 3];
    graph[0] = node((1..=MAX_KEYS + 1).map(Edge::Epsilon).collect());
    for state in graph.iter_mut().skip(1) {
        *state = node(vec![Edge::Token(
            PredictionSymbol::Keyword("x".into()),
            MAX_KEYS + 2,
        )]);
    }
    let (mut a, mut b) = (1_000_000, 1_000_000);
    let mut old = Literal::empty(&graph);
    let mut new = SharedCallPortQueue::empty(&graph);
    let mut xs = BTreeSet::new();
    let mut ys = BTreeSet::new();
    let row = AnalysisConfiguration {
        state: 0,
        alternative: 1,
        calls: SharedCalls::empty(),
    };
    let x = old.share_keys_observed(row.clone(), &mut xs, &mut a, None, None);
    let y = new.share_keys_observed(row, &mut ys, &mut b, None, None);
    assert_eq!(x, y);
    assert!(y.is_err());
    assert!(xs.is_empty() && ys.is_empty());
    assert_eq!(old.summaries.stats(), new.summaries.stats());
    let mut work = 100_000;
    let mut scope = SharedCalls::scope(&mut work)?;
    let mut calls = SharedCalls::empty();
    for pair in 0..128 {
        calls.pushed((pair, pair + 1), &mut scope, &mut work)?;
    }
    assert_eq!(calls.len(), 128);
    assert!(
        calls
            .pushed((129, 130), &mut scope, &mut work)
            .unwrap_err()
            .contains("depth")
    );
    Ok(())
}

#[test]
fn owned_range_actual_pool_conserves_full_partition_and_labels_index_and_materialization_denials()
-> Result<(), String> {
    use super::super::{PoolBlock, PoolWork, pool_scope};
    use crate::grammar::admission_observation::pool_blocks::Snapshot;

    #[derive(Debug)]
    struct Run {
        result: Result<(), String>,
        rows: Vec<Projection>,
        keys: BTreeSet<Projection>,
        remaining: usize,
        shared: bool,
        pool: Option<Snapshot>,
    }
    fn run(graph: &[State], grant: usize, enabled: bool) -> Run {
        let pool = enabled.then(|| PoolWork::new(grant));
        let observer = pool.as_ref();
        let mut work = grant;
        let mut queue = SharedCallPortQueue::empty(graph);
        let mut keys = BTreeSet::new();
        let mut rows = Vec::new();
        let mut shared = false;
        let result = pool_scope(&mut work, observer, PoolBlock::PoolRemainder, |work| {
            pool_scope(work, observer, PoolBlock::ClassifierRemainder, |work| {
                pool_scope(work, observer, PoolBlock::QueueShareRemainder, |work| {
                    let row = AnalysisConfiguration {
                        state: 0,
                        alternative: 3,
                        calls: SharedCalls::empty(),
                    };
                    let result = if let Some(pool) = observer {
                        queue.share_with_pool(row, &mut keys, work, None, None, pool)
                    } else {
                        queue.share_keys_observed(row, &mut keys, work, None, None)
                    };
                    assert!(result?.is_none());
                    Ok::<(), String>(())
                })?;
                shared = true;
                while let Some(row) = queue.pop_front_with_work(work, observer)? {
                    rows.push(project(row));
                }
                Ok(())
            })
        });
        let observed = observer.map(|pool| {
            pool.end(work, result.is_ok());
            let snapshot = pool.snapshot();
            assert!(snapshot.balanced && snapshot.ended);
            assert!(!snapshot.overflowed && !snapshot.inconsistent);
            assert_eq!(
                snapshot.counts.units.iter().sum::<u64>(),
                u64::try_from(grant - work).unwrap_or(u64::MAX)
            );
            snapshot
        });
        Run {
            result,
            rows,
            keys: keys.into_iter().map(project).collect(),
            remaining: work,
            shared,
            pool: observed,
        }
    }
    let graph = graph();
    let plain = run(&graph, 100_000, false);
    let observed = run(&graph, 100_000, true);
    println!("OWNED_RANGE_POOL_PREASSERT {:?}", (&plain, &observed));
    assert_eq!(plain.result, observed.result);
    assert_eq!(plain.rows, observed.rows);
    assert_eq!(plain.keys, observed.keys);
    assert_eq!(plain.remaining, observed.remaining);
    assert_eq!(observed.result, Ok(()));
    assert!(observed.shared);
    assert_eq!(observed.rows.len(), 32);
    let complete_pool = observed.pool.ok_or("Missing actual range pool")?;
    assert_eq!(complete_pool.first_actual_denial, None);
    // All root work lies in tagged Kernel, descriptor and schedule operations.
    // Granted materialization leaves subtract from ClassifierRemainder once.
    assert_eq!(
        complete_pool.counts.units[PoolBlock::ClassifierRemainder as usize],
        0
    );
    let used = 100_000 - observed.remaining;
    let exact = run(&graph, used, true);
    assert_eq!(exact.result, Ok(()));
    assert_eq!(exact.rows, observed.rows);
    assert_eq!(exact.keys, observed.keys);
    assert_eq!(exact.remaining, 0);
    let short = run(&graph, used - 1, true);
    assert!(short.shared && short.result.is_err());
    assert_eq!(short.keys, observed.keys);
    assert_eq!(short.rows.len(), 31);
    assert_eq!(short.remaining, 0);
    let denied = short.pool.ok_or("Missing denied materialization pool")?;
    assert_eq!(denied.first_actual_denial, Some(PoolBlock::PortKernel));
    assert_eq!(
        denied.counts.actual_denied_leaf_attempts[PoolBlock::PortKernel as usize],
        1
    );
    assert_eq!(
        denied.counts.units[PoolBlock::ClassifierRemainder as usize],
        0
    );
    let mut saw_index_denial = false;
    for grant in 0..used {
        let run = run(&graph, grant, true);
        let snapshot = run.pool.ok_or("Missing bounded index denial pool")?;
        if snapshot.first_actual_denial == Some(PoolBlock::PortSchedule) {
            assert!(!run.shared && run.result.is_err());
            assert_eq!(run.remaining, 0);
            assert_eq!(
                snapshot.counts.actual_denied_leaf_attempts[PoolBlock::PortSchedule as usize],
                1
            );
            assert_eq!(snapshot.first_error_exit, Some(PoolBlock::PortSchedule));
            saw_index_denial = true;
            break;
        }
    }
    assert!(
        saw_index_denial,
        "actual canonical index debit must be reached and tagged"
    );
    Ok(())
}
