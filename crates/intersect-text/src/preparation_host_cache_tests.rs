use crate::package::CompilationSourcePolicy;
// Handwritten host-cache controls (language-pack step 4). A real source-bound
// native draft is saved, serialized, restored without recompiling or replaying
// migration, and compared with the fresh definition; tampered origins, policies
// and foreign packages are refused. Trusted saved correspondence is shown
// explicitly: the load accepts it, the replay oracle does not.
use super::*;
use crate::compiler::{
    DraftRuntimeCompilation, prepare_bundle_with_borrowed_runtime_guarded_validation_policy,
};
use crate::grammar::Profile;
use crate::grammar::RuntimeAdmissionLoadMode;
use crate::grammar::RuntimeGuardedPolicy;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use std::sync::Arc;

// Mutable artifact data for corruption controls. Publication always goes back
// through the real opaque saved type and its native cache loader.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSourceBoundDefinition {
    compilation: crate::compiler::SavedDraftRuntimeCompilation,
    origin: BundleMigrationReport,
    requested: CompilationSourcePolicy,
}
fn snapshot(
    bound: &SourceBoundDraftRuntimeDefinition,
) -> Result<SavedSourceBoundDefinition, String> {
    serde_json::from_value(json(&bound.save()?)?).map_err(|error| error.to_string())
}
fn saved_artifact(
    value: SavedSourceBoundDefinition,
) -> Result<super::SavedSourceBoundDefinition, String> {
    serde_json::from_value(json(&value)?).map_err(|error| error.to_string())
}

const GRAMMAR: &str = "grammar deferred hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} '{' (=> Tag)* '}' Item; fragment Tag returns t::Root: 'x'; fragment Item returns t::Root: 'send' (Body | (=> Prefix Body)); fragment Body returns t::Root: 'a' 'b'; fragment Prefix returns t::Root: 'a';";
const MODEL: &str = r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="deferred" nsURI="urn:deferred-choice"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#;
const MODES: [RuntimeAdmissionLoadMode; 2] = [
    RuntimeAdmissionLoadMode::HostCache,
    RuntimeAdmissionLoadMode::Revalidate,
];

fn original(grammar: &str) -> (BundleCompilation, SourceId) {
    let ecore = DefinitionSource::new("memory:bound-cache/model.ecore", SourceKind::Ecore, MODEL);
    let mut syntax = DefinitionSource::new(
        "memory:bound-cache/grammar.xtext",
        SourceKind::Xtext,
        grammar,
    );
    syntax.dependencies.push(ecore.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:bound-cache/empty.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package boundCacheRules version \"1\";",
    );
    behavior.dependencies.push(ecore.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: ecore.id.clone(),
        namespace: "urn:deferred-choice".into(),
        xtext: Some(syntax.id.clone()),
        rcore_behavior: behavior.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), syntax, behavior],
            namespaces: vec![],
        },
    };
    (input, ecore.id)
}

fn targets(ecore: SourceId) -> std::collections::BTreeMap<SourceId, String> {
    [(ecore, "memory:bound-cache/model.rcore".into())].into()
}

