//! Handwritten sufficient proof by erasing shared token symbols.
//! FIRST summaries retain exact call continuations and local return barriers.
use super::{Edge, ParserPrediction, PredictionSymbol, analysis_charge};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[path = "certificates.rs"]
mod certificates;
const DISCOVERY_INCONCLUSIVE: &str = "Projected initial discovery is inconclusive";
fn charge_projection(remaining: &mut usize, discovery: &mut Option<usize>) -> Result<(), String> {
    if *discovery == Some(0) {
        return Err(DISCOVERY_INCONCLUSIVE.into());
    }
    analysis_charge(remaining)?;
    if let Some(remaining) = discovery {
        *remaining -= 1;
    }
    Ok(())
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Summary {
    tokens: u128,
    returned: bool,
}
impl Summary {
    fn include(&mut self, other: Self) {
        self.tokens |= other.tokens;
        self.returned |= other.returned;
    }
}
#[derive(Default)]
struct Entry {
    summary: Summary,
    depth: u8,
    root: bool,
    dependents: BTreeSet<usize>,
}
struct Projection<'program, 'budget> {
    program: &'program ParserPrediction,
    remaining: &'budget mut usize,
    entries: BTreeMap<usize, Entry>,
    queued: BTreeSet<usize>,
    pending: VecDeque<usize>,
    symbols: BTreeMap<PredictionSymbol, usize>,
    users: Vec<BTreeSet<usize>>,
    skipped: u128,
    depth_first: bool,
    visiting: BTreeSet<usize>,
    discovery_remaining: Option<usize>,
}
/// False is inconclusive, never ambiguity or permission to skip another proof.
pub(super) fn prove(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
) -> Result<bool, String> {
    prove_with_initial_discovery(program, roots, remaining, None)
}
/// Distinguish a spent discovery carveout from another insufficient projection.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Discovery {
    Proven,
    Inconclusive,
    Exhausted,
}
/// Partial FIRST summaries are inconclusive; spent discovery is never refunded.
pub(super) fn prove_with_bounded_discovery(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
    allowance: usize,
) -> Result<Discovery, String> {
    match prove_with_initial_discovery(program, roots, remaining, Some(allowance)) {
        Err(message) if message == DISCOVERY_INCONCLUSIVE => Ok(Discovery::Exhausted),
        Ok(true) => Ok(Discovery::Proven),
        Ok(false) => Ok(Discovery::Inconclusive),
        Err(message) => Err(message),
    }
}

