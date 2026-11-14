//! Explicit caller-neutral graph facts from the SAME canonical closure worker.
//! No input answer, source/caller authority or semantic result is retained.
use super::*;
use intersect_model::definitions::conversion::InvocationContext;

const LIVE_HEADER: usize = 16;
const FACT_LIMIT: usize = 1_048_576;
const WORK_LIMIT: usize = 100_000;

fn observed_charge(
    work: &mut Option<usize>,
    original: &InvocationContext<'_>,
) -> Result<(), String> {
    if work.is_some_and(|left| left == 0) {
        return charge(work);
    }
    original.work(1)?;
    charge(work)
}
fn observed_admit(
    work: &mut Option<usize>,
    logical: &mut usize,
    limit: usize,
    bytes: usize,
    original: &InvocationContext<'_>,
) -> Result<(), String> {
    observed_charge(work, original)?;
    let next = logical
        .checked_add(bytes)
        .filter(|total| *total <= limit)
        .ok_or("Prepared caller-neutral storage budget exceeded")?;
    observed_charge(work, original)?;
    for _ in 0..bytes.div_ceil(LOGICAL_WORK_QUANTUM) {
        observed_charge(work, original)?;
    }
    original.retain_storage(bytes)?;
    original.checkpoint()?;
    *logical = next;
    Ok(())
}

