//! Explicit, compiler-minted obligations for one bounded native execution slice.
//! No Deserialize/Default constructor creates a usable admission companion,
//! with ONE host-cache exception: `SavedRuntimeAdmission` (the record/choice
//! payload that `RuntimeGuardedAdmission::save` produces from a compiler-bound
//! admission; deserialization can build one from any bytes, which grants
//! nothing by itself) may be restored by the compiler's
//! `CompiledRuntimeGrammar::load_host_cache`. That load first requires the
//! saved record plus choice keys to EQUAL, as a set, the decision set of the
//! `Requirement` stored in the SAME saved grammar for the same policy, then
//! binds the owner under the compiler-only publication permit. That
//! `Requirement` is itself trusted saved data: neither load mode proves the
//! decision set is the one compilation would derive; only recompilation does.
//! The load trusts the bytes as the host's own previous compiler output (the
//! same contract as `VerifiedExecutable::load_trusted`); this is a convention,
//! not cryptography, and model, network or browser input must never reach it.
//! `RuntimeAdmissionLoadMode::Revalidate` additionally re-runs the seal-time
//! record and choice proof checks; recompilation remains the CI oracle.
use intersect_model::definitions::bundle::SourceId;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) enum Requirement {
    LinearFullRepeats {
        decisions: std::collections::BTreeSet<String>,
    },
    StrictCompletionScopes {
        decisions: std::collections::BTreeSet<String>,
    },
    InputCheckedCompletionScopes {
        decisions: std::collections::BTreeSet<String>,
    },
}
/// Explicit proof selection; a request alone is not executable authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeGuardedPolicy {
    LinearFullRepeats,
    StrictCompletionScopes,
    /// Compiler-bound source/loop and guarded-choice obligations. Qualifying
    /// scopes use canonical per-input decisions, not global disjointness.
    InputCheckedCompletionScopes,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    proof_class: RuntimeGuardedPolicy,
    owner: String,
    callee: String,
    source: SourceId,
    callee_source: SourceId,
    declaration: Range<usize>,
    occurrence: Range<usize>,
    full: Range<usize>,
    operand: Range<usize>,
    graph: [usize; 7],
}
// Equality-only addresses, never dereferenced, ordered or persisted. The boxed
// definition stays alive for every companion borrow. No raw pointer auto-traits.
#[derive(Clone, Copy)]
enum Owner {
    Unsealed,
    Reserved {
        marker: usize,
    },
    Bound {
        marker: usize,
        grammar: usize,
        program: usize,
    },
}
/// A native compiler product, not serialized or user-asserted admission. The
/// single exception is the host-cache restore of a `SavedRuntimeAdmission`
/// through `CompiledRuntimeGrammar::load_host_cache`, under the
/// `VerifiedExecutable::load_trusted` contract (see the module docs).
pub struct RuntimeGuardedAdmission {
    records: BTreeMap<String, Binding>,
    choices: BTreeMap<String, ChoiceBinding>,
    owner: Owner,
    logical_bytes: usize,
    failed: bool,
    policy: RuntimeGuardedPolicy,
    work_budget: super::super::GrammarAnalysisWorkBudget,
    /// Combined call depth of the compiled prediction program.
    call_depth: u16,
    analysis_initial_work: Option<usize>,
    // Component-only cold baseline through the SAME complete compiler worker.
    // Absent from every production library/host/WASM representation.
    #[cfg(test)]
    disable_completed_reuse: bool,
    // Component-only inventory baseline through the SAME canonical compiler.
    #[cfg(test)]
    disable_inventory: bool,
}
impl RuntimeGuardedAdmission {
    pub fn policy(&self) -> RuntimeGuardedPolicy {
        self.policy
    }
    pub fn decision_count(&self) -> usize {
        self.records.len() + self.choices.len()
    }
    pub fn decisions(&self) -> impl Iterator<Item = &str> {
        self.records
            .keys()
            .chain(self.choices.keys())
            .map(String::as_str)
    }
    pub(crate) fn contains(&self, path: &str) -> bool {
        self.records.contains_key(path)
    }
    pub fn decision_is_runtime(&self, path: &str) -> bool {
        self.contains(path) || self.choices.contains_key(path)
    }
    pub(crate) fn new() -> Self {
        Self {
            records: BTreeMap::new(),
            choices: BTreeMap::new(),
            owner: Owner::Unsealed,
            logical_bytes: 0,
            failed: false,
            policy: RuntimeGuardedPolicy::LinearFullRepeats,
            work_budget: super::super::GrammarAnalysisWorkBudget::LEGACY_10K,
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            analysis_initial_work: None,
            #[cfg(test)]
            disable_completed_reuse: false,
            #[cfg(test)]
            disable_inventory: false,
        }
    }
    /// Compile the prediction program for a deeper combined call stack than
    /// `DEFAULT_CALL_DEPTH`, for grammars whose nesting needs it; at most
    /// `MAX_CALL_DEPTH`.
    pub fn with_call_depth(mut self, depth: usize) -> Result<Self, String> {
        if depth == 0 || depth > crate::grammar::MAX_CALL_DEPTH {
            return Err(format!(
                "Prediction call depth must be between 1 and {}",
                crate::grammar::MAX_CALL_DEPTH
            ));
        }
        self.call_depth = u16::try_from(depth).map_err(|_| "Prediction call depth overflow")?;
        Ok(self)
    }
    pub fn call_depth(&self) -> usize {
        usize::from(self.call_depth)
    }
    pub(crate) fn work_budget(&self) -> super::super::GrammarAnalysisWorkBudget {
        self.work_budget
    }
    pub(in crate::grammar::parser_prediction) fn begin_analysis_work(&mut self, initial: usize) {
        self.analysis_initial_work = Some(initial);
    }
    pub(crate) fn new_with_policy_and_budget(
        policy: RuntimeGuardedPolicy,
        budget: super::super::GrammarAnalysisWorkBudget,
    ) -> Self {
        let mut admission = Self::new_with_policy(policy);
        admission.work_budget = budget;
        admission
    }
    // Internal historical compatibility facade used by explicit 10k controls.
    pub(crate) fn new_with_policy(policy: RuntimeGuardedPolicy) -> Self {
        let mut admission = Self::new();
        admission.policy = policy;
        admission
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoiceBinding {
    owner: String,
    source: SourceId,
    declaration: Range<usize>,
    occurrence: Range<usize>,
    graph: [usize; 2],
    entries: Vec<usize>,
    guards: Vec<[usize; 3]>,
    proof: ChoiceProof,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum ChoiceProof {
    AsymmetricPair,
    ConditionalMultiway,
}

// Handwritten host-cache boundary (language-pack step 3). Not generated code.

/// How much a host-cache load re-checks before binding a saved admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAdmissionLoadMode {
    /// Production host cache: the exact record/choice key set must equal the
    /// `Requirement` decision set stored in the same saved package, then the
    /// owner is bound. Record and choice contents, and the `Requirement`
    /// itself, are trusted as the host's own compiler output.
    HostCache,
    /// Tests and CI: `HostCache`, then the seal-time proof checks are re-run
    /// on the loaded program: every record is rebound and its proof
    /// re-verified (`validate_decision_with_remaining`), and every choice is
    /// rebound and its stored proof kind's witness re-verified
    /// (`verify_multiway_scopes`: the old source/scope witness for an
    /// asymmetric pair, the input-checked completion scope for a conditional
    /// multiway). The `Requirement` decision set comes from the same saved
    /// package and is NOT re-derived, so a decision removed from both the
    /// `Requirement` and the admission is not detected; only recompilation
    /// proves the decision set complete. Records, choices and the final owner
    /// check each spend their own fresh pool of the saved work budget,
    /// mirroring the separate compile-time pools.
    Revalidate,
}

/// The persisted payload of one compiler-bound runtime admission.
///
/// `RuntimeGuardedAdmission::save` produces it; all fields are private, but
/// deserialization can build one from any bytes. That grants nothing: it
/// becomes an admission only through the compiler's
/// `CompiledRuntimeGrammar::load_host_cache`, which checks it against the
/// loaded program under the trusted-host contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedRuntimeAdmission {
    policy: RuntimeGuardedPolicy,
    call_depth: u16,
    logical_bytes: usize,
    work_budget: usize,
    records: BTreeMap<String, Binding>,
    choices: BTreeMap<String, ChoiceBinding>,
}
impl SavedRuntimeAdmission {
    pub fn policy(&self) -> RuntimeGuardedPolicy {
        self.policy
    }
    pub fn call_depth(&self) -> usize {
        usize::from(self.call_depth)
    }
    pub fn decision_count(&self) -> usize {
        self.records.len().saturating_add(self.choices.len())
    }
    #[cfg(test)]
    pub(crate) fn test_remove_record(&mut self) -> Option<String> {
        let key = self.records.keys().next_back()?.clone();
        self.records.remove(&key);
        Some(key)
    }
    #[cfg(test)]
    pub(crate) fn test_add_record(&mut self, key: &str) -> bool {
        let Some(record) = self.records.values().next().cloned() else {
            return false;
        };
        self.records.insert(key.into(), record).is_none()
    }
    #[cfg(test)]
    pub(crate) fn test_rename_record(&mut self, key: &str) -> bool {
        let Some(old) = self.records.keys().next().cloned() else {
            return false;
        };
        let Some(record) = self.records.remove(&old) else {
            return false;
        };
        self.records.insert(key.into(), record).is_none()
    }
    #[cfg(test)]
    pub(crate) fn test_rename_choice(&mut self, key: &str) -> bool {
        let Some(old) = self.choices.keys().next().cloned() else {
            return false;
        };
        let Some(choice) = self.choices.remove(&old) else {
            return false;
        };
        self.choices.insert(key.into(), choice).is_none()
    }
    #[cfg(test)]
    pub(crate) fn test_set_policy(&mut self, policy: RuntimeGuardedPolicy) {
        self.policy = policy;
    }
    /// Rewrite the policy AND every record proof class, so the load reaches
    /// its later checks. Optionally drop the choices.
    #[cfg(test)]
    pub(crate) fn test_set_consistent_policy(
        &mut self,
        policy: RuntimeGuardedPolicy,
        keep_choices: bool,
    ) {
        self.policy = policy;
        for record in self.records.values_mut() {
            record.proof_class = policy;
        }
        if !keep_choices {
            self.choices.clear();
        }
    }
    /// Swap the stored proof kind of every choice.
    #[cfg(test)]
    pub(crate) fn test_flip_choice_proofs(&mut self) -> bool {
        for choice in self.choices.values_mut() {
            choice.proof = match choice.proof {
                ChoiceProof::AsymmetricPair => ChoiceProof::ConditionalMultiway,
                ChoiceProof::ConditionalMultiway => ChoiceProof::AsymmetricPair,
            };
        }
        !self.choices.is_empty()
    }
    #[cfg(test)]
    pub(crate) fn test_shift_record_graph(&mut self) -> bool {
        let Some(record) = self.records.values_mut().next() else {
            return false;
        };
        record.graph[0] = record.graph[0].wrapping_add(1);
        true
    }
    #[cfg(test)]
    pub(crate) fn choice_count(&self) -> usize {
        self.choices.len()
    }
    #[cfg(test)]
    pub(crate) fn record_count(&self) -> usize {
        self.records.len()
    }
}
impl RuntimeGuardedAdmission {
    /// Persist this admission for a host-built language pack. Only a sealed,
    /// owner-bound, non-failed admission with at least one decision is saved;
    /// owner addresses are never persisted (the load rebinds them).
    pub fn save(&self) -> Result<SavedRuntimeAdmission, String> {
        if self.failed {
            return Err("A failed runtime admission cannot be saved".into());
        }
        if !matches!(self.owner, Owner::Bound { .. }) {
            return Err("Only a sealed, owner-bound runtime admission can be saved".into());
        }
        if self.records.is_empty() && self.choices.is_empty() {
            return Err("A runtime admission without decisions cannot be saved".into());
        }
        Ok(SavedRuntimeAdmission {
            policy: self.policy,
            call_depth: self.call_depth,
            logical_bytes: self.logical_bytes,
            work_budget: self.work_budget.units(),
            records: self.records.clone(),
            choices: self.choices.clone(),
        })
    }
}
mod implementation {
    use super::super::{Edge, ParserPrediction, analysis_charge};
    use super::*;
    use crate::grammar::Expr;
    use crate::grammar::Grammar;
    use crate::grammar::Kind;
    use crate::grammar::Profile;
    const HEADER: usize = 256;
    const RECORD: usize = 512;
    const LIMIT: usize = 1_048_576;
    const MAX_KEYS: usize = 4096;
    // New direct heap payload plus the owning handle; no transitive buffer clone.
    const OWNER_BYTES: usize = intersect_model::logical_size!(crate::package::Package, 1688)
        + intersect_model::logical_size!(Box<crate::package::Package>, 8);
    // Fixed prepaid operations, detailed in owner-accounting.md. No key walk or
    // variable inspection is permitted in final publication.
    const OWNER_PUBLICATION_WORK: usize = 20;
    const _: () = {
        assert!(OWNER_BYTES > 0 && OWNER_BYTES <= LIMIT);
        assert!(std::mem::size_of::<RuntimeGuardedAdmission>() <= HEADER);
        assert!(
            std::mem::size_of::<ChoiceBinding>() + 2 * std::mem::size_of::<String>() + 128
                <= RECORD
        );
        assert!(std::mem::size_of::<Owner>() <= 4 * std::mem::size_of::<usize>());
    };

