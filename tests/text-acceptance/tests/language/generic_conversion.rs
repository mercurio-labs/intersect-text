use intersect_model::conversion::with_catalog;
use intersect_model::core::model::Model;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::{materialize_closed_types, materialize_closed_types_with_conversions},
    conversion::{ConversionContract, ProviderId},
    generic_types::{ClosedType, Type},
    resources::DeclarationId,
};
use serde_json::json;

#[test]
fn generic_attributes_use_exact_catalog_providers_for_defaults_edits_and_reload() {
    let source = DefinitionSource::new(
        "memory:colors",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/materialize-conversions.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let rgb = id(0);
    let requested = ClosedType::new(Type::Classifier {
        declaration: id(1),
        raw: false,
        arguments: vec![Type::Classifier {
            declaration: rgb.clone(),
            arguments: vec![],
            raw: false,
        }],
    })
    .unwrap();
    let contract = ConversionContract {
        datatype: rgb.clone(),
        provider: ProviderId {
            name: "example.rgb".into(),
            version: "1".into(),
        },
        serializable: true,
        declarations: vec![],
        members: vec![],
    };
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    assert!(materialize_closed_types(&bundle, std::slice::from_ref(&requested)).is_err());
    with_catalog(&bundle, std::slice::from_ref(&contract), |registry| {
        let schema = materialize_closed_types_with_conversions(
            &bundle,
            std::slice::from_ref(&requested),
            registry,
        )
        .unwrap();
        assert_eq!(schema.conversion_contracts, vec![contract.clone()]);
        let mut model = Model::default();
        model.create(
            &schema,
            "h",
            schema.class_key_for_type(&requested).unwrap(),
            "memory:model",
            0,
            0,
        )?;
        let default = serde_json::to_value(registry.parse(&rgb, "#123ABC")?).unwrap();
        assert_eq!(model.elements[0].values["color"], default);
        model.validate_with_conversions(&schema, registry)?;
        assert!(model.validate(&schema).is_err());
        let replacement = serde_json::to_value(registry.parse(&rgb, "#010203")?).unwrap();
        model.edit_with_conversions(
            &schema,
            &[("h".into(), "color".into(), replacement.clone())],
            registry,
        )?;
        let before = model.clone();
        let mut forged = replacement;
        forged["value"]["red"] = json!(256);
        assert!(
            model
                .edit_with_conversions(&schema, &[("h".into(), "color".into(), forged)], registry)
                .is_err()
        );
        assert_eq!(model, before);
        let restored = serde_json::from_value(serde_json::to_value(&schema).unwrap()).unwrap();
        model.validate_with_conversions(&restored, registry)?;
        Ok(())
    })
    .unwrap();
    let mut unknown = contract;
    unknown.provider.version = "unknown".into();
    assert!(with_catalog(&bundle, &[unknown], |_| Ok(())).is_err());
}

#[test]
fn rgb_alias_bounds_require_both_providers_and_preserve_actual_type_identity() {
    use intersect_model::definitions::generic_resolution::Resolver;
    let source = DefinitionSource::new(
        "memory:alias-bounds",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/provider-alias-bounds.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |path: &[usize]| declarations[path].clone();
    let selected = id(&[3, 0, 0]);
    let actual = id(&[1]);
    let requested = ClosedType::new(Type::Classifier {
        declaration: id(&[2]),
        raw: false,
        arguments: vec![Type::Classifier {
            declaration: actual.clone(),
            raw: false,
            arguments: vec![],
        }],
    })
    .unwrap();
    let contracts: Vec<_> = (0..2)
        .map(|n| ConversionContract {
            datatype: id(&[n]),
            provider: ProviderId {
                name: "example.rgb".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![],
        })
        .collect();
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    assert!(
        Resolver::new(&bundle)
            .unwrap()
            .application_bounds(&selected)
            .is_err()
    );
    with_catalog(&bundle, &contracts[..1], |registry| {
        assert!(
            Resolver::new(&bundle)
                .unwrap()
                .with_conversions(registry)
                .unwrap()
                .application_bounds(&selected)
                .is_err()
        );
        Ok(())
    })
    .unwrap();
    with_catalog(&bundle, &contracts, |registry| {
        let resolver = Resolver::new(&bundle)
            .unwrap()
            .with_conversions(registry)
            .unwrap();
        let report = resolver.application_bounds(&selected).unwrap();
        assert!(report.valid);
        assert_eq!(report.checks.len(), 1);
        assert_eq!(
            report.checks[0].argument,
            Type::Classifier {
                declaration: actual.clone(),
                raw: false,
                arguments: vec![]
            }
        );
        let schema = materialize_closed_types_with_conversions(
            &bundle,
            std::slice::from_ref(&requested),
            registry,
        )
        .unwrap();
        assert_eq!(schema.conversion_contracts, contracts);
        assert_eq!(schema.datatypes.len(), 2); // RGB is needed only by the bound.
        let mut model = Model::default();
        model.create(
            &schema,
            "h",
            schema.class_key_for_type(&requested).unwrap(),
            "memory:model",
            0,
            0,
        )?;
        let default = serde_json::to_value(registry.parse(&actual, "#123ABC")?).unwrap();
        assert_eq!(model.elements[0].values["color"], default);
        let edit = serde_json::to_value(registry.parse(&actual, "#010203")?).unwrap();
        model.edit_with_conversions(&schema, &[("h".into(), "color".into(), edit)], registry)?;
        let before = model.clone();
        let wrong_tag =
            serde_json::to_value(registry.parse(&contracts[0].datatype, "#010203")?).unwrap();
        assert!(
            model
                .edit_with_conversions(
                    &schema,
                    &[("h".into(), "color".into(), wrong_tag)],
                    registry
                )
                .is_err()
        );
        assert_eq!(model, before);
        let retained: Vec<ConversionContract> =
            serde_json::from_value(serde_json::to_value(&schema.conversion_contracts).unwrap())
                .unwrap();
        with_catalog(&bundle, &retained, |replayed_registry| {
            let replay = materialize_closed_types_with_conversions(
                &bundle,
                std::slice::from_ref(&requested),
                replayed_registry,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(&replay).unwrap(),
                serde_json::to_value(&schema).unwrap()
            );
            model.validate_with_conversions(&replay, replayed_registry)?;
            Ok(())
        })?;
        with_catalog(&bundle, &retained[1..], |missing_bound_registry| {
            assert!(
                materialize_closed_types_with_conversions(
                    &bundle,
                    std::slice::from_ref(&requested),
                    missing_bound_registry
                )
                .is_err()
            );
            Ok(())
        })?;
        // Exercise retained normal-package compilation inputs, not only schema JSON.
        let ecore = bundle.sources[0].id.clone();
        let mut grammar = DefinitionSource::new("memory:colors.xtext", SourceKind::Xtext,
            "grammar example.Colors import \"urn:colors\" as c\nHolder returns c::Holder: 'holder';");
        grammar.dependencies.push(ecore.clone());
        let mut semantics = DefinitionSource::new("memory:colors-behavior.rcore", SourceKind::RcoreBehavior,
            r#"rcore 1; package emptyRules version "1";"#);
        semantics.dependencies.push(ecore.clone());
        let mut package_bundle = bundle.clone();
        package_bundle.sources.extend([grammar.clone(), semantics.clone()]);
        let compilation = intersect_text::package::BundleCompilation {
            bundle: package_bundle.clone(), ecore, namespace: "urn:colors".into(),
            xtext: Some(grammar.id), rcore_behavior: semantics.id, materialized_types: vec![requested.clone().into()],
        };
        let package = intersect_text::compiler::compile_bundle_with_conversions(compilation, registry).unwrap();
        assert_eq!(package.schema.conversion_contracts, contracts);
        let inputs = serde_json::from_value(serde_json::to_value(package.compilation.clone().unwrap()).unwrap()).unwrap();
        with_catalog(&package_bundle, &package.schema.conversion_contracts, |replay_registry| {
            let replay = intersect_text::compiler::compile_bundle_with_conversions(inputs, replay_registry).unwrap();
            assert_eq!(serde_json::to_value(&replay).unwrap(), serde_json::to_value(&package).unwrap());
            let result = intersect_text::evaluate_with_conversions(&replay,
                &[intersect_text::Source { uri: "memory:holder".into(), text: "holder".into() }],
                intersect_text::EvaluationLimits::default(), replay_registry);
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(result.model.elements.len(), 1);
            assert_eq!(result.model.elements[0].values["color"], default);
            Ok(())
        })?;
        let members = resolver.instantiate_members(&requested).unwrap();
        assert_eq!(members.receiver, requested);
        let color = members
            .members
            .iter()
            .find(|member| member.name == "color")
            .unwrap();
        assert_eq!(
            color.signatures[0].specialized,
            vec![Type::Classifier {
                declaration: actual.clone(),
                raw: false,
                arguments: vec![],
            }]
        );
        Ok(())
    })
    .unwrap();
    let mut stale = bundle.clone();
    stale.sources[0] = DefinitionSource::new(
        stale.sources[0].id.uri.clone(),
        SourceKind::Rcore,
        format!("{} ", stale.sources[0].text),
    );
    with_catalog(&bundle, &contracts, |registry| {
        assert!(
            Resolver::new(&stale)
                .unwrap()
                .with_conversions(registry)
                .is_err()
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn provider_equivalent_declared_operations_keep_one_materialized_group() {
    let text = include_str!("../../../../fixtures/ecore-generics/operation-overrides.ecore")
        .replace("java.lang.String", "example.RGB")
        .replace(
            r##"<eTypeParameters name="U"><eBounds eClassifier="#//B"/></eTypeParameters>"##,
            "",
        )
        .replace(
            r##"eTypeParameter="#//Derived/f/U""##,
            r##"eClassifier="#//B""##,
        );
    let source = DefinitionSource::new("memory:provider-selection", SourceKind::Ecore, text);
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let receiver = ClosedType::new(Type::Classifier {
        declaration: id(3),
        arguments: vec![],
        raw: false,
    })
    .unwrap();
    let contracts: Vec<_> = [0, 1]
        .into_iter()
        .map(|n| ConversionContract {
            datatype: id(n),
            provider: ProviderId {
                name: "example.rgb".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![],
        })
        .collect();
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    assert!(materialize_closed_types(&bundle, std::slice::from_ref(&receiver)).is_err());
    with_catalog(&bundle, &contracts, |registry| {
        let schema = materialize_closed_types_with_conversions(
            &bundle,
            std::slice::from_ref(&receiver),
            registry,
        )
        .unwrap();
        let key = schema.class_key_for_type(&receiver)?;
        let groups = schema.resolved_operations(key)?;
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].selected.operation.declaration,
            declarations[&vec![3, 1]]
        );
        assert_eq!(groups[0].overridden[0], declarations[&vec![2, 1]]);
        assert_eq!(schema.conversion_contracts, contracts);
        let restored: intersect_model::definitions::schema::Schema =
            serde_json::from_value(serde_json::to_value(&schema).unwrap()).unwrap();
        assert_eq!(restored.resolved_operations(key)?, groups);
        let replay = materialize_closed_types_with_conversions(
            &bundle,
            std::slice::from_ref(&receiver),
            registry,
        )
        .unwrap();
        assert_eq!(replay, schema);
        Ok(())
    })
    .unwrap();
}

#[test]
fn generic_declaration_cannot_register_as_an_erased_rgb_domain() {
    let source = DefinitionSource::new(
        "memory:generic-color.ecore",
        SourceKind::Ecore,
        r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="g" nsURI="urn:g"><eClassifiers x:type="e:EDataType" name="Color"><eTypeParameters name="T"/></eClassifiers></e:EPackage>"#,
    );
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let contract = ConversionContract {
        datatype: DeclarationId {
            source: source.id.clone(),
            path: vec![0],
        },
        provider: ProviderId {
            name: "example.rgb".into(),
            version: "1".into(),
        },
        serializable: true,
        declarations: vec![],
        members: vec![],
    };
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let error = with_catalog(&bundle, &[contract], |_| Ok(())).unwrap_err();
    assert!(
        error.contains("specialized application provider"),
        "{error}"
    );
}
