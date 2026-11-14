use intersect_model::conversion::with_catalog_context;
use intersect_model::core::model::Model;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::materialize_closed_types_with_conversions,
    conversion::{
        AppliedConversionContract, ConversionContract, MemberContract, MemberType, ProviderId,
    },
    generic_types::{ClosedType, Type},
    resources::DeclarationId,
};
use serde_json::json;

#[test]
fn applied_bounds_share_selected_relations_in_nested_and_ordinary_members() {
    let source = DefinitionSource::new(
        "memory:applied-provider-bounds",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/applied-provider-bounds.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let ty = |n, arguments| Type::Classifier {
        declaration: id(n),
        arguments,
        raw: false,
    };
    let contract = |n, name: &str, serializable, members| ConversionContract {
        datatype: id(n),
        provider: ProviderId {
            name: name.into(),
            version: "1".into(),
        },
        serializable,
        declarations: vec![],
        members,
    };
    let rgb = contract(0, "example.rgb", true, vec![]);
    let alias = contract(1, "example.rgb", true, vec![]);
    let list = AppliedConversionContract {
        application: ClosedType::new(ty(2, vec![ty(1, vec![])])).unwrap(),
        conversion: contract(
            2,
            "ecore.list",
            false,
            vec![MemberContract {
                nullable: false,
                value: MemberType::Converted {
                    contract: Box::new(alias.clone()),
                },
            }],
        ),
    };
    let member = MemberContract {
        nullable: false,
        value: MemberType::Applied {
            contract: Box::new(list.clone()),
        },
    };
    let nested = AppliedConversionContract {
        application: ClosedType::new(ty(3, vec![list.application.expression().clone()])).unwrap(),
        conversion: contract(3, "ecore.list", false, vec![member.clone()]),
    };
    let wrapper = contract(4, "ecore.list", false, vec![member]);
    let receiver = ClosedType::new(ty(5, vec![])).unwrap();
    let wrapped_receiver = ClosedType::new(ty(6, vec![])).unwrap();
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let applications = vec![list.clone(), nested.clone()];
    // Composite first is deliberate: selection order cannot supply execution order.
    let ordinary = vec![wrapper.clone(), alias.clone(), rgb.clone()];
    let schema = with_catalog_context(&bundle, &ordinary, &applications, |registry| {
        let schema = materialize_closed_types_with_conversions(
            &bundle,
            std::slice::from_ref(&receiver),
            registry,
        )
        .map_err(|e| format!("{e:?}"))?;
        assert!(schema.conversion_contracts.contains(&rgb)); // bound-only dependency
        assert!(schema.conversion_contracts.contains(&alias));
        let color = registry.parse(&alias.datatype, "#123ABC")?;
        let value = registry
            .applied()
            .bind(&list.application, json!({"items":[color]}))?;
        let nested_value = registry
            .applied()
            .bind(&nested.application, json!({"items":[value.clone()]}))?;
        let wrapped = registry.bind(&wrapper.datatype, json!({"items":[value.clone()]}))?;
        let mut model = Model::default();
        model.create(
            &schema,
            "h",
            schema.class_key_for_type(&receiver)?,
            "memory:model",
            0,
            0,
        )?;
        model.edit_with_conversions(
            &schema,
            &[
                ("h".into(), "values".into(), json!(value)),
                ("h".into(), "nested".into(), json!(nested_value)),
                ("h".into(), "wrapped".into(), json!(wrapped)),
            ],
            registry,
        )?;
        model.validate_with_conversions(&schema, registry)?;
        let wrong = registry.parse(&rgb.datatype, "#123ABC")?;
        assert!(
            registry
                .applied()
                .bind(&list.application, json!({"items":[wrong]}))
                .is_err()
        );
        Ok(schema)
    })
    .unwrap();
    let mut reversed = ordinary.clone();
    reversed.reverse();
    with_catalog_context(&bundle, &reversed, &applications, |registry| {
        let replay = materialize_closed_types_with_conversions(
            &bundle,
            std::slice::from_ref(&receiver),
            registry,
        )
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(json!(replay), json!(schema));
        Ok(())
    })
    .unwrap();
    // No direct applied attribute: the bound is consumed only inside the
    // ordinary provider's member. Its dependencies must still survive replay.
    let mut grammar = DefinitionSource::new(
        "memory:bound.xtext",
        SourceKind::Xtext,
        "grammar example.Bound import \"urn:colors\" as c\nOnlyWrapped returns c::OnlyWrapped: 'holder';",
    );
    grammar.dependencies.push(bundle.sources[0].id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "memory:bound-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior
        .dependencies
        .push(bundle.sources[0].id.clone());
    let mut package_bundle = bundle.clone();
    package_bundle
        .sources
        .extend([grammar.clone(), rcore_behavior.clone()]);
    let input = intersect_text::package::BundleCompilation {
        bundle: package_bundle.clone(),
        ecore: bundle.sources[0].id.clone(),
        namespace: "urn:colors".into(),
        xtext: Some(grammar.id),
        rcore_behavior: rcore_behavior.id,
        materialized_types: vec![wrapped_receiver.into()],
    };
    let package = with_catalog_context(&package_bundle, &ordinary, &[], |registry| {
        intersect_text::compiler::compile_bundle_with_conversions(input.clone(), registry)
            .map_err(|e| format!("{e:?}"))
    })
    .unwrap();
    assert!(package.schema.applied_conversion_contracts.is_empty());
    assert_eq!(package.schema.conversion_contracts.len(), 3);
    with_catalog_context(
        &package_bundle,
        &package.schema.conversion_contracts,
        &[],
        |registry| {
            let replay =
                intersect_text::compiler::compile_bundle_with_conversions(input.clone(), registry)
                    .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(replay), json!(package));
            let evaluated = intersect_text::evaluate_with_conversions(
                &replay,
                &[intersect_text::Source {
                    uri: "memory:holder".into(),
                    text: "holder".into(),
                }],
                intersect_text::EvaluationLimits::default(),
                registry,
            );
            assert!(
                evaluated.diagnostics.is_empty(),
                "{:?}",
                evaluated.diagnostics
            );
            assert_eq!(evaluated.model.elements.len(), 1);
            Ok(())
        },
    )
    .unwrap();
    let mut stale = bundle.clone();
    stale.sources[0] = DefinitionSource::new(
        "memory:applied-provider-bounds",
        SourceKind::Ecore,
        format!("{} ", stale.sources[0].text),
    );
    assert!(with_catalog_context(&stale, &ordinary, &applications, |_| Ok(())).is_err());
    for missing in [&ordinary[..2], &ordinary[..1]] {
        assert!(with_catalog_context(&bundle, missing, &applications, |_| Ok(())).is_err());
    }
    // An embedded reference domain must not inherit relation permissions from
    // the outer collection, even when it points at the same source bundle.
    let mut reference_wrapper = wrapper.clone();
    reference_wrapper.members = vec![MemberContract {
        nullable: false,
        value: MemberType::Reference {
            class: DeclarationId {
                source: bundle.sources[0].id.clone(),
                path: vec![6],
            },
            context: Box::new(intersect_model::definitions::conversion::ReferenceContext {
                materialized_types: vec![],
                applied_conversions: vec![],
                constructors: vec![],
                bundle: bundle.clone(),
                entry: bundle.sources[0].id.clone(),
                namespace: "urn:colors".into(),
                conversions: vec![wrapper],
                model: json!({"elements":[]}),
            }),
        },
    }];
    let error = with_catalog_context(&bundle, &[reference_wrapper, rgb, alias], &[], |_| Ok(()))
        .unwrap_err();
    assert!(error.contains("assignability contract"), "{error}");
    let mut wrong_version = ordinary.clone();
    wrong_version[2].provider.version = "wrong".into();
    assert!(with_catalog_context(&bundle, &wrong_version, &applications, |_| Ok(())).is_err());
}
