use intersect_text::package::BundleCompilation;
use intersect_text::package::Package;
#[path = "common/eobject_construction.rs"]
mod common;
use intersect_model::definitions::schema::{
    EOBJECT_REFERENCE_TOP, eobject_reference_top_declaration,
};
use intersect_text::compiler::compile_bundle;
use intersect_text::evaluate;
use serde_json::json;
#[test]
fn pinned_object_and_explicit_subclasses_construct_with_all_source_operations() {
    let input = common::fixture();
    let package = compile_bundle(input.clone()).unwrap();
    let schema = &package.schema;
    let pinned = schema
        .class_key_for_type(&input.materialized_types[0])
        .unwrap();
    assert_ne!(pinned, EOBJECT_REFERENCE_TOP);
    assert!(!schema.classes.contains_key(EOBJECT_REFERENCE_TOP));
    let operations = schema.effective_operations(pinned).unwrap();
    assert_eq!(operations.len(), 15);
    let ids: std::collections::BTreeSet<_> = operations
        .iter()
        .map(|op| op.operation.declaration.clone())
        .collect();
    assert_eq!(ids.len(), 15);
    assert!(
        ids.iter()
            .all(|id| id.source == eobject_reference_top_declaration().unwrap().source)
    );
    assert_eq!(
        operations
            .iter()
            .filter(|op| op.operation.name == "eGet")
            .count(),
        2
    );
    for name in [
        "Child",
        schema
            .class_key_for_type(&input.materialized_types[1])
            .unwrap(),
        schema
            .class_key_for_type(&input.materialized_types[2])
            .unwrap(),
    ] {
        let key = schema.language_class_key(name).unwrap();
        assert_eq!(
            schema
                .effective_operations(key)
                .unwrap()
                .iter()
                .map(|op| op.operation.declaration.clone())
                .collect::<std::collections::BTreeSet<_>>(),
            ids
        );
    }
    let local =
        intersect_text::grammar::type_names::reference_key(schema, &schema.uri, "EObject").unwrap();
    assert_ne!(local, pinned);
    for name in ["Other", "EObject", "Root"] {
        let key = schema.language_class_key(name).unwrap();
        assert!(schema.effective_operations(key).unwrap().is_empty());
        assert!(schema.conforms(key, pinned));
        assert!(schema.conforms(key, EOBJECT_REFERENCE_TOP));
    }
    let result = evaluate(&package, &[common::source()]);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 7);
    result.model.validate(schema).unwrap();
    let object = result
        .model
        .elements
        .iter()
        .find(|e| e.class == pinned)
        .unwrap();
    assert!(object.values.is_empty());
    let root = &result.model.elements[0];
    let mut model = result.model.clone();
    model
        .edit(schema, &[(root.id.clone(), "any".into(), json!(object.id))])
        .unwrap();
    let before = model.clone();
    assert!(
        model
            .edit(
                schema,
                &[
                    (root.id.clone(), "any".into(), json!(root.id)),
                    (root.id.clone(), "local".into(), json!(object.id))
                ]
            )
            .is_err()
    );
    assert_eq!(model, before);
    let model: intersect_model::core::model::Model = serde_json::from_value(json!(model)).unwrap();
    model.validate(schema).unwrap();
    let retained: Package = serde_json::from_value(json!(package)).unwrap();
    let rebuilt = compile_bundle(retained.compilation.clone().unwrap()).unwrap();
    assert_eq!(json!(rebuilt), json!(retained));
    assert_eq!(
        json!(evaluate(&rebuilt, &[common::source()])),
        json!(result)
    );
    model.validate(&rebuilt.schema).unwrap();
}
#[test]
fn pinned_construction_stays_distinct_from_local_type_and_reference_alias() {
    let input = common::fixture();
    let package = compile_bundle(input.clone()).unwrap();
    let bad = intersect_text::Source {
        uri: "memory:wrong.model".into(),
        text: "object child C local C".into(),
    };
    assert!(!evaluate(&package, &[bad]).diagnostics.is_empty());
    let key = package
        .schema
        .class_key_for_type(&input.materialized_types[0])
        .unwrap();
    let mut model = intersect_model::core::model::Model::default();
    assert!(
        model
            .create(
                &package.schema,
                "bad",
                EOBJECT_REFERENCE_TOP,
                "memory:model",
                0,
                0
            )
            .is_err()
    );
    model
        .create(&package.schema, "real", key, "memory:model", 0, 0)
        .unwrap();
    let mut stale = input.materialized_types[0].expression().clone();
    if let intersect_model::definitions::generic_types::Type::Classifier { declaration, .. } =
        &mut stale
    {
        declaration.source.sha256 = "0".repeat(64);
    }
    let stale = intersect_model::definitions::generic_types::RuntimeType::new(stale).unwrap();
    assert!(package.schema.class_key_for_type(&stale).is_err());
}

