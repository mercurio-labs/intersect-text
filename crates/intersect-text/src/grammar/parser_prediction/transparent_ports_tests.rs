//! Authored component controls. No original KerML/SysML utility is inferred.
use super::super::{
    Decision, ParserPrediction, PredictionSymbol, RecursiveFrontierOrder, frontier,
};
use super::*;

fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn stop() -> State {
    State {
        rule_stop: true,
        ..State::default()
    }
}
fn token(name: &str, target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(name.into()), target)
}
fn graph(states: Vec<State>) -> ParserPrediction {
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
fn run<'g, Q: ClosureQueue<'g>>(
    p: &'g ParserPrediction,
    entries: Vec<usize>,
    work: &mut usize,
    shared: bool,
) -> (Result<bool, String>, BTreeMap<(usize, usize), bool>) {
    let mut witnesses = BTreeMap::new();
    let value = p
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
    (value, witnesses)
}

#[test]
fn port_policy_fixed_portable_slots_and_all_root_effects_stay_literal() {
    assert_eq!(
        (
            HEADER_BYTES,
            COLOR_BYTES,
            FRAME_BYTES,
            PORT_BYTES,
            VECTOR_BYTES,
            SUMMARY_BYTES,
            CONFIGURATION_BYTES,
            RETURN_BYTES,
            MAX_KEYS,
            MAX_LOGICAL_BYTES
        ),
        (128, 4, 32, 32, 32, 64, 48, 16, 4096, 1_048_576)
    );
    let roots = vec![
        stop(),
        State {
            accepting: true,
            edges: vec![Edge::Epsilon(1)],
            ..State::default()
        },
        State {
            follow: vec![1],
            edges: vec![Edge::Epsilon(1)],
            ..State::default()
        },
        State::default(),
        node(vec![token("x", 1)]),
        node(vec![Edge::Call {
            target: 1,
            follow: 1,
        }]),
        node(vec![Edge::First {
            path: "actual".into(),
            target: 1,
        }]),
        node(vec![Edge::Full { target: 1, stop: 1 }]),
        node(vec![Edge::Epsilon(1), token("mixed", 1)]),
    ];
    for root in roots {
        let states = vec![root, stop()];
        let mut source = Summaries::new(&states);
        assert_eq!(source.stats(), SummaryStats::default());
        let mut work = 1000;
        assert_eq!(
            source.ports(0, &mut || analysis_charge(&mut work)).unwrap(),
            None
        );
        assert!(work < 1000);
        assert_eq!(source.stats().builds, 0);
        assert_eq!(source.stats().fallbacks, 1);
        assert!(source.completed.is_empty());
        assert!(source.colors.is_empty());
        assert_eq!(source.stats().logical_bytes, 0);
    }
}

#[test]
fn port_diamond_join_preserves_action_transfer_and_actual_boundary_identities() {
    let boundaries = vec![
        stop(),
        State {
            accepting: true,
            ..State::default()
        },
        node(vec![token("x", 0)]),
        node(vec![Edge::Call {
            target: 0,
            follow: 1,
        }]),
        node(vec![Edge::First {
            path: "guard".into(),
            target: 0,
        }]),
        node(vec![Edge::Full { target: 0, stop: 1 }]),
        node(vec![Edge::Epsilon(0), token("mixed", 1)]),
    ];
    for boundary in boundaries {
        let states = vec![
            node(vec![Edge::Epsilon(1), Edge::Action(2)]),
            node(vec![Edge::Epsilon(3)]),
            node(vec![Edge::Epsilon(3)]),
            boundary,
        ];
        let mut source = Summaries::new(&states);
        let mut work = 10000;
        assert_eq!(
            source
                .copy_ports(0, &mut || analysis_charge(&mut work))
                .unwrap(),
            Some(vec![
                Port {
                    state: 3,
                    action_seen: false
                },
                Port {
                    state: 3,
                    action_seen: true
                }
            ])
        );
        let before = source.stats();
        assert_eq!(before.fallbacks, 0); // a black DAG join is not a cycle
        assert_eq!(before.port_copies, 2);
        assert_eq!(
            source
                .copy_ports(0, &mut || analysis_charge(&mut work))
                .unwrap()
                .unwrap()
                .len(),
            2
        );
        let after = source.stats();
        assert_eq!(
            (after.lookups, after.builds, after.hits, after.port_copies),
            (2, 1, 1, 4)
        );
        assert!(after.logical_bytes > before.logical_bytes); // warm copies are paid
    }
}

#[test]
fn port_cycle_fallback_dangling_and_denied_scopes_never_publish_partial_authority() {
    let cycle = vec![node(vec![Edge::Epsilon(1)]), node(vec![Edge::Action(0)])];
    let mut source = Summaries::new(&cycle);
    let mut work = 10000;
    assert_eq!(
        source
            .copy_ports(0, &mut || analysis_charge(&mut work))
            .unwrap(),
        None
    );
    assert_eq!(source.stats().fallbacks, 1);
    assert!(!source.poisoned);
    let dangling = vec![node(vec![Edge::Epsilon(999)])];
    let mut source = Summaries::new(&dangling);
    assert_eq!(
        source.copy_ports(0, &mut || analysis_charge(&mut work)),
        Err("Dangling parser prediction state".into())
    );
    assert!(source.completed.is_empty());
    assert!(source.poisoned);
    assert_eq!(
        source.copy_ports(0, &mut || analysis_charge(&mut work)),
        Err("Transparent port source is poisoned".into())
    );
    let good = vec![node(vec![Edge::Epsilon(1)]), stop()];
    let mut cancelled = Summaries::new(&good);
    let mut attempts = 0;
    assert_eq!(
        cancelled.copy_ports(0, &mut || {
            attempts += 1;
            if attempts == 9 {
                Err("caller cancellation".into())
            } else {
                Ok(())
            }
        }),
        Err("caller cancellation".into())
    );
    assert!(cancelled.poisoned);
    assert!(cancelled.completed.is_empty());
    let mut fresh = Summaries::new(&good);
    assert!(
        fresh
            .copy_ports(0, &mut || analysis_charge(&mut work))
            .unwrap()
            .is_some()
    );
    assert_eq!(fresh.stats().hits, 0);
}

#[test]
fn port_cold_warm_exact_short_zero_preconsumed_and_caught_denial_keep_one_pool() {
    let states = vec![
        node(vec![Edge::Epsilon(1), Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Epsilon(3)]),
        stop(),
    ];
    let mut source = Summaries::new(&states);
    let mut ample = 10000;
    let ports = source
        .copy_ports(0, &mut || analysis_charge(&mut ample))
        .unwrap();
    let cold = 10000 - ample;
    let first_bytes = source.stats().logical_bytes;
    source
        .copy_ports(0, &mut || analysis_charge(&mut ample))
        .unwrap();
    let warm = 10000 - ample - cold;
    println!(
        "PORT_FEES {}",
        serde_json::json!({"cold":cold,"warm":warm,
        "first_logical_bytes":first_bytes,"total_logical_bytes":source.stats().logical_bytes,
        "source_setup":0,"separate_pool":false})
    );
    assert!(cold > warm);
    assert!(warm > 0);
    let mut exact = cold + warm;
    let mut fresh = Summaries::new(&states);
    assert_eq!(
        fresh
            .copy_ports(0, &mut || analysis_charge(&mut exact))
            .unwrap(),
        ports
    );
    assert_eq!(
        fresh
            .copy_ports(0, &mut || analysis_charge(&mut exact))
            .unwrap(),
        ports
    );
    assert_eq!(exact, 0);
    for grant in [0, cold - 1] {
        let mut short = Summaries::new(&states);
        let mut remaining = grant;
        assert_eq!(
            short.copy_ports(0, &mut || analysis_charge(&mut remaining)),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(remaining, 0);
        assert!(short.completed.is_empty());
        assert!(short.poisoned);
    }
    let mut combined = cold + warm + 7;
    for _ in 0..7 {
        analysis_charge(&mut combined).unwrap();
    }
    let mut fresh = Summaries::new(&states);
    fresh
        .copy_ports(0, &mut || analysis_charge(&mut combined))
        .unwrap();
    fresh
        .copy_ports(0, &mut || analysis_charge(&mut combined))
        .unwrap();
    assert_eq!(combined, 0);
    // A denial during a warm copy poisons even the old complete proof.
    let mut remaining = cold + warm - 1;
    let mut fresh = Summaries::new(&states);
    fresh
        .copy_ports(0, &mut || analysis_charge(&mut remaining))
        .unwrap();
    assert!(
        fresh
            .copy_ports(0, &mut || analysis_charge(&mut remaining))
            .is_err()
    );
    assert!(fresh.completed.is_empty());
}

#[test]
fn port_canonical_full_keys_keep_alternatives_and_entire_ordered_returns() {
    let states = vec![
        node(vec![Edge::Epsilon(1), Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![token("x", 4)]),
        stop(),
    ];
    let mut queue = PortQueue::empty(&states);
    let mut keys = BTreeSet::new();
    let mut remaining = 10000;
    let calls = [
        vec![(90, 100), (91, 101)],
        vec![(91, 101), (90, 100)],
        vec![(90, 102), (91, 101)],
    ];
    for (alternative, calls) in calls.iter().enumerate() {
        <BTreeSet<AnalysisConfiguration> as ClosureKeys>::schedule(
            AnalysisConfiguration {
                state: 0,
                alternative,
                calls: calls.clone(),
            },
            &mut queue,
            &mut keys,
            &mut remaining,
            (true, ScheduleSite::Seed, None),
            None,
        )
        .unwrap();
    }
    for _ in 0..3 {
        let row = queue.pop_front().unwrap();
        assert_eq!(row.state, 0);
        assert!(
            queue
                .share_keys_observed(row, &mut keys, &mut remaining, None, None)
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(keys.len(), 6); // same port/action variants use actual full key dedup
    assert!(keys.iter().all(|row| row.state == 0 || row.state == 3));
    for (alternative, expected) in calls.iter().enumerate() {
        let row = queue.pop_front().unwrap();
        assert_eq!((row.state, row.alternative), (3, alternative));
        assert_eq!(&row.calls, expected);
        assert!(
            queue
                .share_keys_observed(row, &mut keys, &mut remaining, None, None)
                .unwrap()
                .is_some()
        );
    }
    assert_eq!(queue.summaries.stats().builds, 1); // Token roots allocate/retain nothing
}

#[test]
fn port_paid_cold_fanout_join_removes_repeated_topology_work_on_same_canonical_result() {
    let diamonds = 32;
    let mut states = Vec::new();
    for i in 0..diamonds {
        let at = 3 * i;
        states.push(node(vec![Edge::Epsilon(at + 1), Edge::Action(at + 2)]));
        states.push(node(vec![Edge::Epsilon(at + 3)]));
        states.push(node(vec![Edge::Epsilon(at + 3)]));
    }
    states.push(node(vec![token("x", states.len() + 1)]));
    states.push(stop());
    let p = graph(states);
    let before = serde_json::to_value(&p).unwrap();
    // This is an authored component stress grant, NOT an original request's
    // 10k/100k ceiling or a promise that either original will fit.
    let grant = 1_000_000;
    let mut old_left = grant;
    let mut new_left = grant;
    let old = run::<GeometricAdjacentQueue<'_>>(&p, vec![0; 256], &mut old_left, true);
    let new = run::<PortQueue<'_>>(&p, vec![0; 256], &mut new_left, true);
    println!(
        "PORT_PAID_COLD_UTILITY {}",
        serde_json::json!({"old":old.0,"new":new.0,
        "old_work":grant-old_left,"new_work_including_cold":grant-new_left,
        "same_program":true,"original_request":false})
    );
    assert_eq!(old.0, Ok(false));
    assert_eq!(new, old);
    assert!(new_left > old_left);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    assert_eq!(
        RecursiveFrontierOrder::default(),
        RecursiveFrontierOrder::Historical
    );
    let mut literal: GeometricAdjacentQueue<'_> = GeometricAdjacentQueue::empty(&p.states);
    let mut zero = 0;
    assert_eq!(literal.internal_target(999, &mut zero), Ok(999));
    assert_eq!(zero, 0);
}

#[test]
fn port_actual_call_follow_witness_and_depth128_guards_remain_canonical() {
    let p = graph(vec![
        node(vec![Edge::Epsilon(2)]),
        node(vec![Edge::Action(2)]),
        node(vec![Edge::Epsilon(3), Edge::Action(3)]),
        node(vec![Edge::Call {
            target: 4,
            follow: 6,
        }]),
        node(vec![token("r", 5)]),
        node(vec![Edge::Call {
            target: 4,
            follow: 6,
        }]),
        stop(),
    ]);
    let mut a = 100000;
    let mut b = 100000;
    let old = run::<GeometricAdjacentQueue<'_>>(&p, vec![0, 1], &mut a, true);
    let new = run::<PortQueue<'_>>(&p, vec![0, 1], &mut b, true);
    assert_eq!(old.0, Ok(true));
    assert_eq!(new, old);
    assert!(new.1.values().any(|value| *value));
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
        let p = graph(states);
        let mut a = 100000;
        let mut b = 100000;
        let old = run::<GeometricAdjacentQueue<'_>>(&p, vec![0, 1], &mut a, false).0;
        let new = run::<PortQueue<'_>>(&p, vec![0, 1], &mut b, false).0;
        assert_eq!(new, old);
        assert_eq!(
            new,
            if depth == 128 {
                Ok(false)
            } else {
                Err("Parser prediction call depth exceeded".into())
            }
        );
    }
}

// New source-only controls for actual data structure/copy work removal.
#[test]
fn port_dense_scratch_2894_states_pays368_before_allocation_and_reuses_epoch_without_clear() {
    //2894 is the observed C16 SysML state count, not its reconstructed topology.
    let states = vec![State::default(); 2894];
    for grant in [0, 367, 368] {
        let mut source = Summaries::new(&states);
        let mut work = grant;
        let result = source.begin_colors(&mut || analysis_charge(&mut work));
        assert_eq!(work, 0);
        if grant == 368 {
            result.unwrap();
            assert_eq!(source.colors.len(), 2894);
            assert!(source.colors.iter().all(|word| *word == 0));
            assert_eq!(source.epoch, 1);
            assert_eq!(source.stats().logical_bytes, 11_608);
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert!(source.colors.is_empty());
            assert_eq!(source.epoch, 0);
        }
    }
    // The compiled graph's existing 10,000-state ceiling remains payable;
    // this is setup arithmetic, not an original topology or admission claim.
    let ceiling_states = vec![State::default(); 10_000];
    for grant in [1255, 1256] {
        let mut ceiling = Summaries::new(&ceiling_states);
        let mut left = grant;
        let result = ceiling.begin_colors(&mut || analysis_charge(&mut left));
        assert_eq!(left, 0);
        if grant == 1256 {
            result.unwrap();
            assert_eq!(ceiling.colors.len(), 10_000);
            assert_eq!(ceiling.stats().logical_bytes, 40_032);
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert!(ceiling.colors.is_empty());
        }
    }
    let mut source = Summaries::new(&states);
    let mut work = 1000;
    source
        .begin_colors(&mut || analysis_charge(&mut work))
        .unwrap();
    source
        .set_color(19, false, 1, &mut || analysis_charge(&mut work))
        .unwrap();
    source
        .set_color(19, true, 2, &mut || analysis_charge(&mut work))
        .unwrap();
    let before = source.stats().logical_bytes;
    let address = source.colors.as_ptr();
    let previous_word = source.colors[19];
    let left = work;
    source
        .begin_colors(&mut || analysis_charge(&mut work))
        .unwrap();
    assert_eq!(left - work, 2); // read/check + prepaid epoch write, no map/clear
    assert_eq!(source.colors.as_ptr(), address);
    assert_eq!(source.colors[19], previous_word); // proof that no full clear ran
    assert_eq!(
        source
            .color(19, false, &mut || analysis_charge(&mut work))
            .unwrap(),
        0
    );
    assert_eq!(
        source
            .color(19, true, &mut || analysis_charge(&mut work))
            .unwrap(),
        0
    );
    assert_eq!(source.stats().logical_bytes, before);
    println!("DENSE_PORT_SETUP observed_state_count=2894 logical11608 cold368 next_epoch2");
}

#[test]
fn port_trivial_effect_roots_repeatedly_pay_eligibility_without_any_cache_or_scratch_mutation() {
    let roots = vec![
        (stop(), 1),
        (
            State {
                accepting: true,
                edges: vec![Edge::Epsilon(1)],
                ..State::default()
            },
            1,
        ),
        (
            State {
                follow: vec![1],
                edges: vec![Edge::Epsilon(1)],
                ..State::default()
            },
            1,
        ),
        (State::default(), 1),
        (node(vec![token("x", 1)]), 2),
        (
            node(vec![Edge::Call {
                target: 1,
                follow: 1,
            }]),
            2,
        ),
        (
            node(vec![Edge::First {
                path: "actual".into(),
                target: 1,
            }]),
            2,
        ),
        (node(vec![Edge::Full { target: 1, stop: 1 }]), 2),
        (node(vec![Edge::Epsilon(1), token("mixed", 1)]), 3),
    ];
    for (root, one_call) in roots {
        let states = vec![root, stop()];
        let mut source = Summaries::new(&states);
        let mut work = one_call * 3;
        for _ in 0..3 {
            assert!(
                source
                    .borrow_ports_for_configurations(0, 2, &mut || analysis_charge(&mut work))
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(work, 0);
        assert!(source.completed.is_empty() && source.colors.is_empty());
        assert!(!source.admitted && !source.poisoned);
        assert_eq!(source.epoch, 0);
        assert_eq!(source.stats().logical_bytes, 0);
        assert_eq!(
            (
                source.stats().lookups,
                source.stats().builds,
                source.stats().hits,
                source.stats().fallbacks
            ),
            (3, 0, 0, 3)
        );
        for short in 0..one_call {
            let mut denied = Summaries::new(&states);
            let mut work = short;
            assert_eq!(
                denied.borrow_ports_for_configurations(0, 2, &mut || analysis_charge(&mut work)),
                Err("Grammar prediction budget exceeded".into())
            );
            assert_eq!(work, 0);
            assert!(denied.poisoned && denied.completed.is_empty() && denied.colors.is_empty());
        }
    }
}

#[test]
fn port_dense_gray_black_action_variants_cross_root_epochs_cycle_cache_and_overflow_poison() {
    let states = vec![
        node(vec![Edge::Epsilon(1), Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Epsilon(4)]),
        node(vec![token("x", 5)]),
        stop(),
        node(vec![Edge::Epsilon(7)]),
        node(vec![Edge::Action(6)]),
        node(vec![Edge::Epsilon(4)]),
    ];
    let expected = vec![
        Port {
            state: 4,
            action_seen: false,
        },
        Port {
            state: 4,
            action_seen: true,
        },
    ];
    let mut source = Summaries::new(&states);
    let mut work = 100_000;
    assert_eq!(
        source
            .copy_ports(0, &mut || analysis_charge(&mut work))
            .unwrap(),
        Some(expected.clone())
    );
    assert_eq!(
        source
            .color(3, false, &mut || analysis_charge(&mut work))
            .unwrap(),
        2
    );
    assert_eq!(
        source
            .color(3, true, &mut || analysis_charge(&mut work))
            .unwrap(),
        2
    );
    assert_eq!(
        source
            .copy_ports(1, &mut || analysis_charge(&mut work))
            .unwrap(),
        Some(vec![Port {
            state: 4,
            action_seen: false
        }])
    );
    assert_eq!(
        source
            .color(3, true, &mut || analysis_charge(&mut work))
            .unwrap(),
        0
    );
    // Cached old-root proof is independent of current scratch generation.
    assert_eq!(
        source
            .copy_ports(0, &mut || analysis_charge(&mut work))
            .unwrap(),
        Some(expected)
    );
    assert!(
        source
            .copy_ports(6, &mut || analysis_charge(&mut work))
            .unwrap()
            .is_none()
    );
    assert!(source.completed.get(&6).unwrap().is_none()); // real cycle proof remains cached
    assert!(
        source
            .copy_ports(8, &mut || analysis_charge(&mut work))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        source
            .color(6, false, &mut || analysis_charge(&mut work))
            .unwrap(),
        0
    );
    source.epoch = MAX_EPOCH;
    source.completed.remove(&1); // force a distinct paid root build
    assert_eq!(
        source.borrow_ports_for_configurations(1, 0, &mut || analysis_charge(&mut work)),
        Err("Transparent port visitation epoch exhausted".into())
    );
    assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
}

fn port_borrowed_component(
    grant: usize,
    preconsumed: usize,
    borrowed: bool,
) -> (Result<Vec<Port>, String>, usize, SummaryStats) {
    let states = vec![
        node(vec![Edge::Epsilon(1), Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![token("x", 4)]),
        stop(),
    ];
    let mut source = Summaries::new(&states);
    let mut work = grant;
    for _ in 0..preconsumed {
        analysis_charge(&mut work).unwrap();
    }
    let result: Result<Vec<Port>, String> = (|| {
        let rows = if borrowed {
            let rows = source
                .borrow_ports_for_configurations(0, 2, &mut || analysis_charge(&mut work))?
                .ok_or("Missing borrowed ports")?;
            // Test-only DTO materialization is outside the admitted library
            // algorithm. Actual consumers lend descriptors and clone full rows.
            for _ in rows {
                analysis_charge(&mut work)?;
            }
            rows.to_vec()
        } else {
            let rows = source
                .copy_ports(0, &mut || analysis_charge(&mut work))?
                .ok_or("Missing owned ports")?;
            for _ in &rows {
                analysis_charge(&mut work)?;
                source.admit_configuration(2, &mut || analysis_charge(&mut work))?;
                source.admit_configuration(2, &mut || analysis_charge(&mut work))?;
            }
            rows
        };
        Ok(rows)
    })();
    (result, grant - work, source.stats())
}
#[test]
fn port_borrowed_descriptors_remove_real_vec_copies_keep_all_full_row_fees_and_one_original_pool() {
    let (owned, old_used, old_stats) = port_borrowed_component(100_000, 0, false);
    let (borrowed, used, stats) = port_borrowed_component(100_000, 0, true);
    assert_eq!(borrowed, owned);
    assert!(used < old_used);
    assert_eq!(old_stats.port_copies, 2);
    assert_eq!(stats.port_copies, 0);
    assert_eq!(old_stats.logical_bytes - stats.logical_bytes, 96); //32header+2*32descriptors
    assert_eq!(old_used - used, 8); //actual owned vector/descriptor operations removed
    for grant in [0, used - 1, used] {
        let (actual, spent, _) = port_borrowed_component(grant, 0, true);
        assert_eq!(spent, grant);
        if grant == used {
            assert_eq!(actual, borrowed);
        } else {
            assert!(actual.unwrap_err().contains("budget"));
        }
    }
    let (actual, spent, _) = port_borrowed_component(used + 7, 7, true);
    assert_eq!(actual, borrowed);
    assert_eq!(spent, used + 7);
    println!(
        "BORROWED_PORT_COMPONENT {}",
        serde_json::json!({"owned":old_used,"borrowed":used,
        "logical_saved":96,"saving_scope":"same authored two-port source; not an original K01/SysML fit"})
    );
}

#[test]
fn port_borrowed_view_denial_dangling_cancellation_and_fresh_scope_preserve_original_errors() {
    let states = vec![
        node(vec![Edge::Epsilon(1)]),
        node(vec![token("x", 2)]),
        stop(),
    ];
    let mut source = Summaries::new(&states);
    let mut attempts = 0;
    assert_eq!(
        source.borrow_ports_for_configurations(0, 3, &mut || {
            attempts += 1;
            if attempts == 23 {
                Err("actual caller cancellation".into())
            } else {
                Ok(())
            }
        }),
        Err("actual caller cancellation".into())
    );
    assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
    let malformed = vec![node(vec![Edge::Epsilon(usize::MAX)])];
    let mut source = Summaries::new(&malformed);
    let mut work = 10_000;
    assert_eq!(
        source.borrow_ports_for_configurations(0, 0, &mut || analysis_charge(&mut work)),
        Err("Dangling parser prediction state".into())
    );
    assert!(source.poisoned && source.completed.is_empty() && source.colors.is_empty());
    let mut fresh = Summaries::new(&states);
    let mut calls = 0usize;
    let expected = fresh
        .borrow_ports_for_configurations(0, 3, &mut || {
            calls += 1;
            Ok(())
        })
        .unwrap()
        .unwrap()
        .to_vec();
    assert_eq!(
        expected,
        vec![Port {
            state: 1,
            action_seen: false
        }]
    );
    // Cancel at every actual callback in the successful cold route, including
    // partially prepaid configurations and the final borrowed-view lookup.
    for cutoff in 1..=calls {
        let mut denied = Summaries::new(&states);
        let mut attempted = 0usize;
        assert_eq!(
            denied.borrow_ports_for_configurations(0, 3, &mut || {
                attempted += 1;
                if attempted == cutoff {
                    Err("actual caller cancellation".into())
                } else {
                    Ok(())
                }
            }),
            Err("actual caller cancellation".into())
        );
        assert_eq!(attempted, cutoff);
        assert!(denied.poisoned && denied.completed.is_empty() && denied.colors.is_empty());
    }
    let mut recovery = Summaries::new(&states);
    let mut left = 10_000;
    assert_eq!(
        recovery
            .borrow_ports_for_configurations(0, 3, &mut || analysis_charge(&mut left))
            .unwrap(),
        Some(expected.as_slice())
    );
}

// Historical C17 comparisons are test-only source witnesses. Every call below
// includes the actual root preparation, dense initialization, publication,
// descriptor reads, and both full configuration/return-stack admissions.
struct CursorComponent {
    result: Result<Vec<Option<Vec<Port>>>, String>,
    work: usize,
    stats: SummaryStats,
    poisoned: bool,
}
fn cursor_component(
    states: &[State],
    start: usize,
    grant: usize,
    preconsumed: usize,
    repeats: usize,
    before_cursor: bool,
) -> CursorComponent {
    let mut source = Summaries::new(states);
    let mut left = grant;
    let result: Result<Vec<Option<Vec<Port>>>, String> = (|| {
        for _ in 0..preconsumed {
            analysis_charge(&mut left)?;
        }
        let mut observations = Vec::new();
        for _ in 0..repeats {
            let ports = if before_cursor {
                source.borrow_ports_before_cursor(start, 3, &mut || analysis_charge(&mut left))?
            } else {
                source
                    .borrow_ports_for_configurations(start, 3, &mut || analysis_charge(&mut left))?
            };
            if let Some(ports) = ports {
                for _ in ports {
                    analysis_charge(&mut left)?;
                } // real consumer descriptor read
            }
            // Test-only raw observation outside the admitted library algorithm.
            observations.push(ports.map(<[Port]>::to_vec));
        }
        Ok(observations)
    })();
    // Match the existing runtime facade's OUTER error boundary: a consumer
    // descriptor denial after a successful borrow is invisible to the core,
    // but still invalidates the invocation-owned source before returning Err.
    if result.is_err() {
        source.invalidate();
    }
    CursorComponent {
        result,
        work: grant - left,
        stats: source.stats(),
        poisoned: source.poisoned,
    }
}
fn cursor_cold_graph() -> Vec<State> {
    let width = 96usize;
    let join = width + 1;
    let boundary = join + 1;
    let mut states = vec![State::default(); 2894];
    states[0] = node(
        (1..=width)
            .map(|state| {
                if state % 2 == 0 {
                    Edge::Epsilon(state)
                } else {
                    Edge::Action(state)
                }
            })
            .collect(),
    );
    for slot in states.iter_mut().take(width + 1).skip(1) {
        *slot = node(vec![Edge::Epsilon(join)]);
    }
    states[join] = node(vec![Edge::Epsilon(boundary)]);
    states[boundary] = node(vec![token("real-boundary", boundary + 1)]);
    states[boundary + 1] = stop();
    states
}

#[test]
fn cursor_first_cold_branch_join_matches_full_c17_wrapper_with_unchanged_fees_and_less_actual_work()
{
    // Actual observed SysML graph SIZE, authored topology. No claim this is
    // the original graph or that a local component saving makes it fit.
    let states = cursor_cold_graph();
    let old = cursor_component(&states, 0, 100_000, 0, 1, true);
    let new = cursor_component(&states, 0, 100_000, 0, 1, false);
    assert_eq!(new.result, old.result);
    assert_eq!(
        new.result.as_ref().unwrap()[0],
        Some(vec![
            Port {
                state: 98,
                action_seen: false
            },
            Port {
                state: 98,
                action_seen: true
            },
        ])
    );
    assert!(new.work < old.work, "old={} new={}", old.work, new.work);
    assert!(new.stats.logical_bytes < old.stats.logical_bytes);
    assert_eq!(
        (new.stats.builds, new.stats.publications, new.stats.hits),
        (1, 1, 0)
    );
    assert_eq!(new.stats.port_copies, 0);
    assert!(!new.poisoned && !old.poisoned);
    // Same fixed policy; no smaller per-frame fee accompanies the new walk.
    assert_eq!(FRAME_BYTES, 32);
    assert!(std::mem::size_of::<CursorFrame<'static>>() <= FRAME_BYTES);
    println!(
        "CURSOR_COLD_RAW {}",
        serde_json::json!({
            "states":states.len(),"authored_topology":true,"old_work":old.work,"new_work":new.work,
            "old_logical":old.stats.logical_bytes,"new_logical":new.stats.logical_bytes,
            "frame_fee_unchanged":FRAME_BYTES,"same_first_cold_wrapper":true,
            "original_language_fit":null,"cpu_or_rss_claim":false
        })
    );
}

#[test]
fn cursor_source_order_gray_cycles_action_variants_and_opaque_ports_match_historical_graph_proof() {
    for edges in [
        vec![Edge::Epsilon(1), Edge::Epsilon(usize::MAX)],
        vec![Edge::Epsilon(usize::MAX), Edge::Epsilon(1)],
    ] {
        let cycle_first = matches!(edges.first(), Some(Edge::Epsilon(1)));
        let states = vec![node(edges), node(vec![Edge::Action(0)])];
        let old = cursor_component(&states, 0, 100_000, 0, 1, true);
        let new = cursor_component(&states, 0, 100_000, 0, 1, false);
        assert_eq!(new.result, old.result);
        if cycle_first {
            assert_eq!(new.result, Ok(vec![None]));
            assert!(!new.poisoned);
        } else {
            assert_eq!(new.result, Err("Dangling parser prediction state".into()));
            assert!(new.poisoned);
        }
    }
    let boundaries = vec![
        stop(),
        State {
            accepting: true,
            ..State::default()
        },
        State {
            follow: vec![usize::MAX],
            ..State::default()
        },
        State::default(),
        node(vec![token("x", usize::MAX)]),
        node(vec![Edge::Call {
            target: usize::MAX,
            follow: usize::MAX,
        }]),
        node(vec![Edge::First {
            path: "unbound-effect-is-not-read-by-kernel".into(),
            target: usize::MAX,
        }]),
        node(vec![Edge::Full {
            target: usize::MAX,
            stop: usize::MAX,
        }]),
        node(vec![Edge::Epsilon(usize::MAX), token("mixed", usize::MAX)]),
    ];
    for boundary in boundaries {
        let states = vec![
            node(vec![Edge::Epsilon(1), Edge::Action(2)]),
            node(vec![Edge::Epsilon(3)]),
            node(vec![Edge::Epsilon(3)]),
            boundary,
        ];
        let old = cursor_component(&states, 0, 100_000, 0, 1, true);
        let new = cursor_component(&states, 0, 100_000, 0, 1, false);
        assert_eq!(new.result, old.result);
        assert_eq!(
            new.result,
            Ok(vec![Some(vec![
                Port {
                    state: 3,
                    action_seen: false
                },
                Port {
                    state: 3,
                    action_seen: true
                }
            ])])
        );
        // Those malformed effect internals remain for the actual canonical
        // consumer. The graph kernel must never interpret or certify them.
        assert!(!new.poisoned);
    }
}

#[test]
fn cursor_cold_warm_exact_one_short_zero_and_preconsumed_pool_keep_real_config_admissions() {
    let states = vec![
        node(vec![Edge::Epsilon(1), Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![token("x", 4)]),
        stop(),
    ];
    let cold = cursor_component(&states, 0, 100_000, 0, 1, false);
    let pair = cursor_component(&states, 0, 100_000, 0, 2, false);
    assert!(pair.work > cold.work);
    assert_eq!(
        (pair.stats.builds, pair.stats.publications, pair.stats.hits),
        (1, 1, 1)
    );
    assert!(pair.stats.logical_bytes > cold.stats.logical_bytes); // full warm rows paid, no refund
    let exact = cursor_component(&states, 0, pair.work, 0, 2, false);
    assert_eq!(exact.result, pair.result);
    assert_eq!(exact.work, pair.work);
    assert!(!exact.poisoned);
    for grant in [0, pair.work - 1] {
        let denied = cursor_component(&states, 0, grant, 0, 2, false);
        assert_eq!(
            denied.result,
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(denied.work, grant);
        assert!(denied.poisoned);
    }
    let pre = cursor_component(&states, 0, pair.work + 7, 7, 2, false);
    assert_eq!(pre.result, pair.result);
    assert_eq!(pre.work, pair.work + 7);
    println!(
        "CURSOR_LEDGER_RAW {}",
        serde_json::json!({"cold":cold.work,"warm":pair.work-cold.work,
        "pair":pair.work,"preconsumed":7,"same_pool":true,"new_allowance":false})
    );
}

#[test]
fn cursor_every_cold_callback_cancellation_clears_partial_proof_and_fresh_scope_recovers() {
    let states = vec![
        node(vec![Edge::Epsilon(1), Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![Edge::Epsilon(3)]),
        node(vec![token("x", 4)]),
        stop(),
    ];
    let mut complete = Summaries::new(&states);
    let mut calls = 0usize;
    let expected = complete
        .borrow_ports_for_configurations(0, 3, &mut || {
            calls += 1;
            Ok(())
        })
        .unwrap()
        .unwrap()
        .to_vec();
    for cutoff in 1..=calls {
        let mut denied = Summaries::new(&states);
        let mut attempts = 0usize;
        assert_eq!(
            denied.borrow_ports_for_configurations(0, 3, &mut || {
                attempts += 1;
                if attempts == cutoff {
                    Err("original caller cancelled".into())
                } else {
                    Ok(())
                }
            }),
            Err("original caller cancelled".into())
        );
        assert_eq!(attempts, cutoff);
        assert!(denied.poisoned && denied.completed.is_empty() && denied.colors.is_empty());
        let mut later_calls = 0usize;
        assert_eq!(
            denied.borrow_ports_for_configurations(0, 3, &mut || {
                later_calls += 1;
                Ok(())
            }),
            Err("Transparent port source is poisoned".into())
        );
        assert_eq!(later_calls, 0);
    }
    let mut recovered = Summaries::new(&states);
    let mut work = 100_000;
    assert_eq!(
        recovered
            .borrow_ports_for_configurations(0, 3, &mut || analysis_charge(&mut work))
            .unwrap(),
        Some(expected.as_slice())
    );
}

#[test]
fn cursor_unique_node_limit_stays4096_and_unneeded_eager_pending_width_is_not_allocated() {
    let mut chain: Vec<State> = (0..=MAX_KEYS)
        .map(|at| node(vec![Edge::Epsilon(at + 1)]))
        .collect();
    chain.push(stop());
    for before in [true, false] {
        let actual = cursor_component(&chain, 0, 100_000, 0, 1, before);
        assert_eq!(
            actual.result,
            Err("Transparent port node bound exceeded".into())
        );
        assert!(actual.poisoned);
    }
    assert_eq!(MAX_KEYS, 4096);
    // This former width overflow was storage of unvisited, duplicate child
    // frames. A cursor keeps only the active path; its existing8192 pending
    // cap is unchanged. No call/depth/semantic boundary has been bypassed.
    let width = MAX_KEYS * 2 + 1;
    let wide = vec![node((0..width).map(|_| Edge::Epsilon(1)).collect()), stop()];
    let old = cursor_component(&wide, 0, 100_000, 0, 1, true);
    let new = cursor_component(&wide, 0, 100_000, 0, 1, false);
    assert_eq!(
        old.result,
        Err("Transparent port pending bound exceeded".into())
    );
    assert_eq!(
        new.result,
        Ok(vec![Some(vec![Port {
            state: 1,
            action_seen: false
        }])])
    );
    assert!(!new.poisoned);
    println!(
        "CURSOR_WIDTH_RAW {}",
        serde_json::json!({"source_edges":width,"pending_cap":MAX_KEYS*2,
        "node_cap":MAX_KEYS,"old_work":old.work,"new_work":new.work,
        "old_error_preserved":old.result.err(),"new_actual_port_count":1,
        "meaning":"different actual DFS storage; not an original input or admission award"})
    );
}
