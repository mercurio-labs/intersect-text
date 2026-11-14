use super::*;
use crate::grammar::Kind;
use crate::grammar::RuntimeGuardedPolicy;
use crate::grammar::admission_observation::FirstReuseEvent;
use crate::grammar::admission_observation::PredictionObservation;
use crate::grammar::admission_observation::RunStage;
use intersect_model::definitions::bundle::SourceId;
use std::collections::BTreeSet;

fn fixture() -> Result<Grammar, String> {
    let source = SourceId::for_text("memory:nullable-inventory-controls", "x");
    serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"Inventory","entry":"Root",
        "rules":{"Root":{"class":"Root","body":{"cardinality":"1","original_range":{"start":0,"end":1},"kind":{"Literal":"x"}}}},
        "declaration_sources":{"Root":{"source":source,"start":0,"end":1}}
    })).map_err(|error| error.to_string())
}
fn expr(grammar: &Grammar, kind: Kind, cardinality: char) -> Expr {
    let mut expr = grammar.rules["Root"].body.clone();
    expr.kind = kind;
    expr.cardinality = cardinality;
    expr
}
fn full(grammar: &Grammar, callee: &str) -> Expr {
    let call = expr(grammar, Kind::Call(callee.into()), '1');
    let assigned = expr(
        grammar,
        Kind::Assign {
            feature: "items".into(),
            many: true,
            value: Box::new(call),
        },
        '1',
    );
    expr(grammar, Kind::SyntacticPredicate(Box::new(assigned)), '*')
}
fn context<'a>(
    grammar: &'a Grammar,
    observation: &'a PredictionObservation,
) -> AnalysisObservation<'a> {
    AnalysisObservation {
        observation,
        recursive_work: None,
        adjacent_work: None,
        entry: &grammar.declaration_sources["Root"].source,
        stage: RunStage::Executable,
    }
}
fn pair(
    grammar: &Grammar,
    policy: RuntimeGuardedPolicy,
) -> Result<
    (
        serde_json::Value,
        PredictionObservation,
        PredictionObservation,
    ),
    String,
> {
    let before = serde_json::to_value(grammar).map_err(|e| e.to_string())?;
    let mut cold = RuntimeGuardedAdmission::new_with_policy(policy);
    cold.inventory_test_configure(true, 0);
    let mut optimized = RuntimeGuardedAdmission::new_with_policy(policy);
    let a = PredictionObservation::new();
    let b = PredictionObservation::new();
    let expected = ParserPrediction::compile_runtime_guarded_linear_repeats(
        grammar,
        &BTreeSet::new(),
        Some(context(grammar, &a)),
        None,
        &mut cold,
    );
    let actual = ParserPrediction::compile_runtime_guarded_linear_repeats(
        grammar,
        &BTreeSet::new(),
        Some(context(grammar, &b)),
        None,
        &mut optimized,
    );
    let expected = serde_json::to_value(expected).map_err(|e| e.to_string())?;
    let actual = serde_json::to_value(actual).map_err(|e| e.to_string())?;
    println!(
        "NULLABLE_INVENTORY_WHOLE {}",
        serde_json::json!({"cold":expected,"indexed":actual,"cold_work":format!("{:?}",a.first_reuse_work()),"indexed_work":format!("{:?}",b.first_reuse_work())})
    );
    assert_eq!(actual, expected);
    assert_eq!(
        serde_json::to_value(grammar).map_err(|e| e.to_string())?,
        before
    );
    Ok((actual, a, b))
}

#[test]
fn real_graph_recovery_inventory_removed_visits_exceed_all_paid_construction() -> Result<(), String>
{
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let mut grammar = fixture()?;
        let call = expr(&grammar, Kind::Call("Target".into()), '1');
        let mut repeated = expr(&grammar, Kind::SyntacticPredicate(Box::new(call)), '*');
        repeated.prediction_path = Some("rule/Root/s0".into());
        let mut target = grammar.rules["Root"].clone();
        target.body.kind = Kind::Literal("r".into());
        grammar.rules.insert("Target".into(), target);
        let source = grammar.declaration_sources["Root"].clone();
        grammar.declaration_sources.insert("Target".into(), source);
        let mut items = vec![repeated];
        for index in 0..100 {
            items.push(expr(&grammar, Kind::Literal(format!("v{index}")), '1'));
        }
        grammar
            .rules
            .get_mut("Root")
            .ok_or("Root missing")?
            .body
            .kind = Kind::Sequence(items);
        let (result, cold, indexed) = pair(&grammar, policy)?;
        assert!(result.get("Ok").is_some());
        let cold = cold.first_reuse_work().ok_or("Missing cold observation")?;
        let indexed = indexed
            .first_reuse_work()
            .ok_or("Missing indexed observation")?;
        assert!(cold.events[FirstReuseEvent::InventoryKey as usize].ok_or("Missing visits")? > 100);
        assert_eq!(
            indexed.events[FirstReuseEvent::InventoryKey as usize],
            Some(0)
        );
        // Exit remaining includes ALL earlier graph/index work as well as collect.
        assert!(
            indexed.exit_remaining.ok_or("Missing indexed balance")?
                > cold.exit_remaining.ok_or("Missing cold balance")?
        );
        assert!(!indexed.overflow && !indexed.multiple_programs);
    }
    Ok(())
}

