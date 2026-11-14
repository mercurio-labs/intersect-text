use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    ecore_migration::migrate_ecore,
    rcore::compile_structure,
};
use intersect_text::package::BundleCompilation;

#[test]
fn explicit_bundle_migration_rebinds_generic_compilation_selections_and_rejects_stale_inputs() {
    use intersect_model::definitions::generic_types::{RuntimeType, Type};
    use intersect_text::compiler;
    let mut original: BundleCompilation = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-generics/language-bundle.json"
    ))
    .unwrap();
    let behavior = original
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == original.rcore_behavior)
        .unwrap();
    let text = intersect_text_acceptance::migration_fixture::canonical_behavior_fixture(
        &behavior.text,
        &original.namespace,
    )
    .unwrap();
    let mut canonical = DefinitionSource::new(&behavior.id.uri, SourceKind::RcoreBehavior, text);
    canonical.dependencies = behavior.dependencies.clone();
    original.rcore_behavior = canonical.id.clone();
    *behavior = canonical;
    // Include a nested generic application, not only named concrete subclasses.
    let declaration = |index| intersect_model::definitions::resources::DeclarationId {
        source: original.ecore.clone(),
        path: vec![index],
    };
    original.materialized_types.push(
        RuntimeType::new(Type::Classifier {
            declaration: declaration(2),
            raw: false,
            arguments: vec![Type::Classifier {
                declaration: declaration(0),
                raw: false,
                arguments: vec![],
            }],
        })
        .unwrap(),
    );
    let targets = [(original.ecore.clone(), "memory:generic.rcore".into())].into();
    let report =
        intersect_model::migration::migrate_ecore_bundle(original.bundle.clone(), &targets)
            .unwrap();
    let converted = intersect_text::migration::remap_compilation(&report, &original).unwrap();
    assert_eq!(report.original, original.bundle);
    assert_ne!(converted.ecore, original.ecore);
    assert_eq!(converted.xtext, original.xtext);
    assert_eq!(converted.rcore_behavior, original.rcore_behavior);
    assert!(
        converted
            .bundle
            .sources
            .iter()
            .all(|s| s.kind != SourceKind::Ecore)
    );
    let Type::Classifier {
        declaration,
        arguments,
        ..
    } = converted.materialized_types.last().unwrap().expression()
    else {
        panic!("classifier");
    };
    assert_eq!(declaration.source, converted.ecore);
    assert!(
        matches!(&arguments[0], Type::Classifier { declaration, .. } if declaration.source == converted.ecore)
    );
    let package = compiler::compile_bundle(converted).unwrap();
    assert_eq!(package.semantics.derived.len(), 1);
    assert_eq!(
        compiler::recompile(&package).unwrap().schema.revision,
        package.schema.revision
    );
    let mut stale = original.clone();
    stale.bundle.sources[0].text.push(' ');
    assert!(
        intersect_text::migration::remap_compilation(&report, &stale)
            .unwrap_err()
            .contains("original bundle")
    );
    let mut missing = original;
    missing.materialized_types = vec![
        RuntimeType::new(Type::Classifier {
            declaration: intersect_model::definitions::resources::DeclarationId {
                source: missing.ecore.clone(),
                path: vec![9999],
            },
            arguments: vec![],
            raw: false,
        })
        .unwrap(),
    ];
    assert!(
        intersect_text::migration::remap_compilation(&report, &missing)
            .unwrap_err()
            .contains("correspondence")
    );
}

fn source(body: &str) -> DefinitionSource {
    DefinitionSource::new(
        "memory:source.ecore",
        SourceKind::Ecore,
        format!(
            r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="demo" nsURI="urn:demo" nsPrefix="d">{body}</e:EPackage>"#
        ),
    )
}

