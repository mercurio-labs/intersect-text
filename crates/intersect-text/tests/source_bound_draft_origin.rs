//! Immutable origin binding controls, not original language admission.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_model::definitions::ecore_migration::BundleMigrationReport;
use intersect_model::migration;
use intersect_text::grammar::Profile;
use intersect_text::grammar::RuntimeGuardedPolicy;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use intersect_text::{compiler, preparation::SharedDraftRuntimeLanguageDefinition};
use std::{sync::Arc, time::Instant};
fn original_inputs() -> BundleCompilation {
    let model = DefinitionSource::new(
        "memory:compiled-proof/Gate.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/xtext-whole-iteration-v76/GateModel.ecore"),
    );
    let mut syntax = DefinitionSource::new(
        "memory:compiled-proof/Iteration.xtext",
        SourceKind::Xtext,
        include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationX.xtext"),
    );
    syntax.dependencies.push(model.id.clone());
    // Authored valid source padding makes actual canonical hashing work material;
    // this is a component fixture, never a substitute original language source.
    let behavior = format!(
        "rcore 1; package emptyRules version \"1\";\n// {}\n",
        "x".repeat(128_000)
    );
    let mut rules = DefinitionSource::new(
        "memory:compiled-proof/rules",
        SourceKind::RcoreBehavior,
        behavior,
    );
    rules.dependencies.push(model.id.clone());
    BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        xtext: Some(syntax.id.clone()),
        rcore_behavior: rules.id.clone(),
        namespace: "urn:intersect:oracle:WholeIterationGate".into(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), syntax, rules],
            namespaces: vec![],
        },
    }
}
fn inputs() -> Result<(BundleCompilation, BundleMigrationReport), String> {
    let original = original_inputs();
    let origin = migration::migrate_ecore_bundle_with_limits(
        original.bundle.clone(),
        &[(
            original.ecore.clone(),
            "memory:compiled-proof/Gate.rcore".into(),
        )]
        .into(),
        ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    Ok((
        intersect_text::migration::remap_compilation(&origin, &original)?,
        origin,
    ))
}
fn owner(input: BundleCompilation) -> Result<SharedDraftRuntimeLanguageDefinition, String> {
    let policy = CompilationSourcePolicy::default();
    let owner = compiler::prepare_bundle_with_borrowed_runtime_guarded_validation_policy(
        input,
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
        &|_, _, _| Ok(()),
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        None,
    )
    .map_err(|e| format!("{e:?}"))?;
    Ok(SharedDraftRuntimeLanguageDefinition::new(Arc::new(owner)))
}
#[test]
fn immutable_bound_views_keep_sources_schema_and_grammar() -> Result<(), String> {
    let (input, origin) = inputs()?;
    let shared = owner(input)?;
    let policy = CompilationSourcePolicy::default();
    let old = shared
        .prepare(&origin, policy)
        .map_err(|e| format!("{e:?}"))?;
    let expected_sources = old.original_sources().clone();
    drop(old);
    let binding = shared
        .bind_origin(origin, policy)
        .map_err(|e| format!("{e:?}"))?;
    let first = binding.prepare(policy).map_err(|e| format!("{e:?}"))?;
    let started = Instant::now();
    for _ in 0..1000 {
        let current = binding.prepare(policy).map_err(|e| format!("{e:?}"))?;
        assert!(std::ptr::eq(first.schema(), current.schema()));
        assert_eq!(current.original_sources(), &expected_sources);
        assert!(std::ptr::eq(
            current.grammar().ok_or("Missing grammar")?,
            first.grammar().ok_or("Missing grammar")?
        ));
    }
    println!(
        "bound_views_1000_seconds={}",
        started.elapsed().as_secs_f64()
    );
    let mut stricter = policy;
    stricter.structure_bytes = 0;
    assert!(binding.prepare(stricter).is_err());
    Ok(())
}
#[test]
fn bound_origin_rejects_forged_mapping_and_different_compiled_owner() -> Result<(), String> {
    let (input, origin) = inputs()?;
    let policy = CompilationSourcePolicy::default();
    let mut forged = origin.clone();
    let row = forged
        .reports
        .first_mut()
        .and_then(|r| r.correspondence.first_mut())
        .ok_or("No mapping")?;
    row.converted.path.push(999);
    assert!(owner(input.clone())?.bind_origin(forged, policy).is_err());
    let mut foreign = input;
    let syntax = foreign
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.kind == SourceKind::Xtext)
        .ok_or("No syntax")?;
    syntax.text.push_str("\n// different immutable owner\n");
    syntax.id = SourceId::for_text(&syntax.id.uri, &syntax.text);
    foreign.xtext = Some(syntax.id.clone());
    assert!(owner(foreign)?.bind_origin(origin, policy).is_err());
    Ok(())
}

