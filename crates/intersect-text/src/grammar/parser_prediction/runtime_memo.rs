// Copyright (c) 2026 Mercurio Labs
// SPDX-License-Identifier: MIT
//! Handwritten runtime prediction memo (not generated): ANTLR-DFA-like reuse
//! of live runtime decisions across the sources one bound language parses.
//!
//! A live decision's selection is a deterministic function of the immutable
//! grammar and admission, the decision, its actual caller returns, the
//! prediction symbols it reads, and the answers it receives from the input's
//! retained nested-decision and viability memos. The memo records, per
//! (decision, caller returns, route), a trie over the exact symbols an
//! uncached prediction read (keywords and terminal rules as distinct
//! symbols, end of input, and reads past it), ending in an entry with the
//! selection, the work the prediction charged, and every answer it took from
//! the input's memos rather than computing from its own reads. A later
//! decision takes the entry only when it reads the same symbols and the
//! input's memos give the same answers at the same symbol positions, so the
//! selection is the one the uncached prediction would make. A hit charges the
//! recorded work; the probe reads use a scratch pool and are never charged.
//!
//! Errors, selections that depend on more than [`MAX_PATH`] symbols, and a
//! failed `choice` (no viable alternative, whose diagnostics depend on how far
//! the uncached prediction read) are never retained. The memo stops growing at
//! its byte bound and then only serves the entries it holds.
use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Symbols a retained selection may depend on.
const MAX_PATH: usize = 128;
/// Memo answers a retained selection may depend on.
const MAX_REQUIREMENTS: usize = 32;
/// Default bound of the estimated retained bytes.
const DEFAULT_BYTES: usize = 128 * 1024 * 1024;
const NODE_BYTES: usize = 64;
const ENTRY_BYTES: usize = 64;

/// Whether a runtime prediction memo is used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimePredictionMemoMode {
    /// Every decision runs the uncached prediction.
    Off,
    /// Decisions reuse retained selections.
    #[default]
    On,
    /// Every retained selection a decision would reuse is also predicted
    /// uncached; a different selection is an error. For equivalence checks.
    Verify,
}

/// Counters of one memo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimePredictionMemoStats {
    /// Decisions that took a retained selection.
    pub hits: usize,
    /// Decisions that ran the uncached prediction with the memo enabled.
    pub misses: usize,
    /// Selections retained.
    pub inserted: usize,
    /// Uncached selections that could not be retained (too long, too many
    /// memo answers, or a failed choice).
    pub uncacheable: usize,
    /// Selections not retained because the memo reached its bound.
    pub over_capacity: usize,
    /// Hits whose uncached prediction was also run and agreed (Verify).
    pub verified: usize,
    /// Hits whose recorded work exceeded the remaining pool, so the uncached
    /// prediction ran instead.
    pub budget_fallbacks: usize,
    /// Trie nodes and entries retained.
    pub nodes: usize,
    pub entries: usize,
    /// Estimated retained bytes.
    pub bytes: usize,
}

/// One input-memo answer a retained selection depends on, at the position
/// after `ordinal` symbols of its lookahead.
#[derive(Clone)]
enum Requirement {
    Viability {
        ordinal: usize,
        key: ViabilityKey,
        accepted: bool,
    },
    Nested {
        ordinal: usize,
        key: NestedDecisionKey,
        selected: Option<usize>,
    },
}

struct Entry {
    requirements: Box<[Requirement]>,
    selected: Option<usize>,
    work: usize,
}

#[derive(Default)]
struct Node {
    children: BTreeMap<Option<PredictionSymbol>, u32>,
    entries: Vec<Entry>,
}

/// Caller context of a decision: its actual caller return states and the
/// prediction route (return storage, live callers, prepared facts).
type Context = (Vec<usize>, u8);

#[derive(Default)]
struct State {
    mode: RuntimePredictionMemoMode,
    capacity: usize,
    /// Addresses of the grammar and admission this memo serves; set by the
    /// first decision. Another grammar or admission bypasses the memo.
    owner: Option<(usize, usize)>,
    roots: BTreeMap<String, BTreeMap<Context, u32>>,
    nodes: Vec<Node>,
    stats: RuntimePredictionMemoStats,
}

