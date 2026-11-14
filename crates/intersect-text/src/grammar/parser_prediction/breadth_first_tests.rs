//! One classifier, two explicit token-frontier schedules. No original request.
use super::frontier::{AdoptFrontiers, FrontierPolicy, PendingFrontier};
use super::*;
use std::cell::Cell;
thread_local! { static PEAK: Cell<usize> = const { Cell::new(0) }; }
struct Width;
impl FrontierPolicy for Width {
    type Pending = PendingFrontier;
    fn initial(
        rows: BTreeSet<AnalysisConfiguration>,
        work: &mut usize,
    ) -> Result<VecDeque<Self::Pending>, String> {
        let pending = AdoptFrontiers::initial(rows, work)?;
        PEAK.with(|peak| peak.set(pending.len()));
        Ok(pending)
    }
    fn take(row: Self::Pending) -> (BTreeSet<AnalysisConfiguration>, bool) {
        AdoptFrontiers::take(row)
    }
    fn extend(
        pending: &mut VecDeque<Self::Pending>,
        tokens: AnalysisTokens,
        work: &mut usize,
    ) -> Result<(), String> {
        AdoptFrontiers::extend(pending, tokens, work)?;
        PEAK.with(|peak| peak.set(peak.get().max(pending.len())));
        Ok(())
    }
    fn uniform_state(
        paid: bool,
        rows: &BTreeSet<AnalysisConfiguration>,
        work: &mut usize,
    ) -> Result<Option<usize>, String> {
        AdoptFrontiers::uniform_state(paid, rows, work)
    }
}
fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn token(s: &str, target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(s.into()), target)
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
type Witnesses = BTreeMap<(usize, usize), bool>;
type Run = (Result<bool, String>, usize, Witnesses);
fn run(p: &ParserPrediction, breadth: bool, work: &mut usize, shared: bool) -> Run {
    let mut witnesses = BTreeMap::new();
    let result = p.multiple_recursive_alternatives_with_frontiers::<adjacent::GeometricAdjacentQueue<'_>, Width, true>(
        &Decision { entries: vec![0,1], exit: None }, work, shared, &mut witnesses,
        !breadth, (true, None, None, None));
    (result, PEAK.with(Cell::get), witnesses)
}
fn fork(long: usize, dangling_short: bool) -> ParserPrediction {
    let mut states = vec![
        node(vec![token("a", 2), token("z", 4)]),
        node(vec![token("a", 3), token("z", 5)]),
        node(vec![Edge::Call {
            target: 6,
            follow: 7,
        }]),
        node(vec![Edge::Call {
            target: 6,
            follow: 7,
        }]),
        State::default(),
        State::default(),
        node(vec![token("r", 8)]),
        State {
            rule_stop: true,
            ..State::default()
        },
        node(vec![Edge::Call {
            target: 6,
            follow: 7,
        }]),
    ];
    if dangling_short {
        states[2] = node(vec![Edge::Epsilon(usize::MAX)]);
        states[3] = node(vec![Edge::Epsilon(usize::MAX)]);
    }
    let mut ends = [4, 5];
    for _ in 0..long {
        let next = [states.len(), states.len() + 1];
        states.push(State::default());
        states.push(State::default());
        for i in 0..2 {
            states[ends[i]] = node(vec![token("z", next[i])]);
        }
        ends = next;
    }
    for end in ends {
        states[end] = State {
            rule_stop: true,
            ..State::default()
        };
    }
    graph(states)
}
#[test]
fn breadth_first_uses_actual_shared_recursive_witness_without_a_false_certificate() {
    let p = fork(96, false);
    let before = serde_json::to_value(&p).unwrap();
    let mut bfs = 10_000;
    let mut dfs = 10_000;
    let b = run(&p, true, &mut bfs, true);
    let d = run(&p, false, &mut dfs, true);
    println!(
        "FRONTIER_WITNESS {:?} {:?}",
        (b.0.clone(), 10_000 - bfs, b.1, &b.2),
        (d.0.clone(), 10_000 - dfs, d.1, &d.2)
    );
    assert_eq!(b.0, Ok(true));
    assert_eq!(d.0, Ok(true));
    assert!(b.2.values().any(|v| *v));
    assert!(d.2.values().any(|v| *v));
    assert!(bfs > dfs);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    // A limit is still a limit; no new allowance or assumed positive result.
    let used = 10_000 - bfs;
    let mut exact = used;
    let mut short = used - 1;
    let mut zero = 0;
    assert_eq!(run(&p, true, &mut exact, true).0, Ok(true));
    assert_eq!(exact, 0);
    assert_eq!(
        run(&p, true, &mut short, true).0,
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(short, 0);
    assert_eq!(
        run(&p, true, &mut zero, true).0,
        Err("Grammar prediction budget exceeded".into())
    );
}
fn tree(depth: usize) -> ParserPrediction {
    let mut states = vec![State::default(), State::default()];
    let mut level = vec![0, 1];
    for _ in 0..depth {
        let mut next = Vec::new();
        for at in level {
            let a = states.len();
            states.push(State::default());
            let z = states.len();
            states.push(State::default());
            states[at] = node(vec![token("a", a), token("z", z)]);
            next.extend([a, z]);
        }
        level = next;
    }
    for at in level {
        states[at] = State {
            rule_stop: true,
            ..State::default()
        };
    }
    graph(states)
}
#[test]
fn breadth_first_negative_closures_keep_whole_keys_and_charge_real_queue_width() {
    let p = tree(5);
    let before = serde_json::to_value(&p).unwrap();
    let mut bfs = 10_000;
    let mut dfs = 10_000;
    let b = run(&p, true, &mut bfs, true);
    let d = run(&p, false, &mut dfs, true);
    println!(
        "FRONTIER_WIDTH {:?} {:?}",
        (b.0.clone(), b.1, 10_000 - bfs),
        (d.0.clone(), d.1, 10_000 - dfs)
    );
    assert_eq!(b.0, Ok(false));
    assert_eq!(d.0, Ok(false));
    assert!(b.2.is_empty() && d.2.is_empty());
    assert_eq!(b.1, 32);
    assert!(d.1 <= 6);
    assert!(b.1 <= 10_000 - bfs);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    let mut denied = 5;
    assert_eq!(
        run(&p, true, &mut denied, true).0,
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(denied, 0);
}
#[test]
fn breadth_first_exposes_real_finite_order_error_differences_without_hiding_them() {
    let p = fork(256, true);
    let mut bfs = 80;
    let mut dfs = 80;
    let b = run(&p, true, &mut bfs, true);
    let d = run(&p, false, &mut dfs, true);
    println!(
        "FRONTIER_ERROR {:?} {:?}",
        (b.0.clone(), bfs),
        (d.0.clone(), dfs)
    );
    assert_eq!(b.0, Err("Dangling parser prediction state".into()));
    assert_eq!(d.0, Err("Grammar prediction budget exceeded".into()));
    assert!(bfs > 0);
    assert_eq!(dfs, 0);
}
fn calls(depth: usize) -> ParserPrediction {
    let mut states = vec![State::default(), State::default()];
    let mut last = [0, 1];
    for _ in 0..depth {
        let next = [states.len(), states.len() + 1];
        states.push(State::default());
        states.push(State::default());
        for i in 0..2 {
            states[last[i]] = node(vec![Edge::Call {
                target: next[i],
                follow: usize::MAX,
            }]);
        }
        last = next;
    }
    for i in 0..2 {
        states[last[i]] = node(vec![token(
            if i == 0 { "left" } else { "right" },
            usize::MAX,
        )]);
    }
    graph(states)
}
#[test]
fn breadth_first_preserves_real_call_depth_and_one_alternative_is_not_multiple_recursion() {
    for breadth in [false, true] {
        let mut work = 10_000;
        assert_eq!(run(&calls(128), breadth, &mut work, false).0, Ok(false));
        let mut work = 10_000;
        assert_eq!(
            run(&calls(129), breadth, &mut work, false).0,
            Err("Parser prediction call depth exceeded".into())
        );
        assert!(work > 0);
        let mut p = fork(0, false);
        p.states[1] = node(vec![token("other", 7)]);
        let mut work = 10_000;
        assert_eq!(run(&p, breadth, &mut work, true).0, Ok(false));
    }
}
#[test]
fn breadth_first_keeps_real_distinct_returns_after_a_shared_nonrecursive_callee() {
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
        State {
            rule_stop: true,
            ..State::default()
        },
        node(vec![token("x", 6)]),
        node(vec![token("y", 6)]),
        State {
            rule_stop: true,
            ..State::default()
        },
    ]);
    for breadth in [false, true] {
        let mut work = 10_000;
        let result = run(&p, breadth, &mut work, true);
        assert_eq!(result.0, Ok(false));
        assert_eq!(result.2.get(&(2, 128)), Some(&false));
        assert!(work > 0);
    }
}
#[test]
fn breadth_first_historical_facade_and_explicit_stack_policy_do_not_enter_serialization() {
    assert_eq!(std::mem::size_of::<RecursiveFrontierOrder>(), 1);
    assert_eq!(
        RecursiveFrontierOrder::default(),
        RecursiveFrontierOrder::Historical
    );
    let p = tree(2);
    let before = serde_json::to_value(&p).unwrap();
    let mut work = 10_000;
    let _ = run(&p, true, &mut work, true);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    assert!(!before.to_string().contains("BreadthFirst"));
}