fn completed_inputs() -> Result<(BundleCompilation, migration::CompletedBundleMigration), String> {
    let original = original_inputs();
    let completed = migration::migrate_ecore_bundle_completed_with_limits(
        original.bundle.clone(),
        &[(
            original.ecore.clone(),
            "memory:compiled-proof/Gate.rcore".into(),
        )]
        .into(),
        ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    let input = intersect_text::migration::remap_compilation(completed.report(), &original)?;
    Ok((input, completed))
}
#[test]
fn native_completed_origin_retains_the_same_sources_and_compiled_owner() -> Result<(), String> {
    let (input, completed) = completed_inputs()?;
    let shared = owner(input.clone())?;
    let policy = CompilationSourcePolicy::default();
    assert_eq!(completed.report().converted.as_ref(), Some(&input.bundle));
    let old = shared
        .prepare(completed.report(), policy)
        .map_err(|e| format!("{e:?}"))?;
    let expected_sources = old.original_sources().clone();
    let expected_schema = old.schema().clone();
    let schema_address = old.schema() as *const _;
    let grammar_address = old.grammar().ok_or("grammar")? as *const _;
    drop(old);
    let bound = shared
        .bind_completed_origin(completed, policy)
        .map_err(|e| format!("{e:?}"))?;
    let first = bound.prepare(policy).map_err(|e| format!("{e:?}"))?;
    assert_eq!(first.original_sources(), &expected_sources);
    assert_eq!(first.schema(), &expected_schema);
    assert_eq!(first.schema() as *const _, schema_address);
    assert_eq!(
        first.grammar().ok_or("grammar")? as *const _,
        grammar_address
    );
    assert_eq!(first.selection().bundle, &input.bundle);
    assert_eq!(first.selection().ecore, &input.ecore);
    assert_eq!(first.selection().xtext, input.xtext.as_ref());
    assert_eq!(first.selection().rcore_behavior, &input.rcore_behavior);
    assert_eq!(first.selection().namespace, input.namespace);
    let current = bound.prepare(policy).map_err(|e| format!("{e:?}"))?;
    assert!(std::ptr::eq(first.schema(), current.schema()));
    let mut tighter = policy;
    tighter.structure_bytes = 0;
    assert!(bound.prepare(tighter).is_err());
    Ok(())
}
#[test]
fn native_completed_origin_rejects_foreign_compilation_and_tighter_host() -> Result<(), String> {
    let (mut foreign, completed) = completed_inputs()?;
    let syntax = foreign
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.kind == SourceKind::Xtext)
        .ok_or("No syntax")?;
    syntax.text.push_str("\n// foreign compiled input\n");
    syntax.id = SourceId::for_text(&syntax.id.uri, &syntax.text);
    foreign.xtext = Some(syntax.id.clone());
    let errors = owner(foreign)?
        .bind_completed_origin(completed, CompilationSourcePolicy::default())
        .err()
        .ok_or("Native completion accepted a different compiled bundle")?;
    assert!(format!("{errors:?}").contains("exact converted migration bundle"));
    let (input, completed) = completed_inputs()?;
    let tighter = CompilationSourcePolicy {
        structure_bytes: 0,
        ..CompilationSourcePolicy::default()
    };
    let errors = owner(input)?
        .bind_completed_origin(completed, tighter)
        .err()
        .ok_or("Native completion accepted a tighter host")?;
    assert!(format!("{errors:?}").contains("exceeds host admission limits"));
    Ok(())
}
#[test]
fn ordinary_reports_cannot_retain_or_reconstruct_native_completion() -> Result<(), String> {
    let (input, completed) = completed_inputs()?;
    let report = completed.into_report();
    let wire = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    let mut decoded: BundleMigrationReport =
        serde_json::from_slice(&wire).map_err(|e| e.to_string())?;
    let policy = CompilationSourcePolicy::default();
    // Authentic deserialized reports are accepted only through legacy replay.
    owner(input.clone())?
        .prepare(&decoded, policy)
        .map_err(|e| format!("{e:?}"))?;
    let row = decoded
        .reports
        .first_mut()
        .and_then(|r| r.correspondence.first_mut())
        .ok_or("No mapping")?;
    row.converted.path.push(999);
    let shared = owner(input)?;
    assert!(shared.prepare(&decoded, policy).is_err());
    assert!(shared.bind_origin(decoded, policy).is_err());
    Ok(())
}
#[test]
fn failed_native_migration_does_not_issue_a_completed_token() -> Result<(), String> {
    let original = original_inputs();
    assert!(
        migration::migrate_ecore_bundle_completed_with_limits(
            original.bundle,
            &std::collections::BTreeMap::new(),
            ParseLimits::default(),
        )
        .is_err()
    );
    let blocked = DefinitionSource::new(
        "memory:blocked-token/model.ecore",
        SourceKind::Ecore,
        "<ecore:EPackage xmlns:ecore='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='p' nsURI='urn:p' unexpected='true'><eClassifiers xsi:type='ecore:EClass' name='C'/></ecore:EPackage>",
    );
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![blocked.clone()],
        namespaces: vec![],
    };
    let targets = [(
        blocked.id.clone(),
        "memory:blocked-token/model.rcore".into(),
    )]
    .into();
    let partial = migration::migrate_ecore_bundle_with_limits(
        bundle.clone(),
        &targets,
        ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    assert!(partial.converted.is_none());
    assert!(
        partial
            .reports
            .iter()
            .flat_map(|r| &r.issues)
            .any(|i| i.blocks_conversion && i.code == "unsupported_attribute")
    );
    let error = migration::migrate_ecore_bundle_completed_with_limits(
        bundle,
        &targets,
        ParseLimits::default(),
    )
    .err()
    .ok_or("Blocked extraction issued a completed token")?;
    assert!(error.message.contains("did not complete"));
    assert_eq!(error.span.source, blocked.id);
    assert!(error.span.start < error.span.end);
    Ok(())
}
