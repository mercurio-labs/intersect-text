//! Explicit host resource ownership; no job registry exists in semantic libraries.
use crate::rcore_behavior_functions::{outcome, with_prepared};
use intersect_model::rcore_behavior_functions::{
    EvaluationSession, ExecutionControl, SessionProgress,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, num::NonZeroUsize};
const MAX_JOBS: usize = 4;
const MAX_RETAINED: usize = 64 * 1024 * 1024;
const MAX_PROGRAM: usize = 16 * 1024 * 1024;
struct Job {
    session: EvaluationSession<'static>,
    program: Value,
    bytes: usize,
}
/// One instance belongs to one host generation. The host supplies a fresh unique
/// namespace (for browsers, crypto.randomUUID); IDs are not portable capabilities.
pub struct FunctionJobs {
    namespace: String,
    next: u64,
    jobs: BTreeMap<String, Job>,
    retained: usize,
    retained_limit: usize,
}
impl FunctionJobs {
    pub fn new(namespace: String) -> Result<Self, String> {
        Self::with_retained_limit(namespace, MAX_RETAINED)
    }
    /// Lower-only host policy for encoded input, image, response and registry metadata.
    /// This does not measure allocator overhead or provider implementation memory.
    pub fn with_retained_limit(namespace: String, retained_limit: usize) -> Result<Self, String> {
        if retained_limit > MAX_RETAINED {
            return Err("Retained job byte limit exceeds host safety ceiling".into());
        }
        if namespace.len() < 8
            || namespace.len() > 128
            || !namespace
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(
                "Job host namespace must be 8..128 ASCII letters, digits or hyphens".into(),
            );
        }
        Ok(Self {
            namespace,
            next: 0,
            jobs: BTreeMap::new(),
            retained: 0,
            retained_limit,
        })
    }
    pub fn active_jobs(&self) -> usize {
        self.jobs.len()
    }
    pub fn retained_encoded_bytes(&self) -> usize {
        self.retained
    }
    pub fn dispatch(&mut self, input: &str) -> Result<Value, String> {
        if input.len() > 4_000_000 {
            return Err("Function job request byte budget exceeded".into());
        }
        let request: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
        let method = request["method"].as_str().ok_or("Missing job method")?;
        if method == "rcore_behavior/startFunction" {
            return self.start(&request, input.len());
        }
        let object = request.as_object().ok_or("Expected job request object")?;
        for key in object.keys() {
            if !(["method", "job"].contains(&key.as_str())
                || method == "rcore_behavior/resumeFunction" && key == "quantum")
            {
                return Err(format!("Unknown job field {key}"));
            }
        }
        let id = request["job"].as_str().ok_or("Missing job identity")?;
        match method {
            "rcore_behavior/resumeFunction" => {
                let quantum = request
                    .get("quantum")
                    .map_or(Ok(256), |v| v.as_u64().ok_or("Invalid scheduling quantum"))?;
                if quantum == 0 || quantum > 4096 {
                    return Err("Scheduling quantum must be 1..4096".into());
                }
                let quantum =
                    NonZeroUsize::new(quantum as usize).ok_or("Invalid scheduling quantum")?;
                self.advance(id, quantum, false)
            }
            "rcore_behavior/cancelFunction" => self.advance(id, NonZeroUsize::MIN, true),
            "rcore_behavior/dropFunction" => {
                self.remove(id)?;
                Ok(json!({"dropped":true}))
            }
            _ => Err("Unknown function job method".into()),
        }
    }
    fn start(&mut self, request: &Value, request_bytes: usize) -> Result<Value, String> {
        if self.jobs.len() >= MAX_JOBS {
            return Err("Active function job budget exceeded (4)".into());
        }
        if request_bytes > self.retained_limit.saturating_sub(self.retained) {
            return Err("Retained job byte budget exceeded".into());
        }
        let response = with_prepared(request, |ready| {
            let program_bytes = intersect_model::definitions::metadata_limits::serialized_size(
                ready.image.program(),
                MAX_PROGRAM,
            )
            .map_err(|e| format!("Job program byte budget: {e}"))?;
            // Fresh compilation verified that this exact manifest describes the
            // owning registry. Reserve its encoded metadata separately from both
            // program representations; native provider heap usage is not measured.
            let registry_bytes = intersect_model::definitions::metadata_limits::serialized_size(
                &ready.image.program().policies,
                MAX_PROGRAM,
            )
            .map_err(|e| format!("Job registry byte budget: {e}"))?;
            let bytes = program_bytes
                .checked_mul(2)
                .and_then(|n| n.checked_add(registry_bytes))
                .and_then(|n| n.checked_add(request_bytes))
                .ok_or("Job byte budget overflow")?;
            if bytes > self.retained_limit.saturating_sub(self.retained) {
                return Err("Retained job byte budget exceeded".into());
            }
            let program = serde_json::to_value(ready.image.program()).map_err(|e| e.to_string())?;
            let session = match EvaluationSession::new_owned(
                ready.image,
                ready.arguments,
                ready.bundle,
                ready.references,
                Some(ready.registry),
                ready.execution,
            ) {
                Ok(session) => session,
                Err(failure) => {
                    return Ok(json!({"pending":false,"result":outcome(program, Err(failure))}));
                }
            };
            let next = self
                .next
                .checked_add(1)
                .ok_or("Job identity sequence exhausted")?;
            let id = format!("{}:{next}", self.namespace);
            self.next = next;
            self.jobs.insert(
                id.clone(),
                Job {
                    session,
                    program,
                    bytes,
                },
            );
            self.retained += bytes;
            Ok(json!({"pending":true,"job":id}))
        })?;
        // Preparation diagnostics use the same terminal envelope as execution.
        if response.get("pending").is_some() {
            Ok(response)
        } else {
            Ok(json!({"pending":false,"result":response}))
        }
    }
    fn remove(&mut self, id: &str) -> Result<Job, String> {
        let job = self
            .jobs
            .remove(id)
            .ok_or("Unknown, stale or foreign function job")?;
        self.retained -= job.bytes;
        Ok(job)
    }
    fn advance(
        &mut self,
        id: &str,
        quantum: NonZeroUsize,
        cancelled: bool,
    ) -> Result<Value, String> {
        struct Control(bool);
        impl ExecutionControl for Control {
            fn cancellation_requested(&self) -> bool {
                self.0
            }
        }
        let progress = self
            .jobs
            .get_mut(id)
            .ok_or("Unknown, stale or foreign function job")?
            .session
            .resume_with_control(quantum, &Control(cancelled));
        match progress {
            SessionProgress::Pending => Ok(json!({"pending":true,"job":id})),
            SessionProgress::Complete(result) => {
                let job = self.remove(id)?;
                Ok(json!({"pending":false,"result":outcome(job.program, *result)}))
            }
            SessionProgress::Finished => {
                self.remove(id)?;
                Err("Function job already finished".into())
            }
        }
    }
}

