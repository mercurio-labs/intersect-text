//! Controls for the explicit generic policy boundary; no domain scoping claim.
use super::*;
use crate::linking::{LinkingContext, ReferenceCandidates, ReferenceRequest, UniqueNames};
use intersect_model::core::query::QueryBudget;
use std::cell::RefCell;

const GOOD: &str = "library L { copy C \"title\"; member M; loan Q copy C to M; }";
fn package() -> Package {
    compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap()
}
fn sources(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "memory:linking.library".into(),
        text: text.into(),
    }]
}
fn observed(result: &EvaluatedStore) -> Value {
    json!({"model": storage::export(&result.store).unwrap(), "evaluation": result.evaluation})
}

#[test]
fn frozen_text_evaluation_preserves_models_and_diagnostics() -> Result<(), String> {
    use intersect_model::core::frozen::{FrozenStore, Metamodel};
    use std::sync::Arc;

    let package = package();
    let metamodel =
        Arc::new(Metamodel::freeze(&package.schema).map_err(|error| error.to_string())?);
    for text in [
        GOOD,
        "library L { copy C \"title\"; member M; loan Q copy Missing to M; }",
        "library L { copy C \"one\"; copy C \"two\"; member M; loan Q copy C to M; }",
        "library L {",
    ] {
        let mut inputs = sources(text);
        inputs.push(Source {
            uri: "memory:second.library".into(),
            text: "library Separate { copy Other \"second\"; }".into(),
        });
        let policy = evaluation_index_policy(&package);
        let mut indexed =
            FrozenStore::with_index_policy(metamodel.clone(), storage::IndexPolicy::AllValues);
        let mut frozen = FrozenStore::with_index_policy(metamodel.clone(), policy);
        let expected = evaluate_on(&package, &inputs, &mut indexed)?;
        let actual = evaluate_on(&package, &inputs, &mut frozen)?;
        assert_eq!(actual, expected, "{text}");
    }
    Ok(())
}

fn evaluate_on(
    package: &Package,
    inputs: &[Source],
    store: &mut impl ModelStore,
) -> Result<(Model, Value), String> {
    let output = storage::atomic(store, |store| {
        evaluate_staged(
            package,
            inputs,
            Invocation::resource(EvaluationLimits::native()),
            store,
            None,
            None,
            &query::PackageDerived(package),
        )
    })?;
    let output = serde_json::to_value(output).map_err(|error| error.to_string())?;
    Ok((storage::export(store)?, output))
}

fn run(
    package: &Package,
    text: &str,
    policy: &dyn ReferenceCandidates,
    work: &mut QueryBudget,
) -> Result<EvaluatedStore, String> {
    evaluate_stored_with_linking_context(
        package,
        &sources(text),
        EvaluationLimits::native(),
        None,
        None,
        &query::PackageDerived(package),
        &mut LinkingContext {
            resolver: policy,
            work,
        },
    )
}

#[test]
fn explicit_unique_names_matches_complete_original_success_and_diagnostics() {
    let package = package();
    for text in [
        GOOD,
        "library L { copy C \"title\"; member M; loan Q copy Missing to M; }",
        "library L { copy C \"one\"; copy C \"two\"; member M; loan Q copy C to M; }",
        "library L {",
    ] {
        let before = evaluate_stored(&package, &sources(text), EvaluationLimits::native()).unwrap();
        let mut work = QueryBudget::new(100);
        let after = run(&package, text, &UniqueNames, &mut work).unwrap();
        assert_eq!(observed(&after), observed(&before), "{text}");
    }
}

#[derive(Default)]
struct Aliases {
    calls: RefCell<Vec<Value>>,
}
impl ReferenceCandidates for Aliases {
    fn validate_definition(
        &self,
        package: crate::package::TextPackageView<'_>,
    ) -> Result<(), String> {
        UniqueNames.validate_definition(package)
    }
    fn candidates(
        &self,
        store: &dyn ModelRead,
        request: &ReferenceRequest<'_>,
        work: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        self.calls
            .borrow_mut()
            .push(json!({"owner": request.owner, "feature": request.feature,
            "spelling": request.spelling, "target_class": request.target_class,
            "uri": request.uri, "start": request.start, "end": request.end}));
        work.charge()?;
        let name = match request.spelling {
            "AliasC" => "C",
            "AliasM" => "M",
            other => other,
        };
        store.query(Query::Value("name", &json!(name)))
    }
}
#[test]
fn explicit_policy_receives_exact_reference_spans_and_shared_linker_sets_targets() {
    let package = package();
    let text = GOOD.replace("copy C to M", "copy AliasC to AliasM");
    let rejected = evaluate_stored(&package, &sources(&text), EvaluationLimits::native()).unwrap();
    assert_eq!(rejected.evaluation.diagnostics.len(), 2);
    let policy = Aliases::default();
    let mut work = QueryBudget::new(6);
    let actual = run(&package, &text, &policy, &mut work).unwrap();
    assert!(
        actual.evaluation.diagnostics.is_empty(),
        "{}",
        json!(actual.evaluation.diagnostics)
    );
    assert_eq!(work.reads, 6);
    let calls = policy.calls.borrow();
    assert_eq!(calls.len(), 2);
    for (index, (spelling, feature, class)) in [
        ("AliasC", "copy", "BookCopy"),
        ("AliasM", "member", "Member"),
    ]
    .into_iter()
    .enumerate()
    {
        let start = text.find(spelling).unwrap();
        assert_eq!(
            calls[index],
            json!({"owner": "memory:linking.library#3", "feature": feature,
            "spelling": spelling, "target_class": class, "uri": "memory:linking.library",
            "start": start, "end": start + spelling.len()})
        );
        let reference = &actual.evaluation.references[index];
        assert_eq!(
            (reference.start, reference.end),
            (start, start + spelling.len())
        );
        let target = actual.store.get(&reference.target).unwrap().unwrap();
        assert_eq!(target.class, class);
        assert_eq!(
            target.values.get("name"),
            Some(&json!(if index == 0 { "C" } else { "M" }))
        );
    }
    let owner = actual
        .store
        .get("memory:linking.library#3")
        .unwrap()
        .unwrap();
    assert_eq!(owner.class, "Loan");
    assert_eq!(owner.values.get("name"), Some(&json!("Q")));
}

