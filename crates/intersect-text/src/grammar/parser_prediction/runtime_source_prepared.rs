//! Separately paid immutable graph facts; no token, syntax result or language authority.
use super::super::super::Grammar;
use super::*;

/// A complete first closure was retained, or this decision must use its old worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialPreparation {
    Prepared,
    Ineligible,
}

/// Bound to the exact live grammar/program and canonical hidden policy. This is
/// not serializable and cannot establish source, model or library validity.
/// The storage limit counts retained logical records, not allocator/RSS peaks.
pub struct PreparedRuntimePredictionContext<'program> {
    grammar: &'program Grammar,
    program: &'program ParserPrediction,
    hidden: &'program [String],
    verification: CompletionVerification,
    selected: BTreeSet<&'program str>,
    rows: BTreeMap<Key<'program>, InitialClosure<'program>>,
    logical_bytes: usize,
    ineligible: usize,
}

impl<'program> PreparedRuntimePredictionContext<'program> {
    /// A separate finite operation, completed before any project invocation.
    /// No partial table escapes an error. Visible predicates are ineligible;
    /// every such invocation still evaluates them in the canonical worker.
    pub fn prepare(
        grammar: &'program Grammar,
        hidden: &'program [String],
        verification: CompletionVerification,
        remaining: &mut Option<usize>,
        storage_limit: usize,
    ) -> Result<Self, String> {
        if remaining.is_none() {
            return Err("Runtime context preparation requires a finite work pool".into());
        }
        charge(remaining)?;
        let program = grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing actual grammar prediction program")?;
        Self::prepare_selected(
            grammar,
            hidden,
            verification,
            program.decisions(),
            remaining,
            storage_limit,
        )
    }

