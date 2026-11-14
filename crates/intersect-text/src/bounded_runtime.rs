//! Explicit native runtime-obligation consumer over the SAME Machine/pipeline.
//! No model executor, cached input answer, Java or new prediction allowance.
use super::*;
use crate::draft::ParseContext;
use intersect_model::core::query::QueryBudget;
use intersect_model::definitions::bundle::SourceId;
use std::cell::Cell;

/// Receipt exists only after actual native pipeline execution. It is not an
/// admission certificate for all inputs or evidence of prepared initial rows.
pub struct RuntimeGuardedEvaluation {
    pub evaluation: Evaluation,
    consumed_runtime_decisions: usize,
}
impl RuntimeGuardedEvaluation {
    pub fn consumed_runtime_decisions(&self) -> usize {
        self.consumed_runtime_decisions
    }
}
struct Context<'a, 'g> {
    package: crate::package::TextPackageView<'g>,
    // The exact grammar the runtime admission was sealed against.
    grammar: &'g grammar::Grammar,
    owner: &'a crate::draft::live_callers::Owner<'g, true>,
    admission: &'g grammar::RuntimeGuardedAdmission,
    consumed: &'a Cell<usize>,
}
impl<S: ModelStore> ParseContext<S> for Context<'_, '_> {
    const LIVE_CALLERS: bool = true;
    fn live_entry(
        &self,
        uri: &str,
        entry: &str,
        begin: bool,
        success: bool,
        work: &mut Option<usize>,
    ) -> Result<(), String> {
        self.owner
            .entry_runtime(uri, entry, begin, success, work, Some(self.admission))
    }
    fn live_call(
        &self,
        uri: &str,
        expression: &Expr,
        enter: bool,
        work: &mut Option<usize>,
    ) -> Result<(), String> {
        self.owner.call(uri, expression, enter, work)
    }
    fn live_prediction(
        &self,
        request: crate::draft::prediction_sources::Request<'_>,
        input: &mut impl grammar::PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        predict_consuming(
            self.owner,
            self.admission,
            self.consumed,
            request,
            input,
            work,
        )
    }
    fn schema(&self) -> &intersect_model::definitions::schema::Schema {
        self.package.schema
    }
    fn grammar(&self) -> Option<&grammar::Grammar> {
        Some(self.grammar)
    }
    fn legacy_package(&self) -> Option<&Package> {
        None
    }
    fn execution_definition(&self) -> Option<crate::package::TextPackageView<'_>> {
        Some(self.package)
    }
    fn create(
        &self,
        store: &mut S,
        request: crate::draft::ParserCreation<'_>,
    ) -> (Result<(), String>, usize, bool) {
        crate::draft::create_for_definition(self.package.model_definition(), store, request)
    }
}

/// The caller chooses this route explicitly and retains its original finite
/// ledger. Default evaluation rejects the typed runtime requirement in the IR.
pub fn evaluate(
    compiled: &compiler::RuntimeGuardedCompilation,
    sources: &[Source],
    limits: EvaluationLimits,
    caller: &mut QueryBudget,
) -> Result<RuntimeGuardedEvaluation, String> {
    let (package, admission) = compiled.parts();
    let grammar = package.grammar.ok_or("Missing runtime-bound grammar")?;
    let program = grammar
        .parser_prediction
        .as_ref()
        .ok_or("Missing actual runtime IR")?;
    if admission.decision_count() == 0 || !program.requires_runtime_capability() {
        return Err("Native executable runtime obligations are required".into());
    }
    if limits.resources.is_some_and(|limit| sources.len() > limit)
        || limits.input_bytes.is_some_and(|limit| {
            sources
                .iter()
                .try_fold(0usize, |n, s| n.checked_add(s.text.len()))
                .is_none_or(|n| n > limit)
        })
    {
        return Err("Model resource budget exceeded".into());
    }
    // New bounded host/context/report holder. Core live holder has its existing
    // independently checked128-byte admission; old Owner retains256-byte fee.
    for _ in 0..3 {
        caller.charge()?;
    }
    if sources.is_empty() {
        return Err("Bounded runtime needs an actual input source".into());
    }
    let mut identity_bytes = 32usize;
    for source in sources {
        caller.charge()?; // actual closed-input metadata inspection
        identity_bytes = identity_bytes
            .checked_add(128)
            .and_then(|n| n.checked_add(source.uri.len()))
            .filter(|n| *n <= 1_048_576)
            .ok_or("Runtime source identity retention bound exceeded")?;
    }
    caller.charge()?;
    for _ in 0..identity_bytes.div_ceil(64) {
        caller.charge()?;
    }
    let mut source_ids = Vec::new();
    source_ids
        .try_reserve_exact(sources.len())
        .map_err(|_| "Runtime input identity allocation denied")?;
    // One exact capacity; no repeated growth/copy of retained identity records.
    for source in sources {
        caller.charge()?; // actual descriptor placement
        for _ in 0..source.text.len().div_ceil(64) {
            caller.charge()?;
        }
        source_ids.push(SourceId::for_text(&source.uri, &source.text));
    }
    let failed = Cell::new(false);
    let consumed = Cell::new(0);
    let owner = crate::draft::live_callers::bounded_owner(grammar, &source_ids, &failed);
    let context = Context {
        package,
        grammar,
        owner: &owner,
        admission,
        consumed: &consumed,
    };
    let mut store = new_evaluation_store_definition(package)?;
    let evaluation = storage::atomic(&mut store, |store| {
        evaluate_staged_with_parse_context(
            package,
            sources,
            Invocation::resource(limits),
            store,
            None,
            None,
            &query::ModelDerived(package.model_definition()),
            None,
            &context,
            &mut resource_accounting::SharedResourceAccounting::new(caller),
        )
    })?;
    Ok(RuntimeGuardedEvaluation {
        evaluation: Evaluation {
            model: store.into_model()?,
            diagnostics: evaluation.diagnostics,
            references: evaluation.references,
            derived: evaluation.derived,
        },
        consumed_runtime_decisions: consumed.get(),
    })
}

