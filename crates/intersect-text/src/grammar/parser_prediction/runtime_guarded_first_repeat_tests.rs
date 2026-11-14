// Kernel source/graph and finite-ledger controls. These fixtures are not native
// execution authority; the Xtext child tests exercise genuine imported sources.
use super::super::super::{Decision, PredictionSymbol};
use std::collections::BTreeSet;

const PATH: &str = "rule/Root/s0/repeat";

fn first_fixture(cardinality: char, grouped: bool) -> Result<(Grammar, ParserPrediction), String> {
    let text = if grouped {
        format!("Root: (-> Target){cardinality}; Target: 'x';")
    } else {
        format!("Root: -> Target{cardinality}; Target: 'x';")
    };
    let root_end = text.find(';').ok_or("Missing source delimiter")? + 1;
    let target_start = text.find("Target:").ok_or("Missing callee declaration")?;
    let begin = if grouped {
        text.find('(')
    } else {
        text.find("->")
    }
    .ok_or("Missing source occurrence")?;
    let guard_start = text.find("->").ok_or("Missing First marker")?;
    let guard_end = if grouped {
        text.find(')').ok_or("Missing group end")?
    } else {
        root_end - 1
    };
    let source = SourceId::for_text("memory:first-repeat-kernel.xtext", &text);
    let guard_path = if grouped {
        "rule/Root/s0/s0"
    } else {
        "rule/Root/s0"
    };
    let guard = serde_json::json!({
        "kind":{"Call":"Target"},"cardinality":if grouped {'1'}else{cardinality},
        "first_set_predicated":true,"prediction_path":guard_path,
        "original_range":{"start":guard_start,"end":guard_end}
    });
    let repeat = if grouped {
        serde_json::json!({"kind":{"Sequence":[guard]},"cardinality":cardinality,
            "prediction_path":"rule/Root/s0","original_range":{"start":begin,"end":root_end-1}})
    } else {
        guard
    };
    let grammar: Grammar = serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"FirstKernel","entry":"Root",
        "rules":{
            "Root":{"class":"Root","body":{"kind":{"Sequence":[repeat]},"cardinality":"1",
                "prediction_path":"rule/Root","original_range":{"start":0,"end":root_end}}},
            "Target":{"class":"Root","fragment":true,"body":{"kind":{"Literal":"x"},"cardinality":"1"}}
        },
        "declaration_sources":{
            "Root":{"source":source,"start":0,"end":root_end},
            "Target":{"source":source,"start":target_start,"end":text.len()}
        }
    })).map_err(|error|error.to_string())?;
    let mut program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: true,
        states: vec![],
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: [(
            guard_path.into(),
            [PredictionSymbol::Keyword("x".into())].into(),
        )]
        .into(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    for name in ["Root", "Target"] {
        let start = program.state()?;
        let stop = program.state()?;
        program.states[stop].rule_stop = true;
        program.rules.insert(name.into(), (start, stop));
    }
    program.decisions.insert(
        "@document-recovery".into(),
        Decision {
            entries: vec![],
            exit: None,
        },
    );
    for (name, rule) in &grammar.rules {
        let (start, stop) = program.rules[name];
        program.expression(
            &rule.body,
            start,
            stop,
            &format!("rule/{name}"),
            (0, None, None),
            None,
        )?;
    }
    Ok((grammar, program))
}

fn first_bound(grammar: &Grammar, program: &ParserPrediction) -> Result<Binding, String> {
    binding(
        grammar,
        program,
        PATH,
        &mut 10_000,
        &mut 0,
        RuntimeGuardedPolicy::StrictCompletionScopes,
        None,
    )?
    .ok_or_else(|| "Exact First repeat was not bound".into())
}

#[test]
fn strict_first_star_plus_and_direct_quantifiers_bind_actual_graph() -> Result<(), String> {
    for cardinality in ['*', '+'] {
        for grouped in [false, true] {
            let (grammar, program) = first_fixture(cardinality, grouped)?;
            let before = serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?;
            let record = first_bound(&grammar, &program)?;
            assert_eq!(record.owner, "Root");
            assert_eq!(record.callee, "Target");
            verify_strict_record(&program, PATH, &record, &mut 10_000)?;
            assert!(
                binding(
                    &grammar,
                    &program,
                    PATH,
                    &mut 10_000,
                    &mut 0,
                    RuntimeGuardedPolicy::LinearFullRepeats,
                    None
                )?
                .is_none()
            );
            let [start, body, _, stop, end, _, _] = record.graph;
            let epsilon = |index: usize| {
                program.states[index]
                    .edges
                    .iter()
                    .map(|edge| match edge {
                        Edge::Epsilon(next) => Ok(*next),
                        _ => Err("Unexpected graph edge".to_string()),
                    })
                    .collect::<Result<Vec<_>, _>>()
            };
            if cardinality == '+' {
                assert_eq!(epsilon(start)?, vec![body]); // no zero-iteration entry
                assert_eq!(epsilon(stop)?, vec![body, end]);
            } else {
                assert_eq!(epsilon(start)?, vec![body, end]);
                assert_eq!(epsilon(stop)?, vec![start]);
            }
            assert_eq!(
                serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?,
                before
            );
        }
    }
    Ok(())
}

#[test]
fn strict_first_binding_rejects_guard_loop_source_and_hidden_drift() -> Result<(), String> {
    for cardinality in ['*', '+'] {
        let (grammar, program) = first_fixture(cardinality, true)?;
        let record = first_bound(&grammar, &program)?;
        for mode in 0..6 {
            let mut changed = program.clone();
            match mode {
                0 => {
                    changed.states[record.graph[0]].edges = vec![
                        Edge::Epsilon(record.graph[1]),
                        Edge::Epsilon(record.graph[4]),
                        Edge::Epsilon(record.graph[1]),
                    ]
                }
                1 => changed.states[record.graph[3]].edges = vec![],
                2 => {
                    changed.states[record.graph[1]].edges = vec![Edge::First {
                        path: "foreign".into(),
                        target: record.graph[2],
                    }]
                }
                3 => {
                    changed.first_guards.clear();
                }
                4 => {
                    for symbols in changed.first_guards.values_mut() {
                        symbols.clear();
                    }
                }
                _ => {
                    changed.states[record.graph[2]].edges = vec![Edge::Call {
                        target: record.graph[5],
                        follow: record.graph[4],
                    }]
                }
            }
            assert!(
                binding(
                    &grammar,
                    &changed,
                    PATH,
                    &mut 10_000,
                    &mut 0,
                    RuntimeGuardedPolicy::StrictCompletionScopes,
                    None
                )?
                .is_none(),
                "mode={mode}"
            );
        }
        let mut hidden = grammar.clone();
        hidden
            .rules
            .get_mut("Target")
            .ok_or("Missing callee")?
            .hidden = Some(vec!["WS".into()]);
        assert!(
            binding(
                &hidden,
                &program,
                PATH,
                &mut 10_000,
                &mut 0,
                RuntimeGuardedPolicy::StrictCompletionScopes,
                None
            )?
            .is_none()
        );
        let mut source = grammar.clone();
        let crate::grammar::Kind::Sequence(items) = &mut source
            .rules
            .get_mut("Root")
            .ok_or("Missing owner")?
            .body
            .kind
        else {
            return Err("Missing sequence".into());
        };
        items[0].prediction_path = Some("foreign".into());
        assert!(first_bound(&source, &program).is_err());
    }
    Ok(())
}

