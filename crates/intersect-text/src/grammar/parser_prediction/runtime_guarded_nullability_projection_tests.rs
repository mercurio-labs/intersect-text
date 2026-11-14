// Same canonical traversal; unit storage is explicitly NOT a full FIRST set.
use crate::grammar::admission_observation::FirstReuseEvent as Event;
use crate::grammar::admission_observation::FirstReuseWindow as Window;
use crate::grammar::Expr;
use crate::grammar::Kind;
use crate::grammar::first_sets as first;
use first::completed_reuse::{Completed, NullabilityCompleted};

fn capacity_fixture() -> Result<Grammar, String> {
    let source = SourceId::for_text("memory:prepaid-first-capacity", "x");
    serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"Capacity","entry":"Root",
        "rules":{"Root":{"class":"Root","body":{"cardinality":"1","original_range":{"start":0,"end":1},"kind":{"Literal":"x"}}}},
        "declaration_sources":{"Root":{"source":source,"start":0,"end":1}}
    })).map_err(|e| e.to_string())
}
fn node(grammar: &Grammar, kind: Kind) -> Expr {
    let mut value = grammar.rules["Root"].body.clone();
    value.kind = kind;
    value
}
fn add(grammar: &mut Grammar, name: &str, body: Expr) {
    let mut rule = grammar.rules["Root"].clone();
    rule.body = body;
    grammar.rules.insert(name.into(), rule);
    grammar
        .declaration_sources
        .insert(name.into(), grammar.declaration_sources["Root"].clone());
}
fn branching() -> Result<(Grammar, Expr), String> {
    let mut grammar = capacity_fixture()?;
    let mut branches = Vec::new();
    for branch in 0..4 {
        for depth in (0..8).rev() {
            let body = if depth == 7 {
                node(&grammar, Kind::Literal(format!("token{branch}")))
            } else {
                node(&grammar, Kind::Call(format!("B{branch}D{}", depth + 1)))
            };
            add(&mut grammar, &format!("B{branch}D{depth}"), body);
        }
        branches.push(node(&grammar, Kind::Call(format!("B{branch}D0"))));
    }
    // The final repeated branch must reuse the completed body, not an active one.
    branches.push(node(&grammar, Kind::Call("B0D0".into())));
    let root = node(&grammar, Kind::Alternative(branches));
    Ok((grammar, root))
}

fn begin(trace: &PredictionObservation, grammar: &Grammar, remaining: usize) -> Result<(), String> {
    trace.reuse_begin(
        RunStage::Executable,
        &grammar
            .declaration_sources
            .values()
            .next()
            .ok_or("Missing source")?
            .source,
        remaining,
    );
    Ok(())
}

#[test]
fn whole_real_fixture_keeps_full2120_and_pays_less_for_nullable_projection() -> Result<(), String> {
    let (grammar, root) = branching()?;
    let before = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
    // Exercise the original cold Full observer API on complete success and a
    // real canonical error. These separate budgets do not enter the cached
    // 2120 baseline or projected-cost measurement below.
    for (label, expression) in [
        ("complete", root.clone()),
        (
            "unknown_scalar",
            node(
                &grammar,
                Kind::ScalarReference {
                    target: "Root".into(),
                    rule: "Missing".into(),
                },
            ),
        ),
    ] {
        let (mut plain_work, mut observed_work) = (10_000, 10_000);
        let plain = first::analyze_with_remaining(&expression, &grammar, 0, &mut plain_work);
        let cold_trace = PredictionObservation::new();
        begin(&cold_trace, &grammar, observed_work)?;
        let start = observed_work;
        let observed = first::analyze_with_remaining_observed(
            &expression,
            &grammar,
            0,
            &mut observed_work,
            &cold_trace,
        );
        cold_trace.reuse_window(Window::First, start, observed_work);
        cold_trace.reuse_end(observed_work, observed.is_err());
        println!(
            "NULLABILITY_FULL_COLD_OBSERVER_WHOLE {:?}",
            (
                label,
                &plain,
                &observed,
                plain_work,
                observed_work,
                cold_trace.first_reuse_work()
            )
        );
        assert_eq!(
            plain, observed,
            "whole Full result includes its actual symbol set or error"
        );
        assert_eq!(
            plain_work, observed_work,
            "passive observer does not debit work"
        );
        assert_eq!(plain.is_ok(), label == "complete");
        let snapshot = cold_trace
            .first_reuse_work()
            .ok_or("Missing cold Full trace")?;
        assert_eq!(snapshot.first_units_total(), snapshot.first_inclusive_work);
        assert_eq!(snapshot.first_inclusive_work, Some(10_000 - observed_work));
    }
    let (mut a, mut b) = (10_000, 10_000);
    let mut full = Completed::new(&grammar, &mut a)?;
    let mut projected = NullabilityCompleted::new(&grammar, &mut b)?;
    let trace = PredictionObservation::new();
    begin(&trace, &grammar, b)?;
    let start = b;
    let old = first::analyze_with_completed(&root, &grammar, 0, &mut a, &mut full);
    let actual = first::analyze_with_storage_observed(
        &root,
        &grammar,
        0,
        &mut b,
        &mut projected,
        Some(&trace),
    );
    trace.reuse_window(Window::First, start, b);
    trace.reuse_end(b, actual.is_err());
    println!(
        "NULLABILITY_PROJECTION_WHOLE {}",
        serde_json::json!({
            "projection":"nullable_only","full_first":old,"projected_nullable":actual.as_ref().map(|s|s.1),
            "full_total_units":10000-a,"projected_total_units":10000-b,
            "remaining":b,"observation":format!("{:?}",trace.first_reuse_work()),
            "scope":"same complete real branching fixture; not original551 admission"
        })
    );
    assert_eq!(
        10_000 - a,
        2120,
        "default full-FIRST cost and assertions remain unchanged"
    );
    assert_eq!(old.as_ref().map(|s| s.1), actual.as_ref().map(|s| s.1));
    assert!(old.is_ok() && actual.is_ok());
    assert_eq!(old.as_ref().map(|s| s.0.len()), Ok(4));
    assert!(
        10_000 - b < 10_000 - a,
        "compare whole actual workers, never mirrored fees"
    );
    let snapshot = trace
        .first_reuse_work()
        .ok_or("Missing real projection trace")?;
    // Four one-step terminal leaves no longer publish cache entries.
    assert_eq!(snapshot.events[Event::Publication as usize], Some(28));
    assert_eq!(snapshot.events[Event::Hit as usize], Some(1));
    assert_eq!(snapshot.events[Event::ClonedSymbols as usize], Some(0));
    assert_eq!(snapshot.events[Event::ClonedStringBytes as usize], Some(0));
    assert_eq!(snapshot.first_units_total(), snapshot.first_inclusive_work);
    assert_eq!(
        serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
        before
    );
    Ok(())
}

