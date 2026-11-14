use super::GrammarAnalysisWorkBudget;
use crate::grammar;
use crate::grammar::Expr;
use crate::grammar::Grammar;
use crate::grammar::Kind;
use crate::grammar::Profile;
use crate::grammar::RecursiveFrontierOrder;
use crate::grammar::RuntimeGuardedAdmission;
use crate::grammar::RuntimeGuardedPolicy;
use crate::grammar::admission_observation::PredictionObservation;
use crate::grammar::admission_observation::RecursiveAnalysisCounters;
use crate::grammar::admission_observation::RunStage;
use crate::grammar::backend_test_support::inspect_ecore_schema;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use intersect_model::definitions::schema::Schema;

fn fixture() -> (Schema, DefinitionSource, Grammar) {
    let schema = inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:budget"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
    let source = DefinitionSource::new(
        "memory:budget/real.xtext",
        SourceKind::Xtext,
        "grammar Budget hidden() import \"urn:budget\" as t Root returns t::Root: {t::Root} '{' (=> Leaf)* '}'; fragment Leaf returns t::Root: 'x';",
    );
    let (mut grammar, _, spans, _) = grammar::parse_declarations(
        &source.text,
        &schema,
        Profile::NativeV80,
        false,
        None,
        128,
        std::rc::Rc::default(),
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
    (schema, source, grammar)
}

fn run(
    budget: GrammarAnalysisWorkBudget,
    policy: RuntimeGuardedPolicy,
) -> (
    Result<serde_json::Value, String>,
    Option<grammar::admission_observation::FirstReuseWorkSnapshot>,
    usize,
) {
    let (schema, source, mut grammar) = fixture();
    let trace = PredictionObservation::new();
    let work = RecursiveAnalysisCounters::new();
    let observation = grammar::AnalysisObservation {
        observation: &trace,
        recursive_work: Some(&work),
        adjacent_work: None,
        entry: &source.id,
        stage: RunStage::Executable,
    };
    let mut admission = RuntimeGuardedAdmission::new_with_policy_and_budget(policy, budget);
    let result = grammar::analyze_declarations_prediction_policy(
        &mut grammar,
        &schema,
        true,
        Some(observation),
        None,
        RecursiveFrontierOrder::Historical,
        None,
        Some(&mut admission),
    )
    .map_err(|error| error.message)
    .map(|()| serde_json::to_value(grammar.parser_prediction.as_ref().unwrap()).unwrap());
    (result, trace.first_reuse_work(), admission.decision_count())
}

#[test]
fn grammar_analysis_work_budget_default_and_legacy_keep_whole_program_and_real_pool_bases() {
    assert_eq!(GrammarAnalysisWorkBudget::default().units(), 100_000_000);
    assert_eq!(
        GrammarAnalysisWorkBudget::CALIBRATION_10M.units(),
        10_000_000
    );
    assert_eq!(GrammarAnalysisWorkBudget::CALIBRATION_1M.units(), 1_000_000);
    assert_eq!(GrammarAnalysisWorkBudget::LEGACY_10K.units(), 10_000);
    assert_eq!(GrammarAnalysisWorkBudget::new(0).units(), 0);
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let old = run(GrammarAnalysisWorkBudget::LEGACY_10K, policy);
        let current = run(GrammarAnalysisWorkBudget::default(), policy);
        println!(
            "GRAMMAR_ANALYSIS_POLICY_WHOLE {:?}",
            (&old.0, &current.0, old.2, current.2)
        );
        assert_eq!(old.0, current.0);
        assert!(
            current.0.is_ok(),
            "actual whole grammar control must compile"
        );
        assert!(current.2 > 0);
        let a = old.1.unwrap();
        let b = current.1.unwrap();
        assert_eq!(a.precollect_work.unwrap() + a.entry_remaining, 10_000);
        assert_eq!(b.precollect_work.unwrap() + b.entry_remaining, 100_000_000);
        assert!(!a.overflow && !b.overflow);
        assert_eq!(a.collect_work, b.collect_work);
        assert_eq!(a.first_units, b.first_units);
        assert_eq!(a.events, b.events);
        assert!(a.finished && b.finished && !a.returned_error && !b.returned_error);
        let denied = run(GrammarAnalysisWorkBudget::new(0), policy);
        println!("GRAMMAR_ANALYSIS_ZERO_WHOLE {:?}", denied.0);
        assert!(denied.0.is_err());
        assert_eq!(denied.2, 0);
    }
}

#[test]
fn grammar_analysis_work_budget_first_exact_short_unknown_and_depth_are_same_worker() {
    let (_, _, grammar) = fixture();
    let expr = &grammar.rules["Leaf"].body;
    let mut remaining = GrammarAnalysisWorkBudget::default().units();
    let full =
        grammar::first_sets::analyze_with_remaining(expr, &grammar, 0, &mut remaining).unwrap();
    let used = GrammarAnalysisWorkBudget::default().units() - remaining;
    println!("GRAMMAR_ANALYSIS_FIRST_EXACT {:?}", (&full, used));
    assert!(used > 0);
    assert_eq!(
        grammar::first_with_work_budget(expr, &grammar, 0, GrammarAnalysisWorkBudget::new(used))
            .unwrap(),
        full
    );
    assert!(
        grammar::first_with_work_budget(
            expr,
            &grammar,
            0,
            GrammarAnalysisWorkBudget::new(used - 1)
        )
        .is_err()
    );
    assert!(
        grammar::first_with_work_budget(expr, &grammar, 0, GrammarAnalysisWorkBudget::new(0))
            .is_err()
    );
    assert_eq!(
        grammar::first_with_work_budget(expr, &grammar, 0, GrammarAnalysisWorkBudget::LEGACY_10K)
            .unwrap(),
        full
    );
    let missing = Expr {
        kind: Kind::Call("Missing".into()),
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '1',
    };
    assert_eq!(
        grammar::first_with_work_budget(
            &missing,
            &grammar,
            0,
            GrammarAnalysisWorkBudget::LEGACY_10K
        ),
        grammar::first_with_work_budget(
            &missing,
            &grammar,
            0,
            GrammarAnalysisWorkBudget::default()
        )
    );
    let legacy =
        grammar::first_with_work_budget(expr, &grammar, 129, GrammarAnalysisWorkBudget::LEGACY_10K);
    let realistic =
        grammar::first_with_work_budget(expr, &grammar, 129, GrammarAnalysisWorkBudget::default());
    assert_eq!(legacy, realistic);
    assert!(realistic.is_err());
}

