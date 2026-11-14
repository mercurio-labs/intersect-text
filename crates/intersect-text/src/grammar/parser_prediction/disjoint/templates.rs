//! Stack-relative closure templates over the shared proof walker.
use super::{AnalysisConfiguration, Closure, ParserPrediction, analysis_charge};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
impl ParserPrediction {
    pub(super) fn relative_proof_closure(
        &self,
        configurations: BTreeSet<AnalysisConfiguration>,
        guard_overapproximation: bool,
        cache: &mut BTreeMap<(usize, usize), Closure>,
        remaining: &mut usize,
    ) -> Result<Closure, String> {
        let mut pending: VecDeque<_> = configurations.into_iter().collect();
        let mut visited = BTreeSet::new();
        let mut result = Closure::default();
        while let Some(row) = pending.pop_front() {
            analysis_charge(remaining)?;
            if !visited.insert(row.clone()) {
                continue;
            }
            let bound = 64usize
                .checked_sub(row.calls.len())
                .ok_or("Grammar prediction budget exceeded")?;
            // There is no internal closure to cache at an existing return.
            // Use the same checked return transition as the uncached walker.
            let state = self
                .states
                .get(row.state)
                .ok_or("Dangling parser prediction state")?;
            if state.rule_stop {
                if state.accepting {
                    result.accepting.insert(row.alternative);
                }
                self.proof_return(row, remaining, &mut pending)?;
                continue;
            }
            let key = (row.state, bound);
            let template = match cache.entry(key) {
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::btree_map::Entry::Vacant(entry) => {
                    let seed = AnalysisConfiguration {
                        state: row.state,
                        alternative: 0,
                        calls: Vec::new(),
                    };
                    entry.insert(self.proof_closure(
                        [seed].into(),
                        guard_overapproximation,
                        remaining,
                        bound,
                        true,
                    )?)
                }
            };
            if !template.accepting.is_empty() {
                result.accepting.insert(row.alternative);
            }
            for (symbol, targets) in &template.tokens {
                for target in targets {
                    analysis_charge(remaining)?;
                    let mut calls = row.calls.clone();
                    calls.extend_from_slice(&target.calls);
                    result.tokens.entry(symbol.clone()).or_default().insert(
                        AnalysisConfiguration {
                            state: target.state,
                            alternative: row.alternative,
                            calls,
                        },
                    );
                }
            }
            for stop in &template.returned {
                analysis_charge(remaining)?;
                self.proof_return(
                    AnalysisConfiguration {
                        state: *stop,
                        ..row.clone()
                    },
                    remaining,
                    &mut pending,
                )?;
            }
        }
        Ok(result)
    }
}
