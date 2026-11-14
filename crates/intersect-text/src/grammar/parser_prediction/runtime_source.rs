//! Paid, invocation-local source rows from the SAME runtime epsilon walker.
//! No input symbol, recognition result, model value or error is retained.
use super::{
    CompletionVerification, Configuration, ParserPrediction, PredictionConfiguration,
    PredictionGoal, PredictionInput, PredictionOutcome, PredictionSymbol, charge,
};
use intersect_model::definitions::bundle::SourceId;
use std::collections::{BTreeMap, BTreeSet};

#[path = "runtime_ports.rs"]
mod ports;
pub use ports::PortRuntimePredictionSource;
#[path = "runtime_source_prepared.rs"]
mod prepared;
pub(super) use prepared::LivePreparedAccess;
pub use prepared::{
    InitialPreparation, PreparedLiveRuntimePredictionContext, PreparedRuntimePredictionContext,
};

// Fixed logical records, independent of native/WASM pointer width. These are
// conservative policy units, not actual allocator/RSS measurements.
const SOURCE_HEADER: usize = 192;
const HIDDEN_SLOT: usize = 32;
const FRONTIER_SLOT: usize = 64;
const RETURN_SLOT: usize = 8;
const ACCEPTING_SLOT: usize = 64;
const CACHE_SLOT: usize = 160;
const LOGICAL_WORK_QUANTUM: usize = 64;
type Frontier<'program, R = Vec<usize>> =
    Vec<(&'program PredictionSymbol, PredictionConfiguration<R>)>;
type Key<'program> = (&'program str, u8, bool);

pub(super) fn match_frontier<'program, T, R: Ord>(
    rows: impl IntoIterator<Item = (&'program PredictionSymbol, T)>,
    symbol: &Option<PredictionSymbol>,
    remaining: &mut Option<usize>,
    mut transfer: impl FnMut(T, &mut Option<usize>) -> Result<PredictionConfiguration<R>, String>,
) -> Result<BTreeSet<PredictionConfiguration<R>>, String> {
    let mut next = BTreeSet::new();
    for (expected, row) in rows {
        charge(remaining)?;
        if symbol.as_ref() == Some(expected) {
            next.insert(transfer(row, remaining)?);
        }
    }
    Ok(next)
}

struct InitialClosure<'program> {
    frontier: Frontier<'program>,
    accepting: BTreeMap<usize, bool>,
}

/// Pure immutable graph source preparation for one source/hidden-policy scope.
/// The borrowed program cannot change during this scope. Source IDs here are
/// checked locators, not proof that a host loaded or parsed the source bytes.
/// Hosts must own the cursor within one invocation and keep their real ledger.
pub struct RuntimePredictionSource<'program> {
    program: &'program ParserPrediction,
    source: SourceId,
    hidden: Vec<String>,
    rows: BTreeMap<Key<'program>, InitialClosure<'program>>,
    failed: bool,
    logical_bytes: usize,
    hits: usize,
    publications: usize,
}

// These counters describe cumulative new logical admissions, including replay
// copies. They are not live bytes, allocator totals, RSS or the existing S09
// allocation/atomicity closure. Original worker allocations remain unchanged.
fn admit(
    remaining: &mut Option<usize>,
    logical_bytes: &mut usize,
    bytes: usize,
) -> Result<(), String> {
    charge(remaining)?;
    for _ in 0..bytes.div_ceil(LOGICAL_WORK_QUANTUM) {
        charge(remaining)?;
    }
    *logical_bytes = logical_bytes
        .checked_add(bytes)
        .ok_or("Runtime prediction source logical admission overflow")?;
    Ok(())
}

fn return_copy_bytes(row: &Configuration) -> Result<usize, String> {
    // The complete Vec's fixed slots were admitted before try_reserve_exact.
    // Each clone still pays copy work plus all separately allocated return data.
    row.returns
        .len()
        .checked_mul(RETURN_SLOT)
        .ok_or_else(|| "Runtime prediction source return admission overflow".into())
}