#[test]
fn migration_preserves_structure_and_exact_origin_without_runtime_xml() {
    for (name, text) in [
        (
            "organization",
            include_str!("../../../../fixtures/rcore/day01/organization.ecore"),
        ),
        (
            "kerml",
            include_str!("../../../../fixtures/rcore/day01/kerml-specialization.ecore"),
        ),
    ] {
        let original =
            DefinitionSource::new(format!("memory:{name}.ecore"), SourceKind::Ecore, text);
        let a = migrate_ecore(original.clone(), &format!("memory:{name}.rcore")).unwrap();
        assert_eq!(
            a,
            migrate_ecore(original.clone(), &format!("memory:{name}.rcore")).unwrap()
        );
        assert_eq!(a.original, original);
        let target = a.converted.unwrap_or_else(|| panic!("{:?}", a.issues));
        assert_ne!(target.id, original.id);
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![target.clone()],
            namespaces: vec![],
        };
        let actual = compile_structure(&bundle, &target.id).unwrap().schema;
        let expected = intersect_text_acceptance::migration_fixture::original_schema_fixture(text);
        assert_classes(&actual, &expected);
        assert!(a.correspondence.len() > actual.classes.len());
        for mapping in a.correspondence {
            assert_eq!(mapping.original.source, original.id);
            assert_eq!(mapping.converted.source, target.id);
            assert!(original.text.is_char_boundary(mapping.original_span.start));
            assert!(target.text.is_char_boundary(mapping.converted_span.end));
        }
    }
}

#[test]
fn flags_defaults_and_quoted_identifiers_survive_conversion() {
    let original = source(
        r#"<eClassifiers x:type="e:EClass" name="Node">
      <eStructuralFeatures x:type="e:EAttribute" name="class" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString" defaultValueLiteral="caf&#233;&amp;&lt;" changeable="false" transient="true" volatile="true" unsettable="true"/>
      <eStructuralFeatures x:type="e:EAttribute" name="count" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EInt" defaultValueLiteral="-7"/>
    </eClassifiers>"#,
    );
    let report = migrate_ecore(original.clone(), "memory:flags.rcore").unwrap();
    let converted = report
        .converted
        .unwrap_or_else(|| panic!("{:?}", report.issues));
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![converted.clone()],
        namespaces: vec![],
    };
    let actual = compile_structure(&bundle, &converted.id).unwrap().schema;
    let expected = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        &original.text,
    )
    .unwrap();
    assert_classes(&actual, &expected);
    assert_eq!(
        actual.classes["Node"].features["class"].default,
        serde_json::json!("café&<")
    );
    assert_eq!(
        actual.classes["Node"].features["count"].default,
        serde_json::json!(-7)
    );
}

#[test]
fn unsupported_constructs_never_produce_an_admitted_source() {
    for body in [
        r#"<eClassifiers x:type="e:EClass" name="C" unknown="true"/>"#,
        r#"<eClassifiers x:type="e:EClass" name="C"><eStructuralFeatures x:type="e:EReference" name="other" eType="other.ecore#//C"/></eClassifiers>"#,
        r#"<eClassifiers x:type="e:EClass" name="C">meaningful text</eClassifiers>"#,
    ] {
        let original = source(body);
        let report = migrate_ecore(original.clone(), "memory:manual.rcore").unwrap();
        assert!(report.converted.is_none());
        assert!(report.correspondence.is_empty());
        assert!(report.issues.iter().any(|i| i.blocks_conversion));
        assert_eq!(report.original, original);
        assert!(!report.draft.is_empty());
    }
}

#[test]
fn operation_conversion_reports_missing_implementation() {
    let report = migrate_ecore(source(r#"<eClassifiers x:type="e:EClass" name="C"><eOperations name="value" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EInt" lowerBound="1"/></eClassifiers>"#), "memory:operation.rcore").unwrap();
    assert!(report.converted.is_some(), "{:?}", report.issues);
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == "operation_implementation" && !i.blocks_conversion)
    );
    assert!(!report.draft.contains("return"));
}

