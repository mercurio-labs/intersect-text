//! Host option/transport checks only. Inference and archive semantics are shared.
use intersect_text::{distribution, inference::InferenceRequest};
use serde_json::Value;
pub fn inputs(request: &Value) -> Result<InferenceRequest, String> {
    if request
        .get("inference_context")
        .is_some_and(|value| !value.is_null())
    {
        return Err("Inferred compilation requires the explicit context method".into());
    }
    source_inputs(request)
}
pub(crate) fn source_inputs(request: &Value) -> Result<InferenceRequest, String> {
    for option in [
        "conversions",
        "applied_conversions",
        "constructors",
        "bindings",
        "xtext_profile",
        "compilation",
    ] {
        if request.get(option).is_some_and(|value| !value.is_null()) {
            return Err(format!("Inferred compilation does not accept {option}"));
        }
    }
    serde_json::from_value(
        request
            .get("inference")
            .cloned()
            .ok_or("Missing inference source inputs")?,
    )
    .map_err(|e| format!("Invalid inference source inputs: {e}"))
}
pub fn export(request: &Value) -> Result<Value, String> {
    let metadata = serde_json::from_value(
        request
            .get("metadata")
            .cloned()
            .ok_or("Missing package metadata")?,
    )
    .map_err(|e| format!("Invalid package metadata: {e}"))?;
    serde_json::to_value(distribution::build_inferred(inputs(request)?, metadata)?)
        .map_err(|e| e.to_string())
}

/// A read-only preview rebuilt from original inference sources. Cached generated
/// inputs are never accepted as a source recipe or executable authority.
pub fn structure_preview(request: &Value) -> Result<Value, String> {
    let language = inputs(request)?
        .compile(intersect_text::grammar::SourceLimits::default())
        .map_err(|diagnostics| format!("Cannot rebuild inferred preview: {diagnostics:?}"))?;
    structure_preview_language(request, &language)
}

pub(crate) fn structure_preview_language(
    request: &Value,
    language: &intersect_text::inference::InferredLanguage,
) -> Result<Value, String> {
    let selected: intersect_model::definitions::bundle::SourceId =
        serde_json::from_value(request["generated_source"].clone())
            .map_err(|e| format!("Invalid generated preview source: {e}"))?;
    let definitions = language.definitions();
    if !definitions
        .generated_sources
        .iter()
        .any(|s| s.id == selected)
    {
        return Err("Preview must select an exact freshly generated source".into());
    }
    let namespace = definitions
        .bundle
        .namespaces
        .iter()
        .find(|n| n.source == selected)
        .ok_or("Missing exact generated namespace binding")?
        .namespace
        .clone();
    let projection = intersect_model::definitions::rcore::lower_structure_bundle(
        &definitions.bundle,
        &selected,
        &namespace,
        "memory:inferred-preview",
    )
    .map_err(|diagnostics| format!("Cannot project inferred resource context: {diagnostics:?}"))?;
    let lowering = projection
        .lowerings
        .iter()
        .find(|l| l.original.id == selected)
        .ok_or("Missing selected generated projection")?;
    let document = intersect_model::definitions::ecore_document::inspect_in_bundle(
        &projection.bundle,
        &projection.entry,
        &namespace,
    )?;
    Ok(
        serde_json::json!({"ok":true,"document":document,"lowering":lowering,"projection":projection}),
    )
}

/// Preserve the established response and diagnostic DTOs for both host routes.
pub(crate) fn compiled_response(
    result: Result<
        intersect_text::inference::InferredLanguage,
        Vec<intersect_model::definitions::schema::Diagnostic>,
    >,
    origins: bool,
) -> Value {
    match result {
        Ok(language) => {
            serde_json::json!({"ok":true,"keywords":intersect_text::keywords(language.package()),"package":language.package(),"generated_definitions":{"sources":language.definitions().generated_sources,"entry":language.definitions().entry_source,"origins":language.definitions().origins}})
        }
        Err(diagnostics) if origins => {
            let origins = diagnostics
                .iter()
                .map(|d| {
                    d.origin.as_deref().cloned().unwrap_or_else(|| {
                        intersect_model::definitions::schema::DiagnosticOrigin::Unavailable {
                            reason: "Native producer did not retain an exact source identity"
                                .into(),
                        }
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({"ok":false,"diagnostics":diagnostics,"diagnostic_origins":origins})
        }
        Err(diagnostics) => serde_json::json!({"ok":false,"diagnostics":diagnostics}),
    }
}
