use super::*;
use crate::grammar;
use crate::grammar::Profile;
use crate::grammar::SourceLimits;
use crate::grammar::admission_observation::AnalysisObservers;
use crate::grammar::admission_observation::Coverage;
use crate::grammar::admission_observation::PredictionObservation;
use crate::grammar::admission_observation::RecursiveAnalysisCounters;
use crate::grammar::backend_test_support::inspect_ecore_schema;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use std::{collections::BTreeSet, rc::Rc};

const PATH: &str = "rule/Root/s2/repeat";

fn parsed(operand: &str, suffix: &str, extra: &str) -> (DefinitionSource, Grammar) {
    let schema = inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
    let source = DefinitionSource::new(
        "memory:repeat/source.xtext",
        SourceKind::Xtext,
        format!(
            "grammar cert hidden() import \"urn:t\" as t Root returns t::Root: {{t::Root}} '{{' => Item* {suffix}; fragment Item returns t::Root: {operand}; {extra}"
        ),
    );
    // This is the existing source parser, not an authored or normalized AST.
    let (mut grammar, _, spans, _) = grammar::parse_declarations(
        &source.text,
        &schema,
        Profile::NativeV74,
        false,
        None,
        128,
        Rc::default(),
    )
    .unwrap();
    grammar.declaration_sources = spans
        .into_iter()
        .map(|(name, range)| {
            (
                name,
                SourceSpan {
                    source: source.id.clone(),
                    start: range.start,
                    end: range.end,
                },
            )
        })
        .collect();
    grammar::terminals::prepare(&mut grammar.terminals, grammar.profile)
        .unwrap_or_else(|error| panic!("{}", error.message));
    for (name, rule) in &mut grammar.rules {
        grammar::bind_prediction_paths(
            &mut rule.body,
            &format!("rule/{name}"),
            &mut BTreeSet::new(),
            true,
        );
    }
    (source, grammar)
}
fn collected(grammar: &Grammar) -> RepeatCertificates {
    grammar::follow::check_required_calls_certified(grammar, &mut BTreeSet::new(), None)
        .unwrap()
        .2
}
fn compiled() -> (
    DefinitionSource,
    Grammar,
    RepeatCertificates,
    ParserPrediction,
) {
    let (source, grammar) = parsed("'x' ('(' Item* ')')?", "'}'", "");
    let mut certificates = collected(&grammar);
    let program = ParserPrediction::compile_certified_repeats(
        &grammar,
        &BTreeSet::new(),
        true,
        None,
        &mut certificates,
    )
    .unwrap();
    (source, grammar, certificates, program)
}

#[test]
fn source_repeat_certificate_is_consumed_from_native_source_and_preserves_whole_program() {
    let (source, grammar, certificates, program) = compiled();
    assert_eq!(certificates.entries.len(), 1);
    assert_eq!((certificates.bound, certificates.consumed), (1, 1));
    assert_eq!(
        (certificates.binding_work, certificates.consumption_work),
        (7, 12)
    );
    let proof = &certificates.entries[PATH];
    assert_eq!(proof.source, source.id);
    assert_eq!(proof.callee_source, source.id);
    assert_eq!(&source.text[proof.occurrence.clone()], "=> Item*");
    assert_eq!(&source.text[proof.operand.clone()], "Item");
    assert_eq!(&source.text[proof.suffix.clone()], "'}'");
    assert!(program.recursive_decisions.is_empty());
    let mut legacy = grammar.clone();
    legacy.profile = Profile::NativeV73;
    let ordinary =
        ParserPrediction::compile_with_recursive_first_predicates(&legacy, &BTreeSet::new())
            .unwrap();
    assert_eq!(
        serde_json::to_value(&ordinary).unwrap(),
        serde_json::to_value(&program).unwrap()
    );
    println!(
        "REPEAT_SOURCE_PROOF {} {} {} {}",
        proof.owner, proof.path, certificates.binding_work, certificates.consumption_work
    );
}

#[test]
fn source_repeat_binding_has_literal_zero_exact_short_fees_and_no_early_mutation() {
    let (_, grammar, complete, _) = compiled();
    let binding = complete.entries[PATH].graph.unwrap();
    let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
        panic!("native sequence");
    };
    for limit in 0..=7 {
        let mut proofs = collected(&grammar);
        let before = proofs.entries[PATH].graph;
        let mut remaining = limit;
        let mut context = GraphRepeatContext {
            grammar: &grammar,
            owner: "Root",
            certificates: &mut proofs,
            remaining: &mut remaining,
            exact: None,
        };
        let result = context.bind(
            &items[2],
            "rule/Root/s2",
            binding,
            Some((items, 3, "rule/Root")),
        );
        if limit < 7 {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert_eq!(proofs.entries[PATH].graph, before);
            assert_eq!(proofs.bound, 0);
        } else {
            assert_eq!(result, Ok(()));
            assert_eq!(proofs.bound, 1);
        }
        assert_eq!(remaining, 0);
        assert_eq!(proofs.binding_work, limit);
    }
}

#[test]
fn source_repeat_consumption_uses_same_pool_exact_twelve_and_one_short() {
    let (_, grammar, complete, program) = compiled();
    let decision = &program.decisions[PATH];
    for limit in 0..=12 {
        let mut proofs = complete.clone();
        proofs.consumed = 0;
        proofs.consumption_work = 0;
        let mut remaining = limit;
        let result = program.certified_repeat_is_nonconflicting(
            PATH,
            decision,
            &grammar,
            &mut proofs,
            &mut remaining,
        );
        if limit < 12 {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
            assert_eq!(proofs.consumed, 0);
        } else {
            assert_eq!(result, Ok(true));
            assert_eq!(proofs.consumed, 1);
        }
        assert_eq!(remaining, 0);
        assert_eq!(proofs.consumption_work, limit);
    }
    let mut proofs = collected(&grammar);
    let binding = complete.entries[PATH].graph.unwrap();
    let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
        panic!("sequence");
    };
    let mut shared = 18;
    GraphRepeatContext {
        grammar: &grammar,
        owner: "Root",
        certificates: &mut proofs,
        remaining: &mut shared,
        exact: None,
    }
    .bind(
        &items[2],
        "rule/Root/s2",
        binding,
        Some((items, 3, "rule/Root")),
    )
    .unwrap();
    assert_eq!(shared, 11);
    assert_eq!(
        program.certified_repeat_is_nonconflicting(
            PATH,
            decision,
            &grammar,
            &mut proofs,
            &mut shared
        ),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(
        (
            shared,
            proofs.binding_work,
            proofs.consumption_work,
            proofs.consumed
        ),
        (0, 7, 11, 0)
    );
}

