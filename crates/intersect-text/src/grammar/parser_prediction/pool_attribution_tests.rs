//! Real canonical classifier and capped-proof worker parity; no original run.
use super::super::admission_observation::{RecursiveCounterSnapshot, RunStage};
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
fn program(recursive: bool) -> ParserPrediction {
    ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states: vec![
            node(vec![Edge::Epsilon(2)]),
            node(vec![Edge::Action(2)]),
            node(vec![token("x", 3)]),
            node(vec![Edge::Call {
                target: 4,
                follow: 6,
            }]),
            node(vec![token("r", 5)]),
            if recursive {
                node(vec![Edge::Call {
                    target: 4,
                    follow: 6,
                }])
            } else {
                stop()
            },
            node(vec![token("end", 7)]),
            State::default(),
        ],
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Run {
    result: Result<bool, String>,
    remaining: usize,
    witnesses: BTreeMap<(usize, usize), bool>,
    old: RecursiveCounterSnapshot,
}

fn run_order(
    program: &ParserPrediction,
    original: usize,
    prefix: usize,
    observed: bool,
    historical: bool,
) -> Result<
    (
        Run,
        Option<super::super::admission_observation::pool_blocks::Snapshot>,
    ),
    String,
> {
    let mut remaining = original;
    for _ in 0..prefix {
        analysis_charge(&mut remaining)?;
    }
    let capture = observed.then(|| PoolWork::new(remaining));
    let old = RecursiveAnalysisCounters::new();
    old.begin_program(RunStage::Executable, remaining);
    let root = capture
        .as_ref()
        .and_then(|work| work.enter(PoolBlock::PoolRemainder, remaining));
    let classifier = capture
        .as_ref()
        .and_then(|work| work.enter(PoolBlock::ClassifierRemainder, remaining));
    let mut witnesses = BTreeMap::new();
    let decision = Decision {
        entries: vec![0, 1],
        exit: None,
    };
    let result = if historical {
        program.multiple_recursive_alternatives_with_frontiers_pool::<
            adjacent::GeometricAdjacentQueue<'_>, frontier::AdoptFrontiers, true,
        >(
            &decision, &mut remaining, true, &mut witnesses, true,
            (true, Some(&old), None, None), capture.as_ref(),
        )
    } else {
        program.multiple_recursive_alternatives_with_frontiers_pool::<
            transparent_ports::PortQueue<'_>, frontier::AdoptFrontiers, true,
        >(
            &decision, &mut remaining, true, &mut witnesses, false,
            (true, Some(&old), None, None), capture.as_ref(),
        )
    };
    old.end_program(remaining, result.is_ok());
    if let Some(work) = &capture {
        work.leave(classifier, remaining, result.is_ok());
        work.leave(root, remaining, result.is_ok());
        work.end(remaining, result.is_ok());
    }
    Ok((
        Run {
            result,
            remaining,
            witnesses,
            old: old.snapshot(),
        },
        capture.as_ref().map(PoolWork::snapshot),
    ))
}

fn run(
    program: &ParserPrediction,
    original: usize,
    prefix: usize,
    observed: bool,
) -> Result<
    (
        Run,
        Option<super::super::admission_observation::pool_blocks::Snapshot>,
    ),
    String,
> {
    run_order(program, original, prefix, observed, false)
}

#[test]
fn compiler_pool_historical_keeps_results_witnesses_counters_and_exact_budgets()
-> Result<(), String> {
    for recursive in [false, true] {
        let p = program(recursive);
        let before = serde_json::to_value(&p).map_err(|e| e.to_string())?;
        let full = run_order(&p, 10000, 0, false, true)?.0;
        let exact = 10000 - full.remaining;
        for (grant, prefix) in [(0, 0), (exact - 1, 0), (exact, 0), (10000, 7)] {
            let plain = run_order(&p, grant, prefix, false, true)?.0;
            let (actual, report) = run_order(&p, grant, prefix, true, true)?;
            let report = report.ok_or("Missing historical pool report")?;
            assert_eq!(actual, plain);
            assert!(report.ended && report.balanced && !report.inconsistent && !report.overflowed);
            assert_eq!(
                report.counts.units.iter().sum::<u64>(),
                (grant - prefix - actual.remaining) as u64
            );
            if grant < exact {
                assert_eq!(
                    actual.result,
                    Err("Grammar prediction budget exceeded".into())
                );
            }
        }
        assert_eq!(serde_json::to_value(&p).map_err(|e| e.to_string())?, before);
    }
    Ok(())
}

#[test]
fn compiler_pool_actual_ports_seeded_calls_witnesses_and_old_seven_are_identical()
-> Result<(), String> {
    for recursive in [false, true] {
        let p = program(recursive);
        let before = serde_json::to_value(&p).map_err(|error| error.to_string())?;
        let plain = run(&p, 10_000, 0, false)?.0;
        let (actual, report) = run(&p, 10_000, 0, true)?;
        let report = report.ok_or("Missing explicit pool report")?;
        println!(
            "COMPILER_POOL_WORKER_PREASSERT {}",
            serde_json::json!({
                "recursive":recursive,"result":actual.result,"remaining":actual.remaining,
                "witnesses":actual.witnesses.iter().collect::<Vec<_>>(),
                "units":report.counts.units,"actual_denied":report.counts.actual_denied_leaf_attempts,
                "queue_share_inclusive":report.queue_share_inclusive,"balanced":report.balanced
            })
        );
        assert_eq!(actual, plain);
        assert_eq!(actual.result, Ok(recursive));
        assert_eq!(
            serde_json::to_value(&p).map_err(|error| error.to_string())?,
            before
        );
        assert!(report.ended && report.balanced && !report.inconsistent && !report.overflowed);
        assert!(
            report.queue_share_inclusive[0] > 0,
            "Actual initial PortQueue"
        );
        assert!(
            report.queue_share_inclusive[1] > 0,
            "Actual paid-token SeededQueue"
        );
        assert!(report.counts.units[PoolBlock::PortKernel as usize] > 0);
        assert!(report.counts.units[PoolBlock::PortDescriptor as usize] > 0);
        assert!(report.counts.units[PoolBlock::PortSchedule as usize] > 0);
        assert_eq!(
            report.counts.units.iter().sum::<u64>(),
            (10000 - actual.remaining) as u64
        );
        assert!(actual.witnesses.values().any(|value| *value == recursive));
    }
    Ok(())
}

#[test]
fn compiler_pool_actual_ports_zero_exact_short_preconsumed_keep_original_errors()
-> Result<(), String> {
    let p = program(false);
    let measured = run(&p, 10_000, 0, false)?.0;
    let exact = 10000 - measured.remaining;
    assert!(exact > 1);
    for (allowance, prefix) in [(0, 0), (exact - 1, 0), (exact, 0), (10000, 7)] {
        let plain = run(&p, allowance, prefix, false)?.0;
        let (actual, report) = run(&p, allowance, prefix, true)?;
        let report = report.ok_or("Missing explicit pool report")?;
        assert_eq!(actual, plain);
        assert!(report.balanced);
        assert_eq!(report.initial, allowance - prefix);
        assert_eq!(
            report.counts.units.iter().sum::<u64>(),
            (allowance - prefix - actual.remaining) as u64
        );
        if allowance == exact {
            assert_eq!(actual.result, Ok(false));
        }
        if allowance < exact {
            assert_eq!(
                actual.result,
                Err("Grammar prediction budget exceeded".into())
            );
        }
    }
    Ok(())
}

#[test]
fn compiler_pool_real_completion_cap_reconciliation_has_no_double_debit() -> Result<(), String> {
    let p = ParserPrediction {
        states: vec![node(vec![token("x", 1)]), stop()],
        ..program(false)
    };
    let mut plain = 20000;
    let old = p
        .verify_completion_scope(0, 1, &mut plain)
        .map(|scope| (scope.decisions.len(), scope.guards));
    let mut actual = 20000;
    let work = PoolWork::new(actual);
    let root = work.enter(PoolBlock::PoolRemainder, actual);
    let result = pool_scope(
        &mut actual,
        Some(&work),
        PoolBlock::NullableGraphProof,
        |remaining| p.verify_completion_scope(0, 1, remaining),
    )
    .map(|scope| (scope.decisions.len(), scope.guards));
    work.leave(root, actual, result.is_ok());
    work.end(actual, result.is_ok());
    assert_eq!(result, old);
    assert_eq!(actual, plain);
    assert!(
        actual > 10000,
        "The proof spent only its real work, not its carved cap"
    );
    let report = work.snapshot();
    assert!(report.balanced);
    assert_eq!(
        report.counts.units[PoolBlock::NullableGraphProof as usize],
        (20000 - actual) as u64
    );
    assert_eq!(report.counts.units[PoolBlock::PoolRemainder as usize], 0);
    Ok(())
}
