use intersect_text::package::Package;
#[path = "common/runtime_operation_providers.rs"]
mod common;
use intersect_model::conversion::with_catalog_policies;
use intersect_model::definitions::generic_resolution::Resolver;
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use serde_json::json;
#[test]
fn raw_methods_consume_exact_method_only_policies_and_reconstruct_reports() {
    let f = common::fixture();
    let (package, reports) = with_catalog_policies(
        &f.input.bundle,
        &f.conversions,
        &[],
        &f.constructors,
        |registry| {
            let package = compile_bundle_with_conversions(f.input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            let resolver = Resolver::new(&f.input.bundle)
                .map_err(|e| e.to_string())?
                .with_conversions(registry)
                .map_err(|e| e.to_string())?;
            let mut reports = Vec::new();
            for (operation, argument) in f.operations.iter().zip(&f.arguments) {
                let report = package
                    .schema
                    .instantiate_runtime_operation(
                        &resolver,
                        &f.receiver,
                        operation,
                        Some(f.receiver.expression()),
                        std::slice::from_ref(argument),
                    )
                    .map_err(|e| e.to_string())?;
                assert_eq!(report.projection.source.owner, *f.receiver.expression());
                assert!(!report.warnings.is_empty());
                assert!(report.argument_bounds.iter().all(|b| b.satisfied));
                reports.push(report);
            }
            assert!(package.schema.conversion_contracts.is_empty());
            assert!(package.schema.constructor_contracts.is_empty());
            assert_eq!(resolver.consumed_relation_datatypes().len(), 2);
            assert_eq!(resolver.consumed_constructor_contracts().len(), 2);
            let outcome = evaluate_with_conversions(
                &package,
                &[Source {
                    uri: "memory:method.model".into(),
                    text: "holder Saved".into(),
                }],
                EvaluationLimits::native(),
                registry,
            );
            assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
            assert_eq!(outcome.model.elements.len(), 1);
            outcome
                .model
                .validate_with_conversions(&package.schema, registry)?;
            Ok((package, reports))
        },
    )
    .unwrap();
    let retained: Package = serde_json::from_value(json!(package)).unwrap();
    let replay = retained.compilation.clone().unwrap();
    with_catalog_policies(
        &replay.bundle,
        &f.conversions,
        &[],
        &f.constructors,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(replay.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(rebuilt), json!(retained));
            let resolver = Resolver::new(&replay.bundle)
                .map_err(|e| e.to_string())?
                .with_conversions(registry)
                .map_err(|e| e.to_string())?;
            for (i, (operation, argument)) in f.operations.iter().zip(&f.arguments).enumerate() {
                assert_eq!(
                    rebuilt
                        .schema
                        .instantiate_runtime_operation(
                            &resolver,
                            &f.receiver,
                            operation,
                            Some(f.receiver.expression()),
                            std::slice::from_ref(argument)
                        )
                        .map_err(|e| e.to_string())?,
                    reports[i]
                );
            }
            let mut stale = f.operations[0].clone();
            stale.source.sha256 = "0".repeat(64);
            assert!(
                rebuilt
                    .schema
                    .instantiate_runtime_operation(
                        &resolver,
                        &f.receiver,
                        &stale,
                        Some(f.receiver.expression()),
                        &f.arguments[..1]
                    )
                    .is_err()
            );
            Ok(())
        },
    )
    .unwrap();
    for missing in 0..2 {
        let conversions = if missing == 0 {
            vec![]
        } else {
            f.conversions.clone()
        };
        let constructors = if missing == 1 {
            vec![]
        } else {
            f.constructors.clone()
        };
        with_catalog_policies(
            &f.input.bundle,
            &conversions,
            &[],
            &constructors,
            |registry| {
                let resolver = Resolver::new(&f.input.bundle)
                    .map_err(|e| e.to_string())?
                    .with_conversions(registry)
                    .map_err(|e| e.to_string())?;
                assert!(
                    resolver
                        .instantiate_runtime_operation_in_owner(
                            &f.receiver,
                            &f.operations[missing],
                            Some(f.receiver.expression()),
                            std::slice::from_ref(&f.arguments[missing])
                        )
                        .is_err()
                );
                Ok(())
            },
        )
        .unwrap();
    }
    let mut stale = f.constructors.clone();
    stale[0].provider.version = "stale".into();
    assert!(
        with_catalog_policies(&f.input.bundle, &f.conversions, &[], &stale, |_| Ok(())).is_err()
    );
    let mut stale = f.conversions.clone();
    stale[0].provider.version = "stale".into();
    assert!(
        with_catalog_policies(&f.input.bundle, &stale, &[], &f.constructors, |_| Ok(())).is_err()
    );
}
#[test]
fn installed_method_policies_reject_wrong_scope_and_source_revision() {
    let f = common::fixture();
    let mut wrong = f.conversions.clone();
    wrong[0].datatype.path = vec![4];
    assert!(
        with_catalog_policies(&f.input.bundle, &wrong, &[], &f.constructors, |_| Ok(())).is_err()
    );
    let mut wrong = f.constructors.clone();
    wrong[0].provider.name = "ecore.map.constructor".into();
    assert!(
        with_catalog_policies(&f.input.bundle, &f.conversions, &[], &wrong, |_| Ok(())).is_err()
    );
    let mut stale = f.conversions.clone();
    stale[0].datatype.source.sha256 = "0".repeat(64);
    assert!(
        with_catalog_policies(&f.input.bundle, &stale, &[], &f.constructors, |_| Ok(())).is_err()
    );
    let source = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:runtime-operation-providers.ecore",
        intersect_model::definitions::bundle::SourceKind::Ecore,
        format!("{}\n", common::SOURCE),
    );
    let bundle = intersect_model::definitions::bundle::DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let foreign = intersect_model::conversion::constructor_contract(
        &bundle,
        &intersect_model::definitions::resources::DeclarationId {
            source: source.id,
            path: vec![2],
        },
        f.constructors[0].provider.clone(),
    )
    .unwrap();
    let mut stale = f.constructors.clone();
    stale[0] = foreign;
    assert!(
        with_catalog_policies(&f.input.bundle, &f.conversions, &[], &stale, |_| Ok(())).is_err()
    );
}
