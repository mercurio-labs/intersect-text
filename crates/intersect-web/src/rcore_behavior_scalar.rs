//! Source-backed scalar execution adapter shared by native and WASM hosts.
use intersect_model::definitions::{
    bundle::{DefinitionBundle, SourceId},
    conversion::{AppliedConversionContract, ConversionContract},
    rcore_behavior::{
        ir::{Program, Value as ScalarValue, VerificationLimits},
        lowering,
        modules::SymbolId,
    },
};
use intersect_model::rcore_behavior_execution::{self, ExecutionLimits};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

fn field<T: DeserializeOwned>(request: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(
        request
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Missing {name}"))?,
    )
    .map_err(|error| format!("Invalid {name}: {error}"))
}
fn bounded_limits<T: DeserializeOwned + Serialize + Default>(
    request: &Value,
    name: &str,
) -> Result<T, String> {
    let limits: T = match request.get(name) {
        Some(value) => {
            // Apply the host ceiling before usize decoding so native and WASM
            // reject oversized limits with the same policy and message.
            let ceiling = serde_json::to_value(T::default()).map_err(|e| e.to_string())?;
            check_ceiling(value, &ceiling, name)?;
            serde_json::from_value(value.clone()).map_err(|e| format!("Invalid {name}: {e}"))?
        }
        None => T::default(),
    };
    let requested = serde_json::to_value(&limits).map_err(|e| e.to_string())?;
    let ceiling = serde_json::to_value(T::default()).map_err(|e| e.to_string())?;
    check_ceiling(&requested, &ceiling, name)?;
    Ok(limits)
}
fn check_ceiling(requested: &Value, ceiling: &Value, path: &str) -> Result<(), String> {
    if let (Some(requested), Some(ceiling)) = (requested.as_object(), ceiling.as_object()) {
        for (key, value) in requested {
            check_ceiling(
                value,
                ceiling
                    .get(key)
                    .ok_or_else(|| format!("Unknown limit {path}.{key}"))?,
                &format!("{path}.{key}"),
            )?;
        }
        return Ok(());
    }
    match (requested.as_u64(), ceiling.as_u64()) {
        (Some(requested), Some(ceiling)) if requested <= ceiling => Ok(()),
        _ => Err(format!("{path} exceeds the scalar host safety ceiling")),
    }
}

pub(crate) fn evaluate(request: &Value) -> Result<Value, String> {
    let fields = [
        "method",
        "bundle",
        "entry",
        "selected",
        "arguments",
        "verification_limits",
        "execution_limits",
        "conversions",
        "applied_conversions",
        "constructors",
        "expected_program",
    ];
    for key in request
        .as_object()
        .ok_or("Expected scalar request object")?
        .keys()
    {
        if !fields.contains(&key.as_str()) {
            return Err(format!("Unknown scalar request field {key}"));
        }
    }
    let bundle: DefinitionBundle = field(request, "bundle")?;
    let entry: SourceId = field(request, "entry")?;
    let selected: SymbolId = field(request, "selected")?;
    if serde_json::to_value(&selected).map_err(|e| e.to_string())? != request["selected"] {
        return Err("Selected scalar symbol must use canonical identity fields".into());
    }
    let arguments: Vec<ScalarValue> = field(request, "arguments")?;
    let verification: VerificationLimits = bounded_limits(request, "verification_limits")?;
    let execution: ExecutionLimits = bounded_limits(request, "execution_limits")?;
    let size = bundle
        .sources
        .iter()
        .try_fold(0usize, |total, source| total.checked_add(source.text.len()))
        .ok_or("Definition size budget exceeded")?;
    if size > verification.max_source_bytes
        || bundle
            .sources
            .iter()
            .any(|source| source.text.len() > 256_000)
    {
        return Ok(
            json!({"stage":"scalar_compile","ok":false,"source_backed":true,"diagnostics":[{"message":"Definition size budget exceeded"}]}),
        );
    }
    let compiled = if ["conversions", "applied_conversions", "constructors"]
        .iter()
        .any(|name| request.get(*name).is_some())
    {
        let contracts: Vec<ConversionContract> =
            serde_json::from_value(request.get("conversions").cloned().unwrap_or(json!([])))
                .map_err(|e| e.to_string())?;
        let applications: Vec<AppliedConversionContract> = serde_json::from_value(
            request
                .get("applied_conversions")
                .cloned()
                .unwrap_or(json!([])),
        )
        .map_err(|e| e.to_string())?;
        let constructors = crate::conversions::constructor_inputs(request, "constructors")?;
        intersect_model::conversion::with_catalog_policies(
            &bundle,
            &contracts,
            &applications,
            &constructors,
            |registry| {
                Ok(lowering::compile_scalar_with_conversions(
                    &bundle,
                    &entry,
                    &selected,
                    registry,
                    &verification,
                ))
            },
        )?
    } else {
        lowering::compile_scalar(&bundle, &entry, &selected, &verification)
    };
    let verified = match compiled {
        Ok(verified) => verified,
        Err(diagnostics) => {
            return Ok(
                json!({"stage":"scalar_compile","ok":false,"source_backed":true,"diagnostics":diagnostics}),
            );
        }
    };
    // Saved executable bytes never replace fresh source compilation. Comparing
    // canonical rebuilt IR makes changed bodies, revisions and forged replay fail.
    if let Some(expected) = request.get("expected_program") {
        let expected: Program = serde_json::from_value(expected.clone())
            .map_err(|e| format!("Invalid expected_program: {e}"))?;
        if &expected != verified.program() {
            return Ok(
                json!({"stage":"scalar_replay","ok":false,"source_backed":true,"diagnostics":[{"message":"Saved scalar IR differs from freshly compiled source"}]}),
            );
        }
    }
    match rcore_behavior_execution::evaluate(&verified, &arguments, &execution) {
        Ok(result) => Ok(
            json!({"stage":"scalar_execution","ok":true,"source_backed":true,"selected":selected,"program":verified.program(),"evaluation":result}),
        ),
        Err(error) => Ok(
            json!({"stage":"scalar_execution","ok":false,"source_backed":true,"selected":selected,"failure":error}),
        ),
    }
}
