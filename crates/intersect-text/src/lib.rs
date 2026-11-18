//! Handwritten Xtext interpreter and explicit unique-name scope policy.
//! Shared Rcore execution belongs to intersect-model.
//! It consumes compiled definitions; it does not implement KerML/SysML policy.
//!
//! # Application APIs
//! Start with [`compiler::compile_rcore_project`], [`Source`] and [`evaluate`].
//! Use [`document`], [`source_edit`] and [`serialization`] for editing, and
//! [`distribution`] for source-authoritative language archives.
//!
//! # Advanced extension contracts
//! [`extensions`] separates getter activation and runtime-guarded execution
//! from ordinary evaluation, and identifies the owning provider/policy modules.
use crate::package::Package;
#[cfg(test)]
use intersect_model::core::storage::Query;
use intersect_model::core::storage::{self, ModelRead, ModelStore};
use intersect_model::core::{
    frozen::{FrozenStore, Metamodel},
    model::Model,
};
pub mod compiler;
pub mod distribution;
pub mod grammar;
pub mod preparation;

/// Identity of Xtext's own package inputs, enabled features and available lock
/// context, computed by the host-only build-support package. Recipe v3 no longer
/// scans model's sources. Use [`compiler_identity`] for combined compiler output
/// or [`compiler_cache_identity`] for profile/provider-specific host caches.
pub const SOURCE_IDENTITY: &str = env!("INTERSECT_TEXT_SOURCE_IDENTITY");
/// Installed language package release, independent of the model package release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
mod identity;
use crate::grammar::Expr;
use crate::grammar::Kind;
use crate::grammar::Token;
use crate::grammar::token_cursor::TokenCursor;
pub use identity::{CACHE_IDENTITY_RECIPE, compiler_cache_identity, compiler_identity};
use intersect_model::definitions::schema::Diagnostic;
use intersect_model::definitions::schema::Expression;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
mod datatype_text;
pub mod document;
pub mod draft;
pub mod entry;
pub mod inference;
pub mod linking;
pub mod migration;
pub mod package;
use linking::ReferenceCandidates;
pub mod extensions;
mod rcore_getters;
pub mod rcore_scopes;
mod rule_trampoline;
pub mod serialization;
pub mod source_edit;
pub mod text_services;
pub use inference::ordinary;
mod replay;
use datatype_text::{DatatypeText, TextCheckpoint};
use intersect_model::initialization;
use intersect_model::query;
pub mod setting;
pub use replay::{
    compile_inputs_with_context as compile_inference_inputs_with_context, recompile,
    recompile_with_context, recompile_with_source_policy,
};

mod bounded_runtime;
#[cfg(test)]
mod bounded_runtime_tests;
mod resource_accounting;
pub mod rule_conversion;

#[cfg(test)]
mod resource_accounting_tests;

#[cfg(test)]
mod document_tests;
#[cfg(test)]
mod index_policy_tests;
#[cfg(test)]
mod linking_tests;
#[cfg(test)]
mod owned_rule_spans_tests;
#[cfg(test)]
mod parser_tests;
#[cfg(test)]
mod source_edit_tests;

#[cfg(test)]
fn compile_migrated_fixture(
    ecore: &str,
    xtext: &str,
    behavior: &str,
) -> Result<Package, Vec<Diagnostic>> {
    compile_migrated_fixture_with_profile(ecore, xtext, behavior, grammar::Profile::default())
}
#[cfg(test)]
fn compile_migrated_fixture_with_profile(
    ecore: &str,
    xtext: &str,
    behavior: &str,
    profile: grammar::Profile,
) -> Result<Package, Vec<Diagnostic>> {
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    let report = intersect_model::migration::migrate_ecore(
        DefinitionSource::new("memory:oracle.ecore", SourceKind::Ecore, ecore),
        "memory:fixture.rcore",
    )
    .map_err(|error| {
        vec![Diagnostic::new(
            "ecore",
            error.span.start,
            error.span.end,
            error.message,
        )]
    })?;
    let converted = report.converted.ok_or_else(|| {
        vec![Diagnostic::new(
            "ecore",
            0,
            ecore.len(),
            format!("Fixture migration failed: {:?}", report.issues),
        )]
    })?;
    compiler::compile_rcore_with_profile(&converted.text, Some(xtext), Some(behavior), profile)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub uri: String,
    pub text: String,
}

#[cfg(feature = "components")]
pub mod components;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reference {
    pub uri: String,
    pub start: usize,
    pub end: usize,
    pub target: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evaluation {
    pub model: Model,
    pub diagnostics: Vec<Diagnostic>,
    pub references: Vec<Reference>,
    pub derived: BTreeMap<String, BTreeMap<String, Value>>,
}
/// Host-selected deterministic evaluation budgets. `None` removes that ceiling.
/// These are not memory quotas; hosts must manage cancellation and memory separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvaluationLimits {
    pub resources: Option<usize>,
    pub input_bytes: Option<usize>,
    pub elements_per_resource: Option<usize>,
    pub tokens_per_resource: Option<usize>,
    pub steps_per_resource: Option<usize>,
    pub nesting: usize,
}
impl EvaluationLimits {
    /// No size or step ceiling. Keep the recursive parser's nesting protection.
    pub const fn native() -> Self {
        Self {
            resources: None,
            input_bytes: None,
            elements_per_resource: None,
            tokens_per_resource: None,
            steps_per_resource: None,
            nesting: 64,
        }
    }
    /// The browser workbench's original resource policy.
    pub const fn workbench() -> Self {
        Self {
            resources: Some(8),
            input_bytes: Some(256_000),
            elements_per_resource: Some(512),
            tokens_per_resource: Some(50_000),
            steps_per_resource: Some(100_000),
            nesting: 64,
        }
    }
}
impl Default for EvaluationLimits {
    fn default() -> Self {
        Self::native()
    }
}
#[derive(Clone)]
struct Pending {
    // A resource's references repeat their owner, feature, target class and
    // URI; each spelling is shared rather than copied per reference.
    owner: std::rc::Rc<str>,
    field: std::rc::Rc<str>,
    spelling: String,
    target: std::rc::Rc<str>,
    uri: std::rc::Rc<str>,
    start: usize,
    end: usize,
    index: Option<usize>,
    active: bool,
}
#[derive(Debug)]
enum ParseError {
    Syntax(String),
    Budget(String),
    Storage(String),
    /// Runtime prediction could not choose one alternative for the actual
    /// input. Not a syntax error (no recovery or backtracking hides it), but
    /// located: the machine records the decision's first token in
    /// `diagnostic_position` and the resource is rejected with a diagnostic.
    Ambiguous(String),
}
impl ParseError {
    fn message(self) -> String {
        match self {
            Self::Syntax(message)
            | Self::Budget(message)
            | Self::Storage(message)
            | Self::Ambiguous(message) => message,
        }
    }
}
// ModelStore savepoints own model rollback; this journal only restores parser-local references.
struct Undo<'a> {
    owner: usize,
    field: &'a str,
    unresolved: Option<usize>,
}
// Origin travels with the actual current-object slot, including rollback.
// An assigned child value uses a separate slot and cannot replace caller origin.
#[derive(Clone, Copy)]
struct Current {
    index: usize,
    delegated: bool,
}
impl Current {
    fn local(index: usize) -> Self {
        Self {
            index,
            delegated: false,
        }
    }
    fn called(index: usize) -> Self {
        Self {
            index,
            delegated: true,
        }
    }
}
// Canonical object-rule context shared by literal and suspended execution.
struct CurrentRulePhase<'a> {
    current: Option<Current>,
    previous_construction: Option<(&'a str, usize)>,
    entry_token: usize,
    start: usize,
    undo_start: usize,
    profile: grammar::Profile,
}
// The same durable once() envelope, retained across a real tail dispatch.
struct OnceDocumentPhase<'a> {
    previous_origin: Option<&'a str>,
    previous_feature: Option<(usize, &'a str)>,
    captured_node: Option<intersect_model::core::text_document::NodeId>,
    node: Option<intersect_model::core::text_document::NodeId>,
}
struct Checkpoint {
    document: Option<document::Mark>,
    /// None when an enclosing checkpoint restores the store: see
    /// `Machine::propagating_checkpoint`.
    storage: Option<storage::Checkpoint>,
    text: Option<TextCheckpoint>,
    cursor: TokenCursor,
    datatype_span: Option<(usize, usize)>,
    elements: usize,
    undo: usize,
    pending: usize,
    depth: usize,
}
#[derive(Clone)]
struct LexicalType {
    /// Key of the schema datatype, when the target is one.
    datatype: Option<String>,
    target: String,
    constrained: bool,
}

/// Parser tables that depend only on a language's schema and grammar:
/// keywords, punctuation keywords, unsettable features and attribute lexical
/// types. Building them walks every class's effective features, so contexts
/// that parse many resources of one language keep them (`parse_tables`).
pub(crate) struct ParseTables {
    keywords: BTreeSet<String>,
    punctuation: Option<grammar::PunctuationKeywords>,
    unsettable: BTreeMap<String, BTreeSet<String>>,
    lexical_types: BTreeMap<String, BTreeMap<String, LexicalType>>,
}
impl ParseTables {
    pub(crate) fn build(
        schema: &intersect_model::definitions::schema::Schema,
        grammar: &grammar::Grammar,
    ) -> Result<Self, String> {
        let keywords: BTreeSet<String> = grammar.keywords().into_iter().collect();
        let punctuation = (!grammar.profile.is_workbench()).then(|| {
            if grammar.hidden.is_some() {
                grammar::PunctuationKeywords::all(keywords.iter().map(String::as_str))
            } else {
                grammar::PunctuationKeywords::new(keywords.iter().map(String::as_str))
            }
        });
        let mut unsettable = BTreeMap::<String, BTreeSet<String>>::new();
        let mut lexical_types = BTreeMap::<String, BTreeMap<String, LexicalType>>::new();
        for class in schema.classes.keys() {
            let effective = schema.effective_features(class)?;
            for member in &effective {
                let field = member.feature;
                if field.unsettable {
                    unsettable
                        .entry(class.clone())
                        .or_default()
                        .insert(field.name.clone());
                }
                if field.reference {
                    continue;
                }
                let constrained = !schema.feature_domains(class, &field.name).is_empty();
                // The class's effective features are already here; the plain
                // lookup would compute them again for every attribute.
                let target = schema.runtime_attribute_lexical_target_among(
                    class,
                    &field.name,
                    &effective,
                )?;
                if let Some(target) = target {
                    let datatype = schema
                        .datatypes
                        .contains_key(target)
                        .then(|| target.to_owned());
                    if constrained || datatype.is_some() {
                        lexical_types.entry(class.clone()).or_default().insert(
                            field.name.clone(),
                            LexicalType {
                                datatype,
                                target: target.to_owned(),
                                constrained,
                            },
                        );
                    }
                }
            }
        }
        Ok(Self {
            keywords,
            punctuation,
            unsettable,
            lexical_types,
        })
    }
}
/// Nested decision selections over one machine's fixed tokens, shared by all of
/// its predictions so sibling members reuse what earlier ones predicted.
#[derive(Default)]
struct NestedDecisions {
    selections: std::collections::HashMap<grammar::NestedDecisionKey, Option<usize>>,
    viability: std::collections::HashMap<grammar::ViabilityKey, bool>,
    /// Boxed so the machine keeps its size.
    reuse: Box<Reuse>,
}
/// An element index and its identity spelling.
type RecentIdentity = Option<(usize, std::rc::Rc<str>)>;
/// Work one machine can reuse because its resource, grammar and tokens are
/// fixed: nothing here changes what the machine computes.
#[derive(Default)]
struct Reuse {
    memo: PredictionMemo,
    pending_names: PendingNames,
    /// Recently spelled element identities, by element index.
    identities: std::cell::RefCell<[RecentIdentity; 8]>,
}
/// Shared spellings of one machine's pending references.
#[derive(Default)]
struct PendingNames {
    names: BTreeSet<std::rc::Rc<str>>,
}
impl PendingNames {
    fn name(&mut self, name: &str) -> std::rc::Rc<str> {
        if let Some(known) = self.names.get(name) {
            return known.clone();
        }
        let shared: std::rc::Rc<str> = name.into();
        self.names.insert(shared.clone());
        shared
    }
}
/// Outcomes of predictions that read only their token classifications: no
/// nested-decision state, budget, document or audit. Such a prediction is a
/// function of its decision and the classifications it read, so the same
/// classifications at another position select the same alternative.
#[derive(Default)]
struct PredictionMemo {
    decisions: std::collections::HashMap<usize, Vec<MemoizedPrediction>>,
}
struct MemoizedPrediction {
    suffix: Box<str>,
    /// Classification of every token the prediction read, in order.
    symbols: Box<[u32]>,
    selected: Option<usize>,
    /// Reported lookahead extent, relative to the first token.
    furthest: usize,
}
impl PredictionMemo {
    const END: u32 = u32::MAX;
    const MAX_SYMBOLS: usize = 8;
    const MAX_OUTCOMES: usize = 32;
    fn symbol<T>(tokens: &[Token<T>], position: usize) -> Option<u32> {
        match tokens.get(position) {
            Some(token) => token
                .lexical_alternative
                .and_then(|alternative| u32::try_from(alternative).ok())
                .filter(|symbol| *symbol != Self::END),
            None => (position == tokens.len()).then_some(Self::END),
        }
    }
    fn get<T>(
        &self,
        expr: usize,
        suffix: &str,
        tokens: &[Token<T>],
        start: usize,
    ) -> Option<(Option<usize>, usize)> {
        self.decisions.get(&expr)?.iter().find_map(|outcome| {
            (&*outcome.suffix == suffix
                && outcome
                    .symbols
                    .iter()
                    .enumerate()
                    .all(|(offset, symbol)| Self::symbol(tokens, start + offset) == Some(*symbol)))
            .then_some((outcome.selected, start + outcome.furthest))
        })
    }
    fn insert<T>(
        &mut self,
        expr: usize,
        suffix: &str,
        tokens: &[Token<T>],
        read: std::ops::Range<usize>,
        selected: Option<usize>,
        furthest: usize,
    ) {
        if read.len() > Self::MAX_SYMBOLS || furthest < read.start {
            return;
        }
        let Some(symbols) = read
            .clone()
            .map(|position| Self::symbol(tokens, position))
            .collect::<Option<Box<[u32]>>>()
        else {
            return;
        };
        let outcomes = self.decisions.entry(expr).or_default();
        if outcomes.len() < Self::MAX_OUTCOMES {
            outcomes.push(MemoizedPrediction {
                suffix: suffix.into(),
                symbols,
                selected,
                furthest: furthest - read.start,
            });
        }
    }
}
struct Machine<'a, 'store, S: ModelStore, C: draft::ParseContext<S> = Package> {
    document: Option<Box<document::Capture>>,
    document_origin: Option<&'a str>,
    document_feature: Option<(usize, &'a str)>,
    retained_tokens: bool,
    hidden_policy: &'a [String],
    raw_entry: bool,
    construction: Option<(&'a str, usize)>,
    settings: Option<&'a setting::TextSettings<'a>>,
    raw_source: &'a str,
    utf16_positions: Vec<usize>,
    datatype_text: Option<DatatypeText>,
    conversions: Option<&'a intersect_model::definitions::conversion::ConversionRegistry<'a>>,
    initializers: Option<&'a initialization::PackageInitializers>,
    lexical_types: &'a BTreeMap<String, BTreeMap<String, LexicalType>>,
    unsettable: &'a BTreeMap<String, BTreeSet<String>>,
    keywords: &'a BTreeSet<String>,
    undo: Vec<Undo<'a>>,
    limits: EvaluationLimits,
    package: &'a C,
    uri: &'a str,
    tokens: Vec<Token<Cow<'a, str>>>,
    cursor: TokenCursor,
    datatype_span: Option<(usize, usize)>,
    store: &'store mut S,
    elements: usize,
    pending: Vec<Pending>,
    scalar_pending: BTreeMap<(usize, &'a str), usize>,
    steps: usize,
    depth: usize,
    // Preserve called-rule values in caller-generic fragment bodies. The model
    // API admits them against the actual owner; field-directed coercion would
    // incorrectly turn a numeric EString into an integer.
    generic_fragment: bool,
    farthest: usize,
    diagnostic_position: Option<usize>,
    expected: String,
    nested_decisions: NestedDecisions,
}
fn hidden_token<T>(
    token: &Token<T>,
    policy: &[String],
    lexical: &grammar::LexerPrediction,
) -> Result<bool, String> {
    let alternative = token
        .lexical_alternative
        .ok_or("Missing retained token classification")?;
    Ok(
        matches!(lexical.alternative(alternative)?, grammar::LexicalAlternative::Terminal(name) if policy.contains(name)),
    )
}
// Borrow the one authoritative classified token stream. This adapter runs no
// lexer, model action or conversion; replay never refunds charged traversal.
struct PredictionTokens<'a, T> {
    cursor: TokenCursor,
    tokens: &'a [Token<T>],
    policy: &'a [String],
    lexical: &'a grammar::LexerPrediction,
    retained: bool,
    document: bool,
    furthest: usize,
    /// Nested decision selections over the machine's retained tokens.
    nested: &'a mut NestedDecisions,
    /// Live predictions in progress that resolve deferred nested decisions,
    /// and the interrupted chain awaiting resolution, deepest first.
    deferral: usize,
    deferred: Vec<grammar::DeferredNestedDecision>,
    /// One past the last token position any read reached; never restored.
    read: usize,
    /// Whether nested-decision state was recorded.
    stateful: bool,
}
impl<T> PredictionTokens<'_, T> {
    fn read_next(
        &mut self,
        work: &mut Option<usize>,
    ) -> Result<Option<grammar::PredictionSymbol>, String> {
        let result = self.read_body(work);
        self.furthest = self.furthest.max(self.cursor.position);
        self.read = self.read.max(self.cursor.position);
        result
    }
    fn read_body(
        &mut self,
        work: &mut Option<usize>,
    ) -> Result<Option<grammar::PredictionSymbol>, String> {
        if self.retained {
            self.cursor.skip_hidden(
                self.tokens,
                |token| {
                    if self.document && token.lexical_alternative.is_none() {
                        Ok(true)
                    } else {
                        hidden_token(token, self.policy, self.lexical)
                    }
                },
                work,
            )?;
        }
        if let Some(token) = self.tokens.get(self.cursor.position) {
            self.cursor.position += 1;
            let alternative = token
                .lexical_alternative
                .ok_or("Missing lexical token classification")?;
            Ok(Some(match self.lexical.alternative(alternative)? {
                grammar::LexicalAlternative::Keyword(value) => {
                    grammar::PredictionSymbol::Keyword(value.clone())
                }
                grammar::LexicalAlternative::Terminal(value) => {
                    grammar::PredictionSymbol::Terminal(value.clone())
                }
            }))
        } else if self.cursor.position == self.tokens.len() {
            self.cursor.position += 1;
            Ok(Some(grammar::PredictionSymbol::End))
        } else {
            Ok(None)
        }
    }
}
impl<T> grammar::PredictionInput for PredictionTokens<'_, T> {
    fn viability(&self, key: &grammar::ViabilityKey) -> Option<bool> {
        self.nested.viability.get(key).copied()
    }
    fn record_viability(&mut self, keys: Vec<grammar::ViabilityKey>, accepted: bool) {
        self.stateful = true;
        for key in keys {
            self.nested.viability.insert(key, accepted);
        }
    }
    fn begin_nested_deferral(&mut self) -> bool {
        self.deferral += 1;
        true
    }
    fn end_nested_deferral(&mut self) {
        self.deferral = self.deferral.saturating_sub(1);
    }
    fn defer_nested_decision(&mut self, deferred: grammar::DeferredNestedDecision) -> bool {
        if self.deferral == 0 {
            return false;
        }
        self.deferred.push(deferred);
        true
    }
    fn take_deferred_nested_decisions(&mut self) -> Vec<grammar::DeferredNestedDecision> {
        std::mem::take(&mut self.deferred)
    }
    fn replay_position(&self) -> Option<usize> {
        Some(self.cursor.position)
    }
    fn lookahead_mark(&self) -> Option<usize> {
        Some(self.furthest)
    }
    fn restore_lookahead(&mut self, mark: usize) {
        self.furthest = mark;
    }
    fn nested_decision_outcome(&self, key: &grammar::NestedDecisionKey) -> Option<Option<usize>> {
        self.nested.selections.get(key).copied()
    }
    fn record_nested_decision_outcome(
        &mut self,
        key: grammar::NestedDecisionKey,
        selected: Option<usize>,
    ) {
        self.stateful = true;
        self.nested.selections.insert(key, selected);
    }
    fn checkpoint(&self) -> Result<usize, String> {
        Ok(self.cursor.position)
    }
    fn rewind(&mut self, checkpoint: usize) -> Result<(), String> {
        if checkpoint > self.tokens.len().saturating_add(1) {
            return Err("Prediction checkpoint escapes retained source tokens".into());
        }
        self.cursor.position = checkpoint;
        Ok(())
    }
    fn read(
        &mut self,
        work: &mut Option<usize>,
    ) -> Result<Option<grammar::PredictionSymbol>, String> {
        self.read_next(work)
    }
}
// Transient observed-only adapter. No field is added to Machine/PredictionTokens.
struct RuntimeAuditedInput<'a, I, P, G> {
    input: &'a mut I,
    phase: P,
    geometry: G,
}
impl<I: grammar::PredictionInput, P, G> grammar::PredictionInput
    for RuntimeAuditedInput<'_, I, P, G>
