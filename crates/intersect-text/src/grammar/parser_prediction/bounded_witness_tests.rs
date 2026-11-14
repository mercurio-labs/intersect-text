//! Canonical graph components. These are not original grammar admission runs.
use super::super::{
    Decision, Edge, ParserPrediction, PredictionSymbol, RecursiveAnalysisCounters, State,
    adjacent::GeometricAdjacentQueue, frontier::AdoptFrontiers,
};
use super::lookup_completed;
use crate::grammar::Profile;
use crate::grammar::admission_observation::RunStage;
use std::collections::{BTreeMap, BTreeSet};

fn state(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
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
fn recursive_callee() -> ParserPrediction {
    let mut states = vec![
        state(vec![Edge::Call {
            target: 2,
            follow: 99,
        }]),
        state(vec![Edge::Call {
            target: 2,
            follow: 99,
        }]),
    ];
    for offset in 0..96 {
        states.push(state(vec![Edge::Epsilon(3 + offset)]));
    }
    states.push(state(vec![Edge::Call {
        target: 2,
        follow: 99,
    }]));
    states.push(State {
        rule_stop: true,
        ..State::default()
    });
    program(states)
}
fn finite_callee() -> ParserPrediction {
    let mut states = vec![
        state(vec![Edge::Call {
            target: 2,
            follow: 6,
        }]),
        state(vec![Edge::Call {
            target: 3,
            follow: 7,
        }]),
        state(vec![Edge::Call {
            target: 8,
            follow: 4,
        }]),
        state(vec![Edge::Call {
            target: 8,
            follow: 5,
        }]),
        State {
            rule_stop: true,
            ..State::default()
        },
        State {
            rule_stop: true,
            ..State::default()
        },
        state(vec![Edge::Token(
            PredictionSymbol::Keyword("a".into()),
            105,
        )]),
        state(vec![Edge::Token(
            PredictionSymbol::Keyword("b".into()),
            105,
        )]),
    ];
    for offset in 0..96 {
        states.push(state(vec![Edge::Epsilon(9 + offset)]));
    }
    states.push(State {
        rule_stop: true,
        ..State::default()
    });
    states.push(State::default());
    program(states)
}
fn analyze<const REUSE: bool>(
    p: &ParserPrediction,
    witnesses: &mut BTreeMap<(usize, usize), bool>,
    remaining: &mut usize,
    measured: Option<&RecursiveAnalysisCounters>,
) -> Result<bool, String> {
    p.multiple_recursive_alternatives_with_frontiers::<
        GeometricAdjacentQueue<'_>, AdoptFrontiers, REUSE,
    >(
        &Decision { entries: vec![0, 1], exit: None },
        remaining,
        true,
        witnesses,
        true,
        (true, measured, None, None),
    )
}
fn witness(p: &ParserPrediction, entry: usize, bound: usize) -> (bool, usize) {
    let mut remaining = 10_000;
    let value = p.recursion_witness(entry, bound, &mut remaining).unwrap();
    (value, 10_000 - remaining)
}

#[test]
fn bound_range_literal_setup_next_exhaustion_and_exact_budget_recovery() {
    let empty = BTreeMap::new();
    for grant in [0, 1] {
        let mut remaining = grant;
        assert_eq!(
            lookup_completed(&empty, 7, 9, &mut remaining),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(remaining, 0);
    }
    let mut exact = 2;
    assert_eq!(lookup_completed(&empty, 7, 9, &mut exact), Ok(None));
    assert_eq!(exact, 0);
    let compatible = BTreeMap::from([((7, 8), true)]);
    let before = compatible.clone();
    for grant in [0, 1, 2] {
        let mut remaining = grant;
        let result = lookup_completed(&compatible, 7, 9, &mut remaining);
        assert_eq!(remaining, 0);
        if grant == 2 {
            assert_eq!(result, Ok(Some(true)));
        } else {
            assert!(result.is_err());
        }
        assert_eq!(compatible, before);
    }
    let mut remaining = 10_000 - 7;
    assert_eq!(
        lookup_completed(&compatible, 7, 9, &mut remaining),
        Ok(Some(true))
    );
    assert_eq!(remaining, 10_000 - 9);
    let incompatible = BTreeMap::from([((7, 10), true)]);
    let mut short = 2;
    assert!(lookup_completed(&incompatible, 7, 9, &mut short).is_err());
    let mut exact = 3;
    assert_eq!(lookup_completed(&incompatible, 7, 9, &mut exact), Ok(None));
    assert_eq!(exact, 0); // Setup, one rejected fact, exhausted iterator attempt.
}

#[test]
fn true_upward_and_false_downward_follow_actual_completed_witnesses() {
    let recursive = recursive_callee();
    let (fact, used) = witness(&recursive, 2, 127);
    assert!(fact);
    assert_eq!(used, 194); // 97 actual state and 97 edge inspections.
    let mut work = 2;
    assert_eq!(
        lookup_completed(&BTreeMap::from([((2, 127), fact)]), 2, 128, &mut work),
        Ok(Some(witness(&recursive, 2, 128).0))
    );
    assert_eq!(work, 0);
    let finite = finite_callee();
    let (fact, used) = witness(&finite, 8, 128);
    assert!(!fact);
    assert_eq!(used, 193); // 97 states; 96 epsilon edges; callee stop never follows callers.
    let mut work = 2;
    assert_eq!(
        lookup_completed(&BTreeMap::from([((8, 128), fact)]), 8, 127, &mut work),
        Ok(Some(witness(&finite, 8, 127).0))
    );
    assert_eq!(work, 0);
}

#[test]
fn invalid_converses_must_miss_and_run_the_unchanged_worker() {
    let p = program(vec![
        state(vec![Edge::Call {
            target: 0,
            follow: 1,
        }]),
        State::default(),
    ]);
    assert_eq!(witness(&p, 0, 1), (false, 2));
    assert_eq!(witness(&p, 0, 2), (true, 2));
    for (completed_bound, requested_bound, completed, requested) in
        [(1, 2, false, true), (2, 1, true, false)]
    {
        let map = BTreeMap::from([((0, completed_bound), completed)]);
        let mut remaining = 5;
        assert_eq!(
            lookup_completed(&map, 0, requested_bound, &mut remaining),
            Ok(None)
        );
        assert_eq!(remaining, 2);
        assert_eq!(
            p.recursion_witness(0, requested_bound, &mut remaining),
            Ok(requested)
        );
        assert_eq!(remaining, 0);
        assert_eq!(map.len(), 1);
    }
}

#[test]
fn graph_entry_is_exact_and_errors_never_become_completed_facts() {
    let p = program(vec![
        state(vec![Edge::Call {
            target: 99,
            follow: 2,
        }]),
        state(vec![Edge::Call {
            target: 99,
            follow: 2,
        }]),
        State::default(),
    ]);
    let map = BTreeMap::from([((98, 1), true), ((100, 128), false)]);
    let mut exact = 2;
    assert_eq!(lookup_completed(&map, 99, 128, &mut exact), Ok(None));
    assert_eq!(exact, 0); // Other entries are outside the lazy range, not inspected facts.
    for reuse in [false, true] {
        let mut facts = map.clone();
        let mut remaining = 10_000;
        let result = if reuse {
            analyze::<true>(&p, &mut facts, &mut remaining, None)
        } else {
            analyze::<false>(&p, &mut facts, &mut remaining, None)
        };
        assert_eq!(result, Err("Dangling parser prediction state".into()));
        assert_eq!(facts, map);
        assert!(remaining < 10_000);
    }
    // A fresh compiler invocation owns a fresh map; no graph facts survive it.
    let mut fresh = BTreeMap::new();
    let mut zero = 0;
    assert!(analyze::<true>(&p, &mut fresh, &mut zero, None).is_err());
    assert!(fresh.is_empty());
}

#[test]
fn same_classifier_long_true_callee_saves_actual_work_without_exact_hit_relabeling() {
    let p = recursive_callee();
    let immutable = serde_json::to_value(&p).unwrap();
    let completed = witness(&p, 2, 127).0;
    let original = BTreeMap::from([((2, 127), completed)]);
    let mut records = Vec::new();
    for reuse in [false, true] {
        let counter = RecursiveAnalysisCounters::new();
        counter.begin_program(RunStage::Executable, 10_000);
        let mut facts = original.clone();
        let mut remaining = 10_000;
        let result = if reuse {
            analyze::<true>(&p, &mut facts, &mut remaining, Some(&counter))
        } else {
            analyze::<false>(&p, &mut facts, &mut remaining, Some(&counter))
        };
        counter.end_program(remaining, result.is_ok());
        assert_eq!(result, Ok(true));
        let work = counter.snapshot().stages[2].work;
        assert_eq!((work.witness_hits, work.witness_misses), (0, 1));
        assert_eq!(work.witness_miss_true, u64::from(!reuse));
        assert_eq!(work.witness_miss_granted_work, if reuse { 0 } else { 194 });
        assert_eq!(facts.len(), if reuse { 1 } else { 2 });
        records.push(10_000 - remaining);
    }
    assert_eq!(records[0] - records[1], 192); // Same worker194 actually replaced by range2.
    assert_eq!(serde_json::to_value(&p).unwrap(), immutable);
    println!(
        "V80_LONG_CALLEE_WORK {}",
        serde_json::json!({
            "scope":"Authored immutable graph component, not full grammar admission",
            "old_used":records[0],"new_used":records[1],"actual_saved":192,
            "old_exact_hits":0,"new_exact_hits":0,"exact_misses_each":1,
            "bound_reuse_results":1,"seven_category_report":"Partial; new range fees are not seven-category events",
        })
    );
}

#[test]
fn same_classifier_false_callee_keeps_follow_context_and_no_redundant_alias() {
    let p = finite_callee();
    let actual = witness(&p, 8, 128).0;
    assert!(!actual);
    let original = BTreeMap::from([((8, 128), actual)]);
    let mut records = Vec::new();
    for reuse in [false, true] {
        let mut facts = original.clone();
        let mut remaining = 10_000;
        let result = if reuse {
            analyze::<true>(&p, &mut facts, &mut remaining, None)
        } else {
            analyze::<false>(&p, &mut facts, &mut remaining, None)
        };
        assert_eq!(result, Ok(false));
        assert_eq!(facts.contains_key(&(8, 127)), !reuse);
        records.push(10_000 - remaining);
    }
    assert_eq!(records[0] - records[1], 191); // Finite witness193 replaced by real range2.
}

#[test]
fn cold_miss_retains_overhead_exact_hit_is_free_and_original_budget_recovers() {
    let p = recursive_callee();
    let mut old_work = 10_000;
    let mut old_facts = BTreeMap::new();
    assert_eq!(
        analyze::<false>(&p, &mut old_facts, &mut old_work, None),
        Ok(true)
    );
    let old_used = 10_000 - old_work;
    let mut new_work = 10_000;
    let mut new_facts = BTreeMap::new();
    assert_eq!(
        analyze::<true>(&p, &mut new_facts, &mut new_work, None),
        Ok(true)
    );
    assert_eq!(10_000 - new_work, old_used + 2);
    assert_eq!(new_facts, old_facts);
    let mut exact_old = 10_000;
    let mut exact_new = 10_000;
    assert_eq!(
        analyze::<false>(&p, &mut old_facts, &mut exact_old, None),
        Ok(true)
    );
    assert_eq!(
        analyze::<true>(&p, &mut new_facts, &mut exact_new, None),
        Ok(true)
    );
    assert_eq!(exact_old, exact_new); // The original exact-key branch is untouched.
    let completed = witness(&p, 2, 127).0;
    let source = BTreeMap::from([((2, 127), completed)]);
    let mut full = 10_000;
    assert_eq!(
        analyze::<true>(&p, &mut source.clone(), &mut full, None),
        Ok(true)
    );
    let used = 10_000 - full;
    for grant in [0, used - 1, used] {
        let mut remaining = grant;
        let mut facts = source.clone();
        let result = analyze::<true>(&p, &mut facts, &mut remaining, None);
        assert_eq!(remaining, 0);
        assert_eq!(facts, source);
        if grant == used {
            assert_eq!(result, Ok(true));
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
        }
    }
    let mut remaining = 10_000 - 7;
    assert_eq!(
        analyze::<true>(&p, &mut source.clone(), &mut remaining, None),
        Ok(true)
    );
    assert_eq!(remaining, 10_000 - 7 - used);
}

#[test]
fn explicit_profile80_preserves_old_aliases_and_program_serialization() {
    for number in 1..=80 {
        let wire = format!("intersect.xtext.native.v{number}");
        let parsed: Profile = serde_json::from_value(serde_json::json!(wire)).unwrap();
        assert_eq!(parsed.as_str(), wire);
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::json!(wire)
        );
    }
    assert_ne!(Profile::NativeV79, Profile::NativeV80);
    let p = recursive_callee();
    let original = serde_json::to_value(&p).unwrap();
    let round_trip: ParserPrediction = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(serde_json::to_value(round_trip).unwrap(), original);
    // No profile flag, source cache or proof field is serialized into the program.
    let object = original.as_object().unwrap();
    assert_eq!(object.len(), 4); // allow_empty_choices is skipped; empty recursive_decisions is omitted.
    assert!(!object.contains_key("witnesses"));
    assert!(!object.contains_key("profile"));
}

#[test]
fn partial_native80_worker_zero_exact_short_and_real_denied_sites_keep_result_and_facts() {
    let p = recursive_callee();
    let serialized = serde_json::to_value(&p).unwrap();
    let run = |grant: usize, measured: Option<&RecursiveAnalysisCounters>| {
        let mut remaining = grant;
        let mut facts = BTreeMap::new();
        let result = analyze::<true>(&p, &mut facts, &mut remaining, measured);
        (result, remaining, facts)
    };
    let full = run(10_000, None);
    assert_eq!(full.0, Ok(true));
    let used = 10_000 - full.1;
    assert!(used > 3);
    for grant in [0, 1, 2, used - 1, used] {
        let old = run(grant, None);
        let counters = RecursiveAnalysisCounters::new();
        counters.begin_program(RunStage::Executable, grant);
        let actual = run(grant, Some(&counters));
        counters.end_program(actual.1, actual.0.is_ok());
        assert_eq!(actual, old);
        assert_eq!(actual.1, 0);
        let snapshot = counters.snapshot();
        let program = snapshot.last_program.unwrap();
        assert!(!snapshot.overflowed && !snapshot.inconsistent);
        assert!(program.granted.iter().sum::<u64>() <= u64::try_from(grant).unwrap());
        if grant == used {
            assert_eq!(actual.0, Ok(true));
            assert_eq!(actual.2, full.2);
        } else {
            assert_eq!(actual.0, Err("Grammar prediction budget exceeded".into()));
        }
        if grant < 2 {
            // Native80's actual initial-frontier publication fails before any
            // seven-category charge. Missing denied coverage is not free work.
            assert_eq!(program.granted, [0; 7]);
            assert_eq!(program.denied, [0; 7]);
        } else if grant == 2 {
            // Two paid initial publication units precede the original worker's
            // first FrontierPop attempt, which is now genuinely denied.
            assert_eq!(program.granted, [0; 7]);
            assert_eq!(program.denied, [1, 0, 0, 0, 0, 0, 0]);
        }
    }
    let counters = RecursiveAnalysisCounters::new();
    counters.begin_program(RunStage::Executable, 10_000 - 7);
    let prepaid = run(10_000 - 7, Some(&counters));
    counters.end_program(prepaid.1, prepaid.0.is_ok());
    assert_eq!(prepaid.0, full.0);
    assert_eq!(prepaid.2, full.2);
    assert_eq!(prepaid.1, full.1 - 7);
    assert_eq!(run(used, None).0, Ok(true));
    assert_eq!(serde_json::to_value(&p).unwrap(), serialized);
}
