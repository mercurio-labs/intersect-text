//! Whole canonical outputs and actual-operation controls for the one-borrow seam.
//! This source packet is not an original SysML admission or performance claim.
use super::super::{ParserPrediction, PredictionSymbol, frontier, partial_token_probe};
use super::*;

fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn token(name: &str, target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(name.into()), target)
}
fn graph() -> Vec<State> {
    let mut graph = vec![State::default(); 9];
    graph[0] = node(vec![Edge::Epsilon(2), Edge::Action(2), Edge::Epsilon(3)]);
    graph[1] = node(vec![Edge::Epsilon(2)]);
    graph[2] = node(vec![token("x", 6)]);
    graph[3] = node(vec![token("y", 7)]);
    graph[4] = node(vec![Edge::Epsilon(5)]);
    graph[5] = node(vec![Edge::Action(4)]);
    graph[6].rule_stop = true;
    graph[7].rule_stop = true;
    graph
}

fn take(
    source: &mut Kernel<'_, BorrowedCompletedComponents>,
    single: bool,
    root: usize,
    returns: usize,
    work: &mut usize,
) -> Result<Option<Vec<Port>>, String> {
    let result = if single {
        source.consume_prepared_ports(root, returns, work, &mut analysis_charge, None, project)
    } else {
        let ports =
            source.borrow_ports_for_configurations(root, returns, &mut || analysis_charge(work))?;
        project(ports, work)
    };
    // Same standalone consumer cleanup boundary used by existing take controls.
    if result.is_err() {
        source.invalidate();
    }
    result
}
fn project(ports: Option<&[Port]>, work: &mut usize) -> Result<Option<Vec<Port>>, String> {
    ports
        .map(|ports| {
            ports
                .iter()
                .map(|port| {
                    analysis_charge(work)?;
                    Ok(*port)
                })
                .collect()
        })
        .transpose()
}

#[test]
fn single_borrow_cold_and_warm_remove_exact_real_map_operations_with_whole_ports()
-> Result<(), String> {
    let graph = graph();
    let original = serde_json::to_value(&graph).map_err(|e| e.to_string())?;
    let mut old = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut new = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let (mut a, mut b) = (10000, 10000);
    for (root, returns) in [(0, 3), (1, 7), (0, 2), (1, 0)] {
        let previous_gap = b - a;
        let x = take(&mut old, false, root, returns, &mut a)?;
        let y = take(&mut new, true, root, returns, &mut b)?;
        let mut map_fee = 100;
        map_work(new.completed.len(), &mut || analysis_charge(&mut map_fee))?;
        println!(
            "SINGLE_BORROW_KERNEL_PREASSERT {:?}",
            (&x, &y, a, b, old.stats(), new.stats())
        );
        assert_eq!(x, y);
        assert_eq!(old.stats(), new.stats());
        assert_eq!(b - a - previous_gap, 100 - map_fee);
        assert_eq!(old.completed.len(), new.completed.len());
        assert!(!old.poisoned && !new.poisoned);
    }
    let actual = take(&mut new, true, 0, 0, &mut b)?.ok_or("Missing ports")?;
    assert_eq!(
        actual,
        vec![
            Port {
                state: 2,
                action_seen: false
            },
            Port {
                state: 2,
                action_seen: true
            },
            Port {
                state: 3,
                action_seen: false
            }
        ]
    );
    assert_eq!(
        serde_json::to_value(&graph).map_err(|e| e.to_string())?,
        original
    );
    Ok(())
}

#[test]
fn single_borrow_opaque_empty_none_and_cycle_preserve_fallback_and_fees() -> Result<(), String> {
    let graph = graph();
    let mut old = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut new = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let (mut a, mut b) = (10000, 10000);
    for root in [2, 8, 4, 4, 5, 5] {
        let x = take(&mut old, false, root, 2, &mut a)?;
        let y = take(&mut new, true, root, 2, &mut b)?;
        assert_eq!(x, None);
        assert_eq!(x, y);
        assert_eq!(a, b);
        assert_eq!(old.stats(), new.stats());
        assert!(!new.poisoned);
    }
    for root in [
        node(vec![token("x", 0)]),
        node(vec![Edge::Call {
            target: 0,
            follow: 0,
        }]),
        node(vec![Edge::First {
            path: "original".into(),
            target: 0,
        }]),
        node(vec![Edge::Full { target: 0, stop: 0 }]),
        node(vec![Edge::Action(0), token("mixed", 0)]),
        State {
            accepting: true,
            ..State::default()
        },
        State {
            rule_stop: true,
            ..State::default()
        },
        State {
            follow: vec![0],
            ..State::default()
        },
    ] {
        let graph = [root];
        let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
        let mut work = 100;
        assert_eq!(take(&mut source, true, 0, 0, &mut work)?, None);
        assert!(source.completed.is_empty() && source.colors.is_empty());
    }
    Ok(())
}