where
    P: FnMut(&'static str, Option<usize>),
    G: FnMut(grammar::RuntimeWorkGeometry),
{
    fn runtime_work_diagnostics(&self) -> bool {
        true
    }
    fn runtime_work_snapshot(&mut self, phase: &'static str, remaining: Option<usize>) {
        (self.phase)(phase, remaining);
    }
    fn runtime_work_geometry(&mut self, event: grammar::RuntimeWorkGeometry) {
        (self.geometry)(event);
    }
    fn replay_position(&self) -> Option<usize> {
        self.input.replay_position()
    }
    fn lookahead_mark(&self) -> Option<usize> {
        self.input.lookahead_mark()
    }
    fn restore_lookahead(&mut self, mark: usize) {
        self.input.restore_lookahead(mark);
    }
    fn viability(&self, key: &grammar::ViabilityKey) -> Option<bool> {
        self.input.viability(key)
    }
    fn record_viability(&mut self, keys: Vec<grammar::ViabilityKey>, accepted: bool) {
        self.input.record_viability(keys, accepted);
    }
    fn begin_nested_deferral(&mut self) -> bool {
        self.input.begin_nested_deferral()
    }
    fn end_nested_deferral(&mut self) {
        self.input.end_nested_deferral();
    }
    fn defer_nested_decision(&mut self, deferred: grammar::DeferredNestedDecision) -> bool {
        self.input.defer_nested_decision(deferred)
    }
    fn take_deferred_nested_decisions(&mut self) -> Vec<grammar::DeferredNestedDecision> {
        self.input.take_deferred_nested_decisions()
    }
    fn nested_decision_outcome(&self, key: &grammar::NestedDecisionKey) -> Option<Option<usize>> {
        self.input.nested_decision_outcome(key)
    }
    fn record_nested_decision_outcome(
        &mut self,
        key: grammar::NestedDecisionKey,
        selected: Option<usize>,
    ) {
        self.input.record_nested_decision_outcome(key, selected);
    }
    fn read(
        &mut self,
        work: &mut Option<usize>,
    ) -> Result<Option<grammar::PredictionSymbol>, String> {
        self.input.read(work)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        self.input.checkpoint()
    }
    fn rewind(&mut self, mark: usize) -> Result<(), String> {
        self.input.rewind(mark)
    }
}
impl<'a, S: ModelStore, C: draft::ParseContext<S>> Machine<'a, '_, S, C> {
    fn previous_token(&self) -> Option<&Token<Cow<'a, str>>> {
        let index = if self.retained_tokens {
            self.cursor.last_matched?
        } else {
            self.cursor.position.saturating_sub(1)
        };
        self.tokens.get(index)
    }
    // NativeV71 reuses the construction slot as the owning rule's retained
    // token entry. Local actions keep that rule scope; delegated values retain
    // their callee's finalized interval. These are charged bounded lookups,
    // not a second token walk or allocator-byte accounting.
    fn has_owned_rule_spans(&self) -> bool {
        self.package.grammar().is_some_and(|g| {
            matches!(
                g.profile,
                grammar::Profile::NativeV71
                    | grammar::Profile::NativeV72
                    | grammar::Profile::NativeV73
                    | grammar::Profile::NativeV74
                    | grammar::Profile::NativeV75
                    | grammar::Profile::NativeV76
                    | grammar::Profile::NativeV77
                    | grammar::Profile::NativeV78
                    | grammar::Profile::NativeV79
                    | grammar::Profile::NativeV80
                    | grammar::Profile::NativeV81
                    | grammar::Profile::NativeV82
            )
        })
    }
    fn construction_offset(&mut self, entry: Option<usize>) -> Result<usize, ParseError> {
        self.step()?;
        let entry = entry
            .ok_or_else(|| ParseError::Storage("Missing owned rule construction context".into()))?;
        if entry > self.tokens.len() {
            return Err(ParseError::Storage(
                "Owned rule entry escapes retained source tokens".into(),
            ));
        }
        // Preserve the existing provisional EOF offset. Final publication uses
        // the actual last matched token, including a zero-token rule interval.
        Ok(self.tokens.get(entry).map_or(0, |token| token.start))
    }
    fn owned_rule_interval(&mut self, entry: Option<usize>) -> Result<(usize, usize), ParseError> {
        self.step()?;
        let entry =
            entry.ok_or_else(|| ParseError::Storage("Missing owned rule span context".into()))?;
        if entry > self.tokens.len() {
            return Err(ParseError::Storage(
                "Owned rule entry escapes retained source tokens".into(),
            ));
        }
        let Some(last) = self.cursor.last_matched else {
            return Ok((0, 0));
        };
        let last_token = self.tokens.get(last).ok_or_else(|| {
            ParseError::Storage("Owned rule last match escapes retained source tokens".into())
        })?;
        if last < entry {
            return Ok((last_token.end, last_token.end));
        }
        let first_token = self
            .tokens
            .get(entry)
            .ok_or_else(|| ParseError::Storage("Missing owned rule first token".into()))?;
        if first_token.start > last_token.end {
            return Err(ParseError::Storage(
                "Owned rule source interval is inverted".into(),
            ));
        }
        Ok((first_token.start, last_token.end))
    }
    fn skip_hidden(&mut self) -> Result<(), ParseError> {
        if !self.retained_tokens {
            return Ok(());
        }
        let lexical = self
            .package
            .grammar()
            .and_then(|grammar| grammar.lexical_prediction.as_ref())
            .ok_or_else(|| ParseError::Storage("Missing retained lexical program".into()))?;
        let mut remaining = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps));
        let before = remaining;
        let document = self.document.is_some();
        let result = self.cursor.skip_hidden(
            &self.tokens,
            |token| {
                if document && token.lexical_alternative.is_none() {
                    Ok(true)
                } else {
                    hidden_token(token, self.hidden_policy, lexical)
                }
            },
            &mut remaining,
        );
        if let (Some(before), Some(after)) = (before, remaining) {
            self.steps = self.steps.saturating_add(before - after);
        }
        result.map_err(|error| {
            if error.contains("budget") {
                ParseError::Budget(error)
            } else {
                ParseError::Storage(error)
            }
        })
    }
    fn enter_hidden(
        &mut self,
        policy: Option<&'a [String]>,
    ) -> Result<Option<&'a [String]>, ParseError> {
        if !self.retained_tokens {
            return Ok(None);
        }
        // Upstream setHiddenTokens observes LT(1) under the OLD policy first.
        self.skip_hidden()?;
        Ok(policy.map(|policy| std::mem::replace(&mut self.hidden_policy, policy)))
    }
    fn restore_hidden(&mut self, previous: Option<&'a [String]>) -> Result<(), ParseError> {
        let Some(previous) = previous else {
            return Ok(());
        };
        // Always restore the parent, including a denied child-trivia admission.
        let result = self.skip_hidden();
        self.hidden_policy = previous;
        result
    }
    fn assignment_owner(&mut self, owner: &mut Option<Current>) -> Result<usize, ParseError> {
        if owner.is_none()
            && self.package.grammar().is_some_and(|g| {
                matches!(
                    g.profile,
                    grammar::Profile::NativeV28
                        | grammar::Profile::NativeV29
                        | grammar::Profile::NativeV30
                        | grammar::Profile::NativeV31
                        | grammar::Profile::NativeV32
                        | grammar::Profile::NativeV33
                        | grammar::Profile::NativeV34
                        | grammar::Profile::NativeV35
                        | grammar::Profile::NativeV36
                        | grammar::Profile::NativeV37
                        | grammar::Profile::NativeV38
                        | grammar::Profile::NativeV39
                        | grammar::Profile::NativeV40
                        | grammar::Profile::NativeV41
                        | grammar::Profile::NativeV42
                        | grammar::Profile::NativeV43
                        | grammar::Profile::NativeV44
                        | grammar::Profile::NativeV45
                        | grammar::Profile::NativeV46
                        | grammar::Profile::NativeV47
                        | grammar::Profile::NativeV48
                        | grammar::Profile::NativeV49
                        | grammar::Profile::NativeV50
                        | grammar::Profile::NativeV51
                        | grammar::Profile::NativeV52
                        | grammar::Profile::NativeV53
                        | grammar::Profile::NativeV54
                        | grammar::Profile::NativeV55
                        | grammar::Profile::NativeV56
                        | grammar::Profile::NativeV57
                        | grammar::Profile::NativeV58
                        | grammar::Profile::NativeV59
                        | grammar::Profile::NativeV60
                        | grammar::Profile::NativeV61
                        | grammar::Profile::NativeV62
                        | grammar::Profile::NativeV63
                        | grammar::Profile::NativeV64
                        | grammar::Profile::NativeV65
                        | grammar::Profile::NativeV66
                        | grammar::Profile::NativeV67
                        | grammar::Profile::NativeV68
                        | grammar::Profile::NativeV69
                        | grammar::Profile::NativeV70
                        | grammar::Profile::NativeV71
                        | grammar::Profile::NativeV72
                        | grammar::Profile::NativeV73
                        | grammar::Profile::NativeV74
                        | grammar::Profile::NativeV75
                        | grammar::Profile::NativeV76
                        | grammar::Profile::NativeV77
                        | grammar::Profile::NativeV78
                        | grammar::Profile::NativeV79
                        | grammar::Profile::NativeV80
                        | grammar::Profile::NativeV81
                        | grammar::Profile::NativeV82
                )
            })
        {
            let owned_start = if self.has_owned_rule_spans() {
                Some(self.construction_offset(self.construction.map(|(_, entry)| entry))?)
            } else {
                None
            };
            let (class, start) = self.construction.ok_or_else(|| {
                ParseError::Storage("Missing implicit construction context".into())
            })?;
            *owner = Some(Current::local(
                self.create_current(class, owned_start.unwrap_or(start))?,
            ));
        }
        owner
            .as_ref()
            .map(|current| current.index)
            .ok_or_else(|| ParseError::Storage("Assignment requires a current object".into()))
    }
    /// `{uri}#{index}`. The spelling depends only on this machine's resource
    /// and the index, so recent ones are shared rather than rebuilt.
    fn identity(&self, index: usize) -> std::rc::Rc<str> {
        if let Ok(mut recent) = self.nested_decisions.reuse.identities.try_borrow_mut() {
            let slot = &mut recent[index % 8];
            if let Some((known, identity)) = slot
                && *known == index
            {
                return identity.clone();
            }
            let identity: std::rc::Rc<str> = self.spell_identity(index).into();
            *slot = Some((index, identity.clone()));
            return identity;
        }
        self.spell_identity(index).into()
    }
    fn spell_identity(&self, index: usize) -> String {
        let mut digits = [0u8; 20];
        let mut at = digits.len();
        let mut rest = index;
        loop {
            at -= 1;
            digits[at] = b'0' + (rest % 10) as u8;
            rest /= 10;
            if rest == 0 {
                break;
            }
        }
        let mut id = String::with_capacity(self.uri.len() + 1 + digits.len() - at);
        id.push_str(self.uri);
        id.push('#');
        id.extend(digits[at..].iter().map(|digit| char::from(*digit)));
        id
    }
    /// Parser state only, for an attempt whose failure always propagates to
    /// an enclosing checkpoint. Outside document recovery, a failed
    /// exactly-once or predicted expression returns its error (see
    /// `finish_expr_attempt`) to the nearest unpredicted optional, repeated
    /// or alternative attempt, or to the resource transaction, which rolls
    /// the store back to an earlier state; nothing between reads or writes
    /// the store. A nested store frame would therefore be
    /// merged into, or discarded with, that outer frame unobserved.
    fn propagating_checkpoint(&self) -> Checkpoint {
        Checkpoint {
            document: None,
            storage: None,
            text: self.datatype_text.as_ref().map(DatatypeText::checkpoint),
            cursor: self.cursor,
            datatype_span: self.datatype_span,
            elements: self.elements,
            undo: self.undo.len(),
            pending: self.pending.len(),
            depth: self.depth,
        }
    }
    fn checkpoint(&mut self) -> Result<Checkpoint, ParseError> {
        let document = self
            .document
            .as_mut()
            .map(|document| document.mark())
            .transpose()
            .map_err(ParseError::Storage)?;
        let storage = match self.store.begin() {
            Ok(storage) => storage,
            Err(error) => {
                if let (Some(document), Some(mark)) = (self.document.as_mut(), document) {
                    document.rollback(mark).map_err(ParseError::Storage)?;
                }
                return Err(ParseError::Storage(error));
            }
        };
        Ok(Checkpoint {
            document,
            storage: Some(storage),
            text: self.datatype_text.as_ref().map(DatatypeText::checkpoint),
            cursor: self.cursor,
            datatype_span: self.datatype_span,
            elements: self.elements,
            undo: self.undo.len(),
            pending: self.pending.len(),
            depth: self.depth,
        })
    }
    fn commit_checkpoint(&mut self, checkpoint: Checkpoint) -> Result<(), ParseError> {
        if let (Some(document), Some(mark)) = (self.document.as_ref(), checkpoint.document.as_ref())
        {
            document.check_mark(mark).map_err(ParseError::Storage)?;
        }
        let Some(storage) = &checkpoint.storage else {
            return Ok(());
        };
        if let Err(error) = self.store.commit(storage) {
            self.store.rollback(storage).map_err(ParseError::Storage)?;
            if let (Some(document), Some(mark)) = (self.document.as_mut(), checkpoint.document) {
                document.rollback(mark).map_err(ParseError::Storage)?;
            }
            return Err(ParseError::Storage(error));
        }
        if let (Some(document), Some(mark)) = (self.document.as_mut(), checkpoint.document) {
            document.commit(mark).map_err(ParseError::Storage)?;
        }
        Ok(())
    }
    fn rollback(&mut self, checkpoint: Checkpoint) -> Result<(), ParseError> {
        match (self.datatype_text.as_mut(), checkpoint.text) {
            (Some(text), Some(previous)) => text.rollback(previous).map_err(ParseError::Storage)?,
            (None, None) => {}
            _ => {
                return Err(ParseError::Storage(
                    "Invalid datatype parsing context".into(),
                ));
            }
        }
        if let Some(storage) = &checkpoint.storage {
            self.store.rollback(storage).map_err(ParseError::Storage)?;
        }
        while self.undo.len() > checkpoint.undo {
            let undo = self
                .undo
                .pop()
                .ok_or_else(|| ParseError::Storage("Missing parser undo entry".into()))?;
            self.set_scalar_pending(undo.owner, undo.field, undo.unresolved)?;
        }
        if let (Some(document), Some(mark)) = (self.document.as_mut(), checkpoint.document) {
            document.rollback(mark).map_err(ParseError::Storage)?;
        }
        self.elements = checkpoint.elements;
        self.scalar_pending
            .retain(|(owner, _), _| *owner < checkpoint.elements);
        self.cursor = checkpoint.cursor;
        self.datatype_span = checkpoint.datatype_span;
        self.pending.truncate(checkpoint.pending);
        self.depth = checkpoint.depth;
        Ok(())
    }
    fn set_scalar_pending(
        &mut self,
        owner: usize,
        field: &'a str,
        next: Option<usize>,
    ) -> Result<(), ParseError> {
        if let Some(previous) = self.scalar_pending.remove(&(owner, field)) {
            self.pending
                .get_mut(previous)
                .ok_or_else(|| ParseError::Storage("Invalid parser reference state".into()))?
                .active = false;
        }
        if let Some(next) = next {
            self.pending
                .get_mut(next)
                .ok_or_else(|| ParseError::Storage("Invalid parser reference state".into()))?
                .active = true;
            self.scalar_pending.insert((owner, field), next);
        }
        Ok(())
    }
    fn fail(&mut self, expected: impl Into<String>) -> ParseError {
        let expected = expected.into();
        if self.cursor.position >= self.farthest {
            self.farthest = self.cursor.position;
            self.expected = expected.clone();
        }
        ParseError::Syntax(expected)
    }
    fn parse_location(&self) -> Result<draft::ParseLocation<'a>, ParseError> {
        let token = self.tokens.get(self.cursor.position);
        // Legacy Package neither inspects capture state nor allocates an origin.
        let capture_origin = if C::NEEDS_ADMISSION {
            self.document
                .as_deref()
                .map(document::Capture::active_origin)
                .transpose()
                .map_err(ParseError::Storage)?
                .flatten()
        } else {
            None
        };
        Ok(draft::ParseLocation {
            uri: self.uri,
            start: token.map_or(self.raw_source.len(), |token| token.start),
            end: token.map_or(self.raw_source.len(), |token| token.end),
            path: self.document_origin,
            capture_origin,
        })
    }
    fn admit_assignment(
        &mut self,
        owner: usize,
        feature: &str,
        many: bool,
    ) -> Result<(), ParseError> {
        if !C::NEEDS_ADMISSION {
            return Ok(());
        }
        let remaining = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps))
            .unwrap_or(usize::MAX);
        let mut budget = intersect_model::core::query::QueryBudget::new(remaining);
        let result = self.package.admit(
            self.store,
            &self.identity(owner),
            feature,
            many,
            self.parse_location()?,
            &mut budget,
        );
        self.steps = self.steps.saturating_add(budget.reads);
        result.map_err(ParseError::Storage)
    }
    fn admit_assignment_value(
        &mut self,
        owner: usize,
        feature: &str,
        many: bool,
        value: &Value,
        mut location: draft::ParseLocation<'a>,
    ) -> Result<(), ParseError> {
        if !C::NEEDS_VALUE_ADMISSION {
            return Ok(());
        }
        location.end = self
            .previous_token()
            .map_or(location.end, |token| location.end.max(token.end));
        let remaining = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps))
            .unwrap_or(usize::MAX);
        let mut budget = intersect_model::core::query::QueryBudget::new(remaining);
        let result = self.package.admit_value(
            self.store,
            (&self.identity(owner), feature, many),
            value,
            location,
            &mut budget,
        );
        self.steps = self.steps.saturating_add(budget.reads);
        result.map_err(ParseError::Storage)
    }
    fn audit_machine_work<T>(
        &self,
        phase: &'static str,
        detail: Option<&str>,
        before: usize,
        result: &Result<T, ParseError>,
    ) {
        if C::AUDIT_PREDICTION {
            self.package.audit_machine_work(draft::MachineWorkAudit {
                uri: self.uri,
                phase,
                detail,
                before,
                after: self.steps,
                resource_grant: self.limits.steps_per_resource,
                result: result.as_ref().map(|_| ()).map_err(|error| match error {
                    ParseError::Syntax(message)
                    | ParseError::Budget(message)
                    | ParseError::Storage(message)
                    | ParseError::Ambiguous(message) => message.as_str(),
                }),
            });
        }
    }
    fn create_current(&mut self, class: &str, start: usize) -> Result<usize, ParseError> {
        let before = self.steps;
        self.audit_machine_work(
            "create_current_begin",
            Some(class),
            before,
            &Ok::<(), ParseError>(()),
        );
        let result = self.create_current_body(class, start);
        self.audit_machine_work("create_current", Some(class), before, &result);
        result
    }
    fn create_current_body(&mut self, class: &str, start: usize) -> Result<usize, ParseError> {
        let index = self.elements;
        let id = self.identity(index);
        let request = draft::ParserCreation {
            class,
            id: &id,
            uri: self.uri,
            start,
            remaining: self
                .limits
                .steps_per_resource
                .map(|limit| limit.saturating_sub(self.steps))
                .unwrap_or(usize::MAX),
            conversions: self.conversions,
            initializers: self.initializers,
            location: self.parse_location()?,
        };
        let (result, work, set_span) = self.package.create(self.store, request);
        self.steps = self.steps.saturating_add(work);
        result.map_err(ParseError::Storage)?;
        if set_span {
            self.store
                .set_span(&id, start, start)
                .map_err(ParseError::Storage)?;
        }
        self.elements += 1;
        if let Some(document) = self.document.as_mut() {
            document.associate(index).map_err(ParseError::Storage)?;
        }
        Ok(index)
    }
    fn try_delegated_assignment(
        &mut self,
        owner: usize,
        feature: &str,
        append: bool,
        value: &Value,
        location: Option<draft::ParseLocation<'a>>,
    ) -> Result<bool, ParseError> {
        if !self.package.has_delegated_assignments() {
            return Ok(false);
        }
        let location = match location {
            Some(location) => location,
            None => self.parse_location()?,
        };
        let remaining = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps))
            .unwrap_or(usize::MAX);
        let mut budget = intersect_model::core::query::QueryBudget::new(remaining);
        let id = self.identity(owner);
        let target = self.package.delegated_assignment_target(
            self.store,
            (&id, feature, append),
            value,
            location,
            &mut budget,
        );
        self.steps = self.steps.saturating_add(budget.reads);
        let Some(target) = target.map_err(ParseError::Storage)? else {
            return Ok(false);
        };
        match self.package.physical_metadata() {
            Some(metadata) => storage::set_with_opposite_prepared(
                self.store,
                self.package.schema(),
                metadata,
                &id,
                &target,
                value.clone(),
            ),
            None => storage::set_with_opposite(
                self.store,
                self.package.schema(),
                &id,
                &target,
                value.clone(),
            ),
        }
        .map_err(ParseError::Storage)?;
        Ok(true)
    }
    fn write_parsed_value(
        &mut self,
        id: &str,
        feature: &str,
        many: bool,
        value: Value,
    ) -> Result<(), ParseError> {
        if !C::RECONCILE_PARSED_CONTAINMENT {
            return if many {
                self.store.append_value(id, feature, value)
            } else {
                self.store.set_value(id, feature, Some(value))
            }
            .map_err(ParseError::Storage);
        }
        let remaining = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps))
            .unwrap_or(usize::MAX);
        let mut budget = intersect_model::core::query::QueryBudget::new(remaining);
        let schema = self.package.schema();
        let metadata = self.package.physical_metadata();
        let result = (|| {
            budget.charge()?;
            let actual = self
                .store
                .class(id)?
                .ok_or("Missing parsed value receiver")?;
            budget.charge()?;
            let field = match self.package.feature_index() {
                Some(index) => index.effective_feature(&actual, feature)?.feature,
                None => schema.effective_feature(&actual, feature)?.feature,
            };
            let reconcile =
                field.reference && !field.derived && field.containment && field.opposite.is_some();
            if reconcile && (!many || field.upper == 1) {
                // Only append construction is admitted here. Replacing an old
                // scalar after its raw write would lose the previous endpoint.
                return Err("Unsupported parsed scalar containment reconciliation".into());
            }
            budget.charge()?;
            if let (true, false, Some(metadata), Some(index)) = (
                reconcile,
                field.unsettable,
                metadata,
                self.package.feature_index(),
            ) {
                // Prepared schema: connect only the appended member's inverse.
                let resolved = value.as_str().is_some_and(|id| !id.starts_with('?'));
                if !resolved {
                    return Err(
                        "Parsed containment reconciliation requires canonical resolved references"
                            .into(),
                    );
                }
                return storage::append_with_opposite_prepared(
                    self.store, index, metadata, id, feature, value,
                );
            }
            storage::atomic(self.store, |store| {
                budget.charge()?;
                if many {
                    store.append_value(id, feature, value)?;
                } else {
                    store.set_value(id, feature, Some(value))?;
                }
                if reconcile {
                    let mut current = None;
                    budget.charge()?;
                    let found = store.inspect(id, &mut |object| {
                        budget.charge()?;
                        let value = object.value(feature).ok_or("Missing parsed containment value")?;
                        budget.charge()?;
                        let values = value.collection().ok_or("Parsed containment append requires a collection")?;
                        for atom in values.iter() {
                            budget.charge()?;
                            // Canonical stores expose reference views; JSON-backed
                            // stores (FrozenStore) expose identity strings.
                            let resolved = match atom.reference() {
                                Some(reference) => matches!(reference, storage::ReferenceView::Resolved(_)),
                                None => atom.as_str().is_some_and(|id| !id.starts_with('?')),
                            };
                            if !resolved {
                                return Err("Parsed containment reconciliation requires canonical resolved references".into());
                            }
                        }
                        // Canonical state was checked before this lossy adapter.
                        budget.charge()?;
                        current = Some(value.to_json());
                        Ok(())
                    })?;
                    if !found {
                        return Err("Parsed containment receiver disappeared".into());
                    }
                    budget.charge()?;
                    // Existing write_inner repairs retained members' inverses.
                    // No new inverse walk or language alias is implemented here.
                    let current = current.ok_or("Missing canonical parsed containment value")?;
                    match metadata {
                        Some(metadata) => storage::set_with_opposite_prepared(
                            store, schema, metadata, id, feature, current,
                        )?,
                        None => storage::set_with_opposite(store, schema, id, feature, current)?,
                    }
                }
                Ok(())
            })
        })();
        self.steps = self.steps.saturating_add(budget.reads);
        result.map_err(ParseError::Storage)
    }
    fn current_index(&self, value: &Value) -> Result<usize, ParseError> {
        value
            .as_str()
            .and_then(|id| id.strip_prefix(self.uri))
            .and_then(|tail| tail.strip_prefix('#'))
            .and_then(|index| index.parse().ok())
            .filter(|index| *index < self.elements)
            .ok_or_else(|| ParseError::Storage("Invalid current-object identity".into()))
    }
    fn computed_assignment(
        &mut self,
        owner: usize,
        feature: &str,
        many: bool,
        value: Value,
    ) -> Result<bool, ParseError> {
        if C::CONTEXT_SETTINGS {
            let context = self.package;
            return match context.computed_settings() {
                Some(settings) => {
                    self.computed_assignment_with(settings, owner, feature, many, value)
                }
                None => Ok(false),
            };
        }
        let Some(settings) = self.settings else {
            return Ok(false);
        };
        self.computed_assignment_with(settings, owner, feature, many, value)
    }
    fn computed_assignment_with<D: draft::ParserComputedSettings<S> + ?Sized>(
        &mut self,
        settings: &D,
        owner: usize,
        feature: &str,
        many: bool,
        value: Value,
    ) -> Result<bool, ParseError> {
        let id = self.identity(owner);
        let class = self
            .store
            .class(&id)
            .map_err(ParseError::Storage)?
            .ok_or_else(|| ParseError::Storage("Missing computed owner".into()))?;
        if !settings.supports(&class, feature) {
            return Ok(false);
        }
        let remaining = self
            .limits
            .steps_per_resource
            .map(|n| n.saturating_sub(self.steps))
            .unwrap_or(usize::MAX);
        let mut budget = intersect_model::core::query::QueryBudget::new(remaining);
        let edit = if many {
            storage::ComputedEdit::Append(value)
        } else {
            storage::ComputedEdit::Set(value)
        };
        let result = settings.apply(
            self.store,
            &id,
            (&class, feature),
            edit,
            &mut budget,
            self.conversions,
        );
        self.steps = self.steps.saturating_add(budget.reads);
        result.map_err(ParseError::Storage)?;
        Ok(true)
    }
    fn capture_current(
        &mut self,
        owner: Current,
        class: &str,
        feature: &'a str,
        many: bool,
    ) -> Result<usize, ParseError> {
        let delegated = self.retained_tokens && owner.delegated;
        let owned_span = if self.has_owned_rule_spans() && !delegated {
            Some(self.owned_rule_interval(self.construction.map(|(_, entry)| entry))?)
        } else {
            None
        };
        let owner = owner.index;
        let mut span = None;
        self.store
            .inspect(&self.identity(owner), &mut |object| {
                span = Some((object.start(), object.end()));
                Ok(())
            })
            .map_err(ParseError::Storage)?;
        let (previous_start, previous_end) =
            span.ok_or_else(|| ParseError::Storage("Missing captured object".into()))?;
        let (start, end) = owned_span.unwrap_or_else(|| {
            (
                previous_start,
                self.previous_token()
                    .map_or(previous_start, |token| token.end),
            )
        });
        if !delegated && (start != previous_start || end != previous_end) {
            self.store
                .set_span(&self.identity(owner), start, end)
                .map_err(ParseError::Storage)?;
        }
        let action_start = if self.has_owned_rule_spans() {
            self.construction_offset(self.construction.map(|(_, entry)| entry))?
        } else if self.retained_tokens {
            self.construction.map_or(start, |(_, start)| start)
        } else {
            start
        };
        let next = self.create_current(class, action_start)?;
        self.admit_assignment(next, feature, many)?;
        let captured = Value::String(self.identity(owner).to_string());
        if C::NEEDS_VALUE_ADMISSION {
            if end < start {
                return Err(ParseError::Storage(
                    "Invalid captured object source range".into(),
                ));
            }
            let mut location = self.parse_location()?;
            // Existing captured-object start and last consumed token, not the
            // following lookahead token, define this already parsed value.
            location.start = start;
            location.end = end;
            self.admit_assignment_value(next, feature, many, &captured, location)?;
        }
        if self.computed_assignment(next, feature, many, captured.clone())? {
            return Ok(next);
        }
        self.assignment_undo(next, feature, many)?;
        self.write_parsed_value(&self.identity(next), feature, many, captured)?;
        if !many {
            self.set_scalar_pending(next, feature, None)?;
        }
        Ok(next)
    }
    fn document_symbol(&self, index: usize) -> Result<grammar::PredictionSymbol, ParseError> {
        let Some(token) = self.tokens.get(index) else {
            return Ok(grammar::PredictionSymbol::End);
        };
        let lexical = self
            .package
            .grammar()
            .and_then(|g| g.lexical_prediction.as_ref())
            .ok_or_else(|| ParseError::Storage("Missing document lexical program".into()))?;
        let selected = token
            .lexical_alternative
            .ok_or_else(|| ParseError::Storage("Missing original lexical winner".into()))?;
        Ok(
            match lexical.alternative(selected).map_err(ParseError::Storage)? {
                grammar::LexicalAlternative::Keyword(s) => {
                    grammar::PredictionSymbol::Keyword(s.clone())
                }
                grammar::LexicalAlternative::Terminal(s) => {
                    grammar::PredictionSymbol::Terminal(s.clone())
                }
            },
        )
    }
    fn document_peek_after(&mut self) -> Result<usize, ParseError> {
        let mut cursor = self.cursor;
        cursor.position = cursor
            .position
            .checked_add(1)
            .ok_or_else(|| ParseError::Storage("Document cursor overflow".into()))?;
        let lexical = self
            .package
            .grammar()
            .and_then(|g| g.lexical_prediction.as_ref())
            .ok_or_else(|| ParseError::Storage("Missing document lexical program".into()))?;
        let mut work = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps));
        let before = work;
        let result = cursor.skip_hidden(
            &self.tokens,
            |t| {
                if t.lexical_alternative.is_none() {
                    Ok(true)
                } else {
                    hidden_token(t, self.hidden_policy, lexical)
                }
            },
            &mut work,
        );
        if let (Some(before), Some(after)) = (before, work) {
            self.steps = self.steps.saturating_add(before - after)
        }
        result.map_err(ParseError::Budget)?;
        Ok(cursor.position)
    }
    fn document_follow(
        &mut self,
        path: &str,
        callers: usize,
    ) -> Result<BTreeSet<grammar::PredictionSymbol>, ParseError> {
        let program = self
            .package
            .grammar()
            .and_then(|g| g.parser_prediction.as_ref())
            .ok_or_else(|| ParseError::Storage("Missing document prediction program".into()))?;
        let document = self
            .document
            .as_ref()
            .ok_or_else(|| ParseError::Storage("Missing document capture".into()))?;
        let callers = document
            .calls
            .get(..callers)
            .ok_or_else(|| ParseError::Storage("Invalid document return context".into()))?
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let mut work = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps));
        let before = work;
        let result = program.recovery_symbols_with_work(path, &callers, &mut work);
        if let (Some(before), Some(after)) = (before, work) {
            self.steps = self.steps.saturating_add(before - after)
        }
        result.map_err(|e| {
            if e.contains("budget") || e.contains("depth") {
                ParseError::Budget(e)
            } else {
                ParseError::Storage(e)
            }
        })
    }
    fn document_expected(symbol: &grammar::PredictionSymbol) -> String {
        match symbol {
            grammar::PredictionSymbol::Keyword(s) => format!("'{s}'"),
            grammar::PredictionSymbol::Terminal(s) => {
                format!("RULE_{}", s.rsplit("::").next().unwrap_or(s))
            }
            grammar::PredictionSymbol::End => "EOF".into(),
        }
    }
    fn document_report(
        &mut self,
        message: String,
        expected: Option<String>,
        recovery: document::Recovery,
        force: bool,
    ) -> Result<(), ParseError> {
        let at = self.cursor.position;
        let start = self.document_at(at);
        let end = self
            .document
            .as_ref()
            .and_then(|d| d.token_range(at))
            .map_or(start, |r| r.end);
        self.document
            .as_mut()
            .ok_or_else(|| ParseError::Storage("Missing document report context".into()))?
            .report(
                message,
                intersect_model::core::text_document::TextRange { start, end },
                expected,
                recovery,
                force,
            )
            .map_err(ParseError::Storage)
    }
    fn document_conversion_failure(
        &mut self,
        start: usize,
        message: String,
        target: document::ConversionNode,
    ) -> Result<Value, ParseError> {
        let end = self
            .cursor
            .position
            .checked_sub(1)
            .ok_or_else(|| ParseError::Storage("Conversion has no committed token".into()))?;
        let capture = self
            .document
            .as_mut()
            .ok_or_else(|| ParseError::Storage("Missing conversion document context".into()))?;
        let first = capture
            .token_range(start)
            .ok_or_else(|| ParseError::Storage("Missing first conversion source token".into()))?;
        let last = capture
            .token_range(end)
            .ok_or_else(|| ParseError::Storage("Missing final conversion source token".into()))?;
        capture
            .conversion_issue(
                message,
                intersect_model::core::text_document::TextRange {
                    start: first.start,
                    end: last.end,
                },
                target,
            )
            .map_err(ParseError::Storage)?;
        Ok(Value::Null)
    }
    fn document_discard(&mut self) -> Result<(), ParseError> {
        self.step()?;
        let index = self.cursor.position;
        if index >= self.tokens.len() {
            return Ok(());
        }
        self.document_flush(index)?;
        let key = match self.document_symbol(index)? {
            grammar::PredictionSymbol::Terminal(name) => Some(format!("terminal/{name}")),
            _ => None,
        };
        let previous = self.document_feature.take();
        let result = self.document_leaf(index, key.as_deref(), false);
        self.document_feature = previous;
        result?;
        self.cursor.position += 1;
        self.skip_hidden()
    }
    /// True consumes an actually observed unwanted token and retries the real
    /// match; false inserts a logical missing token with no synthetic source leaf.
    fn document_recover_token(
        &mut self,
        expected: grammar::PredictionSymbol,
    ) -> Result<bool, ParseError> {
        let index = self.cursor.position;
        let label = Self::document_expected(&expected);
        let following = if index < self.tokens.len() {
            Some(self.document_peek_after()?)
        } else {
            None
        };
        let unwanted = if let Some(at) = following {
            self.document_symbol(at)? == expected
        } else {
            false
        };
        if unwanted {
            let message = self
                .document
                .as_mut()
                .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
                .token_message(index, "extraneous input '", &format!("' expecting {label}"))
                .map_err(ParseError::Storage)?;
            self.document_report(message, Some(label), document::Recovery::Delete, false)?;
            self.document_discard()?;
            self.document
                .as_mut()
                .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
                .recovered_match = true;
            return Ok(true);
        }
        // The generated outer wrapper uses FOLLOW_2={EOR}; with no live
        // caller frames ANTLR keeps EOR, admitting missing EOF at any LA(1).
        // Deletion above still takes precedence. Inner rule end follows remain
        // the exact actual call-return context, never a grammar-wide union.
        if expected == grammar::PredictionSymbol::End {
            let message = self
                .document
                .as_mut()
                .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
                .token_message(index, "missing EOF at '", "'")
                .map_err(ParseError::Storage)?;
            self.document_report(message, Some(label), document::Recovery::Insert, false)?;
            return Ok(false);
        }
        if index < self.tokens.len() {
            let path = self.document_origin.ok_or_else(|| {
                ParseError::Storage("Missing original mismatch occurrence".into())
            })?;
            let calls = self
                .document
                .as_ref()
                .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
                .calls
                .len();
            if self
                .document_follow(path, calls)?
                .contains(&self.document_symbol(index)?)
            {
                let message = self
                    .document
                    .as_mut()
                    .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
                    .token_message(index, &format!("missing {label} at '"), "'")
                    .map_err(ParseError::Storage)?;
                self.document_report(message, Some(label), document::Recovery::Insert, false)?;
                return Ok(false);
            }
        }
        let message = self
            .document
            .as_mut()
            .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
            .token_message(index, "mismatched input '", &format!("' expecting {label}"))
            .map_err(ParseError::Storage)?;
        Err(self.fail(message))
    }
    fn document_recover_rule(&mut self, message: String) -> Result<(), ParseError> {
        let message = if message == "No viable predicted alternative" {
            self.document
                .as_mut()
                .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
                .token_message(
                    self.cursor.position,
                    "no viable alternative at input '",
                    "'",
                )
                .map_err(ParseError::Storage)?
        } else {
            message
        };
        self.document_report(message, None, document::Recovery::Resynchronize, true)?;
        let previous = self
            .document
            .as_ref()
            .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
            .last_error_index;
        if previous == Some(self.cursor.position) && self.cursor.position < self.tokens.len() {
            self.document_discard()?;
        }
        self.document
            .as_mut()
            .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
            .last_error_index = Some(self.cursor.position);
        let program = self
            .package
            .grammar()
            .and_then(|g| g.parser_prediction.as_ref())
            .ok_or_else(|| ParseError::Storage("Missing document prediction program".into()))?;
        let calls = self
            .document
            .as_ref()
            .ok_or_else(|| ParseError::Storage("Missing document recovery context".into()))?
            .calls
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let mut work = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps));
        let before = work;
        let mut follow = BTreeSet::from([grammar::PredictionSymbol::End]);
        let result = (|| -> Result<(), String> {
            for (index, path) in calls.iter().enumerate() {
                follow.extend(program.recovery_symbols_with_work(
                    path,
                    &calls[..index],
                    &mut work,
                )?);
            }
            Ok(())
        })();
        if let (Some(before), Some(after)) = (before, work) {
            self.steps = self.steps.saturating_add(before - after)
        }
        result.map_err(|e| {
            if e.contains("budget") || e.contains("depth") {
                ParseError::Budget(e)
            } else {
                ParseError::Storage(e)
            }
        })?;
        while !follow.contains(&self.document_symbol(self.cursor.position)?) {
            self.document_discard()?;
        }
        Ok(())
    }
    fn document_at(&self, index: usize) -> usize {
        if self.document.is_none() {
            return 0;
        }
        self.utf16_positions
            .get(index)
            .copied()
            .unwrap_or_else(|| self.raw_source.encode_utf16().count())
    }
    fn document_leaf(
        &mut self,
        index: usize,
        origin: Option<&str>,
        hidden: bool,
    ) -> Result<(), ParseError> {
        if self.document.is_none() {
            return Ok(());
        }
        self.tokens
            .get(index)
            .ok_or_else(|| ParseError::Storage("Missing document lexical record".into()))?;
        let range = self
            .document
            .as_ref()
            .and_then(|d| d.token_range(index))
            .ok_or_else(|| {
                ParseError::Storage("Missing exact document source-unit range".into())
            })?;
        let document = self
            .document
            .as_mut()
            .ok_or_else(|| ParseError::Storage("Missing document capture".into()))?;
        let node = if let Some(node) = document
            .lexical_leaf(index, range)
            .map_err(ParseError::Storage)?
        {
            node
        } else {
            document
                .leaf(origin, range, hidden)
                .map_err(ParseError::Storage)?
        };
        if !hidden && let Some((owner, feature)) = self.document_feature {
            document
                .feature(&format!("{}#{owner}", self.uri), feature, node)
                .map_err(ParseError::Storage)?;
        }
        document.emitted = index + 1;
        Ok(())
    }
    fn document_flush(&mut self, until: usize) -> Result<(), ParseError> {
        while let Some(index) = self
            .document
            .as_ref()
            .map(|d| d.emitted)
            .filter(|index| *index < until)
        {
            let token = self
                .tokens
                .get(index)
                .ok_or_else(|| ParseError::Storage("Missing retained document trivia".into()))?;
            let key = token
                .lexical_alternative
                .and_then(|index| {
                    self.package
                        .grammar()?
                        .lexical_prediction
                        .as_ref()?
                        .alternative(index)
                        .ok()
                })
                .and_then(|a| {
                    if let grammar::LexicalAlternative::Terminal(name) = a {
                        Some(format!("terminal/{name}"))
                    } else {
                        None
                    }
                });
            self.document_leaf(index, key.as_deref(), true)?;
        }
        Ok(())
    }
    fn document_append_all(&mut self) -> Result<(), ParseError> {
        if self.document.is_none() {
            return Ok(());
        }
        while let Some(index) = self
            .document
            .as_ref()
            .map(|d| d.emitted)
            .filter(|i| *i < self.tokens.len())
        {
            let token = &self.tokens[index];
            let lexical = self
                .package
                .grammar()
                .and_then(|g| g.lexical_prediction.as_ref())
                .ok_or_else(|| ParseError::Storage("Missing document lexical program".into()))?;
            let hidden = if token.lexical_alternative.is_none() {
                true
            } else {
                hidden_token(token, self.hidden_policy, lexical).map_err(ParseError::Storage)?
            };
            let key = token
                .lexical_alternative
                .and_then(|i| lexical.alternative(i).ok())
                .and_then(|a| {
                    if let grammar::LexicalAlternative::Terminal(name) = a {
                        Some(format!("terminal/{name}"))
                    } else {
                        None
                    }
                });
            self.document_leaf(index, key.as_deref(), hidden)?;
        }
        self.cursor.position = self.tokens.len();
        Ok(())
    }
    fn document_match(&mut self, index: usize) -> Result<(), ParseError> {
        if self.document.is_none() {
            return Ok(());
        }
        self.document
            .as_mut()
            .ok_or_else(|| ParseError::Storage("Missing document match context".into()))?
            .matched();
        self.document_flush(index)?;
        self.document_leaf(index, self.document_origin, false)
    }
    fn reconcile_live_work(
        &mut self,
        before: Option<usize>,
        after: Option<usize>,
    ) -> Result<(), ParseError> {
        match (before, after) {
            (Some(before), Some(after)) => {
                let used = before.checked_sub(after).ok_or_else(|| {
                    ParseError::Budget("Live caller hook expanded the original work pool".into())
                })?;
                self.steps = self.steps.checked_add(used).ok_or_else(|| {
                    ParseError::Budget("Live caller work accounting overflow".into())
                })?;
                Ok(())
            }
            (None, None) => Ok(()),
            _ => Err(ParseError::Budget(
                "Live caller hook changed the original finite-pool association".into(),
            )),
        }
    }
    fn live_entry_event(
        &mut self,
        entry: &str,
        begin: bool,
        success: bool,
    ) -> Result<(), ParseError> {
        let mut work = self
            .limits
            .steps_per_resource
            .map(|n| n.saturating_sub(self.steps));
        let before = work;
        let result = self
            .package
            .live_entry(self.uri, entry, begin, success, &mut work);
        self.reconcile_live_work(before, work)?;
        result.map_err(|e| {
            if e.contains("budget") || e.contains("depth") {
                ParseError::Budget(e)
            } else {
                ParseError::Storage(e)
            }
        })
    }
    fn live_call_event(&mut self, expression: &Expr, enter: bool) -> Result<(), ParseError> {
        let mut work = self
            .limits
            .steps_per_resource
            .map(|n| n.saturating_sub(self.steps));
        let before = work;
        let result = self
            .package
            .live_call(self.uri, expression, enter, &mut work);
        self.reconcile_live_work(before, work)?;
        result.map_err(|e| {
            if e.contains("budget") || e.contains("depth") {
                ParseError::Budget(e)
            } else {
                ParseError::Storage(e)
            }
        })
    }
    fn with_live_call<T>(
        &mut self,
        expression: &'a Expr,
        f: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        self.live_call_event(expression, true)?;
        let result = f(self);
        let closed = self.live_call_event(expression, false);
        match (result, closed) {
            (Err(original), _) => Err(original),
            (_, Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(value),
        }
    }
    fn entry(&mut self, name: &str) -> Result<Value, ParseError> {
        if !C::LIVE_CALLERS {
            return self.entry_literal(name);
        }
        self.live_entry_event(name, true, true)?;
        let result = self.entry_literal(name);
        let closed = self.live_entry_event(name, false, result.is_ok());
        match (result, closed) {
            (Err(original), _) => Err(original),
            (_, Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(value),
        }
    }
    fn entry_literal(&mut self, name: &str) -> Result<Value, ParseError> {
        let at = self.document_at(0);
        let node = self
            .document
            .as_mut()
            .map(|d| d.begin(&format!("rule/{name}/declaration"), at))
            .transpose()
            .map_err(ParseError::Storage)?;
        let result = self.entry_body(name);
        if result.is_ok() {
            self.document_append_all()?;
        } else if self.document.is_some()
            && let Err(ParseError::Syntax(message)) = &result
        {
            self.document_recover_rule(message.clone())?;
        }
        if let (Some(document), Some(node)) = (self.document.as_mut(), node) {
            document.close(node).map_err(ParseError::Storage)?;
        }
        result
    }
    fn entry_body(&mut self, name: &str) -> Result<Value, ParseError> {
        if !self.retained_tokens {
            return self.rule(name);
        }
        let policy = self
            .package
            .grammar()
            .and_then(|grammar| grammar.rules.get(name))
            .and_then(|rule| rule.hidden.as_deref());
        // Generated entryRule and rule both install explicit hidden clauses.
        // The outer entry policy remains active through its EOF match.
        let previous = self.enter_hidden(policy)?;
        let result = self.rule(name).and_then(|value| {
            self.skip_hidden()?;
            if self.cursor.position != self.tokens.len() {
                if self.document.is_some() {
                    self.document_recover_token(grammar::PredictionSymbol::End)?;
                    return Ok(value);
                }
                return Err(self.fail("Unexpected trailing input"));
            }
            Ok(value)
        });
        // An ambiguous prediction already recorded its own decision token.
        if result.is_err() && !matches!(result, Err(ParseError::Ambiguous(_))) {
            self.diagnostic_position = Some(self.farthest.max(self.cursor.position));
        }
        self.restore_hidden(previous)?;
        result
    }
    fn rule(&mut self, name: &str) -> Result<Value, ParseError> {
        if self
            .package
            .grammar()
            .is_some_and(|g| g.profile.has_constant_guards())
        {
            self.rule_with_direct_dispatch(name)
        } else {
            self.rule_literal(name)
        }
    }
    fn rule_literal(&mut self, name: &str) -> Result<Value, ParseError> {
        let policy = self
            .package
            .grammar()
            .and_then(|grammar| grammar.rules.get(name))
            .and_then(|rule| rule.hidden.as_deref());
        let previous = self.enter_hidden(policy)?;
        let result = self.rule_body(name);
        self.restore_hidden(previous)?;
        result
    }
    fn rule_body(&mut self, name: &str) -> Result<Value, ParseError> {
        let undo_start = self.undo.len();
        self.depth += 1;
        if self.depth > self.limits.nesting {
            return Err(ParseError::Budget("Model nesting budget exceeded".into()));
        }
        let package = self.package;
        let grammar = package
            .grammar()
            .ok_or_else(|| ParseError::Syntax("Package has no text grammar".into()))?;
        let rule = grammar
            .rules
            .get(name)
            .ok_or_else(|| ParseError::Syntax("Missing parser rule".into()))?;
        // EObject semantic spans begin at the first significant token under
        // the newly installed policy, not at leading retained trivia.
        self.skip_hidden()?;
        if rule.fragment {
            return Err(ParseError::Syntax(
                "Parser fragment requires a caller object".into(),
            ));
        }
        if rule.datatype.is_some() {
            let datatype_start = self.cursor.position;
            let parent = self.datatype_text.replace(DatatypeText::default());
            let result = self.expr(&rule.body, &mut None);
            let child = self.datatype_text.take();
            self.datatype_text = parent;
            self.depth -= 1;
            if self.document.is_some()
                && let Err(ParseError::Syntax(message)) = &result
            {
                self.document_recover_rule(message.clone())?;
            } else {
                result?;
            }
            let child =
                child.ok_or_else(|| ParseError::Storage("Missing datatype text context".into()))?;
            if let Some(parent) = self.datatype_text.as_mut() {
                parent.merge(&child).map_err(ParseError::Storage)?;
            }
            let first_offset = child.source_start();
            let text = match child.into_text() {
                Some(text) => text,
                None if self.document.is_some() => return Ok(Value::Null),
                None => return Err(self.fail("Nullable datatype parser result is unsupported")),
            };
            if self.retained_tokens {
                let first_offset = first_offset
                    .ok_or_else(|| ParseError::Storage("Missing datatype source start".into()))?;
                let index = self
                    .utf16_positions
                    .binary_search(&first_offset)
                    .map_err(|_| {
                        ParseError::Storage("Datatype source start has no retained token".into())
                    })?;
                let start = self
                    .tokens
                    .get(index)
                    .ok_or_else(|| ParseError::Storage("Missing first datatype token".into()))?
                    .start;
                let end = self
                    .previous_token()
                    .ok_or_else(|| ParseError::Storage("Missing final datatype token".into()))?
                    .end;
                self.datatype_span = Some((start, end));
            }
            if self.raw_entry {
                return Ok(Value::String(text));
            }
            if grammar.profile.has_durable_documents()
                && matches!(
                    rule.datatype,
                    Some(grammar::Datatype::Nominal | grammar::Datatype::Builtin(_))
                )
                && self.datatype_text.is_some()
            {
                // A containing datatype rule consumes this already-merged
                // lexical token text; its own committed return owns conversion.
                return Ok(Value::String(text));
            }
            if rule.datatype == Some(grammar::Datatype::Nominal) {
                if !grammar.profile.has_durable_documents() {
                    return Err(ParseError::Storage(
                        "Nominal datatype execution requires the explicit durable profile".into(),
                    ));
                }
                self.step()?;
                let declaration = rule.nominal_return.as_ref().ok_or_else(|| {
                    ParseError::Storage("Missing exact nominal rule return".into())
                })?;
                let datatype = package
                    .schema()
                    .datatypes
                    .get(&rule.class)
                    .filter(|datatype| datatype.declaration() == declaration)
                    .ok_or_else(|| {
                        ParseError::Storage(
                            "Nominal rule return differs from the current schema datatype".into(),
                        )
                    })?;
                let converted = self
                    .conversions
                    .ok_or_else(|| {
                        "Explicit conversion registry required for datatype rule".to_string()
                    })
                    .and_then(|registry| registry.parse_datatype(datatype, &text));
                return match converted {
                    Ok(value) => Ok(value),
                    Err(error) if self.document.is_some() => self.document_conversion_failure(
                        datatype_start,
                        error,
                        document::ConversionNode::CalledRule,
                    ),
                    Err(error) => {
                        self.cursor.position = datatype_start;
                        Err(self.fail(format!("Invalid {} literal: {error}", datatype.name())))
                    }
                };
            }
            if rule.datatype == Some(grammar::Datatype::EBoolean) {
                if C::HAS_BOOLEAN_RULE_CONVERSIONS {
                    // The committed datatype already consumed its exact tokens.
                    // Lookup/conversion work is debited from this SAME remaining
                    // resource allowance, including error/rollback paths.
                    self.step()?;
                    let mut location = self.parse_location()?;
                    self.step()?;
                    let (start, first_end) = self
                        .tokens
                        .get(datatype_start)
                        .map(|token| (token.start, token.end))
                        .ok_or_else(|| {
                            ParseError::Storage("Missing Boolean datatype start token".into())
                        })?;
                    location.start = start;
                    self.step()?;
                    location.end = self.previous_token().map_or(first_end, |token| token.end);
                    let remaining = self
                        .limits
                        .steps_per_resource
                        .map(|limit| limit.saturating_sub(self.steps))
                        .unwrap_or(usize::MAX);
                    let mut budget = intersect_model::core::query::QueryBudget::new(remaining);
                    let result = package.convert_boolean_rule(name, &text, location, &mut budget);
                    self.steps = self.steps.saturating_add(budget.reads);
                    return result.map_err(ParseError::Storage);
                }
                // Default Package conversion/provider order is unchanged.
                let executable = package.execution_definition().ok_or_else(|| {
                    ParseError::Storage("Unsupported draft Boolean rule conversion".into())
                })?;
                // Charge bounded binding validation and provider lookup before execution.
                for binding in &executable.semantics.rule_conversions {
                    self.step()?;
                    for _ in binding.keywords.keys() {
                        self.step()?;
                    }
                }
                self.step()?;
                return rule_conversion::boolean_definition(executable, name, &text).map_err(|e| {
                    self.cursor.position = datatype_start;
                    self.fail(e)
                });
            }
            if let Some(grammar::Datatype::Builtin(codec)) = rule.datatype {
                if !grammar.profile.has_builtin_scalar_returns() {
                    return Err(ParseError::Storage(
                        "Builtin rule return requires NativeV81".into(),
                    ));
                }
                for _ in 0..text.len().div_ceil(256).max(1) {
                    self.step()?;
                }
                return codec.parse(&text).map_err(|error| {
                    self.cursor.position = datatype_start;
                    self.fail(format!("Invalid {codec:?} rule literal: {error}"))
                });
            }
            if rule.datatype == Some(grammar::Datatype::EDouble) {
                // Reuse the authoritative Ecore IEEE codec, including nonfinite bits.
                for _ in 0..text.len().div_ceil(256).max(1) {
                    self.step()?;
                }
                return intersect_model::definitions::scalars::Builtin::EDouble
                    .parse(&text)
                    .map_err(|e| {
                        self.cursor.position = datatype_start;
                        self.fail(e)
                    });
            }
            if rule.datatype == Some(grammar::Datatype::Enum) {
                let enumeration = self
                    .package
                    .schema()
                    .enumerations
                    .get(&rule.class)
                    .ok_or_else(|| ParseError::Storage("Missing enum rule type".into()))?;
                if matches!(
                    grammar.profile,
                    grammar::Profile::NativeV46
                        | grammar::Profile::NativeV47
                        | grammar::Profile::NativeV48
                        | grammar::Profile::NativeV49
                        | grammar::Profile::NativeV50
                        | grammar::Profile::NativeV51
                        | grammar::Profile::NativeV52
                        | grammar::Profile::NativeV53
                        | grammar::Profile::NativeV54
                        | grammar::Profile::NativeV55
                        | grammar::Profile::NativeV56
                        | grammar::Profile::NativeV57
                        | grammar::Profile::NativeV58
                        | grammar::Profile::NativeV59
                        | grammar::Profile::NativeV60
                        | grammar::Profile::NativeV61
                        | grammar::Profile::NativeV62
                        | grammar::Profile::NativeV63
                        | grammar::Profile::NativeV64
                        | grammar::Profile::NativeV65
                        | grammar::Profile::NativeV66
                        | grammar::Profile::NativeV67
                        | grammar::Profile::NativeV68
                        | grammar::Profile::NativeV69
                        | grammar::Profile::NativeV70
                        | grammar::Profile::NativeV71
                        | grammar::Profile::NativeV72
                        | grammar::Profile::NativeV73
                        | grammar::Profile::NativeV74
                        | grammar::Profile::NativeV75
                        | grammar::Profile::NativeV76
                        | grammar::Profile::NativeV77
                        | grammar::Profile::NativeV78
                        | grammar::Profile::NativeV79
                        | grammar::Profile::NativeV80
                        | grammar::Profile::NativeV81
                        | grammar::Profile::NativeV82
                ) && rule.enum_literals.is_empty()
                {
                    // Ordinary datatype parser rules use the imported enum's
                    // lexical codec. Enum declarations retain keyword bindings.
                    let value = enumeration.parse(&text).map_err(|message| {
                        self.cursor.position = datatype_start;
                        self.fail(message)
                    })?;
                    return serde_json::to_value(value)
                        .map_err(|error| ParseError::Storage(error.to_string()));
                }
                let value = rule
                    .enum_literals
                    .get(&text)
                    .ok_or_else(|| ParseError::Storage("Missing enum keyword binding".into()))?;
                enumeration
                    .validate_value(value)
                    .map_err(ParseError::Storage)?;
                return serde_json::to_value(value)
                    .map_err(|error| ParseError::Storage(error.to_string()));
            }
            if C::HAS_STRING_RULE_CONVERSIONS
                && rule.datatype == Some(grammar::Datatype::EString)
                && self.datatype_text.is_none()
            {
                self.step()?;
                let mut location = self.parse_location()?;
                self.step()?;
                let (start, first_end) = self
                    .tokens
                    .get(datatype_start)
                    .map(|token| (token.start, token.end))
                    .ok_or_else(|| {
                        ParseError::Storage("Missing EString datatype start token".into())
                    })?;
                location.start = start;
                self.step()?;
                location.end = self.previous_token().map_or(first_end, |token| token.end);
                let remaining = self
                    .limits
                    .steps_per_resource
                    .map(|limit| limit.saturating_sub(self.steps))
                    .unwrap_or(usize::MAX);
                let mut budget = intersect_model::core::query::QueryBudget::new(remaining);
                let result = package.convert_string_rule(name, &text, location, &mut budget);
                self.steps = self.steps.saturating_add(budget.reads);
                if let Some(converted) = result.map_err(ParseError::Storage)? {
                    return Ok(Value::String(converted));
                }
            }
            return Ok(Value::String(text));
        }
        if rule.is_dispatch(grammar) {
            // No caller object exists for pure delegation. The selected rule
            // owns construction, initialization, source ordinals and spans.
            let value = if matches!(
                grammar.profile,
                grammar::Profile::NativeV25
                    | grammar::Profile::NativeV26
                    | grammar::Profile::NativeV27
                    | grammar::Profile::NativeV28
                    | grammar::Profile::NativeV29
                    | grammar::Profile::NativeV30
                    | grammar::Profile::NativeV31
                    | grammar::Profile::NativeV32
                    | grammar::Profile::NativeV33
                    | grammar::Profile::NativeV34
                    | grammar::Profile::NativeV35
                    | grammar::Profile::NativeV36
                    | grammar::Profile::NativeV37
                    | grammar::Profile::NativeV38
                    | grammar::Profile::NativeV39
                    | grammar::Profile::NativeV40
                    | grammar::Profile::NativeV41
                    | grammar::Profile::NativeV42
                    | grammar::Profile::NativeV43
                    | grammar::Profile::NativeV44
                    | grammar::Profile::NativeV45
                    | grammar::Profile::NativeV46
                    | grammar::Profile::NativeV47
                    | grammar::Profile::NativeV48
                    | grammar::Profile::NativeV49
                    | grammar::Profile::NativeV50
                    | grammar::Profile::NativeV51
                    | grammar::Profile::NativeV52
                    | grammar::Profile::NativeV53
                    | grammar::Profile::NativeV54
                    | grammar::Profile::NativeV55
                    | grammar::Profile::NativeV56
                    | grammar::Profile::NativeV57
                    | grammar::Profile::NativeV58
                    | grammar::Profile::NativeV59
                    | grammar::Profile::NativeV60
                    | grammar::Profile::NativeV61
                    | grammar::Profile::NativeV62
                    | grammar::Profile::NativeV63
                    | grammar::Profile::NativeV64
                    | grammar::Profile::NativeV65
                    | grammar::Profile::NativeV66
                    | grammar::Profile::NativeV67
                    | grammar::Profile::NativeV68
                    | grammar::Profile::NativeV69
                    | grammar::Profile::NativeV70
                    | grammar::Profile::NativeV71
                    | grammar::Profile::NativeV72
                    | grammar::Profile::NativeV73
                    | grammar::Profile::NativeV74
                    | grammar::Profile::NativeV75
                    | grammar::Profile::NativeV76
                    | grammar::Profile::NativeV77
                    | grammar::Profile::NativeV78
                    | grammar::Profile::NativeV79
                    | grammar::Profile::NativeV80
                    | grammar::Profile::NativeV81
                    | grammar::Profile::NativeV82
            ) {
                let mut current = None;
                let result = self.expr(&rule.body, &mut current);
                self.finish_forwarding_rule(current, result, grammar.profile)?
            } else {
                self.dispatch(&rule.body)?
            };
            self.depth -= 1;
            return Ok(value);
        }
        let mut phase = self.begin_current_rule_phase(rule, grammar, undo_start)?;
        let parsed = self.expr(&rule.body, &mut phase.current);
        self.finish_current_rule_phase(phase, parsed)
    }
    fn begin_current_rule_phase(
        &mut self,
        rule: &'a grammar::Rule,
        grammar: &'a grammar::Grammar,
        undo_start: usize,
    ) -> Result<CurrentRulePhase<'a>, ParseError> {
        let entry_token = self.cursor.position;
        let start = self.tokens.get(entry_token).map_or(0, |t| t.start);
        let current = if grammar.profile.has_constant_guards() || rule.uses_current_flow(grammar) {
            None
        } else {
            Some(Current::local(
                self.create_current(rule.construction_class(), start)?,
            ))
        };
        // Nested rules temporarily replace this context; fragments retain their
        // caller's context. Restore it on both success and failure.
        let construction_entry = if self.has_owned_rule_spans() {
            entry_token
        } else {
            start
        };
        let previous_construction = self
            .construction
            .replace((rule.construction_class(), construction_entry));
        Ok(CurrentRulePhase {
            current,
            previous_construction,
            entry_token,
            start,
            undo_start,
            profile: grammar.profile,
        })
    }
    fn finish_current_rule_phase(
        &mut self,
        phase: CurrentRulePhase<'a>,
        parsed: Result<Value, ParseError>,
    ) -> Result<Value, ParseError> {
        self.construction = phase.previous_construction;
        self.finish_current_rule(
            phase.current,
            parsed,
            phase.entry_token,
            phase.start,
            phase.undo_start,
            phase.profile,
        )
    }
    fn finish_current_rule(
        &mut self,
        current: Option<Current>,
        parsed: Result<Value, ParseError>,
        entry_token: usize,
        start: usize,
        undo_start: usize,
        profile: grammar::Profile,
    ) -> Result<Value, ParseError> {
        if self.document.is_some()
            && let Err(ParseError::Syntax(message)) = &parsed
        {
            self.document_recover_rule(message.clone())?;
        } else {
            parsed?;
        }
        if current.is_none() && profile.has_constant_guards() {
            self.depth -= 1;
            self.undo.truncate(undo_start);
            return Ok(Value::Null);
        }
        let current = current
            .ok_or_else(|| ParseError::Storage("Object rule produced no current object".into()))?;
        let index = current.index;
        let id = self.identity(index);
        // A speculative rule at the end of a repetition must be allowed to fail
        // syntactically without rejecting a model exactly at the element limit.
        if let Some(limit) = self.limits.elements_per_resource
            && self.elements > limit
        {
            return Err(ParseError::Budget(format!(
                "Model element budget exceeded ({limit} per resource)"
            )));
        }
        if !self.retained_tokens || !current.delegated {
            let (start, end) = if self.has_owned_rule_spans() {
                self.owned_rule_interval(Some(entry_token))?
            } else {
                (start, self.previous_token().map_or(start, |t| t.end))
            };
            self.store
                .set_span(&id, start, end)
                .map_err(ParseError::Storage)?;
        }
        self.depth -= 1;
        // A rule owns its newly created objects. Store savepoints restore their
        // model state; reference undo is needed only while the rule is speculative.
        self.undo.truncate(undo_start);
        self.scalar_pending.retain(|(owner, _), _| *owner != index);
        Ok(Value::String(id.to_string()))
    }
    fn finish_forwarding_rule(
        &mut self,
        current: Option<Current>,
        result: Result<Value, ParseError>,
        profile: grammar::Profile,
    ) -> Result<Value, ParseError> {
        if self.document.is_some()
            && let Err(ParseError::Syntax(message)) = &result
        {
            self.document_recover_rule(message.clone())?;
        } else {
            result?;
        }
        match current {
            Some(current) => Ok(Value::String(self.identity(current.index).to_string())),
            None if profile.has_constant_guards() => Ok(Value::Null),
            None => Err(ParseError::Storage(
                "Forwarding rule produced no object".into(),
            )),
        }
    }
    fn dispatch(&mut self, expr: &'a Expr) -> Result<Value, ParseError> {
        self.step()?;
        match &expr.kind {
            Kind::SyntacticPredicate(value) => self.dispatch(value),
            Kind::Call(name) => {
                if C::LIVE_CALLERS {
                    self.with_live_call(expr, |machine| machine.rule(name))
                } else {
                    self.rule(name)
                }
            }
            Kind::Sequence(items) if items.len() == 1 => self.dispatch(&items[0]),
            Kind::Alternative(items) => {
                if items.is_empty() {
                    return Err(self.fail("Boolean guard is false"));
                }
                if expr.prediction_path.is_some() {
                    let selected = self
                        .predict(expr, "choice")?
                        .ok_or_else(|| ParseError::Syntax(self.expected.clone()))?;
                    let item = items.get(selected).ok_or_else(|| {
                        ParseError::Storage("Invalid predicted dispatch alternative".into())
                    })?;
                    return self.dispatch(item);
                }
                for item in items {
                    let checkpoint = self.checkpoint()?;
                    match self.dispatch(item) {
                        Ok(value) => {
                            self.commit_checkpoint(checkpoint)?;
                            return Ok(value);
                        }
                        Err(error) if !matches!(error, ParseError::Syntax(_)) => {
                            self.rollback(checkpoint)?;
                            return Err(error);
                        }
                        Err(_) => self.rollback(checkpoint)?,
                    }
                }
                Err(ParseError::Syntax(self.expected.clone()))
            }
            _ => Err(ParseError::Syntax("Invalid parser dispatch".into())),
        }
    }
    fn predict(&mut self, expr: &Expr, suffix: &str) -> Result<Option<usize>, ParseError> {
        let before = self.steps;
        self.audit_machine_work(
            "predict_begin",
            expr.prediction_path.as_deref(),
            before,
            &Ok::<(), ParseError>(()),
        );
        let result = self.predict_body(expr, suffix);
        self.audit_machine_work("predict", expr.prediction_path.as_deref(), before, &result);
        result
    }
    fn predict_body(&mut self, expr: &Expr, suffix: &str) -> Result<Option<usize>, ParseError> {
        self.skip_hidden()?;
        let path = expr
            .prediction_path
            .as_ref()
            .ok_or_else(|| ParseError::Storage("Missing parser decision path".into()))?;
        let grammar = self
            .package
            .grammar()
            .ok_or_else(|| ParseError::Storage("Missing prediction grammar".into()))?;
        let program = grammar
            .parser_prediction
            .as_ref()
            .ok_or_else(|| ParseError::Storage("Missing parser prediction program".into()))?;
        let lexical = grammar
            .lexical_prediction
            .as_ref()
            .ok_or_else(|| ParseError::Storage("Missing lexical prediction program".into()))?;
        let mut remaining = self
            .limits
            .steps_per_resource
            .map(|limit| limit.saturating_sub(self.steps));
        let before = remaining;
        let mut cursor = self.cursor;
        let prediction_start = cursor.position;
        let tokens = &self.tokens;
        let policy = self.hidden_policy;
        let retained = self.retained_tokens;
        let document = self.document.is_some();
        // With no budget, document, audit or recorded nested decision, the
        // outcome depends only on the classifications the prediction reads.
        let memoizable = !C::LIVE_CALLERS
            && !C::AUDIT_PREDICTION
            && !C::AUDIT_RUNTIME_WORK
            && !document
            && remaining.is_none()
            && matches!(
                grammar.profile,
                grammar::Profile::NativeV74
                    | grammar::Profile::NativeV75
                    | grammar::Profile::NativeV76
                    | grammar::Profile::NativeV77
                    | grammar::Profile::NativeV78
                    | grammar::Profile::NativeV79
                    | grammar::Profile::NativeV80
                    | grammar::Profile::NativeV81
                    | grammar::Profile::NativeV82
            )
            && self.nested_decisions.selections.is_empty()
            && self.nested_decisions.viability.is_empty();
        let expression = std::ptr::from_ref(expr) as usize;
        if memoizable
            && let Some((selected, furthest)) =
                self.nested_decisions
                    .reuse
                    .memo
                    .get(expression, suffix, tokens, prediction_start)
        {
            if selected.is_none() && suffix == "choice" {
                let farthest = furthest.saturating_sub(1).min(tokens.len());
                if farthest >= self.farthest {
                    self.farthest = farthest;
                    self.expected = "No viable predicted alternative".into();
                }
            }
            return Ok(selected);
        }
        let mut input = PredictionTokens {
            cursor,
            tokens,
            policy,
            lexical,
            retained,
            document,
            furthest: cursor.position,
            nested: &mut self.nested_decisions,
            deferral: 0,
            deferred: Vec::new(),
            read: cursor.position,
            stateful: false,
        };
        let decision = format!("{path}/{suffix}");
        let result = if C::LIVE_CALLERS {
            if C::AUDIT_RUNTIME_WORK {
                let mut observed = RuntimeAuditedInput {
                    input: &mut input,
                    phase: |phase, work| {
                        self.package
                            .audit_runtime_work(draft::RuntimePredictionWorkAudit {
                                uri: self.uri,
                                decision: &decision,
                                detail: draft::RuntimePredictionWorkDetail::Phase(phase, work),
                            })
                    },
                    geometry: |event| {
                        self.package
                            .audit_runtime_work(draft::RuntimePredictionWorkAudit {
                                uri: self.uri,
                                decision: &decision,
                                detail: draft::RuntimePredictionWorkDetail::Geometry(event),
                            })
                    },
                };
                self.package.live_prediction(
                    draft::prediction_sources::Request {
                        grammar,
                        expression: expr,
                        suffix,
                        decision: &decision,
                        uri: self.uri,
                        hidden: policy,
                    },
                    &mut observed,
                    &mut remaining,
                )
            } else {
                self.package.live_prediction(
                    draft::prediction_sources::Request {
                        grammar,
                        expression: expr,
                        suffix,
                        decision: &decision,
                        uri: self.uri,
                        hidden: policy,
                    },
                    &mut input,
                    &mut remaining,
                )
            }
        } else if C::SOURCE_PREDICTION && grammar.profile == grammar::Profile::NativeV67 {
            if C::AUDIT_RUNTIME_WORK {
                let mut observed = RuntimeAuditedInput {
                    input: &mut input,
                    phase: |phase, work| {
                        self.package
                            .audit_runtime_work(draft::RuntimePredictionWorkAudit {
                                uri: self.uri,
                                decision: &decision,
                                detail: draft::RuntimePredictionWorkDetail::Phase(phase, work),
                            })
                    },
                    geometry: |event| {
                        self.package
                            .audit_runtime_work(draft::RuntimePredictionWorkAudit {
                                uri: self.uri,
                                decision: &decision,
                                detail: draft::RuntimePredictionWorkDetail::Geometry(event),
                            })
                    },
                };
                self.package.source_prediction(
                    draft::prediction_sources::Request {
                        grammar,
                        expression: expr,
                        suffix,
                        decision: &decision,
                        uri: self.uri,
                        hidden: policy,
                    },
                    &mut observed,
                    &mut remaining,
                )
            } else {
                self.package.source_prediction(
                    draft::prediction_sources::Request {
                        grammar,
                        expression: expr,
                        suffix,
                        decision: &decision,
                        uri: self.uri,
                        hidden: policy,
                    },
                    &mut input,
                    &mut remaining,
                )
            }
        } else if matches!(
            grammar.profile,
            grammar::Profile::NativeV77
                | grammar::Profile::NativeV78
                | grammar::Profile::NativeV79
                | grammar::Profile::NativeV80
                | grammar::Profile::NativeV81
                | grammar::Profile::NativeV82
        ) {
            (|| -> Result<Option<usize>, String> {
                match grammar.qualified_nullable_decision_with_work(&decision, &mut remaining)? {
                    Some(grammar::QualifiedNullableDecision::FullRepeat(proof)) => {
                        if !std::ptr::eq(proof.expression(), expr) {
                            return Err(
                                "Nullable repeat path differs from the actual parser expression"
                                    .into(),
                            );
                        }
                        program.predict_productive_nullable_full_repeat_with_replay_input(
                            &decision,
                            proof.callee(),
                            proof.keyword(),
                            &mut input,
                            &mut remaining,
                            grammar::CompletionVerification::GuardOverapproximation,
                        )
                    }
                    Some(grammar::QualifiedNullableDecision::KeywordOptional(proof)) => {
                        if !std::ptr::eq(proof.expression(), expr) {
                            return Err(
                                "Bounded keyword path differs from the actual parser expression"
                                    .into(),
                            );
                        }
                        program.predict_rule_bounded_keyword_optional_with_replay_input(
                            &decision,
                            proof.owner(),
                            proof.keyword(),
                            &mut input,
                            &mut remaining,
                            grammar::CompletionVerification::GuardOverapproximation,
                        )
                    }
                    None => program.predict_with_replay_input_and_completion_verification(
                        &decision,
                        &mut input,
                        &mut remaining,
                        grammar::CompletionVerification::GuardOverapproximation,
                    ),
                }
            })()
        } else if matches!(
            grammar.profile,
            grammar::Profile::NativeV74
                | grammar::Profile::NativeV75
                | grammar::Profile::NativeV76
                | grammar::Profile::NativeV77
                | grammar::Profile::NativeV78
                | grammar::Profile::NativeV79
                | grammar::Profile::NativeV80
                | grammar::Profile::NativeV81
                | grammar::Profile::NativeV82
        ) {
            program.predict_with_replay_input_and_completion_verification(
                &decision,
                &mut input,
                &mut remaining,
                crate::grammar::CompletionVerification::GuardOverapproximation,
            )
        } else if grammar.profile.has_full_syntactic_predicates() {
            program.predict_with_replay_input(&decision, &mut input, &mut remaining)
        } else {
            program.predict_with_input_work(&decision, |work| input.read_next(work), &mut remaining)
        };
        cursor.position = input.furthest;
        let memoize = (memoizable && !input.stateful).then_some(input.read);
        if let (Some(read), Ok(selected)) = (memoize, &result) {
            self.nested_decisions.reuse.memo.insert(
                expression,
                suffix,
                tokens,
                prediction_start..read,
                *selected,
                cursor.position,
            );
        }
        if let (Some(before), Some(after)) = (before, remaining) {
            self.steps = self.steps.saturating_add(before - after);
        }
        if C::AUDIT_PREDICTION {
            self.package.audit_prediction(draft::PredictionAudit {
                uri: self.uri,
                decision: &decision,
                cursor_start: prediction_start,
                input_furthest: cursor.position,
                remaining_before: before,
                remaining_after: remaining,
                result: result.as_ref().copied().map_err(String::as_str),
            });
        }
        if result.is_ok()
            && let Some(document) = self.document.as_mut()
        {
            document
                .prediction(
                    &format!("{path}/{suffix}"),
                    prediction_start,
                    cursor.position,
                    policy,
                )
                .map_err(ParseError::Storage)?;
        }
        let result = match result {
            Ok(selected) => selected,
            Err(error) if error.contains("budget") || error.contains("depth") => {
                return Err(ParseError::Budget(error));
            }
            Err(error)
                if error.starts_with("Ambiguous")
                    || error.starts_with("Recursive prediction requires visible predicate") =>
            {
                // Locate the rejection at the decision's first token.
                self.diagnostic_position = Some(prediction_start);
                return Err(ParseError::Ambiguous(format!("{error} at {decision}")));
            }
            Err(error) => return Err(ParseError::Storage(error)),
        };
        if result.is_none() && suffix == "choice" {
            let farthest = cursor.position.saturating_sub(1).min(tokens.len());
            if farthest >= self.farthest {
                self.farthest = farthest;
                self.expected = "No viable predicted alternative".into();
            }
        }
        Ok(result)
    }
    fn expr(&mut self, expr: &'a Expr, owner: &mut Option<Current>) -> Result<Value, ParseError> {
        let mut count = 0;
        let mut result = Value::Null;
        loop {
            self.step()?;
            let predicted = expr.prediction_path.is_some() && expr.cardinality != '1';
            if predicted
                && !(expr.cardinality == '+' && count == 0)
                && self.predict(expr, "repeat")?.is_none()
            {
                break;
            }
            let checkpoint = if (expr.cardinality == '1' || predicted) && self.document.is_none() {
                self.propagating_checkpoint()
            } else {
                self.checkpoint()?
            };
            let previous_current = *owner;
            let outcome = self.once(expr, owner);
            match self.finish_expr_attempt(
                expr,
                owner,
                checkpoint,
                previous_current,
                count,
                predicted,
                outcome,
            )? {
                Some(value) => {
                    result = value;
                    count += 1;
                }
                None => break,
            }
            if !['*', '+'].contains(&expr.cardinality) {
                break;
            }
        }
        Ok(result)
    }
    #[allow(clippy::too_many_arguments)]
    fn finish_expr_attempt(
        &mut self,
        expr: &Expr,
        owner: &mut Option<Current>,
        checkpoint: Checkpoint,
        previous_current: Option<Current>,
        count: usize,
        predicted: bool,
        outcome: Result<Value, ParseError>,
    ) -> Result<Option<Value>, ParseError> {
        match outcome {
            Ok(value) => {
                if self.cursor.matched == checkpoint.cursor.matched
                    && ['*', '+'].contains(&expr.cardinality)
                {
                    self.rollback(checkpoint)?;
                    *owner = previous_current;
                    return Err(ParseError::Budget(
                        "Non-consuming grammar repetition".into(),
                    ));
                }
                self.commit_checkpoint(checkpoint)?;
                Ok(Some(value))
            }
            Err(error) => {
                if self.document.is_some()
                    && matches!(error, ParseError::Syntax(_))
                    && (predicted || expr.cardinality == '1')
                {
                    self.commit_checkpoint(checkpoint)?;
                    return Err(error);
                }
                self.rollback(checkpoint)?;
                *owner = previous_current;
                if predicted || !matches!(error, ParseError::Syntax(_)) {
                    return Err(error);
                }
                if count == 0 && !['?', '*'].contains(&expr.cardinality) {
                    return Err(error);
                }
                Ok(None)
            }
        }
    }
    fn step(&mut self) -> Result<(), ParseError> {
        if let Some(limit) = self.limits.steps_per_resource {
            if self.steps >= limit {
                return Err(ParseError::Budget("Execution budget exceeded".into()));
            }
            self.steps += 1;
        }
        Ok(())
    }
    fn recognize_terminal(
        &mut self,
        name: &str,
    ) -> Result<Option<Token<Cow<'a, str>>>, ParseError> {
        loop {
            self.skip_hidden()?;
            let selected = self.tokens.get(self.cursor.position).cloned();
            if selected.is_none() && self.document.is_some() {
                if self.document_recover_token(grammar::PredictionSymbol::Terminal(name.into()))? {
                    continue;
                } else {
                    return Ok(None);
                }
            }
            let t = self
                .tokens
                .get(self.cursor.position)
                .cloned()
                .ok_or_else(|| self.fail(format!("Expected {name}")))?;
            let custom = self
                .package
                .grammar()
                .and_then(|grammar| grammar.terminals.get(name));
            let valid = if let Some(index) = t.lexical_alternative {
                let prediction = self
                    .package
                    .grammar()
                    .and_then(|grammar| grammar.lexical_prediction.as_ref())
                    .ok_or_else(|| {
                        ParseError::Storage("Missing lexical prediction program".into())
                    })?;
                matches!(prediction.alternative(index).map_err(ParseError::Storage)?, grammar::LexicalAlternative::Terminal(selected) if selected == name)
            } else if let Some(terminal) = custom {
                !t.quoted && terminal.starts(&t.text) && !self.keywords.contains(t.text.as_ref())
            } else {
                match name {
                    "STRING" => t.quoted,
                    "INT" => {
                        !t.quoted
                            && !t.text.is_empty()
                            && t.text.chars().all(|c| c.is_ascii_digit())
                    }
                    _ => {
                        !t.quoted
                            && t.text
                                .chars()
                                .next()
                                .is_some_and(|c| c.is_alphabetic() || c == '_')
                            && !self.keywords.contains(t.text.as_ref())
                    }
                }
            };
            if !valid {
                if self.document.is_some() {
                    if self
                        .document_recover_token(grammar::PredictionSymbol::Terminal(name.into()))?
                    {
                        continue;
                    } else {
                        return Ok(None);
                    }
                }
                return Err(self.fail(format!("Expected {name}")));
            }
            self.document_match(self.cursor.position)?;
            self.cursor.consume().map_err(ParseError::Storage)?;
            return Ok(Some(t));
        }
    }
    fn append_datatype_token(&mut self, index: usize) -> Result<(), ParseError> {
        let Some(text) = self.datatype_text.as_mut() else {
            return Ok(());
        };
        let token = self
            .tokens
            .get(index)
            .ok_or_else(|| ParseError::Storage("Missing datatype token".into()))?;
        let raw = self
            .raw_source
            .get(token.start..token.end)
            .ok_or_else(|| ParseError::Storage("Invalid datatype token span".into()))?;
        let offset = self
            .utf16_positions
            .get(index)
            .ok_or_else(|| ParseError::Storage("Missing datatype token UTF-16 position".into()))?;
        text.push(raw, *offset).map_err(ParseError::Storage)
    }
    fn terminal(&mut self, name: &str, lexical: bool) -> Result<Value, ParseError> {
        self.skip_hidden()?;
        let token_index = self.cursor.position;
        let Some(token) = self.recognize_terminal(name)? else {
            return Ok(Value::Null);
        };
        self.terminal_value(name, lexical, token_index, token)
    }
    fn terminal_value(
        &mut self,
        name: &str,
        lexical: bool,
        token_index: usize,
        t: Token<Cow<'a, str>>,
    ) -> Result<Value, ParseError> {
        if self
            .document
            .as_ref()
            .is_some_and(|d| !d.scalar_token(token_index))
        {
            return Err(ParseError::Syntax(
                "Non-scalar lexical values require explicit model source-unit String support"
                    .into(),
            ));
        }
        if self.datatype_text.is_some() {
            self.append_datatype_token(self.cursor.position - 1)?;
            return Ok(Value::Null);
        }
        if self.package.grammar().is_some_and(|grammar| {
            matches!(
                grammar.profile,
                grammar::Profile::NativeV52
                    | grammar::Profile::NativeV53
                    | grammar::Profile::NativeV54
                    | grammar::Profile::NativeV55
                    | grammar::Profile::NativeV56
                    | grammar::Profile::NativeV57
                    | grammar::Profile::NativeV58
                    | grammar::Profile::NativeV59
                    | grammar::Profile::NativeV60
                    | grammar::Profile::NativeV61
                    | grammar::Profile::NativeV62
                    | grammar::Profile::NativeV63
                    | grammar::Profile::NativeV64
                    | grammar::Profile::NativeV65
                    | grammar::Profile::NativeV66
                    | grammar::Profile::NativeV67
                    | grammar::Profile::NativeV68
                    | grammar::Profile::NativeV69
                    | grammar::Profile::NativeV70
                    | grammar::Profile::NativeV71
                    | grammar::Profile::NativeV72
                    | grammar::Profile::NativeV73
                    | grammar::Profile::NativeV74
                    | grammar::Profile::NativeV75
                    | grammar::Profile::NativeV76
                    | grammar::Profile::NativeV77
                    | grammar::Profile::NativeV78
                    | grammar::Profile::NativeV79
                    | grammar::Profile::NativeV80
                    | grammar::Profile::NativeV81
                    | grammar::Profile::NativeV82
            ) && (name == "STRING" || name == "@org.eclipse.xtext.common.Terminals::STRING")
        }) {
            let raw = self
                .raw_source
                .get(t.start..t.end)
                .ok_or_else(|| ParseError::Storage("Invalid STRING token span".into()))?;
            return rule_conversion::string(raw)
                .map(Value::String)
                .map_err(|message| {
                    self.cursor.position = token_index;
                    self.fail(format!("Invalid STRING literal: {message}"))
                });
        }
        // A language's own terminal value converter (for example KerML's
        // comment bodies), when its parse context has one.
        if let Some(converted) = self
            .package
            .convert_terminal(name, &t.text)
            .map_err(ParseError::Storage)?
        {
            return Ok(Value::String(converted));
        }
        if !lexical
            && let Some(key) = self
                .package
                .grammar()
                .and_then(|grammar| grammar.terminals.get(name))
                .and_then(|terminal| terminal.enumeration.as_deref())
        {
            let enumeration = self
                .package
                .schema()
                .enumerations
                .get(key)
                .ok_or_else(|| ParseError::Storage("Missing enum terminal type".into()))?;
            let value = enumeration.parse(&t.text).map_err(|message| {
                self.cursor.position -= 1;
                self.fail(message)
            })?;
            return serde_json::to_value(value)
                .map_err(|error| ParseError::Storage(error.to_string()));
        }
        if !lexical
            && let Some(grammar::Datatype::Builtin(codec)) = self
                .package
                .grammar()
                .and_then(|grammar| grammar.terminals.get(name))
                .map(|terminal| terminal.datatype)
        {
            if self
                .package
                .grammar()
                .is_none_or(|grammar| !grammar.profile.has_builtin_scalar_returns())
            {
                return Err(ParseError::Storage(
                    "Builtin terminal return requires NativeV81".into(),
                ));
            }
            self.step()?;
            return codec.parse(&t.text).map_err(|error| {
                self.fail(format!("Invalid {codec:?} terminal literal: {error}"))
            });
        }
        let integer = self
            .package
            .grammar()
            .and_then(|grammar| grammar.terminals.get(name))
            .map_or(name == "INT", |terminal| {
                terminal.datatype == grammar::Datatype::EInt
            });
        if integer && !lexical {
            intersect_model::definitions::scalars::Builtin::EInt
                .parse(&t.text)
                .map_err(|error| self.fail(format!("INT outside EInt range: {error}")))
        } else {
            Ok(Value::String(t.text.into_owned()))
        }
    }
    fn lexical_assignment(
        &mut self,
        name: &str,
        lexical: LexicalType,
        owner: usize,
        feature: &str,
        prefetched: Option<(usize, Token<Cow<'a, str>>)>,
    ) -> Result<Value, ParseError> {
        self.step()?;
        let start = prefetched
            .as_ref()
            .map_or(self.cursor.position, |(at, _)| *at);
        let text = if let Some((at, token)) = prefetched {
            self.terminal_value(name, true, at, token)?
        } else {
            self.terminal(name, true)?
        };
        if text.is_null() && self.document.is_some() {
            return Ok(Value::Null);
        }
        let text = text
            .as_str()
            .ok_or_else(|| self.fail("Terminal did not produce lexical text"))?;
        let owner_class = self
            .store
            .class(&self.identity(owner))
            .map_err(ParseError::Storage)?
            .ok_or_else(|| ParseError::Storage("Missing parser owner".into()))?;
        let package = self.package;
        let datatype = lexical
            .datatype
            .as_deref()
            .and_then(|key| package.schema().datatypes.get(key));
        let result = if lexical.constrained {
            self.package.schema().parse_runtime_attribute(
                &owner_class,
                feature,
                text,
                self.conversions,
            )
        } else if let Some(datatype) = datatype {
            if datatype.codec().is_ok() {
                datatype
                    .parse(text)
                    .and_then(|value| serde_json::to_value(value).map_err(|e| e.to_string()))
            } else {
                self.conversions
                    .ok_or_else(|| "Explicit conversion registry required for terminal".to_string())
                    .and_then(|registry| registry.parse_datatype(datatype, text))
            }
        } else {
            Err("Missing selected lexical datatype".into())
        };
        if self.document.is_some() {
            return match result {
                Ok(value) => Ok(value),
                Err(error) => self.document_conversion_failure(
                    self.cursor.position.saturating_sub(1),
                    error,
                    document::ConversionNode::MatchedTerminal,
                ),
            };
        }
        result.map_err(|error| {
            // Keep the token that failed conversion, not the following token, as
            // the source of the diagnostic after speculative parsing rolls back.
            self.cursor.position = start;
            self.fail(format!(
                "Invalid {} literal: {error}",
                datatype.map_or(lexical.target.as_str(), |datatype| datatype.name())
            ))
        })
    }
    fn reference_spelling(
        &mut self,
        rule: Option<&str>,
    ) -> Result<Option<(String, usize, usize)>, ParseError> {
        if self.document.is_some() && rule.is_none() {
            self.skip_hidden()?;
            let Some(token) = self.recognize_terminal("ID")? else {
                return Ok(None);
            };
            return Ok(Some((token.text.into_owned(), token.start, token.end)));
        }
        self.skip_hidden()?;
        let first = self
            .tokens
            .get(self.cursor.position)
            .cloned()
            .ok_or_else(|| self.fail("Expected reference name"))?;
        if let Some(rule) = rule {
            let value = self.rule(rule)?;
            if value.is_null() && self.document.is_some() {
                return Ok(None);
            }
            let spelling = value
                .as_str()
                .ok_or_else(|| {
                    ParseError::Storage("Datatype reference produced a non-string".into())
                })?
                .to_owned();
            let end = self
                .previous_token()
                .ok_or_else(|| ParseError::Storage("Missing datatype reference span".into()))?
                .end;
            let (start, end) = if self.retained_tokens {
                self.datatype_span.ok_or_else(|| {
                    ParseError::Storage("Missing matched reference datatype span".into())
                })?
            } else {
                (first.start, end)
            };
            Ok(Some((spelling, start, end)))
        } else {
            // Preserve the original ID-reference lexical contract verbatim.
            if first.quoted
                || !first
                    .text
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
            {
                return Err(self.fail("Expected reference name"));
            }
            self.document_match(self.cursor.position)?;
            self.cursor.consume().map_err(ParseError::Storage)?;
            Ok(Some((first.text.into_owned(), first.start, first.end)))
        }
    }
    fn once(&mut self, expr: &'a Expr, owner: &mut Option<Current>) -> Result<Value, ParseError> {
        if C::LIVE_CALLERS
            && matches!(&expr.kind, Kind::Call(name) if !["ID", "STRING", "INT"].contains(&name.as_str()) && self.package.grammar().is_some_and(|g| g.rules.contains_key(name) && !g.terminals.contains_key(name)))
        {
            return self.with_live_call(expr, |machine| machine.once_literal(expr, owner));
        }
        self.once_literal(expr, owner)
    }
    fn once_literal(
        &mut self,
        expr: &'a Expr,
        owner: &mut Option<Current>,
    ) -> Result<Value, ParseError> {
        let document = self.begin_once_document(expr, owner)?;
        let result = self.once_body(expr, owner);
        self.finish_once_document(expr, owner, document, result)
    }
    fn begin_once_document(
        &mut self,
        expr: &'a Expr,
        owner: &mut Option<Current>,
    ) -> Result<Option<OnceDocumentPhase<'a>>, ParseError> {
        if self.document.is_none() {
            return Ok(None);
        }
        let previous_origin = self.document_origin;
        let previous_feature = self.document_feature;
        self.document_origin = expr.prediction_path.as_deref();
        let captured_node = if matches!(&expr.kind, Kind::Action(_) | Kind::Capture { .. }) {
            let path = expr
                .prediction_path
                .as_deref()
                .ok_or_else(|| ParseError::Storage("Missing action source occurrence".into()))?;
            Some(
                self.document
                    .as_mut()
                    .ok_or_else(|| ParseError::Storage("Missing action document".into()))?
                    .action(path)
                    .map_err(ParseError::Storage)?,
            )
        } else {
            None
        };
        if let Kind::Assign { feature, .. } | Kind::BooleanAssign { feature, .. } = &expr.kind
            && let Some(current) = owner
        {
            self.document_feature = Some((current.index, feature));
        }
        let called = matches!(&expr.kind,Kind::Call(name) if self.package.grammar().is_some_and(|g|g.rules.contains_key(name)));
        let at = self.document_at(
            self.document
                .as_ref()
                .map_or(self.cursor.position, |d| d.emitted),
        );
        let node = if called {
            let path = expr
                .prediction_path
                .as_deref()
                .ok_or_else(|| ParseError::Storage("Missing document call occurrence".into()))?;
            let document = self
                .document
                .as_mut()
                .ok_or_else(|| ParseError::Storage("Missing document capture".into()))?;
            let node = document.begin(path, at).map_err(ParseError::Storage)?;
            if let Some((index, feature)) = self.document_feature {
                document
                    .feature(&format!("{}#{index}", self.uri), feature, node)
                    .map_err(ParseError::Storage)?;
            }
            document.push_call(path).map_err(ParseError::Storage)?;
            self.document_feature = None;
            Some(node)
        } else {
            None
        };
        Ok(Some(OnceDocumentPhase {
            previous_origin,
            previous_feature,
            captured_node,
            node,
        }))
    }
    fn finish_once_document(
        &mut self,
        expr: &'a Expr,
        owner: &mut Option<Current>,
        document: Option<OnceDocumentPhase<'a>>,
        result: Result<Value, ParseError>,
    ) -> Result<Value, ParseError> {
        let Some(OnceDocumentPhase {
            previous_origin,
            previous_feature,
            captured_node,
            node,
        }) = document
        else {
            return result;
        };
        if result.is_ok()
            && let (Kind::Capture { feature, .. }, Some(node), Some(current)) =
                (&expr.kind, captured_node, *owner)
        {
            let id = self.identity(current.index);
            self.document
                .as_mut()
                .ok_or_else(|| ParseError::Storage("Missing captured feature document".into()))?
                .feature(&id, feature, node)
                .map_err(ParseError::Storage)?;
        }
        if let (Some(document), Some(node)) = (self.document.as_mut(), node) {
            document.calls.pop();
            document.close(node).map_err(ParseError::Storage)?;
        }
        self.document_origin = previous_origin;
        self.document_feature = previous_feature;
        result
    }
    fn once_body(
        &mut self,
        expr: &'a Expr,
        owner: &mut Option<Current>,
    ) -> Result<Value, ParseError> {
        self.skip_hidden()?;
        match &expr.kind {
            Kind::SyntacticPredicate(value) => self.expr(value, owner),
            // Mixed-action rules start without an owner. Admission proves this
            // creation occurs before any current object exists on the selected path.
            Kind::Action(class) if owner.is_none() && self.package.grammar().is_some_and(|grammar| matches!(grammar.profile, grammar::Profile::NativeV26 | grammar::Profile::NativeV27 | grammar::Profile::NativeV28 | grammar::Profile::NativeV29 | grammar::Profile::NativeV30 | grammar::Profile::NativeV31 | grammar::Profile::NativeV32 | grammar::Profile::NativeV33 | grammar::Profile::NativeV34 | grammar::Profile::NativeV35 | grammar::Profile::NativeV36 | grammar::Profile::NativeV37 | grammar::Profile::NativeV38 | grammar::Profile::NativeV39 | grammar::Profile::NativeV40 | grammar::Profile::NativeV41 | grammar::Profile::NativeV42 | grammar::Profile::NativeV43 | grammar::Profile::NativeV44 | grammar::Profile::NativeV45 | grammar::Profile::NativeV46 | grammar::Profile::NativeV47 | grammar::Profile::NativeV48 | grammar::Profile::NativeV49 | grammar::Profile::NativeV50 | grammar::Profile::NativeV51 | grammar::Profile::NativeV52 | grammar::Profile::NativeV53 | grammar::Profile::NativeV54 | grammar::Profile::NativeV55 | grammar::Profile::NativeV56 | grammar::Profile::NativeV57 | grammar::Profile::NativeV58 | grammar::Profile::NativeV59 | grammar::Profile::NativeV60 | grammar::Profile::NativeV61 | grammar::Profile::NativeV62 | grammar::Profile::NativeV63 | grammar::Profile::NativeV64 | grammar::Profile::NativeV65 | grammar::Profile::NativeV66 | grammar::Profile::NativeV67 | grammar::Profile::NativeV68 | grammar::Profile::NativeV69 | grammar::Profile::NativeV70 | grammar::Profile::NativeV71 | grammar::Profile::NativeV72 | grammar::Profile::NativeV73 | grammar::Profile::NativeV74 | grammar::Profile::NativeV75 | grammar::Profile::NativeV76 | grammar::Profile::NativeV77 | grammar::Profile::NativeV78 | grammar::Profile::NativeV79 | grammar::Profile::NativeV80 | grammar::Profile::NativeV81 | grammar::Profile::NativeV82)) => {
                let start = self.tokens.get(self.cursor.position).map_or(0, |token| token.start);
                *owner = Some(Current::local(self.create_current(class, start)?));
                Ok(Value::Null)
            },
            // Ordinary leading actions selected their class before rule entry;
            // their owner is already initialized and must not be created twice.
            Kind::Action(_) => Ok(Value::Null),
            Kind::Capture { class, feature, many } => {
                let previous = owner.ok_or_else(|| ParseError::Storage("Capture requires a current object".into()))?;
                let next = self.capture_current(previous, class, feature, *many)?;
                *owner = Some(Current::local(next));
                Ok(Value::String(self.identity(next).to_string()))
            },
            Kind::Literal(text) => {
                if self
                    .tokens
                    .get(self.cursor.position)
                    .is_some_and(|t| {
                        !t.quoted && t.text == *text && t.lexical_alternative.is_none_or(|index| {
                            self.package.grammar()
                                .and_then(|grammar| grammar.lexical_prediction.as_ref())
                                .and_then(|prediction| prediction.alternative(index).ok())
                                .is_some_and(|alternative| matches!(alternative, grammar::LexicalAlternative::Keyword(selected) if selected == text))
                        })
                    })
                {
                    self.document_match(self.cursor.position)?;
                    self.cursor.consume().map_err(ParseError::Storage)?;
                    self.append_datatype_token(self.cursor.position - 1)?;
                    Ok(Value::String(text.clone()))
                } else {
                    if self.document.is_some(){if self.document_recover_token(grammar::PredictionSymbol::Keyword(text.clone()))?{return self.once_body(expr,owner)}else{return Ok(Value::Null)}}
                    Err(self.fail(format!("Expected '{text}'")))
                }
            }
            Kind::Call(name)
                if ["ID", "STRING", "INT"].contains(&name.as_str())
                    || self.package.grammar()
                        .is_some_and(|grammar| grammar.terminals.contains_key(name)) =>
            {
                self.terminal(name, false)
            }
            Kind::Call(name) => {
                let fragment = self.package.grammar()
                    .and_then(|grammar| grammar.rules.get(name))
                    .filter(|rule| rule.fragment);
                if let Some(fragment) = fragment {
                    // An EObject fragment runs on the canonical caller even
                    // when its body takes an empty path. Generated Xtext callers
                    // create that object before passing current to the fragment.
                    // Datatype/terminal paths own no EObject; an already called
                    // or action-created current is preserved without a wrapper.
                    // NativeV82 unordered groups lower to generated fragments, but
                    // Xtext creates no object for an unordered group itself: only
                    // its members' assignments, actions and calls construct one.
                    if owner.is_none()
                        && fragment.datatype.is_none()
                        && self.datatype_text.is_none()
                        && !crate::grammar::is_generated_unordered_rule(name)
                    {
                        self.assignment_owner(owner)?;
                    }
                    self.depth += 1;
                    if self.depth > self.limits.nesting {
                        return Err(ParseError::Budget("Model nesting budget exceeded".into()));
                    }
                    // Fragment writes stay in the caller journal; a surrounding
                    // failed group must restore values, markers and references.
                    let previous = self.generic_fragment;
                    self.generic_fragment |= fragment.wildcard;
                    let prior_policy = self.enter_hidden(fragment.hidden.as_deref())?;
                    let mut result = self.expr(&fragment.body, owner);
                    if self.document.is_some()&&let Err(ParseError::Syntax(message))=&result{self.document_recover_rule(message.clone())?;result=Ok(Value::Null);}
                    let restored = self.restore_hidden(prior_policy);
                    self.generic_fragment = previous;
                    self.depth -= 1;
                    restored?;
                    result
                } else {
                    // Ordinary called rules construct/check their own context.
                    let previous = self.generic_fragment;
                    self.generic_fragment = false;
                    let result = self.rule(name);
                    self.generic_fragment = previous;
                    let value = result?;
                    if !value.is_null() && self.package.grammar().and_then(|g| g.rules.get(name)).is_some_and(|r| r.datatype.is_none()) {
                        *owner = Some(Current::called(self.current_index(&value)?));
                    }
                    Ok(value)
                }
            }
            Kind::Reference(_) | Kind::ScalarReference { .. } => Err(ParseError::Syntax(
                "Cross-reference must be assigned".into(),
            )),
            Kind::Assign {
                feature,
                many,
                value,
            } => {
                let mut prefetched=None;let mut inserted_terminal=false;
                if self.document.is_some()&&owner.is_none()&&value.cardinality=='1'&&let Kind::Call(name)=&value.kind&&(["ID","STRING","INT"].contains(&name.as_str())||self.package.grammar().is_some_and(|g|g.terminals.contains_key(name))){
                    self.skip_hidden()?;let at=self.cursor.position;let previous=self.document_origin;self.document_origin=value.prediction_path.as_deref();let token=self.recognize_terminal(name);self.document_origin=previous;
                    if let Some(token)=token?{prefetched=Some((at,token));}else{inserted_terminal=true;}
                }
                let owner = self.assignment_owner(owner)?;
                self.admit_assignment(owner, feature, *many)?;
                let value_location = if C::NEEDS_VALUE_ADMISSION {
                    // The first terminal may already have been recognized to
                    // create its owner. Preserve that actual token range.
                    let location = if let Some((_, token)) = prefetched.as_ref() {
                        let mut location = self.parse_location()?;
                        location.start = token.start;
                        location.end = token.end;
                        location
                    } else {
                        self.skip_hidden()?;
                        self.parse_location()?
                    };
                    Some(location)
                } else {
                    None
                };
                let owner_id = self.identity(owner);
                if self.document.is_some(){
                    self.document_feature=Some((owner,feature));
                    if prefetched.is_some(){let document=self.document.as_mut().ok_or_else(||ParseError::Storage("Missing document assignment".into()))?;let node=document.builder.last_node().ok_or_else(||ParseError::Storage("Missing recognized assignment leaf".into()))?;document.feature(&owner_id,feature,node).map_err(ParseError::Storage)?;}
                }
                // A genuine missing-token insertion returns to the generated
                // assignment action: create its owner, but publish no synthetic
                // leaf or value. A thrown mismatch never reaches this point.
                if inserted_terminal{return Ok(Value::Null)}
                let reference = match &value.kind {
                    Kind::Reference(target) => Some((target, None)),
                    Kind::ScalarReference { target, rule } => Some((target, Some(rule.as_str()))),
                    _ => None,
                };
                let field_value = if let Some((target, spelling_rule)) = reference {
                    let class = self.store.class(&owner_id).map_err(ParseError::Storage)?.ok_or_else(|| ParseError::Storage("Missing reference owner".into()))?;
                    if self.settings.is_some_and(|settings| settings.supports(&class, feature)) {
                        return Err(ParseError::Storage("Computed reference spellings require deferred provider linking, which is unsupported".into()));
                    }
                    let at=self.document_at(self.cursor.position);
                    let previous_origin=self.document_origin;let previous_feature=self.document_feature;
                    let parser_spelling=spelling_rule.is_some_and(|name|self.package.grammar().is_some_and(|g|g.rules.contains_key(name)));
                    let reference_node=if let Some(document)=self.document.as_mut(){
                        let path=value.prediction_path.as_deref().ok_or_else(||ParseError::Storage("Missing original reference occurrence".into()))?;
                        self.document_origin=Some(path);
                        if parser_spelling{
                            let node=document.begin(path,at).map_err(ParseError::Storage)?;document.feature(&owner_id,feature,node).map_err(ParseError::Storage)?;
                            document.push_call(path).map_err(ParseError::Storage)?;self.document_feature=None;Some(node)
                        }else{self.document_feature=Some((owner,feature));None}
                    }else{None};
                    let spelling=if C::LIVE_CALLERS && parser_spelling {
                        self.with_live_call(value, |machine| machine.reference_spelling(spelling_rule))
                    } else { self.reference_spelling(spelling_rule) };
                    if let(Some(document),Some(node))=(self.document.as_mut(),reference_node){if parser_spelling{document.calls.pop();}document.close(node).map_err(ParseError::Storage)?;}
                    self.document_origin=previous_origin;self.document_feature=previous_feature;
                    let Some((spelling,start,end))=spelling? else{return Ok(Value::Null)};
                    self.validate_generic_assignment(owner, feature, *many, None)?;
                    let index = self.assignment_undo(owner, feature, *many)?;
                    self.store
                        .set_unresolved_reference(&owner_id, feature, &spelling, *many)
                        .map_err(ParseError::Storage)?;
                    let pending = self.pending.len();
                    let names = &mut self.nested_decisions.reuse.pending_names;
                    self.pending.push(Pending {
                        owner: owner_id,
                        field: names.name(feature),
                        spelling,
                        target: names.name(target),
                        uri: names.name(self.uri),
                        start,
                        end,
                        index,
                        active: true,
                    });
                    if !many {
                        self.set_scalar_pending(owner, feature, Some(pending))?;
                    }
                    // Unresolved spelling lives in a reference state, not in a
                    // magic string masquerading as an ordinary model value.
                    return Ok(Value::Null);
                } else if let Kind::Call(name) = &value.kind {
                    let owner_class = self
                        .store
                        .class(&self.identity(owner))
                        .map_err(ParseError::Storage)?
                        .ok_or_else(|| ParseError::Storage("Missing parser owner".into()))?;
                    let datatype = self
                        .lexical_types
                        .get(&owner_class)
                        .and_then(|fields| fields.get(feature))
                        .cloned();
                    if let Some(datatype) = datatype.filter(|_| {
                        !self.generic_fragment
                            && (["ID", "STRING", "INT"].contains(&name.as_str())
                                || self.package.grammar()
                                    .is_some_and(|grammar| grammar.terminals.contains_key(name)))
                    }) {
                        let previous=self.document_origin;
                        if self.document.is_some(){self.document_origin=value.prediction_path.as_deref();}
                        let result=self.lexical_assignment(name, datatype, owner, feature,prefetched.take());
                        self.document_origin=previous;result?
                    } else if let Some((at,token))=prefetched.take(){self.terminal_value(name,false,at,token)?}else {
                        self.expr(value, &mut Some(Current::local(owner)))?
                    }
                } else {
                    self.expr(value, &mut Some(Current::local(owner)))?
                };
                if field_value.is_null() && self.package.grammar().is_some_and(|g| g.profile.has_constant_guards()) {
                    return Ok(Value::Null);
                }
                self.validate_generic_assignment(owner, feature, *many, Some(&field_value))?;
                if self.try_delegated_assignment(owner, feature, *many, &field_value, value_location)? { return Ok(field_value); }
                if let Some(location) = value_location {
                    self.admit_assignment_value(owner, feature, *many, &field_value, location)?;
                }
                if self.computed_assignment(owner, feature, *many, field_value.clone())? { return Ok(field_value); }
                self.assignment_undo(owner, feature, *many)?;
                self.write_parsed_value(&owner_id, feature, *many, field_value.clone())?;
                if !many {
                    self.set_scalar_pending(owner, feature, None)?;
                }
                Ok(field_value)
            }
            Kind::BooleanAssign { feature, value } => {
                let owner = self.assignment_owner(owner)?;
                // Like ordinary Assign, owner creation may occur after the
                // document phase began. Attribute the matched token to that
                // actual owner before a computed setting can return early.
                if self.document.is_some() {
                    self.document_feature = Some((owner, feature));
                }
                self.admit_assignment(owner, feature, false)?;
                let value_location = if C::NEEDS_VALUE_ADMISSION {
                    self.skip_hidden()?;
                    Some(self.parse_location()?)
                } else {
                    None
                };
                // Match before writing the Boolean feature. Shared savepoints preserve
                // unsettable assignment markers when an enclosing branch fails.
                self.expr(value, &mut Some(Current::local(owner)))?;
                self.validate_generic_assignment(owner, feature, false, Some(&Value::Bool(true)))?;
                if self.try_delegated_assignment(owner, feature, false, &Value::Bool(true), value_location)? { return Ok(Value::Bool(true)); }
                if let Some(location) = value_location {
                    self.admit_assignment_value(owner, feature, false, &Value::Bool(true), location)?;
                }
                if self.computed_assignment(owner, feature, false, Value::Bool(true))? { return Ok(Value::Bool(true)); }
                self.assignment_undo(owner, feature, false)?;
                self.store
                    .set_value(&self.identity(owner), feature, Some(Value::Bool(true)))
                    .map_err(ParseError::Storage)?;
                self.set_scalar_pending(owner, feature, None)?;
                Ok(Value::Bool(true))
            }
            Kind::Sequence(items) => {
                if items.len() == 1 && self.package.grammar().is_some_and(|grammar| matches!(grammar.profile, grammar::Profile::NativeV27 | grammar::Profile::NativeV28 | grammar::Profile::NativeV29 | grammar::Profile::NativeV30 | grammar::Profile::NativeV31 | grammar::Profile::NativeV32 | grammar::Profile::NativeV33 | grammar::Profile::NativeV34 | grammar::Profile::NativeV35 | grammar::Profile::NativeV36 | grammar::Profile::NativeV37 | grammar::Profile::NativeV38 | grammar::Profile::NativeV39 | grammar::Profile::NativeV40 | grammar::Profile::NativeV41 | grammar::Profile::NativeV42 | grammar::Profile::NativeV43 | grammar::Profile::NativeV44 | grammar::Profile::NativeV45 | grammar::Profile::NativeV46 | grammar::Profile::NativeV47 | grammar::Profile::NativeV48 | grammar::Profile::NativeV49 | grammar::Profile::NativeV50 | grammar::Profile::NativeV51 | grammar::Profile::NativeV52 | grammar::Profile::NativeV53 | grammar::Profile::NativeV54 | grammar::Profile::NativeV55 | grammar::Profile::NativeV56 | grammar::Profile::NativeV57 | grammar::Profile::NativeV58 | grammar::Profile::NativeV59 | grammar::Profile::NativeV60 | grammar::Profile::NativeV61 | grammar::Profile::NativeV62 | grammar::Profile::NativeV63 | grammar::Profile::NativeV64 | grammar::Profile::NativeV65 | grammar::Profile::NativeV66 | grammar::Profile::NativeV67 | grammar::Profile::NativeV68 | grammar::Profile::NativeV69 | grammar::Profile::NativeV70 | grammar::Profile::NativeV71 | grammar::Profile::NativeV72 | grammar::Profile::NativeV73 | grammar::Profile::NativeV74 | grammar::Profile::NativeV75 | grammar::Profile::NativeV76 | grammar::Profile::NativeV77 | grammar::Profile::NativeV78 | grammar::Profile::NativeV79 | grammar::Profile::NativeV80 | grammar::Profile::NativeV81 | grammar::Profile::NativeV82)) {
                    return self.expr(&items[0], owner);
                }
                for item in items {
                    self.expr(item, owner)?;
                }
                Ok(Value::Null)
            }
            Kind::Alternative(items) => {
                if items.is_empty() { return Err(self.fail("Boolean guard is false")); }
                if expr.prediction_path.is_some() {
                    let selected = self.predict(expr, "choice")?.ok_or_else(|| ParseError::Syntax(self.expected.clone()))?;
                    let item = items.get(selected).ok_or_else(|| ParseError::Storage("Invalid predicted model alternative".into()))?;
                    return self.expr(item, owner);
                }
                for item in items {
                    let checkpoint = self.checkpoint()?;
                    let previous_current = *owner;
                    match self.expr(item, owner) {
                        Ok(value) => { self.commit_checkpoint(checkpoint)?; return Ok(value); },
                        Err(error) if !matches!(error, ParseError::Syntax(_)) => { self.rollback(checkpoint)?; return Err(error); },
                        Err(_) => {
                            self.rollback(checkpoint)?;
                            *owner = previous_current;
                        }
                    }
                }
                Err(ParseError::Syntax(self.expected.clone()))
            }
        }
    }
    fn validate_generic_assignment(
        &self,
        owner: usize,
        feature: &str,
        many: bool,
        value: Option<&Value>,
    ) -> Result<(), ParseError> {
        if !self.generic_fragment {
            return Ok(());
        }
        let schema = self.package.schema();
        let class = self
            .store
            .class(&self.identity(owner))
            .map_err(ParseError::Storage)?
            .ok_or_else(|| ParseError::Storage("Missing parser owner".into()))?;
        let fields = schema.features(&class).map_err(ParseError::Storage)?;
        let field = fields
            .get(feature)
            .ok_or_else(|| ParseError::Storage(format!("Unknown feature {class}.{feature}")))?;
        if many != (field.upper != 1) {
            return Err(ParseError::Storage(format!(
                "Assignment multiplicity mismatch: {class}.{feature}"
            )));
        }
        if let Some(value) = value {
            if field.reference {
                let target = value
                    .as_str()
                    .ok_or_else(|| ParseError::Storage("Reference must be an identity".into()))?;
                let target_class = self
                    .store
                    .class(target)
                    .map_err(ParseError::Storage)?
                    .ok_or_else(|| ParseError::Storage("Missing reference target".into()))?;
                if !schema
                    .reference_conforms(&class, feature, &target_class)
                    .map_err(ParseError::Storage)?
                {
                    return Err(ParseError::Storage(format!(
                        "Reference type mismatch: {class}.{feature}"
                    )));
                }
            } else {
                schema
                    .validate_runtime_attribute(&class, field, value, self.conversions)
                    .map_err(ParseError::Storage)?;
            }
        } else if !field.reference {
            return Err(ParseError::Storage(format!(
                "Cross-reference requires a reference feature: {class}.{feature}"
            )));
        }
        Ok(())
    }
    fn assignment_undo(
        &mut self,
        owner: usize,
        field: &'a str,
        many: bool,
    ) -> Result<Option<usize>, ParseError> {
        let id = self.identity(owner);
        // Legacy grammars retain their existing per-assignment cost. New classes
        // cache effective unsettable names once, including inherited references.
        if !self.unsettable.is_empty() {
            let class = self
                .store
                .class(&id)
                .map_err(ParseError::Storage)?
                .ok_or_else(|| ParseError::Storage("Missing parser owner".into()))?;
            if self
                .unsettable
                .get(&class)
                .is_some_and(|fields| fields.contains(field))
            {
                let mut previous = None;
                self.store
                    .inspect(&id, &mut |object| {
                        let state = object
                            .assignment_state()
                            .ok_or("Missing parser assignment state")?;
                        if !state.explicitly_set(field) {
                            previous = Some(state.clone());
                        }
                        Ok(())
                    })
                    .map_err(ParseError::Storage)?;
                if let Some(previous) = previous {
                    let next = previous
                        .with_assignment(field, true)
                        .map_err(ParseError::Storage)?;
                    self.store
                        .set_assignment_state(&id, Some(next))
                        .map_err(ParseError::Storage)?;
                }
            }
        }
        if many
            && let Some(length) = self
                .store
                .value_len(&id, field)
                .map_err(ParseError::Storage)?
        {
            return Ok(Some(length));
        }
        let unresolved = self.scalar_pending.get(&(owner, field)).copied();
        self.undo.push(Undo {
            owner,
            field,
            unresolved,
        });
        Ok(many.then_some(0))
    }
}
pub fn keywords(package: &Package) -> Vec<String> {
    package
        .grammar
        .as_ref()
        .map_or_else(Vec::new, grammar::Grammar::keywords)
}

