use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{
        AppliedConversionContract, ConversionContract, MemberContract, MemberType, ProviderId,
    },
    generic_types::{ClosedType, Type},
};
use intersect_text::Source;
use intersect_text::package::BundleCompilation;
pub fn input() -> (BundleCompilation, Vec<AppliedConversionContract>) {
    from_text(include_str!(
        "../../../../../fixtures/ecore-generics/serializable-sequence.ecore"
    ))
}
pub fn from_text(text: &str) -> (BundleCompilation, Vec<AppliedConversionContract>) {
    let source = DefinitionSource::new("memory:sequence.ecore", SourceKind::Ecore, text);
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let ty = |n, arguments| Type::Classifier {
        declaration: id(n),
        arguments,
        raw: false,
    };
    let member = |n| MemberContract {
        nullable: false,
        value: MemberType::Datatype { declaration: id(n) },
    };
    let contract = |argument, member| AppliedConversionContract {
        application: ClosedType::new(ty(2, vec![argument])).unwrap(),
        conversion: ConversionContract {
            datatype: id(2),
            provider: ProviderId {
                name: "intersect.sequence".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![member],
        },
    };
    let text = contract(ty(0, vec![]), member(0));
    let number = contract(ty(1, vec![]), member(1));
    let nested = contract(
        text.application.expression().clone(),
        MemberContract {
            nullable: false,
            value: MemberType::Applied {
                contract: Box::new(text.clone()),
            },
        },
    );
    let owner = ClosedType::new(ty(3, vec![])).unwrap();
    let mut grammar = DefinitionSource::new(
        "memory:sequence.xtext",
        SourceKind::Xtext,
        "grammar example.Sequence import \"urn:lexical\" as l\nHolder returns l::Holder: 'holder' name=ID ('texts' texts=STRING)? ('numbers' numbers=STRING)? ('nested' nested=STRING)? ('many' many+=STRING)* ('wide' wide=INT)? ('word' word=ID)?;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:sequence-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    (
        BundleCompilation {
            ecore: source.id.clone(),
            namespace: "urn:lexical".into(),
            xtext: Some(grammar.id.clone()),
            rcore_behavior: semantics.id.clone(),
            materialized_types: vec![owner.into()],
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![source, grammar, semantics],
                namespaces: vec![],
            },
        },
        vec![text, number, nested],
    )
}
pub fn source() -> Source {
    let quote = |value: &str| serde_json::to_string(value).unwrap();
    let nested = serde_json::to_string(&vec![r#"["inside"]"#]).unwrap();
    Source {
        uri: "memory:sequence.model".into(),
        text: format!(
            "holder Demo texts {} numbers {} nested {} many {} many {} wide 0002147483648 word Label",
            quote(r#"["hello"]"#),
            quote(r#"["2147483648"]"#),
            quote(&nested),
            quote(r#"["one"]"#),
            quote(r#"["two"]"#)
        ),
    }
}