/// Runtime-guarded evaluation into the caller's store (for example a
/// frozen-engine `FrozenStore`), with optional computed settings and the
/// caller's linking context. Returns the evaluation and the number of runtime
/// decisions consumed. The caller's ledger is charged per input source.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_into<'a>(
    package: impl Into<crate::package::TextPackageView<'a>>,
    grammar: &grammar::Grammar,
    admission: &grammar::RuntimeGuardedAdmission,
    sources: &[Source],
    limits: EvaluationLimits,
    store: &mut impl ModelStore,
    settings: Option<&setting::TextSettings<'_>>,
    linking: &mut crate::linking::LinkingContext<'_>,
    caller: &mut QueryBudget,
) -> Result<(StoredEvaluation, usize), String> {
    let package = package.into();
    if sources.is_empty() {
        return Err("Bounded runtime needs an actual input source".into());
    }
    let mut source_ids = Vec::new();
    source_ids
        .try_reserve_exact(sources.len())
        .map_err(|_| "Runtime input identity allocation denied")?;
    for source in sources {
        caller.charge()?;
        source_ids.push(SourceId::for_text(&source.uri, &source.text));
    }
    let failed = Cell::new(false);
    let consumed = Cell::new(0);
    let owner = crate::draft::live_callers::bounded_owner(grammar, &source_ids, &failed);
    let context = Context {
        package,
        grammar,
        owner: &owner,
        admission,
        consumed: &consumed,
    };
    let evaluation = storage::atomic(store, |store| {
        evaluate_staged_with_parse_context(
            package,
            sources,
            Invocation::resource(limits),
            store,
            None,
            settings,
            &query::ModelDerived(package.model_definition()),
            Some(linking),
            &context,
            &mut resource_accounting::SharedResourceAccounting::new(caller),
        )
    })?;
    Ok((evaluation, consumed.get()))
}

// Shared counting adapter only. Both callers enter the SAME live owner/worker;
// neither the draft bridge nor this helper grants Package semantic authority.
pub(crate) fn predict_consuming<const SHARED: bool>(
    owner: &crate::draft::live_callers::Owner<'_, SHARED>,
    admission: &grammar::RuntimeGuardedAdmission,
    consumed: &Cell<usize>,
    request: crate::draft::prediction_sources::Request<'_>,
    input: &mut impl grammar::PredictionInput,
    work: &mut Option<usize>,
) -> Result<Option<usize>, String> {
    predict_consuming_with_arena::<SHARED, false>(owner, admission, consumed, request, input, work)
}
pub(crate) fn predict_consuming_with_arena<const SHARED: bool, const ARENA: bool>(
    owner: &crate::draft::live_callers::Owner<'_, SHARED>,
    admission: &grammar::RuntimeGuardedAdmission,
    consumed: &Cell<usize>,
    request: crate::draft::prediction_sources::Request<'_>,
    input: &mut impl grammar::PredictionInput,
    work: &mut Option<usize>,
) -> Result<Option<usize>, String> {
    predict_consuming_with_initial::<SHARED, ARENA>(
        owner, admission, consumed, request, input, work, None,
    )
}
pub(crate) fn predict_consuming_with_initial<'g, const SHARED: bool, const ARENA: bool>(
    owner: &crate::draft::live_callers::Owner<'g, SHARED>,
    admission: &grammar::RuntimeGuardedAdmission,
    consumed: &Cell<usize>,
    request: crate::draft::prediction_sources::Request<'_>,
    input: &mut impl grammar::PredictionInput,
    work: &mut Option<usize>,
    initial: Option<&grammar::PreparedLiveRuntimePredictionContext<'g>>,
) -> Result<Option<usize>, String> {
    let left = work
        .as_mut()
        .ok_or("Bounded runtime needs the original finite pool")?;
    let bound = admission.decision_is_runtime_with_remaining(request.decision, left)?;
    let next = if bound {
        let left = work
            .as_mut()
            .ok_or("Bounded runtime needs the original finite pool")?;
        if *left == 0 {
            return Err("Execution step budget exceeded before runtime obligation".into());
        }
        *left -= 1; // actual consumption receipt reservation, never refunded
        Some(
            consumed
                .get()
                .checked_add(1)
                .ok_or("Runtime consumption count overflow")?,
        )
    } else {
        None
    };
    let result = if let Some(initial) = initial {
        owner.predict_runtime_with_initial::<ARENA>(request, input, work, true, Some(initial))
    } else if ARENA {
        owner.predict_runtime_with_arena::<true>(request, input, work, true)
    } else {
        owner.predict_runtime(request, input, work, true)
    };
    if result.is_ok()
        && let Some(next) = next
    {
        consumed.set(next);
    }
    result
}
