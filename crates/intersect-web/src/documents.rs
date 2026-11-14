//! JSON transport over the same source-rebuilt durable document interpreter.
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentAnalysisSet, DocumentLanguage},
};
use serde_json::Value;

pub fn analyze(request: &Value) -> Result<Value, String> {
    with_analysis(request, |_, analysis| {
        serde_json::to_value(analysis).map_err(|error| error.to_string())
    })
}

/// A locator is checked against a fresh source-authoritative analysis. Serialized
/// nodes, semantic tables and model objects never enter this lookup as authority.
pub fn linked_object(request: &Value) -> Result<Value, String> {
    let expected_source: intersect_model::definitions::bundle::SourceId = serde_json::from_value(
        request
            .get("source")
            .cloned()
            .ok_or("Missing document source identity")?,
    )
    .map_err(|error| format!("Invalid document source identity: {error}"))?;
    let expected_schema = request
        .get("schema_revision")
        .and_then(Value::as_str)
        .ok_or("Missing document schema revision")?;
    let revision = request
        .get("resource_revision")
        .and_then(Value::as_str)
        .ok_or("Missing document resource revision")?
        .parse::<u64>()
        .map_err(|_| "Invalid document resource revision")?;
    let node = usize::try_from(
        request
            .get("node")
            .and_then(Value::as_u64)
            .ok_or("Missing document node index")?,
    )
    .map_err(|_| "Document node index overflow")?;
    with_analysis(request, |package, analysis| {
        let evaluation = analysis
            .valid_model
            .as_ref()
            .ok_or("Document links require a complete valid document batch")?;
        let mut matching = analysis
            .snapshots
            .iter()
            .filter(|snapshot| snapshot.document().source() == &expected_source);
        let snapshot = matching.next().ok_or("Document source identity changed")?;
        if matching.next().is_some() {
            return Err("Ambiguous document source identity".into());
        }
        if snapshot.schema_revision() != expected_schema
            || snapshot.document().resource_revision() != revision
        {
            return Err("Document schema or resource revision changed".into());
        }
        let handle = snapshot
            .document()
            .handle(intersect_model::core::text_document::NodeId(node))?;
        let object = snapshot.linked_object(&handle, &evaluation.model, &package.schema)?;
        Ok(
            serde_json::json!({"source":expected_source,"schema_revision":expected_schema,"resource_revision":revision.to_string(),"node":node,"object":object}),
        )
    })
}
fn with_analysis(
    request: &Value,
    consume: impl FnOnce(
        &intersect_text::package::Package,
        &DocumentAnalysisSet,
    ) -> Result<Value, String>,
) -> Result<Value, String> {
    let package = crate::scripting::package(request)?;
    let sources: Vec<Source> = serde_json::from_value(
        request
            .get("sources")
            .cloned()
            .ok_or("Missing document sources")?,
    )
    .map_err(|error| format!("Invalid document sources: {error}"))?;
    let entry = match request.get("entry_rule") {
        None | Some(Value::Null) => None,
        Some(Value::String(name)) => Some(name.as_str()),
        _ => return Err("entry_rule must be a parser entry name or null".into()),
    };
    crate::conversions::with_package(&package, |registry| {
        let language = DocumentLanguage::prepare_with_context(
            &package,
            crate::inferred_context::contracts(&package),
            Some(registry),
            Default::default(),
            AnalysisLimits::workbench(),
        )
        .map_err(|errors| format!("Invalid document language: {errors:?}"))?;
        let analysis =
            crate::inferred_context::with_text_settings(language.package(), |settings| {
                language.analyze_sources_with_context(
                    &sources,
                    entry,
                    EvaluationLimits::workbench(),
                    Some(registry),
                    settings,
                    &crate::semantic_bindings::Derived::new(language.package()),
                )
            })?;
        consume(language.package(), &analysis)
    })
}
