// Passive diagnostics only: real workers/ledgers, never a new original request.
use super::*;
use crate::grammar::Kind;
use crate::grammar::admission_observation::FirstReuseCharge as Charge;
use crate::grammar::admission_observation::FirstReuseEvent as Event;
use crate::grammar::admission_observation::FirstReuseWindow as Window;
use crate::grammar::admission_observation::FirstReuseWorkSnapshot;
use crate::grammar::first_sets;
use crate::grammar::first_sets::completed_reuse::Completed;

fn start(
    observation: &PredictionObservation,
    grammar: &Grammar,
    work: usize,
) -> Result<(), String> {
    let c = context(observation, grammar)?;
    observation.reuse_begin(c.stage, c.entry, work);
    Ok(())
}
fn snapshot(observation: &PredictionObservation) -> Result<FirstReuseWorkSnapshot, String> {
    observation
        .first_reuse_work()
        .ok_or_else(|| "Missing actual reuse window".into())
}
fn conserved(s: &FirstReuseWorkSnapshot) -> Result<(), String> {
    assert!(!s.overflow && !s.multiple_programs);
    assert_eq!(s.first_units_total(), s.first_inclusive_work);
    let total = s.inventory_work.ok_or("Missing inventory")?
        + s.completed_header_work.ok_or("Missing header")?
        + s.companion_header_work.ok_or("Missing companion header")?
        + s.binding_exclusive_first().ok_or("Missing binding")?
        + s.first_inclusive_work.ok_or("Missing FIRST")?
        + s.collector_outer_work().ok_or("Missing outer")?;
    assert_eq!(Some(total), s.collect_work);
    Ok(())
}

#[test]
fn collector_full_parity_and_partial_zero_short_exact_conservation() -> Result<(), String> {
    let (grammar, program) = fixture()?;
    let original = serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?;
    let mut baseline =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut remaining = 10_000;
    baseline.collect(&grammar, &program, &mut remaining, None)?;
    let exact = 10_000 - remaining;
    for allowance in [0, 1, exact - 1, exact, 10_000] {
        let observation = PredictionObservation::new();
        let mut cold =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let mut observed =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let (mut a, mut b) = (allowance, allowance);
        let expected = cold.collect(&grammar, &program, &mut a, None);
        let actual = observed.collect(
            &grammar,
            &program,
            &mut b,
            Some(context(&observation, &grammar)?),
        );
        println!(
            "REUSE_COLLECTOR_WHOLE {:?}",
            (&expected, &actual, a, b, observation.first_reuse_work())
        );
        assert_eq!(expected, actual);
        assert_eq!(a, b);
        assert_eq!(cold.records, observed.records);
        assert_eq!(cold.logical_bytes, observed.logical_bytes);
        assert_eq!(cold.failed, observed.failed);
        assert_eq!(actual.is_ok(), allowance >= exact);
        let s = snapshot(&observation)?;
        assert!(s.finished);
        assert_eq!(s.returned_error, actual.is_err());
        assert_eq!(s.collect_work, Some(allowance - b));
        conserved(&s)?;
        if allowance == exact {
            assert_eq!(b, 0);
        }
    }
    assert_eq!(
        serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?,
        original
    );
    Ok(())
}

