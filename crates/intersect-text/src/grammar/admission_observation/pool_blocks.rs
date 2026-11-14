//! Explicit opt-in compiler-pool diagnostic; no semantic or work authority.
//!
//! Fixed caller-owned observations of the existing compiler analysis pool.
//! Blocks measure actual before/after balances; children are subtracted once.
//! A block is a source boundary, not a CPU/allocator/semantic-work estimate.
//! In particular, PoolRemainder remains explicitly unclassified.
use std::cell::Cell;
mod first_queue_topology;
pub use first_queue_topology::{
    Coverage as FirstQueueTopologyCoverage, Snapshot as FirstQueueTopologySnapshot,
    Unavailable as FirstQueueTopologyUnavailable,
};

const BLOCKS: usize = 16;
const DEPTH: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Block {
    PoolRemainder,
    GraphSourceBinding,
    CallFreeRuleRetention,
    NullableSourceQualification,
    NullableGraphProof,
    RepeatCertificate,
    ClassifierRemainder,
    FrontierInitial,
    FrontierPreparation,
    QueueShareRemainder,
    PortKernel,
    PortDescriptor,
    PortSchedule,
    Witness,
    CallFreeReplay,
    FrontierPublish,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub units: [u64; BLOCKS],
    pub actual_denied_leaf_attempts: [u64; BLOCKS],
    pub calls: [u64; BLOCKS],
    pub error_exits: [u64; BLOCKS],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Frame {
    block: Block,
    before: usize,
    children: usize,
    serial: u64,
}
const EMPTY: Frame = Frame {
    block: Block::PoolRemainder,
    before: 0,
    children: 0,
    serial: 0,
};

/// Opaque observation token. It carries no budget or parser authority.
#[derive(Clone, Copy)]
pub struct Token<'a> {
    owner: &'a Observation<'a>,
    generation: u64,
    serial: u64,
    depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub initial: usize,
    pub remaining: usize,
    pub counts: Counts,
    pub first_actual_denial: Option<Block>,
    pub first_error_exit: Option<Block>,
    pub completed_result: bool,
    pub ended: bool,
    pub balanced: bool,
    pub overflowed: bool,
    pub inconsistent: bool,
    pub peak_depth: usize,
    /// Overlapping source-block totals, NOT additional conserved units.
    /// 0=PortQueue, 1=SeededQueue, 2=other explicitly named queue.
    pub queue_share_inclusive: [u64; 3],
}

#[derive(Clone, Copy)]
struct State {
    snapshot: Snapshot,
    frames: [Frame; DEPTH],
    depth: usize,
    serial: u64,
    generation: u64,
    root_started: bool,
    root_result_ok: Option<bool>,
}

/// One program invocation only. The integrating owner must retain the actual
/// bounded SourceId/stage/selector binding beside this snapshot and instantiate
/// a fresh observer for each program; no aggregate is a single 10000-unit pool.
pub struct Observation<'a> {
    state: Cell<State>,
    topology: Option<first_queue_topology::Handle<'a>>,
}

fn increment(value: &mut u64, overflow: &mut bool) {
    match value.checked_add(1) {
        Some(next) => *value = next,
        None => *overflow = true,
    }
}
fn add(value: &mut u64, amount: usize, overflow: &mut bool) {
    match u64::try_from(amount)
        .ok()
        .and_then(|n| value.checked_add(n))
    {
        Some(next) => *value = next,
        None => *overflow = true,
    }
}

fn invalid(state: &mut State) {
    state.snapshot.inconsistent = true;
    state.snapshot.balanced = false;
}

/// Every attempted mutation invalidates a previously completed balance claim.
/// Invalid/overflowed/ended observations are sticky and never restart a scope.
fn allow_mutation(state: &mut State, needs_scope: bool) -> bool {
    state.snapshot.balanced = false;
    if state.snapshot.ended
        || state.snapshot.inconsistent
        || state.snapshot.overflowed
        || (needs_scope && state.depth == 0)
    {
        invalid(state);
        false
    } else {
        true
    }
}

impl<'a> Observation<'a> {
    pub fn new(initial: usize) -> Self {
        Self {
            state: Cell::new(State {
                snapshot: Snapshot {
                    initial,
                    remaining: initial,
                    counts: Counts::default(),
                    first_actual_denial: None,
                    first_error_exit: None,
                    completed_result: false,
                    ended: false,
                    balanced: false,
                    overflowed: false,
                    inconsistent: false,
                    peak_depth: 0,
                    queue_share_inclusive: [0; 3],
                },
                frames: [EMPTY; DEPTH],
                depth: 0,
                serial: 0,
                generation: 1,
                root_started: false,
                root_result_ok: None,
            }),
            topology: None,
        }
    }