/// Retained runtime decision selections of one bound language, shared by the
/// sources it parses. Interior mutability lets a shared language own it.
pub struct RuntimePredictionMemo {
    state: Mutex<State>,
}

impl Default for RuntimePredictionMemo {
    fn default() -> Self {
        Self::new(RuntimePredictionMemoMode::On)
    }
}

impl RuntimePredictionMemo {
    pub fn new(mode: RuntimePredictionMemoMode) -> Self {
        Self::with_capacity(mode, DEFAULT_BYTES)
    }
    /// A memo that stops retaining selections at about `bytes` bytes.
    pub fn with_capacity(mode: RuntimePredictionMemoMode, bytes: usize) -> Self {
        Self {
            state: Mutex::new(State {
                mode,
                capacity: bytes,
                ..State::default()
            }),
        }
    }
    pub fn mode(&self) -> RuntimePredictionMemoMode {
        self.state
            .lock()
            .map_or(RuntimePredictionMemoMode::Off, |state| state.mode)
    }
    /// Change the mode; retained selections are kept.
    pub fn set_mode(&self, mode: RuntimePredictionMemoMode) {
        if let Ok(mut state) = self.state.lock() {
            state.mode = mode;
        }
    }
    /// Drop every retained selection and reset the counters.
    pub fn clear(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.roots.clear();
            state.nodes.clear();
            state.stats = RuntimePredictionMemoStats::default();
        }
    }
    pub fn stats(&self) -> RuntimePredictionMemoStats {
        self.state
            .lock()
            .map(|state| state.stats)
            .unwrap_or_default()
    }
}

/// The decision a memo lookup is for.
pub(super) struct Decision<'k> {
    pub(super) owner: (usize, usize),
    pub(super) key: &'k str,
    pub(super) returns: &'k [usize],
    pub(super) route: u8,
    /// Whether a failed selection (`None`) may be retained.
    pub(super) retain_none: bool,
}

/// Input-memo answers a recording prediction took without producing them.
#[derive(Default)]
struct Notes {
    viability: Vec<(ViabilityKey, bool)>,
    nested: Vec<(NestedDecisionKey, Option<usize>)>,
    /// The record cannot describe the dependency exactly.
    unusable: bool,
}

/// The input adapter an uncached live decision runs on. It forwards every
/// call; when recording it notes the symbols read and the input-memo answers
/// the prediction did not produce itself.
pub(super) struct Recorder<'a, I> {
    inner: &'a mut I,
    recording: bool,
    /// Read steps: position before -> (position after, symbol).
    steps: BTreeMap<usize, (usize, Option<PredictionSymbol>)>,
    own_viability: HashSet<ViabilityKey>,
    own_nested: HashSet<NestedDecisionKey>,
    /// The answer queries take `&self`.
    notes: std::cell::RefCell<Notes>,
}

