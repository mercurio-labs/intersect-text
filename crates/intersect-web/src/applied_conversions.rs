//! Pure applied-value construction/checking through installed language providers.
use intersect_model::definitions::{
    bundle::DefinitionBundle,
    conversion::{AppliedConversionContract, AppliedConvertedValue},
    generic_types::ClosedType,
};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Bind {
        application: ClosedType,
        payload: Value,
    },
    Validate {
        application: ClosedType,
        value: Box<AppliedConvertedValue>,
    },
}
pub fn access(request: &Value) -> Result<Value, String> {
    let bundle: DefinitionBundle = serde_json::from_value(
        request
            .get("bundle")
            .cloned()
            .ok_or("Missing definition bundle")?,
    )
    .map_err(|e| e.to_string())?;
    if bundle
        .sources
        .iter()
        .any(|source| source.text.len() > 256_000)
    {
        return Err("Definition size budget exceeded".into());
    }
    let contracts: Vec<AppliedConversionContract> = serde_json::from_value(
        request
            .get("conversions")
            .cloned()
            .ok_or("Missing applied conversions")?,
    )
    .map_err(|e| e.to_string())?;
    let operation: Operation = serde_json::from_value(
        request
            .get("operation")
            .cloned()
            .ok_or("Missing applied conversion operation")?,
    )
    .map_err(|e| e.to_string())?;
    let constructors = crate::conversions::constructor_inputs(request, "constructors")?;
    let result = intersect_model::conversion::with_catalog_policies(
        &bundle,
        &[],
        &contracts,
        &constructors,
        |context| {
            let registry = context.applied();
            let value = match operation {
                Operation::Bind {
                    application,
                    payload,
                } => registry.bind(&application, payload)?,
                Operation::Validate { application, value } => {
                    registry.validate_for(&application, &value)?;
                    *value
                }
            };
            Ok(json!({"ok":true,"value_verified":true,"model_published":false,"value":value}))
        },
    );
    Ok(match result {
        Ok(value) => value,
        Err(error) => {
            json!({"ok":false,"value_verified":false,"model_published":false,"diagnostics":[{"message":error}]})
        }
    })
}
