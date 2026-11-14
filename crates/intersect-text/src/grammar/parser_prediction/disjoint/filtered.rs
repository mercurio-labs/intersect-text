//! Handwritten sufficient syntax proof. FIRST signatures only prune exact closures.
//! Weighted returns preserve pending-call depth and actual caller continuations.
use super::{Edge, ParserPrediction, PredictionSymbol, analysis_charge};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const INCONCLUSIVE: &str = "Filtered prediction summary is inconclusive";
const DEPTH: usize = 64;
const EVALUATOR: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Frame {
    follow: usize,
    weight: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Row {
    state: usize,
    calls: Vec<Frame>,
}
impl Row {
    fn depth(&self) -> usize {
        self.calls.iter().map(|frame| frame.weight).sum()
    }
}
#[derive(Clone, Default)]
struct First {
    tokens: u128,
    returned: BTreeSet<usize>,
    depth: usize,
    accepting: bool,
    unknown_guard: bool,
}
impl First {
    fn include(&mut self, other: Self) {
        self.tokens |= other.tokens;
        self.returned.extend(other.returned);
        self.depth = self.depth.max(other.depth);
        self.accepting |= other.accepting;
        self.unknown_guard |= other.unknown_guard;
    }
}
#[derive(Clone, Copy, Default)]
struct ContextFirst {
    tokens: u128,
    accepting: bool,
}
impl ContextFirst {
    fn include(&mut self, other: Self) {
        self.tokens |= other.tokens;
        self.accepting |= other.accepting;
    }
}
#[derive(Clone, Default)]
struct Closure {
    accepting: bool,
    tokens: BTreeMap<PredictionSymbol, BTreeSet<Row>>,
}
type Group = BTreeSet<Row>;
struct Filtered<'program, 'budget> {
    program: &'program ParserPrediction,
    remaining: &'budget mut usize,
    symbols: BTreeMap<PredictionSymbol, usize>,
    first: BTreeMap<usize, First>,
    visiting: BTreeSet<usize>,
    contexts: BTreeMap<Row, ContextFirst>,
    context_visiting: BTreeSet<Row>,
    tails: BTreeMap<usize, Option<usize>>,
    closures: BTreeMap<(Row, u128), Closure>,
}
/// False is inconclusive. Incomplete summaries can never authorize admission.
pub(super) fn prove(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
) -> Result<bool, String> {
    prove_mode(program, roots, remaining, false)
}
pub(super) fn prove_with_callee_fence(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
) -> Result<bool, String> {
    prove_mode(program, roots, remaining, true)
}
fn prove_mode(
    program: &ParserPrediction,
    roots: &[usize],
    remaining: &mut usize,
    fence: bool,
) -> Result<bool, String> {
    if roots.len() != 2 {
        return Ok(false);
    }
    let mut proof = Filtered::new(program, remaining);
    let result = if fence {
        proof
            .callee_fence(roots)
            .and_then(|proved| if proved { Ok(true) } else { proof.run(roots) })
    } else {
        proof.run(roots)
    };
    match result {
        Err(message) if message == INCONCLUSIVE => Ok(false),
        result => result,
    }
}
// A forced common call and its exact continuation after the call returns.
struct Boundary {
    target: usize,
    after: Row,
    outer_depth: usize,
}
impl Filtered<'_, '_> {
    fn new<'program, 'budget>(
        program: &'program ParserPrediction,
        remaining: &'budget mut usize,
    ) -> Filtered<'program, 'budget> {
        Filtered {
            program,
            remaining,
            symbols: BTreeMap::new(),
            first: BTreeMap::new(),
            visiting: BTreeSet::new(),
            contexts: BTreeMap::new(),
            context_visiting: BTreeSet::new(),
            tails: BTreeMap::new(),
            closures: BTreeMap::new(),
        }
    }

    fn leading_boundaries(&mut self, state: usize) -> Result<Vec<Boundary>, String> {
        let mut row = Row {
            state,
            calls: Vec::new(),
        };
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        while seen.insert(row.clone()) {
            self.charge()?;
            let node = self
                .program
                .states
                .get(row.state)
                .ok_or("Dangling callee boundary")?;
            if node.accepting || node.rule_stop {
                break;
            }
            match node.edges.as_slice() {
                [Edge::Epsilon(state) | Edge::Action(state) | Edge::Full { target: state, .. }] => {
                    row.state = *state
                }
                [Edge::First { path, target }] if self.program.first_guards.contains_key(path) => {
                    row.state = *target
                }
                [Edge::First { .. }] => {
                    return Err("Guarded shared-prefix proof remains unsupported".into());
                }
                [Edge::Call { target, follow }] => {
                    let (target, follow) = (*target, *follow);
                    result.push(Boundary {
                        target,
                        after: Row {
                            state: follow,
                            calls: row.calls.clone(),
                        },
                        outer_depth: row.depth(),
                    });
                    row = self.call(&row, target, follow)?;
                }
                _ => break,
            }
        }
        Ok(result)
    }
    fn callee_alphabet(
        &mut self,
        state: usize,
        outer_depth: usize,
    ) -> Result<Option<u128>, String> {
        let mut pending = VecDeque::from([Row {
            state,
            calls: Vec::new(),
        }]);
        let mut seen = BTreeSet::new();
        let mut alphabet = 0;
        let mut returned = false;
        while let Some(row) = pending.pop_front() {
            self.charge()?;
            if !seen.insert(row.clone()) {
                continue;
            }
            let node = self
                .program
                .states
                .get(row.state)
                .ok_or("Dangling callee alphabet state")?;
            if node.accepting || row.depth() + outer_depth > DEPTH {
                return Ok(None);
            }
            if node.rule_stop {
                if let Some((last, rest)) = row.calls.split_last() {
                    pending.push_back(Row {
                        state: last.follow,
                        calls: rest.to_vec(),
                    });
                } else {
                    returned = true;
                }
                continue;
            }
            for edge in node.edges.clone() {
                self.charge()?;
                let next = match edge {
                    Edge::Epsilon(state)
                    | Edge::Action(state)
                    | Edge::Full { target: state, .. } => Row {
                        state,
                        ..row.clone()
                    },
                    Edge::First { path, target } => {
                        if !self.program.first_guards.contains_key(&path) {
                            return Ok(None);
                        }
                        Row {
                            state: target,
                            ..row.clone()
                        }
                    }
                    Edge::Token(symbol, state) => {
                        alphabet |= self.signature(&symbol);
                        Row {
                            state,
                            ..row.clone()
                        }
                    }
                    Edge::Call { target, follow } => {
                        if row.depth() + outer_depth >= DEPTH {
                            return Ok(None);
                        }
                        self.call(&row, target, follow)?
                    }
                };
                pending.push_back(next);
            }
        }
        Ok(returned.then_some(alphabet))
    }
    fn callee_fence(&mut self, roots: &[usize]) -> Result<bool, String> {
        let right = self.leading_boundaries(roots[1])?;
        let left = self.leading_boundaries(roots[0])?;
        for right in &right {
            for left in &left {
                self.charge()?;
                if left.target != right.target {
                    continue;
                }
                let a = self.context_first(&right.after)?;
                let b = self.context_first(&left.after)?;
                if a.accepting
                    || b.accepting
                    || a.tokens == 0
                    || b.tokens == 0
                    || a.tokens & b.tokens != 0
                {
                    continue;
                }
                let outer_depth = 1 + left.outer_depth.max(right.outer_depth);
                if let Some(alphabet) = self.callee_alphabet(left.target, outer_depth)?
                    && alphabet & (a.tokens | b.tokens) == 0
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn charge(&mut self) -> Result<(), String> {
        analysis_charge(self.remaining)
    }
    fn signature(&mut self, symbol: &PredictionSymbol) -> u128 {
        if let Some(index) = self.symbols.get(symbol) {
            return 1u128 << index;
        }
        // Never change an assigned signature. Overflow is deliberately coarse;
        // equal original symbols always intersect, including after saturation.
        if self.symbols.len() == 127 {
            return 1;
        }
        let index = self.symbols.len() + 1;
        self.symbols.insert(symbol.clone(), index);
        1u128 << index
    }
    fn dependency(&mut self, state: usize) -> Result<First, String> {
        self.charge()?;
        self.local_first(state)
    }
    fn local_first(&mut self, state: usize) -> Result<First, String> {
        if let Some(result) = self.first.get(&state) {
            return Ok(result.clone());
        }
        if self.visiting.len() == EVALUATOR || !self.visiting.insert(state) {
            return Err(INCONCLUSIVE.into());
        }
        self.charge()?;
        let node = self
            .program
            .states
            .get(state)
            .ok_or("Dangling parser prediction state")?;
        let mut result = First {
            accepting: node.accepting,
            ..First::default()
        };
        if node.rule_stop {
            result.returned.insert(state);
        } else {
            for edge in node.edges.clone() {
                self.charge()?;
                match edge {
                    Edge::Token(symbol, _) => result.tokens |= self.signature(&symbol),
                    Edge::Call { target, follow } => {
                        let called = self.dependency(target)?;
                        result.tokens |= called.tokens;
                        result.depth = result.depth.max((called.depth + 1).min(DEPTH + 1));
                        result.accepting |= called.accepting;
                        result.unknown_guard |= called.unknown_guard;
                        if !called.returned.is_empty() {
                            result.include(self.dependency(follow)?);
                        }
                    }
                    Edge::Epsilon(target) | Edge::Action(target) | Edge::Full { target, .. } => {
                        result.include(self.dependency(target)?);
                    }
                    Edge::First { path, target } => {
                        result.unknown_guard |= !self.program.first_guards.contains_key(&path);
                        result.include(self.dependency(target)?);
                    }
                }
            }
        }
        self.visiting.remove(&state);
        self.first.insert(state, result.clone());
        Ok(result)
    }
    fn context_first(&mut self, row: &Row) -> Result<ContextFirst, String> {
        if let Some(result) = self.contexts.get(row) {
            return Ok(*result);
        }
        if self.context_visiting.len() == EVALUATOR || !self.context_visiting.insert(row.clone()) {
            return Err(INCONCLUSIVE.into());
        }
        let local = self.local_first(row.state)?;
        if local.unknown_guard {
            return Err("Guarded shared-prefix proof remains unsupported".into());
        }
        // Validate the unfiltered traversal before discarding any token path.
        if row.depth() + local.depth > DEPTH {
            return Err(INCONCLUSIVE.into());
        }
        let mut result = ContextFirst {
            tokens: local.tokens,
            accepting: local.accepting,
        };
        if !local.returned.is_empty() {
            self.charge()?;
            if let Some((last, rest)) = row.calls.split_last() {
                result.include(self.context_first(&Row {
                    state: last.follow,
                    calls: rest.to_vec(),
                })?);
            } else {
                for stop in local.returned {
                    let follow = self
                        .program
                        .states
                        .get(stop)
                        .ok_or("Dangling parser prediction return")?
                        .follow
                        .clone();
                    if follow.is_empty() {
                        return Err("Caller FOLLOW context required".into());
                    }
                    for state in follow {
                        self.charge()?;
                        result.include(self.context_first(&Row {
                            state,
                            calls: Vec::new(),
                        })?);
                    }
                }
            }
        }
        self.context_visiting.remove(row);
        self.contexts.insert(row.clone(), result);
        Ok(result)
    }
    fn tail(&mut self, start: usize) -> Result<Option<usize>, String> {
        if let Some(result) = self.tails.get(&start) {
            return Ok(*result);
        }
        let mut seen = BTreeSet::new();
        let mut state = start;
        let result = loop {
            if !seen.insert(state) {
                break None;
            }
            self.charge()?;
            let node = self
                .program
                .states
                .get(state)
                .ok_or("Dangling parser prediction continuation")?;
            if node.accepting {
                break None;
            }
            if node.rule_stop {
                break Some(state);
            }
            match node.edges.as_slice() {
                [Edge::Epsilon(target) | Edge::Action(target)] => state = *target,
                _ => break None,
            }
        };
        for state in seen {
            self.tails.insert(state, result);
        }
        Ok(result)
    }
    fn call(&mut self, row: &Row, target: usize, follow: usize) -> Result<Row, String> {
        if row.depth() >= DEPTH {
            return Err(INCONCLUSIVE.into());
        }
        let mut calls = row.calls.clone();
        if let Some(stop) = self.tail(follow)? {
            if let Some(last) = calls.last_mut() {
                last.weight += 1;
            } else {
                calls.push(Frame {
                    follow: stop,
                    weight: 1,
                });
            }
        } else {
            calls.push(Frame { follow, weight: 1 });
        }
        Ok(Row {
            state: target,
            calls,
        })
    }
    fn closure(&mut self, row: &Row, allowed: u128) -> Result<Closure, String> {
        let allowed = allowed & self.context_first(row)?.tokens;
        let key = (row.clone(), allowed);
        if let Some(result) = self.closures.get(&key) {
            return Ok(result.clone());
        }
        let mut pending = VecDeque::from([row.clone()]);
        let mut seen = BTreeSet::new();
        let mut result = Closure::default();
        while let Some(row) = pending.pop_front() {
            self.charge()?;
            if !seen.insert(row.clone()) {
                continue;
            }
            let first = self.context_first(&row)?;
            if first.tokens & allowed == 0 && !first.accepting {
                continue;
            }
            let node = self
                .program
                .states
                .get(row.state)
                .ok_or("Dangling parser prediction state")?;
            result.accepting |= node.accepting;
            if node.rule_stop {
                if let Some((last, rest)) = row.calls.split_last() {
                    pending.push_back(Row {
                        state: last.follow,
                        calls: rest.to_vec(),
                    });
                } else {
                    if node.follow.is_empty() {
                        return Err("Caller FOLLOW context required".into());
                    }
                    for state in node.follow.clone() {
                        self.charge()?;
                        pending.push_back(Row {
                            state,
                            calls: Vec::new(),
                        });
                    }
                }
                continue;
            }
            for edge in node.edges.clone() {
                self.charge()?;
                let next = match edge {
                    Edge::Epsilon(state)
                    | Edge::Action(state)
                    | Edge::Full { target: state, .. } => Row {
                        state,
                        ..row.clone()
                    },
                    Edge::First { path, target } => {
                        if !self.program.first_guards.contains_key(&path) {
                            return Err("Guarded shared-prefix proof remains unsupported".into());
                        }
                        Row {
                            state: target,
                            ..row.clone()
                        }
                    }
                    Edge::Call { target, follow } => self.call(&row, target, follow)?,
                    Edge::Token(symbol, state) => {
                        if self.signature(&symbol) & allowed != 0 {
                            result.tokens.entry(symbol).or_default().insert(Row {
                                state,
                                ..row.clone()
                            });
                        }
                        continue;
                    }
                };
                pending.push_back(next);
            }
        }
        self.closures.insert(key, result.clone());
        Ok(result)
    }
    fn group_first(&mut self, group: &Group) -> Result<ContextFirst, String> {
        let mut result = ContextFirst::default();
        for row in group {
            self.charge()?;
            result.include(self.context_first(row)?);
        }
        Ok(result)
    }
    fn group_closure(&mut self, group: &Group, allowed: u128) -> Result<Closure, String> {
        let mut result = Closure::default();
        for row in group {
            self.charge()?;
            let closure = self.closure(row, allowed)?;
            result.accepting |= closure.accepting;
            for (symbol, targets) in closure.tokens {
                for target in targets {
                    self.charge()?;
                    result
                        .tokens
                        .entry(symbol.clone())
                        .or_default()
                        .insert(target);
                }
            }
        }
        Ok(result)
    }
    fn run(&mut self, roots: &[usize]) -> Result<bool, String> {
        let initial: (Group, Group) = (
            [Row {
                state: roots[0],
                calls: Vec::new(),
            }]
            .into(),
            [Row {
                state: roots[1],
                calls: Vec::new(),
            }]
            .into(),
        );
        let mut pending = VecDeque::from([initial.clone()]);
        let mut queued = BTreeSet::from([initial]);
        let mut seen = BTreeSet::new();
        while let Some((left, right)) = pending.pop_front() {
            self.charge()?;
            queued.remove(&(left.clone(), right.clone()));
            if !seen.insert((left.clone(), right.clone())) {
                continue;
            }
            // Start from the exit-side summary; this affects only search order.
            let right_first = self.group_first(&right)?;
            let left_first = self.group_first(&left)?;
            if (left_first.accepting && (right_first.accepting || right_first.tokens != 0))
                || (right_first.accepting && left_first.tokens != 0)
            {
                return Ok(false);
            }
            if left_first.tokens & right_first.tokens == 0 {
                continue;
            }
            let left = self.group_closure(&left, right_first.tokens)?;
            let right = self.group_closure(&right, left_first.tokens)?;
            if (left.accepting && (right.accepting || !right.tokens.is_empty()))
                || (right.accepting && !left.tokens.is_empty())
            {
                return Ok(false);
            }
            for (symbol, left_targets) in left.tokens {
                if let Some(right_targets) = right.tokens.get(&symbol) {
                    self.charge()?;
                    let pair = (left_targets, right_targets.clone());
                    if !seen.contains(&pair) && queued.insert(pair.clone()) {
                        pending.push_back(pair);
                    }
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
#[path = "filtered_tests.rs"]
mod tests;