#[test]
fn full_wrappers_calls_and_lexical_error_order_use_real_qualifier() -> Result<(), String> {
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let mut grammar = fixture()?;
        let mut items = vec![expr(&grammar, Kind::Literal("x".into()), '1'); 11];
        items[2] = full(&grammar, "Missing2");
        let mut nested = full(&grammar, "Missing10");
        nested.cardinality = '1';
        items[10] = expr(&grammar, Kind::Sequence(vec![nested]), '*');
        // Distinguishable first error: lexical s10 source failure precedes s2 unknown callee.
        if let Some(range) = &mut items[10].original_range {
            range.start = 1;
            range.end = 2;
        }
        grammar
            .rules
            .get_mut("Root")
            .ok_or("Root missing")?
            .body
            .kind = Kind::Sequence(items);
        let (result, _, observation) = pair(&grammar, policy)?;
        assert_eq!(
            observation
                .first_failure()
                .ok_or("Missing real first failure")?
                .decision
                .ok_or("Missing decision")?
                .as_str()
                .map_err(|e| e.to_string())?,
            "rule/Root/s10/repeat"
        );
        assert!(
            result["Err"]
                .as_str()
                .ok_or("Expected whole error")?
                .contains("Nullable Full group has unqualified original source boundaries")
        );
        let program = ParserPrediction::compile(&grammar)?;
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut work = 10_000;
        let mut builder = Builder::new(&grammar, &mut admission, &mut work)?;
        let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
            return Err("Wrong actual fixture".into());
        };
        builder.record(
            "Root",
            &grammar.rules["Root"],
            &items[2],
            "rule/Root/s2",
            &mut work,
        )?;
        builder.record(
            "Root",
            &grammar.rules["Root"],
            &items[10],
            "rule/Root/s10",
            &mut work,
        )?;
        let finished = builder.finish(&program, &mut work)?;
        assert_eq!(
            finished.paths.as_ref().ok_or("No completed paths")?,
            &["rule/Root/s10/repeat", "rule/Root/s2/repeat"]
        );
        drop(finished); // An unconsumed completion cannot be reused.
        assert!(admission.inventory_test_state().1);
        let mut scalar = full(&grammar, "Missing");
        if let Kind::SyntacticPredicate(assignment) = &mut scalar.kind
            && let Kind::Assign { value, .. } = &mut assignment.kind
        {
            value.kind = Kind::ScalarReference {
                target: "Root".into(),
                rule: "Missing".into(),
            };
        }
        assert!(
            !super::super::super::nullable_repetition::inventory_may_qualify(
                &grammar.rules["Root"],
                &scalar
            )
        );
        let mut predicated = full(&grammar, "Missing");
        predicated.first_set_predicated = true;
        assert!(
            !super::super::super::nullable_repetition::inventory_may_qualify(
                &grammar.rules["Root"],
                &predicated
            )
        );
    }
    Ok(())
}

#[test]
fn optional_owner_source_error_precedes_occurrence_pointer_equality() -> Result<(), String> {
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let mut grammar = fixture()?;
        let action = expr(&grammar, Kind::Action("Root".into()), '1');
        let literal = expr(&grammar, Kind::Literal("x".into()), '1');
        let optional = expr(
            &grammar,
            Kind::Assign {
                feature: "flag".into(),
                many: false,
                value: Box::new(literal),
            },
            '?',
        );
        grammar
            .rules
            .get_mut("Root")
            .ok_or("Root missing")?
            .body
            .kind = Kind::Sequence(vec![action, optional]);
        let (result, _, _) = pair(&grammar, policy)?;
        assert!(
            result["Err"]
                .as_str()
                .ok_or("Expected optional rejection")?
                .contains("do not support qualified nullable")
        );
        let mut unrelated = grammar.rules["Root"].body.clone();
        unrelated.kind = Kind::Literal("other".into());
        unrelated.cardinality = '+';
        assert!(
            super::super::super::nullable_repetition::inventory_may_qualify(
                &grammar.rules["Root"],
                &unrelated
            )
        );
        grammar.declaration_sources.clear();
        let mut work = 100;
        let error = grammar
            .qualified_nullable_decision_with_finite_work("rule/Root/s0/repeat", &mut work)
            .err()
            .ok_or("Expected source error before pointer equality")?;
        assert!(error.contains("no retained source identity"));
    }
    Ok(())
}

