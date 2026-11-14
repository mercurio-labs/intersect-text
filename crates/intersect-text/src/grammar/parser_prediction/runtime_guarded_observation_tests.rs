// Direct kernel controls use real source shapes and the canonical charged
// FIRST/nullable/binding workers. This fixture is not minted execution authority.
use crate::grammar::admission_observation::Phase;
use crate::grammar::admission_observation::PredictionObservation;
use crate::grammar::admission_observation::RunStage;

fn fixture() -> Result<(Grammar, ParserPrediction), String> {
    let source = SourceId::for_text(
        "memory:observed-runtime.xtext",
        "Root:(=>Target)*;Target:'x';",
    );
    let grammar: Grammar = serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"Observed","entry":"Root",
        "rules":{
            "Root":{"class":"Root","body":{"cardinality":"*","prediction_path":"rule/Root","original_range":{"start":0,"end":16},"kind":{"SyntacticPredicate":{"cardinality":"1","original_range":{"start":6,"end":14},"kind":{"Call":"Target"}}}}},
            "Target":{"class":"Target","body":{"cardinality":"1","original_range":{"start":23,"end":26},"kind":{"Literal":"x"}}}
        },
        "declaration_sources":{
            "Root":{"source":source,"start":0,"end":16},
            "Target":{"source":source,"start":16,"end":27}
        }
    })).map_err(|e|e.to_string())?;
    let program: ParserPrediction = serde_json::from_value(serde_json::json!({
        "states":[
            {"rule_stop":false,"accepting":false,"edges":[{"Epsilon":1},{"Epsilon":2}],"follow":[]},
            {"rule_stop":false,"accepting":false,"edges":[{"Full":{"target":3,"stop":4}}],"follow":[]},
            {"rule_stop":true,"accepting":true,"edges":[],"follow":[]},
            {"rule_stop":false,"accepting":false,"edges":[{"Call":{"target":5,"follow":4}}],"follow":[]},
            {"rule_stop":false,"accepting":false,"edges":[{"Epsilon":0}],"follow":[]},
            {"rule_stop":false,"accepting":false,"edges":[{"Token":[{"Keyword":"x"},6]}],"follow":[]},
            {"rule_stop":true,"accepting":false,"edges":[],"follow":[]}
        ],
        "rules":{"Root":[0,2],"Target":[5,6]},
        "decisions":{"rule/Root/repeat":{"entries":[1,2],"exit":1},"rule/Root/recovery":{"entries":[0,2],"exit":null}},
        "first_guards":{}
    })).map_err(|e|e.to_string())?;
    Ok((grammar, program))
}
fn context<'a>(
    observation: &'a PredictionObservation,
    grammar: &'a Grammar,
) -> Result<super::super::super::super::AnalysisObservation<'a>, String> {
    Ok(super::super::super::super::AnalysisObservation {
        observation,
        recursive_work: None,
        adjacent_work: None,
        entry: &grammar
            .declaration_sources
            .values()
            .next()
            .ok_or("Missing original source")?
            .source,
        stage: RunStage::Executable,
    })
}
#[test]
fn inventory_repeat_nonrepeat_first_failure_and_reset_preserve_whole_result() -> Result<(), String>
{
    let (grammar, original) = fixture()?;
    let observation = PredictionObservation::new();
    for (key, allowance) in [
        ("rule/Root/choice", 0),
        ("rule/Root/repeat", 0),
        ("rule/Root/repeat", 1),
    ] {
        observation.reset();
        let mut program = original.clone();
        let decision = program
            .decisions
            .remove("rule/Root/repeat")
            .ok_or("Missing repeat")?;
        program.decisions.clear();
        program.decisions.insert(key.into(), decision);
        let mut plain =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let mut observed =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let mut a = allowance;
        let mut b = allowance;
        let old = plain.collect(&grammar, &program, &mut a, None);
        let new = observed.collect(
            &grammar,
            &program,
            &mut b,
            Some(context(&observation, &grammar)?),
        );
        assert_eq!(old, new);
        assert!(new.is_err());
        assert_eq!(a, b);
        assert!(observed.records.is_empty());
        let first = observation
            .first_failure()
            .ok_or("Missing inventory observation")?;
        assert_eq!(first.phase, Phase::RuntimeNullableInventory);
        assert_eq!(first.stage, RunStage::Executable);
        assert_eq!(
            first
                .decision
                .ok_or("Missing exact key")?
                .as_str()
                .map_err(|e| e.to_string())?,
            key
        );
        assert_eq!(first.charged_work, Some(10_000 - b));
        assert_eq!(first.state_count, Some(program.states.len()));
        assert!(first.syntax_depth.is_none() && first.call_depth.is_none());
        let later: Result<(), String> = observe_candidate(
            Err("later failure".into()),
            &grammar,
            &program,
            Some(context(&observation, &grammar)?),
            (Phase::RuntimeCandidateBinding, Some(key)),
            b,
        );
        assert!(later.is_err());
        assert_eq!(observation.first_failure(), Some(first));
    }
    observation.reset();
    assert!(observation.first_failure().is_none());
    Ok(())
}
#[test]
fn binding_and_canonical_first_zero_or_positive_balance_keep_original_error() -> Result<(), String>
{
    let (mut grammar, mut program) = fixture()?;
    grammar
        .rules
        .get_mut("Target")
        .ok_or("Missing callee")?
        .body
        .kind = Kind::Call("Target".into());
    program.states[5].edges = vec![Edge::Call {
        target: 5,
        follow: 6,
    }];
    let observation = PredictionObservation::new();
    let mut witnessed_binding = false;
    let mut witnessed_zero = false;
    let mut witnessed_positive = false;
    // Each allowance models an already consumed prefix of the same10k pool.
    for allowance in 0..256 {
        observation.reset();
        let mut a = allowance;
        let mut b = allowance;
        let mut la = 0;
        let mut lb = 0;
        let plain = binding(
            &grammar,
            &program,
            "rule/Root/repeat",
            &mut a,
            &mut la,
            RuntimeGuardedPolicy::StrictCompletionScopes,
            None,
        );
        let raw = binding(
            &grammar,
            &program,
            "rule/Root/repeat",
            &mut b,
            &mut lb,
            RuntimeGuardedPolicy::StrictCompletionScopes,
            Some(context(&observation, &grammar)?),
        );
        let observed = observe_candidate(
            raw,
            &grammar,
            &program,
            Some(context(&observation, &grammar)?),
            (Phase::RuntimeCandidateBinding, Some("rule/Root/repeat")),
            b,
        );
        assert_eq!(plain, observed);
        assert_eq!(a, b);
        assert_eq!(la, lb);
        let first = observation
            .first_failure()
            .ok_or("Missing real returned failure")?;
        assert!(
            observed
                .as_ref()
                .is_err_and(|e| e == "Grammar prediction budget exceeded")
        );
        assert_eq!(first.charged_work, Some(10_000 - b));
        match first.phase {
            Phase::RuntimeCandidateBinding => witnessed_binding = true,
            Phase::RuntimeCandidateNonnullableFirst if b == 0 => witnessed_zero = true,
            Phase::RuntimeCandidateNonnullableFirst if b > 0 => witnessed_positive = true,
            other => return Err(format!("Unexpected actual boundary {other:?}")),
        }
        assert!(first.syntax_depth.is_none() && first.call_depth.is_none());
    }
    assert!(witnessed_binding && witnessed_zero && witnessed_positive);
    Ok(())
}
#[test]
fn collect_success_no_side_effects_and_oversized_identity_is_unavailable() -> Result<(), String> {
    let (mut grammar, mut program) = fixture()?;
    let observation = PredictionObservation::new();
    let before = serde_json::to_value(&program).map_err(|e| e.to_string())?;
    let mut plain =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut observed =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut a = 10_000;
    let mut b = 10_000;
    assert_eq!(
        plain.collect(&grammar, &program, &mut a, None),
        observed.collect(
            &grammar,
            &program,
            &mut b,
            Some(context(&observation, &grammar)?)
        )
    );
    assert_eq!(plain.records, observed.records);
    assert_eq!(plain.logical_bytes, observed.logical_bytes);
    assert_eq!(a, b);
    assert_eq!(observed.records.len(), 1);
    assert!(observation.first_failure().is_none());
    assert_eq!(
        serde_json::to_value(&program).map_err(|e| e.to_string())?,
        before
    );
    let long = "R".repeat(300);
    let mut rule = grammar.rules.remove("Root").ok_or("Missing root")?;
    rule.body.prediction_path = Some(format!("rule/{long}"));
    grammar.rules.insert(long.clone(), rule);
    let source = grammar
        .declaration_sources
        .remove("Root")
        .ok_or("Missing source")?;
    grammar.declaration_sources.insert(long.clone(), source);
    let decision = program
        .decisions
        .remove("rule/Root/repeat")
        .ok_or("Missing repeat")?;
    program.decisions.clear();
    program
        .decisions
        .insert(format!("rule/{long}/repeat"), decision);
    observation.reset();
    let mut failed = RuntimeGuardedAdmission::new();
    assert!(
        failed
            .collect(
                &grammar,
                &program,
                &mut 0,
                Some(context(&observation, &grammar)?)
            )
            .is_err()
    );
    let first = observation
        .first_failure()
        .ok_or("Missing unavailable record")?;
    assert!(first.capture_unavailable);
    assert!(first.owner_rule.is_none() && first.decision.is_none());
    assert!(first.source.is_some());
    assert_eq!(first.charged_work, Some(10_000));
    Ok(())
}

