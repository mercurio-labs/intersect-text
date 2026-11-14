//! Conservative compile-time proof over the shared runtime prediction graph.
pub(super) mod call_free;
mod filtered;
mod projection;
mod templates;
use super::{
    AdjacentCharge, AdjacentWorkObservation, AnalysisConfiguration, CompletionVerification, Edge,
    ParserPrediction, PredictionSymbol, analysis_charge, observed_analysis_charge,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
trait ClosureRetention {
    fn charge(&self, remaining: &mut usize, _category: AdjacentCharge) -> Result<(), String> {
        analysis_charge(remaining)
    }
    fn before_token_row(&self, remaining: &mut usize) -> Result<(), String>;
}
struct NoRetention;
impl ClosureRetention for NoRetention {
    fn before_token_row(&self, _: &mut usize) -> Result<(), String> {
        Ok(())
    }
}
struct TemplateRetention;
impl ClosureRetention for TemplateRetention {
    fn before_token_row(&self, remaining: &mut usize) -> Result<(), String> {
        analysis_charge(remaining)
    }
}
struct ObservedTemplateRetention<'a>(Option<&'a AdjacentWorkObservation>);
impl ClosureRetention for ObservedTemplateRetention<'_> {
    fn charge(&self, remaining: &mut usize, category: AdjacentCharge) -> Result<(), String> {
        observed_analysis_charge(remaining, self.0, category)
    }
    fn before_token_row(&self, remaining: &mut usize) -> Result<(), String> {
        self.charge(remaining, AdjacentCharge::TemplateTokenRetention)
    }
}
enum Reuse {
    None,
    Exact,
    Relative,
}
#[derive(Debug, Default)]
struct Closure {
    accepting: BTreeSet<usize>,
    returned: BTreeSet<usize>,
    tokens: BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration>>,
}
impl ParserPrediction {
    /// Completion's prerequisite uses the existing conservative proof with the
    /// same caller-owned work allowance; no per-decision allowance is created.
    pub(super) fn verify_completion_decision_remaining(
        &self,
        path: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        self.verify_disjoint_remaining(path, false, Reuse::Relative, remaining)
    }
    pub(super) fn verify_completion_decision_with_verification(
        &self,
        path: &str,
        remaining: &mut usize,
        verification: CompletionVerification,
    ) -> Result<(), String> {
        match verification {
            CompletionVerification::Legacy => {
                self.verify_completion_decision_remaining(path, remaining)
            }
            CompletionVerification::GuardOverapproximation => {
                self.verify_disjoint_remaining(path, true, Reuse::Relative, remaining)
            }
            CompletionVerification::BoundedGuardOverapproximation
            | CompletionVerification::InputCheckedCompletion => {
                self.verify_disjoint_with_discovery_remaining(path, remaining, Some(2000))
            }
        }
    }

    // A source-owned compiler graph supplies these real repeat/fanout entries.
    // This proves iteration progress and exact loop topology, NOT disjointness.
    // The already-existing completion executor must select each actual-input
    // branch with the canonical Decision predictor, checkpoint and rewind.
    pub(super) fn input_checked_literal_repeat(
        &self,
        path: &str,
        decision: &super::Decision,
        points: &[usize],
        remaining: &mut usize,
    ) -> Result<bool, String> {
        analysis_charge(remaining)?;
        let [body, end] = decision.entries.as_slice() else {
            return Ok(false);
        };
        if !path.ends_with("/repeat")
            || decision.exit != Some(1)
            || body == end
            || points.is_empty()
        {
            return Ok(false);
        }
        analysis_charge(remaining)?;
        let body_state = self
            .states
            .get(*body)
            .ok_or("Dangling input-checked repeat body")?;
        let [Edge::Token(PredictionSymbol::Keyword(literal), call)] = body_state.edges.as_slice()
        else {
            return Ok(false);
        };
        if literal.is_empty() || body_state.accepting || body_state.rule_stop {
            return Ok(false);
        }
        analysis_charge(remaining)?;
        let call_state = self
            .states
            .get(*call)
            .ok_or("Dangling input-checked token continuation")?;
        let [Edge::Call { follow, .. }] = call_state.edges.as_slice() else {
            return Ok(false);
        };
        if call_state.accepting || call_state.rule_stop || follow == end {
            return Ok(false);
        }
        for &point in points {
            analysis_charge(remaining)?;
            let choice = self
                .states
                .get(point)
                .ok_or("Dangling input-checked repeat choice")?;
            if choice.accepting
                || choice.rule_stop
                || !matches!(choice.edges.as_slice(),
                [Edge::Epsilon(a),Edge::Epsilon(b)] if a==body && b==end)
            {
                return Ok(false);
            }
            // '+' returns directly to its completed-iteration choice. '*'
            // returns through its one real back edge. Optional/ordinary choices
            // do not satisfy either loop shape; initial entry is not rewritten.
            if *follow != point {
                analysis_charge(remaining)?;
                let back = self
                    .states
                    .get(*follow)
                    .ok_or("Dangling input-checked repeat back edge")?;
                if back.accepting
                    || back.rule_stop
                    || !matches!(back.edges.as_slice(),
                    [Edge::Epsilon(target)] if *target==point)
                {
                    return Ok(false);
                }
            }
        }
        // Only after direct structural eligibility use the SAME charged closure
        // walker. No Call/guard can precede the mandatory visible nonempty token;
        // no depth error is caught and no incomplete proof becomes eligibility.
        let confirmed = self.proof_closure(
            [AnalysisConfiguration {
                state: *body,
                alternative: 0,
                calls: Vec::new(),
            }]
            .into(),
            true,
            remaining,
            0,
            true,
        )?;
        if !confirmed.accepting.is_empty()
            || !confirmed.returned.is_empty()
            || confirmed.tokens.len() != 1
        {
            return Err("Input-checked progress proof changed".into());
        }
        let (_, targets) = confirmed
            .tokens
            .first_key_value()
            .ok_or("Missing input-checked token row")?;
        analysis_charge(remaining)?;
        if targets.len() != 1
            || targets
                .iter()
                .any(|row| row.state != *call || !row.calls.is_empty())
        {
            return Err("Input-checked token continuation changed".into());
        }
        Ok(true)
    }

