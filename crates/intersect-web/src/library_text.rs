//! Retained demonstration host. Language policy is in intersect-library;
//! prepared edits/checkpoints are native capabilities, never JSON authority.
use intersect_text::{
    EvaluationLimits, Source,
    source_edit::{PreparedSourceEdit, SourceEditCheckpoint, SourceEditSession},
};
use serde_json::{Value, json};
use std::cell::RefCell;
thread_local! { static STATE: RefCell<State> = const { RefCell::new(State { serial: 0, host: None }) }; }
struct State {
    serial: u64,
    host: Option<Host>,
}
struct Host {
    id: u64,
    general: bool,
    revision: u64,
    serial: u64,
    session: SourceEditSession,
    dirty: bool,
    pending: Option<Pending>,
    undo: Vec<SourceEditCheckpoint>,
    redo: Vec<SourceEditCheckpoint>,
}
struct Pending {
    token: u64,
    revision: u64,
    mode: String,
    edit: PreparedSourceEdit,
}

pub(super) fn dispatch(method: &str, request: &Value) -> Result<Value, String> {
    let general = method.starts_with("modelText/");
    let normalized;
    let method = if let Some(operation) = method.strip_prefix("modelText/") {
        normalized = format!("library/{operation}");
        normalized.as_str()
    } else {
        method
    };
    if serde_json::to_vec(request)
        .map_err(|e| e.to_string())?
        .len()
        > 1_000_000
    {
        return Err("Library request byte limit exceeded".into());
    }
    STATE.with(|state| {
        let mut state = state.try_borrow_mut().map_err(|_| "Reentrant Library request")?;
        if matches!(method, "library/start" | "library/load") {
            if method == "library/load" { check(state.host.as_ref().ok_or("Library not started")?, request)?; }
            if let Some(host)=state.host.as_mut() { advance(host)?;host.pending=None;host.dirty=true; }
            let language = if general {
                crate::model_text::language(request, if method == "library/load" { state.host.as_ref().map(|host| &host.session) } else { None })?
            } else { crate::model_text::PreparedLanguage { language: std::sync::Arc::new(intersect_library::language()?), services: None } };
            let sources: Vec<Source> = if let Some(sources) = request.get("sources") { serde_json::from_value(sources.clone()).map_err(|e|e.to_string())? } else if general { return Err("Missing source resources".into()); } else { intersect_library::sources() };
            let session = match language.services {
                Some(services) => SourceEditSession::new_with_services(language.language, &sources, EvaluationLimits::workbench(), services)?,
                None => SourceEditSession::new_shared(language.language, &sources, EvaluationLimits::workbench())?,
            };
            let id = state.serial.checked_add(1).ok_or("Library session counter exhausted")?;
            state.serial = id;
            state.host = Some(Host { id, general, revision:0, serial:0, session, dirty:false, pending:None, undo:vec![], redo:vec![] });
            return observation(state.host.as_ref().ok_or("Missing started Library")?);
        }
        let host = state.host.as_mut().ok_or("Library not started")?;
        if host.general != general { return Err("Model/text host language kind changed".into()); }
        if method == "library/inspect" { return observation(host); }
        check(host,request)?;
        match method {
            "library/invalidate" => { advance(host)?; host.pending=None;host.dirty=true;observation(host) }
            "library/discard" => { advance(host)?;host.pending=None;host.dirty=false;observation(host) }
            "library/prepare" => {
                if host.dirty { return Err("Apply or discard source edits before editing the model".into()); }
                host.pending=None;
                let mut action=request.get("action").cloned().ok_or("Missing Library action")?;
                let mode=action["kind"].as_str().ok_or("Missing action kind")?.to_owned();
                let serial=host.serial.checked_add(1).ok_or("Library proposal counter exhausted")?;
                let edit=match mode.as_str() {
                    "undo" => host.session.plan_restore(host.undo.last().ok_or("No undo history")?)?,
                    "redo" => host.session.plan_restore(host.redo.last().ok_or("No redo history")?)?,
                    _ if general => crate::model_text::plan(&host.session, action, request.get("limits"))?,
                    _ => {
                        if mode=="add_loan" && action.get("id").is_none() { action["id"]=json!(format!("library:{}:loan-{serial}",host.id)); }
                        intersect_library::plan(&host.session,serde_json::from_value(action).map_err(|e|format!("Invalid Library action: {e}"))?)?
                    }
                };
                let result=json!({"context":context(host),"token":serial.to_string(),"sources":edit.sources(),"patches":edit.patches(),"evaluation":edit.analysis().valid_model});
                host.serial=serial; host.pending=Some(Pending{token:serial,revision:host.revision,mode,edit});Ok(result)
            }
            "library/serialize" if general => {
                if host.dirty { return Err("Apply or discard source edits before generation".into()); }
                let language=crate::model_text::language(&json!({}),Some(&host.session))?;
                let serializer=intersect_text::serialization::CanonicalSerializer::new_shared(language.language,EvaluationLimits::workbench(),Default::default());
                let result=match language.services { Some(services)=>serializer.serialize_with_native_services(host.session.model(),&services)?, None=>serializer.serialize(host.session.model())? };
                Ok(json!({"context":context(host),"sources":result.sources,"identities":result.identities,"evaluation":result.analysis.valid_model}))
            }
            "library/commit" => {
                let token=request["token"].as_str().ok_or("Missing proposal token")?.parse::<u64>().map_err(|_|"Invalid proposal token")?;
                let pending=host.pending.as_ref().ok_or("Stale Library proposal")?;
                if pending.token!=token || pending.revision!=host.revision || host.dirty { return Err("Stale Library proposal".into()); }
                let revision=host.revision.checked_add(1).ok_or("Library revision exhausted")?;
                let before=host.session.checkpoint();
                let pending=host.pending.take().ok_or("Missing proposal")?;
                host.session.adopt(pending.edit)?;
                if pending.mode=="undo" { host.undo.pop();host.redo.push(before); }
                else if pending.mode=="redo" { host.redo.pop();host.undo.push(before); }
                else {host.undo.push(before);host.redo.clear();}
                if host.undo.len()>20 {host.undo.remove(0);}
                if host.redo.len()>20 {host.redo.remove(0);}
                host.revision=revision;observation(host)
            }
            _ => Err(format!("Unknown Library method {method}")),
        }
    })
}
fn advance(host: &mut Host) -> Result<(), String> {
    host.revision = host
        .revision
        .checked_add(1)
        .ok_or("Library revision exhausted")?;
    Ok(())
}
fn context(host: &Host) -> Value {
    json!({"session":host.id.to_string(),"revision":host.revision.to_string()})
}
fn check(host: &Host, request: &Value) -> Result<(), String> {
    if request.get("context") != Some(&context(host)) {
        return Err("Stale Library context: session or revision changed".into());
    }
    Ok(())
}
fn observation(host: &Host) -> Result<Value, String> {
    Ok(
        json!({"context":context(host),"dirty":host.dirty,"sources":host.session.sources(),"evaluation":host.session.evaluation(),"undo":host.undo.len(),"redo":host.redo.len(),"schema":if host.general {Some(&host.session.package().schema)} else {None}}),
    )
}
