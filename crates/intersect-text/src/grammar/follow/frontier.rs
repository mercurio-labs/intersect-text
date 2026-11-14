//! Source-bound symbolic frontiers retained by the existing FOLLOW walk.
//! Temporary path/entry counts are paid work units, not allocator-byte bounds.
use super::{RequiredObjectCall, Tokens, WalkObservation, apply_conflict, charge};
use crate::grammar::AnalysisObservation;
use crate::grammar::Expr;
use crate::grammar::Grammar;
use crate::grammar::overlaps;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrontierTrigger {
    Always,
    Caller,
}

struct ValidationFrontier<'g> {
    owner: &'g str,
    expression: &'g Expr,
    path: String,
    trigger: FrontierTrigger,
}

pub(super) struct ValidationPlan<'g> {
    grammar: &'g Grammar,
    frontiers: Vec<ValidationFrontier<'g>>,
    repeats: Option<super::repeat_certificates::RepeatCertificates>,
    runtime: Option<&'g mut super::super::RuntimeGuardedAdmission>,
    work_budget: super::super::GrammarAnalysisWorkBudget,
}

impl<'g> ValidationPlan<'g> {
    pub(super) fn new(grammar: &'g Grammar) -> Self {
        Self {
            grammar,
            frontiers: Vec::new(),
            repeats: None,
            runtime: None,
            work_budget: grammar.analysis_work_budget,
        }
    }

    pub(super) fn with_repeats(grammar: &'g Grammar) -> Self {
        Self {
            grammar,
            frontiers: Vec::new(),
            repeats: Some(super::repeat_certificates::RepeatCertificates::default()),
            runtime: None,
            work_budget: grammar.analysis_work_budget,
        }
    }
    pub(super) fn with_runtime_repeats(
        grammar: &'g Grammar,
        runtime: &'g mut super::super::RuntimeGuardedAdmission,
    ) -> Self {
        let mut plan = Self::with_repeats(grammar);
        plan.work_budget = runtime.work_budget();
        if matches!(
            runtime.policy(),
            super::super::RuntimeGuardedPolicy::StrictCompletionScopes
                | super::super::RuntimeGuardedPolicy::InputCheckedCompletionScopes
        ) {
            plan.runtime = Some(runtime);
        }
        plan
    }
    pub(super) fn work_budget(&self) -> super::super::GrammarAnalysisWorkBudget {
        self.work_budget
    }
    pub(super) fn take_repeats(
        &mut self,
    ) -> Result<super::repeat_certificates::RepeatCertificates, String> {
        self.repeats
            .take()
            .ok_or_else(|| "Missing source repeat certificate collector".into())
    }
    pub(super) fn record_repeat(
        &mut self,
        expr: &Expr,
        grammar: &Grammar,
        first: (
            &BTreeSet<String>,
            super::super::first_sets::OccurrenceFacts,
            &Tokens,
        ),
        source: (Option<&str>, &str),
        continuation: Option<(&[Expr], usize, &str)>,
        budget: &mut usize,
    ) -> Result<(), String> {
        if let Some(repeats) = self.repeats.as_mut() {
            repeats.record(expr, grammar, first, source, continuation, budget)?;
            if let Some(runtime) = self.runtime.as_deref_mut() {
                repeats.record_nonnullable(expr, grammar, first.1, source, budget, runtime)?;
            }
        }
        Ok(())
    }

    pub(super) fn record(
        &mut self,
        (expr, grammar): (&'g Expr, &'g Grammar),
        outer: &Tokens,
        (head, nullable): (&BTreeSet<String>, bool),
        (owner, path): (Option<&'g str>, &str),
        budget: &mut usize,
    ) -> Result<(), String> {
        let repeated = matches!(expr.cardinality, '?' | '*' | '+');
        if !(nullable || repeated)
            || head.is_empty()
            || (grammar.profile.has_parser_prediction()
                && crate::grammar::visible_entry_guard(expr))
            // A repeat always precedes the structural-wrapper exemption.
            || (!repeated
                && crate::grammar::is_structural_nullable_continuation(expr, grammar))
        {
            return Ok(());
        }
        let trigger = if overlaps(head, &outer.symbols) {
            FrontierTrigger::Always
        } else if outer.caller {
            FrontierTrigger::Caller
        } else {
            return Ok(());
        };
        // No symbols are retained. Pay before any retained path clone or push.
        charge(budget)?;
        if !std::ptr::eq(self.grammar, grammar) {
            return Err("FOLLOW frontier belongs to a different grammar".into());
        }
        let owner = owner.ok_or("FOLLOW frontier has no actual validation owner")?;
        let (canonical, _) = self
            .grammar
            .rules
            .get_key_value(owner)
            .ok_or_else(|| format!("Unknown FOLLOW frontier owner: {owner}"))?;
        self.frontiers.push(ValidationFrontier {
            owner: canonical,
            expression: expr,
            path: path.into(),
            trigger,
        });
        Ok(())
    }

    pub(super) fn replay(
        &self,
        follow: &BTreeMap<String, Tokens>,
        deferred: &mut BTreeSet<String>,
        mut required: Option<&mut Vec<RequiredObjectCall>>,
        budget: &mut usize,
        observation: Option<AnalysisObservation<'_>>,
    ) -> Result<(), String> {
        for frontier in &self.frontiers {
            // Pay before reading the owner/key or acting on a retained record.
            charge(budget)?;
            if !self.grammar.rules.contains_key(frontier.owner) {
                return Err(format!("Unknown FOLLOW frontier owner: {}", frontier.owner));
            }
            let continuation = follow
                .get(frontier.owner)
                .ok_or_else(|| format!("Missing resolved FOLLOW frontier: {}", frontier.owner))?;
            let conflict = match frontier.trigger {
                FrontierTrigger::Always => true,
                FrontierTrigger::Caller => {
                    // This exact query already succeeded during the shared walk.
                    // FIRST retains its independent work/syntax/call ceiling.
                    let (head, _) = super::super::first_with_work_budget(
                        frontier.expression,
                        self.grammar,
                        0,
                        self.work_budget,
                    )?;
                    overlaps(&head, &continuation.symbols)
                }
            };
            if conflict {
                apply_conflict(
                    frontier.expression,
                    self.grammar,
                    (
                        &frontier.path,
                        observation.map(|context| WalkObservation {
                            context,
                            owner: frontier.owner,
                        }),
                        Some(frontier.owner),
                    ),
                    budget,
                    (deferred, required.as_deref_mut()),
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "frontier_tests.rs"]
mod tests;
