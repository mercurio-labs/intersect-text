// Copyright (c) 2026 Mercurio Labs
// SPDX-License-Identifier: MIT
//! Syntax-only token decisions. Model construction and conversions never run here.
//! The explicit call/return and caller-FOLLOW design follows Mercurio's pinned
//! xtext_nfa_experiment.rs; this compiles Intersect grammar IR natively instead
//! of loading an exported ANTLR graph. Unresolved ambiguity remains an error.

use super::admission_observation::pool_blocks::{
    Block as PoolBlock, CompilerPoolDiagnostic, Observation as PoolWork, scope as pool_scope,
};
mod adjacent;
#[cfg(test)]
mod adjacent_tests;
mod bounded_witness;
mod disjoint;
mod frontier;
mod partial_token_probe;
mod transparent_chain;
mod transparent_ports;
use disjoint::call_free::{CallFreeCandidates, CallFreeTemplates};
mod analysis_calls;
mod guard_facts;
mod live_callers;
mod nullable_inventory;
mod nullable_repeat;
mod runtime_guarded;
pub use runtime_guarded::{
    RuntimeAdmissionLoadMode, RuntimeGuardedAdmission, RuntimeGuardedPolicy, SavedRuntimeAdmission,
};
mod shared_returns;
use analysis_calls::Calls;
pub use live_callers::LiveCallerPrediction;
mod runtime_memo;
pub use runtime_memo::{
    RuntimePredictionMemo, RuntimePredictionMemoMode, RuntimePredictionMemoStats,
};
#[cfg(test)]
mod runtime_memo_tests;
mod runtime_source;
pub use runtime_source::{
    InitialPreparation, PortRuntimePredictionSource, PreparedLiveRuntimePredictionContext,
    PreparedRuntimePredictionContext, RepeatedRuntimePredictionSource, RuntimePredictionSource,
};
mod recursion;
mod repeat_certificates;
use super::follow::repeat_certificates::{GraphBinding, RepeatCertificates};
use nullable_inventory::GraphBuildContext;
mod required_call;
use super::admission_observation::{
    AdjacentCharge, AdjacentContext, AdjacentEvent, AdjacentWorkObservation, ObservedEdge,
    RecursiveAnalysisCounters, RecursiveWorkCategory as Work, ScheduleEvent, ScheduleSite,
};
use super::{Expr, Grammar, Kind};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};
type SequenceContinuation<'e, 'p> = Option<(&'e [Expr], usize, &'p str)>;

type AdjacentAnalysisContext<'measured, 'templates, 'graph, 'exact> = (
    bool,
    Option<&'measured RecursiveAnalysisCounters>,
    Option<&'templates mut CallFreeTemplates<'graph>>,
    (bool, bool),
    Option<&'exact AdjacentWorkObservation>,
);

type AnalysisTokens<C = analysis_calls::VecCalls> =
    BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration<C>>>;

struct FrontierAnalysis<'work, 'template, 'graph> {
    remaining: &'work mut usize,
    witnesses: &'work mut BTreeMap<(usize, usize), bool>,
    recursive: &'work mut BTreeSet<usize>,
    truncated: &'work mut bool,
    templates: &'work mut Option<&'template mut CallFreeTemplates<'graph>>,
    shared_recursion: bool,
    unique_closure: bool,
    measured: Option<&'work RecursiveAnalysisCounters>,
    exact: Option<&'work AdjacentWorkObservation>,
    frontier: u64,
    pending_frontiers: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PredictionSymbol {
    Keyword(String),
    Terminal(String),
    End,
}
/// Explicit compiler search scheduling. This never supplies a recursion result,
/// source certificate, new budget or runtime commitment policy. The selection is
/// transient and unserialized; a later compiler invocation must select it again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecursiveFrontierOrder {
    /// Keep every historical profile's existing traversal and work order.
    #[default]
    Historical,
    /// Visit completed token frontiers in deterministic FIFO order. This can
    /// change which real error/witness is reached first under a finite budget.
    BreadthFirst,
    /// While completing the FIRST epsilon closure, allow one positive-only
    /// continuation proof from actual same-token configurations under two
    /// alternatives. The same classifier runs on a prepaid full-key subset;
    /// false keeps the unfinished outer closure, and real errors propagate.
    /// No partial set becomes completed-token/adoption authority.
    BreadthFirstSharedTokenProbe,
    /// FIFO with lazy, source-checked epsilon/action chain summaries. The
    /// classifier still executes every Call/return/FOLLOW and guard boundary.
    /// Summary work and retained entries use the original analysis pool.
    /// Intermediate pure states can be omitted from the transient full-key
    /// queue; finite error/witness order can therefore differ. No partial-token
    /// probe is enabled, and later compilations must explicitly reselect this.
    BreadthFirstTransparentChains,
    /// Explicit eager pure epsilon/action DAG-port scheduling. The same
    /// classifier executes all effect/Call/return/FOLLOW and predicate ports.
    /// Only immutable topology is shared, with cold/lookup/copy/retention
    /// debits from the original pool. Full alternative and ordered return
    /// keys remain live. Finite error/witness and callback order may differ
    /// from expanded FIFO; successful semantic paths are not approximated.
    /// This transient policy must be reselected after serialization/reload.
    BreadthFirstTransparentPorts,
    /// Same eager ports and canonical effects, with paid reuse of an existing
    /// completed acyclic pure component reached inside another root. The4096
    /// expanded-node proof is conservative; finite denial order can differ.
    BreadthFirstComposedTransparentPorts,
    /// Explicit same-classifier physical sharing of complete ordered call pairs.
    /// Not serialized into the prediction program; recompilation must reselect.
    BreadthFirstSharedCallPorts,
}

