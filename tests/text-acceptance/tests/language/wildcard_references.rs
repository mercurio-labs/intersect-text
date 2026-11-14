use intersect_model::core::model::Model;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::materialize_runtime_bundle,
    conversion::{
        AppliedConversionContract, ConversionContract, MemberContract, MemberType, ProviderId,
        ReferenceContext,
    },
    generic_types::{ClosedType, RuntimeType, Type},
};
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::package::BundleCompilation;

use intersect_model::{
    conversion::{constructor_contract, with_catalog_policies},
    model_reference::ReferenceDomain,
};
use serde_json::json;
#[test]
fn wildcard_constructor_bounds_consume_exact_generic_reference_members() {
    let source = DefinitionSource::new(
        "memory:wildcard-refs.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/wildcard-references.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let ty = |n, args, raw| Type::Classifier {
        declaration: id(n),
        arguments: args,
        raw,
    };
    let target = RuntimeType::new(ty(5, vec![ty(0, vec![], false)], false)).unwrap();
    let other_target = RuntimeType::new(ty(5, vec![ty(1, vec![], false)], false)).unwrap();
    let raw = RuntimeType::new(ty(5, vec![], true)).unwrap();
    let selected = vec![
        target.clone(),
        other_target,
        raw.clone(),
        RuntimeType::new(ty(6, vec![], false)).unwrap(),
        RuntimeType::new(ty(7, vec![], false)).unwrap(),
    ];
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let schema =
        materialize_runtime_bundle(&bundle, &source.id, "urn:wildcard-refs", &selected, None)
            .unwrap();
    let mut embedded = Model::default();
    for (name, ty) in [
        ("base", &selected[0]),
        ("other", &selected[4]),
        ("raw", &selected[2]),
        ("sub", &selected[3]),
    ] {
        embedded
            .create(
                &schema,
                name,
                schema.class_key_for_type(ty).unwrap(),
                "memory:embedded",
                0,
                0,
            )
            .unwrap();
    }
    let context = ReferenceContext {
        bundle: bundle.clone(),
        entry: source.id.clone(),
        namespace: "urn:wildcard-refs".into(),
        conversions: vec![],
        materialized_types: selected,
        applied_conversions: vec![],
        constructors: vec![],
        model: json!(embedded),
    };
    let domain = ReferenceDomain::new_runtime(
        target.clone(),
        context.clone(),
        &intersect_model::definitions::conversion::ConversionRegistry::default(),
    )
    .unwrap();
    let reference = domain.bind("sub").unwrap();
    assert!(domain.bind("other").is_err());
    assert!(domain.bind("raw").is_err());
    let provider = ProviderId {
        name: "ecore.list".into(),
        version: "1".into(),
    };
    let alias = AppliedConversionContract {
        application: ClosedType::new(ty(3, vec![target.expression().clone()], false)).unwrap(),
        conversion: ConversionContract {
            datatype: id(3),
            provider: provider.clone(),
            serializable: false,
            declarations: vec![],
            members: vec![MemberContract {
                nullable: false,
                value: MemberType::RuntimeReference {
                    target: target.clone(),
                    context: Box::new(context.clone()),
                },
            }],
        },
    };
    let bounded = AppliedConversionContract {
        application: ClosedType::new(ty(4, vec![alias.application.expression().clone()], false))
            .unwrap(),
        conversion: ConversionContract {
            datatype: id(4),
            provider: provider.clone(),
            serializable: false,
            declarations: vec![],
            members: vec![MemberContract {
                nullable: false,
                value: MemberType::Applied {
                    contract: Box::new(alias.clone()),
                },
            }],
        },
    };
    let policies = [2, 3]
        .iter()
        .map(|n| {
            constructor_contract(
                &bundle,
                &id(*n),
                ProviderId {
                    name: "ecore.list.constructor".into(),
                    version: "1".into(),
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let mut grammar = DefinitionSource::new(
        "memory:wildcard-refs.xtext",
        SourceKind::Xtext,
        "grammar example.Refs import \"urn:wildcard-refs\" as r\nHolder returns r::Holder: 'holder' name=ID;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:wildcard-refs-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let input = BundleCompilation {
        ecore: source.id.clone(),
        namespace: "urn:wildcard-refs".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: vec![RuntimeType::new(ty(8, vec![], false)).unwrap()],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![source.clone(), grammar, semantics],
            namespaces: vec![],
        },
    };
    with_catalog_policies(
        &input.bundle,
        &[],
        &[alias.clone(), bounded.clone()],
        &policies,
        |registry| {
            let package = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            let mut model = Model::default();
            model.create(
                &package.schema,
                "h",
                package
                    .schema
                    .class_key_for_type(&input.materialized_types[0])?,
                "memory:model",
                0,
                0,
            )?;
            let member = json!(
                registry
                    .applied()
                    .bind(&alias.application, json!({"items":[reference]}))?
            );
            let value = json!(
                registry
                    .applied()
                    .bind(&bounded.application, json!({"items":[member]}))?
            );
            model.edit_with_conversions(
                &package.schema,
                &[("h".into(), "values".into(), value.clone())],
                registry,
            )?;
            let before = model.clone();
            let mut foreign = value;
            foreign["converted"]["value"]["items"][0]["converted"]["value"]["items"][0]["element"] =
                json!("other");
            assert!(
                model
                    .edit_with_conversions(
                        &package.schema,
                        &[
                            ("h".into(), "name".into(), json!("rollback")),
                            ("h".into(), "values".into(), foreign)
                        ],
                        registry
                    )
                    .is_err()
            );
            assert_eq!(model, before);
            model.validate_with_conversions(&package.schema, registry)?;
            let mut wrong = alias.clone();
            wrong.conversion.members[0].value = MemberType::RuntimeReference {
                target: raw.clone(),
                context: Box::new(context.clone()),
            };
            assert!(with_catalog_policies(&input.bundle, &[], &[wrong], &[], |_| Ok(())).is_err());
            let retained =
                serde_json::from_value::<intersect_text::package::Package>(json!(package)).unwrap();
            let replay = retained.compilation.clone().unwrap();
            with_catalog_policies(
                &replay.bundle,
                &retained.schema.conversion_contracts,
                &retained.schema.applied_conversion_contracts,
                &retained.schema.constructor_contracts,
                |replay_registry| {
                    let rebuilt = compile_bundle_with_conversions(replay.clone(), replay_registry)
                        .map_err(|e| format!("{e:?}"))?;
                    assert_eq!(json!(rebuilt), json!(retained));
                    model.validate_with_conversions(&rebuilt.schema, replay_registry)
                },
            )
        },
    )
    .unwrap();
}
