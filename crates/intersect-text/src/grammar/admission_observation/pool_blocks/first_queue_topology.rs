//! Bounded native observation only; never used as a topology/semantic proof.
use std::cell::Cell;

const SOURCE_STATES: usize = 4096;
const WORDS: usize = SOURCE_STATES * 2 / 64;
const _: () = assert!(WORDS * std::mem::size_of::<u64>() == 1024);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Coverage {
    #[default]
    Disabled,
    NotReached,
    FirstQueue,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    UnsupportedPolicy,
    SourceStateBound,
    MultipleQueues,
    ScopeWithoutFirstQueue,
    ChangedGraphLength,
    KeyOutsideGraph,
    ProgramChanged,
    DecisionChanged,
    OutsideProgram,
    CounterOverflow,
}

/// Counts actual observations, not avoided units, saved work, or a proof cache.
/// The source/stage/order/decision binding is the matching program in the
/// existing CompilerPoolDiagnostic snapshot. A different program resets ALL
/// bits; stale observations invalidate coverage rather than becoming a match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub enabled: bool,
    pub program_ordinal: Option<u64>,
    pub coverage: Coverage,
    pub unavailable: Option<Unavailable>,
    pub graph_states: Option<usize>,
    pub queue_scopes: u64,
    pub walks_started: u64,
    pub pure_key_expansions: u64,
    pub earlier_completed_interior_reexpansions: u64,
    pub actual_pure_edge_reads: u64,
    pub edge_reads_from_earlier_completed_keys: u64,
    pub distinct_completed_interior_keys: u64,
    pub window_ended: bool,
}

pub(super) struct Observation {
    state: Cell<Snapshot>,
    active: Cell<bool>,
    // Exactly 1024 bytes. This is caller-owned diagnostic retention outside the
    // semantic work pool. No graph/kernel/configuration gains a field or cache.
    completed: [Cell<u64>; WORDS],
}

impl Observation {
    pub(super) fn new() -> Self {
        Self {
            state: Cell::new(Snapshot::default()),
            active: Cell::new(false),
            completed: std::array::from_fn(|_| Cell::new(0)),
        }
    }

    pub(super) fn enable(&self, unused: bool) {
        if !unused {
            self.unavailable(Unavailable::OutsideProgram);
            return;
        }
        self.state.set(Snapshot {
            enabled: true,
            coverage: Coverage::NotReached,
            ..Snapshot::default()
        });
    }

    pub(super) fn enabled(&self) -> bool {
        self.state.get().enabled
    }

    pub(super) fn snapshot(&self) -> Snapshot {
        self.state.get()
    }

    pub(super) fn begin(&self, ordinal: u64, selected_policy: bool) {
        if !self.enabled() {
            return;
        }
        for word in &self.completed {
            word.set(0);
        }
        self.state.set(Snapshot {
            enabled: true,
            program_ordinal: Some(ordinal),
            coverage: Coverage::NotReached,
            ..Snapshot::default()
        });
        self.active.set(true);
        if !selected_policy {
            self.unavailable(Unavailable::UnsupportedPolicy);
        }
    }

    // A queue observation cannot silently inherit a later compiler decision.
    // Program begin resets bits; changing its decision after any observed queue
    // makes this whole bounded window explicitly unavailable instead of mixing.
    pub(super) fn decision(&self, unchanged: bool) {
        if self.enabled() && self.state.get().queue_scopes != 0 && !unchanged {
            self.unavailable(Unavailable::DecisionChanged);
        }
    }

    pub(super) fn finish(&self, ordinal: u64) {
        if !self.enabled() {
            return;
        }
        if self.state.get().program_ordinal != Some(ordinal) || !self.active.get() {
            self.unavailable(Unavailable::ProgramChanged);
        }
        let mut state = self.state.get();
        state.window_ended = true;
        self.state.set(state);
        self.active.set(false);
    }

    pub(super) fn unavailable(&self, reason: Unavailable) {
        let mut state = self.state.get();
        if state.enabled {
            state.coverage = Coverage::Unavailable;
            if state.unavailable.is_none() {
                state.unavailable = Some(reason);
            }
            self.state.set(state);
        }
    }

