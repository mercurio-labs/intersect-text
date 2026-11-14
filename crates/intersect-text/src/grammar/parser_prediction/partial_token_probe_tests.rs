//! Authored component controls, not evidence about the original SysML graph.
use super::partial_token_probe::{FirstSharedToken, NoProbe, TokenProbe};
use super::*;

fn node(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn token(s: &str, target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(s.into()), target)
}
fn stop() -> State {
    State {
        rule_stop: true,
        ..State::default()
    }
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
#[derive(Default)]
struct Trace {
    selected: usize,
    stacks: Vec<Vec<(usize, usize)>>,
    inner: Option<FirstSharedToken>,
}
impl TokenProbe for Trace {
    fn depth_first(&self) -> bool {
        false
    }
    fn select(
        &mut self,
        symbol: &PredictionSymbol,
        alternative: usize,
        rows: &BTreeSet<AnalysisConfiguration>,
        work: &mut usize,
    ) -> Result<Option<BTreeSet<AnalysisConfiguration>>, String> {
        let value = self
            .inner
            .get_or_insert_with(|| FirstSharedToken::new(false))
            .select(symbol, alternative, rows, work)?;
        if let Some(rows) = &value {
            self.selected += 1;
            // Test-only diagnostics, outside the production source admission.
            self.stacks = rows.iter().map(|row| row.calls.clone()).collect();
        }
        Ok(value)
    }
}
fn run<P: TokenProbe>(
    program: &ParserPrediction,
    work: &mut usize,
    probe: &mut P,
    shared: bool,
) -> (Result<bool, String>, BTreeMap<(usize, usize), bool>) {
    let mut witnesses = BTreeMap::new();
    let value=program.multiple_recursive_alternatives_with_probe::<
        adjacent::GeometricAdjacentQueue<'_>,frontier::AdoptFrontiers,true,P>(
        &Decision{entries:vec![0,1],exit:None},work,shared,&mut witnesses,false,
        (true,None,None,None),probe);
    (value, witnesses)
}
fn early_fork(length: usize) -> ParserPrediction {
    let mut states = vec![
        node(vec![token("a", 2), Edge::Epsilon(7)]),
        node(vec![token("a", 3), Edge::Epsilon(8)]),
        node(vec![Edge::Call {
            target: 4,
            follow: 5,
        }]),
        node(vec![Edge::Call {
            target: 4,
            follow: 5,
        }]),
        node(vec![token("r", 6)]),
        stop(),
        node(vec![Edge::Call {
            target: 4,
            follow: 5,
        }]),
        State::default(),
        State::default(),
    ];
    let mut tails = [7, 8];
    for _ in 0..length {
        let next = [states.len(), states.len() + 1];
        states.push(State::default());
        states.push(State::default());
        for i in 0..2 {
            states[tails[i]] = node(vec![Edge::Epsilon(next[i])]);
        }
        tails = next;
    }
    for at in tails {
        states[at] = stop();
    }
    graph(states)
}
#[test]
fn partial_token_positive_reaches_real_witness_before_unfinished_initial_closure() {
    let p = early_fork(512);
    let before = serde_json::to_value(&p).unwrap();
    let mut ordinary = 200;
    let old = run(&p, &mut ordinary, &mut NoProbe, true);
    assert_eq!(old.0, Err("Grammar prediction budget exceeded".into()));
    assert_eq!(ordinary, 0);
    let mut work = 200;
    let mut trace = Trace::default();
    let new = run(&p, &mut work, &mut trace, true);
    println!(
        "PARTIAL_TOKEN_POSITIVE {}",
        serde_json::json!({"old":old.0,"selected":new.0,
        "selected_work":200-work,"probe_calls":trace.selected,"source_unchanged":true})
    );
    assert_eq!(new.0, Ok(true));
    assert!(work > 0);
    assert_eq!(trace.selected, 1);
    assert_eq!(trace.stacks, vec![vec![], vec![]]);
    assert!(new.1.values().any(|v| *v));
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    // The two pending epsilon chains each still contain 512 edges when the
    // first matching token is recorded. The same worker proves the continuation;
    // it never certifies those unfinished branches as closed/nonrecursive.
    let mut ample = 100_000;
    assert_eq!(run(&p, &mut ample, &mut NoProbe, true).0, Ok(true));
}
#[test]
fn partial_token_overlap_and_distinct_return_frames_never_mint_recursion() {
    let mut end = stop();
    end.follow = vec![6];
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
        end,
        node(vec![token("x", 7)]),
        node(vec![token("y", 7)]),
        node(vec![Edge::Call {
            target: 6,
            follow: 7,
        }]),
        stop(),
    ]);
    let before = serde_json::to_value(&p).unwrap();
    let mut work = 10_000;
    let mut trace = Trace::default();
    let result = run(&p, &mut work, &mut trace, true);
    assert_eq!(result.0, Ok(false));
    assert_eq!(trace.selected, 1);
    assert_eq!(trace.stacks, vec![vec![(0, 4)], vec![(1, 5)]]);
    assert_eq!(result.1.get(&(2, 128)), Some(&false));
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    // Decisive counterexample: losing the real ordered caller frames would
    // wrongly take callerless FOLLOW and manufacture the recursive path at 6.
    let wrong: BTreeSet<_> = [0, 1]
        .into_iter()
        .map(|alternative| AnalysisConfiguration {
            state: 3,
            alternative,
            calls: vec![],
        })
        .collect();
    let mut witnesses = BTreeMap::new();
    let mut wrong_work = 10_000;
    let wrong_result=p.multiple_recursive_configurations_with_frontiers::<
        adjacent::GeometricAdjacentQueue<'_>,frontier::AdoptFrontiers,true,NoProbe>(
        wrong,&mut wrong_work,true,&mut witnesses,false,(true,None,None,None),&mut NoProbe);
    assert_eq!(wrong_result, Ok(true));
}
#[test]
fn partial_token_false_callee_witness_preserves_recursive_actual_caller_suffix() {
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
        stop(),
        node(vec![Edge::Epsilon(6)]),
        node(vec![Edge::Epsilon(6)]),
        node(vec![Edge::Call {
            target: 8,
            follow: 7,
        }]),
        stop(),
        node(vec![token("r", 9)]),
        node(vec![Edge::Call {
            target: 8,
            follow: 7,
        }]),
    ]);
    let mut work = 10_000;
    let mut trace = Trace::default();
    let result = run(&p, &mut work, &mut trace, true);
    assert_eq!(trace.selected, 1);
    assert_eq!(trace.stacks, vec![vec![(0, 4)], vec![(1, 5)]]);
    assert_eq!(result.1.get(&(2, 128)), Some(&false));
    assert!(result.1.iter().any(|((target, _), v)| *target == 8 && *v));
    assert_eq!(result.0, Ok(true));
    let mut old = 10_000;
    assert_eq!(run(&p, &mut old, &mut NoProbe, true).0, Ok(true));
}
#[test]
fn partial_token_false_probe_keeps_later_outer_tokens_and_runs_at_most_once() {
    let mut p = early_fork(6);
    p.states[2] = stop();
    p.states[3] = stop();
    // The first shared a continuation is nonrecursive. Later same-prefix b
    // continuations must still be retained by the unfinished outer closure.
    let terminal = p.states.len() - 2;
    p.states[terminal] = node(vec![token("b", 4)]);
    p.states[terminal + 1] = node(vec![token("b", 6)]);
    let mut work = 10_000;
    let mut trace = Trace::default();
    let result = run(&p, &mut work, &mut trace, true);
    assert_eq!(trace.selected, 1);
    assert_eq!(result.0, Ok(true));
    assert!(result.1.values().any(|v| *v));
    let mut old = 10_000;
    assert_eq!(run(&p, &mut old, &mut NoProbe, true).0, Ok(true));
    let symbol = PredictionSymbol::Keyword("later".into());
    let rows = BTreeSet::new();
    let before = work;
    assert!(
        trace
            .select(&symbol, 7, &rows, &mut work)
            .unwrap()
            .is_none()
    );
    assert_eq!(work, before);
    assert_eq!(trace.selected, 1);
}
fn depth_graph(depth: usize) -> ParserPrediction {
    let mut states = vec![
        node(vec![token("a", 2)]),
        node(vec![token("a", 3)]),
        State::default(),
        State::default(),
    ];
    let mut tails = [2, 3];
    for _ in 0..depth {
        let next = [states.len(), states.len() + 1];
        states.push(State::default());
        states.push(State::default());
        for i in 0..2 {
            states[tails[i]] = node(vec![Edge::Call {
                target: next[i],
                follow: usize::MAX,
            }]);
        }
        tails = next;
    }
    states[tails[0]] = node(vec![token("left", usize::MAX)]);
    states[tails[1]] = node(vec![token("right", usize::MAX)]);
    graph(states)
}
#[test]
fn partial_token_real_depth128_129_and_dangling_errors_remain_live() {
    for depth in [127, 128, 129] {
        let p = depth_graph(depth);
        let mut old = 100_000;
        let mut selected = 100_000;
        let a = run(&p, &mut old, &mut NoProbe, false).0;
        let b = run(&p, &mut selected, &mut Trace::default(), false).0;
        assert_eq!(a, b);
        if depth == 129 {
            assert_eq!(b, Err("Parser prediction call depth exceeded".into()));
        } else {
            assert_eq!(b, Ok(false));
        }
    }
    let p = graph(vec![
        node(vec![token("a", usize::MAX)]),
        node(vec![token("a", usize::MAX)]),
    ]);
    let before = serde_json::to_value(&p).unwrap();
    let mut work = 10_000;
    let mut trace = Trace::default();
    assert_eq!(
        run(&p, &mut work, &mut trace, true).0,
        Err("Dangling parser prediction state".into())
    );
    assert_eq!(trace.selected, 1);
    assert!(work > 0);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
}
#[test]
fn partial_token_nonproductive_stops_and_guard_edges_use_literal_classifier_context() {
    let p = graph(vec![
        node(vec![
            Edge::First {
                path: "actual/static/context".into(),
                target: 2,
            },
            token("a", 4),
        ]),
        node(vec![Edge::Full { target: 3, stop: 6 }, token("a", 5)]),
        node(vec![Edge::Epsilon(2)]),
        node(vec![Edge::Action(3)]),
        stop(),
        stop(),
        stop(),
    ]);
    let before = serde_json::to_value(&p).unwrap();
    let mut old = 10_000;
    let mut new = 10_000;
    let a = run(&p, &mut old, &mut NoProbe, true).0;
    let mut trace = Trace::default();
    let b = run(&p, &mut new, &mut trace, true).0;
    assert_eq!(a, Ok(false));
    assert_eq!(a, b);
    assert_eq!(trace.selected, 1);
    assert_eq!(serde_json::to_value(&p).unwrap(), before);
    // Different actual tokens never form a continuation bucket or a probe.
    let p = graph(vec![
        node(vec![token("left", 2)]),
        node(vec![token("right", 2)]),
        stop(),
    ]);
    let mut work = 10_000;
    let mut trace = Trace::default();
    assert_eq!(run(&p, &mut work, &mut trace, true).0, Ok(false));
    assert_eq!(trace.selected, 0);
    // Static First/Full edges above retain the SAME classifier interpretation.
    // Genuine runtime guard behavior is covered separately by imported fixtures.
}
#[test]
fn partial_token_whole_worker_exact_short_zero_and_preconsumed_original_pool() {
    let p = early_fork(512);
    let mut ample = 10_000;
    assert_eq!(run(&p, &mut ample, &mut Trace::default(), true).0, Ok(true));
    let used = 10_000 - ample;
    assert!(used < 200);
    let mut exact = used;
    assert_eq!(run(&p, &mut exact, &mut Trace::default(), true).0, Ok(true));
    assert_eq!(exact, 0);
    let mut short = used - 1;
    assert_eq!(
        run(&p, &mut short, &mut Trace::default(), true).0,
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(short, 0);
    let mut zero = 0;
    assert_eq!(
        run(&p, &mut zero, &mut Trace::default(), true).0,
        Err("Grammar prediction budget exceeded".into())
    );
    let mut cumulative = used + 7;
    for _ in 0..7 {
        analysis_charge(&mut cumulative).unwrap();
    }
    assert_eq!(
        run(&p, &mut cumulative, &mut Trace::default(), true).0,
        Ok(true)
    );
    assert_eq!(cumulative, 0);
    println!(
        "PARTIAL_TOKEN_POOL {}",
        serde_json::json!({"operation":used,"preconsumed":7,"total":used+7})
    );
}
fn source_rows() -> BTreeSet<AnalysisConfiguration> {
    [0, 1]
        .into_iter()
        .map(|alternative| AnalysisConfiguration {
            state: 2,
            alternative,
            calls: vec![],
        })
        .collect()
}
fn source_operation(work: &mut usize) -> Result<usize, String> {
    let symbol = PredictionSymbol::Keyword("a".into());
    let rows = source_rows();
    let mut probe = FirstSharedToken::new(false);
    assert!(probe.select(&symbol, 0, &rows, work)?.is_none());
    assert_eq!(probe.select(&symbol, 1, &rows, work)?, Some(rows));
    Ok(probe.logical())
}
#[test]
fn partial_token_source_policy_literal23work417logical_no_default_fee_or_host_size() {
    let mut exact = 23;
    assert_eq!(source_operation(&mut exact), Ok(417));
    assert_eq!(exact, 0);
    let mut short = 22;
    assert_eq!(
        source_operation(&mut short),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(short, 0);
    let mut zero = 0;
    assert_eq!(
        source_operation(&mut zero),
        Err("Grammar prediction budget exceeded".into())
    );
    let mut cumulative = 30;
    for _ in 0..7 {
        analysis_charge(&mut cumulative).unwrap();
    }
    assert_eq!(source_operation(&mut cumulative), Ok(417));
    assert_eq!(cumulative, 0);
    let mut work = 0;
    let mut no = NoProbe;
    assert!(
        no.select(&PredictionSymbol::End, 0, &source_rows(), &mut work)
            .unwrap()
            .is_none()
    );
    assert_eq!(work, 0);
    assert_eq!(std::mem::size_of::<NoProbe>(), 0);
    assert_eq!(std::mem::size_of::<RecursiveFrontierOrder>(), 1);
    assert_eq!(
        RecursiveFrontierOrder::default(),
        RecursiveFrontierOrder::Historical
    );
    println!(
        "PARTIAL_TOKEN_SOURCE_FEES {}",
        serde_json::json!({"cold_operation":23,"logical_cumulative":417,
        "preconsumed":7,"same_pool_total":30,"default_work":0,"policy_uses_sizeof":false})
    );
}