#[test]
fn malformed_owner_and_foreign_program_fall_back_to_complete_inventory() -> Result<(), String> {
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let mut grammar = fixture()?;
        let mut rule = grammar.rules["Root"].clone();
        rule.body.cardinality = '*';
        grammar.rules.insert("Bad/Owner".into(), rule);
        let (result, _, _) = pair(&grammar, policy)?;
        assert!(
            result["Err"]
                .as_str()
                .ok_or("Expected malformed owner error")?
                .contains("Unknown nullable decision owner")
        );
        let grammar = fixture()?;
        let program = ParserPrediction::compile(&grammar)?;
        let mut foreign = program.clone();
        foreign.decisions.insert(
            "rule/Missing/repeat".into(),
            super::super::Decision {
                entries: vec![],
                exit: None,
            },
        );
        let mut old = RuntimeGuardedAdmission::new_with_policy(policy);
        let expected = old.collect(&grammar, &foreign, &mut 10_000, None);
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut work = 10_000;
        let builder = Builder::new(&grammar, &mut admission, &mut work)?;
        let finished = builder.finish(&program, &mut work)?;
        let actual = finished.collect(&grammar, &foreign, &mut work, None);
        assert_eq!(actual, expected);
        assert!(actual.is_err());
        assert!(admission.inventory_test_state().1);
        let clone = grammar.clone();
        let mut baseline = RuntimeGuardedAdmission::new_with_policy(policy);
        let expected = baseline.collect(&clone, &foreign, &mut 10_000, None);
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut work = 10_000;
        let builder = Builder::new(&grammar, &mut admission, &mut work)?;
        let finished = builder.finish(&foreign, &mut work)?;
        assert_eq!(
            finished.collect(&clone, &foreign, &mut work, None),
            expected
        );
        assert!(admission.inventory_test_state().1);
    }
    Ok(())
}

#[test]
fn zero_short_header_copy_growth_order_and_companion_cap_never_publish_partial()
-> Result<(), String> {
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let mut grammar = fixture()?;
        let body = full(&grammar, "Missing");
        grammar.rules.get_mut("Root").ok_or("Root missing")?.body = body;
        let program = ParserPrediction::compile(&grammar)?;
        let mut reference = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut remaining = 10_000;
        let mut builder = Builder::new(&grammar, &mut reference, &mut remaining)?;
        // Exercise a replacement buffer and real compare/swap work, not only header.
        for suffix in ["s9", "s8", "s7", "s6", "s5", "s4"] {
            builder.record(
                "Root",
                &grammar.rules["Root"],
                &grammar.rules["Root"].body,
                &format!("rule/Root/{suffix}"),
                &mut remaining,
            )?;
        }
        drop(builder.finish(&program, &mut remaining)?);
        let exact = 10_000 - remaining;
        for allowance in 0..exact {
            let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
            let mut work = allowance;
            let result = (|| {
                let mut builder = Builder::new(&grammar, &mut admission, &mut work)?;
                for suffix in ["s9", "s8", "s7", "s6", "s5", "s4"] {
                    builder.record(
                        "Root",
                        &grammar.rules["Root"],
                        &grammar.rules["Root"].body,
                        &format!("rule/Root/{suffix}"),
                        &mut work,
                    )?;
                }
                let finished = builder.finish(&program, &mut work)?;
                drop(finished);
                Ok::<_, String>(())
            })();
            assert!(result.is_err());
            assert_eq!(work, 0);
            assert!(admission.inventory_test_state().1);
            assert_eq!(admission.decision_count(), 0);
        }
        let mut full_cap = RuntimeGuardedAdmission::new_with_policy(policy);
        full_cap.inventory_test_configure(false, 1_048_576 - HEADER + 1);
        assert!(Builder::new(&grammar, &mut full_cap, &mut 10_000).is_err());
        assert!(full_cap.inventory_test_state().1);
        assert_eq!(full_cap.decision_count(), 0);
        let mut at_cap = RuntimeGuardedAdmission::new_with_policy(policy);
        at_cap.inventory_test_configure(false, 1_048_576 - HEADER);
        let mut work = 10_000;
        let mut builder = Builder::new(&grammar, &mut at_cap, &mut work)?;
        assert!(
            builder
                .record(
                    "Root",
                    &grammar.rules["Root"],
                    &grammar.rules["Root"].body,
                    "rule/Root",
                    &mut work
                )
                .is_err()
        );
        assert!(builder.finish(&program, &mut work).is_err());
        assert_eq!(at_cap.inventory_test_state(), (1_048_576, true));
        let mut caught = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut builder = Builder::new(&grammar, &mut caught, &mut 10_000)?;
        assert!(
            builder
                .record(
                    "Root",
                    &grammar.rules["Root"],
                    &grammar.rules["Root"].body,
                    "rule/Root",
                    &mut 0
                )
                .is_err()
        );
        assert!(builder.finish(&program, &mut 10_000).is_err());
        assert!(caught.inventory_test_state().1);
    }
    Ok(())
}

