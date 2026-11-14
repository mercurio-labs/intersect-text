//! Handwritten operation adapter; exact declarations are rebuilt and checked
//! against the same source-compiled function image before every invocation.
use crate::rcore_behavior_functions;
use intersect_model::definitions::{generic_types::RuntimeType, resources::DeclarationId};
use intersect_model::rcore_behavior_functions::{ExecutionContext, evaluate};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
fn field<T: DeserializeOwned>(request: &Value, name: &str) -> Result<T, String> {
    let raw = request
        .get(name)
        .cloned()
        .ok_or_else(|| format!("Missing {name}"))?;
    serde_json::from_value(raw).map_err(|e| format!("Invalid {name}: {e}"))
}
pub(crate) fn handle(request: &Value) -> Result<Value, String> {
    let receiver: RuntimeType = field(request, "receiver_type")?;
    let operation: DeclarationId = field(request, "operation")?;
    let materialized: Vec<RuntimeType> = field(request, "materialized_types")?;
    let compile_only = request["method"] == "rcore/compileOperation";
    let mut normalized = request.clone();
    let object = normalized
        .as_object_mut()
        .ok_or("Expected operation request object")?;
    for key in ["receiver_type", "operation", "materialized_types"] {
        object.remove(key);
    }
    object.insert(
        "method".into(),
        json!(if compile_only {
            "rcore_behavior/compileFunction"
        } else {
            "rcore_behavior/evaluateFunction"
        }),
    );
    if !compile_only {
        let receiver_argument = object
            .remove("receiver_argument")
            .ok_or("Missing receiver_argument")?;
        let arguments = object
            .get_mut("arguments")
            .and_then(Value::as_array_mut)
            .ok_or("Expected operation arguments array")?;
        arguments.insert(0, receiver_argument);
    }
    rcore_behavior_functions::with_prepared(&normalized, |ready| {
        let binding = match intersect_model::definitions::rcore_operations::verify_binding(
            &ready.bundle,
            &ready.image,
            &receiver,
            &operation,
            &materialized,
            &ready.verification,
        ) {
            Ok(binding) => binding,
            Err(diagnostics) => {
                return Ok(
                    json!({"ok":false,"stage":"operation_binding","source_backed":true,"diagnostics":diagnostics}),
                );
            }
        };
        if ready.compile_only {
            return Ok(
                json!({"ok":true,"stage":"operation_binding","source_backed":true,"binding":binding,"program":ready.image.program(),"effects":ready.image.effects()}),
            );
        }
        let context = ExecutionContext {
            bundle: &ready.bundle,
            conversions: Some(&ready.registry),
            references: &ready.references,
        };
        let result = evaluate(&ready.image, &ready.arguments, &context, &ready.execution);
        let mut outcome = rcore_behavior_functions::outcome(
            serde_json::to_value(ready.image.program()).map_err(|e| e.to_string())?,
            result,
        );
        outcome["stage"] = json!("operation_execution");
        outcome["binding"] = json!(binding);
        Ok(outcome)
    })
}
