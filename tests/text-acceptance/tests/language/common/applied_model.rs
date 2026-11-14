use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{
        AppliedConversionContract, ConversionContract, MemberContract, MemberType, ProviderId,
    },
    generic_types::{ClosedType, Type},
};
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
pub fn input() -> (BundleCompilation, Vec<AppliedConversionContract>) {
    let source = DefinitionSource::new(
        "memory:applied-model.ecore",
        SourceKind::Ecore,
        include_str!("../../../../../fixtures/ecore-generics/applied-model.ecore"),
    );
    let migration =
        intersect_model::migration::migrate_ecore(source, "memory:applied-model.rcore").unwrap();
    let source = migration.converted.unwrap();
    let id = |n| {
        migration
            .correspondence
            .iter()
            .find(|row| row.original.path == [n])
            .unwrap()
            .converted
            .clone()
    };
    let ty = |n, arguments| Type::Classifier {
        declaration: id(n),
        arguments,
        raw: false,
    };
    let member = |n| MemberContract {
        nullable: false,
        value: MemberType::Datatype { declaration: id(n) },
    };
    let contract = |n, name: &str, arguments, members| AppliedConversionContract {
        application: ClosedType::new(ty(n, arguments)).unwrap(),
        conversion: ConversionContract {
            datatype: id(n),
            provider: ProviderId {
                name: name.into(),
                version: "1".into(),
            },
            serializable: false,
            declarations: vec![],
            members,
        },
    };
    let text = contract(2, "ecore.list", vec![ty(0, vec![])], vec![member(0)]);
    let number = contract(2, "ecore.list", vec![ty(1, vec![])], vec![member(1)]);
    let nested = contract(
        2,
        "ecore.list",
        vec![text.application.expression().clone()],
        vec![MemberContract {
            nullable: false,
            value: MemberType::Applied {
                contract: Box::new(text.clone()),
            },
        }],
    );
    let map = contract(
        3,
        "ecore.map",
        vec![ty(0, vec![]), ty(1, vec![])],
        vec![member(0), member(1)],
    );
    let materialized_types = vec![
        ClosedType::new(ty(4, vec![])).unwrap(),
        ClosedType::new(ty(6, vec![])).unwrap(),
    ];
    let mut grammar = DefinitionSource::new(
        "memory:applied-model.xtext",
        SourceKind::Xtext,
        "grammar example.Values import \"urn:values\" as v\nHolder returns v::Holder: 'holder' name=ID;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:applied-model-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    (
        BundleCompilation {
            ecore: source.id.clone(),
            namespace: "urn:values".into(),
            xtext: Some(grammar.id.clone()),
            rcore_behavior: semantics.id.clone(),
            materialized_types: materialized_types.into_iter().map(Into::into).collect(),
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![source, grammar, semantics],
                namespaces: vec![],
            },
        },
        vec![text, number, nested, map],
    )
}
pub fn nominal(input: &BundleCompilation, index: usize, value: Value) -> Value {
    let name = ["Text", "Number"][index];
    let declaration = intersect_model::definitions::resources::ResourceSet::new(&input.bundle)
        .unwrap()
        .resolve(&input.ecore, &format!("#//{name}"))
        .unwrap();
    json!({"kind":"datatype","datatype":declaration,"value":value})
}