    pub(crate) fn new_with_topology(initial: usize, owner: &'a CompilerPoolDiagnostic) -> Self {
        let mut observation = Self::new(initial);
        if owner.topology.enabled() {
            observation.topology = Some(first_queue_topology::Handle {
                owner: &owner.topology,
                generation: Cell::new(None),
            });
        }
        observation
    }

    pub(crate) fn topology_enabled(&self) -> bool {
        self.topology.is_some()
    }
    fn topology_scope_active(&self) -> bool {
        let state = self.state.get();
        state.depth != 0
            && !state.snapshot.ended
            && !state.snapshot.inconsistent
            && !state.snapshot.overflowed
    }
    pub(crate) fn topology_unsupported_policy(&self) {
        if let Some(handle) = &self.topology {
            handle
                .owner
                .unavailable(first_queue_topology::Unavailable::UnsupportedPolicy);
        }
    }
    pub(crate) fn topology_queue(&self, fresh: bool, graph_states: usize) {
        if let Some(handle) = &self.topology {
            handle.owner.queue(
                &handle.generation,
                self.topology_scope_active(),
                fresh,
                graph_states,
            );
        }
    }
    pub(crate) fn topology_walk(&self) {
        if let Some(handle) = &self.topology {
            handle
                .owner
                .walk(&handle.generation, self.topology_scope_active());
        }
    }
    pub(crate) fn topology_expansion(&self, state: usize, action_seen: bool) {
        if let Some(handle) = &self.topology {
            handle.owner.expansion(
                &handle.generation,
                self.topology_scope_active(),
                state,
                action_seen,
            );
        }
    }
    pub(crate) fn topology_edge(&self, state: usize, action_seen: bool) {
        if let Some(handle) = &self.topology {
            handle.owner.edge(
                &handle.generation,
                self.topology_scope_active(),
                state,
                action_seen,
            );
        }
    }
    pub(crate) fn topology_completed_interior(&self, state: usize, action_seen: bool) {
        if let Some(handle) = &self.topology {
            handle.owner.completed_interior(
                &handle.generation,
                self.topology_scope_active(),
                state,
                action_seen,
            );
        }
    }