/// Opaque caller-neutral first closures. Only the observed canonical preparer
/// below can mint this type. Historical prepared rows cannot be substituted.
/// The global prefix peak is conservative for every retained decision/branch.
pub struct PreparedLiveRuntimePredictionContext<'program> {
    inner: PreparedRuntimePredictionContext<'program>,
    prefix_depth: usize,
}
impl<'program> PreparedLiveRuntimePredictionContext<'program> {
    /// Complete host admission under the SAME original observer. This is only
    /// a failure boundary: successful parameters still undergo every canonical
    /// neutral preparation check below, so no host result supplies authority.
    pub fn prepare_selected_after_observed_admission<'selection>(
        admission: Result<
            (
                &'program Grammar,
                &'program [String],
                CompletionVerification,
                usize,
            ),
            String,
        >,
        paths: impl IntoIterator<Item = &'selection str>,
        work: &mut Option<usize>,
        original: &InvocationContext<'_>,
    ) -> Result<Self, String> {
        match admission {
            Ok((grammar, hidden, verification, limit)) => Self::prepare_selected_observed(
                grammar,
                hidden,
                verification,
                paths,
                work,
                limit,
                original,
            ),
            Err(error) => {
                original.failed_result(&error);
                Err(error)
            }
        }
    }

    /// This spends the supplied SAME finite clamp and original observer before
    /// each unit, and the supplied fact limit before each retained allocation.
    /// It does not reset an observer or create another work/storage allowance.
    #[allow(clippy::too_many_arguments)] // Existing selected preparation plus original observer.
    pub fn prepare_selected_observed<'selection>(
        grammar: &'program Grammar,
        hidden: &'program [String],
        verification: CompletionVerification,
        paths: impl IntoIterator<Item = &'selection str>,
        work: &mut Option<usize>,
        storage_limit: usize,
        original: &InvocationContext<'_>,
    ) -> Result<Self, String> {
        const {
            assert!(
                std::mem::size_of::<PreparedLiveRuntimePredictionContext<'static>>()
                    <= SOURCE_HEADER + LIVE_HEADER
            );
        }
        let result: Result<Self, String> = (|| {
            if !original.has_observer() {
                return Err("Caller-neutral preparation requires original observer".into());
            }
            original.checkpoint()?;
            if work.is_none_or(|left| left > WORK_LIMIT) {
                return Err(
                    "Caller-neutral preparation requires the same finite at-most100000 clamp"
                        .into(),
                );
            }
            observed_charge(work, original)?;
            let program = grammar
                .parser_prediction
                .as_ref()
                .ok_or("Missing actual grammar prediction program")?;
            let global = grammar.hidden.as_deref().unwrap_or(&[]);
            let mut canonical =
                std::ptr::eq(global, hidden) || (global.is_empty() && hidden.is_empty());
            if !canonical {
                for rule in grammar.rules.values() {
                    observed_charge(work, original)?;
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
                return Err(
                    "Runtime preparation requires the actual canonical hidden policy".into(),
                );
            }
            let limit = storage_limit.min(FACT_LIMIT);
            let mut inner = PreparedRuntimePredictionContext {
                grammar,
                program,
                hidden,
                verification,
                selected: BTreeSet::new(),
                rows: BTreeMap::new(),
                logical_bytes: 0,
                ineligible: 0,
            };
            observed_admit(
                work,
                &mut inner.logical_bytes,
                limit,
                SOURCE_HEADER,
                original,
            )?;
            observed_admit(work, &mut inner.logical_bytes, limit, LIVE_HEADER, original)?;
            for requested in paths {
                observed_charge(work, original)?;
                let (path, _) = program
                    .decisions
                    .get_key_value(requested)
                    .ok_or("Unknown prepared runtime decision")?;
                observed_charge(work, original)?;
                if inner.selected.contains(path.as_str()) {
                    return Err("Duplicate prepared runtime decision selection".into());
                }
                observed_admit(
                    work,
                    &mut inner.logical_bytes,
                    limit,
                    REPEATED_KEY_SLOT,
                    original,
                )?;
                inner.selected.insert(path.as_str());
            }
            if inner.selected.is_empty() {
                return Err("Prepared runtime decision selection is empty".into());
            }
            let mut prefix_depth = 0;
            for &path in &inner.selected {
                observed_charge(work, original)?;
                let decision = program
                    .decisions
                    .get(path)
                    .ok_or("Selected canonical runtime decision disappeared")?;
                let recursive_first = program.recursive_decisions.contains(path);
                let mut access = NeutralPreparationAccess {
                    row: None,
                    logical: &mut inner.logical_bytes,
                    limit,
                    peak: &mut prefix_depth,
                    original,
                };
                let outcome = program.run_prediction_at_depth_with_local_decision_and_source(
                    PredictionGoal::Decision {
                        decision,
                        recursive_first,
                        boundary: None,
                    },
                    &mut PreparationInput,
                    work,
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
                            .ok_or("Neutral preparation completed without its first closure")?;
                        observed_charge(work, original)?;
                        if inner
                            .rows
                            .insert((path, verification_key(verification), recursive_first), row)
                            .is_some()
                        {
                            return Err("Duplicate prepared neutral frontier".into());
                        }
                    }
                    PredictionOutcome::InitialPrepared(InitialPreparation::Ineligible) => {
                        if access.row.is_some() {
                            return Err("Ineligible neutral preparation retained a frontier".into());
                        }
                        inner.ineligible = inner
                            .ineligible
                            .checked_add(1)
                            .ok_or("Ineligible preparation counter overflow")?;
                    }
                    _ => return Err("Neutral preparation returned an input result".into()),
                }
            }
            let result = Self {
                inner,
                prefix_depth,
            };
            result.check_observed(grammar, program, hidden, verification, work, Some(original))?;
            original.checkpoint()?;
            Ok(result)
        })();
        if let Err(error) = &result {
            original.failed_result(error);
        }
        result
    }
    pub fn selected_decisions(&self) -> usize {
        self.inner.selected_decisions()
    }
    pub fn prepared_decisions(&self) -> usize {
        self.inner.prepared_decisions()
    }
    pub fn ineligible_decisions(&self) -> usize {
        self.inner.ineligible_decisions()
    }
    pub fn logical_bytes(&self) -> usize {
        self.inner.logical_bytes()
    }
    pub fn prefix_depth(&self) -> usize {
        self.prefix_depth
    }
    fn check_observed(
        &self,
        grammar: &Grammar,
        program: &ParserPrediction,
        hidden: &[String],
        verification: CompletionVerification,
        work: &mut Option<usize>,
        original: Option<&InvocationContext<'_>>,
    ) -> Result<(), String> {
        if let Some(original) = original {
            observed_charge(work, original)?;
        } else {
            charge(work)?;
        }
        if !std::ptr::eq(self.inner.grammar, grammar)
            || !std::ptr::eq(self.inner.program, program)
            || !grammar
                .parser_prediction
                .as_ref()
                .is_some_and(|actual| std::ptr::eq(actual, program))
            || self.inner.verification != verification
        {
            return Err(
                "Prepared neutral context differs from exact live grammar/program/verification"
                    .into(),
            );
        }
        if let Some(original) = original {
            observed_charge(work, original)?;
        } else {
            charge(work)?;
        }
        if !(std::ptr::eq(self.inner.hidden, hidden)
            || (self.inner.hidden.is_empty() && hidden.is_empty()))
        {
            return Err("Prepared neutral context differs from canonical hidden policy".into());
        }
        Ok(())
    }
}

struct NeutralPreparationAccess<'call, 'program, 'observer> {
    row: Option<InitialClosure<'program>>,
    logical: &'call mut usize,
    limit: usize,
    peak: &'call mut usize,
    original: &'call InvocationContext<'observer>,
}
impl<'program> InitialSource<'program> for NeutralPreparationAccess<'_, 'program, '_> {
    const ENABLED: bool = true;
    const CALLER_NEUTRAL: bool = true;
    const PREPARE_ONLY: bool = true;
    fn charge(&mut self, remaining: &mut Option<usize>) -> Result<(), String> {
        observed_charge(remaining, self.original)
    }
    fn prefix_depth(&mut self, depth: usize, work: &mut Option<usize>) -> Result<(), String> {
        observed_charge(work, self.original)?;
        *self.peak = (*self.peak).max(depth);
        Ok(())
    }
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
        work: &mut Option<usize>,
    ) -> Result<(), String> {
        if self.row.is_some() {
            return Err("Neutral preparation published twice".into());
        }
        if !accepting.is_empty() {
            return Err("Caller-neutral closure has an accepting alternative".into());
        }
        let mut copied = Vec::new();
        let slots = frontier
            .len()
            .checked_mul(FRONTIER_SLOT)
            .ok_or("Neutral frontier admission overflow")?;
        observed_admit(work, self.logical, self.limit, slots, self.original)?;
        copied
            .try_reserve_exact(frontier.len())
            .map_err(|_| "Neutral frontier allocation denied")?;
        for (symbol, target) in frontier {
            observed_admit(
                work,
                self.logical,
                self.limit,
                return_copy_bytes(target)?,
                self.original,
            )?;
            copied.push((*symbol, target.clone()));
        }
        observed_admit(work, self.logical, self.limit, CACHE_SLOT, self.original)?;
        self.row = Some(InitialClosure {
            frontier: copied,
            accepting: BTreeMap::new(),
        });
        Ok(())
    }
}

