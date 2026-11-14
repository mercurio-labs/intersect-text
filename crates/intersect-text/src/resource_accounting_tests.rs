//! Authored accounting components over the same real parser, not KerML support.
use super::*;
use crate::package::{BundleCompilation, CompilationSourcePolicy};
use intersect_model::core::query::QueryBudget;
use intersect_model::frozen::FrozenStore;
use resource_accounting::SharedResourceAccounting;

fn fixture() -> Package {
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let migrated = intersect_model::migration::migrate_ecore(
        DefinitionSource::new(
            "memory:oracle.ecore",
            SourceKind::Ecore,
            include_str!("../assets/fixtures/library/library.ecore"),
        ),
        "memory:fixture.rcore",
    )
    .unwrap()
    .converted
    .unwrap();
    // Same exact canonical source envelope as the historical fixture helper.
    // V67 must enter the ordinary frontend for fresh authored-source validation;
    // constructing the envelope does not compile under an earlier profile.
    let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, migrated.text);
    let mut parent = DefinitionSource::new(
        "org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        include_str!(
            "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    );
    parent.dependencies.push(model.id.clone());
    let mut grammar = DefinitionSource::new(
        "memory:language.xtext",
        SourceKind::Xtext,
        include_str!("../assets/fixtures/library/library.xtext"),
    );
    grammar.dependencies.push(model.id.clone());
    grammar.dependencies.push(parent.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:behavior.rcore",
        SourceKind::RcoreBehavior,
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    );
    behavior.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: intersect_model::definitions::rcore::declared_namespace(&model).unwrap(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: behavior.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, parent, grammar, behavior],
            namespaces: vec![],
        },
    };
    let package = ordinary::compile_with_context(
        input,
        grammar::Profile::NativeV67,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
    )
    .unwrap();
    assert_eq!(
        package.grammar.as_ref().unwrap().profile,
        grammar::Profile::NativeV67
    );
    assert!(
        package
            .grammar
            .as_ref()
            .unwrap()
            .lexical_prediction
            .is_some()
    );
    package
}

fn sources() -> Vec<Source> {
    vec![
        Source {
            uri: "memory://resource-budget/first.lib".into(),
            text: "library First {}".into(),
        },
        Source {
            uri: "memory://resource-budget/second.lib".into(),
            text: "library Second { member Alice; }".into(),
        },
    ]
}

struct Run {
    whole: Value,
    work: usize,
    rollback: bool,
}

fn run<C: draft::ParseContext<FrozenStore>>(
    context: &C,
    input: &[Source],
    limits: EvaluationLimits,
    shared: Option<&mut QueryBudget>,
) -> Run {
    let mut store = FrozenStore::for_schema(context.schema()).unwrap();
    let before = storage::export(&store).unwrap();
    let mut output = StoredEvaluation::default();
    let mut invocation = Invocation::resource(limits);
    let mut work = 0;
    let mut references = Vec::new();
    let mut parsed = Vec::new();
    let entry = &context.grammar().unwrap().entry;
    let result = storage::atomic(&mut store, |store| {
        storage::bind_schema(store, context.schema())?;
        store.configure_schema(context.schema())?;
        let (pending, resources) = match shared {
            Some(budget) => parse_resources_accounted(
                context,
                input,
                store,
                limits,
                &mut invocation,
                None,
                None,
                None,
                entry,
                &mut output,
                Some(&mut work),
                &mut SharedResourceAccounting::new(budget),
            ),
            None => parse_resources(
                context,
                input,
                store,
                limits,
                &mut invocation,
                None,
                None,
                None,
                entry,
                &mut output,
                Some(&mut work),
            ),
        }?;
        references = pending
            .into_iter()
            .map(|p| {
                json!({"owner":p.owner,"field":p.field,"spelling":p.spelling,
                "target":p.target,"uri":p.uri,"start":p.start,"end":p.end,
                "index":p.index,"active":p.active})
            })
            .collect();
        parsed = resources.into_iter().map(str::to_owned).collect();
        if !output.diagnostics.is_empty() {
            return Err("Authored control rejects parse diagnostics".into());
        }
        Ok(())
    });
    let model = storage::export(&store).unwrap();
    let rollback = model == before;
    Run {
        whole: json!({"result":result,"model":model,"output":output,
            "pending":references,"parsed":parsed}),
        work,
        rollback,
    }
}

fn per_resource_work(package: &Package, input: &[Source]) -> Vec<usize> {
    input
        .iter()
        .map(|s| {
            let r = run(
                package,
                std::slice::from_ref(s),
                EvaluationLimits::workbench(),
                None,
            );
            assert_eq!(r.whole["result"], json!({"Ok":null}), "{}", r.whole);
            assert!(r.work > 0);
            r.work
        })
        .collect()
}

#[test]
fn two_resources_keep_per_resource_caps_and_debit_one_preconsumed_caller() {
    let package = fixture();
    let input = sources();
    let units = per_resource_work(&package, &input);
    let sum = units.iter().sum::<usize>();
    let per_resource = *units.iter().max().unwrap();
    assert!(sum > per_resource);
    let limits = EvaluationLimits {
        steps_per_resource: Some(per_resource),
        ..EvaluationLimits::workbench()
    };
    // Existing API permits this: its step limit is per resource, not a total.
    let plain = run(&package, &input, limits, None);
    assert_eq!(plain.whole["result"], json!({"Ok":null}));
    assert_eq!(plain.work, units[1]); // Preserve original reporting exactly.
    let mut caller = QueryBudget::new(sum + 7);
    for _ in 0..7 {
        caller.charge().unwrap();
    }
    let scoped = run(&package, &input, limits, Some(&mut caller));
    assert_eq!(scoped.whole, plain.whole);
    assert_eq!(scoped.work, sum);
    assert_eq!(caller.reads, sum + 7);
    assert_eq!(caller.remaining(), 0);
    println!(
        "RESOURCE_ACCOUNTING {}",
        json!({"per_resource":units,"sum":sum,
        "preconsumed":7,"caller":caller.reads,"default_work":plain.work})
    );
}

