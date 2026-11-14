//! Distinct inspection boundary; no imported language semantic award.
#[path = "../../../fixtures/prepared-language/controls.rs"]
mod controls;
use intersect_text::ordinary;
use serde_json::json;
use {
    intersect_model::definitions::ecore_migration::BundleMigrationReport,
    intersect_model::definitions::preparation::BuiltinRuleCoverage,
    intersect_model::definitions::preparation::SemanticRequirement,
    intersect_text::grammar::PredictionSymbol, intersect_text::grammar::Profile,
    intersect_text::package::BundleCompilation, intersect_text::package::CompilationSourcePolicy,
    intersect_text::preparation::PreparedLanguageDefinition,
};

#[test]
fn composed_text_view_shares_model_metadata_and_uses_the_intrinsic_model_provider()
-> Result<(), String> {
    use intersect_model::{
        api::ModelEdit,
        core::query::{DerivedValues, QueryBudget},
        query::ModelDerived,
    };
    let (input, _) = controls::inputs(controls::GOOD)?;
    let package =
        ordinary::compile(input, Profile::NativeV67).map_err(|errors| format!("{errors:?}"))?;
    let view = intersect_text::package::TextDefinitionView::from_package(&package)?;
    assert!(std::ptr::eq(view.model().schema(), &package.schema));
    assert!(std::ptr::eq(view.model().semantics(), &package.semantics));
    assert!(std::ptr::eq(
        view.grammar(),
        package.grammar.as_ref().ok_or("Missing grammar")?
    ));
    let provider = ModelDerived(view.model());
    assert!(std::ptr::eq(
        view.compilation().ok_or("Missing recipe")?,
        package
            .compilation
            .as_ref()
            .ok_or("Missing original recipe")?
    ));
    assert!(view.inference().is_none());
    let mut model = intersect_model::api::BasicModel::new(package.schema.clone())?;
    model.create("model", "Sub", "memory:model")?;
    model.set("model", "name", json!("named"))?;
    model.set("model", "values", json!([{ "kind": "int64", "value": "1" }, { "kind": "int64", "value": "2" }, { "kind": "int64", "value": "3" }]))?;
    assert_eq!(
        provider.read(&model, "model", "count", &mut QueryBudget::new(1000))?,
        json!({ "kind": "int64", "value": "3" })
    );
    model.set(
        "model",
        "values",
        json!([{ "kind": "int64", "value": "4" }]),
    )?;
    assert_eq!(
        provider.read(&model, "model", "count", &mut QueryBudget::new(1000))?,
        json!({ "kind": "int64", "value": "1" })
    );
    Ok(())
}

#[test]
fn checked_model_compilation_supplies_text_context_without_exposing_mutable_schema()
-> Result<(), String> {
    use intersect_model::definitions::{
        declarations::DeclarationCollection,
        ecore::ParseLimits,
        model_definition::{CompiledStructure, PhysicalInputCapture, StructureSelection},
    };
    let (input, _) = controls::inputs(controls::GOOD)?;
    let package = ordinary::compile(input.clone(), Profile::NativeV67)
        .map_err(|errors| format!("{errors:?}"))?;
    let mut compiled = CompiledStructure::prepare(
        StructureSelection {
            bundle: &input.bundle,
            entry: &input.ecore,
            namespace: &input.namespace,
            materialized_types: &input.materialized_types,
        },
        DeclarationCollection::Classifiers,
        PhysicalInputCapture::Retain,
        None,
        ParseLimits::default(),
    )
    .map_err(|error| error.to_string())?;
    assert!(std::ptr::eq(compiled.sources(), &input.bundle));
    assert!(compiled.has_physical_inputs());
    assert!(compiled.schema().reference_top.is_none());
    let schema_address = std::ptr::from_ref(compiled.schema());
    let declarations = compiled.take_declarations();
    assert!(std::ptr::eq(schema_address, compiled.schema()));
    assert_eq!(
        declarations.packages()[&input.namespace].source,
        input.ecore
    );
    for (class, path) in [("Model", vec![0]), ("Sub", vec![1])] {
        let declaration = &declarations.classifiers()[&input.namespace][class];
        assert_eq!(declaration.source, input.ecore);
        assert_eq!(declaration.path, path);
    }
    compiled.enable_eobject_references()?;
    assert_eq!(compiled.schema(), &package.schema);
    compiled.into_checked().require_schema(&package.schema)?;
    Ok(())
}

