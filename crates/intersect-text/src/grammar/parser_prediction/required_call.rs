//! Transient qualification of one actual required object-call occurrence.
use super::{CompletionGuard, Edge, ParserPrediction, analysis_charge};

impl ParserPrediction {
    pub(super) fn required_object_call_binding(
        &self,
        path: &str,
        callee: &str,
        remaining: &mut usize,
    ) -> Result<(usize, usize), String> {
        analysis_charge(remaining)?;
        let occurrence = self
            .decisions
            .get(&format!("{path}/recovery"))
            .ok_or("Missing source occurrence for required object call")?;
        let (start, end) = match occurrence.entries.as_slice() {
            [start, end] if occurrence.exit.is_none() && start != end => (*start, *end),
            _ => return Err("Invalid required object call occurrence boundary".into()),
        };
        analysis_charge(remaining)?;
        let &(target, stop) = self
            .rules
            .get(callee)
            .ok_or("Unknown required object call callee")?;
        analysis_charge(remaining)?;
        let entry = self
            .states
            .get(start)
            .ok_or("Dangling required object call entry")?;
        match entry.edges.as_slice() {
            [
                Edge::Call {
                    target: actual_target,
                    follow,
                },
            ] if *actual_target == target && *follow == end => {}
            _ => {
                return Err(
                    "Required object call differs from its actual callee or continuation".into(),
                );
            }
        }
        analysis_charge(remaining)?;
        let returned = self
            .states
            .get(stop)
            .ok_or("Dangling required object call return")?;
        if !returned.rule_stop {
            return Err("Required object call callee has no actual rule return".into());
        }
        let mut real_return = false;
        for continuation in &returned.follow {
            analysis_charge(remaining)?;
            if *continuation == end {
                real_return = true;
                break;
            }
        }
        if !real_return {
            return Err("Required object call continuation is absent from its real return".into());
        }
        Ok((start, end))
    }
    pub(in crate::grammar) fn verify_required_object_call(
        &self,
        path: &str,
        callee: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        let (start, end) = self.required_object_call_binding(path, callee, remaining)?;
        // Start at the real call edge, not the callee entry. That edge supplies
        // the actual return continuation. The exact caller occurrence ends this
        // operand; recovery entries are never interpreted as choice branches.
        let scope = self.verify_completion_scope(start, end, remaining)?;
        // This first slice requires a qualified Full-guarded exit fan-out.
        // All other reachable fan-outs were already qualified by the one shared
        // scope verifier. A merely structural call lookup cannot admit a rule.
        self.verify_required_full_exit(&scope, remaining)
    }

    pub(super) fn verify_required_full_exit(
        &self,
        scope: &super::CompletionScope<'_>,
        remaining: &mut usize,
    ) -> Result<(), String> {
        for bound in scope.decisions.values() {
            analysis_charge(remaining)?;
            if bound.decision.exit.is_some() {
                analysis_charge(remaining)?;
                if let Some(first) = bound.decision.entries.first()
                    && matches!(scope.guards.get(first), Some(CompletionGuard::Full))
                {
                    return Ok(());
                }
            }
        }
        Err("Required nullable object call needs a qualified Full-guarded exit decision".into())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Decision, PredictionSymbol, State};
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn program() -> ParserPrediction {
        let mut states = vec![State::default(); 9];
        states[0].edges = vec![Edge::Call {
            target: 2,
            follow: 1,
        }];
        states[1].edges = vec![Edge::Token(PredictionSymbol::Keyword("after".into()), 8)];
        states[2].edges = vec![Edge::Epsilon(3), Edge::Epsilon(4)];
        states[3].edges = vec![Edge::Full { target: 5, stop: 6 }];
        states[4].edges = vec![Edge::Action(7)];
        states[5].edges = vec![Edge::Token(PredictionSymbol::Keyword("g".into()), 6)];
        states[6].edges = vec![Edge::Token(PredictionSymbol::Keyword(".".into()), 4)];
        states[7].rule_stop = true;
        states[7].follow = vec![1];
        states[8].rule_stop = true;
        ParserPrediction {
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            allow_empty_choices: true,
            states,
            rules: [("Target".into(), (2, 7)), ("Root".into(), (0, 8))].into(),
            decisions: [
                (
                    "rule/Root/s1/value/recovery".into(),
                    Decision {
                        entries: vec![0, 1],
                        exit: None,
                    },
                ),
                (
                    "rule/Target/s0/repeat".into(),
                    Decision {
                        entries: vec![3, 4],
                        exit: Some(4),
                    },
                ),
            ]
            .into(),
            first_guards: BTreeMap::new(),
            recursive_decisions: BTreeSet::new(),
            runtime_requirement: None,
        }
    }

