//! Invocation-local, exact relative prefixes over the existing closure walker.
//! Builder facts and cached closures are never serialized or package authority.
use super::super::analysis_calls::Calls;
use super::super::{
    AdjacentCharge, AdjacentWorkObservation, RecursiveAnalysisCounters, observed_analysis_charge,
};
use super::{Closure, ObservedTemplateRetention, ParserPrediction, TemplateRetention};
use crate::grammar::parser_prediction::AnalysisConfiguration;
use crate::grammar::parser_prediction::PredictionSymbol;
use crate::grammar::parser_prediction::State;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(in crate::grammar::parser_prediction) struct CallFreeCandidates {
    entries: BTreeMap<usize, Candidate>,
}
struct Candidate {
    stop: usize,
    ready: ClosureState,
}
// A completed rejection is only template eligibility, never a parser answer.
// This discriminant occupies the old optional-closure slot; it adds no retained
// graph, result collection or allocation. Both completion states are paid before
// publication and remain tied to this exact immutable graph.
enum ClosureState {
    Uncomputed,
    CompleteIneligible,
    Ready(Closure),
}
const _: () =
    assert!(std::mem::size_of::<ClosureState>() <= std::mem::size_of::<Option<Closure>>());
pub(in crate::grammar::parser_prediction) struct CallFreeTemplates<'graph> {
    graph: &'graph [State],
    entries: BTreeMap<usize, Candidate>,
}

impl CallFreeCandidates {
    pub(in crate::grammar::parser_prediction) fn record_rule(
        &mut self,
        entry: usize,
        stop: usize,
        saw_call: bool,
        remaining: &mut usize,
    ) -> Result<(), String> {
        self.record_rule_observed(entry, stop, saw_call, remaining, None)
    }
    pub(in crate::grammar::parser_prediction) fn record_rule_observed(
        &mut self,
        entry: usize,
        stop: usize,
        saw_call: bool,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        if saw_call {
            return Ok(());
        }
        // The bool comes from the whole owned graph body's existing Call arm.
        // Retain nothing until the original recursive pool grants this fee.
        observed_analysis_charge(remaining, exact, AdjacentCharge::CallFreeRuleRetention)?;
        match self.entries.entry(entry) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(Candidate {
                    stop,
                    ready: ClosureState::Uncomputed,
                });
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                Err("Duplicate call-free prediction rule entry".into())
            }
        }
    }
    pub(in crate::grammar::parser_prediction) fn freeze(
        self,
        graph: &[State],
    ) -> CallFreeTemplates<'_> {
        CallFreeTemplates {
            graph,
            entries: self.entries,
        }
    }
}

