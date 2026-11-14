//! Source-rebuilt callable execution. Hosts never accept a supplied executable proof.
use intersect_model::definitions::{
    bundle::{DefinitionBundle, SourceId},
    conversion::{AppliedConversionContract, ConversionContract, ReferenceContext},
    rcore_behavior::{
        function_lowering,
        ir_v2::{EntrySelection, Limits, Program, ValueWire},
    },
};
use intersect_model::rcore_behavior_functions::{self, ExecutionLimits};
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
fn limits<T: DeserializeOwned + Serialize + Default>(
    request: &Value,
    name: &str,
) -> Result<T, String> {
    let Some(value) = request.get(name) else {
        return Ok(T::default());
    };
    let ceiling = serde_json::to_value(T::default()).map_err(|error| error.to_string())?;
    fn check(value: &Value, ceiling: &Value, path: &str) -> Result<(), String> {
        if let (Some(value), Some(ceiling)) = (value.as_object(), ceiling.as_object()) {
            for (key, value) in value {
                check(
                    value,
                    ceiling
                        .get(key)
                        .ok_or_else(|| format!("Unknown limit {path}.{key}"))?,
                    &format!("{path}.{key}"),
                )?;
            }
            return Ok(());
        }
        if path == "execution_limits.observation.enabled"
            && value.is_boolean()
            && ceiling.is_boolean()
        {
            return Ok(());
        }
        match (value.as_u64(), ceiling.as_u64()) {
            (Some(value), Some(ceiling)) if value <= ceiling => Ok(()),
            _ => Err(format!("{path} exceeds the function host safety ceiling")),
        }
    }
    // Apply ceilings before pointer-width-dependent deserialization for WASM parity.
    check(value, &ceiling, name)?;
    serde_json::from_value(value.clone()).map_err(|error| format!("Invalid {name}: {error}"))
}

pub(crate) struct Prepared {
    pub compile_only: bool,
    pub verification: Limits,
    pub image: intersect_model::definitions::rcore_behavior::ir_v2::VerifiedExecutable,
    pub bundle: DefinitionBundle,
    pub arguments: Vec<ValueWire>,
    pub references: Vec<ReferenceContext>,
    pub registry: intersect_model::definitions::conversion::ConversionRegistry<'static>,
    pub execution: ExecutionLimits,
}
pub(crate) fn outcome(
    program: Value,
    result: Result<rcore_behavior_functions::Evaluation, rcore_behavior_functions::ExecutionError>,
) -> Value {
    match result {
        Ok(evaluation) => {
            json!({"stage":"function_execution","ok":true,"source_backed":true,"program":program,"evaluation":evaluation})
        }
        Err(failure) => {
            json!({"stage":"function_execution","ok":false,"source_backed":true,"program":program,"failure":failure})
        }
    }
}
pub(crate) fn handle(request: &Value) -> Result<Value, String> {
    with_prepared(request, |ready| {
        if ready.compile_only {
            return Ok(
                json!({"stage":"function_compile","ok":true,"source_backed":true,"program":ready.image.program(),"effects":ready.image.effects()}),
            );
        }
        let program = serde_json::to_value(ready.image.program()).map_err(|e| e.to_string())?;
        // Both synchronous native/WASM requests and worker jobs own these inputs.
        // Use the same owning session so actual movement/allocation receipts agree.
        let result = match rcore_behavior_functions::EvaluationSession::new_owned(
            ready.image,
            ready.arguments,
            ready.bundle,
            ready.references,
            Some(ready.registry),
            ready.execution,
        ) {
            Err(error) => Err(error),
            Ok(mut session) => loop {
                match session.resume(std::num::NonZeroUsize::MAX) {
                    rcore_behavior_functions::SessionProgress::Pending => {}
                    rcore_behavior_functions::SessionProgress::Complete(result) => break *result,
                    rcore_behavior_functions::SessionProgress::Finished => {
                        return Err("Owned function session finished without an outcome".into());
                    }
                }
            },
        };
        Ok(outcome(program, result))
    })
}
pub(crate) fn with_prepared(
    request: &Value,
    ready: impl FnOnce(Prepared) -> Result<Value, String>,
) -> Result<Value, String> {
    let compile_only = request["method"] == "rcore_behavior/compileFunction";
    let mut fields = vec![
        "method",
        "bundle",
        "entry",
        "selection",
        "verification_limits",
        "conversions",
        "applied_conversions",
        "constructors",
        "expected_program",
    ];
    if !compile_only {
        fields.extend(["arguments", "execution_limits", "reference_contexts"]);
    }
    for key in request
        .as_object()
        .ok_or("Expected function request object")?
        .keys()
    {
        if !fields.contains(&key.as_str()) {
            return Err(format!("Unknown function request field {key}"));
        }
    }
    let bundle: DefinitionBundle = field(request, "bundle")?;
    let entry: SourceId = field(request, "entry")?;
    let selection: EntrySelection = field(request, "selection")?;
    if serde_json::to_value(&selection).map_err(|error| error.to_string())? != request["selection"]
    {
        return Err("Function selection requires exact canonical identity fields".into());
    }
    let verification: Limits = limits(request, "verification_limits")?;
    let execution: ExecutionLimits = limits(request, "execution_limits")?;
    if bundle
        .sources
        .iter()
        .any(|source| source.text.len() > 256_000)
    {
        return Ok(
            json!({"stage":"function_compile","ok":false,"source_backed":true,
            "diagnostics":[{"message":"Definition size budget exceeded"}]}),
        );
    }
    let contracts: Vec<ConversionContract> =
        serde_json::from_value(request.get("conversions").cloned().unwrap_or(json!([])))
            .map_err(|error| error.to_string())?;
    let applied: Vec<AppliedConversionContract> = serde_json::from_value(
        request
            .get("applied_conversions")
            .cloned()
            .unwrap_or(json!([])),
    )
    .map_err(|error| error.to_string())?;
    let constructors = crate::conversions::constructor_inputs(request, "constructors")?;
    let arguments: Vec<ValueWire> = if compile_only {
        vec![]
    } else {
        field(request, "arguments")?
    };
    if !compile_only
        && serde_json::to_value(&arguments).map_err(|error| error.to_string())?
            != request["arguments"]
    {
        return Err("Function arguments require exact canonical identity fields".into());
    }
    let references: Vec<ReferenceContext> = serde_json::from_value(
        request
            .get("reference_contexts")
            .cloned()
            .unwrap_or(json!([])),
    )
    .map_err(|error| error.to_string())?;
    let registry = intersect_model::conversion::build_catalog_policies(
        &bundle,
        &contracts,
        &applied,
        &constructors,
    )?;
    let image = match function_lowering::compile_function_with_conversions(
        &bundle,
        &entry,
        &selection,
        &registry,
        &verification,
    ) {
        Ok(image) => image,
        Err(diagnostics) => {
            return Ok(
                json!({"stage":"function_compile","ok":false,"source_backed":true,"diagnostics":diagnostics}),
            );
        }
    };
    if let Some(expected) = request.get("expected_program") {
        let retained: Program = serde_json::from_value(expected.clone())
            .map_err(|error| format!("Invalid expected_program: {error}"))?;
        if serde_json::to_value(&retained).map_err(|error| error.to_string())? != *expected
            || &retained != image.program()
        {
            return Ok(
                json!({"stage":"function_replay","ok":false,"source_backed":true,
            "diagnostics":[{"message":"Saved function IR differs from freshly compiled source"}]}),
            );
        }
    }
    ready(Prepared {
        compile_only,
        verification,
        image,
        bundle,
        arguments,
        references,
        registry,
        execution,
    })
}