#[test]
fn source_repeat_nullable_overlapping_or_unknown_exits_do_not_receive_proofs() {
    for (operand, suffix, extra) in [
        ("'x'?", "'}'", ""),
        ("'}'", "'}'", ""),
        ("'x'", "'}'?", ""),
        ("'x'", "('}' | ';')", ""),
        ("'x'", "=> '}'", ""),
        ("ANY", "'}'", "terminal ANY: .;"),
        ("MISSING", "'}'", ""),
        ("INT", "'1'", "terminal INT: ('0'..'9')+;"),
    ] {
        let (_, grammar) = parsed(operand, suffix, extra);
        assert!(
            collected(&grammar).entries.is_empty(),
            "{operand} / {suffix}"
        );
    }
}

#[test]
fn source_repeat_changed_or_unavailable_provenance_is_not_authority() {
    let (_, grammar, complete, program) = compiled();
    let mut missing = grammar.clone();
    missing.declaration_sources.clear();
    assert!(collected(&missing).entries.is_empty());
    let mut bad = complete.clone();
    bad.entries.get_mut(PATH).unwrap().source.sha256 = "different".into();
    let mut remaining = 100;
    assert_eq!(
        program.certified_repeat_is_nonconflicting(
            PATH,
            &program.decisions[PATH],
            &grammar,
            &mut bad,
            &mut remaining
        ),
        Err("Consumed repeat source declaration mismatch".into())
    );
    assert_eq!(remaining, 98);
    assert_eq!(bad.consumed, 1); // prior genuine consumption only, no new one
    let mut altered = program.clone();
    let stop = complete.entries[PATH].graph.unwrap().stop;
    altered.states[stop].edges = vec![Edge::Epsilon(stop)];
    let mut proof = complete.clone();
    proof.consumed = 0;
    assert_eq!(
        altered.certified_repeat_is_nonconflicting(
            PATH,
            &altered.decisions[PATH],
            &grammar,
            &mut proof,
            &mut 100
        ),
        Err("Certified repeat has no exact loop-back boundary".into())
    );
    assert_eq!(proof.consumed, 0);
}

#[test]
fn source_repeat_hidden_policy_and_left_recursion_do_not_acquire_proofs() {
    let (_, mut grammar) = parsed("'x'", "'}'", "");
    // Source-backed parser policy is independently exercised by public controls;
    // here alter only this private negative fixture to test the same traversal.
    grammar.rules.get_mut("Item").unwrap().hidden = Some(vec![]);
    assert!(collected(&grammar).entries.is_empty());
    let (_, grammar) = parsed("Item | 'x'", "'}'", "");
    assert_eq!(
        grammar::follow::check_required_calls_certified(&grammar, &mut BTreeSet::new(), None).err(),
        Some("Grammar prediction budget exceeded".into())
    );
}

#[test]
fn source_repeat_mutual_left_recursion_preserves_the_exact_first_error() {
    let (_, grammar) = parsed(
        "Other | 'x'",
        "'}'",
        "fragment Other returns t::Root: Item;",
    );
    let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
        panic!("sequence");
    };
    let before = grammar::first_sets::analyze(&items[2], &grammar, 0, 10_000);
    let after = grammar::first_sets::analyze_occurrence(&items[2], &grammar, 0, 10_000);
    assert_eq!(before, Err("Grammar prediction budget exceeded".into()));
    assert_eq!(
        after.err(),
        Some("Grammar prediction budget exceeded".into())
    );
    assert_eq!(
        grammar::follow::check_required_calls_certified(&grammar, &mut BTreeSet::new(), None).err(),
        Some("Grammar prediction budget exceeded".into())
    );
}

#[test]
fn source_repeat_foreign_source_path_and_suffix_fail_before_binding_mutation() {
    let (_, grammar, complete, _) = compiled();
    let binding = complete.entries[PATH].graph.unwrap();
    let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
        panic!("sequence");
    };
    for (case, message, used) in [
        (
            0,
            "Certified repeat belongs to a different source declaration",
            2,
        ),
        (1, "Certified repeat source occurrence mismatch", 4),
        (2, "Certified repeat literal source mismatch", 5),
        (
            3,
            "Certified repeat suffix occurrence or binding mismatch",
            6,
        ),
        (4, "Certified repeat callee source mismatch", 3),
    ] {
        let mut proofs = collected(&grammar);
        let proof = proofs.entries.get_mut(PATH).unwrap();
        match case {
            0 => proof.source.sha256 = "foreign".into(),
            1 => proof.path = "rule/Foreign/s2".into(),
            2 => proof.suffix.start += 1,
            3 => proof.suffix_path = "rule/Root/s99".into(),
            _ => proof.callee_source.sha256 = "foreign".into(),
        }
        let mut remaining = 100;
        let result = GraphRepeatContext {
            grammar: &grammar,
            owner: "Root",
            certificates: &mut proofs,
            remaining: &mut remaining,
            exact: None,
        }
        .bind(
            &items[2],
            "rule/Root/s2",
            binding,
            Some((items, 3, "rule/Root")),
        );
        assert_eq!(result, Err(message.into()));
        assert_eq!(remaining, 100 - used);
        assert_eq!((proofs.bound, proofs.binding_work), (0, used));
        assert!(proofs.entries[PATH].graph.is_none());
    }
}

