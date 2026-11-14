//! Explicit construction-only consumer of an opaque prepared definition.
//! Raw draft observations never grant a validated language session.
use super::*;
use intersect_model::definitions::schema::FeatureLookup;
use std::{cell::RefCell, sync::Arc};
use {
    crate::preparation::PreparedLanguageDefinition, intersect_model::core::query::QueryBudget,
    intersect_model::definitions::bundle::SourceId,
    intersect_model::definitions::schema::MemberIdentity,
    intersect_model::definitions::schema::Schema,
    intersect_model::storage::DraftConstructionContext,
    intersect_model::storage::DraftPhysicalDefaults, intersect_model::storage::StorageResult,
};
#[path = "draft_open.rs"]
mod open;
pub use open::{GrammarPolicy, OpenDraftCapability, StringRuleConversion};
#[path = "draft_resources.rs"]
mod source_resources;
pub use source_resources::ClosedSourceResources;
#[path = "draft_live_callers.rs"]
pub(crate) mod live_callers;
#[path = "draft_prediction_sources.rs"]
pub(crate) mod prediction_sources;

#[path = "draft_assignment_routes.rs"]
mod assignment_routes;

/// Borrowed observation of ONE actual shared parser prediction invocation.
/// Cursor positions are retained-token ordinals, not byte/UTF-16 spans. The
/// result is the original worker result before ParseError mapping. No grammar,
/// model, store, budget or semantic authority is exposed or retained here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PredictionAudit<'a> {
    pub uri: &'a str,
    pub decision: &'a str,
    pub cursor_start: usize,
    pub input_furthest: usize,
    pub remaining_before: Option<usize>,
    pub remaining_after: Option<usize>,
    pub result: Result<Option<usize>, &'a str>,
}

/// Borrowed passive samples; no work reference, model or recognition authority.
#[derive(Clone, Copy, Debug)]
pub enum RuntimePredictionWorkDetail {
    Phase(&'static str, Option<usize>),
    Geometry(grammar::RuntimeWorkGeometry),
}
#[derive(Clone, Copy, Debug)]
pub struct RuntimePredictionWorkAudit<'a> {
    pub uri: &'a str,
    pub decision: &'a str,
    pub detail: RuntimePredictionWorkDetail,
}

/// Local Machine counter observation, never a shared caller balance.
/// The `_begin` rows start predict/create_current/resource_machine intervals:
/// before==after and Ok means started, not completed. Existing unsuffixed rows
/// close the SAME synchronous interval, including failures. No clock is read.
/// `create_current` and `predict` are disjoint child intervals. `resource_machine`
/// encloses them and starts after lex/source association; `resource_report` is
/// the complete resource work reported immediately before the existing caller
/// transfer, including early lexical failures. These enclosing rows overlap.
/// `resource_grant` is this resource's actual local limit, not a new allowance.
/// Results are observed before returning/moving their original error values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineWorkAudit<'a> {
    pub uri: &'a str,
    pub phase: &'static str,
    pub detail: Option<&'a str>,
    pub before: usize,
    pub after: usize,
    pub resource_grant: Option<usize>,
    pub result: Result<(), &'a str>,
}

/// Authoritative shared caller balance. Legacy phase-local accounting is unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineCallerWork {
    pub reads: usize,
    pub remaining: usize,
}
/// One completed canonical operation, including its actual failure before mapping.
/// Group operations have no single source identity. No budget/model authority escapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineAudit<'a> {
    pub phase: &'static str,
    pub before: Option<PipelineCallerWork>,
    pub after: Option<PipelineCallerWork>,
    pub result: Result<(), &'a str>,
}

