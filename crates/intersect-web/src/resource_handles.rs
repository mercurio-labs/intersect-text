//! Read-only access to exact resources admitted by a retained provider contract.
use intersect_model::definitions::conversion::ConvertedValue;
use intersect_model::resource_conversion::{ResourceConverter, ResourceKind};
use serde_json::{Value, json};
pub fn access(request: &Value) -> Result<Value, String> {
    let package = crate::conversions::rebuild(request.get("package").ok_or("Missing package")?)?;
    let handle: ConvertedValue = serde_json::from_value(
        request
            .get("handle")
            .cloned()
            .ok_or("Missing resource handle")?,
    )
    .map_err(|e| e.to_string())?;
    let ConvertedValue::Converted {
        datatype,
        provider,
        value,
    } = &handle;
    let kind = match (provider.name.as_str(), provider.version.as_str()) {
        ("ecore.definition-resource", "1") => ResourceKind::Resource,
        ("ecore.definition-resource-set", "1") => ResourceKind::Set,
        _ => return Err("Resource access requires an installed resource adapter".into()),
    };
    crate::conversions::with_package(&package, |registry| {
        registry.validate(&handle)?;
        let contract = package
            .schema
            .conversion_contracts
            .iter()
            .find(|contract| &contract.datatype == datatype)
            .ok_or("Missing resource conversion contract")?;
        let bundle = crate::conversions::retained_bundle(&package)?;
        let adapter = ResourceConverter::new(contract.clone(), kind, &bundle)?;
        let from = serde_json::from_value(
            request
                .get("from")
                .cloned()
                .ok_or("Missing referring resource")?,
        )
        .map_err(|e| e.to_string())?;
        match request["method"].as_str() {
            Some("resource/read") => Ok(json!({"source":adapter.document(value,&from)?.source})),
            Some("resource/resolve") => {
                let href = request
                    .get("href")
                    .and_then(Value::as_str)
                    .ok_or("Missing resource href")?;
                Ok(json!({"declaration":adapter.resolve(value,&from,href)?}))
            }
            _ => Err("Unknown resource access operation".into()),
        }
    })
}