#[test]
fn source_repeat_wrong_call_return_start_and_local_exit_cannot_consume_a_proof() {
    let (_, grammar, complete, original) = compiled();
    let binding = complete.entries[PATH].graph.unwrap();
    let [
        Edge::Full {
            target: operand, ..
        },
    ] = original.states[binding.body].edges.as_slice()
    else {
        panic!("Full edge");
    };
    for (case, message, used) in [
        (0, "Certified repeat operand Call or return mismatch", 9),
        (1, "Certified repeat operand Call or return mismatch", 9),
        (2, "Certified repeat start enter or exit mismatch", 5),
        (3, "Certified repeat literal exit graph mismatch", 11),
        (4, "Certified literal is not the actual repeat exit", 6),
    ] {
        let mut program = original.clone();
        match case {
            0 => {
                let [Edge::Call { target, .. }] = program.states[*operand].edges.as_mut_slice()
                else {
                    panic!("Call");
                };
                *target = program.rules["Root"].0;
            }
            1 => {
                let [Edge::Call { follow, .. }] = program.states[*operand].edges.as_mut_slice()
                else {
                    panic!("Call");
                };
                *follow = binding.end;
            }
            2 => program.states[binding.start].edges.swap(0, 1),
            3 => {
                let [Edge::Token(symbol, _)] = program.states[binding.end].edges.as_mut_slice()
                else {
                    panic!("literal");
                };
                *symbol = PredictionSymbol::Keyword(";".into());
            }
            _ => {
                program
                    .decisions
                    .get_mut(&format!("{}/recovery", complete.entries[PATH].suffix_path))
                    .unwrap()
                    .entries[0] = program.rules["Root"].1
            }
        }
        let mut proofs = complete.clone();
        proofs.consumed = 0;
        proofs.consumption_work = 0;
        let mut remaining = 100;
        assert_eq!(
            program.certified_repeat_is_nonconflicting(
                PATH,
                &program.decisions[PATH],
                &grammar,
                &mut proofs,
                &mut remaining
            ),
            Err(message.into())
        );
        assert_eq!(
            (remaining, proofs.consumption_work, proofs.consumed),
            (100 - used, used, 0)
        );
    }
    // A literal reachable through a separate rule or caller FOLLOW is not the
    // actual immediate mandatory literal source sibling required by this proof.
    for (suffix, extra) in [
        ("Tail", "fragment Tail returns t::Root: '}';"),
        ("", "fragment Caller returns t::Root: Root '}';"),
    ] {
        let (_, grammar) = parsed("'x'", suffix, extra);
        assert!(collected(&grammar).entries.is_empty());
    }
    let mut empty = RepeatCertificates::default();
    let mut remaining = 3;
    assert_eq!(
        original.certified_repeat_is_nonconflicting(
            PATH,
            &original.decisions[PATH],
            &grammar,
            &mut empty,
            &mut remaining
        ),
        Ok(false)
    );
    assert_eq!((remaining, empty.consumption_work), (2, 1));
}

#[test]
fn source_repeat_new_work_coverage_is_explicitly_uninstrumented_without_growing_holders() {
    let (source, grammar) = parsed("'x'", "'}'", "");
    let mut proof = collected(&grammar);
    let phase = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    let _adapters = AnalysisObservers {
        phases: &phase,
        recursive_work: Some(&work),
    };
    let observed = grammar::AnalysisObservation {
        observation: &phase,
        recursive_work: Some(&work),
        adjacent_work: None,
        entry: &source.id,
        stage: grammar::admission_observation::RunStage::Executable,
    };
    ParserPrediction::compile_certified_repeats(
        &grammar,
        &BTreeSet::new(),
        true,
        Some(observed),
        &mut proof,
    )
    .unwrap();
    assert_eq!(
        work.snapshot().coverage,
        Coverage::RepeatCertificateAnalysisUninstrumented
    );
    assert_eq!(work.snapshot().stages[2].program_invocations, 0);
    assert!(phase.first_failure().is_none());
    // Repeat certificate work still adds no holder storage. The independently
    // selected static diagnostic has one explicit, separately bounded extension.
    let selected_bytes = RecursiveAnalysisCounters::SELECTED_STATIC_HOLDER_BYTES;
    assert!(selected_bytes <= 2048);
    assert_eq!(
        std::mem::size_of::<RecursiveAnalysisCounters>() - selected_bytes,
        5456
    );
    assert!(work.selected_static_snapshot().is_none());
    assert_eq!(std::mem::size_of::<Profile>(), 1);
    let _limits = SourceLimits::default();
}

#[test]
fn source_repeat_binding_denial_observes_known_single_pool_work_before_classifier() {
    let (source, grammar) = parsed("'x'", "'}'", "");
    let mut proof = collected(&grammar);
    proof.entries.get_mut(PATH).unwrap().source.sha256 = "foreign".into();
    let phase = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    let observed = grammar::AnalysisObservation {
        observation: &phase,
        recursive_work: Some(&work),
        adjacent_work: None,
        entry: &source.id,
        stage: grammar::admission_observation::RunStage::Executable,
    };
    assert_eq!(
        ParserPrediction::compile_certified_repeats(
            &grammar,
            &BTreeSet::new(),
            true,
            Some(observed),
            &mut proof
        )
        .err(),
        Some("Certified repeat belongs to a different source declaration".into())
    );
    let failure = phase.first_failure().unwrap();
    assert_eq!(
        failure.phase,
        grammar::admission_observation::Phase::ParserRuleGraph
    );
    assert_eq!(failure.charged_work, Some(2));
    assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "Root");
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        source.id.sha256
    );
    assert_eq!(
        work.snapshot().coverage,
        Coverage::RepeatCertificateAnalysisUninstrumented
    );
    assert_eq!(work.snapshot().stages[2].program_invocations, 0);
}