/// A native language policy, not a deserialized receipt or a semantic provider.
pub trait DraftCapability: DraftPhysicalDefaults {
    /// Default false preserves the original physical-write path and callbacks.
    fn has_delegated_assignments(&self) -> bool {
        false
    }
    /// Source-bound routing metadata only. The caller still validates the actual
    /// row, both canonical members/defaults and invokes the existing setter once.
    fn delegated_assignment_target<'a>(
        &'a self,
        _receiver: &str,
        _member: &MemberIdentity,
        _append: bool,
        _budget: &mut QueryBudget,
    ) -> StorageResult<Option<&'a MemberIdentity>> {
        Ok(None)
    }
    fn permits_scalar_postprocess(
        &self,
        _trigger: &str,
        _actual: &str,
        _member: &MemberIdentity,
        _budget: &mut QueryBudget,
    ) -> StorageResult<bool> {
        Ok(false)
    }

    fn permits_postprocess_reference(&self, _actual: &str, _member: &MemberIdentity) -> bool {
        false
    }

    /// Explicit closed-project logical-resource policy. Only associated input
    /// binding captures this callback, exactly once including None. The ordinary
    /// closed/direct APIs do not activate it or assume provider stability.
    fn source_resources(&self) -> Option<&ClosedSourceResources> {
        None
    }
    /// Native diagnostic adapter only. Existing clients perform no callback.
    const AUDIT_PREDICTION: bool = false;
    const AUDIT_RUNTIME_WORK: bool = false;
    /// `Some` is the actual immutable source binding owned by the selected
    /// PreparedDraftContext. `None` explicitly means association unavailable.
    /// Implementations must remain bounded and deterministic. Observation
    /// overhead is outside semantic work; it grants no CPU/RSS/S09 bound.
    fn audit_prediction(&self, _source: Option<&SourceId>, _event: PredictionAudit<'_>) {}
    /// Uses the same native diagnostic opt-in as prediction observations. Capture
    /// must be bounded/deterministic; overhead is outside semantic work, not S09.
    fn audit_pipeline(&self, _event: PipelineAudit<'_>) {}
    /// Same diagnostic opt-in; no local Machine counter is a caller balance.
    fn audit_machine_work(&self, _source: Option<&SourceId>, _event: MachineWorkAudit<'_>) {}
    fn audit_runtime_work(
        &self,
        _source: Option<&SourceId>,
        _event: RuntimePredictionWorkAudit<'_>,
    ) {
    }
    /// Existing native capabilities keep their original callbacks and work.
    const NEEDS_VALUE_ADMISSION: bool = false;
    /// Explicit borrowed source-bound Boolean rule policy; old paths are disabled.
    const HAS_BOOLEAN_RULE_CONVERSIONS: bool = false;
    /// Explicit language-owned EString conversion; default contexts retain raw values.
    const HAS_STRING_RULE_CONVERSIONS: bool = false;
    /// Explicit source-bound continuation on the SAME fresh atomic object.
    /// Default contexts perform no lookup or callback.
    const HAS_NATIVE_INITIALIZERS: bool = false;
    fn native_initializer(
        &self,
        _actual: &str,
        _budget: &mut QueryBudget,
    ) -> StorageResult<Option<storage::InitializationBinding<'_>>> {
        Err("Unsupported source-bound native draft initializer".into())
    }
    /// Opt-in native initialization of computed-setting-created objects. The
    /// default preserves legacy creation and performs no initializer lookup.
    fn initializes_computed_created_elements(&self) -> bool {
        false
    }
    /// Opt-in physical inverse maintenance for parsed containment appends.
    /// This grants no language alias, getter or validation authority.
    const RECONCILE_PARSED_CONTAINMENT: bool = false;
    /// Optional admission over a value already produced by the shared parser.
    /// It supplies no conversion, postprocessing, linking or semantic authority.
    fn validate_assignment_value(
        &self,
        _receiver: &str,
        _member: &MemberIdentity,
        _append: bool,
        _value: &Value,
        _budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        Err("Unsupported draft parsed value admission".into())
    }
    /// Explicit language agreement for installed computed setters. Physical
    /// assignment authority remains separate, including derived operands.
    fn permits_computed_assignment(
        &self,
        _receiver: &str,
        _member: &MemberIdentity,
        _append: bool,
    ) -> bool {
        false
    }
    /// Paid admission for an explicitly selected language implementation.
    /// Existing profiles retain the same boolean, callbacks and zero added debit.
    fn permits_computed_assignment_observed(
        &self,
        receiver: &str,
        member: &MemberIdentity,
        append: bool,
        _budget: &mut QueryBudget,
    ) -> StorageResult<bool> {
        Ok(self.permits_computed_assignment(receiver, member, append))
    }
    /// Complete explicit bindings for this language capability. A missing
    /// binding on the opt-in route is Unsupported, never builtin/Package fallback.
    fn boolean_rule_conversions(
        &self,
    ) -> &[intersect_model::definitions::rule_conversion::RuleConversion] {
        &[]
    }
    /// Convert one committed outer EString rule. None leaves an unselected rule raw.
    /// Exact rule/source/return binding and value semantics belong to the language.
    fn convert_string_rule(
        &self,
        _grammar: &grammar::Grammar,
        _rule: &str,
        _spelling: &str,
        _budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        Err("Unsupported draft EString rule conversion".into())
    }
    /// A language's conversion of a terminal's text (an Xtext terminal value
    /// converter); `None` keeps the text.
    fn convert_terminal(&self, _terminal: &str, _text: &str) -> StorageResult<Option<String>> {
        Ok(None)
    }
    fn original_sources(&self) -> &[SourceId];
    fn converted_sources(&self) -> &[SourceId];
    fn permits_assignment(&self, receiver: &str, member: &MemberIdentity, append: bool) -> bool;
}

#[derive(Clone, Copy)]
pub(crate) struct ParseLocation<'a> {
    pub uri: &'a str,
    pub start: usize,
    pub end: usize,
    pub path: Option<&'a str>,
    pub capture_origin: Option<usize>,
}
pub(crate) struct ParserCreation<'a> {
    pub class: &'a str,
    pub id: &'a str,
    pub uri: &'a str,
    pub start: usize,
    pub remaining: usize,
    pub conversions: Option<&'a intersect_model::definitions::conversion::ConversionRegistry<'a>>,
    pub initializers: Option<&'a initialization::PackageInitializers>,
    pub location: ParseLocation<'a>,
}

