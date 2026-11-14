//! Producer-owned opaque-boundary evidence for the selected paid FIFO drain.
//! This is graph-only evidence, never a FIRST/Full/FOLLOW/caller result.
use super::super::PoolWork;
use super::super::adjacent::AnalysisPop;
use super::owned_fifo_ranges::RangeQueue;
use super::single_borrowed_ports::consume_port_rows;
use super::*;

// Only this module and the sibling selected producer can mint a Boundary.
// Consumers may inspect the row but cannot replace its full context or attach
// the evidence to another state. The selected queue is invocation-local and
// owns the same immutable graph as its Kernel.
pub(in crate::grammar::parser_prediction) struct Boundary<C: Calls> {
    row: AnalysisConfiguration<C>,
}
impl<C: Calls> Boundary<C> {
    pub(super) fn completed(row: AnalysisConfiguration<C>) -> Self {
        Self { row }
    }
    pub(in crate::grammar::parser_prediction) fn row(&self) -> &AnalysisConfiguration<C> {
        &self.row
    }
    pub(super) fn compatibility_row(self) -> AnalysisConfiguration<C> {
        self.row
    }
}
const _: () = assert!(std::mem::size_of::<AnalysisPop<SharedCalls>>() <= CONFIGURATION_BYTES);

impl<P: ComponentPolicy> Kernel<'_, P> {
    pub(super) fn consume_boundary<C: Calls, F: FnMut() -> Result<(), String>>(
        &mut self,
        boundary: Boundary<C>,
        charge: &mut F,
    ) -> Result<AnalysisConfiguration<C>, String> {
        // The old worker checks sticky poison before its root debit too. A
        // queued, already completed proof cannot revive a poisoned invocation.
        if self.poisoned {
            return Err("Transparent port source is poisoned".into());
        }
        // One real metadata-use event is paid BEFORE extracting/using the
        // carried evidence. The redundant graph.get and is_pure scan are gone.
        // Stops/FOLLOW therefore save zero; actual scanned effect edges can
        // save work. No removed lookup is charged as though it still happened.
        if let Err(error) = charge() {
            self.invalidate();
            return Err(error);
        }
        Ok(boundary.row)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn consume_small<K: ClosureKeys<SharedCalls>>(
    ports: &[Port],
    row: AnalysisConfiguration<SharedCalls>,
    target: &mut RangeQueue<'_>,
    keys: &mut K,
    work: &mut usize,
    measured: Option<&RecursiveAnalysisCounters>,
    exact: Option<&AdjacentWorkObservation>,
    pool: Option<&PoolWork>,
) -> Result<(), String> {
    if target.failed {
        return Err("Owned port FIFO is poisoned".into());
    }
    if ports.is_empty() || ports.len() >= 16 {
        return Err("Small completed ports lack their exact producer admission".into());
    }
    // The same Kernel callback supplied these completed ports, admitted both
    // full configurations per candidate, and retained the entire ordered tail.
    // No new heap holder, key, source lookup or semantic callback is added.
    let mut admission = BoundaryAdmission {
        target: Some(target),
        pool,
    };
    consume_port_rows::<SharedCalls, _, K>(
        ports,
        &row,
        &mut admission,
        keys,
        work,
        measured,
        exact,
        pool,
    )
}
struct BoundaryAdmission<'queue, 'graph, 'pool, 'capture> {
    target: Option<&'queue mut RangeQueue<'graph>>,
    pool: Option<&'pool PoolWork<'capture>>,
}
impl<'graph> ClosureQueue<'graph, SharedCalls> for BoundaryAdmission<'_, 'graph, '_, '_> {
    fn empty(_: &'graph [State]) -> Self {
        Self {
            target: None,
            pool: None,
        }
    }
    fn unreserved(
        _: BTreeSet<AnalysisConfiguration<SharedCalls>>,
        _: &'graph [State],
    ) -> Result<Self, String> {
        Err("Completed boundary admission needs its exact source queue".into())
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration<SharedCalls>,
        work: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        let target = self
            .target
            .as_mut()
            .ok_or("Completed boundary admission has no source queue")?;
        if target.failed {
            return Err("Owned port FIFO is poisoned".into());
        }
        target
            .ordinary_published
            .checked_add(1)
            .ok_or("Owned port FIFO publication ordinal overflow")?;
        target
            .ordinary
            .reserve_completed_boundary(row, work, exact, self.pool)
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration<SharedCalls>) -> Result<(), String> {
        let target = self
            .target
            .as_mut()
            .ok_or("Completed boundary admission has no source queue")?;
        if target.failed {
            return Err("Owned port FIFO is poisoned".into());
        }
        let next = target
            .ordinary_published
            .checked_add(1)
            .ok_or("Owned port FIFO publication ordinal overflow")?;
        target.ordinary.push_completed_boundary(row)?;
        target.ordinary_published = next;
        Ok(())
    }
    fn observed_shape(&self) -> (usize, usize) {
        self.target
            .as_ref()
            .map_or((0, 0), |queue| queue.observed_shape())
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<SharedCalls>> {
        None
    }
}