#[test]
fn complete_compiler_result_none_observed_and_cold_mode_parity() -> Result<(), String> {
    use std::collections::BTreeSet;
    for nullable in [false, true] {
        for disable in [false, true] {
            let (mut grammar, _) = fixture()?;
            if nullable {
                grammar
                    .rules
                    .get_mut("Target")
                    .ok_or("Missing Target")?
                    .body
                    .cardinality = '?';
            }
            let before = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
            let observation = PredictionObservation::new();
            let mut plain = RuntimeGuardedAdmission::new_with_policy(
                RuntimeGuardedPolicy::StrictCompletionScopes,
            );
            let mut observed = RuntimeGuardedAdmission::new_with_policy(
                RuntimeGuardedPolicy::StrictCompletionScopes,
            );
            plain.disable_completed_reuse = disable;
            observed.disable_completed_reuse = disable;
            let a = ParserPrediction::compile_runtime_guarded_linear_repeats(
                &grammar,
                &BTreeSet::new(),
                None,
                None,
                &mut plain,
            );
            let b = ParserPrediction::compile_runtime_guarded_linear_repeats(
                &grammar,
                &BTreeSet::new(),
                Some(context(&observation, &grammar)?),
                None,
                &mut observed,
            );
            let whole_a = serde_json::to_value(&a).map_err(|e| e.to_string())?;
            let whole_b = serde_json::to_value(&b).map_err(|e| e.to_string())?;
            println!(
                "REUSE_WHOLE_COMPILER {}",
                serde_json::json!({"nullable":nullable,"cold":disable,"plain":whole_a,"observed":whole_b})
            );
            assert_eq!(whole_a, whole_b);
            assert_eq!(a.is_ok(), !nullable);
            assert_eq!(plain.records, observed.records);
            assert_eq!(plain.logical_bytes, observed.logical_bytes);
            assert_eq!(plain.failed, observed.failed);
            assert_eq!(
                serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
                before
            );
        }
    }
    Ok(())
}

fn shared() -> Result<(Grammar, crate::grammar::Expr), String> {
    let (mut grammar, _) = fixture()?;
    let mut rule = grammar.rules.get("Target").ok_or("Missing Target")?.clone();
    let leaf = rule.body.clone();
    let mut action = leaf.clone();
    action.kind = Kind::Action("Target".into());
    let mut items = vec![action; 200];
    items.push(leaf.clone());
    rule.body.kind = Kind::Sequence(items);
    grammar.rules.insert("Shared".into(), rule);
    let source = grammar
        .declaration_sources
        .get("Target")
        .ok_or("Missing source")?
        .clone();
    grammar.declaration_sources.insert("Shared".into(), source);
    let mut call = leaf.clone();
    call.kind = Kind::Call("Shared".into());
    let mut root = leaf;
    root.kind = Kind::Alternative(vec![call; 24]);
    Ok((grammar, root))
}
fn query<'g>(
    grammar: &'g Grammar,
    root: &crate::grammar::Expr,
    observation: &PredictionObservation,
    work: &mut usize,
    completed: &mut Completed<'g>,
) -> Result<(), String> {
    let before = *work;
    observation.reuse_event(Event::FirstQuery, 1);
    let result = first_sets::analyze_with_completed_observed(
        root,
        grammar,
        0,
        work,
        completed,
        Some(observation),
    );
    observation.reuse_window(Window::First, before, *work);
    result.map(|_| ())
}

#[test]
fn real_hits_clones_publications_and_every_paid_category_conserve() -> Result<(), String> {
    let (grammar, root) = shared()?;
    let observation = PredictionObservation::new();
    start(&observation, &grammar, 10_000)?;
    let mut work = 10_000;
    let before = work;
    let mut completed = Completed::new(&grammar, &mut work)?;
    observation.reuse_window(Window::CompletedHeader, before, work);
    completed.observe_retained(&observation);
    query(&grammar, &root, &observation, &mut work, &mut completed)?;
    let s = snapshot(&observation)?;
    assert_eq!(s.first_units_total(), s.first_inclusive_work);
    assert_eq!(s.completed_header_work, Some(3));
    assert_eq!(
        s.first_inclusive_work.and_then(|n| n.checked_add(3)),
        Some(10_000 - work)
    );
    assert_eq!(s.events[Event::Hit as usize], Some(23));
    assert_eq!(s.events[Event::Publication as usize], Some(1));
    assert_eq!(s.events[Event::ClonedSymbols as usize], Some(24));
    assert_eq!(
        s.events[Event::ClonedStringBytes as usize],
        Some(24 * "literal:x".len())
    );
    for charge in [
        Charge::CanonicalVisit,
        Charge::AffinityAndState,
        Charge::VisitPeakUpdates,
        Charge::SourceKeyAndActiveChecks,
        Charge::OrderedMap,
        Charge::FrameRetentionAndCapacity,
        Charge::SummaryInspectionAndClone,
        Charge::PublicationAndPeakTransfer,
    ] {
        assert!(s.first_units[charge as usize].is_some_and(|n| n > 0));
    }
    assert!(
        s.retained_cache_bytes
            .is_some_and(|n| n > 128 && n <= 1_048_576)
    );
    let mut cold = 10_000;
    let expected = first_sets::analyze_with_remaining(&root, &grammar, 0, &mut cold)?;
    let mut warm = 10_000;
    let mut cache = Completed::new(&grammar, &mut warm)?;
    let actual = first_sets::analyze_with_completed(&root, &grammar, 0, &mut warm, &mut cache)?;
    assert_eq!(expected, actual);
    assert_eq!(warm, work);
    Ok(())
}