/// Transient dispatch only. Machine retains its original concrete resident fields.
pub(crate) trait ParserComputedSettings<S: ModelStore> {
    fn supports(&self, class: &str, feature: &str) -> bool;
    #[allow(clippy::too_many_arguments)]
    fn apply(
        &self,
        store: &mut S,
        id: &str,
        target: (&str, &str),
        edit: storage::ComputedEdit,
        budget: &mut QueryBudget,
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    ) -> StorageResult<()>;
}
impl<S: ModelStore> ParserComputedSettings<S> for setting::TextSettings<'_> {
    fn supports(&self, class: &str, feature: &str) -> bool {
        setting::TextSettings::supports(self, class, feature)
    }
    fn apply(
        &self,
        store: &mut S,
        id: &str,
        target: (&str, &str),
        edit: storage::ComputedEdit,
        budget: &mut QueryBudget,
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    ) -> StorageResult<()> {
        setting::TextSettings::apply(self, store, id, target, edit, budget, conversions)
    }
}
/// Concrete default instantiation stays a single &Package pointer in Machine.
pub(crate) trait ParseContext<S: ModelStore> {
    /// Language-wide parser tables when the context keeps them; otherwise the
    /// parser builds them for each call.
    fn parse_tables(&self) -> Option<&crate::ParseTables> {
        None
    }
    /// Prepared metadata proving that `schema()` passed runtime validation,
    /// so checked writes do not revalidate the whole schema on every call.
    fn physical_metadata(
        &self,
    ) -> Option<&intersect_model::definitions::preparation::PreparedPhysicalMetadata<'_>> {
        None
    }
    /// Effective features per class, when the context keeps them.
    fn feature_index(&self) -> Option<&dyn intersect_model::definitions::schema::FeatureLookup> {
        None
    }
    /// A language's conversion of a terminal's text; `None` keeps the text.
    fn convert_terminal(&self, _terminal: &str, _text: &str) -> StorageResult<Option<String>> {
        Ok(None)
    }
    const LIVE_CALLERS: bool = false;
    fn live_entry(
        &self,
        _uri: &str,
        _entry: &str,
        _begin: bool,
        _success: bool,
        _work: &mut Option<usize>,
    ) -> StorageResult<()> {
        Err("Unsupported live entry context".into())
    }
    fn live_call(
        &self,
        _uri: &str,
        _expression: &Expr,
        _enter: bool,
        _work: &mut Option<usize>,
    ) -> StorageResult<()> {
        Err("Unsupported live caller context".into())
    }
    fn live_prediction(
        &self,
        _request: prediction_sources::Request<'_>,
        _input: &mut impl grammar::PredictionInput,
        _work: &mut Option<usize>,
    ) -> StorageResult<Option<usize>> {
        Err("Unsupported live prediction context".into())
    }
    fn lexical_initial(&self) -> Option<&grammar::PreparedLexicalInitial<'_>> {
        None
    }
    const SOURCE_PREDICTION: bool = false;
    fn source_prediction(
        &self,
        _request: prediction_sources::Request<'_>,
        _input: &mut impl grammar::PredictionInput,
        _work: &mut Option<usize>,
    ) -> StorageResult<Option<usize>> {
        Err("Unsupported private prediction source context".into())
    }
    const AUDIT_PREDICTION: bool = false;
    const AUDIT_RUNTIME_WORK: bool = false;
    fn audit_prediction(&self, _event: PredictionAudit<'_>) {}
    fn audit_machine_work(&self, _event: MachineWorkAudit<'_>) {}
    fn audit_runtime_work(&self, _event: RuntimePredictionWorkAudit<'_>) {}
    const NEEDS_ADMISSION: bool = false;
    const NEEDS_VALUE_ADMISSION: bool = false;
    /// Explicit borrowed source-bound Boolean rule policy; old paths are disabled.
    const HAS_BOOLEAN_RULE_CONVERSIONS: bool = false;
    /// Explicit language-owned EString conversion; default contexts retain raw values.
    const HAS_STRING_RULE_CONVERSIONS: bool = false;
    const RECONCILE_PARSED_CONTAINMENT: bool = false;
    const CONTEXT_SETTINGS: bool = false;
    fn has_delegated_assignments(&self) -> bool {
        false
    }
    fn delegated_assignment_target(
        &self,
        _store: &S,
        _assignment: (&str, &str, bool),
        _value: &Value,
        _location: ParseLocation<'_>,
        _budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        Ok(None)
    }
    fn source_resource(
        &self,
        _source: &Source,
        _budget: &mut QueryBudget,
    ) -> StorageResult<Option<&intersect_model::core::text_document::DocumentResourceAssociation>>
    {
        Ok(None)
    }
    fn computed_settings(&self) -> Option<&dyn ParserComputedSettings<S>> {
        None
    }
    fn schema(&self) -> &Schema;
    fn grammar(&self) -> Option<&grammar::Grammar>;
    fn legacy_package(&self) -> Option<&Package>;
    fn execution_definition(&self) -> Option<crate::package::TextPackageView<'_>> {
        self.legacy_package()
            .map(crate::package::TextPackageView::from_package)
    }

    fn convert_boolean_rule(
        &self,
        _rule: &str,
        _spelling: &str,
        _location: ParseLocation<'_>,
        _budget: &mut QueryBudget,
    ) -> StorageResult<Value> {
        Err("Unsupported draft Boolean rule conversion".into())
    }
    fn convert_string_rule(
        &self,
        _rule: &str,
        _spelling: &str,
        _location: ParseLocation<'_>,
        _budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        Err("Unsupported draft EString rule conversion".into())
    }
    fn create(
        &self,
        store: &mut S,
        request: ParserCreation<'_>,
    ) -> (StorageResult<()>, usize, bool);
    fn admit(
        &self,
        _store: &S,
        _id: &str,
        _feature: &str,
        _append: bool,
        _location: ParseLocation<'_>,
        _budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        Ok(())
    }
    fn admit_value(
        &self,
        _store: &S,
        _assignment: (&str, &str, bool),
        _value: &Value,
        _location: ParseLocation<'_>,
        _budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        Ok(())
    }
}
impl<S: ModelStore> ParseContext<S> for Package {
    fn schema(&self) -> &Schema {
        &self.schema
    }
    fn grammar(&self) -> Option<&grammar::Grammar> {
        self.grammar.as_ref()
    }
    fn legacy_package(&self) -> Option<&Package> {
        Some(self)
    }
    fn create(
        &self,
        store: &mut S,
        request: ParserCreation<'_>,
    ) -> (StorageResult<()>, usize, bool) {
        create_for_definition(self.model_definition(), store, request)
    }
}
pub(crate) fn create_for_definition<S: ModelStore>(
    model: intersect_model::definitions::model_definition::ModelDefinitionView<'_>,
    store: &mut S,
    request: ParserCreation<'_>,
) -> (StorageResult<()>, usize, bool) {
    if model
        .semantics()
        .initializers
        .iter()
        .any(|declaration| declaration.class == request.class)
    {
        let mut budget = QueryBudget::new(request.remaining);
        let result = (|| {
            request
                .initializers
                .ok_or("Missing initializer dispatch")?
                .create_for_text(
                    store,
                    model.schema(),
                    request.class,
                    storage::Construction {
                        id: request.id,
                        resource: request.uri,
                        authored: &[],
                    },
                    request.conversions,
                    &mut budget,
                )?;
            Ok(())
        })();
        (result, budget.reads, true)
    } else {
        (
            store.create_element(
                model.schema(),
                request.id,
                request.class,
                request.uri,
                request.start,
                request.start,
            ),
            0,
            false,
        )
    }
}

#[derive(Debug, Serialize)]
pub struct DraftRejection {
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
    pub message: String,
    pub grammar_origin: Option<document::GrammarOrigin>,
    pub member: Option<MemberIdentity>,
}
// Reporting ownership only. A nonempty existing diagnostic Vec is moved into
// one bounded Box header; the entries are never cloned or recomputed. Empty
// failures allocate no header. This allocation is not semantic query work.
#[derive(Debug, Serialize)]
#[serde(transparent)]
struct DraftFailureDiagnostics(Vec<Diagnostic>);
const _: () = assert!(std::mem::size_of::<DraftFailureDiagnostics>() <= 24);
#[derive(Debug, Serialize)]
pub struct DraftFailure {
    pub message: String,
    /// Already generated diagnostics retained on failure. These observations
    /// do not identify the cause of a later error or confer semantic validity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    diagnostics: Option<Box<DraftFailureDiagnostics>>,
    pub rejection: Option<Box<DraftRejection>>,
    pub source: SourceId,
    pub rollback_unchanged: bool,
    pub work: usize,
    pub phase: &'static str,
}
impl DraftFailure {
    /// Previously generated observations. Their presence does not establish
    /// which observation caused a later error or confer semantic validity.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostics
            .as_deref()
            .map_or(&[], |owned| owned.0.as_slice())
    }
}
const _: () = assert!(std::mem::size_of::<DraftFailure>() <= 120);

