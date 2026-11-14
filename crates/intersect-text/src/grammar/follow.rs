//! Bounded caller-continuation analysis for parser fragments.
pub(super) mod repeat_certificates;
use super::{Expr, Grammar, Kind, overlaps};
use std::collections::{BTreeMap, BTreeSet};
type SequenceContinuation<'g, 'p> = Option<(&'g [Expr], usize, &'p str)>;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Tokens {
    symbols: BTreeSet<String>,
    caller: bool,
}
impl Tokens {
    fn extend(&mut self, other: Self) {
        self.symbols.extend(other.symbols);
        self.caller |= other.caller;
    }
}

fn charge(budget: &mut usize) -> Result<(), String> {
    if *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallWalkLimit {
    Work,
    SyntaxDepth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CallWalkFailure {
    pub reason: CallWalkLimit,
    pub remaining_work: usize,
    pub syntax_depth: usize,
}

/// Temporary direct-call summaries for the explicit V70 caller-analysis mode.
/// Each successful entry/target and each emission is bounded by charged work;
/// these are unit/name counts, not allocator-byte or string-length budgets.
pub(super) struct RuleCallSummaries<'g> {
    grammar: &'g Grammar,
    summaries: BTreeMap<&'g str, BTreeSet<String>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SummaryFailure {
    Lookup { remaining_work: usize },
    Body(CallWalkFailure),
    Edge { remaining_work: usize },
    Retention { remaining_work: usize },
}
impl<'g> RuleCallSummaries<'g> {
    pub(super) fn new(grammar: &'g Grammar) -> Self {
        Self {
            grammar,
            summaries: BTreeMap::new(),
        }
    }
    pub(super) fn extend(
        &mut self,
        effective_name: Option<&str>,
        pending: &mut BTreeSet<String>,
        budget: &mut usize,
        mut failure: Option<&mut Option<SummaryFailure>>,
    ) -> Result<(), String> {
        if let Err(error) = charge(budget) {
            if let Some(failure) = failure.as_deref_mut() {
                *failure = Some(SummaryFailure::Lookup {
                    remaining_work: *budget,
                });
            }
            return Err(error);
        }
        let name =
            effective_name.ok_or("Missing effective rule identity for fragment call summary")?;
        // The canonical key/body come from this exact immutable grammar, never
        // from optional source metadata or an independently supplied expression.
        let (name, rule) =
            self.grammar.rules.get_key_value(name).ok_or_else(|| {
                format!("Unknown effective rule for fragment call summary: {name}")
            })?;
        if let Some(targets) = self.summaries.get(name.as_str()) {
            return emit_summary(targets, pending, budget, failure);
        }
        let mut targets = BTreeSet::new();
        let mut body_failure = None;
        let sink = if failure.is_some() {
            Some(&mut body_failure)
        } else {
            None
        };
        let scanned = calls_with_failure(&rule.body, &mut targets, budget, sink);
        if let Err(error) = scanned {
            if let (Some(failure), Some(body_failure)) = (failure.as_deref_mut(), body_failure) {
                *failure = Some(SummaryFailure::Body(body_failure));
            }
            return Err(error);
        }
        // A failed emission/retention does not retain even a completed summary.
        // Already charged work and partial pending output are never refunded.
        emit_summary(&targets, pending, budget, failure.as_deref_mut())?;
        if let Err(error) = charge(budget) {
            if let Some(failure) = failure {
                *failure = Some(SummaryFailure::Retention {
                    remaining_work: *budget,
                });
            }
            return Err(error);
        }
        self.summaries.insert(name.as_str(), targets);
        Ok(())
    }
}

fn emit_summary(
    targets: &BTreeSet<String>,
    pending: &mut BTreeSet<String>,
    budget: &mut usize,
    mut failure: Option<&mut Option<SummaryFailure>>,
) -> Result<(), String> {
    for target in targets {
        if let Err(error) = charge(budget) {
            if let Some(failure) = failure.as_deref_mut() {
                *failure = Some(SummaryFailure::Edge {
                    remaining_work: *budget,
                });
            }
            return Err(error);
        }
        // Duplicate insert attempts pay the same explicit emission charge.
        pending.insert(target.clone());
    }
    Ok(())
}

pub(super) fn calls(
    expr: &Expr,
    out: &mut BTreeSet<String>,
    budget: &mut usize,
) -> Result<(), String> {
    calls_with_failure(expr, out, budget, None)
}

pub(super) fn calls_with_failure(
    expr: &Expr,
    out: &mut BTreeSet<String>,
    budget: &mut usize,
    mut failure: Option<&mut Option<CallWalkFailure>>,
) -> Result<(), String> {
    // This shared call walk also serves predicate binding on programmatically
    // supplied grammar IR. Enforce the existing structural depth boundary before
    // recursively inspecting a called body; do not depend on the Rust stack.
    let mut pending = vec![(expr, 0usize)];
    while let Some((expr, depth)) = pending.pop() {
        if let Err(error) = charge(budget) {
            if let Some(failure) = failure.as_deref_mut() {
                *failure = Some(CallWalkFailure {
                    reason: CallWalkLimit::Work,
                    remaining_work: *budget,
                    syntax_depth: depth,
                });
            }
            return Err(error);
        }
        if depth > 64 {
            if let Some(failure) = failure.as_deref_mut() {
                *failure = Some(CallWalkFailure {
                    reason: CallWalkLimit::SyntaxDepth,
                    remaining_work: *budget,
                    syntax_depth: depth,
                });
            }
            return Err("Grammar prediction budget exceeded".into());
        }
        match &expr.kind {
            Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
                out.insert(name.clone());
            }
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => {
                pending.push((value, depth + 1));
            }
            Kind::Sequence(items) | Kind::Alternative(items) => {
                for item in items.iter().rev() {
                    pending.push((item, depth + 1));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn check_fragment_cycles(
    grammar: &Grammar,
    work_budget: super::GrammarAnalysisWorkBudget,
) -> Result<(), String> {
    if !grammar
        .rules
        .values()
        .any(|rule| rule.fragment || rule.datatype.is_some())
    {
        return Ok(());
    }
    // NativeV11 admits productive object/fragment cycles. The shared left-recursion,
    // FIRST/FOLLOW and caller checks still reject nonprogress and ambiguity; the
    // runtime retains its nesting and work budgets. Scalar cycles remain unsupported.
    let mut budget = work_budget.units();
    let mut edges = BTreeMap::new();
    for (name, rule) in &grammar.rules {
        let mut targets = BTreeSet::new();
        calls(&rule.body, &mut targets, &mut budget)?;
        edges.insert(name.clone(), targets);
    }
    fn visit(
        name: &str,
        grammar: &Grammar,
        edges: &BTreeMap<String, BTreeSet<String>>,
        path: &mut Vec<String>,
        budget: &mut usize,
    ) -> Result<(), String> {
        charge(budget)?;
        if let Some(index) = path.iter().position(|item| item == name) {
            if path[index..].iter().any(|item| {
                grammar.rules[item].datatype.is_some()
                    || (grammar.rules[item].fragment && !grammar.profile.has_recursive_fragments())
            }) {
                let kind = if path[index..]
                    .iter()
                    .any(|item| grammar.rules[item].datatype.is_some())
                {
                    "datatype parser rule"
                } else {
                    "parser fragment"
                };
                return Err(format!("Recursive {kind} through {name} is unsupported"));
            }
            return Ok(());
        }
        if path.len() >= 64 {
            return Err("Grammar prediction budget exceeded".into());
        }
        path.push(name.into());
        if let Some(targets) = edges.get(name) {
            for target in targets {
                if grammar.rules.contains_key(target) {
                    visit(target, grammar, edges, path, budget)?;
                }
            }
        }
        path.pop();
        Ok(())
    }
    for (name, rule) in &grammar.rules {
        if rule.datatype.is_some() || (rule.fragment && !grammar.profile.has_recursive_fragments())
        {
            visit(name, grammar, &edges, &mut Vec::new(), &mut budget)?;
        }
    }
    Ok(())
}

/// One charged, transient obligation; no resident grammar/program fields grow.
pub(super) struct RequiredObjectCall {
    pub owner: String,
    pub callee: String,
    pub path: String,
}

/// Only the active validation owner is borrowed; no source or graph is retained.
#[derive(Clone, Copy)]
struct WalkObservation<'g, 'o> {
    context: super::AnalysisObservation<'o>,
    owner: &'g str,
}

// Record the actual denied statement only. This invokes the same charge once;
// observer absence changes neither work nor the returned diagnostic. Propagation
// and nested FIRST errors remain the truthful outer Follow fallback.
fn charge_walk(
    budget: &mut usize,
    grammar: &Grammar,
    site: Option<WalkObservation<'_, '_>>,
    owner: Option<&str>,
    path: &str,
    phase: super::admission_observation::Phase,
) -> Result<(), String> {
    super::observe_phase(
        charge(budget),
        grammar,
        site.map(|site| site.context),
        phase,
        owner.or_else(|| site.map(|site| site.owner)),
        Some(path),
    )
}

// One traversal handles both propagation and validation, so continuation rules
// cannot drift between the fixed point and the admission check.
fn apply_conflict(
    expr: &Expr,
    grammar: &Grammar,
    (path, observation, owner): (&str, Option<WalkObservation<'_, '_>>, Option<&str>),
    budget: &mut usize,
    (deferred, required): (&mut BTreeSet<String>, Option<&mut Vec<RequiredObjectCall>>),
) -> Result<(), String> {
    if matches!(
        grammar.profile,
        super::Profile::NativeV32
            | super::Profile::NativeV33
            | super::Profile::NativeV34
            | super::Profile::NativeV35
            | super::Profile::NativeV36
            | super::Profile::NativeV37
            | super::Profile::NativeV38
            | super::Profile::NativeV39
            | super::Profile::NativeV40
            | super::Profile::NativeV41
            | super::Profile::NativeV42
            | super::Profile::NativeV43
            | super::Profile::NativeV44
            | super::Profile::NativeV45
            | super::Profile::NativeV46
            | super::Profile::NativeV47
            | super::Profile::NativeV48
            | super::Profile::NativeV49
            | super::Profile::NativeV50
            | super::Profile::NativeV51
            | super::Profile::NativeV52
            | super::Profile::NativeV53
            | super::Profile::NativeV54
            | super::Profile::NativeV55
            | super::Profile::NativeV56
            | super::Profile::NativeV57
            | super::Profile::NativeV58
            | super::Profile::NativeV59
            | super::Profile::NativeV60
            | super::Profile::NativeV61
            | super::Profile::NativeV62
            | super::Profile::NativeV63
            | super::Profile::NativeV64
            | super::Profile::NativeV65
            | super::Profile::NativeV66
            | super::Profile::NativeV67
            | super::Profile::NativeV68
            | super::Profile::NativeV69
            | super::Profile::NativeV70
            | super::Profile::NativeV71
            | super::Profile::NativeV72
            | super::Profile::NativeV73
            | super::Profile::NativeV74
            | super::Profile::NativeV75
            | super::Profile::NativeV76
            | super::Profile::NativeV77
            | super::Profile::NativeV78
            | super::Profile::NativeV79
            | super::Profile::NativeV80
            | super::Profile::NativeV81
            | super::Profile::NativeV82
    ) && ['?', '*', '+'].contains(&expr.cardinality)
    {
        deferred.insert(format!("{path}/repeat"));
    } else if super::is_structural_nullable_continuation(expr, grammar) {
        // No decision at this wrapper: validate its children or the called
        // fragment with the propagated caller continuation instead.
    } else if grammar.profile.has_required_object_call_certificates()
        && expr.cardinality == '1'
        && let Kind::Call(callee) = &expr.kind
    {
        charge_walk(
            budget,
            grammar,
            observation,
            owner,
            path,
            super::admission_observation::Phase::FollowRequiredLookup,
        )?;
        let target = grammar
            .rules
            .get(callee)
            .ok_or_else(|| format!("Unknown required object call callee: {callee}"))?;
        if target.fragment || target.datatype.is_some() {
            return super::observe_phase(
                Err("Required nullable call is not a nonfragment object rule".into()),
                grammar,
                observation.map(|site| site.context),
                super::admission_observation::Phase::Follow,
                owner,
                Some(path),
            );
        }
        let owner = owner.ok_or("Required object call has no actual validation owner")?;
        let required = required
            .ok_or("Required nullable object call needs an explicit certificate collector")?;
        // Charge retention before cloning names/path. These are unit bounds,
        // not allocator-byte bounds. Failed proofs are never cached.
        charge_walk(
            budget,
            grammar,
            observation,
            Some(owner),
            path,
            super::admission_observation::Phase::FollowRequiredRetention,
        )?;
        required.push(RequiredObjectCall {
            owner: owner.into(),
            callee: callee.clone(),
            path: path.into(),
        });
    } else {
        return super::observe_phase(
            Err("Fragment caller follow conflict requires unsupported Xtext prediction".into()),
            grammar,
            observation.map(|site| site.context),
            super::admission_observation::Phase::Follow,
            observation.map(|site| site.owner),
            Some(path),
        );
    }
    Ok(())
}

fn walk<'g>(
    expr: &'g Expr,
    outer: &Tokens,
    grammar: &'g Grammar,
    updates: &mut BTreeMap<String, Tokens>,
    (validate, path, observation, owner): (
        bool,
        &str,
        Option<WalkObservation<'_, '_>>,
        Option<&'g str>,
    ),
    budget: &mut usize,
    (deferred, required): (&mut BTreeSet<String>, Option<&mut Vec<RequiredObjectCall>>),
) -> Result<(), String> {
    walk_recorded(
        expr,
        outer,
        grammar,
        updates,
        (validate, path, observation, owner),
        budget,
        (deferred, required, None, None),
    )
}

fn walk_recorded<'g>(
    expr: &'g Expr,
    outer: &Tokens,
    grammar: &'g Grammar,
    updates: &mut BTreeMap<String, Tokens>,
    (validate, path, observation, owner): (
        bool,
        &str,
        Option<WalkObservation<'_, '_>>,
        Option<&'g str>,
    ),
    budget: &mut usize,
    (deferred, required, plan, continuation): (
        &mut BTreeSet<String>,
        Option<&mut Vec<RequiredObjectCall>>,
        Option<&mut frontier::ValidationPlan<'g>>,
        SequenceContinuation<'g, '_>,
    ),
) -> Result<(), String> {
    walk_recorded_prefix_inspected(
        expr,
        outer,
        grammar,
        updates,
        (validate, path, observation, owner),
        budget,
        (deferred, required, plan, continuation),
        None,
    )
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
fn walk_recorded_prefix_inspected<'g>(
    expr: &'g Expr,
    outer: &Tokens,
    grammar: &'g Grammar,
    updates: &mut BTreeMap<String, Tokens>,
    (validate, path, observation, owner): (
        bool,
        &str,
        Option<WalkObservation<'_, '_>>,
        Option<&'g str>,
    ),
    budget: &mut usize,
    (deferred, mut required, mut plan, continuation): (
        &mut BTreeSet<String>,
        Option<&mut Vec<RequiredObjectCall>>,
        Option<&mut frontier::ValidationPlan<'g>>,
        SequenceContinuation<'g, '_>,
    ),
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
) -> Result<(), String> {
    let work_budget = plan
        .as_ref()
        .map_or(grammar.analysis_work_budget, |p| p.work_budget());
    charge_walk(
        budget,
        grammar,
        observation,
        owner,
        path,
        super::admission_observation::Phase::FollowWalk,
    )?;
    let ((head, nullable), facts) = if let Some(prefix) =
        prefix.filter(|prefix| prefix.selects(owner, path))
    {
        // This REPLACES the original independent FIRST invocation. It does not
        // debit/reset the shared FOLLOW or recursive-analysis buckets.
        let mut first_remaining = work_budget.units();
        let (summary, facts) = prefix.analyze(grammar, expr, owner, path, &mut first_remaining)?;
        (
            summary,
            (matches!(
                grammar.profile,
                super::Profile::NativeV74
                    | super::Profile::NativeV75
                    | super::Profile::NativeV76
                    | super::Profile::NativeV77
                    | super::Profile::NativeV78
                    | super::Profile::NativeV79
                    | super::Profile::NativeV80
                    | super::Profile::NativeV81
                    | super::Profile::NativeV82
            ) && repeat_certificates::candidate(expr).is_some())
            .then_some(facts),
        )
    } else {
        let occurrence = if matches!(
            grammar.profile,
            super::Profile::NativeV74
                | super::Profile::NativeV75
                | super::Profile::NativeV76
                | super::Profile::NativeV77
                | super::Profile::NativeV78
                | super::Profile::NativeV79
                | super::Profile::NativeV80
                | super::Profile::NativeV81
                | super::Profile::NativeV82
        ) && repeat_certificates::candidate(expr).is_some()
        {
            Some(super::first_sets::analyze_occurrence(
                expr,
                grammar,
                0,
                work_budget.units(),
            )?)
        } else {
            None
        };
        match occurrence {
            Some((summary, facts)) => (summary, Some(facts)),
            None => (
                super::first_with_work_budget(expr, grammar, 0, work_budget)?,
                None,
            ),
        }
    };
    if let (Some(plan), Some(facts)) = (plan.as_deref_mut(), facts) {
        plan.record_repeat(
            expr,
            grammar,
            (&head, facts, outer),
            (owner, path),
            continuation,
            budget,
        )?;
    }
    if let Some(plan) = plan.as_deref_mut() {
        plan.record(
            (expr, grammar),
            outer,
            (&head, nullable),
            (owner, path),
            budget,
        )?;
    }
    if validate
        && (nullable || ['?', '*', '+'].contains(&expr.cardinality))
        && overlaps(&head, &outer.symbols)
        && !(grammar.profile.has_parser_prediction() && super::visible_entry_guard(expr))
    {
        apply_conflict(
            expr,
            grammar,
            (path, observation, owner),
            budget,
            (deferred, required.as_deref_mut()),
        )?;
    }
    let mut inner = outer.clone();
    if ['*', '+'].contains(&expr.cardinality) {
        inner.symbols.extend(head);
    }
    match &expr.kind {
        Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
            if !validate && grammar.rules.contains_key(name) {
                updates.entry(name.clone()).or_default().extend(inner);
            }
        }
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => walk_recorded_prefix_inspected(
            value,
            &inner,
            grammar,
            updates,
            (validate, &format!("{path}/value"), observation, owner),
            budget,
            (deferred, required.as_deref_mut(), plan.as_deref_mut(), None),
            prefix,
        )?,
        Kind::Sequence(items) => {
            let mut suffix = inner;
            for (index, item) in items.iter().enumerate().rev() {
                walk_recorded_prefix_inspected(
                    item,
                    &suffix,
                    grammar,
                    updates,
                    (validate, &format!("{path}/s{index}"), observation, owner),
                    budget,
                    (
                        deferred,
                        required.as_deref_mut(),
                        plan.as_deref_mut(),
                        (matches!(
                            grammar.profile,
                            super::Profile::NativeV74
                                | super::Profile::NativeV75
                                | super::Profile::NativeV76
                                | super::Profile::NativeV77
                                | super::Profile::NativeV78
                                | super::Profile::NativeV79
                                | super::Profile::NativeV80
                                | super::Profile::NativeV81
                                | super::Profile::NativeV82
                        ))
                        .then_some((items.as_slice(), index + 1, path)),
                    ),
                    prefix,
                )?;
                let (symbols, nullable) =
                    super::first_with_work_budget(item, grammar, 0, work_budget)?;
                let mut next = Tokens {
                    symbols,
                    caller: false,
                };
                if nullable {
                    next.extend(suffix);
                }
                suffix = next;
            }
        }
        Kind::Alternative(items) => {
            for (index, item) in items.iter().enumerate() {
                walk_recorded_prefix_inspected(
                    item,
                    &inner,
                    grammar,
                    updates,
                    (validate, &format!("{path}/a{index}"), observation, owner),
                    budget,
                    (deferred, required.as_deref_mut(), plan.as_deref_mut(), None),
                    prefix,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn propagate(grammar: &Grammar, budget: &mut usize) -> Result<BTreeMap<String, Tokens>, String> {
    if matches!(
        grammar.profile,
        super::Profile::NativeV37
            | super::Profile::NativeV38
            | super::Profile::NativeV39
            | super::Profile::NativeV40
            | super::Profile::NativeV41
            | super::Profile::NativeV42
            | super::Profile::NativeV43
            | super::Profile::NativeV44
            | super::Profile::NativeV45
            | super::Profile::NativeV46
            | super::Profile::NativeV47
            | super::Profile::NativeV48
            | super::Profile::NativeV49
            | super::Profile::NativeV50
            | super::Profile::NativeV51
            | super::Profile::NativeV52
            | super::Profile::NativeV53
            | super::Profile::NativeV54
            | super::Profile::NativeV55
            | super::Profile::NativeV56
            | super::Profile::NativeV57
            | super::Profile::NativeV58
            | super::Profile::NativeV59
            | super::Profile::NativeV60
            | super::Profile::NativeV61
            | super::Profile::NativeV62
            | super::Profile::NativeV63
            | super::Profile::NativeV64
            | super::Profile::NativeV65
            | super::Profile::NativeV66
            | super::Profile::NativeV67
            | super::Profile::NativeV68
            | super::Profile::NativeV69
            | super::Profile::NativeV70
            | super::Profile::NativeV71
            | super::Profile::NativeV72
            | super::Profile::NativeV73
            | super::Profile::NativeV74
            | super::Profile::NativeV75
            | super::Profile::NativeV76
            | super::Profile::NativeV77
            | super::Profile::NativeV78
            | super::Profile::NativeV79
            | super::Profile::NativeV80
            | super::Profile::NativeV81
            | super::Profile::NativeV82
    ) {
        return propagate_transfers(grammar, budget);
    }
    let mut follow: BTreeMap<String, Tokens> = grammar
        .rules
        .keys()
        .map(|name| (name.clone(), Tokens::default()))
        .collect();
    if !(matches!(
        grammar.profile,
        super::Profile::NativeV51
            | super::Profile::NativeV52
            | super::Profile::NativeV53
            | super::Profile::NativeV54
            | super::Profile::NativeV55
            | super::Profile::NativeV56
            | super::Profile::NativeV57
            | super::Profile::NativeV58
            | super::Profile::NativeV59
            | super::Profile::NativeV60
            | super::Profile::NativeV61
            | super::Profile::NativeV62
            | super::Profile::NativeV63
            | super::Profile::NativeV64
            | super::Profile::NativeV65
            | super::Profile::NativeV66
            | super::Profile::NativeV67
            | super::Profile::NativeV68
            | super::Profile::NativeV69
            | super::Profile::NativeV70
            | super::Profile::NativeV71
            | super::Profile::NativeV72
            | super::Profile::NativeV73
            | super::Profile::NativeV74
            | super::Profile::NativeV75
            | super::Profile::NativeV76
            | super::Profile::NativeV77
            | super::Profile::NativeV78
            | super::Profile::NativeV79
            | super::Profile::NativeV80
            | super::Profile::NativeV81
            | super::Profile::NativeV82
    ) && grammar.entry.is_empty())
    {
        follow
            .get_mut(&grammar.entry)
            .ok_or("Missing parser entry")?
            .symbols
            .insert("end-of-input".into());
    }
    let mut pending: BTreeSet<_> = grammar.rules.keys().cloned().collect();
    let mut deferred = BTreeSet::new();
    loop {
        let before = follow.clone();
        for name in &pending {
            let rule = &grammar.rules[name];
            walk(
                &rule.body,
                &before[name],
                grammar,
                &mut follow,
                (false, "", None, None),
                budget,
                (&mut deferred, None),
            )?;
        }
        if follow == before {
            break;
        }
        if matches!(
            grammar.profile,
            super::Profile::NativeV31
                | super::Profile::NativeV32
                | super::Profile::NativeV33
                | super::Profile::NativeV34
                | super::Profile::NativeV35
                | super::Profile::NativeV36
                | super::Profile::NativeV37
                | super::Profile::NativeV38
                | super::Profile::NativeV39
                | super::Profile::NativeV40
                | super::Profile::NativeV41
                | super::Profile::NativeV42
                | super::Profile::NativeV43
                | super::Profile::NativeV44
                | super::Profile::NativeV45
                | super::Profile::NativeV46
                | super::Profile::NativeV47
                | super::Profile::NativeV48
                | super::Profile::NativeV49
                | super::Profile::NativeV50
                | super::Profile::NativeV51
                | super::Profile::NativeV52
                | super::Profile::NativeV53
                | super::Profile::NativeV54
                | super::Profile::NativeV55
                | super::Profile::NativeV56
                | super::Profile::NativeV57
                | super::Profile::NativeV58
                | super::Profile::NativeV59
                | super::Profile::NativeV60
                | super::Profile::NativeV61
                | super::Profile::NativeV62
                | super::Profile::NativeV63
                | super::Profile::NativeV64
                | super::Profile::NativeV65
                | super::Profile::NativeV66
                | super::Profile::NativeV67
                | super::Profile::NativeV68
                | super::Profile::NativeV69
                | super::Profile::NativeV70
                | super::Profile::NativeV71
                | super::Profile::NativeV72
                | super::Profile::NativeV73
                | super::Profile::NativeV74
                | super::Profile::NativeV75
                | super::Profile::NativeV76
                | super::Profile::NativeV77
                | super::Profile::NativeV78
                | super::Profile::NativeV79
                | super::Profile::NativeV80
                | super::Profile::NativeV81
                | super::Profile::NativeV82
        ) {
            // A body's transfer function depends only on its caller continuation.
            // Local FIRST contributions are emitted on the initial all-rule pass.
            // Revisit exactly the rules whose continuation grew in this round.
            pending = follow
                .iter()
                .filter(|(name, tokens)| **tokens != before[*name])
                .map(|(name, _)| name.clone())
                .collect();
        }
    }
    Ok(follow)
}

// The same walk evaluates a symbolic caller continuation once per rule.
// This produces constant token contributions and caller -> callee dependencies.
fn propagate_transfers(
    grammar: &Grammar,
    budget: &mut usize,
) -> Result<BTreeMap<String, Tokens>, String> {
    propagate_transfers_recorded(grammar, budget, None)
}

fn propagate_transfers_recorded<'g>(
    grammar: &'g Grammar,
    budget: &mut usize,
    plan: Option<&mut frontier::ValidationPlan<'g>>,
) -> Result<BTreeMap<String, Tokens>, String> {
    propagate_transfers_recorded_prefix_inspected(grammar, budget, plan, None)
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
fn propagate_transfers_recorded_prefix_inspected<'g>(
    grammar: &'g Grammar,
    budget: &mut usize,
    mut plan: Option<&mut frontier::ValidationPlan<'g>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
) -> Result<BTreeMap<String, Tokens>, String> {
    let mut follow: BTreeMap<String, Tokens> = grammar
        .rules
        .keys()
        .map(|name| (name.clone(), Tokens::default()))
        .collect();
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let symbolic = Tokens {
        symbols: BTreeSet::new(),
        caller: true,
    };
    let mut deferred = BTreeSet::new();
    for (name, rule) in &grammar.rules {
        let mut transfers = BTreeMap::new();
        if let Some(plan) = plan.as_deref_mut() {
            walk_recorded_prefix_inspected(
                &rule.body,
                &symbolic,
                grammar,
                &mut transfers,
                (false, &format!("rule/{name}"), None, Some(name)),
                budget,
                (&mut deferred, None, Some(plan), None),
                prefix,
            )?;
        } else {
            walk(
                &rule.body,
                &symbolic,
                grammar,
                &mut transfers,
                (false, "", None, None),
                budget,
                (&mut deferred, None),
            )?;
        }
        for (callee, transfer) in transfers {
            charge(budget)?;
            follow
                .get_mut(&callee)
                .ok_or("Missing FOLLOW callee")?
                .symbols
                .extend(transfer.symbols);
            if transfer.caller {
                edges.entry(name.clone()).or_default().insert(callee);
            }
        }
    }
    if !(matches!(
        grammar.profile,
        super::Profile::NativeV51
            | super::Profile::NativeV52
            | super::Profile::NativeV53
            | super::Profile::NativeV54
            | super::Profile::NativeV55
            | super::Profile::NativeV56
            | super::Profile::NativeV57
            | super::Profile::NativeV58
            | super::Profile::NativeV59
            | super::Profile::NativeV60
            | super::Profile::NativeV61
            | super::Profile::NativeV62
            | super::Profile::NativeV63
            | super::Profile::NativeV64
            | super::Profile::NativeV65
            | super::Profile::NativeV66
            | super::Profile::NativeV67
            | super::Profile::NativeV68
            | super::Profile::NativeV69
            | super::Profile::NativeV70
            | super::Profile::NativeV71
            | super::Profile::NativeV72
            | super::Profile::NativeV73
            | super::Profile::NativeV74
            | super::Profile::NativeV75
            | super::Profile::NativeV76
            | super::Profile::NativeV77
            | super::Profile::NativeV78
            | super::Profile::NativeV79
            | super::Profile::NativeV80
            | super::Profile::NativeV81
            | super::Profile::NativeV82
    ) && grammar.entry.is_empty())
    {
        follow
            .get_mut(&grammar.entry)
            .ok_or("Missing parser entry")?
            .symbols
            .insert("end-of-input".into());
    }
    let mut pending: BTreeMap<String, BTreeSet<String>> = follow
        .iter()
        .filter(|(_, tokens)| !tokens.symbols.is_empty())
        .map(|(name, tokens)| (name.clone(), tokens.symbols.clone()))
        .collect();
    while let Some((name, delta)) = pending.pop_first() {
        charge(budget)?;
        if let Some(callees) = edges.get(&name) {
            for callee in callees {
                charge(budget)?;
                let target = follow.get_mut(callee).ok_or("Missing FOLLOW callee")?;
                for symbol in &delta {
                    if target.symbols.insert(symbol.clone()) {
                        pending
                            .entry(callee.clone())
                            .or_default()
                            .insert(symbol.clone());
                    }
                }
            }
        }
    }
    Ok(follow)
}

pub(super) fn check(grammar: &Grammar, deferred: &mut BTreeSet<String>) -> Result<(), String> {
    check_observed(grammar, deferred, None)
}

pub(super) fn check_observed(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    observation: Option<super::AnalysisObservation<'_>>,
) -> Result<(), String> {
    let mut budget = 10_000;
    check_recorded(grammar, deferred, &mut budget, observation, None)
}

pub(super) fn check_required_calls(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    observation: Option<super::AnalysisObservation<'_>>,
) -> Result<(Vec<RequiredObjectCall>, usize), String> {
    let mut required = Vec::new();
    let mut remaining = 10_000;
    check_recorded(
        grammar,
        deferred,
        &mut remaining,
        observation,
        Some(&mut required),
    )?;
    Ok((required, remaining))
}

pub(super) fn check_required_calls_certified(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    observation: Option<super::AnalysisObservation<'_>>,
) -> Result<
    (
        Vec<RequiredObjectCall>,
        usize,
        repeat_certificates::RepeatCertificates,
    ),
    String,
> {
    check_required_calls_certified_prefix_inspected(grammar, deferred, observation, None)
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
pub(super) fn check_required_calls_certified_prefix_inspected(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    observation: Option<super::AnalysisObservation<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
) -> Result<
    (
        Vec<RequiredObjectCall>,
        usize,
        repeat_certificates::RepeatCertificates,
    ),
    String,
> {
    check_required_calls_certified_runtime(grammar, deferred, observation, prefix, None)
}

pub(super) fn check_required_calls_certified_runtime(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    observation: Option<super::AnalysisObservation<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
    runtime: Option<&mut super::RuntimeGuardedAdmission>,
) -> Result<
    (
        Vec<RequiredObjectCall>,
        usize,
        repeat_certificates::RepeatCertificates,
    ),
    String,
> {
    let mut required = Vec::new();
    let work_budget = runtime
        .as_ref()
        .map_or(grammar.analysis_work_budget, |a| a.work_budget());
    let mut remaining = work_budget.units();
    let mut repeats = repeat_certificates::RepeatCertificates::default();
    let captured = observation.is_some_and(|context| context.observation.first_failure().is_none());
    let mut plan = match runtime {
        Some(runtime) => frontier::ValidationPlan::with_runtime_repeats(grammar, runtime),
        None => frontier::ValidationPlan::with_repeats(grammar),
    };
    let result: Result<(), String> = (|| {
        let follow = propagate_transfers_recorded_prefix_inspected(
            grammar,
            &mut remaining,
            Some(&mut plan),
            prefix,
        )?;
        plan.replay(
            &follow,
            deferred,
            Some(&mut required),
            &mut remaining,
            observation,
        )?;
        repeats = plan.take_repeats()?;
        Ok(())
    })();
    let result = super::observe_phase(
        result,
        grammar,
        observation,
        super::admission_observation::Phase::Follow,
        None,
        None,
    );
    if captured
        && result.is_err()
        && let Some(context) = observation
    {
        context
            .observation
            .follow_measurements(work_budget.units() - remaining);
    }
    result?;
    Ok((required, remaining, repeats))
}

fn check_recorded(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    budget: &mut usize,
    observation: Option<super::AnalysisObservation<'_>>,
    required: Option<&mut Vec<RequiredObjectCall>>,
) -> Result<(), String> {
    let capture_measurements =
        observation.is_some_and(|context| context.observation.first_failure().is_none());
    let result = if required.is_some() {
        check_with_required_work(grammar, deferred, budget, observation, required)
    } else {
        check_with_work(grammar, deferred, budget, observation)
    };
    let result = super::observe_phase(
        result,
        grammar,
        observation,
        super::admission_observation::Phase::Follow,
        None,
        None,
    );
    if capture_measurements
        && result.is_err()
        && let Some(context) = observation
    {
        // This existing Follow bucket excludes the independently bounded FIRST queries.
        context.observation.follow_measurements(10_000 - *budget);
    }
    result
}

fn check_with_work(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    budget: &mut usize,
    observation: Option<super::AnalysisObservation<'_>>,
) -> Result<(), String> {
    check_with_required_work(grammar, deferred, budget, observation, None)
}

fn check_with_required_work(
    grammar: &Grammar,
    deferred: &mut BTreeSet<String>,
    budget: &mut usize,
    observation: Option<super::AnalysisObservation<'_>>,
    mut required: Option<&mut Vec<RequiredObjectCall>>,
) -> Result<(), String> {
    if grammar.profile.has_symbolic_follow_validation() {
        let mut plan = frontier::ValidationPlan::new(grammar);
        let follow = propagate_transfers_recorded(grammar, budget, Some(&mut plan))?;
        return plan.replay(&follow, deferred, required, budget, observation);
    }
    let follow = propagate(grammar, budget)?;
    let mut unused = BTreeMap::new();
    for (name, rule) in &grammar.rules {
        walk(
            &rule.body,
            &follow[name],
            grammar,
            &mut unused,
            (
                true,
                &format!("rule/{name}"),
                observation.map(|context| WalkObservation {
                    context,
                    owner: name,
                }),
                Some(name),
            ),
            budget,
            (deferred, required.as_deref_mut()),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Profile;
    use crate::grammar::Rule;
    fn expression(kind: Kind) -> Expr {
        Expr {
            kind,
            cardinality: '1',
            first_set_predicated: false,
            original_range: None,
            prediction_path: None,
        }
    }
    fn chain(count: usize, width: usize) -> Grammar {
        let mut rules = BTreeMap::new();
        for index in 0..count {
            let mut items = vec![expression(Kind::Literal("x".into())); width];
            if index + 1 < count {
                items.push(expression(Kind::Call(format!("F{:02}", index + 1))));
            }
            rules.insert(
                format!("F{index:02}"),
                Rule {
                    hidden: None,
                    enum_literals: BTreeMap::new(),
                    nominal_return: None,
                    inferred_return: false,
                    override_declared: false,
                    datatype: None,
                    fragment: true,
                    wildcard: false,
                    class: String::new(),
                    body: expression(Kind::Sequence(items)),
                },
            );
        }
        Grammar {
            call_depth: None,
            analysis_work_budget: Default::default(),
            parameterized_rules: Default::default(),
            final_rules: Default::default(),
            computed_settings: vec![],
            profile: Profile::NativeV31,
            name: "FollowTest".into(),
            entry: "F00".into(),
            rules,
            terminals: BTreeMap::new(),
            hidden: None,
            lexical_prediction: None,
            parser_prediction: None,
            declaration_sources: BTreeMap::new(),
            terminal_order: vec![],
            shadowed_declarations: BTreeMap::new(),
        }
    }
    #[test]
    fn changed_callers_match_full_rounds_and_preserve_budget() {
        for width in [1, 3, 9] {
            let mut grammar = chain(8, width);
            let actual = propagate(&grammar, &mut 10_000).unwrap();
            grammar.profile = Profile::NativeV30;
            assert_eq!(actual, propagate(&grammar, &mut 10_000).unwrap());
            for tokens in actual.values() {
                assert_eq!(&tokens.symbols, &BTreeSet::from(["end-of-input".into()]));
            }
        }
        let mut grammar = chain(24, 24);
        check(&grammar, &mut BTreeSet::new()).unwrap();
        grammar.profile = Profile::NativeV30;
        assert_eq!(
            check(&grammar, &mut BTreeSet::new()).unwrap_err(),
            "Grammar prediction budget exceeded"
        );
        grammar.profile = Profile::NativeV31;
        assert!(propagate(&grammar, &mut 1).is_err());
    }
    #[test]
    fn local_continuations_and_productive_cycles_match_full_rounds() {
        let mut grammar = chain(4, 1);
        let mut recursive = expression(Kind::Call("F01".into()));
        recursive.cardinality = '?';
        grammar.rules.get_mut("F03").unwrap().body = expression(Kind::Sequence(vec![
            expression(Kind::Literal("y".into())),
            recursive,
            expression(Kind::Literal("z".into())),
        ]));
        let actual = propagate(&grammar, &mut 10_000).unwrap();
        assert!(actual["F01"].symbols.contains("literal:z"));
        grammar.profile = Profile::NativeV30;
        assert_eq!(actual, propagate(&grammar, &mut 10_000).unwrap());
        grammar.profile = Profile::NativeV37;
        assert_eq!(actual, propagate(&grammar, &mut 10_000).unwrap());
    }
    #[test]
    fn symbolic_transfers_match_existing_worklist_and_clear_repeated_walk_cost() {
        for (count, width) in [(3, 1), (8, 9), (24, 24)] {
            let mut grammar = chain(count, width);
            grammar.profile = Profile::NativeV36;
            let expected = propagate(&grammar, &mut 10_000).unwrap();
            grammar.profile = Profile::NativeV37;
            assert_eq!(propagate(&grammar, &mut 10_000).unwrap(), expected);
        }
        let mut grammar = chain(30, 120);
        grammar.profile = Profile::NativeV36;
        assert_eq!(
            check(&grammar, &mut BTreeSet::new()).unwrap_err(),
            "Grammar prediction budget exceeded"
        );
        grammar.profile = Profile::NativeV37;
        check(&grammar, &mut BTreeSet::new()).unwrap();
        assert!(propagate(&grammar, &mut 1).is_err());
    }
}

#[cfg(test)]
#[path = "follow_observation_tests.rs"]
mod observation_tests;
#[cfg(test)]
#[path = "follow_required_tests.rs"]
mod required_tests;
#[cfg(test)]
#[path = "follow_summary_tests.rs"]
mod summary_tests;

#[cfg(test)]
#[path = "follow_charge_observation_tests.rs"]
mod charge_observation_tests;

mod frontier;
