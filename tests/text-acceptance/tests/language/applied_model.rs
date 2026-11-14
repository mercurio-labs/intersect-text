#[path = "common/applied_model.rs"]
mod common;
use intersect_model::conversion::with_catalog_context;
use intersect_model::core::model::Model;
use intersect_model::definitions::{compiler::materialize_runtime_types, schema::Schema};
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use serde_json::json;
#[test]
fn source_materialized_applications_construct_edit_rollback_and_keep_operation_identity() {
    let (input, contracts) = common::input();
    with_catalog_context(&input.bundle, &[], &contracts, |registry| {
        let schema =
            materialize_runtime_types(&input.bundle, &input.materialized_types, Some(registry))
                .map_err(|e| e.to_string())?;
        assert_eq!(schema.applied_conversion_contracts.len(), 4);
        assert!(schema.conversion_contracts.is_empty());
        let key = schema.language_class_key("Holder")?;
        let inherited = schema.language_class_key("TextHolder")?;
        let mut model = Model::default();
        model.create(&schema, "h", key, "memory:model", 0, 0)?;
        model.create(&schema, "inherited", inherited, "memory:model", 0, 0)?;
        model.validate_with_conversions(&schema, registry)?;
        assert!(model.validate(&schema).is_err());
        let word = common::nominal(&input, 0, json!("hello"));
        let number = common::nominal(&input, 1, json!(42));
        let text = registry
            .applied()
            .bind(&contracts[0].application, json!({"items":[word.clone()]}))?;
        let numeric = registry
            .applied()
            .bind(&contracts[1].application, json!({"items":[number.clone()]}))?;
        let nested = registry
            .applied()
            .bind(&contracts[2].application, json!({"items":[text.clone()]}))?;
        let map = registry.applied().bind(
            &contracts[3].application,
            json!({"entries":[{"key":word,"value":number}]}),
        )?;
        model.edit_with_conversions(
            &schema,
            &[
                ("h".into(), "texts".into(), json!(text)),
                ("h".into(), "numbers".into(), json!(numeric)),
                ("h".into(), "nested".into(), json!(nested)),
                ("h".into(), "mapping".into(), json!(map)),
                ("inherited".into(), "values".into(), json!(text)),
            ],
            registry,
        )?;
        model.edit_with_conversions(
            &schema,
            &[("h".into(), "manyTexts".into(), json!([text.clone()]))],
            registry,
        )?;
        let empty = registry
            .applied()
            .bind(&contracts[0].application, json!({"items":[]}))?;
        model.edit_with_conversions(
            &schema,
            &[("inherited".into(), "values".into(), json!(empty))],
            registry,
        )?;
        let before = serde_json::to_value(&model).unwrap();
        assert!(
            model
                .edit_with_conversions(
                    &schema,
                    &[
                        ("h".into(), "name".into(), json!("must roll back")),
                        ("h".into(), "texts".into(), json!(numeric))
                    ],
                    registry
                )
                .is_err()
        );
        assert_eq!(serde_json::to_value(&model).unwrap(), before);
        assert!(
            model
                .edit_with_conversions(
                    &schema,
                    &[(
                        "h".into(),
                        "manyTexts".into(),
                        json!([text.clone(), text.clone()])
                    )],
                    registry
                )
                .is_err()
        );
        assert_eq!(serde_json::to_value(&model).unwrap(), before);

        let operations = &schema.classes[key].operations;
        assert_ne!(
            schema.operation_signature(&operations[0])?.parameters,
            schema.operation_signature(&operations[1])?.parameters
        );
        let restored: Schema =
            serde_json::from_value(serde_json::to_value(&schema).unwrap()).unwrap();
        model.validate_with_conversions(&restored, registry)?;
        let mut forged = restored;
        forged.applied_conversion_contracts[0]
            .conversion
            .provider
            .version = "wrong".into();
        assert!(model.validate_with_conversions(&forged, registry).is_err());
        Ok(())
    })
    .unwrap();
}
#[test]
fn applied_package_replays_text_and_rejects_missing_provider_context() {
    let (input, contracts) = common::input();
    let package = with_catalog_context(&input.bundle, &[], &contracts, |registry| {
        compile_bundle_with_conversions(input.clone(), registry).map_err(|e| format!("{e:?}"))
    })
    .unwrap();
    let restored = serde_json::from_value(
        serde_json::to_value(package.compilation.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    with_catalog_context(
        &input.bundle,
        &[],
        &package.schema.applied_conversion_contracts,
        |registry| {
            let replay = compile_bundle_with_conversions(restored, registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                serde_json::to_value(&package).unwrap(),
                serde_json::to_value(&replay).unwrap()
            );
            let sources = [Source {
                uri: "one.model".into(),
                text: "holder example".into(),
            }];
            let result = evaluate_with_conversions(
                &replay,
                &sources,
                EvaluationLimits::workbench(),
                registry,
            );
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(result.model.elements.len(), 1);
            assert_eq!(result.model.elements[0].values["name"], "example");
            result
                .model
                .validate_with_conversions(&replay.schema, registry)?;
            Ok(())
        },
    )
    .unwrap();
    assert!(
        with_catalog_context(&input.bundle, &[], &contracts[..1], |registry| {
            compile_bundle_with_conversions(input.clone(), registry).map_err(|e| format!("{e:?}"))
        })
        .is_err()
    );
}
