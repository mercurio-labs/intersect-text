//! Bind analyzer-owned source evidence to this exact shared prediction graph.
use super::{
    AdjacentCharge, AdjacentWorkObservation, Decision, Edge, ParserPrediction, PredictionSymbol,
    observed_analysis_charge,
};
use crate::grammar::Expr;
use crate::grammar::Grammar;
use crate::grammar::Kind;
use crate::grammar::follow::repeat_certificates::GraphBinding;
use crate::grammar::follow::repeat_certificates::RepeatCertificates;
use crate::grammar::follow::repeat_certificates::candidate;
use crate::grammar::follow::repeat_certificates::lemma_identity_work;
use crate::grammar::follow::repeat_certificates::lemma_lookup_work;

pub(super) struct GraphRepeatContext<'g, 'p> {
    pub grammar: &'g Grammar,
    pub owner: &'g str,
    pub certificates: &'p mut RepeatCertificates,
    pub remaining: &'p mut usize,
    pub exact: Option<&'p AdjacentWorkObservation>,
}

#[cfg(test)]
mod tests;

fn pay(
    remaining: &mut usize,
    ledger: &mut usize,
    exact: Option<&AdjacentWorkObservation>,
    category: AdjacentCharge,
) -> Result<(), String> {
    observed_analysis_charge(remaining, exact, category)?;
    // The original 10,000-unit pool bounds this exact ledger.
    *ledger += 1;
    Ok(())
}

