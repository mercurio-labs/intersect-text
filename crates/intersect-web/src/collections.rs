//! Pure checked collection transitions; model publication remains transactional.
use intersect_model::definitions::{
    conversion::{AppliedConvertedValue, ConvertedValue},
    generic_types::ClosedType,
    resources::DeclarationId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Empty,
    Clear,
    Insert {
        index: usize,
        item: Value,
    },
    Set {
        index: usize,
        item: Value,
    },
    RemoveAt {
        index: usize,
    },
    Move {
        new_position: usize,
        old_position: usize,
    },
    Get {
        key: Value,
    },
    Put {
        key: Value,
        item: Value,
    },
    RemoveKey {
        key: Value,
    },
}

pub fn step(request: &Value) -> Result<Value, String> {
    let retained = request.get("package").ok_or("Missing package")?;
    let (datatype, application) = selection(request)?;
    let operation: Operation = serde_json::from_value(
        request
            .get("operation")
            .cloned()
            .ok_or("Missing collection operation")?,
    )
    .map_err(|e| e.to_string())?;
    if serde_json::to_value(&operation).map_err(|e| e.to_string())? != request["operation"] {
        return Err("Collection operation requires explicit canonical arguments".into());
    }
    crate::conversions::with_rebuilt(retained, |package, registry| {
        let adapter =
            selected_provider(package, registry, datatype.as_ref(), application.as_ref())?;
        let mut payload = if matches!(operation, Operation::Empty) {
            if request.get("collection").is_some() {
                return Err(
                    "Empty creates a collection without consuming an input collection".into(),
                );
            }
            adapter.empty()?
        } else {
            let value = request
                .get("collection")
                .cloned()
                .ok_or("Missing input collection")?;
            if let Some(application) = &application {
                let collection: AppliedConvertedValue =
                    serde_json::from_value(value).map_err(|e| e.to_string())?;
                registry
                    .applied()
                    .validate_for(application, &collection)?
                    .clone()
            } else {
                let collection: ConvertedValue =
                    serde_json::from_value(value).map_err(|e| e.to_string())?;
                let ConvertedValue::Converted {
                    datatype: actual, ..
                } = &collection;
                if Some(actual) != datatype.as_ref() {
                    return Err("Collection datatype differs from selected provider".into());
                }
                registry.validate(&collection)?.clone()
            }
        };
        let result = match operation {
            Operation::Empty => None,
            Operation::Clear => {
                adapter.clear(&mut payload)?;
                None
            }
            Operation::Insert { index, item } => {
                adapter.insert(&mut payload, index, item)?;
                None
            }
            Operation::Set { index, item } => Some(adapter.set(&mut payload, index, item)?),
            Operation::RemoveAt { index } => Some(adapter.remove_at(&mut payload, index)?),
            Operation::Move {
                new_position,
                old_position,
            } => Some(adapter.move_item(&mut payload, new_position, old_position)?),
            Operation::Get { key } => adapter.get(&payload, &key)?,
            Operation::Put { key, item } => adapter.put(&mut payload, key, item)?,
            Operation::RemoveKey { key } => adapter.remove_key(&mut payload, &key)?,
        };
        let collection = if let Some(application) = &application {
            json!(registry.applied().bind(application, payload)?)
        } else {
            json!(registry.bind(
                datatype.as_ref().ok_or("Missing collection datatype")?,
                payload
            )?)
        };
        Ok(json!({"collection":collection,"result":{"present":result.is_some(),"value":result}}))
    })
}

fn selection(request: &Value) -> Result<(Option<DeclarationId>, Option<ClosedType>), String> {
    if request.get("datatype").is_some() && request.get("application").is_some() {
        return Err("Select a collection datatype or application, not both".into());
    }
    let application: Option<ClosedType> = request
        .get("application")
        .map(|value| serde_json::from_value(value.clone()).map_err(|e| e.to_string()))
        .transpose()?;
    let datatype: Option<DeclarationId> = if application.is_none() {
        Some(
            serde_json::from_value(
                request
                    .get("datatype")
                    .cloned()
                    .ok_or("Missing collection datatype or application")?,
            )
            .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };
    Ok((datatype, application))
}

fn selected_provider(
    package: &intersect_text::package::Package,
    registry: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
    datatype: Option<&DeclarationId>,
    application: Option<&ClosedType>,
) -> Result<intersect_model::collection_conversion::CollectionConverter, String> {
    let bundle = crate::conversions::retained_bundle(package)?;
    if let Some(application) = application {
        let contract = package
            .schema
            .applied_conversion_contracts
            .iter()
            .find(|contract| &contract.application == application)
            .ok_or("Missing applied collection provider contract")?;
        intersect_model::conversion::applied_collection_provider_with_conversions(
            &bundle, contract, registry,
        )
    } else {
        let datatype = datatype.ok_or("Missing collection datatype")?;
        let contract = package
            .schema
            .conversion_contracts
            .iter()
            .find(|contract| &contract.datatype == datatype)
            .ok_or("Missing collection provider contract")?;
        intersect_model::conversion::collection_provider_with_conversions(
            &bundle, contract, registry,
        )
    }
}

/// Checked read-only access to a selected collection member's immutable model context.
pub fn reference(request: &Value, bind: bool) -> Result<Value, String> {
    let retained = request.get("package").ok_or("Missing package")?;
    let (datatype, application) = selection(request)?;
    let member = request
        .get("member")
        .and_then(Value::as_u64)
        .ok_or("Missing nonnegative collection member index")?;
    let member =
        usize::try_from(member).map_err(|_| "Collection member index exceeds host range")?;
    crate::conversions::with_rebuilt(retained, |package, registry| {
        let adapter =
            selected_provider(package, registry, datatype.as_ref(), application.as_ref())?;
        if bind {
            adapter.bind_reference(
                member,
                request
                    .get("element")
                    .and_then(Value::as_str)
                    .ok_or("Missing reference element identity")?,
            )
        } else {
            serde_json::to_value(adapter.resolve_reference(
                member,
                request.get("reference").ok_or("Missing reference value")?,
            )?)
            .map_err(|error| error.to_string())
        }
    })
}
