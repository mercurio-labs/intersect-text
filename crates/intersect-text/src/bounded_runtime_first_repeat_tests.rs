//! Exact imported First-repeat controls over the existing typed owner and Machine.
use super::*;

fn first_source(cardinality: char, target: &str) -> String {
    format!(
        r#"grammar intersect.iteration.oracle.FirstRepeat hidden(WS)
import "{NS}" as gate
import "http://www.eclipse.org/emf/2002/Ecore" as Ecore
Root returns gate::Root: {{gate::Root}} (-> Item){cardinality} 'z';
fragment Item returns gate::Root: {target};
terminal WS: (' ' | '\t' | '\r' | '\n')+;
"#
    )
}

#[test]
fn strict_first_repeats_preserve_real_star_plus_models_and_mandatory_entry() -> Result<(), String> {
    for cardinality in ['*', '+'] {
        let source = first_source(cardinality, "'x'");
        let compiled = strict_compile("FirstRepeat", &source)?;
        let grammar = strict_parts(&compiled, 1)?;
        assert_eq!(grammar.profile, Profile::NativeV80);
        assert!(
            compiled
                .parts()
                .0
                .compilation
                .as_ref()
                .ok_or("Missing exact compilation")?
                .bundle
                .sources
                .iter()
                .any(|s| s.kind == SourceKind::Xtext && s.text == source)
        );
        assert!(super::super::compiled("LinearDoesNotAdmitFirst", &source).is_err());
        for text in ["z", "x z", "x x z"] {
            let actual = observed_run(&compiled, text)?;
            let accepted = cardinality == '*' || text != "z";
            assert_eq!(
                actual.evaluation.diagnostics.is_empty(),
                accepted,
                "{cardinality}: {text}"
            );
            if accepted {
                assert!(actual.consumed_runtime_decisions() > 0);
                assert_eq!(actual.evaluation.model.elements.len(), 1);
                assert_eq!(actual.evaluation.model.elements[0].class, "Root");
                assert_eq!(
                    actual.evaluation.model.elements[0].type_identity,
                    compiled.parts().0.schema.identity("Root")
                );
            }
        }
        for text in ["", "x", "x y z"] {
            assert!(
                !observed_run(&compiled, text)?
                    .evaluation
                    .diagnostics
                    .is_empty()
            );
        }
    }
    Ok(())
}

#[test]
fn strict_first_direct_flags_and_nested_productive_calls_use_same_machine() -> Result<(), String> {
    for cardinality in ['*', '+'] {
        let grouped = first_source(cardinality, "'x' | 'y'");
        let direct = grouped.replace(
            &format!("(-> Item){cardinality}"),
            &format!("-> Item{cardinality}"),
        );
        for source in [&grouped, &direct] {
            let compiled = strict_compile("FirstNested", source)?;
            strict_parts(&compiled, 1)?;
            let actual = observed_run(&compiled, "x y x z")?;
            assert!(actual.evaluation.diagnostics.is_empty());
            assert!(actual.consumed_runtime_decisions() > 0);
            assert_eq!(actual.evaluation.model.elements.len(), 1);
        }
    }
    Ok(())
}

#[test]
fn strict_first_bare_nullable_and_nonuniform_source_profiles_stay_explicit() -> Result<(), String> {
    let source = first_source('*', "'x'");
    let bare = source.replace("(-> Item)*", "Item*");
    let result = strict_compile("FirstBareNotAdmitted", &bare);
    assert!(result.is_err());
    let nullable = first_source('*', "'x'?");
    let error = strict_compile("FirstNullableNotAdmitted", &nullable)
        .err()
        .ok_or("Nullable First callee admitted")?;
    // Native source admission rejects the nullable repeat before runtime binding.
    assert!(
        error.contains("Nullable repetitions are unsupported"),
        "{error}"
    );
    let hidden = source.replace(
        "fragment Item returns gate::Root:",
        "fragment Item returns gate::Root hidden():",
    );
    assert!(strict_compile("FirstForeignHidden", &hidden).is_err());
    let unknown = source.replace("(-> Item)*", "(-> Missing)*");
    assert!(strict_compile("FirstUnknownCall", &unknown).is_err());
    Ok(())
}

#[test]
fn strict_first_exact_short_preconsumed_and_default_rejection_keep_whole_result()
-> Result<(), String> {
    let source = first_source('+', "'x' | 'y'");
    let compiled = strict_compile("FirstFinite", &source)?;
    strict_parts(&compiled, 1)?;
    let (whole, used) = run(&compiled, "x y z", 100_000, 0);
    let actual = whole?;
    assert!(actual.evaluation.diagnostics.is_empty());
    let expected = serde_json::to_value(&actual.evaluation).map_err(|e| e.to_string())?;
    assert!(used > 1 && used < 100_000);
    for (allowance, spent) in [(used, 0), (used + 7, 7)] {
        let (result, reads) = run(&compiled, "x y z", allowance, spent);
        assert_eq!(
            serde_json::to_value(&result?.evaluation).map_err(|e| e.to_string())?,
            expected
        );
        assert_eq!(reads, allowance);
    }
    let (short, reads) = run(&compiled, "x y z", used - 1, 0);
    assert_failed(short);
    assert_eq!(reads, used - 1);
    assert_failed(run(&compiled, "x y z", 0, 0).0);
    let default = evaluate(
        &compiled.parts().0.to_owned(),
        &[Source {
            uri: "memory:untyped-first.input".into(),
            text: "x y z".into(),
        }],
    );
    assert!(!default.diagnostics.is_empty());
    Ok(())
}