/// Stateful application adapter shared by native harnesses and each WASM instance.
/// Initialization is one-shot: resetting a namespace must never retarget old IDs.
#[derive(Default)]
pub struct FunctionJobHost {
    jobs: Option<FunctionJobs>,
}
impl FunctionJobHost {
    pub fn dispatch(&mut self, input: &str) -> Result<Value, String> {
        if input.len() > 4_000_000 {
            return Err("Host request byte budget exceeded".into());
        }
        #[derive(serde::Deserialize)]
        struct Method {
            method: String,
        }
        let method: Method = serde_json::from_str(input).map_err(|e| e.to_string())?;
        match method.method.as_str() {
            "rcore_behavior/jobHost" => {
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Initialize {
                    method: String,
                    namespace: String,
                    retained_encoded_limit: Option<usize>,
                }
                let request: Initialize = serde_json::from_str(input).map_err(|e| e.to_string())?;
                if request.method != "rcore_behavior/jobHost" {
                    return Err("Invalid host initialization".into());
                }
                if self.jobs.is_some() {
                    return Err("Function job host already initialized".into());
                }
                self.jobs = Some(FunctionJobs::with_retained_limit(
                    request.namespace,
                    request.retained_encoded_limit.unwrap_or(MAX_RETAINED),
                )?);
                Ok(json!({"initialized":true}))
            }
            "rcore_behavior/startFunction"
            | "rcore_behavior/resumeFunction"
            | "rcore_behavior/cancelFunction"
            | "rcore_behavior/dropFunction" => self
                .jobs
                .as_mut()
                .ok_or("Function job host is not initialized")?
                .dispatch(input),
            _ => crate::dispatch(input),
        }
    }
}
