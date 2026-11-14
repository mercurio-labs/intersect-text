use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
use serde_json::json;

fn input(reference: &str) -> BundleCompilation {
    let text = include_str!("../../../../fixtures/ecore-generics/raw-unbounded.ecore").replace(
        "<eClassifiers x:type=\"e:EClass\" name=\"Holder\">",
        "<eClassifiers x:type=\"e:EClass\" name=\"Holder\"><eStructuralFeatures x:type=\"e:EReference\" name=\"values\" eType=\"#//Value\" upperBound=\"-1\" containment=\"true\"/><eStructuralFeatures x:type=\"e:EReference\" name=\"boxes\" eType=\"#//Box\" upperBound=\"-1\" containment=\"true\"/>");
    let ecore = DefinitionSource::new("memory:raw-linking", SourceKind::Ecore, text);
    let mut grammar = DefinitionSource::new(
        "memory:raw-linking.xtext",
        SourceKind::Xtext,
        format!(
            "grammar example.Linking import \"urn:raw-unbounded\" as u Root returns u::Holder: values+=Value* boxes+=Box*; Value returns u::Value: 'value' name=ID; Box returns u::Box: 'box' value=[u::{reference}|ID];"
        ),
    );
    grammar.dependencies.push(ecore.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "memory:raw-linking-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior.dependencies.push(ecore.id.clone());
    let types:Vec<_>=[(0,false),(2,true),(3,false),(4,false)].into_iter().map(|(n,raw)|json!({"kind":"classifier","declaration":{"source":ecore.id,"path":[n]},"arguments":[],"raw":raw})).collect();
    let input = serde_json::from_value(json!({"bundle":{"version":1,"sources":[ecore,grammar,rcore_behavior],"namespaces":[]},"ecore":ecore.id,"namespace":"urn:raw-unbounded","xtext":grammar.id,"rcore_behavior":rcore_behavior.id,"materialized_types":types})).unwrap();
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(input).unwrap()
}
#[test]
fn ordinary_typed_text_links_into_unbounded_raw_reference() {
    let package = compile_bundle(input("Value")).unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "memory:model".into(),
            text: "value V box V".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.model.validate(&package.schema).unwrap();
    let value = result
        .model
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&json!("V")))
        .unwrap();
    let raw = result
        .model
        .elements
        .iter()
        .find(|e| e.values.contains_key("value"))
        .unwrap();
    assert_eq!(raw.values["value"], json!(value.id));
    assert!(compile_bundle(input("__ecore_eobject_reference_top")).is_err());
}