    // Exact Call -> mandatory literal -> repeat return. A successful callee
    // return must cross that consuming edge, even if the callee is nullable.
    // This proves progress, not disjointness or successful callee termination.
    pub(super) fn input_checked_call_suffix_repeat(
        &self,
        path: &str,
        decision: &super::Decision,
        points: &[usize],
        remaining: &mut usize,
    ) -> Result<bool, String> {
        analysis_charge(remaining)?;
        let [body, end] = decision.entries.as_slice() else {
            return Ok(false);
        };
        if !path.ends_with("/repeat")
            || decision.exit != Some(1)
            || body == end
            || points.is_empty()
        {
            return Ok(false);
        }
        analysis_charge(remaining)?;
        let body_state = self
            .states
            .get(*body)
            .ok_or("Dangling input-checked suffix body")?;
        let [Edge::Call { target, follow }] = body_state.edges.as_slice() else {
            return Ok(false);
        };
        if body_state.accepting || body_state.rule_stop || follow == end {
            return Ok(false);
        }
        // Do not hide malformed targets merely because this local proof does
        // not need to execute or statically expand the callee's language.
        analysis_charge(remaining)?;
        self.states
            .get(*target)
            .ok_or("Dangling input-checked suffix callee")?;
        analysis_charge(remaining)?;
        let suffix = self
            .states
            .get(*follow)
            .ok_or("Dangling input-checked suffix token")?;
        let [Edge::Token(PredictionSymbol::Keyword(word), done)] = suffix.edges.as_slice() else {
            return Ok(false);
        };
        if word.is_empty() || suffix.accepting || suffix.rule_stop || done == end {
            return Ok(false);
        }
        for &point in points {
            analysis_charge(remaining)?;
            let choice = self
                .states
                .get(point)
                .ok_or("Dangling input-checked suffix choice")?;
            if choice.accepting
                || choice.rule_stop
                || !matches!(choice.edges.as_slice(),
                [Edge::Epsilon(a), Edge::Epsilon(b)] if a == body && b == end)
            {
                return Ok(false);
            }
            if *done != point {
                analysis_charge(remaining)?;
                let back = self
                    .states
                    .get(*done)
                    .ok_or("Dangling input-checked suffix back edge")?;
                if back.accepting
                    || back.rule_stop
                    || !matches!(back.edges.as_slice(),
                    [Edge::Epsilon(next)] if *next == point)
                {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    // An exact ordinary Alternative has no iteration progress obligation.
    // InputChecked still executes its existing per-input canonical Decision.
    // This does not certify an acyclic call graph or authorize recursive First
    // commitment; the enclosing scope refuses that metadata before this check.
    pub(super) fn input_checked_ordinary_choice(
        &self,
        path: &str,
        decision: &super::Decision,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        analysis_charge(remaining)?;
        if !path.ends_with("/choice") || decision.exit.is_some() || decision.entries.len() < 2 {
            return Ok(false);
        }
        // The enclosing scope already bound every exact ordered fanout to this
        // same Decision. Entry guards retain their existing priority/visibility
        // route; this additional path grants none of their authority.
        for &entry in &decision.entries {
            analysis_charge(remaining)?;
            let state = self
                .states
                .get(entry)
                .ok_or("Dangling input-checked choice entry")?;
            if state.accepting || state.rule_stop {
                return Ok(false);
            }
            for edge in &state.edges {
                analysis_charge(remaining)?;
                if matches!(edge, Edge::First { .. } | Edge::Full { .. }) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    // Conditional multiway eligibility only. The enclosing scope binds every
    // actual ordered fanout and checks all nested decisions, guard visibility
    // and escaped returns. The source companion separately binds every entry.
    // No FIRST/disjointness or recursive commitment authority is produced here.
    pub(super) fn input_checked_multiway_choice(
        &self,
        path: &str,
        decision: &super::Decision,
        reached: &BTreeSet<usize>,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        analysis_charge(remaining)?;
        if !path.ends_with("/choice") || decision.exit.is_some() || decision.entries.len() < 2 {
            return Ok(false);
        }
        for (ordinal, &entry) in decision.entries.iter().enumerate() {
            analysis_charge(remaining)?;
            let state = self
                .states
                .get(entry)
                .ok_or("Dangling input-checked multiway entry")?;
            if state.accepting || state.rule_stop || state.edges.is_empty() {
                return Ok(false);
            }
            // This ordered prefix check is actual paid work, not a rule-name or
            // source-token shortcut. Preserve all distinct alternative owners.
            for &previous in &decision.entries[..ordinal] {
                analysis_charge(remaining)?;
                if previous == entry {
                    return Ok(false);
                }
            }
            let levels = usize::BITS as usize - reached.len().max(1).leading_zeros() as usize;
            for _ in 0..1 + 2 * levels {
                analysis_charge(remaining)?;
            }
            if !reached.contains(&entry) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // Exact Call(body) | (Guard(prefix) Call(the SAME body)) shape, where
    // Guard is Full or a source-bound First over one canonical prefix Call. This
    // proves a bound per-input replay site, never global disjointness or a
    // commitment for the earlier unguarded alternative. All ordinary entries
    // and the post-guard continuation share one canonical callee and join.
    pub(super) fn input_checked_asymmetric_guarded_choice(
        &self,
        path: &str,
        decision: &super::Decision,
        reached: &BTreeSet<usize>,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        self.input_checked_asymmetric_guarded_shape(path, decision, Some(reached), remaining)
    }

    // A shape-only exclusion preserves the qualified direct-prefix pair route.
    // None additionally requires one prefix Call and never grants authority;
    // Some(reached) retains the original broader Full-prefix witness unchanged.
    pub(super) fn input_checked_asymmetric_guarded_shape(
        &self,
        path: &str,
        decision: &super::Decision,
        reached: Option<&BTreeSet<usize>>,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        analysis_charge(remaining)?;
        let [plain, guarded] = decision.entries.as_slice() else {
            return Ok(false);
        };
        if !path.ends_with("/choice") || decision.exit.is_some() || plain == guarded {
            return Ok(false);
        }
        for &entry in &decision.entries {
            analysis_charge(remaining)?;
            let state = self
                .states
                .get(entry)
                .ok_or("Dangling input-checked guarded entry")?;
            if state.accepting || state.rule_stop {
                return Ok(false);
            }
        }
        let [
            Edge::Call {
                target: body,
                follow: join,
            },
        ] = self.states[*plain].edges.as_slice()
        else {
            return Ok(false);
        };
        // Normalize two EXISTING canonical guard representations. Full already
        // records the operand end. First is eligible only when its actual operand
        // is one canonical Call, whose real return supplies that SAME boundary.
        let (prefix, suffix, prefix_callee) = match self.states[*guarded].edges.as_slice() {
            [Edge::Full { target, stop }] => (*target, *stop, None),
            [
                Edge::First {
                    path: guard_path,
                    target,
                },
            ] => {
                let base = path
                    .strip_suffix("/choice")
                    .ok_or("Missing guarded choice base")?;
                let comparison = 1usize
                    .checked_add(base.len().max(guard_path.len()).div_ceil(64))
                    .ok_or("Input-checked guard path comparison overflow")?;
                for _ in 0..comparison {
                    analysis_charge(remaining)?;
                }
                let Some(branch) = guard_path
                    .strip_prefix(base)
                    .and_then(|tail| tail.strip_prefix("/a1"))
                else {
                    return Ok(false);
                };
                analysis_charge(remaining)?;
                if !branch.is_empty() && !branch.starts_with('/') {
                    return Ok(false);
                }
                let levels =
                    usize::BITS as usize - self.first_guards.len().max(1).leading_zeros() as usize;
                let lookup = levels
                    .checked_mul(2)
                    .and_then(|n| n.checked_mul(1 + guard_path.len().div_ceil(64)))
                    .and_then(|n| n.checked_add(1))
                    .ok_or("Input-checked First guard lookup overflow")?;
                for _ in 0..lookup {
                    analysis_charge(remaining)?;
                }
                if self
                    .first_guards
                    .get(guard_path)
                    .is_none_or(|symbols| symbols.is_empty())
                {
                    return Ok(false);
                }
                analysis_charge(remaining)?;
                let operand = self
                    .states
                    .get(*target)
                    .ok_or("Dangling input-checked First operand")?;
                let [
                    Edge::Call {
                        target: callee,
                        follow,
                    },
                ] = operand.edges.as_slice()
                else {
                    return Ok(false);
                };
                (*target, *follow, Some(*callee))
            }
            _ => return Ok(false),
        };
        let (prefix, suffix) = (&prefix, &suffix);
        if prefix == suffix || suffix == join || prefix == body {
            return Ok(false);
        }
        if reached.is_none() {
            // Only the previously qualified single-prefix pair bypasses fresh
            // conditional admission. Complex Full operands need their own new
            // conditional obligation even when the broader old shape could fit.
            analysis_charge(remaining)?;
            let operand = self
                .states
                .get(*prefix)
                .ok_or("Dangling direct-prefix pair exclusion")?;
            if !matches!(operand.edges.as_slice(),
                [Edge::Call { follow, .. }] if follow == suffix)
            {
                return Ok(false);
            }
        }
        for index in [*plain, *guarded, *body, *join, *prefix, *suffix] {
            if let Some(reached) = reached {
                let levels = usize::BITS as usize - reached.len().max(1).leading_zeros() as usize;
                let lookup = levels
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(1))
                    .ok_or("Input-checked guarded lookup overflow")?;
                for _ in 0..lookup {
                    analysis_charge(remaining)?;
                }
                if !reached.contains(&index) {
                    return Ok(false);
                }
            }
            analysis_charge(remaining)?;
            if self.states.get(index).is_none_or(|state| state.accepting) {
                return Ok(false);
            }
        }
        analysis_charge(remaining)?;
        if self.states[*prefix].rule_stop
            || self.states[*suffix].rule_stop
            || !matches!(self.states[*suffix].edges.as_slice(),
                [Edge::Call { target, follow }] if target == body && follow == join)
        {
            return Ok(false);
        }
        let mut owners = 0usize;
        let mut prefix_owners = 0usize;
        for &(start, stop) in self.rules.values() {
            analysis_charge(remaining)?;
            // Operand endpoints are fresh source-expression states, not a
            // foreign rule start/stop smuggled into the Full boundary.
            if [start, stop].iter().any(|i| i == prefix || i == suffix) {
                return Ok(false);
            }
            analysis_charge(remaining)?;
            let prefix_owner = prefix_callee == Some(start);
            if start != *body && !prefix_owner {
                continue;
            }
            if start == *body {
                owners += 1;
            }
            if prefix_owner {
                prefix_owners += 1;
            }
            analysis_charge(remaining)?;
            let end = self
                .states
                .get(stop)
                .ok_or("Dangling input-checked callee stop")?;
            if !end.rule_stop || end.accepting || !end.edges.is_empty() {
                return Ok(false);
            }
        }
        Ok(owners == 1 && (prefix_callee.is_none() || prefix_owners == 1))
    }

    // This exact control flow executes at most once per invocation: body ends
    // at a fresh done state whose sole edge is the actual optional end. Unlike
    // a loop it needs no consuming-progress proof, even for a nullable callee.
    pub(super) fn input_checked_one_shot_optional(
        &self,
        path: &str,
        decision: &super::Decision,
        points: &[usize],
        remaining: &mut usize,
    ) -> Result<bool, String> {
        analysis_charge(remaining)?;
        let [body, end] = decision.entries.as_slice() else {
            return Ok(false);
        };
        if !path.ends_with("/repeat")
            || decision.exit != Some(1)
            || body == end
            || points.is_empty()
        {
            return Ok(false);
        }
        analysis_charge(remaining)?;
        let state = self
            .states
            .get(*body)
            .ok_or("Dangling input-checked optional body")?;
        if state.accepting || state.rule_stop {
            return Ok(false);
        }
        let done = match state.edges.as_slice() {
            [Edge::Call { target, follow }] => {
                analysis_charge(remaining)?;
                self.states
                    .get(*target)
                    .ok_or("Dangling input-checked optional callee")?;
                *follow
            }
            [Edge::Token(PredictionSymbol::Keyword(word), target)] if !word.is_empty() => *target,
            _ => return Ok(false),
        };
        if done == *body || done == *end {
            return Ok(false);
        }
        analysis_charge(remaining)?;
        let completed = self
            .states
            .get(done)
            .ok_or("Dangling input-checked optional done")?;
        if completed.accepting
            || completed.rule_stop
            || !matches!(completed.edges.as_slice(),
            [Edge::Epsilon(target)] if target == end)
        {
            return Ok(false);
        }
        for &point in points {
            analysis_charge(remaining)?;
            if point == *end || point == done {
                return Ok(false);
            }
            let choice = self
                .states
                .get(point)
                .ok_or("Dangling input-checked optional choice")?;
            if choice.accepting
                || choice.rule_stop
                || !matches!(choice.edges.as_slice(),
                [Edge::Epsilon(a),Edge::Epsilon(b)] if a == body && b == end)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Prove disjointness without using guards or model actions to resolve overlap.
    pub fn verify_disjoint_decision(&self, path: &str) -> Result<(), String> {
        self.verify_disjoint(path, false, Reuse::None)
    }
    /// Prove the larger language obtained by removing admitted guard restrictions.
    pub(crate) fn verify_disjoint_with_guard_overapproximation(
        &self,
        path: &str,
    ) -> Result<(), String> {
        self.verify_disjoint(path, true, Reuse::None)
    }
    /// Reuse exact syntax closures; their construction and propagation share the
    /// same proof budget. Return stacks and call-depth checks remain intact.
    pub(crate) fn verify_disjoint_with_cached_closures(&self, path: &str) -> Result<(), String> {
        self.verify_disjoint(path, true, Reuse::Exact)
    }
    /// Reuse internal syntax work while applying exact caller continuations.
    pub(crate) fn verify_disjoint_with_relative_closures(&self, path: &str) -> Result<(), String> {
        self.verify_disjoint(path, true, Reuse::Relative)
    }
    /// Try a sufficient significant-token proof, then retain exact relative
    /// closure analysis with the same attempt's remaining work allowance.
    pub(crate) fn verify_disjoint_with_projection(&self, path: &str) -> Result<(), String> {
        self.verify_disjoint_with_projection_remaining(path, &mut 10_000)
    }
    pub(crate) fn verify_disjoint_with_certificates(&self, path: &str) -> Result<(), String> {
        self.verify_disjoint_with_certificates_remaining(path, &mut 10_000)
    }
    /// Filter exact caller-aware closures without changing runtime prediction.
    pub(crate) fn verify_disjoint_with_filtered_closures(&self, path: &str) -> Result<(), String> {
        self.verify_disjoint_with_filtered_remaining(path, &mut 10_000)
            .map_err(|message| format!("{message} at prediction decision {path}"))
    }
    fn verify_disjoint_with_filtered_remaining(
        &self,
        path: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        self.verify_disjoint_with_discovery_remaining(path, remaining, None)
    }
    /// Preserve fallback room when binary initial FIRST discovery is too wide.
    #[cfg(test)]
    pub(crate) fn verify_disjoint_with_bounded_discovery(&self, path: &str) -> Result<(), String> {
        self.verify_disjoint_with_bounded_discovery_remaining(path, &mut 10_000)
    }
    pub(in crate::grammar) fn verify_disjoint_with_bounded_discovery_remaining(
        &self,
        path: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        self.verify_disjoint_with_discovery_remaining(path, remaining, Some(2000))
            .map_err(|message| format!("{message} at prediction decision {path}"))
    }
    pub(super) fn verify_disjoint_with_discovery_remaining(
        &self,
        path: &str,
        remaining: &mut usize,
        discovery: Option<usize>,
    ) -> Result<(), String> {
        let roots = &self
            .decisions
            .get(path)
            .ok_or("Unknown parser prediction decision")?
            .entries;
        if roots.len() != 2 {
            return self.verify_disjoint_with_certificates_remaining(path, remaining);
        }
        let (projected, exhausted) = if let Some(allowance) = discovery {
            match projection::prove_with_bounded_discovery(self, roots, remaining, allowance)? {
                projection::Discovery::Proven => (true, false),
                projection::Discovery::Inconclusive => (false, false),
                projection::Discovery::Exhausted => (false, true),
            }
        } else {
            (projection::prove(self, roots, remaining)?, false)
        };
        if projected {
            return Ok(());
        }
        let filtered = if exhausted {
            filtered::prove_with_callee_fence(self, roots, remaining)?
        } else {
            filtered::prove(self, roots, remaining)?
        };
        if filtered {
            return Ok(());
        }
        self.verify_disjoint_remaining(path, true, Reuse::Relative, remaining)
    }
    fn verify_disjoint_with_certificates_remaining(
        &self,
        path: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        let roots = &self
            .decisions
            .get(path)
            .ok_or("Unknown parser prediction decision")?
            .entries;
        if roots.len() == 2 {
            return self.verify_disjoint_with_projection_remaining(path, remaining);
        }
        if roots.len() > 2 {
            // Retain cheap exact proofs before speculative summary work. This
            // allowance is carved out of this attempt, never refunded on failure.
            let allowance = (*remaining).min(1000);
            let mut early = allowance;
            let result = self.verify_disjoint_remaining(path, true, Reuse::Relative, &mut early);
            *remaining -= allowance - early;
            match result {
                Ok(()) => return Ok(()),
                Err(message) if message == "Grammar prediction budget exceeded" => {}
                Err(message) => return Err(message),
            }
            if projection::prove_certificates(self, roots, remaining)? {
                return Ok(());
            }
        }
        self.verify_disjoint_remaining(path, true, Reuse::Relative, remaining)
    }
    fn verify_disjoint_with_projection_remaining(
        &self,
        path: &str,
        remaining: &mut usize,
    ) -> Result<(), String> {
        let roots = &self
            .decisions
            .get(path)
            .ok_or("Unknown parser prediction decision")?
            .entries;
        // Use the projected strategy for binary conflicts. Larger decisions
        // retain the existing proof directly, without speculative budget cost.
        if roots.len() != 2 {
            return self.verify_disjoint_remaining(path, true, Reuse::Relative, remaining);
        }
        if projection::prove(self, roots, remaining)? {
            return Ok(());
        }
        self.verify_disjoint_remaining(path, true, Reuse::Relative, remaining)
    }
    fn verify_disjoint(
        &self,
        path: &str,
        guard_overapproximation: bool,
        reuse: Reuse,
    ) -> Result<(), String> {
        self.verify_disjoint_remaining(path, guard_overapproximation, reuse, &mut 10_000)
    }
    fn verify_disjoint_remaining(
        &self,
        path: &str,
        guard_overapproximation: bool,
        reuse: Reuse,
        remaining: &mut usize,
    ) -> Result<(), String> {
        let decision = self
            .decisions
            .get(path)
            .ok_or("Unknown parser prediction decision")?;
        let initial: BTreeSet<_> = decision
            .entries
            .iter()
            .enumerate()
            .map(|(alternative, state)| AnalysisConfiguration {
                state: *state,
                alternative,
                calls: Vec::new(),
            })
            .collect();
        let mut pending = VecDeque::from([initial]);
        let mut visited = BTreeSet::new();
        let mut cache = BTreeMap::new();
        let mut templates = BTreeMap::new();
        while let Some(configurations) = pending.pop_front() {
            analysis_charge(remaining)?;
            if configurations
                .iter()
                .map(|row| row.alternative)
                .collect::<BTreeSet<_>>()
                .len()
                < 2
                || !visited.insert(configurations.clone())
            {
                continue;
            }
            let Closure {
                accepting, tokens, ..
            } = if matches!(reuse, Reuse::Exact | Reuse::Relative) {
                let mut combined = Closure::default();
                for row in configurations {
                    analysis_charge(remaining)?;
                    let key = (row.state, row.calls.clone());
                    if !cache.contains_key(&key) {
                        // Alternative labels do not affect syntax closure. Use one
                        // local label and restore the caller's label on propagation.
                        let seed = AnalysisConfiguration {
                            alternative: 0,
                            ..row.clone()
                        };
                        let result = if matches!(reuse, Reuse::Relative) {
                            self.relative_proof_closure(
                                [seed].into(),
                                guard_overapproximation,
                                &mut templates,
                                remaining,
                            )?
                        } else {
                            self.proof_closure(
                                [seed].into(),
                                guard_overapproximation,
                                remaining,
                                64,
                                false,
                            )?
                        };
                        cache.insert(key.clone(), result);
                    }
                    let result = cache.get(&key).ok_or("Missing cached prediction closure")?;
                    if !result.accepting.is_empty() {
                        combined.accepting.insert(row.alternative);
                    }
                    for (symbol, targets) in &result.tokens {
                        for target in targets {
                            analysis_charge(remaining)?;
                            combined.tokens.entry(symbol.clone()).or_default().insert(
                                AnalysisConfiguration {
                                    alternative: row.alternative,
                                    ..target.clone()
                                },
                            );
                        }
                    }
                }
                combined
            } else {
                self.proof_closure(
                    configurations,
                    guard_overapproximation,
                    remaining,
                    64,
                    false,
                )?
            };
            if accepting.len() > 1
                || (!accepting.is_empty()
                    && tokens
                        .values()
                        .flatten()
                        .any(|row| !accepting.contains(&row.alternative)))
            {
                return Err("Ambiguous parser prediction requires predicate resolution".into());
            }
            pending.extend(tokens.into_values());
        }
        Ok(())
    }
    fn proof_closure(
        &self,
        configurations: BTreeSet<AnalysisConfiguration>,
        guard_overapproximation: bool,
        remaining: &mut usize,
        depth_limit: usize,
        boundary_returns: bool,
    ) -> Result<Closure, String> {
        self.proof_closure_with_retention(
            configurations,
            guard_overapproximation,
            remaining,
            depth_limit,
            boundary_returns,
            NoRetention,
        )
    }
    fn proof_closure_with_retention<R: ClosureRetention>(
        &self,
        configurations: BTreeSet<AnalysisConfiguration>,
        guard_overapproximation: bool,
        remaining: &mut usize,
        depth_limit: usize,
        boundary_returns: bool,
        retention: R,
    ) -> Result<Closure, String> {
        let mut closure: VecDeque<_> = configurations.into_iter().collect();
        let mut closed = BTreeSet::new();
        let mut accepting = BTreeSet::new();
        let mut returned = BTreeSet::new();
        let mut tokens: BTreeMap<PredictionSymbol, BTreeSet<AnalysisConfiguration>> =
            BTreeMap::new();
        while let Some(row) = closure.pop_front() {
            retention.charge(remaining, AdjacentCharge::TemplateClosureState)?;
            if !closed.insert(row.clone()) {
                continue;
            }
            let state = self
                .states
                .get(row.state)
                .ok_or("Dangling parser prediction state")?;
            if state.accepting {
                accepting.insert(row.alternative);
            }
            if state.rule_stop {
                if boundary_returns && row.calls.is_empty() {
                    returned.insert(row.state);
                } else {
                    self.proof_return_with_retention(row, remaining, &mut closure, &retention)?;
                }
                continue;
            }
            for edge in &state.edges {
                retention.charge(remaining, AdjacentCharge::TemplateClosureEdge)?;
                if matches!(edge, Edge::Token(..)) {
                    retention.before_token_row(remaining)?;
                }
                let mut next = row.clone();
                match edge {
                    Edge::Epsilon(target) | Edge::Action(target) => next.state = *target,
                    Edge::First { path, target }
                        if guard_overapproximation && self.first_guards.contains_key(path) =>
                    {
                        next.state = *target;
                    }
                    Edge::Full { target, .. } if guard_overapproximation => {
                        next.state = *target;
                    }
                    Edge::Full { .. } => return Err(
                        "Full-predicate shared-prefix proof requires explicit overapproximation"
                            .into(),
                    ),
                    Edge::First { .. } => {
                        return Err("Guarded shared-prefix proof remains unsupported".into());
                    }
                    Edge::Token(symbol, target) => {
                        next.state = *target;
                        tokens.entry(symbol.clone()).or_default().insert(next);
                        continue;
                    }
                    Edge::Call { target, follow } => {
                        if next.calls.len() >= depth_limit {
                            return Err("Grammar prediction budget exceeded".into());
                        }
                        next.calls.push((row.state, *follow));
                        next.state = *target;
                    }
                }
                closure.push_back(next);
            }
        }
        Ok(Closure {
            accepting,
            tokens,
            returned,
        })
    }
    fn proof_return(
        &self,
        row: AnalysisConfiguration,
        remaining: &mut usize,
        pending: &mut VecDeque<AnalysisConfiguration>,
    ) -> Result<(), String> {
        self.proof_return_with_retention(row, remaining, pending, &NoRetention)
    }
    fn proof_return_with_retention<R: ClosureRetention>(
        &self,
        row: AnalysisConfiguration,
        remaining: &mut usize,
        pending: &mut VecDeque<AnalysisConfiguration>,
        retention: &R,
    ) -> Result<(), String> {
        if let Some(((_, follow), rest)) = row.calls.split_last() {
            pending.push_back(AnalysisConfiguration {
                state: *follow,
                calls: rest.to_vec(),
                ..row
            });
        } else {
            let state = self
                .states
                .get(row.state)
                .ok_or("Dangling parser prediction state")?;
            if state.follow.is_empty() {
                return Err("Caller FOLLOW context required".into());
            }
            for target in &state.follow {
                retention.charge(remaining, AdjacentCharge::TemplateCallerlessFollow)?;
                pending.push_back(AnalysisConfiguration {
                    state: *target,
                    ..row.clone()
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Decision, State};
    use super::*;
    #[test]
    fn pinned_connector_replays_the_original_relative_proof_budget_failure() {
        // Generated native graph, not a hand-lowered substitute for the grammar.
        // This negative control preserves the obstruction; it awards no support.
        use sha2::Digest as _;
        let raw = include_str!("../../../assets/fixtures/xtext/connector-prediction/program.json");
        assert_eq!(
            format!("{:x}", sha2::Sha256::digest(raw.as_bytes())),
            "c1206e3c6817f35a891dfa1f19c3ab5f26e436baebdcd79a7f63fb8cc0d87aab"
        );
        let program: ParserPrediction = serde_json::from_str(raw).unwrap();
        let path = "rule/Connector/s2/s0/choice";
        assert_eq!(program.states.len(), 1710);
        assert_eq!(program.rules.len(), 273);
        assert_eq!(program.decisions.len(), 298);
        assert_eq!(program.first_guards.len(), 2);
        assert_eq!(program.decisions[path].entries.len(), 2);
        assert_eq!(
            program
                .verify_disjoint_with_relative_closures(path)
                .unwrap_err(),
            "Grammar prediction budget exceeded"
        );
    }
    fn program() -> ParserPrediction {
        let token =
            |text: &str, target| Edge::Token(PredictionSymbol::Keyword(text.into()), target);
        ParserPrediction {
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            allow_empty_choices: false,
            states: vec![
                State {
                    edges: vec![token("a", 2)],
                    ..State::default()
                },
                State {
                    edges: vec![token("a", 3)],
                    ..State::default()
                },
                State {
                    edges: vec![token("b", 4)],
                    ..State::default()
                },
                State {
                    edges: vec![token("c", 4)],
                    ..State::default()
                },
                State {
                    accepting: true,
                    ..State::default()
                },
            ],
            rules: BTreeMap::new(),
            decisions: [(
                "choice".into(),
                Decision {
                    entries: vec![0, 1],
                    exit: None,
                },
            )]
            .into(),
            first_guards: BTreeMap::new(),
            recursive_decisions: BTreeSet::new(),
            runtime_requirement: None,
        }
    }
    #[test]
    fn proof_agrees_with_shared_runtime_and_rejects_duplicate_paths() {
        let mut program = program();
        program.verify_disjoint_decision("choice").unwrap();
        for (suffix, expected) in [("b", 0), ("c", 1)] {
            let mut symbols = [
                PredictionSymbol::Keyword("a".into()),
                PredictionSymbol::Keyword(suffix.into()),
            ]
            .into_iter();
            assert_eq!(
                program
                    .predict("choice", || Ok(symbols.next()), &mut Some(1000))
                    .unwrap(),
                Some(expected)
            );
        }
        program.states[3].edges = program.states[2].edges.clone();
        assert!(
            program
                .verify_disjoint_decision("choice")
                .unwrap_err()
                .contains("Ambiguous")
        );
        assert!(program.verify_disjoint_decision("missing").is_err());
    }
    #[test]
    fn proof_rejects_exhausted_work_and_calls_and_unresolved_guards() {
        let mut wide = program();
        wide.states[0].edges = vec![Edge::Epsilon(2); 10000];
        assert!(
            wide.verify_disjoint_decision("choice")
                .unwrap_err()
                .contains("budget")
        );
        let mut recursive = program();
        recursive.states[0].edges = vec![Edge::Call {
            target: 0,
            follow: 4,
        }];
        assert!(
            recursive
                .verify_disjoint_decision("choice")
                .unwrap_err()
                .contains("budget")
        );
        let mut guarded = program();
        guarded.states[0].edges = vec![Edge::First {
            path: "guard".into(),
            target: 2,
        }];
        assert!(
            guarded
                .verify_disjoint_decision("choice")
                .unwrap_err()
                .contains("Guarded")
        );
    }
    #[test]
    fn proof_rejects_accepting_conflicts_and_missing_follow() {
        let mut mixed = program();
        mixed.states[0] = State {
            accepting: true,
            ..State::default()
        };
        assert!(
            mixed
                .verify_disjoint_decision("choice")
                .unwrap_err()
                .contains("Ambiguous")
        );
        mixed.states[0] = State {
            rule_stop: true,
            ..State::default()
        };
        assert!(
            mixed
                .verify_disjoint_decision("choice")
                .unwrap_err()
                .contains("FOLLOW")
        );
    }
    #[test]
    fn admitted_guard_overapproximation_proves_only_disjoint_paths() {
        let mut guarded = program();
        let original = guarded.states[0].edges.clone();
        let body = guarded.states.len();
        guarded.states.push(State {
            edges: original,
            ..State::default()
        });
        guarded.states[0].edges = vec![Edge::First {
            path: "guard".into(),
            target: body,
        }];
        assert!(
            guarded
                .verify_disjoint_with_guard_overapproximation("choice")
                .unwrap_err()
                .contains("Guarded")
        );
        guarded.first_guards.insert(
            "guard".into(),
            [PredictionSymbol::Keyword("a".into())].into(),
        );
        assert!(
            guarded
                .verify_disjoint_decision("choice")
                .unwrap_err()
                .contains("Guarded")
        );
        guarded
            .verify_disjoint_with_guard_overapproximation("choice")
            .unwrap();
        for (suffix, expected) in [("b", 0), ("c", 1)] {
            let mut tokens = [
                PredictionSymbol::Keyword("a".into()),
                PredictionSymbol::Keyword(suffix.into()),
            ]
            .into_iter();
            assert_eq!(
                guarded
                    .predict("choice", || Ok(tokens.next()), &mut Some(1000))
                    .unwrap(),
                Some(expected)
            );
        }
        guarded.states[3].edges = guarded.states[2].edges.clone();
        assert!(
            guarded
                .verify_disjoint_with_guard_overapproximation("choice")
                .unwrap_err()
                .contains("Ambiguous")
        );
        let mut wide = program();
        wide.states[0].edges = vec![Edge::Epsilon(2); 10000];
        assert!(
            wide.verify_disjoint_with_guard_overapproximation("choice")
                .unwrap_err()
                .contains("budget")
        );
    }
    #[test]
    fn cached_closures_preserve_return_contexts_ambiguity_and_limits() {
        let mut shared = program();
        shared.states[0].edges = vec![Edge::Call {
            target: 5,
            follow: 2,
        }];
        shared.states[1].edges = vec![Edge::Call {
            target: 5,
            follow: 3,
        }];
        shared.states.push(State {
            edges: vec![Edge::Token(PredictionSymbol::Keyword("a".into()), 6)],
            ..State::default()
        });
        shared.states.push(State {
            rule_stop: true,
            ..State::default()
        });
        shared.verify_disjoint_decision("choice").unwrap();
        shared
            .verify_disjoint_with_cached_closures("choice")
            .unwrap();
        for (suffix, expected) in [("b", 0), ("c", 1)] {
            let mut tokens = [
                PredictionSymbol::Keyword("a".into()),
                PredictionSymbol::Keyword(suffix.into()),
            ]
            .into_iter();
            assert_eq!(
                shared
                    .predict("choice", || Ok(tokens.next()), &mut Some(1000))
                    .unwrap(),
                Some(expected)
            );
        }
        shared.states[3].edges = shared.states[2].edges.clone();
        assert!(
            shared
                .verify_disjoint_with_cached_closures("choice")
                .unwrap_err()
                .contains("Ambiguous")
        );
        let mut wide = program();
        wide.states[0].edges = vec![Edge::Epsilon(2); 10000];
        assert!(
            wide.verify_disjoint_with_cached_closures("choice")
                .unwrap_err()
                .contains("budget")
        );
        let mut deep = program();
        deep.states[0].edges = vec![Edge::Call {
            target: 0,
            follow: 4,
        }];
        assert!(
            deep.verify_disjoint_with_cached_closures("choice")
                .unwrap_err()
                .contains("budget")
        );
        let mut unknown = program();
        unknown.states[0].edges = vec![Edge::First {
            path: "missing".into(),
            target: 2,
        }];
        assert!(
            unknown
                .verify_disjoint_with_cached_closures("choice")
                .unwrap_err()
                .contains("Guarded")
        );
    }
    #[test]
    fn relative_closures_preserve_return_contexts_ambiguity_and_limits() {
        let mut shared = program();
        shared.states[0].edges = vec![Edge::Call {
            target: 5,
            follow: 2,
        }];
        shared.states[1].edges = vec![Edge::Call {
            target: 5,
            follow: 3,
        }];
        shared.states.push(State {
            edges: vec![Edge::Token(PredictionSymbol::Keyword("a".into()), 6)],
            ..State::default()
        });
        shared.states.push(State {
            rule_stop: true,
            ..State::default()
        });
        shared.verify_disjoint_decision("choice").unwrap();
        shared
            .verify_disjoint_with_relative_closures("choice")
            .unwrap();
        for (suffix, expected) in [("b", 0), ("c", 1)] {
            let mut tokens = [
                PredictionSymbol::Keyword("a".into()),
                PredictionSymbol::Keyword(suffix.into()),
            ]
            .into_iter();
            assert_eq!(
                shared
                    .predict("choice", || Ok(tokens.next()), &mut Some(1000))
                    .unwrap(),
                Some(expected)
            );
        }
        shared.states[3].edges = shared.states[2].edges.clone();
        assert!(
            shared
                .verify_disjoint_with_relative_closures("choice")
                .unwrap_err()
                .contains("Ambiguous")
        );
        let mut wide = program();
        wide.states[0].edges = vec![Edge::Epsilon(2); 10000];
        assert!(
            wide.verify_disjoint_with_relative_closures("choice")
                .unwrap_err()
                .contains("budget")
        );
        let mut deep = program();
        deep.states[0].edges = vec![Edge::Call {
            target: 0,
            follow: 4,
        }];
        assert!(
            deep.verify_disjoint_with_relative_closures("choice")
                .unwrap_err()
                .contains("budget")
        );
        let mut unknown = program();
        unknown.states[0].edges = vec![Edge::First {
            path: "missing".into(),
            target: 2,
        }];
        assert!(
            unknown
                .verify_disjoint_with_relative_closures("choice")
                .unwrap_err()
                .contains("Guarded")
        );
    }
    #[test]
    fn relative_templates_preserve_boundary_returns_and_remaining_depth() {
        let mut context = program();
        context.states.push(State {
            rule_stop: true,
            ..State::default()
        });
        context.states.push(State {
            edges: vec![Edge::Epsilon(5)],
            ..State::default()
        });
        let mut cache = BTreeMap::new();
        for (follow, symbol, alternative) in [(2, "b", 0), (3, "c", 1)] {
            let seed = AnalysisConfiguration {
                state: 6,
                alternative,
                calls: vec![(alternative, follow)],
            };
            let result = context
                .relative_proof_closure([seed].into(), true, &mut cache, &mut 1000)
                .unwrap();
            let rows = &result.tokens[&PredictionSymbol::Keyword(symbol.into())];
            assert_eq!(rows.len(), 1);
            let row = rows.first().unwrap();
            assert_eq!(row.alternative, alternative);
            assert!(row.calls.is_empty());
        }
        let seed = AnalysisConfiguration {
            state: 6,
            alternative: 0,
            calls: Vec::new(),
        };
        assert!(
            context
                .relative_proof_closure([seed].into(), true, &mut cache, &mut 1000)
                .unwrap_err()
                .contains("FOLLOW")
        );
        let mut called = program();
        called.states[0].edges = vec![Edge::Call {
            target: 5,
            follow: 2,
        }];
        called.states.push(State {
            edges: vec![Edge::Token(PredictionSymbol::Keyword("a".into()), 6)],
            ..State::default()
        });
        called.states.push(State {
            rule_stop: true,
            ..State::default()
        });
        let mut cache = BTreeMap::new();
        let prefix = vec![(1, 3); 63];
        let seed = AnalysisConfiguration {
            state: 0,
            alternative: 1,
            calls: prefix.clone(),
        };
        let result = called
            .relative_proof_closure([seed].into(), true, &mut cache, &mut 1000)
            .unwrap();
        let row = result.tokens[&PredictionSymbol::Keyword("a".into())]
            .first()
            .unwrap();
        assert_eq!(row.calls.len(), 64);
        assert_eq!(&row.calls[..63], prefix);
        assert_eq!(row.calls[63], (0, 2));
        let seed = AnalysisConfiguration {
            state: 0,
            alternative: 1,
            calls: vec![(1, 3); 64],
        };
        assert!(
            called
                .relative_proof_closure([seed].into(), true, &mut cache, &mut 1000)
                .unwrap_err()
                .contains("budget")
        );
        let seed = AnalysisConfiguration {
            state: 0,
            alternative: 0,
            calls: Vec::new(),
        };
        let mut work = 1;
        assert!(
            called
                .relative_proof_closure([seed].into(), true, &mut cache, &mut work)
                .is_err()
        );
        assert_eq!(work, 0);
    }
}