impl<'program> RuntimePredictionSource<'program> {
    pub fn new(
        program: &'program ParserPrediction,
        source: &SourceId,
        hidden: &[String],
        remaining: &mut Option<usize>,
    ) -> Result<Self, String> {
        if remaining.is_none() {
            return Err("Runtime prediction sources require the original finite work pool".into());
        }
        let mut logical_bytes = 0;
        let bytes = SOURCE_HEADER
            .checked_add(source.uri.len())
            .and_then(|n| n.checked_add(source.sha256.len()))
            .ok_or("Runtime prediction source header admission overflow")?;
        admit(remaining, &mut logical_bytes, bytes)?;
        let source = source.clone();
        let mut policy = Vec::new();
        admit(
            remaining,
            &mut logical_bytes,
            hidden
                .len()
                .checked_mul(HIDDEN_SLOT)
                .ok_or("Runtime prediction hidden-policy admission overflow")?,
        )?;
        policy
            .try_reserve_exact(hidden.len())
            .map_err(|_| "Runtime prediction hidden-policy allocation denied")?;
        for value in hidden {
            admit(remaining, &mut logical_bytes, value.len())?;
            policy.push(value.clone());
        }
        Ok(Self {
            program,
            source,
            hidden: policy,
            rows: BTreeMap::new(),
            failed: false,
            logical_bytes,
            hits: 0,
            publications: 0,
        })
    }

    pub fn logical_bytes(&self) -> usize {
        self.logical_bytes
    }
    pub fn hits(&self) -> usize {
        self.hits
    }
    pub fn publications(&self) -> usize {
        self.publications
    }
    pub fn is_poisoned(&self) -> bool {
        self.failed
    }
    pub fn invalidate(&mut self) {
        self.failed = true;
        self.rows.clear();
    }
    #[cfg(test)]
    pub(super) fn retained_rows(&self) -> usize {
        self.rows.len()
    }
    #[cfg(test)]
    pub(super) fn logical_policy() -> [usize; 7] {
        [
            SOURCE_HEADER,
            HIDDEN_SLOT,
            FRONTIER_SLOT,
            RETURN_SLOT,
            ACCEPTING_SLOT,
            CACHE_SLOT,
            LOGICAL_WORK_QUANTUM,
        ]
    }
}

// The new sibling owns this additional metadata. The existing public
// RuntimePredictionSource declaration, construction and defaults stay exact.
const REPEATED_HEADER: usize = 64;
const REPEATED_KEY_SLOT: usize = 128;

/// Explicit, invocation-local admission policy. A first visit retains only a
/// canonical program decision key, never tokens, result, error or input state.
/// The second visit may publish the SAME complete eligible initial closure.
/// Cold one-off keys avoid full frontier copies; later hits use the old replay.
pub struct RepeatedRuntimePredictionSource<'program> {
    source: RuntimePredictionSource<'program>,
    seen: BTreeSet<Key<'program>>,
}
impl<'program> RepeatedRuntimePredictionSource<'program> {
    pub fn new(
        program: &'program ParserPrediction,
        source: &SourceId,
        hidden: &[String],
        remaining: &mut Option<usize>,
    ) -> Result<Self, String> {
        let mut source = RuntimePredictionSource::new(program, source, hidden, remaining)?;
        admit(remaining, &mut source.logical_bytes, REPEATED_HEADER)?;
        Ok(Self {
            source,
            seen: BTreeSet::new(),
        })
    }
    pub fn logical_bytes(&self) -> usize {
        self.source.logical_bytes()
    }
    pub fn hits(&self) -> usize {
        self.source.hits()
    }
    pub fn publications(&self) -> usize {
        self.source.publications()
    }
    pub fn is_poisoned(&self) -> bool {
        self.source.is_poisoned()
    }
    pub fn invalidate(&mut self) {
        self.source.invalidate();
        self.seen.clear();
    }
    #[cfg(test)]
    pub(super) fn retained_keys(&self) -> usize {
        self.seen.len()
    }
    #[cfg(test)]
    pub(super) fn retained_rows(&self) -> usize {
        self.source.retained_rows()
    }
}

