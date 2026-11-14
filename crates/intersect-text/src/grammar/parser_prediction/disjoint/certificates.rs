//! Handwritten n-ary search policy over the shared significant-token equations.
//! A settled pair certificate remains valid after later erasure changes.
use super::{BTreeSet, ParserPrediction, Projection, analysis_charge};

pub(super) fn prove(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
) -> Result<bool, String> {
    let mut proof = Projection::new(program, remaining, true);
    let mut pairs = BTreeSet::new();
    for (left, &root) in roots.iter().enumerate() {
        proof.ensure(root, 0);
        proof
            .entries
            .get_mut(&root)
            .ok_or("Missing certificate root")?
            .root = true;
        for right in left + 1..roots.len() {
            analysis_charge(proof.remaining)?;
            pairs.insert((left, right));
        }
    }
    if pairs.is_empty() {
        return Ok(false);
    }
    let mut active: BTreeSet<_> = (0..roots.len()).collect();
    loop {
        while let Some(state) = proof.pending.pop_front() {
            // Dependency visits leave stale queue entries. A queued state alone
            // owns a pending evaluation; a pop never grants another allowance.
            if proof.queued.contains(&state) && !proof.visit(state)? {
                return Ok(false);
            }
        }
        // A wide initial FIRST discovery is unlikely to benefit from erasure.
        // Its bounded work remains spent; preserve room for the original proof.
        proof.discovery_remaining = None;
        for &index in &active {
            analysis_charge(proof.remaining)?;
            let summary = proof
                .entries
                .get(&roots[index])
                .ok_or("Missing certificate summary")?
                .summary;
            if summary.returned || summary.tokens & !proof.skipped == 0 {
                return Ok(false);
            }
        }
        let mut unresolved = BTreeSet::new();
        for &(left, right) in &pairs {
            analysis_charge(proof.remaining)?;
            let left_summary = proof
                .entries
                .get(&roots[left])
                .ok_or("Missing left certificate summary")?
                .summary;
            let right_summary = proof
                .entries
                .get(&roots[right])
                .ok_or("Missing right certificate summary")?
                .summary;
            if left_summary.tokens & right_summary.tokens & !proof.skipped != 0 {
                // Keep alternative indices, including duplicate entry states.
                // Distinct labels with identical languages must never disappear.
                unresolved.insert((left, right));
            }
        }
        pairs = unresolved;
        if pairs.is_empty() {
            return Ok(true);
        }
        active = pairs
            .iter()
            .flat_map(|&(left, right)| [left, right])
            .collect();
        let mut common = u128::MAX;
        let mut conflict = 0;
        let mut active_roots = BTreeSet::new();
        for &index in &active {
            analysis_charge(proof.remaining)?;
            active_roots.insert(roots[index]);
            common &= proof
                .entries
                .get(&roots[index])
                .ok_or("Missing active certificate summary")?
                .summary
                .tokens
                & !proof.skipped;
        }
        for &(left, right) in &pairs {
            analysis_charge(proof.remaining)?;
            conflict |= proof
                .entries
                .get(&roots[left])
                .ok_or("Missing left conflict summary")?
                .summary
                .tokens
                & proof
                    .entries
                    .get(&roots[right])
                    .ok_or("Missing right conflict summary")?
                    .summary
                    .tokens
                & !proof.skipped;
        }
        // Only still-competing roots are return barriers in the next round.
        for root in roots {
            analysis_charge(proof.remaining)?;
            proof
                .entries
                .get_mut(root)
                .ok_or("Missing retired certificate root")?
                .root = active_roots.contains(root);
        }
        let erased = if common != 0 { common } else { conflict };
        proof.skipped |= erased;
        for (index, users) in proof.users.iter().enumerate() {
            analysis_charge(proof.remaining)?;
            if erased & (1u128 << index) != 0 {
                for &state in users {
                    analysis_charge(proof.remaining)?;
                    if proof.queued.insert(state) {
                        proof.pending.push_back(state);
                    }
                }
            }
        }
    }
}
