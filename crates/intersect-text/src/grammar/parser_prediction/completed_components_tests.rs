//! Authored paid graph-component controls, no original input utility inference.
use super::super::{Decision, ParserPrediction, PredictionSymbol, frontier};
use super::*;

fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn token(name: &str, next: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(name.into()), next)
}
fn graph() -> Vec<State> {
    let mut graph = vec![State::default(); 84];
    graph[0] = node(vec![Edge::Epsilon(1), Edge::Action(2)]);
    graph[1] = node(vec![Edge::Epsilon(20)]);
    graph[2] = node(vec![Edge::Epsilon(20)]);
    for (state, row) in graph.iter_mut().enumerate().take(80).skip(20) {
        *row = node(vec![Edge::Epsilon(state + 1)]);
    }
    graph[40] = node(vec![Edge::Epsilon(41), Edge::Action(41)]);
    graph[80] = node(vec![token("x", 81), token("y", 82)]);
    graph
}
fn take<P: ComponentPolicy>(
    source: &mut Kernel<'_, P>,
    state: usize,
    returns: usize,
    remaining: &mut usize,
) -> Result<Option<Vec<Port>>, String> {
    let result = (|| {
        let ports = source
            .borrow_ports_for_configurations(state, returns, &mut || analysis_charge(remaining))?;
        let mut out = Vec::new(); // test projection only, outside production kernel
        if let Some(ports) = ports {
            for port in ports {
                analysis_charge(remaining)?; // real consumer descriptor read
                out.push(*port);
            }
            Ok(Some(out))
        } else {
            Ok(None)
        }
    })();
    if result.is_err() {
        source.invalidate();
    }
    result
}

#[test]
fn composed_component_complete_cold_and_common_suffix_pay_all_work_with_actual_reuse()
-> Result<(), String> {
    let graph = graph();
    let mut old = Summaries::new(&graph);
    let mut new = ComponentSummaries::new(&graph);
    let mut a = 10000;
    let mut b = 10000;
    let old_first = take(&mut old, 20, 2, &mut a)?;
    let new_first = take(&mut new, 20, 2, &mut b)?;
    assert_eq!(old_first, new_first);
    assert_eq!(
        b + 2,
        a,
        "First cold pays eligibility and retained node-certificate copy"
    );
    let cold = (10000 - a, 10000 - b);
    let old_second = take(&mut old, 0, 3, &mut a)?;
    let new_second = take(&mut new, 0, 3, &mut b)?;
    println!(
        "COMPOSED_COLD_PREASSERT {:?}",
        (cold, 10000 - a, 10000 - b, old.stats(), new.stats())
    );
    assert_eq!(old_second, new_second);
    let ports = new_second.ok_or("Missing genuine completed component")?;
    assert!(ports.iter().any(|port| port.action_seen));
    assert!(ports.iter().any(|port| !port.action_seen));
    assert!(
        new.stats().hits >= 2,
        "Actual incoming false and true completed-Some lookups"
    );
    assert!(
        b > a,
        "Whole cold setup+first proof+second full-row/consumer costs, no fee refund"
    );
    assert_eq!(old.stats().publications, new.stats().publications);
    // A complete unrelated cold root is honestly more expensive when the
    // nonempty table supplies no reusable child. Never claim every root wins.
    let mut disconnected = graph.clone();
    disconnected[3] = node(vec![Edge::Epsilon(4)]);
    disconnected[4] = node(vec![Edge::Epsilon(80)]);
    let mut literal = Summaries::new(&disconnected);
    let mut selected = ComponentSummaries::new(&disconnected);
    let mut left = 10000;
    let mut right = 10000;
    assert_eq!(
        take(&mut literal, 20, 0, &mut left)?,
        take(&mut selected, 20, 0, &mut right)?
    );
    assert_eq!(
        take(&mut literal, 3, 0, &mut left)?,
        take(&mut selected, 3, 0, &mut right)?
    );
    println!(
        "COMPOSED_NOHIT_PREASSERT {:?}",
        (10000 - left, 10000 - right, selected.stats())
    );
    assert_eq!(selected.stats().hits, 0);
    assert!(
        right < left,
        "New certificate/read/miss work is not free or refunded"
    );
    Ok(())
}