/// Evaluate without model-size or execution-step ceilings.
/// Recursive nesting remains bounded. Use `evaluate_with_limits` for host budgets.
pub fn evaluate(package: &Package, sources: &[Source]) -> Evaluation {
    evaluate_with_limits(package, sources, EvaluationLimits::native())
}

/// Evaluation metadata without materializing the stored model.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StoredEvaluation {
    pub diagnostics: Vec<Diagnostic>,
    pub references: Vec<Reference>,
    /// Exact active reference occurrences from construction-only runtime parsing.
    /// These carry no resolution claim; a later language linker supplies targets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_references: Vec<UnresolvedReference>,
    pub derived: BTreeMap<String, BTreeMap<String, Value>>,
}

/// Language-neutral source occurrence retained before a separate linking phase.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnresolvedReference {
    pub owner: String,
    pub feature: String,
    pub spelling: String,
    pub target_class: String,
    pub uri: String,
    pub start: usize,
    pub end: usize,
    pub index: Option<usize>,
}

impl From<&Pending> for UnresolvedReference {
    fn from(reference: &Pending) -> Self {
        Self {
            owner: reference.owner.to_string(),
            feature: reference.field.to_string(),
            spelling: reference.spelling.clone(),
            target_class: reference.target.to_string(),
            uri: reference.uri.to_string(),
            start: reference.start,
            end: reference.end,
            index: reference.index,
        }
    }
}

