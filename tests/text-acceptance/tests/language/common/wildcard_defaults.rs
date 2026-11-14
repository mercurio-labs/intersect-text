use intersect_model::conversion::constructor_contract;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{
        AppliedConversionContract, ConstructorContract, ConversionContract, MemberContract,
        MemberType, ProviderId,
    },
    generic_types::{ClosedType, RuntimeType, Type},
};
use intersect_text::package::BundleCompilation;
pub struct Fixture {
    pub input: BundleCompilation,
    pub applications: Vec<AppliedConversionContract>,
    pub constructors: Vec<ConstructorContract>,
    pub selected: usize,
}
pub fn fixture(diamond: bool) -> Fixture {
    from_text(
        include_str!("../../../../../fixtures/ecore-generics/wildcard-defaults.ecore"),
        diamond,
    )
}
pub fn from_text(text: &str, diamond: bool) -> Fixture {
    let source = DefinitionSource::new("memory:wildcard-defaults.ecore", SourceKind::Ecore, text);
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let ty = |n, args| Type::Classifier {
        declaration: id(n),
        arguments: args,
        raw: false,
    };
    let scalar = |n| MemberContract {
        nullable: false,
        value: MemberType::Datatype { declaration: id(n) },
    };
    let applied = |n, argument, member| AppliedConversionContract {
        application: ClosedType::new(ty(n, vec![argument])).unwrap(),
        conversion: ConversionContract {
            datatype: id(n),
            provider: ProviderId {
                name: "intersect.sequence".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![member],
        },
    };
    let sequence = applied(2, ty(0, vec![]), scalar(0));
    let alias = applied(3, ty(0, vec![]), scalar(0));
    let envelope = applied(
        4,
        alias.application.expression().clone(),
        MemberContract {
            nullable: false,
            value: MemberType::Applied {
                contract: Box::new(alias.clone()),
            },
        },
    );
    let name = if diamond { "Diamond" } else { "Defaults" };
    let mut grammar = DefinitionSource::new(
        "memory:wildcard-defaults.xtext",
        SourceKind::Xtext,
        format!(
            "grammar example.Defaults import \"urn:wildcard-defaults\" as d\nRoot returns d::{name}: 'root' name=ID ('values' values=STRING)?;"
        ),
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:wildcard-defaults-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let owner = RuntimeType::new(ty(if diamond { 8 } else { 9 }, vec![])).unwrap();
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone(), grammar.clone(), semantics.clone()],
        namespaces: vec![],
    };
    let constructors = if diamond {
        vec![]
    } else {
        [2, 3]
            .into_iter()
            .map(|n| {
                constructor_contract(
                    &bundle,
                    &id(n),
                    ProviderId {
                        name: "intersect.sequence.constructor".into(),
                        version: "1".into(),
                    },
                )
                .unwrap()
            })
            .collect()
    };
    Fixture {
        input: BundleCompilation {
            ecore: source.id,
            namespace: "urn:wildcard-defaults".into(),
            xtext: Some(grammar.id),
            rcore_behavior: semantics.id,
            materialized_types: vec![owner],
            bundle,
        },
        applications: if diamond {
            vec![sequence]
        } else {
            vec![envelope]
        },
        constructors,
        selected: 0,
    }
}