#[test]
fn adjacent_work_repeat_binding_seven_and_consumption_twelve_share_original_grant()
-> Result<(), String> {
    use crate::grammar::admission_observation::AdjacentCharge;
    use crate::grammar::admission_observation::AdjacentWorkObservation;
    use crate::grammar::admission_observation::RunStage;
    let (source, grammar, complete, program) = compiled();
    let binding = complete.entries[PATH]
        .graph
        .ok_or("Missing graph binding")?;
    let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
        return Err("Expected original sequence".into());
    };
    for grant in [0, 18, 19, 19] {
        let run = |exact: Option<&AdjacentWorkObservation>| {
            let mut remaining = grant;
            let mut certificates = collected(&grammar);
            let result = (|| {
                GraphRepeatContext {
                    grammar: &grammar,
                    owner: "Root",
                    certificates: &mut certificates,
                    remaining: &mut remaining,
                    exact,
                }
                .bind(
                    &items[2],
                    "rule/Root/s2",
                    binding,
                    Some((items, 3, "rule/Root")),
                )?;
                program.certified_repeat_is_nonconflicting_observed(
                    PATH,
                    &program.decisions[PATH],
                    &grammar,
                    &mut certificates,
                    &mut remaining,
                    exact,
                )
            })();
            (
                result,
                remaining,
                certificates.binding_work,
                certificates.consumption_work,
                certificates.bound,
                certificates.consumed,
            )
        };
        let exact = AdjacentWorkObservation::new();
        exact.begin(RunStage::Executable, &source.id, grant);
        let a = run(Some(&exact));
        exact.end(a.1, a.0.is_ok());
        assert_eq!(a, run(None));
        let s = exact.snapshot();
        let p = s.last.ok_or("Missing repeat ledger")?;
        assert!(p.conserved && !s.inconsistent);
        assert_eq!(
            p.granted[AdjacentCharge::RepeatBinding as usize],
            a.2 as u64
        );
        assert_eq!(
            p.granted[AdjacentCharge::RepeatConsumption as usize],
            a.3 as u64
        );
        if grant == 19 {
            assert_eq!((a.0, a.1, a.2, a.3, a.4, a.5), (Ok(true), 0, 7, 12, 1, 1));
        } else {
            assert_eq!(p.denied.iter().sum::<u64>(), 1);
        }
    }
    Ok(())
}

// Existing mandatory FOLLOW FIRST supplies the lemma; these controls exercise
// the complete current runtime binding/sealing route, not a second analyzer.
fn runtime_certificate_fixture() -> (
    DefinitionSource,
    Grammar,
    RepeatCertificates,
    ParserPrediction,
) {
    let mut extra = String::new();
    for branch in 0..4 {
        for depth in 0..8 {
            let body = if depth == 7 {
                format!("'token{branch}'")
            } else {
                format!("B{branch}D{}", depth + 1)
            };
            extra.push_str(&format!(
                "fragment B{branch}D{depth} returns t::Root: {body}; "
            ));
        }
    }
    let (source, mut grammar) = parsed("B0D0 | B1D0 | B2D0 | B3D0", "'}'", &extra);
    grammar.profile = Profile::NativeV80;
    let mut certificates = collected(&grammar);
    let program = ParserPrediction::compile_certified_repeats(
        &grammar,
        &BTreeSet::new(),
        true,
        None,
        &mut certificates,
    )
    .unwrap();
    assert!(
        certificates
            .entries
            .get(PATH)
            .is_some_and(|proof| proof.graph.is_some())
    );
    (source, grammar, certificates, program)
}

#[test]
fn runtime_repeat_certificate_reuses_canonical_first_and_keeps_whole_binding() {
    use crate::grammar::AnalysisObservation;
    use crate::grammar::RuntimeGuardedAdmission;
    use crate::grammar::RuntimeGuardedPolicy;
    use crate::grammar::admission_observation::FirstReuseEvent;
    use crate::grammar::admission_observation::RunStage;
    let (source, grammar, certificates, program) = runtime_certificate_fixture();
    let before = serde_json::to_value((&grammar, &program)).unwrap();
    let (mut a, mut b, mut c) = (10_000, 10_000, 10_000);
    let mut old =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut plain =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut observed =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let (mut first, mut second) = (certificates.clone(), certificates.clone());
    let trace = PredictionObservation::new();
    let context = AnalysisObservation {
        observation: &trace,
        recursive_work: None,
        adjacent_work: None,
        entry: &source.id,
        stage: RunStage::Executable,
    };
    let r0 = old.collect(&grammar, &program, &mut a, None);
    let r1 = plain.collect_with_certificates(&grammar, &program, &mut b, None, Some(&mut first));
    let r2 = observed.collect_with_certificates(
        &grammar,
        &program,
        &mut c,
        Some(context),
        Some(&mut second),
    );
    let collect_units = (10_000 - a, 10_000 - b, 10_000 - c);
    let (mut p0, mut p1, mut p2) = (program.clone(), program.clone(), program.clone());
    let s0 = old.seal(&grammar, &mut p0, &mut a);
    let s1 = plain.seal(&grammar, &mut p1, &mut b);
    let s2 = observed.seal(&grammar, &mut p2, &mut c);
    println!(
        "RUNTIME_REPEAT_CERTIFICATE_WHOLE {}",
        serde_json::json!({
            "source":source.id,"collect":[r0,r1,r2],"seal":[s0,s1,s2],"collect_units":collect_units,
            "total_units":[10000-a,10000-b,10000-c],"programs":[p0,p1,p2],"trace":format!("{:?}",trace.first_reuse_work()),
            "proof_consumption_units":first.consumption_work-certificates.consumption_work,
            "scope":"existing mandatory canonical FOLLOW proof; every new lookup paid; not original551 admission"
        })
    );
    assert_eq!(r0, r1);
    assert_eq!(r1, r2);
    assert!(r0.is_ok());
    assert_eq!(s0, s1);
    assert_eq!(s1, s2);
    assert!(s0.is_ok());
    assert_eq!(
        serde_json::to_value(&p0).unwrap(),
        serde_json::to_value(&p1).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&p1).unwrap(),
        serde_json::to_value(&p2).unwrap()
    );
    assert_eq!(b, c);
    assert!(
        b > a,
        "complete real source/binding/sealing cost must improve"
    );
    assert_eq!(plain.decision_count(), 1);
    assert_eq!(
        observed.decisions().collect::<Vec<_>>(),
        old.decisions().collect::<Vec<_>>()
    );
    assert!(trace.first_failure().is_none());
    let work = trace.first_reuse_work().unwrap();
    assert_eq!(work.events[FirstReuseEvent::FirstQuery as usize], Some(0));
    assert_eq!(work.first_inclusive_work, Some(0));
    assert_eq!(serde_json::to_value((&grammar, &program)).unwrap(), before);
}

