//! Canonical lookup lineage and real FIRST controls, not a second evaluator.
use super::*;
use crate::grammar::Expr;
use crate::grammar::Kind;
use crate::grammar::admission_observation::FirstReuseCharge;
use crate::grammar::admission_observation::FirstReuseEvent;
use crate::grammar::admission_observation::PredictionObservation;
use crate::grammar::admission_observation::RunStage;
use crate::grammar::first_sets;
use crate::grammar::first_sets::completed_reuse::Call;
use crate::grammar::first_sets::completed_reuse::Completed;
use intersect_model::definitions::bundle::SourceId;

fn fixture() -> Result<Grammar, String> {
    let source = SourceId::for_text("memory:resolved-canonical-call", "x");
    serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"Resolved","entry":"Root",
        "rules":{"Root":{"class":"Root","body":{"cardinality":"1","original_range":{"start":0,"end":1},"kind":{"Literal":"x"}}}},
        "declaration_sources":{"Root":{"source":source,"start":0,"end":1}}
    })).map_err(|e| e.to_string())
}
fn node(grammar: &Grammar, kind: Kind) -> Expr {
    let mut value = grammar.rules["Root"].body.clone();
    value.kind = kind;
    value.cardinality = '1';
    value
}
fn add(grammar: &mut Grammar, name: &str, body: Expr, source: bool) {
    let mut rule = grammar.rules["Root"].clone();
    rule.body = body;
    grammar.rules.insert(name.into(), rule);
    if source {
        let span = grammar.declaration_sources["Root"].clone();
        grammar.declaration_sources.insert(name.into(), span);
    }
}
fn source_observation(grammar: &Grammar, work: usize) -> PredictionObservation {
    let observation = PredictionObservation::new();
    observation.reuse_begin(
        RunStage::Executable,
        &grammar.declaration_sources["Root"].source,
        work,
    );
    observation
}
// Published logical map fee for short keys, used only as an independent fee
// assertion. Actual worker/map calls are always the production implementations.
fn short_key_map_units(size: usize) -> usize {
    2 * (1 + (usize::BITS - (size + 1).leading_zeros()) as usize)
}

#[test]
fn canonical_token_reuses_real_lookup_and_completed_source_with_all_remaining_map_fees()
-> Result<(), String> {
    let mut grammar = fixture()?;
    for index in 0..40 {
        let body = node(&grammar, Kind::Literal("x".into()));
        add(&mut grammar, &format!("Rule{index}"), body, true);
    }
    let entry = node(&grammar, Kind::Call("Rule0".into()));
    let original = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
    let canonical = lookup(&grammar, "Rule0").ok_or("Missing canonical call")?;
    let (name, rule) = canonical.for_grammar(&grammar).ok_or("Wrong owner")?;
    assert_eq!(name, "Rule0");
    assert!(std::ptr::eq(rule, &grammar.rules["Rule0"]));
    let mut work = 10_000;
    let mut completed = Completed::new(&grammar, &mut work)?;
    let cold = source_observation(&grammar, work);
    let expected = first_sets::analyze(&entry, &grammar, 0, 10_000);
    let actual = first_sets::analyze_with_completed_observed(
        &entry,
        &grammar,
        0,
        &mut work,
        &mut completed,
        Some(&cold),
    );
    assert_eq!(actual, expected);
    assert!(actual.is_ok());
    let cold_snapshot = cold.first_reuse_work().ok_or("Missing cold observation")?;
    // Empty completed-map probe + real declaration lookup + publication probe.
    let cold_maps =
        short_key_map_units(0) * 2 + short_key_map_units(grammar.declaration_sources.len());
    assert_eq!(
        cold_snapshot.first_units[FirstReuseCharge::OrderedMap as usize],
        Some(cold_maps)
    );
    let warm = source_observation(&grammar, work);
    let actual = first_sets::analyze_with_completed_observed(
        &entry,
        &grammar,
        0,
        &mut work,
        &mut completed,
        Some(&warm),
    );
    assert_eq!(actual, expected);
    let warm_snapshot = warm.first_reuse_work().ok_or("Missing warm observation")?;
    // The actual completed-map lookup still pays its entire unchanged fee.
    let warm_maps = short_key_map_units(1);
    assert_eq!(
        warm_snapshot.first_units[FirstReuseCharge::OrderedMap as usize],
        Some(warm_maps)
    );
    assert_eq!(warm_snapshot.events[FirstReuseEvent::Hit as usize], Some(1));
    let old_redundant_maps = short_key_map_units(grammar.rules.len())
        + short_key_map_units(grammar.declaration_sources.len())
        + warm_maps;
    println!(
        "RESOLVED_CALL_ACTUAL_MAP_UNITS {:?}",
        (cold_snapshot, warm_snapshot, old_redundant_maps)
    );
    assert!(warm_maps < old_redundant_maps);
    assert_eq!(
        serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
        original
    );
    Ok(())
}

