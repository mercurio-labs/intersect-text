//! One positive-only continuation probe, using a real partial token bucket.
//! Fixed logical charges are portable policy units, not allocator/RSS limits.
use super::analysis_calls::{Calls, VecCalls};
use super::{AnalysisConfiguration, PredictionSymbol, analysis_charge};
use std::collections::{BTreeMap, BTreeSet};

type Rows<C = VecCalls> = BTreeSet<AnalysisConfiguration<C>>;

pub(super) trait TokenProbe<C: Calls = VecCalls> {
    fn depth_first(&self) -> bool;
    fn select(
        &mut self,
        symbol: &PredictionSymbol,
        alternative: usize,
        rows: &Rows<C>,
        remaining: &mut usize,
    ) -> Result<Option<Rows<C>>, String>;
}

/// Disabled old route: no table, admission, lookup, copy, or additional debit.
pub(super) struct NoProbe;
impl<C: Calls> TokenProbe<C> for NoProbe {
    fn depth_first(&self) -> bool {
        false
    }
    fn select(
        &mut self,
        _: &PredictionSymbol,
        _: usize,
        _: &Rows<C>,
        _: &mut usize,
    ) -> Result<Option<Rows<C>>, String> {
        Ok(None)
    }
}

pub(super) struct FirstSharedToken {
    first: BTreeMap<PredictionSymbol, usize>,
    started: bool,
    selected: bool,
    depth_first: bool,
    logical: usize,
}
impl FirstSharedToken {
    pub(super) fn new(depth_first: bool) -> Self {
        Self {
            first: BTreeMap::new(),
            started: false,
            selected: false,
            depth_first,
            logical: 0,
        }
    }
    fn admit(&mut self, bytes: usize, remaining: &mut usize) -> Result<(), String> {
        // One copy/admission event plus one unit per fixed logical 64-byte block.
        // Even an empty vector copy is an event. Charge before retention.
        let logical = self
            .logical
            .checked_add(bytes)
            .ok_or("Prediction token probe logical admission overflow")?;
        analysis_charge(remaining)?;
        for _ in 0..bytes.div_ceil(64) {
            analysis_charge(remaining)?;
        }
        self.logical = logical;
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn logical(&self) -> usize {
        self.logical
    }
}
impl TokenProbe for FirstSharedToken {
    fn depth_first(&self) -> bool {
        self.depth_first
    }
    fn select(
        &mut self,
        symbol: &PredictionSymbol,
        alternative: usize,
        rows: &Rows,
        remaining: &mut usize,
    ) -> Result<Option<Rows>, String> {
        if self.selected {
            return Ok(None);
        }
        if !self.started {
            self.admit(32, remaining)?;
            self.started = true;
        }
        let bytes = match symbol {
            PredictionSymbol::Keyword(s) | PredictionSymbol::Terminal(s) => s.len(),
            PredictionSymbol::End => 0,
        };
        // Paid ordered-map lookup plus the borrowed symbol comparison payload.
        // This is a logical BTreeMap operation, not a whole CPU/allocator bound.
        analysis_charge(remaining)?;
        for _ in 0..bytes.div_ceil(64) {
            analysis_charge(remaining)?;
        }
        let first = self.first.get(symbol).copied();
        analysis_charge(remaining)?; // Actual alternative comparison/publication.
        match first {
            None => {
                let row_bytes = 64usize
                    .checked_add(bytes)
                    .ok_or("Prediction token probe key size overflow")?;
                self.admit(row_bytes, remaining)?;
                self.first.insert(symbol.clone(), alternative);
                Ok(None)
            }
            Some(first) if first == alternative => Ok(None),
            Some(_) => {
                // Arm before any snapshot debit. A failed operation is terminal
                // in the enclosing same worker; no retry or allowance appears.
                self.selected = true;
                self.admit(32, remaining)?; // Snapshot Rows header.
                // The nested initial visited clone owns a distinct Rows header;
                // do not hide its admission in old frontier publication fees.
                self.admit(32, remaining)?;
                let mut snapshot = Rows::new();
                for row in rows {
                    // The complete alternative/state and ordered return vector
                    // are retained. No target, caller, or graph key is invented.
                    let stack = row
                        .calls
                        .len()
                        .checked_mul(16)
                        .ok_or("Prediction token probe stack size overflow")?;
                    let row_bytes = 64usize
                        .checked_add(stack)
                        .ok_or("Prediction token probe row size overflow")?;
                    self.admit(row_bytes, remaining)?; // New snapshot copy.
                    // The SAME canonical worker immediately retains a visited
                    // clone of these initial rows. Reserve that second copy
                    // here too, before either copy can be made/published.
                    self.admit(row_bytes, remaining)?;
                    snapshot.insert(row.clone());
                }
                Ok(Some(snapshot))
            }
        }
    }
}