#[test]
fn single_borrow_zero_exact_short_preconsumed_and_every_denial_are_sticky() -> Result<(), String> {
    let graph = graph();
    let mut full = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut work = 10000;
    let first = take(&mut full, true, 0, 3, &mut work)?;
    let second = take(&mut full, true, 0, 2, &mut work)?;
    let required = 10000 - work;
    for allowed in 0..=required {
        let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
        let mut work = allowed;
        let result = take(&mut source, true, 0, 3, &mut work)
            .and_then(|a| take(&mut source, true, 0, 2, &mut work).map(|b| (a, b)));
        assert_eq!(work, 0);
        if allowed == required {
            assert_eq!(result, Ok((first.clone(), second.clone())));
            assert!(!source.poisoned);
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
            let before = work;
            assert_eq!(
                take(&mut source, true, 0, 0, &mut work),
                Err("Transparent port source is poisoned".into())
            );
            assert_eq!(work, before);
        }
    }
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut preconsumed = required + 9;
    for _ in 0..9 {
        analysis_charge(&mut preconsumed)?;
    }
    assert_eq!(take(&mut source, true, 0, 3, &mut preconsumed)?, first);
    assert_eq!(take(&mut source, true, 0, 2, &mut preconsumed)?, second);
    assert_eq!(preconsumed, 0);
    Ok(())
}

#[test]
fn single_borrow_kernel_cancel_and_consumer_error_keep_distinct_invalidation_boundary()
-> Result<(), String> {
    let graph = graph();
    for warm in [false, true] {
        let mut full = Kernel::<BorrowedCompletedComponents>::new(&graph);
        let mut work = 10000;
        if warm {
            take(&mut full, true, 0, 3, &mut work)?;
        }
        let mut required = 0usize;
        full.consume_prepared_ports(
            0,
            3,
            &mut required,
            &mut |calls| {
                *calls += 1;
                Ok(())
            },
            None,
            |_, _| Ok(()),
        )?;
        for cancel_at in 1..=required {
            let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
            let mut work = 10000;
            if warm {
                take(&mut source, true, 0, 3, &mut work)?;
            }
            let mut calls = 0usize;
            let result = source.consume_prepared_ports(
                0,
                3,
                &mut calls,
                &mut |calls| {
                    *calls += 1;
                    if *calls == cancel_at {
                        Err("caller cancelled".into())
                    } else {
                        Ok(())
                    }
                },
                None,
                |_, _| Ok(()),
            );
            assert_eq!(calls, cancel_at);
            assert_eq!(result, Err("caller cancelled".into()));
            assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
        }
    }
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut work = 10000;
    let result: Result<(), String> =
        source.consume_prepared_ports(0, 2, &mut work, &mut analysis_charge, None, |ports, _| {
            assert!(ports.is_some());
            Err("canonical consumer cancellation".into())
        });
    assert_eq!(result, Err("canonical consumer cancellation".into()));
    assert!(!source.poisoned && source.completed.contains_key(&0));
    assert!(take(&mut source, true, 0, 0, &mut work)?.is_some());
    Ok(())
}