impl<'a, I: PredictionInput> Recorder<'a, I> {
    fn new(inner: &'a mut I, recording: bool) -> Self {
        Self {
            inner,
            recording,
            steps: BTreeMap::new(),
            own_viability: HashSet::new(),
            own_nested: HashSet::new(),
            notes: std::cell::RefCell::new(Notes::default()),
        }
    }
    fn note(&self, add: impl FnOnce(&mut Notes)) {
        match self.notes.try_borrow_mut() {
            Ok(mut notes) => add(&mut notes),
            // Unreachable without reentrant queries; the record is dropped.
            Err(_) => {
                if let Ok(mut notes) = self.notes.try_borrow_mut() {
                    notes.unusable = true;
                }
            }
        }
    }
    /// The symbols read from `start` in order, and the input-memo answers
    /// translated to symbol ordinals.
    fn finish(self, start: usize) -> Option<(Vec<Option<PredictionSymbol>>, Vec<Requirement>)> {
        let notes = self.notes.into_inner();
        if notes.unusable || notes.viability.len() + notes.nested.len() > MAX_REQUIREMENTS {
            return None;
        }
        let mut path = Vec::new();
        let mut ordinals = BTreeMap::from([(start, 0usize)]);
        let mut at = start;
        while let Some((after, symbol)) = self.steps.get(&at) {
            path.push(symbol.clone());
            if path.len() > MAX_PATH {
                return None;
            }
            ordinals.entry(*after).or_insert(path.len());
            if symbol.is_none() || *after == at {
                break;
            }
            at = *after;
        }
        // Every read must lie on the one chain from the start.
        if path.len() != self.steps.len() {
            return None;
        }
        let mut requirements = Vec::new();
        for (mut key, accepted) in notes.viability {
            let ordinal = *ordinals.get(&key.position)?;
            key.position = 0;
            requirements.push(Requirement::Viability {
                ordinal,
                key,
                accepted,
            });
        }
        for (mut key, selected) in notes.nested {
            let ordinal = *ordinals.get(&key.start)?;
            key.start = 0;
            requirements.push(Requirement::Nested {
                ordinal,
                key,
                selected,
            });
        }
        Some((path, requirements))
    }
}

impl<I: PredictionInput> PredictionInput for Recorder<'_, I> {
    fn runtime_work_diagnostics(&self) -> bool {
        self.inner.runtime_work_diagnostics()
    }
    fn runtime_work_snapshot(&mut self, phase: &'static str, remaining: Option<usize>) {
        self.inner.runtime_work_snapshot(phase, remaining);
    }
    fn runtime_work_geometry(&mut self, event: RuntimeWorkGeometry) {
        self.inner.runtime_work_geometry(event);
    }
    fn replay_position(&self) -> Option<usize> {
        self.inner.replay_position()
    }
    fn lookahead_mark(&self) -> Option<usize> {
        self.inner.lookahead_mark()
    }
    fn restore_lookahead(&mut self, mark: usize) {
        self.inner.restore_lookahead(mark);
    }
    fn nested_decision_outcome(&self, key: &NestedDecisionKey) -> Option<Option<usize>> {
        let answer = self.inner.nested_decision_outcome(key);
        if self.recording
            && let Some(selected) = answer
            && !self.own_nested.contains(key)
        {
            self.note(|notes| notes.nested.push((key.clone(), selected)));
        }
        answer
    }
    fn record_nested_decision_outcome(&mut self, key: NestedDecisionKey, selected: Option<usize>) {
        if self.recording {
            self.own_nested.insert(key.clone());
        }
        self.inner.record_nested_decision_outcome(key, selected);
    }
    fn viability(&self, key: &ViabilityKey) -> Option<bool> {
        let answer = self.inner.viability(key);
        if self.recording
            && let Some(accepted) = answer
            && !self.own_viability.contains(key)
        {
            self.note(|notes| notes.viability.push((key.clone(), accepted)));
        }
        answer
    }
    fn record_viability(&mut self, keys: Vec<ViabilityKey>, accepted: bool) {
        if self.recording {
            self.own_viability.extend(keys.iter().cloned());
        }
        self.inner.record_viability(keys, accepted);
    }
    fn begin_nested_deferral(&mut self) -> bool {
        self.inner.begin_nested_deferral()
    }
    fn end_nested_deferral(&mut self) {
        self.inner.end_nested_deferral();
    }
    fn defer_nested_decision(&mut self, deferred: DeferredNestedDecision) -> bool {
        self.inner.defer_nested_decision(deferred)
    }
    fn take_deferred_nested_decisions(&mut self) -> Vec<DeferredNestedDecision> {
        self.inner.take_deferred_nested_decisions()
    }
    fn read(&mut self, remaining: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
        if !self.recording {
            return self.inner.read(remaining);
        }
        let before = self.inner.replay_position();
        let symbol = self.inner.read(remaining)?;
        let usable = match (before, self.inner.replay_position()) {
            (Some(before), Some(after)) => {
                let step = (after, symbol.clone());
                match self.steps.get(&before) {
                    Some(previous) => *previous == step,
                    None => {
                        self.steps.insert(before, step);
                        true
                    }
                }
            }
            _ => false,
        };
        if !usable {
            self.note(|notes| notes.unusable = true);
        }
        Ok(symbol)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        self.inner.checkpoint()
    }
    fn rewind(&mut self, checkpoint: usize) -> Result<(), String> {
        self.inner.rewind(checkpoint)
    }
}