#[test]
fn genuine_strict_binding_whole_result_and_observer_parity_use_nullable_readout()
-> Result<(), String> {
    for (policy, nullable) in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ]
    .into_iter()
    .flat_map(|policy| [false, true].map(|nullable| (policy, nullable)))
    {
        let (mut grammar, program) = fixture()?;
        if nullable {
            grammar
                .rules
                .get_mut("Target")
                .ok_or("Missing target")?
                .body
                .cardinality = '?';
        }
        let original = serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?;
        let (mut a, mut b, mut c) = (10_000, 10_000, 10_000);
        let (mut la, mut lb, mut lc) = (0, 0, 0);
        let mut full = Completed::new(&grammar, &mut a)?;
        let mut plain = NullabilityCompleted::new(&grammar, &mut b)?;
        let mut observed = NullabilityCompleted::new(&grammar, &mut c)?;
        let trace = PredictionObservation::new();
        begin(&trace, &grammar, c)?;
        let old = binding_with_completed(
            &grammar,
            &program,
            "rule/Root/repeat",
            &mut a,
            &mut la,
            policy,
            (None, Some(&mut full)),
        );
        let actual = binding_with_storage::<()>(
            &grammar,
            &program,
            "rule/Root/repeat",
            &mut b,
            &mut lb,
            policy,
            (None, Some(&mut plain), None),
        );
        let raw = binding_with_storage::<()>(
            &grammar,
            &program,
            "rule/Root/repeat",
            &mut c,
            &mut lc,
            policy,
            (Some(context(&trace, &grammar)?), Some(&mut observed), None),
        );
        let captured = observe_candidate(
            raw,
            &grammar,
            &program,
            Some(context(&trace, &grammar)?),
            (Phase::RuntimeCandidateBinding, Some("rule/Root/repeat")),
            c,
        );
        println!(
            "NULLABILITY_BINDING_WHOLE {:?}",
            (&old, &actual, &captured, a, b, c, trace.first_failure())
        );
        assert_eq!(old, actual);
        assert_eq!(actual, captured);
        assert_eq!((b, lb), (c, lc));
        assert_eq!(la, lb);
        assert_eq!(actual.is_ok(), !nullable);
        if nullable {
            assert!(
                actual
                    .as_ref()
                    .is_err_and(|e| e.contains("requires a nonnullable operand"))
            );
            assert_eq!(
                trace
                    .first_failure()
                    .ok_or("Missing actual nullable error")?
                    .phase,
                Phase::RuntimeCandidateBinding
            );
        } else {
            assert!(actual.as_ref().is_ok_and(|b| b.is_some()));
            assert!(trace.first_failure().is_none());
        }
        assert_eq!(
            serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?,
            original
        );
    }
    Ok(())
}