#[test]
fn explicit_eobject_diamond_and_override_keep_source_selection_in_text_models() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        generic_types::{RuntimeType, Type},
    };
    let source = DefinitionSource::new(
        "memory:eobject-diamond.ecore",
        SourceKind::Ecore,
        r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="diamond" nsURI="urn:eobject-diamond">
<eClassifiers x:type="e:EClass" name="Base" eSuperTypes="http://www.eclipse.org/emf/2002/Ecore#//EObject"><eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Left" eSuperTypes="#//Base"/><eClassifiers x:type="e:EClass" name="Right" eSuperTypes="#//Base"/><eClassifiers x:type="e:EClass" name="Diamond" eSuperTypes="#//Left #//Right"/>
<eClassifiers x:type="e:EClass" name="Override" eSuperTypes="#//Diamond"><eOperations name="eIsProxy" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBoolean"/></eClassifiers>
</e:EPackage>"##,
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let mut grammar = DefinitionSource::new(
        "memory:eobject-diamond.xtext",
        SourceKind::Xtext,
        "grammar example.Diamond import \"urn:eobject-diamond\" as m Root returns m::Override: 'override' name=ID;",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:eobject-diamond-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let receiver = RuntimeType::new(Type::Classifier {
        declaration: declarations[&vec![4]].clone(),
        arguments: vec![],
        raw: false,
    })
    .unwrap();
    let pinned = RuntimeType::new(Type::Classifier {
        declaration: eobject_reference_top_declaration().unwrap(),
        arguments: vec![],
        raw: false,
    })
    .unwrap();
    let input = BundleCompilation {
        ecore: source.id.clone(),
        namespace: "urn:eobject-diamond".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: vec![pinned, receiver.clone()],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![source.clone(), grammar, semantics],
            namespaces: vec![],
        },
    };
    let package = compile_bundle(input.clone()).unwrap();
    let schema = &package.schema;
    assert_eq!(
        schema
            .effective_operations(schema.language_class_key("Diamond").unwrap())
            .unwrap()
            .len(),
        15
    );
    let key = schema.class_key_for_type(&receiver).unwrap();
    assert_eq!(schema.effective_operations(key).unwrap().len(), 16);
    let resolved = schema.resolved_operations(key).unwrap();
    assert_eq!(resolved.len(), 15);
    let proxy = resolved
        .iter()
        .find(|r| r.selected.operation.name == "eIsProxy")
        .unwrap();
    assert_eq!(
        proxy.selected.operation.declaration,
        declarations[&vec![4, 0]]
    );
    assert_eq!(proxy.overridden.len(), 1);
    let text = intersect_text::Source {
        uri: "memory:diamond.model".into(),
        text: "override Named".into(),
    };
    let result = evaluate(&package, std::slice::from_ref(&text));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    result.model.validate(schema).unwrap();
    let rebuilt = compile_bundle(serde_json::from_value(json!(input)).unwrap()).unwrap();
    assert_eq!(json!(evaluate(&rebuilt, &[text])), json!(result));
}
