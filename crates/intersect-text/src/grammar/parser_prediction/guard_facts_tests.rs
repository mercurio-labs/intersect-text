use super::super::{
    ParserPrediction, RecursiveFrontierOrder, RuntimeGuardedPolicy, collect_guards_with_facts,
};
use super::*;
use crate::grammar;
use crate::grammar::AnalysisObservation;
use crate::grammar::Profile;
use crate::grammar::admission_observation::PredictionObservation;
use crate::grammar::admission_observation::RunStage;
use crate::grammar::backend_test_support::inspect_ecore_schema;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use std::collections::{BTreeMap, BTreeSet};

fn fixture(count: usize) -> (DefinitionSource, Grammar, BTreeSet<String>) {
    let schema = inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
    let mut body = String::from("{t::Root} '{' ");
    let mut rules = String::new();
    for index in 0..count {
        body.push_str(&format!("( -> Item{index} )+ "));
        rules.push_str(&format!(
            "fragment Item{index} returns t::Root: 'token{index}'; "
        ));
    }
    body.push_str("'}'");
    let source = DefinitionSource::new(
        "memory:prepared/first-guard.xtext",
        SourceKind::Xtext,
        format!(
            "grammar guards hidden() import \"urn:t\" as t Root returns t::Root: {body}; {rules}"
        ),
    );
    let (mut grammar, _, spans, _) = grammar::parse_declarations(
        &source.text,
        &schema,
        Profile::NativeV80,
        false,
        None,
        128,
        std::rc::Rc::default(),
    )
    .unwrap();
    grammar.declaration_sources = spans
        .into_iter()
        .map(|(name, range)| {
            (
                name,
                SourceSpan {
                    source: source.id.clone(),
                    start: range.start,
                    end: range.end,
                },
            )
        })
        .collect();
    grammar::terminals::prepare(&mut grammar.terminals, grammar.profile)
        .map_err(|error| error.message)
        .unwrap();
    let mut predicates = BTreeSet::new();
    for (name, rule) in &mut grammar.rules {
        grammar::bind_prediction_paths(
            &mut rule.body,
            &format!("rule/{name}"),
            &mut predicates,
            true,
        );
    }
    (source, grammar, predicates)
}

fn compile<const REUSE: bool>(
    source: &DefinitionSource,
    grammar: &Grammar,
    predicates: &BTreeSet<String>,
    policy: RuntimeGuardedPolicy,
) -> (
    Result<ParserPrediction, String>,
    Option<grammar::admission_observation::FirstReuseWorkSnapshot>,
) {
    let trace = PredictionObservation::new();
    let context = AnalysisObservation {
        observation: &trace,
        recursive_work: None,
        adjacent_work: None,
        entry: &source.id,
        stage: RunStage::Executable,
    };
    let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
    let result = ParserPrediction::compile_predicates_impl_prediction_policy::<REUSE>(
        grammar,
        predicates,
        true,
        Some(context),
        None,
        RecursiveFrontierOrder::Historical,
        None,
        Some(&mut admission),
    );
    (result, trace.first_reuse_work())
}