#[test]
fn equal_foreign_token_and_missing_range_source_keep_exact_canonical_fallback() -> Result<(), String>
{
    let grammar = fixture()?;
    let foreign: Grammar =
        serde_json::from_value(serde_json::to_value(&grammar).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let resolved = lookup(&foreign, "Root").ok_or("Missing foreign call")?;
    assert!(resolved.for_grammar(&grammar).is_none());
    let mut work = 10_000;
    let mut completed = Completed::new(&grammar, &mut work)?;
    let observation = source_observation(&grammar, work);
    assert!(matches!(
        completed.enter_observed(resolved, 0, &mut work, &observation)?,
        Call::Fallback
    ));
    assert_eq!(completed.entry_count(), 0);
    assert_eq!(
        observation
            .first_reuse_work()
            .ok_or("Missing foreign observation")?
            .events[FirstReuseEvent::ForeignGraph as usize],
        Some(1)
    );
    for missing in [true, false] {
        let mut grammar = fixture()?;
        let mut body = node(&grammar, Kind::Literal("x".into()));
        if !missing {
            body.original_range = None;
        }
        add(&mut grammar, "Leaf", body, !missing);
        let root = node(&grammar, Kind::Call("Leaf".into()));
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        let observation = source_observation(&grammar, work);
        assert_eq!(
            first_sets::analyze(&root, &grammar, 0, 10_000),
            first_sets::analyze_with_completed_observed(
                &root,
                &grammar,
                0,
                &mut work,
                &mut completed,
                Some(&observation)
            ),
        );
        assert_eq!(completed.entry_count(), 0);
        assert_eq!(
            observation
                .first_reuse_work()
                .ok_or("Missing ineligible observation")?
                .events[FirstReuseEvent::MissingSourceOrRange as usize],
            Some(1)
        );
    }
    assert!(lookup(&grammar, "Unknown").is_none());
    Ok(())
}

#[test]
fn resolved_calls_preserve_active_recursion_productive_tail_and_unknown_call_semantics()
-> Result<(), String> {
    for productive in [false, true] {
        let mut grammar = fixture()?;
        let recursive = node(&grammar, Kind::Call("Recursive".into()));
        let body = if productive {
            node(
                &grammar,
                Kind::Sequence(vec![node(&grammar, Kind::Literal("x".into())), recursive]),
            )
        } else {
            recursive
        };
        add(&mut grammar, "Recursive", body, true);
        let root = node(&grammar, Kind::Call("Recursive".into()));
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        let actual =
            first_sets::analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed);
        assert_eq!(actual, first_sets::analyze(&root, &grammar, 0, 10_000));
        assert_eq!(actual.is_ok(), productive);
        assert_eq!(completed.entry_count(), usize::from(productive));
        if productive {
            assert_eq!(
                first_sets::analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed),
                actual
            );
            assert_eq!(completed.hits, 1);
        } else {
            assert!(completed.ready().is_err());
            assert_eq!(completed.publications, 0);
        }
    }
    let grammar = fixture()?;
    for kind in [
        Kind::Call("Unknown".into()),
        Kind::ScalarReference {
            target: "Root".into(),
            rule: "Unknown".into(),
        },
    ] {
        let root = node(&grammar, kind);
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        assert_eq!(
            first_sets::analyze(&root, &grammar, 0, 10_000),
            first_sets::analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed)
        );
        assert_eq!(completed.entry_count(), 0);
    }
    Ok(())
}

#[test]
fn resolved_call_full_result_observer_parity_and_exact_short_publication_remain_bounded()
-> Result<(), String> {
    let grammar = fixture()?;
    let root = node(&grammar, Kind::Call("Root".into()));
    let mut probe_work = 10_000;
    let mut probe = Completed::new(&grammar, &mut probe_work)?;
    let expected =
        first_sets::analyze_with_completed(&root, &grammar, 0, &mut probe_work, &mut probe)?;
    let exact = 10_000 - probe_work;
    for budget in [3, exact - 1, exact, 10_000] {
        let (mut a, mut b) = (budget, budget);
        let mut plain = Completed::new(&grammar, &mut a)?;
        let mut observed = Completed::new(&grammar, &mut b)?;
        let observation = source_observation(&grammar, b);
        let left = first_sets::analyze_with_completed(&root, &grammar, 0, &mut a, &mut plain);
        let right = first_sets::analyze_with_completed_observed(
            &root,
            &grammar,
            0,
            &mut b,
            &mut observed,
            Some(&observation),
        );
        println!(
            "RESOLVED_CALL_WHOLE_BOUNDARY {:?}",
            (&left, &right, a, b, observation.first_reuse_work())
        );
        assert_eq!(left, right);
        assert_eq!(a, b);
        assert_eq!(plain.retained_bytes(), observed.retained_bytes());
        assert_eq!(plain.entry_count(), observed.entry_count());
        assert_eq!(right.is_ok(), budget >= exact);
        if let Ok(actual) = right {
            assert_eq!(actual, expected);
        } else {
            assert_eq!(observed.publications, 0);
            assert_eq!(observed.entry_count(), 0);
            assert!(observed.ready().is_err());
        }
    }
    Ok(())
}