    /// Prepare only explicitly requested canonical decisions. Unknown, duplicate
    /// and empty selections fail before any frontier is retained. Every other
    /// runtime decision uses the existing canonical invocation-local fallback.
    pub fn prepare_selected<'selection>(
        grammar: &'program Grammar,
        hidden: &'program [String],
        verification: CompletionVerification,
        paths: impl IntoIterator<Item = &'selection str>,
        remaining: &mut Option<usize>,
        storage_limit: usize,
    ) -> Result<Self, String> {
        if remaining.is_none() {
            return Err("Runtime context preparation requires a finite work pool".into());
        }
        charge(remaining)?;
        let program = grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing actual grammar prediction program")?;
        let global = grammar.hidden.as_deref().unwrap_or(&[]);
        let mut canonical =
            std::ptr::eq(global, hidden) || (global.is_empty() && hidden.is_empty());
        if !canonical {
            for rule in grammar.rules.values() {
                charge(remaining)?;
                if rule
                    .hidden
                    .as_deref()
                    .is_some_and(|policy| std::ptr::eq(policy, hidden))
                {
                    canonical = true;
                    break;
                }
            }
        }
        if !canonical {
            return Err("Runtime preparation requires the actual canonical hidden policy".into());
        }
        let mut result = Self {
            grammar,
            program,
            hidden,
            verification,
            selected: BTreeSet::new(),
            rows: BTreeMap::new(),
            logical_bytes: 0,
            ineligible: 0,
        };
        admit_initial(
            remaining,
            &mut result.logical_bytes,
            Some(storage_limit),
            SOURCE_HEADER,
        )?;
        // Resolve the complete selection to actual program-owned canonical keys
        // before any graph frontier is constructed or retained.
        for requested in paths {
            charge(remaining)?;
            let (path, _) = program
                .decisions
                .get_key_value(requested)
                .ok_or("Unknown prepared runtime decision")?;
            charge(remaining)?;
            if result.selected.contains(path.as_str()) {
                return Err("Duplicate prepared runtime decision selection".into());
            }
            admit_initial(
                remaining,
                &mut result.logical_bytes,
                Some(storage_limit),
                REPEATED_KEY_SLOT,
            )?;
            result.selected.insert(path.as_str());
        }
        if result.selected.is_empty() {
            return Err("Prepared runtime decision selection is empty".into());
        }
        for &path in &result.selected {
            charge(remaining)?;
            let decision = program
                .decisions
                .get(path)
                .ok_or("Selected canonical runtime decision disappeared")?;
            let recursive_first = program.recursive_decisions.contains(path);
            let mut access = PreparationAccess {
                row: None,
                logical: &mut result.logical_bytes,
                limit: storage_limit,
            };
            let outcome = program.run_prediction_at_depth_with_local_decision_and_source(
                PredictionGoal::Decision {
                    decision,
                    recursive_first,
                    boundary: None,
                },
                &mut PreparationInput,
                remaining,
                0,
                verification,
                None,
                &mut access,
            )?;
            match outcome {
                PredictionOutcome::InitialPrepared(InitialPreparation::Prepared) => {
                    let row = access
                        .row
                        .take()
                        .ok_or("Preparation completed without a full initial frontier")?;
                    charge(remaining)?;
                    if result
                        .rows
                        .insert((path, verification_key(verification), recursive_first), row)
                        .is_some()
                    {
                        return Err("Duplicate prepared initial frontier".into());
                    }
                }
                PredictionOutcome::InitialPrepared(InitialPreparation::Ineligible) => {
                    if access.row.is_some() {
                        return Err("Ineligible preparation retained a frontier".into());
                    }
                    result.ineligible = result
                        .ineligible
                        .checked_add(1)
                        .ok_or("Ineligible preparation counter overflow")?;
                }
                _ => return Err("Initial preparation returned a syntax/completion result".into()),
            }
        }
        result.check(grammar, program, hidden, verification, remaining)?;
        Ok(result)
    }
    pub fn selected_decisions(&self) -> usize {
        self.selected.len()
    }
    pub fn prepared_decisions(&self) -> usize {
        self.rows.len()
    }
    pub fn ineligible_decisions(&self) -> usize {
        self.ineligible
    }
    pub fn logical_bytes(&self) -> usize {
        self.logical_bytes
    }

    pub(super) fn check(
        &self,
        grammar: &Grammar,
        program: &ParserPrediction,
        hidden: &[String],
        verification: CompletionVerification,
        remaining: &mut Option<usize>,
    ) -> Result<(), String> {
        charge(remaining)?;
        if !std::ptr::eq(self.grammar, grammar)
            || !std::ptr::eq(self.program, program)
            || !grammar
                .parser_prediction
                .as_ref()
                .is_some_and(|actual| std::ptr::eq(actual, program))
            || self.verification != verification
        {
            return Err(
                "Prepared runtime context differs from the exact live grammar/program/verification"
                    .into(),
            );
        }
        charge(remaining)?;
        if !(std::ptr::eq(self.hidden, hidden) || (self.hidden.is_empty() && hidden.is_empty())) {
            return Err("Prepared runtime context differs from the canonical hidden policy".into());
        }
        Ok(())
    }

    /// Fresh sources and their poison state remain invocation-local. Immutable
    /// rows replay through the SAME borrowed matcher and original project pool.
    #[allow(clippy::too_many_arguments)]
    pub fn predict<'invocation>(
        &self,
        grammar: &Grammar,
        program: &'invocation ParserPrediction,
        sources: &mut RepeatedRuntimePredictionSource<'invocation>,
        source: &SourceId,
        hidden: &[String],
        path: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String>
    where
        'program: 'invocation,
    {
        let result = program.predict_with_source_admission::<_, true>(
            &mut sources.source,
            source,
            hidden,
            path,
            input,
            remaining,
            verification,
            Some((self, grammar)),
            RepeatedAdmission {
                seen: &mut sources.seen,
                publish: false,
            },
        );
        if result.is_err() {
            sources.seen.clear();
        }
        result
    }
}
fn verification_key(value: CompletionVerification) -> u8 {
    match value {
        CompletionVerification::Legacy => 0,
        CompletionVerification::GuardOverapproximation => 1,
        CompletionVerification::BoundedGuardOverapproximation => 2,
        CompletionVerification::InputCheckedCompletion => 3,
    }
}