/// A live, compact model and its evaluation metadata. Creating this result does
/// not materialize the owned compatibility model or retain source text/tokens.
pub struct EvaluatedStore {
    pub store: FrozenStore,
    pub evaluation: StoredEvaluation,
}

/// Evaluate into an owned compact store, retaining successfully parsed resources
/// when another resource has diagnostics. Use `evaluate_into` when any diagnostic
/// must instead roll back an entire replacement of an existing model.
pub fn evaluate_stored(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
) -> Result<EvaluatedStore, String> {
    evaluate_stored_context(
        package,
        sources,
        limits,
        None,
        &query::PackageDerived(package),
    )
}
/// Evaluate with explicitly selected source-bound computed setters. Unresolved
/// references are linked and strictly validated before a result is published.
pub fn evaluate_stored_with_settings(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    settings: &setting::TextSettings<'_>,
) -> Result<EvaluatedStore, String> {
    evaluate_stored_context_with_settings(
        package,
        sources,
        limits,
        None,
        Some(settings),
        &query::PackageDerived(package),
    )
}
/// Evaluate using an explicitly installed language-owned name candidate policy.
/// The shared linker still checks target classes/reference conformance, emits the
/// original unresolved/ambiguous diagnostics and resolves through the same store.
/// The caller owns one logical work budget for the complete linking pass.
pub fn evaluate_stored_with_linking_context(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    derived: &dyn query::DerivedValues,
    linking: &mut linking::LinkingContext<'_>,
) -> Result<EvaluatedStore, String> {
    let mut store = new_evaluation_store(package)?;
    let evaluation = storage::atomic(&mut store, |store| {
        evaluate_staged_with_linking(
            package,
            sources,
            Invocation::resource(limits),
            store,
            conversions,
            settings,
            derived,
            Some(linking),
        )
    })?;
    Ok(EvaluatedStore { store, evaluation })
}