#[test]
fn canonical_cardinality_alternative_errors_and_active_depth_are_not_skipped() -> Result<(), String>
{
    let grammar = capacity_fixture()?;
    let literal = node(&grammar, Kind::Literal(String::new()));
    for cardinality in ['1', '?', '*', '+'] {
        let mut value = literal.clone();
        value.cardinality = cardinality;
        let mut a = 10_000;
        let mut b = 10_000;
        let full = first::analyze_with_remaining(&value, &grammar, 0, &mut a);
        let projected = first::analyze_remaining_storage::<()>(&value, &grammar, 0, &mut b, None);
        assert_eq!(full.map(|s| s.1), projected.map(|s| s.1));
    }
    let action = node(&grammar, Kind::Action("Root".into()));
    let unknown = node(
        &grammar,
        Kind::ScalarReference {
            target: "Root".into(),
            rule: "Missing".into(),
        },
    );
    // These are real canonical expression families, not a second nullable evaluator.
    // Keep expected language behavior explicit alongside whole old/new Results.
    for (label, kind, expected) in [
        (
            "assign",
            Kind::Assign {
                feature: "value".into(),
                many: false,
                value: Box::new(literal.clone()),
            },
            false,
        ),
        (
            "boolean_assign",
            Kind::BooleanAssign {
                feature: "present".into(),
                value: Box::new(action.clone()),
            },
            true,
        ),
        (
            "predicate",
            Kind::SyntacticPredicate(Box::new(action.clone())),
            true,
        ),
        ("reference", Kind::Reference("Any".into()), false),
        (
            "capture",
            Kind::Capture {
                class: "Root".into(),
                feature: "value".into(),
                many: false,
            },
            true,
        ),
        (
            "unknown_call_is_terminal",
            Kind::Call("UnknownToken".into()),
            false,
        ),
        ("empty_sequence", Kind::Sequence(Vec::new()), true),
        ("empty_alternative", Kind::Alternative(Vec::new()), false),
    ] {
        let value = node(&grammar, kind);
        let (mut a, mut b) = (10_000, 10_000);
        let full = first::analyze_with_remaining(&value, &grammar, 0, &mut a);
        let projected = first::analyze_remaining_storage::<()>(&value, &grammar, 0, &mut b, None);
        println!(
            "NULLABILITY_FAMILY_WHOLE {:?}",
            (label, &full, &projected, a, b)
        );
        assert_eq!(full.as_ref().map(|s| s.1), projected.as_ref().map(|s| s.1));
        assert_eq!(projected, Ok(((), expected)));
        assert_eq!(a, b, "the uncached canonical visits are unchanged");
    }
    // Both workers skip the same nonnullable prefix tail, and both visit a
    // nullable prefix tail. There is no new bool short-circuit for alternatives.
    for (label, first, expected) in [
        ("nonnullable_skips_unknown", literal.clone(), Ok(false)),
        (
            "nullable_reaches_unknown",
            action.clone(),
            Err("Unknown datatype reference rule".to_string()),
        ),
    ] {
        let value = node(&grammar, Kind::Sequence(vec![first, unknown.clone()]));
        let (mut a, mut b) = (10_000, 10_000);
        let full = first::analyze_with_remaining(&value, &grammar, 0, &mut a);
        let projected = first::analyze_remaining_storage::<()>(&value, &grammar, 0, &mut b, None);
        println!(
            "NULLABILITY_SEQUENCE_WHOLE {:?}",
            (label, &full, &projected, a, b)
        );
        assert_eq!(
            full.map(|s| s.1),
            projected.as_ref().map(|s| s.1).map_err(Clone::clone)
        );
        assert_eq!(projected.map(|s| s.1), expected);
        assert_eq!(a, b);
        if label == "nonnullable_skips_unknown" {
            assert_eq!(10_000 - b, 2, "exactly sequence and literal visited");
        }
    }
    // Distinguishable errors prove canonical first-winner ordering, rather than
    // comparing two unknown references which happen to have the same error text.
    let mut too_deep = action.clone();
    for _ in 0..65 {
        too_deep = node(&grammar, Kind::SyntacticPredicate(Box::new(too_deep)));
    }
    for (children, expected) in [
        (
            vec![unknown.clone(), too_deep.clone()],
            "Unknown datatype reference rule",
        ),
        (
            vec![too_deep, unknown.clone()],
            "Grammar prediction budget exceeded",
        ),
    ] {
        let value = node(&grammar, Kind::Sequence(children));
        let (mut a, mut b) = (10_000, 10_000);
        let full = first::analyze_with_remaining(&value, &grammar, 0, &mut a);
        let projected = first::analyze_remaining_storage::<()>(&value, &grammar, 0, &mut b, None);
        println!(
            "NULLABILITY_ERROR_ORDER_WHOLE {:?}",
            (&full, &projected, a, b)
        );
        assert_eq!(full.as_ref().map(|s| s.1), projected.as_ref().map(|s| s.1));
        assert_eq!(projected, Err(expected.to_string()));
        assert!(
            a > 0 && b > 0,
            "depth/unknown failure is not work exhaustion"
        );
    }
    let alternatives = node(&grammar, Kind::Alternative(vec![action, unknown]));
    let (mut a, mut b) = (10_000, 10_000);
    let full = first::analyze_with_remaining(&alternatives, &grammar, 0, &mut a);
    let projected =
        first::analyze_remaining_storage::<()>(&alternatives, &grammar, 0, &mut b, None);
    println!(
        "NULLABILITY_CANONICAL_ERROR {:?}",
        (&full, &projected, a, b)
    );
    assert_eq!(full.as_ref().map(|s| s.1), projected.as_ref().map(|s| s.1));
    assert!(
        projected
            .as_ref()
            .is_err_and(|e| e == "Unknown datatype reference rule")
    );
    let mut cycle = grammar.clone();
    cycle.rules.get_mut("Root").ok_or("Missing root")?.body.kind = Kind::Call("Root".into());
    let call = node(&cycle, Kind::Call("Root".into()));
    let (mut a, mut b) = (10_000, 10_000);
    let mut full = Completed::new(&cycle, &mut a)?;
    let mut projected = NullabilityCompleted::new(&cycle, &mut b)?;
    let old = first::analyze_with_completed(&call, &cycle, 0, &mut a, &mut full);
    let actual =
        first::analyze_with_storage_observed(&call, &cycle, 0, &mut b, &mut projected, None);
    assert_eq!(old.map(|s| s.1), actual.map(|s| s.1));
    assert!(
        a > 0 && b > 0,
        "independent depth bound, not exhausted work"
    );
    for depth in [63, 64, 65] {
        let (mut a, mut b) = (10_000, 10_000);
        let old = first::analyze_with_remaining(&literal, &grammar, depth, &mut a);
        let actual =
            first::analyze_remaining_storage::<()>(&literal, &grammar, depth, &mut b, None);
        assert_eq!(old.map(|s| s.1), actual.map(|s| s.1));
    }
    Ok(())
}