#[test]
fn checked_model_output_finishes_after_grammar_without_losing_model_sources() -> Result<(), String>
{
    use intersect_model::definitions::{
        declarations::DeclarationCollection,
        ecore::ParseLimits,
        model_definition::{
            CompiledStructure, ModelPreparationMode, PhysicalInputCapture, StructureSelection,
        },
    };
    use std::sync::Arc;
    let (input, _) = controls::inputs(controls::GOOD)?;
    let expected = ordinary::compile(input.clone(), Profile::NativeV67)
        .map_err(|errors| format!("{errors:?}"))?;
    let sources = Arc::new(input.bundle);
    let compiled = CompiledStructure::prepare(
        StructureSelection {
            bundle: &sources,
            entry: &input.ecore,
            namespace: &input.namespace,
            materialized_types: &input.materialized_types,
        },
        DeclarationCollection::Classifiers,
        PhysicalInputCapture::Retain,
        None,
        ParseLimits::default(),
    )
    .map_err(|error| error.to_string())?;
    let mut checked = compiled
        .check_behavior(&input.rcore_behavior, None, ParseLimits::default())
        .map_err(|errors| format!("{errors:?}"))?;
    assert!(checked.schema().reference_top.is_none());
    let grammar = intersect_text::grammar::compile_sources(
        &sources,
        input.xtext.as_ref().ok_or("Missing grammar")?,
        &mut checked,
        Profile::NativeV21,
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(grammar.name, "prepared.Model");
    assert_eq!(grammar.entry, "Model");
    assert!(checked.schema().reference_top.is_some());
    let compiled = checked
        .finish(
            ParseLimits::default(),
            false,
            ModelPreparationMode::Strict,
            &[],
        )
        .map_err(|errors| format!("{errors:?}"))?;
    assert_eq!(compiled.schema(), &expected.schema);
    assert_eq!(json!(compiled.semantics()), json!(expected.semantics));
    assert!(compiled.has_physical_inputs());
    let owned = compiled.into_owned(Arc::clone(&sources))?;
    assert!(std::ptr::eq(owned.sources(), sources.as_ref()));
    let prepared = owned.prepared_definition();
    assert!(std::ptr::eq(prepared.schema(), owned.schema()));
    assert!(std::ptr::eq(prepared.sources(), sources.as_ref()));
    Ok(())
}

#[test]
fn prepared_text_composes_exact_model_metadata_and_preserves_physical_wire_context()
-> Result<(), String> {
    use intersect_model::{
        core::{
            frozen::FrozenStore,
            storage::{
                ModelStore, PhysicalPersistenceLimits, RetainedModelWire, atomic, set_with_opposite,
            },
        },
        definitions::conversion::InvocationContext,
    };
    use sha2::{Digest, Sha256};
    let (input, origin) = controls::inputs(controls::GOOD)?;
    let prepared = prepare(&input, &origin).map_err(|errors| format!("{errors:?}"))?;
    let model = prepared.model();
    assert!(std::ptr::eq(model.schema(), prepared.schema()));
    assert!(std::ptr::eq(model.sources(), &input.bundle));
    assert!(std::ptr::eq(model.original_sources(), &origin.original));
    assert!(std::ptr::eq(
        model.migration().ok_or("Missing model provenance")?,
        &origin
    ));
    assert_eq!(model.structure_selection().entry, &input.ecore);
    assert_eq!(model.behavior_source(), &input.rcore_behavior);
    #[derive(Default)]
    struct WireMeter {
        work: std::cell::Cell<usize>,
        storage: std::cell::Cell<usize>,
    }
    impl intersect_model::definitions::conversion::InvocationObserver for WireMeter {
        fn before_invoke(
            &self,
            _: &intersect_model::definitions::conversion::ProviderId,
        ) -> Result<(), String> {
            Err("Persistence cannot invoke a provider".into())
        }
        fn work(&self, units: usize) -> Result<(), String> {
            let next = self
                .work
                .get()
                .checked_add(units)
                .filter(|n| *n <= 2_000_000)
                .ok_or("Wire work limit")?;
            self.work.set(next);
            Ok(())
        }
        fn retain_storage(&self, bytes: usize) -> Result<(), String> {
            let next = self
                .storage
                .get()
                .checked_add(bytes)
                .filter(|n| *n <= 64_000_000)
                .ok_or("Wire storage limit")?;
            self.storage.set(next);
            Ok(())
        }
    }
    let meter = std::rc::Rc::new(WireMeter::default());
    let observation = InvocationContext::new(meter.clone());
    let metadata = prepared
        .prepare_physical_metadata(&observation)
        .map_err(|error| error.to_string())?;
    assert!(std::ptr::eq(metadata.definition(), model));
    assert_eq!(
        prepared.requirements().count(),
        model.requirements().count(),
    );
    let mut store = FrozenStore::for_schema(model.schema()).map_err(|error| error.to_string())?;
    atomic(&mut store, |store| {
        store.create_element(model.schema(), "model", "Model", "memory:model", 0, 1)?;
        set_with_opposite(store, model.schema(), "model", "name", json!("named"))
    })?;
    let limits = PhysicalPersistenceLimits {
        objects: 10,
        logical_bytes: 1_048_576,
        wire_bytes: 65_536,
    };
    let wire = RetainedModelWire::save(&store, &prepared, &[], &[], limits, &observation)?;
    // Independent reproduction of the previous format's exact context tuple.
    let legacy = serde_json::to_vec(&(prepared.sources(), prepared.schema(), prepared.grammar()))
        .map_err(|error| error.to_string())?;
    let expected: [u8; 32] = Sha256::digest(legacy).into();
    assert_eq!(wire.json()["context_sha256"], json!(expected));
    let restored = wire.restore_model(&prepared, &[], &[], limits, &observation)?;
    assert!(
        restored
            .elements
            .iter()
            .any(|element| element.id == "model" && element.class == "Model")
    );
    // A model-only view keeps a null extension, and cannot substitute for the
    // actual text wrapper's grammar context when restoring the same wire.
    assert!(
        wire.restore_model(model, &[], &[], limits, &observation)
            .is_err()
    );
    assert!(meter.work.get() > 0 && meter.storage.get() > 0);
    Ok(())
}

#[test]
fn grammar_backend_uses_checked_structure_and_preserves_reference_context_on_failure()
-> Result<(), String> {
    use intersect_model::definitions::{
        bundle::{DefinitionSource, SourceKind},
        declarations::DeclarationCollection,
        ecore::ParseLimits,
        model_definition::{CompiledStructure, PhysicalInputCapture, StructureSelection},
    };
    use intersect_text::grammar;
    let (mut input, _) = controls::inputs(controls::GOOD)?;
    let parent = input
        .xtext
        .as_ref()
        .ok_or("Missing parent grammar")?
        .clone();
    let child = |uri, feature| {
        let mut source = DefinitionSource::new(
            uri,
            SourceKind::Xtext,
            format!(
                "grammar prepared.Child with prepared.Model\nimport \"urn:prepared\" as p\nSub returns p::Sub: 'sub' {feature}=ID;\n"
            ),
        );
        source.dependencies = vec![input.ecore.clone(), parent.clone()];
        source
    };
    let bad = child("memory:prepared/Bad.xtext", "missing");
    let good = child("memory:prepared/Good.xtext", "name");
    input.bundle.sources.extend([bad.clone(), good.clone()]);
    let mut checked = CompiledStructure::prepare(
        StructureSelection {
            bundle: &input.bundle,
            entry: &input.ecore,
            namespace: &input.namespace,
            materialized_types: &[],
        },
        DeclarationCollection::Classifiers,
        PhysicalInputCapture::Retain,
        None,
        ParseLimits::default(),
    )
    .map_err(|error| error.to_string())?;
    let original = checked.schema().clone();
    let address = std::ptr::from_ref(checked.schema());
    let mut metadata = original.clone();
    let profile = Profile::NativeV21;
    let raw_error = grammar::compile_sources(&input.bundle, &bad.id, &mut metadata, profile)
        .err()
        .ok_or("Invalid inherited assignment was accepted")?;
    let checked_error = grammar::compile_sources(&input.bundle, &bad.id, &mut checked, profile)
        .err()
        .ok_or("Checked structure admitted an invalid assignment")?;
    assert_eq!(json!(raw_error), json!(checked_error));
    assert_eq!(checked_error.source, bad.id.uri);
    assert_eq!(checked.schema(), &original);
    assert_eq!(metadata, original);
    let expected = grammar::compile_sources(&input.bundle, &good.id, &mut metadata, profile)
        .map_err(|error| format!("{error:?}"))?;
    let actual = grammar::compile_sources(&input.bundle, &good.id, &mut checked, profile)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(json!(actual), json!(expected));
    assert_eq!(actual.name, "prepared.Child");
    assert!(actual.rules.contains_key("Model"));
    assert!(actual.rules.contains_key("Sub"));
    assert_eq!(checked.schema(), &metadata);
    assert!(checked.schema().reference_top.is_some());
    assert!(std::ptr::eq(address, checked.schema()));
    assert!(std::ptr::eq(checked.sources(), &input.bundle));
    assert!(checked.has_physical_inputs());
    // The single-source facade uses the same transaction and keeps the existing
    // contextual schema intact when an assignment fails after its header.
    let text = &input.bundle.source(&parent)?.text;
    let invalid = text.replace("name=ID", "missing=ID");
    assert!(
        grammar::compile_with_reference_context_and_profile(&invalid, &mut checked, profile)
            .is_err()
    );
    assert_eq!(checked.schema(), &metadata);
    let single = grammar::compile_with_reference_context_and_profile(text, &mut checked, profile)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(single.name, "prepared.Model");
    checked.into_checked().require_schema(&metadata)?;
    Ok(())
}

#[test]
fn neutral_declaration_inventory_keeps_the_prepared_text_sources_exact() -> Result<(), String> {
    use intersect_model::definitions::compiler::inspect_declarations_with_limits;
    use intersect_model::definitions::{conversion::InvocationContext, ecore::ParseLimits};
    let (input, origin) = controls::inputs(controls::EMPTY)?;
    let declarations = inspect_declarations_with_limits(
        &input.bundle,
        &input.ecore,
        &input.namespace,
        true,
        &InvocationContext::default(),
        ParseLimits::default(),
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(
        declarations.packages()[&input.namespace].source,
        input.ecore
    );
    let prepared = prepare(&input, &origin).map_err(|errors| format!("{errors:?}"))?;
    for class in ["Model", "Sub"] {
        let original = &prepared.schema().declarations[class];
        let inspected = &declarations.classifiers()[&input.namespace][class];
        assert_eq!(inspected.source, original.source);
        assert_eq!(inspected.path, original.path);
    }
    Ok(())
}

fn prepare<'a>(
    input: &'a BundleCompilation,
    origin: &'a BundleMigrationReport,
) -> Result<PreparedLanguageDefinition<'a>, Vec<intersect_model::definitions::schema::Diagnostic>> {
    ordinary::prepare_with_context(
        input,
        origin,
        Profile::NativeV67,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
    )
}