    pub fn enter(&self, block: Block, remaining: usize) -> Option<Token<'_>> {
        let mut state = self.state.get();
        if !allow_mutation(&mut state, false) {
            self.state.set(state);
            return None;
        }
        if state.depth == DEPTH {
            invalid(&mut state);
            self.state.set(state);
            return None;
        }
        if state.depth == 0 {
            if state.root_started
                || block != Block::PoolRemainder
                || remaining != state.snapshot.initial
            {
                invalid(&mut state);
                self.state.set(state);
                return None;
            }
            state.root_started = true;
        }
        let Some(serial) = state.serial.checked_add(1) else {
            state.snapshot.overflowed = true;
            self.state.set(state);
            return None;
        };
        state.serial = serial;
        let depth = state.depth;
        state.frames[depth] = Frame {
            block,
            before: remaining,
            children: 0,
            serial,
        };
        state.depth += 1;
        state.snapshot.peak_depth = state.snapshot.peak_depth.max(state.depth);
        increment(
            &mut state.snapshot.counts.calls[block as usize],
            &mut state.snapshot.overflowed,
        );
        self.state.set(state);
        Some(Token {
            owner: self,
            generation: state.generation,
            serial,
            depth,
        })
    }

    /// Only call after the original debit Result exists. A scope returning Err
    /// is NOT itself evidence of a denied debit. No result/error is rewritten.
    pub fn actual_leaf_result(&self, block: Block, granted: bool) {
        let mut state = self.state.get();
        if !allow_mutation(&mut state, true) {
            self.state.set(state);
            return;
        }
        if granted {
            let frame = &mut state.frames[state.depth - 1];
            match frame.children.checked_add(1) {
                Some(next) => frame.children = next,
                None => state.snapshot.overflowed = true,
            }
            increment(
                &mut state.snapshot.counts.units[block as usize],
                &mut state.snapshot.overflowed,
            );
        } else {
            increment(
                &mut state.snapshot.counts.actual_denied_leaf_attempts[block as usize],
                &mut state.snapshot.overflowed,
            );
            if state.snapshot.first_actual_denial.is_none() {
                state.snapshot.first_actual_denial = Some(block);
            }
        }
        self.state.set(state);
    }

    pub fn leave(&self, token: Option<Token<'_>>, remaining: usize, result_ok: bool) {
        let mut state = self.state.get();
        if !allow_mutation(&mut state, true) {
            self.state.set(state);
            return;
        }
        let Some(token) = token else {
            invalid(&mut state);
            self.state.set(state);
            return;
        };
        if !std::ptr::eq(self, token.owner)
            || state.depth != token.depth + 1
            || state.generation != token.generation
            || state.frames[token.depth].serial != token.serial
        {
            invalid(&mut state);
            self.state.set(state);
            return;
        }
        let frame = state.frames[token.depth];
        let Some(consumed) = frame.before.checked_sub(remaining) else {
            invalid(&mut state);
            self.state.set(state);
            return;
        };
        let Some(exclusive) = consumed.checked_sub(frame.children) else {
            invalid(&mut state);
            self.state.set(state);
            return;
        };
        add(
            &mut state.snapshot.counts.units[frame.block as usize],
            exclusive,
            &mut state.snapshot.overflowed,
        );
        if !result_ok {
            increment(
                &mut state.snapshot.counts.error_exits[frame.block as usize],
                &mut state.snapshot.overflowed,
            );
            if state.snapshot.first_error_exit.is_none() {
                state.snapshot.first_error_exit = Some(frame.block);
            }
        }
        state.depth -= 1;
        if state.depth != 0 {
            let parent = &mut state.frames[state.depth - 1];
            match parent.children.checked_add(consumed) {
                Some(next) => parent.children = next,
                None => state.snapshot.overflowed = true,
            }
        } else {
            state.root_result_ok = Some(result_ok);
        }
        self.state.set(state);
    }

    pub fn end(&self, remaining: usize, result_ok: bool) {
        let mut state = self.state.get();
        if !allow_mutation(&mut state, false) {
            self.state.set(state);
            return;
        }
        if state.depth != 0 || !state.root_started || state.root_result_ok != Some(result_ok) {
            invalid(&mut state);
        }
        state.snapshot.remaining = remaining;
        state.snapshot.completed_result = result_ok;
        state.snapshot.ended = true;
        let sum = state
            .snapshot
            .counts
            .units
            .iter()
            .try_fold(0u64, |a, n| a.checked_add(*n));
        let spent = state
            .snapshot
            .initial
            .checked_sub(remaining)
            .and_then(|n| u64::try_from(n).ok());
        // Queue-kind spans overlap the partition, but the concrete share calls
        // are disjoint from one another. They cannot exceed this whole pool.
        let queue_sum = state
            .snapshot
            .queue_share_inclusive
            .iter()
            .try_fold(0u64, |a, n| a.checked_add(*n));
        if queue_sum
            .zip(spent)
            .is_none_or(|(queue, spent)| queue > spent)
        {
            invalid(&mut state);
        }
        state.snapshot.balanced =
            !state.snapshot.inconsistent && !state.snapshot.overflowed && sum == spent;
        self.state.set(state);
    }

    pub fn snapshot(&self) -> Snapshot {
        self.state.get().snapshot
    }

    /// The integrating generic worker must supply a statically selected queue
    /// kind at the actual dispatch. This does not infer kind from a graph/path.
    pub fn queue_span(&self, kind: usize, before: usize, after: usize) {
        let mut state = self.state.get();
        if !allow_mutation(&mut state, true) {
            self.state.set(state);
            return;
        }
        match (
            state.snapshot.queue_share_inclusive.get_mut(kind),
            before.checked_sub(after),
        ) {
            (Some(slot), Some(consumed)) => add(slot, consumed, &mut state.snapshot.overflowed),
            _ => invalid(&mut state),
        }
        self.state.set(state);
    }
}

const _: () = assert!(std::mem::size_of::<Observation>() <= 4096);

/// Source blocks should otherwise use enter/original Result/leave so their
/// whole historical body stays literal. This helper illustrates the protocol.
pub fn scope<T, E>(
    work: &mut usize,
    observation: Option<&Observation>,
    block: Block,
    body: impl FnOnce(&mut usize) -> Result<T, E>,
) -> Result<T, E> {
    let token = observation.and_then(|observer| observer.enter(block, *work));
    let result = body(work);
    if let Some(observer) = observation {
        observer.leave(token, *work, result.is_ok());
    }
    result
}

use super::{CapturedSource, CapturedText, RunStage};
use crate::grammar::RecursiveFrontierOrder;
use intersect_model::definitions::bundle::SourceId;

