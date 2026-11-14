use intersect_text::package::BundleCompilation;
#[path = "../../definitions/common/runtime_lower_bounds.rs"]
mod domain;
use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
pub fn fixture() -> BundleCompilation {
    let f = domain::Fixture::new();
    let mut types = vec![
        f.atom(0),
        f.atom(1),
        f.atom(2),
        f.ty(3, vec![], true),
        f.ty(4, vec![], true),
        f.pair(&f.atom(0), &f.atom(1)),
    ];
    types.extend((12..=16).map(|n| f.atom(n)));
    let schema =
        intersect_model::definitions::compiler::materialize_runtime_types(&f.bundle, &types, None)
            .unwrap();
    let raw_owner = schema.class_key_for_type(&types[4]).unwrap();
    let raw_pair = schema.class_key_for_type(&types[3]).unwrap();
    let mut grammar = DefinitionSource::new(
        "memory:runtime-lower.xtext",
        SourceKind::Xtext,
        "grammar example.Lower import \"urn:runtime-lower\" as l Root returns l::Root: 'model' '{' (items+=BaseSub | items+=SubSub | items+=BaseOther | owners+=RawOwner | owners+=ClosedOwner)* '}'; BaseSub returns l::BaseSub: 'good' name=ID; SubSub returns l::SubSub: 'narrow' name=ID; BaseOther returns l::BaseOther: 'bad' name=ID; RawOwner returns l::Owner: 'raw' name=ID value=[l::Pair|ID]; ClosedOwner returns l::ClosedOwner: 'closed' name=ID value=[l::Pair|ID];".replace("l::Owner", &format!("l::{raw_owner}")).replace("l::Pair", &format!("l::{raw_pair}")),
    );
    grammar.dependencies.push(f.source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:runtime-lower-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(f.source.id.clone());
    let mut bundle = f.bundle;
    bundle.sources.extend([grammar.clone(), semantics.clone()]);
    BundleCompilation {
        bundle,
        ecore: f.source.id,
        namespace: "urn:runtime-lower".into(),
        xtext: Some(grammar.id),
        rcore_behavior: semantics.id,
        materialized_types: types,
    }
}
pub fn source() -> intersect_text::Source {
    intersect_text::Source {
        uri: "memory:lower.model".into(),
        text: "model { good G narrow N bad B raw R G closed C N }".into(),
    }
}