impl GraphRepeatContext<'_, '_> {
    fn bind_nonnullable(
        &mut self,
        expr: &Expr,
        path: &str,
        key: &str,
        callee: &str,
        binding: GraphBinding,
    ) -> Result<(), String> {
        // The old/default collector has no such table and no new fees.
        if self.certificates.nonnullable.is_empty() {
            return Ok(());
        }
        let key_bytes = key.len();
        let levels = usize::BITS as usize
            - self.certificates.nonnullable.len().max(1).leading_zeros() as usize;
        let units = levels
            .checked_mul(2)
            .and_then(|n| n.checked_mul(1 + key_bytes.div_ceil(64)))
            .and_then(|n| n.checked_add(1))
            .ok_or("Nonnullable binding lookup overflow")?;
        for _ in 0..units {
            pay(
                self.remaining,
                &mut self.certificates.binding_work,
                self.exact,
                AdjacentCharge::RepeatBinding,
            )?;
        }
        let Some(proof) = self.certificates.nonnullable.get_mut(key) else {
            return Ok(());
        };
        pay(
            self.remaining,
            &mut self.certificates.binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        for _ in 0..lemma_lookup_work(self.grammar.declaration_sources.len(), self.owner.len())? {
            pay(
                self.remaining,
                &mut self.certificates.binding_work,
                self.exact,
                AdjacentCharge::RepeatBinding,
            )?;
        }
        let source = self
            .grammar
            .declaration_sources
            .get(self.owner)
            .ok_or("Missing nonnullable declaration")?;
        for _ in 0..lemma_identity_work(&source.source, &proof.source)?
            + 1
            + (self.owner.len() + proof.owner.len()).div_ceil(64)
        {
            pay(
                self.remaining,
                &mut self.certificates.binding_work,
                self.exact,
                AdjacentCharge::RepeatBinding,
            )?;
        }
        if self.owner != proof.owner
            || source.source != proof.source
            || (source.start..source.end) != proof.declaration
        {
            return Err("Nonnullable occurrence source declaration mismatch".into());
        }
        pay(
            self.remaining,
            &mut self.certificates.binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        for _ in 0..lemma_lookup_work(self.grammar.declaration_sources.len(), callee.len())? {
            pay(
                self.remaining,
                &mut self.certificates.binding_work,
                self.exact,
                AdjacentCharge::RepeatBinding,
            )?;
        }
        let actual = self
            .grammar
            .declaration_sources
            .get(callee)
            .ok_or("Missing nonnullable callee source")?;
        for _ in 0..lemma_identity_work(&actual.source, &proof.callee_source)?
            + 1
            + (callee.len() + proof.callee.len()).div_ceil(64)
        {
            pay(
                self.remaining,
                &mut self.certificates.binding_work,
                self.exact,
                AdjacentCharge::RepeatBinding,
            )?;
        }
        if callee != proof.callee || actual.source != proof.callee_source {
            return Err("Nonnullable occurrence callee source mismatch".into());
        }
        pay(
            self.remaining,
            &mut self.certificates.binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        let Kind::SyntacticPredicate(value) = &expr.kind else {
            return Err("Nonnullable occurrence lost Full operand".into());
        };
        for _ in 0..1 + (path.len() + proof.path.len()).div_ceil(64) {
            pay(
                self.remaining,
                &mut self.certificates.binding_work,
                self.exact,
                AdjacentCharge::RepeatBinding,
            )?;
        }
        if path != proof.path
            || expr.original_range.as_ref() != Some(&proof.occurrence)
            || value.original_range.as_ref() != Some(&proof.operand)
            || proof.graph.is_some()
        {
            return Err("Nonnullable occurrence source or graph binding mismatch".into());
        }
        proof.graph = Some(binding);
        Ok(())
    }
    pub(super) fn bind(
        &mut self,
        expr: &Expr,
        path: &str,
        binding: GraphBinding,
        continuation: Option<(&[Expr], usize, &str)>,
    ) -> Result<(), String> {
        let Some(callee) = candidate(expr) else {
            return Ok(());
        };
        // Preserve the existing candidate gate/payment/key allocation once.
        // Both proof maps borrow this exact key, including absent candidates.
        pay(
            self.remaining,
            &mut self.certificates.binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        let key = format!("{path}/repeat");
        self.bind_nonnullable(expr, path, &key, callee, binding)?;
        let RepeatCertificates {
            entries,
            bound,
            binding_work,
            ..
        } = self.certificates;
        let Some(proof) = entries.get_mut(&key) else {
            return Ok(());
        };
        pay(
            self.remaining,
            binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        let source = self
            .grammar
            .declaration_sources
            .get(self.owner)
            .ok_or("Missing certified repeat declaration")?;
        if self.owner != proof.owner
            || source.source != proof.source
            || (source.start..source.end) != proof.declaration
        {
            return Err("Certified repeat belongs to a different source declaration".into());
        }
        pay(
            self.remaining,
            binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        let callee_source = self
            .grammar
            .declaration_sources
            .get(callee)
            .ok_or("Missing certified repeat callee source")?;
        if callee != proof.callee || callee_source.source != proof.callee_source {
            return Err("Certified repeat callee source mismatch".into());
        }
        pay(
            self.remaining,
            binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        let Kind::SyntacticPredicate(value) = &expr.kind else {
            return Err("Certified repeat lost its Full operand".into());
        };
        if path != proof.path
            || expr.original_range.as_ref() != Some(&proof.occurrence)
            || value.original_range.as_ref() != Some(&proof.operand)
        {
            return Err("Certified repeat source occurrence mismatch".into());
        }
        let (items, next, parent) =
            continuation.ok_or("Certified repeat has no actual local suffix")?;
        pay(
            self.remaining,
            binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        let suffix = items
            .get(next)
            .ok_or("Certified repeat lost its local suffix")?;
        let Kind::Literal(literal) = &suffix.kind else {
            return Err("Certified repeat suffix is not a literal".into());
        };
        if suffix.cardinality != '1'
            || suffix.first_set_predicated
            || literal != &proof.literal
            || suffix.original_range.as_ref() != Some(&proof.suffix)
        {
            return Err("Certified repeat literal source mismatch".into());
        }
        pay(
            self.remaining,
            binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        if format!("{parent}/s{next}") != proof.suffix_path || proof.graph.is_some() {
            return Err("Certified repeat suffix occurrence or binding mismatch".into());
        }
        pay(
            self.remaining,
            binding_work,
            self.exact,
            AdjacentCharge::RepeatBinding,
        )?;
        proof.graph = Some(binding);
        *bound += 1;
        Ok(())
    }
}

impl ParserPrediction {
    /// Reuse only the exact nonnullable lemma already established by mandatory
    /// canonical FOLLOW FIRST and bound to this live source/graph. The existing
    /// verifier remains authority for every source and graph condition.
    pub(super) fn runtime_repeat_certificate_is_nonnullable(
        &self,
        path: &str,
        decision: &Decision,
        grammar: &Grammar,
        certificates: &mut RepeatCertificates,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        // Pay the new bounded ordered-map probe/key reads before looking. The
        // second, existing verifier lookup and all twelve old charges remain.
        let levels =
            usize::BITS as usize - certificates.entries.len().max(1).leading_zeros() as usize;
        let units = path
            .len()
            .div_ceil(64)
            .checked_add(1)
            .and_then(|key_units| levels.checked_mul(2)?.checked_mul(key_units))
            .and_then(|n| n.checked_add(1))
            .ok_or("Runtime certificate lookup size overflow")?;
        for _ in 0..units {
            pay(
                remaining,
                &mut certificates.consumption_work,
                None,
                AdjacentCharge::RepeatConsumption,
            )?;
        }
        if certificates
            .entries
            .get(path)
            .is_none_or(|proof| proof.graph.is_none())
        {
            return self.runtime_occurrence_is_nonnullable(
                path,
                decision,
                grammar,
                certificates,
                remaining,
            );
        }
        // A bound mismatch is an error. Never turn it into absence or refund.
        self.certified_repeat_is_nonconflicting(path, decision, grammar, certificates, remaining)
    }
    // This verifier proves no suffix, disjointness, or nonconflict fact. The
    // collector still runs every independent source/hidden/completion check.
    fn runtime_occurrence_is_nonnullable(
        &self,
        path: &str,
        decision: &Decision,
        grammar: &Grammar,
        certificates: &mut RepeatCertificates,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        if certificates.nonnullable.is_empty() {
            return Ok(false);
        }
        let levels =
            usize::BITS as usize - certificates.nonnullable.len().max(1).leading_zeros() as usize;
        let units = levels
            .checked_mul(2)
            .and_then(|n| n.checked_mul(1 + path.len().div_ceil(64)))
            .and_then(|n| n.checked_add(1))
            .ok_or("Nonnullable consumption lookup overflow")?;
        for _ in 0..units {
            pay(
                remaining,
                &mut certificates.consumption_work,
                None,
                AdjacentCharge::RepeatConsumption,
            )?;
        }
        let Some(proof) = certificates.nonnullable.get(path) else {
            return Ok(false);
        };
        let Some(binding) = proof.graph else {
            return Ok(false);
        };
        let ledger = &mut certificates.consumption_work;
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        for _ in 0..lemma_lookup_work(grammar.declaration_sources.len(), proof.owner.len())? {
            pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        }
        let source = grammar
            .declaration_sources
            .get(&proof.owner)
            .ok_or("Missing consumed nonnullable source")?;
        for _ in 0..lemma_identity_work(&source.source, &proof.source)? {
            pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        }
        if source.source != proof.source || (source.start..source.end) != proof.declaration {
            return Err("Consumed nonnullable declaration mismatch".into());
        }
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        for _ in 0..lemma_lookup_work(grammar.declaration_sources.len(), proof.callee.len())? {
            pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        }
        let callee_source = grammar
            .declaration_sources
            .get(&proof.callee)
            .ok_or("Missing consumed nonnullable callee source")?;
        for _ in 0..lemma_identity_work(&callee_source.source, &proof.callee_source)? {
            pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        }
        if callee_source.source != proof.callee_source {
            return Err("Consumed nonnullable callee source mismatch".into());
        }
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        for _ in 0..lemma_lookup_work(self.decisions.len(), proof.recovery_key.len())? {
            pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        }
        let occurrence = self
            .decisions
            .get(&proof.recovery_key)
            .ok_or("Missing nonnullable source occurrence")?;
        if occurrence.entries.as_slice() != [binding.start, binding.end]
            || occurrence.exit.is_some()
            || decision.entries.as_slice() != [binding.body, binding.end]
            || decision.exit != Some(1)
        {
            return Err("Nonnullable source occurrence or decision mismatch".into());
        }
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        let start = self
            .states
            .get(binding.start)
            .ok_or("Dangling nonnullable start")?;
        if !matches!(start.edges.as_slice(), [Edge::Epsilon(body),Edge::Epsilon(end)]
            if *body==binding.body && *end==binding.end && !start.rule_stop && !start.accepting)
        {
            return Err("Nonnullable repeat start mismatch".into());
        }
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        let entry = self
            .states
            .get(binding.body)
            .ok_or("Dangling nonnullable Full entry")?;
        let [Edge::Full { target, stop }] = entry.edges.as_slice() else {
            return Err("Nonnullable repeat lost Full edge".into());
        };
        if *stop != binding.stop || entry.rule_stop || entry.accepting {
            return Err("Nonnullable Full boundary mismatch".into());
        }
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        for _ in 0..lemma_lookup_work(self.rules.len(), proof.callee.len())? {
            pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        }
        let &(callee, _) = self
            .rules
            .get(&proof.callee)
            .ok_or("Missing nonnullable callee graph")?;
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        let operand = self
            .states
            .get(*target)
            .ok_or("Dangling nonnullable operand")?;
        if !matches!(operand.edges.as_slice(), [Edge::Call {target,follow}]
            if *target==callee && *follow==*stop && !operand.rule_stop && !operand.accepting)
        {
            return Err("Nonnullable operand Call or return mismatch".into());
        }
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        let done = self.states.get(*stop).ok_or("Dangling nonnullable stop")?;
        if !matches!(done.edges.as_slice(), [Edge::Epsilon(back)]
            if *back==binding.start && !done.rule_stop && !done.accepting && done.follow.is_empty())
        {
            return Err("Nonnullable loop boundary mismatch".into());
        }
        pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
        let end = self
            .states
            .get(binding.end)
            .ok_or("Dangling nonnullable exit")?;
        if end.accepting {
            return Err("Nonnullable repeat exit mismatch".into());
        }
        if end.rule_stop {
            // A whole-rule repeat legitimately exits at its owning stop.
            // This is not a literal suffix/nonconflict proof. Caller follow
            // metadata on that stop remains valid and is not required empty.
            for _ in 0..lemma_lookup_work(self.rules.len(), proof.owner.len())? {
                pay(remaining, ledger, None, AdjacentCharge::RepeatConsumption)?;
            }
            let &(_, owner_stop) = self
                .rules
                .get(&proof.owner)
                .ok_or("Missing nonnullable owning rule graph")?;
            if binding.end != owner_stop || !end.edges.is_empty() {
                return Err("Nonnullable repeat exit mismatch".into());
            }
        }
        Ok(true)
    }
    pub(super) fn certified_repeat_is_nonconflicting(
        &self,
        path: &str,
        decision: &Decision,
        grammar: &Grammar,
        certificates: &mut RepeatCertificates,
        remaining: &mut usize,
    ) -> Result<bool, String> {
        self.certified_repeat_is_nonconflicting_observed(
            path,
            decision,
            grammar,
            certificates,
            remaining,
            None,
        )
    }
    pub(super) fn certified_repeat_is_nonconflicting_observed(
        &self,
        path: &str,
        decision: &Decision,
        grammar: &Grammar,
        certificates: &mut RepeatCertificates,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<bool, String> {
        if !path.ends_with("/repeat") {
            return Ok(false);
        }
        // Every already encountered guarded repeat pays this lookup, including
        // absent/ineligible source proofs. There is no free absence cache.
        let RepeatCertificates {
            entries,
            consumed,
            consumption_work,
            ..
        } = certificates;
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let Some(proof) = entries.get(path) else {
            return Ok(false);
        };
        let binding = proof
            .graph
            .ok_or("Source repeat certificate was not graph-bound")?;
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let source = grammar
            .declaration_sources
            .get(&proof.owner)
            .ok_or("Missing consumed repeat declaration")?;
        if source.source != proof.source || (source.start..source.end) != proof.declaration {
            return Err("Consumed repeat source declaration mismatch".into());
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let callee_source = grammar
            .declaration_sources
            .get(&proof.callee)
            .ok_or("Missing consumed repeat callee source")?;
        if callee_source.source != proof.callee_source {
            return Err("Consumed repeat callee source mismatch".into());
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let occurrence = self
            .decisions
            .get(&format!("{}/recovery", proof.path))
            .ok_or("Missing certified repeat recovery occurrence")?;
        if occurrence.entries.as_slice() != [binding.start, binding.end]
            || occurrence.exit.is_some()
            || decision.entries.as_slice() != [binding.body, binding.end]
            || decision.exit != Some(1)
        {
            return Err("Certified repeat decision or occurrence mismatch".into());
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let start = self
            .states
            .get(binding.start)
            .ok_or("Dangling certified repeat start")?;
        match start.edges.as_slice() {
            [Edge::Epsilon(body), Edge::Epsilon(end)]
                if *body == binding.body
                    && *end == binding.end
                    && !start.rule_stop
                    && !start.accepting => {}
            _ => return Err("Certified repeat start enter or exit mismatch".into()),
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let suffix = self
            .decisions
            .get(&format!("{}/recovery", proof.suffix_path))
            .ok_or("Missing certified literal suffix occurrence")?;
        let [suffix_start, suffix_end] = suffix.entries.as_slice() else {
            return Err("Invalid certified literal suffix boundary".into());
        };
        if *suffix_start != binding.end || suffix.exit.is_some() {
            return Err("Certified literal is not the actual repeat exit".into());
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let entry = self
            .states
            .get(binding.body)
            .ok_or("Dangling certified repeat guard entry")?;
        let [Edge::Full { target, stop }] = entry.edges.as_slice() else {
            return Err("Certified repeat has no exact Full edge".into());
        };
        if *stop != binding.stop || entry.rule_stop || entry.accepting {
            return Err("Certified repeat Full boundary mismatch".into());
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let &(callee, _) = self
            .rules
            .get(&proof.callee)
            .ok_or("Missing certified repeat callee graph")?;
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let operand = self
            .states
            .get(*target)
            .ok_or("Dangling certified repeat operand")?;
        match operand.edges.as_slice() {
            [Edge::Call { target, follow }]
                if *target == callee
                    && *follow == *stop
                    && !operand.rule_stop
                    && !operand.accepting => {}
            _ => return Err("Certified repeat operand Call or return mismatch".into()),
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let done = self
            .states
            .get(*stop)
            .ok_or("Dangling certified repeat stop")?;
        match done.edges.as_slice() {
            [Edge::Epsilon(target)]
                if *target == binding.start
                    && !done.rule_stop
                    && !done.accepting
                    && done.follow.is_empty() => {}
            _ => return Err("Certified repeat has no exact loop-back boundary".into()),
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        let exit = self
            .states
            .get(binding.end)
            .ok_or("Dangling certified repeat literal exit")?;
        match exit.edges.as_slice() {
            [Edge::Token(PredictionSymbol::Keyword(literal), target)]
                if literal == &proof.literal
                    && target == suffix_end
                    && !exit.rule_stop
                    && !exit.accepting => {}
            _ => return Err("Certified repeat literal exit graph mismatch".into()),
        }
        pay(
            remaining,
            consumption_work,
            exact,
            AdjacentCharge::RepeatConsumption,
        )?;
        *consumed += 1;
        Ok(true)
    }
}