/// Explicit capability for the sufficient static proof used by Full replay.
/// This never changes the graph, runtime guard execution or commitment order.
/// It is not serialized authority; checked hosts select it from their profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CompletionVerification {
    #[default]
    Legacy,
    /// Prove disjointness after enlarging branch languages by removing guard
    /// restrictions. Overlap or unproved guard visibility still rejects.
    GuardOverapproximation,
    /// Same enlarged guard language, with existing bounded discovery and
    /// filtered/callee-fence proofs before exact relative fallback. This is a
    /// transient strategy, not serialized admission or a new work allowance.
    BoundedGuardOverapproximation,
    /// Explicit per-input completion contract. Only a checked literal-prefixed
    /// loop may defer global disjointness to the SAME bounded Decision worker.
    /// This selector is not serialized compiler or runtime authority.
    InputCheckedCompletion,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Edge {
    Epsilon(usize),
    Action(usize),
    First { path: String, target: usize },
    Full { target: usize, stop: usize },
    Token(PredictionSymbol, usize),
    Call { target: usize, follow: usize },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    rule_stop: bool,
    accepting: bool,
    edges: Vec<Edge>,
    follow: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Decision {
    entries: Vec<usize>,
    exit: Option<usize>,
}
/// A deterministic, bounded syntax-only lookahead program.
///
/// NativeV6 binds admitted first-token guards into this shared program. Earlier
/// profiles retain their original FIRST/FOLLOW and predicate admission rules.
/// Decisions are named by exact effective rule and expression path. The caller
/// supplies already-lexed symbols and owns the execution-work ceiling.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParserPrediction {
    #[serde(skip)]
    allow_empty_choices: bool,
    /// Combined call depth (outer callers plus prediction returns) the
    /// prediction analysis and runtime accept. Serialized only when it differs
    /// from `DEFAULT_CALL_DEPTH`, so existing programs keep their wire shape.
    #[serde(
        default = "default_call_depth",
        skip_serializing_if = "is_default_call_depth"
    )]
    call_depth: u16,
    states: Vec<State>,
    rules: BTreeMap<String, (usize, usize)>,
    decisions: BTreeMap<String, Decision>,
    first_guards: BTreeMap<String, BTreeSet<PredictionSymbol>>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    recursive_decisions: BTreeSet<String>,
    /// Requirement only; serialized data never supplies the live capability,
    /// except the single host-cache restore through
    /// `CompiledRuntimeGrammar::load_host_cache` under the
    /// `VerifiedExecutable::load_trusted` contract (see `runtime_guarded`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_requirement: Option<Box<runtime_guarded::Requirement>>,
}
/// Combined call depth a prediction program accepts unless its language asks
/// for more (`RuntimeGuardedAdmission::with_call_depth`).
pub const DEFAULT_CALL_DEPTH: usize = 128;
/// Structural bound on return stacks in prediction storage. A configured call
/// depth is at most this; the program's own checks apply the configured one.
pub const MAX_CALL_DEPTH: usize = 1024;
fn default_call_depth() -> u16 {
    DEFAULT_CALL_DEPTH as u16
}
fn is_default_call_depth(depth: &u16) -> bool {
    usize::from(*depth) == DEFAULT_CALL_DEPTH
}
const _: () = assert!(MAX_CALL_DEPTH <= u16::MAX as usize);
impl ParserPrediction {
    /// The combined call depth this program accepts.
    pub fn call_depth(&self) -> usize {
        usize::from(self.call_depth)
    }
}
impl ParserPrediction {
    pub fn compile(grammar: &Grammar) -> Result<Self, String> {
        Self::compile_with_first_predicates(grammar, &BTreeSet::new())
    }
    /// Bind first-token predicates to exact effective expression paths.
    /// Source admission belongs to the grammar compiler; model execution uses
    /// this same syntax-only program without running semantic actions in probes.
    pub fn compile_with_first_predicates(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
    ) -> Result<Self, String> {
        Self::compile_predicates(grammar, predicates, false, None)
    }
    /// Analyze recursive alternatives before admitting first-token commitment.
    /// Existing profiles use `compile_with_first_predicates` and retain rejection.
    pub fn compile_with_recursive_first_predicates(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
    ) -> Result<Self, String> {
        Self::compile_predicates(grammar, predicates, true, None)
    }
    pub(super) fn compile_observed(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: super::AnalysisObservation<'_>,
    ) -> Result<Self, String> {
        Self::compile_predicates(grammar, predicates, recursive, Some(observation))
    }
    fn compile_predicates(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: Option<super::AnalysisObservation<'_>>,
    ) -> Result<Self, String> {
        Self::compile_predicates_impl(grammar, predicates, recursive, observation, None)
    }
    pub(super) fn compile_certified_repeats(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: Option<super::AnalysisObservation<'_>>,
        certificates: &mut RepeatCertificates,
    ) -> Result<Self, String> {
        if !matches!(
            grammar.profile,
            super::Profile::NativeV74
                | super::Profile::NativeV75
                | super::Profile::NativeV76
                | super::Profile::NativeV77
                | super::Profile::NativeV78
                | super::Profile::NativeV79
                | super::Profile::NativeV80
                | super::Profile::NativeV81
                | super::Profile::NativeV82
        ) {
            return Err("Source repeat certificates require their explicit profile".into());
        }
        Self::compile_predicates_impl(
            grammar,
            predicates,
            recursive,
            observation,
            Some(certificates),
        )
    }
    #[allow(clippy::too_many_arguments)] // Same canonical worker; transient diagnostic only.
    pub(super) fn compile_with_recursive_frontier_order_pool(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: Option<super::AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
        order: RecursiveFrontierOrder,
        pool: Option<&CompilerPoolDiagnostic>,
    ) -> Result<Self, String> {
        if order != RecursiveFrontierOrder::Historical
            && grammar.profile != super::Profile::NativeV80
        {
            return Err("Recursive frontier selection requires NativeV80".into());
        }
        Self::compile_predicates_impl_frontier_selected_pool(
            grammar,
            predicates,
            recursive,
            observation,
            certificates,
            order,
            pool,
        )
    }
    fn compile_predicates_impl(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: Option<super::AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
    ) -> Result<Self, String> {
        Self::compile_predicates_impl_frontier_selected(
            grammar,
            predicates,
            recursive,
            observation,
            certificates,
            RecursiveFrontierOrder::Historical,
        )
    }
    #[allow(clippy::too_many_arguments)] // Same worker plus explicit transient search order.
    fn compile_predicates_impl_frontier_selected(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: Option<super::AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
        order: RecursiveFrontierOrder,
    ) -> Result<Self, String> {
        Self::compile_predicates_impl_frontier_selected_pool(
            grammar,
            predicates,
            recursive,
            observation,
            certificates,
            order,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)] // Same canonical worker; transient diagnostic only.
    pub(super) fn compile_runtime_guarded_linear_repeats(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        observation: Option<super::AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
        admission: &mut RuntimeGuardedAdmission,
    ) -> Result<Self, String> {
        if grammar.profile != super::Profile::NativeV80 {
            return Err("Runtime guarded admission requires NativeV80".into());
        }
        Self::compile_predicates_impl_prediction_policy::<true>(
            grammar,
            predicates,
            true,
            observation,
            certificates,
            RecursiveFrontierOrder::Historical,
            None,
            Some(admission),
        )
    }
    fn compile_predicates_impl_frontier_selected_pool(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: Option<super::AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
        order: RecursiveFrontierOrder,
        pool: Option<&CompilerPoolDiagnostic>,
    ) -> Result<Self, String> {
        Self::compile_predicates_impl_prediction_policy::<true>(
            grammar,
            predicates,
            recursive,
            observation,
            certificates,
            order,
            pool,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn compile_predicates_impl_prediction_policy<const REUSE_GUARD_FACTS: bool>(
        grammar: &Grammar,
        predicates: &BTreeSet<String>,
        recursive: bool,
        observation: Option<super::AnalysisObservation<'_>>,
        mut certificates: Option<&mut RepeatCertificates>,
        order: RecursiveFrontierOrder,
        pool: Option<&CompilerPoolDiagnostic>,
        mut runtime: Option<&mut RuntimeGuardedAdmission>,
    ) -> Result<Self, String> {
        if matches!(
            grammar.profile,
            super::Profile::NativeV74
                | super::Profile::NativeV75
                | super::Profile::NativeV76
                | super::Profile::NativeV77
                | super::Profile::NativeV78
                | super::Profile::NativeV79
                | super::Profile::NativeV80
                | super::Profile::NativeV81
                | super::Profile::NativeV82
        ) && let Some(work) = observation.and_then(|context| context.recursive_work)
        {
            work.coverage(
                super::admission_observation::Coverage::RepeatCertificateAnalysisUninstrumented,
            );
        }
        if matches!(
            grammar.profile,
            super::Profile::NativeV75
                | super::Profile::NativeV76
                | super::Profile::NativeV77
                | super::Profile::NativeV78
                | super::Profile::NativeV79
                | super::Profile::NativeV80
                | super::Profile::NativeV81
                | super::Profile::NativeV82
        ) && let Some(work) = observation.and_then(|context| context.recursive_work)
        {
            work.coverage(
                super::admission_observation::Coverage::CallFreeRelativeClosureUninstrumented,
            );
        }
        if matches!(
            grammar.profile,
            super::Profile::NativeV76
                | super::Profile::NativeV77
                | super::Profile::NativeV78
                | super::Profile::NativeV79
                | super::Profile::NativeV80
                | super::Profile::NativeV81
                | super::Profile::NativeV82
        ) && let Some(work) = observation.and_then(|context| context.recursive_work)
        {
            work.coverage(
                super::admission_observation::Coverage::AdjacentClosureSourceUninstrumented,
            );
        }
        let work_budget = runtime
            .as_ref()
            .map_or(grammar.analysis_work_budget, |a| a.work_budget());
        let work_limit = work_budget.units();
        let mut prepared_guards = runtime
            .as_ref()
            .filter(|admission| {
                REUSE_GUARD_FACTS
                    && matches!(
                        admission.policy(),
                        RuntimeGuardedPolicy::StrictCompletionScopes
                            | RuntimeGuardedPolicy::InputCheckedCompletionScopes
                    )
            })
            .map(|_| guard_facts::GuardFacts::new(grammar));
        let mut first_guards = BTreeMap::new();
        let mut unresolved = predicates.clone();
        let mut budget = work_limit;
        if !predicates.is_empty() {
            for (name, rule) in &grammar.rules {
                let prepared = prepared_guards.as_mut().zip(runtime.as_deref_mut());
                let collected = match prepared {
                    Some(prepared) => collect_guards_with_facts_and_budget(
                        &rule.body,
                        grammar,
                        &format!("rule/{name}"),
                        &mut unresolved,
                        &mut first_guards,
                        &mut budget,
                        (0, recursive, work_budget),
                        Some(prepared),
                    ),
                    None => collect_guards(
                        &rule.body,
                        grammar,
                        &format!("rule/{name}"),
                        &mut unresolved,
                        &mut first_guards,
                        &mut budget,
                        (0, recursive, work_budget),
                    ),
                };
                observe_parser_phase(
                    collected,
                    grammar,
                    observation,
                    super::admission_observation::Phase::ParserGuardCollection,
                    (Some(name), None),
                    (Some(work_limit - budget), None),
                )?;
            }
        }
        if !unresolved.is_empty() {
            return Err("Unknown first-token predicate expression path".into());
        }
        let mut result = Self {
            allow_empty_choices: grammar.profile.has_constant_guards(),
            call_depth: u16::try_from(grammar.prediction_call_depth())
                .map_err(|_| "Prediction call depth exceeds its storage")?,
            states: Vec::new(),
            rules: BTreeMap::new(),
            decisions: BTreeMap::new(),
            first_guards,
            recursive_decisions: BTreeSet::new(),
            runtime_requirement: None,
        };
        // Reuse the existing source-checked decision map: older profiles add no
        // decision records and retain their exact serialized program. The typed
        // None requirement holder growth is checked separately from wire shape.
        if grammar.profile.has_durable_documents() {
            result.decisions.insert(
                "@document-recovery".into(),
                Decision {
                    entries: Vec::new(),
                    exit: None,
                },
            );
        }
        for name in grammar.rules.keys() {
            let start = observe_parser_phase(
                result.state(),
                grammar,
                observation,
                super::admission_observation::Phase::ParserRuleStates,
                (Some(name), None),
                (None, Some(result.states.len())),
            )?;
            let end = observe_parser_phase(
                result.state(),
                grammar,
                observation,
                super::admission_observation::Phase::ParserRuleStates,
                (Some(name), None),
                (None, Some(result.states.len())),
            )?;
            result.states[end].rule_stop = true;
            result.rules.insert(name.clone(), (start, end));
        }
        if !(matches!(
            grammar.profile,
            super::Profile::NativeV51
                | super::Profile::NativeV52
                | super::Profile::NativeV53
                | super::Profile::NativeV54
                | super::Profile::NativeV55
                | super::Profile::NativeV56
                | super::Profile::NativeV57
                | super::Profile::NativeV58
                | super::Profile::NativeV59
                | super::Profile::NativeV60
                | super::Profile::NativeV61
                | super::Profile::NativeV62
                | super::Profile::NativeV63
                | super::Profile::NativeV64
                | super::Profile::NativeV65
                | super::Profile::NativeV66
                | super::Profile::NativeV67
                | super::Profile::NativeV68
                | super::Profile::NativeV69
                | super::Profile::NativeV70
                | super::Profile::NativeV71
                | super::Profile::NativeV72
                | super::Profile::NativeV73
                | super::Profile::NativeV74
                | super::Profile::NativeV75
                | super::Profile::NativeV76
                | super::Profile::NativeV77
                | super::Profile::NativeV78
                | super::Profile::NativeV79
                | super::Profile::NativeV80
                | super::Profile::NativeV81
                | super::Profile::NativeV82
        ) && grammar.entry.is_empty())
        {
            let entry = result
                .rules
                .get(&grammar.entry)
                .ok_or("Missing parser entry")?
                .1;
            let eof = observe_parser_phase(
                result.state(),
                grammar,
                observation,
                super::admission_observation::Phase::ParserEntryStates,
                (Some(&grammar.entry), None),
                (None, Some(result.states.len())),
            )?;
            let accepted = observe_parser_phase(
                result.state(),
                grammar,
                observation,
                super::admission_observation::Phase::ParserEntryStates,
                (Some(&grammar.entry), None),
                (None, Some(result.states.len())),
            )?;
            result.states[accepted].accepting = true;
            result.states[entry].follow.push(eof);
            result.states[eof]
                .edges
                .push(Edge::Token(PredictionSymbol::End, accepted));
        }
        // Source-bound wildcard composition retains unused inherited rules in
        // the definition graph. Their callers must not contribute FOLLOW paths
        // to decisions in fragments reachable from the actual source entry.
        // Keep all rule/decision metadata and ordinary-profile behavior intact.
        let mut reachable = BTreeSet::new();
        if grammar.rules.values().any(|rule| rule.wildcard) {
            let mut pending = BTreeSet::from([grammar.entry.clone()]);
            while let Some(name) = pending.pop_first() {
                if !reachable.insert(name.clone()) {
                    continue;
                }
                if let Some(rule) = grammar.rules.get(&name) {
                    let calls = super::follow::calls(&rule.body, &mut pending, &mut budget);
                    observe_parser_phase(
                        calls,
                        grammar,
                        observation,
                        super::admission_observation::Phase::ParserReachability,
                        (Some(&name), None),
                        (Some(work_limit - budget), Some(result.states.len())),
                    )?;
                }
            }
        }
        // This is the SAME pool later used by recursive classification. New
        // source binding fees are consumed now and never reset at that phase.
        let mut analysis_remaining = work_limit;
        if let Some(admission) = runtime.as_deref_mut() {
            admission.begin_analysis_work(analysis_remaining);
        }
        let pool_work =
            pool.map(|capture| PoolWork::new_with_topology(analysis_remaining, capture));
        let pool_work = pool_work.as_ref();
        let pool_ordinal = match (pool, observation) {
            (Some(capture), Some(context)) => capture.begin(context.stage, context.entry, order),
            (Some(capture), None) => {
                capture.unavailable();
                None
            }
            _ => None,
        };
        let pool_root =
            pool_work.and_then(|work| work.enter(PoolBlock::PoolRemainder, analysis_remaining));
        let exact = observation
            .and_then(|context| context.adjacent_work)
            .filter(|work| {
                if grammar.profile == super::Profile::NativeV76 {
                    true
                } else {
                    work.unsupported_profile();
                    false
                }
            });
        if let (Some(work), Some(context)) = (exact, observation) {
            work.begin(context.stage, context.entry, analysis_remaining);
        }
        // Capture only the Result exit; the original pool, body and error order
        // stay shared. Earlier guard/FOLLOW pools are intentionally not counted.
        let pool_result = (|| {
            if let Some(work) = observation.and_then(|context| context.recursive_work) {
                work.selected_static_prepare(
                    grammar.profile == super::Profile::NativeV80
                        && order == RecursiveFrontierOrder::Historical,
                );
            }
            let mut call_free = (recursive
                && matches!(
                    grammar.profile,
                    super::Profile::NativeV75
                        | super::Profile::NativeV76
                        | super::Profile::NativeV77
                        | super::Profile::NativeV78
                        | super::Profile::NativeV79
                        | super::Profile::NativeV80
                        | super::Profile::NativeV81
                        | super::Profile::NativeV82
                ))
            .then(CallFreeCandidates::default);
            {
                // This private collection retains the SAME admission borrow through
                // graph construction and canonical nullable qualification.
                let mut inventory = match runtime.as_deref_mut() {
                    Some(admission) => Some(nullable_inventory::Collection::new(
                        grammar,
                        admission,
                        &mut analysis_remaining,
                    )?),
                    None => None,
                };
                let mut disconnected = BTreeSet::new();
                for (name, rule) in &grammar.rules {
                    let (start, end) =
                        *result.rules.get(name).ok_or("Missing parser rule states")?;
                    let first = result.states.len();
                    let graph_scope = pool_work.and_then(|work| {
                        work.enter(PoolBlock::GraphSourceBinding, analysis_remaining)
                    });
                    let certificates = certificates.as_deref_mut();
                    let inventory = inventory
                        .as_mut()
                        .and_then(nullable_inventory::Collection::builder);
                    let mut repeat_context = if certificates.is_some() || inventory.is_some() {
                        Some(GraphBuildContext {
                            grammar,
                            owner: name,
                            rule,
                            certificates,
                            inventory,
                            remaining: &mut analysis_remaining,
                            exact,
                        })
                    } else {
                        None
                    };
                    let saw_call = std::cell::Cell::new(false);
                    let graph = result
                        .expression(
                            &rule.body,
                            start,
                            end,
                            &format!("rule/{name}"),
                            (0, repeat_context.as_mut(), None),
                            call_free.as_ref().map(|_| &saw_call),
                        )
                        .and_then(|()| match call_free.as_mut() {
                            Some(candidates) => {
                                if exact.is_some() {
                                    candidates.record_rule_observed(
                                        start,
                                        end,
                                        saw_call.get(),
                                        &mut analysis_remaining,
                                        exact,
                                    )
                                } else {
                                    candidates.record_rule(
                                        start,
                                        end,
                                        saw_call.get(),
                                        &mut analysis_remaining,
                                    )
                                }
                            }
                            None => Ok(()),
                        });
                    if let Some(work) = pool_work {
                        work.leave(graph_scope, analysis_remaining, graph.is_ok());
                    }
                    observe_parser_phase(
                        graph,
                        grammar,
                        observation,
                        super::admission_observation::Phase::ParserRuleGraph,
                        (Some(name), None),
                        (
                            (matches!(
                                grammar.profile,
                                super::Profile::NativeV74
                                    | super::Profile::NativeV75
                                    | super::Profile::NativeV76
                                    | super::Profile::NativeV77
                                    | super::Profile::NativeV78
                                    | super::Profile::NativeV79
                                    | super::Profile::NativeV80
                                    | super::Profile::NativeV81
                                    | super::Profile::NativeV82
                            ))
                            .then_some(work_limit - analysis_remaining),
                            Some(result.states.len()),
                        ),
                    )?;
                    if let Some(work) = observation.and_then(|context| context.recursive_work) {
                        // Existing builder IDs only; no new graph/source lookup.
                        work.selected_static_rule(name, [start, end, first, result.states.len()]);
                    }
                    if !reachable.is_empty() && !reachable.contains(name) {
                        disconnected.extend([start, end]);
                        disconnected.extend(first..result.states.len());
                    }
                }
                if !disconnected.is_empty() {
                    for state in &mut result.states {
                        state.follow.retain(|target| !disconnected.contains(target));
                    }
                }
                if let Some(inventory) = inventory {
                    inventory
                        .collect_with_guard_facts(
                            grammar,
                            &result,
                            &mut analysis_remaining,
                            observation,
                            certificates.as_deref_mut(),
                            prepared_guards.as_ref(),
                        )
                        .map_err(|cause| {
                            format!("Runtime guarded candidate collection failed: {cause}")
                        })?;
                }
            }
            if let Some(admission) = runtime.as_deref_mut() {
                admission.prebind_multiway_choices(grammar, &result, &mut analysis_remaining)?;
            }
            let mut recursive_decisions = BTreeSet::new();
            if recursive {
                let mut templates = call_free.map(|candidates| candidates.freeze(&result.states));
                let mut witnesses = BTreeMap::new();
                // Native80 exposes only the seven existing canonical worker
                // categories on explicit request. Source, template, geometric,
                // frontier-adoption and range-lookup fees remain outside them.
                // Coverage stays AdjacentClosureSourceUninstrumented; neither
                // the old V76 exact sidecar nor whole-pool conservation is granted.
                let measured = observation
                    .and_then(|context| context.recursive_work)
                    .filter(|_| {
                        !matches!(
                            grammar.profile,
                            super::Profile::NativeV74
                                | super::Profile::NativeV75
                                | super::Profile::NativeV76
                                | super::Profile::NativeV77
                                | super::Profile::NativeV78
                                | super::Profile::NativeV79
                        )
                    });
                if let (Some(work), Some(context)) = (measured, observation) {
                    work.begin_program(context.stage, analysis_remaining);
                }
                let analysis_result = (|| {
                    for (path, decision) in &result.decisions {
                        if let Some(capture) = pool {
                            capture.decision(path);
                        }
                        // Recovery records retain expression continuations for the
                        // separate recovery API. They do not select parser branches.
                        // Old profiles keep their exact analysis/work/serialized path.
                        if grammar.profile.has_executable_recursive_analysis()
                            && !path.ends_with("/choice")
                            && !path.ends_with("/repeat")
                        {
                            if let Some(work) = measured {
                                work.skip(true);
                            }
                            continue;
                        }
                        if matches!(
                            grammar.profile,
                            super::Profile::NativeV79
                                | super::Profile::NativeV80
                                | super::Profile::NativeV81
                                | super::Profile::NativeV82
                        ) {
                            match pool_scope(
                                &mut analysis_remaining,
                                pool_work,
                                PoolBlock::NullableSourceQualification,
                                |remaining| {
                                    grammar.qualified_nullable_decision_with_finite_work(
                                        path, remaining,
                                    )
                                },
                            )? {
                                Some(super::QualifiedNullableDecision::FullRepeat(proof)) => {
                                    pool_scope(
                                        &mut analysis_remaining,
                                        pool_work,
                                        PoolBlock::NullableGraphProof,
                                        |remaining| {
                                            result.verify_productive_nullable_full_repeat(
                                                path,
                                                proof.callee(),
                                                proof.keyword(),
                                                remaining,
                                            )
                                        },
                                    )?;
                                }
                                Some(super::QualifiedNullableDecision::KeywordOptional(proof)) => {
                                    pool_scope(
                                        &mut analysis_remaining,
                                        pool_work,
                                        PoolBlock::NullableGraphProof,
                                        |remaining| {
                                            result.verify_nullable_keyword_optional_binding(
                                                path,
                                                proof.owner(),
                                                proof.keyword(),
                                                remaining,
                                            )
                                        },
                                    )?;
                                }
                                None => {}
                            }
                        } else {
                            if matches!(
                                grammar.profile,
                                super::Profile::NativeV77
                                    | super::Profile::NativeV78
                                    | super::Profile::NativeV79
                                    | super::Profile::NativeV80
                                    | super::Profile::NativeV81
                                    | super::Profile::NativeV82
                            ) && let Some(proof) = grammar
                                .qualified_nullable_full_repeat_with_work(
                                    path,
                                    &mut analysis_remaining,
                                )?
                            {
                                result.verify_productive_nullable_full_repeat(
                                    path,
                                    proof.callee(),
                                    proof.keyword(),
                                    &mut analysis_remaining,
                                )?;
                            }
                            if matches!(
                                grammar.profile,
                                super::Profile::NativeV77
                                    | super::Profile::NativeV78
                                    | super::Profile::NativeV79
                                    | super::Profile::NativeV80
                                    | super::Profile::NativeV81
                                    | super::Profile::NativeV82
                            ) && let Some(proof) = grammar
                                .qualified_nullable_keyword_optional_with_work(
                                    path,
                                    &mut analysis_remaining,
                                )?
                            {
                                result.verify_nullable_keyword_optional_binding(
                                    path,
                                    proof.owner(),
                                    proof.keyword(),
                                    &mut analysis_remaining,
                                )?;
                            }
                        }
                        if let Some(admission) = runtime.as_deref() {
                            analysis_charge(&mut analysis_remaining)?; // new typed-obligation map lookup
                            if admission.contains(path)
                                || admission.conditional_choice_with_remaining(
                                    path,
                                    &mut analysis_remaining,
                                )?
                            {
                                // Compiler-owned exact source/graph candidate;
                                // its required proof must seal before publication.
                                // No recursion/priority classification is manufactured.
                                continue;
                            }
                        }
                        let guarded = decision.entries.iter().any(|entry| {
                            result.states[*entry]
                                .edges
                                .iter()
                                .any(|edge| matches!(edge, Edge::First { .. } | Edge::Full { .. }))
                        });
                        if !guarded {
                            if let Some(work) = measured {
                                work.skip(false);
                            }
                            continue;
                        }
                        if let (Some(work), Some(context)) = (measured, observation) {
                            work.begin_decision(grammar, context.entry, path);
                            work.selected_static_begin(path, analysis_remaining);
                        }
                        if let Some(work) = exact {
                            work.decision(path);
                        }
                        let certificate_scope = pool_work.and_then(|work| {
                            work.enter(PoolBlock::RepeatCertificate, analysis_remaining)
                        });
                        let certified = match certificates.as_deref_mut() {
                            Some(certificates) => {
                                if exact.is_some() {
                                    result.certified_repeat_is_nonconflicting_observed(
                                        path,
                                        decision,
                                        grammar,
                                        certificates,
                                        &mut analysis_remaining,
                                        exact,
                                    )
                                } else {
                                    result.certified_repeat_is_nonconflicting(
                                        path,
                                        decision,
                                        grammar,
                                        certificates,
                                        &mut analysis_remaining,
                                    )
                                }
                            }
                            None => Ok(false),
                        };
                        if let Some(work) = pool_work {
                            work.leave(certificate_scope, analysis_remaining, certified.is_ok());
                        }
                        let classifier_scope = pool_work.and_then(|work| {
                            work.enter(PoolBlock::ClassifierRemainder, analysis_remaining)
                        });
                        let decision_result = match certified {
                            Ok(true) => Ok(false),
                            Ok(false)
                                if grammar.profile == super::Profile::NativeV80
                                    && order == RecursiveFrontierOrder::BreadthFirstSharedTokenProbe =>
                            {
                                result.multiple_recursive_alternatives_with_probe::<
                                    adjacent::GeometricAdjacentQueue<'_>,
                                    frontier::AdoptFrontiers,
                                    true,
                                    partial_token_probe::FirstSharedToken,
                                >(
                                    decision,
                                    &mut analysis_remaining,
                                    true,
                                    &mut witnesses,
                                    false,
                                    (true, measured, templates.as_mut(), exact),
                                    &mut partial_token_probe::FirstSharedToken::new(false),
                                )
                            }
                            Ok(false)
                                if grammar.profile == super::Profile::NativeV80
                                    && order == RecursiveFrontierOrder::BreadthFirstTransparentChains =>
                            {
                                result.multiple_recursive_alternatives_with_frontiers::<
                                    transparent_chain::ChainQueue<'_>,
                                    frontier::AdoptFrontiers,
                                    true,
                                >(
                                    decision,
                                    &mut analysis_remaining,
                                    true,
                                    &mut witnesses,
                                    false,
                                    (true, measured, templates.as_mut(), exact),
                                )
                            }
                            Ok(false)
                                if grammar.profile == super::Profile::NativeV80
                                    && order == RecursiveFrontierOrder::BreadthFirstSharedCallPorts =>
                            {
                                result.multiple_recursive_alternatives_with_shared_call_ports(
                                    decision,&mut analysis_remaining,true,&mut witnesses,false,
                                    (true,measured,templates.as_mut(),exact),pool_work,
                                )
                            }
                            Ok(false)
                                if grammar.profile == super::Profile::NativeV80
                                    && order == RecursiveFrontierOrder::BreadthFirstComposedTransparentPorts =>
                            {
                                result.multiple_recursive_alternatives_with_frontiers_pool::<
                                    transparent_ports::BorrowedComponentPortQueue<'_>,
                                    frontier::AdoptFrontiers,
                                    true,
                                >(
                                    decision,
                                    &mut analysis_remaining,
                                    true,
                                    &mut witnesses,
                                    false,
                                    (true, measured, templates.as_mut(), exact),
                                pool_work,
)
                            }
                            Ok(false)
                                if grammar.profile == super::Profile::NativeV80
                                    && order == RecursiveFrontierOrder::BreadthFirstTransparentPorts =>
                            {
                                result.multiple_recursive_alternatives_with_frontiers_pool::<
                                    transparent_ports::PortQueue<'_>,
                                    frontier::AdoptFrontiers,
                                    true,
                                >(
                                    decision,
                                    &mut analysis_remaining,
                                    true,
                                    &mut witnesses,
                                    false,
                                    (true, measured, templates.as_mut(), exact),
                                pool_work,
)
                            }
                            Ok(false) if grammar.profile == super::Profile::NativeV80 => result
                                .multiple_recursive_alternatives_with_frontiers_pool::<
                                    adjacent::GeometricAdjacentQueue<'_>,
                                    frontier::AdoptFrontiers,
                                    true,
                                >(
                                    decision,
                                    &mut analysis_remaining,
                                    true,
                                    &mut witnesses,
                                    order == RecursiveFrontierOrder::Historical,
                                    (true, measured, templates.as_mut(), exact),
                                    pool_work,
                                ),
                            Ok(false) => result
                                .multiple_recursive_alternatives_with_queue_mode_observed(
                                    decision,
                                    &mut analysis_remaining,
                                    matches!(
                                        grammar.profile,
                                        super::Profile::NativeV38
                                            | super::Profile::NativeV39
                                            | super::Profile::NativeV40
                                            | super::Profile::NativeV41
                                            | super::Profile::NativeV42
                                            | super::Profile::NativeV43
                                            | super::Profile::NativeV44
                                            | super::Profile::NativeV45
                                            | super::Profile::NativeV46
                                            | super::Profile::NativeV47
                                            | super::Profile::NativeV48
                                            | super::Profile::NativeV49
                                            | super::Profile::NativeV50
                                            | super::Profile::NativeV51
                                            | super::Profile::NativeV52
                                            | super::Profile::NativeV53
                                            | super::Profile::NativeV54
                                            | super::Profile::NativeV55
                                            | super::Profile::NativeV56
                                            | super::Profile::NativeV57
                                            | super::Profile::NativeV58
                                            | super::Profile::NativeV59
                                            | super::Profile::NativeV60
                                            | super::Profile::NativeV61
                                            | super::Profile::NativeV62
                                            | super::Profile::NativeV63
                                            | super::Profile::NativeV64
                                            | super::Profile::NativeV65
                                            | super::Profile::NativeV66
                                            | super::Profile::NativeV67
                                            | super::Profile::NativeV68
                                            | super::Profile::NativeV69
                                            | super::Profile::NativeV70
                                            | super::Profile::NativeV71
                                            | super::Profile::NativeV72
                                            | super::Profile::NativeV73
                                            | super::Profile::NativeV74
                                            | super::Profile::NativeV75
                                            | super::Profile::NativeV76
                                            | super::Profile::NativeV77
                                            | super::Profile::NativeV78
                                            | super::Profile::NativeV79 | super::Profile::NativeV80 | super::Profile::NativeV81 | super::Profile::NativeV82
                                    ),
                                    &mut witnesses,
                                    matches!(
                                        grammar.profile,
                                        super::Profile::NativeV39
                                            | super::Profile::NativeV40
                                            | super::Profile::NativeV41
                                            | super::Profile::NativeV42
                                            | super::Profile::NativeV43
                                            | super::Profile::NativeV44
                                            | super::Profile::NativeV45
                                            | super::Profile::NativeV46
                                            | super::Profile::NativeV47
                                            | super::Profile::NativeV48
                                            | super::Profile::NativeV49
                                            | super::Profile::NativeV50
                                            | super::Profile::NativeV51
                                            | super::Profile::NativeV52
                                            | super::Profile::NativeV53
                                            | super::Profile::NativeV54
                                            | super::Profile::NativeV55
                                            | super::Profile::NativeV56
                                            | super::Profile::NativeV57
                                            | super::Profile::NativeV58
                                            | super::Profile::NativeV59
                                            | super::Profile::NativeV60
                                            | super::Profile::NativeV61
                                            | super::Profile::NativeV62
                                            | super::Profile::NativeV63
                                            | super::Profile::NativeV64
                                            | super::Profile::NativeV65
                                            | super::Profile::NativeV66
                                            | super::Profile::NativeV67
                                            | super::Profile::NativeV68
                                            | super::Profile::NativeV69
                                            | super::Profile::NativeV70
                                            | super::Profile::NativeV71
                                            | super::Profile::NativeV72
                                            | super::Profile::NativeV73
                                            | super::Profile::NativeV74
                                            | super::Profile::NativeV75
                                            | super::Profile::NativeV76
                                            | super::Profile::NativeV77
                                            | super::Profile::NativeV78
                                            | super::Profile::NativeV79 | super::Profile::NativeV80 | super::Profile::NativeV81 | super::Profile::NativeV82
                                    ),
                                    (
                                        matches!(
                                            grammar.profile,
                                            super::Profile::NativeV73
                                                | super::Profile::NativeV74
                                                | super::Profile::NativeV75
                                                | super::Profile::NativeV76
                                                | super::Profile::NativeV77
                                                | super::Profile::NativeV78
                                                | super::Profile::NativeV79 | super::Profile::NativeV80 | super::Profile::NativeV81 | super::Profile::NativeV82
                                        ),
                                        measured,
                                        templates.as_mut(),
                                        (
                                            matches!(
                                                grammar.profile,
                                                super::Profile::NativeV76
                                                    | super::Profile::NativeV77
                                                    | super::Profile::NativeV78
                                                    | super::Profile::NativeV79 | super::Profile::NativeV80 | super::Profile::NativeV81 | super::Profile::NativeV82
                                            ),
                                            matches!(
                                                grammar.profile,
                                                super::Profile::NativeV78
                                                    | super::Profile::NativeV79 | super::Profile::NativeV80 | super::Profile::NativeV81 | super::Profile::NativeV82
                                            ),
                                        ),
                                        exact,
                                    ),
                                ),
                            Err(error) => Err(error),
                        };
                        if let Some(work) = pool_work {
                            work.leave(
                                classifier_scope,
                                analysis_remaining,
                                decision_result.is_ok(),
                            );
                        }
                        if let Some(work) = measured {
                            work.selected_static_end(analysis_remaining, decision_result.is_err());
                            work.end_decision(&decision_result);
                        }
                        if observe_parser_phase(
                            decision_result,
                            grammar,
                            observation,
                            super::admission_observation::Phase::ParserRecursiveAnalysis,
                            (path.split('/').nth(1), Some(path)),
                            (
                                Some(work_limit - analysis_remaining),
                                Some(result.states.len()),
                            ),
                        )? {
                            recursive_decisions.insert(path.clone());
                        }
                    }
                    Ok::<(), String>(())
                })();
                if let Some(work) = measured {
                    work.end_program(analysis_remaining, analysis_result.is_ok());
                }
                analysis_result.map_err(|cause| {
                    if runtime.is_some() {
                        format!("Runtime guarded static remainder failed: {cause}")
                    } else {
                        cause
                    }
                })?;
            }
            result.recursive_decisions = recursive_decisions;
            if let Some(admission) = runtime {
                admission
                    .seal_native_generated(grammar, &mut result, &mut analysis_remaining)
                    .map_err(|cause| format!("Runtime guarded sealing failed: {cause}"))?;
            }
            Ok(result)
        })();
        if let Some(work) = exact {
            work.end(analysis_remaining, pool_result.is_ok());
        }
        if let Some(work) = pool_work {
            work.leave(pool_root, analysis_remaining, pool_result.is_ok());
            work.end(analysis_remaining, pool_result.is_ok());
            if let (Some(capture), Some(ordinal)) = (pool, pool_ordinal) {
                capture.finish(ordinal, work.snapshot());
            }
        }
        pool_result
    }
    // Discover recursion only along still-conflicting token prefixes. Merely
    // reaching a recursive rule in a uniquely selected alternative is harmless.
    fn multiple_recursive_alternatives(
        &self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured): (bool, Option<&RecursiveAnalysisCounters>),
    ) -> Result<bool, String> {
        self.multiple_recursive_alternatives_impl(
            decision,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, None),
        )
    }
    fn multiple_recursive_alternatives_with_templates(
        &self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
        ),
    ) -> Result<bool, String> {
        if templates.is_none() {
            return self.multiple_recursive_alternatives(
                decision,
                remaining,
                shared_recursion,
                witnesses,
                depth_first,
                (unique_closure, measured),
            );
        }
        self.multiple_recursive_alternatives_impl(
            decision,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates),
        )
    }
    fn multiple_recursive_alternatives_with_queue_mode_observed(
        &self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        context: AdjacentAnalysisContext<'_, '_, '_, '_>,
    ) -> Result<bool, String> {
        let (unique, measured, templates, (adjacent, geometric), exact) = context;
        let context = (unique, measured, templates);
        if adjacent {
            if !context.0 {
                return Err("Adjacent closure batching requires exact-key reservations".into());
            }
            if geometric {
                return self.multiple_recursive_alternatives_with_frontiers::<
                    adjacent::GeometricAdjacentQueue<'_>, frontier::AdoptFrontiers, false,
                >(
                    decision,
                    remaining,
                    shared_recursion,
                    witnesses,
                    depth_first,
                    (context.0, context.1, context.2, exact),
                );
            }
            self.multiple_recursive_alternatives_with_queue_observed::<adjacent::AdjacentQueue<'_>>(
                decision,
                remaining,
                shared_recursion,
                witnesses,
                depth_first,
                (context.0, context.1, context.2, exact),
            )
        } else {
            self.multiple_recursive_alternatives_with_templates(
                decision,
                remaining,
                shared_recursion,
                witnesses,
                depth_first,
                context,
            )
        }
    }
    fn multiple_recursive_alternatives_impl(
        &self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
        ),
    ) -> Result<bool, String> {
        self.multiple_recursive_alternatives_with_queue::<VecDeque<AnalysisConfiguration>>(
            decision,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates),
        )
    }
    fn multiple_recursive_alternatives_with_queue<'graph, Q: adjacent::ClosureQueue<'graph>>(
        &'graph self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
        ),
    ) -> Result<bool, String> {
        self.multiple_recursive_alternatives_with_queue_observed::<Q>(
            decision,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates, None),
        )
    }
    fn multiple_recursive_alternatives_with_queue_observed<
        'graph,
        Q: adjacent::ClosureQueue<'graph>,
    >(
        &'graph self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
    ) -> Result<bool, String> {
        self.multiple_recursive_alternatives_with_frontiers::<Q, frontier::LegacyFrontiers, false>(
            decision,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates, exact),
        )
    }
    fn multiple_recursive_alternatives_with_frontiers<
        'graph,
        Q: adjacent::ClosureQueue<'graph>,
        F: frontier::FrontierPolicy,
        const REUSE_WITNESS_BOUNDS: bool,
    >(
        &'graph self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
    ) -> Result<bool, String> {
        self.multiple_recursive_alternatives_with_frontiers_pool::<Q, F, REUSE_WITNESS_BOUNDS>(
            decision,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates, exact),
            None,
        )
    }
    #[allow(clippy::too_many_arguments)] // Same canonical worker; transient diagnostic only.
    fn multiple_recursive_alternatives_with_frontiers_pool<
        'graph,
        Q: adjacent::ClosureQueue<'graph>,
        F: frontier::FrontierPolicy,
        const REUSE_WITNESS_BOUNDS: bool,
    >(
        &'graph self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
        pool: Option<&PoolWork>,
    ) -> Result<bool, String> {
        self.multiple_recursive_alternatives_with_probe_pool::<Q, F, REUSE_WITNESS_BOUNDS, partial_token_probe::NoProbe>(
            decision, remaining, shared_recursion, witnesses, depth_first,
            (unique_closure, measured, templates, exact), &mut partial_token_probe::NoProbe,
        pool,
)
    }
    #[allow(clippy::too_many_arguments)]
    fn multiple_recursive_alternatives_with_shared_call_ports<'graph>(
        &'graph self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
        pool: Option<&PoolWork>,
    ) -> Result<bool, String> {
        let initial = decision
            .entries
            .iter()
            .enumerate()
            .map(|(alternative, state)| AnalysisConfiguration {
                state: *state,
                alternative,
                calls: analysis_calls::SharedCalls::empty(),
            })
            .collect();
        self.multiple_recursive_configurations_with_call_storage::<
            analysis_calls::SharedCalls,transparent_ports::SharedCallPortQueue<'graph>,
            frontier::AdoptFrontiers,true,partial_token_probe::NoProbe,
        >(initial,remaining,shared_recursion,witnesses,depth_first,
          (unique_closure,measured,templates,exact),&mut partial_token_probe::NoProbe,pool)
    }

    #[allow(clippy::too_many_arguments)] // Same worker plus one transient search policy.
    fn multiple_recursive_alternatives_with_probe<
        'graph,
        Q: adjacent::ClosureQueue<'graph>,
        F: frontier::FrontierPolicy,
        const REUSE_WITNESS_BOUNDS: bool,
        P: partial_token_probe::TokenProbe,
    >(
        &'graph self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
        probe: &mut P,
    ) -> Result<bool, String> {
        self.multiple_recursive_alternatives_with_probe_pool::<Q, F, REUSE_WITNESS_BOUNDS, P>(
            decision,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates, exact),
            probe,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)] // Same canonical worker; transient diagnostic only.
    fn multiple_recursive_alternatives_with_probe_pool<
        'graph,
        Q: adjacent::ClosureQueue<'graph>,
        F: frontier::FrontierPolicy,
        const REUSE_WITNESS_BOUNDS: bool,
        P: partial_token_probe::TokenProbe,
    >(
        &'graph self,
        decision: &Decision,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
        probe: &mut P,
        pool: Option<&PoolWork>,
    ) -> Result<bool, String> {
        let initial: BTreeSet<_> = decision
            .entries
            .iter()
            .enumerate()
            .map(|(alternative, state)| AnalysisConfiguration {
                state: *state,
                alternative,
                calls: Vec::new(),
            })
            .collect();
        self.multiple_recursive_configurations_with_frontiers_pool::<Q, F, REUSE_WITNESS_BOUNDS, P>(
            initial,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates, exact),
            probe,
            pool,
        )
    }
    #[allow(clippy::too_many_arguments)] // Exact full keys, same pool and one disabled/enabled policy.
    #[cfg(test)]
    fn multiple_recursive_configurations_with_frontiers<
        'graph,
        Q: adjacent::ClosureQueue<'graph>,
        F: frontier::FrontierPolicy,
        const REUSE_WITNESS_BOUNDS: bool,
        P: partial_token_probe::TokenProbe,
    >(
        &'graph self,
        initial: BTreeSet<AnalysisConfiguration>,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
        probe: &mut P,
    ) -> Result<bool, String> {
        self.multiple_recursive_configurations_with_frontiers_pool::<Q, F, REUSE_WITNESS_BOUNDS, P>(
            initial,
            remaining,
            shared_recursion,
            witnesses,
            depth_first,
            (unique_closure, measured, templates, exact),
            probe,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)] // Same canonical worker; transient diagnostic only.
    fn multiple_recursive_configurations_with_frontiers_pool<
        'graph,
        Q: adjacent::ClosureQueue<'graph>,
        F: frontier::FrontierPolicy,
        const REUSE_WITNESS_BOUNDS: bool,
        P: partial_token_probe::TokenProbe,
    >(
        &'graph self,
        initial: BTreeSet<AnalysisConfiguration>,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
        probe: &mut P,
        pool: Option<&PoolWork>,
    ) -> Result<bool, String> {
        self.multiple_recursive_configurations_with_call_storage::<analysis_calls::VecCalls,Q,F,REUSE_WITNESS_BOUNDS,P>(
            initial,remaining,shared_recursion,witnesses,depth_first,
            (unique_closure,measured,templates,exact),probe,pool,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn multiple_recursive_configurations_with_call_storage<
        'graph,
        C: analysis_calls::Calls,
        Q: adjacent::ClosureQueue<'graph, C>,
        F: frontier::FrontierPolicy<C>,
        const REUSE_WITNESS_BOUNDS: bool,
        P: partial_token_probe::TokenProbe<C>,
    >(
        &'graph self,
        initial: BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        shared_recursion: bool,
        witnesses: &mut BTreeMap<(usize, usize), bool>,
        depth_first: bool,
        (unique_closure, measured, mut templates, exact): (
            bool,
            Option<&RecursiveAnalysisCounters>,
            Option<&mut CallFreeTemplates<'_>>,
            Option<&AdjacentWorkObservation>,
        ),
        probe: &mut P,
        pool: Option<&PoolWork>,
    ) -> Result<bool, String> {
        let mut call_storage = C::scope(remaining)?;
        let mut pending = <F as frontier::FrontierPolicy<C>>::initial(initial, remaining)?;
        let mut visited = BTreeSet::new();
        let mut recursive = BTreeSet::new();
        let mut truncated = false;
        let mut frontier = 0u64;
        while let Some(frontier_rows) = if depth_first {
            pending.pop_back()
        } else {
            pending.pop_front()
        } {
            let (configurations, paid_token) =
                <F as frontier::FrontierPolicy<C>>::take(frontier_rows);
            frontier = frontier.saturating_add(1);
            observed_recursive_charge(
                remaining,
                Work::FrontierPop,
                measured,
                exact,
                AdjacentCharge::FrontierPop,
            )?;
            if measured_condition(
                configurations
                    .iter()
                    .map(|c| c.alternative)
                    .collect::<BTreeSet<_>>()
                    .len()
                    < 2,
                false,
                measured,
            ) {
                continue;
            }
            // The default Vec facade has no new iteration/debit. The shared
            // representation prepays each new Rc copy before the same visited
            // snapshot is retained; no source/result authority is introduced.
            if C::SHARED {
                for row in &configurations {
                    row.calls.snapshot_copy_fee(&mut call_storage, remaining)?;
                }
            }
            if measured_condition(!visited.insert(configurations.clone()), true, measured) {
                continue;
            }
            let tokens = if let Some(state) = <F as frontier::FrontierPolicy<C>>::uniform_state(
                paid_token,
                &configurations,
                remaining,
            )? {
                // The visited snapshot is the exact full-key set already retained above.
                let (mut closure, mut closed) = frontier::bind_adoption(
                    configurations,
                    &visited,
                    &self.states,
                    state,
                    remaining,
                )?;
                self.analyze_frontier_pool::<C, _, _, REUSE_WITNESS_BOUNDS, P>(
                    &mut closure,
                    &mut closed,
                    FrontierAnalysis {
                        remaining,
                        witnesses,
                        recursive: &mut recursive,
                        truncated: &mut truncated,
                        templates: &mut templates,
                        shared_recursion,
                        unique_closure,
                        measured,
                        exact,
                        frontier,
                        pending_frontiers: pending.len(),
                    },
                    probe,
                    pool,
                    &mut call_storage,
                )?
            } else {
                let mut closed = BTreeSet::new();
                let mut closure = if unique_closure {
                    let mut closure = Q::empty(&self.states);
                    for row in configurations {
                        Q::schedule_observed(
                            row,
                            &mut closure,
                            &mut closed,
                            remaining,
                            (true, ScheduleSite::Seed, measured),
                            exact,
                        )?;
                    }
                    closure
                } else {
                    Q::unreserved(configurations, &self.states)?
                };
                self.analyze_frontier_pool::<C, _, _, REUSE_WITNESS_BOUNDS, P>(
                    &mut closure,
                    &mut closed,
                    FrontierAnalysis {
                        remaining,
                        witnesses,
                        recursive: &mut recursive,
                        truncated: &mut truncated,
                        templates: &mut templates,
                        shared_recursion,
                        unique_closure,
                        measured,
                        exact,
                        frontier,
                        pending_frontiers: pending.len(),
                    },
                    probe,
                    pool,
                    &mut call_storage,
                )?
            };
            let Some(tokens) = tokens else {
                return Ok(true);
            };
            <F as frontier::FrontierPolicy<C>>::extend(&mut pending, tokens, remaining)?;
        }
        if truncated {
            return Err("Recursive prediction analysis remains unresolved".into());
        }
        Ok(false)
    }

    #[allow(clippy::too_many_arguments)] // Same canonical worker; transient diagnostic only.
    fn analyze_frontier_pool<
        'graph,
        C: analysis_calls::Calls,
        Q: adjacent::ClosureQueue<'graph, C>,
        K: frontier::ClosureKeys<C>,
        const REUSE_WITNESS_BOUNDS: bool,
        P: partial_token_probe::TokenProbe<C>,
    >(
        &'graph self,
        closure: &mut Q,
        closed: &mut K,
        context: FrontierAnalysis<'_, '_, '_>,
        probe: &mut P,
        pool: Option<&PoolWork>,
        call_storage: &mut C::Scope,
    ) -> Result<Option<AnalysisTokens<C>>, String> {
        let FrontierAnalysis {
            remaining,
            witnesses,
            recursive,
            truncated,
            templates,
            shared_recursion,
            unique_closure,
            measured,
            exact,
            frontier,
            pending_frontiers,
        } = context;
        let mut shared_calls: BTreeMap<usize, BTreeMap<usize, usize>> = BTreeMap::new();
        let mut tokens: BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration<C>>> =
            BTreeMap::new();
        while let Some(popped) = closure.pop_for_analysis(remaining, pool)? {
            let row = popped.row();
            if let Some(work) = measured {
                // The active Historical queues pop without semantic charges.
                work.selected_static_row(row.state, *remaining);
                work.closure_pop();
            }
            if let Some(work) = exact {
                let (queue_batches, front_siblings) = closure.observed_shape();
                work.context(AdjacentContext {
                    event: AdjacentEvent::ClosurePop,
                    frontier,
                    state: row.state,
                    alternative: row.alternative,
                    call_depth: row.calls.len(),
                    pending_frontiers,
                    queue_batches,
                    front_siblings,
                });
            }
            let share_before = *remaining;
            let share_scope =
                pool.and_then(|work| work.enter(PoolBlock::QueueShareRemainder, share_before));
            let shared = match popped {
                adjacent::AnalysisPop::Opaque(boundary) => {
                    closure.share_opaque_boundary(boundary, remaining, pool)
                }
                adjacent::AnalysisPop::Ordinary(row) => {
                    if let Some(work) = pool {
                        closure.share_with_pool(row, closed, remaining, measured, exact, work)
                    } else {
                        K::share(row, closure, closed, remaining, measured, exact)
                    }
                }
            };
            if let Some(work) = pool {
                work.leave(share_scope, *remaining, shared.is_ok());
                work.queue_span(Q::POOL_QUEUE_KIND, share_before, *remaining);
            }
            let Some(mut row) = shared? else {
                continue;
            };
            if !unique_closure {
                observed_recursive_charge(
                    remaining,
                    Work::ClosureVisit,
                    measured,
                    exact,
                    AdjacentCharge::LegacyClosureVisit,
                )?;
                let unique = closed.insert_reserved(row.clone());
                if let Some(work) = measured {
                    work.legacy_key(unique);
                }
                if !unique {
                    continue;
                }
            }
            let state = self
                .states
                .get(row.state)
                .ok_or("Dangling parser prediction state")?;
            if state.rule_stop {
                if let Some(((_, follow), rest)) = row.calls.popped(call_storage, remaining)? {
                    K::schedule(
                        AnalysisConfiguration::<C> {
                            state: follow,
                            calls: rest,
                            ..row
                        },
                        closure,
                        closed,
                        remaining,
                        (unique_closure, ScheduleSite::Return, measured),
                        exact,
                    )?;
                } else {
                    for target in &state.follow {
                        observed_recursive_charge(
                            remaining,
                            Work::CallerlessFollowEdge,
                            measured,
                            exact,
                            AdjacentCharge::CallerlessFollowEdge,
                        )?;
                        K::schedule(
                            AnalysisConfiguration::<C> {
                                state: *target,
                                ..row.clone()
                            },
                            closure,
                            closed,
                            remaining,
                            (unique_closure, ScheduleSite::Follow, measured),
                            exact,
                        )?;
                    }
                }
                continue;
            }
            for edge in &state.edges {
                if let Some(work) = measured {
                    // Close a prior Call's early-continue path BEFORE this edge
                    // receives its own canonical transition debit.
                    work.selected_static_call_end(*remaining);
                }
                observed_recursive_charge(
                    remaining,
                    Work::TransitionEdge,
                    measured,
                    exact,
                    AdjacentCharge::TransitionEdge,
                )?;
                if let Some(work) = exact {
                    work.edge(observed_edge(edge));
                }
                let mut next = row.clone();
                match edge {
                    Edge::Epsilon(target) | Edge::Action(target) => {
                        // The original edge debit and context clone above have
                        // already happened. The default hook is literal/no-fee.
                        next.state = closure.internal_target(*target, remaining)?;
                    }
                    Edge::First { target, .. } | Edge::Full { target, .. } => {
                        next.state = *target;
                    }
                    Edge::Token(symbol, target) => {
                        next.state = *target;
                        let rows = tokens.entry(symbol.clone()).or_default();
                        rows.insert(next);
                        // The original closure and its complete token map remain
                        // owned here. This is only a positive existential probe
                        // on actual full successor keys, never a negative fact.
                        if frontier == 1
                            && let Some(initial) = probe.select(
                                symbol, row.alternative, rows, remaining,
                            )?
                            && self.multiple_recursive_configurations_with_call_storage::<C,
                                adjacent::GeometricAdjacentQueue<'_,C>,
                                frontier::AdoptFrontiers,
                                true,
                                partial_token_probe::NoProbe,
                            >(
                                initial, remaining, shared_recursion, witnesses,
                                probe.depth_first(),
                                (true, measured, templates.as_deref_mut(), exact),
                                &mut partial_token_probe::NoProbe,
                            pool,
)?
                        {
                            return Ok(None);
                        }
                        continue;
                    }
                    Edge::Call { target, follow } => {
                        if let Some(work) = measured {
                            work.selected_static_call_begin(*target, *remaining);
                        }
                        if shared_recursion {
                            let callers = shared_calls.entry(*target).or_default();
                            callers
                                .entry(row.alternative)
                                .and_modify(|depth| *depth = (*depth).min(row.calls.len()))
                                .or_insert(row.calls.len());
                            if callers.len() > 1 {
                                let base = callers
                                    .values()
                                    .copied()
                                    .max()
                                    .ok_or("Missing shared caller")?;
                                if base < self.call_depth() {
                                    let bound = self.call_depth() - base;
                                    let key = (*target, bound);
                                    let recursive = if let Some(value) = witnesses.get(&key) {
                                        if let Some(work) = measured {
                                            work.witness_hit();
                                        }
                                        *value
                                    } else {
                                        if let Some(work) = measured {
                                            work.witness_miss();
                                        }
                                        // This remains an EXACT-key miss in the old counters.
                                        // Bound reuse never fabricates a hit or worker result.
                                        let completed = if REUSE_WITNESS_BOUNDS {
                                            bounded_witness::lookup_completed(
                                                witnesses, *target, bound, remaining,
                                            )?
                                        } else {
                                            None
                                        };
                                        if let Some(value) = completed {
                                            value
                                        } else {
                                            let before = *remaining;
                                            let value = if exact.is_some() {
                                                self.recursion_witness_observed(
                                                    *target, bound, remaining, measured, exact,
                                                )
                                            } else if measured.is_some() {
                                                self.recursion_witness_measured(
                                                    *target, bound, remaining, measured,
                                                )
                                            } else {
                                                self.recursion_witness(*target, bound, remaining)
                                            };
                                            if let Some(work) = measured {
                                                work.witness_result(
                                                    &value,
                                                    before.saturating_sub(*remaining),
                                                );
                                            }
                                            let value = value?;
                                            witnesses.insert(key, value);
                                            value
                                        }
                                    };
                                    if recursive {
                                        return Ok(None);
                                    }
                                }
                            }
                        }
                        let depth = row
                            .calls
                            .iter()
                            .filter(|(site, _)| *site == row.state)
                            .count();
                        if depth > 0 {
                            recursive.insert(row.alternative);
                            if recursive.len() > 1 {
                                return Ok(None);
                            }
                        }
                        if depth >= 2 {
                            *truncated = true;
                            continue;
                        }
                        if next.calls.len() >= self.call_depth() {
                            return Err("Parser prediction call depth exceeded".into());
                        }
                        next.calls
                            .pushed((row.state, *follow), call_storage, remaining)?;
                        next.state = *target;
                        if let Some(work) = measured {
                            work.selected_static_call_end(*remaining);
                        }
                        // Original Call/shared-site/witness/depth operations above
                        // run before any optional syntax-only template query.
                        if let Some(templates) = templates.as_deref_mut()
                            && templates.replay_call_recorded(
                                self,
                                &next,
                                &mut tokens,
                                remaining,
                                exact,
                                measured,
                            )?
                        {
                            continue;
                        }
                    }
                }
                K::schedule(
                    next,
                    closure,
                    closed,
                    remaining,
                    (unique_closure, ScheduleSite::Successor, measured),
                    exact,
                )?;
            }
        }
        if let Some(work) = measured {
            work.selected_static_row_end(*remaining);
        }
        Ok(Some(tokens))
    }
    /// FIRST of the actual expression continuation with explicit live caller
    /// return sites. A rule stop with no caller is EOF; unrelated callers are
    /// never taken from the grammar-wide FOLLOW union. Uses this same charged
    /// NFA, without running model actions or an independent recovery grammar.
    pub fn recovery_symbols_with_work(
        &self,
        path: &str,
        callers: &[&str],
        remaining: &mut Option<usize>,
    ) -> Result<BTreeSet<PredictionSymbol>, String> {
        if !self.decisions.contains_key("@document-recovery") {
            return Err("Document recovery requires its explicit source-checked profile".into());
        }
        let continuation = |path: &str| -> Result<usize, String> {
            let decision = self
                .decisions
                .get(&format!("{path}/recovery"))
                .ok_or("Missing source-checked recovery occurrence")?;
            decision
                .entries
                .get(1)
                .copied()
                .ok_or_else(|| "Invalid recovery continuation".into())
        };
        let mut returns = Vec::new();
        for caller in callers {
            charge(remaining)?;
            if returns.len() >= self.call_depth() {
                return Err("Parser recovery call depth exceeded".into());
            }
            returns.push(continuation(caller)?);
        }
        let mut pending = VecDeque::from([(continuation(path)?, returns)]);
        let mut seen = BTreeSet::new();
        let mut symbols = BTreeSet::new();
        while let Some((at, returns)) = pending.pop_front() {
            charge(remaining)?;
            if !seen.insert((at, returns.clone())) {
                continue;
            }
            let state = self.states.get(at).ok_or("Dangling recovery state")?;
            if state.rule_stop {
                if let Some((&target, rest)) = returns.split_last() {
                    pending.push_back((target, rest.to_vec()));
                } else {
                    symbols.insert(PredictionSymbol::End);
                }
                continue;
            }
            for edge in &state.edges {
                charge(remaining)?;
                match edge {
                    Edge::Epsilon(target)
                    | Edge::Action(target)
                    | Edge::First { target, .. }
                    | Edge::Full { target, .. } => {
                        pending.push_back((*target, returns.clone()));
                    }
                    Edge::Token(symbol, _) => {
                        symbols.insert(symbol.clone());
                    }
                    Edge::Call { target, follow } => {
                        if returns.len() >= self.call_depth() {
                            return Err("Parser recovery call depth exceeded".into());
                        }
                        let mut called = returns.clone();
                        called.push(*follow);
                        pending.push_back((*target, called));
                    }
                }
            }
        }
        Ok(symbols)
    }
    pub fn decisions(&self) -> impl Iterator<Item = &str> {
        self.decisions.keys().map(String::as_str)
    }
    fn state(&mut self) -> Result<usize, String> {
        if self.states.len() >= 10_000 {
            return Err("Grammar prediction budget exceeded".into());
        }
        let id = self.states.len();
        self.states.push(State::default());
        Ok(id)
    }
    fn epsilon(&mut self, from: usize, to: usize) {
        self.states[from].edges.push(Edge::Epsilon(to));
    }
    fn expression(
        &mut self,
        expr: &Expr,
        start: usize,
        end: usize,
        path: &str,
        (depth, mut certified, continuation): (
            usize,
            Option<&mut GraphBuildContext<'_, '_, '_>>,
            SequenceContinuation<'_, '_>,
        ),
        observed_calls: Option<&std::cell::Cell<bool>>,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err("Grammar prediction budget exceeded".into());
        }
        if self.decisions.contains_key("@document-recovery") {
            self.decisions.insert(
                format!("{path}/recovery"),
                Decision {
                    entries: vec![start, end],
                    exit: None,
                },
            );
        }
        match expr.cardinality {
            '1' => self.body(
                &expr.kind,
                start,
                end,
                path,
                (depth, certified),
                observed_calls,
            ),
            '?' | '*' | '+' => {
                let body = self.state()?;
                let done = self.state()?;
                let decision = if expr.cardinality == '+' { done } else { start };
                self.epsilon(decision, body);
                self.epsilon(decision, end);
                if expr.cardinality == '+' {
                    self.epsilon(start, body);
                }
                if expr.cardinality == '*' {
                    self.epsilon(done, start);
                }
                if expr.cardinality == '?' {
                    self.epsilon(done, end);
                }
                self.decisions.insert(
                    format!("{path}/repeat"),
                    Decision {
                        entries: vec![body, end],
                        exit: Some(1),
                    },
                );
                self.body(
                    &expr.kind,
                    body,
                    done,
                    path,
                    (depth, certified.as_deref_mut()),
                    observed_calls,
                )?;
                if let Some(context) = certified {
                    context.bind(
                        expr,
                        path,
                        GraphBinding {
                            start,
                            body,
                            stop: done,
                            end,
                        },
                        continuation,
                    )?;
                }
                Ok(())
            }
            _ => Err("Unsupported parser prediction cardinality".into()),
        }
    }
    fn call(
        &mut self,
        name: &str,
        start: usize,
        end: usize,
        observed_calls: Option<&std::cell::Cell<bool>>,
    ) {
        if let Some(&(target, stop)) = self.rules.get(name) {
            if let Some(observed) = observed_calls {
                observed.set(true);
            }
            self.states[start].edges.push(Edge::Call {
                target,
                follow: end,
            });
            if !self.states[stop].follow.contains(&end) {
                self.states[stop].follow.push(end);
            }
        } else {
            self.states[start]
                .edges
                .push(Edge::Token(PredictionSymbol::Terminal(name.into()), end));
        }
    }
    fn body(
        &mut self,
        kind: &Kind,
        start: usize,
        end: usize,
        path: &str,
        (depth, mut certified): (usize, Option<&mut GraphBuildContext<'_, '_, '_>>),
        observed_calls: Option<&std::cell::Cell<bool>>,
    ) -> Result<(), String> {
        let start = if self.first_guards.contains_key(path) {
            let target = self.state()?;
            self.states[start].edges.push(Edge::First {
                path: path.into(),
                target,
            });
            target
        } else {
            start
        };
        match kind {
            Kind::Action(_) | Kind::Capture { .. } => {
                self.states[start].edges.push(Edge::Action(end))
            }
            Kind::Literal(value) => self.states[start]
                .edges
                .push(Edge::Token(PredictionSymbol::Keyword(value.clone()), end)),
            Kind::Reference(_) => self.call("ID", start, end, observed_calls),
            Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
                self.call(name, start, end, observed_calls)
            }
            Kind::SyntacticPredicate(value) => {
                let target = self.state()?;
                self.states[start]
                    .edges
                    .push(Edge::Full { target, stop: end });
                self.expression(
                    value,
                    target,
                    end,
                    &format!("{path}/value"),
                    (depth + 1, certified, None),
                    observed_calls,
                )?;
            }
            Kind::Assign { value, .. } | Kind::BooleanAssign { value, .. } => self.expression(
                value,
                start,
                end,
                &format!("{path}/value"),
                (depth + 1, certified, None),
                observed_calls,
            )?,
            Kind::Sequence(items) => {
                let mut previous = start;
                for (index, item) in items.iter().enumerate() {
                    let next = if index + 1 == items.len() {
                        end
                    } else {
                        self.state()?
                    };
                    self.expression(
                        item,
                        previous,
                        next,
                        &format!("{path}/s{index}"),
                        (
                            depth + 1,
                            certified.as_deref_mut(),
                            Some((items.as_slice(), index + 1, path)),
                        ),
                        observed_calls,
                    )?;
                    previous = next;
                }
                if items.is_empty() {
                    self.epsilon(start, end);
                }
            }
            Kind::Alternative(items) => {
                let mut entries = Vec::new();
                for (index, item) in items.iter().enumerate() {
                    let branch = self.state()?;
                    self.epsilon(start, branch);
                    entries.push(branch);
                    self.expression(
                        item,
                        branch,
                        end,
                        &format!("{path}/a{index}"),
                        (depth + 1, certified.as_deref_mut(), None),
                        observed_calls,
                    )?;
                }
                if entries.is_empty() {
                    return if self.allow_empty_choices {
                        Ok(())
                    } else {
                        Err("Empty parser prediction decision".into())
                    };
                }
                self.decisions.insert(
                    format!("{path}/choice"),
                    Decision {
                        entries,
                        exit: None,
                    },
                );
            }
        }
        Ok(())
    }
    /// Return the selected zero-based branch, or None for no viable branch / exit.
    /// Selection commits as soon as one alternative remains. This must not scan
    /// an unrelated suffix or execute an action to determine the selected branch.
    pub fn predict(
        &self,
        decision: &str,
        mut input: impl FnMut() -> Result<Option<PredictionSymbol>, String>,
        remaining: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_with_input_work(decision, |_| input(), remaining)
    }
    /// Same prediction engine with a charged retained-stream input adapter.
    /// Legacy callers retain their original callback and exact work stream.
    pub fn predict_with_input_work(
        &self,
        decision: &str,
        input: impl FnMut(&mut Option<usize>) -> Result<Option<PredictionSymbol>, String>,
        remaining: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_with_replay_input(decision, &mut ForwardInput(input), remaining)
    }

    /// Use the same predictor with caller-owned, charged syntax input. A full
    /// syntactic guard requires checkpoint/rewind; no token tape is allocated.
    pub fn predict_with_replay_input(
        &self,
        decision: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_with_replay_input_and_completion_verification(
            decision,
            input,
            remaining,
            CompletionVerification::Legacy,
        )
    }

    /// Opt into a sufficient completion proof without changing actual Full
    /// recognition, guard visibility or the caller's shared allowance.
    /// Direct callers own this explicit capability. Checked language hosts use
    /// their selected profile; serialized prediction data cannot enable it.
    pub fn predict_with_replay_input_and_completion_verification(
        &self,
        decision: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String> {
        if self.requires_runtime_capability() {
            return Err("Parser program requires its typed bounded live runtime capability".into());
        }
        let recursive_first = self.recursive_decisions.contains(decision);
        let decision = self
            .decisions
            .get(decision)
            .ok_or("Unknown parser prediction decision")?;
        match self.run_prediction_at_depth_with_completion_verification(
            PredictionGoal::Decision {
                decision,
                recursive_first,
                boundary: None,
            },
            input,
            remaining,
            0,
            verification,
        )? {
            PredictionOutcome::Selected(selected) => Ok(selected),
            PredictionOutcome::Completed(_) | PredictionOutcome::InitialPrepared(_) => {
                Err("Unexpected completion outcome for a decision".into())
            }
        }
    }

    /// Inspect one complete rule operand using the shared syntax-only engine.
    /// This low-level prerequisite does not admit full syntactic predicates.
    /// Inputs are already visible/classified tokens owned by the caller. A finite
    /// work allowance is mandatory and covers static verification and all reads.
    /// Nested decisions use this same evaluator and exact return context.
    /// Guard visibility or static commitment not proved by the scope rejects.
    pub fn probe_rule_completion(
        &self,
        rule: &str,
        symbols: &[PredictionSymbol],
        remaining: &mut Option<usize>,
    ) -> Result<bool, String> {
        if self.requires_runtime_capability() {
            return Err("Parser program requires its typed bounded live runtime capability".into());
        }
        let &(start, stop) = self.rules.get(rule).ok_or("Unknown completion rule")?;
        let work = remaining
            .as_mut()
            .ok_or("Rule completion requires a finite shared prediction work budget")?;
        let scope = self.verify_completion_scope(start, stop, work)?;
        let mut input = ReplayTokens::new(symbols);
        let checkpoint = input.checkpoint()?;
        let result = self.run_prediction(
            PredictionGoal::Completion {
                start,
                stop,
                scope: &scope,
                productivity: CompletionProductivity::Any,
            },
            &mut input,
            remaining,
        );
        // Restore syntax position on success, failed recognition and budget
        // failure. Consumed work remains consumed; no model action ran.
        input.rewind(checkpoint)?;
        match result? {
            PredictionOutcome::Completed(matched) => Ok(matched),
            PredictionOutcome::InitialPrepared(_) => {
                Err("Unexpected initial preparation outcome for completion".into())
            }
            PredictionOutcome::Selected(_) => {
                Err("Unexpected decision outcome for completion".into())
            }
        }
    }

    // SAME canonical guard-priority qualification used by all completion
    // policies. It grants no new source, runtime or disjointness authority.
    fn completion_entry_priority(
        &self,
        decision: &Decision,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        let mut priority = decision.entries.len() > 1;
        for &entry in decision
            .entries
            .iter()
            .take(decision.entries.len().saturating_sub(1))
        {
            analysis_charge(remaining)?;
            let state = self.states.get(entry).ok_or("Dangling completion entry")?;
            priority &= match state.edges.as_slice() {
                [Edge::Full { .. }] => true,
                [Edge::First { path, .. }] => self
                    .first_guards
                    .get(path)
                    .is_some_and(|symbols| !symbols.is_empty()),
                _ => false,
            };
        }
        Ok(priority)
    }

    fn verify_completion_scope<'program>(
        &'program self,
        start: usize,
        stop: usize,
        remaining: &mut usize,
    ) -> Result<CompletionScope<'program>, String> {
        self.verify_completion_scope_with_verification(
            start,
            stop,
            remaining,
            CompletionVerification::Legacy,
        )
    }

    fn verify_completion_scope_with_verification<'program>(
        &'program self,
        start: usize,
        stop: usize,
        remaining: &mut usize,
        verification: CompletionVerification,
    ) -> Result<CompletionScope<'program>, String> {
        self.verify_completion_scope_with_local_decision(start, stop, remaining, verification, None)
    }

    fn verify_completion_scope_with_local_decision<'program>(
        &'program self,
        start: usize,
        stop: usize,
        remaining: &mut usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
    ) -> Result<CompletionScope<'program>, String> {
        self.verify_completion_scope_with_reach(start, stop, remaining, verification, local)
            .map(|(scope, _)| scope)
    }

    // Return the SAME already-built reach set on complete success only. No
    // clone, second traversal, new allowance or retained runtime fact is made.
    fn verify_completion_scope_with_reach<'program>(
        &'program self,
        start: usize,
        stop: usize,
        remaining: &mut usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
    ) -> Result<(CompletionScope<'program>, BTreeSet<usize>), String> {
        // This is carved out of the caller's allowance and never refunded.
        let allowance = *remaining;
        let mut proof_remaining = allowance;
        let result = self.verify_completion_scope_inner_with_local_decision(
            start,
            stop,
            &mut proof_remaining,
            verification,
            local,
        );
        *remaining -= allowance - proof_remaining;
        result
    }

    fn verify_completion_scope_inner_with_local_decision<'program>(
        &'program self,
        start: usize,
        stop: usize,
        remaining: &mut usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
    ) -> Result<(CompletionScope<'program>, BTreeSet<usize>), String> {
        if self.states.len() > 10_000 {
            return Err("Grammar prediction budget exceeded".into());
        }
        let mut pending = vec![start];
        let mut reached = BTreeSet::new();
        let mut guard_states = BTreeMap::new();
        while let Some(index) = pending.pop() {
            analysis_charge(remaining)?;
            if !reached.insert(index) {
                continue;
            }
            let state = self.states.get(index).ok_or("Dangling completion state")?;
            if index == stop || state.rule_stop {
                continue;
            }
            if state.accepting {
                return Err("Completion scope escapes its exact rule boundary".into());
            }
            for edge in &state.edges {
                analysis_charge(remaining)?;
                match edge {
                    Edge::Epsilon(target) | Edge::Action(target) | Edge::Token(_, target) => {
                        pending.push(*target)
                    }
                    Edge::Call { target, follow } => {
                        pending.push(*follow);
                        pending.push(*target);
                    }
                    Edge::Full { target, stop } => {
                        if self.states.get(*stop).is_none() {
                            return Err("Dangling full-predicate operand boundary".into());
                        }
                        guard_states.insert(index, CompletionGuard::Full);
                        pending.push(*target);
                    }
                    Edge::First { path, target } => {
                        if self
                            .first_guards
                            .get(path)
                            .is_none_or(|symbols| symbols.is_empty())
                        {
                            return Err(format!("Unbound completion first-token guard at {path}"));
                        }
                        guard_states.insert(index, CompletionGuard::First(path.as_str()));
                        pending.push(*target);
                    }
                }
            }
        }
        // Match a real decision's ordered entries against actual epsilon
        // fan-out. No rule owner/path is guessed from a state number.
        let mut fanouts: BTreeMap<Vec<usize>, Vec<usize>> = BTreeMap::new();
        for &index in &reached {
            analysis_charge(remaining)?;
            let state = self.states.get(index).ok_or("Dangling completion state")?;
            if index == stop || state.rule_stop || state.edges.is_empty() {
                continue;
            }
            let mut targets = Vec::new();
            for edge in &state.edges {
                analysis_charge(remaining)?;
                if let Edge::Epsilon(target) = edge {
                    targets.push(*target);
                } else {
                    targets.clear();
                    break;
                }
            }
            if state.edges.len() > 1 && targets.is_empty() {
                return Err("Completion contains an unbound branch state".into());
            }
            if !targets.is_empty() {
                fanouts.entry(targets).or_default().push(index);
            }
        }
        let mut scope = CompletionScope {
            decisions: BTreeMap::new(),
            guards: BTreeMap::new(),
        };
        for (path, decision) in &self.decisions {
            analysis_charge(remaining)?;
            // These source-checked entries describe error recovery, not a
            // choice or repetition executed by the model parser.
            if path == "@document-recovery" || path.ends_with("/recovery") {
                continue;
            }
            if decision.entries.is_empty()
                || !decision.entries.iter().all(|entry| reached.contains(entry))
            {
                continue;
            }
            let points = fanouts
                .get(&decision.entries)
                .ok_or_else(|| format!("Completion cannot bind prediction decision {path}"))?;
            let priority = self.completion_entry_priority(decision, remaining)?;
            let local_bound = if let Some(local) = local {
                analysis_charge(remaining)?;
                if path == local.path {
                    analysis_charge(remaining)?;
                    if !std::ptr::eq(decision, local.decision)
                        || self.recursive_decisions.contains(path)
                    {
                        return Err(
                            "Rule-local completion choice differs from its qualified decision"
                                .into(),
                        );
                    }
                    let mut actual_entry = false;
                    for point in points {
                        analysis_charge(remaining)?;
                        if *point == local.entry {
                            actual_entry = true;
                        }
                    }
                    if !actual_entry {
                        return Err(
                            "Rule-local completion choice lacks its actual source fan-out".into(),
                        );
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            };
            // InputChecked can prove an asymmetric entry-guard shape without
            // declaring its earlier unguarded alternatives committed. Runtime
            // selection still has to decide on the actual input and live tail.
            let input_checked_guarded = !local_bound
                && !priority
                && verification == CompletionVerification::InputCheckedCompletion
                && (self.input_checked_asymmetric_guarded_choice(
                    path, decision, &reached, remaining,
                )? || self
                    .input_checked_multiway_choice(path, decision, &reached, remaining)?);
            if local_bound {
                // Exact action + one optional keyword was bound at its own
                // real rule stop. This is a deterministic local decision, not
                // a manufactured guard or a claim of caller-language disjointness.
            } else if priority || input_checked_guarded {
                // Register only actual visible entry guards. The priority
                // branch retains its existing commitment proof; the asymmetric
                // branch grants ONLY the exact per-input replay obligation.
                // Both use the same Decision evaluator, never NFA membership.
                for &entry in &decision.entries {
                    analysis_charge(remaining)?;
                    if let Some(&guard) = guard_states.get(&entry) {
                        scope.guards.insert(entry, guard);
                    }
                }
            } else {
                if self.recursive_decisions.contains(path) {
                    return Err(format!("Unqualified completion commitment at {path}"));
                }
                // Selection is before the static proof, never a retry after a
                // budget/error. All other choices retain the existing proof.
                let input_checked = verification == CompletionVerification::InputCheckedCompletion
                    && (self.input_checked_literal_repeat(path, decision, points, remaining)?
                        || self
                            .input_checked_call_suffix_repeat(path, decision, points, remaining)?
                        || self.input_checked_ordinary_choice(path, decision, remaining)?
                        || self
                            .input_checked_one_shot_optional(path, decision, points, remaining)?);
                if !input_checked {
                    self.verify_completion_decision_with_verification(
                        path,
                        remaining,
                        verification,
                    )
                    .map_err(|message| {
                        format!("Completion static verification failed at {path}: {message}")
                    })?;
                    if verification == CompletionVerification::InputCheckedCompletion {
                        // This exact decision has now passed the existing static
                        // proof. Its real entry guards still execute in the same
                        // nested Decision predictor before completion traverses
                        // the selected body. This does not expose hidden guards
                        // or grant an unproved alternative early commitment.
                        for &entry in &decision.entries {
                            analysis_charge(remaining)?;
                            if let Some(&guard) = guard_states.get(&entry) {
                                scope.guards.insert(entry, guard);
                            }
                        }
                    }
                }
            }
            for &point in points {
                analysis_charge(remaining)?;
                let bound = BoundCompletionDecision {
                    path,
                    decision,
                    recursive_first: self.recursive_decisions.contains(path),
                };
                if let Some(old) = scope.decisions.insert(point, bound)
                    && old.path != path.as_str()
                {
                    return Err("Conflicting completion decision bindings".into());
                }
            }
        }
        for (state, guard) in guard_states {
            analysis_charge(remaining)?;
            if scope.guards.get(&state) != Some(&guard) {
                return Err(match guard {
                    CompletionGuard::First(path) => {
                        format!("Completion guard visibility remains unsupported at {path}")
                    }
                    CompletionGuard::Full => {
                        format!("Completion guard visibility remains unsupported at state {state}")
                    }
                });
            }
        }
        for (targets, points) in fanouts {
            if targets.len() > 1 {
                for point in points {
                    analysis_charge(remaining)?;
                    if !scope.decisions.contains_key(&point) {
                        return Err("Completion contains an unbound branch state".into());
                    }
                }
            }
        }
        Ok((scope, reached))
    }

    // One evaluator owns decision and completion traversal. Decision mode keeps
    // its old early commitment, callback sequence and every charge in place.
    fn run_prediction(
        &self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
    ) -> Result<PredictionOutcome, String> {
        self.run_prediction_at_depth(goal, input, remaining, 0)
    }

    fn run_prediction_at_depth(
        &self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        base_depth: usize,
    ) -> Result<PredictionOutcome, String> {
        self.run_prediction_at_depth_with_completion_verification(
            goal,
            input,
            remaining,
            base_depth,
            CompletionVerification::Legacy,
        )
    }

    fn run_prediction_at_depth_with_completion_verification(
        &self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        base_depth: usize,
        verification: CompletionVerification,
    ) -> Result<PredictionOutcome, String> {
        self.run_prediction_at_depth_with_local_decision(
            goal,
            input,
            remaining,
            base_depth,
            verification,
            None,
        )
    }

    fn run_prediction_at_depth_with_local_decision(
        &self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        base_depth: usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
    ) -> Result<PredictionOutcome, String> {
        self.run_prediction_at_depth_with_local_decision_and_source(
            goal,
            input,
            remaining,
            base_depth,
            verification,
            local,
            &mut runtime_source::NoInitialSource,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_prediction_at_depth_with_local_decision_and_source<
        'program,
        S: runtime_source::InitialSource<'program>,
    >(
        &'program self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        base_depth: usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
        sources: &mut S,
    ) -> Result<PredictionOutcome, String> {
        self.run_prediction_with_live_seed(
            goal,
            input,
            remaining,
            base_depth,
            verification,
            local,
            sources,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_prediction_with_live_seed<'program, S: runtime_source::InitialSource<'program>>(
        &'program self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        base_depth: usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
        sources: &mut S,
        live: Option<&live_callers::LiveSeed<'_, 'program>>,
    ) -> Result<PredictionOutcome, String> {
        self.run_prediction_with_return_storage::<Vec<usize>, S>(
            goal,
            input,
            remaining,
            base_depth,
            verification,
            local,
            sources,
            live,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn run_prediction_with_return_storage<
        'program,
        R: shared_returns::Returns,
        S: runtime_source::InitialSource<'program, R>,
    >(
        &'program self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        base_depth: usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
        sources: &mut S,
        live: Option<&live_callers::LiveSeed<'_, 'program>>,
    ) -> Result<PredictionOutcome, String> {
        self.run_prediction_with_return_storage_seeded::<R, S>(
            goal,
            input,
            remaining,
            base_depth,
            verification,
            local,
            sources,
            live,
            None,
        )
    }

    /// The completion scope of the grammar's root rule, verified once for the
    /// live decisions of one parse.
    fn live_root_scope(
        &self,
        root_stop: usize,
        work: &mut usize,
        verification: CompletionVerification,
    ) -> Result<CompletionScope<'_>, String> {
        let mut root_start = None;
        for &(start, stop) in self.rules.values() {
            analysis_charge(work)?;
            if stop == root_stop && root_start.replace(start).is_some() {
                return Err("Live root scope has an ambiguous root endpoint".into());
            }
        }
        let root_start = root_start.ok_or("Live root scope has no exact root")?;
        self.verify_completion_scope_with_verification(root_start, root_stop, work, verification)
    }

    /// Predicts a deferred nested decision from a shallow stack and retains its
    /// selection, so the retried outer prediction reuses it.
    fn resolve_deferred_nested_decision<'program>(
        &'program self,
        deferred: DeferredNestedDecision,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
        seed: &live_callers::LiveSeed<'_, 'program>,
    ) -> Result<(), String> {
        let DeferredNestedDecision { key, state } = deferred;
        if key.root != Some((seed.root_stop, seed.eof))
            || key.verification != verification
            || key.local.is_some()
        {
            return Err("Deferred nested decision differs from its live root".into());
        }
        let verified;
        let scope = if let Some(scope) = seed.scope {
            scope
        } else {
            let work = remaining
                .as_mut()
                .ok_or("Deferred nested decision requires finite original work")?;
            verified = self.live_root_scope(seed.root_stop, work, verification)?;
            &verified
        };
        let bound = scope
            .decisions
            .get(&state)
            .ok_or("Deferred nested decision lost its binding")?;
        if std::ptr::from_ref(bound.decision) as usize != key.decision {
            return Err("Deferred nested decision differs from its binding".into());
        }
        let goal = PredictionGoal::Decision {
            decision: bound.decision,
            recursive_first: bound.recursive_first,
            boundary: Some(DecisionBoundary {
                live: key.live,
                stop: key.stop,
                returns: &key.returns,
            }),
        };
        let nested = live_callers::LiveSeed {
            returns: &key.returns,
            root_stop: seed.root_stop,
            eof: seed.eof,
            scope: Some(scope),
            scopes: seed.scopes,
        };
        loop {
            input.rewind(key.start)?;
            let outcome = if key.nested_live {
                self.run_prediction_with_live_seed(
                    goal,
                    input,
                    remaining,
                    0,
                    verification,
                    None,
                    &mut runtime_source::NoInitialSource,
                    Some(&nested),
                )
            } else {
                self.run_prediction_at_depth_with_local_decision(
                    goal,
                    input,
                    remaining,
                    0,
                    verification,
                    None,
                )
            };
            match outcome {
                Err(error) if error == NESTED_DECISION_DEFERRED => {
                    let chain = input.take_deferred_nested_decisions();
                    if chain.is_empty() {
                        return Err("Deferred nested decision disappeared".into());
                    }
                    for inner in chain {
                        self.resolve_deferred_nested_decision(
                            inner,
                            input,
                            remaining,
                            verification,
                            seed,
                        )?;
                    }
                }
                Err(error) => return Err(error),
                Ok(PredictionOutcome::Selected(selected)) => {
                    let selected = selected.or(bound.decision.exit);
                    input.record_nested_decision_outcome(key, selected);
                    return Ok(());
                }
                Ok(PredictionOutcome::Completed(_) | PredictionOutcome::InitialPrepared(_)) => {
                    return Err("Nested decision returned completion".into());
                }
            }
        }
    }

    fn is_input_checked_live_choice(
        &self,
        path: &str,
        remaining: &mut Option<usize>,
    ) -> Result<bool, String> {
        charge(remaining)?;
        if !path.ends_with("/choice") {
            return Ok(false);
        }
        let Some(runtime_guarded::Requirement::InputCheckedCompletionScopes { decisions }) =
            self.runtime_requirement.as_deref()
        else {
            return Ok(false);
        };
        let levels = usize::BITS as usize - decisions.len().max(1).leading_zeros() as usize;
        let units = levels
            .checked_mul(2)
            .and_then(|n| n.checked_mul(1 + path.len().div_ceil(64)))
            .and_then(|n| n.checked_add(1))
            .ok_or("Nested live obligation lookup overflow")?;
        for _ in 0..units {
            charge(remaining)?;
        }
        Ok(decisions.contains(path))
    }

    #[allow(clippy::too_many_arguments)]
    fn run_prediction_with_return_storage_seeded<
        'program,
        R: shared_returns::Returns,
        S: runtime_source::InitialSource<'program, R>,
    >(
        &'program self,
        goal: PredictionGoal<'_>,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        base_depth: usize,
        verification: CompletionVerification,
        local: Option<RuleLocalCompletionDecision<'_>>,
        sources: &mut S,
        live: Option<&live_callers::LiveSeed<'_, 'program>>,
        mut shared_seed: Option<R>,
    ) -> Result<PredictionOutcome, String> {
        runtime_work_snapshot(input, "enter", remaining);
        runtime_work_geometry(
            input,
            R::SHARED,
            if matches!(goal, PredictionGoal::Completion { .. }) {
                "completion_enter"
            } else {
                "decision_enter"
            },
            base_depth,
            0,
            1,
        );
        // Row sets a live decision passed through with one surviving
        // alternative; they learn this scan's verdict.
        let mut viability_trail: Vec<ViabilityKey> = Vec::new();
        let outcome: Result<PredictionOutcome, String> = (|| {
            if goal.live_boundary() {
                for _ in 0..8 {
                    sources.charge(remaining)?;
                }
                if verification != CompletionVerification::InputCheckedCompletion {
                    return Err("Live completion boundary requires its input-checked choice".into());
                }
                let seed = live.ok_or("Live completion boundary has no actual caller seed")?;
                let bound = goal
                    .boundary()
                    .ok_or("Live completion boundary disappeared")?;
                sources.charge(remaining)?;
                if !std::ptr::eq(bound.returns, seed.returns) {
                    return Err("Live completion boundary differs from actual return tail".into());
                }
                sources.charge(remaining)?;
                let eof = self
                    .states
                    .get(seed.eof)
                    .ok_or("Dangling live completion EOF")?;
                let [Edge::Token(PredictionSymbol::End, accepted)] = eof.edges.as_slice() else {
                    return Err("Live completion boundary lacks canonical EOF edge".into());
                };
                sources.charge(remaining)?;
                if *accepted != bound.stop
                    || !self.states.get(bound.stop).is_some_and(|state| {
                        state.accepting && !state.rule_stop && state.edges.is_empty()
                    })
                {
                    return Err(
                        "Live completion boundary differs from actual EOF acceptance".into(),
                    );
                }
            }
            // A LIVE decision must not use unselected nested NFA branches as
            // evidence for outer viability. Reuse the canonical completion
            // decision bindings once for this exact actual-root invocation.
            // This is fresh paid scratch, never retained prediction authority.
            // A nested live decision shares the scope its outermost live caller
            // verified for the same root; it is static for the parse.
            let verified_scope;
            let live_completion_scope = if let Some(scope) = live
                .and_then(|seed| seed.scope)
                .filter(|_| goal.live_boundary())
            {
                let seed = live.ok_or("Live nested commitments have no actual caller seed")?;
                sources.charge(remaining)?;
                Some((scope, seed.root_stop))
            } else if goal.live_boundary() {
                let seed = live.ok_or("Live nested commitments have no actual caller seed")?;
                let work = remaining
                    .as_mut()
                    .ok_or("Live nested commitments require finite original work")?;
                let mut root_start = None;
                for &(start, stop) in self.rules.values() {
                    analysis_charge(work)?;
                    analysis_charge(work)?;
                    if stop == seed.root_stop && root_start.replace(start).is_some() {
                        return Err(
                            "Live nested commitments have an ambiguous root endpoint".into()
                        );
                    }
                }
                let root_start = root_start.ok_or("Live nested commitments have no exact root")?;
                analysis_charge(work)?;
                let root_state = self
                    .states
                    .get(seed.root_stop)
                    .ok_or("Live nested commitments have a dangling root")?;
                if !root_state.rule_stop {
                    return Err("Live nested commitments have a non-root endpoint".into());
                }
                let mut exact_eof = false;
                for &follow in &root_state.follow {
                    analysis_charge(work)?;
                    exact_eof |= follow == seed.eof;
                }
                if !exact_eof {
                    return Err(
                        "Live nested commitments have a foreign root EOF continuation".into(),
                    );
                }
                verified_scope = self.verify_completion_scope_with_verification(
                    root_start,
                    seed.root_stop,
                    work,
                    verification,
                )?;
                Some((&verified_scope, seed.root_stop))
            } else {
                None
            };
            if R::SHARED
                && (live.is_none()
                    || (S::ENABLED && !S::CALLER_NEUTRAL)
                    || (!matches!(goal, PredictionGoal::Decision { boundary: None, .. })
                        && !goal.live_boundary()))
            {
                return Err("Shared returns require the exact uncached outer live decision".into());
            }
            if S::PREPARE_ONLY
                && (live.is_some()
                    || base_depth != 0
                    || local.is_some()
                    || !matches!(goal, PredictionGoal::Decision { boundary: None, .. }))
            {
                return Err("Initial preparation requires an exact outer decision".into());
            }
            // Keep the historical cache destruction point inside this outcome
            // closure; the outer parameter becomes empty before any seeding.
            let mut shared_seed = shared_seed.take();
            let mut pending: VecDeque<_> = match goal {
                PredictionGoal::Decision { decision, .. } => decision
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(alternative, &state)| {
                        let returns = match live {
                            Some(seed) => R::seed(seed.returns, &mut shared_seed, remaining)?,
                            None => R::from_owned(goal.initial_returns())?,
                        };
                        runtime_work_geometry(
                            input,
                            R::SHARED,
                            "initial_row",
                            base_depth,
                            returns.len(),
                            returns.len(),
                        );
                        Ok(PredictionConfiguration {
                            state,
                            returns,
                            alternative,
                            visible: true,
                            guarded: false,
                            full_guard: false,
                        })
                    })
                    .collect::<Result<VecDeque<_>, String>>()?,
                PredictionGoal::Completion { start, .. } => {
                    VecDeque::from([PredictionConfiguration {
                        state: start,
                        returns: R::empty(),
                        alternative: 0,
                        visible: true,
                        guarded: false,
                        full_guard: false,
                    }])
                }
            };
            let mut matched_source_token = false;
            let decision_start = input.replay_position();
            let mut initial = None;
            let mut initial_read = false;
            let mut initial_consumed = false;
            let mut first_closure = true;
            // Full guards break accepting ties with an unguarded exit. A
            // qualified caller walk may establish the guard with its own
            // operand path; otherwise the eager operand probe remains in use.
            let mut full_predicate_conflict = false;
            // The latest alternative that reached a bounded (non-live) stop
            // while longer continuations were still alive; see the greedy
            // bounded acceptance below.
            let mut bounded_fallback: Option<usize> = None;
            loop {
                runtime_work_snapshot(input, "outer", remaining);
                runtime_work_geometry(input, R::SHARED, "pending", base_depth, 0, pending.len());
                sources.charge(remaining)?;
                if pending.is_empty() {
                    if S::PREPARE_ONLY {
                        return Ok(PredictionOutcome::InitialPrepared(
                            runtime_source::InitialPreparation::Ineligible,
                        ));
                    }
                    return Ok(goal.no_match());
                }
                // A live decision reads to the end of input. Once one alternative
                // remains, its verdict depends only on these rows at this
                // position, so a row set an earlier decision already carried to
                // the end is not read again.
                if live_completion_scope.is_some()
                    && !first_closure
                    && local.is_none()
                    && let (Some(position), Some(boundary), Some(first)) =
                        (input.replay_position(), goal.boundary(), pending.front())
                    && Some(first.alternative) != goal.decision().and_then(|d| d.exit)
                    && pending
                        .iter()
                        .all(|row| row.alternative == first.alternative)
                {
                    let alternative = first.alternative;
                    let mut rows = Vec::with_capacity(pending.len());
                    for row in &pending {
                        rows.push((
                            row.state,
                            row.returns.materialized(remaining)?.into_owned(),
                            row.visible,
                            row.guarded,
                        ));
                    }
                    rows.sort_unstable();
                    rows.dedup();
                    let key = ViabilityKey {
                        position,
                        stop: boundary.stop,
                        rows,
                    };
                    if let Some(accepted) = input.viability(&key) {
                        return if accepted {
                            goal.selected(alternative)
                        } else {
                            Ok(goal.no_match())
                        };
                    }
                    viability_trail.push(key);
                }
                // Entry First guards may prefetch the current token during
                // THIS closure without consuming its logical frontier. Nested
                // decisions must begin at the pre-closure cursor, then restore
                // the physical cursor holding that still-unconsumed prefetch.
                let logical_frontier = if live_completion_scope.is_some() {
                    sources.charge(remaining)?;
                    Some(input.checkpoint()?)
                } else {
                    None
                };
                if goal.decision().is_some() && !goal.live_boundary() {
                    let alternatives: BTreeSet<_> =
                        pending.iter().map(|row| row.alternative).collect();
                    if alternatives.len() == 1 {
                        let selected = *alternatives
                            .first()
                            .ok_or("Missing parser prediction alternative")?;
                        if S::PREPARE_ONLY {
                            return Ok(PredictionOutcome::InitialPrepared(
                                runtime_source::InitialPreparation::Ineligible,
                            ));
                        }
                        return goal.selected(selected);
                    }
                    if let Some(first) = pending.front() {
                        let last = pending
                            .iter()
                            .map(|row| row.alternative)
                            .max()
                            .ok_or("Missing prediction alternative")?;
                        if pending
                            .iter()
                            .all(|row| row.state == first.state && row.returns == first.returns)
                            && pending
                                .iter()
                                .all(|row| row.alternative == last || row.guarded)
                        {
                            let selected = *alternatives
                                .first()
                                .ok_or("Missing prediction alternative")?;
                            if S::PREPARE_ONLY {
                                return Ok(PredictionOutcome::InitialPrepared(
                                    runtime_source::InitialPreparation::Ineligible,
                                ));
                            }
                            return goal.selected(selected);
                        }
                    }
                }
                runtime_work_snapshot(input, "frontier_decision", remaining);
                let mut frontier = Vec::new();
                let mut accepting = BTreeMap::new();
                let replayed = if S::ENABLED && first_closure {
                    runtime_work_snapshot(input, "source_replay", remaining);
                    sources.replay(&mut frontier, &mut accepting, remaining)?
                } else {
                    false
                };
                runtime_work_snapshot(input, "frontier_decision", remaining);
                // Mirror the drained cold deque. The normal loop also replaces it
                // after matching; this explicit cleanup is not a behavior fix.
                if replayed {
                    pending.clear();
                }
                if S::BORROWED_FRONTIER && replayed {
                    sources.charge(remaining)?;
                    if !matches!(goal, PredictionGoal::Decision { boundary: None, .. }) {
                        return Err(
                            "Borrowed runtime frontier requires an unbounded decision".into()
                        );
                    }
                }
                let mut cacheable = true;
                if !replayed {
                    let mut visited = <R::Visited as Default>::default();
                    while let Some(row) = pending.pop_front() {
                        runtime_work_snapshot(input, "closure_visit", remaining);
                        runtime_work_geometry(
                            input,
                            R::SHARED,
                            "pending",
                            base_depth,
                            row.returns.len(),
                            pending.len().saturating_add(1),
                        );
                        sources.charge(remaining)?;
                        if live.is_some() {
                            row.returns.copy_fee(remaining)?;
                        }
                        runtime_work_geometry(
                            input,
                            R::SHARED,
                            "row",
                            base_depth,
                            row.returns.len(),
                            1,
                        );
                        let inserted =
                            shared_returns::VisitedRows::insert(&mut visited, row.clone());
                        runtime_work_geometry(
                            input,
                            R::SHARED,
                            "copy_visited",
                            base_depth,
                            row.returns.len(),
                            row.returns.len(),
                        );
                        runtime_work_geometry(
                            input,
                            R::SHARED,
                            if inserted {
                                "visited_unique"
                            } else {
                                "visited_duplicate"
                            },
                            base_depth,
                            row.returns.len(),
                            shared_returns::VisitedRows::len(&visited),
                        );
                        if !inserted {
                            runtime_work_snapshot(input, "visited_duplicate", remaining);
                            continue;
                        }
                        runtime_work_snapshot(input, "source_ports", remaining);
                        // Explicit eager pure-graph macrosteps only. No default or
                        // completion path acquires this policy; actual effects stay
                        // below in the SAME canonical runtime worker.
                        if S::PORTS
                            && matches!(goal, PredictionGoal::Decision { boundary: None, .. })
                            && sources.expand_ports(&row, &mut pending, remaining)?
                        {
                            continue;
                        }
                        runtime_work_snapshot(input, "state_goal", remaining);
                        let state = self
                            .states
                            .get(row.state)
                            .ok_or("Dangling parser prediction state")?;
                        if S::PREPARE_ONLY
                            && S::CALLER_NEUTRAL
                            && (state.accepting || state.rule_stop || state.edges.is_empty())
                        {
                            return Ok(PredictionOutcome::InitialPrepared(
                                runtime_source::InitialPreparation::Ineligible,
                            ));
                        }
                        if let Some(stop) = goal.root_stop() {
                            if row.state == stop && row.returns.is_empty() {
                                if matches!(goal, PredictionGoal::Completion { .. }) {
                                    return Ok(PredictionOutcome::Completed(
                                        !matches!(
                                            goal,
                                            PredictionGoal::Completion {
                                                productivity:
                                                    CompletionProductivity::MustConsumeSourceToken,
                                                ..
                                            }
                                        ) || matched_source_token,
                                    ));
                                }
                                accepting
                                    .entry(row.alternative)
                                    .and_modify(|guarded: &mut bool| *guarded &= row.guarded)
                                    .or_insert(row.guarded);
                                continue;
                            }
                            if state.accepting {
                                return Err("Completion escaped its exact rule boundary".into());
                            }
                        }
                        let completion_context =
                            if let PredictionGoal::Completion { scope, stop, .. } = goal {
                                Some((scope, stop))
                            } else {
                                live_completion_scope
                            };
                        if live_completion_scope.is_some()
                            && let Some((scope, _)) = completion_context
                        {
                            let levels = usize::BITS as usize
                                - scope.decisions.len().max(1).leading_zeros() as usize;
                            for _ in 0..1 + 2 * levels {
                                sources.charge(remaining)?;
                            }
                        }
                        if let Some((scope, stop)) = completion_context
                            && let Some(bound) = scope.decisions.get(&row.state)
                        {
                            let materialized;
                            let (boundary, nested_depth, local_boundary) = if let Some(local) =
                                local
                            {
                                sources.charge(remaining)?;
                                if bound.path == local.path {
                                    sources.charge(remaining)?;
                                    if !std::ptr::eq(bound.decision, local.decision)
                                        || row.state != local.entry
                                    {
                                        return Err(
                                        "Runtime rule-local choice differs from its qualified fan-out"
                                            .into(),
                                    );
                                    }
                                    // The local decision starts at its own rule boundary,
                                    // but outstanding real outer Calls still count.
                                    let depth = base_depth
                                        .checked_add(row.returns.len())
                                        .ok_or("Parser prediction call depth exceeded")?;
                                    if depth > self.call_depth() {
                                        return Err("Parser prediction call depth exceeded".into());
                                    }
                                    (
                                        DecisionBoundary {
                                            live: false,
                                            stop: local.stop,
                                            returns: &[],
                                        },
                                        depth,
                                        true,
                                    )
                                } else {
                                    materialized = row.returns.materialized(remaining)?;
                                    (
                                        DecisionBoundary {
                                            live: false,
                                            stop,
                                            returns: materialized.as_ref(),
                                        },
                                        base_depth,
                                        false,
                                    )
                                }
                            } else {
                                materialized = row.returns.materialized(remaining)?;
                                (
                                    DecisionBoundary {
                                        live: false,
                                        stop,
                                        returns: materialized.as_ref(),
                                    },
                                    base_depth,
                                    false,
                                )
                            };
                            // Each NEW live interception makes a real recursive
                            // worker invocation, including across sibling loop
                            // iterations with no grammar Call. Debit one probe
                            // depth slot before child seeding/checkpoint/read;
                            // never infer Rust recursion safety from return depth.
                            // Keep the existing combined128 envelope unchanged.
                            let mut at_depth_cap = false;
                            let nested_depth = if live_completion_scope.is_some() {
                                sources.charge(remaining)?;
                                let depth = nested_depth
                                    .checked_add(1)
                                    .ok_or("Parser prediction invocation depth overflow")?;
                                let combined = depth
                                    .checked_add(boundary.returns.len())
                                    .ok_or("Parser prediction invocation depth overflow")?;
                                // The invocation depth bounds real Rust recursion
                                // and stays at the default; the combined stack
                                // follows the configured call depth.
                                if combined > self.call_depth() {
                                    return Err(DEPTH_EXCEEDED.into());
                                }
                                // At the recursion cap the decision is deferred to
                                // the outermost live caller unless it was already
                                // predicted.
                                at_depth_cap = depth >= DEFAULT_CALL_DEPTH;
                                depth
                            } else {
                                nested_depth
                            };
                            // An actual conditional choice keeps its exact LIVE
                            // root/EOF uniqueness obligation when reached inside
                            // another live decision. Ordinary source priority
                            // uses the existing canonical nested worker unchanged.
                            let nested_live = live_completion_scope.is_some()
                                && self.is_input_checked_live_choice(bound.path, remaining)?;
                            let boundary = if nested_live {
                                let seed =
                                    live.ok_or("Nested live choice lost actual caller seed")?;
                                sources.charge(remaining)?;
                                let eof = self
                                    .states
                                    .get(seed.eof)
                                    .ok_or("Nested live choice has dangling EOF")?;
                                let [Edge::Token(PredictionSymbol::End, accepted)] =
                                    eof.edges.as_slice()
                                else {
                                    return Err(
                                        "Nested live choice has no exact EOF acceptance".into()
                                    );
                                };
                                DecisionBoundary {
                                    live: true,
                                    stop: *accepted,
                                    returns: boundary.returns,
                                }
                            } else {
                                boundary
                            };
                            let nested_seed = if nested_live {
                                let seed = live.ok_or("Nested live choice lost original root")?;
                                Some(live_callers::LiveSeed {
                                    returns: boundary.returns,
                                    root_stop: seed.root_stop,
                                    eof: seed.eof,
                                    scope: live_completion_scope.map(|(scope, _)| scope),
                                    scopes: seed.scopes,
                                })
                            } else {
                                None
                            };
                            if local_boundary {
                                sources.charge(remaining)?;
                            }
                            runtime_work_snapshot(input, "nested_decision", remaining);
                            let checkpoint = input.checkpoint()?;
                            let rewind_frontier =
                                logical_frontier.filter(|position| *position != checkpoint);
                            if rewind_frontier.is_some() {
                                // Prepay both adapter moves BEFORE any cursor
                                // mutation. A failed/denied nested probe still
                                // restores the parent's physical prefetch state.
                                sources.charge(remaining)?;
                                sources.charge(remaining)?;
                            }
                            let nested_goal = PredictionGoal::Decision {
                                decision: bound.decision,
                                recursive_first: bound.recursive_first,
                                boundary: Some(boundary),
                            };
                            // Sibling members with bodies reach the same nested
                            // decision at the same input position under the same
                            // boundary many times; its selection depends only on
                            // that key, so it is predicted once. Errors are not
                            // retained.
                            let memo_key = NestedDecisionKey {
                                decision: std::ptr::from_ref(bound.decision) as usize,
                                start: rewind_frontier.unwrap_or(checkpoint),
                                live: boundary.live,
                                stop: boundary.stop,
                                returns: boundary.returns.to_vec(),
                                nested_live,
                                local: local
                                    .map(|local| std::ptr::from_ref(local.decision) as usize),
                                verification,
                                root: live.map(|seed| (seed.root_stop, seed.eof)),
                            };
                            let selected = if let Some(selected) =
                                input.nested_decision_outcome(&memo_key)
                            {
                                selected
                            } else {
                                if at_depth_cap {
                                    if local.is_none()
                                        && input.defer_nested_decision(DeferredNestedDecision {
                                            key: memo_key,
                                            state: row.state,
                                        })
                                    {
                                        return Err(NESTED_DECISION_DEFERRED.into());
                                    }
                                    return Err(DEPTH_EXCEEDED.into());
                                }
                                let selection = (|| {
                                    if let Some(position) = rewind_frontier {
                                        input.rewind(position)?;
                                    }
                                    if nested_live {
                                        self.run_prediction_with_live_seed(
                                            nested_goal,
                                            input,
                                            remaining,
                                            nested_depth,
                                            verification,
                                            local,
                                            &mut runtime_source::NoInitialSource,
                                            nested_seed.as_ref(),
                                        )
                                    } else {
                                        self.run_prediction_at_depth_with_local_decision(
                                            nested_goal,
                                            input,
                                            remaining,
                                            nested_depth,
                                            verification,
                                            local,
                                        )
                                    }
                                })();
                                runtime_work_snapshot(input, "cleanup", remaining);
                                let selection = if local_boundary || rewind_frontier.is_some() {
                                    // Restoration was prepaid. Keep the semantic failure
                                    // if the explicit adapter's restoration also fails.
                                    match (selection, input.rewind(checkpoint)) {
                                        (Err(original), Err(cleanup)) => {
                                            return Err(if local_boundary {
                                                format!(
                                                    "{original}; local nullable rewind failed: {cleanup}"
                                                )
                                            } else {
                                                format!(
                                                    "{original}; live frontier rewind failed: {cleanup}"
                                                )
                                            });
                                        }
                                        (_, Err(cleanup)) => return Err(cleanup),
                                        (result, Ok(())) => result,
                                    }
                                } else {
                                    input.rewind(checkpoint)?;
                                    selection
                                };
                                // A deferral below records this decision too, so
                                // the whole interrupted chain is resolved deepest
                                // first before the outer prediction restarts.
                                if local.is_none()
                                    && matches!(&selection, Err(error) if error == NESTED_DECISION_DEFERRED)
                                {
                                    input.defer_nested_decision(DeferredNestedDecision {
                                        key: memo_key.clone(),
                                        state: row.state,
                                    });
                                }
                                let selected = match selection? {
                                    PredictionOutcome::Selected(selected) => {
                                        selected.or(bound.decision.exit)
                                    }
                                    PredictionOutcome::Completed(_)
                                    | PredictionOutcome::InitialPrepared(_) => {
                                        return Err("Nested decision returned completion".into());
                                    }
                                };
                                input.record_nested_decision_outcome(memo_key, selected);
                                selected
                            };
                            let Some(selected) = selected else {
                                if live_completion_scope.is_some() {
                                    // Only THIS actual outer owner/context failed;
                                    // other alternatives keep their FIFO position.
                                    continue;
                                }
                                return Ok(PredictionOutcome::Completed(false));
                            };
                            let target = *bound
                                .decision
                                .entries
                                .get(selected)
                                .ok_or("Invalid completion decision alternative")?;
                            pending.push_back(PredictionConfiguration {
                                state: target,
                                ..row
                            });
                            continue;
                        }
                        if state.accepting {
                            accepting
                                .entry(row.alternative)
                                .and_modify(|guarded: &mut bool| *guarded &= row.guarded)
                                .or_insert(row.guarded);
                            runtime_work_geometry(
                                input,
                                R::SHARED,
                                "accepting",
                                base_depth,
                                row.returns.len(),
                                accepting.len(),
                            );
                        }
                        if state.rule_stop {
                            runtime_work_snapshot(
                                input,
                                if row.returns.len() == 0 {
                                    "follow"
                                } else {
                                    "return"
                                },
                                remaining,
                            );
                            if let Some((target, rest)) =
                                row.returns.popped(remaining, live.is_some())?
                            {
                                runtime_work_geometry(
                                    input,
                                    R::SHARED,
                                    "pop",
                                    base_depth,
                                    rest.len(),
                                    1,
                                );
                                runtime_work_geometry(
                                    input,
                                    R::SHARED,
                                    "copy_pop",
                                    base_depth,
                                    rest.len(),
                                    rest.len(),
                                );
                                pending.push_back(PredictionConfiguration {
                                    state: target,
                                    returns: rest,
                                    alternative: row.alternative,
                                    visible: row.visible,
                                    guarded: row.guarded,
                                    full_guard: row.full_guard,
                                });
                            } else {
                                if goal.root_stop().is_some() && !goal.live_boundary() {
                                    return Err(
                                        "Completion requires an exact rule return context".into()
                                    );
                                }
                                if let Some(seed) = live {
                                    if row.state != seed.root_stop {
                                        return Err(
                                            "Live prediction escaped its actual entry return"
                                                .into(),
                                        );
                                    }
                                    sources.charge(remaining)?;
                                    pending.push_back(PredictionConfiguration {
                                        state: seed.eof,
                                        ..row
                                    });
                                    continue;
                                }
                                if state.follow.is_empty() {
                                    return Err("Caller FOLLOW context required".into());
                                }
                                runtime_work_geometry(
                                    input,
                                    R::SHARED,
                                    "follow_fanout",
                                    base_depth,
                                    row.returns.len(),
                                    state.follow.len(),
                                );
                                for &target in &state.follow {
                                    sources.charge(remaining)?;
                                    pending.push_back(PredictionConfiguration {
                                        state: target,
                                        ..row.clone()
                                    });
                                    runtime_work_geometry(
                                        input,
                                        R::SHARED,
                                        "follow_step",
                                        base_depth,
                                        row.returns.len(),
                                        1,
                                    );
                                    runtime_work_geometry(
                                        input,
                                        R::SHARED,
                                        "copy_follow",
                                        base_depth,
                                        row.returns.len(),
                                        row.returns.len(),
                                    );
                                }
                            }
                            continue;
                        }
                        // The visited set already owns its exact full-key copy.
                        // Earlier siblings clone this row; the last edge can
                        // consume the owned original without another clone.
                        let move_single = R::MOVE_SINGLE_EDGE && state.edges.len() == 1;
                        let last_edge = state.edges.len().saturating_sub(1);
                        let mut edge_row = Some(row);
                        for (edge_index, edge) in state.edges.iter().enumerate() {
                            let move_edge =
                                move_single || (R::MOVE_LAST_EDGE && edge_index == last_edge);
                            let row = edge_row
                                .as_ref()
                                .ok_or("Missing runtime prediction edge row")?;
                            runtime_work_snapshot(
                                input,
                                match edge {
                                    Edge::Epsilon(_) | Edge::Action(_) => "epsilon_action",
                                    Edge::Call { .. } => "call",
                                    Edge::First { .. } => "first_edge",
                                    Edge::Full { .. } => "full_edge",
                                    Edge::Token(..) => "token_edge",
                                },
                                remaining,
                            );
                            sources.charge(remaining)?;
                            if !move_edge && live.is_some() {
                                row.returns.copy_fee(remaining)?;
                            }
                            let mut next = if move_edge {
                                edge_row
                                    .take()
                                    .ok_or("Missing owned runtime prediction edge row")?
                            } else {
                                row.clone()
                            };
                            if !move_edge {
                                runtime_work_geometry(
                                    input,
                                    R::SHARED,
                                    "copy_edge",
                                    base_depth,
                                    next.returns.len(),
                                    next.returns.len(),
                                );
                            }
                            if S::PREPARE_ONLY
                                && S::CALLER_NEUTRAL
                                && matches!(
                                    edge,
                                    Edge::Action(_) | Edge::First { .. } | Edge::Full { .. }
                                )
                            {
                                return Ok(PredictionOutcome::InitialPrepared(
                                    runtime_source::InitialPreparation::Ineligible,
                                ));
                            }
                            match edge {
                                Edge::Epsilon(target) => next.state = *target,
                                Edge::Action(target) => {
                                    next.state = *target;
                                    next.visible = false;
                                }
                                Edge::First { path, target } => {
                                    if let PredictionGoal::Completion { scope, .. } = goal {
                                        if scope.guards.get(&next.state)
                                            != Some(&CompletionGuard::First(path.as_str()))
                                        {
                                            return Err(format!(
                                                "Unbound completion guard at {path}"
                                            ));
                                        }
                                        // The enclosing nested decision has already
                                        // committed. Actual body traversal executes no
                                        // second predicate or semantic callback.
                                        next.state = *target;
                                    } else {
                                        let decision = goal.decision().ok_or_else(|| {
                                        format!(
                                            "Completion requires nested commitment support at {path}"
                                        )
                                    })?;
                                        if next.visible
                                            && decision.entries.get(next.alternative)
                                                == Some(&next.state)
                                        {
                                            if S::PREPARE_ONLY {
                                                return Ok(PredictionOutcome::InitialPrepared(
                                                    runtime_source::InitialPreparation::Ineligible,
                                                ));
                                            }
                                            if S::ENABLED {
                                                cacheable = false;
                                            }
                                            if !initial_read {
                                                initial = runtime_work_read(
                                                    input,
                                                    remaining,
                                                    "first_edge",
                                                )?;
                                                initial_read = true;
                                            }
                                            let guard = self
                                                .first_guards
                                                .get(path)
                                                .ok_or("Unbound first-token predicate")?;
                                            if !initial
                                                .as_ref()
                                                .is_some_and(|symbol| guard.contains(symbol))
                                            {
                                                continue;
                                            }
                                            next.guarded = true;
                                        }
                                        next.state = *target;
                                    }
                                }
                                Edge::Full { target, stop } => {
                                    if let PredictionGoal::Completion { scope, .. } = goal {
                                        if scope.guards.get(&next.state)
                                            != Some(&CompletionGuard::Full)
                                        {
                                            return Err("Unbound completion full predicate".into());
                                        }
                                    } else if let Some(decision) = goal.decision()
                                        && next.visible
                                        && decision.entries.get(next.alternative)
                                            == Some(&next.state)
                                    {
                                        if S::PREPARE_ONLY {
                                            return Ok(PredictionOutcome::InitialPrepared(
                                                runtime_source::InitialPreparation::Ineligible,
                                            ));
                                        }
                                        if S::ENABLED {
                                            cacheable = false;
                                        }
                                        if self.runtime_requirement.is_some()
                                            && (goal.boundary().is_some()
                                                || live
                                                    .is_some_and(|seed| !seed.returns.is_empty()))
                                            && decision
                                                .exit
                                                .is_some_and(|exit| exit != next.alternative)
                                        {
                                            // A qualified runtime enter/exit decision walks the guarded
                                            // operand to its exact caller boundary. An accepted
                                            // enter path itself witnesses the full predicate;
                                            // probing only the operand can be ambiguous at a
                                            // shorter valid prefix (a in a.b). Keep both paths
                                            // alive and use the guard only at acceptance.
                                            if remaining.is_none() {
                                                return Err("Full syntactic predicate requires a finite shared prediction work budget".into());
                                            }
                                            next.guarded = true;
                                            next.full_guard = true;
                                            full_predicate_conflict = true;
                                            next.state = *target;
                                            pending.push_back(next);
                                            continue;
                                        }
                                        let start = decision_start.ok_or(
                                            "Full syntactic predicate requires replayable input",
                                        )?;
                                        let checkpoint = input.checkpoint()?;
                                        // Every speculative read uses this same charged input;
                                        // restoration also occurs after a failed or exhausted probe.
                                        if remaining.is_none() {
                                            return Err("Full syntactic predicate requires a finite shared prediction work budget".into());
                                        }
                                        // Reserve cleanup before any speculative cursor change.
                                        // Exhaustion cannot deny restoration or refund this unit.
                                        runtime_work_snapshot(input, "cleanup", remaining);
                                        sources.charge(remaining)?;
                                        runtime_work_snapshot(input, "full_edge", remaining);
                                        let probe = (|| {
                                            sources.charge(remaining)?;
                                            input.rewind(start)?;
                                            let depth = base_depth
                                                .checked_add(next.returns.len())
                                                .and_then(|value| value.checked_add(1))
                                                .ok_or("Parser prediction call depth exceeded")?;
                                            if depth > self.call_depth() {
                                                return Err(
                                                    "Parser prediction call depth exceeded".into(),
                                                );
                                            }
                                            let work = remaining.as_mut().ok_or("Full syntactic predicate requires a finite shared prediction work budget")?;
                                            // The same graph fact, charged the same
                                            // work, when this source verified it before.
                                            let scope = match live.and_then(|seed| seed.scopes) {
                                                Some(scopes) => scopes.verified(
                                                    self,
                                                    (*target, *stop),
                                                    work,
                                                    verification,
                                                    local,
                                                )?,
                                                None => std::rc::Rc::new(
                                                    self.verify_completion_scope_with_local_decision(
                                                        *target,
                                                        *stop,
                                                        work,
                                                        verification,
                                                        local,
                                                    )?,
                                                ),
                                            };
                                            self.run_prediction_at_depth_with_local_decision(
                                                PredictionGoal::Completion {
                                                    start: *target,
                                                    stop: *stop,
                                                    scope: &scope,
                                                    productivity: CompletionProductivity::Any,
                                                },
                                                input,
                                                remaining,
                                                depth,
                                                verification,
                                                local,
                                            )
                                        })();
                                        // Cleanup was prepaid. Preserve the actual probe
                                        // failure when its allowance is exhausted.
                                        runtime_work_snapshot(input, "cleanup", remaining);
                                        input.rewind(checkpoint)?;
                                        match probe? {
                                            PredictionOutcome::Completed(true) => {
                                                next.guarded = true;
                                                next.full_guard = true;
                                            }
                                            PredictionOutcome::Completed(false) => continue,
                                            PredictionOutcome::InitialPrepared(_) => return Err("Unexpected initial preparation outcome in Full predicate".into()),
                                            PredictionOutcome::Selected(_) => {
                                                return Err("Full predicate returned a decision".into());
                                            }
                                        }
                                    }
                                    next.state = *target;
                                }
                                Edge::Call { target, follow } => {
                                    if next.returns.len().saturating_add(base_depth)
                                        >= self.call_depth()
                                    {
                                        return Err("Parser prediction call depth exceeded".into());
                                    }
                                    next.state = *target;
                                    next.returns.push_return(*follow, remaining)?;
                                    if S::PREPARE_ONLY && S::CALLER_NEUTRAL {
                                        sources.prefix_depth(next.returns.len(), remaining)?;
                                    }
                                    runtime_work_geometry(
                                        input,
                                        R::SHARED,
                                        "push",
                                        base_depth,
                                        next.returns.len(),
                                        1,
                                    );
                                }
                                Edge::Token(symbol, target) => {
                                    next.state = *target;
                                    next.visible = false;
                                    let return_len = next.returns.len();
                                    frontier.push((symbol, next));
                                    runtime_work_geometry(
                                        input,
                                        R::SHARED,
                                        "frontier",
                                        base_depth,
                                        return_len,
                                        frontier.len(),
                                    );
                                    continue;
                                }
                            }
                            pending.push_back(next);
                        }
                    }
                }
                if S::ENABLED && first_closure && !replayed && cacheable {
                    runtime_work_snapshot(input, "source_publish", remaining);
                    sources.remember(&frontier, &accepting, remaining)?;
                }
                runtime_work_snapshot(input, "frontier_decision", remaining);
                runtime_work_geometry(input, R::SHARED, "frontier", base_depth, 0, frontier.len());
                runtime_work_geometry(
                    input,
                    R::SHARED,
                    "accepting",
                    base_depth,
                    0,
                    accepting.len(),
                );
                if S::PREPARE_ONLY {
                    return Ok(PredictionOutcome::InitialPrepared(if cacheable {
                        runtime_source::InitialPreparation::Prepared
                    } else {
                        runtime_source::InitialPreparation::Ineligible
                    }));
                }
                first_closure = false;
                if matches!(goal, PredictionGoal::Completion { .. }) && frontier.is_empty() {
                    return Ok(PredictionOutcome::Completed(false));
                }
                if accepting.len() > 1 {
                    let last = *accepting
                        .last_key_value()
                        .ok_or("Missing accepting alternative")?
                        .0;
                    // A full syntactic predicate also resolves an accepted
                    // guarded/fallback tie at an exact live caller boundary.
                    // Keep ordinary live ambiguity and early commitment unchanged.
                    if (!goal.live_boundary() || full_predicate_conflict)
                        && accepting
                            .iter()
                            .all(|(alternative, guarded)| *alternative == last || *guarded)
                    {
                        let selected = *accepting
                            .first_key_value()
                            .ok_or("Missing accepting alternative")?
                            .0;
                        if S::PREPARE_ONLY {
                            return Ok(PredictionOutcome::InitialPrepared(
                                runtime_source::InitialPreparation::Ineligible,
                            ));
                        }
                        return goal.selected(selected);
                    }
                    return Err("Ambiguous parser prediction requires predicate resolution".into());
                }
                if goal.boundary().is_some() && frontier.is_empty() {
                    return match accepting
                        .first_key_value()
                        .map(|(alternative, _)| *alternative)
                        .or(bounded_fallback)
                    {
                        Some(selected) => goal.selected(selected),
                        None => Ok(goal.no_match()),
                    };
                }
                let symbol = if !initial_consumed {
                    initial_consumed = true;
                    if !initial_read {
                        initial = runtime_work_read(input, remaining, "frontier_decision")?;
                        initial_read = true;
                    }
                    initial.clone()
                } else {
                    runtime_work_read(input, remaining, "frontier_decision")?
                };
                let next = if S::BORROWED_FRONTIER && replayed {
                    if S::CALLER_NEUTRAL {
                        sources.match_live_borrowed(
                            &symbol,
                            live.ok_or("Caller-neutral replay requires actual live returns")?,
                            &mut shared_seed,
                            remaining,
                        )?
                    } else {
                        sources.match_borrowed(&symbol, remaining)?
                    }
                } else {
                    runtime_source::match_frontier(frontier, &symbol, remaining, |row, _| Ok(row))?
                };
                if next.is_empty() {
                    if matches!(goal, PredictionGoal::Completion { .. }) {
                        return Ok(PredictionOutcome::Completed(false));
                    }
                    return match accepting
                        .first_key_value()
                        .map(|(alternative, _)| *alternative)
                        .or(bounded_fallback)
                    {
                        Some(selected) => goal.selected(selected),
                        None => Ok(goal.no_match()),
                    };
                }
                if matches!(
                    goal,
                    PredictionGoal::Completion {
                        productivity: CompletionProductivity::MustConsumeSourceToken,
                        ..
                    }
                ) && symbol
                    .as_ref()
                    .is_some_and(|value| *value != PredictionSymbol::End)
                {
                    // Only this invocation's successful actual token frontier counts.
                    // Nested speculative decisions/probes rewind before this traversal.
                    matched_source_token = true;
                }
                // At an exact operand boundary a repetition exit is a zero-width
                // fallback. A viable enter prefix wins; no unrelated caller suffix
                // is read merely to supply FOLLOW for that exit.
                if goal.boundary().is_some()
                    && let Some(exit) = goal.decision().and_then(|decision| decision.exit)
                {
                    accepting.remove(&exit);
                }
                if !accepting.is_empty() {
                    // Both a predicated alternative and another accept: the
                    // predicate resolves the conflict in favour of its own.
                    if goal.recursive_first()
                        && full_predicate_conflict
                        && let Some((&selected, _)) =
                            accepting.iter().find(|(_, guarded)| **guarded)
                    {
                        return goal.selected(selected);
                    }
                    // Greedy bounded acceptance (docs/library-probe.md,
                    // "greedy bounded acceptance"): an alternative
                    // that reached the bounded caller stop while others
                    // still match the next token is only a fallback. The
                    // stop is where this prediction's scope ends (a nested
                    // decision in a syntactic-predicate probe ends at the
                    // predicate), not where the real parse must end, so the
                    // longer match is read on: a single surviving
                    // alternative is selected at the top of the loop, and
                    // if every continuation dies the latest fallback (its
                    // lowest alternative, ordered choice) is selected.
                    if goal.decision().is_some()
                        && !goal.live_boundary()
                        && goal.boundary().is_some()
                    {
                        bounded_fallback = accepting
                            .first_key_value()
                            .map(|(alternative, _)| *alternative);
                    } else {
                        return Err("Ambiguous accepting parser continuation".into());
                    }
                }
                if goal.recursive_first() {
                    let alternatives: BTreeSet<_> =
                        next.iter().map(|row| row.alternative).collect();
                    let last = *alternatives
                        .last()
                        .ok_or("Missing recursive prediction alternative")?;
                    // A full syntactic predicate (`( => Rule b )?` guards only
                    // `Rule`) resolves a conflict; it does not cut off a last
                    // alternative that is still viable on its own. While
                    // unguarded rows of the last alternative remain, keep
                    // reading: they die, the rows converge (the same-state rule
                    // above), or both accept (the predicated alternative wins).
                    // First-token predicates keep their recursive-entry commit.
                    let full_conflict = next.iter().any(|row| row.full_guard)
                        && next
                            .iter()
                            .any(|row| row.alternative == last && !row.guarded);
                    full_predicate_conflict |= full_conflict;
                    if !goal.live_boundary()
                        && next
                            .iter()
                            .all(|row| row.alternative == last || row.guarded)
                        && !full_conflict
                    {
                        let selected = *alternatives
                            .first()
                            .ok_or("Missing recursive prediction alternative")?;
                        if S::PREPARE_ONLY {
                            return Ok(PredictionOutcome::InitialPrepared(
                                runtime_source::InitialPreparation::Ineligible,
                            ));
                        }
                        return goal.selected(selected);
                    }
                    // While a full-predicate conflict is open, read on; once its
                    // predicated side died, the next step commits to the rest.
                    let reading_on =
                        full_conflict || (full_predicate_conflict && alternatives.len() == 1);
                    if !reading_on
                        && (verification != CompletionVerification::InputCheckedCompletion
                            || goal.boundary().is_none())
                    {
                        return Err(
                            "Recursive prediction requires visible predicate resolution".into()
                        );
                    }
                    if remaining.is_none() {
                        return Err("Input-checked nested commitment requires a finite shared prediction work budget".into());
                    }
                    // No commitment was proved. Continue THIS canonical frontier
                    // with its exact caller returns, stop, guards and original
                    // work; uniqueness/ambiguity must be decided on actual input.
                }
                pending = next.into_iter().collect();
            }
        })();
        if !viability_trail.is_empty()
            && let Ok(PredictionOutcome::Selected(selected)) = &outcome
        {
            input.record_viability(viability_trail, selected.is_some());
        }
        runtime_work_snapshot(input, "exit", remaining);
        outcome
    }
}
#[derive(Clone, Copy)]
enum PredictionGoal<'a> {
    Decision {
        decision: &'a Decision,
        recursive_first: bool,
        boundary: Option<DecisionBoundary<'a>>,
    },
    Completion {
        start: usize,
        stop: usize,
        scope: &'a CompletionScope<'a>,
        productivity: CompletionProductivity,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum CompletionProductivity {
    Any,
    MustConsumeSourceToken,
}

#[derive(Clone, Copy)]
struct DecisionBoundary<'a> {
    live: bool,
    stop: usize,
    returns: &'a [usize],
}
#[derive(Clone, Copy)]
struct RuleLocalCompletionDecision<'a> {
    path: &'a str,
    decision: &'a Decision,
    entry: usize,
    stop: usize,
}
#[derive(Clone, Copy)]
struct BoundCompletionDecision<'a> {
    path: &'a str,
    decision: &'a Decision,
    recursive_first: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletionGuard<'a> {
    First(&'a str),
    Full,
}
struct CompletionScope<'a> {
    decisions: BTreeMap<usize, BoundCompletionDecision<'a>>,
    guards: BTreeMap<usize, CompletionGuard<'a>>,
}
#[derive(Debug)]
enum PredictionOutcome {
    Selected(Option<usize>),
    Completed(bool),
    InitialPrepared(runtime_source::InitialPreparation),
}
impl<'a> PredictionGoal<'a> {
    fn live_boundary(self) -> bool {
        self.boundary().is_some_and(|boundary| boundary.live)
    }
    fn boundary(self) -> Option<DecisionBoundary<'a>> {
        match self {
            Self::Decision { boundary, .. } => boundary,
            Self::Completion { .. } => None,
        }
    }
    fn root_stop(self) -> Option<usize> {
        match self {
            Self::Decision { boundary, .. } => boundary.map(|boundary| boundary.stop),
            Self::Completion { stop, .. } => Some(stop),
        }
    }
    fn initial_returns(self) -> Vec<usize> {
        match self.boundary() {
            Some(boundary) => boundary.returns.to_vec(),
            None => Vec::new(),
        }
    }
    fn decision(self) -> Option<&'a Decision> {
        match self {
            Self::Decision { decision, .. } => Some(decision),
            Self::Completion { .. } => None,
        }
    }
    fn recursive_first(self) -> bool {
        matches!(
            self,
            Self::Decision {
                recursive_first: true,
                ..
            }
        )
    }
    fn no_match(self) -> PredictionOutcome {
        match self {
            Self::Decision { .. } => PredictionOutcome::Selected(None),
            Self::Completion { .. } => PredictionOutcome::Completed(false),
        }
    }
    fn selected(self, selected: usize) -> Result<PredictionOutcome, String> {
        match self {
            Self::Decision { decision, .. } => Ok(PredictionOutcome::Selected(
                (Some(selected) != decision.exit).then_some(selected),
            )),
            Self::Completion { .. } => Err("Completion cannot select a decision branch".into()),
        }
    }
}

