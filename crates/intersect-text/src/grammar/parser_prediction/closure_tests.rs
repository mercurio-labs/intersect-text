//! Exact work and key controls for the V73 FIFO scheduling seam.
use super::*;

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
fn state(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn token(name: &str, target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(name.into()), target)
}
fn analyze(
    p: &ParserPrediction,
    entries: &[usize],
    work: usize,
    unique: bool,
    shared: bool,
) -> (Result<bool, String>, usize) {
    let before = serde_json::to_value(p).unwrap();
    let mut remaining = work;
    let result = p.multiple_recursive_alternatives(
        &Decision {
            entries: entries.to_vec(),
            exit: None,
        },
        &mut remaining,
        shared,
        &mut BTreeMap::new(),
        true,
        (unique, None),
    );
    assert_eq!(serde_json::to_value(p).unwrap(), before);
    (result, remaining)
}
fn diamond() -> ParserPrediction {
    program(vec![
        state(vec![Edge::Epsilon(1), Edge::Epsilon(2)]),
        state(vec![Edge::Action(3)]),
        state(vec![Edge::Action(3)]),
        state(vec![token("a", 4)]),
        State::default(),
        State::default(),
        state(vec![token("b", 7)]),
        State::default(),
    ])
}
#[test]
fn diamond_reserves_fourteen_instead_of_fifteen_and_keeps_program() {
    let p = diamond();
    assert_eq!(analyze(&p, &[0, 6], 15, false, false), (Ok(false), 0));
    assert_eq!(analyze(&p, &[0, 6], 15, true, false), (Ok(false), 1));
    assert_eq!(analyze(&p, &[0, 6], 14, true, false), (Ok(false), 0));
    for (work, unique) in [(14, false), (13, true), (0, false), (0, true)] {
        assert_eq!(
            analyze(&p, &[0, 6], work, unique, false),
            (Err("Grammar prediction budget exceeded".into()), 0)
        );
    }
}
#[test]
fn ordinary_disjoint_rows_have_identical_cost_without_duplicates() {
    let p = program(vec![
        state(vec![token("a", 2)]),
        state(vec![token("b", 3)]),
        State::default(),
        State::default(),
    ]);
    for unique in [false, true] {
        assert_eq!(analyze(&p, &[0, 1], 7, unique, false), (Ok(false), 0));
        assert_eq!(
            analyze(&p, &[0, 1], 6, unique, false),
            (Err("Grammar prediction budget exceeded".into()), 0)
        );
    }
}
#[test]
fn reservation_is_before_retention_and_duplicates_are_free_even_at_zero() {
    let row = AnalysisConfiguration {
        state: 3,
        alternative: 2,
        calls: vec![(9, 10)],
    };
    let mut queue = VecDeque::new();
    let mut scheduled = BTreeSet::new();
    let mut remaining = 0;
    assert_eq!(
        schedule_analysis(
            row.clone(),
            &mut queue,
            &mut scheduled,
            &mut remaining,
            (true, ScheduleSite::Successor, None)
        ),
        Err("Grammar prediction budget exceeded".into())
    );
    assert!(queue.is_empty());
    assert!(scheduled.is_empty());
    remaining = 1;
    schedule_analysis(
        row.clone(),
        &mut queue,
        &mut scheduled,
        &mut remaining,
        (true, ScheduleSite::Successor, None),
    )
    .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(queue, VecDeque::from([row.clone()]));
    schedule_analysis(
        row.clone(),
        &mut queue,
        &mut scheduled,
        &mut remaining,
        (true, ScheduleSite::Successor, None),
    )
    .unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(scheduled, BTreeSet::from([row]));
}
#[test]
fn exact_alternative_and_every_call_return_pair_are_part_of_key() {
    let mut queue = VecDeque::new();
    let mut scheduled = BTreeSet::new();
    let mut remaining = 4;
    let rows = [
        AnalysisConfiguration {
            state: 3,
            alternative: 0,
            calls: vec![(1, 2)],
        },
        AnalysisConfiguration {
            state: 3,
            alternative: 1,
            calls: vec![(1, 2)],
        },
        AnalysisConfiguration {
            state: 3,
            alternative: 0,
            calls: vec![(1, 4)],
        },
        AnalysisConfiguration {
            state: 3,
            alternative: 0,
            calls: vec![(5, 2)],
        },
    ];
    for row in &rows {
        schedule_analysis(
            row.clone(),
            &mut queue,
            &mut scheduled,
            &mut remaining,
            (true, ScheduleSite::Successor, None),
        )
        .unwrap();
    }
    assert_eq!(queue, VecDeque::from(rows));
    assert_eq!(remaining, 0);
    assert_eq!(scheduled.len(), 4);
}
#[test]
fn live_call_returns_keep_distinct_stacks_and_existing_edge_fees() {
    let p = program(vec![
        state(vec![
            Edge::Call {
                target: 2,
                follow: 3,
            },
            Edge::Call {
                target: 2,
                follow: 4,
            },
        ]),
        state(vec![token("b", 5)]),
        State {
            rule_stop: true,
            ..State::default()
        },
        state(vec![token("a", 5)]),
        state(vec![token("a", 5)]),
        State::default(),
    ]);
    for unique in [false, true] {
        assert_eq!(analyze(&p, &[0, 1], 14, unique, false), (Ok(false), 0));
    }
}
#[test]
fn callerless_follow_duplicates_preserve_charged_follow_edges() {
    let p = program(vec![
        State {
            rule_stop: true,
            follow: vec![2, 2],
            ..State::default()
        },
        state(vec![token("b", 3)]),
        state(vec![token("a", 3)]),
        State::default(),
    ]);
    assert_eq!(analyze(&p, &[0, 1], 11, false, false), (Ok(false), 0));
    assert_eq!(analyze(&p, &[0, 1], 10, true, false), (Ok(false), 0));
    assert_eq!(
        analyze(&p, &[0, 1], 9, true, false),
        (Err("Grammar prediction budget exceeded".into()), 0)
    );
}
#[test]
fn seed_and_newly_reached_rows_keep_first_fifo_occurrences() {
    let mut queue = VecDeque::new();
    let mut scheduled = BTreeSet::new();
    let mut remaining = 4;
    let a = AnalysisConfiguration {
        state: 0,
        alternative: 0,
        calls: Vec::new(),
    };
    let b = AnalysisConfiguration {
        state: 1,
        ..a.clone()
    };
    let c = AnalysisConfiguration {
        state: 2,
        ..a.clone()
    };
    for row in [a.clone(), b.clone(), a.clone(), c.clone(), b.clone()] {
        schedule_analysis(
            row,
            &mut queue,
            &mut scheduled,
            &mut remaining,
            (true, ScheduleSite::Successor, None),
        )
        .unwrap();
    }
    assert_eq!(queue, VecDeque::from([a, b, c]));
    assert_eq!(remaining, 1);
}
#[test]
fn early_recursion_true_keeps_prepaid_unprocessed_rows_without_refund() {
    let p = program(vec![
        state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        State::default(),
    ]);
    assert_eq!(analyze(&p, &[0, 1], 7, false, true), (Ok(true), 0));
    assert_eq!(analyze(&p, &[0, 1], 8, true, true), (Ok(true), 0));
    assert_eq!(
        analyze(&p, &[0, 1], 7, true, true),
        (Err("Grammar prediction budget exceeded".into()), 0)
    );
}
#[test]
fn reservation_denial_can_precede_dangling_state_and_retry_is_deterministic() {
    let p = program(Vec::new());
    assert_eq!(
        analyze(&p, &[999, 999], 2, false, false),
        (Err("Dangling parser prediction state".into()), 0)
    );
    assert_eq!(
        analyze(&p, &[999, 999], 2, true, false),
        (Err("Grammar prediction budget exceeded".into()), 0)
    );
    assert_eq!(
        analyze(&p, &[999, 999], 3, true, false),
        (Err("Dangling parser prediction state".into()), 0)
    );
    for _ in 0..2 {
        assert_eq!(
            analyze(&p, &[999, 999], 2, true, false),
            (Err("Grammar prediction budget exceeded".into()), 0)
        );
    }
}
#[test]
fn shared_witness_keys_and_productive_recursive_truncation_remain_exact() {
    let direct = program(vec![
        state(vec![Edge::Call {
            target: 0,
            follow: 0,
        }]),
        state(vec![Edge::Call {
            target: 0,
            follow: 0,
        }]),
    ]);
    for unique in [false, true] {
        assert_eq!(analyze(&direct, &[0, 1], 100, unique, true).0, Ok(true));
    }
    let productive = program(vec![
        state(vec![token("a", 2)]),
        state(vec![token("a", 3)]),
        state(vec![Edge::Call {
            target: 0,
            follow: 4,
        }]),
        state(vec![Edge::Call {
            target: 1,
            follow: 4,
        }]),
        State::default(),
    ]);
    for unique in [false, true] {
        assert_eq!(
            analyze(&productive, &[0, 1], 100, unique, false).0,
            Ok(true)
        );
    }
}
#[test]
fn live_call_depth_ceiling_is_still_one_hundred_twenty_eight() {
    let states = (0..131)
        .map(|index| {
            state(vec![Edge::Call {
                target: index + 1,
                follow: 0,
            }])
        })
        .collect();
    let p = program(states);
    for unique in [false, true] {
        assert_eq!(
            analyze(&p, &[0, 0], 10000, unique, false).0,
            Err("Parser prediction call depth exceeded".into())
        );
    }
}

#[test]
fn productive_mutual_recursion_and_unresolved_single_recursive_branch_stay_explicit() {
    let mutual = program(vec![
        state(vec![Edge::Call {
            target: 2,
            follow: 4,
        }]),
        state(vec![Edge::Call {
            target: 3,
            follow: 4,
        }]),
        state(vec![token("a", 5)]),
        state(vec![token("a", 6)]),
        State::default(),
        state(vec![Edge::Call {
            target: 0,
            follow: 4,
        }]),
        state(vec![Edge::Call {
            target: 1,
            follow: 4,
        }]),
    ]);
    let single = program(vec![
        state(vec![token("a", 2)]),
        state(vec![token("a", 1)]),
        state(vec![Edge::Call {
            target: 0,
            follow: 3,
        }]),
        State::default(),
    ]);
    for unique in [false, true] {
        assert_eq!(analyze(&mutual, &[0, 1], 1000, unique, false).0, Ok(true));
        assert_eq!(
            analyze(&single, &[0, 1], 1000, unique, false).0,
            Err("Recursive prediction analysis remains unresolved".into())
        );
    }
}