#[test]
fn composed_component_zero_exact_short_preconsumed_and_each_denial_poison_original_scope()
-> Result<(), String> {
    let graph = graph();
    let mut measured = ComponentSummaries::new(&graph);
    let mut available = 10000;
    take(&mut measured, 20, 1, &mut available)?;
    take(&mut measured, 0, 2, &mut available)?;
    let required = 10000 - available;
    for (allowance, prefix) in [(0, 0), (required - 1, 0), (required, 0), (required + 7, 7)] {
        let mut source = ComponentSummaries::new(&graph);
        let mut remaining = allowance;
        for _ in 0..prefix {
            analysis_charge(&mut remaining)?;
        }
        let result = take(&mut source, 20, 1, &mut remaining)
            .and_then(|_| take(&mut source, 0, 2, &mut remaining));
        if allowance - prefix == required {
            assert!(result.is_ok());
            assert_eq!(remaining, 0);
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert!(source.poisoned && source.completed.is_empty());
        }
    }
    for allowed in 0..required {
        let mut source = ComponentSummaries::new(&graph);
        let mut remaining = allowed;
        let result = take(&mut source, 20, 1, &mut remaining)
            .and_then(|_| take(&mut source, 0, 2, &mut remaining));
        assert!(result.is_err() && source.poisoned && source.completed.is_empty());
    }
    let mut fresh = ComponentSummaries::new(&graph);
    let mut exact = required;
    take(&mut fresh, 20, 1, &mut exact)?;
    take(&mut fresh, 0, 2, &mut exact)?;
    assert_eq!(exact, 0);
    let mut cancelled = ComponentSummaries::new(&graph);
    let mut calls = 0;
    let error = cancelled
        .borrow_ports_for_configurations(20, 0, &mut || {
            calls += 1;
            if calls == 17 {
                Err("caller cancelled".to_string())
            } else {
                Ok(())
            }
        })
        .map(|_| ());
    assert_eq!(error, Err("caller cancelled".into()));
    assert!(cancelled.poisoned && cancelled.completed.is_empty());
    Ok(())
}

#[test]
fn composed_component_some_does_not_hide_source_order_cycle_dangling_or_effect_ports()
-> Result<(), String> {
    let mut graph = graph();
    graph[0] = node(vec![Edge::Epsilon(20), Edge::Epsilon(0)]);
    graph[3] = node(vec![Edge::Epsilon(0)]);
    let mut source = ComponentSummaries::new(&graph);
    let mut work = 10000;
    assert!(take(&mut source, 20, 0, &mut work)?.is_some());
    assert_eq!(take(&mut source, 0, 0, &mut work)?, None);
    assert!(!source.poisoned && source.completed.get(&0).is_some_and(Option::is_none));
    // A completed structural None is not a reusable recursion certificate.
    // Another pure root must still traverse it and observe its actual cycle.
    // The immutable graph association requires a fresh scope for this graph.
    let mut second = ComponentSummaries::new(&graph);
    let mut second_work = 10000;
    assert_eq!(take(&mut second, 0, 0, &mut second_work)?, None);
    assert_eq!(take(&mut second, 3, 0, &mut second_work)?, None);
    assert_eq!(second.stats().publications, 0);
    assert!(second.completed.get(&3).is_some_and(Option::is_none));
    for effect in [
        token("z", 0),
        Edge::Call {
            target: 0,
            follow: 0,
        },
        Edge::First {
            path: "real".into(),
            target: 0,
        },
        Edge::Full { target: 0, stop: 0 },
    ] {
        let roots = vec![node(vec![effect])];
        let mut scoped = ComponentSummaries::new(&roots);
        let mut budget = 100;
        assert_eq!(take(&mut scoped, 0, 0, &mut budget)?, None);
        assert!(scoped.completed.is_empty() && scoped.colors.is_empty());
    }
    let bad = vec![node(vec![Edge::Epsilon(99)])];
    let mut scoped = ComponentSummaries::new(&bad);
    let mut budget = 1000;
    assert_eq!(
        take(&mut scoped, 0, 0, &mut budget),
        Err("Dangling parser prediction state".into())
    );
    assert!(scoped.poisoned && scoped.completed.is_empty());
    Ok(())
}

