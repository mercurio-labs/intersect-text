//! Same ordinary compiler, existing real fixtures and complete runtime DTO worker.
use intersect_text::grammar::GrammarAnalysisWorkBudget;
use intersect_text::grammar::Profile;
use intersect_text::grammar::RecursiveFrontierOrder;
use intersect_text::grammar::admission_observation::AnalysisObservers;
use intersect_text::grammar::admission_observation::AnalysisWorkObservers;
use intersect_text::grammar::admission_observation::PredictionObservation;
use intersect_text::grammar::admission_observation::RecursiveAnalysisCounters;
use intersect_text::grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};

pub fn compare(input: BundleCompilation, texts: &[&str]) {
    let source = input.xtext.clone().unwrap();
    let plain = intersect_text::ordinary::compile_with_context_frontier_selected(
        input.clone(),
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        None,
        RecursiveFrontierOrder::BreadthFirstComposedTransparentPorts,
    );
    let phases = PredictionObservation::new();
    let seven = RecursiveAnalysisCounters::new();
    let pool = CompilerPoolDiagnostic::new();
    let composed = intersect_text::ordinary::compile_with_context_shared_call_ports_pool_observed(
        input,
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        AnalysisWorkObservers {
            legacy: AnalysisObservers {
                phases: &phases,
                recursive_work: Some(&seven),
            },
            adjacent_work: None,
        },
        &pool,
    );
    let snapshot = pool.snapshot();
    println!(
        "SHARED_ANALYSIS_CALL_SOURCE_PREASSERT {}",
        serde_json::json!({
            "source":source,"plain":plain,"composed":composed,
            "programs":snapshot.programs,"active":snapshot.active,
            "capture_unavailable":snapshot.capture_unavailable,
            "last":snapshot.last.map(|row|serde_json::json!({
                "order":format!("{:?}",row.order),"initial":row.work.initial,
                "remaining":row.work.remaining,"units":row.work.counts.units,
                "balanced":row.work.balanced,"ended":row.work.ended,
            }))
        })
    );
    assert_eq!(
        serde_json::to_value(&plain).unwrap(),
        serde_json::to_value(&composed).unwrap()
    );
    assert!(
        snapshot.programs > 0
            && !snapshot.active
            && !snapshot.inconsistent
            && !snapshot.overflowed
            && !snapshot.capture_unavailable
    );
    let last = snapshot.last.unwrap();
    assert_eq!(
        last.order,
        RecursiveFrontierOrder::BreadthFirstSharedCallPorts
    );
    assert!(last.work.balanced && last.work.ended && last.work.completed_result);
    // The default per-pool grant became GrammarAnalysisWorkBudget::REALISTIC in 1e5cd75b.
    assert_eq!(
        last.work.initial,
        GrammarAnalysisWorkBudget::default().units()
    );
    // Reuse the existing shared Evaluation/DocumentAnalysis/recovery comparator.
    super::pool_attribution_controls::whole_outcomes(&plain.unwrap(), &composed.unwrap(), texts);
}
