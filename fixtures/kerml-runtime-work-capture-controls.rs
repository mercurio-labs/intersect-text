// Shared fixed passive counters. No parser optimization or language award.
use super::*;
use intersect_xtext::package::BundleCompilation;
use intersect_xtext::draft::{RuntimePredictionWorkAudit, RuntimePredictionWorkDetail};

const PHASES: [&str; 20] = [
    "enter",
    "outer",
    "frontier_decision",
    "source_replay",
    "closure_visit",
    "visited_duplicate",
    "source_ports",
    "state_goal",
    "nested_decision",
    "cleanup",
    "follow",
    "return",
    "epsilon_action",
    "call",
    "first_edge",
    "full_edge",
    "token_edge",
    "source_publish",
    "input",
    "exit",
];
const KINDS: [&str; 17] = [
    "decision_enter",
    "completion_enter",
    "initial_row",
    "pending",
    "row",
    "copy_visited",
    "visited_unique",
    "visited_duplicate",
    "pop",
    "copy_pop",
    "follow_fanout",
    "follow_step",
    "copy_follow",
    "copy_edge",
    "push",
    "frontier",
    "accepting",
];
const CAPTURE_BYTES: usize = 32768;
struct Counts {
    units: [usize; 20],
    marks: [usize; 20],
    geometry: [[usize; 17]; 2],
    carrier_events: [usize; 2],
    amounts: [[usize; 17]; 2],
    returns: [usize; 129],
    bases: [usize; 129],
    parents: [usize; 129],
    depth: usize,
    active_source: Option<usize>,
    first: Option<usize>,
    last: Option<usize>,
    worker_work: usize,
    completed: usize,
    worker_peak: usize,
    return_peak: usize,
    total_depth_peak: usize,
    pending_peak: usize,
    frontier_peak: usize,
    accepting_peak: usize,
    visited_peak: usize,
    follow_peak: usize,
    machine_predict_work: usize,
    machine_predict_calls: usize,
    machine_calls: usize,
    pipeline_calls: usize,
    callback_order: u64,
    began: bool,
    observed: bool,
    invalid: bool,
}
impl Counts {
    fn new() -> Self {
        Self {
            units: [0; 20],
            marks: [0; 20],
            geometry: [[0; 17]; 2],
            carrier_events: [0; 2],
            amounts: [[0; 17]; 2],
            returns: [0; 129],
            bases: [0; 129],
            parents: [0; 129],
            depth: 0,
            active_source: None,
            first: None,
            last: None,
            worker_work: 0,
            completed: 0,
            worker_peak: 0,
            return_peak: 0,
            total_depth_peak: 0,
            pending_peak: 0,
            frontier_peak: 0,
            accepting_peak: 0,
            visited_peak: 0,
            follow_peak: 0,
            machine_predict_work: 0,
            machine_predict_calls: 0,
            machine_calls: 0,
            pipeline_calls: 0,
            callback_order: 14695981039346656037,
            began: false,
            observed: false,
            invalid: false,
        }
    }
    // Diagnostic-only streaming fingerprint: no event/string retention. It
    // checks consumer callback order, not arbitrary provider callback order.
    fn callback(&mut self, words: &[usize], texts: &[&str]) {
        for b in words.iter().flat_map(|n| n.to_le_bytes()).chain(
            texts
                .iter()
                .flat_map(|s| s.bytes().chain(std::iter::once(255))),
        ) {
            self.callback_order = (self.callback_order ^ u64::from(b)).wrapping_mul(1099511628211);
        }
    }
    fn phase(&mut self, source: usize, name: &'static str, work: Option<usize>) -> Option<()> {
        let now = work?;
        let index = PHASES.iter().position(|p| *p == name)?;
        if self.depth == 0 {
            if name != "enter" || self.active_source.is_some() {
                return None;
            }
            self.first = Some(now);
            self.last = Some(now);
            self.active_source = Some(source);
        } else {
            if self.active_source != Some(source) {
                return None;
            }
            let delta = self.last?.checked_sub(now)?;
            let parent = *self.parents.get(self.depth.checked_sub(1)?)?;
            self.units[parent] = self.units[parent].checked_add(delta)?;
            self.last = Some(now);
        }
        self.observed = true;
        self.marks[index] = self.marks[index].checked_add(1)?;
        if name == "enter" {
            *self.parents.get_mut(self.depth)? = index;
            self.depth = self.depth.checked_add(1)?;
            self.worker_peak = self.worker_peak.max(self.depth);
        } else if name == "exit" {
            self.depth = self.depth.checked_sub(1)?;
            if self.depth == 0 {
                self.worker_work = self
                    .worker_work
                    .checked_add(self.first?.checked_sub(now)?)?;
                self.completed = self.completed.checked_add(1)?;
                self.active_source = None;
                self.first = None;
                self.last = None;
            }
            // The actual parent phase is preserved in parents[depth-1].
        } else {
            *self.parents.get_mut(self.depth.checked_sub(1)?)? = index;
        }
        Some(())
    }
    fn geometry(
        &mut self,
        source: usize,
        e: intersect_xtext::grammar::RuntimeWorkGeometry,
        shared: bool,
    ) -> Option<()> {
        if self.depth == 0
            || self.active_source != Some(source)
            || (e.shared_storage && !shared)
            || e.base_depth > 128
            || e.return_depth > 128
        {
            return None;
        }
        let carrier = usize::from(e.shared_storage);
        self.carrier_events[carrier] = self.carrier_events[carrier].checked_add(1)?;
        let index = KINDS.iter().position(|p| *p == e.kind)?;
        self.geometry[source][index] = self.geometry[source][index].checked_add(1)?;
        self.amounts[source][index] = self.amounts[source][index].checked_add(e.amount)?;
        self.return_peak = self.return_peak.max(e.return_depth);
        self.total_depth_peak = self
            .total_depth_peak
            .max(e.base_depth.checked_add(e.return_depth)?);
        match e.kind {
            "row" => {
                self.returns[e.return_depth] = self.returns[e.return_depth].checked_add(1)?;
                self.bases[e.base_depth] = self.bases[e.base_depth].checked_add(1)?;
            }
            "pending" => self.pending_peak = self.pending_peak.max(e.amount),
            "frontier" => self.frontier_peak = self.frontier_peak.max(e.amount),
            "accepting" => self.accepting_peak = self.accepting_peak.max(e.amount),
            "visited_unique" => self.visited_peak = self.visited_peak.max(e.amount),
            "follow_fanout" => self.follow_peak = self.follow_peak.max(e.amount),
            _ => {}
        }
        Some(())
    }
}
struct RuntimeAudit<'a, const OBSERVED: bool, const SHARED: bool = false> {
    sources: &'a [SourceId; 2],
    counts: RefCell<Counts>,
    invalid: Cell<bool>,
}
const _: () = assert!(std::mem::size_of::<RuntimeAudit<'_, true>>() <= 16384);
impl<'a, const OBSERVED: bool, const SHARED: bool> RuntimeAudit<'a, OBSERVED, SHARED> {
    fn new(sources: &'a [SourceId; 2]) -> Self {
        Self {
            sources,
            counts: RefCell::new(Counts::new()),
            invalid: Cell::new(false),
        }
    }
    fn source(&self, source: Option<&SourceId>, uri: &str) -> Option<usize> {
        self.sources
            .iter()
            .position(|id| Some(id) == source && id.uri == uri)
    }
    fn snapshot(&self) -> Value {
        let Ok(c) = self.counts.try_borrow() else {
            return json!({"complete":false,"borrow_error":true});
        };
        let complete = OBSERVED
            && c.began
            && c.observed
            && !c.invalid
            && !self.invalid.get()
            && c.depth == 0
            && c.active_source.is_none();
        let disjoint = c.units.iter().try_fold(0usize, |a, b| a.checked_add(*b));
        let conservation = complete
            .then_some(disjoint.zip(Some(c.worker_work)).map(|(a, b)| a == b))
            .flatten();
        let residual = complete
            .then_some(c.machine_predict_work.checked_sub(c.worker_work))
            .flatten();
        json!({"enabled":OBSERVED,"complete":complete,"overflow_unknown_or_unbalanced":c.invalid || self.invalid.get(),
            "phase_order":PHASES,"phase_units":if complete {Some(c.units)} else {None},
            "phase_markers":if complete {Some(c.marks)} else {None},"geometry_order":KINDS,
            "geometry_counts_by_source":if complete {Some(c.geometry)} else {None},
            "geometry_amounts_by_source":if complete {Some(c.amounts)} else {None},
            "row_return_depth_histogram":if complete {Some(c.returns.to_vec())} else {None},
            "row_base_depth_histogram":if complete {Some(c.bases.to_vec())} else {None},
            "worker_nesting_peak":if complete {Some(c.worker_peak)} else {None},"row_return_peak":if complete {Some(c.return_peak)} else {None},"base_plus_return_peak":if complete {Some(c.total_depth_peak)} else {None},
            "pending_sampled_peak":if complete {Some(c.pending_peak)} else {None},"frontier_sampled_peak":if complete {Some(c.frontier_peak)} else {None},
            "accepting_sampled_peak":if complete {Some(c.accepting_peak)} else {None},"visited_peak":if complete {Some(c.visited_peak)} else {None},"global_follow_fanout_peak":if complete {Some(c.follow_peak)} else {None},
            "completed_top_level_invocations":if complete {Some(c.completed)} else {None},"raw_open_worker_depth":c.depth,
            "raw_phase_callback_marks":c.marks.iter().try_fold(0usize, |a,b| a.checked_add(*b)),
            "disjoint_worker_work":if complete {Some(c.worker_work)} else {None},"conservation":conservation,
            "machine_predict_work":c.machine_predict_work,"machine_predict_calls":c.machine_predict_calls,
            "machine_predict_residual_outside_worker":residual,"original_machine_callbacks":c.machine_calls,
            "original_pipeline_callbacks":c.pipeline_calls,"capture_reserved_bytes":CAPTURE_BYTES,
            "counter_struct_bytes":std::mem::size_of::<Self>(),"original_1MiB_envelope":1048576,
            "shared_storage_allowed":SHARED,"geometry_events_by_Vec_then_Shared":if complete {Some(c.carrier_events)} else {None},"scope":"Shared-enabled mode observes both existing Shared outer and Vec nested workers; default Vec-only mode rejects Shared events. Clone amounts cover named canonical-worker sites only; prepared-cache/publication/binding copies outside these sites are unmeasured. These are named canonical-carrier logical element counts, not debit/allocator/RSS/comparator CPU. Shared tails may have different physical storage. Collection peaks are samples at specified worker sites. OFF/unknown/unbalanced authoritative runtime fields are null; explicitly raw counters show diagnostic prefix state only. Runtime counters add no work, result or cache authority; callback/export CPU is outside semantic work."})
    }
    fn original_callbacks(&self) -> Value {
        let c = self.counts.borrow();
        json!({"machine_calls":c.machine_calls,"pipeline_calls":c.pipeline_calls,
            "machine_predict_calls":c.machine_predict_calls,"machine_predict_work":c.machine_predict_work,"consumer_callback_order_fingerprint":c.callback_order})
    }
}
impl<const OBSERVED: bool, const SHARED: bool> BasicKerMLProjectAudit
    for RuntimeAudit<'_, OBSERVED, SHARED>
{
    const RETAINED_BYTES: usize = CAPTURE_BYTES;
    const RUNTIME_WORK_DIAGNOSTICS: bool = OBSERVED;
    fn begin(&self, entry: usize, current: usize) {
        let Ok(mut c) = self.counts.try_borrow_mut() else {
            self.invalid.set(true);
            return;
        };
        if c.began {
            c.invalid = true;
        }
        c.began = true;
        c.callback(&[entry, current], &["begin"]);
    }
    fn pipeline(&self, event: intersect_xtext::draft::PipelineAudit<'_>) {
        let Ok(mut c) = self.counts.try_borrow_mut() else {
            self.invalid.set(true);
            return;
        };
        c.callback(
            &[
                usize::from(event.before.is_some()),
                event.before.map_or(0, |x| x.reads),
                event.before.map_or(0, |x| x.remaining),
                usize::from(event.after.is_some()),
                event.after.map_or(0, |x| x.reads),
                event.after.map_or(0, |x| x.remaining),
                usize::from(event.result.is_ok()),
            ],
            &["pipeline", event.phase, event.result.err().unwrap_or("")],
        );
        if let Some(n) = c.pipeline_calls.checked_add(1) {
            c.pipeline_calls = n;
        } else {
            c.invalid = true;
        }
    }
    fn machine(
        &self,
        source: Option<&SourceId>,
        event: intersect_xtext::draft::MachineWorkAudit<'_>,
    ) {
        let known = self.source(source, event.uri).is_some();
        let Ok(mut c) = self.counts.try_borrow_mut() else {
            self.invalid.set(true);
            return;
        };
        c.callback(
            &[
                event.before,
                event.after,
                usize::from(event.resource_grant.is_some()),
                event.resource_grant.unwrap_or(0),
                usize::from(event.result.is_ok()),
            ],
            &[
                "machine",
                event.uri,
                event.phase,
                event.detail.unwrap_or(""),
                event.result.err().unwrap_or(""),
                source.map_or("", |s| s.uri.as_str()),
                source.map_or("", |s| s.sha256.as_str()),
            ],
        );
        if !known {
            c.invalid = true;
        }
        if let Some(n) = c.machine_calls.checked_add(1) {
            c.machine_calls = n;
        } else {
            c.invalid = true;
        }
        if event.phase == "predict" {
            if let Some(n) = c.machine_predict_calls.checked_add(1) {
                c.machine_predict_calls = n;
            } else {
                c.invalid = true;
            }
            if let Some(n) = event
                .after
                .checked_sub(event.before)
                .and_then(|d| c.machine_predict_work.checked_add(d))
            {
                c.machine_predict_work = n;
            } else {
                c.invalid = true;
            }
        }
    }
    fn runtime(&self, source: Option<&SourceId>, event: RuntimePredictionWorkAudit<'_>) {
        let index = self.source(source, event.uri);
        let Ok(mut c) = self.counts.try_borrow_mut() else {
            self.invalid.set(true);
            return;
        };
        if c.invalid || self.invalid.get() {
            return;
        }
        let Some(index) = index else {
            c.invalid = true;
            return;
        };
        if !c.began {
            c.invalid = true;
            return;
        }
        let result = match event.detail {
            RuntimePredictionWorkDetail::Phase(name, remaining) => c.phase(index, name, remaining),
            RuntimePredictionWorkDetail::Geometry(e) => c.geometry(index, e, SHARED),
        };
        if result.is_none() {
            c.invalid = true;
        }
    }
}