/// Distinct consumer. PreparedLanguageDefinition itself remains inspection-only.
pub struct DraftLanguage<'d, 'i, C: DraftCapability> {
    /// Parser tables of this language, built on first parse.
    tables: std::sync::OnceLock<Result<crate::ParseTables, String>>,
    definition: &'d PreparedLanguageDefinition<'i>,
    capability: &'d C,
    construction: DraftConstructionContext<'d>,
    origins: Arc<document::Origins>,
    binding_work: usize,
    /// Runtime decision selections retained across the sources this language
    /// parses with `parse_runtime_into_with_settings`.
    prediction_memo: grammar::RuntimePredictionMemo,
}
impl<'d, 'i, C: DraftCapability> DraftLanguage<'d, 'i, C> {
    pub fn bind(
        definition: &'d PreparedLanguageDefinition<'i>,
        capability: &'d C,
        limits: document::AnalysisLimits,
    ) -> StorageResult<Self> {
        Self::bind_constructor_mode(definition, capability, limits)
    }
    fn bind_constructor_mode(
        definition: &'d PreparedLanguageDefinition<'i>,
        capability: &'d C,
        limits: document::AnalysisLimits,
    ) -> StorageResult<Self> {
        if (
            std::mem::size_of::<Machine<'_, '_, FrozenStore>>(),
            std::mem::align_of::<Machine<'_, '_, FrozenStore>>(),
        ) != (
            std::mem::size_of::<Machine<'_, '_, FrozenStore, PreparedDraftContext<'_, '_, '_, C>>>(
            ),
            std::mem::align_of::<Machine<'_, '_, FrozenStore, PreparedDraftContext<'_, '_, '_, C>>>(
            ),
        ) {
            return Err("Concrete draft context widened the existing Machine holder".into());
        }
        let original: Vec<_> = definition
            .original_sources()
            .sources
            .iter()
            .map(|source| source.id.clone())
            .collect();
        let converted: Vec<_> = definition
            .sources()
            .sources
            .iter()
            .map(|source| source.id.clone())
            .collect();
        if capability.original_sources() != original || capability.converted_sources() != converted
        {
            return Err("Draft capability belongs to another complete source selection".into());
        }
        let grammar = definition
            .grammar()
            .ok_or("Draft consumer needs the complete prepared grammar")?;
        if !grammar.profile.has_durable_documents() {
            return Err("Draft consumer requires durable document capture".into());
        }
        let mut budget = QueryBudget::new(limits.association_operations);
        let construction = DraftConstructionContext::bind(
            definition.schema(),
            definition.sources(),
            capability,
            &mut budget,
        )?;
        let origins = document::Origins::prepare_parts(
            grammar,
            |id| {
                definition
                    .original_sources()
                    .sources
                    .iter()
                    .chain(&definition.sources().sources)
                    .find(|source| source.id == *id)
                    .map(|source| source.text.as_str())
                    .ok_or_else(|| {
                        "Missing exact prepared grammar source for document origin".to_owned()
                    })
            },
            limits,
        )?;
        Ok(Self {
            tables: std::sync::OnceLock::new(),
            definition,
            capability,
            construction,
            origins: Arc::new(origins),
            binding_work: budget.reads,
            prediction_memo: grammar::RuntimePredictionMemo::default(),
        })
    }
    pub fn binding_work(&self) -> usize {
        self.binding_work
    }
    /// The runtime prediction memo of this language: its mode (on by default;
    /// `Verify` also runs every reused decision uncached and fails on any
    /// difference) and its counters.
    pub fn prediction_memo(&self) -> &grammar::RuntimePredictionMemo {
        &self.prediction_memo
    }
    /// A failed build is not kept here; the parser builds again and reports it.
    fn parse_tables(&self) -> Option<&crate::ParseTables> {
        let grammar = self.definition.grammar()?;
        self.tables
            .get_or_init(|| crate::ParseTables::build(self.definition.schema(), grammar))
            .as_ref()
            .ok()
    }
}

