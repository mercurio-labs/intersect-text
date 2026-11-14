use super::*;
use crate::grammar::Kind;
use crate::grammar::Profile;
use crate::grammar::Rule;
use crate::grammar::first;
use serde_json::json;

fn expr(kind: Kind) -> Expr {
    Expr {
        kind,
        cardinality: '1',
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
    }
}
fn literal(text: &str) -> Expr {
    expr(Kind::Literal(text.into()))
}
fn optional(mut value: Expr) -> Expr {
    value.cardinality = '?';
    value
}
fn make_grammar(body: Expr) -> Grammar {
    let mut grammar: Grammar = serde_json::from_value(json!({
        "profile":Profile::NativeV72,"name":"Frontier","entry":"Root","rules":{}
    }))
    .unwrap();
    let root: Rule = serde_json::from_value(json!({"class":"Root","body":body})).unwrap();
    grammar.rules.insert("Root".into(), root);
    let target: Rule = serde_json::from_value(json!({
        "class":"Target","body":optional(literal("x"))
    }))
    .unwrap();
    grammar.rules.insert("Target".into(), target);
    grammar
}
fn tokens(symbols: &[&str], caller: bool) -> Tokens {
    Tokens {
        symbols: symbols.iter().map(|s| (*s).into()).collect(),
        caller,
    }
}
fn build<'g>(
    grammar: &'g Grammar,
    outer: &Tokens,
    remaining: &mut usize,
    plan: &mut ValidationPlan<'g>,
) -> Result<(), String> {
    super::super::walk_recorded(
        &grammar.rules["Root"].body,
        outer,
        grammar,
        &mut BTreeMap::new(),
        (false, "rule/Root", None, Some("Root")),
        remaining,
        (&mut BTreeSet::new(), None, Some(plan), None),
    )
}
fn projected(required: &[RequiredObjectCall]) -> Vec<(&str, &str, &str)> {
    required
        .iter()
        .map(|r| (r.owner.as_str(), r.callee.as_str(), r.path.as_str()))
        .collect()
}

