use intersect_model::definitions::{
    bundle::{DefinitionBundle, SourceId},
    conversion::{ConversionContract, AppliedConversionContract, ReferenceContext},
    rcore_behavior::{function_lowering::compile_function_with_conversions,
        ir_v2::{EntrySelection, Limits, ValueWire}},
};
use intersect_model::rcore_behavior_functions::{EvaluationSession, ExecutionControl, ExecutionLimits, SessionProgress};
use serde_json::{Value, json};
use std::num::NonZeroUsize;

struct Cancel;
impl ExecutionControl for Cancel {
    fn cancellation_requested(&self) -> bool { true }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Missing original plan")?;
    let plan: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let case = &plan["cases"][0];
    assert_eq!(case["id"], "ascii240000");
    let request = &case["request"];
    let bundle: DefinitionBundle = serde_json::from_value(request["bundle"].clone())?;
    let entry: SourceId = serde_json::from_value(request["entry"].clone())?;
    let selection: EntrySelection = serde_json::from_value(request["selection"].clone())?;
    let args: Vec<ValueWire> = serde_json::from_value(request["arguments"].clone())?;
    let refs: Vec<ReferenceContext> = serde_json::from_value(request["reference_contexts"].clone())?;
    let limits: ExecutionLimits = serde_json::from_value(request["execution_limits"].clone())?;
    let contracts: Vec<ConversionContract> = vec![];
    let applied: Vec<AppliedConversionContract> = vec![];
    let registry = intersect_model::conversion::build_catalog_policies(&bundle, &contracts, &applied, &[])?;
    let image = compile_function_with_conversions(&bundle, &entry, &selection, &registry, &Limits::default())
        .map_err(|e| format!("Fresh original compilation failed: {e:?}"))?;
    assert_eq!(serde_json::to_value(image.program())?, request["expected_program"]);
    let create = || EvaluationSession::new_owned(image.clone(), args.clone(), bundle.clone(), refs.clone(), Some(registry.clone()), limits.clone());
    let q = NonZeroUsize::new(256).ok_or("Quantum")?;
    let mut session = create()?;
    let mut rows = Vec::new();
    let mut copy_poll = None;
    let mut complete = None;
    for poll in 1..100000 {
        match session.resume(q) {
            SessionProgress::Pending => {
                let phase = session.retained_general_json_phase();
                let arena = session.retained_arena_ownership();
                if let Some(("decode", "string-copy", n)) = phase {
                    if n[7] > 0 && n[7] < n[6] && copy_poll.is_none() {
                        assert_eq!(n[3], 240000);
                        assert_eq!(n[6], 240000);
                        assert_eq!(arena[0], case["decoder_admission"].as_u64().unwrap() as usize);
                        assert_eq!(&arena[1..3], &[0, 0]);
                        copy_poll = Some(poll);
                    }
                }
                rows.push(json!({"poll":poll,"phase":phase,"arena":arena}));
            }
            SessionProgress::Complete(result) => { complete = Some(serde_json::to_value((*result)?)?); break; }
            SessionProgress::Finished => return Err("No terminal result".into()),
        }
    }
    assert_eq!(complete.ok_or("No completion")?, case["complete"]["evaluation"]);
    assert!(matches!(session.resume(q), SessionProgress::Finished));
    let copy_poll = copy_poll.ok_or("No partial input String copy")?;
    let mut cancelled = Vec::new();
    for poll in [3, copy_poll] {
        let mut s = create()?;
        for _ in 0..poll { assert!(matches!(s.resume(q), SessionProgress::Pending)); }
        let phase = s.retained_general_json_phase();
        let arena = s.retained_arena_ownership();
        let SessionProgress::Complete(result) = s.resume_with_control(q, &Cancel) else { return Err("Missing cancellation result".into()); };
        let error = (*result).unwrap_err();
        assert_eq!(format!("{:?}", error.kind), "Cancelled");
        assert_eq!(error.consumed.value_bytes, if poll == 3 {576} else {case["decoder_admission"].as_u64().unwrap() as usize});
        assert_eq!(error.consumed.value_nodes, 0);
        assert!(matches!(s.resume(q), SessionProgress::Finished));
        cancelled.push(json!({"poll":poll,"phase":phase,"arena":arena,"failure":error}));
    }
    println!("{}", json!({"version":1,"quantum":256,"original_request":request,"expected_evaluation":case["complete"]["evaluation"],"copy_poll":copy_poll,"pending_rows":rows,"cancellations":cancelled,"post_terminal":"Finished"}));
    Ok(())
}
