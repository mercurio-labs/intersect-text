//! Handwritten JSON adapter. Shared ResourceSet owns every lifecycle operation;
//! the browser owns byte transport and exact source/context selection.
use intersect_model::core::{
    interchange::{FormatRegistry, Resource},
    model::Model,
    resource_cache::{CacheIdentity, CacheLimits, PreparedModelCache, content_digest},
    resource_set::{ResourceContext, ResourceSet, UnloadPolicy},
    resources::ResourceLoadState,
    storage::InvocationContext,
};
use intersect_model::definitions::schema::Diagnostic;
use intersect_text::package::Package;
use intersect_text::{Evaluation, Source};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceState {
    version: u32,
    context: ResourceContext,
    model: Model,
}
fn context(package: &Package) -> Result<ResourceContext, String> {
    let bytes = serde_json::to_vec(package).map_err(|error| error.to_string())?;
    ResourceContext::new(
        &package.schema,
        BTreeMap::from([("compiled-language".into(), content_digest(&bytes))]),
    )
}
fn identity(package: &Package, request: &Value) -> Result<CacheIdentity, String> {
    let sources: Vec<Source> = serde_json::from_value(request["sources"].clone())
        .map_err(|error| format!("Cache requires exact model sources: {error}"))?;
    let mut source_ids = BTreeMap::new();
    for source in sources {
        if source_ids
            .insert(source.uri, content_digest(source.text.as_bytes()))
            .is_some()
        {
            return Err("Duplicate cache source URI".into());
        }
    }
    let expected = CacheIdentity {
        sources: source_ids,
        implementations: context(package)?.dependencies().clone(),
        runtime: request["runtime_identity"]
            .as_str()
            .filter(|identity| !identity.trim().is_empty())
            .ok_or("Prepared caches require the host's exact installed runtime identity")?
            .into(),
        format: "xmi".into(),
        format_revision: "intersect-xmi/selected-resources-v1".into(),
    };
    expected.validate()?;
    Ok(expected)
}
fn checked(package: &Package, model: &Model) -> Result<(), String> {
    let result = crate::scripting::inspect(package, model.clone())?;
    if !result.diagnostics.is_empty() {
        return Err(format!(
            "Resource model fails language validation: {:?}",
            result.diagnostics
        ));
    }
    Ok(())
}
fn check_budget(model: &Model) -> Result<(), String> {
    let reserved = model
        .resources
        .values()
        .filter_map(|record| record.unloaded.as_ref())
        .map(|contents| contents.identities.len())
        .sum::<usize>();
    if model.elements.len().saturating_add(reserved) > 4096 || model.resources.len() > 64 {
        return Err("Resource snapshot exceeds browser object/resource limits".into());
    }
    Ok(())
}
fn evaluation(package: &Package, model: Model) -> Result<Evaluation, String> {
    if let Some((uri, _)) = model
        .resources
        .iter()
        .find(|(_, record)| record.load_state != ResourceLoadState::Loaded)
    {
        let diagnostic = Diagnostic::new(
            uri,
            0,
            0,
            "Resource content unavailable. Reload resources before validation or execution.",
        );
        return Ok(Evaluation {
            model,
            diagnostics: vec![diagnostic],
            references: vec![],
            derived: BTreeMap::new(),
        });
    }
    checked(package, &model)?;
    crate::scripting::inspect(package, model)
}

pub fn dispatch(method: &str, request: &Value) -> Result<Value, String> {
    let package = crate::scripting::package(request)?;
    let expected_context = context(&package)?;
    let model = if method == "model/resourceRestore" {
        let state: ResourceState = serde_json::from_value(request["state"].clone())
            .map_err(|error| format!("Invalid resource snapshot: {error}"))?;
        if state.version != 1 || state.context != expected_context {
            return Err(
                "Resource snapshot belongs to different language sources or implementations".into(),
            );
        }
        state.model
    } else {
        serde_json::from_value(request["model"].clone())
            .map_err(|error| format!("Invalid resource model: {error}"))?
    };
    check_budget(&model)?;
    let mut set =
        ResourceSet::from_resident_model(package.schema.clone(), expected_context.clone(), &model)?;
    let mut registry = FormatRegistry::default();
    registry.register(intersect_model::xmi::XmiFormat)?;
    let selection = || -> Result<Vec<String>, String> {
        serde_json::from_value(request["selected"].clone())
            .map_err(|error| format!("Invalid resource selection: {error}"))
    };
    match method {
        "model/resourceSnapshot" => {
            return Ok(json!(ResourceState {
                version: 1,
                context: expected_context,
                model: set.snapshot()?
            }));
        }
        "model/resourceRestore" => {}
        "model/resourceCreate" => {
            set.create_resource(request["uri"].as_str().ok_or("Missing resource URI")?)?
        }
        "model/resourceRemove" => {
            for uri in selection()? {
                set.remove_resource(&uri)?;
            }
        }
        "model/resourceUnload" => {
            let policy = match request["policy"].as_str() {
                Some("discard") => UnloadPolicy::DiscardChanges,
                Some("saved") | None => UnloadPolicy::RequireSaved,
                _ => return Err("Unknown resource unload policy".into()),
            };
            set.unload(&registry, &selection()?, policy)?;
        }
        "model/resourceReload" => {
            let bytes: Vec<Resource> = serde_json::from_value(request["resources"].clone())
                .map_err(|error| error.to_string())?;
            let prepared = set.prepare_reload(
                &registry,
                "xmi",
                &selection()?,
                &bytes,
                &expected_context,
                &InvocationContext::default(),
            )?;
            set.commit_reload(
                prepared,
                |candidate| checked(&package, candidate),
                &InvocationContext::default(),
            )?;
        }
        "model/cachePrepare" => {
            if model.elements.iter().any(|element| {
                model
                    .resources
                    .get(&element.resource)
                    .is_some_and(|record| record.source.is_none())
            }) {
                return Err("Prepared browser caches require exact current source correspondence; export edited instances as resources instead".into());
            }
            let cache = set.prepare_cache(
                &registry,
                &selection()?,
                identity(&package, request)?,
                |candidate| checked(&package, candidate),
            )?;
            return Ok(
                json!({"bytes": cache.to_bytes()?, "resources": cache.resources().iter().map(|resource| &resource.uri).collect::<Vec<_>>()}),
            );
        }
        "model/cacheLoad" => {
            let bytes: Vec<u8> = serde_json::from_value(request["bytes"].clone())
                .map_err(|error| error.to_string())?;
            let cache = PreparedModelCache::from_bytes(
                &bytes,
                CacheLimits {
                    max_bytes: 500_000,
                    max_resources: 64,
                },
            )?;
            set.load_cache(
                &registry,
                &cache,
                &identity(&package, request)?,
                |candidate| checked(&package, candidate),
                &InvocationContext::default(),
            )?;
        }
        _ => return Err(format!("Unknown resource lifecycle method: {method}")),
    }
    check_budget(&set.snapshot()?)?;
    Ok(
        json!({"evaluation": evaluation(&package, set.snapshot()?)?, "changes": set.drain_changes(), "context": expected_context}),
    )
}