    fn pay(work: &mut usize) -> Result<(), String> {
        analysis_charge(work)
    }
    fn admit(total: &mut usize, bytes: usize, work: &mut usize) -> Result<(), String> {
        let next = total
            .checked_add(bytes)
            .filter(|n| *n <= LIMIT)
            .ok_or("Runtime obligation retention bound exceeded")?;
        pay(work)?;
        for _ in 0..bytes.div_ceil(64) {
            pay(work)?;
        }
        *total = next;
        Ok(())
    }
    include!("runtime_guarded_choices.rs");

    fn whole_full<'g>(expr: &'g Expr, work: &mut usize) -> Result<Option<&'g Expr>, String> {
        if expr.cardinality != '*' || expr.first_set_predicated {
            return Ok(None);
        }
        let mut full = expr;
        for depth in 0..=64 {
            pay(work)?;
            let Kind::Sequence(items) = &full.kind else {
                break;
            };
            let [child] = items.as_slice() else {
                return Ok(None);
            };
            pay(work)?;
            if depth == 64 {
                return Err("Grammar syntax recursion budget exceeded".into());
            }
            if child.cardinality != '1' || child.first_set_predicated {
                return Ok(None);
            }
            let (Some(parent), Some(inner)) = (&full.original_range, &child.original_range) else {
                return Ok(None);
            };
            pay(work)?;
            if parent.start >= parent.end
                || inner.start >= inner.end
                || inner.start < parent.start
                || inner.end > parent.end
            {
                return Err("Runtime Full group source interval differs from parent".into());
            }
            full = child;
        }
        Ok(matches!(full.kind, Kind::SyntacticPredicate(_)).then_some(full))
    }
    // A First commitment is not a Full probe. Only an exact direct parser Call
    // under transparent single-child groups is admitted by this new Strict slice.
    fn whole_first<'g>(
        expr: &'g Expr,
        work: &mut usize,
    ) -> Result<Option<(&'g Expr, &'g str)>, String> {
        if !matches!(expr.cardinality, '*' | '+') {
            return Ok(None);
        }
        let mut first = expr;
        for depth in 0..=64 {
            pay(work)?;
            if first.first_set_predicated {
                return Ok(match &first.kind {
                    Kind::Call(name) => Some((first, name.as_str())),
                    _ => None,
                });
            }
            let Kind::Sequence(items) = &first.kind else {
                return Ok(None);
            };
            let [child] = items.as_slice() else {
                return Ok(None);
            };
            pay(work)?;
            if depth == 64 {
                return Err("Grammar syntax recursion budget exceeded".into());
            }
            if child.cardinality != '1' {
                return Ok(None);
            }
            let (Some(parent), Some(inner)) = (&first.original_range, &child.original_range) else {
                return Ok(None);
            };
            pay(work)?;
            if parent.start >= parent.end
                || inner.start >= inner.end
                || inner.start < parent.start
                || inner.end > parent.end
            {
                return Err("Runtime First group source interval differs from parent".into());
            }
            first = child;
        }
        Ok(None)
    }
    fn callee(expr: &Expr) -> Option<&str> {
        let Kind::SyntacticPredicate(value) = &expr.kind else {
            return None;
        };
        let value = match &value.kind {
            Kind::Assign {
                value: assigned, ..
            } if value.cardinality == '1' && !value.first_set_predicated => assigned.as_ref(),
            _ => value.as_ref(),
        };
        if value.cardinality != '1' || value.first_set_predicated {
            return None;
        }
        match &value.kind {
            Kind::Call(name) => Some(name),
            _ => None,
        }
    }
    fn source_expr<'g>(
        grammar: &'g Grammar,
        path: &str,
        work: &mut usize,
    ) -> Result<(&'g str, &'g Expr), String> {
        pay(work)?;
        let base = path
            .strip_suffix("/repeat")
            .ok_or("Runtime obligation is not a repeat")?;
        let mut parts = base.split('/');
        if parts.next() != Some("rule") {
            return Err("Runtime obligation has no source owner".into());
        }
        let owner = parts.next().ok_or("Runtime obligation has no rule name")?;
        pay(work)?;
        let (name, rule) = grammar
            .rules
            .get_key_value(owner)
            .ok_or("Missing runtime obligation owner")?;
        let mut expr = &rule.body;
        for part in parts {
            pay(work)?;
            expr = match (part, &expr.kind) {
                (
                    "value",
                    Kind::Assign { value, .. }
                    | Kind::BooleanAssign { value, .. }
                    | Kind::SyntacticPredicate(value),
                ) => value,
                (_, Kind::Sequence(items)) if part.starts_with('s') => items
                    .get(
                        part[1..]
                            .parse::<usize>()
                            .map_err(|_| "Invalid runtime source sequence")?,
                    )
                    .ok_or("Runtime source sequence escapes expression")?,
                (_, Kind::Alternative(items)) if part.starts_with('a') => items
                    .get(
                        part[1..]
                            .parse::<usize>()
                            .map_err(|_| "Invalid runtime source alternative")?,
                    )
                    .ok_or("Runtime source alternative escapes expression")?,
                _ => return Err("Runtime obligation path differs from source expression".into()),
            };
        }
        pay(work)?;
        if expr.prediction_path.as_deref() != Some(base) {
            return Err("Runtime obligation differs from actual bound source path".into());
        }
        Ok((name, expr))
    }
    // This slice intentionally has NO nested completion commitments: only one
    // productive linear keyword/action/epsilon callee, with its real rule stop.
    fn linear(
        program: &ParserPrediction,
        start: usize,
        stop: usize,
        work: &mut usize,
    ) -> Result<bool, String> {
        let mut state = start;
        let mut productive = false;
        for _ in 0..MAX_KEYS {
            pay(work)?;
            let node = program
                .states
                .get(state)
                .ok_or("Dangling linear runtime callee")?;
            if state == stop {
                return Ok(node.rule_stop && productive && !node.accepting);
            }
            if node.rule_stop || node.accepting {
                return Ok(false);
            }
            pay(work)?;
            state = match node.edges.as_slice() {
                [Edge::Epsilon(next) | Edge::Action(next)] => *next,
                [Edge::Token(super::super::PredictionSymbol::Keyword(value), next)]
                    if !value.is_empty() =>
                {
                    productive = true;
                    *next
                }
                _ => return Ok(false),
            };
        }
        Ok(false)
    }
    use super::super::super::admission_observation::{
        FirstReuseEvent as ReuseEvent, FirstReuseWindow as ReuseWindow,
    };

    // Reuse the existing bounded first-returned-error observer. No sidecar is
    // retained and no semantic work/debit/result is added, skipped or replaced.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn observe_candidate<T>(
        result: Result<T, String>,
        grammar: &Grammar,
        program: &ParserPrediction,
        observation: Option<super::super::super::AnalysisObservation<'_>>,
        scope: (
            super::super::super::admission_observation::Phase,
            Option<&str>,
        ),
        work: usize,
    ) -> Result<T, String> {
        observe_candidate_with_initial(
            result,
            grammar,
            program,
            observation,
            scope,
            work,
            super::super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units(),
        )
    }
    #[allow(clippy::too_many_arguments)] // Same observer plus captured initial pool.
    fn observe_candidate_with_initial<T>(
        result: Result<T, String>,
        grammar: &Grammar,
        program: &ParserPrediction,
        observation: Option<super::super::super::AnalysisObservation<'_>>,
        (phase, path): (
            super::super::super::admission_observation::Phase,
            Option<&str>,
        ),
        work: usize,
        initial_work: usize,
    ) -> Result<T, String> {
        let Some(observation) = observation else {
            return result;
        };
        let owner = path
            .and_then(|path| path.strip_prefix("rule/"))
            .and_then(|path| path.split('/').next());
        super::super::observe_parser_phase(
            result,
            grammar,
            Some(observation),
            phase,
            (owner, path),
            (initial_work.checked_sub(work), Some(program.states.len())),
        )
    }
    fn binding(
        grammar: &Grammar,
        program: &ParserPrediction,
        path: &str,
        work: &mut usize,
        logical: &mut usize,
        policy: RuntimeGuardedPolicy,
        observation: Option<super::super::super::AnalysisObservation<'_>>,
    ) -> Result<Option<Binding>, String> {
        binding_with_completed(
            grammar,
            program,
            path,
            work,
            logical,
            policy,
            (observation, None),
        )
    }
    fn binding_with_completed<'g>(
        grammar: &'g Grammar,
        program: &ParserPrediction,
        path: &str,
        work: &mut usize,
        logical: &mut usize,
        policy: RuntimeGuardedPolicy,
        (observation, completed): (
            Option<super::super::super::AnalysisObservation<'_>>,
            Option<&mut super::super::super::first_sets::completed_reuse::Completed<'g>>,
        ),
    ) -> Result<Option<Binding>, String> {
        binding_with_storage::<std::collections::BTreeSet<String>>(
            grammar,
            program,
            path,
            work,
            logical,
            policy,
            (observation, completed, None),
        )
    }
    fn binding_with_storage<'g, S: super::super::super::first_sets::symbols::Symbols>(
        grammar: &'g Grammar,
        program: &ParserPrediction,
        path: &str,
        work: &mut usize,
        logical: &mut usize,
        policy: RuntimeGuardedPolicy,
        context: (
            Option<super::super::super::AnalysisObservation<'_>>,
            Option<&mut super::super::super::first_sets::completed_reuse::Storage<'g, S>>,
            Option<&mut super::super::RepeatCertificates>,
        ),
    ) -> Result<Option<Binding>, String> {
        // Legacy direct-binding controls model a consumed prefix of10k.
        // Production collection passes its captured parser initial explicitly.
        let initial_work = super::super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units();
        let (observation, completed, certificates) = context;
        binding_with_prepared_guard_facts(
            grammar,
            program,
            path,
            work,
            logical,
            policy,
            (observation, completed, certificates, initial_work),
            None,
        )
    }
    #[allow(clippy::too_many_arguments)] // Exact original binder plus one borrowed optional result.
    fn binding_with_prepared_guard_facts<
        'g,
        S: super::super::super::first_sets::symbols::Symbols,
    >(
        grammar: &'g Grammar,
        program: &ParserPrediction,
        path: &str,
        work: &mut usize,
        logical: &mut usize,
        policy: RuntimeGuardedPolicy,
        (observation, completed, certificates, initial_work): (
            Option<super::super::super::AnalysisObservation<'_>>,
            Option<&mut super::super::super::first_sets::completed_reuse::Storage<'g, S>>,
            Option<&mut super::super::RepeatCertificates>,
            usize,
        ),
        prepared: Option<&super::super::guard_facts::GuardFacts<'_>>,
    ) -> Result<Option<Binding>, String> {
        let (owner, expr) = source_expr(grammar, path, work)?;
        let (full, operand, name, first) = if let Some(full) = whole_full(expr, work)? {
            let Some(name) = callee(full) else {
                return Ok(None);
            };
            let Kind::SyntacticPredicate(operand) = &full.kind else {
                return Ok(None);
            };
            (full, operand.as_ref(), name, false)
        } else if matches!(
            policy,
            RuntimeGuardedPolicy::StrictCompletionScopes
                | RuntimeGuardedPolicy::InputCheckedCompletionScopes
        ) {
            let Some((first, name)) = whole_first(expr, work)? else {
                return Ok(None);
            };
            (first, first, name, true)
        } else {
            return Ok(None);
        };
        pay(work)?;
        let rule = grammar
            .rules
            .get(name)
            .ok_or("Runtime guarded operand is not a parser rule")?;
        let hidden = grammar.hidden.as_deref().unwrap_or(&[]);
        if rule
            .hidden
            .as_deref()
            .is_some_and(|actual| actual != hidden)
            || grammar
                .rules
                .get(owner)
                .and_then(|r| r.hidden.as_deref())
                .is_some_and(|actual| actual != hidden)
        {
            return Ok(None);
        }
        pay(work)?;
        let source = grammar
            .declaration_sources
            .get(owner)
            .ok_or("Runtime obligation requires original owner source")?;
        pay(work)?;
        let target_source = grammar
            .declaration_sources
            .get(name)
            .ok_or("Runtime obligation requires original callee source")?;
        let Some(occurrence) = expr.original_range.as_ref() else {
            return Ok(None);
        };
        let Some(full_range) = full.original_range.as_ref() else {
            return Ok(None);
        };
        let Some(operand_range) = operand.original_range.as_ref() else {
            return Ok(None);
        };
        pay(work)?;
        if full_range.start < occurrence.start
            || full_range.end > occurrence.end
            || operand_range.start < full_range.start
            || operand_range.end > full_range.end
        {
            return Err("Runtime Full operand source interval differs from occurrence".into());
        }
        for range in [occurrence, full_range, operand_range] {
            pay(work)?;
            if range.start > range.end || range.start < source.start || range.end > source.end {
                return Err("Runtime obligation source interval differs from declaration".into());
            }
        }
        pay(work)?;
        let decision = program
            .decisions
            .get(path)
            .ok_or("Missing runtime repeat decision")?;
        let [body, end] = decision.entries.as_slice() else {
            return Ok(None);
        };
        if decision.exit != Some(1) {
            return Ok(None);
        }
        // Pay bounded temporary key storage before formatting/lookup.
        let recovery_bytes = path
            .len()
            .checked_add(3)
            .ok_or("Runtime recovery key size overflow")?;
        let mut temporary = 0;
        admit(&mut temporary, recovery_bytes, work)?;
        let recovery = format!(
            "{}/recovery",
            path.strip_suffix("/repeat").ok_or("Invalid repeat path")?
        );
        let occurrence_graph = program
            .decisions
            .get(&recovery)
            .ok_or("Missing exact runtime occurrence")?;
        let [start, actual_end] = occurrence_graph.entries.as_slice() else {
            return Ok(None);
        };
        if occurrence_graph.exit.is_some() || actual_end != end {
            return Ok(None);
        }
        pay(work)?;
        let plus = first && expr.cardinality == '+';
        let entry = &program
            .states
            .get(*start)
            .ok_or("Dangling runtime repeat entry")?
            .edges;
        if if plus {
            !matches!(entry.as_slice(), [Edge::Epsilon(b)] if b == body)
        } else {
            !matches!(entry.as_slice(), [Edge::Epsilon(b), Edge::Epsilon(e)] if b == body && e == end)
        } {
            return Ok(None);
        }
        pay(work)?;
        let guard = &program
            .states
            .get(*body)
            .ok_or("Dangling runtime guard entry")?
            .edges;
        let (target, stop) = if first {
            let [
                Edge::First {
                    path: actual_path,
                    target,
                },
            ] = guard.as_slice()
            else {
                return Ok(None);
            };
            let Some(source_path) = full.prediction_path.as_deref() else {
                return Ok(None);
            };
            pay(work)?;
            for _ in 0..actual_path.len().max(source_path.len()).div_ceil(64) {
                pay(work)?;
            }
            if actual_path != source_path {
                return Ok(None);
            }
            pay(work)?;
            if program
                .first_guards
                .get(actual_path)
                .is_none_or(|symbols| symbols.is_empty())
            {
                return Ok(None);
            }
            pay(work)?;
            let [Edge::Call { follow, .. }] = program
                .states
                .get(*target)
                .ok_or("Dangling runtime First operand")?
                .edges
                .as_slice()
            else {
                return Ok(None);
            };
            (*target, *follow)
        } else {
            let [Edge::Full { target, stop }] = guard.as_slice() else {
                return Ok(None);
            };
            (*target, *stop)
        };
        pay(work)?;
        let completed_edges = &program
            .states
            .get(stop)
            .ok_or("Dangling runtime operand stop")?
            .edges;
        if if plus {
            !matches!(completed_edges.as_slice(), [Edge::Epsilon(b), Edge::Epsilon(e)] if b == body && e == end)
        } else {
            !matches!(completed_edges.as_slice(), [Edge::Epsilon(back)] if back == start)
        } {
            return Ok(None);
        }
        pay(work)?;
        let &(callee_start, callee_stop) = program
            .rules
            .get(name)
            .ok_or("Missing runtime callee graph")?;
        pay(work)?;
        if !matches!(program.states.get(target).ok_or("Dangling runtime operand")?.edges.as_slice(),
            [Edge::Call { target: actual, follow }] if *actual == callee_start && *follow == stop)
        {
            return Ok(None);
        }
        match policy {
            RuntimeGuardedPolicy::LinearFullRepeats => {
                if !linear(program, callee_start, callee_stop, work)? {
                    return Ok(None);
                }
            }
            RuntimeGuardedPolicy::StrictCompletionScopes
            | RuntimeGuardedPolicy::InputCheckedCompletionScopes => {
                // Existing Full-star certificates retain their exact verifier.
                // First can reuse only an already-completed scalar for this
                // exact borrowed operand; all mandatory-plus/final proofs stay.
                let certified = if first {
                    match prepared {
                        Some(prepared) => observe_candidate_with_initial(
                            prepared.nonnullable(grammar, operand, work), grammar, program,
                            observation,
                            (super::super::super::admission_observation::Phase::RuntimeCandidateNonnullableFirst, Some(path)),
                            *work, initial_work,
                        )?,
                        None => false,
                    }
                } else {
                    let proof = match certificates {
                        Some(certificates) => program.runtime_repeat_certificate_is_nonnullable(
                            path,
                            decision,
                            grammar,
                            certificates,
                            work,
                        ),
                        None => Ok(false),
                    };
                    observe_candidate_with_initial(
                        proof, grammar, program, observation,
                        (super::super::super::admission_observation::Phase::RuntimeCandidateNonnullableFirst, Some(path)),
                        *work, initial_work,
                    )?
                };
                if !certified {
                    // Reuse canonical FIRST on this exact unquantified operand.
                    // This spends the SAME ParserProgram/deferred work reference;
                    // strict commitment coverage is not a productivity proof.
                    pay(work)?;
                    // The quantifier belongs to the occurrence, not its direct
                    // operand. Grouped First calls are already unquantified. A flag
                    // directly on Call* or Call+ needs a small prepaid temporary,
                    // then the SAME canonical FIRST worker/ledger; never a rule-body
                    // shortcut that changes call-depth or unknown-callee behavior.
                    let unquantified;
                    let first_operand = if first && operand.cardinality != '1' {
                        let bytes = intersect_model::logical_size!(Expr, 112)
                            .checked_add(name.len())
                            .and_then(|n| {
                                n.checked_add(
                                    operand.prediction_path.as_ref().map_or(0, String::len),
                                )
                            })
                            .ok_or("Runtime First operand storage overflow")?;
                        admit(logical, bytes, work)?;
                        let mut value = operand.clone();
                        value.cardinality = '1';
                        unquantified = value;
                        &unquantified
                    } else {
                        operand
                    };
                    let trace = observation.map(|context| context.observation);
                    let first_before = *work;
                    if let Some(trace) = trace {
                        trace.reuse_event(ReuseEvent::FirstQuery, 1);
                    }
                    let first = match completed {
                        Some(completed) => {
                            super::super::super::first_sets::analyze_with_storage_observed(
                                first_operand,
                                grammar,
                                0,
                                work,
                                completed,
                                trace,
                            )
                        }
                        None => super::super::super::first_sets::analyze_remaining_storage::<S>(
                            first_operand,
                            grammar,
                            0,
                            work,
                            trace,
                        ),
                    }
                    .map(|summary| summary.1);
                    if let Some(trace) = trace {
                        trace.reuse_window(ReuseWindow::First, first_before, *work);
                    }
                    if observe_candidate_with_initial(
                        first, grammar, program, observation,
                        (super::super::super::admission_observation::Phase::RuntimeCandidateNonnullableFirst, Some(path)),
                        *work, initial_work,
                    )?
                    {
                        return Err(format!(
                            "Strict runtime scope requires a nonnullable operand at {path}"
                        ));
                    }
                }
            }
        }
        // All owned identity/string/range/graph fields are prepaid before copy.
        let bytes = RECORD
            .checked_add(path.len())
            .and_then(|n| n.checked_add(owner.len()))
            .and_then(|n| n.checked_add(name.len()))
            .and_then(|n| n.checked_add(source.source.uri.len()))
            .and_then(|n| n.checked_add(source.source.sha256.len()))
            .and_then(|n| n.checked_add(target_source.source.uri.len()))
            .and_then(|n| n.checked_add(target_source.source.sha256.len()))
            .ok_or("Runtime obligation record size overflow")?;
        // Temporary revalidation records are charged too; no free clone path.
        admit(logical, bytes, work)?;
        Ok(Some(Binding {
            proof_class: policy,
            owner: owner.into(),
            callee: name.into(),
            source: source.source.clone(),
            callee_source: target_source.source.clone(),
            declaration: source.start..source.end,
            occurrence: occurrence.clone(),
            full: full_range.clone(),
            operand: operand_range.clone(),
            graph: [*start, *body, target, stop, *end, callee_start, callee_stop],
        }))
    }
    fn verify_strict_record(
        program: &ParserPrediction,
        path: &str,
        record: &Binding,
        work: &mut usize,
    ) -> Result<(), String> {
        pay(work)?;
        if record.proof_class != RuntimeGuardedPolicy::StrictCompletionScopes {
            return Err("Strict runtime record has a different required proof class".into());
        }
        // Owned scope is dropped before publication. No self-reference, copied
        // decision authority or extra caller allowance is retained.
        drop(
            program
                .verify_completion_scope_with_verification(
                    record.graph[2],
                    record.graph[3],
                    work,
                    super::super::CompletionVerification::BoundedGuardOverapproximation,
                )
                .map_err(|error| {
                    format!("Strict runtime completion scope unsupported at {path}: {error}")
                })?,
        );
        Ok(())
    }
    fn verify_input_checked_record(
        program: &ParserPrediction,
        path: &str,
        record: &Binding,
        work: &mut usize,
    ) -> Result<(), String> {
        pay(work)?;
        if record.proof_class != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
            return Err("Input-checked runtime record has a different proof class".into());
        }
        drop(
            program
                .verify_completion_scope_with_verification(
                    record.graph[2],
                    record.graph[3],
                    work,
                    super::super::CompletionVerification::InputCheckedCompletion,
                )
                .map_err(|error| {
                    format!("Input-checked runtime completion scope unsupported at {path}: {error}")
                })?,
        );
        Ok(())
    }

    // Private fresh-builder seal only. Binding already checked the exact source,
    // hidden policy, original intervals and nonnullable operand in collect. The
    // compiler changes only final recursive metadata between collect and seal.
    fn native_seal_pair(
        program: &ParserPrediction,
        record: &Binding,
        work: &mut usize,
    ) -> Result<(usize, usize), String> {
        pay(work)?;
        if record.proof_class != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
            return Err("Input-checked runtime record has a different proof class".into());
        }
        let [start, body, target, stop, end, callee_start, callee_stop] = record.graph;
        for index in [start, body, target, stop, end, callee_start, callee_stop] {
            pay(work)?;
            let state = program
                .states
                .get(index)
                .ok_or("Dangling native sealed runtime graph")?;
            if state.accepting || (state.rule_stop && index != callee_stop && index != end) {
                return Err(
                    "Native sealed runtime flags differ from their bound occurrence".into(),
                );
            }
        }
        pay(work)?;
        if !program.states[callee_stop].rule_stop
            || !matches!(program.states[target].edges.as_slice(),
                [Edge::Call{target:actual,follow}] if *actual==callee_start && *follow==stop)
        {
            return Err("Native sealed runtime Call differs from its bound occurrence".into());
        }
        pay(work)?;
        let first = match program.states[body].edges.as_slice() {
            [Edge::First { target: actual, .. }] if *actual == target => true,
            [
                Edge::Full {
                    target: actual,
                    stop: actual_stop,
                },
            ] if *actual == target && *actual_stop == stop => false,
            _ => {
                return Err("Native sealed runtime guard differs from its bound occurrence".into());
            }
        };
        pay(work)?;
        let plus = first
            && matches!(program.states[start].edges.as_slice(),[Edge::Epsilon(b)] if *b==body);
        pay(work)?;
        let exact = if plus {
            matches!(program.states[stop].edges.as_slice(),[Edge::Epsilon(b),Edge::Epsilon(e)] if *b==body&&*e==end)
        } else {
            matches!(program.states[start].edges.as_slice(),[Edge::Epsilon(b),Edge::Epsilon(e)] if *b==body&&*e==end)
                && matches!(program.states[stop].edges.as_slice(),[Edge::Epsilon(back)] if *back==start)
        };
        if !exact {
            return Err("Native sealed runtime loop differs from its bound occurrence".into());
        }
        Ok((callee_start, callee_stop))
    }

    fn verify_native_input_checked_records(
        program: &ParserPrediction,
        records: &BTreeMap<String, Binding>,
        logical: &mut usize,
        work: &mut usize,
    ) -> Result<(), String> {
        use crate::grammar::follow::repeat_certificates::lemma_lookup_work;
        if records.len() < 2 {
            for (path, record) in records {
                verify_input_checked_record(program, path, record, work)?;
            }
            return Ok(());
        }
        // Fixed target-independent conservative logical sizes; no retained graph
        // map or scope is copied. The existing proof set is moved, then dropped.
        const PAIR: usize = 16;
        const VECTOR: usize = 24;
        const {
            assert!(std::mem::size_of::<(usize, usize)>() <= PAIR);
            assert!(std::mem::size_of::<Vec<(usize, usize)>>() <= VECTOR);
        }
        admit(logical, VECTOR, work)?;
        let mut completed: Vec<(usize, usize)> = Vec::new();
        for (path, record) in records {
            let pair = native_seal_pair(program, record, work)?;
            let mut hit = false;
            for &old in &completed {
                pay(work)?;
                if old == pair {
                    hit = true;
                    break;
                }
            }
            if hit {
                continue;
            }
            let mut count = 0;
            for other in records.values() {
                pay(work)?;
                if (other.graph[5], other.graph[6]) == pair {
                    count += 1;
                    if count == 2 {
                        break;
                    }
                }
            }
            if count < 2 {
                verify_input_checked_record(program, path, record, work)?;
                continue;
            }
            // Cold proof is EXACTLY the original occurrence wrapper, including
            // the original stop, graph walk, static proofs and final checks.
            let (scope, reached) = program
                .verify_completion_scope_with_reach(
                    record.graph[2],
                    record.graph[3],
                    work,
                    super::super::CompletionVerification::InputCheckedCompletion,
                    None,
                )
                .map_err(|error| {
                    format!("Input-checked runtime completion scope unsupported at {path}: {error}")
                })?;
            drop(scope);
            let mut callback = false;
            for other in records.values() {
                pay(work)?;
                if (other.graph[5], other.graph[6]) != pair {
                    continue;
                }
                // Canonical builder cross-rule edges ONLY call the rule start;
                // every group owner is checked, not only this representative.
                for _ in 0..lemma_lookup_work(program.rules.len(), other.owner.len())? {
                    pay(work)?;
                }
                let &(owner_start, _) = program
                    .rules
                    .get(&other.owner)
                    .ok_or("Missing native sealed runtime owner")?;
                for _ in 0..lemma_lookup_work(reached.len(), 0)? {
                    pay(work)?;
                }
                if reached.contains(&owner_start) {
                    callback = true;
                }
            }
            drop(reached);
            if callback {
                continue;
            } // proof was completed; no refund or retry
            if completed.capacity() == 0 {
                let bytes = records
                    .len()
                    .checked_mul(PAIR)
                    .ok_or("Native seal pair storage overflow")?;
                admit(logical, bytes, work)?;
                completed
                    .try_reserve_exact(records.len())
                    .map_err(|_| "Native seal pair allocation failed")?;
            }
            pay(work)?;
            completed.push(pair);
        }
        Ok(())
    }

    impl RuntimeGuardedAdmission {
        pub(in crate::grammar::parser_prediction) fn inventory_enabled(&self) -> bool {
            let enabled = matches!(
                self.policy,
                RuntimeGuardedPolicy::StrictCompletionScopes
                    | RuntimeGuardedPolicy::InputCheckedCompletionScopes
            );
            #[cfg(test)]
            let enabled = enabled && !self.disable_inventory;
            enabled
        }
        #[cfg(test)]
        pub(in crate::grammar::parser_prediction) fn inventory_test_configure(
            &mut self,
            disabled: bool,
            retained: usize,
        ) {
            self.disable_inventory = disabled;
            self.logical_bytes = retained;
        }
        #[cfg(test)]
        pub(in crate::grammar::parser_prediction) fn inventory_test_state(&self) -> (usize, bool) {
            (self.logical_bytes, self.failed)
        }
        // The original nonnullable retention method already guards this SAME
        // cumulative companion ledger; no additional allowance or refund.
        pub(in crate::grammar::parser_prediction) fn retain_inventory(
            &mut self,
            bytes: usize,
            work: &mut usize,
        ) -> Result<(), String> {
            self.retain_nonnullable_lemma(bytes, work)
        }
        pub(in crate::grammar::parser_prediction) fn invalidate_inventory(&mut self) {
            self.failed = true;
        }
        // Retain only already-computed FOLLOW occurrence facts in the SAME
        // companion ledger. All work is debited from the caller's FOLLOW pool;
        // default/Linear construction creates no such records or allowance.
        pub(in crate::grammar) fn retain_nonnullable_lemma(
            &mut self,
            bytes: usize,
            work: &mut usize,
        ) -> Result<(), String> {
            let result = if self.failed
                || !matches!(self.owner, Owner::Unsealed)
                || !self.records.is_empty()
            {
                Err("Runtime admission companion is already used or failed".into())
            } else {
                admit(&mut self.logical_bytes, bytes, work)
            };
            if result.is_err() {
                self.failed = true;
            }
            result
        }
        #[cfg(test)]
        pub(in crate::grammar) fn nonnullable_retention_test_fill(&mut self) {
            self.logical_bytes = LIMIT;
        }
        #[cfg(test)]
        pub(in crate::grammar) fn nonnullable_retention_test_state(&self) -> (usize, bool) {
            (self.logical_bytes, self.failed)
        }
        fn reserve_owner(
            &mut self,
            program: &ParserPrediction,
            work: &mut usize,
        ) -> Result<(), String> {
            let result = (|| {
                pay(work)?;
                if self.failed || !matches!(self.owner, Owner::Unsealed) {
                    return Err("Runtime owner reservation is unavailable".into());
                }
                // Token fields live in the already-paid HEADER. The direct new
                // owner allocation/handle is additional, cumulatively bounded.
                admit(&mut self.logical_bytes, OWNER_BYTES, work)?;
                for _ in 0..OWNER_PUBLICATION_WORK {
                    pay(work)?;
                }
                // These fixed reads and token placement are prepaid above,
                // including on a missing-marker failure. No refund is made.
                let marker = program
                    .runtime_requirement
                    .as_deref()
                    .ok_or("Runtime requirement disappeared before owner reservation")?;
                self.owner = Owner::Reserved {
                    marker: marker as *const Requirement as usize,
                };
                Ok(())
            })();
            if result.is_err() {
                self.failed = true;
            }
            result
        }
        pub(crate) fn owner_allocation_reserved(&self) -> bool {
            !self.failed && matches!(self.owner, Owner::Reserved { .. })
        }
        pub(crate) fn bind_owner(
            &mut self,
            _permit: &crate::grammar::RuntimeOwnerPublicationPermit,
            package: &impl crate::grammar::RuntimeGrammarContext,
        ) -> Result<(), String> {
            // ALL work here was prepaid in seal's original analysis pool. The
            // private compiler permit and unchanged move lineage are essential:
            // marker identity alone is not whole-graph authority.
            let result = (|| {
                if self.failed {
                    return Err("Runtime owner companion is poisoned".into());
                }
                let Owner::Reserved { marker } = self.owner else {
                    return Err("Runtime owner was not reserved or is already bound".into());
                };
                let grammar = package
                    .runtime_grammar()
                    .ok_or("Missing runtime owner grammar")?;
                let program = grammar
                    .parser_prediction
                    .as_ref()
                    .ok_or("Missing runtime owner program")?;
                let requirement = program
                    .runtime_requirement
                    .as_deref()
                    .ok_or("Missing sealed runtime marker")?;
                if grammar.profile != Profile::NativeV80
                    || marker != requirement as *const Requirement as usize
                {
                    return Err("Runtime final owner differs from its sealed marker".into());
                }
                let count = match (self.policy, requirement) {
                    (
                        RuntimeGuardedPolicy::LinearFullRepeats,
                        Requirement::LinearFullRepeats { decisions },
                    )
                    | (
                        RuntimeGuardedPolicy::StrictCompletionScopes,
                        Requirement::StrictCompletionScopes { decisions },
                    )
                    | (
                        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
                        Requirement::InputCheckedCompletionScopes { decisions },
                    ) => decisions.len(),
                    _ => return Err("Runtime owner marker differs from proof policy".into()),
                };
                if count == 0 || count != self.records.len() + self.choices.len() {
                    return Err("Runtime owner marker count differs from sealed records".into());
                }
                // NO final record/key walk, clone or content hash. The exact
                // sealed marker allocation carries the already-built key set.
                self.owner = Owner::Bound {
                    marker,
                    grammar: grammar as *const Grammar as usize,
                    program: program as *const ParserPrediction as usize,
                };
                Ok(())
            })();
            if result.is_err() {
                self.failed = true;
            }
            result
        }
        // Handwritten host-cache load (language-pack step 3). Not generated.
        /// Rebuild a saved admission over `package`, which the caller has just
        /// restored and boxed at its final stable address. Before any owner is
        /// published, the saved record keys plus choice keys must EQUAL (as a
        /// set) the decision set of the loaded program's `Requirement` for the
        /// same policy; `bind_owner` alone compares only counts. That
        /// `Requirement` comes from the same trusted saved package and is not
        /// re-derived by either mode. The owner is
        /// then reserved on the loaded `Requirement` box and bound under the
        /// compiler-only permit. See `RuntimeAdmissionLoadMode` for what each
        /// mode trusts. This is a host convention, not cryptographic authority.
        pub(crate) fn load_host_cache(
            saved: SavedRuntimeAdmission,
            permit: &crate::grammar::RuntimeOwnerPublicationPermit,
            package: &impl crate::grammar::RuntimeGrammarContext,
            mode: RuntimeAdmissionLoadMode,
        ) -> Result<Self, String> {
            let SavedRuntimeAdmission {
                policy,
                call_depth,
                logical_bytes,
                work_budget,
                records,
                choices,
            } = saved;
            if records.is_empty() && choices.is_empty() {
                return Err("Saved runtime admission has no decisions".into());
            }
            if records.len() > MAX_KEYS || choices.len() > MAX_KEYS {
                return Err("Saved runtime admission exceeds the obligation count bound".into());
            }
            if logical_bytes > LIMIT {
                return Err("Saved runtime admission exceeds the retention bound".into());
            }
            if records.values().any(|record| record.proof_class != policy) {
                return Err("Saved runtime record has a different proof class".into());
            }
            // Compilation creates choices only on the input-checked native path.
            if !choices.is_empty() && policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
                return Err(
                    "Saved runtime choices require the input-checked completion policy".into(),
                );
            }
            let mut admission = Self::new_with_policy_and_budget(
                policy,
                super::super::super::GrammarAnalysisWorkBudget::new(work_budget),
            )
            .with_call_depth(usize::from(call_depth))?;
            admission.records = records;
            admission.choices = choices;
            admission.logical_bytes = logical_bytes;
            let grammar = package
                .runtime_grammar()
                .ok_or("Loaded runtime package has no grammar")?;
            let program = grammar
                .parser_prediction
                .as_ref()
                .ok_or("Loaded runtime package has no prediction program")?;
            if grammar.profile != Profile::NativeV80 {
                return Err("Loaded runtime admission requires Native80".into());
            }
            if grammar.call_depth != Some(usize::from(call_depth))
                || program.call_depth() != usize::from(call_depth)
            {
                return Err(
                    "Loaded program call depth differs from saved runtime admission".into(),
                );
            }
            let requirement = program
                .runtime_requirement
                .as_deref()
                .ok_or("Loaded program has no runtime requirement")?;
            let decisions = match (policy, requirement) {
                (
                    RuntimeGuardedPolicy::LinearFullRepeats,
                    Requirement::LinearFullRepeats { decisions },
                )
                | (
                    RuntimeGuardedPolicy::StrictCompletionScopes,
                    Requirement::StrictCompletionScopes { decisions },
                )
                | (
                    RuntimeGuardedPolicy::InputCheckedCompletionScopes,
                    Requirement::InputCheckedCompletionScopes { decisions },
                ) => decisions,
                _ => {
                    return Err("Loaded runtime requirement differs from saved proof policy".into());
                }
            };
            // Full key-SET equality, reusing the canonical choice-key validator.
            // It proves |D| == |R| + |C| and D âŠ† R âˆª C. Since |R âˆª C| <= |R| + |C|
            // = |D| <= |R âˆª C|, this gives D == R âˆª C with R and C disjoint, so a
            // removed, added or renamed key is refused even at an equal count.
            let mut work = admission.work_budget.units();
            admission
                .validate_choice_keys(decisions, &mut work)
                .map_err(|cause| format!("Saved runtime admission key set differs: {cause}"))?;
            admission.owner = Owner::Reserved {
                marker: requirement as *const Requirement as usize,
            };
            admission.bind_owner(permit, package)?;
            if mode == RuntimeAdmissionLoadMode::Revalidate {
                admission.revalidate(grammar, program)?;
            }
            Ok(admission)
        }
        /// Re-check a loaded admission with the seal-time proof checks: every
        /// record is rebound and its proof re-verified
        /// (`validate_decision_with_remaining`); every choice is rebound and
        /// its stored proof kind's witness re-verified (`verify_multiway_scopes`,
        /// the same check seal runs); finally the exact owner identity is
        /// validated. Records, choices and the owner check each spend their own
        /// fresh pool of the saved budget, as compilation spreads them over
        /// separate pools. The `Requirement` decision set is not re-derived.
        pub(crate) fn revalidate(
            &self,
            grammar: &Grammar,
            program: &ParserPrediction,
        ) -> Result<(), String> {
            let mut work = self.work_budget.units();
            for path in self.records.keys() {
                self.validate_decision_with_remaining(grammar, program, path, &mut work)
                    .map_err(|cause| {
                        format!("Runtime record {path} revalidation failed: {cause}")
                    })?;
            }
            if !self.choices.is_empty() {
                if self.policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
                    return Err("Runtime choices require the input-checked policy".into());
                }
                let mut work = self.work_budget.units();
                self.verify_multiway_scopes(grammar, program, &mut work)
                    .map_err(|cause| format!("Runtime choice revalidation failed: {cause}"))?;
            }
            self.validate(grammar, program, &mut self.work_budget.units())
        }
        #[cfg(test)]
        pub(crate) fn test_poison(&mut self) {
            self.failed = true;
        }
        #[cfg(test)]
        pub(in crate::grammar) fn collect(
            &mut self,
            grammar: &Grammar,
            program: &ParserPrediction,
            work: &mut usize,
            observation: Option<super::super::super::AnalysisObservation<'_>>,
        ) -> Result<(), String> {
            self.collect_with_certificates(grammar, program, work, observation, None)
        }
        #[cfg(test)]
        pub(in crate::grammar) fn collect_with_certificates(
            &mut self,
            grammar: &Grammar,
            program: &ParserPrediction,
            work: &mut usize,
            observation: Option<super::super::super::AnalysisObservation<'_>>,
            certificates: Option<&mut super::super::RepeatCertificates>,
        ) -> Result<(), String> {
            self.collect_with_inventory_and_certificates(
                grammar,
                program,
                work,
                observation,
                None,
                certificates,
            )
        }
        pub(in crate::grammar::parser_prediction) fn collect_with_inventory_and_certificates(
            &mut self,
            grammar: &Grammar,
            program: &ParserPrediction,
            work: &mut usize,
            observation: Option<super::super::super::AnalysisObservation<'_>>,
            inventory: Option<&[String]>,
            certificates: Option<&mut super::super::RepeatCertificates>,
        ) -> Result<(), String> {
            self.collect_with_inventory_certificates_and_guards(
                grammar,
                program,
                work,
                observation,
                inventory,
                certificates,
                None,
            )
        }
        #[allow(clippy::too_many_arguments)] // Same collector, same work reference, optional private borrow.
        pub(in crate::grammar::parser_prediction) fn collect_with_inventory_certificates_and_guards(
            &mut self,
            grammar: &Grammar,
            program: &ParserPrediction,
            work: &mut usize,
            observation: Option<super::super::super::AnalysisObservation<'_>>,
            inventory: Option<&[String]>,
            mut certificates: Option<&mut super::super::RepeatCertificates>,
            prepared: Option<&super::super::guard_facts::GuardFacts<'_>>,
        ) -> Result<(), String> {
            // Parser calls capture the actual initial pool before precollection.
            // Direct collectors retain the configured whole-pool accounting base;
            // their supplied balance may already exclude earlier work in that pool.
            let initial_work = self
                .analysis_initial_work
                .unwrap_or(self.work_budget.units());
            let trace = observation.map(|context| context.observation);
            if let Some(context) = observation {
                context.observation.reuse_begin_with_initial(
                    context.stage,
                    context.entry,
                    initial_work,
                    *work,
                );
            }
            let result = (|| {
                if self.failed || !matches!(self.owner, Owner::Unsealed) || !self.records.is_empty()
                {
                    return Err("Runtime admission companion is already used or failed".into());
                }
                const {
                    assert!(
                        std::mem::size_of::<RuntimeGuardedAdmission>()
                            + std::mem::size_of::<ParserPrediction>()
                            <= HEADER
                    );
                    assert!(std::mem::size_of::<Binding>() <= RECORD);
                    assert!(std::mem::size_of::<Requirement>() <= 64);
                }
                // LIVE_CALLERS precedes both existing qualified nullable
                // dispatches. Reject their WHOLE program inventory before any
                // record is published, independent of selected callee shape.
                // The ordinary profile path and its capabilities stay literal.
                let inventory_before = *work;
                let inventory = (|| -> Result<(), String> {
                    if let Some(paths) = inventory {
                        for path in paths {
                            if let Some(trace) = trace {
                                trace.reuse_event(ReuseEvent::InventoryKey, 1);
                            }
                            let visit = pay(work);
                            observe_candidate_with_initial(visit, grammar, program, observation,
                                (super::super::super::admission_observation::Phase::RuntimeNullableInventory, Some(path)), *work, initial_work)?;
                            if let Some(trace) = trace {
                                trace.reuse_event(ReuseEvent::InventoryRepeat, 1);
                            }
                            let qualification =
                                grammar.qualified_nullable_decision_with_finite_work(path, work);
                            if observe_candidate_with_initial(qualification, grammar, program, observation,
                                (super::super::super::admission_observation::Phase::RuntimeNullableInventory, Some(path)), *work, initial_work)?.is_some() {
                                return observe_candidate_with_initial(Err(format!(
                                    "Runtime guarded live callers do not support qualified nullable decisions at {path}"
                                )), grammar, program, observation,
                                    (super::super::super::admission_observation::Phase::RuntimeNullableInventory, Some(path)), *work, initial_work);
                            }
                        }
                        return Ok(());
                    }
                    for path in program.decisions.keys() {
                        if let Some(trace) = trace {
                            trace.reuse_event(ReuseEvent::InventoryKey, 1);
                        }
                        // Pay every map-key visit and suffix check, including keys
                        // which do not name a repeat and need no source lookup.
                        let visit = pay(work);
                        observe_candidate_with_initial(visit, grammar, program, observation,
                        (super::super::super::admission_observation::Phase::RuntimeNullableInventory, Some(path)), *work, initial_work)?;
                        if !path.ends_with("/repeat") {
                            continue;
                        }
                        if let Some(trace) = trace {
                            trace.reuse_event(ReuseEvent::InventoryRepeat, 1);
                        }
                        let qualification =
                            grammar.qualified_nullable_decision_with_finite_work(path, work);
                        if observe_candidate_with_initial(qualification, grammar, program, observation,
                        (super::super::super::admission_observation::Phase::RuntimeNullableInventory, Some(path)), *work, initial_work)?.is_some()
                    {
                        return observe_candidate_with_initial(Err(format!(
                            "Runtime guarded live callers do not support qualified nullable decisions at {path}"
                        )), grammar, program, observation,
                            (super::super::super::admission_observation::Phase::RuntimeNullableInventory, Some(path)), *work, initial_work);
                    }
                    }
                    Ok(())
                })();
                if let Some(trace) = trace {
                    trace.reuse_window(ReuseWindow::Inventory, inventory_before, *work);
                }
                inventory?;
                // Transient FIRST reuse begins only after the complete canonical
                // nullable inventory. Default/linear/runtime probes allocate none.
                let completed_enabled = matches!(
                    self.policy,
                    RuntimeGuardedPolicy::StrictCompletionScopes
                        | RuntimeGuardedPolicy::InputCheckedCompletionScopes
                );
                #[cfg(test)]
                let completed_enabled = completed_enabled && !self.disable_completed_reuse;
                let mut completed = if completed_enabled {
                    let header_before = *work;
                    let cache =
                        super::super::super::first_sets::completed_reuse::NullabilityCompleted::new(
                            grammar, work,
                        );
                    if let Some(trace) = trace {
                        trace.reuse_window(ReuseWindow::CompletedHeader, header_before, *work);
                    }
                    if let (Some(trace), Ok(cache)) = (trace, &cache) {
                        cache.observe_retained(trace);
                    }
                    Some(observe_candidate_with_initial(cache, grammar, program, observation,
                        (super::super::super::admission_observation::Phase::RuntimeCandidateBinding, None), *work, initial_work)?)
                } else {
                    None
                };
                let companion_header_before = *work;
                let header = admit(&mut self.logical_bytes, HEADER, work);
                if let Some(trace) = trace {
                    trace.reuse_window(
                        ReuseWindow::CompanionHeader,
                        companion_header_before,
                        *work,
                    );
                }
                observe_candidate_with_initial(
                    header,
                    grammar,
                    program,
                    observation,
                    (
                        super::super::super::admission_observation::Phase::RuntimeCandidateBinding,
                        None,
                    ),
                    *work,
                    initial_work,
                )?;
                for path in program
                    .decisions
                    .keys()
                    .filter(|path| path.ends_with("/repeat"))
                {
                    let visit = pay(work);
                    observe_candidate_with_initial(visit, grammar, program, observation,
                        (super::super::super::admission_observation::Phase::RuntimeCandidateBinding, Some(path)), *work, initial_work)?;
                    let binding_before = *work;
                    if let Some(trace) = trace {
                        trace.reuse_event(ReuseEvent::Binding, 1);
                    }
                    let bound = binding_with_prepared_guard_facts(
                        grammar,
                        program,
                        path,
                        work,
                        &mut self.logical_bytes,
                        self.policy,
                        (
                            observation,
                            completed.as_mut(),
                            certificates.as_deref_mut(),
                            initial_work,
                        ),
                        prepared,
                    );
                    if let Some(trace) = trace {
                        trace.reuse_window(ReuseWindow::Binding, binding_before, *work);
                    }
                    if let Some(record) = observe_candidate_with_initial(bound, grammar, program, observation,
                        (super::super::super::admission_observation::Phase::RuntimeCandidateBinding, Some(path)), *work, initial_work)? {
                        if let Some(trace) = trace { trace.reuse_event(ReuseEvent::EligibleRecord, 1); }
                        if self.records.len() >= MAX_KEYS {
                            return observe_candidate_with_initial(Err("Runtime obligation count bound exceeded".into()),
                                grammar, program, observation,
                                (super::super::super::admission_observation::Phase::RuntimeCandidateBinding, Some(path)), *work, initial_work);
                        }
                        // binding checked cumulative retained bytes and paid the
                        // owned record/key copy before allocation. Pay actual
                        // ordered-map publication too; no debit is refunded.
                        let publication = pay(work);
                        observe_candidate_with_initial(publication, grammar, program, observation,
                            (super::super::super::admission_observation::Phase::RuntimeCandidateBinding, Some(path)), *work, initial_work)?;
                        self.records.insert(path.clone(), record);
                    }
                }
                Ok(())
            })();
            if result.is_err() {
                self.failed = true;
            }
            if let Some(trace) = trace {
                trace.reuse_end(*work, result.is_err());
            }
            result
        }
        // Generic/forged/serialized graph controls keep the original route.
        pub(crate) fn seal(
            &mut self,
            grammar: &Grammar,
            program: &mut ParserPrediction,
            work: &mut usize,
        ) -> Result<(), String> {
            self.seal_inner::<false>(grammar, program, work)
        }

        // Only the fresh expression-builder -> collect -> final-flags chain
        // calls this entry. No public or stored provenance marker is introduced.
        pub(in crate::grammar::parser_prediction) fn seal_native_generated(
            &mut self,
            grammar: &Grammar,
            program: &mut ParserPrediction,
            work: &mut usize,
        ) -> Result<(), String> {
            if self.policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
                return self.seal(grammar, program, work);
            }
            // Fresh native builder only: final recursive flags already exist,
            // while no marker or usable owner has yet been published.
            self.bind_classified_pairs(grammar, program, work)?;
            self.seal_inner::<true>(grammar, program, work)
        }

        fn seal_inner<const NATIVE_GENERATED: bool>(
            &mut self,
            grammar: &Grammar,
            program: &mut ParserPrediction,
            work: &mut usize,
        ) -> Result<(), String> {
            let result = (|| {
                if self.failed || !matches!(self.owner, Owner::Unsealed) {
                    return Err("Runtime admission companion cannot seal".into());
                }
                if self.records.is_empty() && self.choices.is_empty() {
                    return Err(match self.policy {
                        RuntimeGuardedPolicy::LinearFullRepeats => {
                            "No eligible productive linear Full repeat".into()
                        }
                        RuntimeGuardedPolicy::StrictCompletionScopes => {
                            "No eligible nonnullable strict Full repeat".into()
                        }
                        RuntimeGuardedPolicy::InputCheckedCompletionScopes => {
                            "No eligible nonnullable input-checked repeat".into()
                        }
                    });
                }
                if self.policy == RuntimeGuardedPolicy::StrictCompletionScopes {
                    // The typed owner executes Native80's existing explicit
                    // completion policy; decide profile authority before proof.
                    if grammar.profile != Profile::NativeV80 {
                        return Err("Strict completion policy requires Native80".into());
                    }
                    // collect records source/graph candidates only. Certification
                    // observes FINAL recursive metadata before any typed marker,
                    // stamp or usable compilation is published.
                    for (path, record) in &self.records {
                        verify_strict_record(program, path, record, work)?;
                    }
                }
                if self.policy == RuntimeGuardedPolicy::InputCheckedCompletionScopes {
                    if grammar.profile != Profile::NativeV80 {
                        return Err("Input-checked completion policy requires Native80".into());
                    }
                    self.verify_multiway_scopes(grammar, program, work)?;
                    if NATIVE_GENERATED {
                        verify_native_input_checked_records(
                            program,
                            &self.records,
                            &mut self.logical_bytes,
                            work,
                        )?;
                    } else {
                        for (path, record) in &self.records {
                            verify_input_checked_record(program, path, record, work)?;
                        }
                    }
                }
                let mut paths = std::collections::BTreeSet::new();
                admit(&mut self.logical_bytes, 64, work)?;
                for path in self.records.keys().chain(self.choices.keys()) {
                    admit(
                        &mut self.logical_bytes,
                        32usize
                            .checked_add(path.len())
                            .ok_or("Runtime path size overflow")?,
                        work,
                    )?;
                    paths.insert(path.clone());
                }
                program.runtime_requirement = Some(Box::new(match self.policy {
                    RuntimeGuardedPolicy::LinearFullRepeats => {
                        Requirement::LinearFullRepeats { decisions: paths }
                    }
                    RuntimeGuardedPolicy::StrictCompletionScopes => {
                        Requirement::StrictCompletionScopes { decisions: paths }
                    }
                    RuntimeGuardedPolicy::InputCheckedCompletionScopes => {
                        Requirement::InputCheckedCompletionScopes { decisions: paths }
                    }
                }));
                // The marker Box already has its admitted64-byte allocation.
                // Its identity survives the unchanged moves into the final owner.
                self.reserve_owner(program, work)?;
                Ok(())
            })();
            if result.is_err() {
                self.failed = true;
            }
            result
        }
        #[cfg(test)]
        pub(crate) fn validate_decision(
            &self,
            grammar: &Grammar,
            program: &ParserPrediction,
            path: &str,
        ) -> Result<(), String> {
            self.validate_decision_with_remaining(
                grammar,
                program,
                path,
                &mut self.work_budget.units(),
            )
        }
        pub(crate) fn validate_decision_with_remaining(
            &self,
            grammar: &Grammar,
            program: &ParserPrediction,
            path: &str,
            work: &mut usize,
        ) -> Result<(), String> {
            // Revalidation spends the same selected deferred phase pool.
            if self.policy == RuntimeGuardedPolicy::StrictCompletionScopes
                && grammar.profile != Profile::NativeV80
            {
                return Err("Strict completion policy requires Native80".into());
            }
            if self.policy == RuntimeGuardedPolicy::InputCheckedCompletionScopes
                && grammar.profile != Profile::NativeV80
            {
                return Err("Input-checked completion policy requires Native80".into());
            }
            let mut logical = 0;
            let actual = binding(
                grammar,
                program,
                path,
                work,
                &mut logical,
                self.policy,
                None,
            )
            .map_err(|cause| format!("Runtime deferred source/graph binding failed: {cause}"))?
            .ok_or("Runtime deferred decision is no longer eligible")?;
            if self.records.get(path) != Some(&actual) {
                return Err("Runtime deferred source/graph binding differs".into());
            }
            if self.policy == RuntimeGuardedPolicy::StrictCompletionScopes {
                verify_strict_record(program, path, &actual, work)?;
            }
            if self.policy == RuntimeGuardedPolicy::InputCheckedCompletionScopes {
                verify_input_checked_record(program, path, &actual, work)?;
            }
            Ok(())
        }
        pub(in crate::grammar::parser_prediction) fn validate(
            &self,
            grammar: &Grammar,
            program: &ParserPrediction,
            work: &mut usize,
        ) -> Result<(), String> {
            pay(work)?;
            if self.failed
                || grammar.profile != Profile::NativeV80
                || (self.records.is_empty() && self.choices.is_empty())
            {
                return Err("Runtime admission companion is unavailable".into());
            }
            let Some(requirement) = program.runtime_requirement.as_deref() else {
                return Err("Runtime program has no explicit requirement".into());
            };
            let decisions = match (self.policy, requirement) {
                (
                    RuntimeGuardedPolicy::LinearFullRepeats,
                    Requirement::LinearFullRepeats { decisions },
                )
                | (
                    RuntimeGuardedPolicy::StrictCompletionScopes,
                    Requirement::StrictCompletionScopes { decisions },
                )
                | (
                    RuntimeGuardedPolicy::InputCheckedCompletionScopes,
                    Requirement::InputCheckedCompletionScopes { decisions },
                ) => decisions,
                _ => return Err("Runtime requirement differs from its proof policy".into()),
            };
            pay(work)?;
            if !self.choices.is_empty() {
                self.validate_choice_keys(decisions, work)?;
            } else {
                if decisions.len() != self.records.len() {
                    return Err("Runtime requirement differs from compiler companion".into());
                }
                for (actual, bound) in decisions.iter().zip(self.records.keys()) {
                    pay(work)?;
                    for _ in 0..actual.len().max(bound.len()).div_ceil(64) {
                        pay(work)?;
                    }
                    if actual != bound {
                        return Err("Runtime requirement differs from compiler companion".into());
                    }
                }
            }
            // Keep every complete sorted-key comparison above literal. This
            // replaces the removed whole-content serializer/hash traversal only.
            pay(work)?;
            let Owner::Bound {
                marker,
                grammar: bound_grammar,
                program: bound_program,
            } = self.owner
            else {
                return Err("Runtime companion has no published immutable owner".into());
            };
            pay(work)?;
            if bound_grammar != grammar as *const Grammar as usize {
                return Err("Runtime companion differs from exact source/program owner".into());
            }
            pay(work)?;
            if bound_program != program as *const ParserPrediction as usize {
                return Err("Runtime companion differs from exact program owner".into());
            }
            pay(work)?;
            if marker != requirement as *const Requirement as usize {
                return Err("Runtime companion differs from its exact sealed marker".into());
            }
            Ok(())
        }
    }
    impl ParserPrediction {
        /// Host-cache load only: restore the one compiler-transient flag that
        /// serialization skips, exactly as `ParserPrediction` construction sets
        /// it from the grammar profile.
        pub(crate) fn restore_host_cache_transients(&mut self, profile: Profile) {
            self.allow_empty_choices = profile.has_constant_guards();
        }
        pub fn requires_runtime_capability(&self) -> bool {
            self.runtime_requirement.is_some()
        }
        pub fn strict_completion_obligation_count(&self) -> usize {
            match self.runtime_requirement.as_deref() {
                Some(Requirement::StrictCompletionScopes { decisions }) => decisions.len(),
                _ => 0,
            }
        }
        pub fn input_checked_completion_obligation_count(&self) -> usize {
            match self.runtime_requirement.as_deref() {
                Some(Requirement::InputCheckedCompletionScopes { decisions }) => decisions.len(),
                _ => 0,
            }
        }
        pub fn runtime_obligation_count(&self) -> usize {
            self.runtime_requirement
                .as_deref()
                .map_or(0, |requirement| match requirement {
                    Requirement::LinearFullRepeats { decisions }
                    | Requirement::StrictCompletionScopes { decisions }
                    | Requirement::InputCheckedCompletionScopes { decisions } => decisions.len(),
                })
        }
    }
    #[cfg(test)]
    mod first_repeat_tests {
        use super::*;
        include!("runtime_guarded_first_repeat_tests.rs");
    }
    #[cfg(test)]
    mod host_cache_tests {
        use super::*;
        include!("runtime_guarded_host_cache_tests.rs");
    }
    #[cfg(test)]
    mod owner_tests {
        use super::*;
        include!("runtime_guarded_owner_tests.rs");
    }
    #[cfg(test)]
    mod observation_tests {
        use super::*;
        include!("runtime_guarded_observation_tests.rs");
        mod nullability_projection {
            use super::*;
            include!("runtime_guarded_nullability_projection_tests.rs");
        }
    }
}

// Deliberate typed IR-holder growth. These target checks are compile-time
// contracts, not measurements claimed before root qualification. The old
// Grammar state has one complete representation in Xtext.
#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(std::mem::size_of::<Option<Box<Requirement>>>() == 8);
    assert!(
        std::mem::size_of::<super::ParserPrediction>()
            // The data-only build's growth is the u16 call depth (padded).
            == 136
    );
};
#[cfg(target_pointer_width = "32")]
const _: () = {
    assert!(std::mem::size_of::<Option<Box<Requirement>>>() == 4);
    assert!(std::mem::size_of::<super::ParserPrediction>() == 68);
};
