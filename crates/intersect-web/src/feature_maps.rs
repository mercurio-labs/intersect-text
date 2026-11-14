//! Pure feature-map transitions through the language-owned checked adapter.
use intersect_model::definitions::{conversion::ConvertedValue, resources::DeclarationId};
use intersect_model::feature_map_conversion::Entry;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Entry {
        feature: DeclarationId,
        value: Value,
    },
    Empty,
    Clear,
    Insert {
        index: usize,
        entry: Entry,
    },
    Set {
        index: usize,
        entry: Entry,
    },
    Remove {
        index: usize,
    },
    Move {
        new_position: usize,
        old_position: usize,
    },
    Values {
        feature: DeclarationId,
    },
}
pub fn step(request: &Value) -> Result<Value, String> {
    let package = crate::conversions::rebuild(request.get("package").ok_or("Missing package")?)?;
    let datatype: DeclarationId = serde_json::from_value(
        request
            .get("datatype")
            .cloned()
            .ok_or("Missing feature-map datatype")?,
    )
    .map_err(|e| e.to_string())?;
    let contract = package
        .schema
        .conversion_contracts
        .iter()
        .find(|c| c.datatype == datatype)
        .ok_or("Missing feature-map provider contract")?;
    let operation: Operation = serde_json::from_value(
        request
            .get("operation")
            .cloned()
            .ok_or("Missing feature-map operation")?,
    )
    .map_err(|e| e.to_string())?;
    if serde_json::to_value(&operation).map_err(|e| e.to_string())? != request["operation"] {
        return Err("Feature-map operation requires explicit canonical arguments".into());
    }
    crate::conversions::with_package(&package, |registry| {
        let adapter = intersect_model::conversion::feature_map_provider_with_conversions(
            &crate::conversions::retained_bundle(&package)?,
            contract,
            registry,
        )?;
        let mut payload = match &operation {
            Operation::Entry { feature, value } => {
                if contract.provider.name != "ecore.feature-entry" {
                    return Err("Entry construction requires an entry provider".into());
                }
                if request.get("value").is_some() {
                    return Err("Entry construction does not consume an input value".into());
                }
                serde_json::to_value(adapter.entry(feature.clone(), value.clone())?)
                    .map_err(|e| e.to_string())?
            }
            Operation::Empty => {
                if request.get("value").is_some() {
                    return Err("Empty construction does not consume an input value".into());
                }
                adapter.empty()?
            }
            _ => {
                let value: ConvertedValue = serde_json::from_value(
                    request
                        .get("value")
                        .cloned()
                        .ok_or("Missing feature-map input value")?,
                )
                .map_err(|e| e.to_string())?;
                let ConvertedValue::Converted {
                    datatype: actual, ..
                } = &value;
                if *actual != datatype {
                    return Err("Feature-map datatype differs from selected provider".into());
                }
                registry.validate(&value)?.clone()
            }
        };
        let result = match operation {
            Operation::Entry { .. } | Operation::Empty => None,
            Operation::Clear => {
                adapter.clear(&mut payload)?;
                None
            }
            Operation::Insert { index, entry } => {
                adapter.insert(&mut payload, index, entry)?;
                None
            }
            Operation::Set { index, entry } => Some(
                serde_json::to_value(adapter.set(&mut payload, index, entry)?)
                    .map_err(|e| e.to_string())?,
            ),
            Operation::Remove { index } => Some(
                serde_json::to_value(adapter.remove(&mut payload, index)?)
                    .map_err(|e| e.to_string())?,
            ),
            Operation::Move {
                new_position,
                old_position,
            } => Some(
                serde_json::to_value(adapter.move_entry(
                    &mut payload,
                    new_position,
                    old_position,
                )?)
                .map_err(|e| e.to_string())?,
            ),
            Operation::Values { feature } => Some(json!(adapter.values(&payload, &feature)?)),
        };
        Ok(json!({"value":registry.bind(&datatype,payload)?,"result":result}))
    })
}

/// Read-only access to the selected immutable reference context.
pub fn reference(request: &Value, bind: bool) -> Result<Value, String> {
    let package = crate::conversions::rebuild(request.get("package").ok_or("Missing package")?)?;
    let datatype: DeclarationId = serde_json::from_value(
        request
            .get("datatype")
            .cloned()
            .ok_or("Missing feature-map datatype")?,
    )
    .map_err(|e| e.to_string())?;
    let feature: DeclarationId = serde_json::from_value(
        request
            .get("feature")
            .cloned()
            .ok_or("Missing feature identity")?,
    )
    .map_err(|e| e.to_string())?;
    let contract = package
        .schema
        .conversion_contracts
        .iter()
        .find(|c| c.datatype == datatype)
        .ok_or("Missing feature-map provider contract")?;
    crate::conversions::with_package(&package, |registry| {
        let adapter = intersect_model::conversion::feature_map_provider_with_conversions(
            &crate::conversions::retained_bundle(&package)?,
            contract,
            registry,
        )?;
        if bind {
            adapter.bind_reference(
                &feature,
                request
                    .get("element")
                    .and_then(Value::as_str)
                    .ok_or("Missing reference element identity")?,
            )
        } else {
            serde_json::to_value(adapter.resolve_reference(
                &feature,
                request.get("reference").ok_or("Missing reference value")?,
            )?)
            .map_err(|e| e.to_string())
        }
    })
}
