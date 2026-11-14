//! Selected FIFO ownership after the SAME completed-port producer/key worker.
//! Every range owns state IDs and one complete caller context; no Kernel borrow,
//! new Rc, result cache, guard policy, source identity or public selector escapes.
use super::super::{PoolBlock, PoolWork};
use super::single_borrowed_ports::consume_port_rows;
use super::*;
use std::collections::VecDeque;

const MIN_RANGE_PORTS: usize = 16;
const RANGE_HEADER_BYTES: usize = 256;
const STATE_ID_BYTES: usize = 8;

struct Range {
    states: Vec<usize>,
    cursor: usize,
    alternative: usize,
    calls: SharedCalls,
    // Ordinary rows with smaller publication ordinals must pop first. Later
    // ordinary rows stay behind this range even when their states batch alike.
    ordinary_before: usize,
}

pub(in crate::grammar::parser_prediction) struct RangeQueue<'graph> {
    pub(super) ordinary: GeometricAdjacentQueue<'graph, SharedCalls>,
    ranges: VecDeque<Range>,
    pub(super) ordinary_published: usize,
    ordinary_popped: usize,
    pub(super) failed: bool,
}

fn materialize_charge(work: &mut usize, pool: Option<&PoolWork>) -> Result<(), String> {
    // The original key reservation still prepays FIFO row processing. This
    // EXTRA unit pays range-index/context reads and full-row reconstruction,
    // including the actual immutable caller clone, before those operations.
    let result = analysis_charge(work);
    if let Some(pool) = pool {
        pool.actual_leaf_result(PoolBlock::PortKernel, result.is_ok());
    }
    result
}

impl<P: ComponentPolicy> Kernel<'_, P> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn consume_prepared_ports_for_ranges<W, T, F, H>(
        &mut self,
        start: usize,
        returns: usize,
        work: &mut W,
        charge: &mut F,
        topology: Option<&PoolWork>,
        consume: H,
    ) -> Result<T, String>
    where
        F: FnMut(&mut W) -> Result<(), String>,
        H: FnOnce(Option<&[Port]>, Option<usize>, &mut W) -> Result<T, String>,
    {
        let poisoned = self.poisoned;
        let result = self.with_prepared_entry_for_ranges(
            start,
            work,
            charge,
            topology,
            |ports, length, stats, work, charge| {
                let range = length.filter(|length| *length >= MIN_RANGE_PORTS);
                if let Some(length) = length {
                    for _ in 0..length {
                        // The full ordered closed key is still retained eagerly.
                        Self::admit_configuration_fields(stats, poisoned, returns, &mut || {
                            charge(work)
                        })?;
                        if range.is_none() {
                            Self::admit_configuration_fields(
                                stats,
                                poisoned,
                                returns,
                                &mut || charge(work),
                            )?;
                        }
                    }
                }
                if let Some(length) = range {
                    let bytes = length
                        .checked_mul(STATE_ID_BYTES)
                        .and_then(|bytes| {
                            returns
                                .checked_mul(RETURN_BYTES)
                                .and_then(|tail| bytes.checked_add(tail))
                        })
                        .and_then(|bytes| bytes.checked_add(RANGE_HEADER_BYTES))
                        .ok_or("Owned port range size overflow")?;
                    Self::admit_fields(stats, bytes, &mut || charge(work))?;
                    // One immutable caller/context copy for the whole range.
                    charge(work)?;
                }
                Ok(consume(ports, range, work))
            },
        );
        match result {
            Err(error) => {
                self.invalidate();
                Err(error)
            }
            Ok(result) => result,
        }
    }
}

impl<'graph> RangeQueue<'graph> {
    fn range_ready(&self) -> bool {
        self.ranges
            .front()
            .is_some_and(|range| range.ordinary_before == self.ordinary_popped)
    }
    fn pop_paid_classified(
        &mut self,
        work: &mut usize,
        pool: Option<&PoolWork>,
    ) -> Result<Option<AnalysisPop<SharedCalls>>, String> {
        if self.failed {
            return Err("Owned port FIFO is poisoned".into());
        }
        if self.range_ready() {
            if let Err(error) = materialize_charge(work, pool) {
                self.failed = true;
                return Err(error);
            }
            let (row, last) = {
                let range = self
                    .ranges
                    .front_mut()
                    .ok_or("Missing owned port FIFO range")?;
                let state = *range
                    .states
                    .get(range.cursor)
                    .ok_or("Owned port range cursor is outside admitted IDs")?;
                let next = range
                    .cursor
                    .checked_add(1)
                    .ok_or("Owned port range cursor overflow")?;
                let row = AnalysisConfiguration {
                    state,
                    alternative: range.alternative,
                    calls: range.calls.clone(),
                };
                range.cursor = next;
                (row, next == range.states.len())
            };
            if last {
                self.ranges.pop_front();
            }
            return Ok(Some(AnalysisPop::Opaque(
                opaque_port_replay::Boundary::completed(row),
            )));
        }
        let Some((row, opaque)) = self.ordinary.pop_with_existing_source_flag() else {
            if self.ranges.is_empty() {
                return Ok(None);
            }
            self.failed = true;
            return Err("Owned port FIFO lost an earlier ordinary row".into());
        };
        self.ordinary_popped = self
            .ordinary_popped
            .checked_add(1)
            .ok_or("Owned port FIFO pop ordinal overflow")?;
        Ok(Some(if opaque {
            AnalysisPop::Opaque(opaque_port_replay::Boundary::completed(row))
        } else {
            AnalysisPop::Ordinary(row)
        }))
    }
}

