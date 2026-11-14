use intersect_text::package::Package;
#[path = "common/serializable_sequence.rs"]
mod common;
use intersect_model::conversion::with_catalog_context;
use intersect_model::core::model::Model;
use intersect_model::definitions::conversion::{AppliedConvertedValue, MemberType};
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use serde_json::json;

#[test]
fn serializable_applications_consume_defaults_text_and_replayed_packages() {
    let (input, contracts) = common::input();
    let package = with_catalog_context(&input.bundle, &[], &contracts, |registry| {
        let package = compile_bundle_with_conversions(input.clone(), registry)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(package.schema.applied_conversion_contracts.len(), 3);
        let mut model = Model::default();
        model.create(
            &package.schema,
            "h",
            package.schema.language_class_key("Holder")?,
            "memory:model",
            0,
            0,
        )?;
        model.validate_with_conversions(&package.schema, registry)?;
        let initial = registry
            .applied()
            .parse(&contracts[0].application, r#"["initial"]"#)?;
        assert_eq!(model.elements[0].values["texts"], json!(initial));
        let empty = registry
            .applied()
            .default_value(&contracts[1].application)?;
        assert_eq!(model.elements[0].values["numbers"], json!(empty));
        assert!(model.elements[0].values["wide"].is_null());
        let outcome = evaluate_with_conversions(
            &package,
            &[common::source()],
            EvaluationLimits::native(),
            registry,
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        outcome
            .model
            .validate_with_conversions(&package.schema, registry)?;
        let values = &outcome.model.elements[0].values;
        for (index, field, lexical) in [
            (0, "texts", r#"["hello"]"#),
            (1, "numbers", r#"["2147483648"]"#),
        ] {
            let value: AppliedConvertedValue =
                serde_json::from_value(values[field].clone()).unwrap();
            assert_eq!(registry.applied().format(&value)?, lexical);
            assert_eq!(
                json!(
                    registry
                        .applied()
                        .parse(&contracts[index].application, lexical)?
                ),
                values[field]
            );
        }
        let nested: AppliedConvertedValue =
            serde_json::from_value(values["nested"].clone()).unwrap();
        assert_eq!(
            registry.applied().format(&nested)?,
            serde_json::to_string(&vec![r#"["inside"]"#]).unwrap()
        );
        assert_eq!(values["many"].as_array().unwrap().len(), 2);
        assert_eq!(
            values["wide"]["value"],
            json!({"kind":"int64","value":"2147483648"})
        );
        assert_eq!(values["word"]["value"], "Label");
        let before = model.clone();
        assert!(
            model
                .edit_with_conversions(
                    &package.schema,
                    &[
                        ("h".into(), "name".into(), json!("rollback")),
                        ("h".into(), "texts".into(), values["numbers"].clone())
                    ],
                    registry
                )
                .is_err()
        );
        assert_eq!(model, before);
        for bad in [r#"[12]"#, r#"[null]"#, r#"{}"#, r#"["not a number"]"#] {
            assert!(
                registry
                    .applied()
                    .parse(&contracts[1].application, bad)
                    .is_err()
            );
        }
        for bad in ["not json", r#"["not a number"]"#] {
            let quoted = serde_json::to_string(bad).unwrap();
            let text = format!("holder Bad numbers {quoted}");
            let start = text.find(&quoted).unwrap();
            let bad = evaluate_with_conversions(
                &package,
                &[Source {
                    uri: "memory:bad".into(),
                    text: text.clone(),
                }],
                EvaluationLimits::native(),
                registry,
            );
            assert!(!bad.diagnostics.is_empty());
            assert_eq!(bad.diagnostics[0].start, start);
            assert_eq!(bad.diagnostics[0].end, text.len());
            assert!(
                bad.diagnostics[0]
                    .message
                    .contains("Invalid Sequence literal"),
                "{:?}",
                bad.diagnostics
            );
            assert!(bad.model.elements.is_empty());
        }
        Ok(package)
    })
    .unwrap();
    let retained: Package = serde_json::from_slice(&serde_json::to_vec(&package).unwrap()).unwrap();
    let replay = retained.compilation.clone().unwrap();
    with_catalog_context(
        &replay.bundle,
        &retained.schema.conversion_contracts,
        &retained.schema.applied_conversion_contracts,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(replay.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(rebuilt), json!(retained));
            let evaluated = evaluate_with_conversions(
                &rebuilt,
                &[common::source()],
                EvaluationLimits::native(),
                registry,
            );
            assert!(evaluated.diagnostics.is_empty());
            evaluated
                .model
                .validate_with_conversions(&rebuilt.schema, registry)
        },
    )
    .unwrap();
}

#[test]
fn serializable_selection_is_exact_nullable_and_source_checked() {
    let (input, contracts) = common::input();
    for change in 0..5 {
        let mut invalid = contracts.clone();
        match change {
            0 => invalid[0].conversion.provider.version = "stale".into(),
            1 => invalid[0].conversion.serializable = false,
            2 => invalid[0].conversion.members[0] = invalid[1].conversion.members[0].clone(),
            3 => invalid[0].conversion.members.clear(),
            _ => invalid[0].conversion.datatype.path = vec![1],
        }
        assert!(with_catalog_context(&input.bundle, &[], &invalid, |_| Ok(())).is_err());
    }
    assert!(
        with_catalog_context(&input.bundle, &[], &contracts[1..], |registry| {
            compile_bundle_with_conversions(input.clone(), registry).map_err(|e| format!("{e:?}"))
        })
        .is_err()
    );
    let mut nullable = contracts[0].clone();
    nullable.conversion.members[0].nullable = true;
    with_catalog_context(&input.bundle, &[], &[nullable.clone()], |registry| {
        let value = registry.applied().parse(&nullable.application, "[null]")?;
        assert_eq!(registry.applied().format(&value)?, "[null]");
        assert_ne!(
            json!(value),
            json!(registry.applied().default_value(&nullable.application)?)
        );
        Ok(())
    })
    .unwrap();
    let mut wrong_nested = contracts[2].clone();
    if let MemberType::Applied { contract } = &mut wrong_nested.conversion.members[0].value {
        **contract = contracts[1].clone();
    }
    assert!(with_catalog_context(&input.bundle, &[], &[wrong_nested], |_| Ok(())).is_err());
}

#[test]
fn lexical_member_codecs_cover_enum_converted_and_builtin_domains() {
    use intersect_model::definitions::{
        conversion::{ConversionContract, MemberContract, ProviderId},
        generic_types::{ClosedType, Type},
    };
    let (input, contracts) = common::input();
    let resources =
        intersect_model::definitions::resources::ResourceSet::new(&input.bundle).unwrap();
    let id = |name: &str| {
        resources
            .resolve(&input.ecore, &format!("#//{name}"))
            .unwrap()
    };
    let mut enumeration = contracts[0].clone();
    enumeration.application = ClosedType::new(Type::Classifier {
        declaration: id("Sequence"),
        arguments: vec![Type::Classifier {
            declaration: id("Color"),
            arguments: vec![],
            raw: false,
        }],
        raw: false,
    })
    .unwrap();
    enumeration.conversion.members = vec![MemberContract {
        nullable: false,
        value: MemberType::Enumeration {
            declaration: id("Color"),
        },
    }];
    let mut converted = enumeration.clone();
    converted.application = ClosedType::new(Type::Classifier {
        declaration: id("Sequence"),
        arguments: vec![Type::Classifier {
            declaration: id("RGB"),
            arguments: vec![],
            raw: false,
        }],
        raw: false,
    })
    .unwrap();
    converted.conversion.members = vec![MemberContract {
        nullable: false,
        value: MemberType::Converted {
            contract: Box::new(ConversionContract {
                datatype: id("RGB"),
                provider: ProviderId {
                    name: "example.rgb".into(),
                    version: "1".into(),
                },
                serializable: true,
                declarations: vec![],
                members: vec![],
            }),
        },
    }];
    let standard = intersect_model::definitions::compiler::standard_datatype_source();
    let bundle = intersect_model::definitions::bundle::DefinitionBundle {
        version: 1,
        sources: vec![standard.clone()],
        namespaces: vec![],
    };
    let resources =
        intersect_model::definitions::resources::ResourceSet::new_diagnostic(&bundle).unwrap();
    let builtin_id = resources
        .resolve(&standard.id, "#//EIntegerObject")
        .unwrap();
    let mut builtin = enumeration.clone();
    builtin.application = ClosedType::new(Type::Classifier {
        declaration: id("Sequence"),
        arguments: vec![Type::Classifier {
            declaration: builtin_id,
            arguments: vec![],
            raw: false,
        }],
        raw: false,
    })
    .unwrap();
    builtin.conversion.members = vec![MemberContract {
        nullable: false,
        value: MemberType::Builtin {
            codec: intersect_model::definitions::scalars::Builtin::EIntegerObject,
        },
    }];
    with_catalog_context(
        &input.bundle,
        &[],
        &[enumeration.clone(), converted.clone(), builtin.clone()],
        |registry| {
            for (contract, text, formatted) in [
                (&enumeration, r#"["red","blue"]"#, r#"["red","blue"]"#),
                (&converted, r##"["#aabbcc"]"##, r##"["#AABBCC"]"##),
                (&builtin, r#"["00012"]"#, r#"["12"]"#),
            ] {
                let value = registry.applied().parse(&contract.application, text)?;
                assert_eq!(registry.applied().format(&value)?, formatted);
            }
            assert!(
                registry
                    .applied()
                    .parse(&enumeration.application, r#"["green"]"#)
                    .is_err()
            );
            assert!(
                registry
                    .applied()
                    .parse(&converted.application, r#"["bad"]"#)
                    .is_err()
            );
            Ok(())
        },
    )
    .unwrap();
    let text = include_str!("../../../../fixtures/ecore-generics/serializable-sequence.ecore");
    for wrong in [
        text.replace("[&quot;initial&quot;]", "invalid"),
        text.replace("intersect.Sequence", "java.util.List"),
    ] {
        let (input, contracts) = common::from_text(&wrong);
        assert!(
            with_catalog_context(&input.bundle, &[], &contracts, |registry| {
                compile_bundle_with_conversions(input.clone(), registry)
                    .map_err(|e| format!("{e:?}"))
            })
            .is_err()
        );
    }
    let mut stale = contracts[0].clone();
    stale.conversion.datatype.source.sha256 = "0".repeat(64);
    assert!(with_catalog_context(&input.bundle, &[], &[stale], |_| Ok(())).is_err());
}

#[test]
fn converted_speculation_rolls_back_and_recursive_catalog_budgets_remain_checked() {
    use intersect_model::definitions::conversion::AppliedConversionContract;
    use intersect_model::definitions::conversion::MemberContract;
    use intersect_model::definitions::generic_types::ClosedType;
    use intersect_model::definitions::generic_types::Type;
    use intersect_text::grammar::Expr;
    use intersect_text::grammar::Kind;
    let (input, contracts) = common::input();
    with_catalog_context(&input.bundle, &[], &contracts, |registry| {
        let mut package = compile_bundle_with_conversions(input.clone(), registry)
            .map_err(|e| format!("{e:?}"))?;
        let expr = |kind| Expr {
            kind,
            first_set_predicated: false,
            prediction_path: None,
            original_range: None,
            cardinality: '1',
        };
        let assignment = |feature: &str| {
            expr(Kind::Assign {
                feature: feature.into(),
                many: false,
                value: Box::new(expr(Kind::Call("STRING".into()))),
            })
        };
        let body = &mut package
            .grammar
            .as_mut()
            .unwrap()
            .rules
            .get_mut("Holder")
            .unwrap()
            .body;
        let Kind::Sequence(items) = &mut body.kind else {
            panic!("Expected sequence")
        };
        items.truncate(2);
        items.push(Expr {
            first_set_predicated: false,
            prediction_path: None,
            original_range: None,
            cardinality: '?',
            kind: Kind::Sequence(vec![
                expr(Kind::Literal("try".into())),
                assignment("texts"),
                expr(Kind::Literal("end".into())),
            ]),
        });
        items.push(Expr {
            first_set_predicated: false,
            prediction_path: None,
            original_range: None,
            cardinality: '?',
            kind: Kind::Sequence(vec![
                expr(Kind::Literal("try".into())),
                assignment("numbers"),
            ]),
        });
        let source = Source {
            uri: "memory:speculation".into(),
            text: r#"holder Trial try '["12"]'"#.into(),
        };
        let outcome = evaluate_with_conversions(
            &package,
            std::slice::from_ref(&source),
            EvaluationLimits::native(),
            registry,
        );
        assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
        assert_eq!(
            outcome.model.elements[0].values["texts"],
            json!(
                registry
                    .applied()
                    .parse(&contracts[0].application, r#"["initial"]"#)?
            )
        );
        assert_eq!(
            outcome.model.elements[0].values["numbers"],
            json!(
                registry
                    .applied()
                    .parse(&contracts[1].application, r#"["12"]"#)?
            )
        );
        let limited = evaluate_with_conversions(
            &package,
            &[source],
            EvaluationLimits {
                steps_per_resource: Some(4),
                ..EvaluationLimits::native()
            },
            registry,
        );
        assert!(
            limited
                .diagnostics
                .iter()
                .any(|d| d.message.contains("budget"))
        );
        assert!(limited.model.elements.is_empty());
        Ok(())
    })
    .unwrap();
    let wrap = |inner: AppliedConversionContract| {
        let mut outer = contracts[0].clone();
        outer.application = ClosedType::new(Type::Classifier {
            declaration: outer.conversion.datatype.clone(),
            arguments: vec![inner.application.expression().clone()],
            raw: false,
        })
        .unwrap();
        outer.conversion.members = vec![MemberContract {
            nullable: false,
            value: MemberType::Applied {
                contract: Box::new(inner),
            },
        }];
        outer
    };
    let mut deep = contracts[0].clone();
    for _ in 0..4 {
        deep = wrap(deep);
    }
    with_catalog_context(&input.bundle, &[], &[deep.clone()], |_| Ok(())).unwrap();
    for _ in 0..6 {
        deep = wrap(deep);
    }
    let error = with_catalog_context(&input.bundle, &[], &[deep], |_| Ok(())).unwrap_err();
    assert!(error.contains("budget"), "{error}");
    assert!(
        with_catalog_context(
            &input.bundle,
            &[],
            &vec![contracts[0].clone(); 129],
            |_| Ok(())
        )
        .unwrap_err()
        .contains("budget")
    );
}

#[test]
fn existing_nonserializable_collections_cannot_become_lexical_members() {
    use intersect_model::definitions::{
        conversion::{AppliedConversionContract, MemberContract},
        generic_types::{ClosedType, Type},
    };
    let (input, contracts) = common::input();
    let mut list = contracts[0].clone();
    list.conversion.datatype =
        intersect_model::definitions::resources::ResourceSet::new(&input.bundle)
            .unwrap()
            .resolve(&input.ecore, "#//List")
            .unwrap();
    list.conversion.provider.name = "ecore.list".into();
    list.conversion.serializable = false;
    let Type::Classifier { arguments, .. } = contracts[0].application.expression() else {
        panic!("Expected application")
    };
    list.application = ClosedType::new(Type::Classifier {
        declaration: list.conversion.datatype.clone(),
        arguments: arguments.clone(),
        raw: false,
    })
    .unwrap();
    with_catalog_context(&input.bundle, &[], &[list.clone()], |registry| {
        assert!(registry.applied().parse(&list.application, "[]").is_err());
        let value = registry
            .applied()
            .bind(&list.application, json!({"items":[]}))?;
        assert!(registry.applied().format(&value).is_err());
        Ok(())
    })
    .unwrap();
    let nested = AppliedConversionContract {
        application: ClosedType::new(Type::Classifier {
            declaration: contracts[0].conversion.datatype.clone(),
            arguments: vec![list.application.expression().clone()],
            raw: false,
        })
        .unwrap(),
        conversion: intersect_model::definitions::conversion::ConversionContract {
            members: vec![MemberContract {
                nullable: false,
                value: MemberType::Applied {
                    contract: Box::new(list),
                },
            }],
            ..contracts[0].conversion.clone()
        },
    };
    let error = with_catalog_context(&input.bundle, &[], &[nested], |_| Ok(())).unwrap_err();
    assert!(error.contains("no lexical codec"), "{error}");
}