#[test]
fn strict_first_bare_repeat_nullable_and_unknown_callee_are_not_proofs() -> Result<(), String> {
    let (grammar, program) = first_fixture('*', true)?;
    let mut bare = grammar.clone();
    let Kind::Sequence(items) = &mut bare.rules.get_mut("Root").ok_or("Missing owner")?.body.kind
    else {
        return Err("Missing sequence".into());
    };
    let Kind::Sequence(children) = &mut items[0].kind else {
        return Err("Missing group".into());
    };
    children[0].first_set_predicated = false;
    assert!(
        binding(
            &bare,
            &program,
            PATH,
            &mut 10_000,
            &mut 0,
            RuntimeGuardedPolicy::StrictCompletionScopes,
            None
        )?
        .is_none()
    );
    let mut nullable = grammar.clone();
    nullable
        .rules
        .get_mut("Target")
        .ok_or("Missing callee")?
        .body
        .cardinality = '?';
    let error = first_bound(&nullable, &program)
        .err()
        .ok_or("Nullable operand bound")?;
    assert!(error.contains("nonnullable operand"));
    let mut unknown = grammar.clone();
    unknown.rules.remove("Target");
    assert!(first_bound(&unknown, &program).is_err());
    Ok(())
}

#[test]
fn strict_first_binding_zero_short_exact_and_collection_failure_are_sticky() -> Result<(), String> {
    let (grammar, program) = first_fixture('+', true)?;
    let mut baseline = 10_000;
    let record = binding(
        &grammar,
        &program,
        PATH,
        &mut baseline,
        &mut 0,
        RuntimeGuardedPolicy::StrictCompletionScopes,
        None,
    )?
    .ok_or("Missing binding")?;
    let used = 10_000 - baseline;
    for allowance in [0, 1, used - 1, used] {
        let mut work = allowance;
        let actual = binding(
            &grammar,
            &program,
            PATH,
            &mut work,
            &mut 0,
            RuntimeGuardedPolicy::StrictCompletionScopes,
            None,
        );
        if allowance == used {
            assert_eq!(actual?.as_ref(), Some(&record));
            assert_eq!(work, 0);
        } else {
            assert!(actual.is_err());
            assert_eq!(work, 0);
        }
    }
    let mut admission =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    assert!(admission.collect(&grammar, &program, &mut 0, None).is_err());
    assert!(admission.failed && admission.records.is_empty());
    assert!(
        admission
            .collect(&grammar, &program, &mut 10_000, None)
            .is_err()
    );
    Ok(())
}

// Kernel companion to the imported FeatureChain-shaped Machine control.
// The sole change under test is the explicit sufficient static proof policy.
fn first_feature_chain_fixture() -> Result<(Grammar, ParserPrediction), String> {
    let (mut grammar, mut program) = first_fixture('*', true)?;
    let text = "Root: (-> Target)* 'z'; Target: Name ('.' Name)+; Name: -> 'x' | 'y';";
    let root_end = text.find(';').ok_or("Missing Root end")? + 1;
    let target_start = text.find("Target:").ok_or("Missing Target start")?;
    let target_end = text[target_start..].find(';').ok_or("Missing Target end")? + target_start + 1;
    let name_start = text.find("Name:").ok_or("Missing Name start")?;
    let source = SourceId::for_text("memory:first-feature-chain-kernel.xtext", text);
    for (name, start, end) in [("Root", 0, root_end), ("Target", target_start, target_end)] {
        let origin = grammar
            .declaration_sources
            .get_mut(name)
            .ok_or("Missing source row")?;
        origin.source = source.clone();
        origin.start = start;
        origin.end = end;
    }
    let root = &mut grammar.rules.get_mut("Root").ok_or("Missing Root")?.body;
    root.original_range = Some(0..root_end);
    let Kind::Sequence(root_items) = &mut root.kind else {
        return Err("Missing Root sequence".into());
    };
    root_items.push(
        serde_json::from_value(serde_json::json!({"kind":{"Literal":"z"},"cardinality":"1"}))
            .map_err(|e| e.to_string())?,
    );
    grammar
        .rules
        .get_mut("Target")
        .ok_or("Missing Target")?
        .body = serde_json::from_value(serde_json::json!({"kind":{"Sequence":[
            {"kind":{"Call":"Name"},"cardinality":"1"},
            {"kind":{"Sequence":[
                {"kind":{"Literal":"."},"cardinality":"1"},
                {"kind":{"Call":"Name"},"cardinality":"1"}
            ]},"cardinality":"+"}
        ]},"cardinality":"1"}))
    .map_err(|e| e.to_string())?;
    let name_rule = serde_json::from_value(serde_json::json!({
        "class":"Root","fragment":true,"body":{"kind":{"Alternative":[
            {"kind":{"Literal":"x"},"cardinality":"1","first_set_predicated":true},
            {"kind":{"Literal":"y"},"cardinality":"1"}
        ]},"cardinality":"1"}
    }))
    .map_err(|e| e.to_string())?;
    grammar.rules.insert("Name".into(), name_rule);
    let origin = serde_json::from_value(
        serde_json::json!({"source":source,"start":name_start,"end":text.len()}),
    )
    .map_err(|e| e.to_string())?;
    grammar.declaration_sources.insert("Name".into(), origin);
    program.states.clear();
    program.rules.clear();
    program.decisions.clear();
    // Reuse the original fixture's canonical source-recovery switch BEFORE
    // expression construction; the builder mints each exact occurrence entry.
    program.decisions.insert(
        "@document-recovery".into(),
        Decision {
            entries: vec![],
            exit: None,
        },
    );
    program.first_guards.insert(
        "rule/Name/a0".into(),
        [PredictionSymbol::Keyword("x".into())].into(),
    );
    program.first_guards.insert(
        "rule/Root/s0/s0".into(),
        [
            PredictionSymbol::Keyword("x".into()),
            PredictionSymbol::Keyword("y".into()),
        ]
        .into(),
    );
    for name in grammar.rules.keys() {
        let start = program.state()?;
        let stop = program.state()?;
        program.states[stop].rule_stop = true;
        program.rules.insert(name.clone(), (start, stop));
    }
    for (name, rule) in &grammar.rules {
        let (start, stop) = program.rules[name];
        program.expression(
            &rule.body,
            start,
            stop,
            &format!("rule/{name}"),
            (0, None, None),
            None,
        )?;
    }
    Ok((grammar, program))
}