#[test]
fn runtime_repeat_certificate_absent_unbound_and_ineligible_use_canonical_first() {
    use crate::grammar::RuntimeGuardedAdmission;
    use crate::grammar::RuntimeGuardedPolicy;
    let (_, grammar, certificates, program) = runtime_certificate_fixture();
    let mut baseline =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut old_work = 10_000;
    let old = baseline.collect(&grammar, &program, &mut old_work, None);
    for unbound in [false, true] {
        let mut proof = certificates.clone();
        if unbound {
            proof.entries.get_mut(PATH).unwrap().graph = None;
        } else {
            proof.entries.clear();
        }
        let consumed = proof.consumed;
        let mut work = 10_000;
        let mut actual =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let trace = PredictionObservation::new();
        let source = &grammar.declaration_sources["Root"].source;
        let context = crate::grammar::AnalysisObservation {
            observation: &trace,
            recursive_work: None,
            adjacent_work: None,
            entry: source,
            stage: crate::grammar::admission_observation::RunStage::Executable,
        };
        let result = actual.collect_with_certificates(
            &grammar,
            &program,
            &mut work,
            Some(context),
            Some(&mut proof),
        );
        println!(
            "RUNTIME_REPEAT_CERTIFICATE_FALLBACK_WHOLE {:?}",
            (
                unbound,
                &old,
                &result,
                old_work,
                work,
                trace.first_reuse_work()
            )
        );
        assert_eq!(result, old);
        assert!(result.is_ok());
        assert_eq!(actual.decision_count(), baseline.decision_count());
        assert_eq!(proof.consumed, consumed);
        assert!(work < old_work, "new unsuccessful proof probe is charged");
        assert_eq!(
            trace.first_reuse_work().unwrap().events
                [crate::grammar::admission_observation::FirstReuseEvent::FirstQuery as usize],
            Some(1)
        );
    }
    // The existing canonical nullable/error result remains authority when FOLLOW
    // cannot produce a certificate. No truth is inferred from a first token.
    let (_, mut nullable) = parsed("'x'?", "'}'", "");
    nullable.profile = Profile::NativeV80;
    let mut absent = collected(&nullable);
    assert!(absent.entries.is_empty());
    let graph = ParserPrediction::compile_certified_repeats(
        &nullable,
        &BTreeSet::new(),
        true,
        None,
        &mut absent,
    );
    // A prior canonical graph rejection is retained rather than fabricated into
    // a successfully admitted runtime candidate.
    println!("RUNTIME_REPEAT_NULLABLE_GRAPH_WHOLE {:?}", graph);
    if let Ok(graph) = graph {
        let (mut a, mut b) = (10_000, 10_000);
        let mut old =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let mut new =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let r0 = old.collect(&nullable, &graph, &mut a, None);
        let r1 = new.collect_with_certificates(&nullable, &graph, &mut b, None, Some(&mut absent));
        println!(
            "RUNTIME_REPEAT_NULLABLE_FALLBACK_WHOLE {:?}",
            (&r0, &r1, a, b)
        );
        assert_eq!(r0, r1);
        assert!(r1.is_err());
        assert_eq!(new.decision_count(), 0);
    }
}

#[test]
fn runtime_repeat_certificate_bound_source_and_graph_mismatches_stay_errors() {
    let (_, grammar, complete, program) = runtime_certificate_fixture();
    for case in 0..4 {
        let mut proof = complete.clone();
        proof.consumed = 0;
        let mut graph = program.clone();
        match case {
            0 => proof.entries.get_mut(PATH).unwrap().source.sha256 = "foreign".into(),
            1 => proof.entries.get_mut(PATH).unwrap().callee_source.uri = "memory:foreign".into(),
            2 => {
                let stop = proof.entries[PATH].graph.unwrap().stop;
                graph.states[stop].edges = vec![Edge::Epsilon(stop)];
            }
            _ => {
                let target = proof.entries[PATH].graph.unwrap().body;
                graph.states[target].edges.clear();
            }
        }
        let mut work = 10_000;
        let result = graph.runtime_repeat_certificate_is_nonnullable(
            PATH,
            &graph.decisions[PATH],
            &grammar,
            &mut proof,
            &mut work,
        );
        println!(
            "RUNTIME_REPEAT_BOUND_MISMATCH_WHOLE {:?}",
            (case, &result, work, proof.consumed)
        );
        assert!(result.is_err());
        assert_eq!(proof.consumed, 0);
        assert!(work < 10_000);
    }
}