pub(in crate::grammar::parser_prediction) struct LivePreparedAccess<'call, 'program> {
    prepared: &'call PreparedLiveRuntimePredictionContext<'program>,
    key: Key<'program>,
    seed_len: usize,
}
impl<'call, 'program> LivePreparedAccess<'call, 'program> {
    #[allow(clippy::too_many_arguments)] // Exact existing live binding plus immutable prepared proof.
    pub(in crate::grammar::parser_prediction) fn new(
        prepared: &'call PreparedLiveRuntimePredictionContext<'program>,
        grammar: &'program Grammar,
        program: &'program ParserPrediction,
        hidden: &[String],
        verification: CompletionVerification,
        path: &str,
        seed_len: usize,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        prepared.check_observed(grammar, program, hidden, verification, work, None)?;
        charge(work)?;
        let (path, _) = program
            .decisions
            .get_key_value(path)
            .ok_or("Missing exact prepared live decision")?;
        let key = (
            path.as_str(),
            verification_key(verification),
            program.recursive_decisions.contains(path),
        );
        Ok(Self {
            prepared,
            key,
            seed_len,
        })
    }
}
impl<'program, R: super::super::super::shared_returns::Returns> InitialSource<'program, R>
    for LivePreparedAccess<'_, 'program>
{
    const ENABLED: bool = true;
    const CALLER_NEUTRAL: bool = true;
    const BORROWED_FRONTIER: bool = true;
    fn replay(
        &mut self,
        _: &mut Frontier<'program, R>,
        _: &mut BTreeMap<usize, bool>,
        work: &mut Option<usize>,
    ) -> Result<bool, String> {
        charge(work)?;
        if self
            .seed_len
            .checked_add(self.prepared.prefix_depth)
            .is_none_or(|depth| depth > 128)
        {
            return Ok(false);
        }
        Ok(self.prepared.inner.rows.contains_key(&self.key))
    }
    fn remember(
        &mut self,
        _: &[(&'program PredictionSymbol, PredictionConfiguration<R>)],
        _: &BTreeMap<usize, bool>,
        _: &mut Option<usize>,
    ) -> Result<(), String> {
        Ok(())
    }
    fn match_live_borrowed(
        &mut self,
        symbol: &Option<PredictionSymbol>,
        seed: &super::super::super::live_callers::LiveSeed<'_, '_>,
        cached: &mut Option<R>,
        work: &mut Option<usize>,
    ) -> Result<BTreeSet<PredictionConfiguration<R>>, String> {
        const {
            assert!(std::mem::size_of::<PredictionConfiguration<R>>() <= FRONTIER_SLOT);
        }
        charge(work)?;
        if !R::SHARED || seed.returns.len() != self.seed_len {
            return Err("Prepared live returns differ from actual paid shared seed".into());
        }
        let row = self
            .prepared
            .inner
            .rows
            .get(&self.key)
            .ok_or("Missing complete neutral frontier")?;
        match_frontier(
            row.frontier.iter().map(|(symbol, row)| (*symbol, row)),
            symbol,
            work,
            |target, work| {
                // New working record only. Existing per-node/arena/seed tariffs pay
                // the actual caller prefix and matched internal tail storage.
                charge(work)?;
                for _ in 0..FRONTIER_SLOT.div_ceil(LOGICAL_WORK_QUANTUM) {
                    charge(work)?;
                }
                let mut returns = R::seed(seed.returns, cached, work)?;
                for index in 0..target.returns.len() {
                    charge(work)?;
                    let value = *target
                        .returns
                        .get(index)
                        .ok_or("Dangling neutral return suffix")?;
                    if returns.len() >= crate::grammar::MAX_CALL_DEPTH {
                        return Err("Parser prediction call depth exceeded".into());
                    }
                    returns.push_return(value, work)?;
                }
                Ok(PredictionConfiguration {
                    state: target.state,
                    returns,
                    alternative: target.alternative,
                    visible: target.visible,
                    guarded: target.guarded,
                    full_guard: target.full_guard,
                })
            },
        )
    }
}
