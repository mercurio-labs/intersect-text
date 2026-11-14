//! Private, per-closure normalization of actual immutable internal source edges.
//! No runtime graph, source certificate, recursion Boolean or caller is cached.
use super::adjacent::{AdjacentReplay, ClosureQueue, GeometricAdjacentQueue, share_adjacent};
use super::frontier::ClosureKeys;
use super::{
    AdjacentWorkObservation, AnalysisConfiguration, Edge, RecursiveAnalysisCounters, State,
    analysis_charge,
};
use std::collections::{BTreeMap, BTreeSet};

struct Summaries<'graph> {
    graph: &'graph [State],
    // None denotes a completed structural cycle, never semantic nonconflict.
    completed: BTreeMap<usize, Option<usize>>,
    admitted: bool,
}
impl<'graph> Summaries<'graph> {
    fn new(graph: &'graph [State]) -> Self {
        Self {
            graph,
            completed: BTreeMap::new(),
            admitted: false,
        }
    }
    fn target(&mut self, start: usize, remaining: &mut usize) -> Result<usize, String> {
        if !self.admitted {
            // New invocation-local fixed header admission; no heap allocation
            // or cache query precedes it. Unit bounds are logical, not RSS.
            analysis_charge(remaining)?;
            self.admitted = true;
        }
        analysis_charge(remaining)?; // Actual completed-key lookup, hit or miss.
        if let Some(target) = self.completed.get(&start) {
            return Ok(target.unwrap_or(start));
        }
        let mut seen = BTreeSet::new();
        let mut at = start;
        let completed = loop {
            // Each local path lookup/new usize entry is prepaid before insert.
            // Original model allocation atomicity remains unchanged; this is
            // not a claim to bound allocator bytes or total native CPU.
            analysis_charge(remaining)?;
            if !seen.insert(at) {
                break None;
            }
            analysis_charge(remaining)?; // Graph/edge inspection before read.
            let node = self
                .graph
                .get(at)
                .ok_or("Dangling parser prediction state")?;
            if node.rule_stop || node.accepting {
                break Some(at);
            }
            match node.edges.as_slice() {
                [Edge::Epsilon(target) | Edge::Action(target)] => at = *target,
                _ => break Some(at),
            }
        };
        // Publication only follows a completed acyclic walk or an actual
        // detected cycle. Failure/denial is never cached or refunded.
        analysis_charge(remaining)?;
        self.completed.insert(start, completed);
        Ok(completed.unwrap_or(start))
    }
}

/// New transient type only. The existing queue and all old holders stay exact.
/// An owned completed Token frontier retains its existing SeededQueue fast
/// path; this optimization applies to the ordinary closure queue only.
pub(super) struct ChainQueue<'graph> {
    inner: GeometricAdjacentQueue<'graph>,
    summaries: Summaries<'graph>,
}
impl<'graph> ClosureQueue<'graph> for ChainQueue<'graph> {
    fn empty(graph: &'graph [State]) -> Self {
        Self {
            inner: GeometricAdjacentQueue::empty(graph),
            summaries: Summaries::new(graph),
        }
    }
    fn unreserved(_: BTreeSet<AnalysisConfiguration>, _: &'graph [State]) -> Result<Self, String> {
        Err("Transparent chains require exact-key reservations".into())
    }
    fn internal_target(&mut self, target: usize, remaining: &mut usize) -> Result<usize, String> {
        self.summaries.target(target, remaining)
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        self.inner.reserve_placement_observed(row, remaining, exact)
    }
    fn observed_shape(&self) -> (usize, usize) {
        self.inner.observed_shape()
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration) -> Result<(), String> {
        self.inner.push_reserved(row)
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration> {
        self.inner.pop_front()
    }
    fn share_transparent_observed(
        &mut self,
        row: AnalysisConfiguration,
        keys: &mut BTreeSet<AnalysisConfiguration>,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration>, String> {
        self.share_keys_observed(row, keys, remaining, measured, exact)
    }
    fn share_keys_observed<K: ClosureKeys>(
        &mut self,
        row: AnalysisConfiguration,
        keys: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration>, String> {
        share_adjacent(self, row, keys, remaining, (measured, exact))
    }
}
impl<'graph> AdjacentReplay<'graph> for ChainQueue<'graph> {
    fn replay_group(&self) -> Option<(usize, usize)> {
        self.inner.replay_group()
    }
    fn clear_replay_group(&mut self) {
        self.inner.clear_replay_group();
    }
    fn replay_graph(&self) -> &'graph [State] {
        self.summaries.graph
    }
    fn mark_nontransparent(&mut self, state: usize, siblings: usize) -> Result<(), String> {
        self.inner.mark_nontransparent(state, siblings)
    }
}

#[cfg(test)]
#[path = "transparent_chain_tests.rs"]
mod tests;
