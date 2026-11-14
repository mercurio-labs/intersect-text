use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::import_datatype_bundle,
    generic_types::{RuntimeType, Type},
};
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
use serde_json::json;

#[test]
fn inherited_raw_attribute_text_uses_source_codec_and_receiver_domain_on_replay() {
    let source = DefinitionSource::new(
        "memory:raw-attribute.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/raw-feature-domains.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let mut grammar = DefinitionSource::new(
        "memory:raw-attribute.xtext",
        SourceKind::Xtext,
        "grammar example.RawAttribute import \"urn:raw-domains\" as r Root returns r::AttrChild: 'attribute' value=STRING;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:raw-attribute-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let id = |n| declarations[&vec![n]].clone();
    let ty = RuntimeType::new(Type::Classifier {
        declaration: id(9),
        arguments: vec![],
        raw: true,
    })
    .unwrap();
    let input = BundleCompilation {
        ecore: source.id.clone(),
        namespace: "urn:raw-domains".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: vec![ty.clone()],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![source.clone(), grammar, semantics],
            namespaces: vec![],
        },
    };
    let package = compile_bundle(input.clone()).unwrap();
    let sources = [Source {
        uri: "raw.model".into(),
        text: r#"attribute "retained""#.into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    let text = import_datatype_bundle(&input.bundle, &id(6))
        .unwrap()
        .parse("retained")
        .unwrap();
    assert_eq!(result.model.elements[0].values["value"], json!(text));
    let receiver = package.schema.class_key_for_type(&ty).unwrap();
    assert!(!package.schema.feature_domains(receiver, "value").is_empty());
    result.model.validate(&package.schema).unwrap();
    let replay = compile_bundle(serde_json::from_value(json!(input)).unwrap()).unwrap();
    assert_eq!(json!(evaluate(&replay, &sources)), json!(result));
    let mut edited = result.model;
    let before = edited.clone();
    let number = import_datatype_bundle(&replay.compilation.as_ref().unwrap().bundle, &id(7))
        .unwrap()
        .parse("7")
        .unwrap();
    let element = edited.elements[0].id.clone();
    assert!(
        edited
            .edit(&replay.schema, &[(element, "value".into(), json!(number))])
            .is_err()
    );
    assert_eq!(edited, before);
}
