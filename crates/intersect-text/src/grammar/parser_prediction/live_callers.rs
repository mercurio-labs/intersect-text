//! Checked live caller continuations for the SAME runtime prediction worker.
//! No FIRST approximation, cached recognition, or serialized caller authority.
use super::*;
use intersect_model::definitions::bundle::SourceId;

const HEADER_BYTES: usize = 128;
const RETURN_BYTES: usize = 8;

pub struct LiveCallerPrediction<'grammar> {
    program: &'grammar ParserPrediction,
    grammar: &'grammar Grammar,
    source: &'grammar SourceId,
    root_stop: usize,
    eof: usize,
    failed: bool,
    runtime_admission: Option<&'grammar super::RuntimeGuardedAdmission>,
    /// The root completion scope, verified by the first live decision of this
    /// source and shared by the rest.
    scope: std::cell::OnceCell<Box<CompletionScope<'grammar>>>,
    /// Retained selections of the bound language, reused across its sources.
    memo: Option<&'grammar super::RuntimePredictionMemo>,
    /// Verified full-predicate completion scopes of this source.
    scopes: Box<super::runtime_memo::ScopeCache<'grammar>>,
}

#[derive(Clone, Copy)]
enum ReturnStorage {
    Owned,
    Shared,
    Arena,
    BorrowedArena,
}

pub(super) struct LiveSeed<'a, 'g> {
    pub(super) returns: &'a [usize],
    pub(super) root_stop: usize,
    pub(super) eof: usize,
    /// The completion scope already verified for this root by an enclosing
    /// live decision.
    pub(super) scope: Option<&'a CompletionScope<'a>>,
    /// Completion scopes of full syntactic predicates verified earlier in
    /// this source (see `runtime_memo::ScopeCache`).
    pub(super) scopes: Option<&'a super::runtime_memo::ScopeCache<'g>>,
}

fn retain(work: &mut Option<usize>, bytes: usize) -> Result<(), String> {
    charge(work)?;
    for _ in 0..bytes.div_ceil(64) {
        charge(work)?;
    }
    Ok(())
}

pub(super) fn returns_copy(len: usize, work: &mut Option<usize>) -> Result<(), String> {
    if len != 0 {
        retain(
            work,
            len.checked_mul(RETURN_BYTES)
                .ok_or("Live return copy storage overflow")?,
        )?;
        charge(work)?;
    }
    Ok(())
}

fn key(path: &str, suffix: &str, work: &mut Option<usize>) -> Result<String, String> {
    let bytes = path
        .len()
        .checked_add(suffix.len())
        .and_then(|n| n.checked_add(33))
        .ok_or("Live occurrence key storage overflow")?;
    retain(work, bytes)?;
    Ok(format!("{path}/{suffix}"))
}

/// Resolve a borrowed occurrence in the exact immutable grammar. A path or
/// original byte range on an equal-content foreign Expr is never sufficient.
fn occurrence<'g>(
    grammar: &'g Grammar,
    expression: &Expr,
    work: &mut Option<usize>,
) -> Result<(&'g str, &'g Expr), String> {
    charge(work)?;
    let path = expression
        .prediction_path
        .as_deref()
        .ok_or("Live caller has no occurrence")?;
    let mut parts = path.split('/');
    if parts.next() != Some("rule") {
        return Err("Live caller occurrence has no rule owner".into());
    }
    let name = parts
        .next()
        .ok_or("Live caller occurrence has no rule name")?;
    charge(work)?;
    let (owner, rule) = grammar
        .rules
        .get_key_value(name)
        .ok_or("Unknown live caller owner")?;
    let mut actual = &rule.body;
    for part in parts {
        charge(work)?;
        actual = match (part, &actual.kind) {
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
                        .map_err(|_| "Invalid live caller sequence")?,
                )
                .ok_or("Live caller sequence escapes source")?,
            (_, Kind::Alternative(items)) if part.starts_with('a') => items
                .get(
                    part[1..]
                        .parse::<usize>()
                        .map_err(|_| "Invalid live caller alternative")?,
                )
                .ok_or("Live caller alternative escapes source")?,
            _ => return Err("Live caller path differs from source expression".into()),
        };
    }
    charge(work)?;
    if !std::ptr::eq(actual, expression) {
        return Err("Live caller is not the actual borrowed source expression".into());
    }
    Ok((owner.as_str(), actual))
}