#[test]
fn grammar_analysis_work_budget_custom_finite_revalidation_never_mints_source_authority() {
    let (_, _, grammar) = fixture();
    let (whole, _, _) = run(
        GrammarAnalysisWorkBudget::default(),
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    let old: grammar::ParserPrediction = serde_json::from_value(whole.unwrap()).unwrap();
    // Work configuration has no serialized field or altered graph cardinality.
    assert!(
        serde_json::to_value(&old).unwrap()["states"]
            .as_array()
            .unwrap()
            .len()
            < 10_000
    );
    assert_eq!(
        GrammarAnalysisWorkBudget::new(usize::MAX).units(),
        usize::MAX
    );
    let owner = RuntimeGuardedAdmission::new_with_policy_and_budget(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        GrammarAnalysisWorkBudget::new(7),
    );
    assert_eq!(owner.work_budget().units(), 7);
    // Invalid graph/source revalidation remains an error even with a huge grant.
    let huge = RuntimeGuardedAdmission::new_with_policy_and_budget(
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        GrammarAnalysisWorkBudget::new(usize::MAX),
    );
    assert!(
        huge.validate_decision(&grammar, &old, "rule/Missing/repeat")
            .is_err()
    );
}

#[test]
fn grammar_deferred_work_pool_real_parsed_records_share_finite_source_validation() {
    for policy in [
        RuntimeGuardedPolicy::StrictCompletionScopes,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    ] {
        let (schema, _, _) = fixture();
        let source = DefinitionSource::new(
            "memory:budget/deferred-pair.xtext",
            SourceKind::Xtext,
            "grammar Pair hidden() import \"urn:budget\" as t Root returns t::Root: {t::Root} '{' (=> Leaf)* (=> Other)* '}'; fragment Leaf returns t::Root: 'x'; fragment Other returns t::Root: 'y';",
        );
        let (mut g, _, spans, _) = grammar::parse_declarations(
            &source.text,
            &schema,
            Profile::NativeV80,
            false,
            None,
            128,
            std::rc::Rc::default(),
        )
        .unwrap();
        g.declaration_sources = spans
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
        let mut admission = RuntimeGuardedAdmission::new_with_policy_and_budget(
            policy,
            GrammarAnalysisWorkBudget::default(),
        );
        grammar::analyze_declarations_prediction_policy(
            &mut g,
            &schema,
            true,
            None,
            None,
            RecursiveFrontierOrder::Historical,
            None,
            Some(&mut admission),
        )
        .map_err(|error| error.message)
        .unwrap();
        let program = g.parser_prediction.as_ref().unwrap();
        let paths: Vec<_> = program
            .decisions()
            .filter(|path| admission.contains(path))
            .collect();
        assert_eq!(paths.len(), 2);
        let before = serde_json::to_vec(program).unwrap();
        let mut remaining = 100000;
        for path in &paths {
            admission
                .validate_decision_with_remaining(&g, program, path, &mut remaining)
                .unwrap();
        }
        let used = 100000 - remaining;
        println!("DEFERRED_REAL_RECORDS policy={policy:?} paths={paths:?} used={used}");
        assert!(used > 0);
        for grant in [0, used - 1, used] {
            let mut remaining = grant;
            let result = paths.iter().try_for_each(|path| {
                admission.validate_decision_with_remaining(&g, program, path, &mut remaining)
            });
            if grant == used {
                result.unwrap();
            } else {
                assert!(result.unwrap_err().contains("budget"));
            }
            assert_eq!(remaining, 0);
        }
        let mut first_remaining = 100000;
        admission
            .validate_decision_with_remaining(&g, program, paths[0], &mut first_remaining)
            .unwrap();
        let first_used = 100000 - first_remaining;
        let mut preconsumed = used + first_used;
        admission
            .validate_decision_with_remaining(&g, program, paths[0], &mut preconsumed)
            .unwrap();
        for path in &paths {
            admission
                .validate_decision_with_remaining(&g, program, path, &mut preconsumed)
                .unwrap();
        }
        assert_eq!(preconsumed, 0);
        let mut foreign = g.clone();
        foreign
            .declaration_sources
            .get_mut("Root")
            .unwrap()
            .source
            .sha256 = "foreign".into();
        let mut remaining = 100000;
        assert!(
            admission
                .validate_decision_with_remaining(&foreign, program, paths[0], &mut remaining)
                .is_err()
        );
        assert!(
            remaining > 0,
            "source mismatch must not be replaced by pool exhaustion"
        );
        assert_eq!(serde_json::to_vec(program).unwrap(), before);
    }
}
