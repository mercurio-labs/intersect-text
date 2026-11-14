use intersect_text::package::Package;
#[path = "common/generic_reference_context.rs"]
mod common;
use intersect_model::definitions::conversion::{MemberType, ReferenceContext};
use intersect_model::{
    conversion::{
        applied_collection_provider_with_conversions, collection_provider_with_conversions,
        with_catalog_policies,
    },
    model_reference::ReferenceDomain,
};
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use serde_json::json;

#[test]
fn exact_generic_reference_values_use_self_contained_contexts_and_replay() {
    let f = common::fixture();
    let package = with_catalog_policies(
        &f.input.bundle,
        &f.ordinary,
        &f.applications,
        &[],
        |registry| {
            let package = compile_bundle_with_conversions(f.input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert!(package.schema.constructor_contracts.is_empty());
            assert_eq!(package.schema.applied_conversion_contracts.len(), 2);
            let list = applied_collection_provider_with_conversions(
                &f.input.bundle,
                &f.applications[0],
                registry,
            )?;
            let a = list.bind_reference(0, "value")?;
            let b = list.bind_reference(0, "sub")?;
            assert_eq!(list.resolve_reference(0, &b)?.id, "sub");
            for rejected in ["other", "raw", "plain", "missing"] {
                assert!(list.bind_reference(0, rejected).is_err());
            }
            assert!(list.bind_reference(1, "value").is_err());
            let mut payload = list.empty()?;
            list.insert(&mut payload, 0, a.clone())?;
            list.insert(&mut payload, 1, b.clone())?;
            let before = payload.clone();
            let mut wrong = a.clone();
            wrong["context"] = json!("stale");
            assert!(list.insert(&mut payload, 0, wrong).is_err());
            assert_eq!(payload, before);
            let tagged = registry
                .applied()
                .bind(&f.applications[0].application, payload)?;
            let raw =
                collection_provider_with_conversions(&f.input.bundle, &f.ordinary[0], registry)?;
            for accepted in ["value", "other", "sub", "raw"] {
                raw.bind_reference(0, accepted)?;
            }
            assert!(raw.bind_reference(0, "plain").is_err());
            let plain =
                collection_provider_with_conversions(&f.input.bundle, &f.ordinary[1], registry)?;
            plain.bind_reference(0, "plain")?;
            assert!(plain.bind_reference(0, "value").is_err());
            let map = applied_collection_provider_with_conversions(
                &f.input.bundle,
                &f.applications[1],
                registry,
            )?;
            let mut mapping = map.empty()?;
            map.put(
                &mut mapping,
                map.bind_reference(0, "value")?,
                map.bind_reference(1, "sub")?,
            )?;
            assert_eq!(map.get(&mapping, &a)?, Some(b));
            let mapping = registry
                .applied()
                .bind(&f.applications[1].application, mapping)?;
            let result = evaluate_with_conversions(
                &package,
                &[Source {
                    uri: "memory:model".into(),
                    text: "holder Actual".into(),
                }],
                EvaluationLimits::native(),
                registry,
            );
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            let mut model = result.model;
            model.edit_with_conversions(
                &package.schema,
                &[
                    ("memory:model#0".into(), "refs".into(), json!(tagged)),
                    ("memory:model#0".into(), "mapping".into(), json!(mapping)),
                ],
                registry,
            )?;
            let before = model.clone();
            assert!(
                model
                    .edit_with_conversions(
                        &package.schema,
                        &[
                            ("memory:model#0".into(), "name".into(), json!("rollback")),
                            ("memory:model#0".into(), "refs".into(), json!(mapping))
                        ],
                        registry
                    )
                    .is_err()
            );
            assert_eq!(model, before);
            Ok(package)
        },
    )
    .unwrap();
    let package: Package = serde_json::from_slice(&serde_json::to_vec(&package).unwrap()).unwrap();
    let input = package.compilation.clone().unwrap();
    with_catalog_policies(
        &input.bundle,
        &package.schema.conversion_contracts,
        &package.schema.applied_conversion_contracts,
        &package.schema.constructor_contracts,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(rebuilt), json!(package));
            Ok(())
        },
    )
    .unwrap();
    with_catalog_policies(
        &f.context.bundle,
        &f.context.conversions,
        &f.context.applied_conversions,
        &f.context.constructors,
        |registry| {
            assert!(
                ReferenceDomain::new(
                    match f.target.expression() {
                        intersect_model::definitions::generic_types::Type::Classifier {
                            declaration,
                            ..
                        } => declaration.clone(),
                        _ => unreachable!(),
                    },
                    f.context.clone(),
                    registry
                )
                .is_err()
            );
            let raw =
                ReferenceDomain::new_runtime(f.raw_target.clone(), f.context.clone(), registry)?;
            raw.bind("other")?;
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn inner_policy_identity_and_isolation_reject_missing_stale_unused_or_forged_inputs() {
    let f = common::fixture();
    for change in 0..9 {
        let mut selected = f.applications[0].clone();
        let MemberType::RuntimeReference { target, context } =
            &mut selected.conversion.members[0].value
        else {
            panic!("Expected runtime reference")
        };
        match change {
            0 => context.constructors.clear(),
            1 => context.applied_conversions.clear(),
            2 => context.constructors[0].provider.version = "stale".into(),
            3 => context.namespace = "urn:wrong".into(),
            4 => context.entry.sha256 = "0".repeat(64),
            5 => context.model["elements"][0]["type_identity"] = json!("forged"),
            6 => context.materialized_types.clear(),
            7 => *target = f.raw_target.clone(),
            _ => context
                .applied_conversions
                .push(context.applied_conversions[0].clone()),
        }
        let outcome = with_catalog_policies(
            &f.input.bundle,
            &[],
            &[selected],
            &f.context.constructors,
            |_| Ok(()),
        );
        assert!(outcome.is_err(), "case {change}");
    }
    let mut ordinary = f.ordinary[0].clone();
    let MemberType::RuntimeReference { context, .. } = &mut ordinary.members[0].value else {
        panic!("Expected runtime reference")
    };
    let mut unused = context.constructors[0].clone();
    unused.datatype = intersect_model::definitions::compiler::import_datatype_bundle(
        &context.bundle,
        &intersect_model::definitions::resources::ResourceSet::new(&context.bundle)
            .unwrap()
            .resolve(&context.entry, "#//Bounded")
            .unwrap(),
    )
    .unwrap();
    context.constructors.push(unused);
    assert!(with_catalog_policies(&f.input.bundle, &[ordinary], &[], &[], |_| Ok(())).is_err());
}

#[test]
fn ordinary_reference_context_wire_and_digest_remain_unchanged() {
    #[derive(serde::Serialize)]
    struct Old<'a> {
        bundle: &'a intersect_model::definitions::bundle::DefinitionBundle,
        entry: &'a intersect_model::definitions::bundle::SourceId,
        namespace: &'a str,
        conversions: &'a Vec<intersect_model::definitions::conversion::ConversionContract>,
        model: &'a serde_json::Value,
    }
    let f = common::fixture();
    let mut context = f.context;
    context.materialized_types.clear();
    context.applied_conversions.clear();
    context.constructors.clear();
    let old = Old {
        bundle: &context.bundle,
        entry: &context.entry,
        namespace: &context.namespace,
        conversions: &context.conversions,
        model: &context.model,
    };
    let bytes = serde_json::to_vec(&old).unwrap();
    assert_eq!(serde_json::to_vec(&context).unwrap(), bytes);
    let replay: ReferenceContext = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(replay.digest().unwrap(), context.digest().unwrap());
    let before = context.digest().unwrap();
    context.materialized_types.push(f.target);
    assert_ne!(context.digest().unwrap(), before);
}

#[test]
fn embedded_reference_scopes_share_aggregate_nodes_and_context_bytes() {
    use intersect_model::core::model::Model;
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        conversion::{ConversionContract, MemberContract, ProviderId},
        resources::DeclarationId,
    };
    let inner = DefinitionSource::new(
        "memory:budget-inner",
        SourceKind::Ecore,
        r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="inner" nsURI="urn:inner"><eClassifiers x:type="e:EClass" name="Object"/></e:EPackage>"#,
    );
    let (inner, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(inner);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![inner.clone()],
        namespaces: vec![],
    };
    let schema = intersect_model::definitions::compiler::compile_structure_bundle(
        &bundle,
        &inner.id,
        "urn:inner",
    )
    .unwrap();
    let mut model = Model::default();
    model
        .create(
            &schema,
            "obj",
            schema.language_class_key("Object").unwrap(),
            "memory:inner",
            0,
            0,
        )
        .unwrap();
    let context = ReferenceContext {
        bundle,
        entry: inner.id.clone(),
        namespace: "urn:inner".into(),
        conversions: vec![],
        materialized_types: vec![],
        applied_conversions: vec![],
        constructors: vec![],
        model: json!(model),
    };
    let declarations=(0..65).map(|n|format!(r#"<eClassifiers x:type="e:EDataType" name="List{n}" instanceClassName="org.eclipse.emf.common.util.EList" serializable="false"/>"#)).collect::<String>();
    let outer = DefinitionSource::new(
        "memory:budget-outer",
        SourceKind::Ecore,
        format!(
            r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="outer" nsURI="urn:outer">{declarations}</e:EPackage>"#
        ),
    );
    let (outer, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(outer);
    let contracts = (0..65)
        .map(|n| ConversionContract {
            datatype: DeclarationId {
                source: outer.id.clone(),
                path: vec![n],
            },
            provider: ProviderId {
                name: "ecore.list".into(),
                version: "1".into(),
            },
            serializable: false,
            declarations: vec![],
            members: vec![MemberContract {
                nullable: false,
                value: MemberType::Reference {
                    class: DeclarationId {
                        source: inner.id.clone(),
                        path: vec![0],
                    },
                    context: Box::new(context.clone()),
                },
            }],
        })
        .collect::<Vec<_>>();
    let outer = DefinitionBundle {
        version: 1,
        sources: vec![outer],
        namespaces: vec![],
    };
    with_catalog_policies(&outer, &contracts[..64], &[], &[], |_| Ok(())).unwrap();
    let error = with_catalog_policies(&outer, &contracts, &[], &[], |_| Ok(())).unwrap_err();
    assert!(error.contains("budget"), "{error}");
    let mut oversized = contracts[0].clone();
    let MemberType::Reference { context, .. } = &mut oversized.members[0].value else {
        panic!("Expected reference")
    };
    context.model["oversized"] = json!("x".repeat(16 * 1024 * 1024));
    assert!(
        with_catalog_policies(&outer, &[oversized], &[], &[], |_| Ok(()))
            .unwrap_err()
            .contains("byte budget")
    );
}

#[test]
fn runtime_reference_members_do_not_acquire_a_lexical_identity_codec() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        conversion::AppliedConversionContract,
        generic_types::{ClosedType, Type},
        resources::DeclarationId,
    };
    let f = common::fixture();
    let source = DefinitionSource::new(
        "memory:reference-sequence",
        SourceKind::Ecore,
        r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="lexical" nsURI="urn:lexical-reference"><eClassifiers x:type="e:EDataType" name="Sequence" instanceClassName="intersect.Sequence"><eTypeParameters name="T"/></eClassifiers></e:EPackage>"#,
    );
    let (mut source, _) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    source.dependencies.push(f.input.ecore.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone(), f.context.bundle.sources[0].clone()],
        namespaces: vec![],
    };
    let declaration = DeclarationId {
        source: source.id,
        path: vec![0],
    };
    let mut contract = f.applications[0].conversion.clone();
    contract.datatype = declaration.clone();
    contract.provider.name = "intersect.sequence".into();
    contract.serializable = true;
    let selection = AppliedConversionContract {
        application: ClosedType::new(Type::Classifier {
            declaration,
            arguments: vec![f.target.expression().clone()],
            raw: false,
        })
        .unwrap(),
        conversion: contract,
    };
    let error = with_catalog_policies(&bundle, &[], &[selection], &[], |_| Ok(())).unwrap_err();
    assert!(error.contains("no lexical identity codec"), "{error}");
    // An identical spelling in another source does not identify the embedded target.
    let other_source = DefinitionSource::new(
        "memory:same-names-other-resource",
        SourceKind::Rcore,
        f.context.bundle.sources[0].text.clone(),
    );
    for source in [
        other_source.id,
        intersect_model::definitions::bundle::SourceId {
            uri: f.context.entry.uri.clone(),
            sha256: "0".repeat(64),
        },
    ] {
        let mut foreign = f.applications[0].clone();
        let MemberType::RuntimeReference { target, .. } = &mut foreign.conversion.members[0].value
        else {
            panic!("Expected runtime reference")
        };
        let mut wire = json!(target);
        wire["declaration"]["source"] = json!(source);
        *target = serde_json::from_value(wire).unwrap();
        assert!(with_catalog_policies(&f.input.bundle, &[], &[foreign], &[], |_| Ok(())).is_err());
    }
}

#[test]
fn direct_domain_constructor_rejects_duplicate_selection_hiding_a_dependency() {
    let f = common::fixture();
    with_catalog_policies(
        &f.context.bundle,
        &f.context.conversions,
        &f.context.applied_conversions,
        &f.context.constructors,
        |registry| {
            let valid =
                ReferenceDomain::new_runtime(f.target.clone(), f.context.clone(), registry)?;
            valid.bind("value")?;
            let mut hidden = f.context.clone();
            assert_eq!(hidden.constructors.len(), 2);
            hidden.constructors[1] = hidden.constructors[0].clone();
            assert!(
                ReferenceDomain::new_runtime(f.target.clone(), hidden.clone(), registry).is_err()
            );
            let plain =
                intersect_model::definitions::resources::ResourceSet::new(&f.context.bundle)
                    .unwrap()
                    .resolve(&f.context.entry, "#//Plain")
                    .unwrap();
            assert!(ReferenceDomain::new(plain, hidden, registry).is_err());
            Ok(())
        },
    )
    .unwrap();
}