#[test]
fn real_missing_foreign_active_and_depth_fallbacks_are_passive() -> Result<(), String> {
    for mode in 0..4 {
        let (mut grammar, mut root) = shared()?;
        if mode == 0 {
            grammar.declaration_sources.remove("Shared");
        }
        if mode == 2 {
            grammar
                .rules
                .get_mut("Shared")
                .ok_or("Missing Shared")?
                .body
                .kind = Kind::Call("Shared".into());
        }
        if mode == 3 {
            root = grammar
                .rules
                .get("Target")
                .ok_or("Missing Target")?
                .body
                .clone();
            for _ in 0..66 {
                let child = root.clone();
                root.kind = Kind::Sequence(vec![child]);
            }
        }
        let foreign = grammar.clone();
        let observation = PredictionObservation::new();
        start(&observation, &grammar, 10_000)?;
        let (mut a, mut b) = (10_000, 10_000);
        let owner = if mode == 1 { &foreign } else { &grammar };
        let mut plain = Completed::new(owner, &mut a)?;
        let mut observed = Completed::new(owner, &mut b)?;
        let expected = first_sets::analyze_with_completed(&root, &grammar, 0, &mut a, &mut plain);
        let before = b;
        let actual = first_sets::analyze_with_completed_observed(
            &root,
            &grammar,
            0,
            &mut b,
            &mut observed,
            Some(&observation),
        );
        observation.reuse_window(Window::First, before, b);
        assert_eq!(expected, actual);
        assert_eq!(a, b);
        let s = snapshot(&observation)?;
        assert_eq!(s.first_units_total(), s.first_inclusive_work);
        match mode {
            0 => assert!(s.events[Event::MissingSourceOrRange as usize].is_some_and(|n| n > 0)),
            1 => assert_eq!(s.events[Event::ForeignGraph as usize], Some(1)),
            2 => {
                assert!(actual.is_err());
                assert!(s.events[Event::ActiveCall as usize].is_some_and(|n| n > 0));
                assert_eq!(s.events[Event::Publication as usize], Some(0));
            }
            _ => {
                assert!(actual.is_err());
                assert_eq!(s.last_syntax_depth, Some(65));
                assert!(b > 0);
            }
        }
    }
    Ok(())
}

