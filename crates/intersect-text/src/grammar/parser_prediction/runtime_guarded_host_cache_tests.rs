// Handwritten host-cache controls (language-pack step 3). A real native draft
// compilation is saved, serialized, restored without recompiling and compared
// with the freshly compiled owner; forged key sets and policies are refused.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use crate::grammar::CompiledRuntimeGrammar;
use crate::grammar::GrammarAnalysisWorkBudget;
use crate::grammar::LiveCallerPrediction;
use crate::grammar::SavedRuntimeGrammar;
use intersect_model::definitions::model_definition::ModelCompilation;

const GRAMMAR: &str = "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} '{' (=> Tag)* '}' Item; fragment Tag returns t::Root: 'x'; fragment Item returns t::Root: 'send' (Body | (=> Prefix Body)); fragment Body returns t::Root: 'a' 'b'; fragment Prefix returns t::Root: 'a';";
const MODEL: &str = r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="deferred" nsURI="urn:deferred-choice"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#;

fn draft() -> Result<CompiledRuntimeGrammar, String> {
    let ecore = DefinitionSource::new("memory:host-cache/model.ecore", SourceKind::Ecore, MODEL);
    let mut syntax = DefinitionSource::new(
        "memory:host-cache/grammar.xtext",
        SourceKind::Xtext,
        GRAMMAR,
    );
    syntax.dependencies.push(ecore.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:host-cache/empty.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package hostCacheRules version \"1\";",
    );
    behavior.dependencies.push(ecore.id.clone());
    let expected = syntax.id.clone();
    let input = ModelCompilation {
        materialized_types: vec![],
        structure: ecore.id.clone(),
        namespace: "urn:deferred-choice".into(),
        behavior: behavior.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), syntax, behavior],
            namespaces: vec![],
        },
    };
    let migration = intersect_model::migration::migrate_ecore_bundle(
        input.bundle.clone(),
        &[(ecore.id, "memory:host-cache/model.rcore".into())].into(),
    )
    .map_err(|e| e.to_string())?;
    let converted = migration.remap_model_compilation(&input)?;
    crate::grammar::backend_test_support::runtime(
        &converted,
        &expected,
        GrammarAnalysisWorkBudget::default(),
        &|_, _, _| Ok(()),
    )
}

#[test]
fn compiler_identity_mismatch_and_legacy_caches_require_rebuild() -> Result<(), String> {
    let saved = draft()?.save()?;
    for mode in [RuntimeAdmissionLoadMode::HostCache, RuntimeAdmissionLoadMode::Revalidate] {
        let mut stale = saved.clone();
        stale.compiler_identity = "previous-compiler".into();
        assert!(refused(stale, mode).contains("compiler identity differs"));
    }
    let mut legacy = serde_json::to_value(&saved).map_err(|error| error.to_string())?;
    legacy.as_object_mut().ok_or("Missing cache object")?.remove("compiler_identity");
    assert!(serde_json::from_value::<SavedRuntimeGrammar>(legacy).is_err());
    Ok(())
}