impl<'grammar> LiveCallerPrediction<'grammar> {
    pub fn new(
        grammar: &'grammar Grammar,
        source: &'grammar SourceId,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        Self::new_with_admission(grammar, source, work, None)
    }
    /// Requires the non-deserializable companion minted by the native compiler.
    pub fn new_runtime_guarded(
        grammar: &'grammar Grammar,
        source: &'grammar SourceId,
        admission: &'grammar super::RuntimeGuardedAdmission,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        Self::new_with_admission(grammar, source, work, Some(admission))
    }
    fn new_with_admission(
        grammar: &'grammar Grammar,
        source: &'grammar SourceId,
        work: &mut Option<usize>,
        runtime_admission: Option<&'grammar super::RuntimeGuardedAdmission>,
    ) -> Result<Self, String> {
        const {
            assert!(std::mem::size_of::<LiveCallerPrediction<'static>>() <= HEADER_BYTES);
        }
        if work.is_none() {
            return Err("Live caller prediction requires the original finite pool".into());
        }
        charge(work)?;
        if runtime_admission.is_none() && grammar.profile != super::super::Profile::NativeV67 {
            return Err("Live caller prediction currently requires Native67".into());
        }
        let program = grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing live caller program")?;
        if let Some(admission) = runtime_admission {
            admission.validate(
                grammar,
                program,
                work.as_mut()
                    .ok_or("Runtime admission requires original finite work")?,
            )?;
        } else if program.requires_runtime_capability() {
            return Err(
                "Live caller program requires its compiler-minted runtime companion".into(),
            );
        }
        if !program.decisions.contains_key("@document-recovery") {
            return Err("Live caller prediction needs exact source occurrence bindings".into());
        }
        charge(work)?;
        let root_stop = program
            .rules
            .get(&grammar.entry)
            .ok_or("Missing live entry rule")?
            .1;
        let stop = program
            .states
            .get(root_stop)
            .ok_or("Dangling live entry stop")?;
        if !stop.rule_stop {
            return Err("Live entry has no rule stop".into());
        }
        let mut eof = None;
        for &follow in &stop.follow {
            charge(work)?;
            let state = program
                .states
                .get(follow)
                .ok_or("Dangling live entry FOLLOW")?;
            if let [Edge::Token(PredictionSymbol::End, accepted)] = state.edges.as_slice() {
                charge(work)?;
                if !program
                    .states
                    .get(*accepted)
                    .is_some_and(|state| state.accepting && !state.rule_stop)
                {
                    return Err("Live EOF has no accepting destination".into());
                }
                if eof.replace(follow).is_some() {
                    return Err("Live entry has more than one EOF continuation".into());
                }
            }
        }
        let eof = eof.ok_or("Live entry has no exact EOF continuation")?;
        retain(work, HEADER_BYTES)?;
        Ok(Self {
            program,
            grammar,
            source,
            root_stop,
            eof,
            failed: false,
            runtime_admission,
            scope: std::cell::OnceCell::new(),
            memo: None,
            scopes: Box::default(),
        })
    }

    /// Reuse the selections retained in `memo` (see `RuntimePredictionMemo`).
    pub fn set_prediction_memo(&mut self, memo: Option<&'grammar super::RuntimePredictionMemo>) {
        self.memo = memo;
    }

    pub fn is_poisoned(&self) -> bool {
        self.failed
    }
    pub fn invalidate(&mut self) {
        self.failed = true;
    }

    /// Returns the canonical source borrow, not the caller's short-lived ref.
    /// Cardinalities bind the actual Call.follow (the repetition done state),
    /// rather than pretending that a single call returns after the whole repeat.
    pub fn bind_call(
        &mut self,
        expression: &Expr,
        work: &mut Option<usize>,
    ) -> Result<&'grammar Expr, String> {
        let result = (|| {
            charge(work)?;
            if self.failed {
                return Err("Live caller scope is poisoned".into());
            }
            if work.is_none() {
                return Err("Live caller binding requires the original finite pool".into());
            }
            let (_, actual) = occurrence(self.grammar, expression, work)?;
            self.call_edge(actual, work)?;
            Ok(actual)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn call_edge(
        &self,
        actual: &Expr,
        work: &mut Option<usize>,
    ) -> Result<(&'grammar str, usize), String> {
        charge(work)?;
        let callee = match &actual.kind {
            Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => name,
            _ => return Err("Live continuation is not a parser call".into()),
        };
        let (callee, &(target, _)) = self
            .program
            .rules
            .get_key_value(callee)
            .ok_or("Live call is not a parser rule")?;
        let path = actual
            .prediction_path
            .as_deref()
            .ok_or("Missing live call path")?;
        charge(work)?;
        let recovery_key = key(path, "recovery", work)?;
        let recovery = self
            .program
            .decisions
            .get(&recovery_key)
            .ok_or("Missing live call occurrence binding")?;
        let [start, end] = recovery.entries.as_slice() else {
            return Err("Invalid live call occurrence binding".into());
        };
        let mut at = match actual.cardinality {
            '1' => *start,
            '?' | '*' | '+' => {
                charge(work)?;
                let repeat_key = key(path, "repeat", work)?;
                let repeat = self
                    .program
                    .decisions
                    .get(&repeat_key)
                    .ok_or("Missing live call cardinality binding")?;
                let [body, exit] = repeat.entries.as_slice() else {
                    return Err("Invalid live call cardinality binding".into());
                };
                if exit != end || repeat.exit != Some(1) {
                    return Err("Live call cardinality differs from occurrence".into());
                }
                *body
            }
            _ => return Err("Unsupported live call cardinality".into()),
        };
        charge(work)?;
        let mut state = self
            .program
            .states
            .get(at)
            .ok_or("Dangling live call entry")?;
        // A real source-bound first guard can prefix this exact occurrence.
        // It is neither executed nor erased here; prediction still sees it.
        if let [
            Edge::First {
                path: guard,
                target,
            },
        ] = state.edges.as_slice()
        {
            charge(work)?;
            if guard != path || !self.program.first_guards.contains_key(guard) {
                return Err("Unbound live call entry guard".into());
            }
            at = *target;
            state = self
                .program
                .states
                .get(at)
                .ok_or("Dangling live call guard target")?;
        }
        let [
            Edge::Call {
                target: called,
                follow,
            },
        ] = state.edges.as_slice()
        else {
            return Err("Live call occurrence differs from the actual graph Call".into());
        };
        if *called != target {
            return Err("Live call target differs from source rule".into());
        }
        if self.program.states.get(*follow).is_none() {
            return Err("Dangling live call return".into());
        }
        Ok((callee.as_str(), *follow))
    }

    /// Only real caller returns seed this invocation. The same worker retains
    /// full (state, ordered returns, alternative, visible, guarded) keys and all
    /// First/Full/action/input effects. No result or prior frontier is reused.
    pub fn predict(
        &mut self,
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_inner(
            source,
            callers,
            expression,
            suffix,
            input,
            work,
            true,
            ReturnStorage::Owned,
        )
    }

    /// Explicit shared immutable return storage in the SAME canonical worker.
    /// Full actual contexts, lexical key order, depth and predicates are kept;
    /// only physical Vec copies/pop allocations are replaced by paid sharing.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_shared_returns(
        &mut self,
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_inner(
            source,
            callers,
            expression,
            suffix,
            input,
            work,
            true,
            ReturnStorage::Shared,
        )
    }

    /// Explicit invocation-local arena storage in the SAME canonical worker.
    /// The exact source/caller/hidden/runtime-owner checks below are unchanged.
    /// Requires the original finite at-most4000000000 pool; no pool is reset.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_arena_returns(
        &mut self,
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_inner(
            source,
            callers,
            expression,
            suffix,
            input,
            work,
            true,
            ReturnStorage::Arena,
        )
    }

    /// Qualified source/context checks are identical, then the historical
    /// worker preserves a context whose hidden-policy switching is unsupported
    /// by the live-input adapter. No supplied caller becomes skip authority.
    pub fn predict_historical(
        &mut self,
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_inner(
            source,
            callers,
            expression,
            suffix,
            input,
            work,
            false,
            ReturnStorage::Owned,
        )
    }

    /// Exact caller-neutral graph facts, replayed through paid shared returns.
    /// All actual source/caller/owner checks precede the prepared lookup.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_shared_returns_with_prepared(
        &mut self,
        prepared: &PreparedLiveRuntimePredictionContext<'grammar>,
        hidden: &[String],
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_inner_with_initial(
            source,
            callers,
            expression,
            suffix,
            input,
            work,
            true,
            ReturnStorage::Shared,
            Some((prepared, hidden)),
        )
    }
    /// Same source/caller checks and SAME canonical worker, using one paid arena.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_arena_returns_with_prepared(
        &mut self,
        prepared: &PreparedLiveRuntimePredictionContext<'grammar>,
        hidden: &[String],
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_inner_with_initial(
            source,
            callers,
            expression,
            suffix,
            input,
            work,
            true,
            ReturnStorage::Arena,
            Some((prepared, hidden)),
        )
    }
    /// Explicit invocation-borrowed arena rows over the SAME canonical worker.
    /// The stack bank allocates nothing until the existing paid seed site.
    /// Rows cannot outlive this invocation. Original source/caller/owner and
    /// neutral-fact checks, complete keys and all effects remain unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn predict_borrowed_arena_returns_with_prepared(
        &mut self,
        prepared: &PreparedLiveRuntimePredictionContext<'grammar>,
        hidden: &[String],
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        self.predict_inner_with_initial(
            source,
            callers,
            expression,
            suffix,
            input,
            work,
            true,
            ReturnStorage::BorrowedArena,
            Some((prepared, hidden)),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn predict_inner(
        &mut self,
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
        use_live: bool,
        storage: ReturnStorage,
    ) -> Result<Option<usize>, String> {
        self.predict_inner_with_initial(
            source, callers, expression, suffix, input, work, use_live, storage, None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn predict_inner_with_initial(
        &mut self,
        source: &SourceId,
        callers: &[&Expr],
        expression: &Expr,
        suffix: &str,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
        use_live: bool,
        storage: ReturnStorage,
        initial: Option<(&PreparedLiveRuntimePredictionContext<'grammar>, &[String])>,
    ) -> Result<Option<usize>, String> {
        let result = (|| {
            if matches!(storage, ReturnStorage::Arena | ReturnStorage::BorrowedArena)
                && work
                    .as_ref()
                    .is_none_or(|remaining| *remaining > super::shared_returns::arena::WORK_LIMIT)
            {
                return Err(
                    "Arena prediction requires its original finite at-most4000000000 entry pool"
                        .into(),
                );
            }
            charge(work)?;
            if self.failed {
                return Err("Live caller scope is poisoned".into());
            }
            if self.runtime_admission.is_some() && !use_live {
                return Err("Runtime obligation cannot use historical caller enumeration".into());
            }
            if source != self.source {
                return Err("Live caller input source differs".into());
            }
            if work.is_none() {
                return Err("Live caller prediction requires the original finite pool".into());
            }
            if !self
                .grammar
                .parser_prediction
                .as_ref()
                .is_some_and(|program| std::ptr::eq(program, self.program))
            {
                return Err("Live caller program differs from the exact grammar".into());
            }
            if callers.len() > self.program.call_depth() {
                return Err("Parser prediction call depth exceeded".into());
            }
            let mut owner = self.grammar.entry.as_str();
            let bytes = callers
                .len()
                .checked_mul(RETURN_BYTES)
                .ok_or("Live return storage overflow")?;
            if use_live {
                retain(work, bytes)?;
            }
            let mut returns = Vec::new();
            if use_live {
                returns
                    .try_reserve_exact(callers.len())
                    .map_err(|_| "Live return allocation denied")?;
            }
            for &caller in callers {
                let (actual_owner, actual) = occurrence(self.grammar, caller, work)?;
                if owner != actual_owner {
                    return Err("Live caller chain has a foreign rule owner".into());
                }
                let (callee, follow) = self.call_edge(actual, work)?;
                if use_live {
                    returns.push(follow);
                }
                owner = callee;
            }
            let (actual_owner, _) = occurrence(self.grammar, expression, work)?;
            if owner != actual_owner {
                return Err("Live prediction owner differs from actual caller chain".into());
            }
            if !matches!(suffix, "choice" | "repeat") {
                return Err("Unsupported live prediction decision kind".into());
            }
            let path = expression
                .prediction_path
                .as_deref()
                .ok_or("Missing live prediction path")?;
            charge(work)?;
            let key = key(path, suffix, work)?;
            let decision = self
                .program
                .decisions
                .get(&key)
                .ok_or("Missing live prediction decision")?;
            let mut seed = LiveSeed {
                returns: &returns,
                root_stop: self.root_stop,
                eof: self.eof,
                scope: None,
                // Retained scopes follow the memo: none when it is off, and
                // re-verified against the graph in Verify mode.
                scopes: match self.memo.map(super::RuntimePredictionMemo::mode) {
                    Some(
                        mode @ (super::RuntimePredictionMemoMode::On
                        | super::RuntimePredictionMemoMode::Verify),
                    ) => {
                        self.scopes
                            .verify
                            .set(mode == super::RuntimePredictionMemoMode::Verify);
                        Some(&self.scopes)
                    }
                    _ => None,
                },
            };
            let runtime_choice = if let Some(admission) = self.runtime_admission {
                charge(work)?;
                if admission.contains(&key) && self.program.recursive_decisions.contains(&key) {
                    return Err("Runtime obligation cannot assert recursive-first priority".into());
                }
                admission.live_choice_boundary(
                    self.grammar,
                    self.program,
                    &key,
                    actual_owner,
                    work,
                )?
            } else {
                None
            };
            let boundary = if let Some(_owner_stop) = runtime_choice {
                if !use_live {
                    return Err("Runtime choice requires its actual live callers".into());
                }
                // The companion freshly checked the canonical owner stop.
                for _ in 0..4 {
                    charge(work)?;
                }
                let eof = self
                    .program
                    .states
                    .get(self.eof)
                    .ok_or("Dangling live choice EOF")?;
                let [Edge::Token(PredictionSymbol::End, accepted)] = eof.edges.as_slice() else {
                    return Err("Live choice has no exact EOF edge".into());
                };
                Some(DecisionBoundary {
                    live: true,
                    stop: *accepted,
                    returns: &returns,
                })
            } else {
                None
            };
            let goal = PredictionGoal::Decision {
                decision,
                recursive_first: self.program.recursive_decisions.contains(&key),
                boundary,
            };
            // Only the already admitted/owner-validated Strict Native80 scope
            // selects the same transient strategy used to seal/revalidate it.
            // Old untyped and Linear paths retain their exact Legacy dispatch.
            let verification = match self.runtime_admission.map(|admission| admission.policy()) {
                Some(super::RuntimeGuardedPolicy::InputCheckedCompletionScopes) => {
                    CompletionVerification::InputCheckedCompletion
                }
                Some(super::RuntimeGuardedPolicy::StrictCompletionScopes) => {
                    CompletionVerification::BoundedGuardOverapproximation
                }
                _ => CompletionVerification::Legacy,
            };
            if boundary.is_some() {
                if self.scope.get().is_none() {
                    let remaining = work
                        .as_mut()
                        .ok_or("Live root scope requires finite original work")?;
                    let scope =
                        self.program
                            .live_root_scope(self.root_stop, remaining, verification)?;
                    let _ = self.scope.set(Box::new(scope));
                }
                seed.scope = self.scope.get().map(|scope| &**scope);
            }
            // Runtime decisions with actual live callers may reuse a
            // selection retained by the bound language for the same decision,
            // caller returns and lookahead (see `RuntimePredictionMemo`).
            match self.memo {
                Some(memo) if use_live => {
                    let route = match storage {
                        ReturnStorage::Owned => 0u8,
                        ReturnStorage::Shared => 1,
                        ReturnStorage::Arena => 2,
                        ReturnStorage::BorrowedArena => 3,
                    } | if initial.is_some() { 4 } else { 0 };
                    super::runtime_memo::memoized(
                        memo,
                        super::runtime_memo::Decision {
                            owner: (
                                std::ptr::from_ref(self.grammar) as usize,
                                self.runtime_admission
                                    .map_or(0, |admission| std::ptr::from_ref(admission) as usize),
                            ),
                            key: &key,
                            returns: &returns,
                            route,
                            retain_none: suffix != "choice",
                        },
                        input,
                        work,
                        |input, work| {
                            self.run_decision(
                                goal,
                                &seed,
                                &key,
                                boundary.is_some(),
                                verification,
                                storage,
                                initial,
                                use_live,
                                input,
                                work,
                            )
                        },
                    )
                }
                _ => self.run_decision(
                    goal,
                    &seed,
                    &key,
                    boundary.is_some(),
                    verification,
                    storage,
                    initial,
                    use_live,
                    input,
                    work,
                ),
            }
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// The uncached live decision over `input`: the canonical worker, with
    /// deferred nested decisions resolved and the prediction restarted.
    #[allow(clippy::too_many_arguments)]
    fn run_decision(
        &self,
        goal: PredictionGoal<'_>,
        seed: &LiveSeed<'_, 'grammar>,
        key: &str,
        bounded: bool,
        verification: CompletionVerification,
        storage: ReturnStorage,
        initial: Option<(&PreparedLiveRuntimePredictionContext<'grammar>, &[String])>,
        use_live: bool,
        input: &mut impl PredictionInput,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        // Nested decisions that reach the recursion cap are deferred here,
        // resolved from this shallow stack and retained; the prediction
        // then restarts from its original position and reuses them.
        let start = if input.begin_nested_deferral() {
            Some(input.checkpoint()?)
        } else {
            None
        };
        let outcome = loop {
            let outcome = (|| -> Result<PredictionOutcome, String> {
                if bounded {
                    // Validate any offered prepared binding, then use the same cold
                    // canonical worker: its prepared frontier is not a bounded-goal fact.
                    if let Some((prepared, hidden)) = initial {
                        let _checked = runtime_source::LivePreparedAccess::new(
                            prepared,
                            self.grammar,
                            self.program,
                            hidden,
                            verification,
                            key,
                            seed.returns.len(),
                            work,
                        )?;
                    }
                    if matches!(storage, ReturnStorage::BorrowedArena) {
                        use super::shared_returns::arena::borrowed::{
                            ArenaBank, BorrowedArenaReturns,
                        };
                        let bank = ArenaBank::new();
                        self.program
                            .run_prediction_with_return_storage_seeded::<BorrowedArenaReturns<'_>, _>(
                                goal,
                                input,
                                work,
                                0,
                                verification,
                                None,
                                &mut runtime_source::NoInitialSource,
                                Some(seed),
                                Some(BorrowedArenaReturns::unseeded(&bank)),
                            )
                    } else if matches!(storage, ReturnStorage::Arena) {
                        self.program.run_prediction_with_return_storage::<super::shared_returns::arena::ArenaReturns,_>(goal,input,work,0,verification,None,&mut runtime_source::NoInitialSource,Some(seed))
                    } else if matches!(storage, ReturnStorage::Shared) {
                        self.program.run_prediction_with_return_storage::<super::shared_returns::SharedReturns,_>(goal,input,work,0,verification,None,&mut runtime_source::NoInitialSource,Some(seed))
                    } else {
                        self.program.run_prediction_with_live_seed(
                            goal,
                            input,
                            work,
                            0,
                            verification,
                            None,
                            &mut runtime_source::NoInitialSource,
                            Some(seed),
                        )
                    }
                } else if let Some((prepared, hidden)) = initial {
                    if !use_live || matches!(storage, ReturnStorage::Owned) {
                        return Err(
                            "Prepared live facts require actual paid shared caller returns".into(),
                        );
                    }
                    let mut access = runtime_source::LivePreparedAccess::new(
                        prepared,
                        self.grammar,
                        self.program,
                        hidden,
                        verification,
                        key,
                        seed.returns.len(),
                        work,
                    )?;
                    if matches!(storage, ReturnStorage::BorrowedArena) {
                        use super::shared_returns::arena::borrowed::{
                            ArenaBank, BorrowedArenaReturns,
                        };
                        // No allocation, debit or observation before the old seed site.
                        let bank = ArenaBank::new();
                        self.program
                            .run_prediction_with_return_storage_seeded::<BorrowedArenaReturns<'_>, _>(
                                goal,
                                input,
                                work,
                                0,
                                verification,
                                None,
                                &mut access,
                                Some(seed),
                                Some(BorrowedArenaReturns::unseeded(&bank)),
                            )
                    } else if matches!(storage, ReturnStorage::Arena) {
                        self.program.run_prediction_with_return_storage::<super::shared_returns::arena::ArenaReturns,_>(
                            goal,input,work,0,verification,None,&mut access,Some(seed),
                        )
                    } else {
                        self.program.run_prediction_with_return_storage::<super::shared_returns::SharedReturns,_>(
                            goal,input,work,0,verification,None,&mut access,Some(seed),
                        )
                    }
                } else if matches!(storage, ReturnStorage::BorrowedArena) {
                    Err("Borrowed arena requires the exact prepared live context".into())
                } else if matches!(storage, ReturnStorage::Arena) {
                    self.program.run_prediction_with_return_storage::<super::shared_returns::arena::ArenaReturns, _>(
                        goal, input, work, 0, verification, None,
                        &mut runtime_source::NoInitialSource, Some(seed),
                    )
                } else if matches!(storage, ReturnStorage::Shared) {
                    self.program
                        .run_prediction_with_return_storage::<super::shared_returns::SharedReturns, _>(
                            goal,
                            input,
                            work,
                            0,
                            verification,
                            None,
                            &mut runtime_source::NoInitialSource,
                            Some(seed),
                        )
                } else {
                    self.program.run_prediction_with_live_seed(
                        goal,
                        input,
                        work,
                        0,
                        verification,
                        None,
                        &mut runtime_source::NoInitialSource,
                        if use_live { Some(seed) } else { None },
                    )
                }
            })();
            match (outcome, start) {
                (Err(error), Some(start)) if error == super::NESTED_DECISION_DEFERRED => {
                    let chain = input.take_deferred_nested_decisions();
                    let mut resolved = if chain.is_empty() {
                        Err("Deferred nested decision disappeared".into())
                    } else {
                        Ok(())
                    };
                    for deferred in chain {
                        if resolved.is_err() {
                            break;
                        }
                        resolved = self.program.resolve_deferred_nested_decision(
                            deferred,
                            input,
                            work,
                            verification,
                            seed,
                        );
                    }
                    if let Err(error) = resolved.and_then(|()| input.rewind(start)) {
                        break Err(error);
                    }
                }
                (outcome, _) => break outcome,
            }
        };
        if start.is_some() {
            input.end_nested_deferral();
        }
        match outcome? {
            PredictionOutcome::Selected(value) => Ok(value),
            PredictionOutcome::Completed(_) | PredictionOutcome::InitialPrepared(_) => {
                Err("Live decision returned completion".into())
            }
        }
    }
}
