use super::*;
use crate::grammar;
use crate::grammar::Kind;
use crate::grammar::Profile;
use crate::grammar::backend_test_support::inspect_ecore_schema;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use std::rc::Rc;

fn fixture() -> (DefinitionSource, Grammar) {
    let schema = inspect_ecore_schema(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:follow-prefix"><eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="children" upperBound="-1" containment="true" eType="#//Root"/></eClassifiers></e:EPackage>"##).unwrap();
    let source = DefinitionSource::new(
        "memory://follow-prefix/actual.xtext",
        SourceKind::Xtext,
        "grammar Prefix hidden() import \"urn:follow-prefix\" as t Root returns t::Root: 'r' (=>children+=Target)* 'end'; Target returns t::Root: {t::Root} 'x'; Unused returns t::Root: 'z';",
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
        .unwrap_or_else(|error| panic!("Terminal preparation failed: {}", error.message));
    (source, grammar)
}
fn selected(grammar: &Grammar) -> &Expr {
    let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
        panic!("Expected actual parsed sequence");
    };
    &items[1]
}
fn probe(callee: &str) -> SourcePrefixDiagnostic<'_> {
    SourcePrefixDiagnostic::new("Root", "rule/Root/s1", callee)
}
fn strings(values: &[Option<PrefixDiagnosticText<256>>; 16]) -> Vec<&str> {
    values
        .iter()
        .flatten()
        .map(|value| value.as_str().unwrap())
        .collect()
}

#[test]
fn follow_prefix_diagnostic_selected_source_replaces_first_and_moves_whole_summary() {
    let (source, grammar) = fixture();
    let before = serde_json::to_vec(&grammar).unwrap();
    let expression = selected(&grammar);
    let mut old_remaining = 10_000;
    let expected =
        grammar::first_sets::analyze_with_remaining(expression, &grammar, 0, &mut old_remaining)
            .unwrap();
    let diagnostic = probe("Target");
    let mut remaining = 10_000;
    let (summary, occurrence) = diagnostic
        .analyze(
            &grammar,
            expression,
            Some("Root"),
            "rule/Root/s1",
            &mut remaining,
        )
        .unwrap();
    assert_eq!(summary, expected);
    assert_eq!(summary, (["literal:x".into()].into(), true));
    assert_eq!(occurrence.body_nullable, Some(false));
    assert!(!occurrence.explicit_prefix_hidden_policy);
    let report = diagnostic.record().unwrap();
    println!("FOLLOW_PREFIX_ACTUAL_COMPONENT {report:?}");
    assert_eq!(report.state, PrefixDiagnosticState::Complete);
    assert_eq!(report.absence, PrefixDiagnosticAbsence::None);
    assert_eq!(report.owner.unwrap().as_str().unwrap(), "Root");
    assert_eq!(report.path.unwrap().as_str().unwrap(), "rule/Root/s1");
    assert_eq!(report.source.unwrap().uri.as_str().unwrap(), source.id.uri);
    assert_eq!(
        report.source.unwrap().sha256.as_str().unwrap(),
        source.id.sha256
    );
    assert_eq!(
        report.source.unwrap().declaration,
        (
            grammar.declaration_sources["Root"].start,
            grammar.declaration_sources["Root"].end
        )
    );
    assert_eq!(
        report.source.unwrap().body,
        (
            expression.original_range.as_ref().unwrap().start,
            expression.original_range.as_ref().unwrap().end
        )
    );
    assert_eq!(report.nullable, Some(true));
    assert_eq!(report.body_nullable, Some(false));
    assert_eq!(strings(&report.first), ["literal:x"]);
    assert_eq!(report.first_count, 1);
    assert_eq!(report.callee.unwrap().as_str().unwrap(), "Target");
    assert_eq!(
        report.callee_source.unwrap().declaration,
        (
            grammar.declaration_sources["Target"].start,
            grammar.declaration_sources["Target"].end
        )
    );
    assert_eq!(report.callee_nullable, Some(false));
    assert_eq!(strings(&report.callee_first), ["literal:x"]);
    assert_eq!(report.callee_first_count, 1);
    assert_eq!(report.maximum_source_call_depth, Some(1));
    assert_eq!(report.original_first_remaining, remaining);
    assert!(remaining < old_remaining); // New capture work is not free.
    assert_eq!(serde_json::to_vec(&grammar).unwrap(), before);
    assert!(!diagnostic.selects(Some("Root"), "rule/Root/s1"));
    assert!(std::mem::size_of::<SourcePrefixDiagnosticRecord>() <= 16_384);
    assert!(diagnostic.begin().is_ok());
    assert_eq!(
        diagnostic.begin().unwrap_err(),
        "Source prefix diagnostic belongs to a previous compilation invocation"
    );
}