trait SourceAdmission<'program> {
    const DEFERRED: bool;
    fn replay_allowed(
        &mut self,
        key: Key<'program>,
        remaining: &mut Option<usize>,
        logical: &mut usize,
    ) -> Result<bool, String>;
    fn publish_allowed(&self) -> bool;
}
// Kept by value in SourceAccess: zero-sized and compile-time disabled. The old
// route performs no new lookup, callback, retention or debit.
struct ImmediateAdmission;
impl<'program> SourceAdmission<'program> for ImmediateAdmission {
    const DEFERRED: bool = false;
    fn replay_allowed(
        &mut self,
        _: Key<'program>,
        _: &mut Option<usize>,
        _: &mut usize,
    ) -> Result<bool, String> {
        Ok(true)
    }
    fn publish_allowed(&self) -> bool {
        true
    }
}
struct RepeatedAdmission<'call, 'program> {
    seen: &'call mut BTreeSet<Key<'program>>,
    publish: bool,
}
impl<'program> SourceAdmission<'program> for RepeatedAdmission<'_, 'program> {
    const DEFERRED: bool = true;
    fn replay_allowed(
        &mut self,
        key: Key<'program>,
        remaining: &mut Option<usize>,
        logical: &mut usize,
    ) -> Result<bool, String> {
        self.publish = false;
        charge(remaining)?; // every actual key lookup, including existing keys
        if self.seen.contains(&key) {
            self.publish = true;
        } else {
            // The key borrows its canonical program path; no string or stack
            // is cloned. Admit the logical node BEFORE any tree insertion.
            admit(remaining, logical, REPEATED_KEY_SLOT)?;
            self.seen.insert(key);
        }
        Ok(self.publish)
    }
    fn publish_allowed(&self) -> bool {
        self.publish
    }
}

impl ParserPrediction {
    /// Opt into source preparation; all token reads and commitment remain in
    /// the canonical runtime worker. Only its first complete epsilon closure
    /// without an actually evaluated input guard may be retained. Visible
    /// Full/First predicates stay literal; hidden guards follow the same
    /// worker's deterministic epsilon behavior, without input recognition.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_with_runtime_source<'program>(
        &'program self,
        sources: &mut RuntimePredictionSource<'program>,
        source: &SourceId,
        hidden: &[String],
        path: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String> {
        self.predict_with_source_admission::<_, false>(
            sources,
            source,
            hidden,
            path,
            input,
            remaining,
            verification,
            None,
            ImmediateAdmission,
        )
    }

    /// Explicitly delay complete closure retention until the second invocation
    /// of the same canonical decision/policy key. Recognition, input reads and
    /// guard visibility remain in the same worker. No result is memoized.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_with_repeated_runtime_source<'program>(
        &'program self,
        sources: &mut RepeatedRuntimePredictionSource<'program>,
        source: &SourceId,
        hidden: &[String],
        path: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String> {
        let result = self.predict_with_source_admission::<_, false>(
            &mut sources.source,
            source,
            hidden,
            path,
            input,
            remaining,
            verification,
            None,
            RepeatedAdmission {
                seen: &mut sources.seen,
                publish: false,
            },
        );
        if result.is_err() {
            // The shared worker has already poisoned/cleared the real rows.
            sources.seen.clear();
        }
        result
    }