impl<'graph> ClosureQueue<'graph, SharedCalls> for RangeQueue<'graph> {
    const OWNED_PORT_RANGES: bool = true;
    fn pop_for_analysis(
        &mut self,
        work: &mut usize,
        pool: Option<&PoolWork>,
    ) -> Result<Option<AnalysisPop<SharedCalls>>, String> {
        let result = self.pop_paid_classified(work, pool);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    #[allow(clippy::too_many_arguments)]
    fn consume_small_completed_ports<K: ClosureKeys<SharedCalls>>(
        &mut self,
        ports: &[Port],
        row: AnalysisConfiguration<SharedCalls>,
        keys: &mut K,
        work: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&PoolWork>,
    ) -> Result<(), String> {
        opaque_port_replay::consume_small(ports, row, self, keys, work, measured, exact, pool)
    }
    fn empty(graph: &'graph [State]) -> Self {
        Self {
            ordinary: GeometricAdjacentQueue::empty(graph),
            ranges: VecDeque::new(),
            ordinary_published: 0,
            ordinary_popped: 0,
            failed: false,
        }
    }
    fn unreserved(
        _: BTreeSet<AnalysisConfiguration<SharedCalls>>,
        _: &'graph [State],
    ) -> Result<Self, String> {
        Err("Owned port FIFO requires canonical exact-key reservations".into())
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration<SharedCalls>,
        work: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        if self.failed {
            return Err("Owned port FIFO is poisoned".into());
        }
        self.ordinary_published
            .checked_add(1)
            .ok_or("Owned port FIFO publication ordinal overflow")?;
        self.ordinary.reserve_placement_observed(row, work, exact)
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration<SharedCalls>) -> Result<(), String> {
        if self.failed {
            return Err("Owned port FIFO is poisoned".into());
        }
        let next = self
            .ordinary_published
            .checked_add(1)
            .ok_or("Owned port FIFO publication ordinal overflow")?;
        self.ordinary.push_reserved(row)?;
        self.ordinary_published = next;
        Ok(())
    }
    fn observed_shape(&self) -> (usize, usize) {
        let (ordinary, _) = self.ordinary.observed_shape();
        (ordinary.saturating_add(self.ranges.len()), 0)
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<SharedCalls>> {
        // This legacy unpaid interface can service only ordinary rows. All
        // canonical classifier pops use pop_front_with_work. An accidental
        // attempt to bypass range materialization fails closed/sticky.
        if self.failed {
            return None;
        }
        if self.range_ready() {
            self.failed = true;
            return None;
        }
        let row = self.ordinary.pop_front()?;
        self.ordinary_popped = match self.ordinary_popped.checked_add(1) {
            Some(next) => next,
            None => {
                self.failed = true;
                return None;
            }
        };
        Some(row)
    }
    fn pop_front_with_work(
        &mut self,
        work: &mut usize,
        pool: Option<&PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<SharedCalls>>, String> {
        // Compatibility callers still receive only the full row and therefore
        // repeat the literal Kernel eligibility path. Only the canonical typed
        // drain consumes the proof; it cannot become a sticky last-pop flag.
        Ok(self
            .pop_for_analysis(work, pool)?
            .map(|popped| match popped {
                AnalysisPop::Ordinary(row) => row,
                AnalysisPop::Opaque(boundary) => boundary.compatibility_row(),
            }))
    }
    #[allow(clippy::too_many_arguments)]
    fn consume_owned_port_range<K: ClosureKeys<SharedCalls>>(
        &mut self,
        ports: &[Port],
        row: AnalysisConfiguration<SharedCalls>,
        keys: &mut K,
        work: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&PoolWork>,
    ) -> Result<(), String> {
        if self.failed {
            return Err("Owned port FIFO is poisoned".into());
        }
        if !(MIN_RANGE_PORTS..=MAX_KEYS).contains(&ports.len()) {
            return Err("Owned port range lacks its bounded capacity admission".into());
        }
        const {
            assert!(std::mem::size_of::<usize>() <= STATE_ID_BYTES);
            assert!(std::mem::size_of::<Range>() <= 128);
        }
        // Kernel prepaid the header, ID capacity and single immutable context
        // before any descriptor/key operation. Allocation failures remain real.
        self.ranges
            .try_reserve_exact(1)
            .map_err(|_| "Owned port FIFO range slot allocation denied")?;
        let mut states = Vec::new();
        states
            .try_reserve_exact(ports.len())
            .map_err(|_| "Owned port FIFO state ID allocation denied")?;
        let range = Range {
            states,
            cursor: 0,
            alternative: row.alternative,
            calls: row.calls.clone(),
            ordinary_before: self.ordinary_published,
        };
        let mut admission = RangeAdmission {
            target: Some(self),
            pool,
            range,
            limit: ports.len(),
            reserved: false,
        };
        // This is the SOLE canonical descriptor/row/key loop, with exactly this
        // immutable source row. No other caller/context can enter the builder.
        let result = consume_port_rows::<SharedCalls, _, K>(
            ports,
            &row,
            &mut admission,
            keys,
            work,
            measured,
            exact,
            pool,
        );
        let prefix = admission.finish();
        match result {
            Err(error) => {
                prefix?;
                Err(error)
            }
            Ok(()) => prefix,
        }
    }
}

struct RangeAdmission<'queue, 'graph, 'pool, 'capture> {
    target: Option<&'queue mut RangeQueue<'graph>>,
    pool: Option<&'pool PoolWork<'capture>>,
    range: Range,
    limit: usize,
    reserved: bool,
}
impl RangeAdmission<'_, '_, '_, '_> {
    fn finish(self) -> Result<(), String> {
        let target = self
            .target
            .ok_or("Range admission has no exact source queue")?;
        if self.range.states.is_empty() {
            return Ok(());
        }
        if target.ranges.len() == target.ranges.capacity() {
            target.failed = true;
            return Err("Owned port FIFO lost its prepaid range slot".into());
        }
        // Also publish an actual accepted prefix on consumer error. There is no
        // fallible fee/allocation after eager full-key insertion. The exclusive
        // builder borrow prevents any range/ordinary pop while it is assembled.
        target.ranges.push_back(self.range);
        Ok(())
    }
}
impl<'graph> ClosureQueue<'graph, SharedCalls> for RangeAdmission<'_, 'graph, '_, '_> {
    fn empty(_: &'graph [State]) -> Self {
        // A generic classifier cannot obtain a paid builder/context. Its first
        // attempted reservation fails explicitly, before retaining any key.
        Self {
            target: None,
            pool: None,
            range: Range {
                states: Vec::new(),
                cursor: 0,
                alternative: 0,
                calls: SharedCalls::empty(),
                ordinary_before: 0,
            },
            limit: 0,
            reserved: false,
        }
    }
    fn unreserved(
        _: BTreeSet<AnalysisConfiguration<SharedCalls>>,
        _: &'graph [State],
    ) -> Result<Self, String> {
        Err("Range admission needs its exact owned source configuration".into())
    }
    fn reserve_placement(
        &mut self,
        _: &AnalysisConfiguration<SharedCalls>,
        work: &mut usize,
    ) -> Result<(), String> {
        // One extra bounded index-publication operation, paid before capacity
        // checks, key mutation and the state-ID write. No caller clone occurs.
        let debit = analysis_charge(work);
        if let Some(pool) = self.pool {
            // This actual index-publication debit remains inside PortSchedule.
            // Recording its leaf subtracts it once from that scope's remainder.
            pool.actual_leaf_result(PoolBlock::PortSchedule, debit.is_ok());
        }
        debit?;
        if self.target.is_none()
            || self.reserved
            || self.range.states.len() >= self.limit
            || self.range.states.len() == self.range.states.capacity()
        {
            return Err("Owned port FIFO state ID reservation differs".into());
        }
        self.reserved = true;
        Ok(())
    }
    fn observed_shape(&self) -> (usize, usize) {
        self.target
            .as_ref()
            .map_or((0, 0), |queue| queue.observed_shape())
    }
    fn publish_reserved<K: ClosureKeys<SharedCalls>>(
        &mut self,
        row: AnalysisConfiguration<SharedCalls>,
        keys: &mut K,
        site: ScheduleSite,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        if !self.reserved {
            return Err("Owned port FIFO key lacks its prepaid ID slot".into());
        }
        let state = row.state;
        // Ownership transfer removes the actual closed-key clone. The complete
        // key still has the unchanged ordered SharedCalls equality/Ord semantics.
        keys.insert_reserved(row);
        if let Some(work) = exact {
            work.key(site, super::super::ScheduleEvent::Retained);
        }
        if let Some(work) = measured {
            work.scheduling(site, super::super::ScheduleEvent::Retained);
        }
        self.range.states.push(state);
        self.reserved = false;
        Ok(())
    }
    fn push_reserved(&mut self, _: AnalysisConfiguration<SharedCalls>) -> Result<(), String> {
        Err("Owned port FIFO needs the canonical eager closed-key publication".into())
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<SharedCalls>> {
        None
    }
}