#[test]
fn invalid_sources_and_identity_reuse_fail_explicitly() {
    let original = source("");
    assert!(migrate_ecore(original.clone(), &original.id.uri).is_err());
    let mut stale = original;
    stale.text.push(' ');
    assert!(migrate_ecore(stale, "memory:stale.rcore").is_err());
    let report = migrate_ecore(
        source(r#"<eClassifiers x:type="e:EClass" name="C" abstract="perhaps"/>"#),
        "memory:invalid.rcore",
    )
    .unwrap();
    assert!(report.converted.is_none());
    assert!(report.issues.iter().any(|i| i.code == "invalid_boolean"
        && i.blocks_conversion
        && i.message.contains("abstract")));
}

fn assert_classes(
    actual: &intersect_model::definitions::schema::Schema,
    expected: &intersect_model::definitions::schema::Schema,
) {
    assert_eq!(actual.classes.len(), expected.classes.len());
    for (name, a) in &actual.classes {
        let b = &expected.classes[name];
        assert_eq!(a.features, b.features);
        assert_eq!(a.abstract_type, b.abstract_type);
        assert_eq!(a.interface_type, b.interface_type);
        assert_eq!(a.parents, b.parents);
        assert_eq!(a.feature_order, b.feature_order);
        assert_eq!(a.operations, b.operations);
    }
}

#[test]
fn enum_numeric_spellings_migrate_without_changing_values_or_original_bytes() {
    for (spelling, expected) in [
        ("٧", 7),
        ("-٠٠٧", -7),
        ("+００７", 7),
        ("-٢١٤٧٤٨٣٦٤٨", i32::MIN),
        ("٢١٤٧٤٨٣٦٤٧", i32::MAX),
    ] {
        let original = source(&format!(
            r##"<eClassifiers x:type="e:EEnum" name="EInt"><eLiterals name="Value" value="{spelling}" literal="value"/></eClassifiers>
            <eClassifiers x:type="e:EClass" name="Item"><eStructuralFeatures x:type="e:EAttribute" name="count" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EInt" defaultValueLiteral="1"/><eStructuralFeatures x:type="e:EAttribute" name="choice" eType="#//EInt" defaultValueLiteral="value"/></eClassifiers>"##
        ));
        let report = migrate_ecore(original.clone(), "memory:enum.rcore").unwrap();
        assert_eq!(report.original, original);
        let target = report
            .converted
            .as_ref()
            .unwrap_or_else(|| panic!("{:?}", report.issues));
        assert!(target.text.contains(&format!("Value = {expected} literal")));
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![target.clone()],
            namespaces: vec![],
        };
        let schema = compile_structure(&bundle, &target.id).unwrap().schema;
        assert_eq!(schema.enumerations["EInt"].literals()[0].value, expected);
        assert!(
            report
                .correspondence
                .iter()
                .any(|entry| entry.original.path == vec![0, 0])
        );
    }
    for spelling in ["٢١٤٧٤٨٣٦٤٨", "-٢١٤٧٤٨٣٦٤٩", "7x"] {
        let original = source(&format!(
            r#"<eClassifiers x:type="e:EEnum" name="Bad"><eLiterals name="Value" value="{spelling}"/></eClassifiers>"#
        ));
        let report = migrate_ecore(original.clone(), "memory:bad.rcore").unwrap();
        assert_eq!(report.original, original);
        assert!(report.converted.is_none());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.blocks_conversion && issue.span.source == original.id)
        );
    }
}

