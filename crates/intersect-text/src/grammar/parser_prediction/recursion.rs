//! A bounded recursion witness within a shared callee, never its caller suffix.
use super::{
    AdjacentCharge, AdjacentWorkObservation, Edge, ParserPrediction, RecursiveAnalysisCounters,
    Work, observed_recursive_charge,
};
use std::collections::{BTreeSet, VecDeque};
impl ParserPrediction {
    pub(super) fn recursion_witness(
        &self,
        entry: usize,
        depth_limit: usize,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        self.recursion_witness_measured(entry, depth_limit, remaining, None)
    }
    pub(super) fn recursion_witness_measured(
        &self,
        entry: usize,
        depth_limit: usize,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
    ) -> Result<bool, String> {
        self.recursion_witness_observed(entry, depth_limit, remaining, measured, None)
    }
    pub(super) fn recursion_witness_observed(
        &self,
        entry: usize,
        depth_limit: usize,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<bool, String> {
        // The root return is never followed. Its placeholder equals a real state
        // so no sentinel graph identity or synthetic token is introduced.
        let mut pending = VecDeque::from([(entry, vec![(entry, entry)])]);
        let mut visited = BTreeSet::new();
        let mut peak = 1usize;
        while let Some((state_id, calls)) = pending.pop_front() {
            observed_recursive_charge(
                remaining,
                Work::WitnessState,
                measured,
                exact,
                AdjacentCharge::WitnessState,
            )?;
            if !visited.insert((state_id, calls.clone())) {
                continue;
            }
            peak = peak.max(calls.len());
            let state = self
                .states
                .get(state_id)
                .ok_or("Dangling parser prediction state")?;
            if state.rule_stop {
                if calls.len() > 1 {
                    let (_, follow) = calls.last().ok_or("Missing recursion return")?;
                    pending.push_back((*follow, calls[..calls.len() - 1].to_vec()));
                }
                continue;
            }
            for edge in &state.edges {
                observed_recursive_charge(
                    remaining,
                    Work::WitnessEdge,
                    measured,
                    exact,
                    AdjacentCharge::WitnessEdge,
                )?;
                match edge {
                    Edge::Epsilon(target)
                    | Edge::Action(target)
                    | Edge::First { target, .. }
                    | Edge::Full { target, .. }
                    | Edge::Token(_, target) => {
                        pending.push_back((*target, calls.clone()));
                    }
                    Edge::Call { target, follow } => {
                        if let Some(index) = calls.iter().position(|(callee, _)| callee == target) {
                            // Replaying this cycle reaches an already active call
                            // site. Reserve its additional depth, including any
                            // completed subcalls traversed before the witness.
                            if peak.saturating_add(calls.len() - index) <= depth_limit {
                                return Ok(true);
                            }
                            continue;
                        }
                        if calls.len() >= depth_limit {
                            continue;
                        }
                        let mut next = calls.clone();
                        next.push((*target, *follow));
                        pending.push_back((*target, next));
                    }
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod source_prefix_fact_controls {
    use super::*;
    use crate::grammar;
    use crate::grammar::Profile;
    use crate::grammar::backend_test_support::inspect_ecore_schema;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    use std::rc::Rc;

    #[test]
    fn successful_empty_first_derivation_is_checked_against_same_graph_witness() {
        let schema=inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:empty"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
        let source = DefinitionSource::new(
            "memory://prefix/empty.xtext",
            SourceKind::Xtext,
            "grammar EmptyTest hidden() import \"urn:empty\" as t Root returns t::Root: 'r'; Empty returns t::Root: {t::Root}; Wrap returns t::Root: Empty; Productive returns t::Root: 'x' Productive?;",
        );
        let (mut grammar, _, spans, _) = grammar::parse_declarations(
            &source.text,
            &schema,
            Profile::NativeV80,
            false,
            None,
            128,
            Rc::default(),
        )
        .unwrap();
        grammar.declaration_sources = spans
            .into_iter()
            .map(|(name, range)| {
                (
                    name,
                    SourceSpan {
                        source: source.id.clone(),
                        start: range.start,
                        end: range.end,
                    },
                )
            })
            .collect();
        let mut remaining = 10_000;
        let empty = grammar
            .inspect_source_prefix_with_work("rule/Wrap", &mut remaining)
            .unwrap();
        assert!(empty.nullable());
        assert!(empty.symbols().is_empty());
        let facts: Vec<_> = empty.calls().collect();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].name(), "Empty");
        assert!(facts[0].complete_empty_derivation());
        let program = ParserPrediction::compile(&grammar).unwrap();
        assert!(
            !program
                .recursion_witness(program.rules["Empty"].0, 128, &mut remaining)
                .unwrap()
        );
        assert!(
            !program
                .recursion_witness(program.rules["Wrap"].0, 128, &mut remaining)
                .unwrap()
        );
        let productive = grammar
            .inspect_source_prefix_with_work("rule/Productive", &mut remaining)
            .unwrap();
        assert!(!productive.nullable());
        assert_eq!(productive.symbols(), &BTreeSet::from(["literal:x".into()]));
        assert_eq!(productive.calls().count(), 0); // Recursive call lies AFTER x.
        assert!(
            program
                .recursion_witness(program.rules["Productive"].0, 128, &mut remaining)
                .unwrap()
        );
        // FIRST success alone must never be promoted to a whole-rule no-witness
        // Boolean. All original calls still run; no pruning API is installed.
    }
}
