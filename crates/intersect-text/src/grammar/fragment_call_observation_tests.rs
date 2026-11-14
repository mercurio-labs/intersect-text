use super::*;
use admission_observation::{Phase, PredictionObservation, RunStage};
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;

fn expression(kind: Kind) -> Expr {
    Expr {
        kind,
        cardinality: '1',
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
    }
}

#[test]
fn fragment_call_observation_preserves_exact_outputs_work_and_failure_order() {
    let body = expression(Kind::Sequence(vec![
        expression(Kind::Call("B".into())),
        expression(Kind::Assign {
            feature: "name".into(),
            many: false,
            value: Box::new(expression(Kind::Call("A".into()))),
        }),
    ]));
    for (initial, names, remaining, depth) in [
        (0, vec![], 0, Some(0)),
        (1, vec![], 0, Some(1)),
        (2, vec!["B"], 0, Some(1)),
        (3, vec!["B"], 0, Some(2)),
        (4, vec!["A", "B"], 0, None),
        (5, vec!["A", "B"], 1, None),
    ] {
        let mut plain = BTreeSet::new();
        let mut observed = BTreeSet::new();
        let mut plain_work = initial;
        let mut observed_work = initial;
        let old = follow::calls(&body, &mut plain, &mut plain_work);
        let mut failure = None;
        let new = follow::calls_with_failure(
            &body,
            &mut observed,
            &mut observed_work,
            Some(&mut failure),
        );
        assert_eq!(old, new);
        assert_eq!(plain, observed);
        assert_eq!(plain, names.into_iter().map(str::to_owned).collect());
        assert_eq!(plain_work, remaining);
        assert_eq!(observed_work, remaining);
        assert_eq!(
            failure,
            depth.map(|syntax_depth| follow::CallWalkFailure {
                reason: follow::CallWalkLimit::Work,
                remaining_work: remaining,
                syntax_depth,
            })
        );
        if depth.is_some() {
            assert_eq!(old, Err("Grammar prediction budget exceeded".into()));
        }
    }
}

#[test]
fn fragment_call_observation_keeps_charge_before_depth_and_work_precedence() {
    for (depth, initial, remaining, reason) in [
        (64, 65, 0, None),
        (65, 65, 0, Some(follow::CallWalkLimit::Work)),
        (65, 66, 0, Some(follow::CallWalkLimit::SyntaxDepth)),
        (65, 67, 1, Some(follow::CallWalkLimit::SyntaxDepth)),
    ] {
        let mut body = expression(Kind::Call("Leaf".into()));
        for _ in 0..depth {
            body = expression(Kind::Sequence(vec![body]));
        }
        let mut plain = BTreeSet::new();
        let mut observed = BTreeSet::new();
        let mut plain_work = initial;
        let mut observed_work = initial;
        let mut failure = None;
        let old = follow::calls(&body, &mut plain, &mut plain_work);
        let new = follow::calls_with_failure(
            &body,
            &mut observed,
            &mut observed_work,
            Some(&mut failure),
        );
        assert_eq!(old, new);
        assert_eq!(plain, observed);
        assert_eq!((plain_work, observed_work), (remaining, remaining));
        assert_eq!(
            failure,
            reason.map(|reason| follow::CallWalkFailure {
                reason,
                remaining_work: remaining,
                syntax_depth: 65,
            })
        );
        if reason.is_some() {
            assert_eq!(old, Err("Grammar prediction budget exceeded".into()));
            assert!(plain.is_empty());
        } else {
            assert_eq!(old, Ok(()));
            assert_eq!(plain, BTreeSet::from(["Leaf".into()]));
        }
    }
}

fn rule(body: Expr, fragment: bool) -> Rule {
    Rule {
        hidden: None,
        enum_literals: BTreeMap::new(),
        nominal_return: None,
        inferred_return: false,
        override_declared: false,
        datatype: None,
        fragment,
        wildcard: false,
        class: "Node".into(),
        body,
    }
}

fn grammar(nested: bool, width: usize) -> (Grammar, DefinitionSource, DefinitionSource) {
    // These direct-IR controls retain exact authored declaration identities.
    // The full original source admission below remains a separate host proof.
    let root_text = if nested {
        "A returns Node: F;\n".to_owned()
    } else {
        format!("A returns Node: {};\n", vec!["'x'"; width].join(" "))
    };
    let root = DefinitionSource::new("memory:root-calls.xtext", SourceKind::Xtext, root_text);
    let fragment = DefinitionSource::new(
        "memory:fragment-calls.xtext",
        SourceKind::Xtext,
        format!(
            "fragment F returns Node: {};\n",
            vec!["'x'"; width].join(" ")
        ),
    );
    let wide = || {
        expression(Kind::Sequence(
            (0..width)
                .map(|_| expression(Kind::Literal("x".into())))
                .collect(),
        ))
    };
    let body = if nested {
        expression(Kind::Call("F".into()))
    } else {
        wide()
    };
    let rules = BTreeMap::from([
        ("A".into(), rule(body, false)),
        ("F".into(), rule(wide(), true)),
    ]);
    let declarations = BTreeMap::from([
        (
            "A".into(),
            SourceSpan {
                source: root.id.clone(),
                start: 0,
                end: root.text.len(),
            },
        ),
        (
            "F".into(),
            SourceSpan {
                source: fragment.id.clone(),
                start: 0,
                end: fragment.text.len(),
            },
        ),
    ]);
    (
        Grammar {
            call_depth: None,
            analysis_work_budget: Default::default(),
            parameterized_rules: Default::default(),
            final_rules: Default::default(),
            computed_settings: vec![],
            profile: Profile::NativeV69,
            name: "CallObservation".into(),
            entry: "A".into(),
            rules,
            terminals: BTreeMap::new(),
            hidden: None,
            lexical_prediction: None,
            parser_prediction: None,
            declaration_sources: declarations,
            terminal_order: vec![],
            shadowed_declarations: BTreeMap::new(),
        },
        root,
        fragment,
    )
}

