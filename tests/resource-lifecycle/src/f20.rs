//! Independent lifecycle observations shared by native and actual WASM controls.
use super::*;
use intersect_model::core::{
    resource_cache::{CacheIdentity, CacheLimits, PreparedModelCache, content_digest},
    resource_set::{ResourceChangeKind, UnloadPolicy},
    storage::{InvocationContext, ReferenceView},
};

fn registry() -> Result<FormatRegistry, String> {
    let mut registry = FormatRegistry::default();
    registry.register(intersect_model::xmi::XmiFormat)?;
    Ok(registry)
}
fn identity() -> CacheIdentity {
    CacheIdentity {
        sources: BTreeMap::from([
            (
                "catalog.library".into(),
                content_digest(include_bytes!("../../../fixtures/library/catalog.library")),
            ),
            (
                "loans.library".into(),
                content_digest(include_bytes!("../../../fixtures/library/loans.library")),
            ),
        ]),
        implementations: BTreeMap::from([(
            "library-semantics".into(),
            "independent-resource-control-v1".into(),
        )]),
        runtime: "intersect-resource-contract-v1".into(),
        format: "xmi".into(),
        format_revision: "intersect-xmi-resources-v1".into(),
    }
}

pub fn residency() -> Result<(), String> {
    let package = package()?;
    let model = build_direct(&package.schema)?;
    let context = make_context(&package.schema)?;
    let registry = registry()?;
    let mut set = stores::resources_from_model::<FrozenStore>(
        package.schema.clone(),
        context.clone(),
        &model,
    )?;
    set.create_resource("empty.library")?;
    set.bind_fragment("catalog.library", "book", "dune")?;
    let handle = set.reader().handle("dune").ok_or("Missing book handle")?;
    let loan_handle = set.reader().handle("first").ok_or("Missing loan handle")?;
    let all: Vec<_> = set.resources()?.into_keys().collect();
    let selection = vec!["catalog.library".into()];
    let before = set.snapshot()?;
    set.drain_changes();
    require(
        set.unload(&registry, &selection, UnloadPolicy::RequireSaved)
            .is_err(),
        "Dirty unload succeeded",
    )?;
    require(
        set.snapshot()? == before && set.drain_changes().is_empty(),
        "Failed unload published changes",
    )?;
    let ticket = set.prepare_save(&registry, "xmi", &all, ReferenceClosure::SelectedOnly)?;
    let catalog: Vec<_> = ticket
        .resources()
        .iter()
        .filter(|resource| resource.uri == "catalog.library")
        .cloned()
        .collect();
    set.acknowledge_save(&registry, &ticket)?;
    set.unload(&registry, &selection, UnloadPolicy::RequireSaved)?;
    let suspended = set.snapshot()?;
    require(
        suspended.elements.len() == 2,
        "Unload retained catalog values or lost loans",
    )?;
    require(
        handle_id(set.reader(), &handle).is_err(),
        "Unload left the old book handle live",
    )?;
    require(
        handle_id(set.reader(), &loan_handle)? == "first",
        "Unload invalidated an unrelated handle",
    )?;
    require(
        suspended.resources["catalog.library"]
            .unloaded
            .as_ref()
            .ok_or("Missing identity reservations")?
            .identities
            .len()
            == 4,
        "Unload lost external identities",
    )?;
    let mut unavailable = false;
    set.reader().inspect("first", &mut |object| {
        unavailable = matches!(
            object.value("copy").and_then(|value| value.reference()),
            Some(ReferenceView::Unavailable { identity: "dune" })
        );
        Ok(())
    })?;
    require(
        unavailable,
        "Loan reference did not become explicitly unavailable",
    )?;
    require(
        matches!(
            set.resolve("loans.library", "catalog.library#book")?,
            TargetResolution::UnavailableResource { .. }
        ),
        "Unloaded catalog resolved as empty success",
    )?;
    require(
        set.validate().is_err()
            && set
                .transaction(|tx| tx.set("first", "name", json!("hidden edit")))
                .is_err(),
        "Unavailable content acquired validation or edit authority",
    )?;
    let change = set.drain_changes();
    require(
        change.len() == 1
            && change[0].kind == ResourceChangeKind::Unloaded
            && change[0].invalidated.contains(&"loans.library".into()),
        "Missing committed invalidation event",
    )?;
    let mut corrupt = catalog.clone();
    corrupt[0].bytes = b"<unsupported/>".to_vec();
    require(
        set.prepare_reload(
            &registry,
            "xmi",
            &selection,
            &corrupt,
            &context,
            &InvocationContext::default(),
        )
        .is_err(),
        "Malformed reload succeeded",
    )?;
    require(
        set.snapshot()? == suspended && set.drain_changes().is_empty(),
        "Failed reload changed published state",
    )?;
    let rejected = set.prepare_reload(
        &registry,
        "xmi",
        &selection,
        &catalog,
        &context,
        &InvocationContext::default(),
    )?;
    require(
        set.commit_reload(
            rejected,
            |_| Err("language rejection".into()),
            &InvocationContext::default(),
        )
        .is_err(),
        "Language rejection committed reload",
    )?;
    require(
        set.snapshot()? == suspended && set.drain_changes().is_empty(),
        "Rejected reload changed state or events",
    )?;
    let stale = set.prepare_reload(
        &registry,
        "xmi",
        &selection,
        &catalog,
        &context,
        &InvocationContext::default(),
    )?;
    let prepared = set.prepare_reload(
        &registry,
        "xmi",
        &selection,
        &catalog,
        &context,
        &InvocationContext::default(),
    )?;
    set.commit_reload(prepared, |_| Ok(()), &InvocationContext::default())?;
    set.validate()?;
    require(
        handle_id(set.reader(), &handle).is_err(),
        "Reload revived the old handle",
    )?;
    require(
        matches!(set.resolve("loans.library", "catalog.library#book")?, TargetResolution::Resolved { element, .. } if element == "dune"),
        "Reload lost explicit fragment binding",
    )?;
    require(
        observed(&set.snapshot()?, &package.schema)? == observed(&model, &package.schema)?,
        "Reload changed independent Library observations",
    )?;
    require(
        set.commit_reload(stale, |_| Ok(()), &InvocationContext::default())
            .is_err(),
        "Stale prepared reload committed",
    )?;
    let mut recovered = ResourceSet::from_resident_store(
        package.schema.clone(),
        context.clone(),
        FrozenStore::from_materialized(&package.schema, &suspended)?,
    )?;
    let prepared = recovered.prepare_reload(
        &registry,
        "xmi",
        &selection,
        &catalog,
        &context,
        &InvocationContext::default(),
    )?;
    recovered.commit_reload(prepared, |_| Ok(()), &InvocationContext::default())?;
    recovered.validate()?;
    require(
        observed(&recovered.snapshot()?, &package.schema)? == observed(&model, &package.schema)?,
        "Portable residency recovery changed observations",
    )?;
    recovered.remove_resource("empty.library")?;
    require(
        !recovered.resources()?.contains_key("empty.library"),
        "Deletion retained an empty resource",
    )
}

