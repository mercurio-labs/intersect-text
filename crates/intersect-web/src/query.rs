//! Handwritten query host boundary; no query or language semantics live in JS.
use crate::semantic_bindings::Derived;
use intersect_model::core::{
    model::Model,
    query::{Parameters, QueryLimits, QueryPlan},
    storage::ModelView,
};
use serde_json::Value;

pub fn execute(request: &Value) -> Result<Value, String> {
    execute_context(request, false)
}
pub fn execute_with_conversions(request: &Value) -> Result<Value, String> {
    execute_context(request, true)
}
fn execute_context(request: &Value, converted_columns: bool) -> Result<Value, String> {
    let package = crate::scripting::package(request)?;
    let model: Model = serde_json::from_value(request["model"].clone())
        .map_err(|e| format!("Invalid query model: {e}"))?;
    if model.elements.len() > 4096 {
        return Err("Workbench query model budget exceeded (4096 elements)".into());
    }
    // This adapter accepts compatibility models across a trust boundary.
    // Native callers query their existing store directly.
    crate::conversions::with_package(&package, |registry| {
        model.validate_with_conversions(&package.schema, registry)
    })?;
    let plan: QueryPlan = serde_json::from_value(request["query"].clone())
        .map_err(|e| format!("Invalid query plan: {e}"))?;
    let parameters: Parameters = serde_json::from_value(
        request
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
    )
    .map_err(|e| format!("Invalid query parameters: {e}"))?;
    let view = ModelView::new(&model)?;
    let result = crate::conversions::with_package(&package, |registry| {
        if !converted_columns {
            return plan.execute(
                &package.schema,
                &view,
                &parameters,
                &Derived::new(&package),
                QueryLimits::default(),
            );
        }
        plan.execute_with_conversions(
            &package.schema,
            &view,
            &parameters,
            &Derived::new(&package),
            registry,
            QueryLimits::default(),
        )
    })?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}