fn satisfied(
    requirements: &[Requirement],
    positions: &[usize],
    input: &impl PredictionInput,
) -> bool {
    requirements.iter().all(|requirement| match requirement {
        Requirement::Viability {
            ordinal,
            key,
            accepted,
        } => positions.get(*ordinal).is_some_and(|&position| {
            let mut key = key.clone();
            key.position = position;
            input.viability(&key) == Some(*accepted)
        }),
        Requirement::Nested {
            ordinal,
            key,
            selected,
        } => positions.get(*ordinal).is_some_and(|&position| {
            let mut key = key.clone();
            key.start = position;
            input.nested_decision_outcome(&key) == Some(*selected)
        }),
    })
}

fn requirement_bytes(requirements: &[Requirement]) -> usize {
    requirements
        .iter()
        .map(|requirement| match requirement {
            Requirement::Viability { key, .. } => {
                64 + key
                    .rows
                    .iter()
                    .map(|row| 48 + row.1.len() * 8)
                    .sum::<usize>()
            }
            Requirement::Nested { key, .. } => 96 + key.returns.len() * 8,
        })
        .sum()
}

fn symbol_bytes(symbol: &Option<PredictionSymbol>) -> usize {
    match symbol {
        Some(PredictionSymbol::Keyword(text) | PredictionSymbol::Terminal(text)) => text.len(),
        _ => 0,
    }
}

impl State {
    fn bytes_left(&self) -> usize {
        self.capacity.saturating_sub(self.stats.bytes)
    }
    /// The root node of a decision context, if retained.
    fn root(&self, decision: &Decision<'_>) -> Option<u32> {
        self.roots
            .get(decision.key)?
            .get(&(decision.returns.to_vec(), decision.route))
            .copied()
    }
    fn new_node(&mut self) -> Result<u32, String> {
        let index = u32::try_from(self.nodes.len()).map_err(|_| "Prediction memo is full")?;
        self.nodes.push(Node::default());
        self.stats.nodes += 1;
        self.stats.bytes += NODE_BYTES;
        Ok(index)
    }
    /// Retain one selection; false when the memo is at its bound.
    fn insert(
        &mut self,
        decision: &Decision<'_>,
        path: Vec<Option<PredictionSymbol>>,
        requirements: Vec<Requirement>,
        selected: Option<usize>,
        work: usize,
    ) -> Result<bool, String> {
        let returns_bytes = decision.key.len() + decision.returns.len() * 8 + 64;
        let needed = path.len() * NODE_BYTES
            + path.iter().map(symbol_bytes).sum::<usize>()
            + ENTRY_BYTES
            + requirement_bytes(&requirements)
            + returns_bytes;
        if needed > self.bytes_left() {
            return Ok(false);
        }
        let root = match self.root(decision) {
            Some(root) => root,
            None => {
                let root = self.new_node()?;
                self.stats.bytes += returns_bytes;
                self.roots
                    .entry(decision.key.to_string())
                    .or_default()
                    .insert((decision.returns.to_vec(), decision.route), root);
                root
            }
        };
        let mut at = root;
        for symbol in path {
            let next = self
                .nodes
                .get(at as usize)
                .ok_or("Dangling prediction memo node")?
                .children
                .get(&symbol)
                .copied();
            at = match next {
                Some(next) => next,
                None => {
                    let next = self.new_node()?;
                    self.stats.bytes += symbol_bytes(&symbol);
                    self.nodes
                        .get_mut(at as usize)
                        .ok_or("Dangling prediction memo node")?
                        .children
                        .insert(symbol, next);
                    next
                }
            };
        }
        let bytes = ENTRY_BYTES + requirement_bytes(&requirements);
        let node = self
            .nodes
            .get_mut(at as usize)
            .ok_or("Dangling prediction memo node")?;
        node.entries.push(Entry {
            requirements: requirements.into_boxed_slice(),
            selected,
            work,
        });
        self.stats.entries += 1;
        self.stats.bytes += bytes;
        Ok(true)
    }
}

