use intersect_model::conversion::constructor_contract;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{ConstructorContract, ConversionContract, ProviderId},
    generic_types::{RuntimeType, Type},
    resources::DeclarationId,
};
use intersect_text::package::BundleCompilation;
pub const SOURCE: &str = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="methodproviders" nsURI="urn:method-providers">
<eClassifiers x:type="e:EDataType" name="RGB" instanceClassName="example.RGB"/>
<eClassifiers x:type="e:EDataType" name="AliasRGB" instanceClassName="example.RGB"/>
<eClassifiers x:type="e:EDataType" name="List" instanceClassName="org.eclipse.emf.common.util.EList" serializable="false"><eTypeParameters name="T"/></eClassifiers>
<eClassifiers x:type="e:EDataType" name="AliasList" instanceClassName="org.eclipse.emf.common.util.EList" serializable="false"><eTypeParameters name="T"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Holder"><eTypeParameters name="T"/>
<eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eOperations name="ordinary"><eTypeParameters name="U"><eBounds eClassifier="#//RGB"/></eTypeParameters><eGenericType eTypeParameter="#//Holder/ordinary/U"/></eOperations>
<eOperations name="constructed"><eTypeParameters name="U"><eBounds eClassifier="#//List"><eTypeArguments eClassifier="#//RGB"/></eBounds></eTypeParameters><eGenericType eTypeParameter="#//Holder/constructed/U"/></eOperations>
</eClassifiers></e:EPackage>"##;
pub struct Fixture {
    pub input: BundleCompilation,
    pub conversions: Vec<ConversionContract>,
    pub constructors: Vec<ConstructorContract>,
    pub receiver: RuntimeType,
    pub operations: Vec<DeclarationId>,
    pub arguments: Vec<RuntimeType>,
}
pub fn fixture() -> Fixture {
    let source = DefinitionSource::new(
        "memory:runtime-operation-providers.ecore",
        SourceKind::Ecore,
        SOURCE,
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| declarations[&vec![n]].clone();
    let ty = |n, args, raw| Type::Classifier {
        declaration: id(n),
        arguments: args,
        raw,
    };
    let receiver = RuntimeType::new(ty(4, vec![], true)).unwrap();
    let arguments = vec![
        RuntimeType::new(ty(1, vec![], false)).unwrap(),
        RuntimeType::new(ty(3, vec![ty(0, vec![], false)], false)).unwrap(),
    ];
    let operations = vec![
        declarations[&vec![4, 2]].clone(),
        declarations[&vec![4, 3]].clone(),
    ];
    let conversions = [0, 1]
        .into_iter()
        .map(|n| ConversionContract {
            datatype: id(n),
            provider: ProviderId {
                name: "example.rgb".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![],
        })
        .collect();
    let mut grammar = DefinitionSource::new(
        "memory:runtime-operation-providers.xtext",
        SourceKind::Xtext,
        "grammar example.MethodProviders import \"urn:method-providers\" as m Root returns m::Holder: 'holder' name=ID;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:runtime-operation-providers-behavior.rcore",
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
        .into_iter()
        .map(|n| {
            constructor_contract(
                &bundle,
                &id(n),
                ProviderId {
                    name: "ecore.list.constructor".into(),
                    version: "1".into(),
                },
            )
            .unwrap()
        })
        .collect();
    let input = BundleCompilation {
        bundle,
        ecore: source.id,
        namespace: "urn:method-providers".into(),
        xtext: Some(grammar.id),
        rcore_behavior: semantics.id,
        materialized_types: vec![receiver.clone()],
    };
    Fixture {
        input,
        conversions,
        constructors,
        receiver,
        operations,
        arguments,
    }
}
