//! PRIVATE proposed strict-certified nonlinear single-Call Full-star controls.
//! Generated-Xtext32/ANTLR11 and original551 remain separate staged exits.
use super::*;

fn strict_compile(name: &str, source: &str) -> Result<compiler::RuntimeGuardedCompilation, String> {
    ordinary::compile_with_runtime_guarded_policy(
        input(name, source),
        Profile::NativeV80,
        &[],
        None,
        Default::default(),
        Default::default(),
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
    )
    .map_err(|errors| format!("{errors:?}"))
}
fn strict_parts(
    compiled: &compiler::RuntimeGuardedCompilation,
    count: usize,
) -> Result<&grammar::Grammar, String> {
    let (package, admission) = compiled.parts();
    let grammar = package.grammar.as_ref().ok_or("Missing actual grammar")?;
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("Missing actual native IR")?;
    assert_eq!(
        admission.policy(),
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes
    );
    assert_eq!(admission.decision_count(), count);
    assert_eq!(program.runtime_obligation_count(), count);
    assert_eq!(program.strict_completion_obligation_count(), count);
    assert!(program.requires_runtime_capability());
    let wire = serde_json::to_value(program).map_err(|e| e.to_string())?;
    assert!(wire["runtime_requirement"]["StrictCompletionScopes"].is_object());
    Ok(grammar)
}
fn assert_model<'a>(
    package: impl Into<crate::package::TextPackageView<'a>>,
    actual: &Evaluation,
    text: &str,
    children: &[(Value, usize, usize)],
) -> Result<(), String> {
    assert!(actual.diagnostics.is_empty());
    let mut expected = vec![
        json!({"class":format!("{NS}#//Root"),"values":{"children":(1..=children.len()).collect::<Vec<_>>()},"span_bytes":[0,text.len()]}),
    ];
    expected.extend(children.iter().map(|(values,start,end)|json!({"class":format!("{NS}#//Target"),"values":values,"span_bytes":[start,end]})));
    assert_eq!(graph_projection(package, actual)?.0, json!(expected));
    Ok(())
}
fn observed_run(
    compiled: &compiler::RuntimeGuardedCompilation,
    text: &str,
) -> Result<bounded_runtime::RuntimeGuardedEvaluation, String> {
    let (result, reads) = run(compiled, text, 100_000, 0);
    println!(
        "STRICT_SCOPE_WHOLE_RESULT {}",
        json!({"text":text,"actual":result.as_ref().map(|r| json!({"evaluation":&r.evaluation,"consumed_runtime_decisions":r.consumed_runtime_decisions()})),"reads":reads,"full_sysml_award":false})
    );
    result
}
const BRANCH: &str = "tag=('x'|'y')";

#[test]
fn strict_scope_original_iteration_oracle_models_consume_new_explicit_kind() -> Result<(), String> {
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut observed = 0;
    for (name, source) in [("IterationX", X), ("IterationCallerTail", CALLER)] {
        let compiled = strict_compile(name, source)?;
        strict_parts(&compiled, 1)?;
        let (package, _) = compiled.parts();
        assert!(
            package
                .compilation
                .as_ref()
                .ok_or("Missing original input")?
                .bundle
                .sources
                .iter()
                .any(|s| s.kind == SourceKind::Xtext && s.text == source)
        );
        for row in rows.iter().filter(|row| row["case"]["grammar"] == name) {
            let text = row["case"]["text"].as_str().ok_or("Missing oracle input")?;
            let actual = observed_run(&compiled, text)?;
            println!(
                "STRICT_SCOPE_ORIGINAL_ORACLE {}",
                json!({"oracle":row,"actual":&actual.evaluation,"consumed":actual.consumed_runtime_decisions()})
            );
            assert_eq!(
                actual.evaluation.diagnostics.is_empty(),
                row["raw_complete_dto"]["accepted"]
                    .as_bool()
                    .ok_or("Missing oracle truth")?
            );
            assert!(actual.consumed_runtime_decisions() > 0);
            if actual.evaluation.diagnostics.is_empty() {
                assert_eq!(
                    graph_projection(package, &actual.evaluation)?.0,
                    expected_graph(row)
                );
            }
            observed += 1;
        }
    }
    assert_eq!(observed, 10);
    Ok(())
}