#[test]
fn strict_first_exact_owner_and_failed_input_cannot_reuse_scope() -> Result<(), String> {
    let source_text = first_source('*', "'x'");
    let compiled = strict_compile("FirstAffinity", &source_text)?;
    let (package, admission) = compiled.parts();
    let grammar = package.grammar.ok_or("Missing actual grammar")?;
    let foreign = grammar.clone();
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:first.input", "x z");
    assert!(
        grammar::LiveCallerPrediction::new_runtime_guarded(
            &foreign,
            &source,
            admission,
            &mut Some(100_000)
        )
        .is_err()
    );
    let mut work = Some(100_000);
    let mut scope =
        grammar::LiveCallerPrediction::new_runtime_guarded(grammar, &source, admission, &mut work)?;
    struct Denied;
    impl grammar::PredictionInput for Denied {
        fn checkpoint(&self) -> Result<usize, String> {
            Ok(0)
        }
        fn rewind(&mut self, checkpoint: usize) -> Result<(), String> {
            if checkpoint == 0 {
                Ok(())
            } else {
                Err("Invalid denied-input checkpoint".into())
            }
        }
        fn read(
            &mut self,
            _: &mut Option<usize>,
        ) -> Result<Option<grammar::PredictionSymbol>, String> {
            Err("injected First input failure".into())
        }
    }
    let error = scope
        .predict_shared_returns(
            &source,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut Denied,
            &mut work,
        )
        .err()
        .ok_or("Denied First read succeeded")?;
    assert!(error.contains("injected First input failure"), "{error}");
    assert!(scope.is_poisoned());
    assert!(
        scope
            .predict_shared_returns(
                &source,
                &[],
                repeat_expr(grammar),
                "repeat",
                &mut TraceInput::new("x z"),
                &mut work
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn strict_first_feature_chain_nested_guard_keeps_whole_machine_and_original_budget()
-> Result<(), String> {
    let source = first_source('*', "Name ('.' Name)+").replace(
        "terminal WS:",
        "fragment Name returns gate::Root: -> 'x' | 'y';\nterminal WS:",
    );
    let compiled = strict_compile("FirstFeatureChain", &source)?;
    strict_parts(&compiled, 1)?;
    for text in ["z", "x . y z", "y . x . y z"] {
        let (whole, used) = run(&compiled, text, 100000, 0);
        println!(
            "STRICT_FIRST_FEATURE_CHAIN {}",
            serde_json::json!({"text":text,"whole_result":whole.as_ref().map(|actual| serde_json::json!({"evaluation":&actual.evaluation,"consumed_runtime_decisions":actual.consumed_runtime_decisions()})),"reads":used})
        );
        let actual = whole?;
        assert!(actual.evaluation.diagnostics.is_empty(), "{text}");
        assert_eq!(actual.evaluation.model.elements.len(), 1);
        let expected = serde_json::to_value(&actual.evaluation).map_err(|e| e.to_string())?;
        assert!(used > 1 && used < 100000);
        let (exact, reads) = run(&compiled, text, used, 0);
        assert_eq!(
            serde_json::to_value(&exact?.evaluation).map_err(|e| e.to_string())?,
            expected
        );
        assert_eq!(reads, used);
        let (short, reads) = run(&compiled, text, used - 1, 0);
        assert_failed(short);
        assert_eq!(reads, used - 1);
    }
    for text in ["x z", "x . z", "x . y"] {
        assert!(
            !observed_run(&compiled, text)?
                .evaluation
                .diagnostics
                .is_empty()
        );
    }
    Ok(())
}

#[test]
fn strict_full_feature_chain_nested_first_executes_same_machine_and_finite_pool()
-> Result<(), String> {
    let first = first_source('*', "Name ('.' Name)+").replace(
        "terminal WS:",
        "fragment Name returns gate::Root: -> 'x' | 'y';\nterminal WS:",
    );
    let full = first.replace("(-> Item)*", "(=> Item)*");
    assert_ne!(first, full);
    let expected_compiled = strict_compile("StrictFeatureChain", &first)?;
    let compiled = strict_compile("StrictFeatureChain", &full)?;
    strict_parts(&compiled, 1)?;
    for text in ["z", "x . y z", "y . x . y z"] {
        let expected = observed_run(&expected_compiled, text)?;
        let (whole, used) = run(&compiled, text, 100000, 0);
        println!(
            "STRICT_FULL_FEATURE_CHAIN {}",
            serde_json::json!({"text":text,"whole_result":whole.as_ref().map(|actual|serde_json::json!({"evaluation":&actual.evaluation,"consumed_runtime_decisions":actual.consumed_runtime_decisions()})),"reads":used})
        );
        let actual = whole?;
        assert!(actual.evaluation.diagnostics.is_empty(), "{text}");
        assert!(actual.consumed_runtime_decisions() > 0);
        let expected = serde_json::to_value(&expected.evaluation).map_err(|e| e.to_string())?;
        assert_eq!(
            serde_json::to_value(&actual.evaluation).map_err(|e| e.to_string())?,
            expected
        );
        assert!(used > 1 && used < 100000);
        let (exact, reads) = run(&compiled, text, used, 0);
        assert_eq!(
            serde_json::to_value(&exact?.evaluation).map_err(|e| e.to_string())?,
            expected
        );
        assert_eq!(reads, used);
        let (short, reads) = run(&compiled, text, used - 1, 0);
        assert_failed(short);
        assert_eq!(reads, used - 1);
    }
    for text in ["x z", "x . z", "x . y"] {
        assert!(
            !observed_run(&compiled, text)?
                .evaluation
                .diagnostics
                .is_empty()
        );
    }
    Ok(())
}

fn input_checked_compile(
    name: &str,
    source: &str,
) -> Result<compiler::RuntimeGuardedCompilation, String> {
    ordinary::compile_with_runtime_guarded_policy(
        input(name, source),
        Profile::NativeV80,
        &[],
        None,
        Default::default(),
        Default::default(),
        grammar::RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    )
    .map_err(|errors| format!("{errors:?}"))
}
fn input_checked_whole(
    result: &Result<bounded_runtime::RuntimeGuardedEvaluation, String>,
) -> serde_json::Value {
    match result {
        Ok(actual) => json!({"Ok":{"evaluation":&actual.evaluation,
            "consumed_runtime_decisions":actual.consumed_runtime_decisions()}}),
        Err(error) => json!({"Err":error}),
    }
}
fn input_checked_full_source(name: &str) -> String {
    first_source('*', "Name ('.' Name)+")
        .replace("(-> Item)*", "(=> Item)*")
        .replace(
            "terminal WS:",
            &format!("fragment Name returns gate::Root: {name};\nterminal WS:"),
        )
}

#[test]
fn input_checked_full_repeat_preserves_strict_whole_result_and_nonrecursive_priority()
-> Result<(), String> {
    for name in ["-> 'x' | 'y'", "-> 'x' 'q' | 'x' 'r'"] {
        let source = input_checked_full_source(name);
        let strict = strict_compile("InputCheckedWholeParity", &source)?;
        let checked = input_checked_compile("InputCheckedWholeParity", &source)?;
        let (package, admission) = checked.parts();
        assert_eq!(
            admission.policy(),
            grammar::RuntimeGuardedPolicy::InputCheckedCompletionScopes
        );
        assert_eq!(admission.decision_count(), 1);
        let program = package
            .grammar
            .as_ref()
            .ok_or("Missing grammar")?
            .parser_prediction
            .as_ref()
            .ok_or("Missing program")?;
        assert_eq!(program.input_checked_completion_obligation_count(), 1);
        assert_eq!(program.strict_completion_obligation_count(), 0);
        let wire = serde_json::to_value(program).map_err(|e| e.to_string())?;
        assert!(wire["runtime_requirement"]["InputCheckedCompletionScopes"].is_object());
        assert_eq!(
            serde_json::to_value(package.compilation).map_err(|e| e.to_string())?,
            serde_json::to_value(strict.parts().0.compilation).map_err(|e| e.to_string())?
        );
        let cases: &[(&str, bool)] = if name == "-> 'x' | 'y'" {
            &[
                ("z", true),
                ("x . y z", true),
                ("y . x . y z", true),
                ("x . y . z", false),
                ("x z", false),
                ("x . z", false),
                ("x . y", false),
            ]
        } else {
            // Nonrecursive First resolves among ACCEPTED alternatives. The
            // canonical worker commits early only for recursive_first goals;
            // an 'x' prefix alone cannot reject the valid unguarded 'x r'.
            &[
                ("z", true),
                ("x q . x q z", true),
                ("x r . x q z", true),
                ("x q . x r z", true),
                ("x q . x q . z", false),
                ("x q . z", false),
                ("x q . x s z", false),
            ]
        };
        for &(text, accepted) in cases {
            let (old, old_reads) = run(&strict, text, 100000, 0);
            let (new, new_reads) = run(&checked, text, 100000, 0);
            let old_whole = input_checked_whole(&old);
            let new_whole = input_checked_whole(&new);
            println!(
                "INPUT_CHECKED_FULL_WHOLE_PARITY {}",
                json!({"name":name,"text":text,
                "strict_whole":old_whole,"input_checked_whole":new_whole,
                "strict_reads":old_reads,"input_checked_reads":new_reads,
                "expected_acceptance":accepted,"normalized_fields":[],
                "nonrecursive_first_contract":"Accepted-alternative priority; no recursive early commitment claim"})
            );
            assert_eq!(new_whole, old_whole, "{name}: {text}");
            let actual = new?;
            assert_eq!(
                actual.evaluation.diagnostics.is_empty(),
                accepted,
                "{name}: {text}"
            );
            if accepted {
                assert_eq!(actual.evaluation.model.elements.len(), 1);
                assert!(actual.consumed_runtime_decisions() > 0);
                assert!(new_reads > 1 && new_reads < 100000);
                for (allowance, spent) in [(new_reads, 0), (new_reads + 7, 7)] {
                    let (exact, reads) = run(&checked, text, allowance, spent);
                    assert_eq!(input_checked_whole(&exact), new_whole);
                    assert_eq!(reads, allowance);
                }
                let (short, reads) = run(&checked, text, new_reads - 1, 0);
                assert_failed(short);
                assert_eq!(reads, new_reads - 1);
            }
        }
    }
    Ok(())
}

#[test]
fn input_checked_star_loop_and_structural_static_fallback_preserve_whole_machine()
-> Result<(), String> {
    let source = input_checked_full_source("-> 'x' | 'y'").replace("('.' Name)+", "('.' Name)*");
    let strict = strict_compile("InputCheckedStar", &source)?;
    let checked = input_checked_compile("InputCheckedStar", &source)?;
    for text in ["z", "x z", "x . y z", "x . y . z", "x . z"] {
        let (a, aw) = run(&strict, text, 100000, 0);
        let (b, bw) = run(&checked, text, 100000, 0);
        println!(
            "INPUT_CHECKED_STAR_WHOLE {}",
            json!({"text":text,"strict":input_checked_whole(&a),
            "input_checked":input_checked_whole(&b),"strict_reads":aw,"input_checked_reads":bw})
        );
        assert_eq!(input_checked_whole(&a), input_checked_whole(&b));
    }
    // No literal-prefix eligibility: the old canonical static proof must still
    // establish this terminal-prefixed loop (and optional/one-shot decisions).
    let fallback = source
        .replace("('.' Name)*", "(DOT Name)*")
        .replace("terminal WS:", "terminal DOT: '.';\nterminal WS:");
    let strict = strict_compile("InputCheckedStaticFallback", &fallback)?;
    let checked = input_checked_compile("InputCheckedStaticFallback", &fallback)?;
    for text in ["z", "x z", "x . y z", "x . z"] {
        assert_eq!(
            input_checked_whole(&run(&strict, text, 100000, 0).0),
            input_checked_whole(&run(&checked, text, 100000, 0).0)
        );
    }
    Ok(())
}

#[test]
fn input_checked_keeps_real_ambiguity_hidden_unknown_and_owner_rejections() -> Result<(), String> {
    let source = input_checked_full_source("-> 'x' | 'y'");
    // Unqualified ambiguous alternatives remain static, not deferred choices.
    let ambiguous = source.replace("-> 'x' | 'y'", "'x' | 'x'");
    assert!(strict_compile("InputCheckedAmbiguous", &ambiguous).is_err());
    assert!(input_checked_compile("InputCheckedAmbiguous", &ambiguous).is_err());
    let hidden = source.replace(
        "fragment Item returns gate::Root:",
        "fragment Item returns gate::Root hidden():",
    );
    assert!(input_checked_compile("InputCheckedHidden", &hidden).is_err());
    let unknown = source.replace("('.' Name)+", "('.' Missing)+");
    assert!(input_checked_compile("InputCheckedUnknown", &unknown).is_err());
    let bare = source.replace("(=> Item)*", "Item*");
    assert!(input_checked_compile("InputCheckedBare", &bare).is_err());
    let compiled = input_checked_compile("InputCheckedOwner", &source)?;
    let (package, admission) = compiled.parts();
    let grammar = package.grammar.ok_or("Missing grammar")?;
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:checked.input", "x . y z");
    let foreign = grammar.clone();
    assert!(
        grammar::LiveCallerPrediction::new_runtime_guarded(
            &foreign,
            &source,
            admission,
            &mut Some(100000)
        )
        .is_err()
    );
    let serialized = serde_json::to_value(grammar).map_err(|e| e.to_string())?;
    let forged: grammar::Grammar = serde_json::from_value(serialized).map_err(|e| e.to_string())?;
    assert!(
        grammar::LiveCallerPrediction::new_runtime_guarded(
            &forged,
            &source,
            admission,
            &mut Some(100000)
        )
        .is_err()
    );
    let untyped = evaluate(
        &package.to_owned(),
        &[Source {
            uri: "memory:checked.input".into(),
            text: "x . y z".into(),
        }],
    );
    assert!(!untyped.diagnostics.is_empty());
    Ok(())
}

// Both repeat bodies are structurally eligible for the new scope check, but
// they are not outer Full obligation records: the existing post-program deferred
// verifier still rejects their real ambiguity before runtime. No widened scope.
#[test]
fn input_checked_two_eligible_loop_candidates_preserve_compiler_ambiguity_rejection()
-> Result<(), String> {
    let source = input_checked_full_source("-> 'x' | 'y'")
        .replace("Name ('.' Name)+", "Name ('.' Name)* ('.' Name)*");
    let strict = strict_compile("InputCheckedAmbiguousLoops", &source);
    let checked = input_checked_compile("InputCheckedAmbiguousLoops", &source);
    println!(
        "INPUT_CHECKED_AMBIGUOUS_LOOP_COMPILE {}",
        json!({"source":source,"strict":strict.as_ref().map(|compiled|compiled.parts().0),
            "input_checked":checked.as_ref().map(|compiled|compiled.parts().0),
            "runtime_executed":false,"dynamic_eligible_loop_ambiguity_coverage":false,
            "scope":"Both exact compiler rejections; inner bare loops remain under the canonical post-program deferred verifier"})
    );
    let uri = "memory:whole-iteration/InputCheckedAmbiguousLoops.xtext";
    let start = source
        .find("fragment Item")
        .ok_or("Missing original Item declaration")?;
    let end = start
        + source[start..]
            .find(';')
            .ok_or("Missing Item declaration end")?
        + 1;
    let ambiguity = "Ambiguous parser prediction requires predicate resolution";
    let strict_message = format!(
        "Runtime guarded sealing failed: Strict runtime completion scope unsupported at rule/Root/s1/repeat: Completion static verification failed at rule/Item/s1/repeat: {ambiguity}"
    );
    let strict_error = strict
        .err()
        .ok_or("Strict admitted the actual ambiguous loop split")?;
    let checked_error = checked
        .err()
        .ok_or("InputChecked admitted the actual ambiguous loop split")?;
    assert_eq!(
        strict_error,
        format!(
            "[Diagnostic {{ source: {uri:?}, start: 0, end: 1, message: {strict_message:?} }}]"
        )
    );
    assert_eq!(
        checked_error,
        format!(
            "[Diagnostic {{ source: {uri:?}, start: {start}, end: {end}, message: {ambiguity:?} }}]"
        )
    );
    for error in [&strict_error, &checked_error] {
        assert!(error.contains(ambiguity), "{error}");
        assert!(!error.contains("budget exceeded"), "{error}");
    }
    Ok(())
}

fn input_checked_call_suffix_source(card: char) -> String {
    // Preserve the original KerML Name frontier: two terminal-valued choices.
    // The narrower test terminals recognize ordinary and quoted names; the
    // repeat itself is the exact Qualification (Name '::')* / + source shape.
    first_source('*', &format!("(Name '::'){card} Name 'q'"))
        .replace("(-> Item)*", "(=> Item)*")
        .replace(
            "terminal WS:",
            r#"Name returns Ecore::EString: ID | UNRESTRICTED_NAME;
terminal ID: ('a'..'z')+;
terminal UNRESTRICTED_NAME returns Ecore::EString: '\'' ('a'..'z')+ '\'';
terminal WS:"#,
        )
}

#[test]
fn input_checked_call_suffix_imported_qualification_terminal_names_whole_and_short()
-> Result<(), String> {
    for card in ['*', '+'] {
        let source = input_checked_call_suffix_source(card);
        let strict = strict_compile("InputCheckedQualification", &source)?;
        let checked = input_checked_compile("InputCheckedQualification", &source)?;
        for (text, accepted) in [
            ("z", true),
            ("x q z", card == '*'),
            ("x :: y q z", true),
            ("'x' :: y :: 'u' q z", true),
            ("x :: y q u :: v q z", true),
            ("x :: q z", false),
            ("x :: y q", false),
            ("x : y q z", false),
        ] {
            let (a, aw) = run(&strict, text, 100000, 0);
            let (b, bw) = run(&checked, text, 100000, 0);
            let old = input_checked_whole(&a);
            let new = input_checked_whole(&b);
            println!(
                "INPUT_CHECKED_CALL_SUFFIX_MACHINE {}",
                json!({"cardinality":card,"text":text,"source":source,"strict":old,"input_checked":new,"strict_work":aw,"input_checked_work":bw,"expected_acceptance":accepted})
            );
            assert_eq!(old, new, "{card}: {text}");
            assert_eq!(
                b?.evaluation.diagnostics.is_empty(),
                accepted,
                "{card}: {text}"
            );
            if accepted {
                assert!(bw > 1 && bw < 100000);
                assert_eq!(input_checked_whole(&run(&checked, text, bw, 0).0), new);
                assert_eq!(input_checked_whole(&run(&checked, text, bw + 7, 7).0), new);
                let (short, used) = run(&checked, text, bw - 1, 0);
                assert_failed(short);
                assert_eq!(used, bw - 1);
                let (zero, used) = run(&checked, text, 0, 0);
                assert_failed(zero);
                assert_eq!(used, 0);
            }
        }
    }
    Ok(())
}

#[test]
fn input_checked_call_suffix_nullable_callee_consumes_suffix_and_hidden_stays_explicit()
-> Result<(), String> {
    let source = first_source('*', "(Maybe '::')+ 'q'")
        .replace("(-> Item)*", "(=> Item)*")
        .replace(
            "terminal WS:",
            "fragment Maybe returns gate::Root: 'x'?;\nterminal WS:",
        );
    let strict = strict_compile("InputCheckedNullableSuffix", &source)?;
    let checked = input_checked_compile("InputCheckedNullableSuffix", &source)?;
    for (text, accepted) in [
        ("z", true),
        (":: q z", true),
        ("x :: :: q z", true),
        ("q z", false),
        ("x q z", false),
        (":: q", false),
    ] {
        let (a, _) = run(&strict, text, 100000, 0);
        let (b, work) = run(&checked, text, 100000, 0);
        let old = input_checked_whole(&a);
        let new = input_checked_whole(&b);
        println!(
            "INPUT_CHECKED_NULLABLE_CALL_SUFFIX {}",
            json!({"text":text,"strict":old,"input_checked":new,"charged_work":work,"expected_acceptance":accepted})
        );
        assert_eq!(old, new);
        assert_eq!(b?.evaluation.diagnostics.is_empty(), accepted);
    }
    // Cooperative input cancellation uses the same public PredictionInput
    // failure seam; a caught read failure must poison the existing owner.
    let (package, admission) = checked.parts();
    let grammar = package.grammar.ok_or("Missing actual grammar")?;
    let source_id = intersect_model::definitions::bundle::SourceId::for_text(
        "memory:suffix-cancel.input",
        ":: q z",
    );
    let mut work = Some(100000);
    let mut scope = grammar::LiveCallerPrediction::new_runtime_guarded(
        grammar, &source_id, admission, &mut work,
    )?;
    struct Cancelled {
        reads: usize,
    }
    impl grammar::PredictionInput for Cancelled {
        fn replay_position(&self) -> Option<usize> {
            Some(0)
        }
        fn checkpoint(&self) -> Result<usize, String> {
            Ok(0)
        }
        fn rewind(&mut self, point: usize) -> Result<(), String> {
            if point == 0 {
                Ok(())
            } else {
                Err("Invalid cancellation checkpoint".into())
            }
        }
        fn read(
            &mut self,
            _: &mut Option<usize>,
        ) -> Result<Option<grammar::PredictionSymbol>, String> {
            self.reads += 1;
            Err("injected suffix input cancellation".into())
        }
    }
    let mut cancelled = Cancelled { reads: 0 };
    let error = scope
        .predict_shared_returns(
            &source_id,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut cancelled,
            &mut work,
        )
        .err()
        .ok_or("Cancelled suffix input succeeded")?;
    assert!(
        error.contains("injected suffix input cancellation"),
        "{error}"
    );
    assert!(scope.is_poisoned());
    let reads = cancelled.reads;
    assert!(reads > 0);
    assert!(
        scope
            .predict_shared_returns(
                &source_id,
                &[],
                repeat_expr(grammar),
                "repeat",
                &mut cancelled,
                &mut work
            )
            .is_err()
    );
    assert_eq!(cancelled.reads, reads);
    let original = input_checked_call_suffix_source('+');
    let hidden = original.replace(
        "Name returns Ecore::EString:",
        "Name returns Ecore::EString hidden():",
    );
    assert_ne!(hidden, original);
    for (policy, compiled) in [
        (
            "Strict",
            strict_compile("InputCheckedSuffixNestedHidden", &hidden)?,
        ),
        (
            "InputChecked",
            input_checked_compile("InputCheckedSuffixNestedHidden", &hidden)?,
        ),
    ] {
        let (actual, work) = run(&compiled, "x :: y q z", 100000, 0);
        let whole = input_checked_whole(&actual);
        println!(
            "INPUT_CHECKED_CALL_SUFFIX_HIDDEN {}",
            json!({"policy":policy,"whole_result":whole,"charged_work":work})
        );
        assert!(
            whole
                .to_string()
                .contains("Bounded runtime prediction cannot switch hidden policy"),
            "{policy}: {whole}"
        );
        assert_failed(actual);
    }
    let guarded = original.replace(
        "fragment Item returns gate::Root:",
        "fragment Item returns gate::Root hidden():",
    );
    assert_ne!(guarded, original);
    assert!(strict_compile("InputCheckedSuffixGuardedHidden", &guarded).is_err());
    assert!(input_checked_compile("InputCheckedSuffixGuardedHidden", &guarded).is_err());
    let unknown = original.replace("(Name '::')+", "(Missing '::')+");
    assert_ne!(unknown, original);
    assert!(input_checked_compile("InputCheckedSuffixUnknown", &unknown).is_err());
    Ok(())
}

#[test]
fn input_checked_ordinary_choice_machine_whole_parity_valid_invalid_and_short() -> Result<(), String>
{
    let source = input_checked_full_source("'x' 'q' | 'x' 'r'");
    let strict = strict_compile("InputCheckedPlainChoice", &source)?;
    let checked = input_checked_compile("InputCheckedPlainChoice", &source)?;
    for (text, accepted) in [
        ("z", true),
        ("x q . x r z", true),
        ("x r . x q z", true),
        ("x r . x r . x q z", true),
        ("x q . z", false),
        ("x r . x s z", false),
        ("x q . x r", false),
    ] {
        let (old, old_work) = run(&strict, text, 100000, 0);
        let (new, new_work) = run(&checked, text, 100000, 0);
        let a = input_checked_whole(&old);
        let b = input_checked_whole(&new);
        println!(
            "INPUT_CHECKED_ORDINARY_MACHINE_WHOLE {}",
            json!({"text":text,"strict":a,"input_checked":b,"strict_work":old_work,"input_checked_work":new_work,"expected_acceptance":accepted,"recursive_first_commitment_claim":false})
        );
        assert_eq!(a, b);
        assert_eq!(new?.evaluation.diagnostics.is_empty(), accepted);
        if accepted {
            assert!(new_work > 1 && new_work < 100000);
            assert_eq!(input_checked_whole(&run(&checked, text, new_work, 0).0), b);
            assert_eq!(
                input_checked_whole(&run(&checked, text, new_work + 7, 7).0),
                b
            );
            let (short, used) = run(&checked, text, new_work - 1, 0);
            assert_failed(short);
            assert_eq!(used, new_work - 1);
        }
    }
    Ok(())
}

#[test]
fn input_checked_ordinary_choice_native_ambiguity_still_rejected_by_compiler() {
    let source = input_checked_full_source("'x' | 'x'");
    let strict = strict_compile("InputCheckedOrdinaryAmbiguous", &source);
    let checked = input_checked_compile("InputCheckedOrdinaryAmbiguous", &source);
    println!(
        "INPUT_CHECKED_ORDINARY_NATIVE_AMBIGUITY {}",
        json!({
        "source":source,"strict":strict.as_ref().map(|c|c.parts().0),
        "input_checked":checked.as_ref().map(|c|c.parts().0),"runtime_executed":false,
        "kernel_dynamic_ambiguity_is_not_native_admission":true})
    );
    for result in [strict, checked] {
        let error = result
            .err()
            .unwrap_or_else(|| panic!("Ambiguous native grammar admitted"));
        assert!(
            error.contains("Ambiguous parser prediction requires predicate resolution"),
            "{error}"
        );
        assert!(!error.contains("budget exceeded"), "{error}");
    }
}

fn input_checked_one_shot_optional_source() -> String {
    input_checked_call_suffix_source('+')
        .replace(
            "(Name '::')+ Name 'q'",
            "GlobalQualification? Qualification? Name 'q'",
        )
        .replace(
            "Name returns Ecore::EString:",
            r#"GlobalQualification returns Ecore::EString: '$' '::';
Qualification returns Ecore::EString: (Name '::')+;
Name returns Ecore::EString:"#,
        )
}

#[test]
fn input_checked_one_shot_optional_imported_qualified_name_whole_and_short() -> Result<(), String> {
    let source = input_checked_one_shot_optional_source();
    assert!(source.contains("GlobalQualification? Qualification? Name"));
    let strict = strict_compile("InputCheckedOptionalQualifiedName", &source)?;
    let checked = input_checked_compile("InputCheckedOptionalQualifiedName", &source)?;
    for (text, accepted) in [
        ("z", true),
        ("x q z", true),
        ("$ :: x q z", true),
        ("x :: y q z", true),
        ("$ :: x :: y :: 'u' q z", true),
        ("x q y q z", true),
        ("$ x q z", false),
        ("x :: q z", false),
        ("$ :: q z", false),
        ("x :: y q", false),
    ] {
        let (a, aw) = run(&strict, text, 100000, 0);
        let (b, bw) = run(&checked, text, 100000, 0);
        let old = input_checked_whole(&a);
        let new = input_checked_whole(&b);
        println!(
            "INPUT_CHECKED_OPTIONAL_QUALIFIED_NAME {}",
            json!({"text":text,"source":source,"strict":old,"input_checked":new,"strict_work":aw,"input_checked_work":bw,"expected_acceptance":accepted})
        );
        assert_eq!(old, new);
        assert_eq!(b?.evaluation.diagnostics.is_empty(), accepted, "{text}");
        if accepted {
            assert!(bw > 1 && bw < 100000);
            assert_eq!(input_checked_whole(&run(&checked, text, bw, 0).0), new);
            assert_eq!(input_checked_whole(&run(&checked, text, bw + 7, 7).0), new);
            let (short, used) = run(&checked, text, bw - 1, 0);
            assert_failed(short);
            assert_eq!(used, bw - 1);
        }
    }
    Ok(())
}

#[test]
fn input_checked_one_shot_optional_keyword_falsebody_hidden_and_unknown_stay_explicit()
-> Result<(), String> {
    let original = input_checked_call_suffix_source('+');
    let source = original.replace("(Name '::')+ Name 'q'", "'g'? Name 'q'");
    assert_ne!(source, original);
    let strict = strict_compile("InputCheckedOptionalKeyword", &source)?;
    let checked = input_checked_compile("InputCheckedOptionalKeyword", &source)?;
    for (text, accepted) in [
        ("z", true),
        ("x q z", true),
        ("g x q z", true),
        ("g q z", false),
        ("g x q", false),
    ] {
        let (a, _) = run(&strict, text, 100000, 0);
        let (b, _) = run(&checked, text, 100000, 0);
        let old = input_checked_whole(&a);
        let new = input_checked_whole(&b);
        println!(
            "INPUT_CHECKED_OPTIONAL_KEYWORD {}",
            json!({"text":text,"strict":old,"input_checked":new,"expected_acceptance":accepted})
        );
        assert_eq!(old, new);
        assert_eq!(b?.evaluation.diagnostics.is_empty(), accepted);
    }
    let guarded = source.replace(
        "fragment Item returns gate::Root:",
        "fragment Item returns gate::Root hidden():",
    );
    assert_ne!(guarded, source);
    assert!(strict_compile("InputCheckedOptionalGuardedHidden", &guarded).is_err());
    assert!(input_checked_compile("InputCheckedOptionalGuardedHidden", &guarded).is_err());
    let qualified = input_checked_one_shot_optional_source();
    let unknown = qualified.replace("GlobalQualification?", "Missing?");
    assert_ne!(unknown, qualified);
    assert!(input_checked_compile("InputCheckedOptionalUnknown", &unknown).is_err());
    Ok(())
}

#[test]
fn input_checked_native_seal_reuse_two_source_occurrences_keep_complete_machine_parity()
-> Result<(), String> {
    let source =
        first_source('+', "'x' 'p'").replace("(-> Item)+ 'z'", "(-> Item)+ 'q' (-> Item)+ 'z'");
    assert!(source.contains("'q' (-> Item)+"));
    let strict = strict_compile("NativePairTwoOccurrences", &source)?;
    let checked = input_checked_compile("NativePairTwoOccurrences", &source)?;
    assert_eq!(checked.parts().1.decision_count(), 2);
    for (text, accepted) in [
        ("x p q x p z", true),
        ("x p x p q x p x p z", true),
        ("q x p z", false),
        ("x p q z", false),
        ("x q x p z", false),
        ("x p q x z", false),
    ] {
        let (old, _) = run(&strict, text, 100000, 0);
        let (new, used) = run(&checked, text, 100000, 0);
        assert_eq!(
            input_checked_whole(&new),
            input_checked_whole(&old),
            "{text}"
        );
        assert_eq!(
            new.as_ref()
                .is_ok_and(|v| v.evaluation.diagnostics.is_empty()),
            accepted,
            "{text}: {}",
            input_checked_whole(&new)
        );
        if accepted {
            let (exact, reads) = run(&checked, text, used, 0);
            assert_eq!(input_checked_whole(&exact), input_checked_whole(&new));
            assert_eq!(reads, used);
            let (short, reads) = run(&checked, text, used - 1, 0);
            assert_failed(short);
            assert_eq!(reads, used - 1);
        }
    }
    let hidden = source.replace("fragment Item returns", "fragment Item hidden() returns");
    assert_ne!(source, hidden);
    assert!(strict_compile("HiddenNativePair", &hidden).is_err());
    assert!(input_checked_compile("HiddenNativePair", &hidden).is_err());

    // Actual imported/native source owners differ, as in the original FSP/PFSP
    // pair. Both fragments still call the SAME Item with unchanged constructor.
    let distinct=source.replace("{gate::Root} (-> Item)+ 'q' (-> Item)+ 'z';",
        "{gate::Root} OwnerA 'q' OwnerB 'z';\nfragment OwnerA returns gate::Root: (-> Item)+;\nfragment OwnerB returns gate::Root: (-> Item)+;");
    assert_ne!(source, distinct);
    let strict = strict_compile("DistinctNativeSourceOwners", &distinct)?;
    let checked = input_checked_compile("DistinctNativeSourceOwners", &distinct)?;
    assert_eq!(checked.parts().1.decision_count(), 2);
    let owners: std::collections::BTreeSet<_> = checked
        .parts()
        .1
        .decisions()
        .filter_map(|path| path.split('/').nth(1))
        .collect();
    assert_eq!(owners, ["OwnerA", "OwnerB"].into());
    for (text, accepted) in [
        ("x p q x p z", true),
        ("x p x p q x p x p z", true),
        ("q x p z", false),
        ("x p q z", false),
        ("x q x p z", false),
        ("x p q x z", false),
    ] {
        let (old, _) = run(&strict, text, 100000, 0);
        let (new, used) = run(&checked, text, 100000, 0);
        assert_eq!(
            input_checked_whole(&new),
            input_checked_whole(&old),
            "{text}"
        );
        assert_eq!(
            new.as_ref()
                .is_ok_and(|v| v.evaluation.diagnostics.is_empty()),
            accepted,
            "{text}: {}",
            input_checked_whole(&new)
        );
        if accepted {
            let (exact, reads) = run(&checked, text, used, 0);
            assert_eq!(input_checked_whole(&exact), input_checked_whole(&new));
            assert_eq!(reads, used);
            let (short, reads) = run(&checked, text, used - 1, 0);
            assert_failed(short);
            assert_eq!(reads, used - 1);
        }
    }
    Ok(())
}

#[test]
fn full_predicate_assignment_target_prefers_binding_when_both_readings_accept() -> Result<(), String>
{
    // The same conflict as SysML TargetParameter, inside a live caller body.
    // tag records the optional binding; tail records the target member.
    // The binding has a valid short prefix while an invocation reading is
    // still alive, as BaseExpression does inside SysML TargetBinding.
    let source = format!(
        r#"grammar intersect.iteration.oracle.AssignmentTarget hidden(WS)
import "{NS}" as gate
import "http://www.eclipse.org/emf/2002/Ecore" as Ecore
Root returns gate::Root: {{gate::Root}} '{{' (=> children+=Assignment)* '}}';
Assignment returns gate::Target: 'assign' (=> tag=TargetBinding '.')? tail=TargetFeatureMember ':=' '1' ';';
TargetBinding returns Ecore::EString: Name | Name '.' Name '(' ')';
TargetFeatureMember returns Ecore::EString: Name ('.' Name)*;
Name returns Ecore::EString: 'a' | 'b' | 'index';
terminal WS: (' ' | '\t' | '\r' | '\n')+;
"#
    );
    let unguarded = source.replace("=> tag=TargetBinding", "tag=TargetBinding");
    if let Ok(plain) = input_checked_compile("UnguardedAssignmentTarget", &unguarded) {
        let (actual, _) = run(&plain, "{ assign a.b := 1; }", 1_000_000, 0);
        assert!(
            actual.is_err() || actual.is_ok_and(|value| !value.evaluation.diagnostics.is_empty()),
            "An unguarded ambiguity must remain explicit"
        );
    }
    let compiled = input_checked_compile("AssignmentTarget", &source)?;
    for (text, binding, member) in [
        ("{ assign a.b := 1; }", "a", "b"),
        ("{ assign index := 1; }", "", "index"),
    ] {
        let (actual, _) = run(&compiled, text, 1_000_000, 0);
        let actual = actual?;
        assert!(
            actual.evaluation.diagnostics.is_empty(),
            "{text}: {:?}",
            actual.evaluation.diagnostics
        );
        let assignments = actual
            .evaluation
            .model
            .elements
            .iter()
            .filter(|e| e.class == "Target")
            .collect::<Vec<_>>();
        assert_eq!(assignments.len(), 1, "{text}");
        assert_eq!(
            assignments[0]
                .values
                .get("tag")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            binding,
            "{text}"
        );
        assert_eq!(
            assignments[0].values.get("tail").and_then(Value::as_str),
            Some(member),
            "{text}"
        );
    }
    Ok(())
}
