use super::*;
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
fn rule(class: &str, body: Expr, fragment: bool) -> Rule {
    Rule {
        hidden: None,
        enum_literals: BTreeMap::new(),
        nominal_return: None,
        inferred_return: false,
        override_declared: false,
        datatype: None,
        fragment,
        wildcard: false,
        class: class.into(),
        body,
    }
}
fn grammar(profile: Profile, rules: BTreeMap<String, Rule>) -> Grammar {
    Grammar {
        call_depth: None,
        analysis_work_budget: Default::default(),
        parameterized_rules: Default::default(),
        final_rules: Default::default(),
        computed_settings: vec![],
        profile,
        name: "SharedCalls".into(),
        entry: "A".into(),
        rules,
        terminals: BTreeMap::new(),
        hidden: None,
        lexical_prediction: None,
        parser_prediction: None,
        declaration_sources: BTreeMap::new(),
        terminal_order: vec![],
        shadowed_declarations: BTreeMap::new(),
    }
}
fn schema(valid_b: bool) -> Schema {
    let mut schema: Schema = serde_json::from_value(json!({
        "revision":"fragment-call-summaries-v1", "uri":"urn:fragment-call-summaries-v1", "name":"FragmentCallSummaries",
        "classes": {
            "Base": {"name":"Base", "abstract_type":false, "parents":[], "features":{}},
            "A": {"name":"A", "abstract_type":false, "parents":["Base"], "features":{"name":{
                "name":"name", "target":"EString", "reference":false, "containment":false, "opposite":null,
                "lower":0, "upper":1, "unique":true, "ordered":true, "derived":false, "default":null
            }}},
            "B": {"name":"B", "abstract_type":false, "parents":["Base"], "features":{}}
        }
    })).unwrap();
    if valid_b {
        let field = schema.classes["A"].features["name"].clone();
        schema
            .classes
            .get_mut("B")
            .unwrap()
            .features
            .insert("name".into(), field);
    }
    schema
}
fn shared(profile: Profile) -> Grammar {
    grammar(
        profile,
        BTreeMap::from([
            ("A".into(), rule("A", expr(Kind::Call("F".into())), false)),
            ("B".into(), rule("B", expr(Kind::Call("F".into())), false)),
            (
                "F".into(),
                rule(
                    "Base",
                    expr(Kind::Assign {
                        feature: "name".into(),
                        many: false,
                        value: Box::new(expr(Kind::Call("ID".into()))),
                    }),
                    true,
                ),
            ),
        ]),
    )
}

#[test]
fn summaries_validate_each_class_pair_preserve_dedup_and_keep_actions_facade_legacy() {
    let grammar = shared(Profile::NativeV70);
    let schema = schema(true);
    let mut context = FragmentCallContext {
        initial_work: 10_000,
        observation: None,
        mode: FragmentCallMode::Summaries(follow::RuleCallSummaries::new(&grammar)),
    };
    let mut checked = BTreeSet::new();
    let mut work = 10_000;
    for (name, expected_work, expected_pairs) in [
        ("A", 9991, BTreeSet::from([("A".into(), "F".into())])),
        ("A", 9989, BTreeSet::from([("A".into(), "F".into())])),
        (
            "B",
            9983,
            BTreeSet::from([("A".into(), "F".into()), ("B".into(), "F".into())]),
        ),
    ] {
        let root = &grammar.rules[name];
        assert_eq!(
            check_fragment_context_observed(
                &root.body,
                (Some(name), &root.class),
                &grammar,
                &schema,
                &mut checked,
                &mut work,
                &mut context
            ),
            Ok(())
        );
        assert_eq!(work, expected_work);
        assert_eq!(checked, expected_pairs);
    }
    let mut legacy_pairs = BTreeSet::new();
    let mut legacy_work = 3;
    assert_eq!(
        check_fragment_context(
            &grammar.rules["A"].body,
            "A",
            &grammar,
            &schema,
            &mut legacy_pairs,
            &mut legacy_work
        ),
        Ok(())
    );
    assert_eq!(legacy_work, 0); // Existing facade pays only3 AST visits even in V70.
    assert_eq!(legacy_pairs, BTreeSet::from([("A".into(), "F".into())]));
}

