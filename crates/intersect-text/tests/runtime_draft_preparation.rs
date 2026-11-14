//! Inspection-only runtime ownership controls; not a full original language award.
use intersect_text::ordinary;
use std::sync::Arc;
use {
    intersect_model::definitions::bundle::DefinitionBundle,
    intersect_model::definitions::bundle::DefinitionSource,
    intersect_model::definitions::bundle::SourceKind,
    intersect_model::definitions::ecore_migration::BundleMigrationReport,
    intersect_model::definitions::preparation::BuiltinRuleCoverage,
    intersect_model::definitions::preparation::SemanticRequirement,
    intersect_text::compiler::DraftRuntimeCompilation,
    intersect_text::grammar::LiveCallerPrediction, intersect_text::grammar::Profile,
    intersect_text::grammar::RuntimeGuardedPolicy, intersect_text::package::BundleCompilation,
    intersect_text::package::CompilationSourcePolicy,
    intersect_text::preparation::SharedDraftRuntimeLanguageDefinition,
};

fn inputs(behavior: &str) -> Result<(BundleCompilation, BundleMigrationReport), String> {
    let text = include_str!("../../../fixtures/xtext-whole-iteration-v76/GateModel.ecore");
    let needle = "<eClassifiers xsi:type=\"ecore:EClass\" name=\"Root\">";
    assert_eq!(text.matches(needle).count(), 1);
    let text = text.replace(needle, &format!("{needle}\n<eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"computed\" derived=\"true\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//ELong\"/>"));
    let model = DefinitionSource::new(
        "memory:draft-runtime/GateModel.ecore",
        SourceKind::Ecore,
        &text,
    );
    let mut syntax = DefinitionSource::new(
        "memory:draft-runtime/IterationX.xtext",
        SourceKind::Xtext,
        include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationX.xtext"),
    );
    syntax.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:draft-runtime/rules.rcore",
        SourceKind::RcoreBehavior,
        behavior,
    );
    rules.dependencies.push(model.id.clone());
    let original = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:intersect:oracle:WholeIterationGate".into(),
        xtext: Some(syntax.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), syntax, rules],
            namespaces: vec![],
        },
    };
    let origin = intersect_model::migration::migrate_ecore_bundle_with_limits(
        original.bundle.clone(),
        &[(model.id, "memory:draft-runtime/GateModel.rcore".into())].into(),
        intersect_model::definitions::ecore::ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    Ok((
        intersect_text::migration::remap_compilation(&origin, &original)?,
        origin,
    ))
}
const EMPTY: &str = "rcore 1; package emptyRules version \"1\";";
fn draft(input: BundleCompilation) -> Result<DraftRuntimeCompilation, String> {
    ordinary::prepare_with_runtime_guarded_policy(
        input,
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        None,
    )
    .map_err(|e| format!("{e:?}"))
}

