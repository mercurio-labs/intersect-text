use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use intersect_text::package::BundleCompilation;
use serde_json::json;
pub fn input() -> BundleCompilation {
    let source = DefinitionSource::new(
        "memory:raw",
        SourceKind::Ecore,
        include_str!("../../../../../fixtures/ecore-generics/raw-runtime.ecore"),
    );
    let mut grammar = DefinitionSource::new(
        "memory:raw.xtext",
        SourceKind::Xtext,
        "grammar example.Raw import \"urn:raw\" as r Root returns r::Derived: 'raw';",
    );
    grammar.dependencies.push(source.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "memory:raw-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior.dependencies.push(source.id.clone());
    let input = serde_json::from_value(json!({"bundle":{"version":1,"sources":[source,grammar,rcore_behavior],"namespaces":[]},"ecore":source.id,"namespace":"urn:raw","xtext":grammar.id,"rcore_behavior":rcore_behavior.id,"materialized_types":[{"kind":"classifier","declaration":{"source":source.id,"path":[5]},"arguments":[],"raw":true},{"kind":"classifier","declaration":{"source":source.id,"path":[0]},"arguments":[],"raw":false}] })).unwrap();
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(input).unwrap()
}