fn round_trip(saved: &SavedRuntimeGrammar) -> Result<SavedRuntimeGrammar, String> {
    let bytes = serde_json::to_vec(saved).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn refused(saved: SavedRuntimeGrammar, mode: RuntimeAdmissionLoadMode) -> String {
    match CompiledRuntimeGrammar::load_host_cache(saved, mode) {
        Ok(_) => "UNEXPECTEDLY LOADED".into(),
        Err(errors) => format!("{errors:?}"),
    }
}

fn symbols(text: &str) -> Vec<crate::grammar::PredictionSymbol> {
    text.split_whitespace()
        .map(|word| crate::grammar::PredictionSymbol::Keyword(word.into()))
        .chain([crate::grammar::PredictionSymbol::End])
        .collect()
}

// The real deferred Item choice, predicted from its outside-repeat caller.
fn predictions(
    owner: &CompiledRuntimeGrammar,
) -> Result<Vec<Result<Option<usize>, String>>, String> {
    let grammar = owner.grammar();
    let admission = owner.admission();
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("No native program")?;
    let Kind::Sequence(item) = &grammar.rules["Item"].body.kind else {
        return Err("No actual Item sequence".into());
    };
    // The parenthesized group wraps the actual Alternative occurrence.
    let mut expression = item.get(1).ok_or("No actual Item choice")?;
    while let Kind::Sequence(inner) = &expression.kind {
        let [child] = inner.as_slice() else { break };
        expression = child;
    }
    if !matches!(expression.kind, Kind::Alternative(_)) {
        return Err("No actual Item alternative".into());
    }
    let base = expression
        .prediction_path
        .as_deref()
        .ok_or("No actual choice path")?;
    let path = format!("{base}/choice");
    if !program.decisions.contains_key(&path) || !admission.decision_is_runtime(&path) {
        return Err(format!("Actual choice {path} is not a runtime decision"));
    }
    let Kind::Sequence(root) = &grammar.rules["Root"].body.kind else {
        return Err("No actual root sequence".into());
    };
    let call = root
        .iter()
        .find(|e| matches!(&e.kind, Kind::Call(name) if name == "Item"))
        .ok_or("No actual outside-repeat caller")?;
    let mut results = Vec::new();
    for text in [
        "{ } send a b",
        "{ } send a a b",
        "{ } send a c",
        "{ } send b",
    ] {
        let source = DefinitionSource::new("memory:host-cache/input", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = crate::grammar::parser_prediction::ReplayTokens::new(&tokens);
        input.position = tokens
            .iter()
            .position(|t| matches!(t, crate::grammar::PredictionSymbol::Keyword(w) if w == "send"))
            .ok_or("No send")?
            + 1;
        let mut work = Some(100_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = live.predict(
            &source.id,
            &[call],
            expression,
            "choice",
            &mut input,
            &mut work,
        );
        results.push(result.map(|choice| choice.map(|c| c * 1000 + input.position)));
    }
    Ok(results)
}

// The real Root `(=> Tag)*` repeat record, predicted from just after '{'.
fn repeat_predictions(
    owner: &CompiledRuntimeGrammar,
) -> Result<Vec<Result<Option<usize>, String>>, String> {
    let grammar = owner.grammar();
    let admission = owner.admission();
    let Kind::Sequence(root) = &grammar.rules["Root"].body.kind else {
        return Err("No actual root sequence".into());
    };
    let expression = root
        .iter()
        .find(|e| e.prediction_path.as_deref() == Some("rule/Root/s2"))
        .ok_or("No actual Root repeat")?;
    let base = expression
        .prediction_path
        .as_deref()
        .ok_or("No actual repeat path")?;
    let path = format!("{base}/repeat");
    if !admission.decision_is_runtime(&path) {
        return Err(format!("Actual repeat {path} is not a runtime decision"));
    }
    let mut results = Vec::new();
    for text in [
        "{ } send a b",
        "{ x } send a b",
        "{ x x } send a b",
        "{ y } send a b",
    ] {
        let source = DefinitionSource::new("memory:host-cache/input", SourceKind::Xtext, text);
        let tokens = symbols(text);
        let mut input = crate::grammar::parser_prediction::ReplayTokens::new(&tokens);
        input.position = 1;
        let mut work = Some(100_000);
        let mut live =
            LiveCallerPrediction::new_runtime_guarded(grammar, &source.id, admission, &mut work)?;
        let result = live.predict(&source.id, &[], expression, "repeat", &mut input, &mut work);
        results.push(result.map(|choice| choice.map(|c| c * 1000 + input.position)));
    }
    Ok(results)
}

#[test]
fn host_cache_round_trip_restores_decisions_policy_depth_and_predictions() -> Result<(), String> {
    let fresh = draft()?;
    let saved = fresh.save()?;
    let counts = saved.admission.clone();
    assert!(counts.record_count() > 0 && counts.choice_count() > 0);
    let decoded = round_trip(&saved)?;
    assert_eq!(decoded.admission, saved.admission);
    for mode in [
        RuntimeAdmissionLoadMode::HostCache,
        RuntimeAdmissionLoadMode::Revalidate,
    ] {
        let loaded = CompiledRuntimeGrammar::load_host_cache(round_trip(&saved)?, mode)
            .map_err(|e| format!("{e:?}"))?;
        let fresh_admission = fresh.admission();
        let admission = loaded.admission();
        assert_eq!(
            fresh_admission.decisions().collect::<Vec<_>>(),
            admission.decisions().collect::<Vec<_>>()
        );
        assert!(admission.decision_count() > 0);
        assert_eq!(fresh_admission.policy(), admission.policy());
        assert_eq!(fresh_admission.call_depth(), admission.call_depth());
        let fresh_grammar = fresh.grammar();
        let grammar = loaded.grammar();
        assert_eq!(fresh_grammar.call_depth, grammar.call_depth);
        let (fresh_program, program) = (
            fresh_grammar
                .parser_prediction
                .as_ref()
                .ok_or("No fresh program")?,
            grammar
                .parser_prediction
                .as_ref()
                .ok_or("No loaded program")?,
        );
        assert_eq!(
            fresh_program.allow_empty_choices,
            program.allow_empty_choices
        );
        assert_eq!(fresh_program.call_depth(), program.call_depth());
        assert_eq!(
            serde_json::to_value(fresh_grammar).map_err(|e| e.to_string())?,
            serde_json::to_value(grammar).map_err(|e| e.to_string())?
        );
        // The restored owner is bound to its OWN loaded allocation.
        let mut work = 1_000_000;
        admission.validate(grammar, program, &mut work)?;
        assert!(
            fresh_admission
                .validate(grammar, program, &mut 1_000_000)
                .is_err()
        );
        let expected = predictions(&fresh)?;
        assert!(expected.iter().any(|r| matches!(r, Ok(Some(_)))));
        assert_eq!(expected, predictions(&loaded)?);
        // The restored repeat RECORD also predicts identically, entering and
        // leaving `(=> Tag)*`.
        let expected = repeat_predictions(&fresh)?;
        assert!(
            expected.iter().any(|r| matches!(r, Ok(Some(_)))),
            "{expected:?}"
        );
        assert_eq!(expected, repeat_predictions(&loaded)?);
        // A loaded owner can itself be saved again, byte-identically.
        assert_eq!(
            serde_json::to_vec(&loaded.save()?).map_err(|e| e.to_string())?,
            serde_json::to_vec(&saved).map_err(|e| e.to_string())?
        );
    }
    Ok(())
}

#[test]
fn host_cache_refuses_changed_key_sets_and_policy() -> Result<(), String> {
    let saved = draft()?.save()?;
    for mode in [
        RuntimeAdmissionLoadMode::HostCache,
        RuntimeAdmissionLoadMode::Revalidate,
    ] {
        let mut removed = round_trip(&saved)?;
        assert!(removed.admission.test_remove_record().is_some());
        let message = refused(removed, mode);
        assert!(message.contains("key set differs"), "{message}");

        let mut added = round_trip(&saved)?;
        assert!(added.admission.test_add_record("rule/Root/s9/repeat"));
        let message = refused(added, mode);
        assert!(message.contains("key set differs"), "{message}");

        // Equal count, different key: bind_owner alone would accept this.
        let mut renamed = round_trip(&saved)?;
        assert!(renamed.admission.test_rename_record("rule/Root/s9/repeat"));
        let message = refused(renamed, mode);
        assert!(message.contains("key set differs"), "{message}");

        let mut renamed_choice = round_trip(&saved)?;
        assert!(
            renamed_choice
                .admission
                .test_rename_choice("rule/Item/s9/choice")
        );
        let message = refused(renamed_choice, mode);
        assert!(message.contains("key set differs"), "{message}");

        for policy in [
            RuntimeGuardedPolicy::LinearFullRepeats,
            RuntimeGuardedPolicy::StrictCompletionScopes,
        ] {
            let mut wrong = round_trip(&saved)?;
            wrong.admission.test_set_policy(policy);
            let message = refused(wrong, mode);
            assert!(message.contains("different proof class"), "{message}");

            // Consistent policy and record classes, no choices: the saved
            // policy must still match the loaded Requirement variant.
            let mut wrong = round_trip(&saved)?;
            wrong.admission.test_set_consistent_policy(policy, false);
            let message = refused(wrong, mode);
            assert!(
                message.contains("differs from saved proof policy"),
                "{message}"
            );

            // Choices exist only under the input-checked policy.
            let mut wrong = round_trip(&saved)?;
            wrong.admission.test_set_consistent_policy(policy, true);
            let message = refused(wrong, mode);
            assert!(message.contains("require the input-checked"), "{message}");
        }

        // A differing call depth cannot be restored onto the saved program.
        let mut deeper = round_trip(&saved)?;
        let grammar = &mut deeper.grammar;
        let program = grammar.parser_prediction.as_mut().ok_or("No program")?;
        program.call_depth = program.call_depth.wrapping_add(1);
        let message = refused(deeper, mode);
        assert!(message.contains("call depth"), "{message}");

        // Unknown fields are refused by the strict wire form.
        let mut value = serde_json::to_value(&saved).map_err(|e| e.to_string())?;
        value["admission"]["extra"] = serde_json::json!(1);
        assert!(serde_json::from_value::<SavedRuntimeGrammar>(value).is_err());
    }
    // A forged record body with the exact key set is trusted by HostCache
    // (the documented convention) but refused by Revalidate.
    let mut forged = round_trip(&saved)?;
    assert!(forged.admission.test_shift_record_graph());
    assert!(
        CompiledRuntimeGrammar::load_host_cache(
            round_trip(&forged)?,
            RuntimeAdmissionLoadMode::HostCache
        )
        .is_ok()
    );
    let message = refused(forged, RuntimeAdmissionLoadMode::Revalidate);
    assert!(message.contains("revalidation failed"), "{message}");
    // A swapped choice proof kind is likewise refused by Revalidate, which
    // re-verifies the stored kind's own witness as seal does.
    let mut flipped = round_trip(&saved)?;
    assert!(flipped.admission.test_flip_choice_proofs());
    let message = refused(flipped, RuntimeAdmissionLoadMode::Revalidate);
    assert!(message.contains("choice revalidation failed"), "{message}");
    Ok(())
}

#[test]
fn host_cache_save_requires_a_bound_live_admission() -> Result<(), String> {
    let unsealed = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    let message = unsealed
        .save()
        .err()
        .ok_or("Unsealed admission was saved")?;
    assert!(message.contains("owner-bound"), "{message}");
    let mut failed = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    failed.invalidate_inventory();
    let message = failed.save().err().ok_or("Failed admission was saved")?;
    assert!(message.contains("failed"), "{message}");
    // A bound admission that later failed is no longer saveable either.
    let loaded = CompiledRuntimeGrammar::load_host_cache(
        draft()?.save()?,
        RuntimeAdmissionLoadMode::HostCache,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut bound = {
        let admission = loaded.admission();
        let mut copy = RuntimeGuardedAdmission::new_with_policy(admission.policy());
        copy.records = admission.records.clone();
        copy.choices = admission.choices.clone();
        copy.owner = admission.owner;
        copy
    };
    assert!(bound.save().is_ok());
    bound.test_poison();
    let message = bound.save().err().ok_or("Poisoned admission was saved")?;
    assert!(message.contains("failed"), "{message}");
    Ok(())
}
