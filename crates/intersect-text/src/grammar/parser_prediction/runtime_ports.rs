//! Explicit eager pure-graph macrosteps over the SAME runtime worker.
//! Effect ports are executed there, never interpreted by the graph kernel.
use super::super::transparent_ports::Summaries;
use super::*;
use std::collections::VecDeque;

/// Invocation-local graph summaries for an explicitly selected runtime policy.
/// This scope borrows the exact immutable program, raw source identity and
/// hidden policy. It retains no input token, recognition result or model value.
/// Pure Epsilon/Action DAGs become eager macrosteps. This preserves effect-port
/// semantics, but can change their ordering relative to other pending rows and
/// therefore the first real error reached under finite work. Old facades never
/// select this policy. It supplies no new allowance or depth limit.
pub struct PortRuntimePredictionSource<'program> {
    source: RuntimePredictionSource<'program>,
    summaries: Summaries<'program>,
    emitted: usize,
}

impl<'program> PortRuntimePredictionSource<'program> {
    pub fn new(
        program: &'program ParserPrediction,
        source: &SourceId,
        hidden: &[String],
        remaining: &mut Option<usize>,
    ) -> Result<Self, String> {
        let mut source = RuntimePredictionSource::new(program, source, hidden, remaining)?;
        // Additional sidecar/kernel headers: logical policy, not actual RSS.
        // The kernel's own 128-byte retained header is separately lazy-paid.
        admit(remaining, &mut source.logical_bytes, 128)?;
        Ok(Self {
            source,
            summaries: Summaries::new(&program.states),
            emitted: 0,
        })
    }
    /// Completed kernel lookups, including conservative fallback rows. These
    /// are NOT recognized inputs, complete-frontier hits, or proof of expansion.
    pub fn hits(&self) -> usize {
        self.summaries.stats().hits
    }
    /// Completed graph-summary builds, NOT parser/model publications.
    pub fn publications(&self) -> usize {
        self.summaries.stats().publications
    }
    /// Cumulative logical policy units, including temporary port/row copies.
    pub fn logical_bytes(&self) -> usize {
        self.logical_total().unwrap_or(usize::MAX)
    }
    pub fn emitted_configurations(&self) -> usize {
        self.emitted
    }
    pub fn is_poisoned(&self) -> bool {
        self.source.is_poisoned()
    }
    pub fn invalidate(&mut self) {
        self.source.invalidate();
        self.summaries.invalidate();
    }
    fn logical_total(&self) -> Result<usize, String> {
        self.source
            .logical_bytes
            .checked_add(self.summaries.stats().logical_bytes)
            .ok_or_else(|| "Runtime graph-port logical admission overflow".into())
    }
}

