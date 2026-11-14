use super::*;
use crate::grammar::AnalysisObservation;
use crate::grammar::Profile;
use crate::grammar::admission_observation::*;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use serde_json::json;

const CONFLICT: &str = "Fragment caller follow conflict requires unsupported Xtext prediction";

fn expression(kind: Kind) -> Expr {
    Expr {
        kind,
        cardinality: '1',
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
    }
}

fn rule(body: Expr, fragment: bool) -> super::super::Rule {
    serde_json::from_value(json!({"body":body,"fragment":fragment,"class":"Node"})).unwrap()
}

// Direct-IR controls with literal declaration identities. They test the one
// FOLLOW walker, not source parsing or admission of these modeling operations.
fn grammar(fragment_b: bool, nested: bool, owner: &str) -> (Grammar, DefinitionSource) {
    let mut grammar: Grammar = serde_json::from_value(json!({
        "profile":Profile::NativeV70,"name":"FollowObservation","entry":"A","rules":{}
    }))
    .unwrap();
    grammar.rules.insert(
        "A".into(),
        rule(
            expression(Kind::Sequence(vec![
                expression(Kind::Call(owner.into())),
                expression(Kind::Literal("x".into())),
            ])),
            false,
        ),
    );
    let mut optional = expression(Kind::Literal("x".into()));
    optional.cardinality = '?';
    grammar.rules.insert("B".into(), rule(optional, fragment_b));
    let body = if nested {
        let mut tag = expression(Kind::Assign {
            feature: "tag".into(),
            many: false,
            value: Box::new(expression(Kind::Literal("z".into()))),
        });
        tag.cardinality = '?';
        expression(Kind::Sequence(vec![
            tag,
            expression(Kind::Assign {
                feature: "child".into(),
                many: false,
                value: Box::new(expression(Kind::Alternative(vec![
                    expression(Kind::Call("B".into())),
                    expression(Kind::Literal("y".into())),
                ]))),
            }),
        ]))
    } else {
        expression(Kind::Call("B".into()))
    };
    grammar.rules.insert(owner.into(), rule(body, true));
    let declarations = [
        ("A", format!("A returns Node: {owner} 'x';\n")),
        (
            "B",
            format!(
                "{}B returns Node: 'x'?;\n",
                if fragment_b { "fragment " } else { "" }
            ),
        ),
        (
            owner,
            format!(
                "fragment {owner} returns Node: {};\n",
                if nested {
                    "tag='z'? child=(B | 'y')"
                } else {
                    "B"
                }
            ),
        ),
    ];
    let mut retained_owner = None;
    for (name, text) in declarations {
        let source = DefinitionSource::new(
            format!("memory:follow/{name}.xtext"),
            SourceKind::Xtext,
            text,
        );
        grammar.declaration_sources.insert(
            name.into(),
            SourceSpan {
                source: source.id.clone(),
                start: 0,
                end: source.text.len(),
            },
        );
        if name == owner {
            retained_owner = Some(source);
        }
    }
    (grammar, retained_owner.unwrap())
}

fn context<'a>(
    observation: &'a PredictionObservation,
    entry: &'a DefinitionSource,
) -> AnalysisObservation<'a> {
    AnalysisObservation {
        recursive_work: None,
        adjacent_work: None,
        observation,
        entry: &entry.id,
        stage: RunStage::Executable,
    }
}

fn assert_no_inferred_counters(failure: PhaseFailure) {
    assert_eq!(failure.syntax_depth, None);
    assert_eq!(failure.call_depth, None);
    assert_eq!(failure.state_count, None);
    assert_eq!(failure.repeated_state_count, None);
}

