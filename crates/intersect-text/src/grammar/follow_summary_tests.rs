use super::*;
use crate::grammar::Profile;
use crate::grammar::Rule;

fn expr(kind: Kind) -> Expr {
    Expr {
        kind,
        cardinality: '1',
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
    }
}
fn grammar(body: Expr) -> Grammar {
    Grammar {
        call_depth: None,
        analysis_work_budget: Default::default(),
        parameterized_rules: Default::default(),
        final_rules: Default::default(),
        computed_settings: vec![],
        profile: Profile::NativeV70,
        name: "Summaries".into(),
        entry: "F".into(),
        rules: BTreeMap::from([(
            "F".into(),
            Rule {
                hidden: None,
                enum_literals: BTreeMap::new(),
                nominal_return: None,
                inferred_return: false,
                override_declared: false,
                datatype: None,
                fragment: true,
                wildcard: false,
                class: "Node".into(),
                body,
            },
        )]),
        terminals: BTreeMap::new(),
        hidden: None,
        lexical_prediction: None,
        parser_prediction: None,
        declaration_sources: BTreeMap::new(),
        terminal_order: vec![],
        shadowed_declarations: BTreeMap::new(),
    }
}
fn names(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn summaries_literal_miss_fees_partial_outputs_and_error_retries_are_exact() {
    for (body, success_work, rows) in [
        (
            expr(Kind::Literal("x".into())),
            3,
            vec![
                (3, None, vec![]),
                (2, Some("retain"), vec![]),
                (1, Some("body"), vec![]),
                (0, Some("lookup"), vec![]),
            ],
        ),
        (
            expr(Kind::Call("G".into())),
            4,
            vec![
                (4, None, vec!["G"]),
                (3, Some("retain"), vec!["G"]),
                (2, Some("edge"), vec![]),
                (1, Some("body"), vec![]),
                (0, Some("lookup"), vec![]),
            ],
        ),
        (
            expr(Kind::Sequence(vec![
                expr(Kind::Call("B".into())),
                expr(Kind::Call("A".into())),
            ])),
            7,
            vec![
                (7, None, vec!["A", "B"]),
                (6, Some("retain"), vec!["A", "B"]),
                (5, Some("edge"), vec!["A"]),
            ],
        ),
    ] {
        let grammar = grammar(body);
        for (initial, operation, expected_pending) in rows {
            let mut cache = RuleCallSummaries::new(&grammar);
            let mut pending = BTreeSet::new();
            let mut work = initial;
            let mut failure = None;
            let result = cache.extend(Some("F"), &mut pending, &mut work, Some(&mut failure));
            assert_eq!(work, 0);
            assert_eq!(pending, names(&expected_pending));
            assert_eq!(cache.summaries.len(), usize::from(operation.is_none()));
            if let Some(operation) = operation {
                assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
                let measured = match failure.unwrap() {
                    SummaryFailure::Lookup { remaining_work } => {
                        assert_eq!(remaining_work, 0);
                        "lookup"
                    }
                    SummaryFailure::Body(failure) => {
                        assert_eq!(failure.reason, CallWalkLimit::Work);
                        assert_eq!(failure.remaining_work, 0);
                        "body"
                    }
                    SummaryFailure::Edge { remaining_work } => {
                        assert_eq!(remaining_work, 0);
                        "edge"
                    }
                    SummaryFailure::Retention { remaining_work } => {
                        assert_eq!(remaining_work, 0);
                        "retain"
                    }
                };
                assert_eq!(measured, operation);
                // The same failed cache still misses. Retain partial output;
                // duplicates must pay emission again, not receive a refund.
                let mut retry_work = success_work;
                assert_eq!(
                    cache.extend(Some("F"), &mut pending, &mut retry_work, None),
                    Ok(())
                );
                assert_eq!(retry_work, 0);
                assert_eq!(cache.summaries.len(), 1);
            } else {
                assert_eq!(result, Ok(()));
                assert_eq!(failure, None);
            }
        }
    }
}

#[test]
fn summaries_hit_fees_duplicates_and_failed_hit_preserve_valid_entry() {
    for (body, miss, hit, targets) in [
        (expr(Kind::Literal("x".into())), 3, 1, vec![]),
        (expr(Kind::Call("G".into())), 4, 2, vec!["G"]),
    ] {
        let grammar = grammar(body);
        let mut cache = RuleCallSummaries::new(&grammar);
        let mut warm = miss;
        assert_eq!(
            cache.extend(Some("F"), &mut BTreeSet::new(), &mut warm, None),
            Ok(())
        );
        let old = cache.summaries.clone();
        let mut pending = names(&targets);
        let mut work = hit;
        assert_eq!(
            cache.extend(Some("F"), &mut pending, &mut work, None),
            Ok(())
        );
        assert_eq!(work, 0);
        assert_eq!(pending, names(&targets));
        assert_eq!(cache.summaries, old);
        for initial in 0..hit {
            let mut pending = BTreeSet::new();
            let mut work = initial;
            let mut failure = None;
            assert_eq!(
                cache.extend(Some("F"), &mut pending, &mut work, Some(&mut failure)),
                Err("Grammar prediction budget exceeded".into())
            );
            assert_eq!(work, 0);
            assert!(pending.is_empty());
            assert_eq!(cache.summaries, old);
            assert_eq!(
                failure,
                Some(if initial == 0 {
                    SummaryFailure::Lookup { remaining_work: 0 }
                } else {
                    SummaryFailure::Edge { remaining_work: 0 }
                })
            );
        }
    }
}

#[test]
fn summaries_keep_original_body_depth_boundary_work_precedence_and_no_error_entry() {
    for (depth, initial, reason) in [
        (64, 67, None),
        (65, 67, Some(CallWalkLimit::SyntaxDepth)),
        (65, 66, Some(CallWalkLimit::Work)),
    ] {
        let mut body = expr(Kind::Literal("x".into()));
        for _ in 0..depth {
            body = expr(Kind::Sequence(vec![body]));
        }
        let grammar = grammar(body);
        let mut cache = RuleCallSummaries::new(&grammar);
        let mut pending = BTreeSet::new();
        let mut work = initial;
        let mut failure = None;
        let result = cache.extend(Some("F"), &mut pending, &mut work, Some(&mut failure));
        assert_eq!(work, 0);
        assert!(pending.is_empty());
        if let Some(reason) = reason {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert_eq!(
                failure,
                Some(SummaryFailure::Body(CallWalkFailure {
                    reason,
                    remaining_work: 0,
                    syntax_depth: 65
                }))
            );
            assert!(cache.summaries.is_empty());
            let mut retry = 100;
            assert_eq!(
                cache.extend(Some("F"), &mut pending, &mut retry, None),
                Err("Grammar prediction budget exceeded".into())
            );
            assert_eq!(retry, 33); // lookup1 +66 real nodes; not a cached error.
            assert!(cache.summaries.is_empty());
        } else {
            assert_eq!(result, Ok(()));
            assert_eq!(failure, None);
            assert_eq!(cache.summaries.len(), 1);
        }
    }
}

#[test]
fn summaries_charge_lookup_before_invalid_effective_key_without_panics_or_entries() {
    let grammar = grammar(expr(Kind::Literal("x".into())));
    for (key, message) in [
        (
            Some("Missing"),
            "Unknown effective rule for fragment call summary: Missing",
        ),
        (
            None,
            "Missing effective rule identity for fragment call summary",
        ),
    ] {
        let mut cache = RuleCallSummaries::new(&grammar);
        let mut work = 1;
        let mut pending = BTreeSet::new();
        assert_eq!(
            cache.extend(key, &mut pending, &mut work, None),
            Err(message.into())
        );
        assert_eq!(work, 0);
        assert!(cache.summaries.is_empty() && pending.is_empty());
        assert_eq!(
            cache.extend(key, &mut pending, &mut work, None),
            Err("Grammar prediction budget exceeded".into())
        );
        assert!(cache.summaries.is_empty() && pending.is_empty());
    }
}

#[test]
fn summaries_optional_failure_capture_preserves_whole_cache_output_and_work() {
    let grammar = grammar(expr(Kind::Sequence(vec![
        expr(Kind::Call("B".into())),
        expr(Kind::Call("A".into())),
    ])));
    for initial in 0..=8 {
        let mut plain = RuleCallSummaries::new(&grammar);
        let mut traced = RuleCallSummaries::new(&grammar);
        let mut a = BTreeSet::new();
        let mut b = BTreeSet::new();
        let mut aw = initial;
        let mut bw = initial;
        let mut failure = None;
        assert_eq!(
            plain.extend(Some("F"), &mut a, &mut aw, None),
            traced.extend(Some("F"), &mut b, &mut bw, Some(&mut failure))
        );
        assert_eq!(a, b);
        assert_eq!(aw, bw);
        assert_eq!(plain.summaries, traced.summaries);
    }
}