#[test]
fn failed_partial_publication_and_hit_never_fabricate_success() -> Result<(), String> {
    let (grammar, root) = shared()?;
    let mut reached_partial = false;
    for allowance in 3..900 {
        let observation = PredictionObservation::new();
        start(&observation, &grammar, allowance)?;
        let (mut a, mut b) = (allowance, allowance);
        let mut plain = Completed::new(&grammar, &mut a)?;
        let mut observed = Completed::new(&grammar, &mut b)?;
        let expected = first_sets::analyze_with_completed(&root, &grammar, 0, &mut a, &mut plain);
        let before = b;
        let actual = first_sets::analyze_with_completed_observed(
            &root,
            &grammar,
            0,
            &mut b,
            &mut observed,
            Some(&observation),
        );
        observation.reuse_window(Window::First, before, b);
        assert_eq!(expected, actual);
        assert_eq!(a, b);
        assert!(actual.is_err());
        let s = snapshot(&observation)?;
        assert_eq!(s.first_units_total(), s.first_inclusive_work);
        assert!(s.events[Event::Publication as usize].is_some_and(|n| n <= 1));
        assert!(s.events[Event::Hit as usize].is_some_and(|n| n <= 23));
        reached_partial |=
            s.first_units[Charge::SummaryInspectionAndClone as usize].is_some_and(|n| n > 0);
    }
    assert!(reached_partial);
    Ok(())
}

#[test]
fn passive_reset_unknown_overflow_and_first_error_are_independent() -> Result<(), String> {
    let (grammar, _) = fixture()?;
    let observation = PredictionObservation::new();
    assert!(observation.first_reuse_work().is_none());
    let source = SourceId::for_text("u".repeat(513), "x");
    observation.reuse_begin(RunStage::Executable, &source, 10_000);
    observation.reuse_event(Event::Hit, usize::MAX);
    observation.reuse_event(Event::Hit, 1);
    let s = snapshot(&observation)?;
    assert!(s.capture_unavailable && s.source.is_none() && s.overflow);
    assert_eq!(s.events[Event::Hit as usize], None);
    assert_eq!(s.events[Event::Publication as usize], Some(0));
    assert_eq!(s.last_call_depth, None);
    assert_eq!(s.retained_cache_bytes, None);
    observation.returned_error(
        &source,
        RunStage::Executable,
        Phase::RuntimeCandidateNonnullableFirst,
        None,
        Some("first"),
    );
    let first = observation
        .first_failure()
        .ok_or("Missing real first record")?;
    observation.returned_error(
        &source,
        RunStage::Executable,
        Phase::RuntimeCandidateBinding,
        None,
        Some("outer"),
    );
    assert_eq!(observation.first_failure(), Some(first));
    observation.reuse_begin(RunStage::Parent, &source, 9000);
    assert!(snapshot(&observation)?.multiple_programs);
    assert_eq!(snapshot(&observation)?.first_units_total(), None);
    observation.reset();
    assert!(observation.first_reuse_work().is_none() && observation.first_failure().is_none());
    start(&observation, &grammar, 10_000)?;
    observation.reuse_charge(Charge::CanonicalVisit);
    observation.reuse_end(9999, true);
    observation.reuse_charge(Charge::CanonicalVisit);
    let ended = snapshot(&observation)?;
    assert!(ended.finished && ended.returned_error);
    assert_eq!(ended.first_units[Charge::CanonicalVisit as usize], Some(1));
    Ok(())
}

