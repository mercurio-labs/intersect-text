use super::*;
use crate::grammar::parser_prediction::Decision;
use crate::grammar::parser_prediction::Edge;

fn state(edges: Vec<Edge>) -> State {
    State {
        edges,
        ..State::default()
    }
}
fn token(text: &str, target: usize) -> Edge {
    Edge::Token(PredictionSymbol::Keyword(text.into()), target)
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
fn leaf() -> ParserPrediction {
    program(vec![
        state(vec![Edge::Epsilon(1), Edge::Epsilon(2)]),
        state(vec![token("a", 3)]),
        state(vec![token("b", 3)]),
        State {
            rule_stop: true,
            ..State::default()
        },
        state(vec![Edge::Call {
            target: 0,
            follow: 5,
        }]),
        state(vec![token("left", 6)]),
        State::default(),
        state(vec![Edge::Call {
            target: 0,
            follow: 8,
        }]),
        state(vec![token("right", 9)]),
        State::default(),
    ])
}
fn candidates(entry: usize, stop: usize) -> CallFreeCandidates {
    let mut result = CallFreeCandidates::default();
    let mut fee = 1;
    result.record_rule(entry, stop, false, &mut fee).unwrap();
    assert_eq!(fee, 0);
    result
}
fn row(alternative: usize, calls: &[(usize, usize)]) -> AnalysisConfiguration {
    AnalysisConfiguration {
        state: 0,
        alternative,
        calls: calls.to_vec(),
    }
}

#[test]
fn call_free_builder_observes_the_entire_body_after_tokens_and_nested_wrappers() {
    use crate::grammar::Expr;
    use crate::grammar::Kind;
    fn expression(kind: Kind) -> Expr {
        Expr {
            kind,
            cardinality: '1',
            first_set_predicated: false,
            original_range: None,
            prediction_path: None,
        }
    }
    for cardinality in ['1', '?', '*', '+'] {
        let mut base = program(vec![State::default(); 4]);
        base.rules.insert("Nested".into(), (2, 3));
        base.states[1].rule_stop = true;
        base.states[3].rule_stop = true;
        let mut body = expression(Kind::Sequence(vec![
            expression(Kind::Literal("prefix".into())),
            expression(Kind::Assign {
                feature: "child".into(),
                many: false,
                value: Box::new(expression(Kind::SyntacticPredicate(Box::new(expression(
                    Kind::Call("Nested".into()),
                ))))),
            }),
        ]));
        body.cardinality = cardinality;
        let mut plain = base.clone();
        plain
            .expression(&body, 0, 1, "rule/Leaf", (0, None, None), None)
            .unwrap();
        let seen = std::cell::Cell::new(false);
        base.expression(&body, 0, 1, "rule/Leaf", (0, None, None), Some(&seen))
            .unwrap();
        assert!(seen.get());
        assert_eq!(
            serde_json::to_vec(&base).unwrap(),
            serde_json::to_vec(&plain).unwrap()
        );
        let mut candidate = CallFreeCandidates::default();
        candidate.record_rule(0, 1, seen.get(), &mut 0).unwrap();
        assert!(candidate.entries.is_empty());
    }
    let mut p = program(vec![State::default(); 2]);
    let seen = std::cell::Cell::new(false);
    p.expression(
        &expression(Kind::Call("ID".into())),
        0,
        1,
        "rule/Leaf",
        (0, None, None),
        Some(&seen),
    )
    .unwrap();
    assert!(
        !seen.get(),
        "a real terminal edge is not a hidden rule call"
    );
}

#[test]
fn call_free_retention_is_precharged_and_legacy_policy_is_zero_sized() {
    assert_eq!(std::mem::size_of::<super::super::NoRetention>(), 0);
    assert_eq!(std::mem::size_of::<TemplateRetention>(), 0);
    let mut value = CallFreeCandidates::default();
    assert_eq!(
        value.record_rule(0, 3, false, &mut 0),
        Err("Grammar prediction budget exceeded".into())
    );
    assert!(value.entries.is_empty());
    value.record_rule(0, 3, true, &mut 0).unwrap();
    assert!(value.entries.is_empty());
    value.record_rule(0, 3, false, &mut 1).unwrap();
    assert_eq!(
        value.record_rule(0, 3, false, &mut 1),
        Err("Duplicate call-free prediction rule entry".into())
    );
    assert_eq!(value.entries.len(), 1);
}

#[test]
fn call_free_miss_fifteen_hit_three_and_every_one_short_keeps_actual_partial_state() {
    let p = leaf();
    for allowance in 0..=15 {
        let mut cache = candidates(0, 3).freeze(&p.states);
        let mut tokens = BTreeMap::new();
        let mut remaining = allowance;
        let result = cache.replay_call(&p, &row(2, &[(4, 5)]), &mut tokens, &mut remaining);
        assert_eq!(remaining, 0);
        if allowance == 15 {
            assert_eq!(result, Ok(true));
            assert_eq!(tokens.len(), 2);
            let mut hit = BTreeMap::new();
            assert_eq!(
                cache.replay_call(&p, &row(2, &[(4, 5)]), &mut hit, &mut 3),
                Ok(true)
            );
            assert_eq!(hit, tokens);
        } else {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert_eq!(
                matches!(cache.entries[&0].ready, ClosureState::Ready(_)),
                allowance >= 13
            );
            assert_eq!(tokens.len(), usize::from(allowance == 14));
        }
    }
    let mut cache = candidates(0, 3).freeze(&p.states);
    assert!(
        cache
            .replay_call(&p, &row(0, &[]), &mut BTreeMap::new(), &mut 12)
            .is_err()
    );
    assert!(matches!(cache.entries[&0].ready, ClosureState::Uncomputed));
    assert_eq!(
        cache.replay_call(&p, &row(0, &[]), &mut BTreeMap::new(), &mut 15),
        Ok(true)
    );
    assert!(
        cache
            .replay_call(&p, &row(0, &[]), &mut BTreeMap::new(), &mut 2)
            .is_err()
    );
    assert!(matches!(cache.entries[&0].ready, ClosureState::Ready(_)));
}

#[test]
fn call_free_relative_rows_preserve_different_entire_suffixes_and_overlapping_seeds() {
    let p = leaf();
    let original = serde_json::to_vec(&p).unwrap();
    let mut cache = candidates(0, 3).freeze(&p.states);
    let a = row(0, &[(91, 92), (4, 5)]);
    let b = row(1, &[(93, 94), (7, 8)]);
    let mut tokens = BTreeMap::new();
    assert_eq!(cache.replay_call(&p, &a, &mut tokens, &mut 15), Ok(true));
    assert_eq!(cache.replay_call(&p, &a, &mut tokens, &mut 3), Ok(true));
    assert_eq!(cache.replay_call(&p, &b, &mut tokens, &mut 3), Ok(true));
    assert_eq!(tokens.len(), 2);
    for values in tokens.values() {
        assert_eq!(values.len(), 2);
        assert!(values.contains(&AnalysisConfiguration {
            state: 3,
            ..a.clone()
        }));
        assert!(values.contains(&AnalysisConfiguration {
            state: 3,
            ..b.clone()
        }));
    }
    assert_eq!(serde_json::to_vec(&p).unwrap(), original);
    let decision = Decision {
        entries: vec![4, 7],
        exit: None,
    };
    let mut old_witness = BTreeMap::new();
    let mut new_witness = BTreeMap::new();
    let old = p.multiple_recursive_alternatives(
        &decision,
        &mut 10_000,
        true,
        &mut old_witness,
        true,
        (true, None),
    );
    let new = p.multiple_recursive_alternatives_with_templates(
        &decision,
        &mut 10_000,
        true,
        &mut new_witness,
        true,
        (true, None, Some(&mut cache)),
    );
    assert_eq!(old, Ok(false));
    assert_eq!(new, old);
    assert_eq!(new_witness, old_witness);
}

#[test]
fn call_free_depth_zero_backstop_unknown_guard_and_malformed_graph_never_cache_errors() {
    for (edge, expected) in [
        (
            Edge::Call {
                target: 1,
                follow: 3,
            },
            "Grammar prediction budget exceeded",
        ),
        (
            Edge::First {
                path: "unknown".into(),
                target: 1,
            },
            "Guarded shared-prefix proof remains unsupported",
        ),
        (Edge::Epsilon(999), "Dangling parser prediction state"),
    ] {
        let mut p = leaf();
        p.states[0].edges = vec![edge];
        // Deliberately forge the private builder fact. The real walker is the
        // backstop; an asserted depth or fake NoCall flag is never sufficient.
        let mut cache = candidates(0, 3).freeze(&p.states);
        for _ in 0..2 {
            let mut remaining = 1000;
            assert_eq!(
                cache.replay_call(&p, &row(0, &[]), &mut BTreeMap::new(), &mut remaining),
                Err(expected.into())
            );
            assert!(remaining < 1000 && remaining > 0);
            assert!(matches!(cache.entries[&0].ready, ClosureState::Uncomputed));
        }
    }
    let p = leaf();
    let mut cache = candidates(0, 999).freeze(&p.states);
    assert_eq!(
        cache.replay_call(&p, &row(0, &[]), &mut BTreeMap::new(), &mut 1000),
        Err("Dangling call-free prediction rule stop".into())
    );
    assert!(matches!(cache.entries[&0].ready, ClosureState::Uncomputed));
}

#[test]
fn call_free_nullable_and_accepting_paths_do_not_follow_or_replace_the_caller() {
    for accepting in [false, true] {
        let mut p = leaf();
        p.states[0].edges = vec![Edge::Epsilon(3)];
        p.states[0].accepting = accepting;
        // A relative stop must not follow this unrelated/dangling caller.
        p.states[3].follow = vec![999];
        let mut cache = candidates(0, 3).freeze(&p.states);
        let mut tokens = BTreeMap::new();
        let mut work = 6;
        assert_eq!(
            cache.replay_call(&p, &row(0, &[(4, 5)]), &mut tokens, &mut work),
            Ok(false)
        );
        assert_eq!(work, 0);
        assert!(tokens.is_empty());
        assert!(matches!(
            cache.entries[&0].ready,
            ClosureState::CompleteIneligible
        ));
        // A different caller suffix still takes its own ordinary fallback.
        let mut warm = 1;
        assert_eq!(
            cache.replay_call(&p, &row(1, &[(7, 8), (4, 5)]), &mut tokens, &mut warm),
            Ok(false)
        );
        assert_eq!(warm, 0);
        assert!(tokens.is_empty());
    }
}

#[test]
fn call_free_foreign_graph_and_prefix_over128_fail_after_the_lookup_fee() {
    let p = leaf();
    let other = p.clone();
    let mut cache = candidates(0, 3).freeze(&p.states);
    let mut work = 1;
    assert_eq!(
        cache.replay_call(&other, &row(0, &[]), &mut BTreeMap::new(), &mut work),
        Err("Relative closure cache belongs to another prediction graph".into())
    );
    assert_eq!(work, 0);
    let mut work = 1;
    assert_eq!(
        cache.replay_call(
            &p,
            &row(0, &vec![(4, 5); 129]),
            &mut BTreeMap::new(),
            &mut work
        ),
        Err("Parser prediction call depth exceeded".into())
    );
    assert_eq!(work, 0);
    assert_eq!(
        cache.replay_call(
            &p,
            &row(0, &vec![(4, 5); 128]),
            &mut BTreeMap::new(),
            &mut 15
        ),
        Ok(true)
    );
}

fn recursive_with_leaf(leaf_first: bool) -> ParserPrediction {
    let mut first = vec![Edge::Call {
        target: 2,
        follow: 3,
    }];
    if leaf_first {
        first.insert(
            0,
            Edge::Call {
                target: 4,
                follow: 3,
            },
        );
    }
    program(vec![
        state(first),
        state(vec![
            Edge::Call {
                target: 2,
                follow: 3,
            },
            Edge::Call {
                target: 4,
                follow: 3,
            },
        ]),
        state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        State::default(),
        state(vec![Edge::Epsilon(5), Edge::Epsilon(6)]),
        state(vec![token("a", 7)]),
        state(vec![token("b", 7)]),
        State {
            rule_stop: true,
            ..State::default()
        },
    ])
}

#[test]
fn call_free_unrelated_early_true_keeps_original_call_and_witness_order() {
    let p = recursive_with_leaf(false);
    let decision = Decision {
        entries: vec![0, 1],
        exit: None,
    };
    let mut cache = candidates(4, 7).freeze(&p.states);
    let mut old = BTreeMap::new();
    let mut new = BTreeMap::new();
    assert_eq!(
        p.multiple_recursive_alternatives(&decision, &mut 8, true, &mut old, true, (true, None)),
        Ok(true)
    );
    assert_eq!(
        p.multiple_recursive_alternatives_with_templates(
            &decision,
            &mut 9,
            true,
            &mut new,
            true,
            (true, None, Some(&mut cache))
        ),
        Ok(true)
    );
    assert_eq!(old, new);
    assert_eq!(new, [((2, 128), true)].into());
    assert!(
        matches!(cache.entries[&4].ready, ClosureState::Uncomputed),
        "later leaf must remain untouched"
    );
}

#[test]
fn call_free_upfront_miss_can_deny_before_a_later_true_without_refund() {
    let p = recursive_with_leaf(true);
    let decision = Decision {
        entries: vec![0, 1],
        exit: None,
    };
    let mut cache = candidates(4, 7).freeze(&p.states);
    assert_eq!(
        p.multiple_recursive_alternatives(
            &decision,
            &mut 10,
            true,
            &mut BTreeMap::new(),
            true,
            (true, None)
        ),
        Ok(true)
    );
    let mut remaining = 10;
    assert_eq!(
        p.multiple_recursive_alternatives_with_templates(
            &decision,
            &mut remaining,
            true,
            &mut BTreeMap::new(),
            true,
            (true, None, Some(&mut cache))
        ),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(remaining, 0);
    assert!(matches!(cache.entries[&4].ready, ClosureState::Uncomputed));
    assert_eq!(
        p.multiple_recursive_alternatives_with_templates(
            &decision,
            &mut 100,
            true,
            &mut BTreeMap::new(),
            true,
            (true, None, Some(&mut cache))
        ),
        Ok(true)
    );
}

#[test]
fn call_free_shared_minimum_depth_and_live128_error_stay_in_the_original_call_arm() {
    let p = program(vec![
        state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        state(vec![Edge::Call {
            target: 4,
            follow: 3,
        }]),
        state(vec![Edge::Call {
            target: 4,
            follow: 3,
        }]),
        State::default(),
        state(vec![Edge::Call {
            target: 4,
            follow: 3,
        }]),
    ]);
    let decision = Decision {
        entries: vec![0, 1],
        exit: None,
    };
    let mut cache = CallFreeCandidates::default().freeze(&p.states);
    let mut witness = BTreeMap::new();
    assert_eq!(
        p.multiple_recursive_alternatives_with_templates(
            &decision,
            &mut 1000,
            true,
            &mut witness,
            true,
            (true, None, Some(&mut cache))
        ),
        Ok(true)
    );
    assert!(witness.contains_key(&(4, 127)));
    let p = program(
        (0..131)
            .map(|index| {
                state(vec![Edge::Call {
                    target: index + 1,
                    follow: 0,
                }])
            })
            .collect(),
    );
    let mut cache = CallFreeCandidates::default().freeze(&p.states);
    assert_eq!(
        p.multiple_recursive_alternatives_with_templates(
            &Decision {
                entries: vec![0, 0],
                exit: None
            },
            &mut 10_000,
            false,
            &mut BTreeMap::new(),
            true,
            (true, None, Some(&mut cache))
        ),
        Err("Parser prediction call depth exceeded".into())
    );
}

#[test]
fn adjacent_work_template_setup_replay_and_denial_are_actual_charge_sites() -> Result<(), String> {
    use crate::grammar::admission_observation::AdjacentCharge;
    use crate::grammar::admission_observation::AdjacentWorkObservation;
    use crate::grammar::admission_observation::RunStage;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let p = leaf();
    let source = DefinitionSource::new(
        "memory:template-work.xtext",
        SourceKind::Xtext,
        "Root: ->'a';",
    );
    for grant in 0..=18 {
        let run = |exact: Option<&AdjacentWorkObservation>| {
            let mut remaining = grant;
            let mut tokens = BTreeMap::new();
            let result = (|| {
                let mut candidates = CallFreeCandidates::default();
                candidates.record_rule_observed(0, 3, false, &mut remaining, exact)?;
                let mut cache = candidates.freeze(&p.states);
                cache.replay_call_observed(
                    &p,
                    &row(2, &[(4, 5)]),
                    &mut tokens,
                    &mut remaining,
                    exact,
                )
            })();
            (result, remaining, tokens)
        };
        let exact = AdjacentWorkObservation::new();
        exact.begin(RunStage::Executable, &source.id, grant);
        let a = run(Some(&exact));
        exact.end(a.1, a.0.is_ok());
        assert_eq!(a, run(None));
        let s = exact.snapshot();
        let last = s.last.ok_or("Missing template ledger")?;
        assert!(last.conserved && !s.inconsistent && !s.overflowed);
        if a.0 == Ok(true) {
            // Existing literal replay is 15; the original rule retention adds one.
            assert_eq!(grant - a.1, 16);
            for category in [
                AdjacentCharge::CallFreeRuleRetention,
                AdjacentCharge::TemplateLookup,
                AdjacentCharge::TemplateSeed,
                AdjacentCharge::TemplateStopCheck,
                AdjacentCharge::TemplatePublication,
            ] {
                assert_eq!(last.granted[category as usize], 1);
            }
            assert_eq!(last.granted[AdjacentCharge::TemplateEmission as usize], 2);
        } else {
            assert_eq!(last.denied.iter().sum::<u64>(), 1);
        }
    }
    Ok(())
}

#[test]
fn complete_ineligible_publication_is_paid_and_never_caches_a_partial_proof() {
    let mut p = leaf();
    p.states[0].edges = vec![Edge::Epsilon(3)];
    p.states[3].follow = vec![999];
    for grant in 0..=6 {
        let mut cache = candidates(0, 3).freeze(&p.states);
        let mut tokens = BTreeMap::new();
        let mut work = grant;
        let result = cache.replay_call(&p, &row(0, &[]), &mut tokens, &mut work);
        assert_eq!(work, 0);
        assert!(tokens.is_empty());
        if grant < 6 {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert!(matches!(cache.entries[&0].ready, ClosureState::Uncomputed));
            assert_eq!(
                cache.replay_call(&p, &row(0, &[]), &mut tokens, &mut 6),
                Ok(false)
            );
        } else {
            assert_eq!(result, Ok(false));
        }
        assert!(matches!(
            cache.entries[&0].ready,
            ClosureState::CompleteIneligible
        ));
        assert_eq!(
            cache.replay_call(&p, &row(0, &[]), &mut tokens, &mut 0),
            Err("Grammar prediction budget exceeded".into())
        );
        assert!(matches!(
            cache.entries[&0].ready,
            ClosureState::CompleteIneligible
        ));
        assert_eq!(
            cache.replay_call(&p, &row(0, &[]), &mut tokens, &mut 1),
            Ok(false)
        );
    }
}

#[test]
fn complete_ineligible_keeps_foreign_graph_and_actual_call_depth_guards() {
    let mut p = leaf();
    p.states[0].edges = vec![Edge::Epsilon(3)];
    let other = p.clone();
    let before = serde_json::to_vec(&p).unwrap();
    let mut cache = candidates(0, 999).freeze(&p.states);
    let mut tokens = BTreeMap::new();
    // The old cold ineligible route does not inspect the unused stop fact.
    assert_eq!(
        cache.replay_call(&p, &row(0, &[]), &mut tokens, &mut 6),
        Ok(false)
    );
    assert_eq!(
        cache.replay_call(&other, &row(0, &[]), &mut tokens, &mut 1),
        Err("Relative closure cache belongs to another prediction graph".into())
    );
    assert_eq!(
        cache.replay_call(&p, &row(0, &vec![(4, 5); 129]), &mut tokens, &mut 1),
        Err("Parser prediction call depth exceeded".into())
    );
    assert_eq!(
        cache.replay_call(&p, &row(0, &vec![(4, 5); 128]), &mut tokens, &mut 1),
        Ok(false)
    );
    assert!(tokens.is_empty());
    assert_eq!(serde_json::to_vec(&p).unwrap(), before);
}

#[test]
fn complete_ineligible_empty_action_and_optional_token_proofs_keep_fallback() {
    for edges in [
        vec![],
        vec![Edge::Action(3)],
        vec![Edge::Epsilon(3), token("a", 3)],
    ] {
        let mut p = leaf();
        p.states[0].edges = edges;
        let mut cache = candidates(0, 3).freeze(&p.states);
        let mut tokens = BTreeMap::new();
        assert_eq!(
            cache.replay_call(&p, &row(0, &[]), &mut tokens, &mut 100),
            Ok(false)
        );
        assert!(matches!(
            cache.entries[&0].ready,
            ClosureState::CompleteIneligible
        ));
        assert_eq!(
            cache.replay_call(&p, &row(1, &[(7, 8)]), &mut tokens, &mut 1),
            Ok(false)
        );
        assert!(tokens.is_empty());
    }
}

#[test]
fn complete_ineligible_reuses_only_eligibility_and_full_classifier_results_agree() {
    let p = program(vec![
        state(vec![Edge::Call {
            target: 2,
            follow: 3,
        }]),
        state(vec![Edge::Call {
            target: 2,
            follow: 4,
        }]),
        state(vec![Edge::Epsilon(5)]),
        state(vec![token("a", 6)]),
        state(vec![token("b", 6)]),
        State {
            rule_stop: true,
            ..State::default()
        },
        State::default(),
    ]);
    let decision = Decision {
        entries: vec![0, 1],
        exit: None,
    };
    let before = serde_json::to_vec(&p).unwrap();
    let mut cache = candidates(2, 5).freeze(&p.states);
    let mut old_witnesses = BTreeMap::new();
    let old = p.multiple_recursive_alternatives(
        &decision,
        &mut 1000,
        true,
        &mut old_witnesses,
        true,
        (true, None),
    );
    let mut costs = Vec::new();
    for _ in 0..2 {
        let mut witnesses = BTreeMap::new();
        let mut remaining = 1000;
        let actual = p.multiple_recursive_alternatives_with_templates(
            &decision,
            &mut remaining,
            true,
            &mut witnesses,
            true,
            (true, None, Some(&mut cache)),
        );
        assert_eq!(actual, old);
        assert_eq!(actual, Ok(false));
        assert_eq!(witnesses, old_witnesses);
        costs.push(1000 - remaining);
    }
    assert!(
        costs[1] < costs[0],
        "the second classifier must avoid the completed proof"
    );
    assert!(matches!(
        cache.entries[&2].ready,
        ClosureState::CompleteIneligible
    ));
    assert_eq!(serde_json::to_vec(&p).unwrap(), before);
}

#[test]
fn complete_ineligible_observer_parity_covers_publication_and_warm_lookup() -> Result<(), String> {
    use crate::grammar::admission_observation::AdjacentWorkObservation;
    use crate::grammar::admission_observation::RunStage;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    let mut p = leaf();
    p.states[0].edges = vec![Edge::Epsilon(3)];
    p.states[3].follow = vec![999];
    let source = DefinitionSource::new(
        "memory:ineligible-template.xtext",
        SourceKind::Xtext,
        "Root: Leaf; Leaf: {Empty};",
    );
    for grant in 0..=9 {
        let run = |exact: Option<&AdjacentWorkObservation>| {
            let mut remaining = grant;
            let mut tokens = BTreeMap::new();
            let result = (|| {
                let mut candidates = CallFreeCandidates::default();
                candidates.record_rule_observed(0, 3, false, &mut remaining, exact)?;
                let mut cache = candidates.freeze(&p.states);
                assert!(!cache.replay_call_observed(
                    &p,
                    &row(0, &[(4, 5)]),
                    &mut tokens,
                    &mut remaining,
                    exact
                )?);
                cache.replay_call_observed(
                    &p,
                    &row(1, &[(7, 8)]),
                    &mut tokens,
                    &mut remaining,
                    exact,
                )
            })();
            (result, remaining, tokens)
        };
        let exact = AdjacentWorkObservation::new();
        exact.begin(RunStage::Executable, &source.id, grant);
        let actual = run(Some(&exact));
        exact.end(actual.1, actual.0.is_ok());
        assert_eq!(actual, run(None));
        let snapshot = exact.snapshot();
        let last = snapshot.last.ok_or("Missing completed-ineligible ledger")?;
        assert!(last.conserved && !snapshot.inconsistent && !snapshot.overflowed);
        if actual.0 == Ok(false) {
            assert_eq!(grant - actual.1, 8);
            assert!(actual.2.is_empty());
            for (category, count) in [
                (AdjacentCharge::CallFreeRuleRetention, 1),
                (AdjacentCharge::TemplateLookup, 2),
                (AdjacentCharge::TemplateSeed, 1),
                (AdjacentCharge::TemplatePublication, 1),
                (AdjacentCharge::TemplateStopCheck, 0),
                (AdjacentCharge::TemplateEmission, 0),
            ] {
                assert_eq!(last.granted[category as usize], count);
            }
        } else {
            assert_eq!(last.denied.iter().sum::<u64>(), 1);
        }
    }
    Ok(())
}