#[test]
fn composed_component_node_certificate_never_bypasses4096_and_default_holders_keep_types()
-> Result<(), String> {
    assert_eq!(
        (
            MAX_KEYS,
            MAX_LOGICAL_BYTES,
            HEADER_BYTES,
            FRAME_BYTES,
            SUMMARY_BYTES
        ),
        (4096, 1048576, 128, 32, 64)
    );
    let mut g = vec![node(vec![]); MAX_KEYS + 2];
    for (state, row) in g.iter_mut().enumerate().take(MAX_KEYS + 1) {
        *row = node(vec![Edge::Epsilon(state + 1)]);
    }
    g[MAX_KEYS + 1] = node(vec![token("end", 0)]);
    let literal = Summaries::new(&g);
    let exact_old_map: &BTreeMap<usize, Option<Vec<Port>>> = &literal.completed;
    assert!(exact_old_map.is_empty() && literal.colors.is_empty() && !literal.admitted);
    let mut scope = ComponentSummaries::new(&g);
    let mut work = 500000;
    assert!(take(&mut scope, 1, 0, &mut work)?.is_some()); // exact4096 pure-node certificate
    assert_eq!(
        take(&mut scope, 0, 0, &mut work),
        Err("Transparent port node bound exceeded".into())
    );
    assert!(scope.poisoned && scope.completed.is_empty());
    Ok(())
}

fn run<'g, Q: ClosureQueue<'g>>(
    p: &'g ParserPrediction,
    entries: Vec<usize>,
    shared: bool,
    work: &mut usize,
) -> (Result<bool, String>, BTreeMap<(usize, usize), bool>) {
    let mut witnesses = BTreeMap::new();
    let result = p
        .multiple_recursive_alternatives_with_frontiers::<Q, frontier::AdoptFrontiers, true>(
            &Decision {
                entries,
                exit: None,
            },
            work,
            shared,
            &mut witnesses,
            false,
            (true, None, None, None),
        );
    (result, witnesses)
}
#[test]
fn composed_component_canonical_full_keys_call_return_follow_and_witness_result_match_old() {
    for recursive in [false, true] {
        let mut g = graph();
        // These are real canonical Calls. Their distinct ordered return
        // frames are produced by the worker, not fabricated by the test.
        g[8] = node(vec![Edge::Call {
            target: 20,
            follow: 10,
        }]);
        g[9] = node(vec![Edge::Call {
            target: 0,
            follow: 11,
        }]);
        g[10] = State {
            rule_stop: true,
            ..State::default()
        };
        g[11] = State {
            rule_stop: true,
            ..State::default()
        };
        g[80] = node(vec![Edge::Call {
            target: 81,
            follow: 83,
        }]);
        g[81] = node(vec![token("r", 82)]);
        g[82] = if recursive {
            node(vec![Edge::Call {
                target: 81,
                follow: 83,
            }])
        } else {
            State {
                rule_stop: true,
                ..State::default()
            }
        };
        g[83] = State {
            rule_stop: true,
            ..State::default()
        };
        let p = ParserPrediction {
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            allow_empty_choices: false,
            states: g,
            rules: BTreeMap::new(),
            decisions: BTreeMap::new(),
            first_guards: BTreeMap::new(),
            recursive_decisions: BTreeSet::new(),
            runtime_requirement: None,
        };
        let before = serde_json::to_value(&p).unwrap();
        let mut old = 10000;
        let mut new = 10000;
        let a = run::<PortQueue<'_>>(&p, vec![8, 9], true, &mut old);
        let b = run::<ComponentPortQueue<'_>>(&p, vec![8, 9], true, &mut new);
        println!("COMPOSED_CLASSIFIER_PREASSERT {:?}", (&a, &b, old, new));
        assert_eq!(a, b);
        assert_eq!(a.0, Ok(recursive));
        assert_eq!(serde_json::to_value(&p).unwrap(), before);
    }
    // Real canonical Calls at128 and129; the structural queue cannot waive
    // depth. This larger authored component grant isolates the128 guard and
    // is not a change to any original compiler10000 allowance.
    for depth in [128, 129] {
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
        states[2 * depth] = node(vec![token("left", end)]);
        states[2 * depth + 1] = node(vec![token("right", end)]);
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
        let mut old = 100000;
        let mut new = 100000;
        let a = run::<PortQueue<'_>>(&p, vec![0, 1], false, &mut old);
        let b = run::<ComponentPortQueue<'_>>(&p, vec![0, 1], false, &mut new);
        assert_eq!(a, b);
        assert_eq!(
            b.0,
            if depth == 128 {
                Ok(false)
            } else {
                Err("Parser prediction call depth exceeded".into())
            }
        );
    }
}