/// Passive geometry of an operation already performed by the canonical worker.
/// Counts are diagnostics, not semantic debit, allocator bytes or comparison CPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeWorkGeometry {
    pub kind: &'static str,
    pub shared_storage: bool,
    pub base_depth: usize,
    pub return_depth: usize,
    pub amount: usize,
}

/// One nested decision prediction: the decision, where its input starts and
/// the exact boundary it is resolved against. Inputs that replay a fixed token
/// sequence may retain the selection for this key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NestedDecisionKey {
    decision: usize,
    start: usize,
    live: bool,
    stop: usize,
    returns: Vec<usize>,
    nested_live: bool,
    local: Option<usize>,
    verification: CompletionVerification,
    root: Option<(usize, usize)>,
}

/// The rows of a live decision with one surviving alternative at an input
/// position, against the decision's end-of-input stop. Whether they reach that
/// stop does not depend on which decision carries them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViabilityKey {
    position: usize,
    stop: usize,
    rows: Vec<(usize, Vec<usize>, bool, bool)>,
}

/// A nested decision that reached the recursion cap, resolved later from the
/// outermost live caller.
#[derive(Debug)]
pub struct DeferredNestedDecision {
    key: NestedDecisionKey,
    state: usize,
}

const DEPTH_EXCEEDED: &str =
    "Parser prediction invocation depth exceeded during live nested commitment";
