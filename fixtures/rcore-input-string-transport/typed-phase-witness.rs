#[path = "native/src/driver.rs"]
mod driver;
use intersect_model::{
    definitions::{
        bundle::DefinitionBundle,
        rcore_behavior::ir_v2::{ValueWire, VerifiedExecutable},
    },
    rcore_behavior_functions::{
        EvaluationSession, ExecutionContext, ExecutionControl, ExecutionLimits, SessionProgress,
    },
};
use serde_json::{Value, json};
use std::num::NonZeroUsize;
struct Cancel;
impl ExecutionControl for Cancel {
    fn cancellation_requested(&self) -> bool {
        true
    }
}
fn create<'a>(
    owned: bool,
    image: &'a VerifiedExecutable,
    args: &'a [ValueWire],
    bundle: &'a DefinitionBundle,
    context: &ExecutionContext<'a>,
    limits: &'a ExecutionLimits,
) -> EvaluationSession<'a> {
    if owned {
        EvaluationSession::new_owned(
            image.clone(),
            args.to_vec(),
            bundle.clone(),
            vec![],
            None,
            limits.clone(),
        )
    } else {
        EvaluationSession::new(image, args, context, limits)
    }
    .unwrap()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Missing original fixture root")?,
    );
    let requests: Vec<Value> = std::fs::read_to_string(root.join("requests.jsonl"))?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let before: Vec<Value> =
        serde_json::from_slice(&std::fs::read(root.join("original-terminals.json"))?)?;
    let accepted: Vec<Value> = std::fs::read_to_string(
        std::env::args()
            .nth(2)
            .ok_or("Missing pinned retained record")?,
    )?
    .lines()
    .map(serde_json::from_str)
    .collect::<Result<_, _>>()?;
    let plan: Value = serde_json::from_slice(&std::fs::read(root.join("exact-plan.json"))?)?;
    assert_eq!(requests.len(), 282);
    assert_eq!(before.len(), 282);
    assert_eq!(accepted.len(), 282);
    for (request, record) in requests.iter().zip(&accepted) {
        assert_eq!(request["case"], record["case"]);
    }
    let mut source_records = vec![];
    let mut originals = vec![];
    let mut copies = vec![];
    for id in ["ascii4096", "ascii131072", "escaped-unicode", "ascii240000"] {
        for owned in [false, true] {
            let baseline = requests
                .iter()
                .position(|r| {
                    r["id"] == id
                        && r["owned"] == owned
                        && r["quantum"] == 1
                        && ["cancel", "drop_at", "limit", "per_value", "nodes", "depth"]
                            .iter()
                            .all(|k| r[*k].is_null())
                })
                .ok_or("Missing original unbounded q1 baseline")?;
            let (image, args, bundle, references) = driver::fixture(id)?;
            assert!(references.is_empty());
            let context = ExecutionContext::new(&bundle);
            let limits = ExecutionLimits::default();
            let mut session = create(owned, &image, &args, &bundle, &context, &limits);
            let mut rows = vec![];
            let mut complete = None;
            for poll in 1..100_000 {
                match session.resume(NonZeroUsize::MIN){
   SessionProgress::Pending=>rows.push(json!({"poll":poll,"phase":session.retained_general_json_phase(),"arena":session.retained_arena_ownership()})),
   SessionProgress::Complete(result)=>{complete=Some(serde_json::to_value(result)?);break},
   SessionProgress::Finished=>return Err("Missing terminal".into()),
  }
            }
            assert_eq!(
                Value::Array(vec![complete.ok_or("Missing completion")?]),
                before[baseline]["complete"]
            );
            assert!(matches!(
                session.resume(NonZeroUsize::MIN),
                SessionProgress::Finished
            ));
            let mut input_copies: Vec<_> = rows
                .iter()
                .filter(|r| {
                    r["phase"][0] == "decode"
                        && r["phase"][1] == "string-copy"
                        && r["phase"][2][7].as_u64().is_some_and(|offset| {
                            offset > 0 && offset < r["phase"][2][6].as_u64().unwrap()
                        })
                })
                .collect();
            input_copies.dedup_by_key(|r| r["phase"][2][7].as_u64().unwrap());
            assert!(!input_copies.is_empty());
            assert!(
                input_copies
                    .windows(2)
                    .all(|w| w[0]["phase"][2][7].as_u64().unwrap()
                        < w[1]["phase"][2][7].as_u64().unwrap())
            );
            let admission = plan["formulas"]
                .as_array()
                .ok_or("Missing formulas")?
                .iter()
                .find(|f| f["id"] == id)
                .ok_or("Missing original admission")?["decoder_admission"]
                .as_u64()
                .ok_or("Missing admission")?;
            for request in requests.iter().filter(|r| {
                r["id"] == id
                    && r["owned"] == owned
                    && (!r["cancel"].is_null() || !r["drop_at"].is_null())
            }) {
                let action = if request["cancel"].is_null() {
                    "drop_at"
                } else {
                    "cancel"
                };
                let prefix = request[action].as_u64().ok_or("Missing original prefix")? as usize;
                let mut s = create(owned, &image, &args, &bundle, &context, &limits);
                for _ in 0..prefix {
                    assert!(matches!(
                        s.resume(NonZeroUsize::MIN),
                        SessionProgress::Pending
                    ));
                }
                let original_phase = serde_json::to_value(s.retained_general_json_phase())?;
                let original_arena = s.retained_arena_ownership();
                assert_eq!(original_phase, rows[prefix - 1]["phase"]);
                let observation = driver::run(serde_json::from_value(request.clone())?)?;
                if action == "cancel" {
                    let SessionProgress::Complete(result) =
                        s.resume_with_control(NonZeroUsize::MIN, &Cancel)
                    else {
                        return Err("Missing original cancellation".into());
                    };
                    assert_eq!(
                        serde_json::to_value(result)?,
                        observation["events"].as_array().unwrap().last().unwrap()["complete"]
                    );
                    assert!(matches!(
                        s.resume(NonZeroUsize::MIN),
                        SessionProgress::Finished
                    ));
                }
                drop(s);
                originals.push(json!({"request":request,"phase":original_phase,"arena":original_arena,"observation":observation}));
                let ordinal: usize = request["case"]
                    .as_str()
                    .ok_or("Missing case")?
                    .split("/input-copy-")
                    .nth(1)
                    .ok_or("Missing original copy ordinal")?
                    .split('/')
                    .next()
                    .unwrap()
                    .parse()?;
                let row = *input_copies
                    .get(ordinal - 1)
                    .ok_or("Original copy ordinal absent from retained copy phase")?;
                let poll = row["poll"].as_u64().unwrap() as usize;
                let mut adjusted = request.clone();
                adjusted[action] = json!(poll);
                let mut s = create(owned, &image, &args, &bundle, &context, &limits);
                for _ in 0..poll {
                    assert!(matches!(
                        s.resume(NonZeroUsize::MIN),
                        SessionProgress::Pending
                    ));
                }
                assert_eq!(
                    serde_json::to_value(s.retained_general_json_phase())?,
                    row["phase"]
                );
                let arena = s.retained_arena_ownership();
                assert_eq!(&arena[1..3], &[0, 0]);
                let observation = driver::run(serde_json::from_value(adjusted.clone())?)?;
                let original = requests
                    .iter()
                    .position(|r| r["case"] == request["case"])
                    .unwrap();
                let accepted_complete: Vec<_> = accepted[original]["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|e| e.get("complete").cloned())
                    .collect();
                if action == "cancel" {
                    let SessionProgress::Complete(result) =
                        s.resume_with_control(NonZeroUsize::MIN, &Cancel)
                    else {
                        return Err("Missing adjusted cancellation".into());
                    };
                    let result = serde_json::to_value(result)?;
                    assert_eq!(result["Err"]["consumed"]["value_bytes"], admission);
                    assert_eq!(result["Err"]["consumed"]["value_nodes"], 0);
                    assert_eq!(
                        result,
                        observation["events"].as_array().unwrap().last().unwrap()["complete"]
                    );
                    assert_eq!(
                        Value::Array(vec![result]),
                        Value::Array(accepted_complete.clone())
                    );
                    assert!(matches!(
                        s.resume(NonZeroUsize::MIN),
                        SessionProgress::Finished
                    ));
                }
                drop(s);
                copies.push(json!({"original_request":request,"request":adjusted,"copy_ordinal":ordinal,"phase":row["phase"],"arena":arena,"observation":observation,"accepted_complete":accepted_complete}));
            }
            source_records.push(json!({"id":id,"owned":owned,"original_baseline_request":requests[baseline],"original_baseline_terminal":before[baseline],"source_ids":bundle.sources.iter().map(|s|&s.id).collect::<Vec<_>>(),"program":image.program(),"pending_rows":rows}));
        }
    }
    assert_eq!(source_records.len(), 8);
    assert_eq!(originals.len(), 48);
    assert_eq!(copies.len(), 48);
    println!(
        "{}",
        json!({"version":1,"source_records":source_records,"original_controls":originals,"copy_controls":copies,"accepted_retained_record_sha256":"cd6b87588d1dc8e76765977951860d40d18c6155f9b1d6764c9a203e225220fd","scope":"Exact original282request corpus and pre-retained historical terminals remain unchanged. Original48fixed schedule controls recorded with current source phases; additional48controls change only cancellation/drop poll to reach the same original1/middle/late copy ordinal. Every completed baseline equals its original expectation; every adjusted copy cancellation whole terminal equals the independently pinned accepted retained-String recording, including zero published nodes. Historical pre-retained cancellation with one published node remains separate. No current error/fee field is removed or normalized; no runtime changes."})
    );
    Ok(())
}
