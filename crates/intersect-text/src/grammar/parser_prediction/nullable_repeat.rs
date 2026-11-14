//! Explicit productive whole-Full repeat capability on the SAME completion engine.
use super::{
    CompletionProductivity, CompletionVerification, Decision, DecisionBoundary, Edge,
    ParserPrediction, PredictionGoal, PredictionInput, PredictionOutcome, PredictionSymbol,
    RuleLocalCompletionDecision, analysis_charge, charge,
};

#[derive(Debug)]
pub(super) enum DeferredChoiceProof {
    Unsupported,
    RequiresRuntimeBoundary,
    Qualified,
}

impl ParserPrediction {
    fn nullable_full_operand(
        &self,
        path: &str,
        callee: &str,
        remaining: &mut usize,
    ) -> Result<(usize, usize), String> {
        analysis_charge(remaining)?;
        let decision = self
            .decisions
            .get(path)
            .ok_or("Unknown productive nullable repeat decision")?;
        let Decision {
            entries,
            exit: Some(1),
        } = decision
        else {
            return Err("Productive nullable repeat needs its exact ordered exit decision".into());
        };
        let [body, end] = entries.as_slice() else {
            return Err("Productive nullable repeat needs exactly body and exit entries".into());
        };
        analysis_charge(remaining)?;
        let occurrence = self
            .decisions
            .get(&format!(
                "{}/recovery",
                path.strip_suffix("/repeat")
                    .ok_or("Productive nullable repeat path has no repeat suffix")?
            ))
            .ok_or("Productive nullable repeat has no original occurrence")?;
        let [start, actual_end] = occurrence.entries.as_slice() else {
            return Err("Invalid productive nullable repeat occurrence".into());
        };
        if occurrence.exit.is_some() || actual_end != end || start == end {
            return Err("Productive nullable repeat occurrence boundary differs".into());
        }
        analysis_charge(remaining)?;
        if !matches!(self.states.get(*start).ok_or("Dangling nullable repeat entry")?.edges.as_slice(), [Edge::Epsilon(actual_body), Edge::Epsilon(actual_exit)] if actual_body == body && actual_exit == end)
        {
            return Err("Productive nullable repeat fan-out differs from its source order".into());
        }
        analysis_charge(remaining)?;
        let (target, stop) = match self
            .states
            .get(*body)
            .ok_or("Dangling nullable repeat body")?
            .edges
            .as_slice()
        {
            [Edge::Full { target, stop }] => (*target, *stop),
            _ => {
                return Err(
                    "Productive nullable repeat needs the entire visible Full operand".into(),
                );
            }
        };
        analysis_charge(remaining)?;
        if !matches!(self.states.get(stop).ok_or("Dangling nullable operand stop")?.edges.as_slice(), [Edge::Epsilon(back)] if back == start)
        {
            return Err(
                "Productive nullable repeat stop does not return to its actual loop entry".into(),
            );
        }
        analysis_charge(remaining)?;
        let &(rule_start, rule_stop) = self
            .rules
            .get(callee)
            .ok_or("Unknown productive nullable repeat callee")?;
        analysis_charge(remaining)?;
        if !matches!(self.states.get(target).ok_or("Dangling nullable operand call")?.edges.as_slice(), [Edge::Call {target: actual_target, follow}] if *actual_target == rule_start && *follow == stop)
        {
            return Err(
                "Productive nullable repeat differs from its exact callee/return context".into(),
            );
        }
        analysis_charge(remaining)?;
        let returned = self
            .states
            .get(rule_stop)
            .ok_or("Dangling nullable callee stop")?;
        if !returned.rule_stop {
            return Err("Productive nullable callee has no actual rule stop".into());
        }
        let mut present = false;
        for follow in &returned.follow {
            analysis_charge(remaining)?;
            if *follow == stop {
                present = true;
                break;
            }
        }
        if !present {
            return Err("Productive nullable call has no matching actual return".into());
        }
        Ok((target, stop))
    }

    pub(super) fn verify_productive_nullable_full_repeat(
        &self,
        path: &str,
        callee: &str,
        keyword: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        let (start, stop) = self.nullable_full_operand(path, callee, remaining)?;
        analysis_charge(remaining)?;
        let optional_path = format!("rule/{callee}/s1/repeat");
        let local =
            self.nullable_keyword_local_decision(&optional_path, callee, keyword, remaining)?;
        // Do not waive an optional/fan-out/guard proof. Failure remains explicit.
        self.verify_completion_scope_with_local_decision(
            start,
            stop,
            remaining,
            CompletionVerification::GuardOverapproximation,
            Some(local),
        )?;
        Ok(())
    }