fn schema() -> Schema {
    serde_json::from_value(serde_json::json!({
        "revision": "fragment-call-observation-v1",
        "uri": "urn:fragment-call-observation-v1",
        "name": "FragmentCallObservation",
        "classes": {}
    }))
    .unwrap()
}

#[test]
fn fragment_call_observation_owns_actual_root_or_fragment_source_and_span() {
    let entry = DefinitionSource::new("memory:actual-entry.xtext", SourceKind::Xtext, "entry");
    for nested in [false, true] {
        let (grammar, root, fragment) = grammar(nested, 10_000);
        let before = serde_json::to_value(&grammar).unwrap();
        let observation = PredictionObservation::new();
        let context = AnalysisObservation {
            observation: &observation,
            recursive_work: None,
            adjacent_work: None,
            entry: &entry.id,
            stage: RunStage::Executable,
        };
        let ordinary = check_fragment_callers(
            &grammar,
            &schema(),
            None,
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K,
        );
        let observed = check_fragment_callers(
            &grammar,
            &schema(),
            Some(context),
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K,
        );
        assert_eq!(ordinary, Err("Grammar prediction budget exceeded".into()));
        assert_eq!(observed, ordinary);
        let failure = observation.first_failure().unwrap();
        let (owner, source, decision) = if nested {
            ("F", fragment, "fragment-body-calls/work-limit")
        } else {
            ("A", root, "root-body-calls/work-limit")
        };
        assert_eq!(failure.phase, Phase::FragmentCallScan);
        assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), owner);
        assert_eq!(failure.source.unwrap().uri.as_str().unwrap(), source.id.uri);
        assert_eq!(
            failure.source.unwrap().sha256.as_str().unwrap(),
            source.id.sha256
        );
        assert_eq!(failure.declaration_span, Some((0, source.text.len())));
        assert_eq!(failure.decision.unwrap().as_str().unwrap(), decision);
        assert_eq!(failure.charged_work, Some(10_000));
        assert_eq!(failure.syntax_depth, Some(1));
        assert_eq!(failure.call_depth, None);
        assert_eq!(failure.state_count, None);
        assert_eq!(failure.repeated_state_count, None);
        assert!(!failure.capture_unavailable);
        let outer = observe_phase(
            observed,
            &grammar,
            Some(context),
            Phase::FragmentCallers,
            None,
            None,
        );
        assert_eq!(outer, ordinary);
        assert_eq!(observation.first_failure(), Some(failure));
        assert_eq!(serde_json::to_value(&grammar).unwrap(), before);
    }
}

#[test]
fn fragment_call_observation_preserves_missing_origin_and_success_whole_dto() {
    let entry = DefinitionSource::new("memory:actual-entry.xtext", SourceKind::Xtext, "entry");
    let (mut missing, _, _) = grammar(true, 10_000);
    missing.declaration_sources.remove("F");
    let observation = PredictionObservation::new();
    let context = AnalysisObservation {
        observation: &observation,
        recursive_work: None,
        adjacent_work: None,
        entry: &entry.id,
        stage: RunStage::Executable,
    };
    assert_eq!(
        check_fragment_callers(
            &missing,
            &schema(),
            Some(context),
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        check_fragment_callers(
            &missing,
            &schema(),
            None,
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        )
    );
    let failure = observation.first_failure().unwrap();
    assert_eq!(failure.owner_rule, None);
    assert_eq!(failure.declaration_span, None);
    assert_eq!(failure.source.unwrap().uri.as_str().unwrap(), entry.id.uri);
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        entry.id.sha256
    );
    assert_eq!(failure.charged_work, Some(10_000));
    observation.reset();
    let (success, _, _) = grammar(true, 2);
    let before = serde_json::to_value(&success).unwrap();
    assert_eq!(
        check_fragment_callers(
            &success,
            &schema(),
            None,
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        Ok(())
    );
    assert_eq!(
        check_fragment_callers(
            &success,
            &schema(),
            Some(context),
            crate::grammar::GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        Ok(())
    );
    assert_eq!(observation.first_failure(), None);
    assert_eq!(serde_json::to_value(&success).unwrap(), before);
}