    fn increment(&self, field: impl FnOnce(&mut Snapshot) -> &mut u64) {
        let mut state = self.state.get();
        let slot = field(&mut state);
        if let Some(next) = slot.checked_add(1) {
            *slot = next;
            self.state.set(state);
        } else {
            self.unavailable(Unavailable::CounterOverflow);
        }
    }

    fn scope(&self, generation: &Cell<Option<u64>>, semantic_scope_active: bool) -> bool {
        let state = self.state.get();
        if !state.enabled || state.coverage == Coverage::Unavailable {
            return false;
        }
        if !self.active.get() || !semantic_scope_active {
            self.unavailable(Unavailable::OutsideProgram);
            return false;
        }
        if let Some(earlier) = generation.get() {
            if Some(earlier) != state.program_ordinal {
                self.unavailable(Unavailable::ProgramChanged);
                return false;
            }
        } else {
            generation.set(state.program_ordinal);
        }
        state.program_ordinal.is_some()
    }

    pub(super) fn queue(
        &self,
        generation: &Cell<Option<u64>>,
        semantic_scope_active: bool,
        fresh_queue: bool,
        graph_states: usize,
    ) {
        if !self.scope(generation, semantic_scope_active) {
            return;
        }
        let mut state = self.state.get();
        if fresh_queue {
            self.increment(|row| &mut row.queue_scopes);
            state = self.state.get();
            if state.queue_scopes != 1 {
                self.unavailable(Unavailable::MultipleQueues);
                return;
            }
            if graph_states > SOURCE_STATES {
                self.unavailable(Unavailable::SourceStateBound);
                return;
            }
            state.graph_states = Some(graph_states);
            state.coverage = Coverage::FirstQueue;
            self.state.set(state);
        } else if state.queue_scopes != 1 {
            self.unavailable(Unavailable::ScopeWithoutFirstQueue);
        } else if state.graph_states != Some(graph_states) {
            self.unavailable(Unavailable::ChangedGraphLength);
        }
    }

    pub(super) fn walk(&self, generation: &Cell<Option<u64>>, semantic_scope_active: bool) {
        if self.scope(generation, semantic_scope_active)
            && self.state.get().coverage == Coverage::FirstQueue
        {
            self.increment(|row| &mut row.walks_started);
        }
    }

    fn key(
        &self,
        generation: &Cell<Option<u64>>,
        semantic_scope_active: bool,
        state: usize,
        action_seen: bool,
    ) -> Option<(usize, u64)> {
        if !self.scope(generation, semantic_scope_active) {
            return None;
        }
        let report = self.state.get();
        if report.coverage != Coverage::FirstQueue {
            self.unavailable(Unavailable::ScopeWithoutFirstQueue);
            return None;
        }
        if report.graph_states.is_none_or(|length| state >= length) || state >= SOURCE_STATES {
            self.unavailable(Unavailable::KeyOutsideGraph);
            return None;
        }
        let key = state * 2 + usize::from(action_seen);
        Some((key / 64, 1u64 << (key % 64)))
    }

    pub(super) fn expansion(
        &self,
        generation: &Cell<Option<u64>>,
        semantic_scope_active: bool,
        state: usize,
        action_seen: bool,
    ) {
        if let Some((word, bit)) = self.key(generation, semantic_scope_active, state, action_seen) {
            self.increment(|row| &mut row.pure_key_expansions);
            if self.completed[word].get() & bit != 0 {
                self.increment(|row| &mut row.earlier_completed_interior_reexpansions);
            }
        }
    }

    pub(super) fn edge(
        &self,
        generation: &Cell<Option<u64>>,
        semantic_scope_active: bool,
        state: usize,
        action_seen: bool,
    ) {
        if let Some((word, bit)) = self.key(generation, semantic_scope_active, state, action_seen) {
            self.increment(|row| &mut row.actual_pure_edge_reads);
            if self.completed[word].get() & bit != 0 {
                self.increment(|row| &mut row.edge_reads_from_earlier_completed_keys);
            }
        }
    }

    pub(super) fn completed_interior(
        &self,
        generation: &Cell<Option<u64>>,
        semantic_scope_active: bool,
        state: usize,
        action_seen: bool,
    ) {
        if let Some((word, bit)) = self.key(generation, semantic_scope_active, state, action_seen)
            && self.completed[word].get() & bit == 0
        {
            self.completed[word].set(self.completed[word].get() | bit);
            self.increment(|row| &mut row.distinct_completed_interior_keys);
        }
    }
}