#[test]
fn strict_scope_productive_branching_full_callee_models_preserve_suffix() -> Result<(), String> {
    let source = X.replace("tag='x'", BRANCH);
    assert!(compiled("LinearStillRejectsBranch", &source).is_err());
    let compiled = strict_compile("StrictBranch", &source)?;
    strict_parts(&compiled, 1)?;
    for (text, children) in [
        ("x", vec![]),
        ("x x", vec![(json!({"tag":"x","tail":null}), 0, 1)]),
        ("y x", vec![(json!({"tag":"y","tail":null}), 0, 1)]),
        (
            "x y x",
            vec![
                (json!({"tag":"x","tail":null}), 0, 1),
                (json!({"tag":"y","tail":null}), 2, 3),
            ],
        ),
    ] {
        let actual = observed_run(&compiled, text)?;
        assert!(actual.consumed_runtime_decisions() > 0);
        assert_model(compiled.parts().0, &actual.evaluation, text, &children)?;
    }
    for text in ["", "y", "x y", "y y"] {
        let actual = observed_run(&compiled, text)?;
        assert!(!actual.evaluation.diagnostics.is_empty(), "{text}");
    }
    Ok(())
}

#[test]
fn strict_scope_nested_visible_full_priority_is_complete_and_syntax_only() -> Result<(), String> {
    let source = X.replace("tag='x'", "(=> (tag='x' tail='y') | tag='x' tail='z')");
    assert!(compiled("LinearStillRejectsGuardedChoice", &source).is_err());
    let compiled = strict_compile("StrictVisibleGuard", &source)?;
    strict_parts(&compiled, 1)?;
    for (text, children) in [
        ("x", vec![]),
        ("x y x", vec![(json!({"tag":"x","tail":"y"}), 0, 3)]),
        ("x z x", vec![(json!({"tag":"x","tail":"z"}), 0, 3)]),
        (
            "x y x z x",
            vec![
                (json!({"tag":"x","tail":"y"}), 0, 3),
                (json!({"tag":"x","tail":"z"}), 4, 7),
            ],
        ),
    ] {
        let actual = observed_run(&compiled, text)?;
        assert!(actual.consumed_runtime_decisions() > 0);
        assert_model(compiled.parts().0, &actual.evaluation, text, &children)?;
    }
    for text in ["x y", "x z", "x w x"] {
        let actual = observed_run(&compiled, text)?;
        assert!(!actual.evaluation.diagnostics.is_empty(), "{text}");
    }
    Ok(())
}