    #[test]
    fn required_call_certificate_has_literal_shared_work_and_no_error_cache() {
        let program = program();
        let before = serde_json::to_value(&program).unwrap();
        // Five actual occurrence/callee/edge/return checks; the shared scope
        // consumes 16 reach + 15 fan-out + 8 binding/visibility units; final
        // qualified exit inspection is two. No independent allowance is added.
        for _ in 0..2 {
            let mut short = 45;
            assert_eq!(
                program.verify_required_object_call("rule/Root/s1/value", "Target", &mut short),
                Err("Grammar prediction budget exceeded".into())
            );
            assert_eq!(short, 0);
            let mut exact = 46;
            assert_eq!(
                program.verify_required_object_call("rule/Root/s1/value", "Target", &mut exact),
                Ok(())
            );
            assert_eq!(exact, 0);
        }
        assert_eq!(serde_json::to_value(&program).unwrap(), before);
    }

    #[test]
    fn required_call_certificate_rejects_forged_callee_return_and_occurrence_with_zero_precedence()
    {
        let mut program = program();
        let path = "rule/Root/s1/value";
        program.states[0].edges = vec![Edge::Call {
            target: 3,
            follow: 1,
        }];
        assert_eq!(
            program.verify_required_object_call(path, "Target", &mut 0),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(
            program.verify_required_object_call(path, "Target", &mut 100),
            Err("Required object call differs from its actual callee or continuation".into())
        );
        program.states[0].edges = vec![Edge::Call {
            target: 2,
            follow: 1,
        }];
        program.states[7].follow = vec![8];
        assert_eq!(
            program.verify_required_object_call(path, "Target", &mut 100),
            Err("Required object call continuation is absent from its real return".into())
        );
        program
            .decisions
            .get_mut(&format!("{path}/recovery"))
            .unwrap()
            .exit = Some(1);
        assert_eq!(
            program.verify_required_object_call(path, "Target", &mut 100),
            Err("Invalid required object call occurrence boundary".into())
        );
    }

    #[test]
    fn unrelated_full_guard_does_not_mask_an_unbound_nullable_frontier() {
        let mut program = program();
        program.states.push(State {
            edges: vec![Edge::Token(PredictionSymbol::Keyword("after".into()), 7)],
            ..State::default()
        });
        program.states[4].edges = vec![Edge::Epsilon(9), Edge::Epsilon(7)];
        // This distinct interacting fan-out deliberately lacks source-bound
        // decision metadata. The unrelated visible Full guard remains valid.
        let result =
            program.verify_required_object_call("rule/Root/s1/value", "Target", &mut 1_000);
        assert_eq!(
            result,
            Err("Completion contains an unbound branch state".into())
        );
    }

    #[test]
    fn required_call_certificate_does_not_accept_a_structural_call_without_qualified_guard() {
        let mut program = program();
        program.states[2].edges = vec![Edge::Action(4)];
        program.decisions.remove("rule/Target/s0/repeat");
        assert_eq!(
            program.verify_required_object_call("rule/Root/s1/value", "Target", &mut 1_000),
            Err(
                "Required nullable object call needs a qualified Full-guarded exit decision".into()
            )
        );
    }
}
