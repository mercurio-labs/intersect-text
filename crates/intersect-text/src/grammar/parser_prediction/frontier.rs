//! Private ownership of completed, paid Token frontiers. No public authority.
use super::adjacent::{AdjacentReplay, ClosureQueue, GeometricAdjacentQueue, share_adjacent};
use super::analysis_calls::{Calls, VecCalls};
use super::{
    AdjacentWorkObservation, AnalysisConfiguration, PredictionSymbol, RecursiveAnalysisCounters,
    ScheduleSite, State, analysis_charge, schedule_analysis_with_keys_observed,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque, btree_set};

type CallRows<C = VecCalls> = BTreeSet<AnalysisConfiguration<C>>;
type CallTokens<C = VecCalls> = BTreeMap<PredictionSymbol, CallRows<C>>;
// Keep historical concrete test aliases: a constructor such as Rows::new()
// must retain its original Vec type even before any value constrains inference.
#[cfg(test)]
type Rows = CallRows<VecCalls>;
#[cfg(test)]
type Tokens = CallTokens<VecCalls>;

pub(super) trait FrontierPolicy<C: Calls = VecCalls> {
    type Pending;
    fn initial(rows: CallRows<C>, remaining: &mut usize)
    -> Result<VecDeque<Self::Pending>, String>;
    fn take(frontier: Self::Pending) -> (CallRows<C>, bool);
    fn extend(
        pending: &mut VecDeque<Self::Pending>,
        tokens: CallTokens<C>,
        remaining: &mut usize,
    ) -> Result<(), String>;
    fn uniform_state(
        paid_token: bool,
        rows: &CallRows<C>,
        remaining: &mut usize,
    ) -> Result<Option<usize>, String>;
}

pub(super) struct LegacyFrontiers;
impl<C: Calls> FrontierPolicy<C> for LegacyFrontiers {
    type Pending = CallRows<C>;
    fn initial(rows: CallRows<C>, _: &mut usize) -> Result<VecDeque<CallRows<C>>, String> {
        Ok(VecDeque::from([rows]))
    }
    fn take(rows: CallRows<C>) -> (CallRows<C>, bool) {
        (rows, false)
    }
    fn extend(
        pending: &mut VecDeque<CallRows<C>>,
        tokens: CallTokens<C>,
        _: &mut usize,
    ) -> Result<(), String> {
        pending.extend(tokens.into_values());
        Ok(())
    }
    fn uniform_state(_: bool, _: &CallRows<C>, _: &mut usize) -> Result<Option<usize>, String> {
        Ok(None)
    }
}

// This origin can only be attached inside this module. Initial and incomplete
// maps cannot become completed-token frontiers through a public assertion.
enum Origin {
    Initial,
    PaidToken,
}
pub(super) struct PendingFrontier<C = VecCalls> {
    rows: CallRows<C>,
    origin: Origin,
}
pub(super) struct AdoptFrontiers;
impl AdoptFrontiers {
    fn publish<C: Calls>(
        pending: &mut VecDeque<PendingFrontier<C>>,
        rows: CallRows<C>,
        origin: Origin,
        remaining: &mut usize,
    ) -> Result<(), String> {
        // Header/origin retention is new work, including for the initial set.
        analysis_charge(remaining)?;
        if pending.len() == pending.capacity() {
            // An actual pending-buffer reserve is paid before allocation.
            analysis_charge(remaining)?;
            pending
                .try_reserve_exact(1)
                .map_err(|_| "Prediction frontier allocation unavailable".to_string())?;
        }
        pending.push_back(PendingFrontier::<C> { rows, origin });
        Ok(())
    }
}
impl<C: Calls> FrontierPolicy<C> for AdoptFrontiers {
    type Pending = PendingFrontier<C>;
    fn initial(
        rows: CallRows<C>,
        remaining: &mut usize,
    ) -> Result<VecDeque<PendingFrontier<C>>, String> {
        let mut pending = VecDeque::new();
        Self::publish(&mut pending, rows, Origin::Initial, remaining)?;
        Ok(pending)
    }
    fn take(frontier: PendingFrontier<C>) -> (CallRows<C>, bool) {
        (frontier.rows, matches!(frontier.origin, Origin::PaidToken))
    }
    fn extend(
        pending: &mut VecDeque<PendingFrontier<C>>,
        tokens: CallTokens<C>,
        remaining: &mut usize,
    ) -> Result<(), String> {
        // Called only after the SAME worker has successfully completed every
        // charged Token/template copy/publication for this token map.
        for rows in tokens.into_values() {
            Self::publish(pending, rows, Origin::PaidToken, remaining)?;
        }
        Ok(())
    }
    fn uniform_state(
        paid_token: bool,
        rows: &CallRows<C>,
        remaining: &mut usize,
    ) -> Result<Option<usize>, String> {
        if !paid_token {
            return Ok(None);
        }
        analysis_charge(remaining)?;
        let first = rows.first().ok_or("Empty completed Token frontier")?.state;
        analysis_charge(remaining)?;
        let last = rows.last().ok_or("Empty completed Token frontier")?.state;
        // AnalysisConfiguration<C>'s unchanged Ord compares state first. This
        // proves uniformity without an uncharged scan or source-ID shortcut.
        Ok((first == last).then_some(first))
    }
}

