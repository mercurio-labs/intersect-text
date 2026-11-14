#[allow(dead_code)]
#[path = "common/serializable_sequence.rs"]
mod common;
use intersect_model::conversion::with_catalog_context;
use intersect_model::core::model::Model;
use intersect_model::definitions::{
    compiler::materialize_runtime_types,
    generic_types::{RuntimeType, Type},
};
use serde_json::json;

#[test]
fn nested_raw_attribute_applications_keep_exact_tags_and_unresolved_evidence() {
    let extra = r##"<eClassifiers x:type="e:EClass" name="RawOwner"><eTypeParameters name="T"><eBounds eClassifier="#//Text"/></eTypeParameters>
    <eStructuralFeatures x:type="e:EAttribute" name="data"><eGenericType eClassifier="#//Sequence"><eTypeArguments eTypeParameter="#//RawOwner/T"/></eGenericType></eStructuralFeatures>
    <eStructuralFeatures x:type="e:EAttribute" name="nested"><eGenericType eClassifier="#//Sequence"><eTypeArguments eClassifier="#//Sequence"><eTypeArguments eTypeParameter="#//RawOwner/T"/></eTypeArguments></eGenericType></eStructuralFeatures>
    </eClassifiers>"##;
    let text = include_str!("../../../../fixtures/ecore-generics/serializable-sequence.ecore")
        .replace("</e:EPackage>", &format!("{extra}</e:EPackage>"));
    let (input, contracts) = common::from_text(&text);
    let owner = RuntimeType::new(Type::Classifier {
        declaration: intersect_text_acceptance::migration_fixture::named_declaration_fixture(
            input
                .bundle
                .sources
                .iter()
                .find(|source| source.id == input.ecore)
                .unwrap(),
            "RawOwner",
        ),
        arguments: vec![],
        raw: true,
    })
    .unwrap();
    with_catalog_context(&input.bundle, &[], &contracts, |registry| {
        let schema =
            materialize_runtime_types(&input.bundle, std::slice::from_ref(&owner), Some(registry))
                .map_err(|e| e.to_string())?;
        let key = schema.class_key_for_type(&owner)?;
        let domains = schema.feature_domains(key, "data");
        assert!(!domains[0].unresolved.is_empty());
        assert_eq!(schema.runtime_attribute_lexical_target(key, "data")?, None);
        assert!(
            schema
                .parse_runtime_attribute(key, "data", "[]", Some(registry))
                .is_err()
        );
        let mut model = Model::default();
        model.create(&schema, "r", key, "memory:model", 0, 0)?;
        assert!(model.elements[0].values["data"].is_null());
        let value = json!(
            registry
                .applied()
                .parse(&contracts[0].application, r#"["word"]"#)?
        );
        let nested = json!(registry.applied().parse(
            &contracts[2].application,
            &serde_json::to_string(&vec![r#"["inside"]"#]).map_err(|e| e.to_string())?
        )?);
        model.edit_with_conversions(
            &schema,
            &[
                ("r".into(), "data".into(), value.clone()),
                ("r".into(), "nested".into(), nested.clone()),
            ],
            registry,
        )?;
        assert_eq!(model.elements[0].values["data"], value);
        assert_eq!(model.elements[0].values["nested"], nested);
        let before = model.clone();
        let error = model
            .edit_with_conversions(&schema, &[("r".into(), "data".into(), nested)], registry)
            .unwrap_err();
        assert!(
            error.contains("Unresolved runtime feature membership"),
            "{error}"
        );
        assert_eq!(model, before);
        let wrong = json!(
            registry
                .applied()
                .parse(&contracts[1].application, r#"["7"]"#)?
        );
        assert!(
            model
                .edit_with_conversions(&schema, &[("r".into(), "data".into(), wrong)], registry)
                .is_err()
        );
        assert_eq!(model, before);
        model.validate_with_conversions(&schema, registry)?;
        let replay =
            materialize_runtime_types(&input.bundle, std::slice::from_ref(&owner), Some(registry))
                .map_err(|e| e.to_string())?;
        assert_eq!(schema, replay);
        model.validate_with_conversions(&replay, registry)?;
        Ok(())
    })
    .unwrap();
}
