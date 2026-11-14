//! Observations of the real kernel and canonical classifier; no saved-work claim.
use super::super::{ParserPrediction, PredictionSymbol, frontier};
use super::*;
use crate::grammar::RecursiveFrontierOrder;
use crate::grammar::admission_observation::RunStage;
use crate::grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic;
use crate::grammar::admission_observation::pool_blocks::FirstQueueTopologyCoverage as Coverage;
use crate::grammar::admission_observation::pool_blocks::FirstQueueTopologySnapshot;
use crate::grammar::admission_observation::pool_blocks::FirstQueueTopologyUnavailable as Unavailable;
use crate::grammar::admission_observation::pool_blocks::Snapshot as WorkSnapshot;
use intersect_model::definitions::bundle::SourceId;
use std::cell::Cell;

fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn word(target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword("x".into()), target)
}
fn graph() -> Vec<State> {
    let mut graph = vec![State::default(); 8];
    graph[0] = node(vec![Edge::Epsilon(2)]);
    graph[1] = node(vec![Edge::Epsilon(2)]);
    graph[2] = node(vec![Edge::Epsilon(3)]);
    graph[3] = node(vec![word(6)]);
    graph[4] = node(vec![Edge::Action(2)]);
    graph[5] = node(vec![Edge::Action(2)]);
    graph[6].rule_stop = true;
    graph
}
fn debit(
    work: &mut usize,
    pool: &super::super::PoolWork<'_>,
    block: super::super::PoolBlock,
    calls: &Cell<usize>,
    cancel: Option<usize>,
) -> Result<(), String> {
    calls.set(calls.get() + 1);
    let result = if cancel == Some(calls.get()) {
        Err("actual kernel cancellation".into())
    } else {
        analysis_charge(work)
    };
    pool.actual_leaf_result(block, result.is_ok());
    result
}
fn take(
    source: &mut Kernel<'_, BorrowedCompletedComponents>,
    root: usize,
    work: &mut usize,
    pool: &super::super::PoolWork<'_>,
    calls: &Cell<usize>,
    cancel: Option<usize>,
) -> Result<Option<Vec<Port>>, String> {
    let result = (|| {
        let mut charge = || {
            debit(
                work,
                pool,
                super::super::PoolBlock::PortKernel,
                calls,
                cancel,
            )
        };
        let ports = if pool.topology_enabled() {
            source.borrow_ports_for_configurations_with_topology(root, 0, &mut charge, pool)?
        } else {
            source.borrow_ports_for_configurations(root, 0, &mut charge)?
        };
        if let Some(ports) = ports {
            let mut projected = Vec::new();
            for port in ports {
                debit(
                    work,
                    pool,
                    super::super::PoolBlock::PortDescriptor,
                    calls,
                    cancel,
                )?;
                projected.push(*port); // test projection only, same paid consumer reads
            }
            Ok(Some(projected))
        } else {
            Ok(None)
        }
    })();
    if result.is_err() {
        source.invalidate();
    }
    result
}
struct Run {
    result: Result<Vec<Option<Vec<Port>>>, String>,
    remaining: usize,
    attempts: usize,
    poisoned: bool,
    topology: FirstQueueTopologySnapshot,
    work: WorkSnapshot,
}
fn run(
    enabled: bool,
    graph: &[State],
    roots: &[usize],
    grant: usize,
    cancel: Option<usize>,
) -> Result<Run, String> {
    let capture = CompilerPoolDiagnostic::new();
    if enabled {
        capture.enable_first_queue_topology();
    }
    let ordinal = capture
        .begin(
            RunStage::Executable,
            &SourceId::for_text("test:kernel", "original graph"),
            RecursiveFrontierOrder::BreadthFirstSharedCallPorts,
        )
        .ok_or("Missing actual diagnostic program")?;
    let pool = super::super::PoolWork::new_with_topology(grant, &capture);
    let mut remaining = grant;
    let token = pool.enter(super::super::PoolBlock::PoolRemainder, remaining);
    let mut source = Kernel::<BorrowedCompletedComponents>::new(graph);
    let calls = Cell::new(0);
    let result: Result<Vec<Option<Vec<Port>>>, String> = roots
        .iter()
        .map(|root| take(&mut source, *root, &mut remaining, &pool, &calls, cancel))
        .collect();
    pool.leave(token, remaining, result.is_ok());
    pool.end(remaining, result.is_ok());
    let work = pool.snapshot();
    capture.finish(ordinal, work);
    assert!(work.balanced && work.ended);
    Ok(Run {
        result,
        remaining,
        attempts: calls.get(),
        poisoned: source.poisoned,
        topology: capture.first_queue_topology_snapshot(),
        work,
    })
}
#[test]
fn topology_observer_default_off_keeps_whole_kernel_output_and_original_pool() -> Result<(), String>
{
    let graph = graph();
    let before = serde_json::to_value(&graph).map_err(|e| e.to_string())?;
    let old = run(false, &graph, &[0, 1, 4, 5], 10000, None)?;
    let new = run(true, &graph, &[0, 1, 4, 5], 10000, None)?;
    println!(
        "FIRST_QUEUE_TOPOLOGY_KERNEL {:?}",
        (
            &old.result,
            &new.result,
            10000 - old.remaining,
            10000 - new.remaining,
            new.topology
        )
    );
    assert_eq!(old.result, new.result);
    assert_eq!(old.remaining, new.remaining);
    assert_eq!(old.work, new.work);
    assert_eq!(old.attempts, new.attempts);
    assert_eq!(old.topology, FirstQueueTopologySnapshot::default());
    assert_eq!(new.topology.coverage, Coverage::FirstQueue);
    assert!(new.topology.window_ended);
    assert_eq!(new.topology.program_ordinal, Some(1));
    assert_eq!(
        serde_json::to_value(&graph).map_err(|e| e.to_string())?,
        before
    );
    Ok(())
}
#[test]
fn topology_observer_counts_only_actual_reexpanded_completed_full_action_keys() -> Result<(), String>
{
    let measured = run(true, &graph(), &[0, 1, 4, 5], 10000, None)?;
    assert_eq!(
        measured.result,
        Ok(vec![
            Some(vec![Port {
                state: 3,
                action_seen: false
            }]),
            Some(vec![Port {
                state: 3,
                action_seen: false
            }]),
            Some(vec![Port {
                state: 3,
                action_seen: true
            }]),
            Some(vec![Port {
                state: 3,
                action_seen: true
            }])
        ])
    );
    let row = measured.topology;
    assert_eq!(row.graph_states, Some(8));
    assert_eq!(row.queue_scopes, 1);
    assert_eq!(row.walks_started, 4);
    assert_eq!(row.pure_key_expansions, 8);
    assert_eq!(row.earlier_completed_interior_reexpansions, 2);
    assert_eq!(row.actual_pure_edge_reads, 8);
    assert_eq!(row.edge_reads_from_earlier_completed_keys, 2);
    assert_eq!(row.distinct_completed_interior_keys, 2);
    // The second root's first true-action visit never matches the completed
    // false-action key. Both variants have independent actual completion bits.
    let partial = run(true, &graph(), &[0, 4], 10000, None)?;
    assert_eq!(partial.topology.earlier_completed_interior_reexpansions, 0);
    assert_eq!(partial.topology.edge_reads_from_earlier_completed_keys, 0);
    Ok(())
}
#[test]
fn topology_observer_zero_exact_short_preconsumed_and_oversized_source_preserve_results()
-> Result<(), String> {
    let graph = graph();
    let full = run(false, &graph, &[0, 1], 10000, None)?;
    let used = 10000 - full.remaining;
    for grant in [0, 1, used - 1, used] {
        let old = run(false, &graph, &[0, 1], grant, None)?;
        let new = run(true, &graph, &[0, 1], grant, None)?;
        assert_eq!(old.result, new.result);
        assert_eq!(old.remaining, new.remaining);
        assert_eq!(old.work, new.work);
        assert_eq!(new.remaining, 0);
        if grant == used {
            assert_eq!(new.result, full.result);
            assert!(!new.poisoned);
        } else {
            assert!(new.result.is_err() && new.poisoned);
        }
    }
    let mut preconsumed = used + 7;
    for _ in 0..7 {
        analysis_charge(&mut preconsumed)?;
    }
    let exact = run(true, &graph, &[0, 1], preconsumed, None)?;
    assert_eq!(exact.result, full.result);
    assert_eq!(exact.remaining, 0);
    let mut large = graph;
    large.resize(4097, State::default());
    let old = run(false, &large, &[0, 1], 10000, None)?;
    let new = run(true, &large, &[0, 1], 10000, None)?;
    assert_eq!(old.result, new.result);
    assert_eq!(old.remaining, new.remaining);
    assert_eq!(old.work, new.work);
    assert_eq!(new.topology.coverage, Coverage::Unavailable);
    assert_eq!(
        new.topology.unavailable,
        Some(Unavailable::SourceStateBound)
    );
    assert_eq!(new.topology.earlier_completed_interior_reexpansions, 0);
    Ok(())
}
#[test]
fn topology_observer_cycles_opaque_boundaries_dangling_and_each_cancel_do_not_publish_fake_completion()
-> Result<(), String> {
    let cycle = [node(vec![Edge::Epsilon(1)]), node(vec![Edge::Action(0)])];
    let old = run(false, &cycle, &[0, 1], 10000, None)?;
    let new = run(true, &cycle, &[0, 1], 10000, None)?;
    assert_eq!(old.result, new.result);
    assert_eq!(old.remaining, new.remaining);
    assert_eq!(new.result, Ok(vec![None, None]));
    assert_eq!(new.topology.distinct_completed_interior_keys, 0);
    assert_eq!(new.topology.earlier_completed_interior_reexpansions, 0);
    assert!(!new.poisoned);
    for root in [
        node(vec![word(0)]),
        node(vec![Edge::Call {
            target: 0,
            follow: 0,
        }]),
        node(vec![Edge::First {
            path: "original".into(),
            target: 0,
        }]),
        node(vec![Edge::Full { target: 0, stop: 0 }]),
        node(vec![Edge::Epsilon(0), word(0)]),
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
        State::default(),
    ] {
        let graph = [root];
        let old = run(false, &graph, &[0, 0], 10000, None)?;
        let new = run(true, &graph, &[0, 0], 10000, None)?;
        assert_eq!(old.result, new.result);
        assert_eq!(old.remaining, new.remaining);
        assert_eq!(new.topology.pure_key_expansions, 0);
        assert_eq!(new.topology.distinct_completed_interior_keys, 0);
    }
    let dangling = [node(vec![Edge::Epsilon(99)])];
    let old = run(false, &dangling, &[0], 10000, None)?;
    let new = run(true, &dangling, &[0], 10000, None)?;
    assert_eq!(old.result, new.result);
    assert_eq!(old.remaining, new.remaining);
    assert!(new.poisoned);
    assert_eq!(new.topology.distinct_completed_interior_keys, 0);
    let graph = graph();
    let full = run(true, &graph, &[0, 1], 10000, None)?;
    for cancellation in 1..=full.attempts {
        let old = run(false, &graph, &[0, 1], 10000, Some(cancellation))?;
        let new = run(true, &graph, &[0, 1], 10000, Some(cancellation))?;
        assert_eq!(old.result, new.result);
        assert_eq!(old.remaining, new.remaining);
        assert_eq!(old.work, new.work);
        assert_eq!(new.result, Err("actual kernel cancellation".into()));
        assert!(new.poisoned);
    }
    let fresh = run(true, &graph, &[0, 1], 10000, None)?;
    assert_eq!(fresh.result, full.result);
    assert_eq!(fresh.topology, full.topology);
    Ok(())
}
#[test]
fn topology_observer_second_queue_and_different_program_never_merge_dependencies()
-> Result<(), String> {
    let capture = CompilerPoolDiagnostic::new();
    capture.enable_first_queue_topology();
    let graph = graph();
    let source = SourceId::for_text("test:first", "source-A");
    let ordinal = capture
        .begin(
            RunStage::Executable,
            &source,
            RecursiveFrontierOrder::BreadthFirstSharedCallPorts,
        )
        .ok_or("Missing program")?;
    let pool = super::super::PoolWork::new_with_topology(10000, &capture);
    let mut remaining = 10000;
    let token = pool.enter(super::super::PoolBlock::PoolRemainder, remaining);
    let calls = Cell::new(0);
    let mut a = Kernel::<BorrowedCompletedComponents>::new(&graph);
    take(&mut a, 0, &mut remaining, &pool, &calls, None)?;
    let mut b = Kernel::<BorrowedCompletedComponents>::new(&graph);
    take(&mut b, 1, &mut remaining, &pool, &calls, None)?;
    pool.leave(token, remaining, true);
    pool.end(remaining, true);
    capture.finish(ordinal, pool.snapshot());
    let first = capture.first_queue_topology_snapshot();
    assert_eq!(first.coverage, Coverage::Unavailable);
    assert_eq!(first.unavailable, Some(Unavailable::MultipleQueues));
    assert_eq!(first.queue_scopes, 2);
    assert_eq!(first.earlier_completed_interior_reexpansions, 0);
    let ordinal = capture
        .begin(
            RunStage::Parent,
            &SourceId::for_text("test:second", "source-B"),
            RecursiveFrontierOrder::BreadthFirstSharedCallPorts,
        )
        .ok_or("Missing second program")?;
    let second = super::super::PoolWork::new_with_topology(10000, &capture);
    let token = second.enter(super::super::PoolBlock::PoolRemainder, 10000);
    let mut remaining = 10000;
    let mut fresh = Kernel::<BorrowedCompletedComponents>::new(&graph);
    take(&mut fresh, 1, &mut remaining, &second, &calls, None)?;
    second.leave(token, remaining, true);
    second.end(remaining, true);
    capture.finish(ordinal, second.snapshot());
    let row = capture.first_queue_topology_snapshot();
    assert_eq!(row.program_ordinal, Some(2));
    assert_eq!(row.coverage, Coverage::FirstQueue);
    assert_eq!(row.earlier_completed_interior_reexpansions, 0);
    assert_eq!(
        capture
            .snapshot()
            .last
            .ok_or("Missing source binding")?
            .source
            .ok_or("Missing source")?
            .uri
            .as_str()
            .map_err(|e| e.to_string())?,
        "test:second"
    );
    let ordinal = capture
        .begin(
            RunStage::Executable,
            &source,
            RecursiveFrontierOrder::BreadthFirstSharedCallPorts,
        )
        .ok_or("Missing decision program")?;
    capture.decision("decision-A");
    let scoped = super::super::PoolWork::new_with_topology(10000, &capture);
    let token = scoped.enter(super::super::PoolBlock::PoolRemainder, 10000);
    let mut remaining = 10000;
    let mut fresh = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let actual = take(&mut fresh, 0, &mut remaining, &scoped, &calls, None)?;
    assert_eq!(
        actual,
        Some(vec![Port {
            state: 3,
            action_seen: false
        }])
    );
    let before_decision = scoped.snapshot();
    capture.decision("decision-A");
    assert_eq!(
        capture.first_queue_topology_snapshot().coverage,
        Coverage::FirstQueue
    );
    capture.decision("decision-B");
    assert_eq!(scoped.snapshot(), before_decision);
    scoped.leave(token, remaining, true);
    scoped.end(remaining, true);
    capture.finish(ordinal, scoped.snapshot());
    let row = capture.first_queue_topology_snapshot();
    assert_eq!(row.coverage, Coverage::Unavailable);
    assert_eq!(row.unavailable, Some(Unavailable::DecisionChanged));
    assert_eq!(row.queue_scopes, 1);
    assert_eq!(row.earlier_completed_interior_reexpansions, 0);
    assert!(row.window_ended);
    assert_eq!(
        capture
            .snapshot()
            .last
            .ok_or("Missing last program")?
            .decision
            .ok_or("Missing decision")?
            .as_str()
            .map_err(|error| error.to_string())?,
        "decision-B"
    );
    Ok(())
}
fn prediction(depth: usize) -> ParserPrediction {
    let mut states = vec![State::default(); 2 * (depth + 1) + 1];
    let end = states.len() - 1;
    for i in 0..depth {
        for side in 0..2 {
            states[2 * i + side] = node(vec![Edge::Call {
                target: 2 * (i + 1) + side,
                follow: end,
            }]);
        }
    }
    states[2 * depth] = node(vec![Edge::Token(
        PredictionSymbol::Keyword("left".into()),
        end,
    )]);
    states[2 * depth + 1] = node(vec![Edge::Token(
        PredictionSymbol::Keyword("right".into()),
        end,
    )]);
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
struct Classified {
    result: Result<bool, String>,
    witnesses: BTreeMap<(usize, usize), bool>,
    remaining: usize,
    topology: FirstQueueTopologySnapshot,
    work: WorkSnapshot,
}
fn classify(
    p: &ParserPrediction,
    entries: Vec<usize>,
    enabled: bool,
    grant: usize,
) -> Result<Classified, String> {
    let capture = CompilerPoolDiagnostic::new();
    if enabled {
        capture.enable_first_queue_topology();
    }
    let ordinal = capture
        .begin(
            RunStage::Executable,
            &SourceId::for_text("test:canonical", "one immutable graph"),
            RecursiveFrontierOrder::BreadthFirstSharedCallPorts,
        )
        .ok_or("Missing program")?;
    let pool = super::super::PoolWork::new_with_topology(grant, &capture);
    let mut remaining = grant;
    let token = pool.enter(super::super::PoolBlock::PoolRemainder, remaining);
    let mut witnesses = BTreeMap::new();
    let initial = entries
        .into_iter()
        .enumerate()
        .map(|(alternative, state)| AnalysisConfiguration {
            state,
            alternative,
            calls: SharedCalls::empty(),
        })
        .collect();
    let result=p.multiple_recursive_configurations_with_call_storage::<SharedCalls,SharedCallPortQueue<'_>,frontier::AdoptFrontiers,true,super::super::partial_token_probe::NoProbe>(initial,&mut remaining,true,&mut witnesses,false,(true,None,None,None),&mut super::super::partial_token_probe::NoProbe,Some(&pool));
    pool.leave(token, remaining, result.is_ok());
    pool.end(remaining, result.is_ok());
    let work = pool.snapshot();
    capture.finish(ordinal, work);
    assert!(work.balanced && work.ended);
    Ok(Classified {
        result,
        witnesses,
        remaining,
        topology: capture.first_queue_topology_snapshot(),
        work,
    })
}
#[test]
fn topology_observer_canonical_call_depth128_recursion_and_dangling_keep_full_results()
-> Result<(), String> {
    for depth in [1, 8, 128, 129] {
        let p = prediction(depth);
        let old = classify(&p, vec![0, 1], false, 100000)?;
        let new = classify(&p, vec![0, 1], true, 100000)?;
        assert_eq!(old.result, new.result);
        assert_eq!(old.witnesses, new.witnesses);
        assert_eq!(old.remaining, new.remaining);
        assert_eq!(old.work, new.work);
        assert_eq!(
            new.result,
            if depth <= 128 {
                Ok(false)
            } else {
                Err("Parser prediction call depth exceeded".into())
            }
        );
    }
    let mut p = prediction(2);
    p.states[2] = node(vec![Edge::Call {
        target: 2,
        follow: p.states.len() - 1,
    }]);
    for dangling in [false, true] {
        if dangling {
            p.states[2] = node(vec![Edge::Epsilon(p.states.len() + 1)]);
        }
        let old = classify(&p, vec![0, 1], false, 10000)?;
        let new = classify(&p, vec![0, 1], true, 10000)?;
        assert_eq!(old.result, new.result);
        assert_eq!(old.witnesses, new.witnesses);
        assert_eq!(old.remaining, new.remaining);
        assert_eq!(old.work, new.work);
    }
    Ok(())
}
#[test]
fn topology_observer_whole_cold_canonical_consumer_preserves_work_witnesses_and_each_budget_boundary()
-> Result<(), String> {
    let mut states = graph();
    let end = states.len() - 1;
    states[end].rule_stop = true;
    let a = states.len();
    states.push(node(vec![Edge::Call {
        target: 0,
        follow: end,
    }]));
    let b = states.len();
    states.push(node(vec![Edge::Call {
        target: 1,
        follow: end - 1,
    }]));
    let p = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states,
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    let old = classify(&p, vec![a, b], false, 10000)?;
    let new = classify(&p, vec![a, b], true, 10000)?;
    println!(
        "FIRST_QUEUE_TOPOLOGY_CANONICAL {:?}",
        (
            &old.result,
            &new.result,
            &old.witnesses,
            &new.witnesses,
            10000 - old.remaining,
            10000 - new.remaining,
            new.topology
        )
    );
    assert_eq!(old.result, new.result);
    assert_eq!(old.witnesses, new.witnesses);
    assert_eq!(old.remaining, new.remaining);
    assert_eq!(old.work, new.work);
    let used = 10000 - old.remaining;
    for grant in [0, 1, used - 1, used] {
        let old = classify(&p, vec![a, b], false, grant)?;
        let new = classify(&p, vec![a, b], true, grant)?;
        assert_eq!(old.result, new.result);
        assert_eq!(old.witnesses, new.witnesses);
        assert_eq!(old.remaining, new.remaining);
        assert_eq!(old.work, new.work);
        assert_eq!(new.remaining, 0);
    }
    Ok(())
}