#[test]
fn projected_zero_short_exact_quota_and_foreign_fallback_keep_actual_results() -> Result<(), String>
{
    let (grammar, root) = branching()?;
    let mut work = 10_000;
    let mut owner = NullabilityCompleted::new(&grammar, &mut work)?;
    let result =
        first::analyze_with_storage_observed(&root, &grammar, 0, &mut work, &mut owner, None);
    assert!(result.is_ok());
    let exact = 10_000 - work;
    let retention = PredictionObservation::new();
    begin(&retention, &grammar, work)?;
    owner.observe_retained(&retention);
    let retained = retention
        .first_reuse_work()
        .and_then(|s| s.retained_cache_bytes)
        .ok_or("Missing quota")?;
    for allowance in [0, exact - 1, exact] {
        let mut balance = allowance;
        let actual = match NullabilityCompleted::new(&grammar, &mut balance) {
            Err(error) => Err(error),
            Ok(mut owner) => {
                let value = first::analyze_with_storage_observed(
                    &root,
                    &grammar,
                    0,
                    &mut balance,
                    &mut owner,
                    None,
                );
                if value.is_err() {
                    let mut fresh_work = 10_000;
                    assert!(
                        first::analyze_with_storage_observed(
                            &root,
                            &grammar,
                            0,
                            &mut fresh_work,
                            &mut owner,
                            None
                        )
                        .is_err()
                    );
                }
                value.map(|s| s.1)
            }
        };
        println!(
            "NULLABILITY_BOUNDED_WHOLE {:?}",
            (allowance, &actual, balance)
        );
        assert_eq!(actual.is_ok(), allowance == exact);
    }
    // The full result may genuinely need more work for unused symbol storage.
    let mut balance = exact;
    let mut full = Completed::new(&grammar, &mut balance)?;
    let full_result = first::analyze_with_completed(&root, &grammar, 0, &mut balance, &mut full);
    println!(
        "NULLABILITY_FULL_OUTPUT_WORK_BOUNDARY {:?}",
        (exact, &full_result, balance)
    );
    assert!(full_result.is_err());
    for limit in [retained - 1, retained] {
        let mut balance = 10_000;
        let mut owner = NullabilityCompleted::with_limit(&grammar, &mut balance, limit)?;
        let actual = first::analyze_with_storage_observed(
            &root,
            &grammar,
            0,
            &mut balance,
            &mut owner,
            None,
        );
        assert_eq!(actual.is_ok(), limit == retained);
    }
    let mut missing = grammar.clone();
    missing.declaration_sources.remove("B0D0");
    let (mut a, mut b) = (10_000, 10_000);
    let mut full = Completed::new(&missing, &mut a)?;
    let mut projected = NullabilityCompleted::new(&missing, &mut b)?;
    let trace = PredictionObservation::new();
    begin(&trace, &missing, b)?;
    let old = first::analyze_with_completed(&root, &missing, 0, &mut a, &mut full);
    let actual = first::analyze_with_storage_observed(
        &root,
        &missing,
        0,
        &mut b,
        &mut projected,
        Some(&trace),
    );
    assert_eq!(old.map(|s| s.1), actual.map(|s| s.1));
    assert!(
        trace
            .first_reuse_work()
            .and_then(|s| s.events[Event::MissingSourceOrRange as usize])
            .is_some_and(|n| n > 0)
    );
    // A present but out-of-source body range must take the same canonical
    // fallback as absent source metadata; it is never a projected cache proof.
    let mut malformed = grammar.clone();
    let range = malformed
        .rules
        .get_mut("B0D0")
        .ok_or("Missing range fixture rule")?
        .body
        .original_range
        .as_mut()
        .ok_or("Missing body range")?;
    range.start = 2;
    range.end = 3;
    let (mut a, mut b) = (10_000, 10_000);
    let mut full = Completed::new(&malformed, &mut a)?;
    let mut projected = NullabilityCompleted::new(&malformed, &mut b)?;
    let trace = PredictionObservation::new();
    begin(&trace, &malformed, b)?;
    let old = first::analyze_with_completed(&root, &malformed, 0, &mut a, &mut full);
    let actual = first::analyze_with_storage_observed(
        &root,
        &malformed,
        0,
        &mut b,
        &mut projected,
        Some(&trace),
    );
    println!(
        "NULLABILITY_MALFORMED_RANGE_WHOLE {:?}",
        (&old, &actual, a, b, trace.first_reuse_work())
    );
    assert_eq!(old.as_ref().map(|s| s.1), actual.as_ref().map(|s| s.1));
    assert!(actual.is_ok());
    assert!(
        trace
            .first_reuse_work()
            .and_then(|s| s.events[Event::MissingSourceOrRange as usize])
            .is_some_and(|n| n > 0)
    );
    let foreign = grammar.clone();
    let mut balance = 10_000;
    let mut owner = NullabilityCompleted::new(&grammar, &mut balance)?;
    let foreign_result =
        first::analyze_with_storage_observed(&root, &foreign, 0, &mut balance, &mut owner, None);
    let mut cold_work = 10_000;
    let canonical = first::analyze_with_remaining(&root, &foreign, 0, &mut cold_work);
    assert_eq!(foreign_result.map(|s| s.1), canonical.map(|s| s.1));
    assert_ne!(
        std::any::type_name::<Completed<'_>>(),
        std::any::type_name::<NullabilityCompleted<'_>>()
    );
    Ok(())
}