#[test]
fn strict_packages_and_diagnostic_streams_equal_original_executable_witness() -> Result<(), String>
{
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/prepared-language/strict-original-observations.json"
    ))
    .map_err(|error| error.to_string())?;
    assert_eq!(controls::strict_observations()?, expected);
    Ok(())
}

#[test]
fn missing_providers_are_explicit_but_nonderived_and_class_behavior_stay_unverified()
-> Result<(), String> {
    let (input, origin) = controls::inputs(controls::EMPTY)?;
    let before = json!({"input": input, "origin": origin});
    let old_error = ordinary::compile(input.clone(), Profile::NativeV67).unwrap_err();
    assert!(
        old_error
            .iter()
            .any(|error| error.message == "Expected exactly one semantic provider for Model.count")
    );
    let prepared = prepare(&input, &origin).map_err(|errors| format!("{errors:?}"))?;
    assert!(std::ptr::eq(prepared.sources(), &input.bundle));
    assert!(std::ptr::eq(prepared.original_sources(), &origin.original));
    assert_eq!(prepared.selection().ecore, &input.ecore);
    let mut missing = Vec::new();
    let mut stored = Vec::new();
    let mut classes = Vec::new();
    for row in prepared.requirements() {
        match row? {
            SemanticRequirement::ClassBehaviorUnverified {
                receiver,
                original,
                declared_operations,
                declared_initializers,
                ..
            } => {
                assert!(original.is_some());
                assert_eq!((declared_operations, declared_initializers), (0, 0));
                classes.push(receiver.to_owned());
            }
            SemanticRequirement::MemberBehaviorUnverified {
                receiver,
                original,
                feature,
                rule_coverage,
                ..
            } => {
                assert!(original.is_some(), "{receiver}.{}", feature.name);
                if feature.name == "count" {
                    assert!(feature.derived);
                    assert_eq!(rule_coverage, BuiltinRuleCoverage::Missing);
                    missing.push(receiver.to_owned());
                }
                if feature.name == "name" {
                    assert!(!feature.derived);
                    assert_eq!(rule_coverage, BuiltinRuleCoverage::StoredMetadataOnly);
                    stored.push(receiver.to_owned());
                }
            }
        }
    }
    assert_eq!(classes, ["Model", "Sub"]);
    assert_eq!(missing, ["Model", "Sub"]);
    assert_eq!(stored, ["Model", "Sub"]);
    assert_eq!(json!({"input": input, "origin": origin}), before);
    Ok(())
}