    /// Explicitly borrow an already complete eligible initial frontier while
    /// matching its actual first token. Only matching full configurations are
    /// cloned/admitted; no input token, result, error, or partial set is cached.
    /// Cold visits/publications retain the existing repeated-admission behavior.
    /// This may change the finite-work failure point; it promises no actual
    /// original-language saving or admission. Immediate and repeated facades
    /// keep their existing owned replay behavior.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_with_borrowed_repeated_runtime_source<'program>(
        &'program self,
        sources: &mut RepeatedRuntimePredictionSource<'program>,
        source: &SourceId,
        hidden: &[String],
        path: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
    ) -> Result<Option<usize>, String> {
        let result = self.predict_with_source_admission::<_, true>(
            &mut sources.source,
            source,
            hidden,
            path,
            input,
            remaining,
            verification,
            None,
            RepeatedAdmission {
                seen: &mut sources.seen,
                publish: false,
            },
        );
        if result.is_err() {
            // The shared worker has already poisoned/cleared the real rows.
            sources.seen.clear();
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn predict_with_source_admission<
        'program,
        A: SourceAdmission<'program>,
        const BORROWED: bool,
    >(
        &'program self,
        sources: &mut RuntimePredictionSource<'program>,
        source: &SourceId,
        hidden: &[String],
        path: &str,
        input: &mut impl PredictionInput,
        remaining: &mut Option<usize>,
        verification: CompletionVerification,
        prepared: Option<(
            &PreparedRuntimePredictionContext<'program>,
            &super::super::Grammar,
        )>,
        admission: A,
    ) -> Result<Option<usize>, String> {
        let result = (|| {
            if self.requires_runtime_capability() {
                return Err(
                    "Runtime-bound decisions require actual live callers and their typed companion"
                        .into(),
                );
            }
            if remaining.is_none() {
                return Err(
                    "Runtime prediction sources require the original finite work pool".into(),
                );
            }
            charge(remaining)?;
            if sources.failed {
                return Err("Runtime prediction source scope is poisoned".into());
            }
            charge(remaining)?;
            if !std::ptr::eq(self, sources.program) {
                return Err("Runtime prediction source belongs to another exact program".into());
            }
            charge(remaining)?;
            if source != &sources.source || hidden.len() != sources.hidden.len() {
                return Err("Runtime prediction source/hidden-policy context differs".into());
            }
            for (actual, expected) in hidden.iter().zip(&sources.hidden) {
                charge(remaining)?;
                if actual != expected {
                    return Err("Runtime prediction source/hidden-policy context differs".into());
                }
            }
            charge(remaining)?;
            let (path, decision) = self
                .decisions
                .get_key_value(path)
                .ok_or("Unknown parser prediction decision")?;
            let recursive_first = self.recursive_decisions.contains(path);
            let verification_key = match verification {
                CompletionVerification::Legacy => 0,
                CompletionVerification::GuardOverapproximation => 1,
                CompletionVerification::BoundedGuardOverapproximation => 2,
                CompletionVerification::InputCheckedCompletion => 3,
            };
            let mut access = SourceAccess::<_, BORROWED> {
                sources,
                key: (path.as_str(), verification_key, recursive_first),
                admission,
            };
            let outcome = if let Some((prepared, grammar)) = prepared {
                if !BORROWED {
                    return Err("Prepared runtime context requires borrowed matching".into());
                }
                prepared.check(grammar, self, hidden, verification, remaining)?;
                let mut prepared_access = prepared::PreparedAccess::new(prepared, &mut access);
                self.run_prediction_at_depth_with_local_decision_and_source(
                    PredictionGoal::Decision {
                        decision,
                        recursive_first,
                        boundary: None,
                    },
                    input,
                    remaining,
                    0,
                    verification,
                    None,
                    &mut prepared_access,
                )
            } else {
                self.run_prediction_at_depth_with_local_decision_and_source(
                    PredictionGoal::Decision {
                        decision,
                        recursive_first,
                        boundary: None,
                    },
                    input,
                    remaining,
                    0,
                    verification,
                    None,
                    &mut access,
                )
            };
            let value = match outcome? {
                PredictionOutcome::Selected(value) => value,
                PredictionOutcome::Completed(_) | PredictionOutcome::InitialPrepared(_) => {
                    return Err("Unexpected completion outcome for a decision".into());
                }
            };
            charge(remaining)?;
            if let Some((prepared, grammar)) = prepared {
                prepared.check(grammar, self, hidden, verification, remaining)?;
            }
            if access.sources.failed {
                return Err("Runtime prediction source scope is poisoned".into());
            }
            Ok(value)
        })();
        if result.is_err() {
            sources.invalidate();
        }
        result
    }
}

pub(super) trait InitialSource<'program, R = Vec<usize>> {
    const ENABLED: bool;
    // Only the private observed neutral preparer and its opaque live access opt in.
    const CALLER_NEUTRAL: bool = false;
    const PREPARE_ONLY: bool = false;
    fn charge(&mut self, remaining: &mut Option<usize>) -> Result<(), String> {
        charge(remaining)
    }
    fn prefix_depth(&mut self, _: usize, _: &mut Option<usize>) -> Result<(), String> {
        Ok(())
    }
    fn match_live_borrowed(
        &mut self,
        _: &Option<PredictionSymbol>,
        _: &super::live_callers::LiveSeed<'_, '_>,
        _: &mut Option<R>,
        _: &mut Option<usize>,
    ) -> Result<BTreeSet<PredictionConfiguration<R>>, String> {
        Err("Caller-neutral frontier is not enabled".into())
    }
    const BORROWED_FRONTIER: bool = false;
    // Compile-time disabled on every historical source facade/default worker.
    const PORTS: bool = false;
    fn replay(
        &mut self,
        frontier: &mut Frontier<'program, R>,
        accepting: &mut BTreeMap<usize, bool>,
        remaining: &mut Option<usize>,
    ) -> Result<bool, String>;
    fn remember(
        &mut self,
        frontier: &[(&'program PredictionSymbol, PredictionConfiguration<R>)],
        accepting: &BTreeMap<usize, bool>,
        remaining: &mut Option<usize>,
    ) -> Result<(), String>;
    fn expand_ports(
        &mut self,
        _: &PredictionConfiguration<R>,
        _: &mut std::collections::VecDeque<PredictionConfiguration<R>>,
        _: &mut Option<usize>,
    ) -> Result<bool, String> {
        Ok(false)
    }
    fn match_borrowed(
        &mut self,
        _: &Option<PredictionSymbol>,
        _: &mut Option<usize>,
    ) -> Result<BTreeSet<PredictionConfiguration<R>>, String> {
        Err("Borrowed runtime frontier is not enabled".into())
    }
}

pub(super) struct NoInitialSource;
impl<'program, R> InitialSource<'program, R> for NoInitialSource {
    const ENABLED: bool = false;
    fn replay(
        &mut self,
        _: &mut Frontier<'program, R>,
        _: &mut BTreeMap<usize, bool>,
        _: &mut Option<usize>,
    ) -> Result<bool, String> {
        Ok(false)
    }
    fn remember(
        &mut self,
        _: &[(&'program PredictionSymbol, PredictionConfiguration<R>)],
        _: &BTreeMap<usize, bool>,
        _: &mut Option<usize>,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct SourceAccess<'call, 'program, A, const BORROWED: bool = false> {
    sources: &'call mut RuntimePredictionSource<'program>,
    key: Key<'program>,
    admission: A,
}
impl<'program, A: SourceAdmission<'program>, const BORROWED: bool> InitialSource<'program>
    for SourceAccess<'_, 'program, A, BORROWED>
{
    const ENABLED: bool = true;
    const BORROWED_FRONTIER: bool = BORROWED;
    fn replay(
        &mut self,
        frontier: &mut Frontier<'program>,
        accepting: &mut BTreeMap<usize, bool>,
        remaining: &mut Option<usize>,
    ) -> Result<bool, String> {
        if A::DEFERRED
            && !self.admission.replay_allowed(
                self.key,
                remaining,
                &mut self.sources.logical_bytes,
            )?
        {
            return Ok(false);
        }
        charge(remaining)?; // attempted lookup, including a cold miss
        let Some(row) = self.sources.rows.get(&self.key) else {
            return Ok(false);
        };
        if BORROWED {
            // No whole frontier or return-stack clone before the real token.
            // Accepting alternatives still precede the SAME ambiguity/fallback
            // checks. The canonical worker checks this unbounded Decision scope.
            for (&alternative, &guarded) in &row.accepting {
                admit(remaining, &mut self.sources.logical_bytes, ACCEPTING_SLOT)?;
                accepting.insert(alternative, guarded);
            }
            self.sources.hits = self
                .sources
                .hits
                .checked_add(1)
                .ok_or("Runtime prediction source hit counter overflow")?;
            return Ok(true);
        }
        admit(
            remaining,
            &mut self.sources.logical_bytes,
            row.frontier
                .len()
                .checked_mul(FRONTIER_SLOT)
                .ok_or("Runtime prediction replay frontier admission overflow")?,
        )?;
        frontier
            .try_reserve_exact(row.frontier.len())
            .map_err(|_| "Runtime prediction replay frontier allocation denied")?;
        for (symbol, target) in &row.frontier {
            admit(
                remaining,
                &mut self.sources.logical_bytes,
                return_copy_bytes(target)?,
            )?;
            frontier.push((*symbol, target.clone()));
        }
        for (&alternative, &guarded) in &row.accepting {
            admit(remaining, &mut self.sources.logical_bytes, ACCEPTING_SLOT)?;
            accepting.insert(alternative, guarded);
        }
        self.sources.hits = self
            .sources
            .hits
            .checked_add(1)
            .ok_or("Runtime prediction source hit counter overflow")?;
        Ok(true)
    }
    fn match_borrowed(
        &mut self,
        symbol: &Option<PredictionSymbol>,
        remaining: &mut Option<usize>,
    ) -> Result<BTreeSet<Configuration>, String> {
        if !BORROWED {
            return Err("Borrowed runtime frontier is not enabled".into());
        }
        // Actual second lookup, paid before reading retained frontier rows.
        charge(remaining)?;
        let row = self
            .sources
            .rows
            .get(&self.key)
            .ok_or("Missing complete borrowed runtime frontier")?;
        let logical = &mut self.sources.logical_bytes;
        match_frontier(
            row.frontier.iter().map(|(symbol, row)| (*symbol, row)),
            symbol,
            remaining,
            |target, remaining| {
                let bytes = FRONTIER_SLOT
                    .checked_add(return_copy_bytes(target)?)
                    .ok_or("Borrowed runtime frontier match admission overflow")?;
                admit(remaining, logical, bytes)?;
                Ok(target.clone())
            },
        )
    }
    fn remember(
        &mut self,
        frontier: &[(&'program PredictionSymbol, Configuration)],
        accepting: &BTreeMap<usize, bool>,
        remaining: &mut Option<usize>,
    ) -> Result<(), String> {
        if A::DEFERRED && !self.admission.publish_allowed() {
            return Ok(());
        }
        let copied = retain_initial(
            frontier,
            accepting,
            remaining,
            &mut self.sources.logical_bytes,
            None,
        )?;
        if self.sources.rows.insert(self.key, copied).is_some() {
            return Err("Duplicate complete runtime prediction source publication".into());
        }
        self.sources.publications = self
            .sources
            .publications
            .checked_add(1)
            .ok_or("Runtime prediction source publication counter overflow")?;
        Ok(())
    }
}

fn admit_initial(
    remaining: &mut Option<usize>,
    logical: &mut usize,
    limit: Option<usize>,
    bytes: usize,
) -> Result<(), String> {
    if let Some(limit) = limit {
        charge(remaining)?;
        if logical.checked_add(bytes).is_none_or(|total| total > limit) {
            return Err("Prepared runtime initial storage budget exceeded".into());
        }
    }
    admit(remaining, logical, bytes)
}
fn retain_initial<'program>(
    frontier: &[(&'program PredictionSymbol, Configuration)],
    accepting: &BTreeMap<usize, bool>,
    remaining: &mut Option<usize>,
    logical: &mut usize,
    limit: Option<usize>,
) -> Result<InitialClosure<'program>, String> {
    let mut copied = Vec::new();
    admit_initial(
        remaining,
        logical,
        limit,
        frontier
            .len()
            .checked_mul(FRONTIER_SLOT)
            .ok_or("Runtime prediction retained frontier admission overflow")?,
    )?;
    copied
        .try_reserve_exact(frontier.len())
        .map_err(|_| "Runtime prediction retained frontier allocation denied")?;
    for (symbol, target) in frontier {
        admit_initial(remaining, logical, limit, return_copy_bytes(target)?)?;
        copied.push((*symbol, target.clone()));
    }
    let mut accepted = BTreeMap::new();
    for (&alternative, &guarded) in accepting {
        admit_initial(remaining, logical, limit, ACCEPTING_SLOT)?;
        accepted.insert(alternative, guarded);
    }
    admit_initial(remaining, logical, limit, CACHE_SLOT)?;
    Ok(InitialClosure {
        frontier: copied,
        accepting: accepted,
    })
}

#[cfg(test)]
mod repeated_admission_tests {
    use super::*;

    #[test]
    fn runtime_repeated_admission_literal_metadata_fees_short_before_insert_and_disabled_zst() {
        assert_eq!(std::mem::size_of::<ImmediateAdmission>(), 0);
        const { assert!(!ImmediateAdmission::DEFERRED) };
        let mut immediate = ImmediateAdmission;
        let mut unlimited = None;
        let mut logical = 19;
        assert!(
            immediate
                .replay_allowed(("canonical", 0, false), &mut unlimited, &mut logical)
                .unwrap()
        );
        assert!(immediate.publish_allowed());
        assert_eq!((unlimited, logical), (None, 19));
        // All three fixed-record policies stay literal, not pointer-size/RSS.
        assert_eq!(
            (REPEATED_HEADER, REPEATED_KEY_SLOT, LOGICAL_WORK_QUANTUM),
            (64, 128, 64)
        );
        for allowance in [0, 1, 2, 3, 4] {
            let mut work = Some(allowance);
            let mut retained = 0;
            let mut seen = BTreeSet::new();
            let mut admission = RepeatedAdmission {
                seen: &mut seen,
                publish: false,
            };
            let result =
                admission.replay_allowed(("canonical", 0, false), &mut work, &mut retained);
            assert_eq!(work, Some(0));
            if allowance == 4 {
                assert_eq!(result, Ok(false));
                assert_eq!(retained, 128);
                assert_eq!(admission.seen.len(), 1);
                assert!(!admission.publish_allowed());
            } else {
                assert!(result.unwrap_err().contains("budget"));
                assert_eq!(retained, 0);
                assert!(admission.seen.is_empty());
            }
        }
        let mut work = Some(6);
        let mut retained = 0;
        let mut seen = BTreeSet::new();
        let mut admission = RepeatedAdmission {
            seen: &mut seen,
            publish: false,
        };
        assert_eq!(
            admission.replay_allowed(("canonical", 0, false), &mut work, &mut retained),
            Ok(false)
        );
        assert_eq!(work, Some(2));
        assert_eq!(
            admission.replay_allowed(("canonical", 0, false), &mut work, &mut retained),
            Ok(true)
        );
        assert_eq!(work, Some(1));
        assert_eq!(
            admission.replay_allowed(("canonical", 0, false), &mut work, &mut retained),
            Ok(true)
        );
        assert_eq!(work, Some(0));
        assert_eq!(retained, 128);
        assert_eq!(admission.seen.len(), 1);
        assert!(admission.publish_allowed());
        // An attempted existing-key lookup at zero is a real denial; no refund
        // or stale publish permit survives the failure.
        assert!(
            admission
                .replay_allowed(("canonical", 0, false), &mut work, &mut retained)
                .is_err()
        );
        assert!(!admission.publish_allowed());
    }
}

#[cfg(test)]
mod borrowed_matcher_tests {
    use super::*;

    #[test]
    fn runtime_borrowed_shared_matcher_charges_before_matching_full_return_stack_clone() {
        let a = PredictionSymbol::Keyword("a".into());
        let b = PredictionSymbol::Keyword("b".into());
        let row = Configuration {
            state: 3,
            returns: vec![7, 9, 11],
            alternative: 1,
            visible: false,
            guarded: true,
            full_guard: true,
        };
        let mut owned_work = Some(1);
        let owned = match_frontier(
            [(&a, row.clone())],
            &Some(a.clone()),
            &mut owned_work,
            |row, _| Ok(row),
        )
        .unwrap();
        assert_eq!(owned_work, Some(0));
        assert_eq!(owned, BTreeSet::from([row.clone()]));
        // SourceAccess uses the SAME helper; this test isolates its exact clone
        // admission boundary from caller-key/replay/acceptance setup.
        let mut logical = 0usize;
        let mut exact = Some(4); //1 compare + (1 event + ceil((64+3*8)/64))
        let borrowed = match_frontier(
            [(&a, &row)],
            &Some(a.clone()),
            &mut exact,
            |target, remaining| {
                admit(remaining, &mut logical, 64 + target.returns.len() * 8)?;
                Ok(target.clone())
            },
        )
        .unwrap();
        assert_eq!(borrowed, owned);
        assert_eq!((exact, logical), (Some(0), 88));
        let mut short = Some(3);
        let mut cloned = false;
        let failure = match_frontier(
            [(&a, &row)],
            &Some(a.clone()),
            &mut short,
            |target, remaining| {
                admit(remaining, &mut 0, 64 + target.returns.len() * 8)?;
                cloned = true;
                Ok(target.clone())
            },
        );
        assert!(failure.unwrap_err().contains("budget"));
        assert_eq!(short, Some(0));
        assert!(!cloned);
        let mut miss = Some(1);
        assert!(
            match_frontier(
                [(&b, &row)],
                &None,
                &mut miss,
                |_, _| -> Result<Configuration, String> {
                    panic!("Nonmatching row must not clone or admit returns")
                }
            )
            .unwrap()
            .is_empty()
        );
        assert_eq!(miss, Some(0));
    }
}