#[test]
fn map_cost_opt_in_keeps_whole_compiler_result_and_first_error_order() -> Result<(), String> {
    use std::collections::BTreeSet;
    for broken in [false, true] {
        let (mut grammar, _) = fixture()?;
        if broken {
            grammar
                .rules
                .get_mut("Target")
                .ok_or("Missing Target")?
                .body
                .kind = Kind::ScalarReference {
                target: "Target".into(),
                rule: "Missing".into(),
            };
        }
        let before = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
        let (off, on) = (PredictionObservation::new(), PredictionObservation::new());
        assert!(on.enable_first_map_cost_diagnostic());
        // The public observed facade resets measurements before compilation;
        // this explicit reset checks that diagnostic configuration survives it.
        on.reset();
        assert!(on.first_map_cost().is_none());
        let mut a =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let mut b =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let expected = ParserPrediction::compile_runtime_guarded_linear_repeats(
            &grammar,
            &BTreeSet::new(),
            Some(context(&off, &grammar)?),
            None,
            &mut a,
        );
        let actual = ParserPrediction::compile_runtime_guarded_linear_repeats(
            &grammar,
            &BTreeSet::new(),
            Some(context(&on, &grammar)?),
            None,
            &mut b,
        );
        let whole_off = serde_json::to_value(&expected).map_err(|e| e.to_string())?;
        let whole_on = serde_json::to_value(&actual).map_err(|e| e.to_string())?;
        println!(
            "MAP_COST_WHOLE_COMPILER {}",
            serde_json::json!({
                "broken":broken,"off":whole_off,"on":whole_on,
                "old_observation_off":format!("{:?}",off.first_reuse_work()),
                "old_observation_on":format!("{:?}",on.first_reuse_work()),
                "diagnostic":format!("{:?}",on.first_map_cost()),
                "first_error_off":format!("{:?}",off.first_failure()),
                "first_error_on":format!("{:?}",on.first_failure()),
                "scope":"whole ParserPrediction compiler result; synthetic control, not original551 admission"
            })
        );
        assert_eq!(whole_off, whole_on);
        assert_eq!(actual.is_err(), broken);
        assert_eq!(a.records, b.records);
        assert_eq!(a.logical_bytes, b.logical_bytes);
        assert_eq!(a.failed, b.failed);
        assert_eq!(off.first_reuse_work(), on.first_reuse_work());
        assert_eq!(off.first_failure(), on.first_failure());
        assert_eq!(off.coverage(), on.coverage());
        assert!(off.first_map_cost().is_none());
        let map = on
            .first_map_cost()
            .ok_or("Missing optional map diagnostic")?;
        assert!(map.finished && !map.multiple_programs && !map.overflow);
        assert_eq!(
            map.total_paid_units(),
            snapshot(&on)?.first_units[Charge::OrderedMap as usize]
        );
        if broken {
            let first = on
                .first_failure()
                .ok_or("Missing first canonical failure")?;
            let c = context(&on, &grammar)?;
            on.returned_error(
                c.entry,
                c.stage,
                crate::grammar::admission_observation::Phase::ParserRuleStates,
                None,
                None,
            );
            let c = context(&off, &grammar)?;
            off.returned_error(
                c.entry,
                c.stage,
                crate::grammar::admission_observation::Phase::ParserRuleStates,
                None,
                None,
            );
            assert_eq!(on.first_failure(), Some(first));
            assert_eq!(off.first_failure(), Some(first));
        }
        assert_eq!(
            serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
            before
        );
    }
    // Direct collect exposes every caller-owned remaining balance, including
    // zero/one-short failures, while both observers remain otherwise identical.
    let (grammar, program) = fixture()?;
    let mut base =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut remaining = 10_000;
    base.collect(&grammar, &program, &mut remaining, None)?;
    let exact = 10_000 - remaining;
    for allowance in [0, 1, exact - 1, exact, 10_000] {
        let (off, on) = (PredictionObservation::new(), PredictionObservation::new());
        assert!(on.enable_first_map_cost_diagnostic());
        let mut a =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let mut b =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let (mut wa, mut wb) = (allowance, allowance);
        let expected = a.collect(&grammar, &program, &mut wa, Some(context(&off, &grammar)?));
        let actual = b.collect(&grammar, &program, &mut wb, Some(context(&on, &grammar)?));
        println!(
            "MAP_COST_COLLECT_WHOLE {:?}",
            (allowance, &expected, &actual, wa, wb, on.first_map_cost())
        );
        assert_eq!(expected, actual);
        assert_eq!(wa, wb);
        assert_eq!(a.records, b.records);
        assert_eq!(a.logical_bytes, b.logical_bytes);
        assert_eq!(a.failed, b.failed);
        assert_eq!(off.first_failure(), on.first_failure());
        assert_eq!(off.first_reuse_work(), on.first_reuse_work());
        assert_eq!(
            on.first_map_cost().and_then(|s| s.total_paid_units()),
            snapshot(&on)?.first_units[Charge::OrderedMap as usize]
        );
    }
    Ok(())
}