#[test]
fn prepared_schema_grammar_and_identity_equal_strict_complete_path() -> Result<(), String> {
    let (input, origin) = controls::inputs(controls::GOOD)?;
    let strict = ordinary::compile(input.clone(), Profile::NativeV67)
        .map_err(|errors| format!("{errors:?}"))?;
    let prepared = prepare(&input, &origin).map_err(|errors| format!("{errors:?}"))?;
    assert_eq!(json!(prepared.schema()), json!(strict.schema));
    assert_eq!(json!(prepared.grammar()), json!(strict.grammar));
    let mut checked = Vec::new();
    for row in prepared.requirements() {
        if let SemanticRequirement::MemberBehaviorUnverified {
            receiver,
            feature,
            rule_coverage,
            ..
        } = row?
            && feature.name == "count"
        {
            assert_eq!(
                rule_coverage,
                BuiltinRuleCoverage::CheckedBuiltinDeclaration
            );
            checked.push(receiver.to_owned());
        }
    }
    assert_eq!(checked, ["Model", "Sub"]);
    Ok(())
}

#[test]
fn invalid_and_duplicate_behavior_remain_fatal_with_original_diagnostics() -> Result<(), String> {
    for source in [controls::INVALID, controls::DUPLICATE] {
        let (input, origin) = controls::inputs(source)?;
        let strict = ordinary::compile(input.clone(), Profile::NativeV67).unwrap_err();
        let prepared = match prepare(&input, &origin) {
            Ok(_) => return Err("Invalid behavior became an unmet registration".into()),
            Err(errors) => errors,
        };
        assert_eq!(json!(prepared), json!(strict));
    }
    let (input, origin) = controls::inputs(controls::GOOD)?;
    prepare(&input, &origin).map_err(|errors| format!("Failed recovery: {errors:?}"))?;
    Ok(())
}