impl CallFreeTemplates<'_> {
    /// Called only after the original incoming Call effects and depth check.
    /// `true` means this step emitted its exact token rows, never a cached
    /// recursive-conflict Boolean or an early parser commitment.
    pub(in crate::grammar::parser_prediction) fn replay_call<C: Calls>(
        &mut self,
        program: &ParserPrediction,
        row: &AnalysisConfiguration<C>,
        tokens: &mut BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration<C>>>,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        self.replay_call_observed(program, row, tokens, remaining, None)
    }
    pub(in crate::grammar::parser_prediction) fn replay_call_observed<C: Calls>(
        &mut self,
        program: &ParserPrediction,
        row: &AnalysisConfiguration<C>,
        tokens: &mut BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration<C>>>,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<bool, String> {
        self.replay_call_inner(program, row, tokens, remaining, exact, None)
    }
    pub(in crate::grammar::parser_prediction) fn replay_call_recorded<C: Calls>(
        &mut self,
        program: &ParserPrediction,
        row: &AnalysisConfiguration<C>,
        tokens: &mut BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration<C>>>,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
        measured: Option<&RecursiveAnalysisCounters>,
    ) -> Result<bool, String> {
        if measured.is_none() {
            return if exact.is_some() {
                self.replay_call_observed(program, row, tokens, remaining, exact)
            } else {
                self.replay_call(program, row, tokens, remaining)
            };
        }
        let before = *remaining;
        let result = self.replay_call_inner(program, row, tokens, remaining, exact, measured);
        if let Some(work) = measured {
            work.selected_static_template_end(
                row.state,
                before,
                *remaining,
                result.as_ref().ok().copied(),
            );
        }
        result
    }
    fn replay_call_inner<C: Calls>(
        &mut self,
        program: &ParserPrediction,
        row: &AnalysisConfiguration<C>,
        tokens: &mut BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration<C>>>,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
        measured: Option<&RecursiveAnalysisCounters>,
    ) -> Result<bool, String> {
        // Includes absence. This is the only candidate/cache-map lookup.
        observed_analysis_charge(remaining, exact, AdjacentCharge::TemplateLookup)?;
        if !std::ptr::eq(self.graph, program.states.as_slice()) {
            return Err("Relative closure cache belongs to another prediction graph".into());
        }
        if row.calls.len() > program.call_depth() {
            return Err("Parser prediction call depth exceeded".into());
        }
        let Some(candidate) = self.entries.get_mut(&row.state) else {
            if let Some(work) = measured {
                work.selected_static_template_lookup(row.state, false, false);
            }
            return Ok(false);
        };
        if let Some(work) = measured {
            work.selected_static_template_lookup(
                row.state,
                matches!(candidate.ready, ClosureState::Ready(_)),
                matches!(candidate.ready, ClosureState::Uncomputed),
            );
        }
        if matches!(candidate.ready, ClosureState::CompleteIneligible) {
            // The caller still schedules the same ordinary closure. Only the
            // already-completed, unsuccessful optimization proof is skipped.
            return Ok(false);
        }
        if matches!(candidate.ready, ClosureState::Uncomputed) {
            // One relative seed is retained before the shared walker starts.
            observed_analysis_charge(remaining, exact, AdjacentCharge::TemplateSeed)?;
            let configurations = [AnalysisConfiguration {
                state: row.state,
                alternative: 0,
                calls: Vec::new(),
            }]
            .into();
            let closure = if exact.is_some() {
                program.proof_closure_with_retention(
                    configurations,
                    true,
                    remaining,
                    0,
                    true,
                    ObservedTemplateRetention(exact),
                )
            } else {
                program.proof_closure_with_retention(
                    configurations,
                    true,
                    remaining,
                    0,
                    true,
                    TemplateRetention,
                )
            }?;
            // A depth-zero walk cannot hide a nested Call. Boundary returns
            // stay relative: no actual callerless FOLLOW is read here.
            if !closure.accepting.is_empty()
                || !closure.returned.is_empty()
                || closure.tokens.is_empty()
            {
                // No stop lookup or callerless FOLLOW is introduced here: the
                // original ineligible path returned before either operation.
                // A denied publication leaves Uncomputed and cannot hide a
                // failed/partial proof on a later request.
                observed_analysis_charge(remaining, exact, AdjacentCharge::TemplatePublication)?;
                candidate.ready = ClosureState::CompleteIneligible;
                return Ok(false);
            }
            // Check the actual builder-owned stop before publishing. It is not
            // a source rule inferred from a state index or external metadata.
            observed_analysis_charge(remaining, exact, AdjacentCharge::TemplateStopCheck)?;
            let stop = program
                .states
                .get(candidate.stop)
                .ok_or("Dangling call-free prediction rule stop")?;
            if !stop.rule_stop {
                return Err("Invalid call-free prediction rule stop".into());
            }
            // Complete-only publication: a denied fee leaves Uncomputed.
            observed_analysis_charge(remaining, exact, AdjacentCharge::TemplatePublication)?;
            candidate.ready = ClosureState::Ready(closure);
        }
        let ClosureState::Ready(closure) = &candidate.ready else {
            return Err("Missing complete relative closure".into());
        };
        for (symbol, targets) in &closure.tokens {
            for target in targets {
                // Pay before either symbol or bounded entire-prefix clone and
                // before insertion. Partial failed replay receives no refund.
                observed_analysis_charge(remaining, exact, AdjacentCharge::TemplateEmission)?;
                if let Some(work) = measured {
                    work.selected_static_template_row(row.state);
                }
                if !target.calls.is_empty() {
                    return Err("Call-free relative closure retained internal calls".into());
                }
                tokens
                    .entry(symbol.clone())
                    .or_default()
                    .insert(AnalysisConfiguration {
                        state: target.state,
                        alternative: row.alternative,
                        calls: row.calls.clone(),
                    });
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
