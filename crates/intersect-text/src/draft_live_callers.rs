//! Explicit live-call sidecar; Machine and every old context keep their fields.
use super::*;
use std::cell::Cell;

const OWNER_BYTES: usize = 256;
const CALL_BYTES: usize = 8;
/// The live call stack depth when the program carries none (its default).
const MAX_CALLS: usize = crate::grammar::DEFAULT_CALL_DEPTH;

struct Active<'g> {
    scope: grammar::LiveCallerPrediction<'g>,
    source: &'g SourceId,
    calls: Vec<&'g Expr>,
}
pub(crate) struct Owner<'g, const SHARED: bool = false> {
    grammar: Option<&'g grammar::Grammar>,
    sources: &'g [SourceId],
    failed: &'g Cell<bool>,
    busy: Cell<bool>,
    paid: Cell<bool>,
    uniform_hidden: Cell<Option<bool>>,
    active: RefCell<Option<Active<'g>>>,
    /// Selections retained by the bound language, reused by every source.
    memo: Option<&'g grammar::RuntimePredictionMemo>,
}
struct Ticket<'a> {
    busy: &'a Cell<bool>,
    failed: &'a Cell<bool>,
    completed: bool,
}
impl Drop for Ticket<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.failed.set(true);
        }
        self.busy.set(false);
    }
}
fn debit(work: &mut Option<usize>) -> StorageResult<()> {
    let left = work
        .as_mut()
        .ok_or("Live caller host needs the original finite pool")?;
    if *left == 0 {
        return Err("Execution step budget exceeded during live caller binding".into());
    }
    *left -= 1;
    Ok(())
}
fn retain(work: &mut Option<usize>, bytes: usize) -> StorageResult<()> {
    debit(work)?;
    for _ in 0..bytes.div_ceil(64) {
        debit(work)?;
    }
    Ok(())
}
impl<'g, const SHARED: bool> Owner<'g, SHARED> {
    fn apply<T>(
        &self,
        work: &mut Option<usize>,
        f: impl FnOnce(&mut Option<Active<'g>>, &mut Option<usize>) -> StorageResult<T>,
    ) -> StorageResult<T> {
        if self.busy.replace(true) {
            self.failed.set(true);
            return Err("Reentrant live caller input".into());
        }
        let mut ticket = Ticket {
            busy: &self.busy,
            failed: self.failed,
            completed: false,
        };
        let result = (|| {
            debit(work)?;
            if self.failed.get() {
                return Err("Live caller input is failed".into());
            }
            let mut active = self
                .active
                .try_borrow_mut()
                .map_err(|_| "Live caller state is already borrowed")?;
            let result = f(&mut active, work);
            if self.failed.get() {
                return Err("Live caller input failed during prediction".into());
            }
            result
        })();
        if result.is_err() {
            self.failed.set(true);
            if let Ok(mut active) = self.active.try_borrow_mut()
                && let Some(active) = active.as_mut()
            {
                active.scope.invalidate();
            }
        }
        ticket.completed = true;
        result
    }
    pub(crate) fn entry_runtime(
        &self,
        uri: &str,
        entry: &str,
        begin: bool,
        success: bool,
        work: &mut Option<usize>,
        admission: Option<&'g grammar::RuntimeGuardedAdmission>,
    ) -> StorageResult<()> {
        self.apply(work, |active, work| {
            if begin {
                if active.is_some() {
                    return Err("Live entry overlaps an active source".into());
                }
                if !self.paid.get() {
                    const {
                        assert!(std::mem::size_of::<Owner<'static>>() <= OWNER_BYTES);
                    }
                    retain(work, OWNER_BYTES)?;
                    self.paid.set(true);
                }
                let grammar = self.grammar.ok_or("Missing live caller grammar")?;
                if entry != grammar.entry {
                    return Err("Live entry differs from the actual grammar entry".into());
                }
                if self.uniform_hidden.get().is_none() {
                    let global = grammar.hidden.as_deref().unwrap_or(&[]);
                    let mut uniform = true;
                    for rule in grammar.rules.values() {
                        debit(work)?;
                        if let Some(hidden) = rule.hidden.as_deref() {
                            debit(work)?;
                            if hidden != global {
                                uniform = false;
                            }
                        }
                    }
                    // Completed immutable metadata fact only. A failed scan
                    // publishes nothing; the enclosing ticket poisons owner.
                    self.uniform_hidden.set(Some(uniform));
                }
                let mut source = None;
                for actual in self.sources {
                    debit(work)?;
                    if actual.uri == uri && source.replace(actual).is_some() {
                        return Err("Duplicate live input source URI".into());
                    }
                }
                let source = source.ok_or("Live entry is outside the closed input sources")?;
                let mut scope = if let Some(admission) = admission {
                    grammar::LiveCallerPrediction::new_runtime_guarded(
                        grammar, source, admission, work,
                    )?
                } else {
                    grammar::LiveCallerPrediction::new(grammar, source, work)?
                };
                scope.set_prediction_memo(self.memo);
                *active = Some(Active {
                    scope,
                    source,
                    calls: Vec::new(),
                });
            } else {
                let current = active
                    .as_ref()
                    .ok_or("Live entry ended without a bound source")?;
                debit(work)?;
                if current.source.uri != uri || !current.calls.is_empty() {
                    return Err(
                        "Live entry ended with a foreign or unbalanced caller context".into(),
                    );
                }
                if !success {
                    return Err("Live entry failed in the original parser".into());
                }
                *active = None;
            }
            Ok(())
        })
    }
    pub(crate) fn call(
        &self,
        uri: &str,
        expression: &Expr,
        enter: bool,
        work: &mut Option<usize>,
    ) -> StorageResult<()> {
        self.apply(work, |active, work| {
            let active = active
                .as_mut()
                .ok_or("Live call has no active input source")?;
            debit(work)?;
            if active.source.uri != uri {
                return Err("Live call belongs to another input source".into());
            }
            if enter {
                // The live call stack follows the program's call depth.
                let max_calls = self
                    .grammar
                    .and_then(|g| g.parser_prediction.as_ref())
                    .map_or(MAX_CALLS, |p| p.call_depth());
                if active.calls.len() >= max_calls {
                    return Err("Parser prediction call depth exceeded".into());
                }
                let actual = active.scope.bind_call(expression, work)?;
                if active.calls.capacity() == 0 {
                    const {
                        assert!(std::mem::size_of::<&Expr>() <= CALL_BYTES);
                    }
                    retain(work, max_calls * CALL_BYTES)?;
                    active
                        .calls
                        .try_reserve_exact(max_calls)
                        .map_err(|_| "Live caller stack allocation denied")?;
                }
                debit(work)?;
                active.calls.push(actual);
            } else {
                debit(work)?;
                if !active
                    .calls
                    .last()
                    .is_some_and(|actual| std::ptr::eq(*actual, expression))
                {
                    return Err("Live call return differs from its exact source occurrence".into());
                }
                active.calls.pop();
            }
            Ok(())
        })
    }
    pub(crate) fn predict_runtime(
        &self,
        request: prediction_sources::Request<'_>,
        input: &mut impl grammar::PredictionInput,
        work: &mut Option<usize>,
        bounded: bool,
    ) -> StorageResult<Option<usize>> {
        self.predict_runtime_with_arena::<false>(request, input, work, bounded)
    }
    pub(crate) fn predict_runtime_with_arena<const ARENA: bool>(
        &self,
        request: prediction_sources::Request<'_>,
        input: &mut impl grammar::PredictionInput,
        work: &mut Option<usize>,
        bounded: bool,
    ) -> StorageResult<Option<usize>> {
        self.predict_runtime_with_initial::<ARENA>(request, input, work, bounded, None)
    }
    pub(crate) fn predict_runtime_with_initial<const ARENA: bool>(
        &self,
        request: prediction_sources::Request<'_>,
        input: &mut impl grammar::PredictionInput,
        work: &mut Option<usize>,
        bounded: bool,
        initial: Option<&grammar::PreparedLiveRuntimePredictionContext<'g>>,
    ) -> StorageResult<Option<usize>> {
        self.apply(work, |active, work| {
            let active = active
                .as_mut()
                .ok_or("Live prediction has no active source")?;
            debit(work)?;
            if active.source.uri != request.uri
                || !self
                    .grammar
                    .is_some_and(|grammar| std::ptr::eq(grammar, request.grammar))
            {
                return Err("Live prediction has a foreign source or grammar".into());
            }
            debit(work)?;
            if request
                .decision
                .strip_suffix(request.suffix)
                .and_then(|prefix| prefix.strip_suffix('/'))
                != request.expression.prediction_path.as_deref()
            {
                return Err("Live decision differs from its actual source occurrence".into());
            }
            let grammar = self.grammar.ok_or("Missing live caller grammar")?;
            let mut hidden = grammar
                .rules
                .get(&grammar.entry)
                .and_then(|rule| rule.hidden.as_deref())
                .or(grammar.hidden.as_deref())
                .unwrap_or(&[]);
            for &caller in &active.calls {
                debit(work)?;
                let name = match &caller.kind {
                    Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => name,
                    _ => return Err("Live stack contains a non-call source occurrence".into()),
                };
                if let Some(policy) = grammar
                    .rules
                    .get(name)
                    .ok_or("Live stack has no actual callee")?
                    .hidden
                    .as_deref()
                {
                    hidden = policy;
                }
            }
            debit(work)?;
            if !(std::ptr::eq(hidden, request.hidden)
                || (hidden.is_empty() && request.hidden.is_empty()))
            {
                return Err("Live prediction hidden policy is not the actual caller policy".into());
            }
            // The SAME Machine supplies the actual current hidden policy and
            // PredictionTokens. No hidden policy or input-dependent fact is
            // cached, replaced, or copied into a second input adapter here.
            if bounded && self.uniform_hidden.get() != Some(true) {
                return Err("Bounded runtime prediction cannot switch hidden policy".into());
            }
            if self.uniform_hidden.get() == Some(true) {
                if let Some(initial) = initial {
                    if ARENA {
                        active.scope.predict_borrowed_arena_returns_with_prepared(
                            initial,
                            request.hidden,
                            active.source,
                            &active.calls,
                            request.expression,
                            request.suffix,
                            input,
                            work,
                        )
                    } else if SHARED {
                        active.scope.predict_shared_returns_with_prepared(
                            initial,
                            request.hidden,
                            active.source,
                            &active.calls,
                            request.expression,
                            request.suffix,
                            input,
                            work,
                        )
                    } else {
                        Err("Caller-neutral runtime facts require actual shared return mode".into())
                    }
                } else if ARENA {
                    active.scope.predict_arena_returns(
                        active.source,
                        &active.calls,
                        request.expression,
                        request.suffix,
                        input,
                        work,
                    )
                } else if SHARED {
                    active.scope.predict_shared_returns(
                        active.source,
                        &active.calls,
                        request.expression,
                        request.suffix,
                        input,
                        work,
                    )
                } else {
                    active.scope.predict(
                        active.source,
                        &active.calls,
                        request.expression,
                        request.suffix,
                        input,
                        work,
                    )
                }
            } else {
                // Policy-changing Calls remain historical. This route cannot
                // invent source-policy switching inside PredictionTokens.
                // All old Full/First/FOLLOW effects and actual errors execute
                // in that same worker with the same original remaining pool.
                active.scope.predict_historical(
                    active.source,
                    &active.calls,
                    request.expression,
                    request.suffix,
                    input,
                    work,
                )
            }
        })
    }
}

/// Crate-private factory for the explicitly selected native runtime capability.
pub(crate) fn bounded_owner<'g>(
    grammar: &'g grammar::Grammar,
    sources: &'g [SourceId],
    failed: &'g Cell<bool>,
) -> Owner<'g, true> {
    Owner {
        grammar: Some(grammar),
        sources,
        failed,
        busy: Cell::new(false),
        paid: Cell::new(false),
        uniform_hidden: Cell::new(None),
        active: RefCell::new(None),
        memo: None,
    }
}
impl<'g, const SHARED: bool> Owner<'g, SHARED> {
    /// Reuse (and extend) the runtime selections retained in `memo`.
    pub(crate) fn with_memo(mut self, memo: &'g grammar::RuntimePredictionMemo) -> Self {
        self.memo = Some(memo);
        self
    }
}