#[test]
fn strict_scope_unproved_ambiguity_and_nullable_operands_remain_explicit() -> Result<(), String> {
    let ambiguous = X.replace("tag='x'", "tag=('x'|'x')");
    let result = ordinary::compile_with_runtime_guarded_policy(
        input("StrictAmbiguous", &ambiguous),
        Profile::NativeV80,
        &[],
        None,
        Default::default(),
        Default::default(),
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
    );
    println!(
        "STRICT_SCOPE_UNSUPPORTED_AMBIGUITY {}",
        json!({"actual":result.as_ref().map(|compiled|compiled.parts().0)})
    );
    let errors = result
        .err()
        .ok_or("Unproved ordinary ambiguity acquired authority")?;
    assert!(errors.iter().any(
        |d| d.message.contains("Completion static verification failed")
            || d.message.contains("Unqualified completion commitment")
            || d.message.contains("Overlapping or nullable alternatives")
            || d.message.contains("Ambiguous")
    ));
    assert!(
        !errors.iter().any(|d| d.message.contains("budget exceeded")),
        "ambiguity must be rejected by its actual proof, not incidental exhaustion"
    );
    let nullable = X.replace("tag='x'", "tag=('x'|'y')?");
    let result = strict_compile("StrictNullableUnsupported", &nullable);
    println!(
        "STRICT_SCOPE_UNSUPPORTED_NULLABLE {}",
        json!({"actual":result.as_ref().map(|compiled|compiled.parts().0)})
    );
    let error = result
        .err()
        .ok_or("Nullable operand acquired nonnullable strict authority")?;
    assert!(
        error.contains("Nullable repetitions are unsupported")
            || error.contains("requires a nonnullable operand")
    );
    // Preserve the exact original nullable Root. Its independent oracle
    // rejects all three original inputs; success must not be invented. Add a
    // separate nonnullable Full caller without changing Root's language.
    let baseline_source =
        include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationNullable.xtext");
    let baseline = ordinary::compile(
        input("IterationNullableBaseline", baseline_source),
        Profile::NativeV80,
    );
    println!(
        "STRICT_SCOPE_NULLABLE_LITERAL_BASELINE_COMPILE {}",
        json!({"actual":&baseline})
    );
    let baseline = baseline.map_err(|errors| format!("{errors:?}"))?;
    let mixed = baseline_source.replace("terminal WS:", "Independent returns gate::Root: {gate::Root} (=> children+=Nonnullable)* 'z';\nNonnullable returns gate::Target: {gate::Target} tag='y';\nterminal WS:");
    assert_eq!(
        baseline_source
            .lines()
            .find(|line| line.starts_with("Root ")),
        mixed.lines().find(|line| line.starts_with("Root "))
    );
    let ordinary = ordinary::compile(input("MixedQualifiedNullable", &mixed), Profile::NativeV80);
    println!(
        "STRICT_SCOPE_MIXED_DEFAULT_COMPILE {}",
        json!({"actual":&ordinary})
    );
    let package = ordinary.map_err(|errors| format!("{errors:?}"))?;
    let grammar = package
        .grammar
        .as_ref()
        .ok_or("Missing mixed default grammar")?;
    assert!(
        grammar
            .qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut 10_000)?
            .is_some()
    );
    assert!(grammar.rules.contains_key("Independent") && grammar.rules.contains_key("Nonnullable"));
    assert!(
        !grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing mixed default IR")?
            .requires_runtime_capability()
    );
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
    ))
    .map_err(|error| error.to_string())?;
    let mut original_rows = 0;
    for row in rows
        .iter()
        .filter(|row| row["case"]["grammar"] == "IterationNullable")
    {
        let text = row["case"]["text"]
            .as_str()
            .ok_or("Missing original nullable input")?;
        let inputs = [Source {
            uri: "memory:mixed-nullable.input".into(),
            text: text.into(),
        }];
        let actual_baseline =
            evaluate_with_limits(&baseline, &inputs, EvaluationLimits::workbench());
        let actual_mixed = evaluate_with_limits(&package, &inputs, EvaluationLimits::workbench());
        println!(
            "STRICT_SCOPE_MIXED_LITERAL_BASELINE_RESULTS {}",
            json!({"oracle":row,"baseline_actual":&actual_baseline,"mixed_actual":&actual_mixed,"successful_mixed_execution_awarded":false})
        );
        assert_eq!(row["raw_complete_dto"]["accepted"], json!(false));
        assert!(!actual_baseline.diagnostics.is_empty());
        assert_eq!(
            serde_json::to_value(&actual_mixed).map_err(|e| e.to_string())?,
            serde_json::to_value(&actual_baseline).map_err(|e| e.to_string())?
        );
        original_rows += 1;
    }
    assert_eq!(original_rows, 3);
    for policy in [
        grammar::RuntimeGuardedPolicy::LinearFullRepeats,
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
    ] {
        let result = ordinary::compile_with_runtime_guarded_policy(
            input("MixedQualifiedNullable", &mixed),
            Profile::NativeV80,
            &[],
            None,
            Default::default(),
            Default::default(),
            policy,
        );
        println!(
            "STRICT_SCOPE_MIXED_OPTIN_UNSUPPORTED {}",
            json!({"policy":format!("{policy:?}"),"actual":result.as_ref().map(|compiled|compiled.parts().0)})
        );
        let errors = result
            .err()
            .ok_or("Mixed nullable program acquired a generic live execution capability")?;
        assert!(errors.iter().any(|d| {
            d.message
                .contains("do not support qualified nullable decisions")
        }));
        assert!(!errors.iter().any(|d| d.message.contains("budget exceeded")));
    }
    // A qualified keyword-optional is NOT a whole-Full single-Call candidate.
    // Its presence must still be found by the canonical inventory, even in an
    // otherwise linear program, before either opt-in record map is published.
    let optional_only = X.replace(
        "terminal WS:",
        "Unused returns gate::Target: {gate::Target} tag='q'?;\nterminal WS:",
    );
    let ordinary = ordinary::compile(
        input("SeparateQualifiedOptional", &optional_only),
        Profile::NativeV80,
    );
    println!(
        "STRICT_SCOPE_SEPARATE_OPTIONAL_DEFAULT {}",
        json!({"actual":&ordinary})
    );
    let package = ordinary.map_err(|errors| format!("{errors:?}"))?;
    assert!(
        package
            .grammar
            .as_ref()
            .ok_or("Missing optional default grammar")?
            .qualified_nullable_keyword_optional_with_work("rule/Unused/s1/repeat", &mut 10_000)?
            .is_some()
    );
    for policy in [
        grammar::RuntimeGuardedPolicy::LinearFullRepeats,
        grammar::RuntimeGuardedPolicy::StrictCompletionScopes,
    ] {
        let result = ordinary::compile_with_runtime_guarded_policy(
            input("SeparateQualifiedOptional", &optional_only),
            Profile::NativeV80,
            &[],
            None,
            Default::default(),
            Default::default(),
            policy,
        );
        println!(
            "STRICT_SCOPE_SEPARATE_OPTIONAL_UNSUPPORTED {}",
            json!({"policy":format!("{policy:?}"),"actual":result.as_ref().map(|compiled|compiled.parts().0)})
        );
        let errors = result
            .err()
            .ok_or("Nonselected qualified optional escaped whole-program exclusion")?;
        assert!(errors.iter().any(|d| {
            d.message
                .contains("do not support qualified nullable decisions at rule/Unused/s1/repeat")
        }));
    }
    Ok(())
}