const NESTED_DECISION_DEFERRED: &str = "Parser prediction nested decision deferred";

pub trait PredictionInput {
    /// Explicit passive runtime-debit diagnostics only. Existing adapters retain
    /// the literal false default; no work, input or predicate callback is added.
    /// A method keeps this trait dyn-compatible. Implementations opting in own
    /// bounded capture and must not mutate the work pool or reenter prediction.
    #[inline(always)]
    fn runtime_work_diagnostics(&self) -> bool {
        false
    }
    /// One chronological scalar sample from the SAME runtime worker. Static
    /// phase labels are diagnostics, not source/model/recognition authority.
    /// Callback CPU and export storage are outside semantic work accounting.
    #[inline(always)]
    fn runtime_work_snapshot(&mut self, _phase: &'static str, _remaining: Option<usize>) {}
    /// Same passive opt-in as phase samples. No input/work/model authority escapes.
    #[inline(always)]
    fn runtime_work_geometry(&mut self, _event: RuntimeWorkGeometry) {}

    /// Return the actual retained input position without reading or allocating.
    /// Forward-only adapters keep the established default and cannot probe `=>`.
    fn replay_position(&self) -> Option<usize> {
        None
    }
    /// The adapter's lookahead bookkeeping (for example the furthest position
    /// read), so a runtime prediction memo can probe its lookahead and restore
    /// it before an uncached prediction. Adapters without such bookkeeping
    /// keep `None`, which disables the memo for them.
    fn lookahead_mark(&self) -> Option<usize> {
        None
    }
    /// Restore the bookkeeping returned by `lookahead_mark`.
    fn restore_lookahead(&mut self, _mark: usize) {}
    /// A selection already predicted for this nested decision over the same
    /// retained input. Inputs without a fixed token sequence keep `None`.
    fn nested_decision_outcome(&self, _key: &NestedDecisionKey) -> Option<Option<usize>> {
        None
    }
    fn record_nested_decision_outcome(
        &mut self,
        _key: NestedDecisionKey,
        _selected: Option<usize>,
    ) {
    }
    /// Starts a live prediction that resolves deferred nested decisions; inputs
    /// that cannot retain them return false and keep the depth error.
    /// Whether these rows were already carried to the end of the same input.
    fn viability(&self, _key: &ViabilityKey) -> Option<bool> {
        None
    }
    fn record_viability(&mut self, _keys: Vec<ViabilityKey>, _accepted: bool) {}
    fn begin_nested_deferral(&mut self) -> bool {
        false
    }
    fn end_nested_deferral(&mut self) {}
    fn defer_nested_decision(&mut self, _deferred: DeferredNestedDecision) -> bool {
        false
    }
    /// The interrupted chain of deferred nested decisions, deepest first.
    fn take_deferred_nested_decisions(&mut self) -> Vec<DeferredNestedDecision> {
        Vec::new()
    }
    fn read(&mut self, remaining: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String>;
    fn checkpoint(&self) -> Result<usize, String>;
    fn rewind(&mut self, checkpoint: usize) -> Result<(), String>;
}
// The established host callback remains forward-only and has no replay storage,
// new read charge or counter. Legacy Decision mode never asks it to rewind.
struct ForwardInput<F>(F);
impl<F> PredictionInput for ForwardInput<F>
where
    F: FnMut(&mut Option<usize>) -> Result<Option<PredictionSymbol>, String>,
{
    fn read(&mut self, remaining: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
        (self.0)(remaining)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        Err("Prediction input is not replayable".into())
    }
    fn rewind(&mut self, _checkpoint: usize) -> Result<(), String> {
        Err("Prediction input is not replayable".into())
    }
}

/// Borrowed immutable syntax tokens, not a second tokenizer or model parser.
/// Core allocates no token tape; the caller owns the already classified input.
/// Replayed reads use the same work stream and never refund failed lookahead.
struct ReplayTokens<'a> {
    symbols: &'a [PredictionSymbol],
    position: usize,
}
impl<'a> ReplayTokens<'a> {
    fn new(symbols: &'a [PredictionSymbol]) -> Self {
        Self {
            symbols,
            position: 0,
        }
    }
}
impl PredictionInput for ReplayTokens<'_> {
    fn replay_position(&self) -> Option<usize> {
        Some(self.position)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        Ok(self.position)
    }
    fn rewind(&mut self, checkpoint: usize) -> Result<(), String> {
        if checkpoint > self.symbols.len() {
            return Err("Prediction input checkpoint escapes its retained tokens".into());
        }
        self.position = checkpoint;
        Ok(())
    }
    fn read(&mut self, remaining: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
        charge(remaining)?;
        let result = self.symbols.get(self.position).cloned();
        if self.position < self.symbols.len() {
            self.position += 1;
        }
        Ok(result)
    }
}

