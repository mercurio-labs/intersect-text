//! Keep the first paid lookup/publication borrow through the canonical consumer.
//! All row copies, descriptor reads and scheduling remain charged and ordered.
use super::super::{PoolBlock, PoolWork, pool_scope};
use super::*;
use std::collections::btree_map::Entry;

// A stack-only borrow passed to an owned-result callback, never retained in a
// Kernel/queue/configuration. Length is read once under the existing metadata fee.
struct PreparedPorts<'entry> {
    prepared: bool,
    length: Option<usize>,
    ports: Option<&'entry [Port]>,
}

impl<P: ComponentPolicy> Kernel<'_, P> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn with_prepared_entry_for_ranges<W, T, F, H>(
        &mut self,
        start: usize,
        work: &mut W,
        charge: &mut F,
        topology: Option<&PoolWork>,
        consume: H,
    ) -> Result<T, String>
    where
        F: FnMut(&mut W) -> Result<(), String>,
        H: FnOnce(
            Option<&[Port]>,
            Option<usize>,
            &mut SummaryStats,
            &mut W,
            &mut F,
        ) -> Result<T, String>,
    {
        self.with_prepared_entry(
            start,
            work,
            charge,
            topology,
            |entry, stats, work, charge| {
                consume(
                    entry.ports,
                    entry.length.filter(|_| entry.prepared),
                    stats,
                    work,
                    charge,
                )
            },
        )
    }

    pub(super) fn prepare_with_length_and_topology<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
        topology: Option<&PoolWork>,
    ) -> Result<(bool, Option<usize>), String> {
        self.with_prepared_entry(
            start,
            charge,
            &mut |charge: &mut F| charge(),
            topology,
            |entry, _, _, _| Ok((entry.prepared, entry.length)),
        )
    }

    // The result T is fixed outside the entry's borrow lifetime. In particular,
    // neither the entry nor a partially admitted port slice can escape via T.
    // Work is passed sequentially to the charge adaptor and the callback so the
    // real compiler pool is never captured by two simultaneous mutable borrows.
    fn with_prepared_entry<W, T, F, H>(
        &mut self,
        start: usize,
        work: &mut W,
        charge: &mut F,
        topology: Option<&PoolWork>,
        consume: H,
    ) -> Result<T, String>
    where
        F: FnMut(&mut W) -> Result<(), String>,
        H: FnOnce(PreparedPorts<'_>, &mut SummaryStats, &mut W, &mut F) -> Result<T, String>,
    {
        if self.poisoned {
            return Err("Transparent port source is poisoned".into());
        }
        bump(&mut self.stats.lookups)?;
        charge(work)?;
        let root = self
            .graph
            .get(start)
            .ok_or("Dangling parser prediction state")?;
        if let Some(observer) = topology {
            observer.topology_queue(self.stats.lookups == 1, self.graph.len());
        }
        if !self.is_pure(root, &mut || charge(work))? {
            bump(&mut self.stats.fallbacks)?;
            return consume(
                PreparedPorts {
                    prepared: false,
                    length: None,
                    ports: None,
                },
                &mut self.stats,
                work,
                charge,
            );
        }
        if !self.admitted {
            self.admit(HEADER_BYTES, &mut || charge(work))?;
            self.admitted = true;
        }
        map_work(self.completed.len(), &mut || charge(work))?;
        if P::BORROW_PORTS {
            if let Some(entry) = self.completed.get(&start) {
                bump(&mut self.stats.hits)?;
                charge(work)?;
                let length = entry.as_ref().map(|entry| entry.len());
                // This is the FIRST paid lookup. There is no later map reborrow.
                return consume(
                    PreparedPorts {
                        prepared: true,
                        length,
                        ports: entry.as_deref(),
                    },
                    &mut self.stats,
                    work,
                    charge,
                );
            }
        } else if self.completed.contains_key(&start) {
            bump(&mut self.stats.hits)?;
            return consume(
                PreparedPorts {
                    prepared: true,
                    length: None,
                    ports: None,
                },
                &mut self.stats,
                work,
                charge,
            );
        }
        bump(&mut self.stats.builds)?;
        let value = if topology.is_some() {
            self.build_with_topology(start, &mut || charge(work), topology)?
        } else {
            self.build(start, &mut || charge(work))?
        };
        if value.is_none() {
            bump(&mut self.stats.fallbacks)?;
        }
        if self.completed.len() >= MAX_KEYS {
            return Err("Transparent port summary bound exceeded".into());
        }
        self.admit(SUMMARY_BYTES, &mut || charge(work))?;
        map_work(self.completed.len(), &mut || charge(work))?;
        charge(work)?;
        if value.is_some() {
            bump(&mut self.stats.publications)?;
        }
        let length = if P::BORROW_PORTS {
            charge(work)?;
            value.as_ref().map(|entry| entry.len())
        } else {
            None
        };
        // One publication operation replaces insert, under its SAME paid fee.
        // It also lends that inserted value. No contains/get follows publication.
        // The occupied arm retains insert's exact replacement semantics.
        let entry = match self.completed.entry(start) {
            Entry::Vacant(entry) => entry.insert(value),
            Entry::Occupied(mut entry) => {
                let _ = entry.insert(value);
                entry.into_mut()
            }
        };
        consume(
            PreparedPorts {
                prepared: true,
                length,
                ports: if P::BORROW_PORTS {
                    entry.as_deref()
                } else {
                    None
                },
            },
            &mut self.stats,
            work,
            charge,
        )
    }

    pub(super) fn admit_configuration_fields<F: FnMut() -> Result<(), String>>(
        stats: &mut SummaryStats,
        poisoned: bool,
        returns: usize,
        charge: &mut F,
    ) -> Result<(), String> {
        if poisoned {
            return Err("Transparent port source is poisoned".into());
        }
        let bytes = returns
            .checked_mul(RETURN_BYTES)
            .and_then(|n| n.checked_add(CONFIGURATION_BYTES))
            .ok_or("Transparent port configuration size overflow")?;
        Self::admit_fields(stats, bytes, charge)?;
        charge()
    }

    pub(super) fn consume_prepared_ports<W, T, F, H>(
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
        H: FnOnce(Option<&[Port]>, &mut W) -> Result<T, String>,
    {
        let poisoned = self.poisoned;
        let result = self.with_prepared_entry(
            start,
            work,
            charge,
            topology,
            |entry, stats, work, charge| {
                let ports = if let Some(length) = entry.length.filter(|_| entry.prepared) {
                    for _ in 0..length {
                        Self::admit_configuration_fields(stats, poisoned, returns, &mut || {
                            charge(work)
                        })?;
                        Self::admit_configuration_fields(stats, poisoned, returns, &mut || {
                            charge(work)
                        })?;
                    }
                    entry.ports
                } else {
                    None
                };
                // Nested owned Result preserves the prior poison boundary:
                // kernel/admission errors invalidate, consumer errors propagate.
                Ok(consume(ports, work))
            },
        );
        match result {
            Err(error) => {
                self.invalidate();
                Err(error)
            }
            Ok(consumer_result) => consumer_result,
        }
    }
}

impl<'graph, P: ComponentPolicy, C: Calls, I: ClosureQueue<'graph, C>> Queue<'graph, P, C, I> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn share_keys_pool<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        keys: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        if P::BORROW_PORTS && P::REUSE && C::SHARED {
            let (inner, summaries) = (&mut self.inner, &mut self.summaries);
            if I::OWNED_PORT_RANGES {
                return summaries.consume_prepared_ports_for_ranges(
                    row.state,
                    row.calls.copied_pairs(),
                    remaining,
                    &mut |remaining| {
                        let result = analysis_charge(remaining);
                        if let Some(work) = pool {
                            work.actual_leaf_result(PoolBlock::PortKernel, result.is_ok());
                        }
                        result
                    },
                    pool.filter(|work| work.topology_enabled()),
                    |ports, capacity, remaining| {
                        if capacity.is_some() {
                            let ports = ports.ok_or("Prepared range lost its exact ports")?;
                            inner.consume_owned_port_range(
                                ports, row, keys, remaining, measured, exact, pool,
                            )?;
                            Ok(None)
                        } else if let Some(ports) = ports {
                            inner.consume_small_completed_ports(
                                ports, row, keys, remaining, measured, exact, pool,
                            )?;
                            Ok(None)
                        } else {
                            Ok(Some(row))
                        }
                    },
                );
            }
            return summaries.consume_prepared_ports(
                row.state,
                row.calls.copied_pairs(),
                remaining,
                &mut |remaining| {
                    let result = analysis_charge(remaining);
                    if let Some(work) = pool {
                        work.actual_leaf_result(PoolBlock::PortKernel, result.is_ok());
                    }
                    result
                },
                pool.filter(|work| work.topology_enabled()),
                |ports, remaining| {
                    Self::consume_ports::<K>(
                        ports, row, inner, keys, remaining, measured, exact, pool,
                    )
                },
            );
        }
        self.share_keys_pool_reborrow(row, keys, remaining, measured, exact, pool)
    }

    // Historical policies and the test-only old shared-call reference keep the
    // old map operations/fees. Both use the exact same canonical consumer below.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn share_keys_pool_reborrow<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        keys: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        let (inner, summaries) = (&mut self.inner, &mut self.summaries);
        let mut charge = || {
            let result = analysis_charge(remaining);
            if let Some(work) = pool {
                work.actual_leaf_result(PoolBlock::PortKernel, result.is_ok());
            }
            result
        };
        let ports = if let Some(work) = pool.filter(|work| work.topology_enabled()) {
            if C::SHARED {
                summaries.borrow_ports_for_configurations_with_topology(
                    row.state,
                    row.calls.copied_pairs(),
                    &mut charge,
                    work,
                )?
            } else {
                work.topology_unsupported_policy();
                summaries.borrow_ports_for_configurations(
                    row.state,
                    row.calls.copied_pairs(),
                    &mut charge,
                )?
            }
        } else {
            summaries.borrow_ports_for_configurations(
                row.state,
                row.calls.copied_pairs(),
                &mut charge,
            )?
        };
        Self::consume_ports::<K>(ports, row, inner, keys, remaining, measured, exact, pool)
    }

    #[allow(clippy::too_many_arguments)]
    fn consume_ports<K: ClosureKeys<C>>(
        ports: Option<&[Port]>,
        row: AnalysisConfiguration<C>,
        inner: &mut I,
        keys: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: Option<&PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        let Some(ports) = ports else {
            return Ok(Some(row));
        };
        consume_port_rows::<C, I, K>(ports, &row, inner, keys, remaining, measured, exact, pool)?;
        Ok(None)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn consume_port_rows<'graph, C: Calls, I: ClosureQueue<'graph, C>, K: ClosureKeys<C>>(
    ports: &[Port],
    row: &AnalysisConfiguration<C>,
    inner: &mut I,
    keys: &mut K,
    remaining: &mut usize,
    measured: Option<&RecursiveAnalysisCounters>,
    exact: Option<&AdjacentWorkObservation>,
    pool: Option<&PoolWork>,
) -> Result<(), String> {
    for port in ports {
        let result = analysis_charge(remaining);
        if let Some(work) = pool {
            work.actual_leaf_result(PoolBlock::PortDescriptor, result.is_ok());
        }
        result?;
        let next = AnalysisConfiguration::<C> {
            state: port.state,
            alternative: row.alternative,
            calls: row.calls.clone(),
        };
        pool_scope(remaining, pool, PoolBlock::PortSchedule, |remaining| {
            K::schedule(
                next,
                inner,
                keys,
                remaining,
                (true, ScheduleSite::Successor, measured),
                exact,
            )
        })?;
    }
    Ok(())
}