#[test]
fn strict_completion_existing_policy_keeps_legacy_and_exact_short_work() -> Result<(), String> {
    let (grammar, program) = first_feature_chain_fixture()?;
    let before = serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?;
    let record = first_bound(&grammar, &program)?;
    let legacy = program
        .verify_completion_scope(record.graph[2], record.graph[3], &mut 10000)
        .err()
        .ok_or("Legacy completion unexpectedly admitted nested First")?;
    assert!(legacy.contains("Guarded shared-prefix"), "{legacy}");
    let mut remaining = 10000;
    verify_strict_record(&program, PATH, &record, &mut remaining)?;
    let used = 10000 - remaining;
    assert!(used > 1 && used < 10000);
    for allowance in [0, used - 1, used] {
        let mut remaining = allowance;
        let result = verify_strict_record(&program, PATH, &record, &mut remaining);
        if allowance == used {
            result?;
        } else {
            assert!(result.err().ok_or("Short proof passed")?.contains("budget"));
        }
        assert_eq!(remaining, 0);
    }
    assert_eq!(
        serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?,
        before
    );
    println!(
        "STRICT_COMPLETION_POLICY {}",
        serde_json::json!({"legacy":legacy,"complete_policy_work":used,"whole_source_and_program_unchanged":true})
    );
    Ok(())
}

#[test]
fn strict_completion_existing_policy_rejects_unbound_hidden_and_wrong_profile() -> Result<(), String>
{
    let (grammar, program) = first_feature_chain_fixture()?;
    let record = first_bound(&grammar, &program)?;
    for empty in [false, true] {
        let mut changed = program.clone();
        if empty {
            changed
                .first_guards
                .get_mut("rule/Name/a0")
                .ok_or("Missing guard")?
                .clear();
        } else {
            changed.first_guards.remove("rule/Name/a0");
        }
        let error = verify_strict_record(&changed, PATH, &record, &mut 10000)
            .err()
            .ok_or("Unbound guard admitted")?;
        assert!(
            error.contains("Unbound completion first-token guard"),
            "{error}"
        );
    }
    let mut hidden = program.clone();
    let guarded = hidden
        .states
        .iter()
        .position(
            |state| matches!(state.edges.as_slice(),[Edge::First{path,..}] if path=="rule/Name/a0"),
        )
        .ok_or("Missing real First state")?;
    let old = hidden.states[guarded].edges.clone();
    let shifted = hidden.state()?;
    hidden.states[shifted].edges = old;
    hidden.states[guarded].edges =
        vec![Edge::Token(PredictionSymbol::Keyword("p".into()), shifted)];
    let error = verify_strict_record(&hidden, PATH, &record, &mut 10000)
        .err()
        .ok_or("Hidden guard admitted")?;
    assert!(error.contains("guard visibility"), "{error}");
    let mut admission =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    // Isolate the seal profile boundary with the already checked root binding.
    // The genuine imported-source test exercises full collection separately.
    admission.records.insert(PATH.into(), record);
    let mut wrong = grammar.clone();
    wrong.profile = Profile::NativeV67;
    let mut changed = program.clone();
    let error = admission
        .seal(&wrong, &mut changed, &mut 10000)
        .err()
        .ok_or("Wrong profile sealed")?;
    assert!(error.contains("requires Native80"), "{error}");
    assert!(admission.failed);
    assert!(changed.runtime_requirement.is_none());
    Ok(())
}

// The original fixture's canonical expression builder and recovery marker mint
// the exact real source occurrences; no state number grants runtime authority.
fn input_checked_bound(grammar: &Grammar, program: &ParserPrediction) -> Result<Binding, String> {
    binding(
        grammar,
        program,
        PATH,
        &mut 10000,
        &mut 0,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        None,
    )?
    .ok_or_else(|| "Missing exact input-checked binding".into())
}
fn literal_loop(program: &ParserPrediction) -> Result<(&Decision, Vec<usize>), String> {
    let decision = program
        .decisions
        .get("rule/Target/s1/repeat")
        .ok_or("Missing real literal repeat")?;
    let [body, end] = decision.entries.as_slice() else {
        return Err("Invalid repeat".into());
    };
    let points = program
        .states
        .iter()
        .enumerate()
        .filter_map(|(index, state)| {
            matches!(state.edges.as_slice(),[Edge::Epsilon(a),Edge::Epsilon(b)]
            if a==body && b==end)
            .then_some(index)
        })
        .collect();
    Ok((decision, points))
}

#[test]
fn input_checked_literal_loop_uses_actual_shape_and_same_finite_proof_pool() -> Result<(), String> {
    let (grammar, program) = first_feature_chain_fixture()?;
    let before = serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?;
    let (decision, points) = literal_loop(&program)?;
    let mut remaining = 10000;
    assert!(program.input_checked_literal_repeat(
        "rule/Target/s1/repeat",
        decision,
        &points,
        &mut remaining
    )?);
    let used = 10000 - remaining;
    assert!(used > 1 && used < 10000);
    for allowance in [0, used - 1, used] {
        let mut work = allowance;
        let result = program.input_checked_literal_repeat(
            "rule/Target/s1/repeat",
            decision,
            &points,
            &mut work,
        );
        if allowance == used {
            assert!(result?);
        } else {
            assert!(result.is_err());
        }
        assert_eq!(work, 0);
    }
    let record = input_checked_bound(&grammar, &program)?;
    let mut remaining = 10000;
    verify_input_checked_record(&program, PATH, &record, &mut remaining)?;
    let whole_work = 10000 - remaining;
    for allowance in [0, whole_work - 1, whole_work] {
        let mut remaining = allowance;
        let result = verify_input_checked_record(&program, PATH, &record, &mut remaining);
        if allowance == whole_work {
            result?;
        } else {
            assert!(result.is_err());
        }
        assert_eq!(remaining, 0);
    }
    assert!(verify_strict_record(&program, PATH, &record, &mut 10000).is_err());
    assert!(
        verify_input_checked_record(
            &program,
            PATH,
            &first_bound(&grammar, &program)?,
            &mut 10000
        )
        .is_err()
    );
    assert_eq!(
        serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?,
        before
    );
    println!(
        "INPUT_CHECKED_LITERAL_LOOP {}",
        serde_json::json!({
        "actual_repeat":"rule/Target/s1/repeat","eligibility_work":used,
        "whole_scope_work":whole_work,"source_and_program_unchanged":true,
        "static_disjointness_claim":false})
    );
    Ok(())
}

