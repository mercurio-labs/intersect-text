use intersect_model::conversion::with_catalog;
use intersect_model::core::model::Model;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{ConversionContract, ProviderId},
    generic_types::{ClosedType, Type},
};
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use serde_json::json;

#[test]
fn inherited_generic_provider_defaults_text_and_replay_keep_actual_nominal_identity() {
    let model = DefinitionSource::new(
        "memory:generic-provider-feature-matrix",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/effective-provider-matrix.ecore"),
    );
    let (model, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(model);
    let id = |index| declarations[&vec![index]].clone();
    let contracts: Vec<_> = [0, 1]
        .into_iter()
        .map(|index| ConversionContract {
            datatype: id(index),
            provider: ProviderId {
                name: "example.rgb".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![],
        })
        .collect();
    let roots = [5, 6]
        .into_iter()
        .map(|index| {
            ClosedType::new(Type::Classifier {
                declaration: id(index),
                arguments: vec![],
                raw: false,
            })
            .unwrap()
            .into()
        })
        .collect();
    let mut grammar = DefinitionSource::new(
        "memory:generic-provider-feature-matrix.xtext",
        SourceKind::Xtext,
        "grammar example.ProviderFeatures import \"urn:generic-provider-features\" as m Diamond returns m::Diamond: 'record' name=ID color=STRING;",
    );
    grammar.dependencies.push(model.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:generic-provider-feature-matrix-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        ecore: model.id.clone(),
        namespace: "urn:generic-provider-features".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: roots,
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), grammar, semantics],
            namespaces: vec![],
        },
    };
    let sources = [Source {
        uri: "provider-features.model".into(),
        text: "record sample \"#010203\"".into(),
    }];
    let package = with_catalog(&input.bundle, &contracts, |registry| {
        let package = compile_bundle_with_conversions(input.clone(), registry)
            .map_err(|errors| format!("{errors:?}"))?;
        let schema = &package.schema;
        let diamond = schema.language_class_key("Diamond")?;
        let other = schema.language_class_key("OtherRecord")?;
        let feature = schema.effective_feature(diamond, "color")?;
        assert_eq!(feature.feature.datatype, Some(id(0)));
        assert_eq!(
            schema.effective_feature(other, "color")?.feature.datatype,
            Some(id(1))
        );
        assert_eq!(
            feature.identity,
            schema.effective_feature(other, "color")?.identity
        );
        assert_ne!(
            feature.owner,
            schema.effective_feature(other, "color")?.owner
        );
        let mut direct = Model::default();
        direct.create(schema, "d", diamond, "api", 0, 0)?;
        direct.create(schema, "o", other, "api", 0, 0)?;
        assert_eq!(
            direct.elements[0].values["color"],
            json!(registry.parse(&id(0), "#123ABC")?)
        );
        assert_eq!(
            direct.elements[1].values["color"],
            json!(registry.parse(&id(1), "#123ABC")?)
        );
        let before = direct.clone();
        assert!(
            direct
                .edit_with_conversions(
                    schema,
                    &[
                        ("d".into(), "name".into(), json!("rollback")),
                        (
                            "d".into(),
                            "color".into(),
                            json!(registry.parse(&id(1), "#010203")?)
                        )
                    ],
                    registry
                )
                .is_err()
        );
        assert_eq!(direct, before);
        direct.edit_with_conversions(
            schema,
            &[
                ("d".into(), "name".into(), json!("sample")),
                (
                    "d".into(),
                    "color".into(),
                    json!(registry.parse(&id(0), "#010203")?),
                ),
            ],
            registry,
        )?;
        let outcome =
            evaluate_with_conversions(&package, &sources, EvaluationLimits::native(), registry);
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(outcome.model.elements[0].values, direct.elements[0].values);
        assert_eq!(
            outcome.model.elements[0].type_identity,
            direct.elements[0].type_identity
        );
        outcome.model.validate_with_conversions(schema, registry)?;
        let bad = evaluate_with_conversions(
            &package,
            &[Source {
                uri: "bad.model".into(),
                text: "record bad \"not a color\"".into(),
            }],
            EvaluationLimits::native(),
            registry,
        );
        assert!(!bad.diagnostics.is_empty());
        Ok((package, outcome))
    })
    .unwrap();
    let retained: BundleCompilation = serde_json::from_value(
        serde_json::to_value(package.0.compilation.clone().unwrap()).unwrap(),
    )
    .unwrap();
    let restored = with_catalog(
        &retained.bundle,
        &package.0.schema.conversion_contracts,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(retained.clone(), registry)
                .map_err(|errors| format!("{errors:?}"))?;
            Ok(evaluate_with_conversions(
                &rebuilt,
                &sources,
                EvaluationLimits::native(),
                registry,
            ))
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(package.1).unwrap()
    );
    assert!(
        with_catalog(&input.bundle, &contracts[..1], |registry| {
            compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|errors| format!("{errors:?}"))
        })
        .is_err()
    );
}
