use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::materialize_runtime_types,
    generic_types::{RuntimeType, Type},
};
use intersect_text::package::BundleCompilation;
pub fn fixture() -> BundleCompilation {
    from_text(include_str!(
        "../../../../../fixtures/ecore-generics/eobject-grammar.ecore"
    ))
}
pub fn from_text(text: &str) -> BundleCompilation {
    let source = DefinitionSource::new("memory:eobject-grammar.ecore", SourceKind::Ecore, text);
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let ty = |n, args, raw| Type::Classifier {
        declaration: declarations[&vec![n]].clone(),
        arguments: args,
        raw,
    };
    let raw = RuntimeType::new(ty(4, vec![], true)).unwrap();
    let applied = RuntimeType::new(ty(4, vec![ty(0, vec![], false)], false)).unwrap();
    let mut requested = vec![
        raw.clone(),
        applied.clone(),
        RuntimeType::new(ty(5, vec![], true)).unwrap(),
    ];
    requested.extend(
        [1, 2, 3, 6]
            .into_iter()
            .map(|n| RuntimeType::new(ty(n, vec![], false)).unwrap()),
    );
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let schema = materialize_runtime_types(&bundle, &requested, None).unwrap();
    let raw_key = schema.class_key_for_type(&raw).unwrap();
    let applied_key = schema.class_key_for_type(&applied).unwrap();
    let grammar = format!(
        "grammar example.EObject import \"urn:eobject-grammar\" as m import \"http://www.eclipse.org/emf/2002/Ecore\" as e\nRoot returns m::Root: (nodes+=Node | others+=Other | users+=User | boxes+=Raw | boxes+=Applied | bounds+=Bound)* ('any' any+=[e::EObject|ID])* ('narrow' narrow=[e::EObject|ID])? ('user' user=[m::EObject|ID])?; Node returns m::Node: 'node' name=ID; Other returns m::Other: 'other' name=ID; User returns m::EObject: 'usernode' name=ID; Raw returns m::{raw_key}: 'raw' name=ID; Applied returns m::{applied_key}: 'applied' name=ID; Bound returns m::Bound: 'bound' name=ID 'target' target=[e::EObject|ID];"
    );
    let mut grammar =
        DefinitionSource::new("memory:eobject-grammar.xtext", SourceKind::Xtext, grammar);
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:eobject-grammar-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    BundleCompilation {
        ecore: source.id.clone(),
        namespace: "urn:eobject-grammar".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: requested,
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![source, grammar, semantics],
            namespaces: vec![],
        },
    }
}
pub fn source() -> intersect_text::Source {
    intersect_text::Source {uri:"memory:eobject.model".into(),text:"node N other O usernode U raw R applied A bound B target N any N any O any R any A any U narrow N user U".into()}
}