#[test]
fn input_checked_ineligible_prefix_and_nonloop_keep_existing_static_route() -> Result<(), String> {
    let (_, program) = first_feature_chain_fixture()?;
    let (decision, points) = literal_loop(&program)?;
    let body = decision.entries[0];
    let [Edge::Token(_, call)] = program.states[body].edges.as_slice() else {
        return Err("Missing prefix".into());
    };
    let call = *call;
    for mode in 0..7 {
        let mut changed = program.clone();
        match mode {
            0 => {
                changed.states[body].edges =
                    vec![Edge::Token(PredictionSymbol::Keyword(String::new()), call)]
            }
            1 => {
                changed.states[body].edges =
                    vec![Edge::Token(PredictionSymbol::Terminal("ID".into()), call)]
            }
            2 => changed.states[body].edges = changed.states[call].edges.clone(), // Call before token
            3 => {
                changed.states[body].edges = vec![Edge::First {
                    path: "rule/Name/a0".into(),
                    target: call,
                }]
            }
            4 => {
                if let [Edge::Call { follow, .. }] = changed.states[call].edges.as_mut_slice() {
                    *follow = decision.entries[1];
                }
            }
            5 => changed.states[points[0]].edges.reverse(),
            _ => changed.states[body].accepting = true,
        }
        let (actual, actual_points) = literal_loop(&changed)?;
        assert!(
            !changed.input_checked_literal_repeat(
                "rule/Target/s1/repeat",
                actual,
                &actual_points,
                &mut 10000
            )?,
            "mode={mode}"
        );
    }
    assert!(!program.input_checked_literal_repeat(
        "rule/Target/s1/choice",
        decision,
        &points,
        &mut 10000
    )?);
    assert!(!program.input_checked_literal_repeat(
        "rule/Target/s1/repeat",
        decision,
        &[],
        &mut 10000
    )?);
    // A real source-bound optional uses the same builder but cannot prove a
    // backedge. Test this structural exclusion on the actual graph continuation.
    let mut optional = program.clone();
    if let [Edge::Call { follow, .. }] = optional.states[call].edges.as_mut_slice() {
        let done = *follow;
        optional.states[done].edges = vec![Edge::Epsilon(decision.entries[1])];
    }
    let (actual, actual_points) = literal_loop(&optional)?;
    assert!(!optional.input_checked_literal_repeat(
        "rule/Target/s1/repeat",
        actual,
        &actual_points,
        &mut 10000
    )?);
    // New mode's non-deferred proof is exactly the already-qualified bounded
    // static worker, with the same result and original caller debits.
    for path in ["rule/Name", "rule/Target/s1/repeat"] {
        let mut old = 10000;
        let mut new = 10000;
        let a = program.verify_completion_decision_with_verification(
            path,
            &mut old,
            super::super::super::CompletionVerification::BoundedGuardOverapproximation,
        );
        let b = program.verify_completion_decision_with_verification(
            path,
            &mut new,
            super::super::super::CompletionVerification::InputCheckedCompletion,
        );
        assert_eq!(a, b);
        assert_eq!(old, new);
    }
    Ok(())
}

#[test]
fn input_checked_keeps_unbound_visibility_recursive_and_profile_rejections() -> Result<(), String> {
    let (grammar, program) = first_feature_chain_fixture()?;
    let record = input_checked_bound(&grammar, &program)?;
    for empty in [false, true] {
        let mut changed = program.clone();
        if empty {
            changed
                .first_guards
                .get_mut("rule/Name/a0")
                .ok_or("Missing guard")?
                .clear();
        } else {
            changed.first_guards.remove("rule/Name/a0");
        }
        let error = verify_input_checked_record(&changed, PATH, &record, &mut 10000)
            .err()
            .ok_or("Unbound guard admitted")?;
        assert!(
            error.contains("Unbound completion first-token guard"),
            "{error}"
        );
    }
    let mut hidden = program.clone();
    let guarded = hidden
        .states
        .iter()
        .position(|state| {
            matches!(state.edges.as_slice(),
        [Edge::First{path,..}] if path=="rule/Name/a0")
        })
        .ok_or("Missing guard")?;
    let shifted = hidden.state()?;
    hidden.states[shifted].edges = hidden.states[guarded].edges.clone();
    hidden.states[guarded].edges =
        vec![Edge::Token(PredictionSymbol::Keyword("p".into()), shifted)];
    let error = verify_input_checked_record(&hidden, PATH, &record, &mut 10000)
        .err()
        .ok_or("Hidden guard admitted")?;
    assert!(error.contains("guard visibility"), "{error}");
    let mut recursive = program.clone();
    recursive
        .recursive_decisions
        .insert("rule/Target/s1/repeat".into());
    let error = verify_input_checked_record(&recursive, PATH, &record, &mut 10000)
        .err()
        .ok_or("Recursive unqualified commitment admitted")?;
    assert!(
        error.contains("Unqualified completion commitment"),
        "{error}"
    );
    let mut wrong = grammar.clone();
    wrong.profile = Profile::NativeV79;
    let mut admission = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    admission.records.insert(PATH.into(), record);
    let mut changed = program.clone();
    assert!(admission.seal(&wrong, &mut changed, &mut 10000).is_err());
    assert!(admission.failed);
    assert!(changed.runtime_requirement.is_none());
    assert!(admission.seal(&grammar, &mut changed, &mut 10000).is_err());
    Ok(())
}