#[test]
fn map_cost_six_cells_include_real_hits_and_partial_paid_errors() -> Result<(), String> {
    // Full storage still exercises all six original map fee cells.
    let (mut grammar, _) = fixture()?;
    let leaf = grammar.rules.get("Target").ok_or("Missing Target")?.clone();
    grammar.rules.insert("Leaf".into(), leaf);
    let span = grammar
        .declaration_sources
        .get("Target")
        .ok_or("Missing source")?
        .clone();
    grammar.declaration_sources.insert("Leaf".into(), span);
    let mut leaf_call = grammar
        .rules
        .get("Target")
        .ok_or("Missing Target")?
        .body
        .clone();
    leaf_call.kind = Kind::Call("Leaf".into());
    grammar
        .rules
        .get_mut("Target")
        .ok_or("Missing Target")?
        .body
        .kind = Kind::Alternative(vec![leaf_call.clone(), leaf_call]);
    let mut root = grammar
        .rules
        .get("Target")
        .ok_or("Missing Target")?
        .body
        .clone();
    root.kind = Kind::Call("Target".into());
    let before = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
    for allowance in [7, 10_000] {
        let (off, on) = (PredictionObservation::new(), PredictionObservation::new());
        assert!(on.enable_first_map_cost_diagnostic());
        // Independent query-only budgets after identical paid empty-cache
        // construction. No reset or alternative pool is used by the original.
        let (mut seed_a, mut seed_b) = (10_000, 10_000);
        let mut a = Completed::new(&grammar, &mut seed_a)?;
        let mut b = Completed::new(&grammar, &mut seed_b)?;
        assert_eq!(seed_a, seed_b);
        start(&off, &grammar, allowance)?;
        start(&on, &grammar, allowance)?;
        let (mut wa, mut wb) = (allowance, allowance);
        let expected = first_sets::analyze_with_storage_observed(
            &root,
            &grammar,
            0,
            &mut wa,
            &mut a,
            Some(&off),
        );
        let actual = first_sets::analyze_with_storage_observed(
            &root,
            &grammar,
            0,
            &mut wb,
            &mut b,
            Some(&on),
        );
        off.reuse_window(Window::First, allowance, wa);
        on.reuse_window(Window::First, allowance, wb);
        off.reuse_end(wa, expected.is_err());
        on.reuse_end(wb, actual.is_err());
        let map = on.first_map_cost().ok_or("Missing six cells")?;
        println!(
            "MAP_COST_CANONICAL_WHOLE {:?}",
            (allowance, &expected, &actual, wa, wb, &map, snapshot(&on)?)
        );
        assert_eq!(expected, actual);
        assert_eq!(wa, wb);
        assert_eq!(off.first_reuse_work(), on.first_reuse_work());
        assert_eq!(
            map.total_paid_units(),
            snapshot(&on)?.first_units[Charge::OrderedMap as usize]
        );
        if allowance == 7 {
            // Affinity3 + Visit1 + visit-meter1 + canonical-owner1 precede
            // the first real map debit; its key-byte debit then fails at zero.
            assert!(actual.is_err());
            assert_eq!(wb, 0);
            assert_eq!(map.total_paid_units(), Some(1));
            assert_eq!(map.attempted_operations[3], Some(1));
            assert_eq!(map.completed_fee_windows[3], Some(0));
            let mut later = 10_000;
            assert!(
                first_sets::analyze_with_storage_observed(
                    &root,
                    &grammar,
                    0,
                    &mut later,
                    &mut b,
                    Some(&on)
                )
                .is_err()
            );
        } else {
            assert!(actual.is_ok());
            assert!(map.paid_units.into_iter().all(|v| v.is_some_and(|n| n > 0)));
            assert_eq!(map.attempted_operations, map.completed_fee_windows);
            assert_eq!(map.hits_by_body, [Some(1), Some(0)]);
        }
        assert_eq!(
            serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
            before
        );
    }
    Ok(())
}