struct Empty;
impl ReferenceCandidates for Empty {
    fn validate_definition(&self, _: crate::package::TextPackageView<'_>) -> Result<(), String> {
        Ok(())
    }
    fn candidates(
        &self,
        _: &dyn ModelRead,
        _: &ReferenceRequest<'_>,
        _: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
}
#[test]
fn empty_provider_has_no_global_fallback_and_keeps_source_diagnostics() {
    let package = package();
    let actual = run(&package, GOOD, &Empty, &mut QueryBudget::new(2)).unwrap();
    assert_eq!(actual.evaluation.references.len(), 0);
    let c = GOOD.rfind("C to").unwrap();
    let m = GOOD.rfind("M;").unwrap();
    assert_eq!(
        json!(actual.evaluation.diagnostics),
        json!([
            Diagnostic::new(
                "memory:linking.library",
                c,
                c + 1,
                "Unresolved reference 'C' (BookCopy)"
            ),
            Diagnostic::new(
                "memory:linking.library",
                m,
                m + 1,
                "Unresolved reference 'M' (Member)"
            ),
        ])
    );
}

struct WrongClass;
impl ReferenceCandidates for WrongClass {
    fn validate_definition(&self, _: crate::package::TextPackageView<'_>) -> Result<(), String> {
        Ok(())
    }
    fn candidates(
        &self,
        store: &dyn ModelRead,
        _: &ReferenceRequest<'_>,
        work: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        work.charge()?;
        store.query(Query::Class("Member"))
    }
}
#[test]
fn shared_linker_filters_wrong_classes_and_preserves_ambiguity() {
    let package = package();
    let text = GOOD.replace("member M;", "member M; member N;");
    let result = run(&package, &text, &WrongClass, &mut QueryBudget::new(8)).unwrap();
    let c = text.rfind("C to").unwrap();
    let m = text.rfind("M;").unwrap();
    assert_eq!(
        json!(result.evaluation.diagnostics),
        json!([
            Diagnostic::new(
                "memory:linking.library",
                c,
                c + 1,
                "Unresolved reference 'C' (BookCopy)"
            ),
            Diagnostic::new(
                "memory:linking.library",
                m,
                m + 1,
                "Ambiguous reference 'M' (Member)"
            ),
        ])
    );
}

struct RejectPackage;
impl ReferenceCandidates for RejectPackage {
    fn validate_definition(&self, _: crate::package::TextPackageView<'_>) -> Result<(), String> {
        Err("unqualified language context".into())
    }
    fn candidates(
        &self,
        _: &dyn ModelRead,
        _: &ReferenceRequest<'_>,
        _: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        panic!("preflight must reject")
    }
}
struct RejectLookup;
impl ReferenceCandidates for RejectLookup {
    fn validate_definition(&self, _: crate::package::TextPackageView<'_>) -> Result<(), String> {
        Ok(())
    }
    fn candidates(
        &self,
        _: &dyn ModelRead,
        _: &ReferenceRequest<'_>,
        _: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        Err("language lookup rejected".into())
    }
}
#[test]
fn budget_and_provider_failures_return_no_partial_result_and_next_call_recovers() {
    let package = package();
    let mut short = QueryBudget::new(5);
    assert_eq!(
        run(&package, GOOD, &UniqueNames, &mut short)
            .err()
            .as_deref(),
        Some("Query work budget exhausted; no complete result")
    );
    assert_eq!(short.reads, 5);
    let mut work = QueryBudget::new(6);
    let complete = run(&package, GOOD, &UniqueNames, &mut work).unwrap();
    assert_eq!(work.reads, 6);
    let original = evaluate_stored(&package, &sources(GOOD), EvaluationLimits::native()).unwrap();
    assert_eq!(observed(&complete), observed(&original));
    assert_eq!(
        run(
            &package,
            "invalid source",
            &RejectPackage,
            &mut QueryBudget::new(6)
        )
        .err()
        .as_deref(),
        Some("unqualified language context")
    );
    assert_eq!(
        run(&package, GOOD, &RejectLookup, &mut QueryBudget::new(6))
            .err()
            .as_deref(),
        Some("language lookup rejected")
    );
    let mut wrong = package.clone();
    wrong.semantics.scope = "other".into();
    assert_eq!(
        run(&wrong, GOOD, &UniqueNames, &mut QueryBudget::new(6))
            .err()
            .as_deref(),
        Some("UniqueNames requires the package's unique_names scope")
    );
    assert_eq!(
        observed(&run(&package, GOOD, &UniqueNames, &mut QueryBudget::new(6)).unwrap()),
        observed(&original)
    );
}