fn evaluate_stored_context(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    derived: &dyn query::DerivedValues,
) -> Result<EvaluatedStore, String> {
    evaluate_stored_context_with_settings(package, sources, limits, conversions, None, derived)
}
fn evaluate_stored_context_with_settings(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    derived: &dyn query::DerivedValues,
) -> Result<EvaluatedStore, String> {
    let mut store = new_evaluation_store(package)?;
    let evaluation = storage::atomic(&mut store, |s| {
        evaluate_staged(
            package,
            sources,
            Invocation::resource(limits),
            s,
            conversions,
            settings,
            derived,
        )
    })?;
    Ok(EvaluatedStore { store, evaluation })
}

fn new_evaluation_store(package: &Package) -> Result<FrozenStore, String> {
    new_evaluation_store_definition(crate::package::TextPackageView::from_package(package))
}
fn new_evaluation_store_definition(
    package: crate::package::TextPackageView<'_>,
) -> Result<FrozenStore, String> {
    let metamodel = Metamodel::freeze(package.schema).map_err(|error| error.to_string())?;
    Ok(FrozenStore::with_index_policy(
        std::sync::Arc::new(metamodel),
        evaluation_index_policy_definition(package),
    ))
}

#[cfg(test)]
fn evaluation_index_policy(package: &Package) -> storage::IndexPolicy {
    evaluation_index_policy_definition(crate::package::TextPackageView::from_package(package))
}
fn evaluation_index_policy_definition(
    package: crate::package::TextPackageView<'_>,
) -> storage::IndexPolicy {
    // Unique-name linking, incoming-edge semantics and reciprocal validation
    // require repeated lookups. Index opposite endpoints too: checking each
    // child's membership in an unindexed parent collection would be quadratic.
    // Other features retain identical query behavior through the store's scan
    // fallback; their values need not also occupy a secondary index.
    let indexed_features = std::iter::once("name".to_owned())
        .chain(
            package
                .semantics
                .derived
                .iter()
                .filter_map(|rule| match &rule.expression {
                    Expression::NoIncoming { feature, .. } => Some(feature.clone()),
                    Expression::Count { .. }
                    | Expression::Stored { .. }
                    | Expression::External { .. }
                    | Expression::Source { .. } => None,
                }),
        )
        .chain(
            package
                .semantics
                .validations
                .iter()
                .map(|rule| rule.feature.clone()),
        )
        .chain(
            package
                .schema
                .classes
                .values()
                .flat_map(|class| class.features.values())
                .filter_map(|field| field.opposite.as_deref())
                .filter_map(|opposite| opposite.split_once('/'))
                .map(|(_, feature)| feature.to_owned()),
        );
    storage::IndexPolicy::selected(indexed_features)
}

