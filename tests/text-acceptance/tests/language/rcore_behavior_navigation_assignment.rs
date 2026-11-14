use super::Store;
use intersect_model::core::storage::{self, ModelStore};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::compile_structure_bundle_with_conversions,
    conversion::{ConversionRegistry, ReferenceContext},
    rcore_behavior::{
        function_lowering::compile_function,
        ir_v2::{DataWire, EntrySelection, Limits, ValueWire},
        modules::SymbolId,
    },
};
use intersect_model::rcore_behavior_functions::{ExecutionContext, ExecutionLimits, evaluate};
use serde_json::{Value, json};

store_test! {
pub fn source_reads_replay_pinned_unsettable_scalar_and_many_assignment_values() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-feature-flags/emf-dynamic-observations.json"
    ))
    .unwrap();
    let mut checked = 0;
    for case in oracle["cases"].as_array().unwrap().iter().filter(|case| {
        case["changeable"] == true
            && case["unsettable"] == true
            && case["derived"] == false
            && case["volatile"] == false
            && case["transient"] == false
    }) {
        let many = case["many"].as_bool().unwrap();
        let upper = if many { "-1" } else { "1" };
        let default = if many {
            ""
        } else {
            "defaultValueLiteral=\"seed\""
        };
        let model = intersect_text_acceptance::migration_fixture::migrate_source_fixture(DefinitionSource::new(
            "memory:assignment.ecore",
            SourceKind::Ecore,
            format!(
                r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="assignment" nsURI="urn:assignment"><eClassifiers x:type="e:EClass" name="Base"><eStructuralFeatures x:type="e:EAttribute" name="value" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString" unsettable="true" upperBound="{upper}" {default}/></eClassifiers><eClassifiers x:type="e:EClass" name="Item" eSuperTypes="#//Base"/></e:EPackage>"##
            ),
        )).0;
        let model_bundle = DefinitionBundle {
            version: 1,
            sources: vec![model.clone()],
            namespaces: vec![],
        };
        let registry = ConversionRegistry::default();
        let schema = compile_structure_bundle_with_conversions(
            &model_bundle,
            &model.id,
            "urn:assignment",
            &registry,
        )
        .unwrap();
        let result_type = if many {
            "Sequence<String>"
        } else {
            "Option<String>"
        };
        let getter = if many { "Sequence(x.value)" } else { "x.value" };
        let mut source = DefinitionSource::new(
            "memory:assignment-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!(
                "rcore 1;package assignment version \"1\";import model \"urn:assignment\" as m;fn read(x:m.Base)->{result_type} ={{let read=|unused:Int8|{getter};read(0)}};"
            ),
        );
        source.dependencies.push(model.id.clone());
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), source.clone()],
            namespaces: vec![],
        };
        let image = compile_function(
            &bundle,
            &source.id,
            &EntrySelection {
                symbol: SymbolId {
                    source: source.id.clone(),
                    declaration: 1,
                },
                types: vec![],
                arguments: vec![],
            },
            &Limits::default(),
        )
        .unwrap();
        let parameter = image.program().functions[image.program().entry.0 as usize].parameters[0];
        let mut store = crate::stores::store::<Store>(&schema).unwrap();
        storage::atomic(&mut store, |store| {
            store.create_element(&schema, "object", "Item", "memory:model", 0, 0)
        })
        .unwrap();
        let mut fresh_digest = None;
        let mut fresh_model = None;
        for step in case["steps"].as_array().unwrap() {
            assert_eq!(step["accepted"], true);
            match step["operation"].as_str().unwrap() {
                "fresh" => (),
                "unset" | "unset-again" | "unset-default" => {
                    storage::unset(&mut store, &schema, "object", "value").unwrap()
                }
                _ => storage::edit(
                    &mut store,
                    &schema,
                    &[("object".into(), "value".into(), step["value"].clone())],
                )
                .unwrap(),
            }
            storage::validate(&store, &schema).unwrap();
            assert_eq!(
                json!(storage::is_explicitly_set(&store, &schema, "object", "value").unwrap()),
                step["is_set"]
            );
            let snapshot = storage::export(&store).unwrap();
            let reference = ReferenceContext {
                bundle: model_bundle.clone(),
                entry: model.id.clone(),
                namespace: "urn:assignment".into(),
                conversions: vec![],
                materialized_types: vec![],
                applied_conversions: vec![],
                constructors: vec![],
                model: serde_json::to_value(&snapshot).unwrap(),
            };
            let digest = reference.digest().unwrap();
            if step["operation"] == "fresh" {
                fresh_digest = Some(digest.clone());
                fresh_model = Some(snapshot.clone());
            }
            if step["operation"] == "explicit-default" {
                assert_ne!(Some(&digest), fresh_digest.as_ref());
            }
            if step["operation"] == "unset-default" {
                // Reset restores the exact value and assignment state, while the
                // newer resource revision still identifies a different snapshot.
                assert_ne!(Some(&digest), fresh_digest.as_ref());
                let fresh = fresh_model.as_ref().unwrap();
                let mut restored_content = snapshot.clone();
                for (uri, record) in &mut restored_content.resources {
                    let original = &fresh.resources[uri];
                    assert!(record.content_revision > original.content_revision);
                    record.content_revision = original.content_revision;
                }
                assert_eq!(&restored_content, fresh);
            }
            let before = reference.clone();
            let mut context = ExecutionContext::new(&bundle);
            context.references = std::slice::from_ref(&reference);
            let result = evaluate(
                &image,
                &[ValueWire {
                    ty: parameter,
                    data: DataWire::Model(
                        json!({"kind":"model_reference","context":digest,"element":"object"}),
                    ),
                }],
                &context,
                &ExecutionLimits::default(),
            )
            .unwrap();
            let observed = match result.value.data {
                DataWire::None => Value::Null,
                DataWire::Some(value) => {
                    let DataWire::Scalar(value) = value.data else {
                        panic!("scalar")
                    };
                    value.payload
                }
                DataWire::Collection(values) => json!(
                    values
                        .into_iter()
                        .map(|value| {
                            let DataWire::Scalar(value) = value.data else {
                                panic!("Many scalar")
                            };
                            value.payload
                        })
                        .collect::<Vec<_>>()
                ),
                _ => panic!("unexpected getter result"),
            };
            assert_eq!(observed, step["value"], "{many} {}", step["operation"]);
            assert_eq!(reference, before);
            checked += 1;
        }
    }
    assert_eq!(checked, 16);
}
}

