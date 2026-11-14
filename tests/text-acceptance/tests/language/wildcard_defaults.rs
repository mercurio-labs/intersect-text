use intersect_text::package::Package;
#[path = "common/wildcard_defaults.rs"]
mod common;
use intersect_model::conversion::{constructor_contract, with_catalog_policies};
use intersect_model::core::model::Model;
use intersect_model::definitions::{
    compiler::materialize_runtime_types,
    conversion::{AppliedConvertedValue, MemberType, ProviderId},
    generic_types::{ClosedType, RuntimeType, Type},
    resources::DeclarationId,
};
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use serde_json::json;

#[test]
fn aliased_and_diamond_defaults_text_and_package_replay_keep_exact_tags() {
    for diamond in [false, true] {
        let f = common::fixture(diamond);
        let lexical = if diamond {
            r#"["initial"]"#.to_owned()
        } else {
            serde_json::to_string(&vec![r#"["initial"]"#]).unwrap()
        };
        let package = with_catalog_policies(
            &f.input.bundle,
            &[],
            &f.applications,
            &f.constructors,
            |registry| {
                let package = compile_bundle_with_conversions(f.input.clone(), registry)
                    .map_err(|e| format!("{e:?}"))?;
                let schema = &package.schema;
                let class = schema.class_key_for_type(&f.input.materialized_types[0])?;
                let mut model = Model::default();
                model.create(schema, "h", class, "memory:model", 0, 0)?;
                let expected = json!(
                    registry
                        .applied()
                        .parse(&f.applications[f.selected].application, &lexical)?
                );
                assert_eq!(model.elements[0].values["values"], expected);
                let empty = registry
                    .applied()
                    .default_value(&f.applications[f.selected].application)?;
                assert_ne!(json!(empty), expected);
                if !diamond {
                    assert_eq!(schema.constructor_contracts.len(), 2);
                }
                let text = format!(
                    "root Demo values {}",
                    serde_json::to_string(&lexical).unwrap()
                );
                let outcome = evaluate_with_conversions(
                    &package,
                    &[Source {
                        uri: "memory:model".into(),
                        text,
                    }],
                    EvaluationLimits::native(),
                    registry,
                );
                assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
                assert_eq!(outcome.model.elements[0].values["values"], expected);
                let value: AppliedConvertedValue =
                    serde_json::from_value(expected.clone()).unwrap();
                assert_eq!(registry.applied().format(&value)?, lexical);
                let before = model.clone();
                let mut forged = expected;
                let source = f
                    .input
                    .bundle
                    .sources
                    .iter()
                    .find(|source| source.id == f.input.ecore)
                    .unwrap();
                let wrong = intersect_text_acceptance::migration_fixture::named_declaration_fixture(
                    source,
                    if diamond { "Alias" } else { "Sequence" },
                );
                forged["application"]["declaration"] = json!(wrong);
                assert!(
                    model
                        .edit_with_conversions(
                            schema,
                            &[
                                ("h".into(), "name".into(), json!("rollback")),
                                ("h".into(), "values".into(), forged)
                            ],
                            registry
                        )
                        .is_err()
                );
                assert_eq!(model, before);
                model.validate_with_conversions(schema, registry)?;
                Ok(package)
            },
        )
        .unwrap();
        let retained: Package = serde_json::from_value(json!(package)).unwrap();
        let replay = retained.compilation.clone().unwrap();
        with_catalog_policies(
            &replay.bundle,
            &retained.schema.conversion_contracts,
            &retained.schema.applied_conversion_contracts,
            &retained.schema.constructor_contracts,
            |registry| {
                let rebuilt = compile_bundle_with_conversions(replay.clone(), registry)
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(json!(rebuilt), json!(retained));
                Ok(())
            },
        )
        .unwrap();
    }
}

#[test]
fn sequence_constructor_policy_is_explicit_versioned_and_source_exact() {
    let f = common::fixture(false);
    assert!(with_catalog_policies(&f.input.bundle, &[], &f.applications, &[], |_| Ok(())).is_err());
    for version in ["0", "2"] {
        let mut policies = f.constructors.clone();
        policies[0].provider.version = version.into();
        assert!(
            with_catalog_policies(&f.input.bundle, &[], &f.applications, &policies, |_| Ok(()))
                .is_err()
        );
    }
    let mut wrong = f.constructors[0].provider.clone();
    wrong.name = "ecore.list.constructor".into();
    assert!(
        constructor_contract(
            &f.input.bundle,
            f.constructors[0].datatype.declaration(),
            wrong
        )
        .is_err()
    );
    let scalar = intersect_model::definitions::resources::ResourceSet::new(&f.input.bundle)
        .unwrap()
        .resolve(&f.input.ecore, "#//Text")
        .unwrap();
    assert!(
        constructor_contract(
            &f.input.bundle,
            &scalar,
            ProviderId {
                name: "intersect.sequence.constructor".into(),
                version: "1".into()
            }
        )
        .is_err()
    );
    let changed = common::from_text(
        &include_str!("../../../../fixtures/ecore-generics/wildcard-defaults.ecore").replace(
            "name=\"Text\"",
            "name=\"Text\" instanceTypeName=\"Changed\"",
        ),
        false,
    );
    assert!(
        with_catalog_policies(
            &changed.input.bundle,
            &[],
            &changed.applications,
            &f.constructors,
            |_| Ok(())
        )
        .is_err()
    );
}

#[test]
fn specialized_invalid_and_open_defaults_never_choose_an_arbitrary_codec() {
    let original = include_str!("../../../../fixtures/ecore-generics/wildcard-defaults.ecore");
    let malformed = common::from_text(&original.replace("initial", "bad\\escape"), false);
    assert!(
        with_catalog_policies(
            &malformed.input.bundle,
            &[],
            &malformed.applications,
            &malformed.constructors,
            |registry| compile_bundle_with_conversions(malformed.input.clone(), registry)
                .map(|_| ())
                .map_err(|e| format!("{e:?}"))
        )
        .is_err()
    );
    let f = common::fixture(true);
    let resources =
        intersect_model::definitions::resources::ResourceSet::new(&f.input.bundle).unwrap();
    let id = |name: &str| {
        resources
            .resolve(&f.input.ecore, &format!("#//{name}"))
            .unwrap()
    };
    let ty = |n, args, raw| Type::Classifier {
        declaration: id(n),
        arguments: args,
        raw,
    };
    let mut number = f.applications[0].clone();
    number.application =
        ClosedType::new(ty("Sequence", vec![ty("Number", vec![], false)], false)).unwrap();
    number.conversion.members[0].value = MemberType::Datatype {
        declaration: id("Number"),
    };
    with_catalog_policies(
        &f.input.bundle,
        &[],
        &[f.applications[0].clone(), number],
        &[],
        |registry| {
            let selected =
                RuntimeType::new(ty("Holder", vec![ty("Number", vec![], false)], false))?;
            assert!(
                materialize_runtime_types(&f.input.bundle, &[selected], Some(registry)).is_err()
            );
            let raw = RuntimeType::new(ty("Holder", vec![], true))?;
            let error = materialize_runtime_types(&f.input.bundle, &[raw], Some(registry))
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("source-established exact application codec"),
                "{error}"
            );
            Ok(())
        },
    )
    .unwrap();
    let absent = common::from_text(
        &original.replace(" defaultValueLiteral=\"[&quot;initial&quot;]\"", ""),
        true,
    );
    with_catalog_policies(
        &absent.input.bundle,
        &[],
        &absent.applications,
        &[],
        |registry| {
            let raw = RuntimeType::new(Type::Classifier {
                declaration: intersect_model::definitions::resources::ResourceSet::new(
                    &absent.input.bundle,
                )
                .unwrap()
                .resolve(&absent.input.ecore, "#//Holder")
                .unwrap(),
                arguments: vec![],
                raw: true,
            })?;
            let schema = materialize_runtime_types(
                &absent.input.bundle,
                std::slice::from_ref(&raw),
                Some(registry),
            )
            .map_err(|e| e.to_string())?;
            let mut model = Model::default();
            model.create(
                &schema,
                "raw",
                schema.class_key_for_type(&raw)?,
                "memory:model",
                0,
                0,
            )?;
            assert!(model.elements[0].values["values"].is_null());
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn sequence_constructor_metadata_and_cross_family_relations_are_checked() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        conversion::ClassifierRelation,
    };
    let source_text = |attrs: &str, formals: &str| {
        format!(
            r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="policies" nsURI="urn:policies"><eClassifiers x:type="e:EDataType" name="Sequence" {attrs}>{formals}</eClassifiers><eClassifiers x:type="e:EDataType" name="List" instanceClassName="org.eclipse.emf.common.util.EList" serializable="false"><eTypeParameters name="T"/></eClassifiers></e:EPackage>"#
        )
    };
    let provider = ProviderId {
        name: "intersect.sequence.constructor".into(),
        version: "1".into(),
    };
    for (attrs, formals) in [
        (
            r#"instanceClassName="intersect.Sequence" serializable="false""#,
            r#"<eTypeParameters name="T"/>"#,
        ),
        (
            r#"instanceClassName="intersect.Sequence""#,
            r#"<eTypeParameters name="T"/><eTypeParameters name="U"/>"#,
        ),
        (
            r#"instanceClassName="intersect.Sequence" instanceTypeName="unrelated.Sequence""#,
            r#"<eTypeParameters name="T"/>"#,
        ),
        (
            r#"instanceClassName="java.util.List""#,
            r#"<eTypeParameters name="T"/>"#,
        ),
    ] {
        let source = DefinitionSource::new(
            "memory:bad-policy.ecore",
            SourceKind::Ecore,
            source_text(attrs, formals),
        );
        let (source, _) =
            intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
        let id = DeclarationId {
            source: source.id.clone(),
            path: vec![0],
        };
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![source],
            namespaces: vec![],
        };
        assert!(constructor_contract(&bundle, &id, provider.clone()).is_err());
    }
    let source = DefinitionSource::new(
        "memory:policies.ecore",
        SourceKind::Ecore,
        source_text(
            r#"instanceClassName="intersect.Sequence""#,
            r#"<eTypeParameters name="T"/>"#,
        ),
    );
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let first = DeclarationId {
        source: source.id.clone(),
        path: vec![0],
    };
    let second = DeclarationId {
        source: source.id.clone(),
        path: vec![1],
    };
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source],
        namespaces: vec![],
    };
    let policies = vec![
        constructor_contract(&bundle, &first, provider).unwrap(),
        constructor_contract(
            &bundle,
            &second,
            ProviderId {
                name: "ecore.list.constructor".into(),
                version: "1".into(),
            },
        )
        .unwrap(),
    ];
    with_catalog_policies(&bundle, &[], &[], &policies, |registry| {
        assert_eq!(
            registry
                .constructors()
                .constructor_relation(&first, &second)?,
            Some(ClassifierRelation::Unrelated)
        );
        assert_eq!(
            registry
                .constructors()
                .constructor_relation(&second, &first)?,
            Some(ClassifierRelation::Unrelated)
        );
        Ok(())
    })
    .unwrap();
}