struct PreparedDraftContext<'a, 'd, 'i, C: DraftCapability> {
    language: &'a DraftLanguage<'d, 'i, C>,
    source: SourceId,
    rejection: RefCell<Option<DraftRejection>>,
}
impl<'a, 'd, 'i, C: DraftCapability> PreparedDraftContext<'a, 'd, 'i, C> {
    fn new(language: &'a DraftLanguage<'d, 'i, C>, source: SourceId) -> Self {
        Self {
            language,
            source,
            rejection: RefCell::new(None),
        }
    }
}
impl<C: DraftCapability> PreparedDraftContext<'_, '_, '_, C> {
    fn assignment_member_in<S: ModelStore>(
        &self,
        lookup: &dyn FeatureLookup,
        store: &S,
        id: &str,
        feature: &str,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<(String, MemberIdentity)> {
        budget.charge()?;
        let mut header = None;
        store.inspect(id, &mut |object| {
            header = Some((object.class().to_owned(), object.type_identity().to_owned()));
            Ok(())
        })?;
        let (class, identity) = header.ok_or("Missing draft actual receiver")?;
        if identity != <Self as ParseContext<S>>::schema(self).identity(&class) {
            return Err(self.reject(
                "Draft actual receiver identity differs from its source contract".into(),
                location,
                None,
            ));
        }
        let member = lookup.effective_feature(&class, feature)?.identity;
        Ok((class, member))
    }

    fn reject(
        &self,
        message: String,
        location: ParseLocation<'_>,
        member: Option<MemberIdentity>,
    ) -> String {
        let message = if location.uri == self.source.uri {
            message
        } else {
            "Draft rejection source identity mismatch".into()
        };
        let origin = location
            .path
            .and_then(|path| self.language.origins.origin(path))
            .or_else(|| {
                location
                    .capture_origin
                    .and_then(|index| self.language.origins.indexed_origin(index))
            })
            .cloned();
        *self.rejection.borrow_mut() = Some(DraftRejection {
            source: self.source.clone(),
            start: location.start,
            end: location.end,
            grammar_origin: origin,
            member,
            message: message.clone(),
        });
        message
    }
}
impl<S: ModelStore, C: DraftCapability> ParseContext<S> for PreparedDraftContext<'_, '_, '_, C> {
    fn convert_terminal(&self, terminal: &str, text: &str) -> StorageResult<Option<String>> {
        self.language.capability.convert_terminal(terminal, text)
    }
    fn parse_tables(&self) -> Option<&crate::ParseTables> {
        self.language.parse_tables()
    }
    const AUDIT_PREDICTION: bool = C::AUDIT_PREDICTION;
    const AUDIT_RUNTIME_WORK: bool = C::AUDIT_RUNTIME_WORK;
    fn audit_prediction(&self, event: PredictionAudit<'_>) {
        let source = (self.source.uri == event.uri).then_some(&self.source);
        self.language.capability.audit_prediction(source, event);
    }
    fn audit_runtime_work(&self, event: RuntimePredictionWorkAudit<'_>) {
        let source = (self.source.uri == event.uri).then_some(&self.source);
        self.language.capability.audit_runtime_work(source, event);
    }
    fn audit_machine_work(&self, event: MachineWorkAudit<'_>) {
        let source = (self.source.uri == event.uri).then_some(&self.source);
        self.language.capability.audit_machine_work(source, event);
    }
    const NEEDS_ADMISSION: bool = true;
    const NEEDS_VALUE_ADMISSION: bool = C::NEEDS_VALUE_ADMISSION;
    const HAS_BOOLEAN_RULE_CONVERSIONS: bool = C::HAS_BOOLEAN_RULE_CONVERSIONS;
    const HAS_STRING_RULE_CONVERSIONS: bool = C::HAS_STRING_RULE_CONVERSIONS;
    const RECONCILE_PARSED_CONTAINMENT: bool = C::RECONCILE_PARSED_CONTAINMENT;
    fn schema(&self) -> &Schema {
        self.language.definition.schema()
    }
    fn grammar(&self) -> Option<&grammar::Grammar> {
        self.language.definition.grammar()
    }
    fn legacy_package(&self) -> Option<&Package> {
        None
    }
    fn convert_boolean_rule(
        &self,
        rule: &str,
        spelling: &str,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<Value> {
        let result = (|| {
            // Charge before invoking even the borrowed binding accessor.
            budget.charge()?;
            let bindings = self.language.capability.boolean_rule_conversions();
            rule_conversion::bound_boolean(
                self.language.definition.grammar(),
                bindings,
                rule,
                spelling,
                budget,
            )
        })();
        result.map_err(|message| self.reject(message, location, None))
    }
    fn convert_string_rule(
        &self,
        rule: &str,
        spelling: &str,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        let result = (|| {
            budget.charge()?;
            let grammar = self
                .language
                .definition
                .grammar()
                .ok_or("Missing EString rule grammar")?;
            self.language
                .capability
                .convert_string_rule(grammar, rule, spelling, budget)
        })();
        result.map_err(|message| self.reject(message, location, None))
    }
    fn create(
        &self,
        store: &mut S,
        request: ParserCreation<'_>,
    ) -> (StorageResult<()>, usize, bool) {
        let mut budget = QueryBudget::new(request.remaining);
        // A direct context has no original source lease. It cannot promote a
        // language initializer into source-aware construction authority.
        if C::HAS_NATIVE_INITIALIZERS {
            let selected = self
                .language
                .capability
                .native_initializer(request.class, &mut budget);
            match selected {
                Ok(None) => {}
                Ok(Some(_)) => {
                    return (
                        Err(self.reject(
                            "Native initialized draft construction requires a source-aware project"
                                .into(),
                            request.location,
                            None,
                        )),
                        budget.reads,
                        true,
                    );
                }
                Err(error) => {
                    return (
                        Err(self.reject(error, request.location, None)),
                        budget.reads,
                        true,
                    );
                }
            }
        }
        let result = self
            .language
            .construction
            .create(
                store,
                storage::Construction {
                    id: request.id,
                    resource: request.uri,
                    authored: &[],
                },
                request.class,
                request.start,
                request.start,
                &mut budget,
            )
            .map(|_| ());
        let result = result.map_err(|message| self.reject(message, request.location, None));
        (result, budget.reads, false)
    }
    fn admit(
        &self,
        store: &S,
        id: &str,
        feature: &str,
        append: bool,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        let schema = <Self as ParseContext<S>>::schema(self);
        self.admit_in(schema, store, (id, feature, append), location, budget)
    }
    fn has_delegated_assignments(&self) -> bool {
        self.language.capability.has_delegated_assignments()
    }
    fn delegated_assignment_target(
        &self,
        store: &S,
        assignment: (&str, &str, bool),
        value: &Value,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        assignment_routes::target(
            self.language.definition,
            self.language.capability,
            store,
            assignment.0,
            assignment.1,
            assignment.2,
            value,
            budget,
        )
        .map_err(|message| self.reject(message, location, None))
    }
    fn admit_value(
        &self,
        store: &S,
        assignment: (&str, &str, bool),
        value: &Value,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        let schema = <Self as ParseContext<S>>::schema(self);
        self.admit_value_in(schema, store, assignment, value, location, budget)
    }
}
impl<C: DraftCapability> PreparedDraftContext<'_, '_, '_, C> {
    fn admit_in<S: ModelStore>(
        &self,
        lookup: &dyn FeatureLookup,
        store: &S,
        assignment: (&str, &str, bool),
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        let (id, feature, append) = assignment;
        let (class, member) =
            self.assignment_member_in(lookup, store, id, feature, location, budget)?;
        if !self
            .language
            .capability
            .permits_assignment(&class, &member, append)
        {
            return Err(self.reject(
                format!("Unsupported draft assignment: {class}.{feature}"),
                location,
                Some(member),
            ));
        }
        Ok(())
    }
    fn admit_value_in<S: ModelStore>(
        &self,
        lookup: &dyn FeatureLookup,
        store: &S,
        assignment: (&str, &str, bool),
        value: &Value,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        let (id, feature, append) = assignment;
        let (class, member) =
            self.assignment_member_in(lookup, store, id, feature, location, budget)?;
        self.language
            .capability
            .validate_assignment_value(&class, &member, append, value, budget)
            .map_err(|message| self.reject(message, location, Some(member)))
    }
}

#[path = "draft_runtime_text.rs"]
mod runtime_text;