control_test! {
pub fn provider_default_optional_and_many_reads_follow_checked_unset_transitions() {
    use intersect_model::conversion::with_catalog;
    use intersect_model::core::model::Model;
    use intersect_model::definitions::{
        conversion::{ConversionContract, ProviderId},
        rcore_behavior::function_lowering::compile_function_with_conversions,
    };
    let model =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(DefinitionSource::new(
            "memory:paint.ecore",
            SourceKind::Ecore,
            include_str!("../../../../fixtures/ecore-unsettable/provider.ecore"),
        ))
        .0;
    let model_bundle = DefinitionBundle {
        version: 1,
        sources: vec![model.clone()],
        namespaces: vec![],
    };
    let contract = ConversionContract {
        datatype: intersect_text_acceptance::migration_fixture::named_declaration_fixture(&model, "RGB"),
        provider: ProviderId {
            name: "example.rgb".into(),
            version: "1".into(),
        },
        serializable: true,
        declarations: vec![],
        members: vec![],
    };
    let default = json!({"kind":"converted","datatype":contract.datatype,"provider":contract.provider,"value":{"red":18,"green":58,"blue":188}});
    let changed = json!({"kind":"converted","datatype":contract.datatype,"provider":contract.provider,"value":{"red":1,"green":2,"blue":3}});
    for field in ["color", "optional", "colors"] {
        let many = field == "colors";
        let return_type = if many {
            "Sequence<m.RGB>"
        } else {
            "Option<m.RGB>"
        };
        let getter = if many {
            "Sequence(x.colors)".into()
        } else {
            format!("x.{field}")
        };
        let mut source = DefinitionSource::new(
            "memory:paint-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!(
                "rcore 1;package paint version \"1\";import model \"urn:paint\" as m;fn read(x:m.Paint)->{return_type} ={{let f=|unused:Int8|{getter};f(0)}};"
            ),
        );
        source.dependencies.push(model.id.clone());
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), source.clone()],
            namespaces: vec![],
        };
        with_catalog(&bundle, std::slice::from_ref(&contract), |registry| {
            let schema = compile_structure_bundle_with_conversions(&model_bundle, &model.id, "urn:paint", registry).unwrap();
            let image = compile_function_with_conversions(&bundle, &source.id, &EntrySelection { symbol: SymbolId { source: source.id.clone(), declaration: 1 }, types: vec![], arguments: vec![] }, registry, &Limits::default()).unwrap();
            let parameter = image.program().functions[image.program().entry.0 as usize].parameters[0];
            let mut snapshot = Model::default(); snapshot.create(&schema, "paint", "Paint", "r", 0, 0).unwrap();
            let initial = if field == "color" { default.clone() } else if many { json!([]) } else { Value::Null };
            for transition in 0..5 {
                let expected = match transition {
                    0 => initial.clone(),
                    1 => { snapshot.edit_with_conversions(&schema, &[("paint".into(), field.into(), initial.clone())], registry).unwrap(); initial.clone() },
                    2 => { let value = if many { json!([changed]) } else { changed.clone() }; snapshot.edit_with_conversions(&schema, &[("paint".into(), field.into(), value.clone())], registry).unwrap(); value },
                    _ => { snapshot.unset_with_conversions(&schema, "paint", field, registry).unwrap(); initial.clone() },
                };
                assert_eq!(snapshot.is_set(&schema, "paint", field).unwrap(), transition == 1 || transition == 2);
                snapshot.validate_with_conversions(&schema, registry).unwrap();
                let reference = ReferenceContext { bundle: model_bundle.clone(), entry: model.id.clone(), namespace: "urn:paint".into(), conversions: vec![contract.clone()], materialized_types: vec![], applied_conversions: vec![], constructors: vec![], model: serde_json::to_value(&snapshot).unwrap() };
                let before = reference.clone(); let mut context = ExecutionContext::new(&bundle); context.references = std::slice::from_ref(&reference); context.conversions = Some(registry);
                let result = evaluate(&image, &[ValueWire { ty: parameter, data: DataWire::Model(json!({"kind":"model_reference","context":reference.digest().unwrap(),"element":"paint"})) }], &context, &ExecutionLimits::default()).unwrap();
                let observed = match result.value.data {
                    DataWire::None => Value::Null,
                    DataWire::Some(value) => { let DataWire::Datatype(value) = value.data else { panic!("datatype") }; value },
                    DataWire::Collection(values) => json!(values.into_iter().map(|value| { let DataWire::Datatype(value) = value.data else { panic!("Many datatype") }; value }).collect::<Vec<_>>()),
                    _ => panic!("unexpected provider getter"),
                };
                assert_eq!(observed, expected, "{field} transition {transition}"); assert_eq!(reference, before);
                assert!(result.consumed_providers.contains(&contract.provider));
            }
            Ok(())
        }).unwrap();
    }
}
}