#[test]
fn source_origin_and_correspondence_tampering_reject_without_mutating_valid_inputs()
-> Result<(), String> {
    let (input, origin) = controls::inputs(controls::EMPTY)?;
    let before = json!({"input": input, "origin": origin});
    let mut changed = origin.clone();
    changed.original.sources[0].text.push(' ');
    assert!(prepare(&input, &changed).is_err());
    let mut changed = origin.clone();
    changed.reports[0].correspondence[0].original_span.end += 1;
    let errors = match prepare(&input, &changed) {
        Ok(_) => return Err("Tampered correspondence was trusted".into()),
        Err(errors) => errors,
    };
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("replay differs"))
    );
    let mut changed = input.clone();
    changed.bundle.sources[0].text.push(' ');
    assert!(prepare(&changed, &origin).is_err());
    let prepared = prepare(&input, &origin).map_err(|errors| format!("{errors:?}"))?;
    assert_eq!(prepared.original_sources(), &origin.original);
    assert_eq!(json!({"input": input, "origin": origin}), before);
    Ok(())
}

#[test]
fn inspection_entry_profiles_limits_and_syntax_probe_keep_explicit_boundaries() -> Result<(), String>
{
    let (input, origin) = controls::inputs(controls::EMPTY)?;
    let policy = CompilationSourcePolicy::default();
    assert!(
        ordinary::prepare_with_context(
            &input,
            &origin,
            Profile::default(),
            &[],
            None,
            policy,
            policy
        )
        .is_err()
    );
    let mut no_entry = input.clone();
    no_entry.xtext = None;
    assert!(prepare(&no_entry, &origin).is_err());
    let host = CompilationSourcePolicy {
        structure_bytes: 1,
        ..policy
    };
    assert!(
        ordinary::prepare_with_context(
            &input,
            &origin,
            Profile::NativeV67,
            &[],
            None,
            policy,
            host
        )
        .is_err()
    );
    let prepared = prepare(&input, &origin).map_err(|errors| format!("{errors:?}"))?;
    let program = prepared
        .grammar()
        .and_then(|grammar| grammar.parser_prediction.as_ref())
        .ok_or("Missing shared prediction program")?;
    let tokens = [
        PredictionSymbol::Keyword("model".into()),
        PredictionSymbol::Terminal("ID".into()),
    ];
    let mut work = Some(1000);
    assert!(program.probe_rule_completion("Model", &tokens, &mut work)?);
    let used = 1000 - work.ok_or("Lost finite work allowance")?;
    assert!(used > 0);
    assert!(program.probe_rule_completion("Model", &tokens, &mut Some(used))?);
    assert!(
        program
            .probe_rule_completion("Model", &tokens, &mut Some(used - 1))
            .is_err()
    );
    assert!(
        program
            .probe_rule_completion("Model", &tokens, &mut None)
            .is_err()
    );
    Ok(())
}