fn compile(input: BundleCompilation) -> Result<SharedDraftRuntimeLanguageDefinition, String> {
    let policy = CompilationSourcePolicy::default();
    let owner = prepare_bundle_with_borrowed_runtime_guarded_validation_policy(
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

// Ordinary report path: binding replays the migration.
fn bound_by_replay(grammar: &str) -> Result<SourceBoundDraftRuntimeDefinition, String> {
    let (input, ecore) = original(grammar);
    let origin = intersect_model::migration::migrate_ecore_bundle_with_limits(
        input.bundle.clone(),
        &targets(ecore),
        intersect_model::definitions::ecore::ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    compile(crate::migration::remap_compilation(&origin, &input)?)?
        .bind_origin(origin, CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))
}

// Native token path: binding consumes the sealed migration without replay.
fn bound_by_token(grammar: &str) -> Result<SourceBoundDraftRuntimeDefinition, String> {
    let (input, ecore) = original(grammar);
    let completed = intersect_model::migration::migrate_ecore_bundle_completed_with_limits(
        input.bundle.clone(),
        &targets(ecore),
        intersect_model::definitions::ecore::ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    compile(crate::migration::remap_compilation(
        completed.report(),
        &input,
    )?)?
    .bind_completed_origin(completed, CompilationSourcePolicy::default())
    .map_err(|e| format!("{e:?}"))
}

fn round_trip(saved: &SavedSourceBoundDefinition) -> Result<SavedSourceBoundDefinition, String> {
    let bytes = serde_json::to_vec(saved).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

fn load(
    saved: SavedSourceBoundDefinition,
    mode: RuntimeAdmissionLoadMode,
) -> Result<SourceBoundDraftRuntimeDefinition, String> {
    SourceBoundDraftRuntimeDefinition::load_host_cache(
        saved_artifact(saved)?,
        CompilationSourcePolicy::default(),
        mode,
    )
    .map_err(|e| format!("{e:?}"))
}

fn refused(saved: SavedSourceBoundDefinition, mode: RuntimeAdmissionLoadMode) -> String {
    match load(saved, mode) {
        Ok(_) => "UNEXPECTEDLY LOADED".into(),
        Err(errors) => errors,
    }
}

fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<serde_json::Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

// Every public observation downstream consumers make of a bound definition.
fn observe(bound: &SourceBoundDraftRuntimeDefinition) -> Result<Vec<serde_json::Value>, String> {
    let policy = CompilationSourcePolicy::default();
    let view = bound.prepare(policy).map_err(|e| format!("{e:?}"))?;
    let admission = view.runtime_admission().ok_or("No runtime admission")?;
    let requirements = view
        .requirements()
        .map(|row| row.map(|row| format!("{row:?}")))
        .collect::<Result<Vec<_>, _>>()?;
    let classes = view
        .requirement_classes()
        .map(|class| {
            class.map(|mut class| {
                let mut rows = vec![format!("{:?}", class.header())];
                while let Some(member) = class.next_member() {
                    rows.push(format!("{member:?}"));
                }
                rows
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let metadata = view
        .prepare_physical_metadata(&Default::default())
        .map_err(|e| e.to_string())?;
    if !std::ptr::eq(metadata.definition().schema(), view.schema()) {
        return Err("Physical metadata belongs to another schema".into());
    }
    let mut stricter = policy;
    stricter.structure_bytes = 0;
    Ok(vec![
        json(view.schema())?,
        json(&view.grammar())?,
        json(view.sources())?,
        json(view.original_sources())?,
        json(view.migration())?,
        json(&view.selection())?,
        json(&admission.save()?)?,
        json(&requirements)?,
        json(&classes)?,
        json(&view.has_source_scope_bindings())?,
        json(&view.source_getter_rules().collect::<Vec<_>>())?,
        json(&bound.prepare(stricter).is_err())?,
    ])
}

#[test]
fn source_bound_round_trip_restores_identical_views_in_both_modes() -> Result<(), String> {
    for (path, fresh) in [
        ("replay", bound_by_replay(GRAMMAR)?),
        ("token", bound_by_token(GRAMMAR)?),
    ] {
        let saved = snapshot(&fresh)?;
        assert_eq!(saved.requested, fresh.requested);
        assert_eq!(&saved.origin, fresh.origin.report());
        let expected = observe(&fresh)?;
        let expected_bytes = serde_json::to_vec(&saved).map_err(|e| e.to_string())?;
        for mode in MODES {
            let loaded = load(round_trip(&saved)?, mode)?;
            assert_eq!(
                loaded.origin.report(),
                fresh.origin.report(),
                "{path} {mode:?} origin"
            );
            assert_eq!(loaded.requested, fresh.requested, "{path} {mode:?} policy");
            assert_eq!(observe(&loaded)?, expected, "{path} {mode:?} views");
            // Saving the loaded definition reproduces the same bytes.
            let again = serde_json::to_vec(&loaded.save()?).map_err(|e| e.to_string())?;
            assert_eq!(again, expected_bytes, "{path} {mode:?} resave");
            // Views borrow the single loaded owner, not a copy per call.
            let first = loaded.prepare(CompilationSourcePolicy::default());
            let second = loaded.prepare(CompilationSourcePolicy::default());
            let (first, second) = (
                first.map_err(|e| format!("{e:?}"))?,
                second.map_err(|e| format!("{e:?}"))?,
            );
            assert!(std::ptr::eq(first.schema(), second.schema()));
        }
    }
    Ok(())
}

#[test]
fn loaded_origin_still_passes_the_replay_oracle() -> Result<(), String> {
    let saved = snapshot(&bound_by_token(GRAMMAR)?)?;
    let SavedSourceBoundDefinition {
        compilation,
        origin,
        requested,
    } = round_trip(&saved)?;
    let owner =
        DraftRuntimeCompilation::load_host_cache(compilation, RuntimeAdmissionLoadMode::Revalidate)
            .map_err(|e| format!("{e:?}"))?;
    // Optional completed model inputs are never restored by a text host-cache load.
    assert!(!owner.model_owner().has_physical_inputs());
    let replayed = SharedDraftRuntimeLanguageDefinition::new(Arc::new(owner))
        .bind_origin(origin, CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(replayed.requested, requested);
    Ok(())
}

#[test]
fn deserialization_refuses_unknown_fields() -> Result<(), String> {
    let saved = snapshot(&bound_by_replay(GRAMMAR)?)?;
    let mut value = json(&saved)?;
    value
        .as_object_mut()
        .ok_or("Saved definition is not an object")?
        .insert("authority".into(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<super::SavedSourceBoundDefinition>(value).is_err());
    Ok(())
}

#[test]
fn tampered_origin_is_refused_without_replay() -> Result<(), String> {
    let saved = snapshot(&bound_by_token(GRAMMAR)?)?;
    let syntax = |origin: &BundleMigrationReport| -> Result<usize, String> {
        origin
            .original
            .sources
            .iter()
            .position(|source| source.kind == SourceKind::Xtext)
            .ok_or("No original syntax".into())
    };
    for mode in MODES {
        // Changed original text under its old digest.
        let mut stale = saved.clone();
        let index = syntax(&stale.origin)?;
        stale.origin.original.sources[index].text.push(' ');
        let error = refused(stale, mode);
        assert!(
            error.contains("Source content digest mismatch"),
            "{mode:?} stale digest: {error}"
        );

        // Changed original text with a consistent new digest: the bundle still
        // validates, but the report no longer equals the source at its position.
        let mut redigested = saved.clone();
        let ecore = redigested
            .origin
            .original
            .sources
            .iter_mut()
            .find(|source| source.kind == SourceKind::Ecore)
            .ok_or("No original model")?;
        ecore.text.push_str("<!-- changed -->");
        let old = ecore.id.clone();
        ecore.id = SourceId::for_text(&ecore.id.uri, &ecore.text);
        let new = ecore.id.clone();
        for source in &mut redigested.origin.original.sources {
            for dependency in &mut source.dependencies {
                if *dependency == old {
                    *dependency = new.clone();
                }
            }
        }
        redigested.origin.original.validate()?;
        let error = refused(redigested, mode);
        assert!(
            error.contains("original differs from its bundle source at its position"),
            "{mode:?} redigested: {error}"
        );

        // The report's own original edited under its unchanged identity.
        for edit in [
            (|source: &mut DefinitionSource| source.text.push(' ')) as fn(&mut DefinitionSource),
            |source| source.dependencies.push(source.id.clone()),
        ] {
            let mut edited = saved.clone();
            edit(&mut edited.origin.reports[0].original);
            let error = refused(edited, mode);
            assert!(
                error.contains("original differs from its bundle source at its position"),
                "{mode:?} edited report original: {error}"
            );
        }

        // A pass-through original changed with a consistent digest no longer
        // matches its compiled pass-through source.
        let mut passthrough = saved.clone();
        let index = syntax(&passthrough.origin)?;
        let source = &mut passthrough.origin.original.sources[index];
        source.text.push(' ');
        source.id = SourceId::for_text(&source.id.uri, &source.text);
        passthrough.origin.original.validate()?;
        let error = refused(passthrough, mode);
        assert!(
            error.contains("changes a pass-through source"),
            "{mode:?} pass-through: {error}"
        );

        // A converted bundle that is not the loaded compilation's.
        let mut converted = saved.clone();
        let bundle = converted
            .origin
            .converted
            .as_mut()
            .ok_or("No converted bundle")?;
        bundle
            .namespaces
            .push(intersect_model::definitions::bundle::NamespaceBinding {
                namespace: "urn:forged".into(),
                source: bundle.sources[0].id.clone(),
            });
        let error = refused(converted, mode);
        assert!(
            error.contains("exact converted migration bundle"),
            "{mode:?} converted: {error}"
        );

        let mut version = saved.clone();
        version.origin.version = 2;
        assert!(refused(version, mode).contains("exact converted migration bundle"));

        let mut report = saved.clone();
        report.origin.reports[0].version = 9;
        assert!(refused(report, mode).contains("unsupported version"));

        let mut blocked = saved.clone();
        blocked.origin.reports[0].converted = None;
        assert!(refused(blocked, mode).contains("missing converted"));

        let mut duplicated = saved.clone();
        let first = duplicated.origin.reports[0].clone();
        duplicated.origin.reports.push(first);
        assert!(refused(duplicated, mode).contains("no Ecore original"));

        // Incomplete reports: cleared, or the only report dropped.
        let mut cleared = saved.clone();
        cleared.origin.reports.clear();
        assert!(refused(cleared, mode).contains("missing a resource report"));

        // A report remapped to the Xtext pass-through source.
        let mut remapped = saved.clone();
        let index = syntax(&remapped.origin)?;
        let passthrough = remapped
            .origin
            .converted
            .as_ref()
            .ok_or("No converted bundle")?
            .sources[index]
            .clone();
        remapped.origin.reports[0].converted = Some(passthrough);
        let error = refused(remapped, mode);
        assert!(
            error.contains("converted source differs from the compiled bundle source"),
            "{mode:?} remapped: {error}"
        );

        // Converted content that differs under the same identity.
        for edit in [
            (|source: &mut DefinitionSource| source.text.push(' ')) as fn(&mut DefinitionSource),
            |source| source.dependencies.push(source.id.clone()),
            |source| source.kind = SourceKind::Ecore,
        ] {
            let mut edited = saved.clone();
            edit(
                edited.origin.reports[0]
                    .converted
                    .as_mut()
                    .ok_or("No converted source")?,
            );
            let error = refused(edited, mode);
            assert!(
                error.contains("converted source differs from the compiled bundle source"),
                "{mode:?} edited converted: {error}"
            );
        }

        // A draft that is not the converted text.
        let mut draft = saved.clone();
        draft.origin.reports[0].draft.push(' ');
        assert!(refused(draft, mode).contains("is not its Rcore draft"));

        // A blocking issue on a completed migration.
        let mut issue = saved.clone();
        let span = issue.origin.reports[0]
            .correspondence
            .first()
            .ok_or("No correspondence")?
            .original_span
            .clone();
        issue.origin.reports[0]
            .issues
            .push(intersect_model::migration::MigrationIssue {
                code: "forged".into(),
                message: "forged".into(),
                span,
                blocks_conversion: true,
            });
        assert!(refused(issue, mode).contains("carries a blocking issue"));

        // A correspondence row naming another resource.
        let mut row = saved.clone();
        let other = row.origin.original.sources[syntax(&row.origin)?].id.clone();
        row.origin.reports[0]
            .correspondence
            .first_mut()
            .ok_or("No correspondence")?
            .converted
            .source = other;
        assert!(refused(row, mode).contains("names another resource"));
    }
    Ok(())
}

#[test]
fn mismatched_policy_and_tighter_host_are_refused() -> Result<(), String> {
    let saved = snapshot(&bound_by_replay(GRAMMAR)?)?;
    for mode in MODES {
        let mut requested = saved.clone();
        requested.requested.xtext_declarations =
            requested.requested.xtext_declarations.saturating_sub(1);
        let error = refused(requested, mode);
        assert!(
            error.contains("requested source policy differs"),
            "{mode:?}: {error}"
        );

        let tighter = CompilationSourcePolicy {
            structure_bytes: 0,
            ..CompilationSourcePolicy::default()
        };
        let error = match SourceBoundDraftRuntimeDefinition::load_host_cache(
            saved_artifact(saved.clone())?,
            tighter,
            mode,
        ) {
            Ok(_) => "UNEXPECTEDLY LOADED".into(),
            Err(errors) => format!("{errors:?}"),
        };
        assert!(error.contains("exceeds host admission limits"), "{error}");
    }
    Ok(())
}

#[test]
fn package_from_another_origin_is_refused() -> Result<(), String> {
    let saved = snapshot(&bound_by_token(GRAMMAR)?)?;
    // The same language with one more Tag keyword: a different compiled
    // bundle with a valid admission of its own.
    let other = GRAMMAR.replace("returns t::Root: 'x';", "returns t::Root: 'x' | 'y';");
    assert_ne!(other, GRAMMAR);
    let foreign = snapshot(&bound_by_token(&other)?)?;
    for mode in MODES {
        let swapped = SavedSourceBoundDefinition {
            compilation: foreign.compilation.clone(),
            origin: saved.origin.clone(),
            requested: saved.requested,
        };
        let error = refused(swapped, mode);
        assert!(
            error.contains("exact converted migration bundle"),
            "{mode:?}: {error}"
        );
        // The foreign pairing loads on its own.
        load(foreign.clone(), mode)?;
    }
    Ok(())
}

#[test]
fn trusted_correspondence_loads_but_fails_the_replay_oracle() -> Result<(), String> {
    // Documented trust boundary: correspondence rows are trusted saved data.
    // The host-cache load does not replay migration and accepts the forgery;
    // the replaying bind_origin oracle refuses the same origin.
    let mut saved = snapshot(&bound_by_token(GRAMMAR)?)?;
    let row = saved
        .origin
        .reports
        .first_mut()
        .and_then(|report| report.correspondence.first_mut())
        .ok_or("No correspondence")?;
    row.converted.path.push(999);
    let forged = saved.origin.clone();
    load(saved.clone(), RuntimeAdmissionLoadMode::Revalidate)?;
    let owner = DraftRuntimeCompilation::load_host_cache(
        saved.compilation,
        RuntimeAdmissionLoadMode::Revalidate,
    )
    .map_err(|e| format!("{e:?}"))?;
    assert!(
        SharedDraftRuntimeLanguageDefinition::new(Arc::new(owner))
            .bind_origin(forged, CompilationSourcePolicy::default())
            .is_err()
    );
    Ok(())
}

#[test]
fn tampered_admission_is_refused_through_the_source_bound_load() -> Result<(), String> {
    let saved = snapshot(&bound_by_replay(GRAMMAR)?)?;
    for mode in MODES {
        let mut removed = saved.clone();
        let mut data =
            serde_json::to_value(&removed.compilation).map_err(|error| error.to_string())?;
        let records = data["grammar"]["admission"]["records"]
            .as_object_mut()
            .ok_or("No record map")?;
        let key = records.keys().next().cloned().ok_or("No record")?;
        records.remove(&key);
        removed.compilation = serde_json::from_value(data).map_err(|error| error.to_string())?;
        assert!(refused(removed, mode).contains("key set differs"));
        // The step-3 load refuses a package without a source policy before
        // the source-bound defensive check is reached.
        let mut policy = saved.clone();
        let mut data = json(&policy.compilation)?;
        data["source_policy"] = serde_json::Value::Null;
        policy.compilation = serde_json::from_value(data).map_err(|error| error.to_string())?;
        assert!(refused(policy, mode).contains("ordinary retained compilation"));
    }
    Ok(())
}

#[test]
fn native_cache_version_is_explicit_and_legacy_payloads_are_rejected() -> Result<(), String> {
    let fresh = bound_by_token(GRAMMAR)?;
    let saved = snapshot(&fresh)?;
    for version in [0, 2] {
        let mut altered = saved.clone();
        let mut data = json(&altered.compilation)?;
        data["version"] = serde_json::json!(version);
        altered.compilation = serde_json::from_value(data).map_err(|error| error.to_string())?;
        assert!(
            refused(altered, RuntimeAdmissionLoadMode::HostCache)
                .contains("Unsupported saved draft cache version")
        );
    }
    let view = fresh
        .prepare(CompilationSourcePolicy::default())
        .map_err(|errors| format!("{errors:?}"))?;
    let (package, admission) = fresh.shared.owner.prepared_parts();
    let legacy = serde_json::json!({"package":package,"admission":admission.save()?});
    assert!(
        serde_json::from_value::<crate::compiler::SavedDraftRuntimeCompilation>(legacy).is_err()
    );
    assert_eq!(
        view.selection().bundle,
        package.compilation.ok_or("Missing inputs")?.bundle
    );
    Ok(())
}

#[test]
fn ordinary_host_cache_preserves_views_and_rejects_foreign_sources() -> Result<(), String> {
    let (input, ecore) = original(
        "grammar ordinary hidden() import \"urn:deferred-choice\" as t Root returns t::Root: {t::Root} 'x';",
    );
    let origin = intersect_model::migration::migrate_ecore_bundle_with_limits(
        input.bundle.clone(),
        &targets(ecore),
        intersect_model::definitions::ecore::ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    let compilation = crate::migration::remap_compilation(&origin, &input)?;
    let policy = CompilationSourcePolicy::default();
    let fresh = crate::compiler::prepare_bundle_with_borrowed_authored_validation(
        &compilation,
        &origin,
        Profile::NativeV78,
        &[],
        None,
        policy,
        policy,
        &|_, _, _| Ok(()),
    )
    .map_err(|e| format!("{e:?}"))?;
    let artifact = json(&fresh.save_ordinary_host_cache()?)?;
    let mut saved: super::SavedOrdinaryDefinition =
        serde_json::from_value(artifact.clone()).map_err(|e| e.to_string())?;
    let loaded = saved
        .prepare_host_cache(Profile::NativeV78, policy)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(json(fresh.schema())?, json(loaded.schema())?);
    assert_eq!(json(&fresh.grammar())?, json(&loaded.grammar())?);
    assert_eq!(fresh.sources(), loaded.sources());
    assert_eq!(fresh.original_sources(), loaded.original_sources());
    assert_eq!(fresh.selection().namespace, loaded.selection().namespace);

    for (path, replacement) in [
        (vec!["compiler"], serde_json::json!("foreign")),
        (
            vec!["compilation", "namespace"],
            serde_json::json!("urn:foreign"),
        ),
        (vec!["origin", "version"], serde_json::json!(999)),
        (vec!["model", "namespace"], serde_json::json!("urn:foreign")),
    ] {
        let mut altered = artifact.clone();
        let mut target = &mut altered;
        for key in path {
            target = &mut target[key];
        }
        *target = replacement;
        let mut forged: super::SavedOrdinaryDefinition =
            serde_json::from_value(altered).map_err(|e| e.to_string())?;
        assert!(
            forged
                .prepare_host_cache(Profile::NativeV78, policy)
                .is_err()
        );
    }
    assert!(
        saved
            .prepare_host_cache(Profile::NativeV65, policy)
            .is_err()
    );
    saved.grammar = None;
    assert!(
        saved
            .prepare_host_cache(Profile::NativeV78, policy)
            .is_err()
    );
    Ok(())
}
