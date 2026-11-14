//! Complete, input-independent first-unit partitions of the SAME lexer NFA.
//! Selected bodies, UTF-16 input and EOF remain canonical runtime operations.
use super::*;
const STEP_BYTES: usize = 128;
pub(super) struct FirstStep {
    low: u16,
    initial: BTreeSet<usize>,
    owners: BTreeSet<usize>,
    accepting: BTreeSet<usize>,
}
impl PreparedLexicalInitial<'_> {
    /// Extend this exact program's paid initial facts with every UTF-16 first
    /// transition. Failure consumes this candidate and cannot publish a prefix.
    /// The cap includes already retained initial facts; callers keep one ledger.
    #[cfg(test)]
    pub(crate) fn with_first_steps(
        self,
        work: &mut Option<usize>,
        logical_storage_limit: usize,
    ) -> Result<Self, String> {
        self.with_first_steps_observed(work, logical_storage_limit, None)
    }
    pub(super) fn with_first_steps_observed(
        mut self,
        work: &mut Option<usize>,
        logical_storage_limit: usize,
        observation: Option<&InvocationContext<'_>>,
    ) -> Result<Self, String> {
        if observation.is_some() {
            LexerPrediction::consume_observed(work, observation)?;
        } else {
            self.check(self.program, work)?;
        }
        if self.first_steps.is_some() {
            return Err("Lexical first steps are already prepared".into());
        }
        let intervals = self.program.interval_targets_observed(
            &self.initial,
            work,
            &mut ReachabilityStats::default(),
            observation,
        )?;
        let mut rows = Vec::new();
        let mut logical = self.logical_bytes;
        for (low, targets) in intervals {
            let initial = self.program.closure_observed(targets, work, observation)?;
            let (owners, accepting) =
                self.program
                    .frontier_owners_observed(&initial, work, observation)?;
            let bytes = initial
                .len()
                .checked_add(owners.len())
                .and_then(|n| n.checked_add(accepting.len()))
                .and_then(|n| n.checked_mul(RECORD_BYTES))
                .and_then(|n| n.checked_add(STEP_BYTES))
                .ok_or("Lexical first-step storage overflow")?;
            logical = logical
                .checked_add(bytes)
                .ok_or("Lexical first-step storage overflow")?;
            if logical > logical_storage_limit {
                return Err("Prepared lexical first-step storage budget exceeded".into());
            }
            for _ in 0..bytes.div_ceil(ADMISSION_QUANTUM) {
                LexerPrediction::consume_observed(work, observation)?;
            }
            if let Some(observation) = observation {
                observation.retain_storage(bytes)?;
            }
            rows.try_reserve_exact(1)
                .map_err(|_| "Lexical first-step allocation failed")?;
            rows.push(FirstStep {
                low,
                initial,
                owners,
                accepting,
            });
        }
        if rows.first().is_none_or(|row| row.low != 0) {
            return Err("Incomplete lexical first-step partition".into());
        }
        self.logical_bytes = logical;
        self.first_steps = Some(rows);
        Ok(self)
    }
    #[cfg(test)]
    pub(crate) fn first_step_intervals(&self) -> usize {
        self.first_steps.as_ref().map_or(0, Vec::len)
    }
    #[allow(clippy::type_complexity)]
    pub(in super::super) fn first_step(
        &self,
        unit: u16,
        work: &mut Option<usize>,
    ) -> Result<Option<(&BTreeSet<usize>, &BTreeSet<usize>, &BTreeSet<usize>)>, String> {
        let Some(rows) = &self.first_steps else {
            return Ok(None);
        };
        let mut low = 0;
        let mut high = rows.len();
        while low < high {
            LexerPrediction::consume(work)?;
            let mid = low + (high - low) / 2;
            if rows[mid].low <= unit {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        LexerPrediction::consume(work)?;
        let index = low
            .checked_sub(1)
            .ok_or("Incomplete lexical first-step lookup")?;
        let row = rows.get(index).ok_or("Invalid lexical first-step lookup")?;
        Ok(Some((&row.initial, &row.owners, &row.accepting)))
    }
}
const _: () = assert!(std::mem::size_of::<FirstStep>() <= STEP_BYTES);
#[cfg(test)]
#[path = "lexer_initial_first_steps_tests.rs"]
mod tests;