/// One actual compiler program's source binding and observed original pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProgramSnapshot {
    pub ordinal: u64,
    pub stage: RunStage,
    pub source: Option<CapturedSource>,
    pub order: RecursiveFrontierOrder,
    pub decision: Option<CapturedText<256>>,
    pub work: Snapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Binding {
    ordinal: u64,
    stage: RunStage,
    source: Option<CapturedSource>,
    order: RecursiveFrontierOrder,
    decision: Option<CapturedText<256>>,
}

/// Bounded report; last/first_failed are whole individual program observations.
/// Programs is a count, never an aggregate presented as one 10000-unit pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticSnapshot {
    pub programs: u64,
    pub active: bool,
    pub last: Option<ProgramSnapshot>,
    pub first_failed: Option<ProgramSnapshot>,
    pub capture_unavailable: bool,
    pub inconsistent: bool,
    pub overflowed: bool,
}

/// Explicit caller-owned diagnostic only. It cannot grant work or admit syntax.
pub struct CompilerPoolDiagnostic {
    state: Cell<DiagnosticSnapshot>,
    binding: Cell<Option<Binding>>,
    topology: first_queue_topology::Observation,
}

impl Default for CompilerPoolDiagnostic {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerPoolDiagnostic {
    pub fn new() -> Self {
        Self {
            state: Cell::new(DiagnosticSnapshot::default()),
            binding: Cell::new(None),
            topology: first_queue_topology::Observation::new(),
        }
    }
    /// Native diagnostic only; no topology bits become parser authority.
    pub fn enable_first_queue_topology(&self) {
        let state = self.state.get();
        let unused = state.programs == 0 && !state.active && self.binding.get().is_none();
        self.topology.enable(unused);
        if !unused {
            self.unavailable();
        }
    }
    pub fn first_queue_topology_snapshot(&self) -> FirstQueueTopologySnapshot {
        self.topology.snapshot()
    }
    pub fn snapshot(&self) -> DiagnosticSnapshot {
        self.state.get()
    }

    pub(crate) fn unavailable(&self) {
        let mut state = self.state.get();
        state.capture_unavailable = true;
        self.state.set(state);
    }

    pub(crate) fn begin(
        &self,
        stage: RunStage,
        source: &SourceId,
        order: RecursiveFrontierOrder,
    ) -> Option<u64> {
        let mut state = self.state.get();
        if state.active || self.binding.get().is_some() || state.inconsistent || state.overflowed {
            state.inconsistent = true;
            self.state.set(state);
            return None;
        }
        let Some(ordinal) = state.programs.checked_add(1) else {
            state.overflowed = true;
            self.state.set(state);
            return None;
        };
        let source = CapturedSource::copy(source);
        state.capture_unavailable |= source.is_none();
        state.programs = ordinal;
        state.active = true;
        self.topology.begin(
            ordinal,
            order == RecursiveFrontierOrder::BreadthFirstSharedCallPorts,
        );
        self.binding.set(Some(Binding {
            ordinal,
            stage,
            source,
            order,
            decision: None,
        }));
        self.state.set(state);
        Some(ordinal)
    }

    pub(crate) fn decision(&self, path: &str) {
        let mut state = self.state.get();
        if let Some(mut binding) = self.binding.get() {
            let unchanged = binding
                .decision
                .is_some_and(|prior| prior.as_str().is_ok_and(|previous| previous == path));
            self.topology.decision(unchanged);
            binding.decision = CapturedText::copy(path);
            state.capture_unavailable |= binding.decision.is_none();
            self.binding.set(Some(binding));
        } else {
            state.capture_unavailable = true;
        }
        self.state.set(state);
    }

    pub(crate) fn finish(&self, ordinal: u64, work: Snapshot) {
        let mut state = self.state.get();
        let Some(binding) = self.binding.take() else {
            state.inconsistent = true;
            self.state.set(state);
            return;
        };
        if !state.active || binding.ordinal != ordinal {
            state.inconsistent = true;
        }
        state.active = false;
        state.inconsistent |= !work.ended || !work.balanced || work.inconsistent;
        state.overflowed |= work.overflowed;
        let result = ProgramSnapshot {
            ordinal,
            stage: binding.stage,
            source: binding.source,
            order: binding.order,
            decision: binding.decision,
            work,
        };
        if !work.completed_result && state.first_failed.is_none() {
            state.first_failed = Some(result);
        }
        state.last = Some(result);
        self.state.set(state);
        self.topology.finish(ordinal);
    }
}

// Portable fixed logical diagnostic bound only; not allocator or RSS telemetry.
const _: () = assert!(std::mem::size_of::<CompilerPoolDiagnostic>() <= 12288);