#[test]
fn draft_runtime_missing_providers_stay_explicit_and_strict_stays_rejected() -> Result<(), String> {
    let (input, origin) = inputs(EMPTY)?;
    let strict = ordinary::compile_with_runtime_guarded_policy(
        input.clone(),
        Profile::NativeV80,
        &[],
        None,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
    );
    let strict = match strict {
        Ok(_) => return Err("Strict accepted missing behavior".into()),
        Err(e) => e,
    };
    assert!(
        strict
            .iter()
            .any(|e| e.message == "Expected exactly one semantic provider for Root.computed")
    );
    let shared = SharedDraftRuntimeLanguageDefinition::new(Arc::new(draft(input)?));
    let view = shared
        .prepare(&origin, CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))?;
    let mut missing = 0;
    for row in view.requirements() {
        if let SemanticRequirement::MemberBehaviorUnverified {
            receiver,
            feature,
            rule_coverage,
            ..
        } = row?
            && receiver == "Root"
            && feature.name == "computed"
        {
            assert!(feature.derived);
            assert_eq!(rule_coverage, BuiltinRuleCoverage::Missing);
            missing += 1;
        }
    }
    println!(
        "DRAFT_RUNTIME_BOUNDARY {}",
        serde_json::json!({"strict":strict,"missing":missing,
        "runtime_decisions":view.runtime_admission().map(|a|a.decision_count()),
        "language_verified":false,"installed_semantics":false})
    );

    // No fallback provider is synthesized. The existing semantic reader rejects
    // an explicitly unsupported getter before even consulting the empty store.
    // This is a capability boundary control, not a completed parsed model.
    struct Uninstalled<'a> {
        schema: &'a intersect_model::definitions::schema::Schema,
        calls: std::cell::Cell<usize>,
    }
    impl intersect_model::semantic_reads::SemanticReadPolicy for Uninstalled<'_> {
        fn validate(
            &self,
            definition: &intersect_model::definitions::preparation::PreparedModelDefinition<'_>,
        ) -> Result<(), String> {
            if std::ptr::eq(self.schema, definition.schema()) {
                Ok(())
            } else {
                Err("Foreign definition in unsupported-getter control".into())
            }
        }
        fn plan(
            &self,
            _: intersect_model::semantic_reads::ReadMember<'_>,
        ) -> intersect_model::semantic_reads::ReadPlan {
            intersect_model::semantic_reads::ReadPlan::Unsupported
        }
        fn dependencies(&self, _: intersect_model::semantic_reads::ReadMember<'_>) -> &[&str] {
            &[]
        }
        fn read(
            &self,
            _: intersect_model::semantic_reads::ReadMember<'_>,
            _: &dyn intersect_model::semantic_reads::ReadDependencies,
            _: &mut intersect_model::core::query::QueryBudget,
        ) -> Result<serde_json::Value, String> {
            self.calls.set(self.calls.get() + 1);
            Err("Uninstalled getter callback must not run".into())
        }
    }
    let policy = Uninstalled {
        schema: view.schema(),
        calls: std::cell::Cell::new(0),
    };
    let store = intersect_model::frozen::FrozenStore::for_schema(view.schema())
        .map_err(|error| error.to_string())?;
    let reads = intersect_model::semantic_reads::SemanticReadSession::new(&view, &store, &policy)?;
    let unknown = reads.value(
        "Root",
        "absent-by-design",
        "computed",
        &mut intersect_model::core::query::QueryBudget::new(1000),
    );
    let unknown = match unknown {
        Ok(_) => return Err("Missing provider became a default value".into()),
        Err(e) => e,
    };
    assert!(
        unknown.contains("Unsupported semantic read Root:"),
        "{unknown}"
    );
    assert_eq!(policy.calls.get(), 0);
    assert_eq!(missing, 1);
    assert_eq!(
        view.runtime_admission()
            .ok_or("Missing proof")?
            .decision_count(),
        1
    );
    Ok(())
}

#[test]
fn draft_runtime_owner_identity_origin_policy_and_arc_lifetime_fail_closed() -> Result<(), String> {
    let (input, origin) = inputs(EMPTY)?;
    let owner = Arc::new(draft(input.clone())?);
    let weak = Arc::downgrade(&owner);
    let shared = SharedDraftRuntimeLanguageDefinition::new(owner.clone());
    drop(owner);
    let survivor = shared.clone();
    drop(shared);
    let view = survivor
        .prepare(&origin, CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))?;
    let again = survivor
        .prepare(&origin, CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))?;
    assert!(std::ptr::eq(view.schema(), again.schema()));
    assert!(std::ptr::eq(
        view.grammar().ok_or("grammar")?,
        again.grammar().ok_or("grammar")?
    ));
    assert!(std::ptr::eq(
        view.selection().bundle,
        again.selection().bundle
    ));
    assert!(std::ptr::eq(view.original_sources(), &origin.original));
    let foreign = SharedDraftRuntimeLanguageDefinition::new(Arc::new(draft(input)?));
    let foreign = foreign
        .prepare(&origin, CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))?;
    let source = view.selection().xtext.ok_or("source")?;
    let mut work = Some(10000);
    assert!(
        LiveCallerPrediction::new_runtime_guarded(
            view.grammar().ok_or("grammar")?,
            source,
            view.runtime_admission().ok_or("proof")?,
            &mut work
        )
        .is_ok()
    );
    assert!(
        LiveCallerPrediction::new_runtime_guarded(
            foreign.grammar().ok_or("foreign grammar")?,
            source,
            view.runtime_admission().ok_or("proof")?,
            &mut Some(10000)
        )
        .is_err()
    );
    let mut changed = origin.clone();
    changed.version = 0;
    assert!(
        survivor
            .prepare(&changed, CompilationSourcePolicy::default())
            .is_err()
    );
    let host = CompilationSourcePolicy {
        xtext_bytes: 0,
        ..CompilationSourcePolicy::default()
    };
    assert!(survivor.prepare(&origin, host).is_err());
    drop(view);
    drop(again);
    drop(survivor);
    assert!(weak.upgrade().is_none());
    Ok(())
}

