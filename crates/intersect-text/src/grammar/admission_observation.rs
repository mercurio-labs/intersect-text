//! Explicit caller-owned observations of returned grammar-analysis errors.
//! This adapter changes no admission rule, semantic budget or serialized program.
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceSpan;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    NotReached,
    CoreSourceAnalysis,
    NoXtext,
    LegacyFrontend,
    SourceProgramFrontend,
    /// Existing seven-category counter does not cover new certificate fees.
    RepeatCertificateAnalysisUninstrumented,
    /// Dedicated new template fees are outside the historical seven categories.
    CallFreeRelativeClosureUninstrumented,
    /// Adjacent source reads have distinct accounting from the old seven categories.
    AdjacentClosureSourceUninstrumented,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStage {
    Parent,
    TerminalParent,
    Executable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    DatatypeInference,
    TerminalPreparation,
    TerminalCheck,
    EntryCategory,
    FragmentCycles,
    RuleConstruction,
    LeftRecursion,
    PredictionCheck,
    FragmentCallers,
    FragmentCallScan,
    Follow,
    LexicalProgram,
    LexicalReachability,
    ParserProgram,
    ParserGuardCollection,
    ParserRuleStates,
    ParserEntryStates,
    ParserReachability,
    ParserRuleGraph,
    ParserRecursiveAnalysis,
    /// Failed whole-program qualified-nullable safety inventory in explicit runtime mode.
    RuntimeNullableInventory,
    /// Failed exact source/graph binding for one explicit runtime candidate.
    RuntimeCandidateBinding,
    /// Failed canonical FIRST/non-nullability query for that exact candidate.
    RuntimeCandidateNonnullableFirst,
    DeferredProof,
    /// Denied existing shared FOLLOW walk-entry charge.
    FollowWalk,
    /// Denied existing nullable required-call lookup charge.
    FollowRequiredLookup,
    /// Denied existing nullable required-call retention charge.
    FollowRequiredRetention,
}