#[test]
fn interface_and_enum_lexical_aliases_are_preserved() {
    let original = source(
        r##"<eClassifiers x:type="e:EClass" name="Named" interface="true" abstract="true"/>
      <eClassifiers x:type="e:EEnum" name="Status"><eLiterals name="Ready" value="4" literal="ready-now"/><eLiterals name="Alias" value="4" literal="alias"/></eClassifiers>
      <eClassifiers x:type="e:EClass" name="Item" eSuperTypes="#//Named"><eStructuralFeatures x:type="e:EAttribute" name="status" eType="#//Status" defaultValueLiteral="ready-now"/></eClassifiers>"##,
    );
    let report = migrate_ecore(original, "memory:enum.rcore").unwrap();
    let target = report
        .converted
        .unwrap_or_else(|| panic!("{:?}", report.issues));
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![target.clone()],
        namespaces: vec![],
    };
    let schema = compile_structure(&bundle, &target.id).unwrap().schema;
    assert!(schema.classes["Named"].interface_type);
    assert_eq!(
        schema.enumerations["Status"]
            .format(&schema.enumerations["Status"].parse("ready-now").unwrap())
            .unwrap(),
        "ready-now"
    );
    assert!(schema.enumerations["Status"].parse("Ready").is_err());
    assert!(
        report
            .correspondence
            .iter()
            .any(|m| m.original.path == vec![1] && m.converted.path == vec![2])
    );
}
#[test]
fn executable_bundles_require_explicit_migration_of_every_ecore_source() {
    use intersect_text::compiler::{compile_bundle, recompile};
    use intersect_text::package::BundleCompilation;
    let original: BundleCompilation = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-generics/language-bundle.json"
    ))
    .unwrap();
    let errors = compile_bundle(original.clone()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("migrate Ecore explicitly")),
        "{errors:?}"
    );

    let canonical: BundleCompilation = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-generics/language-bundle-canonical.json"
    ))
    .unwrap();
    let package = compile_bundle(canonical.clone()).unwrap();
    assert_eq!(recompile(&package).unwrap().schema, package.schema);
    let mut mixed = canonical;
    mixed
        .bundle
        .sources
        .push(original.bundle.source(&original.ecore).unwrap().clone());
    let errors = compile_bundle(mixed).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("convert the complete bundle")),
        "{errors:?}"
    );

    let mut retained = package;
    retained.compilation = Some(original);
    assert!(recompile(&retained).is_err());
}
#[test]
fn executable_behavior_and_replay_require_canonical_sources() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        rcore_behavior::{
            function_lowering::compile_function, ir, ir_v2, lowering::compile_scalar,
            modules::SymbolId,
        },
    };
    let original = source(r#"<eClassifiers x:type="e:EClass" name="Item"/>"#);
    let mut behavior = DefinitionSource::new(
        "memory:behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package behavior version \"1\"; fn run()->Int8=7;",
    );
    behavior.dependencies.push(original.id.clone());
    let mut bundle = DefinitionBundle {
        version: 1,
        sources: vec![original.clone(), behavior.clone()],
        namespaces: vec![],
    };
    let symbol = SymbolId {
        source: behavior.id.clone(),
        declaration: 0,
    };
    let selection = ir_v2::EntrySelection {
        symbol: symbol.clone(),
        types: vec![],
        arguments: vec![],
    };
    let scalar_limits = ir::VerificationLimits::default();
    let function_limits = ir_v2::Limits::default();
    for errors in [
        compile_scalar(&bundle, &behavior.id, &symbol, &scalar_limits).unwrap_err(),
        compile_function(&bundle, &behavior.id, &selection, &function_limits).unwrap_err(),
    ] {
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("migration inputs"))
        );
    }
    let migrated = migrate_ecore(original.clone(), "memory:model.rcore")
        .unwrap()
        .converted
        .unwrap();
    behavior.dependencies = vec![migrated.id.clone()];
    bundle.sources = vec![migrated, behavior.clone()];
    let scalar = compile_scalar(&bundle, &behavior.id, &symbol, &scalar_limits).unwrap();
    let function = compile_function(&bundle, &behavior.id, &selection, &function_limits).unwrap();
    // Even an unselected retained Ecore source must not enter a verified image.
    bundle.sources.push(original);
    assert!(
        ir::verify(scalar.program().clone(), &bundle, &scalar_limits)
            .unwrap_err()
            .message
            .contains("migration inputs")
    );
    assert!(
        ir_v2::verify(function.program().clone(), &bundle, &function_limits)
            .unwrap_err()
            .message
            .contains("migration inputs")
    );
}

#[test]
fn empty_packages_migrate_as_metadata_with_an_explicit_runtime_requirement() {
    let report = migrate_ecore(source(""), "memory:empty.rcore").unwrap();
    let converted = report.converted.unwrap();
    assert!(report.issues.iter().all(|issue| !issue.blocks_conversion));
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "runtime_requirement")
    );
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![converted.clone()],
        namespaces: vec![],
    };
    let (classifiers, operations) =
        intersect_model::definitions::rcore::resolve_structure(&bundle, &converted.id).unwrap();
    assert!(classifiers.is_empty() && operations.is_empty());
    assert!(compile_structure(&bundle, &converted.id).is_err());
}