#[test]
fn partial_builder_linear_and_none_routes_cannot_create_shortlist_authority() -> Result<(), String>
{
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let grammar = fixture()?;
        let program = ParserPrediction::compile(&grammar)?;
        let before = serde_json::to_value(&program).map_err(|e| e.to_string())?;
        let mut partial = RuntimeGuardedAdmission::new_with_policy(policy);
        drop(Builder::new(&grammar, &mut partial, &mut 10_000)?);
        assert!(partial.inventory_test_state().1);
        let mut duplicate = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut work = 10_000;
        let repeated = full(&grammar, "Missing");
        let mut builder = Builder::new(&grammar, &mut duplicate, &mut work)?;
        for _ in 0..2 {
            builder.record(
                "Root",
                &grammar.rules["Root"],
                &repeated,
                "rule/Root",
                &mut work,
            )?;
        }
        let finished = builder.finish(&program, &mut work)?;
        assert!(finished.paths.is_none());
        let mut baseline = RuntimeGuardedAdmission::new_with_policy(policy);
        assert_eq!(
            finished.collect(&grammar, &program, &mut work, None),
            baseline.collect(&grammar, &program, &mut 10_000, None)
        );
        let mut linear = RuntimeGuardedAdmission::new();
        let mut old = RuntimeGuardedAdmission::new();
        let mut a = 10_000;
        let mut b = 10_000;
        let mut collection = Collection::new(&grammar, &mut linear, &mut a)?;
        assert!(collection.builder().is_none());
        assert_eq!(a, b);
        let expected = old.collect(&grammar, &program, &mut b, None);
        let actual = collection.collect(&grammar, &program, &mut a, None);
        assert_eq!(actual, expected);
        assert_eq!(a, b);
        assert_eq!(linear.inventory_test_state(), old.inventory_test_state());
        let mut disabled = RuntimeGuardedAdmission::new_with_policy(policy);
        disabled.inventory_test_configure(true, 0);
        let mut work = 10_000;
        let mut collection = Collection::new(&grammar, &mut disabled, &mut work)?;
        assert!(collection.builder().is_none());
        drop(collection); // A graph error on the old disabled route must not poison it.
        assert_eq!(disabled.inventory_test_state(), (0, false));
        assert_eq!(work, 10_000);
        assert_eq!(
            serde_json::to_value(ParserPrediction::compile(&grammar)?).map_err(|e| e.to_string())?,
            before
        );
    }
    Ok(())
}

type OwnerBlockOrderOutcome = Result<(Result<(Vec<String>, bool), String>, usize), String>;

fn owner_block_order<const BLOCKS: bool>(keys: &[String], grant: usize) -> OwnerBlockOrderOutcome {
    let grammar = fixture()?;
    let mut admission = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    let mut setup = 10000;
    let mut builder = Builder::new(&grammar, &mut admission, &mut setup)?;
    let value = full(&grammar, "Root");
    for key in keys {
        let path = key.strip_suffix("/repeat").ok_or("test repeat key")?;
        let owner = path
            .strip_prefix("rule/")
            .and_then(|s| s.split('/').next())
            .unwrap_or("Root");
        builder.record(owner, &grammar.rules["Root"], &value, path, &mut setup)?;
    }
    let construction = 10000 - setup;
    let mut work = grant;
    let result = builder
        .order_with_owner_blocks::<BLOCKS>(&mut work)
        .map(|()| (builder.paths.clone(), builder.fallback));
    println!(
        "OWNER_BLOCK_ORDER_RAW {}",
        serde_json::json!({
            "blocks":BLOCKS,"input":keys,"result":result,"construction_work":construction,
            "ordering_work":grant-work,"total_work":construction+grant-work,
            "source_semantics_claim":false,
        })
    );
    Ok((result, grant - work))
}

