//! Invocation-local reuse of completed callee witnesses on one immutable graph.
//! No caller continuation, whole decision, error, or graph identity is cached here.
use super::analysis_charge;
use std::collections::BTreeMap;

/// Called only after the original exact-key lookup missed. A true witness at
/// a smaller bound is still true at this bound; a completed false witness at a
/// larger bound is still false here. The converse implications are invalid.
pub(super) fn lookup_completed(
    witnesses: &BTreeMap<(usize, usize), bool>,
    entry: usize,
    bound: usize,
    remaining: &mut usize,
) -> Result<Option<bool>, String> {
    // Charge before constructing the actual lazy range iterator. A different
    // graph entry cannot contribute a fact, even if source labels coincide.
    analysis_charge(remaining)?;
    let mut facts = witnesses.range((entry, 0)..=(entry, usize::MAX));
    loop {
        // Includes an exhausted next() attempt; no fact is read before payment.
        analysis_charge(remaining)?;
        let Some(((_, completed_bound), value)) = facts.next() else {
            return Ok(None);
        };
        if (*value && *completed_bound <= bound) || (!*value && *completed_bound >= bound) {
            return Ok(Some(*value));
        }
    }
}

#[cfg(test)]
#[path = "bounded_witness_tests.rs"]
mod tests;