#[test]
fn summaries_cannot_hide_second_class_failure_and_preserve_first_semantic_error() {
    let schema = schema(false);
    let old = shared(Profile::NativeV69);
    let new = shared(Profile::NativeV70);
    assert_eq!(
        check_fragment_callers(
            &old,
            &schema,
            None,
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        Err("Unknown feature B.name".into())
    );
    assert_eq!(
        check_fragment_callers(
            &new,
            &schema,
            None,
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        Err("Unknown feature B.name".into())
    );
    let mut context = FragmentCallContext {
        initial_work: 10_000,
        observation: None,
        mode: FragmentCallMode::Summaries(follow::RuleCallSummaries::new(&new)),
    };
    let mut checked = BTreeSet::new();
    let mut work = 10_000;
    assert_eq!(
        check_fragment_context_observed(
            &new.rules["A"].body,
            (Some("A"), "A"),
            &new,
            &schema,
            &mut checked,
            &mut work,
            &mut context
        ),
        Ok(())
    );
    assert_eq!(work, 9991);
    assert_eq!(
        check_fragment_context_observed(
            &new.rules["B"].body,
            (Some("B"), "B"),
            &new,
            &schema,
            &mut checked,
            &mut work,
            &mut context
        ),
        Err("Unknown feature B.name".into())
    );
    assert_eq!(work, 9987); // B root miss4; reject before cached F lookup/emission.
    assert_eq!(
        checked,
        BTreeSet::from([("A".into(), "F".into()), ("B".into(), "F".into())])
    );
}

#[test]
fn summaries_preserve_productive_cycles_sorted_pending_and_wildcard_skips() {
    let schema = schema(true);
    for profile in [Profile::NativeV69, Profile::NativeV70] {
        let cyclic = grammar(
            profile,
            BTreeMap::from([
                (
                    "A".into(),
                    rule(
                        "A",
                        expr(Kind::Sequence(vec![
                            expr(Kind::Call("G".into())),
                            expr(Kind::Call("F".into())),
                            expr(Kind::Call("F".into())),
                        ])),
                        false,
                    ),
                ),
                ("F".into(), rule("Base", expr(Kind::Call("G".into())), true)),
                ("G".into(), rule("Base", expr(Kind::Call("F".into())), true)),
            ]),
        );
        assert_eq!(
            check_fragment_callers(
                &cyclic,
                &schema,
                None,
                crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
            ),
            Ok(())
        );
        let mut checked = BTreeSet::new();
        let mut context = FragmentCallContext {
            initial_work: 10_000,
            observation: None,
            mode: if profile.has_fragment_call_summaries() {
                FragmentCallMode::Summaries(follow::RuleCallSummaries::new(&cyclic))
            } else {
                FragmentCallMode::Legacy
            },
        };
        assert_eq!(
            check_fragment_context_observed(
                &cyclic.rules["A"].body,
                (Some("A"), "A"),
                &cyclic,
                &schema,
                &mut checked,
                &mut 10_000,
                &mut context
            ),
            Ok(())
        );
        assert_eq!(
            checked,
            BTreeSet::from([("A".into(), "F".into()), ("A".into(), "G".into())])
        );
        // Existing wildcard boundary skips nested typed fragments. Datatype and
        // current-flow roots also remain outside this caller-analysis phase.
        let mut skips = shared(profile);
        skips.rules.remove("B");
        skips.rules.get_mut("A").unwrap().body = expr(Kind::Call("W".into()));
        let mut wildcard = rule("Base", expr(Kind::Call("F".into())), true);
        wildcard.wildcard = true;
        skips.rules.insert("W".into(), wildcard);
        let mut datatype = rule("Base", expr(Kind::Call("Absent".into())), false);
        datatype.datatype = Some(Datatype::EString);
        skips.rules.insert("D".into(), datatype);
        skips.rules.insert(
            "C".into(),
            rule(
                "A",
                expr(Kind::Capture {
                    class: "A".into(),
                    feature: "name".into(),
                    many: false,
                }),
                false,
            ),
        );
        assert_eq!(
            check_fragment_callers(
                &skips,
                &schema,
                None,
                crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
            ),
            Ok(())
        );
        let mut checked = BTreeSet::new();
        let mut context = FragmentCallContext {
            initial_work: 10_000,
            observation: None,
            mode: if profile.has_fragment_call_summaries() {
                FragmentCallMode::Summaries(follow::RuleCallSummaries::new(&skips))
            } else {
                FragmentCallMode::Legacy
            },
        };
        assert_eq!(
            check_fragment_context_observed(
                &skips.rules["A"].body,
                (Some("A"), "A"),
                &skips,
                &schema,
                &mut checked,
                &mut 10_000,
                &mut context
            ),
            Ok(())
        );
        assert!(checked.is_empty());
    }
}

#[test]
fn summaries_old_profiles_opt_out_at_exact_legacy_fees_and_serialized_names() {
    let schema = schema(true);
    for index in 1..=69 {
        let name = format!("intersect.xtext.native.v{index}");
        let profile: Profile = serde_json::from_value(json!(name)).unwrap();
        assert!(!profile.has_fragment_call_summaries());
        assert_eq!(profile.as_str(), name);
        assert_eq!(serde_json::to_value(profile).unwrap(), json!(name));
    }
    assert_eq!(Profile::default(), Profile::WorkbenchV1);
    for profile in [
        Profile::NativeV65,
        Profile::NativeV66,
        Profile::NativeV67,
        Profile::NativeV68,
        Profile::NativeV69,
    ] {
        let grammar = shared(profile);
        let before = serde_json::to_value(&grammar).unwrap();
        let mut context = FragmentCallContext {
            initial_work: 10_000,
            observation: None,
            mode: FragmentCallMode::Legacy,
        };
        let mut work = 8;
        let mut checked = BTreeSet::new();
        for (name, rule) in &grammar.rules {
            assert_eq!(
                check_fragment_context_observed(
                    &rule.body,
                    (Some(name), &rule.class),
                    &grammar,
                    &schema,
                    &mut checked,
                    &mut work,
                    &mut context
                ),
                Ok(())
            );
        }
        assert_eq!(work, 0);
        assert_eq!(
            checked,
            BTreeSet::from([("A".into(), "F".into()), ("B".into(), "F".into())])
        );
        assert_eq!(serde_json::to_value(&grammar).unwrap(), before);
    }
    assert!(Profile::NativeV70.has_fragment_call_summaries());
    assert_eq!(Profile::NativeV70.as_str(), "intersect.xtext.native.v70");
}

#[test]
fn summaries_clear_repeated_ast_work_with_the_same_ten_thousand_unit_bucket() {
    let mut rules = BTreeMap::new();
    let mut schema = schema(true);
    let base = schema.classes["Base"].clone();
    for index in 0..60 {
        let name = format!("A{index:02}");
        let mut class = base.clone();
        class.name = name.clone();
        class.parents = vec!["Base".into()];
        schema.classes.insert(name.clone(), class);
        rules.insert(
            name.clone(),
            rule(&name, expr(Kind::Call("F".into())), false),
        );
    }
    rules.insert(
        "F".into(),
        rule(
            "Base",
            expr(Kind::Sequence(
                (0..200).map(|_| expr(Kind::Literal("x".into()))).collect(),
            )),
            true,
        ),
    );
    let mut grammar = grammar(Profile::NativeV69, rules);
    grammar.entry = "A00".into();
    assert_eq!(
        check_fragment_callers(
            &grammar,
            &schema,
            None,
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        Err("Grammar prediction budget exceeded".into())
    );
    grammar.profile = Profile::NativeV70;
    assert_eq!(
        check_fragment_callers(
            &grammar,
            &schema,
            None,
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        Ok(())
    );
    let mut context = FragmentCallContext {
        initial_work: 10_000,
        observation: None,
        mode: FragmentCallMode::Summaries(follow::RuleCallSummaries::new(&grammar)),
    };
    let mut work = 503;
    let mut checked = BTreeSet::new();
    for (name, rule) in &grammar.rules {
        assert_eq!(
            check_fragment_context_observed(
                &rule.body,
                (Some(name), &rule.class),
                &grammar,
                &schema,
                &mut checked,
                &mut work,
                &mut context
            ),
            Ok(())
        );
    }
    assert_eq!(work, 0); // 60 root misses240 + F miss203 +59 F hits59 +F-root hit1.
    assert_eq!(checked.len(), 60);
}