#[test]
fn follow_observation_preserves_exact_conflict_and_captures_real_declaration_path_work() {
    let (grammar, owner) = grammar(false, false, "F");
    let before = serde_json::to_value(&grammar).unwrap();
    let entry = DefinitionSource::new("memory:follow/entry.xtext", SourceKind::Xtext, "entry");
    let observation = PredictionObservation::new();
    let mut plain = BTreeSet::new();
    let mut observed = BTreeSet::new();
    assert_eq!(check(&grammar, &mut plain), Err(CONFLICT.into()));
    assert_eq!(
        check_observed(&grammar, &mut observed, Some(context(&observation, &entry))),
        Err(CONFLICT.into())
    );
    assert_eq!(plain, observed);
    assert_eq!(plain, BTreeSet::from(["rule/B/repeat".into()]));
    let failure = observation.first_failure().unwrap();
    assert_eq!(failure.stage, RunStage::Executable);
    assert_eq!(failure.phase, Phase::Follow);
    assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "F");
    assert_eq!(failure.source.unwrap().uri.as_str().unwrap(), owner.id.uri);
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        owner.id.sha256
    );
    assert_eq!(failure.declaration_span, Some((0, owner.text.len())));
    assert_eq!(failure.decision.unwrap().as_str().unwrap(), "rule/F");
    // Literal shared-bucket accounting: propagation 7 walk/transfer + 4
    // propagation queue/edge; validation A 3 + B 1 + F 1 = 16.
    assert_eq!(failure.charged_work, Some(16));
    assert_no_inferred_counters(failure);
    assert!(!failure.capture_unavailable);
    assert_eq!(serde_json::to_value(&grammar).unwrap(), before);
}

#[test]
fn follow_observation_preserves_charge_failure_precedence_and_partial_deferred_output() {
    let (grammar, _) = grammar(false, false, "F");
    let entry = DefinitionSource::new("memory:follow/entry.xtext", SourceKind::Xtext, "entry");
    for initial in [0, 1, 14, 15, 16, 17] {
        let observation = PredictionObservation::new();
        let mut plain = BTreeSet::new();
        let mut observed = BTreeSet::new();
        let mut old_work = initial;
        let mut new_work = initial;
        let old = check_with_work(&grammar, &mut plain, &mut old_work, None);
        let new = check_with_work(
            &grammar,
            &mut observed,
            &mut new_work,
            Some(context(&observation, &entry)),
        );
        assert_eq!(new, old);
        assert_eq!(plain, observed);
        assert_eq!(
            (old_work, new_work),
            (initial.saturating_sub(16), initial.saturating_sub(16))
        );
        if initial < 16 {
            assert_eq!(old, Err("Grammar prediction budget exceeded".into()));
            if initial < 14 {
                // Untouched propagation charge failure remains unlocalized.
                assert_eq!(observation.first_failure(), None);
            } else {
                let failure = observation.first_failure().unwrap();
                assert_eq!(failure.phase, Phase::FollowWalk);
                let owner = if initial == 14 { "B" } else { "F" };
                assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), owner);
                assert_eq!(
                    failure.decision.unwrap().as_str().unwrap(),
                    format!("rule/{owner}")
                );
                assert_eq!(
                    failure.charged_work, None,
                    "lower helper does not invent outer accounting"
                );
                assert_no_inferred_counters(failure);
            }
        } else {
            assert_eq!(old, Err(CONFLICT.into()));
            let failure = observation.first_failure().unwrap();
            assert_eq!(failure.decision.unwrap().as_str().unwrap(), "rule/F");
            // This lower-level helper does not fabricate outer-bucket accounting.
            assert_eq!(failure.charged_work, None);
            assert_no_inferred_counters(failure);
        }
        assert_eq!(
            plain,
            if initial >= 15 {
                BTreeSet::from(["rule/B/repeat".into()])
            } else {
                BTreeSet::new()
            }
        );
    }
}

#[test]
fn follow_observation_preserves_nested_path_and_reverse_sequence_order() {
    let (grammar, owner) = grammar(false, true, "F");
    let entry = DefinitionSource::new("memory:follow/entry.xtext", SourceKind::Xtext, "entry");
    let observation = PredictionObservation::new();
    let before = serde_json::to_value(&grammar).unwrap();
    let mut plain = BTreeSet::new();
    let mut observed = BTreeSet::new();
    assert_eq!(check(&grammar, &mut plain), Err(CONFLICT.into()));
    assert_eq!(
        check_observed(&grammar, &mut observed, Some(context(&observation, &entry))),
        Err(CONFLICT.into())
    );
    assert_eq!(plain, observed);
    // Reverse sequence reaches child/B before the preceding optional tag.
    assert_eq!(plain, BTreeSet::from(["rule/B/repeat".into()]));
    let failure = observation.first_failure().unwrap();
    assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "F");
    assert_eq!(failure.source.unwrap().uri.as_str().unwrap(), owner.id.uri);
    assert_eq!(
        failure.decision.unwrap().as_str().unwrap(),
        "rule/F/s1/value/a0"
    );
    assert_no_inferred_counters(failure);
    assert_eq!(serde_json::to_value(&grammar).unwrap(), before);
}

