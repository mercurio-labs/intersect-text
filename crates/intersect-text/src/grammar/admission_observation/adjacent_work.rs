//! Bounded caller-owned accounting of the existing V76 parser-analysis pool.
//! This is observation, never a new work grant or retained parser state.
use super::{AnalysisObservers, CapturedSource, CapturedText, Coverage, RunStage};
use std::cell::Cell;

#[derive(Clone, Copy)]
pub struct AnalysisWorkObservers<'a> {
    pub legacy: AnalysisObservers<'a>,
    pub adjacent_work: Option<&'a AdjacentWorkObservation>,
}
impl<'a> From<AnalysisObservers<'a>> for AnalysisWorkObservers<'a> {
    fn from(legacy: AnalysisObservers<'a>) -> Self {
        Self {
            legacy,
            adjacent_work: None,
        }
    }
}
impl AnalysisWorkObservers<'_> {
    pub fn reset(self) {
        self.legacy.reset();
        if let Some(work) = self.adjacent_work {
            work.reset();
        }
    }
    pub fn not_instrumented(self, reason: Coverage) {
        self.legacy.not_instrumented(reason);
    }
    pub(crate) fn analysis_reached(self) {
        self.legacy.analysis_reached();
    }
}

/// Successful debits are disjoint. Denied attempts do not increase spent work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum AdjacentCharge {
    CallFreeRuleRetention,
    RepeatBinding,
    RepeatConsumption,
    FrontierPop,
    LegacyClosureVisit,
    KeySeed,
    KeyReturn,
    KeyFollow,
    KeySuccessor,
    CallerlessFollowEdge,
    TransitionEdge,
    WitnessState,
    WitnessEdge,
    ShapeInspection,
    OverflowGrowth,
    TemplateLookup,
    TemplateSeed,
    TemplateClosureState,
    TemplateClosureEdge,
    TemplateTokenRetention,
    TemplateStopCheck,
    TemplatePublication,
    TemplateEmission,
    TemplateCallerlessFollow,
}
pub const ADJACENT_CHARGE_COUNT: usize = 24;
/// Post-charge edge classification. A denied inspection never guesses its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ObservedEdge {
    Epsilon,
    Action,
    First,
    Full,
    Token,
    Call,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AdjacentEvent {
    #[default]
    ClosurePop,
    KeyAttempt,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdjacentContext {
    pub event: AdjacentEvent,
    pub frontier: u64,
    pub state: usize,
    pub alternative: usize,
    pub call_depth: usize,
    pub pending_frontiers: usize,
    pub queue_batches: usize,
    pub front_siblings: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdjacentProgramWork {
    pub ordinal: u64,
    pub stage: RunStage,
    pub source: Option<CapturedSource>,
    pub initial: usize,
    pub remaining: usize,
    pub granted: [u64; ADJACENT_CHARGE_COUNT],
    pub denied: [u64; ADJACENT_CHARGE_COUNT],
    pub completed: bool,
    pub conserved: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdjacentWorkSnapshot {
    pub programs: u64,
    pub covered_v76: bool,
    pub unsupported_profile: bool,
    pub granted: [u64; ADJACENT_CHARGE_COUNT],
    pub denied: [u64; ADJACENT_CHARGE_COUNT],
    pub current: Option<AdjacentProgramWork>,
    pub last: Option<AdjacentProgramWork>,
    pub first_failed: Option<AdjacentProgramWork>,
    pub decision: Option<CapturedText<256>>,
    pub capture_unavailable: bool,
    pub transitions: [u64; 6],
    pub shape_edges: [u64; 6],
    pub transparent_shapes: u64,
    pub fallback_shapes: u64,
    pub fallback_stop_or_fanout: u64,
    pub keys_absent: [u64; 4],
    pub keys_duplicate: [u64; 4],
    pub keys_retained: [u64; 4],
    pub max_frontier: u64,
    pub peak_call_depth: usize,
    pub peak_queue_batches: usize,
    pub peak_siblings: usize,
    pub contexts: [AdjacentContext; 16],
    pub context_count: u64,
    pub overflowed: bool,
    pub inconsistent: bool,
}
impl Default for AdjacentWorkSnapshot {
    fn default() -> Self {
        Self {
            programs: 0,
            covered_v76: false,
            unsupported_profile: false,
            granted: [0; ADJACENT_CHARGE_COUNT],
            denied: [0; ADJACENT_CHARGE_COUNT],
            current: None,
            last: None,
            first_failed: None,
            decision: None,
            capture_unavailable: false,
            transitions: [0; 6],
            shape_edges: [0; 6],
            transparent_shapes: 0,
            fallback_shapes: 0,
            fallback_stop_or_fanout: 0,
            keys_absent: [0; 4],
            keys_duplicate: [0; 4],
            keys_retained: [0; 4],
            max_frontier: 0,
            peak_call_depth: 0,
            peak_queue_batches: 0,
            peak_siblings: 0,
            contexts: [AdjacentContext::default(); 16],
            context_count: 0,
            overflowed: false,
            inconsistent: false,
        }
    }
}
pub struct AdjacentWorkObservation {
    state: Cell<AdjacentWorkSnapshot>,
}
impl Default for AdjacentWorkObservation {
    fn default() -> Self {
        Self::new()
    }
}
fn increment(value: &mut u64, overflow: &mut bool) {
    match value.checked_add(1) {
        Some(next) => *value = next,
        None => *overflow = true,
    }
}
impl AdjacentWorkObservation {
    pub fn new() -> Self {
        Self {
            state: Cell::new(AdjacentWorkSnapshot::default()),
        }
    }
    pub fn reset(&self) {
        self.state.set(AdjacentWorkSnapshot::default());
    }
    pub fn snapshot(&self) -> AdjacentWorkSnapshot {
        self.state.get()
    }
    pub(crate) fn unsupported_profile(&self) {
        let mut s = self.state.get();
        s.unsupported_profile = true;
        self.state.set(s);
    }
    pub(crate) fn begin(
        &self,
        stage: RunStage,
        source: &intersect_model::definitions::bundle::SourceId,
        initial: usize,
    ) {
        let mut s = self.state.get();
        if s.current.is_some() {
            s.inconsistent = true;
        }
        increment(&mut s.programs, &mut s.overflowed);
        s.covered_v76 = true;
        let captured = CapturedSource::copy(source);
        s.capture_unavailable |= captured.is_none();
        s.current = Some(AdjacentProgramWork {
            ordinal: s.programs,
            stage,
            source: captured,
            initial,
            remaining: initial,
            granted: [0; ADJACENT_CHARGE_COUNT],
            denied: [0; ADJACENT_CHARGE_COUNT],
            completed: false,
            conserved: false,
        });
        self.state.set(s);
    }
    pub(crate) fn end(&self, remaining: usize, completed: bool) {
        let mut s = self.state.get();
        if let Some(mut p) = s.current.take() {
            p.remaining = remaining;
            p.completed = completed;
            let sum = p.granted.iter().try_fold(0u64, |a, b| a.checked_add(*b));
            p.conserved =
                sum == Some(p.initial.saturating_sub(remaining) as u64) && remaining <= p.initial;
            s.inconsistent |= !p.conserved;
            if !completed && s.first_failed.is_none() {
                s.first_failed = Some(p);
            }
            s.last = Some(p);
        } else {
            s.inconsistent = true;
        }
        self.state.set(s);
    }
    pub(crate) fn decision(&self, path: &str) {
        let mut s = self.state.get();
        s.decision = CapturedText::copy(path);
        s.capture_unavailable |= s.decision.is_none();
        self.state.set(s);
    }
    pub(crate) fn charge(&self, category: AdjacentCharge, granted: bool) {
        let mut s = self.state.get();
        let i = category as usize;
        if granted {
            increment(&mut s.granted[i], &mut s.overflowed);
        } else {
            increment(&mut s.denied[i], &mut s.overflowed);
        }
        if let Some(ref mut p) = s.current {
            if granted {
                increment(&mut p.granted[i], &mut s.overflowed);
            } else {
                increment(&mut p.denied[i], &mut s.overflowed);
            }
        } else {
            s.inconsistent = true;
        }
        self.state.set(s);
    }
    pub(crate) fn edge(&self, edge: ObservedEdge) {
        let mut s = self.state.get();
        increment(&mut s.transitions[edge as usize], &mut s.overflowed);
        self.state.set(s);
    }
    pub(crate) fn shape(&self, transparent: bool, edge: Option<ObservedEdge>) {
        let mut s = self.state.get();
        if transparent {
            increment(&mut s.transparent_shapes, &mut s.overflowed);
        } else {
            increment(&mut s.fallback_shapes, &mut s.overflowed);
        }
        if let Some(edge) = edge {
            increment(&mut s.shape_edges[edge as usize], &mut s.overflowed);
        } else {
            increment(&mut s.fallback_stop_or_fanout, &mut s.overflowed);
        }
        self.state.set(s);
    }
    pub(crate) fn key(&self, site: super::ScheduleSite, event: super::ScheduleEvent) {
        let mut s = self.state.get();
        let i = site as usize;
        let counts = match event {
            super::ScheduleEvent::Absent => &mut s.keys_absent,
            super::ScheduleEvent::Retained => &mut s.keys_retained,
            super::ScheduleEvent::Duplicate => &mut s.keys_duplicate,
        };
        increment(&mut counts[i], &mut s.overflowed);
        self.state.set(s);
    }
    pub(crate) fn key_context(
        &self,
        state: usize,
        alternative: usize,
        call_depth: usize,
        queue: (usize, usize),
    ) {
        let s = self.state.get();
        let mut context = if s.context_count == 0 {
            AdjacentContext::default()
        } else {
            s.contexts[((s.context_count - 1) % 16) as usize]
        };
        context.event = AdjacentEvent::KeyAttempt;
        context.state = state;
        context.alternative = alternative;
        context.call_depth = call_depth;
        context.queue_batches = queue.0;
        context.front_siblings = queue.1;
        self.context(context);
    }
    pub(crate) fn context(&self, context: AdjacentContext) {
        let mut s = self.state.get();
        s.max_frontier = s.max_frontier.max(context.frontier);
        s.peak_call_depth = s.peak_call_depth.max(context.call_depth);
        s.peak_queue_batches = s.peak_queue_batches.max(context.queue_batches);
        s.peak_siblings = s.peak_siblings.max(context.front_siblings);
        let i = (s.context_count % 16) as usize;
        s.contexts[i] = context;
        increment(&mut s.context_count, &mut s.overflowed);
        self.state.set(s);
    }
}
