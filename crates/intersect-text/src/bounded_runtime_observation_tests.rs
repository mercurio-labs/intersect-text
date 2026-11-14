//! Actual imported input -> same canonical compiler. No observed path grants new proof.
use super::*;
use grammar::admission_observation::{Phase, PredictionObservation};
fn outcome(
    result: &Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>>,
) -> Result<Value, String> {
    match result {
        Ok(value) => Ok(json!({"Ok":value.parts().0})),
        Err(errors) => Ok(json!({"Err":errors})),
    }
}
#[test]
fn observed_imported_linear_and_strict_success_keep_whole_ir_and_models() -> Result<(), String> {
    for policy in [
        grammar::RuntimeGuardedPolicy::LinearFullRepeats,
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
    ] {
        let source = input("ObservedIterationCallerTail", CALLER);
        let observation = PredictionObservation::new();
        let plain = ordinary::compile_with_runtime_guarded_policy(
            source.clone(),
            Profile::NativeV80,
            &[],
            None,
            Default::default(),
            Default::default(),
            policy,
        );
        let observed = ordinary::compile_with_runtime_guarded_policy_observed(
            source,
            Profile::NativeV80,
            &[],
            None,
            Default::default(),
            Default::default(),
            policy,
            &observation,
        );
        println!(
            "RUNTIME_CANDIDATE_OBSERVER_WHOLE_SUCCESS {}",
            json!({"policy":format!("{policy:?}"),"plain":outcome(&plain)?,"observed":outcome(&observed)?,"first":format!("{:?}",observation.first_failure())})
        );
        assert_eq!(outcome(&plain)?, outcome(&observed)?);
        let plain = plain.map_err(|e| format!("{e:?}"))?;
        let observed = observed.map_err(|e| format!("{e:?}"))?;
        assert!(observation.first_failure().is_none());
        assert_eq!(
            plain.parts().1.decision_count(),
            observed.parts().1.decision_count()
        );
        for text in ["x", "xx", "xxx"] {
            let (a, wa) = run(&plain, text, 100_000, 0);
            let (b, wb) = run(&observed, text, 100_000, 0);
            let a = a?;
            let b = b?;
            assert_eq!(wa, wb);
            assert_eq!(
                serde_json::to_value(&a.evaluation).map_err(|e| e.to_string())?,
                serde_json::to_value(&b.evaluation).map_err(|e| e.to_string())?
            );
            assert_eq!(
                a.consumed_runtime_decisions(),
                b.consumed_runtime_decisions()
            );
        }
    }
    Ok(())
}
#[test]
fn observed_imported_nullable_inventory_keeps_whole_error_and_resets_early_rejection()
-> Result<(), String> {
    let optional = X.replace(
        "terminal WS:",
        "Unused returns gate::Target: {gate::Target} tag='q'?;\nterminal WS:",
    );
    let observation = PredictionObservation::new();
    for policy in [
        grammar::RuntimeGuardedPolicy::LinearFullRepeats,
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
    ] {
        let source = input("ObservedQualifiedOptional", &optional);
        let plain = ordinary::compile_with_runtime_guarded_policy(
            source.clone(),
            Profile::NativeV80,
            &[],
            None,
            Default::default(),
            Default::default(),
            policy,
        );
        let observed = ordinary::compile_with_runtime_guarded_policy_observed(
            source,
            Profile::NativeV80,
            &[],
            None,
            Default::default(),
            Default::default(),
            policy,
            &observation,
        );
        println!(
            "RUNTIME_CANDIDATE_OBSERVER_WHOLE_FAILURE {}",
            json!({"plain":outcome(&plain)?,"observed":outcome(&observed)?,"failure":format!("{:?}",observation.first_failure())})
        );
        assert_eq!(outcome(&plain)?, outcome(&observed)?);
        assert!(observed.is_err());
        let first = observation
            .first_failure()
            .ok_or("Missing qualified nullable boundary")?;
        assert_eq!(first.phase, Phase::RuntimeNullableInventory);
        assert_eq!(
            first
                .decision
                .ok_or("Missing exact source key")?
                .as_str()
                .map_err(|e| e.to_string())?,
            "rule/Unused/s1/repeat"
        );
        assert!(first.charged_work.is_some() && first.state_count.is_some());
        assert!(first.syntax_depth.is_none() && first.call_depth.is_none());
    }
    let source = input("ObservedBadProfile", X);
    let plain = ordinary::compile_with_runtime_guarded_policy(
        source.clone(),
        Profile::NativeV67,
        &[],
        None,
        Default::default(),
        Default::default(),
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
    );
    let observed = ordinary::compile_with_runtime_guarded_policy_observed(
        source,
        Profile::NativeV67,
        &[],
        None,
        Default::default(),
        Default::default(),
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
        &observation,
    );
    assert_eq!(outcome(&plain)?, outcome(&observed)?);
    assert!(observed.is_err());
    assert!(observation.first_failure().is_none());
    Ok(())
}