// The actual expression builder creates this source-shaped Call/literal loop.
// Only the preexisting checked Root occurrence supplies admission authority.
fn input_checked_call_suffix_fixture(
    cardinality: char,
) -> Result<(Grammar, ParserPrediction), String> {
    let (mut grammar, mut program) = first_feature_chain_fixture()?;
    let text = format!(
        "Root: (-> Target)* 'z'; Target: (Name '::'){cardinality} Name 'q'; Name: -> 'x' | 'y';"
    );
    let source = SourceId::for_text("memory:call-suffix-kernel.xtext", &text);
    for name in ["Root", "Target", "Name"] {
        let start = text
            .find(&format!("{name}:"))
            .ok_or("Missing declaration")?;
        let end = start + text[start..].find(';').ok_or("Missing declaration end")? + 1;
        let origin = grammar
            .declaration_sources
            .get_mut(name)
            .ok_or("Missing source")?;
        origin.source = source.clone();
        origin.start = start;
        origin.end = end;
    }
    grammar
        .rules
        .get_mut("Target")
        .ok_or("Missing Target")?
        .body = serde_json::from_value(serde_json::json!({"kind":{"Sequence":[
            {"kind":{"Sequence":[{"kind":{"Call":"Name"},"cardinality":"1"},
                {"kind":{"Literal":"::"},"cardinality":"1"}]},"cardinality":cardinality},
            {"kind":{"Call":"Name"},"cardinality":"1"},
            {"kind":{"Literal":"q"},"cardinality":"1"}
        ]},"cardinality":"1"}))
    .map_err(|e| e.to_string())?;
    program.states.clear();
    program.rules.clear();
    program.decisions.clear();
    program.decisions.insert(
        "@document-recovery".into(),
        Decision {
            entries: vec![],
            exit: None,
        },
    );
    for name in grammar.rules.keys() {
        let start = program.state()?;
        let stop = program.state()?;
        program.states[stop].rule_stop = true;
        program.rules.insert(name.clone(), (start, stop));
    }
    for (name, rule) in &grammar.rules {
        let (start, stop) = program.rules[name];
        program.expression(
            &rule.body,
            start,
            stop,
            &format!("rule/{name}"),
            (0, None, None),
            None,
        )?;
    }
    Ok((grammar, program))
}
fn input_checked_call_suffix_loop(
    program: &ParserPrediction,
) -> Result<(&Decision, Vec<usize>), String> {
    let decision = program
        .decisions
        .get("rule/Target/s0/repeat")
        .ok_or("Missing real suffix loop")?;
    let [body, end] = decision.entries.as_slice() else {
        return Err("Invalid suffix loop".into());
    };
    let points = program
        .states
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            matches!(s.edges.as_slice(),[Edge::Epsilon(a),Edge::Epsilon(b)] if a==body && b==end)
                .then_some(i)
        })
        .collect();
    Ok((decision, points))
}

#[test]
fn input_checked_call_suffix_star_plus_actual_scope_exact_short_and_immutable() -> Result<(), String>
{
    for card in ['*', '+'] {
        let (grammar, program) = input_checked_call_suffix_fixture(card)?;
        let before = serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?;
        let (decision, points) = input_checked_call_suffix_loop(&program)?;
        assert!(!program.input_checked_literal_repeat(
            "rule/Target/s0/repeat",
            decision,
            &points,
            &mut 10000
        )?);
        let mut remaining = 10000;
        assert!(program.input_checked_call_suffix_repeat(
            "rule/Target/s0/repeat",
            decision,
            &points,
            &mut remaining
        )?);
        let used = 10000 - remaining;
        for allowance in [0, used - 1, used] {
            let mut work = allowance;
            let actual = program.input_checked_call_suffix_repeat(
                "rule/Target/s0/repeat",
                decision,
                &points,
                &mut work,
            );
            if allowance == used {
                assert!(actual?);
            } else {
                assert!(
                    actual
                        .err()
                        .ok_or("Short local proof passed")?
                        .contains("budget")
                );
            }
            assert_eq!(work, 0);
        }
        let record = input_checked_bound(&grammar, &program)?;
        let mut work = 10000;
        verify_input_checked_record(&program, PATH, &record, &mut work)?;
        let whole = 10000 - work;
        for allowance in [0, whole - 1, whole] {
            let mut work = allowance;
            let actual = verify_input_checked_record(&program, PATH, &record, &mut work);
            if allowance == whole {
                actual?;
            } else {
                assert!(actual.is_err());
            }
            assert_eq!(work, 0);
        }
        assert_eq!(
            serde_json::to_value((&grammar, &program)).map_err(|e| e.to_string())?,
            before
        );
        println!(
            "INPUT_CHECKED_CALL_SUFFIX_POOL {}",
            serde_json::json!({"cardinality":card,"local_work":used,"whole_scope_work":whole})
        );
    }
    Ok(())
}

#[test]
fn input_checked_call_suffix_rejects_optional_missing_empty_and_foreign_edges() -> Result<(), String>
{
    let (_, program) = input_checked_call_suffix_fixture('*')?;
    let (decision, points) = input_checked_call_suffix_loop(&program)?;
    let body = decision.entries[0];
    let end = decision.entries[1];
    let [Edge::Call { follow, .. }] = program.states[body].edges.as_slice() else {
        return Err("Missing Call".into());
    };
    let suffix = *follow;
    let [Edge::Token(_, done)] = program.states[suffix].edges.as_slice() else {
        return Err("Missing suffix".into());
    };
    let done = *done;
    for mode in 0..10 {
        let mut changed = program.clone();
        match mode {
            0 => {
                changed.states[suffix].edges =
                    vec![Edge::Token(PredictionSymbol::Keyword(String::new()), done)]
            }
            1 => {
                changed.states[suffix].edges =
                    vec![Edge::Token(PredictionSymbol::Terminal("SEP".into()), done)]
            }
            2 => changed.states[suffix].edges = vec![Edge::Epsilon(done)],
            3 => changed.states[suffix].edges.push(Edge::Epsilon(done)),
            4 => changed.states[suffix].accepting = true,
            5 => changed.states[suffix].rule_stop = true,
            6 => changed.states[done].edges = vec![Edge::Epsilon(end)],
            7 => changed.states[body].edges = vec![Edge::Action(suffix)],
            8 => {
                changed.states[body].edges = vec![Edge::First {
                    path: "rule/Name/a0".into(),
                    target: suffix,
                }]
            }
            _ => {
                if let [Edge::Call { follow, .. }] = changed.states[body].edges.as_mut_slice() {
                    *follow = end;
                }
            }
        }
        assert!(
            !changed.input_checked_call_suffix_repeat(
                "rule/Target/s0/repeat",
                decision,
                &points,
                &mut 10000
            )?,
            "mode={mode}"
        );
    }
    let (_, optional) = input_checked_call_suffix_fixture('?')?;
    let (d, p) = input_checked_call_suffix_loop(&optional)?;
    assert!(!optional.input_checked_call_suffix_repeat(
        "rule/Target/s0/repeat",
        d,
        &p,
        &mut 10000
    )?);
    for site in 0..3 {
        let mut changed = program.clone();
        let dangling = changed.states.len();
        match site {
            0 => {
                if let [Edge::Call { target, .. }] = changed.states[body].edges.as_mut_slice() {
                    *target = dangling;
                }
            }
            1 => {
                if let [Edge::Call { follow, .. }] = changed.states[body].edges.as_mut_slice() {
                    *follow = dangling;
                }
            }
            _ => {
                changed.states[suffix].edges = vec![Edge::Token(
                    PredictionSymbol::Keyword("::".into()),
                    dangling,
                )]
            }
        }
        assert!(
            changed
                .input_checked_call_suffix_repeat(
                    "rule/Target/s0/repeat",
                    decision,
                    &points,
                    &mut 10000
                )
                .err()
                .ok_or("Dangling graph admitted")?
                .contains("Dangling")
        );
    }
    Ok(())
}