// This is deliberately incapable of token/checkpoint access. Reaching any of
// these methods is a preparation error, never an invented EOF/syntax success.
struct PreparationInput;
impl PredictionInput for PreparationInput {
    fn read(&mut self, _: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
        Err("Initial preparation attempted token access".into())
    }
    fn checkpoint(&self) -> Result<usize, String> {
        Err("Initial preparation attempted checkpoint access".into())
    }
    fn rewind(&mut self, _: usize) -> Result<(), String> {
        Err("Initial preparation attempted rewind access".into())
    }
}
struct PreparationAccess<'call, 'program> {
    row: Option<InitialClosure<'program>>,
    logical: &'call mut usize,
    limit: usize,
}
impl<'program> InitialSource<'program> for PreparationAccess<'_, 'program> {
    const ENABLED: bool = true;
    const PREPARE_ONLY: bool = true;
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
        frontier: &[(&'program PredictionSymbol, Configuration)],
        accepting: &BTreeMap<usize, bool>,
        remaining: &mut Option<usize>,
    ) -> Result<(), String> {
        if self.row.is_some() {
            return Err("Initial preparation published twice".into());
        }
        self.row = Some(retain_initial(
            frontier,
            accepting,
            remaining,
            self.logical,
            Some(self.limit),
        )?);
        Ok(())
    }
}

pub(super) struct PreparedAccess<'call, 'borrow, 'source, 'program, A, const BORROWED: bool> {
    prepared: &'call PreparedRuntimePredictionContext<'program>,
    local: &'borrow mut SourceAccess<'source, 'program, A, BORROWED>,
    using_prepared: bool,
}
impl<'call, 'borrow, 'source, 'program, A, const BORROWED: bool>
    PreparedAccess<'call, 'borrow, 'source, 'program, A, BORROWED>
{
    pub(super) fn new(
        prepared: &'call PreparedRuntimePredictionContext<'program>,
        local: &'borrow mut SourceAccess<'source, 'program, A, BORROWED>,
    ) -> Self {
        Self {
            prepared,
            local,
            using_prepared: false,
        }
    }
}
impl<'program, A: SourceAdmission<'program>, const BORROWED: bool> InitialSource<'program>
    for PreparedAccess<'_, '_, '_, 'program, A, BORROWED>
{
    const ENABLED: bool = true;
    const BORROWED_FRONTIER: bool = true;
    fn replay(
        &mut self,
        frontier: &mut Frontier<'program>,
        accepting: &mut BTreeMap<usize, bool>,
        remaining: &mut Option<usize>,
    ) -> Result<bool, String> {
        charge(remaining)?;
        if let Some(row) = self.prepared.rows.get(&self.local.key) {
            for (&alternative, &guarded) in &row.accepting {
                admit(
                    remaining,
                    &mut self.local.sources.logical_bytes,
                    ACCEPTING_SLOT,
                )?;
                accepting.insert(alternative, guarded);
            }
            self.local.sources.hits = self
                .local
                .sources
                .hits
                .checked_add(1)
                .ok_or("Runtime context hit counter overflow")?;
            self.using_prepared = true;
            Ok(true)
        } else {
            self.local.replay(frontier, accepting, remaining)
        }
    }
    fn remember(
        &mut self,
        frontier: &[(&'program PredictionSymbol, Configuration)],
        accepting: &BTreeMap<usize, bool>,
        remaining: &mut Option<usize>,
    ) -> Result<(), String> {
        self.local.remember(frontier, accepting, remaining)
    }
    fn match_borrowed(
        &mut self,
        symbol: &Option<PredictionSymbol>,
        remaining: &mut Option<usize>,
    ) -> Result<BTreeSet<Configuration>, String> {
        if !self.using_prepared {
            return self.local.match_borrowed(symbol, remaining);
        }
        charge(remaining)?;
        let row = self
            .prepared
            .rows
            .get(&self.local.key)
            .ok_or("Missing complete prepared frontier")?;
        let logical = &mut self.local.sources.logical_bytes;
        match_frontier(
            row.frontier.iter().map(|(symbol, row)| (*symbol, row)),
            symbol,
            remaining,
            |target, remaining| {
                let bytes = FRONTIER_SLOT
                    .checked_add(return_copy_bytes(target)?)
                    .ok_or("Prepared matching admission overflow")?;
                admit(remaining, logical, bytes)?;
                Ok(target.clone())
            },
        )
    }
}

#[path = "runtime_live_prepared.rs"]
mod live;
pub(in crate::grammar::parser_prediction) use live::LivePreparedAccess;
pub use live::PreparedLiveRuntimePredictionContext;