pub fn evaluate_with_limits(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
) -> Evaluation {
    evaluate_context(
        package,
        sources,
        limits,
        None,
        &query::PackageDerived(package),
    )
}
/// Evaluate text and model semantics using an explicit shared derived provider.
pub fn evaluate_with_provider(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    derived: &dyn query::DerivedValues,
) -> Evaluation {
    evaluate_context(package, sources, limits, None, derived)
}

pub fn evaluate_with_conversions(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
) -> Evaluation {
    evaluate_context(
        package,
        sources,
        limits,
        Some(conversions),
        &query::PackageDerived(package),
    )
}
fn evaluate_context(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    derived: &dyn query::DerivedValues,
) -> Evaluation {
    evaluate_with_context(package, sources, limits, conversions, None, derived)
}
/// Evaluate the default resource entry with explicit conversions, source-bound
/// setters and derived reads. The original default-entry diagnostics and partial
/// resource semantics are shared with the context-free evaluation APIs.
pub fn evaluate_with_context(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    derived: &dyn query::DerivedValues,
) -> Evaluation {
    match evaluate_stored_context_with_settings(
        package,
        sources,
        limits,
        conversions,
        settings,
        derived,
    )
    .and_then(|result| Ok((result.evaluation, result.store.into_model()?)))
    {
        Ok((result, model)) => Evaluation {
            model,
            diagnostics: result.diagnostics,
            references: result.references,
            derived: result.derived,
        },
        Err(error) => Evaluation {
            model: Model::default(),
            diagnostics: vec![Diagnostic::new("model", 0, 0, error)],
            references: vec![],
            derived: BTreeMap::new(),
        },
    }
}