    fn nullable_keyword_optional_binding<'p>(
        &'p self,
        path: &str,
        owner: &str,
        keyword: &str,
        remaining: &mut usize,
    ) -> Result<(&'p Decision, usize, usize), String> {
        analysis_charge(remaining)?;
        let &(rule_start, rule_stop) = self
            .rules
            .get(owner)
            .ok_or("Unknown bounded nullable keyword rule")?;
        analysis_charge(remaining)?;
        let decision = self
            .decisions
            .get(path)
            .ok_or("Unknown bounded nullable keyword decision")?;
        let [body, end] = decision.entries.as_slice() else {
            return Err("Bounded keyword optional needs ordered body/exit entries".into());
        };
        if decision.exit != Some(1) || *end != rule_stop {
            return Err("Bounded keyword optional differs from its actual rule stop".into());
        }
        analysis_charge(remaining)?;
        let occurrence = self
            .decisions
            .get(&format!(
                "{}/recovery",
                path.strip_suffix("/repeat")
                    .ok_or("Bounded keyword optional has no actual occurrence")?
            ))
            .ok_or("Bounded keyword optional lacks its original occurrence")?;
        let [start, actual_end] = occurrence.entries.as_slice() else {
            return Err("Invalid bounded keyword optional occurrence".into());
        };
        if occurrence.exit.is_some() || actual_end != end {
            return Err("Bounded keyword optional occurrence escapes the rule".into());
        }
        analysis_charge(remaining)?;
        if !matches!(self.states.get(rule_start).ok_or("Dangling bounded keyword rule entry")?.edges.as_slice(), [Edge::Action(target)] if target == start)
        {
            return Err("Bounded keyword rule differs from its exact local action".into());
        }
        analysis_charge(remaining)?;
        if !matches!(self.states.get(*start).ok_or("Dangling bounded keyword optional start")?.edges.as_slice(), [Edge::Epsilon(actual_body),Edge::Epsilon(actual_end)] if actual_body==body && actual_end==end)
        {
            return Err("Bounded keyword optional fan-out differs from its source".into());
        }
        analysis_charge(remaining)?;
        let done = match self
            .states
            .get(*body)
            .ok_or("Dangling bounded keyword optional body")?
            .edges
            .as_slice()
        {
            [Edge::Token(PredictionSymbol::Keyword(actual), target)] if actual == keyword => {
                *target
            }
            _ => {
                return Err(
                    "Bounded keyword optional differs from its exact consuming keyword".into(),
                );
            }
        };
        analysis_charge(remaining)?;
        if !matches!(self.states.get(done).ok_or("Dangling bounded keyword optional completion")?.edges.as_slice(),[Edge::Epsilon(target)] if target == end)
        {
            return Err("Bounded keyword optional completion escapes its rule".into());
        }
        analysis_charge(remaining)?;
        if !self
            .states
            .get(rule_stop)
            .ok_or("Dangling bounded keyword rule stop")?
            .rule_stop
        {
            return Err("Bounded keyword optional needs its actual rule stop".into());
        }
        Ok((decision, *start, rule_stop))
    }

    fn nullable_keyword_local_decision<'p>(
        &'p self,
        path: &'p str,
        owner: &str,
        keyword: &str,
        remaining: &mut usize,
    ) -> Result<RuleLocalCompletionDecision<'p>, String> {
        let (decision, entry, stop) =
            self.nullable_keyword_optional_binding(path, owner, keyword, remaining)?;
        Ok(RuleLocalCompletionDecision {
            path,
            decision,
            entry,
            stop,
        })
    }

    #[cfg(test)]
    pub(in crate::grammar) fn verify_nullable_deferred_decision(
        &self,
        path: &str,
        grammar: &super::Grammar,
    ) -> Result<(), String> {
        // Explicit historical regression wrapper; production lends its phase pool.
        self.verify_nullable_deferred_decision_with_remaining(path, grammar, &mut 10_000)
    }
    pub(in crate::grammar) fn verify_nullable_deferred_decision_with_remaining(
        &self,
        path: &str,
        grammar: &super::Grammar,
        remaining: &mut usize,
    ) -> Result<(), String> {
        if let Some(proof) =
            grammar.qualified_nullable_keyword_optional_with_work(path, remaining)?
        {
            self.nullable_keyword_local_decision(path, proof.owner(), proof.keyword(), remaining)?;
            Ok(())
        } else {
            self.verify_disjoint_with_discovery_remaining(path, remaining, Some(2000))
        }
    }

    // Selected ONLY by the actual InputChecked runtime companion in the fresh
    // compiler. This is a per-input replay obligation, not global disjointness,
    // a repeat record, or authority for a serialized/untyped program.
    pub(in crate::grammar) fn verify_input_checked_deferred_decision_with_remaining(
        &self,
        path: &str,
        grammar: &super::Grammar,
        remaining: &mut usize,
    ) -> Result<(), String> {
        match self.input_checked_source_choice(path, grammar, remaining)? {
            DeferredChoiceProof::Qualified => Ok(()),
            proof => {
                // Eligibility declines keep the literal old proof and SAME
                // actual remaining pool. Qualification errors never retry.
                let old =
                    self.verify_nullable_deferred_decision_with_remaining(path, grammar, remaining);
                if matches!(proof, DeferredChoiceProof::RequiresRuntimeBoundary) {
                    old.map_err(|message| format!("{message}; final recursive-first metadata requires an exact runtime completion boundary at {path}"))
                } else {
                    old
                }
            }
        }
    }

    pub(super) fn input_checked_source_choice(
        &self,
        path: &str,
        grammar: &super::Grammar,
        remaining: &mut usize,
    ) -> Result<DeferredChoiceProof, String> {
        self.input_checked_source_choice_inner(path, grammar, remaining, false)
    }

    pub(super) fn input_checked_runtime_source_choice(
        &self,
        path: &str,
        grammar: &super::Grammar,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        source_choice_lookup_work(self.recursive_decisions.len(), path.len(), remaining)?;
        if !self.recursive_decisions.contains(path) {
            return Ok(false);
        }
        Ok(matches!(
            self.input_checked_source_choice_inner(path, grammar, remaining, true)?,
            DeferredChoiceProof::Qualified
        ))
    }

    fn input_checked_source_choice_inner(
        &self,
        path: &str,
        grammar: &super::Grammar,
        remaining: &mut usize,
        allow_runtime_boundary: bool,
    ) -> Result<DeferredChoiceProof, String> {
        analysis_charge(remaining)?;
        if grammar.profile != super::super::Profile::NativeV80 || !path.ends_with("/choice") {
            return Ok(DeferredChoiceProof::Unsupported);
        }
        // A cheap MAY filter avoids traversing whole owners for unrelated
        // choices. It grants no proof; all endpoints and fanout are checked by
        // the same completion-scope verifier and guarded-choice witness below.
        source_choice_lookup_work(self.decisions.len(), path.len(), remaining)?;
        let decision = self
            .decisions
            .get(path)
            .ok_or("Unknown deferred input-checked choice")?;
        let multiway = decision.entries.len() >= 3 && decision.exit.is_none();
        if multiway && !allow_runtime_boundary {
            return Ok(DeferredChoiceProof::RequiresRuntimeBoundary);
        }
        if !multiway && decision.entries.len() != 2 {
            return Ok(DeferredChoiceProof::Unsupported);
        }
        let plain = decision
            .entries
            .first()
            .ok_or("Missing deferred choice entry")?;
        let guarded = decision
            .entries
            .get(1)
            .ok_or("Missing deferred guarded choice entry")?;
        analysis_charge(remaining)?;
        let plain = self
            .states
            .get(*plain)
            .ok_or("Dangling deferred plain choice entry")?;
        analysis_charge(remaining)?;
        let guarded = self
            .states
            .get(*guarded)
            .ok_or("Dangling deferred guarded choice entry")?;
        if !multiway
            && (decision.exit.is_some()
                || !matches!(plain.edges.as_slice(), [Edge::Call { .. }])
                || !matches!(
                    guarded.edges.as_slice(),
                    [Edge::Full { .. }] | [Edge::First { .. }]
                ))
        {
            return Ok(DeferredChoiceProof::Unsupported);
        }
        source_choice_lookup_work(self.recursive_decisions.len(), path.len(), remaining)?;
        if self.recursive_decisions.contains(path) && !allow_runtime_boundary {
            return Ok(DeferredChoiceProof::RequiresRuntimeBoundary);
        }
        let base = path
            .strip_suffix("/choice")
            .ok_or("Missing deferred choice suffix")?;
        let Some((owner, rule, expression)) =
            super::super::nullable_repetition::prefix_source_occurrence_with_work(
                grammar, base, remaining,
            )?
        else {
            return Ok(DeferredChoiceProof::Unsupported);
        };
        analysis_charge(remaining)?;
        if expression.cardinality != '1'
            || expression.first_set_predicated
            || !matches!(&expression.kind, super::super::Kind::Alternative(items) if items.len() == decision.entries.len())
        {
            return Ok(DeferredChoiceProof::Unsupported);
        }
        source_choice_lookup_work(grammar.declaration_sources.len(), owner.len(), remaining)?;
        let source = grammar
            .declaration_sources
            .get(owner)
            .ok_or("Deferred choice requires original owner source")?;
        for expression in [&rule.body, expression] {
            analysis_charge(remaining)?;
            let range = expression
                .original_range
                .as_ref()
                .ok_or("Deferred choice lacks original source interval")?;
            if source.start >= source.end
                || range.start >= range.end
                || range.start < source.start
                || range.end > source.end
            {
                return Err("Deferred choice source interval differs from its owner".into());
            }
        }
        // Same source-scoped hidden policy required by runtime completion
        // admission. An override remains on the existing conservative route.
        let hidden = grammar.hidden.as_deref().unwrap_or(&[]);
        if let Some(actual) = rule.hidden.as_deref() {
            analysis_charge(remaining)?;
            for (left, right) in actual.iter().zip(hidden) {
                for _ in 0..(1 + left.len().max(right.len()).div_ceil(64)) {
                    analysis_charge(remaining)?;
                }
            }
            if actual != hidden {
                return Ok(DeferredChoiceProof::Unsupported);
            }
        }
        source_choice_lookup_work(self.rules.len(), owner.len(), remaining)?;
        let &(start, stop) = self
            .rules
            .get(owner)
            .ok_or("Deferred choice has no actual prediction owner")?;
        let (scope, reached) = self.verify_completion_scope_with_reach(
            start,
            stop,
            remaining,
            CompletionVerification::InputCheckedCompletion,
            None,
        )?;
        if !(if multiway {
            self.input_checked_multiway_choice(path, decision, &reached, remaining)?
        } else {
            self.input_checked_asymmetric_guarded_choice(path, decision, &reached, remaining)?
        }) {
            return Ok(DeferredChoiceProof::Unsupported);
        }
        for bound in scope.decisions.values() {
            for _ in 0..(1 + bound.path.len().max(path.len()).div_ceil(64)) {
                analysis_charge(remaining)?;
            }
            if bound.path == path {
                analysis_charge(remaining)?;
                if !std::ptr::eq(bound.decision, decision) {
                    return Err("Deferred choice differs from exact completion binding".into());
                }
                return Ok(DeferredChoiceProof::Qualified);
            }
        }
        Err("Deferred choice lacks an exact owner completion binding".into())
    }

    pub(in crate::grammar) fn verify_required_object_call_with_nullable_full_source(
        &self,
        grammar: &super::Grammar,
        owner: &str,
        path: &str,
        callee: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        self.verify_required_object_call_with_source_companion(
            grammar, owner, path, callee, remaining, None,
        )
    }

    pub(in crate::grammar) fn verify_required_object_call_with_source_companion(
        &self,
        grammar: &super::Grammar,
        owner: &str,
        path: &str,
        callee: &str,
        remaining: &mut usize,
        companion: Option<&mut super::RuntimeGuardedAdmission>,
    ) -> Result<(), String> {
        let Some((proof, repeat_path)) = grammar
            .qualified_required_nullable_full_call_with_work(owner, path, callee, remaining)?
        else {
            return if let Some(companion) = companion {
                companion.verify_required_input_checked_call(
                    grammar, self, owner, path, callee, remaining,
                )
            } else {
                self.verify_required_object_call(path, callee, remaining)
            };
        };
        // Shared ordinary binding checks the actual mandatory Call occurrence,
        // callee, real return and exact stop before the stronger proof is used.
        let (start, end) = self.required_object_call_binding(path, callee, remaining)?;
        let (full_start, full_stop) =
            self.nullable_full_operand(&repeat_path, callee, remaining)?;
        analysis_charge(remaining)?;
        if start != full_start || end != full_stop {
            return Err(
                "Required nullable Call differs from its productive whole-Full boundary".into(),
            );
        }
        analysis_charge(remaining)?;
        let optional_path = format!("rule/{callee}/s1/repeat");
        let local = self.nullable_keyword_local_decision(
            &optional_path,
            callee,
            proof.keyword(),
            remaining,
        )?;
        self.verify_completion_scope_with_local_decision(
            start,
            end,
            remaining,
            CompletionVerification::GuardOverapproximation,
            Some(local),
        )?;
        Ok(())
    }

    pub(super) fn verify_nullable_keyword_optional_binding(
        &self,
        path: &str,
        owner: &str,
        keyword: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        self.nullable_keyword_optional_binding(path, owner, keyword, remaining)?;
        Ok(())
    }

    /// Exact-rule local decision capability. The new checked profile source
    /// qualifies this action+optional-keyword rule; other callers of that rule
    /// intentionally receive this local greedy choice too. No caller suffix is read.
    pub fn predict_rule_bounded_keyword_optional_with_replay_input(
        &self,
        path: &str,
        owner: &str,
        keyword: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String> {
        let work = remaining
            .as_mut()
            .ok_or("Bounded nullable keyword optional requires a finite shared work budget")?;
        let (decision, _, stop) =
            self.nullable_keyword_optional_binding(path, owner, keyword, work)?;
        match self.run_prediction_at_depth_with_completion_verification(
            PredictionGoal::Decision {
                decision,
                recursive_first: self.recursive_decisions.contains(path),
                boundary: Some(DecisionBoundary {
                    live: false,
                    stop,
                    returns: &[],
                }),
            },
            input,
            remaining,
            0,
            verification,
        )? {
            PredictionOutcome::Selected(selected) => Ok(selected),
            PredictionOutcome::Completed(_) | PredictionOutcome::InitialPrepared(_) => {
                Err("Bounded keyword optional returned completion".into())
            }
        }
    }

    /// Explicit low-level capability; a checked language host must additionally
    /// qualify its canonical authored nullable whole-Full source occurrence.
    /// Uses no second recognizer, model action, token tape or new allowance.
    pub fn predict_productive_nullable_full_repeat_with_replay_input(
        &self,
        path: &str,
        callee: &str,
        keyword: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String> {
        let work = remaining
            .as_mut()
            .ok_or("Productive nullable repeat requires a finite shared work budget")?;
        let (start, stop) = self.nullable_full_operand(path, callee, work)?;
        analysis_charge(work)?;
        let optional_path = format!("rule/{callee}/s1/repeat");
        let local = self.nullable_keyword_local_decision(&optional_path, callee, keyword, work)?;
        let scope = self.verify_completion_scope_with_local_decision(
            start,
            stop,
            work,
            verification,
            Some(local),
        )?;
        let checkpoint = input.checkpoint()?;
        // Reserve cleanup before any speculative input change; never refund it.
        charge(remaining)?;
        let result = (|| {
            charge(remaining)?;
            self.run_prediction_at_depth_with_local_decision(
                PredictionGoal::Completion {
                    start,
                    stop,
                    scope: &scope,
                    productivity: CompletionProductivity::MustConsumeSourceToken,
                },
                input,
                remaining,
                0,
                verification,
                Some(local),
            )
        })();
        let restored = input.rewind(checkpoint);
        if let Err(error) = restored {
            return Err(match result {
                Err(original) => format!("{original}; productive repeat rewind failed: {error}"),
                Ok(_) => error,
            });
        }
        match result? {
            PredictionOutcome::Completed(true) => Ok(Some(0)),
            PredictionOutcome::Completed(false) => Ok(None),
            PredictionOutcome::InitialPrepared(_) => {
                Err("Unexpected initial preparation outcome for productive completion".into())
            }
            PredictionOutcome::Selected(_) => {
                Err("Productive completion returned a decision".into())
            }
        }
    }
}

fn source_choice_lookup_work(
    count: usize,
    key_bytes: usize,
    remaining: &mut usize,
) -> Result<(), String> {
    let levels = usize::BITS as usize - count.max(1).leading_zeros() as usize;
    let work = levels
        .checked_mul(2)
        .and_then(|n| n.checked_mul(1 + key_bytes.div_ceil(64)))
        .and_then(|n| n.checked_add(1))
        .ok_or("Deferred source-choice lookup work overflow")?;
    for _ in 0..work {
        analysis_charge(remaining)?;
    }
    Ok(())
}