#[test]
fn published_unit_validity_reuse_keeps_full_and_nested_observer_results() -> Result<(), String> {
    use crate::grammar::admission_observation::FirstReuseCharge as Charge;
    for nested in [false, true] {
        let mut grammar = capacity_fixture()?;
        let leaf = node(
            &grammar,
            Kind::Sequence(vec![node(&grammar, Kind::Literal("leaf".into()))]),
        );
        add(&mut grammar, "Leaf", leaf);
        let leaf_call = node(&grammar, Kind::Call("Leaf".into()));
        add(&mut grammar, "Outer", leaf_call.clone());
        let outer_call = node(&grammar, Kind::Call("Outer".into()));
        let query = if nested { &outer_call } else { &leaf_call };
        let before = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
        let (mut a, mut b, mut c) = (10_000, 10_000, 10_000);
        let mut full = Completed::new(&grammar, &mut a)?;
        let mut unit = NullabilityCompleted::new(&grammar, &mut b)?;
        let mut plain = NullabilityCompleted::new(&grammar, &mut c)?;
        let old_prime = first::analyze_with_completed(&leaf_call, &grammar, 0, &mut a, &mut full);
        let unit_prime =
            first::analyze_with_storage_observed(&leaf_call, &grammar, 0, &mut b, &mut unit, None);
        let plain_prime =
            first::analyze_with_storage_observed(&leaf_call, &grammar, 0, &mut c, &mut plain, None);
        println!(
            "PUBLISHED_UNIT_PRIME_WHOLE {:?}",
            (&old_prime, &unit_prime, &plain_prime)
        );
        assert_eq!(
            old_prime.as_ref().map(|s| s.1),
            unit_prime.as_ref().map(|s| s.1)
        );
        assert_eq!(unit_prime, plain_prime);
        assert!(old_prime.is_ok() && unit_prime.is_ok());
        let (full_trace, unit_trace) = (PredictionObservation::new(), PredictionObservation::new());
        begin(&full_trace, &grammar, a)?;
        begin(&unit_trace, &grammar, b)?;
        let (full_start, unit_start) = (a, b);
        let old = first::analyze_with_completed_observed(
            query,
            &grammar,
            0,
            &mut a,
            &mut full,
            Some(&full_trace),
        );
        let actual = first::analyze_with_storage_observed(
            query,
            &grammar,
            0,
            &mut b,
            &mut unit,
            Some(&unit_trace),
        );
        let unobserved =
            first::analyze_with_storage_observed(query, &grammar, 0, &mut c, &mut plain, None);
        full_trace.reuse_window(Window::First, full_start, a);
        full_trace.reuse_end(a, old.is_err());
        unit_trace.reuse_window(Window::First, unit_start, b);
        unit_trace.reuse_end(b, actual.is_err());
        let old_snapshot = full_trace.first_reuse_work().ok_or("Missing Full trace")?;
        let snapshot = unit_trace.first_reuse_work().ok_or("Missing Unit trace")?;
        println!(
            "PUBLISHED_UNIT_VALIDITY_WHOLE {}",
            serde_json::json!({
                "nested":nested,"full":old,"unit":actual,"unobserved":unobserved,
                "full_query_units":full_start-a,"unit_query_units":unit_start-b,
                "full_observation":format!("{old_snapshot:?}"),
                "unit_observation":format!("{snapshot:?}"),
                "scope":"published immutable callee hit; not original551 admission"
            })
        );
        assert!(old.is_ok() && actual.is_ok());
        assert_eq!(old.as_ref().map(|s| s.1), actual.as_ref().map(|s| s.1));
        assert_eq!(actual, unobserved);
        assert_eq!(b, c);
        let old_checks = old_snapshot.first_units[Charge::SourceKeyAndActiveChecks as usize]
            .ok_or("Missing Full validity units")?;
        let checks = snapshot.first_units[Charge::SourceKeyAndActiveChecks as usize]
            .ok_or("Missing Unit validity units")?;
        // The retained Unit validity fast path saves one check per real hit
        // level, now offset by the paid one-tag classification at each level.
        // This is not an optimization claim; all map/depth/clone fees remain.
        assert_eq!(old_checks.checked_sub(checks), Some(0));
        assert_eq!(
            old_snapshot.first_units[Charge::OrderedMap as usize],
            snapshot.first_units[Charge::OrderedMap as usize]
        );
        assert_eq!(snapshot.events[Event::Hit as usize], Some(1));
        assert_eq!(
            snapshot.events[Event::Publication as usize],
            Some(usize::from(nested))
        );
        assert_eq!(snapshot.first_units_total(), snapshot.first_inclusive_work);
        assert_eq!(
            serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
            before
        );
    }
    Ok(())
}