struct PortAccess<'call, 'program> {
    source: &'call mut PortRuntimePredictionSource<'program>,
}
impl<'program> InitialSource<'program> for PortAccess<'_, 'program> {
    // This policy never replays or publishes the old first-closure cache.
    const ENABLED: bool = false;
    const PORTS: bool = true;
    fn replay(
        &mut self,
        _: &mut Frontier<'program>,
        _: &mut BTreeMap<usize, bool>,
        _: &mut Option<usize>,
    ) -> Result<bool, String> {
        Ok(false)
    }
    fn remember(
        &mut self,
        _: &[(&'program PredictionSymbol, Configuration)],
        _: &BTreeMap<usize, bool>,
        _: &mut Option<usize>,
    ) -> Result<(), String> {
        Ok(())
    }
    fn expand_ports(
        &mut self,
        row: &Configuration,
        pending: &mut VecDeque<Configuration>,
        remaining: &mut Option<usize>,
    ) -> Result<bool, String> {
        if remaining.is_none() {
            return Err("Runtime prediction sources require the original finite work pool".into());
        }
        // One shared proof, with no owned temporary descriptor vector.
        // Field separation permits queue placement while the summary is lent.
        let (summaries, emitted) = (&mut self.source.summaries, &mut self.source.emitted);
        let ports =
            summaries.borrow_ports_for_configurations(row.state, row.returns.len(), &mut || {
                charge(remaining)
            })?;
        let Some(ports) = ports else {
            return Ok(false);
        };
        if ports.is_empty() {
            return Err("Runtime graph-port summary has no effect boundary".into());
        }
        for port in ports {
            charge(remaining)?; // actual descriptor read, not waived
            let count = emitted
                .checked_add(1)
                .ok_or("Runtime graph-port emitted counter overflow")?;
            // Both complete pending/visited copies and all ordered returns
            // were prepaid before the borrowed view. Actual clone stays here.
            let mut next = row.clone();
            next.state = port.state;
            next.visible &= !port.action_seen;
            pending.push_back(next);
            *emitted = count;
        }
        self.source.logical_total()?;
        Ok(true)
    }
}

impl ParserPrediction {
    /// Explicitly select eager pure-DAG port expansion in an unbounded runtime
    /// decision. All Calls, real return stacks, FOLLOW, First/Full predicates,
    /// matching and input callbacks still execute in the same worker. Nested
    /// Full/completion uses the historical worker and exact original depth128.
    /// Macrostep scheduling can change the first finite-work error/callback
    /// order; it is not a silent Native67 behavior or successful-result cache.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_with_port_runtime_source<'program>(
        &'program self,
        sources: &mut PortRuntimePredictionSource<'program>,
        source: &SourceId,
        hidden: &[String],
        path: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String> {
        let result = (|| {
            if remaining.is_none() {
                return Err(
                    "Runtime prediction sources require the original finite work pool".into(),
                );
            }
            charge(remaining)?;
            if sources.source.failed {
                return Err("Runtime prediction source scope is poisoned".into());
            }
            charge(remaining)?;
            if !std::ptr::eq(self, sources.source.program) {
                return Err("Runtime prediction source belongs to another exact program".into());
            }
            charge(remaining)?;
            if source != &sources.source.source || hidden.len() != sources.source.hidden.len() {
                return Err("Runtime prediction source/hidden-policy context differs".into());
            }
            for (actual, expected) in hidden.iter().zip(&sources.source.hidden) {
                charge(remaining)?;
                if actual != expected {
                    return Err("Runtime prediction source/hidden-policy context differs".into());
                }
            }
            charge(remaining)?;
            let decision = self
                .decisions
                .get(path)
                .ok_or("Unknown parser prediction decision")?;
            let mut access = PortAccess { source: sources };
            let value = self.run_prediction_at_depth_with_local_decision_and_source(
                PredictionGoal::Decision {
                    decision,
                    recursive_first: self.recursive_decisions.contains(path),
                    boundary: None,
                },
                input,
                remaining,
                0,
                verification,
                None,
                &mut access,
            )?;
            if remaining.is_none() {
                return Err(
                    "Runtime prediction sources require the original finite work pool".into(),
                );
            }
            charge(remaining)?;
            access.source.logical_total()?;
            if access.source.source.failed {
                return Err("Runtime prediction source scope is poisoned".into());
            }
            match value {
                PredictionOutcome::Selected(value) => Ok(value),
                PredictionOutcome::Completed(_) | PredictionOutcome::InitialPrepared(_) => {
                    Err("Unexpected completion outcome for a decision".into())
                }
            }
        })();
        if result.is_err() {
            sources.invalidate();
        }
        result
    }
}

#[cfg(test)]
mod action_controls {
    use super::super::super::{Decision, Edge, State};
    use super::*;

    #[test]
    fn runtime_ports_action_transfer_retains_exact_alternative_guarded_and_entire_caller_tail() {
        let program = ParserPrediction {
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            allow_empty_choices: false,
            states: vec![
                State {
                    edges: vec![Edge::Action(1)],
                    ..State::default()
                },
                State {
                    edges: vec![Edge::Token(PredictionSymbol::End, 2)],
                    ..State::default()
                },
                State {
                    rule_stop: true,
                    ..State::default()
                },
            ],
            rules: BTreeMap::new(),
            decisions: BTreeMap::<String, Decision>::new(),
            first_guards: BTreeMap::new(),
            recursive_decisions: BTreeSet::new(),
            runtime_requirement: None,
        };
        let id = SourceId::for_text("memory:runtime-ports/action", "x");
        let mut work = Some(10_000);
        let mut scope = PortRuntimePredictionSource::new(&program, &id, &[], &mut work).unwrap();
        for visible in [false, true] {
            let row = Configuration {
                state: 0,
                returns: vec![17, 23, 29],
                alternative: 4,
                visible,
                guarded: true,
                full_guard: true,
            };
            let mut pending = VecDeque::new();
            assert!(
                PortAccess { source: &mut scope }
                    .expand_ports(&row, &mut pending, &mut work)
                    .unwrap()
            );
            assert_eq!(
                pending,
                VecDeque::from([Configuration {
                    state: 1,
                    visible: false,
                    ..row
                }])
            );
        }
        assert_eq!(scope.emitted_configurations(), 2);
        assert!(scope.hits() > 0 && !scope.is_poisoned());
    }
}