#[test]
fn href_signature_nodes_migrate_without_losing_types_or_original_payloads() {
    let original = source(
        r##"<eClassifiers x:type="e:EClass" name="A">
        <eStructuralFeatures x:type="e:EReference" name="next"><eType href="#//A"/></eStructuralFeatures>
        <eOperations name="select"><eType href="#//A"/>
            <eParameters name="index"><eType href="http://www.eclipse.org/emf/2002/Ecore#//EInt"/></eParameters>
            <eExceptions href="#//A"/>
            <eAnnotations source="delegate"><details key="body" value="not executed"/></eAnnotations>
        </eOperations></eClassifiers>"##,
    );
    let report = migrate_ecore(original.clone(), "memory:href.rcore").unwrap();
    assert_eq!(report.original, original);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "operation_implementation")
    );
    let target = report.converted.unwrap();
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![target.clone()],
        namespaces: vec![],
    };
    let schema = compile_structure(&bundle, &target.id).unwrap().schema;
    assert_eq!(schema.classes["A"].features["next"].target, "A");
    let operation = &schema.classes["A"].operations[0];
    assert_eq!(
        operation.parameters[0].value.target,
        Some(intersect_model::definitions::operations::TypeTarget::Builtin("EInt".into()))
    );
    assert!(operation.source_payload.contains("not executed"));
    for replacement in [
        "href=\"#//A\" extra=\"unknown\"",
        "href=\"#//A\"><unknown/></eType><eType href=\"#//A\"",
    ] {
        let changed = DefinitionSource::new(
            "memory:unsupported.ecore",
            SourceKind::Ecore,
            original.text.replacen("href=\"#//A\"", replacement, 1),
        );
        let report = migrate_ecore(changed, "memory:unsupported.rcore").unwrap();
        assert!(report.converted.is_none());
        assert!(report.issues.iter().any(|issue| issue.blocks_conversion));
    }
}