#[test]
fn single_borrow_bounds_fresh_graph_and_admission_overflow_remain_authoritative()
-> Result<(), String> {
    assert_eq!(
        (
            MAX_KEYS,
            MAX_LOGICAL_BYTES,
            CONFIGURATION_BYTES,
            RETURN_BYTES
        ),
        (4096, 1048576, 48, 16)
    );
    let graph = graph();
    for returns in [usize::MAX, MAX_LOGICAL_BYTES / RETURN_BYTES, 20000] {
        let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
        let mut work = 100000;
        assert!(take(&mut source, true, 0, returns, &mut work).is_err());
        assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
        assert!(source.stats().logical_bytes <= MAX_LOGICAL_BYTES);
        if returns == 20000 {
            assert!(source.stats().logical_bytes > 2 * 320000);
        }
    }
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
    source.epoch = MAX_EPOCH;
    let mut work = 10000;
    assert_eq!(
        take(&mut source, true, 0, 0, &mut work),
        Err("Transparent port visitation epoch exhausted".into())
    );
    assert!(source.poisoned);
    let dangling = [node(vec![Edge::Epsilon(99)])];
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&dangling);
    assert_eq!(
        take(&mut source, true, 0, 0, &mut work),
        Err("Dangling parser prediction state".into())
    );
    assert!(source.poisoned && source.completed.is_empty());
    let fresh = [node(vec![Edge::Action(1)]), node(vec![token("fresh", 0)])];
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&fresh);
    assert_eq!(
        take(&mut source, true, 0, 0, &mut work)?,
        Some(vec![Port {
            state: 1,
            action_seen: true
        }])
    );
    let mut chain = vec![State::default(); MAX_KEYS + 2];
    for (i, row) in chain.iter_mut().enumerate().take(MAX_KEYS + 1) {
        *row = node(vec![Edge::Epsilon(i + 1)]);
    }
    chain[MAX_KEYS + 1] = node(vec![token("end", 0)]);
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&chain);
    let mut work = 500000;
    assert!(take(&mut source, true, 1, 0, &mut work)?.is_some());
    assert_eq!(
        take(&mut source, true, 0, 0, &mut work),
        Err("Transparent port node bound exceeded".into())
    );
    assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
    Ok(())
}

