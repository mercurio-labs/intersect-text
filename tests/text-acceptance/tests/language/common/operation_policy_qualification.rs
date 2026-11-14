use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{ConversionContract, ProviderId},
    generic_types::{RuntimeType, Type},
    resources::DeclarationId,
};
use intersect_text::package::BundleCompilation;
pub struct Fixture {
    pub input: BundleCompilation,
    pub conversions: Vec<ConversionContract>,
    pub receiver: RuntimeType,
    pub base: DeclarationId,
    pub child: DeclarationId,
}
pub fn fixture(generic: bool) -> Fixture {
    let parameter = |name| {
        if generic {
            format!("<eParameters name='p'><eGenericType eClassifier='#//{name}'/></eParameters>")
        } else {
            format!("<eParameters name='p' eType='#//{name}'/>")
        }
    };
    let source = DefinitionSource::new(
        "memory:operation-policy-provider.ecore",
        SourceKind::Ecore,
        format!(
            "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:x='http://www.w3.org/2001/XMLSchema-instance' name='policy' nsURI='urn:operation-policy-provider'><eClassifiers x:type='e:EDataType' name='RGB' instanceClassName='example.RGB'/><eClassifiers x:type='e:EDataType' name='Alias' instanceClassName='example.RGB'/><eClassifiers x:type='e:EClass' name='Base'><eOperations name='f'>{}</eOperations></eClassifiers><eClassifiers x:type='e:EClass' name='Child' eSuperTypes='#//Base'><eStructuralFeatures x:type='e:EAttribute' name='name' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString'/><eOperations name='f'>{}</eOperations></eClassifiers></e:EPackage>",
            parameter("RGB"),
            parameter("Alias")
        ),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |path: Vec<usize>| declarations[&path].clone();
    let receiver = RuntimeType::new(Type::Classifier {
        declaration: id(vec![3]),
        arguments: vec![],
        raw: false,
    })
    .unwrap();
    let conversions = [0, 1]
        .into_iter()
        .map(|n| ConversionContract {
            datatype: id(vec![n]),
            provider: ProviderId {
                name: "example.rgb".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![],
        })
        .collect();
    let base = id(vec![2, 0]);
    let child = id(vec![3, 1]);
    let mut grammar = DefinitionSource::new(
        "memory:operation-policy-provider.xtext",
        SourceKind::Xtext,
        "grammar policy.Operations import \"urn:operation-policy-provider\" as m Root returns m::Child: 'child' name=ID;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "memory:operation-policy-provider-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior.dependencies.push(source.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone(), grammar.clone(), rcore_behavior.clone()],
        namespaces: vec![],
    };
    Fixture {
        input: BundleCompilation {
            bundle,
            ecore: source.id,
            namespace: "urn:operation-policy-provider".into(),
            xtext: Some(grammar.id),
            rcore_behavior: rcore_behavior.id,
            materialized_types: vec![receiver.clone()],
        },
        conversions,
        receiver,
        base,
        child,
    }
}