#[test]
fn every_structural_schema_boundary_requires_a_fully_migrated_bundle() {
    use compiler::{self as materialization, OwnedRuntimeSourceArrayPreparation};
    use intersect_model::definitions::compiler;
    use intersect_model::definitions::{
        conversion::{ConversionRegistry, InvocationContext},
        ecore::{ErrorKind, ParseLimits},
        generic_types::{ClosedType, RuntimeType, Type},
        resources::DeclarationId,
    };
    let original = source(r#"<eClassifiers x:type="e:EClass" name="C"/>"#);
    let migrated = migrate_ecore(original.clone(), "memory:boundary.rcore")
        .unwrap()
        .converted
        .unwrap();
    let requested = [RuntimeType::from(
        &ClosedType::new(Type::Classifier {
            declaration: DeclarationId {
                source: migrated.id.clone(),
                path: vec![0],
            },
            arguments: vec![],
            raw: false,
        })
        .unwrap(),
    )];
    let canonical = DefinitionBundle {
        version: 1,
        sources: vec![migrated.clone()],
        namespaces: vec![],
    };
    let context = InvocationContext::default();
    let conversions = ConversionRegistry::default();
    assert!(compiler::compile_structure_bundle(&canonical, &migrated.id, "urn:demo").is_ok());
    assert!(materialization::materialize_runtime_types(&canonical, &requested, None).is_ok());
    for bundle in [
        DefinitionBundle {
            version: 1,
            sources: vec![original.clone()],
            namespaces: vec![],
        },
        DefinitionBundle {
            version: 1,
            sources: vec![migrated.clone(), original.clone()],
            namespaces: vec![],
        },
    ] {
        let entry = &bundle.sources[0].id;
        let error = compiler::compile_structure_bundle(&bundle, entry, "urn:demo").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Unsupported);
        assert_eq!(error.span.source, original.id);
        assert!(error.message.contains("migration inputs"));
        assert!(
            compiler::compile_structure_bundle_with_limits(
                &bundle,
                entry,
                "urn:demo",
                ParseLimits::default()
            )
            .is_err()
        );
        assert!(
            compiler::compile_structure_bundle_with_conversions(
                &bundle,
                entry,
                "urn:demo",
                &conversions
            )
            .is_err()
        );
        assert!(
            compiler::compile_structure_bundle_with_limits_observed(
                &bundle,
                entry,
                "urn:demo",
                None,
                &context,
                ParseLimits::default()
            )
            .is_err()
        );
        assert!(
            compiler::compile_structure_bundle_with_conversions_observed(
                &bundle,
                entry,
                "urn:demo",
                &conversions,
                &context
            )
            .is_err()
        );
        assert!(materialization::materialize_runtime_types(&bundle, &requested, None).is_err());
        assert!(
            materialization::materialize_runtime_bundle(
                &bundle, entry, "urn:demo", &requested, None
            )
            .is_err()
        );
        assert!(
            materialization::materialize_runtime_bundle_observed(
                &bundle, entry, "urn:demo", &requested, None, &context
            )
            .is_err()
        );
        assert!(
            materialization::materialize_runtime_bundle_inputs_observed(
                &bundle, entry, "urn:demo", &requested, None, &context
            )
            .is_err()
        );
        assert!(
            materialization::materialize_runtime_domain_inputs_observed(
                &bundle, entry, "urn:demo", &requested, None, &context
            )
            .is_err()
        );
        assert!(
            materialization::materialize_runtime_array_inputs_observed(
                &bundle, entry, "urn:demo", &requested, None, &context
            )
            .is_err()
        );
        assert!(
            materialization::materialize_runtime_array_view_inputs_observed(
                &bundle, entry, "urn:demo", &requested, None, &context
            )
            .is_err()
        );
        assert!(
            materialization::materialize_runtime_source_array_inputs_observed(
                &bundle, entry, "urn:demo", &requested, None, &context
            )
            .is_err()
        );
        for begin in [
            OwnedRuntimeSourceArrayPreparation::begin,
            OwnedRuntimeSourceArrayPreparation::begin_with_compiled_signature_inputs,
        ] {
            let result = begin(
                bundle.clone(),
                entry.clone(),
                "urn:demo".into(),
                &requested,
                None,
                &context,
            );
            let failed = match result {
                Err(failed) => failed,
                Ok(_) => panic!("Ecore was admitted"),
            };
            assert_eq!(failed.materialization.bundle, bundle);
            assert!(failed.materialization.result.is_err());
        }
    }
}

#[test]
fn migration_source_limits_apply_to_both_originals_and_generated_resources() {
    use intersect_model::definitions::ecore::{ErrorKind, ParseLimits};
    let mut original = source(r#"<eClassifiers x:type="e:EClass" name="C"/>"#);
    original = DefinitionSource::new(
        original.id.uri,
        SourceKind::Ecore,
        format!("<!--{}-->\n{}", "padding".repeat(40_000), original.text),
    );
    assert_eq!(
        migrate_ecore(original.clone(), "memory:large.rcore")
            .unwrap_err()
            .kind,
        ErrorKind::ResourceLimit
    );
    let limits = ParseLimits {
        source_bytes: original.text.len(),
    };
    let single = intersect_model::migration::migrate_ecore_with_limits(
        original.clone(),
        "memory:large.rcore",
        limits,
    )
    .unwrap();
    assert!(single.converted.is_some(), "{:?}", single.issues);
    assert_eq!(single.original, original);
    let input = DefinitionBundle {
        version: 1,
        sources: vec![original.clone()],
        namespaces: vec![],
    };
    let targets =
        std::collections::BTreeMap::from([(original.id.clone(), "memory:large.rcore".to_owned())]);
    assert_eq!(
        intersect_model::migration::migrate_ecore_bundle(input.clone(), &targets)
            .unwrap_err()
            .kind,
        ErrorKind::ResourceLimit
    );
    let report =
        intersect_model::migration::migrate_ecore_bundle_with_limits(input, &targets, limits)
            .unwrap();
    let converted = report.converted.unwrap();
    assert_eq!(converted.sources[0], single.converted.unwrap());
    assert!(compile_structure(&converted, &converted.sources[0].id).is_ok());
}