control_test! {
pub fn nested_safe_navigation_skips_absent_receivers_and_flattens_optional_results() {
    use intersect_model::core::model::Model;
    use intersect_model::definitions::rcore_behavior::ir_v2::TypeNode;
    use intersect_model::rcore_behavior_functions::ExecutionErrorKind;
    let model =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(DefinitionSource::new(
            "memory:safe-navigation.ecore",
            SourceKind::Ecore,
            include_str!("../../../../fixtures/ecore-navigation/projection.ecore")
                .replace("name=\"children\" upperBound=\"-1\"", "name=\"next\""),
        ))
        .0;
    let model_bundle = DefinitionBundle {
        version: 1,
        sources: vec![model.clone()],
        namespaces: vec![],
    };
    let registry = ConversionRegistry::default();
    let schema = compile_structure_bundle_with_conversions(
        &model_bundle,
        &model.id,
        "urn:intersect:navigation",
        &registry,
    )
    .unwrap();
    let mut source = DefinitionSource::new(
        "memory:safe-navigation-behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1;package navigation version \"1\";import model \"urn:intersect:navigation\" as m;fn read(x:Option<m.Base>)->Option<String> =x?.next?.label;",
    );
    source.dependencies.push(model.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![model.clone(), source.clone()],
        namespaces: vec![],
    };
    let image = compile_function(
        &bundle,
        &source.id,
        &EntrySelection {
            symbol: SymbolId {
                source: source.id.clone(),
                declaration: 1,
            },
            types: vec![],
            arguments: vec![],
        },
        &Limits::default(),
    )
    .unwrap();
    let parameter = image.program().functions[image.program().entry.0 as usize].parameters[0];
    let TypeNode::Option(inner) = image.program().types[parameter.0 as usize] else {
        panic!("optional model")
    };
    let absent = evaluate(
        &image,
        &[ValueWire {
            ty: parameter,
            data: DataWire::None,
        }],
        &ExecutionContext::new(&bundle),
        &ExecutionLimits::default(),
    )
    .unwrap();
    assert_eq!(absent.value.data, DataWire::None);
    assert_eq!(absent.consumed.model_contexts, 0);
    let mut snapshot = Model::default();
    snapshot
        .create(&schema, "object", "Diamond", "r", 0, 0)
        .unwrap();
    for present in [false, true] {
        if present {
            snapshot
                .edit(
                    &schema,
                    &[("object".into(), "next".into(), json!("object"))],
                )
                .unwrap();
        }
        let reference = ReferenceContext {
            bundle: model_bundle.clone(),
            entry: model.id.clone(),
            namespace: "urn:intersect:navigation".into(),
            conversions: vec![],
            materialized_types: vec![],
            applied_conversions: vec![],
            constructors: vec![],
            model: serde_json::to_value(&snapshot).unwrap(),
        };
        let before = reference.clone();
        let argument = ValueWire {
            ty: parameter,
            data: DataWire::Some(Box::new(ValueWire {
                ty: inner,
                data: DataWire::Model(
                    json!({"kind":"model_reference","context":reference.digest().unwrap(),"element":"object"}),
                ),
            })),
        };
        let missing = evaluate(
            &image,
            std::slice::from_ref(&argument),
            &ExecutionContext::new(&bundle),
            &ExecutionLimits::default(),
        )
        .unwrap_err();
        assert_eq!(missing.kind, ExecutionErrorKind::MissingDependency);
        let mut context = ExecutionContext::new(&bundle);
        context.references = std::slice::from_ref(&reference);
        let result = evaluate(&image, &[argument], &context, &ExecutionLimits::default()).unwrap();
        if present {
            let DataWire::Some(value) = result.value.data else {
                panic!("one optional wrapper")
            };
            let DataWire::Scalar(value) = value.data else {
                panic!("string")
            };
            assert_eq!(value.payload, json!("seed"));
        } else {
            assert_eq!(result.value.data, DataWire::None);
        }
        assert_eq!(reference, before);
    }
}
}
