// Verification-only process adapter. RPC mode owns ONE host for its whole life.
// Negative native dispatch results use the exact public ABI error envelope.
use intersect_model::{
    definitions::{
        bundle::DefinitionBundle,
        rcore_behavior::{
            function_lowering::compile_function_with_conversions,
            ir_v2::{Limits, ValueWire},
        },
    },
    rcore_behavior_functions::{
        EvaluationSession, ExecutionControl, ExecutionLimits, SessionProgress,
    },
};
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Read, Write},
    num::NonZeroUsize,
};

struct Control(bool);
impl ExecutionControl for Control {
    fn cancellation_requested(&self) -> bool {
        self.0
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("layouts") {
        println!(
            "{}",
            json!({
                "roots": EvaluationSession::retained_root_layouts(),
                "bindings": EvaluationSession::retained_binding_root_layouts(),
                "phases": EvaluationSession::retained_phase_layouts(),
                "numeric": intersect_model::rcore_behavior_functions::numeric_validation_candidate_layouts(),
            })
        );
        return Ok(());
    }

    if std::env::args().nth(1).as_deref() == Some("rpc") {
        let mut host = intersect_web::function_jobs::FunctionJobHost::default();
        let stdin = io::stdin();
        let mut stdout = io::BufWriter::new(io::stdout().lock());
        for request in stdin.lock().lines() {
            let request = request?;
            let response = match host.dispatch(&request) {
                Ok(value) => value,
                Err(error) => json!({"error":error}),
            };
            serde_json::to_writer(&mut stdout, &response)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
        return Ok(());
    }
    // This owned typed phase witness mirrors FunctionJobs::start's SomeRegistry
    // route. It must be coupled to actual host polling before worker claims.
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let input: Value = serde_json::from_str(&input)?;
    let request = &input["request"];
    let bundle: DefinitionBundle = serde_json::from_value(request["bundle"].clone())?;
    let entry = serde_json::from_value(request["entry"].clone())?;
    let selection = serde_json::from_value(request["selection"].clone())?;
    for field in [
        "verification_limits",
        "execution_limits",
        "conversions",
        "applied_conversions",
        "constructors",
    ] {
        assert!(
            request.get(field).is_none(),
            "This exact supplement covers only the frozen absent-policy request route"
        );
    }
    let registry = intersect_model::conversion::build_catalog_policies(&bundle, &[], &[], &[])?;
    let image = compile_function_with_conversions(
        &bundle,
        &entry,
        &selection,
        &registry,
        &Limits::default(),
    )
    .map_err(|errors| format!("{errors:?}"))?;
    assert_eq!(
        serde_json::to_value(image.program())?,
        request["expected_program"]
    );
    let arguments: Vec<ValueWire> = serde_json::from_value(request["arguments"].clone())?;
    let quantum = NonZeroUsize::new(input["quantum"].as_u64().ok_or("Missing quantum")? as usize)
        .ok_or("Zero quantum")?;
    let references = serde_json::from_value(request["reference_contexts"].clone())?;
    let mut session = EvaluationSession::new_owned(
        image,
        arguments,
        bundle,
        references,
        Some(registry),
        ExecutionLimits::default(),
    )?;
    let mut rows = Vec::new();
    for count in 1..100000 {
        let progress = session.resume_with_control(quantum, &Control(false));
        let phase = session.retained_numeric_phase();
        let arena = session.retained_arena_ownership();
        let complete = matches!(progress, SessionProgress::Complete(_));
        let value = match progress {
            SessionProgress::Pending => json!({"pending":true}),
            SessionProgress::Complete(result) => json!({"complete":result}),
            SessionProgress::Finished => return Err("Missing complete outcome".into()),
        };
        rows.push(json!({"resumes":count,"progress":value,"numeric_phase":phase,"arena":arena}));
        if complete {
            assert!(matches!(
                session.resume_with_control(NonZeroUsize::MIN, &Control(false)),
                SessionProgress::Finished
            ));
            println!("{}", serde_json::to_string(&rows)?);
            return Ok(());
        }
    }
    Err("Finite phase witness deadline exceeded".into())
}