fn prove_with_initial_discovery(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
    discovery: Option<usize>,
) -> Result<bool, String> {
    let mut proof = Projection::new(program, remaining, false);
    proof.discovery_remaining = discovery;
    for &root in roots {
        proof.ensure(root, 0);
        proof
            .entries
            .get_mut(&root)
            .ok_or("Missing projected prediction root")?
            .root = true;
    }
    loop {
        while let Some(state) = proof.pending.pop_front() {
            proof.charge()?;
            proof.queued.remove(&state);
            if !proof.evaluate(state)? {
                return Ok(false);
            }
        }
        // Only a complete initial fixed point releases the discovery cap.
        // Subsequent erasure rounds retain the same attempt's remaining work.
        proof.discovery_remaining = None;
        let mut seen = 0;
        let mut conflict = 0;
        for root in roots {
            analysis_charge(proof.remaining)?;
            let summary = proof
                .entries
                .get(root)
                .ok_or("Missing projected prediction root")?
                .summary;
            let tokens = summary.tokens & !proof.skipped;
            // A return/acceptance before a significant token cannot establish
            // prefix disjointness. Empty/unproductive roots are inconclusive too.
            if summary.returned || tokens == 0 {
                return Ok(false);
            }
            conflict |= seen & tokens;
            seen |= tokens;
        }
        if conflict == 0 {
            return Ok(true);
        }
        proof.skipped |= conflict;
        for (index, users) in proof.users.iter().enumerate() {
            analysis_charge(proof.remaining)?;
            if conflict & (1u128 << index) != 0 {
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
pub(super) fn prove_certificates(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
) -> Result<bool, String> {
    match certificates::prove(program, roots, remaining) {
        Err(message) if message == DISCOVERY_INCONCLUSIVE => Ok(false),
        result => result,
    }
}
impl Projection<'_, '_> {
    fn new<'program, 'budget>(
        program: &'program ParserPrediction,
        remaining: &'budget mut usize,
        depth_first: bool,
    ) -> Projection<'program, 'budget> {
        Projection {
            program,
            remaining,
            entries: BTreeMap::new(),
            queued: BTreeSet::new(),
            pending: VecDeque::new(),
            symbols: BTreeMap::new(),
            users: Vec::new(),
            skipped: 0,
            depth_first,
            visiting: BTreeSet::new(),
            discovery_remaining: depth_first.then_some(1000),
        }
    }
    fn charge(&mut self) -> Result<(), String> {
        charge_projection(self.remaining, &mut self.discovery_remaining)
    }
    fn visit(&mut self, state: usize) -> Result<bool, String> {
        // This bounds the handwritten evaluator's Rust stack independently of
        // the grammar's pending-call depth. Inconclusive retains fallback work.
        if self.visiting.len() == 128 {
            return Ok(false);
        }
        self.charge()?;
        self.queued.remove(&state);
        self.visiting.insert(state);
        let result = self.evaluate(state);
        self.visiting.remove(&state);
        result
    }
    fn ensure(&mut self, state: usize, depth: u8) {
        let changed = match self.entries.entry(state) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Entry {
                    depth,
                    ..Entry::default()
                });
                true
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let deeper = depth > entry.get().depth;
                if deeper {
                    entry.get_mut().depth = depth;
                }
                deeper
            }
        };
        if changed && self.queued.insert(state) {
            self.pending.push_back(state);
        }
    }
    fn dependency(
        &mut self,
        owner: usize,
        state: usize,
        depth: u8,
    ) -> Result<Option<Summary>, String> {
        self.charge()?;
        if depth > 64 {
            return Ok(None);
        }
        self.ensure(state, depth);
        self.entries
            .get_mut(&state)
            .ok_or("Missing projected prediction dependency")?
            .dependents
            .insert(owner);
        if self.depth_first
            && self.queued.contains(&state)
            && !self.visiting.contains(&state)
            && !self.visit(state)?
        {
            return Ok(None);
        }
        Ok(Some(
            self.entries
                .get(&state)
                .ok_or("Missing projected prediction dependency")?
                .summary,
        ))
    }
    fn evaluate(&mut self, state: usize) -> Result<bool, String> {
        let program = self.program;
        let Some(node) = program.states.get(state) else {
            return Ok(false);
        };
        let entry = self
            .entries
            .get(&state)
            .ok_or("Missing projected prediction state")?;
        let (old, depth, root) = (entry.summary, entry.depth, entry.root);
        let mut next = old;
        next.returned |= node.rule_stop || node.accepting;
        if !node.rule_stop {
            for edge in &node.edges {
                self.charge()?;
                match edge {
                    Edge::Epsilon(target)
                    | Edge::Action(target)
                    | Edge::First { target, .. }
                    | Edge::Full { target, .. } => {
                        if let Edge::First { path, .. } = edge
                            && !program.first_guards.contains_key(path)
                        {
                            return Ok(false);
                        }
                        let Some(summary) = self.dependency(state, *target, depth)? else {
                            return Ok(false);
                        };
                        next.include(summary);
                    }
                    Edge::Token(symbol, target) => {
                        let index = if let Some(index) = self.symbols.get(symbol) {
                            *index
                        } else {
                            // A bounded machine-word alphabet. Larger analyses
                            // retain the original proof and its remaining budget.
                            if self.symbols.len() == 128 {
                                return Ok(false);
                            }
                            let index = self.symbols.len();
                            self.symbols.insert(symbol.clone(), index);
                            self.users.push(BTreeSet::new());
                            index
                        };
                        let bit = 1u128 << index;
                        next.tokens |= bit;
                        self.users[index].insert(state);
                        if self.skipped & bit != 0 {
                            let Some(summary) = self.dependency(state, *target, depth)? else {
                                return Ok(false);
                            };
                            next.include(summary);
                        }
                    }
                    Edge::Call { target, follow } => {
                        let Some(callee) = self.dependency(state, *target, depth + 1)? else {
                            return Ok(false);
                        };
                        next.tokens |= callee.tokens;
                        if callee.returned {
                            let Some(suffix) = self.dependency(state, *follow, depth)? else {
                                return Ok(false);
                            };
                            next.include(suffix);
                        }
                    }
                }
            }
        }
        // A root return is a permanent counterexample to this sufficient
        // projection. Do not spend the fallback budget finishing unrelated work.
        if root && next.returned {
            return Ok(false);
        }
        if next != old {
            let entry = self
                .entries
                .get_mut(&state)
                .ok_or("Missing projected prediction update")?;
            entry.summary = next;
            for &dependent in &entry.dependents {
                charge_projection(self.remaining, &mut self.discovery_remaining)?;
                if self.queued.insert(dependent) {
                    self.pending.push_back(dependent);
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