#[test]
fn follow_prefix_diagnostic_same_follow_worker_none_foreign_and_selected_scope() {
    let (_, grammar) = fixture();
    let before = serde_json::to_vec(&grammar).unwrap();
    let mut old_deferred = BTreeSet::new();
    let (old_required, old_remaining, _) =
        grammar::follow::check_required_calls_certified(&grammar, &mut old_deferred, None).unwrap();
    let mut absent_deferred = BTreeSet::new();
    let (absent_required, absent_remaining, _) =
        grammar::follow::check_required_calls_certified_prefix_inspected(
            &grammar,
            &mut absent_deferred,
            None,
            None,
        )
        .unwrap();
    assert_eq!(absent_remaining, old_remaining);
    assert_eq!(absent_required.len(), old_required.len());
    assert_eq!(absent_deferred, old_deferred);
    let not_selected = SourcePrefixDiagnostic::new("Other", "rule/Root/s1", "Target");
    let mut foreign_deferred = BTreeSet::new();
    let (_, foreign_remaining, _) =
        grammar::follow::check_required_calls_certified_prefix_inspected(
            &grammar,
            &mut foreign_deferred,
            None,
            Some(&not_selected),
        )
        .unwrap();
    assert_eq!(foreign_remaining, old_remaining);
    assert_eq!(foreign_deferred, old_deferred);
    assert!(not_selected.record().is_none());
    let diagnostic = probe("Target");
    let mut observed_deferred = BTreeSet::new();
    let (observed_required, observed_remaining, _) =
        grammar::follow::check_required_calls_certified_prefix_inspected(
            &grammar,
            &mut observed_deferred,
            None,
            Some(&diagnostic),
        )
        .unwrap();
    // New work belongs to the independently bounded FIRST query. Shared FOLLOW
    // transfer, fixed point, replay and certificate work is unchanged.
    assert_eq!(observed_remaining, old_remaining);
    assert_eq!(observed_required.len(), old_required.len());
    assert_eq!(observed_deferred, old_deferred);
    assert_eq!(
        diagnostic.record().unwrap().state,
        PrefixDiagnosticState::Complete
    );
    assert_eq!(serde_json::to_vec(&grammar).unwrap(), before);
}

#[test]
fn follow_prefix_diagnostic_original_pool_zero_exact_short_and_actual_denial() {
    let (_, grammar) = fixture();
    let expression = selected(&grammar);
    let mut generous = 10_000;
    let diagnostic = probe("Target");
    let expected = diagnostic
        .analyze(
            &grammar,
            expression,
            Some("Root"),
            "rule/Root/s1",
            &mut generous,
        )
        .unwrap();
    let used = 10_000 - generous;
    for allowance in 0..=used {
        let diagnostic = probe("Target");
        let mut remaining = allowance;
        let actual = diagnostic.analyze(
            &grammar,
            expression,
            Some("Root"),
            "rule/Root/s1",
            &mut remaining,
        );
        if allowance == used {
            assert_eq!(actual.unwrap(), expected);
            assert_eq!(remaining, 0);
            assert_eq!(
                diagnostic.record().unwrap().state,
                PrefixDiagnosticState::Complete
            );
        } else {
            assert_eq!(actual.unwrap_err(), "Grammar prediction budget exceeded");
            assert_eq!(remaining, 0);
            let report = diagnostic.record().unwrap();
            assert_eq!(report.state, PrefixDiagnosticState::Denied);
            assert_eq!(
                report.error.unwrap().as_str().unwrap(),
                "Grammar prediction budget exceeded"
            );
            assert!(report.source.is_none()); // No inferred identity after denial.
            assert!(report.callee.is_none());
            assert_eq!(report.original_first_remaining, 0);
        }
    }
    println!("FOLLOW_PREFIX_CAPTURE_FEE observed_after_runtime={used}");
}

