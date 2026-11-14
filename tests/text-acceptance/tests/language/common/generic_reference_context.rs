use intersect_model::conversion::{constructor_contract, with_catalog_policies};
use intersect_model::core::model::Model;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::{import_datatype_bundle, materialize_runtime_bundle},
    conversion::{
        AppliedConversionContract, ConversionContract, MemberContract, MemberType, ProviderId,
        ReferenceContext,
    },
    generic_types::{ClosedType, RuntimeType, Type},
};
use intersect_text::package::BundleCompilation;
use serde_json::json;
pub struct Fixture {
    pub input: BundleCompilation,
    pub ordinary: Vec<ConversionContract>,
    pub applications: Vec<AppliedConversionContract>,
    pub context: ReferenceContext,
    pub target: RuntimeType,
    pub raw_target: RuntimeType,
}
pub fn fixture() -> Fixture {
    let source = DefinitionSource::new(
        "memory:reference-context.ecore",
        SourceKind::Ecore,
        include_str!("../../../../../fixtures/ecore-generics/reference-context.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let ty = |n, arguments| Type::Classifier {
        declaration: id(n),
        arguments,
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
    let alias = AppliedConversionContract {
        application: ClosedType::new(ty(2, vec![ty(0, vec![])])).unwrap(),
        conversion: codec(
            2,
            "ecore.list",
            vec![MemberContract {
                nullable: false,
                value: MemberType::Datatype { declaration: id(0) },
            }],
        ),
    };
    let bounded = AppliedConversionContract {
        application: ClosedType::new(ty(3, vec![alias.application.expression().clone()])).unwrap(),
        conversion: codec(
            3,
            "ecore.list",
            vec![MemberContract {
                nullable: false,
                value: MemberType::Applied {
                    contract: Box::new(alias.clone()),
                },
            }],
        ),
    };
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let constructors = vec![
        constructor_contract(&bundle, &id(1), provider("ecore.list.constructor")).unwrap(),
        constructor_contract(&bundle, &id(2), provider("ecore.list.constructor")).unwrap(),
    ];
    let target = RuntimeType::new(ty(6, vec![ty(4, vec![])])).unwrap();
    let other = RuntimeType::new(ty(6, vec![ty(5, vec![])])).unwrap();
    let raw_target = RuntimeType::new(Type::Classifier {
        declaration: id(6),
        arguments: vec![],
        raw: true,
    })
    .unwrap();
    let sub = RuntimeType::new(ty(7, vec![])).unwrap();
    let plain = RuntimeType::new(ty(10, vec![])).unwrap();
    let materialized_types = vec![
        target.clone(),
        other.clone(),
        raw_target.clone(),
        sub.clone(),
        plain.clone(),
    ];
    let model=with_catalog_policies(&bundle,&[],std::slice::from_ref(&bounded),&constructors,|registry| {
        let schema=materialize_runtime_bundle(&bundle,&source.id,"urn:generic-refs",&materialized_types,Some(registry)).map_err(|e|e.to_string())?;
        let text=import_datatype_bundle(&bundle,&id(0)).map_err(|e|e.to_string())?.parse("retained")?;
        let alias_value=json!({"kind":"converted_application_v1","application":alias.application,"converted":{"kind":"converted","datatype":alias.conversion.datatype,"provider":alias.conversion.provider,"value":{"items":[text]}}});
        let value=registry.applied().bind(&bounded.application,json!({"items":[alias_value]}))?;
        let mut model=Model::default();
        for (name,ty) in [("value",&target),("other",&other),("raw",&raw_target),("sub",&sub),("plain",&plain)] {
            model.create(&schema,name,schema.class_key_for_type(ty)?,"memory:embedded",0,0)?;
            model.edit_with_conversions(&schema,&[(name.into(),"values".into(),json!(value))],registry)?;
        }
        model.validate_with_conversions(&schema,registry)?;
        serde_json::to_value(model).map_err(|e|e.to_string())
    }).unwrap();
    let context = ReferenceContext {
        bundle,
        entry: source.id.clone(),
        namespace: "urn:generic-refs".into(),
        conversions: vec![],
        materialized_types,
        applied_conversions: vec![bounded],
        constructors,
        model,
    };
    let member = MemberContract {
        nullable: false,
        value: MemberType::RuntimeReference {
            target: target.clone(),
            context: Box::new(context.clone()),
        },
    };
    let list = AppliedConversionContract {
        application: ClosedType::new(ty(1, vec![target.expression().clone()])).unwrap(),
        conversion: codec(1, "ecore.list", vec![member.clone()]),
    };
    let map = AppliedConversionContract {
        application: ClosedType::new(ty(11, vec![target.expression().clone(); 2])).unwrap(),
        conversion: codec(11, "ecore.map", vec![member; 2]),
    };
    let ordinary = vec![
        codec(
            9,
            "ecore.list",
            vec![MemberContract {
                nullable: false,
                value: MemberType::RuntimeReference {
                    target: raw_target.clone(),
                    context: Box::new(context.clone()),
                },
            }],
        ),
        codec(
            12,
            "ecore.list",
            vec![MemberContract {
                nullable: false,
                value: MemberType::Reference {
                    class: id(10),
                    context: Box::new(context.clone()),
                },
            }],
        ),
    ];
    let mut grammar = DefinitionSource::new(
        "memory:reference-context.xtext",
        SourceKind::Xtext,
        "grammar example.Refs import \"urn:generic-refs\" as r\nHolder returns r::Holder: 'holder' name=ID;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:reference-context-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let input = BundleCompilation {
        ecore: source.id.clone(),
        namespace: "urn:generic-refs".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: vec![RuntimeType::new(ty(8, vec![])).unwrap()],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![source.clone(), grammar, semantics],
            namespaces: vec![],
        },
    };
    Fixture {
        input,
        ordinary,
        applications: vec![list, map],
        context,
        target,
        raw_target,
    }
}