#[test]
fn prepared_first_guard_whole_program_reuses_exact_query_in_exhausted_pool() {
    let (source, grammar, predicates) = fixture(2);
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let old = compile::<false>(&source, &grammar, &predicates, policy);
        let new = compile::<true>(&source, &grammar, &predicates, policy);
        let guard_work = |reuse| {
            let mut work = 10_000;
            let mut facts = GuardFacts::new(&grammar);
            let mut owner = RuntimeGuardedAdmission::new_with_policy(policy);
            let result = if reuse {
                prepare(&grammar, &predicates, &mut work, &mut facts, &mut owner)
            } else {
                let mut unresolved = predicates.clone();
                let mut symbols = BTreeMap::new();
                grammar.rules.iter().try_for_each(|(name, rule)| {
                    super::super::collect_guards(
                        &rule.body,
                        &grammar,
                        &format!("rule/{name}"),
                        &mut unresolved,
                        &mut symbols,
                        &mut work,
                        (0, true, grammar::GrammarAnalysisWorkBudget::LEGACY_10K),
                    )
                })
            };
            assert_eq!(result, Ok(()));
            10_000 - work
        };
        let (old_guard, new_guard) = (guard_work(false), guard_work(true));
        println!(
            "PREPARED_FIRST_GUARD_WHOLE {}",
            serde_json::json!({
                "old_result":old.0,"new_result":new.0,
                "old_work":format!("{:?}",old.1),"new_work":format!("{:?}",new.1),
                "global_follow_work_unchanged":true,
            "guard_work":[old_guard,new_guard],
            })
        );
        assert!(old.0.is_ok());
        assert_eq!(
            serde_json::to_value(&old.0).unwrap(),
            serde_json::to_value(&new.0).unwrap()
        );
        let (old, new) = (old.1.unwrap(), new.1.unwrap());
        assert!(old.finished && new.finished && !old.returned_error && !new.returned_error);
        assert_eq!(old.events[4], Some(2));
        assert_eq!(new.events[4], Some(0));
        assert_eq!(new.first_inclusive_work, Some(0));
        let collector_saved = new.exit_remaining.unwrap() - old.exit_remaining.unwrap();
        let guard_added = new_guard - old_guard;
        assert!(
            collector_saved > guard_added,
            "real total includes ALL added guard preparation work"
        );
        // No equality claim for Completed cache/visit/clone telemetry: that work
        // is deliberately absent, and a later consumer may lose a warm entry.
    }
    let old = compile::<false>(
        &source,
        &grammar,
        &predicates,
        RuntimeGuardedPolicy::LinearFullRepeats,
    );
    let new = compile::<true>(
        &source,
        &grammar,
        &predicates,
        RuntimeGuardedPolicy::LinearFullRepeats,
    );
    assert_eq!(
        serde_json::to_value(old.0).unwrap(),
        serde_json::to_value(new.0).unwrap()
    );
    assert_eq!(
        old.1, new.1,
        "Linear is outside this opt-in preparation path"
    );
}

fn prepare<'g>(
    grammar: &'g Grammar,
    predicates: &BTreeSet<String>,
    work: &mut usize,
    facts: &mut GuardFacts<'g>,
    admission: &mut RuntimeGuardedAdmission,
) -> Result<(), String> {
    let mut unresolved = predicates.clone();
    let mut symbols = BTreeMap::new();
    for (name, rule) in &grammar.rules {
        collect_guards_with_facts(
            &rule.body,
            grammar,
            &format!("rule/{name}"),
            &mut unresolved,
            &mut symbols,
            work,
            (0, true),
            Some((facts, admission)),
        )?;
    }
    assert!(unresolved.is_empty());
    Ok(())
}

