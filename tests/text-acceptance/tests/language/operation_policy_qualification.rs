use intersect_text::package::Package;
#[path = "common/operation_policy_qualification.rs"]
mod common;
use intersect_model::conversion::with_catalog_policies;
use intersect_model::definitions::{generic_resolution::Resolver, generic_types::ClosedType};
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};

#[test]
fn installed_equivalence_groups_nominal_operations_without_changing_bridge_identity() {
    for generic in [false, true] {
        let f = common::fixture(generic);
        let bare = Resolver::new(&f.input.bundle).unwrap();
        assert!(
            !bare
                .operation_override(&f.child, &f.base)
                .unwrap()
                .overrides
        );
        let mut baseline = None;
        for reverse in [false, true] {
            let mut contracts = f.conversions.clone();
            let mut input = f.input.clone();
            if reverse {
                contracts.reverse();
                input.bundle.sources.reverse();
            }
            let result = with_catalog_policies(&input.bundle, &contracts, &[], &[], |registry| {
                let resolver = Resolver::new(&input.bundle)
                    .map_err(|e| e.to_string())?
                    .with_conversions(registry)
                    .map_err(|e| e.to_string())?;
                assert!(
                    resolver
                        .operation_override(&f.child, &f.base)
                        .map_err(|e| e.to_string())?
                        .overrides
                );
                let selected = resolver
                    .resolved_generic_operations(
                        &ClosedType::new(f.receiver.expression().clone())
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                assert_eq!(selected.operations.len(), 1);
                assert_eq!(selected.operations[0].selected.member.declaration, f.child);
                assert_eq!(
                    selected.operations[0].overridden[0].member.declaration,
                    f.base
                );
                let package = compile_bundle_with_conversions(input.clone(), registry)
                    .map_err(|e| format!("{e:?}"))?;
                let key = package.schema.class_key_for_type(&f.receiver)?;
                assert_eq!(package.schema.resolved_operations(key)?.len(), 1);
                let report = package
                    .schema
                    .instantiate_runtime_operation(&resolver, &f.receiver, &f.child, None, &[])
                    .map_err(|e| e.to_string())?;
                assert_eq!(report.operation.declaration, f.child);
                assert_eq!(report.projection.source.owner, *f.receiver.expression());
                assert_eq!(resolver.consumed_relation_datatypes().len(), 2);
                let outcome = evaluate_with_conversions(
                    &package,
                    &[Source {
                        uri: "memory:policy.model".into(),
                        text: "child Saved".into(),
                    }],
                    EvaluationLimits::native(),
                    registry,
                );
                assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
                assert_eq!(outcome.model.elements.len(), 1);
                let retained: Package =
                    serde_json::from_value(serde_json::to_value(&package).unwrap()).unwrap();
                let rebuilt = compile_bundle_with_conversions(
                    retained.compilation.clone().unwrap(),
                    registry,
                )
                .map_err(|e| format!("{e:?}"))?;
                assert_eq!(
                    serde_json::to_value(&rebuilt).unwrap(),
                    serde_json::to_value(&retained).unwrap()
                );
                assert_eq!(
                    rebuilt
                        .schema
                        .instantiate_runtime_operation(&resolver, &f.receiver, &f.child, None, &[])
                        .map_err(|e| e.to_string())?,
                    report
                );
                let mut stale = f.child.clone();
                stale.source.sha256 = "0".repeat(64);
                assert!(
                    rebuilt
                        .schema
                        .instantiate_runtime_operation(&resolver, &f.receiver, &stale, None, &[])
                        .is_err()
                );
                outcome
                    .model
                    .validate_with_conversions(&rebuilt.schema, registry)?;
                Ok((selected, report))
            })
            .unwrap();
            if let Some(expected) = &baseline {
                assert_eq!(expected, &result);
            } else {
                baseline = Some(result);
            }
        }
        let mut stale = f.conversions.clone();
        stale[0].provider.version = "stale".into();
        assert!(with_catalog_policies(&f.input.bundle, &stale, &[], &[], |_| Ok(())).is_err());
        let mut stale = f.conversions.clone();
        stale[0].datatype.source.sha256 = "0".repeat(64);
        assert!(with_catalog_policies(&f.input.bundle, &stale, &[], &[], |_| Ok(())).is_err());
    }
}

#[test]
fn installed_constructor_subtyping_does_not_become_override_equivalence() {
    use intersect_model::conversion::constructor_contract;
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        conversion::ProviderId,
        resources::DeclarationId,
    };
    for (class, provider, equivalent) in [
        (
            "org.eclipse.emf.common.util.EList",
            "ecore.list.constructor",
            true,
        ),
        ("java.util.List", "java.list.constructor", false),
    ] {
        let source = DefinitionSource::new(
            "memory:operation-constructor-policy",
            SourceKind::Ecore,
            format!(
                "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:x='http://www.w3.org/2001/XMLSchema-instance' name='p' nsURI='urn:p'><eClassifiers x:type='e:EDataType' name='A' instanceClassName='org.eclipse.emf.common.util.EList' serializable='false'><eTypeParameters name='T'/></eClassifiers><eClassifiers x:type='e:EDataType' name='B' instanceClassName='{class}' serializable='false'><eTypeParameters name='T'/></eClassifiers><eClassifiers x:type='e:EClass' name='Base'><eOperations name='f'><eParameters name='p' eType='#//A'/></eOperations></eClassifiers><eClassifiers x:type='e:EClass' name='Child' eSuperTypes='#//Base'><eOperations name='f'><eParameters name='p' eType='#//B'/></eOperations></eClassifiers></e:EPackage>"
            ),
        );
        let id = |path| DeclarationId {
            source: source.id.clone(),
            path,
        };
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![source.clone()],
            namespaces: vec![],
        };
        let contracts = vec![
            constructor_contract(
                &bundle,
                &id(vec![0]),
                ProviderId {
                    name: "ecore.list.constructor".into(),
                    version: "1".into(),
                },
            )
            .unwrap(),
            constructor_contract(
                &bundle,
                &id(vec![1]),
                ProviderId {
                    name: provider.into(),
                    version: "1".into(),
                },
            )
            .unwrap(),
        ];
        for reverse in [false, true] {
            let mut selected = contracts.clone();
            if reverse {
                selected.reverse();
            }
            with_catalog_policies(&bundle, &[], &[], &selected, |registry| {
                let resolver = Resolver::new(&bundle)
                    .map_err(|e| e.to_string())?
                    .with_conversions(registry)
                    .map_err(|e| e.to_string())?;
                assert_eq!(
                    resolver
                        .operation_override(&id(vec![3, 0]), &id(vec![2, 0]))
                        .map_err(|e| e.to_string())?
                        .overrides,
                    equivalent
                );
                assert_eq!(resolver.consumed_constructor_contracts().len(), 2);
                Ok(())
            })
            .unwrap();
        }
    }
}
