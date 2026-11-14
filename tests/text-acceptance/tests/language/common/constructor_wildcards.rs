use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{
        AppliedConversionContract, ConstructorContract, ConversionContract, MemberContract,
        MemberType, ProviderId,
    },
    generic_types::{ClosedType, RuntimeType, Type},
    resources::DeclarationId,
};
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
#[derive(Clone)]
pub struct Spec(pub usize, pub Vec<Spec>);
pub fn scalar(n: usize) -> Spec {
    Spec(n, vec![])
}
pub fn app(n: usize, args: Vec<Spec>) -> Spec {
    Spec(n, args)
}
const NAMES: [&str; 10] = [
    "Text",
    "Number",
    "EList",
    "Alias",
    "List",
    "Collection",
    "Map",
    "MapAlias",
    "Bounded",
    "Holder",
];
impl Spec {
    fn ty(&self, ids: &std::collections::BTreeMap<Vec<usize>, DeclarationId>) -> Type {
        Type::Classifier {
            declaration: ids[&vec![self.0]].clone(),
            arguments: self.1.iter().map(|a| a.ty(ids)).collect(),
            raw: false,
        }
    }
    pub fn xml(&self, tag: &str) -> String {
        format!(
            "<{tag} eClassifier=\"#//{}\">{}</{tag}>",
            NAMES[self.0],
            self.1
                .iter()
                .map(|a| a.xml("eTypeArguments"))
                .collect::<String>()
        )
    }
}
fn provider(n: usize) -> &'static str {
    match n {
        4 => "java.list",
        5 => "java.collection",
        6 | 7 => "ecore.map",
        _ => "ecore.list",
    }
}
pub struct Fixture {
    pub input: BundleCompilation,
    pub applications: Vec<AppliedConversionContract>,
    pub constructors: Vec<ConstructorContract>,
    pub value: Value,
}
pub fn fixture(bound: &str, actual: Spec, policies: &[usize]) -> Fixture {
    let original =
        include_str!("../../../../../fixtures/ecore-generics/constructor-wildcards.ecore");
    let default_bound = "<eBounds eClassifier=\"#//EList\"><eTypeArguments/></eBounds>";
    let default_actual = "<eTypeArguments eClassifier=\"#//Alias\"><eTypeArguments eClassifier=\"#//Text\"/></eTypeArguments>";
    let text = original
        .replace(default_bound, bound)
        .replace(default_actual, &actual.xml("eTypeArguments"));
    let source = DefinitionSource::new(
        "memory:constructor-wildcards.ecore",
        SourceKind::Ecore,
        text,
    );
    let (source, ids) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| ids[&vec![n]].clone();
    fn contract(
        spec: &Spec,
        ids: &std::collections::BTreeMap<Vec<usize>, DeclarationId>,
    ) -> AppliedConversionContract {
        AppliedConversionContract {
            application: ClosedType::new(spec.ty(ids)).unwrap(),
            conversion: ConversionContract {
                datatype: ids[&vec![spec.0]].clone(),
                provider: ProviderId {
                    name: provider(spec.0).into(),
                    version: "1".into(),
                },
                serializable: false,
                declarations: vec![],
                members: spec
                    .1
                    .iter()
                    .map(|a| MemberContract {
                        nullable: false,
                        value: if a.1.is_empty() {
                            MemberType::Datatype {
                                declaration: ids[&vec![a.0]].clone(),
                            }
                        } else {
                            MemberType::Applied {
                                contract: Box::new(contract(a, ids)),
                            }
                        },
                    })
                    .collect(),
            },
        }
    }
    fn value(spec: &Spec, ids: &std::collections::BTreeMap<Vec<usize>, DeclarationId>) -> Value {
        if spec.1.is_empty() {
            return json!({"kind":"datatype","datatype":ids[&vec![spec.0]],"value":if spec.0==0 {json!("text")}else{json!(7)}});
        }
        let c = contract(spec, ids);
        let payload = if spec.0 == 6 || spec.0 == 7 {
            json!({"entries":[{"key":value(&spec.1[0],ids),"value":value(&spec.1[1],ids)}]})
        } else {
            json!({"items":[value(&spec.1[0],ids)]})
        };
        json!({"kind":"converted_application_v1","application":c.application,"converted":{"kind":"converted","datatype":c.conversion.datatype,"provider":c.conversion.provider,"value":payload}})
    }
    let root = app(8, vec![actual]);
    let applied = contract(&root, &ids);
    let value = value(&root, &ids);
    let mut grammar = DefinitionSource::new(
        "memory:constructor-wildcards.xtext",
        SourceKind::Xtext,
        "grammar example.Wildcards import \"urn:constructor-wildcards\" as w\nHolder returns w::Holder: 'holder' name=ID;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:constructor-wildcards-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone(), grammar.clone(), semantics.clone()],
        namespaces: vec![],
    };
    let constructors = policies
        .iter()
        .map(|n| {
            intersect_model::conversion::constructor_contract(
                &bundle,
                &id(*n),
                ProviderId {
                    name: format!("{}.constructor", provider(*n)),
                    version: "1".into(),
                },
            )
            .unwrap()
        })
        .collect();
    Fixture {
        input: BundleCompilation {
            ecore: source.id.clone(),
            namespace: "urn:constructor-wildcards".into(),
            xtext: Some(grammar.id),
            rcore_behavior: semantics.id,
            materialized_types: vec![RuntimeType::new(scalar(9).ty(&ids)).unwrap()],
            bundle,
        },
        applications: vec![applied],
        constructors,
        value,
    }
}