#[test]
fn published_unit_warm_short_budget_is_sticky_and_depth_fallback_still_runs() -> Result<(), String>
{
    let mut grammar = capacity_fixture()?;
    let leaf = node(
        &grammar,
        Kind::Sequence(vec![node(&grammar, Kind::Literal("leaf".into()))]),
    );
    add(&mut grammar, "Leaf", leaf);
    let call = node(&grammar, Kind::Call("Leaf".into()));
    let mut seed_work = 10_000;
    let mut seed = NullabilityCompleted::new(&grammar, &mut seed_work)?;
    let prime =
        first::analyze_with_storage_observed(&call, &grammar, 0, &mut seed_work, &mut seed, None);
    assert!(prime.is_ok());
    let start = seed_work;
    let expected =
        first::analyze_with_storage_observed(&call, &grammar, 0, &mut seed_work, &mut seed, None);
    let needed = start - seed_work;
    assert!(expected.is_ok() && needed > 0);
    // Independent fixture invocations use fresh synthetic budgets AFTER the
    // same paid priming. This never resets an original request's shared pool.
    for allowance in [0, needed - 1, needed] {
        let mut prime_work = 10_000;
        let mut owner = NullabilityCompleted::new(&grammar, &mut prime_work)?;
        let primed = first::analyze_with_storage_observed(
            &call,
            &grammar,
            0,
            &mut prime_work,
            &mut owner,
            None,
        );
        assert!(primed.is_ok());
        let trace = PredictionObservation::new();
        begin(&trace, &grammar, allowance)?;
        let mut balance = allowance;
        let actual = first::analyze_with_storage_observed(
            &call,
            &grammar,
            0,
            &mut balance,
            &mut owner,
            Some(&trace),
        );
        println!(
            "PUBLISHED_UNIT_WARM_BOUNDARY_WHOLE {:?}",
            (allowance, &actual, balance, trace.first_reuse_work())
        );
        assert_eq!(actual.is_ok(), allowance == needed);
        if actual.is_err() {
            let mut later_work = 10_000;
            assert!(
                first::analyze_with_storage_observed(
                    &call,
                    &grammar,
                    0,
                    &mut later_work,
                    &mut owner,
                    None
                )
                .is_err()
            );
        } else {
            assert_eq!(actual, expected);
            assert_eq!(balance, 0);
        }
    }
    // Prepublish Leaf, then reach it at calls=64. Its stored peak requires65,
    // so the unchanged depth guard must fall back and the canonical Visit of
    // its body must reject calls=65, rather than using a convenient bool hit.
    for index in (0..64).rev() {
        let target = if index == 63 {
            "Leaf".into()
        } else {
            format!("D{}", index + 1)
        };
        let body = node(&grammar, Kind::Call(target));
        add(&mut grammar, &format!("D{index}"), body);
    }
    let deep = node(&grammar, Kind::Call("D0".into()));
    let (mut a, mut b) = (10_000, 10_000);
    let mut full = Completed::new(&grammar, &mut a)?;
    let mut unit = NullabilityCompleted::new(&grammar, &mut b)?;
    assert!(first::analyze_with_completed(&call, &grammar, 0, &mut a, &mut full).is_ok());
    assert!(
        first::analyze_with_storage_observed(&call, &grammar, 0, &mut b, &mut unit, None).is_ok()
    );
    let trace = PredictionObservation::new();
    begin(&trace, &grammar, b)?;
    let old = first::analyze_with_completed(&deep, &grammar, 0, &mut a, &mut full);
    let actual =
        first::analyze_with_storage_observed(&deep, &grammar, 0, &mut b, &mut unit, Some(&trace));
    println!(
        "PUBLISHED_UNIT_DEPTH_FALLBACK_WHOLE {:?}",
        (&old, &actual, a, b, trace.first_reuse_work())
    );
    assert!(old.is_err() && actual.is_err());
    assert_eq!(old.map(|s| s.1), actual.map(|s| s.1));
    assert!(
        trace
            .first_reuse_work()
            .and_then(|s| s.events[Event::DepthOrCapacity as usize])
            .is_some_and(|n| n > 0)
    );
    Ok(())
}

#[test]
fn selective_one_step_real_worker_cost_and_full_result_parity() -> Result<(), String> {
    use crate::grammar::admission_observation::FirstReuseCharge as Charge;
    let mut grammar = capacity_fixture()?;
    let mut calls = Vec::new();
    for i in 0..15 {
        let kind = match i % 4 {
            0 => Kind::Literal(format!("word{i}")),
            1 => Kind::Action("Root".into()),
            2 => Kind::Capture {
                class: "Root".into(),
                feature: "name".into(),
                many: false,
            },
            _ => Kind::Reference("UnknownReferenceSpelling".into()),
        };
        let body = node(&grammar, kind);
        add(&mut grammar, &format!("Leaf{i}"), body);
        calls.push(node(&grammar, Kind::Call(format!("Leaf{i}"))));
    }
    calls.push(calls[0].clone());
    let root = node(&grammar, Kind::Alternative(calls));
    let before = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
    let (mut fw, mut ow, mut nw, mut pw) = (10_000, 10_000, 10_000, 10_000);
    let mut full = Completed::new(&grammar, &mut fw)?;
    let mut old = NullabilityCompleted::new(&grammar, &mut ow)?;
    old.memoize_one_step_for_test();
    let mut new = NullabilityCompleted::new(&grammar, &mut nw)?;
    let mut plain = NullabilityCompleted::new(&grammar, &mut pw)?;
    let (ot, nt) = (PredictionObservation::new(), PredictionObservation::new());
    assert!(ot.enable_first_map_cost_diagnostic() && nt.enable_first_map_cost_diagnostic());
    begin(&ot, &grammar, ow)?;
    begin(&nt, &grammar, nw)?;
    let (ob, nb) = (ow, nw);
    let fr = first::analyze_with_completed(&root, &grammar, 0, &mut fw, &mut full);
    let or = first::analyze_with_storage_observed(&root, &grammar, 0, &mut ow, &mut old, Some(&ot));
    let nr = first::analyze_with_storage_observed(&root, &grammar, 0, &mut nw, &mut new, Some(&nt));
    let pr = first::analyze_with_storage_observed(&root, &grammar, 0, &mut pw, &mut plain, None);
    ot.reuse_window(Window::First, ob, ow);
    ot.reuse_end(ow, or.is_err());
    nt.reuse_window(Window::First, nb, nw);
    nt.reuse_end(nw, nr.is_err());
    let os = ot
        .first_reuse_work()
        .ok_or("Missing original-policy counters")?;
    let ns = nt.first_reuse_work().ok_or("Missing selective counters")?;
    let maps = nt
        .first_map_cost()
        .ok_or("Missing selective map counters")?;
    println!(
        "SELECTIVE_UNIT_WHOLE {}",
        serde_json::json!({
            "full":fr,"old_unit":or,"selective_unit":nr,"plain_unit":pr,
            "full_total":10000-fw,"old_unit_total":10000-ow,"selective_total":10000-nw,
            "old_observation":format!("{os:?}"),"new_observation":format!("{ns:?}"),
            "old_map":format!("{:?}",ot.first_map_cost()),"new_map":format!("{maps:?}"),
            "scope":"complete synthetic canonical fixture with15 distinct one-step callees/one repeated call; no original551 award"
        })
    );
    assert!(fr.is_ok() && or.is_ok() && nr.is_ok());
    assert_eq!(fr.as_ref().map(|s| s.1), nr.as_ref().map(|s| s.1));
    assert_eq!(or, nr);
    assert_eq!(nr, pr);
    assert_eq!(nw, pw);
    assert!(
        10000 - nw < 10000 - ow,
        "whole actual old-policy Unit worker includes all cache setup, movement and publication"
    );
    assert_eq!(ns.events[Event::Publication as usize], Some(0));
    assert_eq!(ns.events[Event::Hit as usize], Some(0));
    assert_eq!(os.events[Event::Publication as usize], Some(15));
    assert_eq!(os.events[Event::Hit as usize], Some(1));
    assert!(ns.retained_cache_bytes < os.retained_cache_bytes);
    assert!(
        ns.first_units[Charge::FrameRetentionAndCapacity as usize].is_some_and(|n| n > 0),
        "real worker frame allocation is still paid"
    );
    assert_eq!(maps.paid_units[0], Some(0));
    assert_eq!(maps.paid_units[2], Some(0));
    assert!(
        maps.paid_units[1].is_some_and(|n| n > 0),
        "declaration/source lookup remains paid"
    );
    assert_eq!(
        maps.total_paid_units(),
        ns.first_units[Charge::OrderedMap as usize]
    );
    assert_eq!(
        serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
        before
    );
    Ok(())
}