#[test]
fn consuming_sequence_has_literal_n4_k0_and_no_replay_fee() {
    let grammar = make_grammar(expr(Kind::Sequence(vec![
        literal("a"),
        literal("b"),
        literal("c"),
    ])));
    let mut plan = ValidationPlan::new(&grammar);
    let mut exact = 4;
    assert_eq!(
        build(&grammar, &tokens(&[], true), &mut exact, &mut plan),
        Ok(())
    );
    assert_eq!(exact, 0);
    assert!(plan.frontiers.is_empty());
    assert_eq!(
        plan.replay(&BTreeMap::new(), &mut BTreeSet::new(), None, &mut 0, None),
        Ok(())
    );
    let mut old = 8;
    for validate in [false, true] {
        super::super::walk(
            &grammar.rules["Root"].body,
            &tokens(&[], true),
            &grammar,
            &mut BTreeMap::new(),
            (validate, "rule/Root", None, Some("Root")),
            &mut old,
            (&mut BTreeSet::new(), None),
        )
        .unwrap();
    }
    assert_eq!(old, 0);
    let mut short = 3;
    let mut failed = ValidationPlan::new(&grammar);
    assert_eq!(
        build(&grammar, &tokens(&[], true), &mut short, &mut failed),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(short, 0);
    assert!(failed.frontiers.is_empty());
}

#[test]
fn optional_single_has_literal_n1_k1_retention_and_replay_boundary() {
    let grammar = make_grammar(optional(literal("x")));
    for limit in [0, 1] {
        let mut remaining = limit;
        let mut plan = ValidationPlan::new(&grammar);
        assert_eq!(
            build(&grammar, &tokens(&[], true), &mut remaining, &mut plan),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(remaining, 0);
        assert!(plan.frontiers.is_empty());
    }
    let mut plan = ValidationPlan::new(&grammar);
    let mut remaining = 3;
    build(&grammar, &tokens(&[], true), &mut remaining, &mut plan).unwrap();
    assert_eq!(remaining, 1);
    assert_eq!(plan.frontiers.len(), 1);
    assert_eq!(plan.frontiers[0].trigger, FrontierTrigger::Caller);
    let follow = [("Root".into(), tokens(&["literal:x"], false))].into();
    let mut deferred = BTreeSet::new();
    assert_eq!(
        plan.replay(&follow, &mut deferred, None, &mut 0, None),
        Err("Grammar prediction budget exceeded".into())
    );
    assert!(deferred.is_empty());
    plan.replay(&follow, &mut deferred, None, &mut remaining, None)
        .unwrap();
    assert_eq!(remaining, 0);
    assert_eq!(deferred, ["rule/Root/repeat".into()].into());
    assert_eq!(plan.frontiers.len(), 1);
}

#[test]
fn constant_numeric_overlap_uses_existing_union_rule_without_retained_symbols() {
    let grammar = make_grammar(optional(expr(Kind::Call("INT".into()))));
    for (constants, caller, trigger) in [
        (vec!["literal:12"], false, Some(FrontierTrigger::Always)),
        (vec!["literal:y"], true, Some(FrontierTrigger::Caller)),
        (vec!["literal:y"], false, None),
    ] {
        let mut plan = ValidationPlan::new(&grammar);
        let mut remaining = 10;
        build(
            &grammar,
            &tokens(&constants, caller),
            &mut remaining,
            &mut plan,
        )
        .unwrap();
        assert_eq!(plan.frontiers.first().map(|f| f.trigger), trigger);
        assert_eq!(remaining, if trigger.is_some() { 8 } else { 9 });
        for incoming in [vec!["literal:12"], vec!["literal:y"], vec!["terminal:INT"]] {
            let mut resolved = tokens(&constants, false);
            if caller {
                resolved.extend(tokens(&incoming, false));
            }
            let mut expected = BTreeSet::new();
            super::super::walk(
                &grammar.rules["Root"].body,
                &resolved,
                &grammar,
                &mut BTreeMap::new(),
                (true, "rule/Root", None, Some("Root")),
                &mut 100,
                (&mut expected, None),
            )
            .unwrap();
            let mut actual = BTreeSet::new();
            let follow = [("Root".into(), tokens(&incoming, false))].into();
            plan.replay(&follow, &mut actual, None, &mut 100, None)
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn required_occurrences_keep_reverse_order_partial_outputs_and_existing_fees() {
    let grammar = make_grammar(expr(Kind::Sequence(vec![
        expr(Kind::Call("Target".into())),
        expr(Kind::Call("Target".into())),
        literal("x"),
    ])));
    let mut plan = ValidationPlan::new(&grammar);
    let mut build_work = 6; // N4 + K2; both actual local continuations conflict.
    build(&grammar, &tokens(&[], true), &mut build_work, &mut plan).unwrap();
    assert_eq!(build_work, 0);
    assert_eq!(
        plan.frontiers
            .iter()
            .map(|f| (f.path.as_str(), f.trigger))
            .collect::<Vec<_>>(),
        vec![
            ("rule/Root/s1", FrontierTrigger::Always),
            ("rule/Root/s0", FrontierTrigger::Always)
        ]
    );
    let follow = [("Root".into(), tokens(&[], false))].into();
    for limit in [0, 1, 2, 5] {
        let mut remaining = limit;
        let mut required = Vec::new();
        assert_eq!(
            plan.replay(
                &follow,
                &mut BTreeSet::new(),
                Some(&mut required),
                &mut remaining,
                None
            ),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(remaining, 0);
        assert_eq!(
            projected(&required),
            if limit == 5 {
                vec![("Root", "Target", "rule/Root/s1")]
            } else {
                vec![]
            }
        );
    }
    for _ in 0..2 {
        let mut required = Vec::new();
        let mut exact = 6; // each replay1 + eligibility1 + retention1.
        plan.replay(
            &follow,
            &mut BTreeSet::new(),
            Some(&mut required),
            &mut exact,
            None,
        )
        .unwrap();
        assert_eq!(exact, 0);
        assert_eq!(
            projected(&required),
            vec![
                ("Root", "Target", "rule/Root/s1"),
                ("Root", "Target", "rule/Root/s0")
            ]
        );
    }
}

#[test]
fn invalid_target_checks_remain_conditional_and_after_charged_replay() {
    let mut grammar = make_grammar(expr(Kind::Call("Target".into())));
    grammar.rules.get_mut("Target").unwrap().datatype = Some(crate::grammar::Datatype::EString);
    let mut plan = ValidationPlan::new(&grammar);
    build(&grammar, &tokens(&[], true), &mut 2, &mut plan).unwrap();
    let no_conflict = [("Root".into(), tokens(&["literal:y"], false))].into();
    assert_eq!(
        plan.replay(
            &no_conflict,
            &mut BTreeSet::new(),
            Some(&mut Vec::new()),
            &mut 1,
            None
        ),
        Ok(())
    );
    let conflict = [("Root".into(), tokens(&["literal:x"], false))].into();
    assert_eq!(
        plan.replay(
            &conflict,
            &mut BTreeSet::new(),
            Some(&mut Vec::new()),
            &mut 1,
            None
        ),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(
        plan.replay(
            &conflict,
            &mut BTreeSet::new(),
            Some(&mut Vec::new()),
            &mut 2,
            None
        ),
        Err("Required nullable call is not a nonfragment object rule".into())
    );
    assert_eq!(
        plan.replay(&BTreeMap::new(), &mut BTreeSet::new(), None, &mut 0, None),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(
        plan.replay(&BTreeMap::new(), &mut BTreeSet::new(), None, &mut 1, None),
        Err("Missing resolved FOLLOW frontier: Root".into())
    );
}

#[test]
fn guarded_wrappers_cardinalities_and_alternative_paths_match_concrete_walk() {
    let mut guarded = optional(literal("x"));
    guarded.first_set_predicated = true;
    let mut cases = vec![
        expr(Kind::Action("Root".into())),
        expr(Kind::Sequence(vec![optional(literal("x")), literal("z")])),
        expr(Kind::Alternative(vec![
            optional(literal("x")),
            optional(literal("y")),
        ])),
        expr(Kind::Assign {
            feature: "child".into(),
            many: false,
            value: Box::new(optional(literal("x"))),
        }),
        expr(Kind::BooleanAssign {
            feature: "flag".into(),
            value: Box::new(optional(literal("x"))),
        }),
        expr(Kind::SyntacticPredicate(Box::new(optional(literal("x"))))),
        guarded,
    ];
    for cardinality in ['1', '?', '*', '+'] {
        let mut value = expr(Kind::Sequence(vec![literal("x")]));
        value.cardinality = cardinality;
        cases.push(value);
    }
    for body in cases {
        let grammar = make_grammar(body);
        for local in [vec![], vec!["literal:x"]] {
            for caller in [false, true] {
                let symbolic = tokens(&local, caller);
                let mut plan = ValidationPlan::new(&grammar);
                build(&grammar, &symbolic, &mut 1000, &mut plan).unwrap();
                for incoming in [vec![], vec!["literal:x"], vec!["literal:y"]] {
                    let mut resolved = tokens(&local, false);
                    if caller {
                        resolved.extend(tokens(&incoming, false));
                    }
                    let mut expected = BTreeSet::new();
                    let legacy = super::super::walk(
                        &grammar.rules["Root"].body,
                        &resolved,
                        &grammar,
                        &mut BTreeMap::new(),
                        (true, "rule/Root", None, Some("Root")),
                        &mut 1000,
                        (&mut expected, None),
                    );
                    let mut actual = BTreeSet::new();
                    let follow = [("Root".into(), tokens(&incoming, false))].into();
                    let new = plan.replay(&follow, &mut actual, None, &mut 1000, None);
                    assert_eq!(new, legacy);
                    assert_eq!(actual, expected);
                }
            }
        }
    }
}

#[test]
fn canonical_grammar_owner_and_retry_failure_have_paid_precedence() {
    let grammar = make_grammar(optional(literal("x")));
    let foreign = grammar.clone();
    let body = &grammar.rules["Root"].body;
    let (head, nullable) = first(body, &grammar, 0).unwrap();
    let mut plan = ValidationPlan::new(&grammar);
    for (owner, expected) in [
        (None, "FOLLOW frontier has no actual validation owner"),
        (Some("Missing"), "Unknown FOLLOW frontier owner: Missing"),
    ] {
        for _ in 0..2 {
            assert_eq!(
                plan.record(
                    (body, &grammar),
                    &tokens(&[], true),
                    (&head, nullable),
                    (owner, "rule/Root"),
                    &mut 0
                ),
                Err("Grammar prediction budget exceeded".into())
            );
            let mut exact = 1;
            assert_eq!(
                plan.record(
                    (body, &grammar),
                    &tokens(&[], true),
                    (&head, nullable),
                    (owner, "rule/Root"),
                    &mut exact
                ),
                Err(expected.into())
            );
            assert_eq!(exact, 0);
            assert!(plan.frontiers.is_empty());
        }
    }
    assert_eq!(
        plan.record(
            (&foreign.rules["Root"].body, &foreign),
            &tokens(&[], true),
            (&head, nullable),
            (Some("Root"), "rule/Root"),
            &mut 1
        ),
        Err("FOLLOW frontier belongs to a different grammar".into())
    );
    plan.frontiers.push(ValidationFrontier {
        owner: "Missing",
        expression: body,
        path: "rule/Missing".into(),
        trigger: FrontierTrigger::Caller,
    });
    assert_eq!(
        plan.replay(&BTreeMap::new(), &mut BTreeSet::new(), None, &mut 0, None),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(
        plan.replay(&BTreeMap::new(), &mut BTreeSet::new(), None, &mut 1, None),
        Err("Unknown FOLLOW frontier owner: Missing".into())
    );
}

#[test]
fn failed_collection_keeps_prior_records_and_first_limits_do_not_gain_credit() {
    let grammar = make_grammar(expr(Kind::Alternative(vec![
        optional(literal("x")),
        optional(literal("y")),
    ])));
    let mut plan = ValidationPlan::new(&grammar);
    let mut short = 4; // outer1, first child visit1+retain1, next visit1, no retention credit.
    assert_eq!(
        build(&grammar, &tokens(&[], true), &mut short, &mut plan),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(short, 0);
    assert_eq!(
        plan.frontiers
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        vec!["rule/Root/a0"]
    );
    let mut deep = literal("x");
    for _ in 0..65 {
        deep = expr(Kind::Sequence(vec![deep]));
    }
    let grammar = make_grammar(deep);
    let mut plan = ValidationPlan::new(&grammar);
    let mut remaining = 100;
    assert_eq!(
        build(&grammar, &tokens(&[], true), &mut remaining, &mut plan),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(remaining, 99); // failure is independent FIRST depth, not FOLLOW100.
    assert!(plan.frontiers.is_empty());
}

#[test]
fn actual_source_declaration_owner_is_preserved_at_shared_conflict_error() {
    use crate::grammar::SourceLimits;
    use crate::grammar::admission_observation::*;
    use crate::grammar::syntax;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    let source = DefinitionSource::new(
        "memory:follow/frontier.xtext",
        SourceKind::Xtext,
        "grammar t\nimport \"urn:t\" as m\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nRoot returns m::Root: child=Target 'x';\nTarget returns ecore::EString: 'x'?;\n",
    );
    let parsed = syntax::parse(
        &source,
        SourceLimits {
            source_bytes: source.text.len(),
            declarations: 2,
        },
    )
    .unwrap();
    let declaration = parsed
        .rules
        .iter()
        .find(|r| r.value.name.value == "Root")
        .unwrap();
    let syntax::ElementKind::Sequence(items) = &declaration.value.body.value else {
        panic!("authored sequence")
    };
    let syntax::ElementKind::Assignment {
        feature,
        operator,
        value,
    } = &items[0].value
    else {
        panic!("authored assignment")
    };
    assert_eq!((feature.value.as_str(), operator.as_str()), ("child", "="));
    assert!(matches!(&value.value,syntax::ElementKind::Call{rule,..} if rule.value=="Target"));
    let syntax::ElementKind::Keyword(keyword) = &items[1].value else {
        panic!("authored continuation")
    };
    assert_eq!(keyword.to_scalar_string().unwrap(), "x");
    let mut grammar = make_grammar(expr(Kind::Sequence(vec![
        expr(Kind::Assign {
            feature: "child".into(),
            many: false,
            value: Box::new(expr(Kind::Call("Target".into()))),
        }),
        literal("x"),
    ])));
    grammar.rules.get_mut("Target").unwrap().datatype = Some(crate::grammar::Datatype::EString);
    grammar.declaration_sources.insert(
        "Root".into(),
        SourceSpan {
            source: source.id.clone(),
            start: declaration.start,
            end: declaration.end,
        },
    );
    let mut plan = ValidationPlan::new(&grammar);
    let mut exact = 5;
    build(&grammar, &tokens(&[], true), &mut exact, &mut plan).unwrap();
    assert_eq!(exact, 0);
    assert_eq!(plan.frontiers[0].path, "rule/Root/s0/value");
    let observation = PredictionObservation::new();
    let context = AnalysisObservation {
        observation: &observation,
        recursive_work: None,
        adjacent_work: None,
        entry: &source.id,
        stage: RunStage::Executable,
    };
    let result = plan.replay(
        &[("Root".into(), tokens(&[], false))].into(),
        &mut BTreeSet::new(),
        Some(&mut Vec::new()),
        &mut 2,
        Some(context),
    );
    assert_eq!(
        result,
        Err("Required nullable call is not a nonfragment object rule".into())
    );
    let failure = observation.first_failure().unwrap();
    assert_eq!(failure.phase, Phase::Follow);
    assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "Root");
    assert_eq!(
        failure.decision.unwrap().as_str().unwrap(),
        "rule/Root/s0/value"
    );
    assert_eq!(failure.source.unwrap().uri.as_str().unwrap(), source.id.uri);
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        source.id.sha256
    );
    assert_eq!(
        failure.declaration_span,
        Some((declaration.start, declaration.end))
    );
    assert_eq!(
        (
            failure.charged_work,
            failure.syntax_depth,
            failure.call_depth,
            failure.state_count,
            failure.repeated_state_count
        ),
        (None, None, None, None, None)
    );
}

#[test]
fn new_profile_inherits_certificates_guards_summaries_and_native_resident_profile_size() {
    for profile in [Profile::NativeV71, Profile::NativeV72] {
        assert!(profile.has_required_object_call_certificates());
        assert!(profile.has_fragment_call_summaries());
        assert!(profile.has_computed_boolean_settings());
        assert!(profile.has_full_syntactic_predicates());
        assert!(profile.has_executable_recursive_analysis());
        assert!(profile.has_parser_prediction());
        assert!(profile.has_recursive_predicates());
        assert!(profile.has_recursive_fragments());
        assert!(profile.has_durable_documents());
        assert_eq!(
            profile.has_symbolic_follow_validation(),
            profile == Profile::NativeV72
        );
    }
    assert_eq!(
        (
            std::mem::size_of::<Profile>(),
            std::mem::align_of::<Profile>()
        ),
        (1, 1)
    );
    assert_eq!(
        serde_json::to_value(Profile::NativeV71).unwrap(),
        json!("intersect.xtext.native.v71")
    );
    assert_eq!(
        serde_json::to_value(Profile::NativeV72).unwrap(),
        json!("intersect.xtext.native.v72")
    );
}
