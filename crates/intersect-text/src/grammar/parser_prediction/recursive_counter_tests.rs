// Same engine and graph data; counter expectations are literal work units.
use super::super::admission_observation::{RecursiveAnalysisCounters, RunStage};
fn metric_program(states: Vec<State>) -> ParserPrediction {
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
fn metric_state(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn metric_token(value: &str, target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(value.into()), target)
}
fn metric_run(
    p: &ParserPrediction,
    entries: &[usize],
    budget: usize,
    unique: bool,
    shared: bool,
) -> (
    Result<bool, String>,
    usize,
    super::super::admission_observation::RecursiveCounterSnapshot,
) {
    let counter = RecursiveAnalysisCounters::new();
    counter.begin_program(RunStage::Executable, budget);
    let mut remaining = budget;
    let before = serde_json::to_value(p).unwrap();
    let result = p.multiple_recursive_alternatives(
        &Decision {
            entries: entries.to_vec(),
            exit: None,
        },
        &mut remaining,
        shared,
        &mut BTreeMap::new(),
        true,
        (unique, Some(&counter)),
    );
    counter.end_program(remaining, result.is_ok());
    assert_eq!(serde_json::to_value(p).unwrap(), before);
    let mut plain = budget;
    assert_eq!(
        p.multiple_recursive_alternatives(
            &Decision {
                entries: entries.to_vec(),
                exit: None
            },
            &mut plain,
            shared,
            &mut BTreeMap::new(),
            true,
            (unique, None)
        ),
        result
    );
    assert_eq!(plain, remaining);
    (result, remaining, counter.snapshot())
}
#[test]
fn recursive_counters_diamond_conserves_literal_old15_new14_and_denial_category() {
    let p = metric_program(vec![
        metric_state(vec![Edge::Epsilon(1), Edge::Epsilon(2)]),
        metric_state(vec![Edge::Action(3)]),
        metric_state(vec![Edge::Action(3)]),
        metric_state(vec![metric_token("a", 4)]),
        State::default(),
        State::default(),
        metric_state(vec![metric_token("b", 7)]),
        State::default(),
    ]);
    for (unique, budget, expected) in [
        (false, 15, [3, 6, 0, 0, 6, 0, 0]),
        (true, 14, [3, 0, 5, 0, 6, 0, 0]),
    ] {
        let (result, remaining, s) = metric_run(&p, &[0, 6], budget, unique, false);
        assert_eq!((result, remaining), (Ok(false), 0));
        let program = s.last_program.unwrap();
        assert!(program.conserved);
        assert_eq!(program.granted, expected);
        assert_eq!(program.denied, [0; 7]);
        let counts = s.stages[2].work;
        if unique {
            assert_eq!(counts.scheduling[0], [2, 2, 0]);
            assert_eq!(counts.scheduling[3], [3, 3, 1]);
            assert_eq!(counts.closure_pops, 5);
        } else {
            assert_eq!(
                (counts.legacy_unique_pops, counts.legacy_duplicate_pops),
                (5, 1)
            );
        }
        for work in [0, budget - 1] {
            let (result, _, s) = metric_run(&p, &[0, 6], work, unique, false);
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            let p = s.last_program.unwrap();
            assert!(p.conserved);
            assert_eq!(p.denied, [1, 0, 0, 0, 0, 0, 0]);
        }
    }
}
#[test]
fn recursive_counters_early_true_keeps_prepaid_cost_and_witness_units() {
    let p = metric_program(vec![
        metric_state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        metric_state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        metric_state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        State::default(),
    ]);
    for (unique, budget, expected) in [
        (false, 7, [1, 2, 0, 0, 2, 1, 1]),
        (true, 8, [1, 0, 3, 0, 2, 1, 1]),
    ] {
        let (result, remaining, s) = metric_run(&p, &[0, 1], budget, unique, true);
        assert_eq!((result, remaining), (Ok(true), 0));
        assert_eq!(s.last_program.unwrap().granted, expected);
        assert!(s.last_program.unwrap().conserved);
        let work = s.stages[2].work;
        assert_eq!(
            (
                work.witness_misses,
                work.witness_miss_true,
                work.witness_miss_granted_work
            ),
            (1, 1, 2)
        );
        assert_eq!(work.closure_pops, 2);
    }
}
#[test]
fn recursive_counters_measure_one_real_witness_miss_then_hit_and_uncached_denial() {
    let p = metric_program(vec![
        metric_state(vec![Edge::Call {
            target: 3,
            follow: 5,
        }]),
        metric_state(vec![Edge::Call {
            target: 3,
            follow: 5,
        }]),
        metric_state(vec![Edge::Call {
            target: 3,
            follow: 5,
        }]),
        metric_state(vec![metric_token("a", 4)]),
        State {
            rule_stop: true,
            ..State::default()
        },
        metric_state(vec![metric_token("z", 6)]),
        State {
            rule_stop: true,
            ..State::default()
        },
    ]);
    for unique in [false, true] {
        let (result, remaining, s) = metric_run(&p, &[0, 1, 2], 30, unique, true);
        assert_eq!((result, remaining), (Ok(false), 0));
        let work = s.stages[2].work;
        assert_eq!(
            (
                work.witness_misses,
                work.witness_hits,
                work.witness_miss_false,
                work.witness_miss_granted_work
            ),
            (1, 1, 1, 3)
        );
        assert_eq!(s.last_program.unwrap().granted[5..], [2, 1]);
        assert!(s.last_program.unwrap().conserved);
        let budget = if unique { 9 } else { 7 };
        for _ in 0..2 {
            let (result, _, s) = metric_run(&p, &[0, 1, 2], budget, unique, true);
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            let work = s.stages[2].work;
            assert_eq!(
                (
                    work.witness_hits,
                    work.witness_misses,
                    work.witness_miss_errors,
                    work.witness_miss_granted_work
                ),
                (0, 1, 1, 2)
            );
            assert_eq!(s.last_program.unwrap().denied, [0, 0, 0, 0, 0, 1, 0]);
            assert!(s.last_program.unwrap().conserved);
        }
    }
}
#[test]
fn recursive_counters_all_three_enqueue_sites_keep_original_charges_and_full_keys() {
    let callerless = metric_program(vec![
        State {
            rule_stop: true,
            follow: vec![2, 2],
            ..State::default()
        },
        metric_state(vec![metric_token("b", 3)]),
        metric_state(vec![metric_token("a", 3)]),
        State::default(),
    ]);
    let (result, _, s) = metric_run(&callerless, &[0, 1], 10, true, false);
    assert_eq!(result, Ok(false));
    assert_eq!(s.stages[2].work.scheduling[2], [1, 1, 1]);
    assert_eq!(s.last_program.unwrap().granted, [3, 0, 3, 2, 2, 0, 0]);
    let returns = metric_program(vec![
        metric_state(vec![
            Edge::Call {
                target: 2,
                follow: 3,
            },
            Edge::Call {
                target: 2,
                follow: 4,
            },
        ]),
        metric_state(vec![metric_token("b", 5)]),
        State {
            rule_stop: true,
            ..State::default()
        },
        metric_state(vec![metric_token("a", 5)]),
        metric_state(vec![metric_token("a", 5)]),
        State::default(),
    ]);
    let (result, _, s) = metric_run(&returns, &[0, 1], 14, true, false);
    assert_eq!(result, Ok(false));
    assert_eq!(s.stages[2].work.scheduling[1], [2, 2, 0]);
    assert_eq!(s.stages[2].work.scheduling[3], [2, 2, 0]);
    assert!(s.last_program.unwrap().conserved);
}
#[test]
fn recursive_counters_whole_program_none_and_old_phase_results_remain_exact() {
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let mut g = grammar(choice(vec![keyword("a"), keyword("b")]), vec![]);
    g.profile = Profile::NativeV73;
    let source = DefinitionSource::new(
        "memory:counter-control.xtext",
        SourceKind::Xtext,
        "Entry: ->'a'|'b';",
    );
    let plain = ParserPrediction::compile_with_recursive_first_predicates(
        &g,
        &BTreeSet::from(["rule/Entry/a0".into()]),
    )
    .unwrap();
    let old = super::super::admission_observation::PredictionObservation::new();
    let measured = super::super::admission_observation::PredictionObservation::new();
    let counters = RecursiveAnalysisCounters::new();
    let run = |phase: &super::super::admission_observation::PredictionObservation, work| {
        ParserPrediction::compile_observed(
            &g,
            &BTreeSet::from(["rule/Entry/a0".into()]),
            true,
            super::super::AnalysisObservation {
                observation: phase,
                recursive_work: work,
                adjacent_work: None,
                entry: &source.id,
                stage: RunStage::Executable,
            },
        )
        .unwrap()
    };
    let a = run(&old, None);
    let b = run(&measured, Some(&counters));
    assert_eq!(
        serde_json::to_value(plain).unwrap(),
        serde_json::to_value(a).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&b).unwrap(),
        serde_json::to_value(run(&old, None)).unwrap()
    );
    assert_eq!(old.first_failure(), measured.first_failure());
    assert_eq!(old.coverage(), measured.coverage());
    let s = counters.snapshot();
    assert!(s.last_program.unwrap().conserved);
    assert_eq!(s.stages[2].program_invocations, 1);
    assert!(s.current.is_none());
    assert!(s.heaviest_completed[0].unwrap().capture_unavailable);
}