#[test]
fn runtime_repeat_certificate_short_shared_pool_never_publishes_partial_proof() {
    use crate::grammar::RuntimeGuardedAdmission;
    use crate::grammar::RuntimeGuardedPolicy;
    let (_, grammar, complete, program) = runtime_certificate_fixture();
    let mut proof = complete.clone();
    let mut work = 10_000;
    assert_eq!(
        program.runtime_repeat_certificate_is_nonnullable(
            PATH,
            &program.decisions[PATH],
            &grammar,
            &mut proof,
            &mut work
        ),
        Ok(true)
    );
    let needed = 10_000 - work;
    // Long common-prefix keys multiply the conservative comparison bound by
    // key-read chunks. Compare real unsuccessful probes of the same map, not
    // a test reimplementation of the production fee formula.
    let long = format!("rule/{}/s0/repeat", "shared_prefix".repeat(24));
    let other = format!("rule/{}/s1/repeat", "shared_prefix".repeat(24));
    let mut paths = complete.clone();
    let mut entry = paths.entries.remove(PATH).unwrap();
    entry.graph = None;
    paths.entries.insert(long.clone(), entry.clone());
    paths.entries.insert(other, entry);
    let (mut short_work, mut long_work) = (10_000, 10_000);
    let short = program.runtime_repeat_certificate_is_nonnullable(
        "rule/X/repeat",
        &program.decisions[PATH],
        &grammar,
        &mut paths,
        &mut short_work,
    );
    let longer = program.runtime_repeat_certificate_is_nonnullable(
        &long,
        &program.decisions[PATH],
        &grammar,
        &mut paths,
        &mut long_work,
    );
    println!(
        "RUNTIME_REPEAT_LONG_KEY_WHOLE {:?}",
        (&short, &longer, short_work, long_work)
    );
    assert_eq!(short, Ok(false));
    assert_eq!(longer, Ok(false));
    assert!(
        long_work < short_work,
        "long shared-prefix key reads are really charged"
    );
    for limit in [0, 1, needed - 1, needed] {
        let mut proof = complete.clone();
        proof.consumed = 0;
        let mut work = limit;
        let result = program.runtime_repeat_certificate_is_nonnullable(
            PATH,
            &program.decisions[PATH],
            &grammar,
            &mut proof,
            &mut work,
        );
        println!(
            "RUNTIME_REPEAT_PROOF_SHORT_WHOLE {:?}",
            (limit, &result, work, proof.consumed)
        );
        assert_eq!(result.is_ok(), limit == needed);
        assert_eq!(work, 0);
        assert_eq!(proof.consumed, usize::from(limit == needed));
    }
    let mut proof = complete.clone();
    let mut owner =
        RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
    let mut work = 10_000;
    assert!(
        owner
            .collect_with_certificates(&grammar, &program, &mut work, None, Some(&mut proof))
            .is_ok()
    );
    let total = 10_000 - work;
    for limit in [0, 1, total - 1, total] {
        let mut plain =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let mut observed =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::StrictCompletionScopes);
        let (mut a, mut b) = (limit, limit);
        let (mut p, mut q) = (complete.clone(), complete.clone());
        let trace = PredictionObservation::new();
        let context = crate::grammar::AnalysisObservation {
            observation: &trace,
            recursive_work: None,
            adjacent_work: None,
            entry: &grammar.declaration_sources["Root"].source,
            stage: crate::grammar::admission_observation::RunStage::Executable,
        };
        let r0 = plain.collect_with_certificates(&grammar, &program, &mut a, None, Some(&mut p));
        let r1 = observed.collect_with_certificates(
            &grammar,
            &program,
            &mut b,
            Some(context),
            Some(&mut q),
        );
        println!(
            "RUNTIME_REPEAT_COLLECT_SHORT_WHOLE {:?}",
            (
                limit,
                &r0,
                &r1,
                a,
                b,
                plain.decision_count(),
                observed.decision_count(),
                trace.first_failure()
            )
        );
        assert_eq!(r0, r1);
        assert_eq!(a, b);
        assert_eq!(r1.is_ok(), limit == total);
        assert_eq!(observed.decision_count(), usize::from(limit == total));
        if r1.is_err() {
            assert!(
                observed
                    .collect(&grammar, &program, &mut 10_000, None)
                    .is_err()
            );
        }
    }
}

fn occurrence_fixture() -> (DefinitionSource, Grammar) {
    let mut extra = String::new();
    for branch in 0..4 {
        for depth in 0..8 {
            let body = if depth == 7 {
                format!("'token{branch}'")
            } else {
                format!("B{branch}D{}", depth + 1)
            };
            extra.push_str(&format!(
                "fragment B{branch}D{depth} returns t::Root: {body}; "
            ));
        }
    }
    let (source, mut grammar) = parsed("B0D0 | B1D0 | B2D0 | B3D0", "('tail')?", &extra);
    grammar.profile = Profile::NativeV80;
    (source, grammar)
}

fn occurrence_collected(
    grammar: &Grammar,
    admission: &mut grammar::RuntimeGuardedAdmission,
) -> (usize, RepeatCertificates) {
    let (_, remaining, proofs) = grammar::follow::check_required_calls_certified_runtime(
        grammar,
        &mut BTreeSet::new(),
        None,
        None,
        Some(admission),
    )
    .unwrap();
    (remaining, proofs)
}

#[test]
fn runtime_occurrence_nonnullable_reuses_follow_without_nonconflict_authority() {
    use grammar::{RuntimeGuardedAdmission, RuntimeGuardedPolicy};
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let (source, grammar) = occurrence_fixture();
        let mut old = RuntimeGuardedAdmission::new_with_policy(policy);
        let mut new = RuntimeGuardedAdmission::new_with_policy(policy);
        let (_, old_follow, mut old_proofs) =
            grammar::follow::check_required_calls_certified(&grammar, &mut BTreeSet::new(), None)
                .unwrap();
        let (new_follow, mut new_proofs) = occurrence_collected(&grammar, &mut new);
        assert!(!new_proofs.entries.contains_key(PATH));
        assert!(new_proofs.nonnullable.contains_key(PATH));
        let mut p0 = ParserPrediction::compile_certified_repeats(
            &grammar,
            &BTreeSet::new(),
            true,
            None,
            &mut old_proofs,
        )
        .unwrap();
        let mut p1 = ParserPrediction::compile_certified_repeats(
            &grammar,
            &BTreeSet::new(),
            true,
            None,
            &mut new_proofs,
        )
        .unwrap();
        let (mut a, mut b) = (10_000, 10_000);
        let r0 = old.collect_with_certificates(&grammar, &p0, &mut a, None, Some(&mut old_proofs));
        let r1 = new.collect_with_certificates(&grammar, &p1, &mut b, None, Some(&mut new_proofs));
        let s0 = old.seal(&grammar, &mut p0, &mut a);
        let s1 = new.seal(&grammar, &mut p1, &mut b);
        // FOLLOW starts from its configured grant; collect/seal above use
        // their separate explicit 10k balances. Keep the full paid comparison.
        let old_follow_initial = grammar::GrammarAnalysisWorkBudget::default().units();
        let new_follow_initial = new.work_budget().units();
        assert_eq!(
            new_follow_initial,
            grammar::GrammarAnalysisWorkBudget::LEGACY_10K.units()
        );
        let total0 = old_follow_initial - old_follow + old_proofs.binding_work + 10_000 - a;
        let total1 = new_follow_initial - new_follow + new_proofs.binding_work + 10_000 - b;
        println!(
            "RUNTIME_OCCURRENCE_WHOLE {}",
            serde_json::json!({
            "source":source.id,"collect":[r0,r1],"seal":[s0,s1],"programs":[p0,p1],
            "remaining":[a,b],"complete_retention_binding_collect_seal_units":[total0,total1],
            "retained_bytes":new.nonnullable_retention_test_state().0,"records":new_proofs.nonnullable.len(),
            "authority":"nonnullable only; no literal suffix or disjointness proof"})
        );
        assert_eq!(r0, r1);
        assert!(r0.is_ok());
        assert_eq!(s0, s1);
        assert!(s0.is_ok());
        assert_eq!(
            serde_json::to_value(&p0).unwrap(),
            serde_json::to_value(&p1).unwrap()
        );
        assert!(total1 < total0, "whole paid reuse must beat recomputation");
        let mut work = 10_000;
        assert!(
            !p1.certified_repeat_is_nonconflicting(
                PATH,
                &p1.decisions[PATH],
                &grammar,
                &mut new_proofs,
                &mut work
            )
            .unwrap()
        );
    }
}

