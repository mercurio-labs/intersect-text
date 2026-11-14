use super::*;
#[test]
fn recursive_counter_storage_is_separate_bounded_and_resets_explicitly() {
    println!(
        "RECURSIVE_COUNTER_NATIVE_LAYOUT {}/{}",
        std::mem::size_of::<RecursiveAnalysisCounters>(),
        std::mem::align_of::<RecursiveAnalysisCounters>()
    );
    assert!(std::mem::size_of::<RecursiveAnalysisCounters>() <= 8192);
    let counters = RecursiveAnalysisCounters::new();
    assert_eq!(counters.snapshot(), RecursiveCounterSnapshot::default());
    counters.begin_program(RunStage::Parent, 1);
    counters.charge(RecursiveWorkCategory::FrontierPop, true);
    counters.end_program(0, true);
    counters.begin_program(RunStage::Executable, 2);
    counters.charge(RecursiveWorkCategory::TransitionEdge, true);
    counters.end_program(1, false);
    let s = counters.snapshot();
    assert_eq!(s.stages[0].program_invocations, 1);
    assert_eq!(s.stages[2].program_invocations, 1);
    assert_eq!(s.stages[0].work.granted[0], 1);
    assert_eq!(s.stages[2].work.granted[4], 1);
    assert_eq!(s.last_program.unwrap().ordinal, 1);
    assert!(s.last_program.unwrap().conserved);
    assert!(!s.overflowed);
    assert!(!s.inconsistent);
    counters.reset();
    assert_eq!(counters.snapshot(), RecursiveCounterSnapshot::default());
}
#[test]
fn recursive_counter_overflow_is_explicit_and_never_replaces_semantic_failure() {
    let counters = RecursiveAnalysisCounters::new();
    counters.begin_program(RunStage::Executable, 1);
    let mut snapshot = counters.snapshot();
    snapshot.stages[2].work.granted[0] = u64::MAX;
    counters.state.set(snapshot);
    counters.charge(RecursiveWorkCategory::FrontierPop, true);
    counters.end_program(0, false);
    let s = counters.snapshot();
    assert!(s.overflowed);
    assert_eq!(s.stages[2].work.granted[0], u64::MAX);
    assert!(!s.last_program.unwrap().conserved);
    assert_eq!(s.last_program.unwrap().granted[0], 1);
    counters.reset();
    let mut snapshot = counters.snapshot();
    snapshot.stages[0].program_invocations = u64::MAX;
    snapshot.stages[1].program_invocations = 1;
    counters.state.set(snapshot);
    counters.begin_program(RunStage::Executable, 0);
    counters.end_program(0, false);
    assert!(counters.snapshot().overflowed);
    assert!(!counters.snapshot().last_program.unwrap().conserved);
}
#[test]
fn recursive_counter_unavailable_capture_never_truncates_or_guesses_owner() {
    let grammar = Grammar {
        call_depth: None,
        analysis_work_budget: Default::default(),
        parameterized_rules: Default::default(),
        final_rules: Default::default(),
        computed_settings: vec![],
        profile: crate::grammar::Profile::NativeV73,
        name: "control".into(),
        entry: "Entry".into(),
        rules: Default::default(),
        terminals: Default::default(),
        hidden: None,
        lexical_prediction: None,
        parser_prediction: None,
        declaration_sources: Default::default(),
        terminal_order: vec![],
        shadowed_declarations: Default::default(),
    };
    let source = SourceId {
        uri: "x".repeat(513),
        sha256: "f".repeat(64),
    };
    let counters = RecursiveAnalysisCounters::new();
    counters.begin_program(RunStage::Executable, 1);
    counters.begin_decision(
        &grammar,
        &source,
        &format!("rule/Unknown/{}", "x".repeat(300)),
    );
    counters.charge(RecursiveWorkCategory::FrontierPop, true);
    let result = Err("original semantic error".to_owned());
    counters.end_decision(&result);
    counters.end_program(0, false);
    assert_eq!(result, Err("original semantic error".to_owned()));
    let s = counters.snapshot();
    let current = s.current.unwrap();
    assert!(current.capture_unavailable);
    assert!(current.source.is_none());
    assert!(current.owner_rule.is_none());
    assert!(current.decision.is_none());
    assert!(current.declaration_span.is_none());
    assert!(current.failed);
    assert!(s.last_program.unwrap().conserved);
}