pub fn caches() -> Result<(), String> {
    let package = package()?;
    let textual = evaluate(
        &package,
        &[
            Source {
                uri: "catalog.library".into(),
                text: include_str!("../../../fixtures/library/catalog.library").into(),
            },
            Source {
                uri: "loans.library".into(),
                text: include_str!("../../../fixtures/library/loans.library").into(),
            },
        ],
    );
    require(
        textual.diagnostics.is_empty(),
        "Fresh Library sources failed",
    )?;
    let expected = observed(&textual.model, &package.schema)?;
    require(
        expected == observed(&build_direct(&package.schema)?, &package.schema)?,
        "Fresh source and independently constructed Library differ",
    )?;
    let context = make_context(&package.schema)?;
    let registry = registry()?;
    let mut source =
        ResourceSet::from_model(package.schema.clone(), context.clone(), &textual.model)?;
    source.create_resource("empty.library")?;
    let selected: Vec<_> = source.resources()?.into_keys().collect();
    let cache = source.prepare_cache(&registry, &selected, identity(), |_| Ok(()))?;
    let bytes = cache.to_bytes()?;
    require(
        !bytes.is_empty() && cache.resources().len() == 3,
        "Prepared cache lost empty or related resources",
    )?;
    let cache = PreparedModelCache::from_bytes(&bytes, CacheLimits::default())?;
    let mut diagnosed = textual.model.clone();
    diagnosed
        .resources
        .get_mut("catalog.library")
        .ok_or("Missing catalog record")?
        .diagnostics
        .push("Unresolved source error".into());
    let mut diagnosed =
        ResourceSet::from_model(package.schema.clone(), context.clone(), &diagnosed)?;
    require(
        diagnosed
            .prepare_cache(&registry, &["catalog.library".into()], identity(), |_| {
                Ok(())
            })
            .is_err(),
        "Cache preparation discarded unresolved diagnostics",
    )?;
    let mut loaded = ResourceSet::new(package.schema.clone(), context.clone())?;
    loaded.load_cache(
        &registry,
        &cache,
        &identity(),
        |_| Ok(()),
        &InvocationContext::default(),
    )?;
    let reloaded = loaded.snapshot()?;
    require(
        observed(&reloaded, &package.schema)? == expected,
        "Cached reload changed independent source observations",
    )?;
    require(
        reloaded.resources["empty.library"].roots.is_empty(),
        "Cached empty library lost identity",
    )?;
    let before = loaded.snapshot()?;
    let book = before
        .elements
        .iter()
        .find(|element| element.values.get("name") == Some(&json!("dune")))
        .ok_or("Missing source book")?
        .id
        .clone();
    loaded.bind_fragment("catalog.library", "book", &book)?;
    loaded.drain_changes();
    for variant in 0..4 {
        let mut changed = identity();
        match variant {
            0 => {
                changed.sources.insert(
                    "catalog.library".into(),
                    content_digest(b"changed transitive input"),
                );
            }
            1 => {
                changed
                    .implementations
                    .insert("library-semantics".into(), "changed".into());
            }
            2 => changed.runtime.push_str("-changed"),
            _ => changed.format_revision.push_str("-changed"),
        }
        require(
            loaded
                .load_cache(
                    &registry,
                    &cache,
                    &changed,
                    |_| Ok(()),
                    &InvocationContext::default(),
                )
                .is_err(),
            "Stale cache context was accepted",
        )?;
    }
    let mut corrupt: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    corrupt["resources"][0]["bytes"][0] = json!(0);
    let corrupt: PreparedModelCache =
        serde_json::from_value(corrupt).map_err(|error| error.to_string())?;
    require(
        loaded
            .load_cache(
                &registry,
                &corrupt,
                &identity(),
                |_| Ok(()),
                &InvocationContext::default(),
            )
            .is_err(),
        "Corrupt cache artifact was accepted",
    )?;
    require(
        loaded
            .load_cache(
                &registry,
                &cache,
                &identity(),
                |_| Err("language rejection".into()),
                &InvocationContext::default(),
            )
            .is_err(),
        "Cache bypassed language validation",
    )?;
    require(
        loaded.snapshot()? == before && loaded.drain_changes().is_empty(),
        "Rejected cache changed published state",
    )?;
    require(
        PreparedModelCache::from_bytes(&bytes[..bytes.len() / 2], CacheLimits::default()).is_err(),
        "Truncated cache was accepted",
    )?;
    require(
        PreparedModelCache::from_bytes(
            &bytes,
            CacheLimits {
                max_bytes: bytes.len() - 1,
                max_resources: 3,
            },
        )
        .is_err(),
        "Cache byte admission was bypassed",
    )?;
    let selected: Vec<_> = loaded.resources()?.into_keys().collect();
    loaded.unload(&registry, &selected, UnloadPolicy::DiscardChanges)?;
    require(
        loaded.snapshot()?.validate(&package.schema).is_err(),
        "A fully unloaded graph was validated as an empty model",
    )?;
    require(
        loaded.snapshot()?.elements.is_empty(),
        "Whole-library unload retained values",
    )?;
    loaded.load_cache(
        &registry,
        &cache,
        &identity(),
        |_| Ok(()),
        &InvocationContext::default(),
    )?;
    require(
        matches!(loaded.resolve("loans.library", "catalog.library#book")?, TargetResolution::Resolved { element, .. } if element == book),
        "Cached reload lost explicit fragment bindings",
    )?;
    require(
        observed(&loaded.snapshot()?, &package.schema)? == expected,
        "Cache did not recover unloaded library",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn verify_resource_f20_residency() -> u32 {
    u32::from(residency().is_err())
}
#[unsafe(no_mangle)]
pub extern "C" fn verify_resource_f20_cache() -> u32 {
    u32::from(caches().is_err())
}

pub fn cancellation() -> Result<(), String> {
    use intersect_model::definitions::conversion::{InvocationObserver, ProviderId};
    use std::{cell::Cell, rc::Rc};
    struct Meter {
        calls: Cell<usize>,
        stop: usize,
    }
    impl InvocationObserver for Meter {
        fn before_invoke(&self, _: &ProviderId) -> Result<(), String> {
            Err("No semantic provider may be invoked by residency metadata".into())
        }
        fn checkpoint(&self) -> Result<(), String> {
            let call = self.calls.get();
            self.calls.set(call + 1);
            if call == self.stop {
                Err("cancelled resource operation".into())
            } else {
                Ok(())
            }
        }
    }
    let package = package()?;
    let model = build_direct(&package.schema)?;
    let context = make_context(&package.schema)?;
    let registry = registry()?;
    let selection = vec!["catalog.library".into()];
    let make = || {
        stores::resources_from_model::<FrozenStore>(package.schema.clone(), context.clone(), &model)
    };
    let mut sample = make()?;
    let save = sample.prepare_save(&registry, "xmi", &selection, ReferenceClosure::SelectedOnly)?;
    let meter = Rc::new(Meter {
        calls: Cell::new(0),
        stop: usize::MAX,
    });
    sample.unload_observed(
        &registry,
        &selection,
        UnloadPolicy::DiscardChanges,
        &InvocationContext::new(meter.clone()),
    )?;
    let suspended = sample.snapshot()?;
    for stop in 0..meter.calls.get() {
        let mut set = make()?;
        let old = set
            .reader()
            .handle("dune")
            .ok_or("Missing cancellation handle")?;
        let meter = Rc::new(Meter {
            calls: Cell::new(0),
            stop,
        });
        require(
            set.unload_observed(
                &registry,
                &selection,
                UnloadPolicy::DiscardChanges,
                &InvocationContext::new(meter),
            )
            .is_err(),
            "Unload ignored cancellation",
        )?;
        require(
            set.snapshot()? == model
                && handle_id(set.reader(), &old)? == "dune"
                && set.drain_changes().is_empty(),
            "Cancelled unload changed model, handles or events",
        )?;
    }
    let meter = Rc::new(Meter {
        calls: Cell::new(0),
        stop: usize::MAX,
    });
    let reload = sample.prepare_reload(
        &registry,
        "xmi",
        &selection,
        save.resources(),
        &context,
        &InvocationContext::default(),
    )?;
    sample.commit_reload(reload, |_| Ok(()), &InvocationContext::new(meter.clone()))?;
    for stop in 0..meter.calls.get() {
        let mut set = ResourceSet::from_resident_store(
            package.schema.clone(),
            context.clone(),
            FrozenStore::from_materialized(&package.schema, &suspended)?,
        )?;
        let reload = set.prepare_reload(
            &registry,
            "xmi",
            &selection,
            save.resources(),
            &context,
            &InvocationContext::default(),
        )?;
        let meter = Rc::new(Meter {
            calls: Cell::new(0),
            stop,
        });
        require(
            set.commit_reload(reload, |_| Ok(()), &InvocationContext::new(meter))
                .is_err(),
            "Reload ignored cancellation",
        )?;
        require(
            set.snapshot()? == suspended && set.drain_changes().is_empty(),
            "Cancelled reload published partial content or events",
        )?;
    }
    // The raw adapter must also protect reserved identities, including when no
    // currently resident object refers to them and incremental creation applies.
    let mut raw = FrozenStore::from_materialized(&package.schema, &suspended)?;
    let book = model
        .elements
        .iter()
        .find(|element| element.id == "dune")
        .ok_or("Missing book fixture")?
        .clone();
    require(
        storage::atomic(&mut raw, |store| {
            storage::insert(store, &package.schema, book)
        })
        .is_err(),
        "Raw insertion stole an unloaded identity",
    )?;
    require(
        storage::export(&raw)? == suspended,
        "Failed reservation check changed raw storage",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn verify_resource_f20_cancellation() -> u32 {
    u32::from(cancellation().is_err())
}

pub fn kerml_cache() -> Result<(), String> {
    let package = intersect_model::compile_rcore(include_str!(
        "../../../fixtures/rcore/day04/kerml-specialization.rcore"
    ))
    .map_err(|error| error.to_string())?;
    let schema = package.schema().clone();
    let context = make_context(&schema)?;
    let ty = class_key(&schema, "Type")?;
    let specialization = class_key(&schema, "Specialization")?;
    let mut set = stores::resources::<FrozenStore>(schema.clone(), context.clone())?;
    set.transaction(|edit| {
        edit.create("general", &ty, "library.kerml")?;
        edit.create("specific", &ty, "user.kerml")?;
        edit.create("relation", &specialization, "user.kerml")?;
        edit.set("relation", "general", json!("general"))?;
        edit.set("relation", "specific", json!("specific"))
    })?;
    let mut expected = identity();
    expected.sources = BTreeMap::from([(
        "bounded.kerml".into(),
        content_digest(b"type general; type specific; specialization specific general;"),
    )]);
    expected.implementations = BTreeMap::from([("projection".into(), "explicit-day04-v1".into())]);
    let selected = vec!["library.kerml".into(), "user.kerml".into()];
    let registry = registry()?;
    let cache = set.prepare_cache(&registry, &selected, expected.clone(), |_| Ok(()))?;
    let before = set.snapshot()?;
    let old = set
        .reader()
        .handle("general")
        .ok_or("Missing general handle")?;
    set.unload(&registry, &selected, UnloadPolicy::DiscardChanges)?;
    set.load_cache(
        &registry,
        &cache,
        &expected,
        |_| Ok(()),
        &InvocationContext::default(),
    )?;
    require(
        handle_id(set.reader(), &old).is_err(),
        "KerML cache revived a stale handle",
    )?;
    require(
        set.reader()
            .get("relation")?
            .ok_or("Missing specialization")?
            .values["general"]
            == json!("general"),
        "KerML cache changed selected target",
    )?;
    let after = set.snapshot()?;
    for element in &before.elements {
        require(
            after
                .elements
                .iter()
                .find(|candidate| candidate.id == element.id)
                == Some(element),
            "KerML cache changed model content",
        )?;
    }
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn verify_resource_f20_kerml() -> u32 {
    u32::from(kerml_cache().is_err())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn measure_cache() -> Result<Value, String> {
    use std::time::Instant;
    let package = package()?;
    let sources = [
        Source {
            uri: "catalog.library".into(),
            text: include_str!("../../../fixtures/library/catalog.library").into(),
        },
        Source {
            uri: "loans.library".into(),
            text: include_str!("../../../fixtures/library/loans.library").into(),
        },
    ];
    let started = Instant::now();
    let fresh = evaluate(&package, &sources);
    let fresh_ns = started.elapsed().as_nanos();
    require(fresh.diagnostics.is_empty(), "Measured fresh source failed")?;
    let context = make_context(&package.schema)?;
    let registry = registry()?;
    let mut source =
        ResourceSet::from_model(package.schema.clone(), context.clone(), &fresh.model)?;
    let selected = source.resources()?.into_keys().collect::<Vec<_>>();
    let cache = source.prepare_cache(&registry, &selected, identity(), |_| Ok(()))?;
    let bytes = cache.to_bytes()?;
    let started = Instant::now();
    let cache = PreparedModelCache::from_bytes(&bytes, CacheLimits::default())?;
    let mut loaded = ResourceSet::new(package.schema.clone(), context)?;
    let mut derived = None;
    loaded.load_cache(
        &registry,
        &cache,
        &identity(),
        |model| {
            let mut result = intersect_text::Evaluation {
                model: model.clone(),
                diagnostics: vec![],
                references: vec![],
                derived: BTreeMap::new(),
            };
            intersect_text::apply_semantics_with_provider(
                &package,
                &mut result,
                &intersect_model::query::PackageDerived(&package),
            );
            require(
                result.diagnostics.is_empty(),
                "Measured cache failed language validation",
            )?;
            derived = Some(result.derived);
            Ok(())
        },
        &InvocationContext::default(),
    )?;
    let cached_ns = started.elapsed().as_nanos();
    require(
        derived == Some(fresh.derived),
        "Measured reload changed derived observations",
    )?;
    require(
        observed(&loaded.snapshot()?, &package.schema)? == observed(&fresh.model, &package.schema)?,
        "Measured reload changed model observations",
    )?;
    Ok(
        json!({"model_elements":fresh.model.elements.len(),"resources":selected.len(),"canonical_cache_bytes":bytes.len(),"fresh_source_nanoseconds":fresh_ns,"cached_reload_nanoseconds":cached_ns,"scope":"single bounded Library observation; no general performance claim"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_residency_reopens_and_reloads_external_identities() -> Result<(), String> {
        let package = package()?;
        let schema = &package.schema;
        let original = build_direct(schema)?;
        let context = make_context(schema)?;
        let formats = registry()?;
        let mut set: ResourceSet =
            ResourceSet::from_model(schema.clone(), context.clone(), &original)?;
        let selection = vec!["catalog.library".to_owned()];
        let all: Vec<_> = set.resources()?.into_keys().collect();
        let ticket = set.prepare_save(&formats, "xmi", &all, ReferenceClosure::SelectedOnly)?;
        let catalog: Vec<_> = ticket
            .resources()
            .iter()
            .filter(|resource| resource.uri == "catalog.library")
            .cloned()
            .collect();
        set.acknowledge_save(&formats, &ticket)?;
        let retained = set.reader().id_page(None, 64)?;
        let reference = set.reader().value("first", "copy")?;
        let book = set.reader().handle("dune").ok_or("Missing book handle")?;
        let loan = set.reader().handle("first").ok_or("Missing loan handle")?;

        set.unload(&formats, &selection, UnloadPolicy::RequireSaved)?;
        assert!(set.reader().inspect_handle(&book, &mut |_| Ok(())).is_err());
        set.reader().inspect_handle(&loan, &mut |view| {
            assert_eq!(view.id(), "first");
            assert!(matches!(
                view.value("copy").and_then(|value| value.reference()),
                Some(ReferenceView::Unavailable { identity: "dune" })
            ));
            Ok(())
        })?;
        let suspended = set.snapshot()?;
        assert_eq!(set.reader().value("first", "copy")?, reference);
        assert_eq!(set.reader().class("dune")?, None);
        assert_eq!(
            set.reader().id_page(None, 64)?,
            retained
                .into_iter()
                .filter(|(_, id)| suspended.elements.iter().any(|row| row.id == *id))
                .collect::<Vec<_>>(),
        );
        assert!(set.validate().is_err());
        assert!(
            set.transaction(|edit| edit.set("first", "name", json!("blocked")))
                .is_err()
        );
        assert!(matches!(
            set.resolve("loans.library", "catalog.library#dune")?,
            TargetResolution::UnavailableResource { .. }
        ));

        let mut reopened =
            ResourceSet::from_resident_model(schema.clone(), context.clone(), &suspended)?;
        let control = InvocationContext::default();
        let prepared =
            reopened.prepare_reload(&formats, "xmi", &selection, &catalog, &context, &control)?;
        reopened.commit_reload(prepared, |_| Ok(()), &control)?;
        reopened.validate()?;
        let book_reloaded = reopened
            .reader()
            .handle("dune")
            .ok_or("Missing reloaded book")?;
        assert_ne!(book_reloaded, book);
        assert!(
            reopened
                .reader()
                .inspect_handle(&book, &mut |_| Ok(()))
                .is_err()
        );
        assert!(reopened.reader().inspect("first", &mut |view| {
            assert!(matches!(
                view.value("copy").and_then(|value| value.reference()),
                Some(ReferenceView::Resolved(target)) if target == book_reloaded
            ));
            Ok(())
        })?);
        assert_eq!(
            observed(&reopened.snapshot()?, schema)?,
            observed(&original, schema)?,
        );
        Ok(())
    }

    #[test]
    fn bounded_kerml_resource_graph_survives_cached_reload() -> Result<(), String> {
        super::kerml_cache()
    }
    #[test]
    fn cancellation_at_each_residency_boundary_restores_exact_state() -> Result<(), String> {
        super::cancellation()
    }
    #[test]
    fn unload_and_reload_preserve_independent_library_observations() -> Result<(), String> {
        super::residency()
    }
    #[test]
    fn fresh_and_cached_libraries_agree_and_invalid_artifacts_roll_back() -> Result<(), String> {
        super::caches()
    }
}
