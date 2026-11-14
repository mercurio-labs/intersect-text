//! Real canonical FIRST and reservation seams; no language request or allocator metric.
use super::super::{Expr, Frame, Kind, analyze_with_completed, analyze_with_completed_observed};
use super::*;
use crate::grammar::admission_observation::FirstReuseWindow;
use crate::grammar::admission_observation::RunStage;
use intersect_model::definitions::bundle::SourceId;

fn fixture() -> Result<Grammar, String> {
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
    let mut grammar = fixture()?;
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

#[test]
fn prepaid_worker_capacity_reuses_real_reservation_but_keeps_per_call_retention()
-> Result<(), String> {
    let grammar = fixture()?;
    let mut work = 10_000;
    let mut completed = Completed::new(&grammar, &mut work)?;
    let mut frames: Vec<_> = (0..24).map(|_| Frame::Cardinality('1')).collect();
    let mut guaranteed = 0;
    let before = work;
    completed.return_frame(&mut frames, &mut guaranteed, &mut work)?;
    assert_eq!(guaranteed, 32);
    assert_eq!(frames.len(), 24);
    assert_eq!(completed.retained, HEADER + FRAME + 32 * FRAME);
    let first = before - work;
    let retained = completed.retained;
    let before = work;
    for _ in 0..7 {
        completed.return_frame(&mut frames, &mut guaranteed, &mut work)?;
    }
    let warm = before - work;
    assert_eq!(guaranteed, 32);
    assert_eq!(frames.len(), 24);
    assert_eq!(completed.retained - retained, 7 * FRAME);
    // Each fit still pays the unchanged FRAME retention (1 + 128/64)
    // and one planning debit. This compares paid logical work, not timing.
    assert_eq!(warm, 7 * 4);
    // Exact old source formula at THIS seam: each of eight calls paid the
    // same retention plus transfer of 24 live FRAME slots, then reserved.
    let old_same_seam_units = 8 * (1 + FRAME.div_ceil(64) + (24 * FRAME).div_ceil(64));
    println!(
        "PREPAID_CAPACITY_SEAM {:?}",
        (first, warm, old_same_seam_units)
    );
    assert!(first + warm < old_same_seam_units);
    Ok(())
}

#[test]
fn capacity_growth_is_prepaid_exact_short_zero_memory_and_overflow_bounded() -> Result<(), String> {
    let grammar = fixture()?;
    let mut setup = 10_000;
    let mut probe = Completed::new(&grammar, &mut setup)?;
    let mut frames: Vec<_> = (0..10).map(|_| Frame::Cardinality('1')).collect();
    let mut guaranteed = 0;
    let before = setup;
    probe.return_frame(&mut frames, &mut guaranteed, &mut setup)?;
    let exact = before - setup;
    let retained = probe.retained;
    assert_eq!(guaranteed, 16);
    for allowance in [0, exact - 1, exact] {
        let mut constructor = 10_000;
        let mut owner = Completed::new(&grammar, &mut constructor)?;
        let mut work = allowance;
        let mut bound = 0;
        let result = owner.return_frame(&mut frames, &mut bound, &mut work);
        println!(
            "PREPAID_CAPACITY_BOUNDARY {:?}",
            (&result, allowance, work, bound, owner.retained)
        );
        assert_eq!(result.is_ok(), allowance == exact);
        assert_eq!(bound, if result.is_ok() { 16 } else { 0 });
        assert_eq!(frames.len(), 10);
        if result.is_ok() {
            assert_eq!(work, 0);
            assert_eq!(owner.retained, retained);
        }
    }
    for limit in [retained - 1, retained] {
        let mut work = 10_000;
        let mut owner = Completed::with_limit(&grammar, &mut work, limit)?;
        let mut bound = 0;
        let result = owner.return_frame(&mut frames, &mut bound, &mut work);
        assert_eq!(result.is_ok(), limit == retained);
        assert_eq!(bound, if result.is_ok() { 16 } else { 0 });
        assert_eq!(frames.len(), 10);
        assert!(work > 0);
    }
    let mut work = 10_000;
    let mut owner = Completed::new(&grammar, &mut work)?;
    for (length, additional) in [(usize::MAX, 2), (LIMIT / FRAME, 1)] {
        let retained = owner.retained;
        let result = owner.plan_capacity(
            length,
            additional,
            0,
            &mut work,
            Meter::new(None, Charge::FrameRetentionAndCapacity),
        );
        assert!(result.is_err());
        assert_eq!(owner.retained, retained);
        assert_eq!(owner.entry_count(), 0);
    }
    Ok(())
}

#[test]
fn query_local_worker_guarantee_cannot_survive_a_fresh_vec() -> Result<(), String> {
    let grammar = fixture()?;
    let mut work = 10_000;
    let mut owner = Completed::new(&grammar, &mut work)?;
    let mut first: Vec<_> = (0..10).map(|_| Frame::Cardinality('1')).collect();
    let mut first_bound = 0;
    let before = work;
    owner.return_frame(&mut first, &mut first_bound, &mut work)?;
    let first_units = before - work;
    let first_retained = owner.retained;
    // Query-local bound starts at zero even though Completed itself persists.
    let mut second: Vec<_> = (0..10).map(|_| Frame::Cardinality('1')).collect();
    let mut second_bound = 0;
    let before = work;
    owner.return_frame(&mut second, &mut second_bound, &mut work)?;
    assert_eq!(before - work, first_units);
    assert_eq!(second_bound, first_bound);
    assert_eq!(owner.retained - first_retained, FRAME + 16 * FRAME);
    assert_eq!(first.len(), second.len());
    Ok(())
}

#[test]
fn canonical_nested_worker_whole_result_and_observer_parity_with_sticky_short_failure()
-> Result<(), String> {
    let (grammar, root) = branching()?;
    let original = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
    let expected = super::super::analyze(&root, &grammar, 0, 10_000);
    let mut probe_work = 10_000;
    let mut probe = Completed::new(&grammar, &mut probe_work)?;
    let full = analyze_with_completed(&root, &grammar, 0, &mut probe_work, &mut probe);
    assert_eq!(full, expected);
    assert!(full.is_ok());
    assert_eq!(probe.hits, 1);
    assert_eq!(probe.publications, 32);
    assert_eq!(probe.active_capacity, 8);
    let exact = 10_000 - probe_work;
    for allowance in [3, exact - 1, exact, 10_000] {
        let observation = PredictionObservation::new();
        let (mut a, mut b) = (allowance, allowance);
        let mut cold = Completed::new(&grammar, &mut a)?;
        let mut observed = Completed::new(&grammar, &mut b)?;
        observation.reuse_begin(
            RunStage::Executable,
            &grammar.declaration_sources["Root"].source,
            b,
        );
        let before = b;
        let plain = analyze_with_completed(&root, &grammar, 0, &mut a, &mut cold);
        let measured = analyze_with_completed_observed(
            &root,
            &grammar,
            0,
            &mut b,
            &mut observed,
            Some(&observation),
        );
        observation.reuse_window(FirstReuseWindow::First, before, b);
        observation.reuse_end(b, measured.is_err());
        let snapshot = observation
            .first_reuse_work()
            .ok_or("Missing capacity snapshot")?;
        println!(
            "PREPAID_CANONICAL_WHOLE {:?}",
            (&plain, &measured, a, b, snapshot)
        );
        assert_eq!(plain, measured);
        assert_eq!(a, b);
        assert_eq!(cold.retained, observed.retained);
        assert_eq!(cold.active_capacity, observed.active_capacity);
        assert_eq!(cold.entry_count(), observed.entry_count());
        assert_eq!(cold.failed, observed.failed);
        assert_eq!(snapshot.first_units_total(), Some(before - b));
        assert!(!snapshot.overflow && !snapshot.multiple_programs);
        assert_eq!(measured.is_ok(), allowance >= exact);
        if measured.is_err() {
            assert!(observed.failed);
            let before_entries = observed.entry_count();
            let mut refilled = 10_000;
            assert!(
                analyze_with_completed(&root, &grammar, 0, &mut refilled, &mut observed).is_err()
            );
            assert_eq!(observed.entry_count(), before_entries);
        }
    }
    assert_eq!(
        serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
        original
    );
    Ok(())
}