#[test]
fn draft_runtime_duplicate_invalid_behavior_and_wrong_profile_remain_errors() -> Result<(), String>
{
    let derive = "derive g.Root.computed using count(children);";
    let good = format!(
        "rcore 1; package rules version \"1\"; import model \"urn:intersect:oracle:WholeIterationGate\" as g; {derive}"
    );
    let duplicate = format!("{good} {derive}");
    let invalid = good.replace("count(children)", "count(uninstalled)");
    let (valid, _) = inputs(&good)?;
    draft(valid)?; // Establish that the single provider/source/grammar is valid.
    for (behavior, cause) in [
        (&duplicate, "Duplicate declaration g.Root.computed"),
        (&invalid, "InvalidDefinition: Unknown provider feature"),
    ] {
        let (input, _) = inputs(behavior)?;
        let strict = ordinary::compile_with_runtime_guarded_policy(
            input.clone(),
            Profile::NativeV80,
            &[],
            None,
            CompilationSourcePolicy::default(),
            CompilationSourcePolicy::default(),
            RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        );
        let strict = match strict {
            Ok(_) => return Err("Strict accepted invalid behavior".into()),
            Err(e) => e,
        };
        let actual = ordinary::prepare_with_runtime_guarded_policy(
            input,
            Profile::NativeV80,
            &[],
            None,
            CompilationSourcePolicy::default(),
            CompilationSourcePolicy::default(),
            RuntimeGuardedPolicy::InputCheckedCompletionScopes,
            None,
        );
        let actual = match actual {
            Ok(_) => return Err("Draft accepted invalid behavior".into()),
            Err(e) => e,
        };
        assert!(actual.iter().any(|e| e.message == cause), "{actual:?}");
        assert_eq!(
            serde_json::to_value(actual).map_err(|e| e.to_string())?,
            serde_json::to_value(strict).map_err(|e| e.to_string())?
        );
    }
    let (input, _) = inputs(EMPTY)?;
    assert!(
        ordinary::prepare_with_runtime_guarded_policy(
            input,
            Profile::NativeV67,
            &[],
            None,
            CompilationSourcePolicy::default(),
            CompilationSourcePolicy::default(),
            RuntimeGuardedPolicy::InputCheckedCompletionScopes,
            None
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn runtime_backend_rejects_equal_foreign_source_worlds() -> Result<(), String> {
    use intersect_model::definitions::{
        declarations::DeclarationCollection,
        ecore::ParseLimits,
        model_definition::{CompiledStructure, PhysicalInputCapture, StructureSelection},
    };
    use intersect_text::grammar;
    let (input, _) = inputs(EMPTY)?;
    let mut checked = CompiledStructure::prepare(
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
    .map_err(|error| error.to_string())?
    .check_behavior(&input.rcore_behavior, None, ParseLimits::default())
    .map_err(|errors| format!("{errors:?}"))?;
    let foreign = input.bundle.clone();
    let entry = input.xtext.as_ref().ok_or("Missing syntax")?;
    let request = || grammar::RuntimeGrammarRequest {
        policy: grammar::RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        call_depth: grammar::DEFAULT_CALL_DEPTH,
    };
    let limits = grammar::SourceLimits {
        source_bytes: 256_000,
        declarations: 128,
    };
    let validate = |_: &intersect_model::definitions::bundle::DefinitionBundle,
                    _: &intersect_model::definitions::bundle::SourceId,
                    _: grammar::SourceLimits| Ok(());
    let imports = || {
        let mut imports = grammar::PackageImports::default();
        imports.collect_classifiers = true;
        imports.validate_authored = Some(&validate);
        imports
    };
    let result = grammar::compile_runtime_sources_with_package_imports(
        &foreign,
        entry,
        &mut checked,
        grammar::Profile::NativeV80,
        limits,
        &[],
        imports(),
        None,
        None,
        grammar::RecursiveFrontierOrder::Historical,
        None,
        request(),
    );
    assert!(
        matches!(result, Err(grammar::RuntimeGrammarError::Publication(message)) if message.contains("actual checked model source world"))
    );
    let compiled = grammar::compile_runtime_sources_with_package_imports(
        &input.bundle,
        entry,
        &mut checked,
        grammar::Profile::NativeV80,
        limits,
        &[],
        imports(),
        None,
        None,
        grammar::RecursiveFrontierOrder::Historical,
        None,
        request(),
    )
    .map_err(|error| match error {
        grammar::RuntimeGrammarError::Diagnostic(error) => format!("{error:?}"),
        grammar::RuntimeGrammarError::NoObligations => "No obligations".into(),
        grammar::RuntimeGrammarError::Publication(message) => message,
    })?;
    assert!(compiled.admission().decision_count() > 0);
    let grammar_address = std::ptr::from_ref(compiled.grammar());
    let moved = std::sync::Arc::new(compiled);
    assert!(std::ptr::eq(grammar_address, moved.grammar()));
    Ok(())
}