/// A retained selection found for the input: the selection and recorded work.
struct Hit {
    selected: Option<usize>,
    work: usize,
}

/// Probe the retained selections of `decision` against the input from its
/// current position. Reads charge a scratch pool. The input is left after
/// the hit's symbols, or anywhere on a miss.
fn lookup<I: PredictionInput>(
    state: &State,
    decision: &Decision<'_>,
    input: &mut I,
    start: usize,
) -> Option<Hit> {
    let mut at = state.root(decision)?;
    let mut positions = vec![start];
    let mut scratch = Some(usize::MAX);
    loop {
        let node = state.nodes.get(at as usize)?;
        if let Some(entry) = node
            .entries
            .iter()
            .find(|entry| satisfied(&entry.requirements, &positions, input))
        {
            return Some(Hit {
                selected: entry.selected,
                work: entry.work,
            });
        }
        if node.children.is_empty() || positions.len() > MAX_PATH {
            return None;
        }
        let symbol = input.read(&mut scratch).ok()?;
        positions.push(input.replay_position()?);
        at = *node.children.get(&symbol)?;
    }
}

/// Run one live decision through the memo. `run` is the uncached
/// prediction; it is called at most once on a hit (Verify) or a miss.
pub(super) fn memoized<I: PredictionInput>(
    memo: &RuntimePredictionMemo,
    decision: Decision<'_>,
    input: &mut I,
    work: &mut Option<usize>,
    mut run: impl FnMut(&mut Recorder<'_, I>, &mut Option<usize>) -> Result<Option<usize>, String>,
) -> Result<Option<usize>, String> {
    let mode = memo.mode();
    let probe = match (input.replay_position(), input.lookahead_mark()) {
        (Some(start), Some(mark))
            if mode != RuntimePredictionMemoMode::Off && !input.runtime_work_diagnostics() =>
        {
            Some((start, mark))
        }
        _ => None,
    };
    let Some((start, mark)) = probe else {
        let mut recorder = Recorder::new(input, false);
        return run(&mut recorder, work);
    };
    let hit = {
        let mut state = memo
            .state
            .lock()
            .map_err(|_| "Prediction memo is poisoned")?;
        match state.owner {
            Some(owner) if owner != decision.owner => {
                drop(state);
                let mut recorder = Recorder::new(input, false);
                return run(&mut recorder, work);
            }
            Some(_) => {}
            None => state.owner = Some(decision.owner),
        }
        lookup(&state, &decision, input, start)
    };
    if let Some(hit) = &hit
        && mode == RuntimePredictionMemoMode::On
    {
        if let Some(left) = work.as_mut()
            && *left >= hit.work
        {
            *left -= hit.work;
            if let Ok(mut state) = memo.state.lock() {
                state.stats.hits += 1;
            }
            return Ok(hit.selected);
        }
        if let Ok(mut state) = memo.state.lock() {
            state.stats.budget_fallbacks += 1;
        }
    }
    // Uncached prediction from the original position and lookahead.
    input.rewind(start)?;
    input.restore_lookahead(mark);
    let before = *work;
    let mut recorder = Recorder::new(input, hit.is_none());
    let result = run(&mut recorder, work);
    let mut state = memo
        .state
        .lock()
        .map_err(|_| "Prediction memo is poisoned")?;
    if let Some(hit) = hit {
        return match result {
            Ok(selected) if selected == hit.selected => {
                state.stats.verified += 1;
                Ok(selected)
            }
            Ok(selected) => Err(format!(
                "Runtime prediction memo selected {:?} for {} where the uncached prediction selected {selected:?}",
                hit.selected, decision.key
            )),
            Err(error) => Err(format!(
                "Runtime prediction memo selected {:?} for {} where the uncached prediction failed: {error}",
                hit.selected, decision.key
            )),
        };
    }
    state.stats.misses += 1;
    let Ok(selected) = result else {
        return result;
    };
    let charged = match (before, *work) {
        (Some(before), Some(after)) => before.checked_sub(after),
        _ => None,
    };
    match (charged, recorder.finish(start)) {
        (Some(charged), Some((path, requirements)))
            if selected.is_some() || decision.retain_none =>
        {
            if state.insert(&decision, path, requirements, selected, charged)? {
                state.stats.inserted += 1;
            } else {
                state.stats.over_capacity += 1;
            }
        }
        _ => state.stats.uncacheable += 1,
    }
    Ok(selected)
}

/// Most completion scopes one source retains.
const MAX_SCOPES: usize = 4096;

type ScopeKey = (
    usize,
    usize,
    CompletionVerification,
    Option<(usize, usize, usize, usize, usize)>,
);

/// Completion scopes of full syntactic predicates verified by the live
/// decisions of one source. A scope is a fact of the immutable graph: its
/// verification depends only on its start and stop, the verification policy
/// and the rule-local decision, and consumes the same work each time it
/// succeeds. A retained scope is reused when that much work remains and is
/// charged exactly that work, so pools, results and errors are those of the
/// uncached verification; failures are not retained.
#[derive(Default)]
pub(super) struct ScopeCache<'g> {
    scopes: std::cell::RefCell<HashMap<ScopeKey, (std::rc::Rc<CompletionScope<'g>>, usize)>>,
    /// Verify mode: a retained scope is verified again and must agree.
    pub(super) verify: std::cell::Cell<bool>,
}

impl<'g> ScopeCache<'g> {
    pub(super) fn verified(
        &self,
        program: &'g ParserPrediction,
        (start, stop): (usize, usize),
        remaining: &mut usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
    ) -> Result<std::rc::Rc<CompletionScope<'g>>, String> {
        let key = (
            start,
            stop,
            verification,
            local.map(|local| {
                (
                    std::ptr::from_ref(local.decision) as usize,
                    local.entry,
                    local.stop,
                    local.path.as_ptr() as usize,
                    local.path.len(),
                )
            }),
        );
        let retained = self
            .scopes
            .try_borrow()
            .ok()
            .and_then(|scopes| scopes.get(&key).cloned());
        if let Some((scope, work)) = &retained
            && *remaining >= *work
            && !self.verify.get()
        {
            *remaining -= work;
            return Ok(scope.clone());
        }
        let before = *remaining;
        let scope = std::rc::Rc::new(program.verify_completion_scope_with_local_decision(
            start,
            stop,
            remaining,
            verification,
            local,
        )?);
        if let Some((retained, work)) = retained {
            let same = |a: &CompletionScope<'_>, b: &CompletionScope<'_>| {
                a.decisions.len() == b.decisions.len()
                    && a.decisions.iter().zip(&b.decisions).all(|(x, y)| {
                        x.0 == y.0
                            && std::ptr::eq(x.1.decision, y.1.decision)
                            && x.1.path == y.1.path
                            && x.1.recursive_first == y.1.recursive_first
                    })
                    && a.guards == b.guards
            };
            if work != before.saturating_sub(*remaining) || !same(&retained, &scope) {
                return Err(format!(
                    "Retained completion scope {start}..{stop} differs from its verification"
                ));
            }
            return Ok(scope);
        }
        if let Ok(mut scopes) = self.scopes.try_borrow_mut()
            && scopes.len() < MAX_SCOPES
        {
            scopes.insert(key, (scope.clone(), before.saturating_sub(*remaining)));
        }
        Ok(scope)
    }
}