#[test]
fn selective_one_step_canonical_shapes_sources_unknowns_and_error_order() -> Result<(), String> {
    let mut grammar = capacity_fixture()?;
    for (name, kind, cardinality) in [
        ("Literal", Kind::Literal("x".into()), '1'),
        ("Action", Kind::Action("Root".into()), '1'),
        (
            "Capture",
            Kind::Capture {
                class: "Root".into(),
                feature: "name".into(),
                many: false,
            },
            '+',
        ),
        ("Reference", Kind::Reference("NotAParserRule".into()), '?'),
    ] {
        let mut body = node(&grammar, kind);
        body.cardinality = cardinality;
        add(&mut grammar, name, body);
    }
    let mut cases: Vec<(String, Expr, bool)> = ["Literal", "Action", "Capture", "Reference"]
        .into_iter()
        .map(|n| (n.into(), node(&grammar, Kind::Call(n.into())), false))
        .collect();
    let bad = node(
        &grammar,
        Kind::ScalarReference {
            target: "Root".into(),
            rule: "Missing".into(),
        },
    );
    cases.push((
        "unknown_terminal".into(),
        node(&grammar, Kind::Call("Missing".into())),
        false,
    ));
    cases.push(("unknown_scalar".into(), bad.clone(), true));
    cases.push((
        "nonnullable_sequence_skips_error".into(),
        node(
            &grammar,
            Kind::Sequence(vec![
                node(&grammar, Kind::Call("Literal".into())),
                bad.clone(),
            ]),
        ),
        false,
    ));
    cases.push((
        "nullable_alternative_does_not_skip_error".into(),
        node(
            &grammar,
            Kind::Alternative(vec![node(&grammar, Kind::Call("Action".into())), bad]),
        ),
        true,
    ));
    for (label, root, error) in cases {
        for invalid in [0, 1, 2] {
            let mut g = grammar.clone();
            if invalid == 1 {
                g.declaration_sources.remove("Literal");
            }
            if invalid == 2 {
                g.rules
                    .get_mut("Literal")
                    .ok_or("Missing literal")?
                    .body
                    .original_range = Some(2..3);
            }
            let (mut a, mut b) = (10_000, 10_000);
            let mut full = Completed::new(&g, &mut a)?;
            let mut unit = NullabilityCompleted::new(&g, &mut b)?;
            let f = first::analyze_with_completed(&root, &g, 0, &mut a, &mut full);
            let u = first::analyze_with_storage_observed(&root, &g, 0, &mut b, &mut unit, None);
            println!(
                "SELECTIVE_UNIT_GUARDS_WHOLE {:?}",
                (label.as_str(), invalid, &f, &u, a, b)
            );
            assert_eq!(f.as_ref().map(|s| s.1), u.as_ref().map(|s| s.1));
            assert_eq!(u.is_err(), error);
        }
    }
    // A foreign graph must use the canonical foreign path, never this cache.
    let foreign = grammar.clone();
    let root = node(&foreign, Kind::Call("Literal".into()));
    let (mut a, mut b) = (10_000, 10_000);
    let mut owner = NullabilityCompleted::new(&grammar, &mut a)?;
    let u = first::analyze_with_storage_observed(&root, &foreign, 0, &mut a, &mut owner, None);
    let f = first::analyze_with_remaining(&root, &foreign, 0, &mut b);
    println!("SELECTIVE_UNIT_FOREIGN_WHOLE {:?}", (&u, &f, a, b));
    assert_eq!(u.map(|s| s.1), f.map(|s| s.1));
    Ok(())
}

