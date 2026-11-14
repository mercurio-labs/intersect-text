//! Invocation-local FIFO representation. No recursive result or caller is cached.
use super::analysis_calls::{Calls, VecCalls};
use super::frontier::ClosureKeys;
use super::{
    AdjacentCharge, AdjacentWorkObservation, AnalysisConfiguration, Edge,
    RecursiveAnalysisCounters, ScheduleSite, State, Work, observed_analysis_charge, observed_edge,
    observed_recursive_charge, schedule_analysis_with_queue, schedule_analysis_with_queue_observed,
};
use std::collections::{BTreeSet, VecDeque};

// The producer-owned proof cannot be separated from, or attached to, a
// different full configuration by the canonical classifier.
pub(super) enum AnalysisPop<C: Calls> {
    Ordinary(AnalysisConfiguration<C>),
    Opaque(super::transparent_ports::opaque_port_replay::Boundary<C>),
}
impl<C: Calls> AnalysisPop<C> {
    pub(super) fn row(&self) -> &AnalysisConfiguration<C> {
        match self {
            Self::Ordinary(row) => row,
            Self::Opaque(boundary) => boundary.row(),
        }
    }
}

pub(super) trait ClosureQueue<'graph, C: Calls = VecCalls>: Sized {
    // Type-only diagnostic identity; no queue holder or scheduling change.
    const POOL_QUEUE_KIND: usize = 2;
    // Selected compiler FIFO representation only. Historical queues stay literal.
    const OWNED_PORT_RANGES: bool = false;
    fn pop_for_analysis(
        &mut self,
        remaining: &mut usize,
        pool: Option<&super::PoolWork>,
    ) -> Result<Option<AnalysisPop<C>>, String> {
        Ok(self
            .pop_front_with_work(remaining, pool)?
            .map(AnalysisPop::Ordinary))
    }
    fn share_opaque_boundary(
        &mut self,
        _boundary: super::transparent_ports::opaque_port_replay::Boundary<C>,
        _remaining: &mut usize,
        _pool: Option<&super::PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        Err("Completed opaque boundary is unavailable for this queue".into())
    }
    #[allow(clippy::too_many_arguments)]
    fn consume_small_completed_ports<K: ClosureKeys<C>>(
        &mut self,
        _ports: &[super::transparent_ports::Port],
        _row: AnalysisConfiguration<C>,
        _scheduled: &mut K,
        _remaining: &mut usize,
        _measured: Option<&RecursiveAnalysisCounters>,
        _exact: Option<&AdjacentWorkObservation>,
        _pool: Option<&super::PoolWork>,
    ) -> Result<(), String> {
        Err("Small completed ports are unavailable for this queue".into())
    }
    #[allow(clippy::too_many_arguments)]
    fn consume_owned_port_range<K: ClosureKeys<C>>(
        &mut self,
        _ports: &[super::transparent_ports::Port],
        _row: AnalysisConfiguration<C>,
        _scheduled: &mut K,
        _remaining: &mut usize,
        _measured: Option<&RecursiveAnalysisCounters>,
        _exact: Option<&AdjacentWorkObservation>,
        _pool: Option<&super::PoolWork>,
    ) -> Result<(), String> {
        Err("Owned port range is unavailable for this queue".into())
    }
    fn publish_reserved<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        scheduled: &mut K,
        site: ScheduleSite,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        scheduled.insert_reserved(row.clone());
        if let Some(work) = exact {
            work.key(site, super::ScheduleEvent::Retained);
        }
        if let Some(work) = measured {
            work.scheduling(site, super::ScheduleEvent::Retained);
        }
        self.push_reserved(row)
    }
    fn pop_front_with_work(
        &mut self,
        _remaining: &mut usize,
        _pool: Option<&super::PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        Ok(self.pop_front())
    }
    fn share_with_pool<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        scheduled: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        _pool: &super::PoolWork,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        K::share(row, self, scheduled, remaining, measured, exact)
    }
    fn empty(graph: &'graph [State]) -> Self;
    fn unreserved(
        rows: BTreeSet<AnalysisConfiguration<C>>,
        graph: &'graph [State],
    ) -> Result<Self, String>;
    /// Called after the old key fee, before key cloning or any queue mutation.
    fn reserve_placement(
        &mut self,
        _row: &AnalysisConfiguration<C>,
        _remaining: &mut usize,
    ) -> Result<(), String> {
        Ok(())
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
        _exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        self.reserve_placement(row, remaining)
    }
    /// Default literal target. Only an explicit compiler queue may summarize
    /// a source-proven internal epsilon/action continuation using the same pool.
    fn internal_target(&mut self, target: usize, _remaining: &mut usize) -> Result<usize, String> {
        Ok(target)
    }
    fn observed_shape(&self) -> (usize, usize) {
        (0, 0)
    }
    /// Caller has paid key reservation AND any new overflow allocation fee.
    fn push_reserved(&mut self, row: AnalysisConfiguration<C>) -> Result<(), String>;
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<C>>;
    fn schedule(
        row: AnalysisConfiguration<C>,
        queue: &mut Self,
        scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        context: (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
    ) -> Result<(), String> {
        schedule_analysis_with_queue(row, queue, scheduled, remaining, context)
    }
    fn schedule_observed(
        row: AnalysisConfiguration<C>,
        queue: &mut Self,
        scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        context: (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        if exact.is_none() {
            Self::schedule(row, queue, scheduled, remaining, context)
        } else {
            schedule_analysis_with_queue_observed(row, queue, scheduled, remaining, context, exact)
        }
    }
    fn share_keys_observed<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        _scheduled: &mut K,
        _remaining: &mut usize,
        _measured: Option<&RecursiveAnalysisCounters>,
        _exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        Ok(Some(row))
    }
    fn share_transparent_observed(
        &mut self,
        row: AnalysisConfiguration<C>,
        scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        _exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        self.share_transparent(row, scheduled, remaining, measured)
    }
    fn share_transparent(
        &mut self,
        row: AnalysisConfiguration<C>,
        _scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
        _remaining: &mut usize,
        _measured: Option<&RecursiveAnalysisCounters>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        Ok(Some(row))
    }
}

impl<'graph, C: Calls> ClosureQueue<'graph, C> for VecDeque<AnalysisConfiguration<C>> {
    fn empty(_: &'graph [State]) -> Self {
        VecDeque::new()
    }
    fn unreserved(
        rows: BTreeSet<AnalysisConfiguration<C>>,
        _: &'graph [State],
    ) -> Result<Self, String> {
        Ok(rows.into_iter().collect())
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration<C>) -> Result<(), String> {
        VecDeque::push_back(self, row);
        Ok(())
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<C>> {
        VecDeque::pop_front(self)
    }
    fn schedule(
        row: AnalysisConfiguration<C>,
        queue: &mut Self,
        scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        context: (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
    ) -> Result<(), String> {
        super::schedule_analysis(row, queue, scheduled, remaining, context)
    }
}

struct Context<C = VecCalls> {
    alternative: usize,
    calls: C,
}
struct Batch<C = VecCalls> {
    state: usize,
    first: Option<Context<C>>,
    overflow: VecDeque<Context<C>>,
    // Only a completed, paid immutable shape inspection can set this flag.
    // It is local to this FIFO item; no graph-wide result or error is cached.
    nontransparent: bool,
}
pub(super) type AdjacentQueue<'graph, C = VecCalls> = GrowthQueue<'graph, false, C>;
pub(super) type GeometricAdjacentQueue<'graph, C = VecCalls> = GrowthQueue<'graph, true, C>;

// Compile-time policy; the fields and all FIFO/context operations stay shared.
pub(super) struct GrowthQueue<'graph, const GEOMETRIC: bool, C = VecCalls> {
    graph: &'graph [State],
    batches: VecDeque<Batch<C>>,
    // The exact remaining siblings of the last actual, prepaid context pop.
    last_pop: Option<(usize, usize)>,
}

impl<'graph, const GEOMETRIC: bool, C: Calls> ClosureQueue<'graph, C>
    for GrowthQueue<'graph, GEOMETRIC, C>
{
    fn empty(graph: &'graph [State]) -> Self {
        Self {
            graph,
            batches: VecDeque::new(),
            last_pop: None,
        }
    }
    fn unreserved(
        _: BTreeSet<AnalysisConfiguration<C>>,
        _: &'graph [State],
    ) -> Result<Self, String> {
        Err("Adjacent closure batching requires exact-key reservations".into())
    }
    fn reserve_placement(
        &mut self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
    ) -> Result<(), String> {
        self.reserve_placement_observed(row, remaining, None)
    }
    fn observed_shape(&self) -> (usize, usize) {
        (
            self.batches.len(),
            self.last_pop.map_or(0, |(_, siblings)| siblings),
        )
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        if let Some(batch) = self.batches.back_mut()
            && batch.state == row.state
            && batch.overflow.len() == batch.overflow.capacity()
        {
            // A singleton is inline. Every actual overflow allocation/growth
            // is EXTRA work, paid before allocation and before key publication.
            // The old route still requests four. V78 requests the current
            // capacity (at least four), so a full buffer requests a doubling.
            // VecDeque may provide more; no exact allocator capacity is assumed.
            let additional = if GEOMETRIC {
                batch.overflow.capacity().max(4)
            } else {
                4
            };
            reserve_overflow(&mut batch.overflow, additional, remaining, exact)?;
        }
        Ok(())
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration<C>) -> Result<(), String> {
        self.push_with_source_flag(row, false)
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<C>> {
        self.pop_with_existing_source_flag().map(|(row, _)| row)
    }
    fn share_transparent(
        &mut self,
        row: AnalysisConfiguration<C>,
        scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        self.share_transparent_observed(row, scheduled, remaining, measured, None)
    }
    fn share_transparent_observed(
        &mut self,
        row: AnalysisConfiguration<C>,
        scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        self.share_keys_observed(row, scheduled, remaining, measured, exact)
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

impl<'graph, const GEOMETRIC: bool, C: Calls> GrowthQueue<'graph, GEOMETRIC, C> {
    pub(super) fn pop_with_existing_source_flag(
        &mut self,
    ) -> Option<(AnalysisConfiguration<C>, bool)> {
        let (row, empty, last_pop, opaque) = {
            let batch = self.batches.front_mut()?;
            let context = batch.first.take()?;
            batch.first = batch.overflow.pop_front();
            let opaque = batch.nontransparent;
            let last = (!opaque && batch.first.is_some())
                .then_some((batch.state, 1 + batch.overflow.len()));
            (
                AnalysisConfiguration::<C> {
                    state: batch.state,
                    alternative: context.alternative,
                    calls: context.calls,
                },
                batch.first.is_none(),
                last,
                opaque,
            )
        };
        if empty {
            self.batches.pop_front();
        }
        self.last_pop = last_pop;
        Some((row, opaque))
    }

    // Only RangeQueue uses this producer flag. It never implements/calls
    // AdjacentReplay, whose nontransparent fact has a different meaning.
    pub(super) fn reserve_completed_boundary(
        &mut self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&super::PoolWork>,
    ) -> Result<(), String> {
        if self
            .batches
            .back()
            .is_some_and(|batch| batch.state == row.state)
        {
            // A new batch already initializes this same existing flag slot.
            // A merge is an actual additional marker mutation, paid first.
            let debit = super::analysis_charge(remaining);
            if let Some(pool) = pool {
                pool.actual_leaf_result(super::PoolBlock::PortSchedule, debit.is_ok());
            }
            debit?;
        }
        self.reserve_placement_observed(row, remaining, exact)
    }
    pub(super) fn push_completed_boundary(
        &mut self,
        row: AnalysisConfiguration<C>,
    ) -> Result<(), String> {
        self.push_with_source_flag(row, true)
    }
    fn push_with_source_flag(
        &mut self,
        row: AnalysisConfiguration<C>,
        opaque: bool,
    ) -> Result<(), String> {
        // The old row fee covers exact-key cloning and replacement FIFO
        // placement. The opt-in header is larger; its inline first row needs
        // no new per-batch allocation. Overflow capacity was separately paid.
        let context = Context {
            alternative: row.alternative,
            calls: row.calls,
        };
        if let Some(batch) = self.batches.back_mut()
            && batch.state == row.state
        {
            if batch.overflow.len() == batch.overflow.capacity() {
                return Err("Adjacent closure overflow capacity was not reserved".into());
            }
            batch.overflow.push_back(context);
            if opaque {
                batch.nontransparent = true;
            }
        } else {
            self.batches.push_back(Batch {
                state: row.state,
                first: Some(context),
                overflow: VecDeque::new(),
                nontransparent: opaque,
            });
        }
        Ok(())
    }
}

pub(super) trait AdjacentReplay<'graph, C: Calls = VecCalls>:
    ClosureQueue<'graph, C>
{
    fn replay_group(&self) -> Option<(usize, usize)>;
    fn clear_replay_group(&mut self);
    fn replay_graph(&self) -> &'graph [State];
    fn mark_nontransparent(&mut self, state: usize, siblings: usize) -> Result<(), String>;
}
impl<'graph, const GEOMETRIC: bool, C: Calls> AdjacentReplay<'graph, C>
    for GrowthQueue<'graph, GEOMETRIC, C>
{
    fn replay_group(&self) -> Option<(usize, usize)> {
        self.last_pop
    }
    fn clear_replay_group(&mut self) {
        self.last_pop = None;
    }
    fn replay_graph(&self) -> &'graph [State] {
        self.graph
    }
    fn mark_nontransparent(&mut self, state: usize, siblings: usize) -> Result<(), String> {
        let batch = self
            .batches
            .front_mut()
            .ok_or("Missing inspected adjacent FIFO batch")?;
        if batch.state != state || batch.first.is_none() || 1 + batch.overflow.len() < siblings {
            return Err("Adjacent FIFO batch changed before source-shape fallback".into());
        }
        batch.nontransparent = true;
        Ok(())
    }
}

pub(super) fn share_adjacent<'graph, C: Calls, Q: AdjacentReplay<'graph, C>, K: ClosureKeys<C>>(
    queue: &mut Q,
    row: AnalysisConfiguration<C>,
    scheduled: &mut K,
    remaining: &mut usize,
    (measured, exact): (
        Option<&RecursiveAnalysisCounters>,
        Option<&AdjacentWorkObservation>,
    ),
) -> Result<Option<AnalysisConfiguration<C>>, String> {
    let Some((state, siblings)) = queue.replay_group() else {
        return Ok(Some(row));
    };
    // This new fee precedes graph lookup, edge-kind/target inspection and
    // shape-state mutation. It is never refunded on a fallback or failure.
    observed_recursive_charge(
        remaining,
        Work::TransitionEdge,
        measured,
        exact,
        AdjacentCharge::ShapeInspection,
    )?;
    queue.clear_replay_group();
    if row.state != state {
        return Err("Adjacent closure work item does not match its actual FIFO state".into());
    }
    let node = queue
        .replay_graph()
        .get(state)
        .ok_or("Dangling parser prediction state")?;
    let target = if node.rule_stop || node.accepting {
        None
    } else {
        match node.edges.as_slice() {
            [
                Edge::Epsilon(target)
                | Edge::Action(target)
                | Edge::First { target, .. }
                | Edge::Full { target, .. },
            ] => Some(*target),
            _ => None,
        }
    };
    if let Some(work) = exact {
        let edge = if node.rule_stop || node.accepting {
            None
        } else {
            match node.edges.as_slice() {
                [edge] => Some(observed_edge(edge)),
                _ => None,
            }
        };
        work.shape(target.is_some(), edge);
    }
    if let Some(target) = target {
        let target = match node.edges.as_slice() {
            [Edge::Epsilon(_) | Edge::Action(_)] => queue.internal_target(target, remaining)?,
            _ => target,
        };
        let mut first = Some(row);
        for ordinal in 0..=siblings {
            let mut next = if ordinal == 0 {
                first
                    .take()
                    .ok_or("Missing first adjacent closure context")?
            } else {
                let next = queue
                    .pop_front()
                    .ok_or("Missing adjacent closure sibling")?;
                if let Some(work) = measured {
                    work.closure_pop();
                }
                next
            };
            if next.state != state {
                return Err("Adjacent closure sibling has a different actual FIFO state".into());
            }
            next.state = target;
            // Each actual context still uses the original full key, dedup,
            // reservation and insertion order before moving into a queue.
            K::schedule(
                next,
                queue,
                scheduled,
                remaining,
                (true, ScheduleSite::Successor, measured),
                exact,
            )?;
        }
        queue.clear_replay_group();
        Ok(None)
    } else {
        queue.mark_nontransparent(state, siblings)?;
        Ok(Some(row))
    }
}

// The same charge/reserve/error sequence serves both policies. A denied charge
// cannot allocate or publish a key; capacity/allocator failures retain the debit.
fn reserve_overflow<C: Calls>(
    overflow: &mut VecDeque<Context<C>>,
    additional: usize,
    remaining: &mut usize,
    exact: Option<&AdjacentWorkObservation>,
) -> Result<(), String> {
    observed_analysis_charge(remaining, exact, AdjacentCharge::OverflowGrowth)?;
    overflow
        .try_reserve_exact(additional)
        .map_err(|_| "Adjacent closure overflow allocation unavailable".to_string())
}

#[cfg(test)]
#[path = "geometric_growth_tests.rs"]
mod geometric_growth_tests;
