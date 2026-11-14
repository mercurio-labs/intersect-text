//! Handwritten browser/native host adapter over registered model codecs.
use intersect_model::core::{
    interchange::{FormatRegistry, ReferenceClosure, Resource},
    resource_set::{ResourceContext, ResourceSet},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn selected(request: &Value) -> Result<Option<Vec<String>>, String> {
    request
        .get("selected")
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|e| format!("Invalid resource selection: {e}"))
        })
        .transpose()
}

pub fn dispatch(method: &str, request: &Value) -> Result<Value, String> {
    if method.starts_with("model/resource") || method.starts_with("model/cache") {
        return crate::resource_lifecycle::dispatch(method, request);
    }
    let mut registry = FormatRegistry::default();
    registry.register(intersect_model::xmi::XmiFormat)?;
    if method == "model/formats" {
        return Ok(json!(registry.formats().map(|f| json!({"id": f.id(), "extensions": f.extensions(), "media_type": f.media_type()})).collect::<Vec<_>>()));
    }
    let package = crate::scripting::package(request)?;
    let format = request["format"].as_str().ok_or("Missing model format")?;
    match method {
        "model/import" => {
            let resources: Vec<Resource> =
                serde_json::from_value(request["resources"].clone()).map_err(|e| e.to_string())?;
            if resources.len() > 64
                || resources.iter().map(|r| r.bytes.len() as u64).sum::<u64>() > 4_000_000
            {
                return Err("Model import exceeds browser resource/byte budget".into());
            }
            let model = if let Some(selected) = selected(request)? {
                let current = serde_json::from_value(request["model"].clone())
                    .map_err(|e| format!("Selected import requires the retained model: {e}"))?;
                // The source package was rebuilt at the host trust boundary above.
                // This request has exactly that package's schema/semantic context.
                let context = ResourceContext::new(&package.schema, BTreeMap::new())?;
                let mut set =
                    ResourceSet::from_model(package.schema.clone(), context.clone(), &current)?;
                set.import_with(
                    &registry,
                    format,
                    &selected,
                    &resources,
                    &context,
                    |candidate| {
                        let evaluation = crate::scripting::inspect(&package, candidate.clone())?;
                        if !evaluation.diagnostics.is_empty() {
                            return Err(format!(
                                "Imported model fails language validation: {:?}",
                                evaluation.diagnostics
                            ));
                        }
                        Ok(())
                    },
                )?;
                set.snapshot()?
            } else {
                registry.import(format, &package.schema, &resources)?
            };
            let evaluation = crate::scripting::inspect(&package, model)?;
            if !evaluation.diagnostics.is_empty() {
                return Err(format!(
                    "Imported model fails language validation: {:?}",
                    evaluation.diagnostics
                ));
            }
            Ok(json!(evaluation))
        }
        "model/export" => {
            let model =
                serde_json::from_value(request["model"].clone()).map_err(|e| e.to_string())?;
            let evaluation = crate::scripting::inspect(&package, model)?;
            if !evaluation.diagnostics.is_empty() {
                return Err("Cannot export a model with language diagnostics".into());
            }
            let resources = if let Some(selected) = selected(request)? {
                let closure = match request["closure"].as_str().unwrap_or("selected") {
                    "selected" => ReferenceClosure::SelectedOnly,
                    "dependencies" => ReferenceClosure::IncludeDependencies,
                    _ => return Err("Unknown serialization closure policy".into()),
                };
                registry.export_selected(
                    format,
                    &package.schema,
                    &evaluation.model,
                    &selected,
                    closure,
                )?
            } else {
                registry.export(format, &package.schema, &evaluation.model)?
            };
            Ok(json!({"resources": resources}))
        }
        _ => Err(format!("Unknown model interchange method: {method}")),
    }
}