pub(super) trait ClosureKeys<C: Calls = VecCalls> {
    fn share<'graph, Q: ClosureQueue<'graph, C>>(
        row: AnalysisConfiguration<C>,
        queue: &mut Q,
        scheduled: &mut Self,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String>
    where
        Self: Sized,
    {
        queue.share_keys_observed(row, scheduled, remaining, measured, exact)
    }
    fn contains_observed(
        &self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
    ) -> Result<bool, String>;
    fn insert_reserved(&mut self, row: AnalysisConfiguration<C>) -> bool;
    fn schedule<'graph, Q: ClosureQueue<'graph, C>>(
        row: AnalysisConfiguration<C>,
        queue: &mut Q,
        scheduled: &mut Self,
        remaining: &mut usize,
        context: (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String>
    where
        Self: Sized,
    {
        schedule_analysis_with_keys_observed(row, queue, scheduled, remaining, context, exact)
    }
}
impl<C: Calls> ClosureKeys<C> for CallRows<C> {
    fn share<'graph, Q: ClosureQueue<'graph, C>>(
        row: AnalysisConfiguration<C>,
        queue: &mut Q,
        scheduled: &mut Self,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String>
    where
        Self: Sized,
    {
        queue.share_transparent_observed(row, scheduled, remaining, measured, exact)
    }
    fn contains_observed(
        &self,
        row: &AnalysisConfiguration<C>,
        _: &mut usize,
    ) -> Result<bool, String> {
        Ok(self.contains(row))
    }
    fn insert_reserved(&mut self, row: AnalysisConfiguration<C>) -> bool {
        self.insert(row)
    }
    fn schedule<'graph, Q: ClosureQueue<'graph, C>>(
        row: AnalysisConfiguration<C>,
        queue: &mut Q,
        scheduled: &mut Self,
        remaining: &mut usize,
        context: (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        // Preserve old trait facades, including VecDeque's original dispatch.
        Q::schedule_observed(row, queue, scheduled, remaining, context, exact)
    }
}

type AdoptedFrontier<'graph, 'frontier, C = VecCalls> =
    (SeededQueue<'graph, C>, SeededKeys<'frontier, C>);

pub(super) fn bind_adoption<'graph, 'frontier, C: Calls>(
    rows: CallRows<C>,
    visited: &'frontier BTreeSet<CallRows<C>>,
    graph: &'graph [State],
    state: usize,
    remaining: &mut usize,
) -> Result<AdoptedFrontier<'graph, 'frontier, C>, String> {
    analysis_charge(remaining)?;
    let seeds = visited.get(&rows).ok_or("Missing exact visited frontier")?;
    analysis_charge(remaining)?;
    Ok((
        SeededQueue::<'graph, C>::adopt(rows, graph, state),
        SeededKeys::<'frontier, C>::new(seeds, state),
    ))
}

pub(super) struct SeededKeys<'frontier, C = VecCalls> {
    seeds: &'frontier CallRows<C>,
    uniform_state: usize,
    added: CallRows<C>,
}
impl<'frontier, C: Calls> SeededKeys<'frontier, C> {
    pub(super) fn new(seeds: &'frontier CallRows<C>, uniform_state: usize) -> Self {
        Self {
            seeds,
            uniform_state,
            added: CallRows::<C>::new(),
        }
    }
}
impl<C: Calls> ClosureKeys<C> for SeededKeys<'_, C> {
    fn contains_observed(
        &self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        if row.state == self.uniform_state {
            if self.seeds.contains(row) {
                return Ok(true);
            }
            // This is a real SECOND full-key lookup, including on a duplicate
            // in added. A denied debit precedes the lookup/publication.
            analysis_charge(remaining)?;
        }
        Ok(self.added.contains(row))
    }
    fn insert_reserved(&mut self, row: AnalysisConfiguration<C>) -> bool {
        self.added.insert(row)
    }
}

// Initial seeds are consumed in canonical order, ahead of all new appends.
// Their whole call vectors MOVE from the existing set. No seed set/queue is
// cloned or allocated here; the already-existing visited clone remains live.
pub(super) struct SeededQueue<'graph, C = VecCalls> {
    seeds: btree_set::IntoIter<AnalysisConfiguration<C>>,
    appended: GeometricAdjacentQueue<'graph, C>,
    seed_nontransparent: bool,
    last_seed_pop: Option<(usize, usize)>,
}
impl<'graph, C: Calls> SeededQueue<'graph, C> {
    pub(super) fn adopt(rows: CallRows<C>, graph: &'graph [State], _: usize) -> Self {
        Self {
            seeds: rows.into_iter(),
            appended: GeometricAdjacentQueue::<'graph, C>::empty(graph),
            seed_nontransparent: false,
            last_seed_pop: None,
        }
    }
}
impl<'graph, C: Calls> ClosureQueue<'graph, C> for SeededQueue<'graph, C> {
    const POOL_QUEUE_KIND: usize = 1;
    fn empty(graph: &'graph [State]) -> Self {
        Self::adopt(CallRows::<C>::new(), graph, 0)
    }
    fn unreserved(_: CallRows<C>, _: &'graph [State]) -> Result<Self, String> {
        Err("Owned frontier adoption requires exact paid keys".into())
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        self.appended
            .reserve_placement_observed(row, remaining, exact)
    }
    fn observed_shape(&self) -> (usize, usize) {
        let (batches, siblings) = self.appended.observed_shape();
        if self.seeds.len() == 0 {
            (batches, siblings)
        } else {
            (batches + 1, self.seeds.len())
        }
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration<C>) -> Result<(), String> {
        self.appended.push_reserved(row)
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<C>> {
        if let Some(row) = self.seeds.next() {
            self.last_seed_pop = (!self.seed_nontransparent && self.seeds.len() > 0)
                .then_some((row.state, self.seeds.len()));
            Some(row)
        } else {
            self.last_seed_pop = None;
            self.appended.pop_front()
        }
    }
    fn share_keys_observed<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        scheduled: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        share_adjacent(self, row, scheduled, remaining, (measured, exact))
    }
}
impl<'graph, C: Calls> AdjacentReplay<'graph, C> for SeededQueue<'graph, C> {
    fn replay_group(&self) -> Option<(usize, usize)> {
        self.last_seed_pop.or_else(|| self.appended.replay_group())
    }
    fn clear_replay_group(&mut self) {
        self.last_seed_pop = None;
        self.appended.clear_replay_group();
    }
    fn replay_graph(&self) -> &'graph [State] {
        self.appended.replay_graph()
    }
    fn mark_nontransparent(&mut self, state: usize, siblings: usize) -> Result<(), String> {
        if self.seeds.len() > 0 {
            if self.seeds.len() < siblings {
                return Err("Owned seed frontier changed before source-shape fallback".into());
            }
            self.seed_nontransparent = true;
            Ok(())
        } else {
            self.appended.mark_nontransparent(state, siblings)
        }
    }
}

#[cfg(test)]
#[path = "frontier_adoption_tests.rs"]
mod tests;
