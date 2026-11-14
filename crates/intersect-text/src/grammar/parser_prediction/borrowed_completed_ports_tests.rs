//! Authored full-cost source controls. Original SysML admission is unproven.
use super::super::{Decision, ParserPrediction, PredictionSymbol, frontier};
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
fn take<P: ComponentPolicy>(
    source: &mut Kernel<'_, P>,
    start: usize,
    returns: usize,
    work: &mut usize,
) -> Result<Option<Vec<Port>>, String> {
    // SAME complete outer boundary as the real consumer: an external descriptor
    // read can fail after the immutable view was returned, so clean up that Err.
    let result = (|| {
        let ports = source
            .borrow_ports_for_configurations(start, returns, &mut || analysis_charge(work))?;
        let mut result = Vec::new(); // test projection only
        if let Some(ports) = ports {
            for port in ports {
                analysis_charge(work)?;
                result.push(*port);
            }
            Ok(Some(result))
        } else {
            Ok(None)
        }
    })();
    if result.is_err() {
        source.invalidate();
    }
    result
}
fn joins() -> Vec<State> {
    let mut graph = vec![State::default(); 12];
    graph[0] = node(
        (0..32)
            .flat_map(|_| [Edge::Epsilon(8), Edge::Action(8)])
            .collect(),
    );
    graph[1] = node(vec![Edge::Epsilon(0), Edge::Action(0)]);
    graph[2] = node(vec![Edge::Epsilon(3)]);
    graph[3] = node(vec![Edge::Epsilon(8)]);
    graph[8] = node(vec![token("x", 9), token("y", 10)]);
    graph
}
#[test]
fn borrowed_completed_entries_cold_warm_and_unrelated_roots_pay_all_rows_and_remove_real_work()
-> Result<(), String> {
    let graph = joins();
    let unchanged = serde_json::to_value(&graph).map_err(|e| e.to_string())?;
    let mut old = Kernel::<CompletedComponents>::new(&graph);
    let mut new = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let (mut a, mut b) = (10000, 10000);
    for (ordinal, root, returns) in [(0, 0, 3), (1, 1, 7), (2, 2, 1), (3, 0, 2)] {
        let x = take(&mut old, root, returns, &mut a)?;
        let y = take(&mut new, root, returns, &mut b)?;
        println!(
            "BORROWED_COMPONENT_COST {:?}",
            (
                ordinal,
                root,
                returns,
                &x,
                &y,
                10000 - a,
                10000 - b,
                old.stats(),
                new.stats()
            )
        );
        assert_eq!(x, y);
        assert!(
            b > a,
            "whole first-cold and later costs include setup, original full row copies and consumer reads"
        );
        assert_eq!(old.stats().logical_bytes, new.stats().logical_bytes);
        assert_eq!(old.stats().publications, new.stats().publications);
    }
    let ports = take(&mut new, 0, 0, &mut b)?.ok_or("Expected real opaque ports")?;
    assert_eq!(
        ports,
        vec![
            Port {
                state: 8,
                action_seen: false
            },
            Port {
                state: 8,
                action_seen: true
            }
        ]
    );
    assert_eq!(
        serde_json::to_value(&graph).map_err(|e| e.to_string())?,
        unchanged
    );
    Ok(())
}
#[test]
fn borrowed_completed_entries_root_effects_mixed_flags_cycles_and_dangling_are_literal()
-> Result<(), String> {
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
        node(vec![Edge::Epsilon(0), token("x", 0)]),
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
        let mut old = Kernel::<CompletedComponents>::new(&graph);
        let mut new = Kernel::<BorrowedCompletedComponents>::new(&graph);
        let (mut a, mut b) = (100, 100);
        for _ in 0..3 {
            assert_eq!(take(&mut old, 0, 2, &mut a)?, None);
            assert_eq!(take(&mut new, 0, 2, &mut b)?, None);
            assert_eq!(a, b);
            assert!(old.completed.is_empty() && new.completed.is_empty());
            assert!(old.colors.is_empty() && new.colors.is_empty());
        }
    }
    let graph = [node(vec![Edge::Epsilon(1)]), node(vec![Edge::Action(0)])];
    let mut scope = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut work = 10000;
    assert_eq!(take(&mut scope, 0, 0, &mut work)?, None);
    assert_eq!(take(&mut scope, 1, 0, &mut work)?, None);
    assert_eq!(scope.stats().publications, 0);
    assert!(!scope.poisoned);
    let bad = [node(vec![Edge::Epsilon(99)])];
    let mut scope = Kernel::<BorrowedCompletedComponents>::new(&bad);
    assert_eq!(
        take(&mut scope, 0, 0, &mut work),
        Err("Dangling parser prediction state".into())
    );
    assert!(scope.poisoned && scope.completed.is_empty() && scope.colors.is_empty());
    Ok(())
}
#[test]
fn borrowed_completed_entries_exact_short_preconsumed_each_denial_and_cancel_poison()
-> Result<(), String> {
    let graph = joins();
    let mut full = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut remaining = 10000;
    take(&mut full, 0, 3, &mut remaining)?;
    let first_required = 10000 - remaining;
    take(&mut full, 1, 7, &mut remaining)?;
    let required = 10000 - remaining;
    for allowed in 0..=required {
        let mut scope = Kernel::<BorrowedCompletedComponents>::new(&graph);
        let mut work = allowed;
        let result =
            take(&mut scope, 0, 3, &mut work).and_then(|_| take(&mut scope, 1, 7, &mut work));
        assert_eq!(work, 0);
        if allowed == required {
            assert!(result.is_ok() && !scope.poisoned);
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert!(scope.poisoned && scope.completed.is_empty() && scope.colors.is_empty());
        }
    }
    let mut scope = Kernel::<BorrowedCompletedComponents>::new(&graph);
    let mut work = required + 7;
    for _ in 0..7 {
        analysis_charge(&mut work)?;
    }
    take(&mut scope, 0, 3, &mut work)?;
    take(&mut scope, 1, 7, &mut work)?;
    assert_eq!(work, 0);
    for denied_at in [1, 17, first_required / 2] {
        let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
        let mut calls = 0;
        let mut charge = || {
            calls += 1;
            if calls == denied_at {
                Err("actual caller cancellation".to_string())
            } else {
                Ok(())
            }
        };
        let actual = source
            .borrow_ports_for_configurations(0, 3, &mut charge)
            .map(|_| ());
        // All selected positions precede the first returned view on this graph.
        assert_eq!(actual, Err("actual caller cancellation".into()));
        assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
    }
    Ok(())
}
fn run<'g, Q: ClosureQueue<'g>>(
    p: &'g ParserPrediction,
    work: &mut usize,
    entries: Vec<usize>,
) -> (Result<bool, String>, BTreeMap<(usize, usize), bool>) {
    let mut witnesses = BTreeMap::new();
    let result = p
        .multiple_recursive_alternatives_with_frontiers::<Q, frontier::AdoptFrontiers, true>(
            &Decision {
                entries,
                exit: None,
            },
            work,
            true,
            &mut witnesses,
            false,
            (true, None, None, None),
        );
    (result, witnesses)
}
#[test]
fn borrowed_completed_entries_real_calls_follow_full_keys_and_witness_outcomes_match()
-> Result<(), String> {
    for recursive in [false, true] {
        let mut graph = vec![State::default(); 8];
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
        graph[4] = node(vec![token("r", 5)]);
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
        let p = ParserPrediction {
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            allow_empty_choices: false,
            states: graph,
            rules: BTreeMap::new(),
            decisions: BTreeMap::new(),
            first_guards: BTreeMap::new(),
            recursive_decisions: BTreeSet::new(),
            runtime_requirement: None,
        };
        let (mut a, mut b) = (10000, 10000);
        let old = run::<ComponentPortQueue<'_>>(&p, &mut a, vec![0, 1]);
        let new = run::<BorrowedComponentPortQueue<'_>>(&p, &mut b, vec![0, 1]);
        println!("BORROWED_COMPONENT_CLASSIFIER {:?}", (&old, &new, a, b));
        assert_eq!(old, new);
        assert_eq!(new.0, Ok(recursive));
        assert!(b > a);
    }
    // Each caller tail remains in the exact configuration key; no port table
    // owns caller authority or may waive the same canonical128 call guard.
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
        let p = ParserPrediction {
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            allow_empty_choices: false,
            states: graph,
            rules: BTreeMap::new(),
            decisions: BTreeMap::new(),
            first_guards: BTreeMap::new(),
            recursive_decisions: BTreeSet::new(),
            runtime_requirement: None,
        };
        let (mut a, mut b) = (100000, 100000);
        let x = run::<ComponentPortQueue<'_>>(&p, &mut a, vec![0, 1]);
        let y = run::<BorrowedComponentPortQueue<'_>>(&p, &mut b, vec![0, 1]);
        assert_eq!(x, y);
        assert_eq!(
            y.0,
            if depth == 128 {
                Ok(false)
            } else {
                Err("Parser prediction call depth exceeded".into())
            }
        );
    }
    Ok(())
}
#[test]
fn borrowed_completed_entries_caps_epochs_default_layout_and_fresh_graph_are_preserved()
-> Result<(), String> {
    assert_eq!(
        (
            MAX_KEYS,
            MAX_LOGICAL_BYTES,
            HEADER_BYTES,
            FRAME_BYTES,
            PORT_BYTES,
            CONFIGURATION_BYTES,
            RETURN_BYTES
        ),
        (4096, 1048576, 128, 32, 32, 48, 16)
    );
    let graph = joins();
    let literal = Summaries::new(&graph);
    let exact: &BTreeMap<usize, Option<Vec<Port>>> = &literal.completed;
    assert!(exact.is_empty());
    assert_eq!(
        std::mem::size_of::<Kernel<'_, CompletedComponents>>(),
        std::mem::size_of::<Kernel<'_, BorrowedCompletedComponents>>()
    );
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&graph);
    source.epoch = MAX_EPOCH;
    let mut work = 10000;
    assert_eq!(
        take(&mut source, 0, 0, &mut work),
        Err("Transparent port visitation epoch exhausted".into())
    );
    assert!(source.poisoned && source.completed.is_empty());
    let mut chain = vec![State::default(); MAX_KEYS + 2];
    for (i, row) in chain.iter_mut().enumerate().take(MAX_KEYS + 1) {
        *row = node(vec![Edge::Epsilon(i + 1)]);
    }
    chain[MAX_KEYS + 1] = node(vec![token("end", 0)]);
    let mut source = Kernel::<BorrowedCompletedComponents>::new(&chain);
    let mut work = 500000;
    assert!(take(&mut source, 1, 0, &mut work)?.is_some());
    assert_eq!(
        take(&mut source, 0, 0, &mut work),
        Err("Transparent port node bound exceeded".into())
    );
    assert!(source.poisoned && source.completed.is_empty());
    let other = [node(vec![Edge::Epsilon(1)]), node(vec![token("other", 0)])];
    let mut fresh = Kernel::<BorrowedCompletedComponents>::new(&other);
    let mut work = 10000;
    assert_eq!(
        take(&mut fresh, 0, 0, &mut work)?,
        Some(vec![Port {
            state: 1,
            action_seen: false
        }])
    );
    Ok(())
}