/// Full UTF-8 text or explicit absence; a truncated value is never an identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}
impl<const N: usize> CapturedText<N> {
    fn copy(value: &str) -> Option<Self> {
        if value.len() > N {
            return None;
        }
        let mut result = Self {
            bytes: [0; N],
            len: value.len(),
        };
        result.bytes[..value.len()].copy_from_slice(value.as_bytes());
        Some(result)
    }
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes[..self.len])
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedSource {
    pub uri: CapturedText<512>,
    pub sha256: CapturedText<64>,
}
impl CapturedSource {
    fn copy(source: &SourceId) -> Option<Self> {
        Some(Self {
            uri: CapturedText::copy(&source.uri)?,
            sha256: CapturedText::copy(&source.sha256)?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseFailure {
    pub stage: RunStage,
    pub phase: Phase,
    /// A declaration source when proven; otherwise the actual analysis entry.
    pub source: Option<CapturedSource>,
    pub owner_rule: Option<CapturedText<128>>,
    pub declaration_span: Option<(usize, usize)>,
    pub decision: Option<CapturedText<256>>,
    pub capture_unavailable: bool,
    /// This phase-only increment does not fabricate measurements of inner guards.
    pub charged_work: Option<usize>,
    pub syntax_depth: Option<usize>,
    pub call_depth: Option<usize>,
    pub state_count: Option<usize>,
    pub repeated_state_count: Option<usize>,
}

/// One bounded record, supplied by the caller and deliberately not Sync.
/// Reusing an observer starts a new observation at the explicit observed API.
/// It owns no graph, callback, allocation, host resource or semantic allowance.
pub struct PredictionObservation {
    coverage: Cell<Coverage>,
    first: Cell<Option<PhaseFailure>>,
    first_reuse: Cell<Option<FirstReuseWorkSnapshot>>,
    // Caller-owned diagnostic configuration; reset clears data, not this opt-in.
    first_map_cost_enabled: Cell<bool>,
    first_map_cost: Cell<Option<FirstMapCostSnapshot>>,
}
impl Default for PredictionObservation {
    fn default() -> Self {
        Self::new()
    }
}
impl PredictionObservation {
    pub const fn new() -> Self {
        Self {
            coverage: Cell::new(Coverage::NotReached),
            first: Cell::new(None),
            first_reuse: Cell::new(None),
            first_map_cost_enabled: Cell::new(false),
            first_map_cost: Cell::new(None),
        }
    }
    pub fn reset(&self) {
        self.coverage.set(Coverage::NotReached);
        self.first.set(None);
        self.first_reuse.set(None);
        self.first_map_cost.set(None);
    }
    pub fn coverage(&self) -> Coverage {
        self.coverage.get()
    }
    pub fn first_failure(&self) -> Option<PhaseFailure> {
        self.first.get()
    }
    /// Host/frontend plumbing reports absent coverage without rejecting inputs.
    pub fn not_instrumented(&self, reason: Coverage) {
        self.coverage.set(reason);
    }
    pub(crate) fn analysis_reached(&self) {
        self.coverage.set(Coverage::CoreSourceAnalysis);
    }
    /// Attach only directly measured counters to this newly recorded phase.
    /// Work is cumulative within the existing shared program-analysis bucket;
    /// it is not inferred from elapsed time, graph size or an error message.
    pub(crate) fn parser_measurements(
        &self,
        phase: Phase,
        work: Option<usize>,
        states: Option<usize>,
    ) {
        if let Some(mut failure) = self.first.get()
            && failure.phase == phase
        {
            failure.charged_work = work;
            failure.state_count = states;
            self.first.set(Some(failure));
        }
    }
    /// Direct measurements of the existing fragment caller syntax walk only.
    pub(crate) fn fragment_call_measurements(&self, work: usize, depth: Option<usize>) {
        if let Some(mut failure) = self.first.get()
            && failure.phase == Phase::FragmentCallScan
        {
            failure.charged_work = Some(work);
            failure.syntax_depth = depth;
            self.first.set(Some(failure));
        }
    }
    /// Existing shared FOLLOW walk/transfer bucket only, excluding nested FIRST.
    pub(crate) fn follow_measurements(&self, work: usize) {
        if let Some(mut failure) = self.first.get()
            && matches!(
                failure.phase,
                Phase::Follow
                    | Phase::FollowWalk
                    | Phase::FollowRequiredLookup
                    | Phase::FollowRequiredRetention
            )
        {
            failure.charged_work = Some(work);
            self.first.set(Some(failure));
        }
    }
    pub(crate) fn returned_error(
        &self,
        entry: &SourceId,
        stage: RunStage,
        phase: Phase,
        owner: Option<(&str, &SourceSpan)>,
        decision: Option<&str>,
    ) {
        if self.first.get().is_some() {
            return;
        }
        let identity = owner.map_or(entry, |(_, span)| &span.source);
        let source = CapturedSource::copy(identity);
        let owner_rule = owner.and_then(|(name, _)| CapturedText::copy(name));
        let captured_decision = decision.and_then(CapturedText::copy);
        self.first.set(Some(PhaseFailure {
            stage,
            phase,
            source,
            owner_rule,
            declaration_span: owner.map(|(_, span)| (span.start, span.end)),
            decision: captured_decision,
            capture_unavailable: source.is_none()
                || (owner.is_some() && owner_rule.is_none())
                || (decision.is_some() && captured_decision.is_none()),
            charged_work: None,
            syntax_depth: None,
            call_depth: None,
            state_count: None,
            repeated_state_count: None,
        }));
    }
}

mod adjacent_work;
pub use adjacent_work::{
    ADJACENT_CHARGE_COUNT, AdjacentCharge, AdjacentContext, AdjacentEvent, AdjacentProgramWork,
    AdjacentWorkObservation, AdjacentWorkSnapshot, AnalysisWorkObservers, ObservedEdge,
};
mod recursive_counters;
pub use recursive_counters::{
    AnalysisObservers, RecursiveAnalysisCounters, RecursiveCounterSnapshot,
    RecursiveDecisionCounts, RecursiveProgramCounts, RecursiveStageCounts, RecursiveWorkCategory,
    RecursiveWorkCounts, SelectedStaticSnapshot,
};
pub(crate) use recursive_counters::{ScheduleEvent, ScheduleSite};

/// Explicit caller-owned compiler balance diagnostics; no admission authority.
pub mod pool_blocks;

mod first_reuse_work;
pub use first_reuse_work::{
    FIRST_REUSE_CHARGES, FIRST_REUSE_EVENTS, FirstMapCostSnapshot, FirstReuseCharge,
    FirstReuseEvent, FirstReuseWorkSnapshot,
};
pub(in crate::grammar) use first_reuse_work::{
    MapOperation as FirstMapOperation, Meter as FirstReuseMeter, Window as FirstReuseWindow,
};