#[test]
fn completed_first_reuse_cold_and_enabled_keep_whole_compiler_result() -> Result<(), String> {
    use std::collections::BTreeSet;
    for nullable in [false, true] {
        let (mut grammar, _) = fixture()?;
        if nullable {
            grammar
                .rules
                .get_mut("Target")
                .ok_or("Missing target")?
                .body
                .cardinality = '?';
        }
        let before = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
        let mut cold =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        cold.disable_completed_reuse = true;
        let mut enabled =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let a = ParserPrediction::compile_runtime_guarded_linear_repeats(
            &grammar,
            &BTreeSet::new(),
            None,
            None,
            &mut cold,
        );
        let b = ParserPrediction::compile_runtime_guarded_linear_repeats(
            &grammar,
            &BTreeSet::new(),
            None,
            None,
            &mut enabled,
        );
        let whole_a = serde_json::to_value(&a).map_err(|e| e.to_string())?;
        let whole_b = serde_json::to_value(&b).map_err(|e| e.to_string())?;
        println!(
            "COMPLETED_FIRST_WHOLE_COMPILER_RESULT {}",
            serde_json::json!({"nullable":nullable,"cold":whole_a,"enabled":whole_b})
        );
        assert_eq!(whole_a, whole_b);
        assert_eq!(a.is_ok(), !nullable);
        assert_eq!(cold.records, enabled.records);
        assert_eq!(cold.failed, enabled.failed);
        assert_eq!(
            serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
            before
        );
    }
    Ok(())
}

mod reuse_work {
    include!("runtime_guarded_reuse_work_tests.rs");
}