#[test]
fn follow_prefix_diagnostic_canonical_pointer_absent_callee_and_fixed_identity_bound() {
    let (source, grammar) = fixture();
    let copied = selected(&grammar).clone();
    let diagnostic = probe("Target");
    let mut remaining = 10_000;
    assert_eq!(
        diagnostic
            .analyze(
                &grammar,
                &copied,
                Some("Root"),
                "rule/Root/s1",
                &mut remaining
            )
            .unwrap_err(),
        "Selected FIRST diagnostic expression differs from canonical source"
    );
    assert_eq!(
        diagnostic.record().unwrap().state,
        PrefixDiagnosticState::Unavailable
    );
    assert!(diagnostic.record().unwrap().source.is_none());
    let diagnostic = probe("Unused");
    let mut remaining = 10_000;
    let (summary, _) = diagnostic
        .analyze(
            &grammar,
            selected(&grammar),
            Some("Root"),
            "rule/Root/s1",
            &mut remaining,
        )
        .unwrap();
    assert_eq!(summary, (["literal:x".into()].into(), true));
    let report = diagnostic.record().unwrap();
    assert_eq!(report.state, PrefixDiagnosticState::Unavailable);
    assert_eq!(
        report.absence,
        PrefixDiagnosticAbsence::RequestedCalleeNotTraversed
    );
    assert!(report.callee.is_none());
    let mut grammar = grammar;
    for declaration in grammar.declaration_sources.values_mut() {
        declaration.source.uri = format!("memory://{}", "a".repeat(600));
    }
    let diagnostic = probe("Target");
    let mut remaining = 10_000;
    let (summary, _) = diagnostic
        .analyze(
            &grammar,
            selected(&grammar),
            Some("Root"),
            "rule/Root/s1",
            &mut remaining,
        )
        .unwrap();
    assert_eq!(summary, (["literal:x".into()].into(), true));
    let report = diagnostic.record().unwrap();
    assert_eq!(report.state, PrefixDiagnosticState::Unavailable);
    assert_eq!(
        report.absence,
        PrefixDiagnosticAbsence::FixedCaptureCapacity
    );
    assert!(report.capture_unavailable);
    assert!(report.source.is_none());
    assert!(report.callee_source.is_none());
    assert_ne!(grammar.declaration_sources["Root"].source, source.id);
}

#[test]
fn follow_prefix_diagnostic_owner_policy_does_not_change_original_worker_facts() {
    let (_, mut grammar) = fixture();
    // Adversarial retained-IR metadata control, not source/model admission.
    grammar.rules.get_mut("Root").unwrap().hidden = Some(vec![]);
    let expression = selected(&grammar);
    let (expected, original_facts) =
        grammar::first_sets::analyze_occurrence(expression, &grammar, 0, 10_000).unwrap();
    assert!(!original_facts.explicit_prefix_hidden_policy);
    let diagnostic = probe("Target");
    let mut remaining = 10_000;
    let (summary, returned) = diagnostic
        .analyze(
            &grammar,
            expression,
            Some("Root"),
            "rule/Root/s1",
            &mut remaining,
        )
        .unwrap();
    assert_eq!(summary, expected);
    assert_eq!(returned, original_facts);
    assert_eq!(
        diagnostic.record().unwrap().prefix_hidden_policy,
        Some(true)
    );
}

#[test]
fn follow_prefix_diagnostic_nonzero_ceiling_error_is_not_a_work_denial() {
    let schema = inspect_ecore_schema(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:follow-prefix"><eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="children" upperBound="-1" containment="true" eType="#//Root"/></eClassifiers></e:EPackage>"##).unwrap();
    let source = DefinitionSource::new(
        "memory://follow-prefix/left-recursive.xtext",
        SourceKind::Xtext,
        "grammar Prefix hidden() import \"urn:follow-prefix\" as t Root returns t::Root: 'r' (=>children+=Target)* 'end'; Target returns t::Root: Target;",
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
    let diagnostic = probe("Target");
    let mut remaining = 10_000;
    assert_eq!(
        diagnostic
            .analyze(
                &grammar,
                selected(&grammar),
                Some("Root"),
                "rule/Root/s1",
                &mut remaining
            )
            .unwrap_err(),
        "Grammar prediction budget exceeded"
    );
    assert!(remaining > 0); // Existing call ceiling, not depleted work.
    let record = diagnostic.record().unwrap();
    assert_eq!(record.state, PrefixDiagnosticState::Unavailable);
    assert_eq!(record.absence, PrefixDiagnosticAbsence::FirstFailure);
    assert_eq!(record.original_first_remaining, remaining);
    assert_eq!(
        record.error.unwrap().as_str().unwrap(),
        "Grammar prediction budget exceeded"
    );
    assert!(record.source.is_none());
    assert!(record.callee.is_none());
}
