//! Authored controls only; original SysML utility remains unmeasured.
use super::super::{Decision, ParserPrediction, PredictionSymbol, ScheduleSite, frontier};
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
fn chain_summary_cold9_warm1_exact_short_and_original_preconsumed_pool() {
    let states = vec![
        node(vec![Edge::Epsilon(1)]),
        node(vec![Edge::Action(2)]),
        stop(),
    ];
    for grant in 0..9 {
        let mut cache = Summaries::new(&states);
        let mut work = grant;
        assert_eq!(
            cache.target(0, &mut work),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(work, 0);
        assert!(cache.completed.is_empty());
        let mut retry = 9;
        assert_eq!(cache.target(0, &mut retry), Ok(2));
        // A paid header can survive a denied invocation; no fee refund/reset.
        assert_eq!(retry, usize::from(cache.admitted && grant > 0));
    }
    let mut cache = Summaries::new(&states);
    let mut exact = 9;
    assert_eq!(cache.target(0, &mut exact), Ok(2));
    assert_eq!(exact, 0);
    let mut warm = 1;
    assert_eq!(cache.target(0, &mut warm), Ok(2));
    assert_eq!(warm, 0);
    let mut cumulative = 16;
    for _ in 0..7 {
        analysis_charge(&mut cumulative).unwrap();
    }
    let mut fresh = Summaries::new(&states);
    assert_eq!(fresh.target(0, &mut cumulative), Ok(2));
    assert_eq!(cumulative, 0);
}
#[test]
fn chain_summary_stops_at_actual_guards_calls_returns_tokens_fanouts_and_accepting() {
    let boundaries = vec![
        stop(),
        State {
            accepting: true,
            edges: vec![Edge::Epsilon(0)],
            ..State::default()
        },
        node(vec![token("x", 0)]),
        node(vec![Edge::Call {
            target: 0,
            follow: 1,
        }]),
        node(vec![Edge::First {
            path: "actual/guard".into(),
            target: 0,
        }]),
        node(vec![Edge::Full { target: 0, stop: 1 }]),
        node(vec![Edge::Epsilon(0), Edge::Action(1)]),
    ];
    for boundary in boundaries {
        let states = vec![node(vec![Edge::Epsilon(1)]), boundary];
        let mut cache = Summaries::new(&states);
        let mut work = 100;
        assert_eq!(cache.target(0, &mut work), Ok(1));
    }
    let states = vec![node(vec![Edge::Epsilon(1)]), node(vec![Edge::Action(0)])];
    let mut cache = Summaries::new(&states);
    let mut work = 100;
    assert_eq!(cache.target(0, &mut work), Ok(0));
    assert_eq!(cache.completed.get(&0), Some(&None));
    let states = vec![node(vec![Edge::Epsilon(999)])];
    let mut cache = Summaries::new(&states);
    let mut work = 100;
    assert_eq!(
        cache.target(0, &mut work),
        Err("Dangling parser prediction state".into())
    );
    assert!(cache.completed.is_empty());
}
#[test]
fn chain_default_hook_zero_work_and_old_program_serialization_remain_exact() {
    let states = vec![stop()];
    let mut old: GeometricAdjacentQueue<'_> = GeometricAdjacentQueue::empty(&states);
    let mut work = 0;
    assert_eq!(old.internal_target(999, &mut work), Ok(999));
    assert_eq!(work, 0);
    let p = graph(vec![
        node(vec![Edge::Epsilon(2)]),
        node(vec![Edge::Action(2)]),
        stop(),
    ]);
    let before = serde_json::to_value(&p).unwrap();
    let mut a = 10000;
    let mut b = 10000;
    assert_eq!(
        run::<GeometricAdjacentQueue<'_>>(&p, vec![0, 1], &mut a, true).0,
        run::<ChainQueue<'_>>(&p, vec![0, 1], &mut b, true).0
    );
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    assert_eq!(
        super::super::RecursiveFrontierOrder::default(),
        super::super::RecursiveFrontierOrder::Historical
    );
}
#[test]
fn chain_canonical_many_contexts_omits_intermediate_full_keys_without_closing_other_branches() {
    let mut states = vec![node(vec![Edge::Epsilon(1)])];
    for n in 1..17 {
        states.push(node(vec![Edge::Action(n + 1)]));
    }
    states.push(node(vec![token("x", 18)]));
    states.push(stop());
    let p = graph(states);
    let before = serde_json::to_value(&p).unwrap();
    let mut a = 10000;
    let mut b = 10000;
    let old = run::<GeometricAdjacentQueue<'_>>(&p, vec![0; 32], &mut a, true);
    let new = run::<ChainQueue<'_>>(&p, vec![0; 32], &mut b, true);
    println!(
        "TRANSPARENT_CHAIN_UTILITY {}",
        serde_json::json!({"old":old.0,"new":new.0,
        "old_work":10000-a,"new_work":10000-b,"source_unchanged":true})
    );
    assert_eq!(old.0, Ok(false));
    assert_eq!(new.0, old.0);
    assert!(b > a);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    // Direct queue proof: only original and final full keys are retained.
    let mut queue = ChainQueue::empty(&p.states);
    let mut keys = BTreeSet::new();
    let mut work = 1000;
    for alternative in 0..3 {
        <BTreeSet<AnalysisConfiguration> as ClosureKeys>::schedule(
            AnalysisConfiguration {
                state: 0,
                alternative,
                calls: vec![(90 + alternative, 100 + alternative)],
            },
            &mut queue,
            &mut keys,
            &mut work,
            (true, ScheduleSite::Seed, None),
            None,
        )
        .unwrap();
    }
    let row = queue.pop_front().unwrap();
    assert!(
        queue
            .share_keys_observed(row, &mut keys, &mut work, None, None)
            .unwrap()
            .is_none()
    );
    assert_eq!(keys.len(), 6);
    assert!(keys.iter().all(|r| r.state == 0 || r.state == 17));
    for alternative in 0..3 {
        let row = queue.pop_front().unwrap();
        assert_eq!(row.state, 17);
        assert_eq!(row.alternative, alternative);
        assert_eq!(row.calls, vec![(90 + alternative, 100 + alternative)]);
    }
}
#[test]
fn chain_actual_calls_follow_and_shared_witness_remain_in_the_canonical_worker() {
    let p = graph(vec![
        node(vec![Edge::Epsilon(2)]),
        node(vec![Edge::Action(2)]),
        node(vec![Edge::Epsilon(3)]),
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
    let mut a = 10000;
    let mut b = 10000;
    let old = run::<GeometricAdjacentQueue<'_>>(&p, vec![0, 1], &mut a, true);
    let new = run::<ChainQueue<'_>>(&p, vec![0, 1], &mut b, true);
    assert_eq!(old.0, Ok(true));
    assert_eq!(new, old);
    assert!(new.1.values().any(|v| *v));
    // Explicit return contexts must not use a recursive callerless FOLLOW.
    let mut ending = stop();
    ending.follow = vec![7];
    let p = graph(vec![
        node(vec![Edge::Call {
            target: 2,
            follow: 4,
        }]),
        node(vec![Edge::Call {
            target: 2,
            follow: 5,
        }]),
        node(vec![token("a", 3)]),
        ending,
        node(vec![token("left", 6)]),
        node(vec![token("right", 6)]),
        stop(),
        node(vec![Edge::Call {
            target: 7,
            follow: 6,
        }]),
    ]);
    let mut a = 10000;
    let mut b = 10000;
    assert_eq!(
        run::<GeometricAdjacentQueue<'_>>(&p, vec![0, 1], &mut a, true).0,
        Ok(false)
    );
    assert_eq!(
        run::<ChainQueue<'_>>(&p, vec![0, 1], &mut b, true).0,
        Ok(false)
    );
}
#[test]
fn chain_whole_worker_zero_short_and_real_call_depth128_129_remain_live() {
    let p = graph(vec![
        node(vec![Edge::Epsilon(2)]),
        node(vec![Edge::Epsilon(2)]),
        node(vec![Edge::Action(3)]),
        node(vec![token("x", 4)]),
        stop(),
    ]);
    let mut ample = 10000;
    assert_eq!(
        run::<ChainQueue<'_>>(&p, vec![0, 1], &mut ample, true).0,
        Ok(false)
    );
    let used = 10000 - ample;
    let mut exact = used;
    assert_eq!(
        run::<ChainQueue<'_>>(&p, vec![0, 1], &mut exact, true).0,
        Ok(false)
    );
    assert_eq!(exact, 0);
    for grant in [0, used - 1] {
        let mut work = grant;
        assert_eq!(
            run::<ChainQueue<'_>>(&p, vec![0, 1], &mut work, true).0,
            Err("Grammar prediction budget exceeded".into())
        );
    }
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
        let new = run::<ChainQueue<'_>>(&p, vec![0, 1], &mut b, false).0;
        assert_eq!(old, new);
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