#[test]
fn owner_blocks_same_full_keys_many_owners_and_real_paid_work() -> Result<(), String> {
    let mut keys = Vec::new();
    for owner in 0..64 {
        for index in [2, 0, 1] {
            keys.push(format!("rule/Owner{owner:03}/s{index}/repeat"));
        }
    }
    let old = owner_block_order::<false>(&keys, 10000)?;
    let new = owner_block_order::<true>(&keys, 10000)?;
    assert_eq!(new.0, old.0);
    assert!(new.0.as_ref().is_ok_and(|(_, fallback)| !fallback));
    assert!(
        new.1 < old.1,
        "all block scans and full-key guards must be repaid"
    );
    let mut expected = keys.clone();
    expected.sort();
    assert_eq!(new.0.as_ref().map(|x| &x.0), Ok(&expected));
    for grant in [0, new.1 - 1, new.1] {
        let actual = owner_block_order::<true>(&keys, grant)?;
        assert_eq!(actual.1, grant);
        if grant == new.1 {
            assert_eq!(actual.0, new.0)
        } else {
            assert!(actual.0.is_err_and(|e| e.contains("budget")))
        }
    }
    Ok(())
}

#[test]
fn owner_blocks_prefix_names_duplicates_single_owner_and_adversarial_fallback() -> Result<(), String>
{
    for raw in [
        vec![
            "rule/A/s2/repeat",
            "rule/A/s0/repeat",
            "rule/A.B/s1/repeat",
            "rule/A0/s0/repeat",
        ],
        vec![
            "rule/A0/s0/repeat",
            "rule/A/s0/repeat",
            "rule/A.B/s0/repeat",
        ],
        vec!["rule/A/s2/repeat", "rule/B/s0/repeat", "rule/A/s1/repeat"],
        vec!["rule/A/s1/repeat", "rule/A/s0/repeat", "rule/A/s0/repeat"],
        vec!["rule/A/s10/repeat", "rule/A/s2/repeat", "rule/A/s1/repeat"],
        vec!["rule/A/s0/repeat"],
        vec![],
        vec![
            "rule/Alpha/s0/repeat",
            "rule/\u{03b2}/s0/repeat",
            "rule/Alpha.Beta/s0/repeat",
        ],
    ] {
        let keys = raw.into_iter().map(String::from).collect::<Vec<_>>();
        let old = owner_block_order::<false>(&keys, 10000)?;
        let new = owner_block_order::<true>(&keys, 10000)?;
        assert_eq!(new.0, old.0, "complete keys/fallback {keys:?}");
    }
    Ok(())
}

#[test]
fn owner_blocks_budget_error_and_unwind_never_publish_partial_companion() -> Result<(), String> {
    let grammar = fixture()?;
    let value = full(&grammar, "Root");
    let mut admission = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    let mut setup = 10000;
    {
        let mut builder = Builder::new(&grammar, &mut admission, &mut setup)?;
        for owner in ["Owner0", "Owner1", "Owner2"] {
            for index in [2, 0, 1] {
                builder.record(
                    owner,
                    &grammar.rules["Root"],
                    &value,
                    &format!("rule/{owner}/s{index}"),
                    &mut setup,
                )?;
            }
        }
        let mut denied = 0;
        assert!(
            builder
                .finish(&ParserPrediction::compile(&grammar)?, &mut denied)
                .is_err()
        );
    }
    assert!(admission.inventory_test_state().1);
    // This grant fails after a real in-place extraction swap. The unpublished
    // companion still poisons on drop; no partial ordered list may escape.
    let mut partial = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    {
        let mut builder = Builder::new(&grammar, &mut partial, &mut setup)?;
        for owner in ["Owner0", "Owner1", "Owner2"] {
            for index in [2, 0, 1] {
                builder.record(
                    owner,
                    &grammar.rules["Root"],
                    &value,
                    &format!("rule/{owner}/s{index}"),
                    &mut setup,
                )?;
            }
        }
        let before = builder.paths.clone();
        assert!(builder.order_with_owner_blocks::<true>(&mut 20).is_err());
        assert_ne!(
            builder.paths, before,
            "actual partial sort occurred before denial"
        );
    }
    assert!(partial.inventory_test_state().1);
    let mut admission = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _builder = Builder::new(&grammar, &mut admission, &mut setup).unwrap();
        panic!("authored builder cancellation unwind");
    }));
    assert!(result.is_err());
    assert!(admission.inventory_test_state().1);
    Ok(())
}
