use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{
        AppliedConversionContract, ConstructorContract, ConversionContract, MemberContract,
        MemberType, ProviderId,
    },
    generic_types::{ClosedType, Type},
};
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
pub struct Fixture {
    pub input: BundleCompilation,
    pub ordinary: Vec<ConversionContract>,
    pub applications: Vec<AppliedConversionContract>,
    pub constructors: Vec<ConstructorContract>,
    pub field: &'static str,
    pub value: Value,
    pub map: Value,
}
pub fn fixture(wrapped: bool) -> Fixture {
    let source = DefinitionSource::new(
        "memory:constructor-map",
        SourceKind::Ecore,
        include_str!("../../../../../fixtures/ecore-generics/constructor-map.ecore"),
    );
    let migration =
        intersect_model::migration::migrate_ecore(source, "memory:constructor-map.rcore").unwrap();
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
    let ty = |n, args| Type::Classifier {
        declaration: id(n),
        arguments: args,
        raw: false,
    };
    let provider = |name: &str| ProviderId {
        name: name.into(),
        version: "1".into(),
    };
    let codec = |n, name: &str, members| ConversionContract {
        datatype: id(n),
        provider: provider(name),
        serializable: false,
        declarations: vec![],
        members,
    };
    let scalar = |n| MemberContract {
        nullable: false,
        value: MemberType::Datatype { declaration: id(n) },
    };
    let alias = AppliedConversionContract {
        application: ClosedType::new(ty(3, vec![ty(0, vec![]), ty(1, vec![])])).unwrap(),
        conversion: codec(3, "ecore.map", vec![scalar(0), scalar(1)]),
    };
    let bounded = AppliedConversionContract {
        application: ClosedType::new(ty(4, vec![alias.application.expression().clone()])).unwrap(),
        conversion: codec(
            4,
            "ecore.list",
            vec![MemberContract {
                nullable: false,
                value: MemberType::Applied {
                    contract: Box::new(alias.clone()),
                },
            }],
        ),
    };
    let wrapper = codec(
        6,
        "ecore.list",
        vec![MemberContract {
            nullable: false,
            value: MemberType::Applied {
                contract: Box::new(bounded.clone()),
            },
        }],
    );
    let name = if wrapped { "OnlyWrapped" } else { "Holder" };
    let mut grammar = DefinitionSource::new(
        "memory:constructor-map.xtext",
        SourceKind::Xtext,
        format!(
            r#"grammar example.Maps import "urn:constructor-maps" as m
{name} returns m::{name}: 'holder' name=ID;"#
        ),
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:constructor-map-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone(), grammar.clone(), semantics.clone()],
        namespaces: vec![],
    };
    let constructors = [2, 3]
        .map(|n| {
            intersect_model::conversion::constructor_contract(
                &bundle,
                &id(n),
                provider("ecore.map.constructor"),
            )
            .unwrap()
        })
        .to_vec();
    let map = json!({"kind":"converted_application_v1","application":alias.application,"converted":{"kind":"converted","datatype":id(3),"provider":alias.conversion.provider,"value":{"entries":[{"key":{"kind":"datatype","datatype":id(0),"value":"key"},"value":{"kind":"datatype","datatype":id(1),"value":7}}]}}});
    let value = json!({"kind":"converted_application_v1","application":bounded.application,"converted":{"kind":"converted","datatype":id(4),"provider":bounded.conversion.provider,"value":{"items":[map]}}});
    Fixture {
        input: BundleCompilation {
            ecore: source.id.clone(),
            namespace: "urn:constructor-maps".into(),
            xtext: Some(grammar.id),
            rcore_behavior: semantics.id,
            materialized_types: vec![
                ClosedType::new(ty(if wrapped { 7 } else { 5 }, vec![]))
                    .unwrap()
                    .into(),
            ],
            bundle,
        },
        ordinary: if wrapped {
            vec![wrapper.clone()]
        } else {
            vec![]
        },
        applications: if wrapped {
            vec![]
        } else {
            vec![alias, bounded]
        },
        constructors,
        field: if wrapped { "wrapped" } else { "values" },
        value: if wrapped {
            json!({"kind":"converted","datatype":id(6),"provider":wrapper.provider,"value":{"items":[value]}})
        } else {
            value
        },
        map,
    }
}