#[test]
fn runtime_occurrence_nonnullable_missing_unbound_and_nullable_fall_back() {
    use grammar::{RuntimeGuardedAdmission, RuntimeGuardedPolicy};
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let (_, grammar) = occurrence_fixture();
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let (_, proofs) = occurrence_collected(&grammar, &mut admission);
        assert!(proofs.nonnullable[PATH].graph.is_none());
        let mut bound = proofs.clone();
        let program = ParserPrediction::compile_certified_repeats(
            &grammar,
            &BTreeSet::new(),
            true,
            None,
            &mut bound,
        )
        .unwrap();
        for mut absent in [RepeatCertificates::default(), proofs] {
            let mut work = 10_000;
            assert!(
                !program
                    .runtime_repeat_certificate_is_nonnullable(
                        PATH,
                        &program.decisions[PATH],
                        &grammar,
                        &mut absent,
                        &mut work
                    )
                    .unwrap()
            );
        }
        let (_, mut nullable) = parsed("'x'?", "('tail')?", "");
        nullable.profile = Profile::NativeV80;
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let (_, proofs) = occurrence_collected(&nullable, &mut admission);
        assert!(proofs.nonnullable.is_empty());

        let mut linear =
            RuntimeGuardedAdmission::new_with_policy(RuntimeGuardedPolicy::LinearFullRepeats);
        let (_, linear_proofs) = occurrence_collected(&grammar, &mut linear);
        assert!(linear_proofs.nonnullable.is_empty());
        assert_eq!(linear.nonnullable_retention_test_state(), (0, false));
    }
}

#[test]
fn runtime_occurrence_nonnullable_bound_source_graph_and_operand_mismatches_error() {
    use grammar::{RuntimeGuardedAdmission, RuntimeGuardedPolicy};
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let (_, grammar) = occurrence_fixture();
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let (_, mut proofs) = occurrence_collected(&grammar, &mut admission);
        let program = ParserPrediction::compile_certified_repeats(
            &grammar,
            &BTreeSet::new(),
            true,
            None,
            &mut proofs,
        )
        .unwrap();
        for case in 0..4 {
            let mut altered = proofs.clone();
            let mut graph = program.clone();
            match case {
                0 => altered
                    .nonnullable
                    .get_mut(PATH)
                    .unwrap()
                    .source
                    .sha256
                    .push('0'),
                1 => altered
                    .nonnullable
                    .get_mut(PATH)
                    .unwrap()
                    .callee_source
                    .sha256
                    .push('0'),
                2 => {
                    altered
                        .nonnullable
                        .get_mut(PATH)
                        .unwrap()
                        .graph
                        .as_mut()
                        .unwrap()
                        .body += 1
                }
                _ => {
                    let binding = altered.nonnullable[PATH].graph.unwrap();
                    graph.states[binding.stop].edges.clear();
                }
            }
            let mut work = 10_000;
            let result = graph.runtime_repeat_certificate_is_nonnullable(
                PATH,
                &graph.decisions[PATH],
                &grammar,
                &mut altered,
                &mut work,
            );
            println!("RUNTIME_OCCURRENCE_NEGATIVE case={case} {result:?} remaining={work}");
            assert!(result.is_err());
        }
        // Real source ranges are checked at graph binding, not caller assertions.
        let mut altered = proofs.clone();
        let p = altered.nonnullable.get_mut(PATH).unwrap();
        p.graph = None;
        p.operand.start += 1;
        assert!(
            ParserPrediction::compile_certified_repeats(
                &grammar,
                &BTreeSet::new(),
                true,
                None,
                &mut altered
            )
            .is_err()
        );
    }
}

#[test]
fn runtime_occurrence_nonnullable_short_work_quota_and_observer_parity() {
    use grammar::{AnalysisObservation, RuntimeGuardedAdmission, RuntimeGuardedPolicy};
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        use grammar::admission_observation::RunStage;
        let (source, grammar) = occurrence_fixture();
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let (_, mut proofs) = occurrence_collected(&grammar, &mut admission);
        let program = ParserPrediction::compile_certified_repeats(
            &grammar,
            &BTreeSet::new(),
            true,
            None,
            &mut proofs,
        )
        .unwrap();
        let mut work = 10_000;
        let result = program.runtime_repeat_certificate_is_nonnullable(
            PATH,
            &program.decisions[PATH],
            &grammar,
            &mut proofs.clone(),
            &mut work,
        );
        let used = 10_000 - work;
        assert_eq!(result, Ok(true));
        for grant in [0, used - 1, used] {
            let mut remaining = grant;
            let mut p = proofs.clone();
            let result = program.runtime_repeat_certificate_is_nonnullable(
                PATH,
                &program.decisions[PATH],
                &grammar,
                &mut p,
                &mut remaining,
            );
            println!("RUNTIME_OCCURRENCE_BOUNDARY grant={grant} {result:?} remaining={remaining}");
            assert_eq!(result.is_ok(), grant == used);
            assert_eq!(remaining, 0);
        }
        let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
            panic!("sequence");
        };
        let expr = &items[2];
        let (_, facts) =
            grammar::first_sets::analyze_occurrence(expr, &grammar, 0, 10_000).unwrap();
        let mut full = RuntimeGuardedAdmission::new_with_policy(policy);
        full.nonnullable_retention_test_fill();
        let mut short = RepeatCertificates::default();
        let mut work = 10_000;
        assert!(
            short
                .record_nonnullable(
                    expr,
                    &grammar,
                    facts,
                    (Some("Root"), "rule/Root/s2"),
                    &mut work,
                    &mut full
                )
                .is_err()
        );
        assert!(short.nonnullable.is_empty());
        assert!(full.nonnullable_retention_test_state().1);
        let trace = PredictionObservation::new();
        let context = AnalysisObservation {
            observation: &trace,
            recursive_work: None,
            adjacent_work: None,
            entry: &source.id,
            stage: RunStage::Executable,
        };
        let run = |context| {
            let mut a = RuntimeGuardedAdmission::new_with_policy(policy);
            let mut p = proofs.clone();
            let mut remaining = 10_000;
            let result = a.collect_with_certificates(
                &grammar,
                &program,
                &mut remaining,
                context,
                Some(&mut p),
            );
            (result, remaining)
        };
        let observed = run(Some(context));
        let plain = run(None);
        println!(
            "RUNTIME_OCCURRENCE_PARITY {}",
            serde_json::json!({"observed":observed,"plain":plain,"snapshot":format!("{:?}",trace.first_reuse_work())})
        );
        assert_eq!(observed, plain);
        assert!(plain.0.is_ok());
    }
}

