//! Ordinary source/validation path, actual compiler, full DTO and old counters.
use intersect_text::grammar::GrammarAnalysisWorkBudget;
use intersect_text::grammar::Profile;
use intersect_text::grammar::RecursiveFrontierOrder;
use intersect_text::grammar::admission_observation::AnalysisObservers;
use intersect_text::grammar::admission_observation::AnalysisWorkObservers;
use intersect_text::grammar::admission_observation::PredictionObservation;
use intersect_text::grammar::admission_observation::RecursiveAnalysisCounters;
use intersect_text::grammar::admission_observation::RunStage;
use intersect_text::grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy, Package};
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    evaluate_with_limits, ordinary,
};

fn observers<'a>(
    phases: &'a PredictionObservation,
    recursive_work: &'a RecursiveAnalysisCounters,
) -> AnalysisWorkObservers<'a> {
    AnalysisWorkObservers {
        legacy: AnalysisObservers {
            phases,
            recursive_work: Some(recursive_work),
        },
        adjacent_work: None,
    }
}

pub fn compile_pair(input: BundleCompilation) -> (Package, Package) {
    let source = input.xtext.clone().unwrap();
    let old_phase = PredictionObservation::new();
    let new_phase = PredictionObservation::new();
    let old_seven = RecursiveAnalysisCounters::new();
    let new_seven = RecursiveAnalysisCounters::new();

    let old = ordinary::compile_with_context_frontier_selected(
        input.clone(),
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        Some(observers(&old_phase, &old_seven)),
        RecursiveFrontierOrder::BreadthFirstTransparentPorts,
    );
    let pool = CompilerPoolDiagnostic::new();
    let new = ordinary::compile_with_context_pool_observed(
        input,
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        observers(&new_phase, &new_seven),
        &pool,
    );
    let report = pool.snapshot();
    println!(
        "COMPILER_POOL_SOURCE_PREASSERT {}",
        serde_json::json!({
            "source":source,"plain":old,"observed":new,"programs":report.programs,
            "active":report.active,"capture_unavailable":report.capture_unavailable,
            "inconsistent":report.inconsistent,"overflowed":report.overflowed,
            "last":report.last.map(|p| serde_json::json!({
                "ordinal":p.ordinal,"stage":format!("{:?}",p.stage),
                "source_uri":p.source.and_then(|s| s.uri.as_str().ok().map(str::to_owned)),
                "source_sha256":p.source.and_then(|s| s.sha256.as_str().ok().map(str::to_owned)),
                "decision":p.decision.and_then(|s| s.as_str().ok().map(str::to_owned)),
                "initial":p.work.initial,"remaining":p.work.remaining,"balanced":p.work.balanced,
                "units":p.work.counts.units,"actual_denied":p.work.counts.actual_denied_leaf_attempts,
                "queue_share_inclusive":p.work.queue_share_inclusive
            }))
        })
    );
    assert_eq!(
        serde_json::to_value(&new).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    assert_eq!(new_seven.snapshot(), old_seven.snapshot());
    assert_eq!(new_phase.coverage(), old_phase.coverage());
    assert_eq!(new_phase.first_failure(), old_phase.first_failure());
    assert!(report.programs > 0 && !report.active && !report.capture_unavailable);
    assert!(!report.inconsistent && !report.overflowed);
    let last = report.last.unwrap();
    assert_eq!(last.stage, RunStage::Executable);
    assert_eq!(
        last.order,
        RecursiveFrontierOrder::BreadthFirstTransparentPorts
    );
    let captured = last.source.unwrap();
    assert_eq!(captured.uri.as_str().unwrap(), source.uri);
    assert_eq!(captured.sha256.as_str().unwrap(), source.sha256);
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
    (old.unwrap(), new.unwrap())
}

pub fn whole_outcomes(plain: &Package, observed: &Package, texts: &[&str]) {
    let a =
        DocumentLanguage::prepare(plain, Default::default(), AnalysisLimits::workbench()).unwrap();
    let b = DocumentLanguage::prepare(observed, Default::default(), AnalysisLimits::workbench())
        .unwrap();
    for (index, text) in texts.iter().enumerate() {
        let source = Source {
            uri: format!("memory:pool-source-{index}.model"),
            text: (*text).into(),
        };
        let x = evaluate_with_limits(
            plain,
            std::slice::from_ref(&source),
            EvaluationLimits::workbench(),
        );
        let y = evaluate_with_limits(
            observed,
            std::slice::from_ref(&source),
            EvaluationLimits::workbench(),
        );
        let dx = a.analyze(&source, None, EvaluationLimits::workbench());
        let dy = b.analyze(&source, None, EvaluationLimits::workbench());
        println!(
            "COMPILER_POOL_SOURCE_OUTCOME_PREASSERT {}",
            serde_json::json!({
                "source":source,"plain":x,"observed":y,"plain_document":dx,"observed_document":dy
            })
        );
        assert_eq!(
            serde_json::to_value(&x).unwrap(),
            serde_json::to_value(&y).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&dx).unwrap(),
            serde_json::to_value(&dy).unwrap()
        );
    }
}