#[test]
fn input_checked_call_suffix_nullable_and_terminal_callees_keep_final_guard_contracts()
-> Result<(), String> {
    let (grammar, program) = input_checked_call_suffix_fixture('+')?;
    let (decision, points) = input_checked_call_suffix_loop(&program)?;
    let (start, stop) = program.rules["Name"];
    for terminal in [false, true] {
        let mut changed = program.clone();
        changed.states[start].edges = if terminal {
            vec![Edge::Token(PredictionSymbol::Terminal("ID".into()), stop)]
        } else {
            vec![Edge::Epsilon(stop)]
        };
        assert!(changed.input_checked_call_suffix_repeat(
            "rule/Target/s0/repeat",
            decision,
            &points,
            &mut 10000
        )?);
    }
    let record = input_checked_bound(&grammar, &program)?;
    let mut recursive = program.clone();
    recursive
        .recursive_decisions
        .insert("rule/Target/s0/repeat".into());
    assert!(
        verify_input_checked_record(&recursive, PATH, &record, &mut 10000)
            .err()
            .ok_or("Recursive commitment admitted")?
            .contains("Unqualified completion commitment")
    );
    let mut hidden = program.clone();
    let guard = hidden
        .states
        .iter()
        .position(|s| matches!(s.edges.as_slice(),[Edge::First{path,..}] if path=="rule/Name/a0"))
        .ok_or("Missing real guard")?;
    let shifted = hidden.state()?;
    hidden.states[shifted].edges = hidden.states[guard].edges.clone();
    hidden.states[guard].edges = vec![Edge::Token(PredictionSymbol::Keyword("p".into()), shifted)];
    assert!(
        verify_input_checked_record(&hidden, PATH, &record, &mut 10000)
            .err()
            .ok_or("Hidden guard admitted")?
            .contains("guard visibility")
    );
    let mut foreign = program.clone();
    let extra = foreign.state()?;
    foreign.states[extra].edges = vec![
        Edge::Epsilon(decision.entries[0]),
        Edge::Epsilon(decision.entries[1]),
    ];
    let mut all = points;
    all.push(extra);
    assert!(!foreign.input_checked_call_suffix_repeat(
        "rule/Target/s0/repeat",
        decision,
        &all,
        &mut 10000
    )?);
    Ok(())
}

// Every occurrence and recovery entry is built by the existing expression
// builder from distinct, exact source ranges. No fabricated state IDs are used.
fn native_seal_fixture(
    count: usize,
    tokens: usize,
    callback: bool,
) -> Result<(Grammar, ParserPrediction), String> {
    let mut text = String::from("Root: ");
    let mut items = Vec::new();
    let mut guards = BTreeMap::new();
    for index in 0..count {
        if index > 0 {
            text.push_str(" 'q' ");
            items.push(serde_json::json!({"kind":{"Literal":"q"},"cardinality":"1"}));
        }
        let begin = text.len();
        text.push_str("(-> Target)*");
        let end = text.len();
        let path = format!("rule/Root/s{}", items.len());
        let guard_path = format!("{path}/s0");
        guards.insert(
            guard_path.clone(),
            [PredictionSymbol::Keyword("x".into())].into(),
        );
        items.push(serde_json::json!({"kind":{"Sequence":[{
            "kind":{"Call":"Target"},"cardinality":"1","first_set_predicated":true,
            "prediction_path":guard_path,"original_range":{"start":begin+1,"end":end-2}
        }]},"cardinality":"*","prediction_path":path,"original_range":{"start":begin,"end":end}}));
    }
    text.push(';');
    let root_end = text.len();
    text.push_str(" Target: ");
    let target_begin = root_end + 1;
    let mut target = Vec::new();
    for index in 0..tokens {
        if index > 0 {
            text.push(' ');
        }
        text.push_str("'x'");
        target.push(serde_json::json!({"kind":{"Literal":"x"},"cardinality":"1"}));
    }
    if callback {
        text.push_str(" Root");
        target.push(serde_json::json!({"kind":{"Call":"Root"},"cardinality":"1"}));
    }
    text.push(';');
    let source = SourceId::for_text("memory:seal-reuse-kernel.xtext", &text);
    let grammar:Grammar=serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"SealReuse","entry":"Root",
        "rules":{
            "Root":{"class":"Root","body":{"kind":{"Sequence":items},"cardinality":"1",
                "prediction_path":"rule/Root","original_range":{"start":0,"end":root_end}}},
            "Target":{"class":"Root","fragment":true,"body":{"kind":{"Sequence":target},"cardinality":"1"}}
        },"declaration_sources":{
            "Root":{"source":source,"start":0,"end":root_end},
            "Target":{"source":source,"start":target_begin,"end":text.len()}}
    })).map_err(|e|e.to_string())?;
    let mut program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: true,
        states: vec![],
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: guards,
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    for name in ["Root", "Target"] {
        let start = program.state()?;
        let stop = program.state()?;
        program.states[stop].rule_stop = true;
        program.rules.insert(name.into(), (start, stop));
    }
    program.decisions.insert(
        "@document-recovery".into(),
        Decision {
            entries: vec![],
            exit: None,
        },
    );
    for (name, rule) in &grammar.rules {
        let (start, stop) = program.rules[name];
        program.expression(
            &rule.body,
            start,
            stop,
            &format!("rule/{name}"),
            (0, None, None),
            None,
        )?;
    }
    Ok((grammar, program))
}
fn native_seal_records(
    grammar: &Grammar,
    program: &ParserPrediction,
) -> Result<BTreeMap<String, Binding>, String> {
    let mut records = BTreeMap::new();
    for path in program
        .decisions
        .keys()
        .filter(|path| path.ends_with("/repeat"))
    {
        if let Some(record) = binding(
            grammar,
            program,
            path,
            &mut 10000,
            &mut 0,
            RuntimeGuardedPolicy::InputCheckedCompletionScopes,
            None,
        )? {
            records.insert(path.clone(), record);
        }
    }
    Ok(records)
}
fn native_scope_value(scope: &super::super::super::CompletionScope<'_>) -> serde_json::Value {
    serde_json::json!({
        "decisions":scope.decisions.iter().map(|(point,bound)|(*point,bound.path,bound.decision.entries.clone(),bound.decision.exit,bound.recursive_first)).collect::<Vec<_>>(),
        "guards":scope.guards.iter().map(|(point,guard)|(*point,format!("{guard:?}"))).collect::<Vec<_>>()
    })
}