#[test]
fn runtime_occurrence_nonnullable_whole_rule_stop_and_forgeries() {
    use grammar::{RuntimeGuardedAdmission, RuntimeGuardedPolicy};
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let schema=inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
        let source = DefinitionSource::new(
            "memory:whole-rule/nonnullable.xtext",
            SourceKind::Xtext,
            "grammar whole hidden() import \"urn:t\" as t Root returns t::Root: {t::Root} Body; fragment Body returns t::Root: => Item*; fragment Item returns t::Root: 'x'; fragment Other returns t::Root: 'y';",
        );
        let (mut grammar, _, spans, _) = grammar::parse_declarations(
            &source.text,
            &schema,
            Profile::NativeV80,
            false,
            None,
            128,
            Rc::default(),
        )
        .unwrap();
        grammar.declaration_sources = spans
            .into_iter()
            .map(|(name, range)| {
                (
                    name,
                    SourceSpan {
                        source: source.id.clone(),
                        start: range.start,
                        end: range.end,
                    },
                )
            })
            .collect();
        grammar::terminals::prepare(&mut grammar.terminals, grammar.profile)
            .unwrap_or_else(|e| panic!("{}", e.message));
        for (name, rule) in &mut grammar.rules {
            grammar::bind_prediction_paths(
                &mut rule.body,
                &format!("rule/{name}"),
                &mut BTreeSet::new(),
                true,
            );
        }
        let mut admission = RuntimeGuardedAdmission::new_with_policy(policy);
        let (_, mut proofs) = occurrence_collected(&grammar, &mut admission);
        const P: &str = "rule/Body/repeat";
        let program = ParserPrediction::compile_certified_repeats(
            &grammar,
            &BTreeSet::new(),
            true,
            None,
            &mut proofs,
        )
        .unwrap();
        let binding = proofs.nonnullable[P].graph.unwrap();
        assert_eq!(binding.end, program.rules["Body"].1);
        assert!(program.states[binding.end].rule_stop);
        assert!(program.states[binding.end].edges.is_empty());
        assert!(
            !program.states[binding.end].follow.is_empty(),
            "real caller follow must be preserved"
        );
        let mut old = RuntimeGuardedAdmission::new_with_policy(policy);
        let (mut a, mut b) = (10_000, 10_000);
        let r0 = old.collect(&grammar, &program, &mut a, None);
        let r1 = admission.collect_with_certificates(
            &grammar,
            &program,
            &mut b,
            None,
            Some(&mut proofs),
        );
        let (mut p0, mut p1) = (program.clone(), program.clone());
        let s0 = old.seal(&grammar, &mut p0, &mut a);
        let s1 = admission.seal(&grammar, &mut p1, &mut b);
        println!(
            "RUNTIME_OCCURRENCE_WHOLE_RULE {}",
            serde_json::json!({"source":source.id,"collect":[r0,r1],"seal":[s0,s1],"remaining":[a,b],"owner_stop":binding.end,"caller_follow":program.states[binding.end].follow})
        );
        assert_eq!(r0, r1);
        assert!(r0.is_ok());
        assert_eq!(s0, s1);
        assert!(s0.is_ok());
        assert_eq!(
            serde_json::to_value(&p0).unwrap(),
            serde_json::to_value(&p1).unwrap()
        );
        for case in 0..4 {
            let mut graph = program.clone();
            let mut altered = proofs.clone();
            match case {
                0 => graph.states[binding.end].accepting = true,
                1 => graph.states[binding.end]
                    .edges
                    .push(Edge::Epsilon(binding.start)),
                _ => {
                    let end = if case == 2 {
                        graph.rules["Other"].1
                    } else {
                        graph.rules["Item"].0
                    };
                    if case == 3 {
                        graph.states[end].rule_stop = true;
                    }
                    // Keep the forged decision/recovery/start internally coherent:
                    // only the canonical owning-stop condition may reject it.
                    altered
                        .nonnullable
                        .get_mut(P)
                        .unwrap()
                        .graph
                        .as_mut()
                        .unwrap()
                        .end = end;
                    graph.decisions.get_mut(P).unwrap().entries[1] = end;
                    graph
                        .decisions
                        .get_mut("rule/Body/recovery")
                        .unwrap()
                        .entries[1] = end;
                    graph.states[binding.start].edges[1] = Edge::Epsilon(end);
                }
            }
            let mut work = 10_000;
            let result = graph.runtime_repeat_certificate_is_nonnullable(
                P,
                &graph.decisions[P],
                &grammar,
                &mut altered,
                &mut work,
            );
            println!("RUNTIME_OCCURRENCE_STOP_NEGATIVE case={case} {result:?} remaining={work}");
            assert_eq!(result, Err("Nonnullable repeat exit mismatch".into()));
        }
    }
}