#[test]
fn selective_one_step_short_work_and_retention_fail_sticky_without_free_frames()
-> Result<(), String> {
    let grammar = capacity_fixture()?;
    let root = node(&grammar, Kind::Call("Root".into()));
    let mut work = 10_000;
    let mut owner = NullabilityCompleted::new(&grammar, &mut work)?;
    let result =
        first::analyze_with_storage_observed(&root, &grammar, 0, &mut work, &mut owner, None);
    assert!(result.is_ok());
    let needed = 10_000 - work;
    for allowance in [0, 1, needed - 1, needed] {
        let (mut a, mut b) = (allowance, allowance);
        let trace = PredictionObservation::new();
        begin(&trace, &grammar, b)?;
        let plain = match NullabilityCompleted::new(&grammar, &mut a) {
            Ok(mut owner) => {
                first::analyze_with_storage_observed(&root, &grammar, 0, &mut a, &mut owner, None)
            }
            Err(e) => Err(e),
        };
        let mut observed_owner = NullabilityCompleted::new(&grammar, &mut b);
        let observed = match &mut observed_owner {
            Ok(owner) => first::analyze_with_storage_observed(
                &root,
                &grammar,
                0,
                &mut b,
                owner,
                Some(&trace),
            ),
            Err(e) => Err(e.clone()),
        };
        println!(
            "SELECTIVE_UNIT_SHORT_WHOLE {:?}",
            (allowance, &plain, &observed, a, b, trace.first_reuse_work())
        );
        assert_eq!(plain, observed);
        assert_eq!(a, b);
        assert_eq!(observed.is_ok(), allowance == needed);
        if observed.is_err()
            && let Ok(owner) = &mut observed_owner
        {
            let mut later = 10_000;
            assert!(
                first::analyze_with_storage_observed(&root, &grammar, 0, &mut later, owner, None)
                    .is_err()
            );
        }
    }
    // Header128 is all that fits: even without a cache entry, the real body
    // frame must fail its retained-storage charge rather than allocate free.
    let mut balance = 10_000;
    let mut tiny = NullabilityCompleted::with_limit(&grammar, &mut balance, 128)?;
    let result =
        first::analyze_with_storage_observed(&root, &grammar, 0, &mut balance, &mut tiny, None);
    println!("SELECTIVE_UNIT_QUOTA_WHOLE {:?}", (&result, balance));
    assert!(result.is_err());
    let mut later = 10_000;
    assert!(
        first::analyze_with_storage_observed(&root, &grammar, 0, &mut later, &mut tiny, None)
            .is_err()
    );
    Ok(())
}

#[test]
fn selective_one_step_keeps_nested_parent_depth_and_active_recursion() -> Result<(), String> {
    let mut grammar = capacity_fixture()?;
    let leaf = node(&grammar, Kind::Literal("x".into()));
    add(&mut grammar, "Leaf", leaf);
    let parent = node(&grammar, Kind::Call("Leaf".into()));
    add(&mut grammar, "Parent", parent);
    for i in (0..63).rev() {
        let target = if i == 62 {
            "Parent".into()
        } else {
            format!("D{}", i + 1)
        };
        let body = node(&grammar, Kind::Call(target));
        add(&mut grammar, &format!("D{i}"), body);
    }
    for (entry, error) in [("D1", false), ("D0", true)] {
        let prime = node(&grammar, Kind::Call("Parent".into()));
        let root = node(&grammar, Kind::Call(entry.into()));
        let (mut a, mut b) = (10_000, 10_000);
        let mut full = Completed::new(&grammar, &mut a)?;
        let mut unit = NullabilityCompleted::new(&grammar, &mut b)?;
        assert!(first::analyze_with_completed(&prime, &grammar, 0, &mut a, &mut full).is_ok());
        assert!(
            first::analyze_with_storage_observed(&prime, &grammar, 0, &mut b, &mut unit, None)
                .is_ok()
        );
        let f = first::analyze_with_completed(&root, &grammar, 0, &mut a, &mut full);
        let u = first::analyze_with_storage_observed(&root, &grammar, 0, &mut b, &mut unit, None);
        println!(
            "SELECTIVE_UNIT_PARENT_DEPTH_WHOLE {:?}",
            (entry, &f, &u, a, b)
        );
        assert_eq!(f.as_ref().map(|s| s.1), u.as_ref().map(|s| s.1));
        assert_eq!(u.is_err(), error);
    }
    let cycle = node(&grammar, Kind::Call("Cycle".into()));
    add(&mut grammar, "Cycle", cycle.clone());
    let (mut a, mut b) = (10_000, 10_000);
    let mut full = Completed::new(&grammar, &mut a)?;
    let mut unit = NullabilityCompleted::new(&grammar, &mut b)?;
    let f = first::analyze_with_completed(&cycle, &grammar, 0, &mut a, &mut full);
    let u = first::analyze_with_storage_observed(&cycle, &grammar, 0, &mut b, &mut unit, None);
    println!("SELECTIVE_UNIT_ACTIVE_WHOLE {:?}", (&f, &u, a, b));
    assert!(f.is_err() && u.is_err());
    assert_eq!(f.map(|s| s.1), u.map(|s| s.1));
    Ok(())
}