// Distinct source owners match the FSP/PFSP sharing shape. With callback=true,
// the representative OwnerA proof reaches ONLY the other owner OwnerB's entry.
// All graph nodes still come from the SAME canonical expression builder.
fn native_distinct_owner_fixture(callback: bool) -> Result<(Grammar, ParserPrediction), String> {
    let text = format!(
        "ARoot: OwnerA 'q' OwnerB 'z'; OwnerA: (-> Target)*; OwnerB: (-> Target)*; Target: 'x'{};",
        if callback { " OwnerB" } else { "" }
    );
    let source = SourceId::for_text("memory:native-distinct-owner-kernel.xtext", &text);
    let declaration = |name: &str| -> Result<(usize, usize), String> {
        let start = text
            .find(&format!("{name}:"))
            .ok_or("Missing exact source declaration")?;
        let end = start
            + text[start..]
                .find(';')
                .ok_or("Missing source declaration end")?
            + 1;
        Ok((start, end))
    };
    let (a_start, a_end) = declaration("OwnerA")?;
    let (b_start, b_end) = declaration("OwnerB")?;
    let (root_start, root_end) = declaration("ARoot")?;
    let (target_start, target_end) = declaration("Target")?;
    let repeat = |owner: &str, start: usize, end: usize| -> Result<serde_json::Value, String> {
        let begin = start
            + text[start..end]
                .find("(-> Target)*")
                .ok_or("Missing guarded repeat source")?;
        let finish = begin + "(-> Target)*".len();
        Ok(serde_json::json!({"kind":{"Sequence":[{
            "kind":{"Call":"Target"},"cardinality":"1","first_set_predicated":true,
            "prediction_path":format!("rule/{owner}/s0"),"original_range":{"start":begin+1,"end":finish-2}
        }]},"cardinality":"*","prediction_path":format!("rule/{owner}"),
            "original_range":{"start":begin,"end":finish}}))
    };
    let mut target = vec![serde_json::json!({"kind":{"Literal":"x"},"cardinality":"1"})];
    if callback {
        target.push(serde_json::json!({"kind":{"Call":"OwnerB"},"cardinality":"1"}));
    }
    let grammar:Grammar=serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"DistinctOwners","entry":"ARoot",
        "rules":{
            "ARoot":{"class":"Root","body":{"kind":{"Sequence":[
                {"kind":{"Call":"OwnerA"},"cardinality":"1"},
                {"kind":{"Literal":"q"},"cardinality":"1"},
                {"kind":{"Call":"OwnerB"},"cardinality":"1"},
                {"kind":{"Literal":"z"},"cardinality":"1"}]},"cardinality":"1"}},
            "OwnerA":{"class":"Root","fragment":true,"body":repeat("OwnerA",a_start,a_end)?},
            "OwnerB":{"class":"Root","fragment":true,"body":repeat("OwnerB",b_start,b_end)?},
            "Target":{"class":"Root","fragment":true,"body":{"kind":{"Sequence":target},"cardinality":"1"}}
        },"declaration_sources":{
            "ARoot":{"source":source,"start":root_start,"end":root_end},
            "OwnerA":{"source":source,"start":a_start,"end":a_end},
            "OwnerB":{"source":source,"start":b_start,"end":b_end},
            "Target":{"source":source,"start":target_start,"end":target_end}}
    })).map_err(|e|e.to_string())?;
    let mut program = ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: true,
        states: vec![],
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: [
            (
                "rule/OwnerA/s0".into(),
                [PredictionSymbol::Keyword("x".into())].into(),
            ),
            (
                "rule/OwnerB/s0".into(),
                [PredictionSymbol::Keyword("x".into())].into(),
            ),
        ]
        .into(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    };
    for name in grammar.rules.keys() {
        let start = program.state()?;
        let stop = program.state()?;
        program.states[stop].rule_stop = true;
        program.rules.insert(name.clone(), (start, stop));
    }
    program.decisions.insert(
        "@document-recovery".into(),
        Decision {
            entries: vec![],
            exit: None,
        },
    );
    for (name, rule) in &grammar.rules {
        let (start, stop) = program.rules[name];
        program.expression(
            &rule.body,
            start,
            stop,
            &format!("rule/{name}"),
            (0, None, None),
            None,
        )?;
    }
    Ok((grammar, program))
}

#[test]
fn input_checked_native_seal_reuse_keeps_original_scope_and_saves_actual_two_group_work()
-> Result<(), String> {
    for tokens in [1, 96] {
        let (grammar, program) = native_seal_fixture(2, tokens, false)?;
        let records = native_seal_records(&grammar, &program)?;
        assert_eq!(records.len(), 2);
        let mut generic_remaining = 10000;
        let mut expected_scope = None;
        for (path, record) in &records {
            let mut old = 10000;
            let scope = program.verify_completion_scope_with_verification(
                record.graph[2],
                record.graph[3],
                &mut old,
                super::super::super::CompletionVerification::InputCheckedCompletion,
            )?;
            let mut with_reach = 10000;
            let (same, reached) = program.verify_completion_scope_with_reach(
                record.graph[2],
                record.graph[3],
                &mut with_reach,
                super::super::super::CompletionVerification::InputCheckedCompletion,
                None,
            )?;
            assert_eq!(native_scope_value(&scope), native_scope_value(&same));
            assert_eq!(old, with_reach); // original proof walk and all fees unchanged
            assert!(!reached.contains(&program.rules["Root"].0));
            let actual = native_scope_value(&scope);
            if let Some(expected) = &expected_scope {
                assert_eq!(&actual, expected);
            } else {
                expected_scope = Some(actual);
            }
            verify_input_checked_record(&program, path, record, &mut generic_remaining)?;
        }
        let mut remaining = 10000;
        let mut logical = 0;
        verify_native_input_checked_records(&program, &records, &mut logical, &mut remaining)?;
        if tokens == 96 {
            assert!(
                remaining > generic_remaining,
                "{} vs {}",
                10000 - remaining,
                10000 - generic_remaining
            );
        }
        assert_eq!(logical, 24 + 16 * records.len());
        println!(
            "NATIVE_SEAL_REUSE_WHOLE_WORK {}",
            serde_json::json!({"occurrences":2,"tokens":tokens,
            "old_work":10000-generic_remaining,"native_work":10000-remaining,"new_logical_bytes":logical,
            "original_scope_and_fees_equal":true,"physical_peak_bytes":null})
        );
    }

    let (grammar, program) = native_distinct_owner_fixture(false)?;
    let records = native_seal_records(&grammar, &program)?;
    assert_eq!(
        records
            .values()
            .map(|r| r.owner.as_str())
            .collect::<BTreeSet<_>>(),
        ["OwnerA", "OwnerB"].into()
    );
    let mut old = 10000;
    let mut scope_value = None;
    for (path, record) in &records {
        verify_input_checked_record(&program, path, record, &mut old)?;
        let (scope, reached) = program.verify_completion_scope_with_reach(
            record.graph[2],
            record.graph[3],
            &mut 10000,
            super::super::super::CompletionVerification::InputCheckedCompletion,
            None,
        )?;
        for owner in ["OwnerA", "OwnerB"] {
            assert!(!reached.contains(&program.rules[owner].0));
        }
        let value = native_scope_value(&scope);
        if let Some(expected) = &scope_value {
            assert_eq!(&value, expected);
        } else {
            scope_value = Some(value);
        }
    }
    let mut new = 10000;
    let mut logical = 0;
    verify_native_input_checked_records(&program, &records, &mut logical, &mut new)?;
    assert_eq!(logical, 24 + 16 * records.len());
    println!(
        "NATIVE_SEAL_DISTINCT_OWNER_WHOLE_WORK {}",
        serde_json::json!({
        "owners":["OwnerA","OwnerB"],"old_work":10000-old,"native_work":10000-new,
        "new_logical_bytes":logical,"scope_bindings_equal":true})
    );

    // A unique callee does not create a memo or change its original proof fees.
    let (grammar, program) = native_seal_fixture(1, 4, false)?;
    let records = native_seal_records(&grammar, &program)?;
    let (path, record) = records.iter().next().ok_or("Missing record")?;
    let mut old = 10000;
    verify_input_checked_record(&program, path, record, &mut old)?;
    let mut new = 10000;
    let mut logical = 0;
    verify_native_input_checked_records(&program, &records, &mut logical, &mut new)?;
    assert_eq!(old, new);
    assert_eq!(logical, 0);
    Ok(())
}

