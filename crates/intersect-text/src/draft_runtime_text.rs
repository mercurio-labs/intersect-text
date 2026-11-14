//! The frozen-path text parse (handwritten): construction-only parsing of one
//! source into any `ModelStore` with a bound `DraftLanguage`, for a later
//! linker such as the frozen engine's Rcore scopes. It uses the draft
//! language's binding and parse context, not the draft project pipeline.
use super::*;

/// A direct draft context that constructs outside the project pipeline, into
/// any `ModelStore`. Every capability decision is made by the wrapped
/// `PreparedDraftContext`. With `LIVE`, the runtime-guarded live caller hooks
/// are added for a grammar compiled for the live runtime capability; without
/// it, the parser uses the grammar's ordinary prediction.
pub(super) struct RuntimeDraftContext<'a, 'l, 'd, 'i, C: DraftCapability, const LIVE: bool = true> {
    pub(super) inner: PreparedDraftContext<'l, 'd, 'i, C>,
    /// Present exactly when `LIVE`.
    pub(super) owner: Option<&'a live_callers::Owner<'a, true>>,
    pub(super) admission: Option<&'a grammar::RuntimeGuardedAdmission>,
    pub(super) consumed: &'a std::cell::Cell<usize>,
    /// Create objects with the store's plain constructor instead of the draft
    /// physical contract (construction-only parsing for a later linker).
    pub(super) plain: bool,
    pub(super) metadata:
        Option<&'a intersect_model::definitions::preparation::PreparedPhysicalMetadata<'a>>,
    /// Effective features per class, computed once for this parse.
    pub(super) features: intersect_model::definitions::schema::FeatureIndex<'d>,
    /// Computed-setting providers for the grammar's computed assignments.
    pub(super) settings: Option<&'a setting::CheckedDraftTextSettings<'a>>,
}
impl<C: DraftCapability, const LIVE: bool> RuntimeDraftContext<'_, '_, '_, '_, C, LIVE> {
    /// A computed assignment the settings select, checked against the
    /// capability; false when the settings do not select it.
    fn computed_admission<S: ModelStore>(
        &self,
        store: &S,
        assignment: (&str, &str, bool),
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<bool> {
        let Some(settings) = self.settings else {
            return Ok(false);
        };
        let (id, feature, append) = assignment;
        let (class, member) = self.inner.assignment_member_in(
            &self.features,
            store,
            id,
            feature,
            location,
            budget,
        )?;
        budget.charge()?;
        if !settings.admits(&class, feature, &member, append)? {
            return Ok(false);
        }
        if !self
            .inner
            .language
            .capability
            .permits_computed_assignment_observed(&class, &member, append, budget)?
        {
            return Err(self.inner.reject(
                "Unsupported draft computed assignment capability".into(),
                location,
                Some(member),
            ));
        }
        Ok(true)
    }
}
impl<S: ModelStore, C: DraftCapability, const LIVE: bool> ParserComputedSettings<S>
    for RuntimeDraftContext<'_, '_, '_, '_, C, LIVE>
{
    fn supports(&self, class: &str, feature: &str) -> bool {
        self.settings
            .is_some_and(|settings| settings.supports(class, feature))
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
        let settings = self.settings.ok_or("Missing checked draft text settings")?;
        if !settings.belongs_to(self.inner.language.definition) {
            return Err("Draft text settings belong to another prepared definition".into());
        }
        match self.metadata {
            // This parse's prepared metadata validated the definition's schema
            // once; its feature index serves the same schema borrow.
            Some(metadata) => settings.apply_plain_prepared(
                store,
                id,
                target,
                edit,
                budget,
                conversions,
                metadata,
                &self.features,
            ),
            None => settings.apply_plain(store, id, target, edit, budget, conversions),
        }
    }
}
impl<S: ModelStore, C: DraftCapability, const LIVE: bool> ParseContext<S>
    for RuntimeDraftContext<'_, '_, '_, '_, C, LIVE>
{
    fn feature_index(&self) -> Option<&dyn intersect_model::definitions::schema::FeatureLookup> {
        Some(&self.features)
    }
    fn convert_terminal(&self, terminal: &str, text: &str) -> StorageResult<Option<String>> {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::convert_terminal(
            &self.inner,
            terminal,
            text,
        )
    }
    const CONTEXT_SETTINGS: bool = true;
    fn computed_settings(&self) -> Option<&dyn ParserComputedSettings<S>> {
        self.settings
            .map(|_| self as &dyn ParserComputedSettings<S>)
    }
    fn parse_tables(&self) -> Option<&crate::ParseTables> {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::parse_tables(&self.inner)
    }
    fn physical_metadata(
        &self,
    ) -> Option<&intersect_model::definitions::preparation::PreparedPhysicalMetadata<'_>> {
        self.metadata
    }
    const AUDIT_PREDICTION: bool = C::AUDIT_PREDICTION;
    const AUDIT_RUNTIME_WORK: bool = C::AUDIT_RUNTIME_WORK;
    const NEEDS_ADMISSION: bool = true;
    const NEEDS_VALUE_ADMISSION: bool = C::NEEDS_VALUE_ADMISSION;
    // Without language conversions, plain construction applies Ecore's EBoolean
    // default; draft construction still rejects (see `convert_boolean_rule`).
    const HAS_BOOLEAN_RULE_CONVERSIONS: bool = true;
    const HAS_STRING_RULE_CONVERSIONS: bool = C::HAS_STRING_RULE_CONVERSIONS;
    const RECONCILE_PARSED_CONTAINMENT: bool = C::RECONCILE_PARSED_CONTAINMENT;
    const LIVE_CALLERS: bool = LIVE;
    fn audit_prediction(&self, event: PredictionAudit<'_>) {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::audit_prediction(
            &self.inner,
            event,
        )
    }
    fn audit_runtime_work(&self, event: RuntimePredictionWorkAudit<'_>) {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::audit_runtime_work(
            &self.inner,
            event,
        )
    }
    fn audit_machine_work(&self, event: MachineWorkAudit<'_>) {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::audit_machine_work(
            &self.inner,
            event,
        )
    }
    fn live_entry(
        &self,
        uri: &str,
        entry: &str,
        begin: bool,
        success: bool,
        work: &mut Option<usize>,
    ) -> StorageResult<()> {
        self.owner
            .ok_or("Missing live caller owner")?
            .entry_runtime(uri, entry, begin, success, work, self.admission)
    }
    fn live_call(
        &self,
        uri: &str,
        expression: &grammar::Expr,
        enter: bool,
        work: &mut Option<usize>,
    ) -> StorageResult<()> {
        self.owner
            .ok_or("Missing live caller owner")?
            .call(uri, expression, enter, work)
    }
    fn live_prediction(
        &self,
        request: prediction_sources::Request<'_>,
        input: &mut impl grammar::PredictionInput,
        work: &mut Option<usize>,
    ) -> StorageResult<Option<usize>> {
        crate::bounded_runtime::predict_consuming(
            self.owner.ok_or("Missing live caller owner")?,
            self.admission.ok_or("Missing runtime admission")?,
            self.consumed,
            request,
            input,
            work,
        )
    }
    fn schema(&self) -> &Schema {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::schema(&self.inner)
    }
    fn grammar(&self) -> Option<&grammar::Grammar> {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::grammar(&self.inner)
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
        // In plain construction, a rule the language does not convert falls
        // back like a language with no conversions at all.
        let unconverted = !C::HAS_BOOLEAN_RULE_CONVERSIONS
            || (self.plain
                && !self
                    .inner
                    .language
                    .capability
                    .boolean_rule_conversions()
                    .iter()
                    .any(|binding| binding.rule == rule));
        if unconverted {
            budget.charge()?;
            // Ecore EBoolean createFromString: "true"/"false", case-insensitive.
            return match (self.plain, spelling.to_ascii_lowercase().as_str()) {
                (true, "true") => Ok(Value::Bool(true)),
                (true, "false") => Ok(Value::Bool(false)),
                (true, _) => Err(self.inner.reject(
                    format!("Invalid EBoolean literal {spelling:?} for rule {rule}"),
                    location,
                    None,
                )),
                (false, _) => Err("Unsupported draft Boolean rule conversion".into()),
            };
        }
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::convert_boolean_rule(
            &self.inner,
            rule,
            spelling,
            location,
            budget,
        )
    }
    fn convert_string_rule(
        &self,
        rule: &str,
        spelling: &str,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::convert_string_rule(
            &self.inner,
            rule,
            spelling,
            location,
            budget,
        )
    }
    fn create(
        &self,
        store: &mut S,
        request: ParserCreation<'_>,
    ) -> (StorageResult<()>, usize, bool) {
        if !self.plain {
            return self.inner.create(store, request);
        }
        let schema = <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::schema(&self.inner);
        (
            store.create_element(
                schema,
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
    fn admit(
        &self,
        store: &S,
        id: &str,
        feature: &str,
        append: bool,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        if self.computed_admission(store, (id, feature, append), location, budget)? {
            return Ok(());
        }
        self.inner.admit_in(
            &self.features,
            store,
            (id, feature, append),
            location,
            budget,
        )
    }
    fn has_delegated_assignments(&self) -> bool {
        <PreparedDraftContext<'_, '_, '_, C> as ParseContext<S>>::has_delegated_assignments(
            &self.inner,
        )
    }
    fn delegated_assignment_target(
        &self,
        store: &S,
        assignment: (&str, &str, bool),
        value: &Value,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        self.inner
            .delegated_assignment_target(store, assignment, value, location, budget)
    }
    fn admit_value(
        &self,
        store: &S,
        assignment: (&str, &str, bool),
        value: &Value,
        location: ParseLocation<'_>,
        budget: &mut QueryBudget,
    ) -> StorageResult<()> {
        if self.computed_admission(store, assignment, location, budget)? {
            return Ok(());
        }
        self.inner
            .admit_value_in(&self.features, store, assignment, value, location, budget)
    }
}
impl<C: DraftCapability> DraftLanguage<'_, '_, C> {
    /// Parse and construct one source into the caller's store with this draft
    /// language's capability and the definition's runtime-guarded live callers.
    /// Construction only: no linker, opposite reconciliation or validation runs.
    /// Each cross reference stays in the store's unresolved-reference state
    /// (`?spelling` in JSON-backed stores) for a later linker, such as the
    /// frozen engine's Rcore scopes. Objects are created plainly, with no
    /// draft physical contract check. Returns the parse diagnostics, the number
    /// of unresolved references and the runtime decisions consumed.
    pub fn parse_runtime_into<S: ModelStore>(
        &self,
        source: &Source,
        limits: EvaluationLimits,
        store: &mut S,
        caller: &mut QueryBudget,
    ) -> Result<(StoredEvaluation, usize, usize), String> {
        self.parse_runtime_into_with_settings(source, limits, None, store, caller)
    }
    /// `parse_runtime_into` with the providers of the grammar's computed
    /// assignments (for KerML, the Pilot `operand` parsing extension). The
    /// providers construct what they create with the store's plain constructor.
    pub fn parse_runtime_into_with_settings<S: ModelStore>(
        &self,
        source: &Source,
        limits: EvaluationLimits,
        settings: Option<&setting::CheckedDraftTextSettings<'_>>,
        store: &mut S,
        caller: &mut QueryBudget,
    ) -> Result<(StoredEvaluation, usize, usize), String> {
        if let Some(settings) = settings
            && !settings.belongs_to(self.definition)
        {
            return Err("Draft text settings belong to another prepared definition".into());
        }
        let grammar = self
            .definition
            .grammar()
            .ok_or("Definition has no grammar")?;
        let id = SourceId::for_text(&source.uri, &source.text);
        let ids = [id.clone()];
        let failed = std::cell::Cell::new(false);
        let consumed = std::cell::Cell::new(0);
        // One schema validation for this parse instead of one per checked write.
        let metadata = self
            .definition
            .prepare_physical_metadata(&Default::default())
            .map_err(|e| e.to_string())?;
        let features =
            intersect_model::definitions::schema::FeatureIndex::new(self.definition.schema());
        match self.definition.runtime_admission() {
            // A grammar compiled for the guarded runtime capability.
            Some(admission) => {
                let owner = live_callers::bounded_owner(grammar, &ids, &failed)
                    .with_memo(&self.prediction_memo);
                let context = RuntimeDraftContext::<'_, '_, '_, '_, C, true> {
                    inner: PreparedDraftContext::new(self, id),
                    owner: Some(&owner),
                    admission: Some(admission),
                    consumed: &consumed,
                    plain: true,
                    metadata: Some(&metadata),
                    features,
                    settings,
                };
                self.parse_plain(context, source, limits, store, caller)
            }
            // Any other grammar uses its ordinary prediction; the parser
            // refuses a program that needs the runtime capability.
            None => {
                let context = RuntimeDraftContext::<'_, '_, '_, '_, C, false> {
                    inner: PreparedDraftContext::new(self, id),
                    owner: None,
                    admission: None,
                    consumed: &consumed,
                    plain: true,
                    metadata: Some(&metadata),
                    features,
                    settings,
                };
                self.parse_plain(context, source, limits, store, caller)
            }
        }
    }
    fn parse_plain<S: ModelStore, const LIVE: bool>(
        &self,
        context: RuntimeDraftContext<'_, '_, '_, '_, C, LIVE>,
        source: &Source,
        limits: EvaluationLimits,
        store: &mut S,
        caller: &mut QueryBudget,
    ) -> Result<(StoredEvaluation, usize, usize), String> {
        let grammar = self
            .definition
            .grammar()
            .ok_or("Definition has no grammar")?;
        let sources = std::slice::from_ref(source);
        let mut output = StoredEvaluation::default();
        let result = storage::atomic(store, |store| {
            storage::bind_schema(store, self.definition.schema())?;
            store.configure_schema(self.definition.schema())?;
            let mut invocation = Invocation::resource(limits);
            let (pending, _) = crate::parse_resources_accounted(
                &context,
                sources,
                store,
                limits,
                &mut invocation,
                None,
                None,
                None,
                &grammar.entry,
                &mut output,
                None,
                &mut crate::resource_accounting::SharedResourceAccounting::new(caller),
            )?;
            Ok(pending.iter().filter(|reference| reference.active).count())
        });
        match result {
            Ok(proxies) => Ok((output, proxies, context.consumed.get())),
            Err(error) => Err(match context.inner.rejection.into_inner() {
                Some(rejection) => format!("{error} ({})", rejection.message),
                None => error,
            }),
        }
    }
}
