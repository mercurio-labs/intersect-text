use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    schema::EOBJECT_REFERENCE_TOP,
};
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
use serde_json::json;

#[test]
fn unbounded_raw_package_replays_reference_domain_and_text_construction() {
    let ecore = DefinitionSource::new(
        "memory:raw-unbounded",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/raw-unbounded.ecore"),
    );
    let mut grammar = DefinitionSource::new(
        "memory:raw-unbounded.xtext",
        SourceKind::Xtext,
        "grammar example.Raw import \"urn:raw-unbounded\" as u Root returns u::Box: 'raw';",
    );
    grammar.dependencies.push(ecore.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "memory:raw-unbounded-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior.dependencies.push(ecore.id.clone());
    let input: BundleCompilation = serde_json::from_value(json!({
        "bundle":{"version":1,"sources":[ecore,grammar,rcore_behavior],"namespaces":[]},
        "ecore":ecore.id,"namespace":"urn:raw-unbounded","xtext":grammar.id,"rcore_behavior":rcore_behavior.id,
        "materialized_types":[
            {"kind":"classifier","declaration":{"source":ecore.id,"path":[2]},"arguments":[],"raw":true},
            {"kind":"classifier","declaration":{"source":ecore.id,"path":[1]},"arguments":[],"raw":false}
        ]
    })).unwrap();
    let input =
        intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(input).unwrap();
    let package = compile_bundle(input.clone()).unwrap();
    assert!(package.schema.is_reference_target(EOBJECT_REFERENCE_TOP));
    let source = [Source {
        uri: "memory:model".into(),
        text: "raw".into(),
    }];
    let mut result = evaluate(&package, &source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    let id = result.model.elements[0].id.clone();
    let other = package
        .schema
        .class_key_for_type(&input.materialized_types[1])
        .unwrap();
    result
        .model
        .create(&package.schema, "other", other, "memory:model", 0, 0)
        .unwrap();
    result
        .model
        .edit(&package.schema, &[(id, "value".into(), json!("other"))])
        .unwrap();
    let replay: BundleCompilation = serde_json::from_value(
        serde_json::to_value(package.compilation.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    let rebuilt = compile_bundle(replay).unwrap();
    assert_eq!(rebuilt.schema, package.schema);
    result.model.validate(&rebuilt.schema).unwrap();
    assert_eq!(
        serde_json::to_value(evaluate(&package, &source)).unwrap(),
        serde_json::to_value(evaluate(&rebuilt, &source)).unwrap()
    );
}
