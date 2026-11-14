use super::super::{
    ParserPrediction, PoolBlock, PoolWork, PredictionSymbol, analysis_calls::Calls, frontier,
    partial_token_probe, pool_scope,
};
use super::owned_fifo_ranges::RangeQueue;
use super::*;

type Row = AnalysisConfiguration<SharedCalls>;
type Projection = (usize, usize, Vec<(usize, usize)>);
type RowsAndKeys = (Vec<Projection>, BTreeSet<Projection>);
fn project(row: Row) -> Projection {
    (row.state, row.alternative, row.calls.to_vec())
}
fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn graph(width: usize) -> Vec<State> {
    let mut states = vec![State::default(); 96];
    states[0] = node((2..2 + width).map(Edge::Epsilon).collect());
    states[1] = node((2..2 + width).map(Edge::Action).collect());
    for state in states.iter_mut().skip(2).take(width) {
        *state = node(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 60)]);
    }
    states[60].rule_stop = true;
    states
}
// Reference keeps the exact current FIFO and sole descriptor/key producer;
// only typed producer-proof use is disabled. No second classifier is present.
struct Unproven<'g>(RangeQueue<'g>);
impl<'g> ClosureQueue<'g, SharedCalls> for Unproven<'g> {
    const OWNED_PORT_RANGES: bool = true;
    fn empty(graph: &'g [State]) -> Self {
        Self(RangeQueue::empty(graph))
    }
    fn unreserved(_: BTreeSet<Row>, _: &'g [State]) -> Result<Self, String> {
        Err("Reference needs exact key reservations".into())
    }
    fn reserve_placement_observed(
        &mut self,
        row: &Row,
        work: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        self.0.reserve_placement_observed(row, work, exact)
    }
    fn observed_shape(&self) -> (usize, usize) {
        self.0.observed_shape()
    }
    fn push_reserved(&mut self, row: Row) -> Result<(), String> {
        self.0.push_reserved(row)
    }
    fn pop_front(&mut self) -> Option<Row> {
        self.0.pop_front()
    }
    fn pop_front_with_work(
        &mut self,
        work: &mut usize,
        pool: Option<&PoolWork>,
    ) -> Result<Option<Row>, String> {
        self.0.pop_front_with_work(work, pool)
    }
    #[allow(clippy::too_many_arguments)]
    fn consume_owned_port_range<K: ClosureKeys<SharedCalls>>(
        &mut self,
        ports: &[Port],
        row: Row,
        keys: &mut K,
        work: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&PoolWork>,
    ) -> Result<(), String> {
        self.0
            .consume_owned_port_range(ports, row, keys, work, measured, exact, pool)
    }
    #[allow(clippy::too_many_arguments)]
    fn consume_small_completed_ports<K: ClosureKeys<SharedCalls>>(
        &mut self,
        ports: &[Port],
        row: Row,
        keys: &mut K,
        work: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&PoolWork>,
    ) -> Result<(), String> {
        single_borrowed_ports::consume_port_rows(
            ports,
            &row,
            &mut self.0,
            keys,
            work,
            measured,
            exact,
            pool,
        )
    }
}
type Reference<'g> = Queue<'g, BorrowedCompletedComponents, SharedCalls, Unproven<'g>>;
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
        Row {
            state: 0,
            alternative: 0,
            calls: SharedCalls::empty(),
        },
        Row {
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
fn opaque_boundary_real_canonical_small_and_range_classifiers_preserve_results_witnesses_and_work_rows()
-> Result<(), String> {
    for width in [8, 32] {
        for recursive in [false, true] {
            let mut states = graph(width);
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
            let (x, y) = (
                RecursiveAnalysisCounters::new(),
                RecursiveAnalysisCounters::new(),
            );
            let old = classify::<Reference<'_>>(&p, &mut a, &x);
            let new = classify::<SharedCallPortQueue<'_>>(&p, &mut b, &y);
            println!(
                "OPAQUE_CANONICAL_PREASSERT {:?}",
                (
                    width,
                    recursive,
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
            assert!(
                b > a,
                "paid evidence must remove real completed canonical eligibility work"
            );
            assert_eq!(x.snapshot().stages, y.snapshot().stages);
            assert!(!y.snapshot().inconsistent && !y.snapshot().overflowed);
        }
    }
    Ok(())
}
fn schedule<'g, Q: ClosureQueue<'g, SharedCalls>>(
    q: &mut Q,
    keys: &mut BTreeSet<Row>,
    row: Row,
    work: &mut usize,
) -> Result<(), String> {
    <BTreeSet<_> as ClosureKeys<SharedCalls>>::schedule(
        row,
        q,
        keys,
        work,
        (true, ScheduleSite::Successor, None),
        None,
    )
}
fn share_pop<'g, Q: ClosureQueue<'g, SharedCalls>>(
    q: &mut Q,
    popped: AnalysisPop<SharedCalls>,
    keys: &mut BTreeSet<Row>,
    work: &mut usize,
    pool: Option<&PoolWork>,
) -> Result<Option<Row>, String> {
    match popped {
        AnalysisPop::Opaque(boundary) => q.share_opaque_boundary(boundary, work, pool),
        AnalysisPop::Ordinary(row) => {
            if let Some(pool) = pool {
                q.share_with_pool(row, keys, work, None, None, pool)
            } else {
                <BTreeSet<_> as ClosureKeys<SharedCalls>>::share(row, q, keys, work, None, None)
            }
        }
    }
}
fn mixed<'g, Q: ClosureQueue<'g, SharedCalls>>(
    states: &'g [State],
    width: usize,
    work: &mut usize,
) -> Result<RowsAndKeys, String> {
    let mut q = Q::empty(states);
    let mut keys = BTreeSet::new();
    let mut scope = SharedCalls::scope(work)?;
    // A pre-existing ordinary item sharing a future proven state must not move.
    schedule(
        &mut q,
        &mut keys,
        Row {
            state: 2,
            alternative: 90,
            calls: SharedCalls::empty(),
        },
        work,
    )?;
    for (alt, frames) in [(7, [(4, 70), (5, 71)]), (7, [(5, 71), (4, 70)])] {
        let mut calls = SharedCalls::empty();
        for pair in frames {
            calls.pushed(pair, &mut scope, work)?;
        }
        let row = Row {
            state: 0,
            alternative: alt,
            calls,
        };
        assert!(
            q.share_keys_observed(row, &mut keys, work, None, None)?
                .is_none()
        );
        // Duplicate complete keys are still checked eagerly, before all pops.
        if width == 8 {
            assert_eq!(keys.len() % 8, 1);
        }
    }
    let mut rows = Vec::new();
    while let Some(popped) = q.pop_for_analysis(work, None)? {
        let row = share_pop(&mut q, popped, &mut keys, work, None)?
            .ok_or("Completed port was resummarized")?;
        let add = rows.is_empty();
        rows.push(project(row));
        if add {
            schedule(
                &mut q,
                &mut keys,
                Row {
                    state: 80,
                    alternative: 91,
                    calls: SharedCalls::empty(),
                },
                work,
            )?;
        }
    }
    Ok((rows, keys.into_iter().map(project).collect()))
}
#[test]
fn opaque_boundary_fifo_keeps_full_ordered_tails_eager_keys_and_new_successor_interleaving()
-> Result<(), String> {
    for width in [8, 32] {
        let states = graph(width);
        let (mut a, mut b) = (100_000, 100_000);
        let old = mixed::<Reference<'_>>(&states, width, &mut a);
        let new = mixed::<SharedCallPortQueue<'_>>(&states, width, &mut b);
        println!(
            "OPAQUE_FIFO_PREASSERT {:?}",
            (width, &old, &new, 100_000 - a, 100_000 - b)
        );
        assert_eq!(old, new);
        let (rows, keys) = new?;
        assert_eq!(rows.len(), 2 * width + 2);
        assert_eq!(keys.len(), rows.len());
        assert_eq!(
            (rows[0].0, rows.last().ok_or("Missing last row")?.0),
            (2, 80)
        );
        assert!(b > a);
    }
    Ok(())
}
fn seed<'g>(
    states: &'g [State],
    work: &mut usize,
) -> Result<(SharedCallPortQueue<'g>, BTreeSet<Row>), String> {
    let mut q = SharedCallPortQueue::empty(states);
    let mut keys = BTreeSet::new();
    assert!(
        q.share_keys_observed(
            Row {
                state: 0,
                alternative: 3,
                calls: SharedCalls::empty()
            },
            &mut keys,
            work,
            None,
            None
        )?
        .is_none()
    );
    Ok((q, keys))
}
#[test]
fn opaque_boundary_zero_exact_short_sticky_poison_and_fresh_recovery_use_actual_producer()
-> Result<(), String> {
    let states = graph(8);
    let mut enough = 100_000;
    let (mut q, mut keys) = seed(&states, &mut enough)?;
    let popped = q
        .pop_for_analysis(&mut enough, None)?
        .ok_or("Missing proven port")?;
    assert!(matches!(&popped, AnalysisPop::Opaque(_)));
    let mut zero = 0;
    assert!(share_pop(&mut q, popped, &mut keys, &mut zero, None).is_err());
    assert!(q.summaries.poisoned);
    let popped = q
        .pop_for_analysis(&mut enough, None)?
        .ok_or("Missing owned port after failure")?;
    let before = enough;
    assert!(
        share_pop(&mut q, popped, &mut keys, &mut enough, None)
            .unwrap_err()
            .contains("poisoned")
    );
    assert_eq!(enough, before);
    let (mut fresh, mut keys) = seed(&states, &mut enough)?;
    let popped = fresh
        .pop_for_analysis(&mut enough, None)?
        .ok_or("Missing fresh proven port")?;
    let mut exact = 1;
    assert!(share_pop(&mut fresh, popped, &mut keys, &mut exact, None)?.is_some());
    assert_eq!(exact, 0);
    assert!(!fresh.summaries.poisoned);
    Ok(())
}
#[test]
fn opaque_boundary_cancelled_proof_use_is_sticky_without_reusing_failed_or_partial_evidence()
-> Result<(), String> {
    let states = graph(8);
    let mut work = 100_000;
    let (mut q, _) = seed(&states, &mut work)?;
    let popped = q
        .pop_for_analysis(&mut work, None)?
        .ok_or("Missing proven port")?;
    let AnalysisPop::Opaque(boundary) = popped else {
        return Err("Expected producer evidence".into());
    };
    let mut attempts = 0;
    let result = q.summaries.consume_boundary(boundary, &mut || {
        attempts += 1;
        Err("Cancelled actual proof use".into())
    });
    assert_eq!(result, Err("Cancelled actual proof use".into()));
    assert_eq!(attempts, 1);
    assert!(q.summaries.poisoned);
    assert!(q.summaries.stats().logical_bytes > 0);
    Ok(())
}
#[test]
fn opaque_boundary_plain_seed_pure_and_cycle_rows_never_acquire_boundary_authority()
-> Result<(), String> {
    let mut states = graph(8);
    states[80] = node(vec![Edge::Epsilon(81)]);
    states[81] = node(vec![Edge::Epsilon(80)]);
    let mut work = 100_000;
    let mut q = SharedCallPortQueue::empty(&states);
    let mut keys = BTreeSet::new();
    for state in [2, 80] {
        schedule(
            &mut q,
            &mut keys,
            Row {
                state,
                alternative: state,
                calls: SharedCalls::empty(),
            },
            &mut work,
        )?;
    }
    for state in [2, 80] {
        let popped = q
            .pop_for_analysis(&mut work, None)?
            .ok_or("Missing ordinary source row")?;
        assert!(matches!(&popped, AnalysisPop::Ordinary(_)));
        let row = share_pop(&mut q, popped, &mut keys, &mut work, None)?
            .ok_or("Cycle fallback was replaced")?;
        assert_eq!(row.state, state);
    }
    assert!(!q.summaries.poisoned);
    Ok(())
}
#[test]
fn opaque_boundary_only_removed_edge_scans_save_work_stops_and_follow_remain_opaque()
-> Result<(), String> {
    for kind in 0..6 {
        let mut states = graph(1);
        states[2] = match kind {
            0 => node(vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 60)]),
            1 => node(vec![Edge::Call {
                target: 61,
                follow: 62,
            }]),
            2 => node(vec![Edge::First {
                target: 60,
                path: "guard".into(),
            }]),
            3 => node(vec![Edge::Full {
                target: 60,
                stop: 60,
            }]),
            4 => State {
                rule_stop: true,
                ..State::default()
            },
            _ => State {
                follow: vec![60],
                ..State::default()
            },
        };
        let (mut a, mut b) = (100_000, 100_000);
        fn use_one<'g, Q: ClosureQueue<'g, SharedCalls>>(
            states: &'g [State],
            work: &mut usize,
        ) -> Result<Row, String> {
            let mut q = Q::empty(states);
            let mut keys = BTreeSet::new();
            assert!(
                q.share_keys_observed(
                    Row {
                        state: 0,
                        alternative: 7,
                        calls: SharedCalls::empty()
                    },
                    &mut keys,
                    work,
                    None,
                    None
                )?
                .is_none()
            );
            let popped = q
                .pop_for_analysis(work, None)?
                .ok_or("Missing actual effect port")?;
            share_pop(&mut q, popped, &mut keys, work, None)?
                .ok_or("Opaque port was resummarized".into())
        }
        assert_eq!(
            use_one::<Reference<'_>>(&states, &mut a),
            use_one::<SharedCallPortQueue<'_>>(&states, &mut b)
        );
        assert_eq!(b - a, usize::from(kind < 4));
    }
    Ok(())
}
#[test]
fn opaque_boundary_actual_pool_conservation_exact_short_and_preconsumed_ledgers()
-> Result<(), String> {
    use crate::grammar::admission_observation::pool_blocks::Snapshot;
    #[derive(Debug)]
    struct Run {
        result: Result<(), String>,
        rows: Vec<Projection>,
        keys: BTreeSet<Projection>,
        remaining: usize,
        pool: Option<Snapshot>,
    }
    fn run(states: &[State], grant: usize, preconsume: usize, enabled: bool) -> Run {
        let pool = enabled.then(|| PoolWork::new(grant));
        let observer = pool.as_ref();
        let mut work = grant;
        let mut q = SharedCallPortQueue::empty(states);
        let mut keys = BTreeSet::new();
        let mut rows = Vec::new();
        let result = pool_scope(&mut work, observer, PoolBlock::PoolRemainder, |work| {
            for _ in 0..preconsume {
                analysis_charge(work)?;
            }
            pool_scope(work, observer, PoolBlock::ClassifierRemainder, |work| {
                pool_scope(work, observer, PoolBlock::QueueShareRemainder, |work| {
                    let row = Row {
                        state: 0,
                        alternative: 3,
                        calls: SharedCalls::empty(),
                    };
                    let shared = if let Some(pool) = observer {
                        q.share_with_pool(row, &mut keys, work, None, None, pool)
                    } else {
                        q.share_keys_observed(row, &mut keys, work, None, None)
                    };
                    assert!(shared?.is_none());
                    Ok::<(), String>(())
                })?;
                while let Some(popped) = q.pop_for_analysis(work, observer)? {
                    let shared =
                        pool_scope(work, observer, PoolBlock::QueueShareRemainder, |work| {
                            share_pop(&mut q, popped, &mut keys, work, observer)
                        })?;
                    rows.push(project(shared.ok_or("Missing exact opaque row")?));
                }
                Ok::<(), String>(())
            })
        });
        let snapshot = observer.map(|pool| {
            pool.end(work, result.is_ok());
            let s = pool.snapshot();
            assert!(s.balanced && s.ended && !s.inconsistent && !s.overflowed);
            assert_eq!(
                s.counts.units.iter().sum::<u64>(),
                u64::try_from(grant - work).unwrap_or(u64::MAX)
            );
            s
        });
        Run {
            result,
            rows,
            keys: keys.into_iter().map(project).collect(),
            remaining: work,
            pool: snapshot,
        }
    }
    for width in [8, 32] {
        let states = graph(width);
        let plain = run(&states, 100_000, 0, false);
        let observed = run(&states, 100_000, 0, true);
        println!("OPAQUE_POOL_PREASSERT {:?}", (width, &plain, &observed));
        assert_eq!(plain.result, observed.result);
        assert_eq!(plain.rows, observed.rows);
        assert_eq!(plain.keys, observed.keys);
        assert_eq!(plain.remaining, observed.remaining);
        assert_eq!(observed.result, Ok(()));
        assert_eq!(observed.rows.len(), width);
        let used = 100_000 - observed.remaining;
        let exact = run(&states, used, 0, true);
        assert_eq!(exact.result, Ok(()));
        assert_eq!(exact.remaining, 0);
        assert_eq!(exact.rows, observed.rows);
        assert_eq!(exact.keys, observed.keys);
        let pre = run(&states, used + 13, 13, true);
        assert_eq!(pre.result, Ok(()));
        assert_eq!(pre.remaining, 0);
        assert_eq!(pre.rows, observed.rows);
        assert_eq!(pre.keys, observed.keys);
        let short = run(&states, used - 1, 0, true);
        assert!(short.result.is_err());
        assert_eq!(short.rows.len(), width - 1);
        assert_eq!(short.keys, observed.keys);
        assert_eq!(short.remaining, 0);
        let denied = short.pool.ok_or("Missing denied proof pool")?;
        assert_eq!(denied.first_actual_denial, Some(PoolBlock::PortKernel));
        assert_eq!(
            denied.counts.actual_denied_leaf_attempts[PoolBlock::PortKernel as usize],
            1
        );
        assert_eq!(
            denied.counts.units[PoolBlock::ClassifierRemainder as usize],
            0
        );
    }
    Ok(())
}
#[test]
fn opaque_boundary_duplicate_action_ports_and_paid_merge_failure_preserve_actual_eager_prefix()
-> Result<(), String> {
    let mut states = graph(4);
    states[0] = node(
        (2..6)
            .flat_map(|state| [Edge::Action(state), Edge::Epsilon(state)])
            .collect(),
    );
    let (mut a, mut b) = (100_000, 100_000);
    fn run<'g, Q: ClosureQueue<'g, SharedCalls>>(
        states: &'g [State],
        work: &mut usize,
    ) -> Result<RowsAndKeys, String> {
        let mut q = Q::empty(states);
        let mut keys = BTreeSet::new();
        schedule(
            &mut q,
            &mut keys,
            Row {
                state: 2,
                alternative: 9,
                calls: SharedCalls::empty(),
            },
            work,
        )?;
        assert!(
            q.share_keys_observed(
                Row {
                    state: 0,
                    alternative: 3,
                    calls: SharedCalls::empty()
                },
                &mut keys,
                work,
                None,
                None
            )?
            .is_none()
        );
        let mut rows = Vec::new();
        while let Some(popped) = q.pop_for_analysis(work, None)? {
            rows.push(project(
                share_pop(&mut q, popped, &mut keys, work, None)?.ok_or("Missing boundary row")?,
            ));
        }
        Ok((rows, keys.into_iter().map(project).collect()))
    }
    let old = run::<Reference<'_>>(&states, &mut a);
    let new = run::<SharedCallPortQueue<'_>>(&states, &mut b);
    println!(
        "OPAQUE_ACTION_PREASSERT {:?}",
        (&old, &new, 100_000 - a, 100_000 - b)
    );
    assert_eq!(old, new);
    let (rows, keys) = new?;
    assert_eq!(rows.len(), 5);
    assert_eq!(keys.len(), 5);
    assert!(b > a);
    // Earlier preparation denials still preserve the existing eager prefix.
    // This loop is preparation coverage, NOT proof of the later merge debit.
    let mut saw_preparation_failure = false;
    for grant in 0..1000 {
        let mut q = SharedCallPortQueue::empty(&states);
        let mut keys = BTreeSet::new();
        let mut work = grant;
        if schedule(
            &mut q,
            &mut keys,
            Row {
                state: 2,
                alternative: 9,
                calls: SharedCalls::empty(),
            },
            &mut work,
        )
        .is_err()
        {
            continue;
        }
        let result = q.share_keys_observed(
            Row {
                state: 0,
                alternative: 3,
                calls: SharedCalls::empty(),
            },
            &mut keys,
            &mut work,
            None,
            None,
        );
        assert!(keys.len() <= 5);
        let produced: Vec<_> = keys
            .iter()
            .filter(|row| row.alternative == 3)
            .map(|row| row.state)
            .collect();
        assert_eq!(produced, (2..2 + produced.len()).collect::<Vec<_>>());
        if result.is_err() && keys.len() == 1 && q.summaries.poisoned {
            saw_preparation_failure = true;
        }
        if result.is_ok() {
            break;
        }
    }
    assert!(saw_preparation_failure);

    // Isolate the actual merge-marker debit through its real tagged leaf.
    // Small completed ports have no range-index fee, so PortSchedule's tagged
    // denial identifies reserve_completed_boundary, rather than a preparation
    // debit or the untagged old full-key reservation. This uses the genuine
    // completed Kernel producer, not a fabricated Boundary or queue adapter.
    let mut complete = 100_000;
    let mut calibrated = SharedCallPortQueue::empty(&states);
    let mut calibrated_keys = BTreeSet::new();
    schedule(
        &mut calibrated,
        &mut calibrated_keys,
        Row {
            state: 2,
            alternative: 9,
            calls: SharedCalls::empty(),
        },
        &mut complete,
    )?;
    let before = complete;
    assert!(
        calibrated
            .share_keys_observed(
                Row {
                    state: 0,
                    alternative: 3,
                    calls: SharedCalls::empty()
                },
                &mut calibrated_keys,
                &mut complete,
                None,
                None,
            )?
            .is_none()
    );
    let producer_used = before - complete;
    let mut saw_actual_marker_denial = false;
    for grant in 0..=producer_used {
        let mut q = SharedCallPortQueue::empty(&states);
        let mut keys = BTreeSet::new();
        let mut placement_work = 100_000;
        schedule(
            &mut q,
            &mut keys,
            Row {
                state: 2,
                alternative: 9,
                calls: SharedCalls::empty(),
            },
            &mut placement_work,
        )?;
        let before_keys = keys.clone();
        let pool = PoolWork::new(grant);
        let mut work = grant;
        let result = pool_scope(&mut work, Some(&pool), PoolBlock::PoolRemainder, |work| {
            pool_scope(work, Some(&pool), PoolBlock::ClassifierRemainder, |work| {
                pool_scope(work, Some(&pool), PoolBlock::QueueShareRemainder, |work| {
                    q.share_with_pool(
                        Row {
                            state: 0,
                            alternative: 3,
                            calls: SharedCalls::empty(),
                        },
                        &mut keys,
                        work,
                        None,
                        None,
                        &pool,
                    )
                })
            })
        });
        pool.end(work, result.is_ok());
        let snapshot = pool.snapshot();
        assert!(snapshot.ended && snapshot.balanced);
        assert!(!snapshot.overflowed && !snapshot.inconsistent);
        assert_eq!(
            snapshot.counts.units.iter().sum::<u64>(),
            u64::try_from(grant - work).unwrap_or(u64::MAX)
        );
        if snapshot.first_actual_denial != Some(PoolBlock::PortSchedule) {
            continue;
        }
        println!(
            "OPAQUE_ACTUAL_MERGE_DENIAL_PREASSERT {:?}",
            (
                grant,
                &result,
                &keys,
                q.summaries.stats(),
                q.summaries.poisoned,
                q.inner.failed,
                &snapshot,
            )
        );
        assert!(result.is_err());
        assert_eq!(work, 0);
        assert_eq!(snapshot.first_error_exit, Some(PoolBlock::PortSchedule));
        assert_eq!(
            snapshot.counts.actual_denied_leaf_attempts[PoolBlock::PortSchedule as usize],
            1
        );
        // Complete/publication and all per-port row admissions preceded this
        // consumer error. It must NOT be relabelled a failed Kernel proof.
        assert_eq!(q.summaries.stats().builds, 1);
        assert_eq!(q.summaries.stats().publications, 1);
        assert!(!q.summaries.poisoned);
        assert!(!q.inner.failed);
        assert_eq!(keys, before_keys);
        let mut pop_work = 0;
        let popped = q
            .pop_for_analysis(&mut pop_work, None)?
            .ok_or("Marker denial lost the old ordinary row")?;
        assert!(matches!(&popped, AnalysisPop::Ordinary(_)));
        assert_eq!(
            project(match popped {
                AnalysisPop::Ordinary(row) => row,
                AnalysisPop::Opaque(_) =>
                    return Err("Denied marker published opaque authority".into()),
            }),
            (2, 9, vec![])
        );
        assert_eq!(pop_work, 0);
        assert!(q.pop_for_analysis(&mut pop_work, None)?.is_none());
        assert_eq!(keys, before_keys);
        saw_actual_marker_denial = true;
        break;
    }
    assert!(
        saw_actual_marker_denial,
        "Must reach the actual producer's tagged merge-marker denial"
    );
    Ok(())
}