/// Atomically replace a model from source. Syntax, linking, structural and
/// semantic diagnostics roll back the entire import, retaining the previous
/// model and its indexes. Storage failures abort; failures encountered during
/// validation or opposite reconciliation may be attached as diagnostics.
/// Resource parsers construct directly into shared store savepoints. A compact
/// parser journal tracks pending references; current-resource tokens remain in memory.
/// Ok returns metadata for the attempted import, including when diagnostics
/// reject it. Only an empty diagnostics list means the replacement committed;
/// references and derived values from a rejected attempt do not describe the
/// retained store.
pub fn evaluate_into(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    store: &mut impl ModelStore,
) -> Result<StoredEvaluation, String> {
    evaluate_into_context(package, sources, limits, store, None)
}
pub fn evaluate_into_with_conversions(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    store: &mut impl ModelStore,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
) -> Result<StoredEvaluation, String> {
    evaluate_into_context(package, sources, limits, store, Some(conversions))
}
fn evaluate_into_context(
    package: &Package,
    sources: &[Source],
    limits: EvaluationLimits,
    store: &mut impl ModelStore,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
) -> Result<StoredEvaluation, String> {
    let mut result = StoredEvaluation::default();
    let mut rejected = false;
    let outcome = storage::atomic(store, |store| {
        storage::bind_schema(store, &package.schema)?;
        let mut after = None;
        loop {
            let page = store.id_page(after, 128)?;
            if page.is_empty() {
                break;
            }
            for (cursor, id) in page {
                after = Some(cursor);
                store.remove(&id)?;
            }
        }
        for uri in store.resource_records()?.keys() {
            store.remove_resource_record(uri)?;
        }
        result = evaluate_staged(
            package,
            sources,
            Invocation::resource(limits),
            store,
            conversions,
            None,
            &query::PackageDerived(package),
        )?;
        if !result.diagnostics.is_empty() {
            rejected = true;
            return Err("Import rejected by diagnostics".into());
        }
        Ok(())
    });
    match outcome {
        Ok(()) => Ok(result),
        Err(error) if rejected && error == "Import rejected by diagnostics" => Ok(result),
        Err(error) => Err(error),
    }
}

struct Invocation<'entry, 'values> {
    documents: Option<document::Request<'values>>,
    limits: EvaluationLimits,
    entry: Option<&'entry str>,
    values: Option<&'values mut BTreeMap<String, entry::EntryValue>>,
}
impl Invocation<'static, 'static> {
    fn resource(limits: EvaluationLimits) -> Self {
        Self {
            limits,
            entry: None,
            values: None,
            documents: None,
        }
    }
}
fn evaluate_staged(
    package: &Package,
    sources: &[Source],
    invocation: Invocation<'_, '_>,
    store: &mut impl ModelStore,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    derived: &dyn query::DerivedValues,
) -> Result<StoredEvaluation, String> {
    evaluate_staged_with_linking(
        package,
        sources,
        invocation,
        store,
        conversions,
        settings,
        derived,
        None,
    )
}

#[allow(clippy::too_many_arguments)] // Keep transient linking separate from existing resident invocation/parser layouts.
fn evaluate_staged_with_linking(
    package: &Package,
    sources: &[Source],
    invocation: Invocation<'_, '_>,
    store: &mut impl ModelStore,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    derived: &dyn query::DerivedValues,
    linking: Option<&mut linking::LinkingContext<'_>>,
) -> Result<StoredEvaluation, String> {
    evaluate_staged_with_parse_context(
        crate::package::TextPackageView::from_package(package),
        sources,
        invocation,
        store,
        conversions,
        settings,
        derived,
        linking,
        package,
        &mut resource_accounting::NoResourceAccounting,
    )
}
#[allow(clippy::too_many_arguments)]
fn evaluate_staged_with_parse_context<
    S: ModelStore,
    C: draft::ParseContext<S>,
    A: resource_accounting::ResourceAccounting,