fn observe_parser_phase<T, E>(
    result: Result<T, E>,
    grammar: &Grammar,
    context: Option<super::AnalysisObservation<'_>>,
    phase: super::admission_observation::Phase,
    (owner, decision): (Option<&str>, Option<&str>),
    (work, states): (Option<usize>, Option<usize>),
) -> Result<T, E> {
    let fresh = result.is_err()
        && context.is_some_and(|context| context.observation.first_failure().is_none());
    let result = super::observe_phase(result, grammar, context, phase, owner, decision);
    if fresh && let Some(context) = context {
        context.observation.parser_measurements(phase, work, states);
    }
    result
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PredictionConfiguration<R> {
    state: usize,
    returns: R,
    alternative: usize,
    visible: bool,
    guarded: bool,
    /// Guarded by a full syntactic predicate (`=> Rule`), which an LL(*)
    /// parser evaluates only to resolve a conflict lookahead cannot.
    full_guard: bool,
}
type Configuration = PredictionConfiguration<Vec<usize>>;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AnalysisConfiguration<C = analysis_calls::VecCalls> {
    state: usize,
    alternative: usize,
    calls: C,
}
// V73 reserves the same row-processing unit before retaining a new exact key.
// The FIFO therefore visits the same first occurrences as legacy pop/dedup.
// Early returns retain the cost of reserved, unprocessed rows; no refunds.
fn schedule_analysis<C: analysis_calls::Calls>(
    row: AnalysisConfiguration<C>,
    closure: &mut VecDeque<AnalysisConfiguration<C>>,
    scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
    remaining: &mut usize,
    (unique, site, measured): (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
) -> Result<(), String> {
    schedule_analysis_with_queue(row, closure, scheduled, remaining, (unique, site, measured))
}

fn schedule_analysis_with_queue<
    'graph,
    C: analysis_calls::Calls,
    Q: adjacent::ClosureQueue<'graph, C>,
>(
    row: AnalysisConfiguration<C>,
    closure: &mut Q,
    scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
    remaining: &mut usize,
    (unique, site, measured): (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
) -> Result<(), String> {
    schedule_analysis_with_queue_observed(
        row,
        closure,
        scheduled,
        remaining,
        (unique, site, measured),
        None,
    )
}

fn schedule_analysis_with_queue_observed<
    'graph,
    C: analysis_calls::Calls,
    Q: adjacent::ClosureQueue<'graph, C>,
>(
    row: AnalysisConfiguration<C>,
    closure: &mut Q,
    scheduled: &mut BTreeSet<AnalysisConfiguration<C>>,
    remaining: &mut usize,
    (unique, site, measured): (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
    exact: Option<&AdjacentWorkObservation>,
) -> Result<(), String> {
    schedule_analysis_with_keys_observed(
        row,
        closure,
        scheduled,
        remaining,
        (unique, site, measured),
        exact,
    )
}

fn schedule_analysis_with_keys_observed<
    'graph,
    C: analysis_calls::Calls,
    Q: adjacent::ClosureQueue<'graph, C>,
    K: frontier::ClosureKeys<C>,
>(
    row: AnalysisConfiguration<C>,
    closure: &mut Q,
    scheduled: &mut K,
    remaining: &mut usize,
    (unique, site, measured): (bool, ScheduleSite, Option<&RecursiveAnalysisCounters>),
    exact: Option<&AdjacentWorkObservation>,
) -> Result<(), String> {
    if let Some(work) = exact {
        work.key_context(
            row.state,
            row.alternative,
            row.calls.len(),
            closure.observed_shape(),
        );
    }
    if unique {
        if scheduled.contains_observed(&row, remaining)? {
            if let Some(work) = exact {
                work.key(site, ScheduleEvent::Duplicate);
            }
            if let Some(work) = measured {
                work.scheduling(site, ScheduleEvent::Duplicate);
            }
            return Ok(());
        }
        if let Some(work) = exact {
            work.key(site, ScheduleEvent::Absent);
        }
        if let Some(work) = measured {
            work.scheduling(site, ScheduleEvent::Absent);
        }
        let category = match site {
            ScheduleSite::Seed => AdjacentCharge::KeySeed,
            ScheduleSite::Return => AdjacentCharge::KeyReturn,
            ScheduleSite::Follow => AdjacentCharge::KeyFollow,
            ScheduleSite::Successor => AdjacentCharge::KeySuccessor,
        };
        observed_recursive_charge(
            remaining,
            Work::ClosureReservation,
            measured,
            exact,
            category,
        )?;
        closure.reserve_placement_observed(&row, remaining, exact)?;
        return closure.publish_reserved(row, scheduled, site, measured, exact);
    }
    closure.push_reserved(row)?;
    Ok(())
}

#[cfg(test)]
mod closure_tests;

#[cfg(test)]
#[path = "parser_prediction/shared_analysis_call_tests.rs"]
mod shared_analysis_call_tests;

fn recursive_charge(
    remaining: &mut usize,
    category: Work,
    measured: Option<&RecursiveAnalysisCounters>,
) -> Result<(), String> {
    let result = analysis_charge(remaining);
    if let Some(work) = measured {
        work.charge(category, result.is_ok());
    }
    result
}
fn observed_recursive_charge(
    remaining: &mut usize,
    category: Work,
    measured: Option<&RecursiveAnalysisCounters>,
    exact: Option<&AdjacentWorkObservation>,
    actual: AdjacentCharge,
) -> Result<(), String> {
    let result = recursive_charge(remaining, category, measured);
    if let Some(work) = exact {
        work.charge(actual, result.is_ok());
    }
    result
}
fn observed_analysis_charge(
    remaining: &mut usize,
    exact: Option<&AdjacentWorkObservation>,
    category: AdjacentCharge,
) -> Result<(), String> {
    let result = analysis_charge(remaining);
    if let Some(work) = exact {
        work.charge(category, result.is_ok());
    }
    result
}
fn observed_edge(edge: &Edge) -> ObservedEdge {
    match edge {
        Edge::Epsilon(_) => ObservedEdge::Epsilon,
        Edge::Action(_) => ObservedEdge::Action,
        Edge::First { .. } => ObservedEdge::First,
        Edge::Full { .. } => ObservedEdge::Full,
        Edge::Token(..) => ObservedEdge::Token,
        Edge::Call { .. } => ObservedEdge::Call,
    }
}
fn measured_condition(
    value: bool,
    repeated: bool,
    measured: Option<&RecursiveAnalysisCounters>,
) -> bool {
    if value && let Some(work) = measured {
        work.frontier_skip(repeated);
    }
    value
}
fn analysis_charge(remaining: &mut usize) -> Result<(), String> {
    if *remaining == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *remaining -= 1;
    Ok(())
}
fn charge(remaining: &mut Option<usize>) -> Result<(), String> {
    if let Some(value) = remaining {
        if *value == 0 {
            return Err("Execution step budget exceeded during parser prediction".into());
        }
        *value -= 1;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn collect_guards_with_facts<'g>(
    expr: &'g Expr,
    grammar: &'g Grammar,
    path: &str,
    unresolved: &mut BTreeSet<String>,
    guards: &mut BTreeMap<String, BTreeSet<PredictionSymbol>>,
    budget: &mut usize,
    (depth, recursive): (usize, bool),
    prepared: Option<(
        &mut guard_facts::GuardFacts<'g>,
        &mut RuntimeGuardedAdmission,
    )>,
) -> Result<(), String> {
    collect_guards_with_facts_and_budget(
        expr,
        grammar,
        path,
        unresolved,
        guards,
        budget,
        (
            depth,
            recursive,
            super::GrammarAnalysisWorkBudget::LEGACY_10K,
        ),
        prepared,
    )
}

fn collect_guards(
    expr: &Expr,
    grammar: &Grammar,
    path: &str,
    unresolved: &mut BTreeSet<String>,
    guards: &mut BTreeMap<String, BTreeSet<PredictionSymbol>>,
    budget: &mut usize,
    state: (usize, bool, super::GrammarAnalysisWorkBudget),
) -> Result<(), String> {
    collect_guards_with_facts_and_budget(
        expr, grammar, path, unresolved, guards, budget, state, None,
    )
}

#[allow(clippy::too_many_arguments)] // Only one compiler-local borrowed output sibling.
fn collect_guards_with_facts_and_budget<'g>(
    expr: &'g Expr,
    grammar: &'g Grammar,
    path: &str,
    unresolved: &mut BTreeSet<String>,
    guards: &mut BTreeMap<String, BTreeSet<PredictionSymbol>>,
    budget: &mut usize,
    (depth, recursive, work_budget): (usize, bool, super::GrammarAnalysisWorkBudget),
    mut prepared: Option<(
        &mut guard_facts::GuardFacts<'g>,
        &mut RuntimeGuardedAdmission,
    )>,
) -> Result<(), String> {
    if depth > 64 || *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    if unresolved.remove(path) {
        if !recursive {
            ensure_predicate_calls_acyclic(expr, grammar, &mut BTreeSet::new(), budget)?;
        }
        // FIRST is independent of the occurrence cardinality; nullability
        // never substitutes for a matching first token.
        let (first, nullable) = super::first_with_work_budget(expr, grammar, 0, work_budget)?;
        if let Some((prepared, admission)) = prepared.as_mut() {
            prepared.record(expr, nullable, admission, budget)?;
        }
        let mut symbols = BTreeSet::new();
        for symbol in first {
            if let Some(keyword) = symbol.strip_prefix("literal:") {
                symbols.insert(PredictionSymbol::Keyword(keyword.into()));
            } else if let Some(terminal) = symbol.strip_prefix("terminal:") {
                symbols.insert(PredictionSymbol::Terminal(terminal.into()));
            } else {
                return Err("Unsupported first-token prediction symbol".into());
            }
        }
        if symbols.is_empty() {
            return Err("Empty first-token predicate is unsupported".into());
        }
        guards.insert(path.into(), symbols);
    }
    match &expr.kind {
        Kind::Sequence(items) | Kind::Alternative(items) => {
            let prefix = if matches!(expr.kind, Kind::Sequence(_)) {
                's'
            } else {
                'a'
            };
            for (index, item) in items.iter().enumerate() {
                collect_guards_with_facts_and_budget(
                    item,
                    grammar,
                    &format!("{path}/{prefix}{index}"),
                    unresolved,
                    guards,
                    budget,
                    (depth + 1, recursive, work_budget),
                    prepared
                        .as_mut()
                        .map(|(facts, admission)| (&mut **facts, &mut **admission)),
                )?;
            }
        }
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => collect_guards_with_facts_and_budget(
            value,
            grammar,
            &format!("{path}/value"),
            unresolved,
            guards,
            budget,
            (depth + 1, recursive, work_budget),
            prepared
                .as_mut()
                .map(|(facts, admission)| (&mut **facts, &mut **admission)),
        )?,
        _ => {}
    }
    Ok(())
}
fn ensure_predicate_calls_acyclic(
    expr: &Expr,
    grammar: &Grammar,
    active: &mut BTreeSet<String>,
    budget: &mut usize,
) -> Result<(), String> {
    if active.len() >= 64 {
        return Err("Grammar prediction budget exceeded".into());
    }
    let mut calls = BTreeSet::new();
    super::follow::calls(expr, &mut calls, budget)?;
    for call in calls {
        if let Some(rule) = grammar.rules.get(&call) {
            if !active.insert(call.clone()) {
                return Err("Recursive first-token predicate decision requires additional prediction support".into());
            }
            ensure_predicate_calls_acyclic(&rule.body, grammar, active, budget)?;
            active.remove(&call);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Profile;
    use crate::grammar::Rule;
    mod selected_static_tests {
        use super::*;
        use crate::grammar::admission_observation::PredictionObservation;
        use crate::grammar::admission_observation::RecursiveCounterSnapshot;
        use crate::grammar::admission_observation::RunStage;
        use crate::grammar::admission_observation::SelectedStaticSnapshot;
        use intersect_model::definitions::bundle::DefinitionSource;
        use intersect_model::definitions::bundle::SourceKind;
        use intersect_model::definitions::bundle::SourceSpan;

        // Manufactured canonical AST fixture, not original grammar admission.
        // Both entry alternatives need the nullable wrapper and real child.
        fn fixture() -> (Grammar, DefinitionSource) {
            let mut wrapper = call("Leaf");
            wrapper.cardinality = '?';
            let mut grammar = grammar(
                choice(vec![
                    sequence(vec![call("Wrapper"), keyword("left")]),
                    sequence(vec![
                        expr(Kind::SyntacticPredicate(Box::new(call("Wrapper")))),
                        keyword("right"),
                    ]),
                ]),
                vec![
                    ("Wrapper", wrapper),
                    ("Leaf", choice(vec![keyword("public"), keyword("private")])),
                ],
            );
            grammar.profile = Profile::NativeV80;
            let source = DefinitionSource::new(
                "memory:selected-static.xtext",
                SourceKind::Xtext,
                "Entry: Wrapper 'left' | =>Wrapper 'right'; Leaf: 'public' | 'private'; Wrapper: Leaf?;",
            );
            for name in grammar.rules.keys() {
                grammar.declaration_sources.insert(
                    name.clone(),
                    SourceSpan {
                        source: source.id.clone(),
                        start: 0,
                        end: source.text.len(),
                    },
                );
            }
            (grammar, source)
        }
        fn compile(
            grammar: &Grammar,
            source: &DefinitionSource,
            phases: &PredictionObservation,
            work: &RecursiveAnalysisCounters,
        ) -> Result<ParserPrediction, String> {
            ParserPrediction::compile_with_recursive_frontier_order_pool(
                grammar,
                &BTreeSet::new(),
                true,
                Some(super::super::super::AnalysisObservation {
                    observation: phases,
                    recursive_work: Some(work),
                    adjacent_work: None,
                    entry: &source.id,
                    stage: RunStage::Executable,
                }),
                None,
                RecursiveFrontierOrder::Historical,
                None,
            )
        }
        fn selected() -> RecursiveAnalysisCounters {
            RecursiveAnalysisCounters::with_selected_static("rule/Entry/choice", "Wrapper", "Leaf")
                .unwrap()
        }
        #[test]
        fn selected_static_observation_preserves_whole_program_and_old_reports() {
            let (grammar, source) = fixture();
            let plain = RecursiveAnalysisCounters::new();
            let measured = selected();
            let off = PredictionObservation::new();
            let on = PredictionObservation::new();
            let a = compile(&grammar, &source, &off, &plain);
            let b = compile(&grammar, &source, &on, &measured);
            println!(
                "SELECTED_STATIC_WHOLE_PARITY {:?}",
                measured.selected_static_snapshot()
            );
            assert_eq!(
                serde_json::to_value(&a).unwrap(),
                serde_json::to_value(&b).unwrap()
            );
            assert!(a.is_ok(), "real canonical fixture must compile: {a:?}");
            assert_eq!(plain.snapshot(), measured.snapshot());
            assert_eq!(off.coverage(), on.coverage());
            assert_eq!(off.first_failure(), on.first_failure());
            assert_eq!(off.first_reuse_work(), on.first_reuse_work());
            let snapshot = measured.selected_static_snapshot().unwrap();
            assert!(snapshot.finished && !snapshot.returned_error);
            assert_eq!(
                snapshot.source.unwrap().sha256.as_str().unwrap(),
                source.id.sha256
            );
            assert_eq!(
                snapshot.entry_remaining.unwrap() - snapshot.exit_remaining.unwrap(),
                snapshot.decision_work.unwrap()
            );
            assert!(snapshot.wrapper_call_attempts >= 2);
            assert!(snapshot.wrapper_uncached_entries >= 2);
            assert!(snapshot.wrapper_rows > 0 && snapshot.child_build_lookups > 0);
            assert!(snapshot.child_ready_lookups > 0 && snapshot.child_replays > 0);
            assert_eq!(
                snapshot.residual_wrapper_upper_bound,
                snapshot
                    .wrapper_row_work
                    .checked_sub(snapshot.required_call_work)
                    .and_then(|n| n.checked_sub(snapshot.required_return_work))
                    .and_then(|n| n.checked_sub(snapshot.child_template_work))
            );
            assert!(!snapshot.overflowed && !snapshot.inconsistent && !snapshot.unsupported_route);
            measured.reset();
            assert_eq!(
                measured.snapshot(),
                RecursiveAnalysisCounters::new().snapshot()
            );
            assert!(!measured.selected_static_snapshot().unwrap().finished);
        }

        struct Run {
            result: Result<bool, String>,
            remaining: usize,
            witnesses: BTreeMap<(usize, usize), bool>,
            old: RecursiveCounterSnapshot,
            selected: Option<SelectedStaticSnapshot>,
        }
        fn run(
            program: &ParserPrediction,
            decision: &Decision,
            grammar: &Grammar,
            source: &DefinitionSource,
            ids: SelectedStaticSnapshot,
            grant: usize,
            observe: bool,
        ) -> Run {
            let work = if observe {
                selected()
            } else {
                RecursiveAnalysisCounters::new()
            };
            let mut remaining = grant;
            let mut candidates = CallFreeCandidates::default();
            // The actual fixture leaf is call-free; retain via the same paid worker.
            let (start, stop) = program.rules["Leaf"];
            let prepared = candidates.record_rule(start, stop, false, &mut remaining);
            let mut witnesses = BTreeMap::new();
            let result = prepared.and_then(|()| {
                work.selected_static_prepare(true);
                work.selected_static_rule("Wrapper", ids.wrapper_states.unwrap());
                work.selected_static_rule("Leaf", ids.child_states.unwrap());
                work.begin_program(RunStage::Executable, remaining);
                work.begin_decision(grammar, &source.id, "rule/Entry/choice");
                work.selected_static_begin("rule/Entry/choice", remaining);
                let mut templates = candidates.freeze(&program.states);
                let result = program.multiple_recursive_alternatives_with_frontiers_pool::<
                    adjacent::GeometricAdjacentQueue<'_>, frontier::AdoptFrontiers, true>(
                    decision, &mut remaining, true, &mut witnesses, true,
                    (true, Some(&work), Some(&mut templates), None), None);
                work.selected_static_end(remaining, result.is_err());
                work.end_decision(&result);
                work.end_program(remaining, result.is_ok());
                result
            });
            Run {
                result,
                remaining,
                witnesses,
                old: work.snapshot(),
                selected: work.selected_static_snapshot(),
            }
        }
        fn parity(a: &Run, b: &Run) {
            assert_eq!(a.result, b.result);
            assert_eq!(a.remaining, b.remaining);
            assert_eq!(a.witnesses, b.witnesses);
            assert_eq!(a.old, b.old);
        }
        #[test]
        fn selected_static_observation_preserves_short_partial_and_malformed_outcomes() {
            let (grammar, source) = fixture();
            let counter = selected();
            let program =
                compile(&grammar, &source, &PredictionObservation::new(), &counter).unwrap();
            let ids = counter.selected_static_snapshot().unwrap();
            let decision = &program.decisions["rule/Entry/choice"];
            let ample = run(&program, decision, &grammar, &source, ids, 10_000, false);
            assert!(ample.result.is_ok());
            let exact = 10_000 - ample.remaining;
            assert!(exact > 2);
            for grant in [0, 2, exact - 1, exact] {
                let off = run(&program, decision, &grammar, &source, ids, grant, false);
                let on = run(&program, decision, &grammar, &source, ids, grant, true);
                println!(
                    "SELECTED_STATIC_PARTIAL grant={grant} result={:?} snapshot={:?}",
                    on.result, on.selected
                );
                parity(&off, &on);
                assert_eq!(on.result.is_ok(), grant == exact);
                let s = on.selected.unwrap();
                if grant == 0 {
                    assert!(!s.finished);
                } else {
                    assert!(s.finished);
                    assert_eq!(s.returned_error, on.result.is_err());
                    assert_eq!(
                        s.decision_work,
                        s.entry_remaining.unwrap().checked_sub(on.remaining)
                    );
                    assert!(!s.overflowed && !s.inconsistent);
                }
            }
            let mut malformed = decision.clone();
            malformed.entries[0] = program.states.len();
            let off = run(&program, &malformed, &grammar, &source, ids, 10_000, false);
            let on = run(&program, &malformed, &grammar, &source, ids, 10_000, true);
            parity(&off, &on);
            assert!(on.result.is_err());
            assert!(on.selected.unwrap().returned_error);
            assert!(
                RecursiveAnalysisCounters::with_selected_static(
                    &"x".repeat(257),
                    "Wrapper",
                    "Leaf"
                )
                .is_err()
            );
            let unknown = RecursiveAnalysisCounters::with_selected_static(
                "rule/Entry/choice",
                "missing",
                "missing",
            )
            .unwrap();
            let result =
                compile(&grammar, &source, &PredictionObservation::new(), &unknown).unwrap();
            assert_eq!(
                serde_json::to_value(result).unwrap(),
                serde_json::to_value(program).unwrap()
            );
            assert_eq!(
                unknown
                    .selected_static_snapshot()
                    .unwrap()
                    .residual_wrapper_upper_bound,
                None
            );
        }
    }
    fn expr(kind: Kind) -> Expr {
        Expr {
            kind,
            first_set_predicated: false,
            original_range: None,
            prediction_path: None,
            cardinality: '1',
        }
    }
    #[test]
    fn observed_parser_leaf_success_keeps_entire_program() {
        let grammar = grammar(sequence(vec![keyword("a"), keyword("b")]), vec![]);
        let source = intersect_model::definitions::bundle::DefinitionSource::new(
            "memory:leaf-control.xtext",
            intersect_model::definitions::bundle::SourceKind::Xtext,
            "Entry: 'a' 'b';",
        );
        let observation = super::super::admission_observation::PredictionObservation::new();
        let context = super::super::AnalysisObservation {
            observation: &observation,
            recursive_work: None,
            adjacent_work: None,
            entry: &source.id,
            stage: super::super::admission_observation::RunStage::Executable,
        };
        let ordinary = ParserPrediction::compile(&grammar).unwrap();
        let observed =
            ParserPrediction::compile_observed(&grammar, &BTreeSet::new(), false, context).unwrap();
        assert_eq!(
            serde_json::to_value(ordinary).unwrap(),
            serde_json::to_value(observed).unwrap()
        );
        assert!(observation.first_failure().is_none());
    }
    #[test]
    fn observed_parser_leaf_separates_rule_state_and_body_state_ceilings() {
        use super::super::admission_observation::{Phase, PredictionObservation, RunStage};
        use intersect_model::definitions::bundle::DefinitionSource;
        use intersect_model::definitions::bundle::SourceKind;
        use intersect_model::definitions::bundle::SourceSpan;
        let rows: Vec<_> = (0..5001).map(|index| format!("R{index:05}")).collect();
        let mut allocation = grammar(keyword("x"), vec![]);
        let template = allocation.rules.remove("Entry").unwrap();
        let mut raw = String::new();
        let mut spans = Vec::new();
        for name in &rows {
            let start = raw.len();
            raw.push_str(&format!("{name}: 'x';\n"));
            spans.push((name, start, raw.len()));
            allocation.rules.insert(name.clone(), template.clone());
        }
        allocation.entry = rows[0].clone();
        let source = DefinitionSource::new("memory:leaf-rule-states.xtext", SourceKind::Xtext, raw);
        for (name, start, end) in spans {
            allocation.declaration_sources.insert(
                name.clone(),
                SourceSpan {
                    source: source.id.clone(),
                    start,
                    end,
                },
            );
        }
        let body_source = DefinitionSource::new(
            "memory:leaf-body-states.xtext",
            SourceKind::Xtext,
            format!("Entry: {};", vec!["'x'"; 10002].join(" ")),
        );
        let mut body = grammar(sequence((0..10002).map(|_| keyword("x")).collect()), vec![]);
        body.declaration_sources.insert(
            "Entry".into(),
            SourceSpan {
                source: body_source.id.clone(),
                start: 0,
                end: body_source.text.len(),
            },
        );
        for (grammar, source, phase, owner) in [
            (&allocation, &source, Phase::ParserRuleStates, "R05000"),
            (&body, &body_source, Phase::ParserRuleGraph, "Entry"),
        ] {
            let observation = PredictionObservation::new();
            let ordinary = ParserPrediction::compile(grammar).err().unwrap();
            let observed = ParserPrediction::compile_observed(
                grammar,
                &BTreeSet::new(),
                false,
                super::super::AnalysisObservation {
                    observation: &observation,
                    recursive_work: None,
                    adjacent_work: None,
                    entry: &source.id,
                    stage: RunStage::Executable,
                },
            )
            .err()
            .unwrap();
            assert_eq!(ordinary, "Grammar prediction budget exceeded");
            assert_eq!(observed, ordinary);
            let failure = observation.first_failure().unwrap();
            assert_eq!(failure.phase, phase);
            assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), owner);
            assert_eq!(failure.state_count, Some(10000));
            assert_eq!(failure.charged_work, None);
            assert_eq!(
                failure.source.unwrap().sha256.as_str().unwrap(),
                source.id.sha256
            );
            assert_eq!(
                failure.declaration_span,
                Some((
                    grammar.declaration_sources[owner].start,
                    grammar.declaration_sources[owner].end
                ))
            );
            assert_eq!(failure.decision, None);
            assert_eq!(failure.syntax_depth, None);
            assert_eq!(failure.call_depth, None);
        }
    }
    fn keyword(value: &str) -> Expr {
        expr(Kind::Literal(value.into()))
    }
    fn sequence(items: Vec<Expr>) -> Expr {
        expr(Kind::Sequence(items))
    }
    fn choice(items: Vec<Expr>) -> Expr {
        expr(Kind::Alternative(items))
    }
    fn call(name: &str) -> Expr {
        expr(Kind::Call(name.into()))
    }
    fn grammar(body: Expr, rules: Vec<(&str, Expr)>) -> Grammar {
        let mut rules: BTreeMap<_, _> = rules
            .into_iter()
            .map(|(name, body)| {
                (
                    name.into(),
                    Rule {
                        hidden: None,
                        enum_literals: BTreeMap::new(),
                        nominal_return: None,
                        inferred_return: false,
                        override_declared: false,
                        datatype: None,
                        fragment: false,
                        wildcard: false,
                        class: String::new(),
                        body,
                    },
                )
            })
            .collect();
        rules.insert(
            "Entry".into(),
            Rule {
                hidden: None,
                enum_literals: BTreeMap::new(),
                nominal_return: None,
                inferred_return: false,
                override_declared: false,
                datatype: None,
                fragment: false,
                wildcard: false,
                class: String::new(),
                body,
            },
        );
        Grammar {
            call_depth: None,
            analysis_work_budget: Default::default(),
            parameterized_rules: Default::default(),
            final_rules: Default::default(),
            computed_settings: Vec::new(),
            profile: Profile::NativeV5,
            name: "PredictionTest".into(),
            entry: "Entry".into(),
            rules,
            terminals: BTreeMap::new(),
            hidden: None,
            lexical_prediction: None,
            parser_prediction: None,
            declaration_sources: BTreeMap::new(),
            terminal_order: Vec::new(),
            shadowed_declarations: BTreeMap::new(),
        }
    }
    fn symbols(text: &str) -> Vec<PredictionSymbol> {
        text.split_whitespace()
            .map(|word| PredictionSymbol::Keyword(word.into()))
            .chain([PredictionSymbol::End])
            .collect()
    }
    fn selected(
        program: &ParserPrediction,
        decision: &str,
        text: &str,
        remaining: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        let mut input = symbols(text).into_iter();
        program.predict(decision, || Ok(input.next()), remaining)
    }
    #[test]
    fn decisions_consume_only_the_distinguishing_prefix_and_replay_exactly() {
        let g = grammar(
            choice(vec![
                sequence(vec![keyword("a"), keyword("b")]),
                sequence(vec![keyword("a"), keyword("c")]),
            ]),
            vec![],
        );
        let program = ParserPrediction::compile(&g).unwrap();
        let mut input = symbols("a c").into_iter();
        let mut reads = 0;
        let result = program
            .predict(
                "rule/Entry/choice",
                || {
                    reads += 1;
                    if reads > 2 {
                        return Err("Unrelated suffix was inspected".into());
                    }
                    Ok(input.next())
                },
                &mut Some(1000),
            )
            .unwrap();
        assert_eq!(result, Some(1));
        assert_eq!(reads, 2);
        assert_eq!(
            selected(&program, "rule/Entry/choice", "a b extra", &mut None).unwrap(),
            Some(0)
        );
        assert_eq!(
            selected(&program, "rule/Entry/choice", "a", &mut None).unwrap(),
            None
        );
        let replay: ParserPrediction =
            serde_json::from_value(serde_json::to_value(&program).unwrap()).unwrap();
        assert_eq!(
            selected(&replay, "rule/Entry/choice", "a c", &mut None).unwrap(),
            Some(1)
        );
    }
    #[test]
    fn optional_and_repeat_decisions_use_caller_continuations() {
        for cardinality in ['?', '*', '+'] {
            let mut repeated = sequence(vec![keyword("a"), keyword("b")]);
            repeated.cardinality = cardinality;
            let g = grammar(
                sequence(vec![call("Body"), keyword("a"), keyword("c")]),
                vec![("Body", repeated)],
            );
            let program = ParserPrediction::compile(&g).unwrap();
            program
                .verify_disjoint_decision("rule/Body/repeat")
                .unwrap();
            assert_eq!(
                selected(&program, "rule/Body/repeat", "a c", &mut None).unwrap(),
                None
            );
            assert_eq!(
                selected(&program, "rule/Body/repeat", "a b a c", &mut None).unwrap(),
                Some(0)
            );
            assert_eq!(
                selected(&program, "rule/Body/repeat", "a", &mut None).unwrap(),
                None
            );
        }
    }
    #[test]
    fn recursive_calls_return_to_their_exact_suffix_and_work_is_not_refunded() {
        let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
        inner.cardinality = '?';
        let g = grammar(
            choice(vec![
                sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
                sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
            ]),
            vec![("Inner", inner)],
        );
        let program = ParserPrediction::compile(&g).unwrap();
        for (text, expected) in [
            ("a b", 0),
            ("a c", 1),
            ("a a b b", 0),
            ("a a b c", 1),
            ("a a a b b c", 1),
        ] {
            let mut remaining = Some(10_000);
            assert_eq!(
                selected(&program, "rule/Entry/choice", text, &mut remaining).unwrap(),
                Some(expected)
            );
            let used = 10_000 - remaining.unwrap();
            assert!(used > 1);
            let mut exact = Some(used);
            assert_eq!(
                selected(&program, "rule/Entry/choice", text, &mut exact).unwrap(),
                Some(expected)
            );
            assert_eq!(exact, Some(0));
            let mut short = Some(used - 1);
            assert!(
                selected(&program, "rule/Entry/choice", text, &mut short)
                    .unwrap_err()
                    .contains("step budget")
            );
            assert_eq!(short, Some(0));
        }
    }
    #[test]
    fn actions_and_assignments_are_syntax_only_and_unresolved_ambiguity_rejects() {
        let syntax = sequence(vec![
            expr(Kind::Action("NotConstructed".into())),
            expr(Kind::Assign {
                feature: "notWritten".into(),
                many: false,
                value: Box::new(call("ID")),
            }),
        ]);
        let g = grammar(
            choice(vec![
                sequence(vec![syntax.clone(), keyword("b")]),
                sequence(vec![syntax, keyword("c")]),
            ]),
            vec![],
        );
        let program = ParserPrediction::compile(&g).unwrap();
        let mut input = [
            PredictionSymbol::Terminal("ID".into()),
            PredictionSymbol::Keyword("c".into()),
        ]
        .into_iter();
        assert_eq!(
            program
                .predict("rule/Entry/choice", || Ok(input.next()), &mut None)
                .unwrap(),
            Some(1)
        );
        let ambiguous =
            ParserPrediction::compile(&grammar(choice(vec![keyword("a"), keyword("a")]), vec![]))
                .unwrap();
        assert!(
            selected(&ambiguous, "rule/Entry/choice", "a", &mut None)
                .unwrap_err()
                .contains("Ambiguous")
        );
    }
    #[test]
    fn regular_decisions_match_independent_accepted_oracle_branches() {
        let observations: serde_json::Value = serde_json::from_str(include_str!(
            "../../assets/fixtures/xtext/first-token-predicates/decision-observations.json"
        ))
        .unwrap();
        let mut compared = 0;
        for row in observations["decisions"].as_array().unwrap() {
            let parser = row["parser"].as_str().unwrap();
            if !row["accepted"].as_bool().unwrap() || parser.ends_with("Recursive") {
                continue;
            }
            let guarded = sequence(vec![keyword("a"), keyword("b")]);
            let (g, decision) = if parser.ends_with("Choice") {
                (
                    grammar(
                        choice(vec![guarded, sequence(vec![keyword("a"), keyword("c")])]),
                        vec![],
                    ),
                    "rule/Entry/choice",
                )
            } else {
                let mut repeated = guarded;
                repeated.cardinality = if parser.ends_with("Optional") {
                    '?'
                } else {
                    '*'
                };
                (
                    grammar(sequence(vec![repeated, keyword("a"), keyword("c")]), vec![]),
                    "rule/Entry/s0/repeat",
                )
            };
            let program = ParserPrediction::compile(&g).unwrap();
            program.verify_disjoint_decision(decision).unwrap();
            let expected = if row["branch"].as_str().unwrap().starts_with('g') {
                Some(0)
            } else if parser.ends_with("Choice") {
                Some(1)
            } else {
                None
            };
            assert_eq!(
                selected(
                    &program,
                    decision,
                    row["text"].as_str().unwrap(),
                    &mut Some(10_000)
                )
                .unwrap(),
                expected,
                "{row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 14);
        // Malformed-input recovery and recursive predicate commitment are not
        // claimed by this unpredicated graph comparison.
    }
    #[test]
    fn compiled_teaching_grammar_produces_a_readonly_prediction_program() {
        let source = intersect_model::definitions::bundle::DefinitionSource::new(
            "memory:oracle.ecore",
            intersect_model::definitions::bundle::SourceKind::Ecore,
            include_str!("../../assets/fixtures/xtext/boolean-keywords/model.ecore"),
        );
        let converted = intersect_model::migration::migrate_ecore(source, "memory:fixture.rcore")
            .unwrap()
            .converted
            .unwrap();
        let entry = converted.id.clone();
        let namespace =
            intersect_model::definitions::rcore::declared_namespace(&converted).unwrap();
        let bundle = intersect_model::definitions::bundle::DefinitionBundle {
            version: 1,
            sources: vec![converted],
            namespaces: vec![],
        };
        let mut schema = intersect_model::definitions::compiler::compile_structure_bundle(
            &bundle, &entry, &namespace,
        )
        .unwrap();
        let grammar = super::super::compile_with_reference_context_and_profile(
            include_str!("../../assets/fixtures/xtext/boolean-keywords/model.xtext"),
            &mut schema,
            Profile::NativeV1,
        )
        .unwrap();
        let retained = serde_json::to_value(&grammar).unwrap();
        let program = ParserPrediction::compile(&grammar).unwrap();
        assert!(
            program
                .decisions()
                .any(|name| name == "rule/Model/s3/repeat")
        );
        for text in ["abstract type", "type"] {
            assert_eq!(
                selected(&program, "rule/Model/s3/repeat", text, &mut Some(1000)).unwrap(),
                Some(0)
            );
        }
        assert_eq!(
            selected(&program, "rule/Model/s3/repeat", "}", &mut Some(1000)).unwrap(),
            None
        );
        assert_eq!(serde_json::to_value(&grammar).unwrap(), retained);
    }
    #[test]
    fn visible_guards_resolve_shared_continuations_without_backtracking() {
        let repeated = sequence(vec![keyword("a"), keyword("b")]);
        let g = grammar(choice(vec![repeated.clone(), repeated]), vec![]);
        let program =
            ParserPrediction::compile_with_first_predicates(&g, &["rule/Entry/a0".into()].into())
                .unwrap();
        let observations: serde_json::Value = serde_json::from_str(include_str!(
            "../../assets/fixtures/xtext/first-token-predicates/visibility-observations.json"
        ))
        .unwrap();
        let mut compared = 0;
        for row in observations["decisions"].as_array().unwrap() {
            if row["parser"] != "EntryCommit" {
                continue;
            }
            let expected = match row["branch"].as_str().unwrap() {
                "g" => Some(0),
                "f" => Some(1),
                "" => None,
                _ => panic!("Unexpected oracle branch"),
            };
            assert_eq!(
                selected(
                    &program,
                    "rule/Entry/choice",
                    row["text"].as_str().unwrap(),
                    &mut Some(10_000)
                )
                .unwrap(),
                expected,
                "{row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 11);
        let mut budget = Some(10_000);
        assert_eq!(
            selected(&program, "rule/Entry/choice", "a b", &mut budget).unwrap(),
            Some(0)
        );
        let used = 10_000 - budget.unwrap();
        assert_eq!(
            selected(&program, "rule/Entry/choice", "a b", &mut Some(used)).unwrap(),
            Some(0)
        );
        assert!(
            selected(&program, "rule/Entry/choice", "a b", &mut Some(used - 1))
                .unwrap_err()
                .contains("step budget")
        );
        assert_eq!(
            selected(&program, "rule/Entry/choice", "b", &mut None).unwrap(),
            None
        );
        let replay: ParserPrediction =
            serde_json::from_value(serde_json::to_value(program).unwrap()).unwrap();
        assert_eq!(
            selected(&replay, "rule/Entry/choice", "a b", &mut None).unwrap(),
            Some(0)
        );
    }
    #[test]
    fn actions_tokens_and_called_rule_boundaries_hide_enclosing_guards() {
        let ordinary = sequence(vec![keyword("a"), keyword("b")]);
        let action = sequence(vec![
            expr(Kind::Action("NoModelEffect".into())),
            ordinary.clone(),
        ]);
        let cases = [
            (
                grammar(choice(vec![action, ordinary.clone()]), vec![]),
                "rule/Entry/a0/s1",
            ),
            (
                grammar(choice(vec![ordinary.clone(), ordinary.clone()]), vec![]),
                "rule/Entry/a0/s1",
            ),
            (
                grammar(
                    choice(vec![call("One"), call("Two")]),
                    vec![("One", ordinary.clone()), ("Two", ordinary)],
                ),
                "rule/One",
            ),
        ];
        for (g, path) in cases {
            let program =
                ParserPrediction::compile_with_first_predicates(&g, &[path.into()].into()).unwrap();
            assert!(
                selected(&program, "rule/Entry/choice", "a b", &mut None)
                    .unwrap_err()
                    .contains("Ambiguous"),
                "{path}"
            );
        }
    }
    #[test]
    fn guard_binding_preserves_regular_oracle_choices_and_rejects_unverified_recursion() {
        let observations: serde_json::Value = serde_json::from_str(include_str!(
            "../../assets/fixtures/xtext/first-token-predicates/decision-observations.json"
        ))
        .unwrap();
        let mut compared = 0;
        for row in observations["decisions"].as_array().unwrap() {
            let parser = row["parser"].as_str().unwrap();
            if !parser.starts_with("First")
                || !row["accepted"].as_bool().unwrap()
                || parser.ends_with("Recursive")
            {
                continue;
            }
            let guarded = sequence(vec![keyword("a"), keyword("b")]);
            let (g, decision, path) = if parser.ends_with("Choice") {
                (
                    grammar(
                        choice(vec![guarded, sequence(vec![keyword("a"), keyword("c")])]),
                        vec![],
                    ),
                    "rule/Entry/choice",
                    "rule/Entry/a0",
                )
            } else {
                let mut repeated = guarded;
                repeated.cardinality = if parser.ends_with("Optional") {
                    '?'
                } else {
                    '*'
                };
                (
                    grammar(sequence(vec![repeated, keyword("a"), keyword("c")]), vec![]),
                    "rule/Entry/s0/repeat",
                    "rule/Entry/s0",
                )
            };
            let program =
                ParserPrediction::compile_with_first_predicates(&g, &[path.into()].into()).unwrap();
            let expected = if row["branch"].as_str().unwrap().starts_with('g') {
                Some(0)
            } else if parser.ends_with("Choice") {
                Some(1)
            } else {
                None
            };
            assert_eq!(
                selected(
                    &program,
                    decision,
                    row["text"].as_str().unwrap(),
                    &mut Some(10_000)
                )
                .unwrap(),
                expected,
                "{row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 7);
        let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
        inner.cardinality = '?';
        let recursive = grammar(
            choice(vec![
                sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
                sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
            ]),
            vec![("Inner", inner)],
        );
        assert!(
            ParserPrediction::compile_with_first_predicates(
                &recursive,
                &["rule/Entry/a0".into()].into()
            )
            .unwrap_err()
            .contains("Recursive first-token")
        );
        assert!(
            ParserPrediction::compile_with_first_predicates(
                &recursive,
                &["rule/missing".into()].into()
            )
            .unwrap_err()
            .contains("Unknown first-token")
        );
        let empty = grammar(expr(Kind::Action("NoToken".into())), vec![]);
        assert!(
            ParserPrediction::compile_with_first_predicates(&empty, &["rule/Entry".into()].into())
                .unwrap_err()
                .contains("Empty first-token")
        );
    }
    #[test]
    fn guarded_specialization_repetition_resolves_the_pinned_continuation_shape() {
        let mut first = call("Specialization");
        first.cardinality = '+';
        let mut multiplicity = sequence(vec![keyword("["), call("INT"), keyword("]")]);
        multiplicity.cardinality = '?';
        let mut tail = call("Specialization");
        tail.cardinality = '*';
        let part = sequence(vec![first, multiplicity, tail]);
        let specialization = choice(vec![
            sequence(vec![keyword("typed"), call("ID")]),
            sequence(vec![keyword(":"), call("ID")]),
        ]);
        let g = grammar(
            sequence(vec![call("Part"), keyword(";")]),
            vec![("Part", part), ("Specialization", specialization)],
        );
        let program =
            ParserPrediction::compile_with_first_predicates(&g, &["rule/Part/s0".into()].into())
                .unwrap();
        for (tokens, expected) in [
            (
                vec![
                    PredictionSymbol::Keyword("typed".into()),
                    PredictionSymbol::Terminal("ID".into()),
                    PredictionSymbol::Keyword(";".into()),
                    PredictionSymbol::End,
                ],
                Some(0),
            ),
            (
                vec![
                    PredictionSymbol::Keyword(":".into()),
                    PredictionSymbol::Terminal("ID".into()),
                    PredictionSymbol::Keyword(";".into()),
                    PredictionSymbol::End,
                ],
                Some(0),
            ),
            (
                vec![
                    PredictionSymbol::Keyword("[".into()),
                    PredictionSymbol::Terminal("INT".into()),
                    PredictionSymbol::Keyword("]".into()),
                    PredictionSymbol::Keyword(";".into()),
                    PredictionSymbol::End,
                ],
                None,
            ),
            (
                vec![PredictionSymbol::Keyword(";".into()), PredictionSymbol::End],
                None,
            ),
        ] {
            let mut input = tokens.into_iter();
            assert_eq!(
                program
                    .predict(
                        "rule/Part/s0/repeat",
                        || Ok(input.next()),
                        &mut Some(10_000)
                    )
                    .unwrap(),
                expected
            );
        }
    }
    #[test]
    fn guard_binding_enforces_structural_depth_before_walking_called_rules() {
        let mut nested = keyword("a");
        for _ in 0..66 {
            nested = sequence(vec![nested]);
        }
        let g = grammar(call("Deep"), vec![("Deep", nested)]);
        assert!(
            ParserPrediction::compile_with_first_predicates(&g, &["rule/Entry".into()].into())
                .unwrap_err()
                .contains("budget")
        );
    }
    #[test]
    fn graph_and_context_errors_stay_explicit() {
        let g = grammar(
            keyword("entry"),
            vec![("Unused", choice(vec![keyword("a"), keyword("a")]))],
        );
        let program = ParserPrediction::compile(&g).unwrap();
        assert!(
            selected(&program, "missing", "a", &mut None)
                .unwrap_err()
                .contains("Unknown")
        );
        assert!(
            selected(&program, "rule/Unused/choice", "a", &mut None)
                .unwrap_err()
                .contains("FOLLOW")
        );
        let recursive = grammar(
            choice(vec![call("Loop"), keyword("a")]),
            vec![("Loop", call("Loop"))],
        );
        let program = ParserPrediction::compile(&recursive).unwrap();
        assert!(
            selected(&program, "rule/Entry/choice", "a", &mut Some(10_000))
                .unwrap_err()
                .contains("call depth")
        );
        let huge = grammar(choice((0..5001).map(|_| keyword("a")).collect()), vec![]);
        // A second large rule forces the unchanged 10,000-state admission ceiling.
        let mut huge = huge;
        huge.rules
            .insert("Other".into(), huge.rules["Entry"].clone());
        assert!(
            ParserPrediction::compile(&huge)
                .unwrap_err()
                .contains("budget")
        );
    }
    #[test]
    fn recursive_first_commit_matches_independent_branch_entry_and_preserves_old_admission() {
        let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
        inner.cardinality = '?';
        let g = grammar(
            choice(vec![
                sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
                sequence(vec![keyword("a"), call("Inner"), keyword("c")]),
            ]),
            vec![("Inner", inner)],
        );
        let guards = ["rule/Entry/a0".into()].into();
        assert!(
            ParserPrediction::compile_with_first_predicates(&g, &guards)
                .unwrap_err()
                .contains("Recursive first-token")
        );
        let program =
            ParserPrediction::compile_with_recursive_first_predicates(&g, &guards).unwrap();
        assert!(program.recursive_decisions.contains("rule/Entry/choice"));
        let rows: serde_json::Value = serde_json::from_str(include_str!(
            "../../assets/fixtures/xtext/recursive-predicates/observations.json"
        ))
        .unwrap();
        let mut compared = 0;
        for row in rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["parser"] == "RecursiveEntryFirst")
        {
            let expected = match row["branch"].as_str().unwrap() {
                "g" => Some(0),
                "f" => Some(1),
                "" => None,
                _ => panic!("Unexpected oracle branch"),
            };
            assert_eq!(
                selected(
                    &program,
                    "rule/Entry/choice",
                    row["text"].as_str().unwrap(),
                    &mut Some(10_000)
                )
                .unwrap(),
                expected,
                "{row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 11);
        let encoded = serde_json::to_string(&program).unwrap();
        let replay: ParserPrediction = serde_json::from_str(&encoded).unwrap();
        let mut remaining = Some(10_000);
        assert_eq!(
            selected(&replay, "rule/Entry/choice", "a c", &mut remaining).unwrap(),
            Some(0)
        );
        let used = 10_000 - remaining.unwrap();
        assert_eq!(
            selected(&replay, "rule/Entry/choice", "a c", &mut Some(used)).unwrap(),
            Some(0)
        );
        assert!(
            selected(&replay, "rule/Entry/choice", "a c", &mut Some(used - 1))
                .unwrap_err()
                .contains("budget")
        );
        let mut reads = 0;
        assert_eq!(
            program
                .predict(
                    "rule/Entry/choice",
                    || {
                        reads += 1;
                        assert_eq!(reads, 1);
                        Ok(Some(PredictionSymbol::Keyword("a".into())))
                    },
                    &mut Some(10_000)
                )
                .unwrap(),
            Some(0)
        );
    }
    #[test]
    fn recursion_outside_conflicting_prefixes_does_not_force_commit() {
        for profile in [Profile::NativeV5, Profile::NativeV38, Profile::NativeV39] {
            let mut inner = sequence(vec![keyword("a"), call("Inner"), keyword("b")]);
            inner.cardinality = '?';
            for branches in [
                vec![
                    sequence(vec![keyword("a"), call("Inner"), keyword("b")]),
                    sequence(vec![keyword("a"), keyword("c")]),
                ],
                vec![
                    sequence(vec![keyword("a"), keyword("b"), call("Inner")]),
                    sequence(vec![keyword("a"), keyword("c"), call("Inner")]),
                ],
                vec![
                    sequence(vec![keyword("a"), call("Inner")]),
                    sequence(vec![keyword("c"), call("Inner")]),
                ],
            ] {
                let mut g = grammar(choice(branches), vec![("Inner", inner.clone())]);
                g.profile = profile;
                let program = ParserPrediction::compile_with_recursive_first_predicates(
                    &g,
                    &["rule/Entry/a0".into()].into(),
                )
                .unwrap();
                assert!(program.recursive_decisions.is_empty());
                let input = if matches!(&g.rules["Entry"].body.kind, Kind::Alternative(items) if matches!(&items[1].kind, Kind::Sequence(parts) if matches!(&parts[0].kind, Kind::Literal(s) if s == "c")))
                {
                    "c"
                } else {
                    "a c"
                };
                assert_eq!(
                    selected(&program, "rule/Entry/choice", input, &mut Some(10_000)).unwrap(),
                    Some(1)
                );
            }
        }
    }
    #[test]
    fn shared_callee_witness_preserves_budget_and_caller_boundary() {
        let mut recursive = sequence(vec![keyword("("), call("Inner"), keyword(")")]);
        recursive.cardinality = '?';
        let mut body = vec![keyword("p"); 2500];
        body.push(recursive);
        let mut g = grammar(
            choice(vec![
                sequence(vec![call("Inner"), keyword("left")]),
                sequence(vec![call("Inner"), keyword("right")]),
            ]),
            vec![("Inner", sequence(body))],
        );
        let guards = ["rule/Entry/a0".into()].into();
        g.profile = Profile::NativeV37;
        let old = ParserPrediction::compile_with_recursive_first_predicates(&g, &guards).unwrap();
        let (result, remaining) = bounded_recursive_decision(
            &old,
            "rule/Entry/choice",
            super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units(),
            false,
            false,
        );
        assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
        assert_eq!(remaining, 0);
        g.profile = Profile::NativeV38;
        let program =
            ParserPrediction::compile_with_recursive_first_predicates(&g, &guards).unwrap();
        assert!(program.recursive_decisions.contains("rule/Entry/choice"));
        let (result, remaining) = bounded_recursive_decision(
            &program,
            "rule/Entry/choice",
            super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units(),
            true,
            false,
        );
        assert_eq!(result, Ok(true));
        assert!(remaining > 0);

        assert_eq!(
            selected(&program, "rule/Entry/choice", "p", &mut Some(100)),
            Ok(Some(0))
        );
        let entry = program.rules["Inner"].0;
        assert!(!program.recursion_witness(entry, 1, &mut 10_000).unwrap());
        assert!(program.recursion_witness(entry, 128, &mut 1).is_err());
        let mut tail = sequence(vec![keyword("a"), call("Tail"), keyword("b")]);
        tail.cardinality = '?';
        let mut g = grammar(
            choice(vec![
                sequence(vec![call("Inner"), keyword("left"), call("Tail")]),
                sequence(vec![call("Inner"), keyword("right"), call("Tail")]),
            ]),
            vec![("Inner", keyword("p")), ("Tail", tail)],
        );
        g.profile = Profile::NativeV38;
        let program =
            ParserPrediction::compile_with_recursive_first_predicates(&g, &guards).unwrap();
        assert!(program.recursive_decisions.is_empty());
        assert_eq!(
            selected(&program, "rule/Entry/choice", "p right", &mut Some(100)),
            Ok(Some(1))
        );
    }
    include!("parser_prediction/recursive_counter_tests.rs");
    include!("parser_prediction/completion_tests.rs");
    include!("parser_prediction/nested_completion_tests.rs");
    include!("parser_prediction/executable_decisions_tests.rs");
    include!("parser_prediction/full_predicate_tests.rs");
    include!("parser_prediction/completion_policy_tests.rs");
    include!("parser_prediction/nullable_repeat_tests.rs");
    include!("parser_prediction/runtime_source_tests.rs");
    include!("parser_prediction/runtime_prepared_tests.rs");
    include!("parser_prediction/runtime_live_prepared_tests.rs");
    include!("parser_prediction/live_caller_tests.rs");
    include!("parser_prediction/shared_return_tests.rs");
    include!("parser_prediction/runtime_geometry_tests.rs");
    include!("parser_prediction/runtime_ports_tests.rs");
    mod full_layout_tests {
        include!("parser_prediction/full_layout_tests.rs");
    }
}

#[cfg(test)]
#[path = "parser_prediction/breadth_first_tests.rs"]
mod breadth_first_tests;

#[cfg(test)]
#[path = "parser_prediction/partial_token_probe_tests.rs"]
mod partial_token_probe_tests;

#[cfg(test)]
#[path = "parser_prediction/pool_attribution_tests.rs"]
mod pool_attribution_tests;

#[cfg(test)]
#[path = "parser_prediction/pool_protocol_tests.rs"]
mod pool_protocol_tests;

// Inline, default-inert adaptation. Existing concrete PredictionInput adapters
// inherit false, so monomorphized optimized workers have no snapshot callbacks.
// No observer reads input, changes a fee, supplies a budget or interprets graph.
#[inline(always)]
fn runtime_work_geometry<I: PredictionInput>(
    input: &mut I,
    shared_storage: bool,
    kind: &'static str,
    base_depth: usize,
    return_depth: usize,
    amount: usize,
) {
    if input.runtime_work_diagnostics() {
        input.runtime_work_geometry(RuntimeWorkGeometry {
            kind,
            shared_storage,
            base_depth,
            return_depth,
            amount,
        });
    }
}

#[inline(always)]
fn runtime_work_snapshot<I: PredictionInput>(
    input: &mut I,
    phase: &'static str,
    remaining: &Option<usize>,
) {
    if input.runtime_work_diagnostics() {
        input.runtime_work_snapshot(phase, *remaining);
    }
}

#[inline(always)]
fn runtime_work_read<I: PredictionInput>(
    input: &mut I,
    remaining: &mut Option<usize>,
    after: &'static str,
) -> Result<Option<PredictionSymbol>, String> {
    runtime_work_snapshot(input, "input", remaining);
    let result = input.read(remaining);
    runtime_work_snapshot(input, after, remaining);
    result
}
