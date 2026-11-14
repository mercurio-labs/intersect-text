// Source-level ownership bookkeeping controls only. Genuine imported grammar,
// strict proof and complete runtime models are exercised by the Xtext controls.
#[test]
fn owner_reservation_zero_short_exact_retention_and_sticky_failure() -> Result<(), String> {
    let required = 2 + OWNER_BYTES.div_ceil(64) + OWNER_PUBLICATION_WORK;
    let program: ParserPrediction = serde_json::from_value(serde_json::json!({
        "states":[],"rules":{},"decisions":{},"first_guards":{},
        "runtime_requirement":{"LinearFullRepeats":{"decisions":["rule/Root/repeat"]}}
    }))
    .map_err(|error| error.to_string())?;
    println!(
        "RUNTIME_OWNER_LAYOUT {}",
        serde_json::json!({
            "legacy_composite_reservation":OWNER_BYTES,
            "owner_tokens":std::mem::size_of::<Owner>(),"admission":std::mem::size_of::<RuntimeGuardedAdmission>(),
            "owner_bytes":OWNER_BYTES,"fixed_publication":OWNER_PUBLICATION_WORK,"reservation_work":required,
            "prediction":std::mem::size_of::<ParserPrediction>()
        })
    );
    for allowance in 0..required {
        let mut admission = RuntimeGuardedAdmission::new();
        let mut work = allowance;
        assert!(admission.reserve_owner(&program, &mut work).is_err());
        assert_eq!(work, 0);
        assert!(admission.failed);
        assert!(!admission.owner_allocation_reserved());
        assert!(matches!(admission.owner, Owner::Unsealed));
    }
    let mut exact = RuntimeGuardedAdmission::new();
    let mut work = required;
    exact.reserve_owner(&program, &mut work)?;
    assert_eq!(work, 0);
    assert_eq!(exact.logical_bytes, OWNER_BYTES);
    assert!(exact.owner_allocation_reserved());
    assert_eq!(exact.decision_count(), 0); // reservation is not semantic authority
    assert!(exact.reserve_owner(&program, &mut 1000).is_err());
    assert!(exact.failed && !exact.owner_allocation_reserved());

    let mut limit = RuntimeGuardedAdmission::new();
    limit.logical_bytes = LIMIT - OWNER_BYTES;
    let mut limit_work = required;
    limit.reserve_owner(&program, &mut limit_work)?;
    assert_eq!(limit.logical_bytes, LIMIT);
    let mut exceeded = RuntimeGuardedAdmission::new();
    exceeded.logical_bytes = LIMIT - OWNER_BYTES + 1;
    let mut exceeded_work = required;
    let error = exceeded
        .reserve_owner(&program, &mut exceeded_work)
        .err()
        .ok_or("Over-bound reservation succeeded")?;
    assert!(error.contains("retention bound exceeded"));
    assert_eq!(exceeded_work, required - 1);
    assert!(exceeded.failed && !exceeded.owner_allocation_reserved());
    let mut missing_program = program.clone();
    missing_program.runtime_requirement = None;
    let mut missing_marker = RuntimeGuardedAdmission::new();
    let mut missing_work = required;
    let error = missing_marker
        .reserve_owner(&missing_program, &mut missing_work)
        .err()
        .ok_or("Missing marker reserved")?;
    assert!(error.contains("Runtime requirement disappeared"));
    assert_eq!(missing_work, 0);
    assert!(missing_marker.failed && !missing_marker.owner_allocation_reserved());
    let mut fresh = RuntimeGuardedAdmission::new();
    let mut fresh_work = required;
    fresh.reserve_owner(&program, &mut fresh_work)?;
    assert!(fresh.owner_allocation_reserved());
    Ok(())
}

impl RuntimeGuardedAdmission {
    // Test-only assertions, NOT a factory: no permit, owner, companion or
    // compilation is returned. Only compiler's child test can mint its permit.
    pub(crate) fn check_owner_publication_failure_boundaries() -> Result<(), String> {
        let permit = &crate::grammar::RuntimeOwnerPublicationPermit { _private: () };
        let grammar: Grammar = serde_json::from_value(serde_json::json!({
            "profile":"intersect.xtext.native.v80","name":"Owner","entry":"Root","rules":{},
            "parser_prediction":{"states":[],"rules":{},"decisions":{},"first_guards":{},
                "runtime_requirement":{"LinearFullRepeats":{"decisions":["rule/Root/repeat"]}}}
        })).map_err(|error| error.to_string())?;
        let grammar = &grammar;
        let program = grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing control program")?;
        let mut unsealed = RuntimeGuardedAdmission::new();
        let error = unsealed
            .bind_owner(permit, grammar)
            .err()
            .ok_or("Unsealed owner published")?;
        assert!(error.contains("not reserved or is already bound"));
        assert!(unsealed.failed);
        let record = || Binding {
            proof_class: RuntimeGuardedPolicy::LinearFullRepeats,
            owner: "Root".into(),
            callee: "Target".into(),
            source: SourceId::for_text("memory:owner-control", "R"),
            callee_source: SourceId::for_text("memory:owner-control", "R"),
            declaration: 0..1,
            occurrence: 0..1,
            full: 0..1,
            operand: 0..1,
            graph: [0; 7],
        };
        let reserved = || -> Result<RuntimeGuardedAdmission, String> {
            let mut admission = RuntimeGuardedAdmission::new();
            admission
                .records
                .insert("rule/Root/repeat".into(), record());
            admission.reserve_owner(program, &mut 10_000)?;
            Ok(admission)
        };
        let mut admission = reserved()?;
        let error = admission
            .validate(grammar, program, &mut 1000)
            .err()
            .ok_or("Unbound owner validated")?;
        assert!(error.contains("no published immutable owner"));
        assert!(!admission.failed); // unavailable reservation was never authority
        admission.bind_owner(permit, grammar)?;
        assert!(!admission.owner_allocation_reserved());
        // This synthetic fixture tests bookkeeping only, not semantic validity.
        // No synthetic semantic execution is attempted or claimed.
        let clone = grammar.clone();
        let foreign = &clone;
        let foreign_program = foreign
            .parser_prediction
            .as_ref()
            .ok_or("Missing cloned program")?;
        assert!(
            admission
                .validate(foreign, foreign_program, &mut 1000)
                .is_err()
        );
        let error = admission
            .bind_owner(permit, grammar)
            .err()
            .ok_or("Rebound owner published")?;
        assert!(error.contains("not reserved or is already bound"));
        assert!(admission.failed);
        assert!(admission.validate(grammar, program, &mut 1000).is_err());
        let mut marker_mismatch = reserved()?;
        let error = marker_mismatch
            .bind_owner(permit, &clone)
            .err()
            .ok_or("Different marker published")?;
        assert!(error.contains("differs from its sealed marker"));
        assert!(marker_mismatch.failed && !marker_mismatch.owner_allocation_reserved());
        let mut fresh = reserved()?;
        fresh.bind_owner(permit, grammar)?;
        assert!(!fresh.failed);
        Ok(())
    }
}

#[test]
fn unsealed_marker_mismatch_rebind_and_unbound_validation_fail_closed() -> Result<(), String> {
    RuntimeGuardedAdmission::check_owner_publication_failure_boundaries()
}
