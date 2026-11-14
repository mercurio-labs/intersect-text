//! Explicit snapshot transformation through the retained package's checked provider.
use intersect_model::definitions::conversion::ConvertedValue;
use intersect_model::diagnostic_chain_conversion::{Diagnostic, DiagnosticChain};
use serde_json::{Value, json};
pub fn collect(request: &Value) -> Result<Value, String> {
    let package = crate::conversions::rebuild(request.get("package").ok_or("Missing package")?)?;
    let converted: ConvertedValue =
        serde_json::from_value(request.get("chain").cloned().ok_or("Missing chain")?)
            .map_err(|e| e.to_string())?;
    let ConvertedValue::Converted {
        datatype,
        provider,
        value,
    } = &converted;
    if provider.name != "ecore.diagnostic-chain" || provider.version != "1" {
        return Err("Diagnostic accumulation requires the installed chain provider".into());
    }
    crate::conversions::with_package(&package, |registry| {
        registry.validate(&converted)?;
        let mut chain = DiagnosticChain::from_wire(value)?;
        let diagnostic =
            Diagnostic::from_wire(request.get("diagnostic").ok_or("Missing diagnostic")?)?;
        match request.get("operation").and_then(Value::as_str) {
            Some("add") => chain.add(diagnostic)?,
            Some("add_all") => chain.add_all(diagnostic)?,
            Some("merge") => chain.merge(diagnostic)?,
            _ => return Err("Unknown diagnostic chain operation".into()),
        }
        let severity = chain.severity();
        let result = registry.bind(
            datatype,
            serde_json::to_value(chain).map_err(|e| e.to_string())?,
        )?;
        Ok(json!({"chain":result,"severity":severity}))
    })
}