#[test]
fn two_resources_exact_short_zero_rollback_and_fresh_recovery() {
    let package = fixture();
    let input = sources();
    let sum = per_resource_work(&package, &input).iter().sum::<usize>();
    let limits = EvaluationLimits::workbench();
    let mut exact = QueryBudget::new(sum);
    let complete = run(&package, &input, limits, Some(&mut exact));
    assert_eq!(complete.whole["result"], json!({"Ok":null}));
    assert_eq!(complete.work, sum);
    assert_eq!(exact.reads, sum);
    let mut short = QueryBudget::new(sum - 1);
    let failed = run(&package, &input, limits, Some(&mut short));
    assert!(failed.whole["result"].get("Err").is_some());
    assert!(failed.rollback);
    assert_eq!(failed.work, sum - 1);
    assert_eq!(short.reads, sum - 1);
    assert_eq!(short.remaining(), 0);
    assert!(
        failed.whole["output"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["source"] == input[1].uri
                && d["message"] == "Retained token traversal budget exceeded")
    );
    let mut zero = QueryBudget::new(0);
    let denied = run(&package, &input, limits, Some(&mut zero));
    assert!(denied.whole["result"].get("Err").is_some());
    assert!(denied.rollback);
    assert_eq!(denied.work, 0);
    assert_eq!(zero.reads, 0);
    assert!(denied.whole["parsed"].as_array().unwrap().is_empty());
    let mut recovery = QueryBudget::new(sum);
    let recovered = run(&package, &input, limits, Some(&mut recovery));
    assert_eq!(recovered.whole, complete.whole);
    assert_eq!(recovered.work, sum);
}

#[test]
fn lexical_rejection_debits_work_and_preserves_whole_default_failure() {
    let package = fixture();
    let input = vec![Source {
        uri: "memory://resource-budget/lexical.lib".into(),
        text: "library Broken {}".into(),
    }];
    // Common Terminals has ANY_OTHER, so an arbitrary character is not an
    // independent lexical rejection. Use the actual shared token-limit branch.
    let limits = EvaluationLimits {
        tokens_per_resource: Some(1),
        ..EvaluationLimits::workbench()
    };
    let plain = run(&package, &input, limits, None);
    assert!(plain.whole["result"].get("Err").is_some());
    assert_eq!(plain.work, 0); // Historical report excludes failed lexing.
    assert!(
        plain.whole["output"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["message"] == "Token budget exceeded")
    );
    let mut caller = QueryBudget::new(100_000);
    let failed = run(&package, &input, limits, Some(&mut caller));
    assert_eq!(failed.whole, plain.whole);
    assert!(failed.rollback);
    assert!(failed.work > 0);
    assert_eq!(caller.reads, failed.work);
    assert_eq!(caller.remaining(), 100_000 - failed.work);
    let second = run(
        &package,
        &sources(),
        EvaluationLimits::workbench(),
        Some(&mut caller),
    );
    assert_eq!(second.whole["result"], json!({"Ok":null}));
    assert_eq!(caller.reads, failed.work + second.work);
}

#[test]
fn parser_rejection_debits_before_outer_rollback_and_recovers() {
    let package = fixture();
    let input = vec![Source {
        uri: "memory://resource-budget/syntax.lib".into(),
        text: "library Broken { member Alice;".into(),
    }];
    let plain = run(&package, &input, EvaluationLimits::workbench(), None);
    assert!(plain.whole["result"].get("Err").is_some());
    assert!(plain.work > 0);
    let mut caller = QueryBudget::new(100_000);
    let failed = run(
        &package,
        &input,
        EvaluationLimits::workbench(),
        Some(&mut caller),
    );
    assert_eq!(failed.whole, plain.whole);
    assert!(failed.rollback);
    assert_eq!(failed.work, plain.work);
    assert_eq!(caller.reads, plain.work);
    let recovered = run(
        &package,
        &sources(),
        EvaluationLimits::workbench(),
        Some(&mut caller),
    );
    assert_eq!(recovered.whole["result"], json!({"Ok":null}));
    assert_eq!(caller.reads, failed.work + recovered.work);
}

#[test]
fn default_holders_and_duplicate_uri_behavior_remain_exact() {
    assert_eq!(
        std::mem::size_of::<resource_accounting::NoResourceAccounting>(),
        0
    );
    let package = fixture();
    let mut input = sources();
    input[1] = input[0].clone();
    let plain = run(&package, &input, EvaluationLimits::workbench(), None);
    let mut caller = QueryBudget::new(100_000);
    let scoped = run(
        &package,
        &input,
        EvaluationLimits::workbench(),
        Some(&mut caller),
    );
    assert_eq!(scoped.whole, plain.whole);
    assert_eq!(scoped.work, plain.work);
    assert_eq!(caller.reads, scoped.work);
    assert!(scoped.rollback);
    println!(
        "RESOURCE_ACCOUNTING_LAYOUT {}",
        json!({
        "default_policy":std::mem::size_of::<resource_accounting::NoResourceAccounting>(),
        "shared_policy":std::mem::size_of::<SharedResourceAccounting<'_>>(),
        "machine":std::mem::size_of::<Machine<'_, '_, FrozenStore>>(),
        "invocation":std::mem::size_of::<Invocation<'_, '_>>() })
    );
}