// Test-only old queue reference: no production selector or stored mode. Every
// operation delegates to the same queue/storage/schedule worker; only share uses
// the old reborrow path and that same single canonical descriptor consumer.
struct ReborrowQueue<'graph>(SharedCallPortQueue<'graph>);
impl<'graph> ClosureQueue<'graph, SharedCalls> for ReborrowQueue<'graph> {
    const POOL_QUEUE_KIND: usize = 0;
    fn empty(graph: &'graph [State]) -> Self {
        Self(SharedCallPortQueue::empty(graph))
    }
    fn unreserved(
        rows: BTreeSet<AnalysisConfiguration<SharedCalls>>,
        graph: &'graph [State],
    ) -> Result<Self, String> {
        SharedCallPortQueue::unreserved(rows, graph).map(Self)
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration<SharedCalls>,
        work: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        self.0.reserve_placement_observed(row, work, exact)
    }
    fn observed_shape(&self) -> (usize, usize) {
        self.0.observed_shape()
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration<SharedCalls>) -> Result<(), String> {
        self.0.push_reserved(row)
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<SharedCalls>> {
        self.0.pop_front()
    }
    fn share_keys_observed<K: ClosureKeys<SharedCalls>>(
        &mut self,
        row: AnalysisConfiguration<SharedCalls>,
        keys: &mut K,
        work: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<SharedCalls>>, String> {
        self.0
            .share_keys_pool_reborrow(row, keys, work, measured, exact, None)
    }
    fn share_transparent_observed(
        &mut self,
        row: AnalysisConfiguration<SharedCalls>,
        keys: &mut BTreeSet<AnalysisConfiguration<SharedCalls>>,
        work: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<SharedCalls>>, String> {
        self.0
            .share_keys_pool_reborrow(row, keys, work, measured, exact, None)
    }
    fn share_with_pool<K: ClosureKeys<SharedCalls>>(
        &mut self,
        row: AnalysisConfiguration<SharedCalls>,
        keys: &mut K,
        work: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: &super::super::PoolWork,
    ) -> Result<Option<AnalysisConfiguration<SharedCalls>>, String> {
        self.0
            .share_keys_pool_reborrow(row, keys, work, measured, exact, Some(pool))
    }
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
    let mut witnesses = BTreeMap::new();
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
    let result = p.multiple_recursive_configurations_with_call_storage::<SharedCalls,Q,frontier::AdoptFrontiers,true,partial_token_probe::NoProbe>(
        initial,work,true,&mut witnesses,false,(true,Some(counters),None,None),&mut partial_token_probe::NoProbe,None);
    (result, witnesses)
}

type ProjectedRow = (usize, usize, Vec<(usize, usize)>);
type ProjectedQueue = (Vec<ProjectedRow>, BTreeSet<ProjectedRow>);
fn rows_with_ordered_tails<'g, Q: ClosureQueue<'g, SharedCalls>>(
    graph: &'g [State],
    work: &mut usize,
) -> Result<ProjectedQueue, String> {
    let mut queue = Q::empty(graph);
    let mut keys = BTreeSet::new();
    let mut scope = SharedCalls::scope(work)?;
    for frames in [[(4, 6), (5, 7)], [(5, 7), (4, 6)]] {
        let mut calls = SharedCalls::empty();
        for pair in frames {
            calls.pushed(pair, &mut scope, work)?;
        }
        let row = AnalysisConfiguration {
            state: 0,
            alternative: 4,
            calls,
        };
        assert!(
            queue
                .share_keys_observed(row, &mut keys, work, None, None)?
                .is_none()
        );
    }
    let mut rows = Vec::new();
    while let Some(row) = queue.pop_front() {
        rows.push((row.state, row.alternative, row.calls.to_vec()));
    }
    let keys = keys
        .into_iter()
        .map(|row| (row.state, row.alternative, row.calls.to_vec()))
        .collect();
    Ok((rows, keys))
}

#[test]
fn single_borrow_real_canonical_classifier_completes_same_full_tail_witnesses_with_less_work()
-> Result<(), String> {
    for recursive in [false, true] {
        let mut graph = vec![State::default(); 10];
        graph[0] = node((0..12).map(|_| Edge::Epsilon(2)).collect());
        graph[1] = node((0..12).map(|_| Edge::Action(3)).collect());
        graph[2] = node(vec![Edge::Call {
            target: 4,
            follow: 6,
        }]);
        graph[3] = node(vec![Edge::Call {
            target: 4,
            follow: 7,
        }]);
        graph[4] = node(vec![Edge::Epsilon(8), Edge::Action(8)]);
        graph[8] = node(vec![token("r", 5)]);
        graph[5] = if recursive {
            node(vec![Edge::Call {
                target: 4,
                follow: 6,
            }])
        } else {
            State {
                rule_stop: true,
                ..State::default()
            }
        };
        graph[6].rule_stop = true;
        graph[7].rule_stop = true;
        let p = prediction(graph);
        let old_counts = RecursiveAnalysisCounters::new();
        let new_counts = RecursiveAnalysisCounters::new();
        let (mut a, mut b) = (10000, 10000);
        let old = classify::<ReborrowQueue<'_>>(&p, &mut a, &old_counts);
        let new = classify::<SharedCallPortQueue<'_>>(&p, &mut b, &new_counts);
        println!(
            "SINGLE_BORROW_CANONICAL_PREASSERT {:?}",
            (
                &old,
                &new,
                10000 - a,
                10000 - b,
                old_counts.snapshot(),
                new_counts.snapshot()
            )
        );
        assert_eq!(old, new);
        assert_eq!(new.0, Ok(recursive));
        assert!(
            b > a,
            "same completed canonical classification, not a finite-prefix descriptor count"
        );
        assert_eq!(old_counts.snapshot(), new_counts.snapshot());
    }
    Ok(())
}

#[test]
fn single_borrow_ordered_unequal_call_tails_and_depth128_guard_are_unchanged() -> Result<(), String>
{
    let graph = graph();
    let (mut a, mut b) = (10000, 10000);
    let old = rows_with_ordered_tails::<ReborrowQueue<'_>>(&graph, &mut a)?;
    let new = rows_with_ordered_tails::<SharedCallPortQueue<'_>>(&graph, &mut b)?;
    println!(
        "SINGLE_BORROW_ORDERED_TAILS_PREASSERT {:?}",
        (&old, &new, a, b)
    );
    assert_eq!(old, new);
    assert_eq!(new.0.len(), 4);
    assert_eq!(new.1.len(), 4);
    assert_eq!(
        new.1
            .iter()
            .map(|(_, _, calls)| calls.clone())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([vec![(4, 6), (5, 7)], vec![(5, 7), (4, 6)]])
    );
    assert!(b > a);
    for depth in [128, 129] {
        let mut graph = vec![State::default(); 2 * (depth + 1) + 1];
        let end = graph.len() - 1;
        for i in 0..depth {
            for side in 0..2 {
                graph[2 * i + side] = node(vec![Edge::Call {
                    target: 2 * (i + 1) + side,
                    follow: end,
                }]);
            }
        }
        graph[2 * depth] = node(vec![token("left", end)]);
        graph[2 * depth + 1] = node(vec![token("right", end)]);
        let p = prediction(graph);
        let (mut a, mut b) = (100000, 100000);
        let old = classify::<ReborrowQueue<'_>>(&p, &mut a, &RecursiveAnalysisCounters::new());
        let new =
            classify::<SharedCallPortQueue<'_>>(&p, &mut b, &RecursiveAnalysisCounters::new());
        assert_eq!(old, new);
        assert_eq!(
            a, b,
            "all roots opaque: no summary reborrow operation can be removed"
        );
        assert_eq!(
            new.0,
            if depth == 128 {
                Ok(false)
            } else {
                Err("Parser prediction call depth exceeded".into())
            }
        );
    }
    Ok(())
}