#[test]
fn strict_scope_same_callee_distinct_real_caller_returns_do_not_alias() -> Result<(), String> {
    let source=CALLER.replace("Payload 'x'","Left 'a' Right 'b'")
        .replace("fragment Payload returns gate::Root: (=> children+=Target)*;","fragment Left returns gate::Root: (=> children+=Target)*;\nfragment Right returns gate::Root: (=> children+=Target)*;")
        .replace("tag='x'",BRANCH);
    let compiled = strict_compile("StrictTwoCallers", &source)?;
    strict_parts(&compiled, 2)?;
    let (_, admission) = compiled.parts();
    assert!(
        admission
            .decisions()
            .any(|p| p.starts_with("rule/Left/") || p == "rule/Left/repeat")
    );
    assert!(
        admission
            .decisions()
            .any(|p| p.starts_with("rule/Right/") || p == "rule/Right/repeat")
    );
    for (text, children) in [
        ("a b", vec![]),
        (
            "x a y b",
            vec![
                (json!({"tag":"x","tail":null}), 0, 1),
                (json!({"tag":"y","tail":null}), 4, 5),
            ],
        ),
        (
            "x y a y x b",
            vec![
                (json!({"tag":"x","tail":null}), 0, 1),
                (json!({"tag":"y","tail":null}), 2, 3),
                (json!({"tag":"y","tail":null}), 6, 7),
                (json!({"tag":"x","tail":null}), 8, 9),
            ],
        ),
    ] {
        let actual = observed_run(&compiled, text)?;
        assert!(actual.consumed_runtime_decisions() >= 2);
        assert_model(compiled.parts().0, &actual.evaluation, text, &children)?;
    }
    for text in ["x b y a", "x a y a", "x b"] {
        assert!(
            !observed_run(&compiled, text)?
                .evaluation
                .diagnostics
                .is_empty(),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn strict_scope_exact_short_preconsumed_budget_uses_same_whole_model() -> Result<(), String> {
    let source = X.replace("tag='x'", BRANCH);
    let compiled = strict_compile("StrictBudget", &source)?;
    strict_parts(&compiled, 1)?;
    let (result, used) = run(&compiled, "x y x", 100_000, 0);
    let reference = result?;
    let expected = serde_json::to_value(&reference.evaluation).map_err(|e| e.to_string())?;
    assert!(used > 1 && used < 100_000);
    let (exact, exact_used) = run(&compiled, "x y x", used, 0);
    assert_eq!(
        serde_json::to_value(&exact?.evaluation).map_err(|e| e.to_string())?,
        expected
    );
    assert_eq!(exact_used, used);
    let (short, short_used) = run(&compiled, "x y x", used - 1, 0);
    assert_failed(short);
    assert_eq!(short_used, used - 1);
    let (pre, pre_used) = run(&compiled, "x y x", used + 7, 7);
    assert_eq!(
        serde_json::to_value(&pre?.evaluation).map_err(|e| e.to_string())?,
        expected
    );
    assert_eq!(pre_used, used + 7);
    let (fresh, _) = run(&compiled, "x y x", 100_000, 0);
    assert_eq!(
        serde_json::to_value(&fresh?.evaluation).map_err(|e| e.to_string())?,
        expected
    );
    Ok(())
}

#[test]
fn strict_scope_policy_marker_affinity_and_real_replay_do_not_mint_new_priority()
-> Result<(), String> {
    let compiled = strict_compile("StrictAffinity", X)?;
    let grammar = strict_parts(&compiled, 1)?;
    let linear = super::compiled("StrictAffinity", X)?;
    assert_eq!(
        linear.parts().1.policy(),
        grammar::RuntimeGuardedPolicy::LinearFullRepeats
    );
    assert_eq!(
        linear
            .parts()
            .0
            .grammar
            .as_ref()
            .ok_or("Missing linear grammar")?
            .parser_prediction
            .as_ref()
            .ok_or("Missing linear IR")?
            .strict_completion_obligation_count(),
        0
    );
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:strict.input", "x");
    assert!(
        grammar::LiveCallerPrediction::new_runtime_guarded(
            grammar,
            &source,
            linear.parts().1,
            &mut Some(100_000)
        )
        .is_err()
    );
    let (_, admission) = compiled.parts();
    let mut wire = serde_json::to_value(grammar).map_err(|e| e.to_string())?;
    wire["profile"] = serde_json::to_value(Profile::NativeV67).map_err(|e| e.to_string())?;
    let loaded: grammar::Grammar = serde_json::from_value(wire).map_err(|e| e.to_string())?;
    assert!(grammar::LiveCallerPrediction::new(&loaded, &source, &mut Some(100_000)).is_err());
    let mut work = Some(100_000);
    let mut live =
        grammar::LiveCallerPrediction::new_runtime_guarded(grammar, &source, admission, &mut work)?;
    let mut input = TraceInput::new("x");
    let result = live.predict_shared_returns(
        &source,
        &[],
        repeat_expr(grammar),
        "repeat",
        &mut input,
        &mut work,
    );
    println!(
        "STRICT_SCOPE_REAL_REPLAY {}",
        json!({"actual":&result,"position":input.position,"furthest":input.furthest,"rewinds":input.rewinds,"remaining":work})
    );
    assert_eq!(result?, None);
    assert!(input.rewinds.iter().any(|(a, b)| *a > 0 && *b == 0));
    let foreign = intersect_model::definitions::bundle::SourceId::for_text(
        "memory:strict-foreign.input",
        "x",
    );
    assert!(
        live.predict_shared_returns(
            &foreign,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut input,
            &mut work
        )
        .is_err()
    );
    assert!(live.is_poisoned());
    assert!(
        live.predict_shared_returns(
            &source,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut input,
            &mut work
        )
        .is_err()
    );
    let mut fresh_work = Some(100_000);
    let mut fresh = grammar::LiveCallerPrediction::new_runtime_guarded(
        grammar,
        &source,
        admission,
        &mut fresh_work,
    )?;
    assert_eq!(
        fresh.predict_shared_returns(
            &source,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut TraceInput::new("x x"),
            &mut fresh_work
        )?,
        Some(0)
    );
    Ok(())
}

#[test]
fn strict_owner_move_two_contexts_and_send_sync_preserve_complete_models() -> Result<(), String> {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<compiler::RuntimeGuardedCompilation>();
    send_sync::<grammar::RuntimeGuardedAdmission>();
    let compiled = strict_compile("StrictMovedOwner", X)?;
    strict_parts(&compiled, 1)?;
    let original = compiled.parts().0.schema as *const _ as usize;
    let reference = observed_run(&compiled, "x x")?;
    let expected =
        serde_json::to_value(&reference.evaluation).map_err(|error| error.to_string())?;
    let holder = Box::new(compiled); // outer move does not move its owned Package
    let moved = *holder;
    assert_eq!(moved.parts().0.schema as *const _ as usize, original);
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:owner.input", "x");
    let (package, admission) = moved.parts();
    let grammar = package.grammar.as_ref().ok_or("Missing moved grammar")?;
    let mut first_work = Some(100_000);
    let mut second_work = Some(100_000);
    let mut first = grammar::LiveCallerPrediction::new_runtime_guarded(
        grammar,
        &source,
        admission,
        &mut first_work,
    )?;
    let mut second = grammar::LiveCallerPrediction::new_runtime_guarded(
        grammar,
        &source,
        admission,
        &mut second_work,
    )?;
    let mut a = TraceInput::new("x");
    let mut b = TraceInput::new("x");
    let first_result = first.predict_shared_returns(
        &source,
        &[],
        repeat_expr(grammar),
        "repeat",
        &mut a,
        &mut first_work,
    );
    let second_result = second.predict_shared_returns(
        &source,
        &[],
        repeat_expr(grammar),
        "repeat",
        &mut b,
        &mut second_work,
    );
    println!(
        "STRICT_OWNER_TWO_CONTEXT_RESULTS {}",
        json!({"first":&first_result,"second":&second_result,"first_remaining":first_work,"second_remaining":second_work})
    );
    assert_eq!(first_result?, None);
    assert_eq!(second_result?, None);
    for _ in 0..2 {
        let actual = observed_run(&moved, "x x")?;
        println!(
            "STRICT_OWNER_MOVED_WHOLE_RESULT {}",
            json!({"actual":&actual.evaluation,"consumed":actual.consumed_runtime_decisions()})
        );
        assert_eq!(
            serde_json::to_value(&actual.evaluation).map_err(|error| error.to_string())?,
            expected
        );
        assert!(actual.consumed_runtime_decisions() > 0);
    }
    Ok(())
}

#[test]
fn strict_owner_equal_cloned_loaded_and_other_companions_reject() -> Result<(), String> {
    let first = strict_compile("StrictExactOwner", X)?;
    let second = strict_compile("StrictExactOwner", X)?;
    let (package, admission) = first.parts();
    let grammar = package
        .grammar
        .as_ref()
        .ok_or("Missing actual owner grammar")?;
    let cloned = (*grammar).clone();
    let wire = serde_json::to_value(grammar).map_err(|error| error.to_string())?;
    let loaded: grammar::Grammar =
        serde_json::from_value(wire.clone()).map_err(|error| error.to_string())?;
    let mut altered = wire;
    altered["name"] = json!("foreign-mutated-owner");
    let mutated: grammar::Grammar =
        serde_json::from_value(altered).map_err(|error| error.to_string())?;
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:owner.input", "x");
    for foreign in [
        &cloned,
        &loaded,
        &mutated,
        second
            .parts()
            .0
            .grammar
            .as_ref()
            .ok_or("Missing independent owner")?,
    ] {
        let mut work = Some(100_000);
        let result = grammar::LiveCallerPrediction::new_runtime_guarded(
            foreign, &source, admission, &mut work,
        );
        println!(
            "STRICT_OWNER_FOREIGN_RESULT {}",
            json!({"error":result.as_ref().err(),"remaining":work})
        );
        assert!(result.is_err());
    }
    assert!(
        grammar::LiveCallerPrediction::new_runtime_guarded(
            grammar,
            &source,
            second.parts().1,
            &mut Some(100_000)
        )
        .is_err()
    );
    // Plain marker data is never a native owner capability.
    assert!(grammar::LiveCallerPrediction::new(&loaded, &source, &mut Some(100_000)).is_err());
    let baseline = observed_run(&first, "x x")?;
    assert!(baseline.evaluation.diagnostics.is_empty());
    assert!(baseline.consumed_runtime_decisions() > 0);
    let independent = observed_run(&second, "x x")?;
    assert_eq!(
        serde_json::to_value(&independent.evaluation).map_err(|error| error.to_string())?,
        serde_json::to_value(&baseline.evaluation).map_err(|error| error.to_string())?
    );
    Ok(())
}

#[path = "bounded_runtime_first_repeat_tests.rs"]
mod first_repeat;
