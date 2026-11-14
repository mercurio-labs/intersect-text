//! Actual ordinary compiler and complete shared runtime DTOs, observer off/on.
use intersect_text::grammar::GrammarAnalysisWorkBudget;
use intersect_text::grammar::Profile;
use intersect_text::grammar::admission_observation::AnalysisObservers;
use intersect_text::grammar::admission_observation::AnalysisWorkObservers;
use intersect_text::grammar::admission_observation::PredictionObservation;
use intersect_text::grammar::admission_observation::RecursiveAnalysisCounters;
use intersect_text::grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic;
use intersect_text::grammar::admission_observation::pool_blocks::FirstQueueTopologyCoverage;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};

pub fn compare(input: BundleCompilation, texts: &[&str]) {
    let old_phase = PredictionObservation::new();
    let new_phase = PredictionObservation::new();
    let old_seven = RecursiveAnalysisCounters::new();
    let new_seven = RecursiveAnalysisCounters::new();
    let old_pool = CompilerPoolDiagnostic::new();
    let new_pool = CompilerPoolDiagnostic::new();
    new_pool.enable_first_queue_topology();
    let compile = |input, phases, seven, pool| {
        intersect_text::ordinary::compile_with_context_shared_call_ports_pool_observed(
            input,
            Profile::NativeV80,
            &[],
            None,
            CompilationSourcePolicy::default(),
            CompilationSourcePolicy::default(),
            AnalysisWorkObservers {
                legacy: AnalysisObservers {
                    phases,
                    recursive_work: Some(seven),
                },
                adjacent_work: None,
            },
            pool,
        )
    };
    let old = compile(input.clone(), &old_phase, &old_seven, &old_pool);
    let new = compile(input, &new_phase, &new_seven, &new_pool);
    let old_snapshot = old_pool.snapshot();
    let new_snapshot = new_pool.snapshot();
    let topology = new_pool.first_queue_topology_snapshot();
    // Retain BOTH whole actual Results before assertion, including errors.
    println!(
        "FIRST_QUEUE_TOPOLOGY_GENUINE_PREASSERT {}",
        serde_json::json!({
            "old":old,"new":new,"old_pool":format!("{old_snapshot:?}"),
            "new_pool":format!("{new_snapshot:?}"),"topology":format!("{topology:?}")
        })
    );
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(&new).unwrap()
    );
    assert_eq!(old_snapshot, new_snapshot);
    assert_eq!(old_seven.snapshot(), new_seven.snapshot());
    assert_eq!(old_phase.coverage(), new_phase.coverage());
    assert_eq!(old_phase.first_failure(), new_phase.first_failure());
    assert_eq!(
        old_pool.first_queue_topology_snapshot().coverage,
        FirstQueueTopologyCoverage::Disabled
    );
    assert!(topology.enabled && topology.window_ended);
    assert_eq!(
        topology.program_ordinal,
        new_snapshot.last.map(|row| row.ordinal)
    );
    assert!(new_snapshot.programs > 0 && !new_snapshot.active);
    assert!(
        !new_snapshot.inconsistent && !new_snapshot.overflowed && !new_snapshot.capture_unavailable
    );
    let last = new_snapshot.last.unwrap();
    // The default per-pool grant became GrammarAnalysisWorkBudget::REALISTIC in 1e5cd75b.
    assert_eq!(
        last.work.initial,
        GrammarAnalysisWorkBudget::default().units()
    );
    assert!(last.work.ended && last.work.balanced);
    assert_eq!(
        last.work.counts.units.iter().sum::<u64>(),
        (GrammarAnalysisWorkBudget::default().units() - last.work.remaining) as u64
    );
    // These genuine graphs need not offer any completed-interior repetition.
    // Unavailable is explicit; it never licenses cross-queue or cross-program matches.
    assert_eq!(
        topology.coverage == FirstQueueTopologyCoverage::Unavailable,
        topology.unavailable.is_some()
    );
    super::pool_attribution_controls::whole_outcomes(&old.unwrap(), &new.unwrap(), texts);
}