#[test]
fn follow_observation_preserves_structural_nullable_success_and_existing_record() {
    let (grammar, _) = grammar(true, false, "F");
    let entry = DefinitionSource::new("memory:follow/entry.xtext", SourceKind::Xtext, "entry");
    let observation = PredictionObservation::new();
    let before = serde_json::to_value(&grammar).unwrap();
    let mut plain = BTreeSet::new();
    let mut observed = BTreeSet::new();
    assert_eq!(check(&grammar, &mut plain), Ok(()));
    assert_eq!(
        check_observed(&grammar, &mut observed, Some(context(&observation, &entry))),
        Ok(())
    );
    assert_eq!(plain, observed);
    assert_eq!(plain, BTreeSet::from(["rule/B/repeat".into()]));
    assert_eq!(observation.first_failure(), None);
    assert_eq!(serde_json::to_value(&grammar).unwrap(), before);
    let (conflicting, _) = self::grammar(false, false, "F");
    observation.returned_error(
        &entry.id,
        RunStage::Parent,
        Phase::RuleConstruction,
        None,
        None,
    );
    let previous = observation.first_failure();
    assert_eq!(
        check_observed(
            &conflicting,
            &mut BTreeSet::new(),
            Some(context(&observation, &entry))
        ),
        Err(CONFLICT.into())
    );
    assert_eq!(observation.first_failure(), previous);
    observation.reset();
    observation.returned_error(&entry.id, RunStage::Parent, Phase::Follow, None, None);
    observation.follow_measurements(77);
    let previous = observation.first_failure();
    assert_eq!(
        check_observed(
            &conflicting,
            &mut BTreeSet::new(),
            Some(context(&observation, &entry))
        ),
        Err(CONFLICT.into())
    );
    assert_eq!(observation.first_failure(), previous);
}

#[test]
fn follow_observation_preserves_missing_origin_and_bounded_whole_identity_capture() {
    let (mut grammar, _) = grammar(false, false, "F");
    grammar.declaration_sources.remove("F");
    let entry = DefinitionSource::new("memory:follow/entry.xtext", SourceKind::Xtext, "entry");
    let observation = PredictionObservation::new();
    assert_eq!(
        check_observed(
            &grammar,
            &mut BTreeSet::new(),
            Some(context(&observation, &entry))
        ),
        Err(CONFLICT.into())
    );
    let missing = observation.first_failure().unwrap();
    assert_eq!(missing.owner_rule, None);
    assert_eq!(missing.declaration_span, None);
    assert_eq!(missing.source.unwrap().uri.as_str().unwrap(), entry.id.uri);
    assert_eq!(missing.decision.unwrap().as_str().unwrap(), "rule/F");
    assert_eq!(missing.charged_work, Some(16));
    assert_no_inferred_counters(missing);
    observation.reset();
    let long_name = "F".repeat(260);
    let (long, owner) = self::grammar(false, false, &long_name);
    assert_eq!(
        check_observed(
            &long,
            &mut BTreeSet::new(),
            Some(context(&observation, &entry))
        ),
        Err(CONFLICT.into())
    );
    let bounded = observation.first_failure().unwrap();
    assert_eq!(bounded.owner_rule, None);
    assert_eq!(bounded.decision, None);
    assert_eq!(bounded.declaration_span, Some((0, owner.text.len())));
    assert_eq!(bounded.source.unwrap().uri.as_str().unwrap(), owner.id.uri);
    assert!(bounded.capture_unavailable);
    assert_no_inferred_counters(bounded);
}
