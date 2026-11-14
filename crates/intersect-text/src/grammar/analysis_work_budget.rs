//! Finite compiler-analysis work policy. This changes work grants only.
//! State cardinality, source limits, syntax/call depth and storage stay separate.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrammarAnalysisWorkBudget {
    units: usize,
}

impl GrammarAnalysisWorkBudget {
    /// Historical per-pool grant for explicit regression/compatibility requests.
    pub const LEGACY_10K: Self = Self::new(10_000);
    /// Retained initial calibration profile; original SysML exhausted this grant.
    pub const CALIBRATION_1M: Self = Self::new(1_000_000);
    /// Retained ten-million calibration profile; original SysML exhausted it.
    pub const CALIBRATION_10M: Self = Self::new(10_000_000);
    /// Realistic finite production grant; calibration remains observable.
    pub const REALISTIC: Self = Self::new(100_000_000);

    /// Zero is a valid deterministic denial policy. There is no unlimited mode.
    pub const fn new(units: usize) -> Self {
        Self { units }
    }
    pub const fn units(self) -> usize {
        self.units
    }
}
impl Default for GrammarAnalysisWorkBudget {
    fn default() -> Self {
        Self::REALISTIC
    }
}

#[cfg(test)]
#[path = "analysis_work_budget_tests.rs"]
mod tests;
