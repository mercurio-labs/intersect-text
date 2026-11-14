use intersect_model::core::model::Model;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    generic_types::{ClosedType, Type},
};
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
use serde_json::json;

#[test]
fn substituted_enum_defaults_match_direct_construction_and_fresh_text_package_replay() {
    let model_source = DefinitionSource::new(
        "memory:generic-feature-matrix.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/effective-feature-matrix.ecore"),
    );
    let (model_source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(model_source);
    let mut grammar = DefinitionSource::new(
        "memory:generic-feature-matrix.xtext",
        SourceKind::Xtext,
        "grammar example.Features import \"urn:generic-features\" as m Diamond returns m::Diamond: 'record' name=ID;",
    );
    grammar.dependencies.push(model_source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:generic-feature-matrix-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(model_source.id.clone());
    let application = ClosedType::new(Type::Classifier {
        declaration: declarations[&vec![5]].clone(),
        arguments: vec![],
        raw: false,
    })
    .unwrap();
    let request = BundleCompilation {
        ecore: model_source.id.clone(),
        namespace: "urn:generic-features".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: vec![application.clone().into()],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model_source, grammar, semantics],
            namespaces: vec![],
        },
    };
    let package = compile_bundle(request).unwrap();
    let sources = [Source {
        uri: "features.model".into(),
        text: "record sample".into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    result.model.validate(&package.schema).unwrap();
    let class = package.schema.class_key_for_type(&application).unwrap();
    let field = package.schema.effective_feature(class, "selected").unwrap();
    let enumeration = &package.schema.enumerations[&field.feature.target];
    let alias = serde_json::to_value(enumeration.parse("alias").unwrap()).unwrap();
    let primary = serde_json::to_value(enumeration.parse("primary").unwrap()).unwrap();
    assert_eq!(result.model.elements[0].values["selected"], alias);
    assert_eq!(result.model.elements[0].values["first"], primary);
    let mut direct = Model::default();
    direct
        .create(&package.schema, "direct", class, "api", 0, 0)
        .unwrap();
    direct
        .edit(
            &package.schema,
            &[("direct".into(), "name".into(), json!("sample"))],
        )
        .unwrap();
    assert_eq!(direct.elements[0].values, result.model.elements[0].values);
    assert_eq!(
        direct.elements[0].type_identity,
        result.model.elements[0].type_identity
    );
    let retained =
        serde_json::from_value(serde_json::to_value(package.compilation.clone().unwrap()).unwrap())
            .unwrap();
    let replay = compile_bundle(retained).unwrap();
    assert_eq!(
        serde_json::to_value(evaluate(&replay, &sources)).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    let mut edited = result.model;
    let before = edited.clone();
    let element = edited.elements[0].id.clone();
    assert!(
        edited
            .edit(
                &replay.schema,
                &[
                    (element.clone(), "name".into(), json!("changed")),
                    (element, "selected".into(), json!("alias"))
                ]
            )
            .is_err()
    );
    assert_eq!(edited, before);
}