#[test]
fn input_checked_native_seal_reuse_owner_callback_refuses_pair_without_retry() -> Result<(), String>
{
    let (grammar, program) = native_distinct_owner_fixture(true)?;
    let records = native_seal_records(&grammar, &program)?;
    assert_eq!(records.len(), 2);
    assert_eq!(
        records
            .values()
            .map(|r| r.owner.as_str())
            .collect::<BTreeSet<_>>(),
        ["OwnerA", "OwnerB"].into()
    );
    let first = records
        .values()
        .next()
        .ok_or("Missing first source owner")?;
    assert_eq!(first.owner, "OwnerA");
    let (_, first_reached) = program.verify_completion_scope_with_reach(
        first.graph[2],
        first.graph[3],
        &mut 10000,
        super::super::super::CompletionVerification::InputCheckedCompletion,
        None,
    )?;
    assert!(!first_reached.contains(&program.rules["OwnerA"].0));
    assert!(first_reached.contains(&program.rules["OwnerB"].0));
    let mut old = 10000;
    for (path, record) in &records {
        verify_input_checked_record(&program, path, record, &mut old)?;
        let (_, reached) = program.verify_completion_scope_with_reach(
            record.graph[2],
            record.graph[3],
            &mut 10000,
            super::super::super::CompletionVerification::InputCheckedCompletion,
            None,
        )?;
        assert!(reached.contains(&program.rules["OwnerB"].0));
    }
    let mut logical = 0;
    let mut work = 10000;
    verify_native_input_checked_records(&program, &records, &mut logical, &mut work)?;
    assert_eq!(logical, 24); // no pair capacity/publication: both original proofs run
    assert!(work < old);
    Ok(())
}

#[test]
fn input_checked_native_seal_reuse_generic_forgery_and_invalid_graph_stay_hard()
-> Result<(), String> {
    let (grammar, mut program) = native_seal_fixture(2, 3, false)?;
    let mut records = native_seal_records(&grammar, &program)?;
    let record = records.values().next().ok_or("Missing record")?;
    let target = record.graph[2];
    let callee_start = record.graph[5];
    // Only the GENERIC entry receives this forged graph; endpoint equality and
    // a non-deserializable companion do not grant fresh-builder provenance.
    program.decisions.insert(
        "rule/forged/choice".into(),
        Decision {
            entries: vec![target, callee_start],
            exit: None,
        },
    );
    let mut admission = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    admission.records = records;
    let error = admission
        .seal(&grammar, &mut program, &mut 10000)
        .err()
        .ok_or("Generic forged graph sealed")?;
    assert!(
        error.contains("Completion cannot bind prediction decision"),
        "{error}"
    );
    assert!(admission.failed);
    assert!(program.runtime_requirement.is_none());
    let (grammar, program) = native_seal_fixture(2, 3, false)?;
    records = native_seal_records(&grammar, &program)?;
    let record = records.values().next().ok_or("Missing record")?;
    let mut wrong = program.clone();
    wrong.states[record.graph[2]].edges = vec![Edge::Call {
        target: record.graph[5],
        follow: record.graph[4],
    }];
    assert!(verify_native_input_checked_records(&wrong, &records, &mut 0, &mut 10000).is_err());
    let path = records.keys().next().ok_or("Missing path")?.clone();
    records.get_mut(&path).ok_or("Missing record")?.proof_class =
        RuntimeGuardedPolicy::StrictCompletionScopes;
    assert!(verify_native_input_checked_records(&program, &records, &mut 0, &mut 10000).is_err());
    Ok(())
}

#[test]
fn input_checked_native_seal_reuse_exact_short_storage_and_sticky_denials() -> Result<(), String> {
    let (grammar, program) = native_seal_fixture(2, 4, false)?;
    let records = native_seal_records(&grammar, &program)?;
    let mut remaining = 10000;
    verify_native_input_checked_records(&program, &records, &mut 0, &mut remaining)?;
    let used = 10000 - remaining;
    for allowance in [0, used - 1, used] {
        let mut work = allowance;
        let result = verify_native_input_checked_records(&program, &records, &mut 0, &mut work);
        if allowance == used {
            result?;
        } else {
            assert!(result.is_err());
        }
        assert_eq!(work, 0);
    }
    for bytes in [LIMIT - 23, LIMIT - 24] {
        let mut logical = bytes;
        assert!(
            verify_native_input_checked_records(&program, &records, &mut logical, &mut 10000)
                .is_err()
        );
    }
    let mut admission = RuntimeGuardedAdmission::new_with_policy(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    admission.records = records;
    let mut unchanged = program.clone();
    assert!(
        admission
            .seal_native_generated(&grammar, &mut unchanged, &mut 0)
            .is_err()
    );
    assert!(admission.failed);
    assert!(unchanged.runtime_requirement.is_none());
    assert!(
        admission
            .seal_native_generated(&grammar, &mut unchanged, &mut 10000)
            .is_err()
    );
    Ok(())
}
