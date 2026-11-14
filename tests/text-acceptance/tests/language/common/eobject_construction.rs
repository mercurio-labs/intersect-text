use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::materialize_runtime_types,
    generic_types::{RuntimeType, Type},
    schema::eobject_reference_top_declaration,
};
use intersect_text::package::BundleCompilation;
pub const SOURCE: &str = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="constructed" nsURI="urn:eobject-construction">
<eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="objects" eType="e:EClass http://www.eclipse.org/emf/2002/Ecore#//EObject" containment="true" upperBound="-1"/><eStructuralFeatures x:type="e:EReference" name="any" eType="e:EClass http://www.eclipse.org/emf/2002/Ecore#//EObject"/><eStructuralFeatures x:type="e:EReference" name="local" eType="#//EObject"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Child" eSuperTypes="http://www.eclipse.org/emf/2002/Ecore#//EObject"><eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Generic" eSuperTypes="http://www.eclipse.org/emf/2002/Ecore#//EObject"><eTypeParameters name="T"/><eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Other"><eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="EObject"><eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
</e:EPackage>"##;
pub fn fixture() -> BundleCompilation {
    let source = DefinitionSource::new(
        "memory:eobject-construction.ecore",
        SourceKind::Ecore,
        SOURCE,
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let ty = |n, args, raw| Type::Classifier {
        declaration: declarations[&vec![n]].clone(),
        arguments: args,
        raw,
    };
    let pinned = RuntimeType::new(Type::Classifier {
        declaration: eobject_reference_top_declaration().unwrap(),
        arguments: vec![],
        raw: false,
    })
    .unwrap();
    let raw = RuntimeType::new(ty(2, vec![], true)).unwrap();
    let applied = RuntimeType::new(ty(2, vec![ty(3, vec![], false)], false)).unwrap();
    let mut requested = vec![pinned, raw.clone(), applied.clone()];
    requested.extend(
        [0, 1, 3, 4]
            .into_iter()
            .map(|n| RuntimeType::new(ty(n, vec![], false)).unwrap()),
    );
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let schema = materialize_runtime_types(&bundle, &requested, None).unwrap();
    let grammar = format!(
        "grammar example.Constructed import \"urn:eobject-construction\" as m import \"http://www.eclipse.org/emf/2002/Ecore\" as e Root returns m::Root: (objects+=Object | objects+=Child | objects+=Raw | objects+=Applied | objects+=Other | objects+=Local)* ('any' any=[e::EObject|ID])? ('local' local=[m::EObject|ID])?; Object returns e::EObject: 'object'; Child returns m::Child: 'child' name=ID; Raw returns m::{}: 'raw' name=ID; Applied returns m::{}: 'applied' name=ID; Other returns m::Other: 'other' name=ID; Local returns m::EObject: 'userobject' name=ID;",
        schema.class_key_for_type(&raw).unwrap(),
        schema.class_key_for_type(&applied).unwrap()
    );
    let mut grammar = DefinitionSource::new(
        "memory:eobject-construction.xtext",
        SourceKind::Xtext,
        grammar,
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:eobject-construction-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    BundleCompilation {
        ecore: source.id.clone(),
        namespace: "urn:eobject-construction".into(),
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
    intersect_text::Source {
        uri: "memory:constructed.model".into(),
        text: "object child C raw R applied A other O userobject U any O local U".into(),
    }
}
