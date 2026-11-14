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
impl ExecutionControl for Cancel { fn cancellation_requested(&self) -> bool { true } }
fn observe(case:&Value)->Result<Value,Box<dyn std::error::Error>> {
    let request=&case["request"];
    let bundle:DefinitionBundle=serde_json::from_value(request["bundle"].clone())?;
    let entry:SourceId=serde_json::from_value(request["entry"].clone())?;
    let selection:EntrySelection=serde_json::from_value(request["selection"].clone())?;
    let args:Vec<ValueWire>=serde_json::from_value(request["arguments"].clone())?;
    let refs:Vec<ReferenceContext>=serde_json::from_value(request["reference_contexts"].clone())?;
    let limits:ExecutionLimits=serde_json::from_value(request["execution_limits"].clone())?;
    let contracts:Vec<ConversionContract>=vec![];let applied:Vec<AppliedConversionContract>=vec![];
    let registry=intersect_model::conversion::build_catalog_policies(&bundle,&contracts,&applied,&[])?;
    let image=compile_function_with_conversions(&bundle,&entry,&selection,&registry,&Limits::default()).map_err(|e|format!("{e:?}"))?;
    assert_eq!(serde_json::to_value(image.program())?,request["expected_program"]);
    let create=||EvaluationSession::new_owned(image.clone(),args.clone(),bundle.clone(),refs.clone(),Some(registry.clone()),limits.clone());
    let mut session=create()?;let mut rows=vec![];let mut complete=None;
    for poll in 1..100000 {
        match session.resume(NonZeroUsize::MIN) {
            SessionProgress::Pending=>rows.push(json!({"poll":poll,"phase":session.retained_general_json_phase(),"arena":session.retained_arena_ownership()})),
            SessionProgress::Complete(result)=>{complete=Some(serde_json::to_value((*result)?)?);break},
            SessionProgress::Finished=>return Err("Missing completion".into()),
        }
    }
    assert_eq!(complete.ok_or("Missing terminal")?,case["complete"]["evaluation"]);
    assert!(matches!(session.resume(NonZeroUsize::MIN),SessionProgress::Finished));
    let measure=rows.iter().find(|r|r["phase"][0]=="decode"&&r["phase"][1]=="measure").ok_or("Missing measure phase")?;
    let copies:Vec<_>=rows.iter().filter(|r|r["phase"][0]=="decode"&&r["phase"][1]=="string-copy"&&r["phase"][2][7].as_u64().is_some_and(|n|n>0&&n<r["phase"][2][6].as_u64().unwrap())).collect();
    let middle=copies.iter().find(|r|r["phase"][2][7].as_u64().unwrap()>=case["literal_bytes"].as_u64().unwrap()/2).ok_or("Missing middle copy")?;
    let late=copies.last().ok_or("Missing late copy")?;
    let mut checkpoints=serde_json::Map::new();
    for (name,row)in[("measure",measure),("copy_middle",*middle),("copy_late",*late)] {
        let poll=row["poll"].as_u64().unwrap();let mut s=create()?;
        for _ in 0..poll{assert!(matches!(s.resume(NonZeroUsize::MIN),SessionProgress::Pending));}
        assert_eq!(serde_json::to_value(s.retained_general_json_phase())?,row["phase"]);
        let arena=s.retained_arena_ownership();assert_eq!(&arena[1..3],&[0,0]);
        let SessionProgress::Complete(result)=s.resume_with_control(NonZeroUsize::MIN,&Cancel)else{return Err("Missing cancellation".into())};
        let error=(*result).unwrap_err();assert_eq!(format!("{:?}",error.kind),"Cancelled");
        assert_eq!(error.consumed.value_bytes,if name=="measure"{576}else{case["decoder_admission"].as_u64().unwrap()as usize});
        assert_eq!(error.consumed.value_nodes,0);assert!(matches!(s.resume(NonZeroUsize::MIN),SessionProgress::Finished));
        checkpoints.insert(name.into(),json!({"poll":poll,"phase":row["phase"],"arena":arena,"failure":error}));
    }
    Ok(json!({"case":case["id"],"quantum":1,"original_request":request,"expected_evaluation":case["complete"]["evaluation"],"pending_rows":rows,"checkpoints":checkpoints}))
}
fn main()->Result<(),Box<dyn std::error::Error>>{
    let path=std::env::args().nth(1).ok_or("Missing original plan")?;
    let plan:Value=serde_json::from_slice(&std::fs::read(path)?)?;
    let cases:Vec<_>=plan["cases"].as_array().ok_or("Missing cases")?.iter().map(observe).collect::<Result<_,_>>()?;
    assert_eq!(cases.len(),2);println!("{}",json!({"version":2,"cases":cases}));Ok(())
}