pub(super) struct Handle<'a> {
    pub(super) owner: &'a Observation,
    pub(super) generation: Cell<Option<u64>>,
}

const _: () = assert!(std::mem::size_of::<Observation>() <= 1280);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn topology_counter_invalid_context_bounds_and_overflow_are_explicit() {
        let observer = Observation::new();
        observer.enable(true);
        observer.begin(1, true);
        let generation = Cell::new(None);
        observer.queue(&generation, true, true, 8);
        observer.expansion(&generation, true, 2, false);
        observer.expansion(&generation, true, 2, false);
        observer.edge(&generation, true, 2, false);
        assert_eq!(
            observer.snapshot().earlier_completed_interior_reexpansions,
            0
        );
        assert_eq!(
            observer.snapshot().edge_reads_from_earlier_completed_keys,
            0
        );
        observer.completed_interior(&generation, true, 2, false);
        observer.expansion(&generation, true, 2, true);
        assert_eq!(
            observer.snapshot().earlier_completed_interior_reexpansions,
            0
        );
        observer.expansion(&generation, true, 2, false);
        assert_eq!(
            observer.snapshot().earlier_completed_interior_reexpansions,
            1
        );
        observer.finish(1);
        observer.begin(2, true);
        observer.expansion(&generation, true, 2, false);
        assert_eq!(
            observer.snapshot().unavailable,
            Some(Unavailable::ProgramChanged)
        );
        assert_eq!(
            observer.snapshot().earlier_completed_interior_reexpansions,
            0
        );
        observer.finish(2);
        observer.begin(3, true);
        let generation = Cell::new(None);
        observer.queue(&generation, true, true, 8);
        observer.expansion(&generation, true, 8, false);
        assert_eq!(
            observer.snapshot().unavailable,
            Some(Unavailable::KeyOutsideGraph)
        );
        observer.finish(3);
        observer.begin(4, true);
        let generation = Cell::new(None);
        observer.queue(&generation, true, true, 8);
        let mut row = observer.state.get();
        row.pure_key_expansions = u64::MAX;
        observer.state.set(row);
        observer.expansion(&generation, true, 2, false);
        assert_eq!(
            observer.snapshot().unavailable,
            Some(Unavailable::CounterOverflow)
        );
        observer.finish(4);
        observer.begin(5, false);
        assert_eq!(
            observer.snapshot().unavailable,
            Some(Unavailable::UnsupportedPolicy)
        );
        observer.finish(5);
        observer.begin(6, true);
        let generation = Cell::new(None);
        observer.queue(&generation, false, true, 8);
        assert_eq!(
            observer.snapshot().unavailable,
            Some(Unavailable::OutsideProgram)
        );
        observer.finish(6);
        observer.begin(7, true);
        let generation = Cell::new(None);
        observer.queue(&generation, true, false, 8);
        assert_eq!(
            observer.snapshot().unavailable,
            Some(Unavailable::ScopeWithoutFirstQueue)
        );
        observer.finish(7);
        observer.begin(8, true);
        let generation = Cell::new(None);
        observer.queue(&generation, true, true, 8);
        observer.queue(&generation, true, false, 9);
        assert_eq!(
            observer.snapshot().unavailable,
            Some(Unavailable::ChangedGraphLength)
        );
        observer.finish(8);
        observer.begin(9, true);
        let generation = Cell::new(None);
        observer.queue(&generation, true, true, SOURCE_STATES);
        observer.completed_interior(&generation, true, SOURCE_STATES - 1, true);
        observer.expansion(&generation, true, SOURCE_STATES - 1, true);
        assert_eq!(observer.snapshot().coverage, Coverage::FirstQueue);
        assert_eq!(observer.snapshot().distinct_completed_interior_keys, 1);
        assert_eq!(
            observer.snapshot().earlier_completed_interior_reexpansions,
            1
        );
        observer.finish(9);
        observer.begin(10, true);
        assert_eq!(observer.snapshot().coverage, Coverage::NotReached);
        assert_eq!(observer.snapshot().graph_states, None);
        assert_eq!(observer.snapshot().distinct_completed_interior_keys, 0);
    }
}