>(
    package: crate::package::TextPackageView<'_>,
    sources: &[Source],
    mut invocation: Invocation<'_, '_>,
    store: &mut S,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    derived: &dyn query::DerivedValues,
    mut linking: Option<&mut linking::LinkingContext<'_>>,
    parse_context: &C,
    accounting: &mut A,
) -> Result<StoredEvaluation, String> {
    crate::rcore_scopes::validate_retained_scope_bindings_for_definition(package)?;
    if let Some(context) = linking.as_deref_mut() {
        if !package.semantics.scope_bindings.is_empty() {
            context
                .resolver
                .rcore_scopes()
                .ok_or("Source-authored Rcore scopes require their compiled scope context")?
                .validate_definition(package)?;
        }
        context.resolver.validate_definition(package)?;
    } else if !package.semantics.scope_bindings.is_empty() {
        return Err("Source-authored Rcore scopes require an installed Rcore scope context".into());
    }
    let limits = invocation.limits;
    let initializers = if package.semantics.initializers.is_empty() {
        None
    } else {
        Some(initialization::PackageInitializers::with_model_definition(
            package.model_definition(),
            package
                .compilation
                .ok_or("Missing retained initializer compilation inputs")?
                .bundle,
        )?)
    };
    let mut output = StoredEvaluation::default();
    let Some(grammar) = package.grammar.as_ref() else {
        output.diagnostics.push(Diagnostic::new(
            "model",
            0,
            0,
            "Ecore-only package has no text grammar; construct instances through the model API",
        ));
        return Ok(output);
    };
    let selected_entry = invocation.entry.unwrap_or(&grammar.entry);
    if invocation.entry.is_none() && grammar.parameterized_rules.contains(selected_entry) {
        for source in sources {
            output.diagnostics.push(Diagnostic::new(
                &source.uri,
                0,
                0,
                "Named parameterized rule has no no-argument parser entry",
            ));
        }
        return Ok(output);
    }
    if let Some(selected) = settings {
        if !selected.belongs_to_definition(package) {
            return Err("Text providers belong to another package".into());
        }
    } else if !grammar.computed_settings.is_empty() {
        return Err("Text computed-setting providers are not installed".into());
    }

    storage::bind_schema(store, package.schema)?;
    store.configure_schema(package.schema)?;
    let input_exceeded = limits.input_bytes.is_some_and(|limit| {
        sources
            .iter()
            .try_fold(0usize, |total, s| total.checked_add(s.text.len()))
            .is_none_or(|total| total > limit)
    });
    if limits.resources.is_some_and(|limit| sources.len() > limit) || input_exceeded {
        output.diagnostics.push(Diagnostic::new(
            "model",
            0,
            0,
            "Model resource budget exceeded",
        ));
        return Ok(output);
    }
    let reuse_link_metadata = linking.is_none()
        && invocation.documents.is_none()
        && conversions.is_none()
        && settings.is_none()
        && initializers.is_none()
        && accounting.reuse_link_metadata();
    let (pending, parsed_resources) = parse_resources_accounted(
        parse_context,
        sources,
        store,
        limits,
        &mut invocation,
        initializers.as_ref(),
        conversions,
        settings,
        selected_entry,
        &mut output,
        None,
        accounting,
    )?;
    linking::link_pending(
        package.schema,
        pending,
        store,
        &mut linking::LegacyPending { context: linking },
        &mut output,
        reuse_link_metadata,
    )?;
    reconcile_stored_opposites(store, package.schema, &mut output)?;
    if output.diagnostics.is_empty() {
        let validation = match conversions {
            Some(registry) => storage::validate_with_conversions(store, package.schema, registry),
            None => storage::validate(store, package.schema),
        };
        if let Err(error) = validation {
            output.diagnostics.push(Diagnostic::new(
                sources.first().map_or("model", |s| &s.uri),
                0,
                1,
                error,
            ));
        } else {
            intersect_model::model_rules::apply_model_with_provider(
                package.model_definition(),
                store,
                &mut output.derived,
                &mut output.diagnostics,
                derived,
            )?;
        }
    }
    attach_source_correspondence(store, sources, &parsed_resources, &output.diagnostics)?;
    Ok(output)
}

/// Recompute derived values through the shared storage evaluator and append
/// semantic diagnostics to the existing list. Use a fresh diagnostics list when
/// recomputing after edits to avoid retaining diagnostics from an earlier pass.
pub fn apply_semantics(package: &Package, result: &mut Evaluation) {
    apply_semantics_with_provider(package, result, &query::PackageDerived(package));
}

/// Recompute model semantics with an explicitly installed derived provider.
pub fn apply_semantics_with_provider<'a>(
    package: impl Into<intersect_model::definitions::model_definition::ModelDefinitionView<'a>>,
    result: &mut Evaluation,
    derived: &dyn query::DerivedValues,
) {
    let package = package.into();
    let mut metadata = StoredEvaluation::default();
    let outcome = storage::ModelView::new(&result.model).and_then(|view| {
        apply_stored_semantics_with_provider(package, &view, &mut metadata, derived)
    });
    result.derived = metadata.derived;
    result.diagnostics.extend(metadata.diagnostics);
    if let Err(error) = outcome {
        result
            .diagnostics
            .push(Diagnostic::new("model", 0, 0, error));
    }
}

/// Recompute derived values and append semantic diagnostics. The caller owns
/// diagnostic provenance: pass fresh metadata for an independent rerun instead
/// of accumulating obsolete findings. Shared model-rule services own execution;
/// stores supply generic indexed reads.
pub fn apply_stored_semantics(
    package: &Package,
    store: &impl ModelRead,
    result: &mut StoredEvaluation,
) -> Result<(), String> {
    apply_stored_semantics_with_provider(package, store, result, &query::PackageDerived(package))
}

pub fn apply_stored_semantics_with_provider<'a>(
    package: impl Into<intersect_model::definitions::model_definition::ModelDefinitionView<'a>>,
    store: &impl ModelRead,
    result: &mut StoredEvaluation,
    derived: &dyn query::DerivedValues,
) -> Result<(), String> {
    intersect_model::model_rules::apply_with_provider(
        package,
        store,
        &mut result.derived,
        &mut result.diagnostics,
        derived,
    )
}

#[allow(clippy::too_many_arguments)] // Exact shared stage; keep resident invocation and Machine holders unchanged.
#[cfg(test)]
fn parse_resources<'sources, S: ModelStore, C: draft::ParseContext<S>>(
    package: &C,
    sources: &'sources [Source],
    store: &mut S,
    limits: EvaluationLimits,
    invocation: &mut Invocation<'_, '_>,
    initializers: Option<&initialization::PackageInitializers>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    selected_entry: &str,
    output: &mut StoredEvaluation,
    work: Option<&mut usize>,
) -> Result<(Vec<Pending>, BTreeSet<&'sources str>), String> {
    parse_resources_accounted(
        package,
        sources,
        store,
        limits,
        invocation,
        initializers,
        conversions,
        settings,
        selected_entry,
        output,
        work,
        &mut resource_accounting::NoResourceAccounting,
    )
}

#[allow(clippy::too_many_arguments)] // Exact shared stage; keep resident invocation and Machine holders unchanged.
fn parse_resources_accounted<
    'sources,
    S: ModelStore,
    C: draft::ParseContext<S>,
    A: resource_accounting::ResourceAccounting,
>(
    package: &C,
    sources: &'sources [Source],
    store: &mut S,
    limits: EvaluationLimits,
    invocation: &mut Invocation<'_, '_>,
    initializers: Option<&initialization::PackageInitializers>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    settings: Option<&setting::TextSettings<'_>>,
    selected_entry: &str,
    output: &mut StoredEvaluation,
    mut work: Option<&mut usize>,
    accounting: &mut A,
) -> Result<(Vec<Pending>, BTreeSet<&'sources str>), String> {
    let grammar = package.grammar().ok_or("Missing parser grammar")?;
    if !C::LIVE_CALLERS
        && grammar
            .parser_prediction
            .as_ref()
            .is_some_and(|program| program.requires_runtime_capability())
    {
        return Err("Parser program requires its typed bounded live runtime capability".into());
    }
    // Language-wide tables: kept by the context when it can, else built here.
    let built;
    let tables = match package.parse_tables() {
        Some(tables) => tables,
        None => {
            built = ParseTables::build(package.schema(), grammar)?;
            &built
        }
    };
    let (keywords, punctuation, unsettable, lexical_types) = (
        &tables.keywords,
        &tables.punctuation,
        &tables.unsettable,
        &tables.lexical_types,
    );
    let mut pending = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut parsed_resources = BTreeSet::new();
    for source in sources {
        if !seen.insert(&source.uri) {
            output
                .diagnostics
                .push(Diagnostic::new(&source.uri, 0, 0, "Duplicate resource URI"));
            continue;
        }
        let limits = accounting.begin(limits);
        let mut resource_work = 0;
        let parsed_resource = (|| -> Result<(), String> {
            // Exact remaining-resource view, followed by unconditional debit
            // transfer. This is a ledger bridge, not a new allowance.
            let mut lexical_work = limits.steps_per_resource;
            let mut association_work =
                intersect_model::core::query::QueryBudget::new(lexical_work.unwrap_or(usize::MAX));
            let associated = package.source_resource(source, &mut association_work);
            if let Some(remaining) = lexical_work.as_mut() {
                *remaining = remaining
                    .checked_sub(association_work.reads)
                    .ok_or("Source/resource work exceeded its original allowance")?;
            }
            resource_work = association_work.reads;
            let associated = associated?;
            let resource = associated.map_or(source.uri.as_str(), |row| row.resource_uri());
            intersect_model::core::resources::create_resource(store, resource)?;
            let mut capture = invocation
                .documents
                .as_ref()
                .map(|request| match associated {
                    Some(row) => request.capture_associated(source, row),
                    None => request.capture(source),
                })
                .transpose()?;
            let lexed = if capture.is_some() {
                let prediction = grammar
                    .lexical_prediction
                    .as_ref()
                    .ok_or("Missing document lexical program")?;
                let lexical = match package.lexical_initial() {
                    Some(prepared) => grammar::lex_document_with_prepared_initial(
                        &source.text,
                        limits.tokens_per_resource,
                        &grammar.terminals,
                        prediction,
                        prepared,
                        &mut lexical_work,
                    ),
                    None => grammar::lex_document(
                        &source.text,
                        limits.tokens_per_resource,
                        &grammar.terminals,
                        prediction,
                        &mut lexical_work,
                    ),
                };
                lexical.and_then(|(tokens, issues, ranges)| {
                    capture
                        .as_mut()
                        .ok_or((0, "Missing document capture".into()))?
                        .lexical_issues(issues, ranges)
                        .map_err(|e| (0, e))?;
                    Ok(tokens)
                })
            } else {
                grammar::lex_borrowed_with_prediction(
                    &source.text,
                    limits.tokens_per_resource,
                    punctuation.as_ref(),
                    Some(&grammar.terminals),
                    if grammar.profile.has_scoped_hidden_tokens() {
                        Some(&[])
                    } else {
                        grammar.hidden.as_deref()
                    },
                    grammar.lexical_prediction.as_ref(),
                    &mut lexical_work,
                )
            };
            resource_work = limits
                .steps_per_resource
                .zip(lexical_work)
                .map_or(0, |(limit, remaining)| limit.saturating_sub(remaining));
            let tokens = match lexed {
                Ok(t) => t,
                Err((at, error)) => {
                    output
                        .diagnostics
                        .push(Diagnostic::new(&source.uri, at, at + 1, error));
                    return Ok(());
                }
            };
            let utf16_positions = if let Some(capture) = &capture {
                capture.token_positions()
            } else if grammar.rules.values().any(|rule| rule.datatype.is_some()) {
                datatype_text::utf16_offsets(&source.text, tokens.iter().map(|token| token.start))?
            } else {
                Vec::new()
            };
            let mut rejected = None;
            let document_mode = capture.is_some();
            let mut document_diagnostics = Vec::new();
            let parsed = storage::atomic(store, |store| {
                let mut machine = Machine {
                    nested_decisions: Default::default(),
                    document: capture.map(Box::new),
                    document_origin: None,
                    document_feature: None,
                    retained_tokens: grammar.profile.has_scoped_hidden_tokens(),
                    hidden_policy: grammar.hidden.as_deref().unwrap_or(&[]),
                    raw_entry: invocation.entry.is_some()
                        && grammar
                            .rules
                            .get(selected_entry)
                            .is_some_and(|rule| rule.datatype.is_some()),
                    construction: None,
                    settings,
                    raw_source: &source.text,
                    utf16_positions,
                    datatype_text: None,
                    initializers,
                    conversions,
                    lexical_types,
                    unsettable,
                    keywords,
                    undo: Vec::new(),
                    limits,
                    package,
                    uri: &source.uri,
                    tokens,
                    cursor: TokenCursor::default(),
                    datatype_span: None,
                    store,
                    elements: 0,
                    pending: vec![],
                    scalar_pending: BTreeMap::new(),
                    steps: limits
                        .steps_per_resource
                        .zip(lexical_work)
                        .map_or(0, |(limit, remaining)| limit.saturating_sub(remaining)),
                    depth: 0,
                    generic_fragment: false,
                    farthest: 0,
                    diagnostic_position: None,
                    expected: String::new(),
                };
                // A successfully consumed resource can have no EObject root. Keep
                // that outcome separate from syntax rejection and storage failure.
                let machine_entry = machine.steps;
                machine.audit_machine_work(
                    "resource_machine_begin",
                    Some(selected_entry),
                    machine_entry,
                    &Ok::<(), ParseError>(()),
                );
                let result = machine.entry(selected_entry);
                machine.audit_machine_work(
                    "resource_machine",
                    Some(selected_entry),
                    machine_entry,
                    &result,
                );
                resource_work = machine.steps;
                if let Some(work) = work.as_deref_mut() {
                    *work = machine.steps;
                }
                if let Err(ParseError::Storage(error)) = result {
                    return Err(error);
                }
                if document_mode && let Err(ParseError::Budget(error)) = &result {
                    return Err(error.clone());
                }
                // Document recovery covers syntax errors only; an ambiguous
                // prediction stops the document with its source offset.
                if document_mode && let Err(ParseError::Ambiguous(error)) = &result {
                    let offset = machine
                        .diagnostic_position
                        .and_then(|position| machine.tokens.get(position))
                        .map_or(source.text.len(), |token| token.start);
                    return Err(format!("{error} (offset {offset})"));
                }
                if let Some(document) = machine.document.take() {
                    let mut snapshot = (*document).finish()?;
                    for node in snapshot.document.nodes() {
                        if let Some(index) = node.data.syntax_error {
                            let issue = &snapshot.syntax_issues[index];
                            if let Some(range) = snapshot.document.utf8_range(node.range)? {
                                document_diagnostics.push(Diagnostic::new(
                                    &source.uri,
                                    range.start,
                                    range.end,
                                    issue.message.clone(),
                                ));
                            }
                        }
                    }
                    if !snapshot.syntax_issues.is_empty() {
                        for node in snapshot
                            .document
                            .nodes()
                            .iter()
                            .filter(|n| n.data.direct_semantic_association)
                        {
                            if let Some(index) = node.data.semantic_object {
                                let range = snapshot
                                    .document
                                    .utf8_range(node.range)?
                                    .ok_or("Recovered object has an interior UTF16 boundary")?;
                                machine.store.set_span(
                                    &format!("{}#{index}", source.uri),
                                    range.start,
                                    range.end,
                                )?;
                            }
                        }
                    }
                    snapshot.capture_parse_stage(
                        machine.store,
                        machine.elements,
                        &machine.pending,
                        result.as_ref().ok(),
                        grammar
                            .rules
                            .get(selected_entry)
                            .is_some_and(|r| r.datatype.is_some()),
                        invocation
                            .documents
                            .as_ref()
                            .ok_or("Missing document request")?
                            .limits(),
                    )?;
                    invocation
                        .documents
                        .as_mut()
                        .ok_or("Missing document request")?
                        .push(snapshot);
                }
                if result.is_err() || machine.cursor.position != machine.tokens.len() {
                    let at = machine.tokens.get(
                        machine
                            .diagnostic_position
                            .unwrap_or_else(|| machine.farthest.max(machine.cursor.position)),
                    );
                    let start = at.map_or(source.text.len(), |t| t.start);
                    let end = at.map_or(start, |t| t.end);
                    let message = result.err().map(ParseError::message).unwrap_or_else(|| {
                        if machine.expected.is_empty() {
                            "Unexpected trailing input".into()
                        } else {
                            machine.expected.clone()
                        }
                    });
                    rejected = Some(Diagnostic::new(&source.uri, start, end, message));
                    if document_mode {
                        Ok((machine.pending, Value::Null))
                    } else {
                        Err("Resource parse rejected".into())
                    }
                } else {
                    Ok((machine.pending, result.map_err(ParseError::message)?))
                }
            });
            match parsed {
                Ok((references, value)) => {
                    if document_mode
                        && document_diagnostics.is_empty()
                        && let Some(diagnostic) = rejected.take()
                    {
                        output.diagnostics.push(diagnostic);
                    }
                    output.diagnostics.append(&mut document_diagnostics);
                    if let Some(values) = invocation.values.as_mut() {
                        let selected = if grammar
                            .rules
                            .get(selected_entry)
                            .is_some_and(|rule| rule.datatype.is_some())
                        {
                            entry::EntryValue::Datatype(
                                value
                                    .as_str()
                                    .ok_or("Datatype entry did not return text")?
                                    .to_owned(),
                            )
                        } else if value.is_null() {
                            entry::EntryValue::Empty
                        } else {
                            entry::EntryValue::Object(
                                value
                                    .as_str()
                                    .ok_or("Object entry did not return an identity")?
                                    .to_owned(),
                            )
                        };
                        values.insert(source.uri.clone(), selected);
                    }
                    parsed_resources.insert(source.uri.as_str());
                    pending.extend(references);
                }
                Err(error) if rejected.is_some() && error == "Resource parse rejected" => {
                    if let Some(diagnostic) = rejected {
                        output.diagnostics.push(diagnostic);
                    }
                }
                Err(error) => return Err(error),
            }
            Ok(())
        })();
        if C::AUDIT_PREDICTION {
            package.audit_machine_work(draft::MachineWorkAudit {
                uri: &source.uri,
                phase: "resource_report",
                detail: Some(selected_entry),
                before: 0,
                after: resource_work,
                resource_grant: limits.steps_per_resource,
                result: parsed_resource.as_ref().map(|_| ()).map_err(String::as_str),
            });
        }
        // Debit all lex/parser work even if an existing storage/capture error
        // left the resource early. Accounting adds no parser work units.
        accounting.finish(resource_work, &mut work)?;
        parsed_resource?;
    }
    Ok((pending, parsed_resources))
}

fn reconcile_stored_opposites(
    store: &mut impl ModelStore,
    schema: &intersect_model::definitions::schema::Schema,
    output: &mut StoredEvaluation,
) -> Result<(), String> {
    reconcile_stored_opposites_with_metadata(store, schema, output, None)
}
fn reconcile_stored_opposites_with_metadata(
    store: &mut impl ModelStore,
    schema: &intersect_model::definitions::schema::Schema,
    output: &mut StoredEvaluation,
    metadata: Option<&intersect_model::definitions::preparation::PreparedPhysicalMetadata<'_>>,
) -> Result<(), String> {
    // Retain original populated endpoints, rather than a second model snapshot.
    let mut opposite_edits = Vec::new();
    let mut opposite_fields = BTreeMap::new();
    for name in schema.classes.keys() {
        opposite_fields.insert(
            name.as_str(),
            schema
                .features(name)?
                .into_iter()
                .filter(|(_, field)| field.opposite.is_some())
                .collect::<Vec<_>>(),
        );
    }
    storage::visit_views(store, |element| {
        for (name, field) in opposite_fields
            .get(element.class())
            .ok_or("Unknown class")?
        {
            if field.opposite.is_some()
                && let Some(value) = element
                    .value(name)
                    .filter(|v| !v.is_null() && v.collection().is_none_or(|a| !a.is_empty()))
            {
                opposite_edits.push((element.id().to_owned(), name.clone(), value.to_json()));
            }
        }
        Ok(())
    })?;
    for (id, name, value) in opposite_edits {
        let result = match metadata {
            Some(metadata) => {
                storage::set_with_opposite_prepared(store, schema, metadata, &id, &name, value)
            }
            None => storage::set_with_opposite(store, schema, &id, &name, value),
        };
        if let Err(error) = result {
            let element = store.get(&id)?.ok_or("Missing opposite owner")?;
            output.diagnostics.push(Diagnostic::new(
                &element.resource,
                element.start,
                element.end,
                error,
            ));
        }
    }
    Ok(())
}

fn attach_source_correspondence(
    store: &mut impl ModelStore,
    sources: &[Source],
    parsed_resources: &BTreeSet<&str>,
    diagnostics: &[Diagnostic],
) -> Result<(), String> {
    attach_source_correspondence_with_resources(
        store,
        sources,
        parsed_resources,
        diagnostics,
        |_| Ok(None),
    )
}
fn attach_source_correspondence_with_resources<'a>(
    store: &mut impl ModelStore,
    sources: &[Source],
    parsed_resources: &BTreeSet<&str>,
    diagnostics: &[Diagnostic],
    mut association: impl FnMut(
        &Source,
    ) -> Result<
        Option<&'a intersect_model::core::text_document::DocumentResourceAssociation>,
        String,
    >,
) -> Result<(), String> {
    // Attach correspondence after linking/opposite writes, which invalidate old
    // source evidence. Parse-rejected resources remain resident and explicitly
    // diagnosed, without claiming element/source correspondence.
    let mut retained = BTreeSet::new();
    for source in sources {
        if !retained.insert(source.uri.as_str()) {
            continue;
        }
        let associated = association(source)?;
        let resource = associated.map_or(source.uri.as_str(), |row| row.resource_uri());
        if let Some(mut record) = store.resource_record(resource)? {
            record.source = if parsed_resources.contains(source.uri.as_str()) {
                let identity = intersect_model::definitions::bundle::SourceId::for_text(
                    &source.uri,
                    &source.text,
                );
                Some(intersect_model::core::resources::SourceCorrespondence {
                    uri: identity.uri,
                    revision: identity.sha256,
                })
            } else {
                None
            };
            record.diagnostics = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.source == source.uri)
                .map(|diagnostic| {
                    format!(
                        "{}..{}: {}",
                        diagnostic.start, diagnostic.end, diagnostic.message
                    )
                })
                .collect();
            store.set_resource_record(record)?;
        }
    }
    Ok(())
}