#[test]
fn prepared_first_guard_capacity_exact_short_quota_and_caught_error_stay_paid() {
    let (_, grammar, predicates) = fixture(5); // genuinely grows logical4 ->8
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let mut facts = GuardFacts::new(&grammar);
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut work = 10_000;
        assert_eq!(
            prepare(&grammar, &predicates, &mut work, &mut facts, &mut admission),
            Ok(())
        );
        assert_eq!(facts.entries.len(), 5);
        assert_eq!(facts.guaranteed, 8);
        let used = 10_000 - work;
        for grant in [0, used - 1, used] {
            let mut facts = GuardFacts::new(&grammar);
            let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
            let mut work = grant;
            let result = prepare(&grammar, &predicates, &mut work, &mut facts, &mut admission);
            println!("PREPARED_FIRST_GUARD_BOUNDARY grant={grant} {result:?} remaining={work}");
            assert_eq!(result.is_ok(), grant == used);
            assert_eq!(work, 0);
        }
        // Direct new capture denial after the SAME successful canonical call.
        let expr = facts.entries[0];
        let (_, nullable) = grammar::first(expr, &grammar, 0).unwrap();
        for grant in 0..4 {
            let mut denied = GuardFacts::new(&grammar);
            let mut owner = RuntimeGuardedAdmission::new_with_policy(policy);
            let mut work = grant;
            assert!(
                denied
                    .record(expr, nullable, &mut owner, &mut work)
                    .is_err()
            );
            assert!(denied.failed && denied.entries.is_empty());
            assert!(owner.nonnullable_retention_test_state().1);
            assert!(
                denied
                    .record(expr, nullable, &mut owner, &mut 10_000)
                    .is_err()
            );
            assert!(denied.nonnullable(&grammar, expr, &mut 10_000).is_err());
        }
        let mut denied = GuardFacts::new(&grammar);
        let mut owner = RuntimeGuardedAdmission::new_with_policy(policy);
        owner.nonnullable_retention_test_fill();
        assert!(
            denied
                .record(expr, nullable, &mut owner, &mut 10_000)
                .is_err()
        );
        assert!(denied.failed && denied.entries.is_empty());
        let mut remaining = 1;
        assert!(facts.nonnullable(&grammar, expr, &mut remaining).is_err());
        assert_eq!(remaining, 0);
    }
}

#[test]
fn prepared_first_guard_foreign_equal_text_expression_and_quantifiers_are_no_authority() {
    let (_, grammar, predicates) = fixture(1);
    let (_, foreign, _) = fixture(1);
    let mut facts = GuardFacts::new(&grammar);
    let mut owner =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    assert_eq!(
        prepare(&grammar, &predicates, &mut 10_000, &mut facts, &mut owner),
        Ok(())
    );
    let expr = facts.entries[0];
    assert_eq!(facts.nonnullable(&grammar, expr, &mut 10_000), Ok(true));
    assert_eq!(facts.nonnullable(&foreign, expr, &mut 10_000), Ok(false));
    let equal_expr = expr.clone();
    assert_eq!(
        facts.nonnullable(&grammar, &equal_expr, &mut 10_000),
        Ok(false)
    );
    let absent = GuardFacts::new(&grammar);
    let mut zero = 0;
    assert_eq!(absent.nonnullable(&grammar, expr, &mut zero), Ok(false));
    for cardinality in ['*', '+', '?'] {
        let mut altered = expr.clone();
        altered.cardinality = cardinality;
        let (_, nullable) = grammar::first(&altered, &grammar, 0).unwrap();
        let mut unsupported = GuardFacts::new(&grammar);
        assert_eq!(
            unsupported.record(&altered, nullable, &mut owner, &mut 10_000),
            Ok(())
        );
        assert!(unsupported.entries.is_empty());
    }
    // Changing the program/source/range cannot use a borrowed fact from the
    // original grammar. The existing binder still rejects bound source/graph
    // mismatches; retained runtime/source-certificate controls cover those.
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        for case in 0..4 {
            let (source, mut bad, predicates) = fixture(1);
            let rule = bad.rules.get_mut("Item0").unwrap();
            match case {
                0 => {
                    rule.body.kind = Kind::ScalarReference {
                        target: "Root".into(),
                        rule: "MissingScalar".into(),
                    }
                }
                1 => rule.body.kind = Kind::Action("Root".into()),
                2 => rule.body.kind = Kind::Call("Item0".into()),
                _ => rule.hidden = Some(vec!["DIFFERENT_HIDDEN".into()]),
            }
            let old = compile::<false>(&source, &bad, &predicates, policy);
            let new = compile::<true>(&source, &bad, &predicates, policy);
            println!(
                "PREPARED_FIRST_GUARD_ERROR case={case} old={:?} new={:?}",
                old.0, new.0
            );
            assert!(old.0.is_err());
            assert_eq!(
                serde_json::to_value(old.0).unwrap(),
                serde_json::to_value(new.0).unwrap()
            );
        }
    }
}
