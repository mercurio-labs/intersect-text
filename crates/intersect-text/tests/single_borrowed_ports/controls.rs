//! Actual ordinary compiler and complete common Evaluation/DocumentAnalysis DTOs.
//! These retained genuine inputs are controls, not original SysML admission.
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
    let old = intersect_text::ordinary::compile_with_context_frontier_selected(
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
    let new = intersect_text::ordinary::compile_with_context_shared_call_ports_pool_observed(
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
        "SINGLE_BORROW_GENUINE_PREASSERT {}",
        serde_json::json!({
            "source":source,"old":old,"new":new,"pool":format!("{snapshot:?}"),
            "seven":format!("{:?}",seven.snapshot()),"coverage":format!("{:?}",phases.coverage()),
        })
    );
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(&new).unwrap()
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
    // The default per-pool grant became GrammarAnalysisWorkBudget::REALISTIC in 1e5cd75b.
    assert_eq!(
        last.work.initial,
        GrammarAnalysisWorkBudget::default().units()
    );
    assert!(last.work.ended && last.work.balanced && last.work.completed_result);
    assert_eq!(
        last.work.counts.units.iter().sum::<u64>(),
        (GrammarAnalysisWorkBudget::default().units() - last.work.remaining) as u64
    );
    // One existing evaluator/document/recovery worker compares all source/model
    // identities, spans, diagnostics and whole error/recovery Results.
    super::pool_attribution_controls::whole_outcomes(&old.unwrap(), &new.unwrap(), texts);
}
