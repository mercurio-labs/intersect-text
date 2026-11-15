//! Bounded scan-local transitions of one exact immutable lexer program.
//! These are NFA frontiers, not token/body results or prepared admission facts.
//! Sharing is confined to one synchronous scan, so reference counts are local.
use super::*;
use rustc_hash::FxHashMap;
use std::{ops::Deref, rc::Rc};

const MAX_ENTRIES: usize = 128;
const MAX_STATES: usize = 8192;
const MAX_OWNER_INDICES: usize = 8192;

pub(super) struct OwnerSets {
    pub(super) owners: BTreeSet<usize>,
    pub(super) accepting: BTreeSet<usize>,
}

pub(super) struct Transition {
    states: Rc<BTreeSet<usize>>,
    frontier: usize,
    work: usize,
}

pub(super) enum Current<'a> {
    Canonical(Cow<'a, BTreeSet<usize>>),
    Reused(Rc<Transition>),
}

impl Deref for Current<'_> {
    type Target = BTreeSet<usize>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Canonical(states) => states,
            Self::Reused(step) => &step.states,
        }
    }
}

impl Current<'_> {
    pub(super) fn is_borrowed(&self) -> bool {
        matches!(self, Self::Canonical(Cow::Borrowed(_)))
    }
    pub(super) fn to_mut(&mut self) -> Result<&mut BTreeSet<usize>, String> {
        if let Self::Reused(step) = self {
            *self = Self::Canonical(Cow::Owned(step.states.as_ref().clone()));
        }
        match self {
            Self::Canonical(states) => Ok(states.to_mut()),
            Self::Reused(_) => Err("Lexical transition frontier was not materialized".into()),
        }
    }
}

pub(super) struct Transitions<'program> {
    program: &'program LexerPrediction,
    rows: FxHashMap<(usize, u16), Rc<Transition>>,
    frontiers: FxHashMap<Rc<BTreeSet<usize>>, usize>,
    retained_states: usize,
    owners: FxHashMap<usize, Rc<OwnerSets>>,
    retained_owner_indices: usize,
}

impl<'program> Transitions<'program> {
    pub(super) fn new(program: &'program LexerPrediction) -> Self {
        Self {
            program,
            rows: FxHashMap::default(),
            frontiers: FxHashMap::default(),
            retained_states: 0,
            owners: FxHashMap::default(),
            retained_owner_indices: 0,
        }
    }

    pub(super) fn owner_sets(
        &mut self,
        program: &LexerPrediction,
        current: &Current<'_>,
        work: &mut Option<usize>,
    ) -> Result<Option<Rc<OwnerSets>>, String> {
        if !std::ptr::eq(self.program, program) {
            return Err("Lexical owner cache belongs to another program".into());
        }
        let Current::Reused(step) = current else {
            return Ok(None);
        };
        // Short budgets retain the canonical collector's partial work and
        // error order. No observer is bypassed by this scan-local worker.
        if work.is_some_and(|remaining| remaining < current.len()) {
            return Ok(None);
        }
        if let Some(sets) = self.owners.get(&step.frontier) {
            if let Some(remaining) = work {
                *remaining -= current.len();
            }
            return Ok(Some(Rc::clone(sets)));
        }
        // Both sets have at most one entry per state. Check this conservative
        // bound before collecting, so a declined admission does no extra work.
        if self.owners.len() >= MAX_ENTRIES
            || current.len() > MAX_OWNER_INDICES.saturating_sub(self.retained_owner_indices) / 2
        {
            return Ok(None);
        }
        let (owners, accepting) = program.frontier_owners(current, work)?;
        self.retained_owner_indices += owners.len() + accepting.len();
        let sets = Rc::new(OwnerSets { owners, accepting });
        self.owners.insert(step.frontier, Rc::clone(&sets));
        Ok(Some(sets))
    }

    pub(super) fn advance(
        &mut self,
        program: &LexerPrediction,
        current: &Current<'_>,
        unit: u16,
        work: &mut Option<usize>,
    ) -> Result<Current<'static>, String> {
        if !std::ptr::eq(self.program, program) {
            return Err("Lexical transition cache belongs to another program".into());
        }
        // Zero denotes the caller's canonical initial frontier. Other IDs
        // belong to exact interned state sets in this immutable program.
        let source = match current {
            Current::Canonical(Cow::Borrowed(_)) => Some(0),
            Current::Reused(step) => Some(step.frontier),
            Current::Canonical(Cow::Owned(_)) => None,
        };
        if let Some(source) = source
            && let Some(step) = self.rows.get(&(source, unit))
            && work.is_none_or(|remaining| remaining >= step.work)
        {
            if let Some(remaining) = work {
                *remaining -= step.work;
            }
            return Ok(Current::Reused(Rc::clone(step)));
        }
        // Short budgets and misses follow the SAME worker, preserving partial
        // debits and error ordering. Failed work never publishes a frontier.
        let targets = program.advance(current, unit, work)?;
        let states = if targets.is_empty() {
            targets
        } else {
            program.closure(targets, work)?
        };
        if let Some(source) = source
            && self.rows.len() < MAX_ENTRIES
            && let Some(cost) = self.cost(current, &states)
        {
            let (states, frontier) = match self.intern(states) {
                Ok(frontier) => frontier,
                Err(states) => return Ok(Current::Canonical(Cow::Owned(states))),
            };
            let step = Rc::new(Transition {
                states,
                frontier,
                work: cost,
            });
            self.rows.insert((source, unit), Rc::clone(&step));
            return Ok(Current::Reused(step));
        }
        Ok(Current::Canonical(Cow::Owned(states)))
    }

    fn intern(
        &mut self,
        states: BTreeSet<usize>,
    ) -> Result<(Rc<BTreeSet<usize>>, usize), BTreeSet<usize>> {
        if let Some((states, &frontier)) = self.frontiers.get_key_value(&states) {
            return Ok((Rc::clone(states), frontier));
        }
        if states.len() > MAX_STATES.saturating_sub(self.retained_states) {
            return Err(states);
        }
        self.retained_states += states.len();
        let frontier = self.frontiers.len() + 1;
        let states = Rc::new(states);
        self.frontiers.insert(Rc::clone(&states), frontier);
        Ok((states, frontier))
    }

    fn cost(&self, initial: &BTreeSet<usize>, states: &BTreeSet<usize>) -> Option<usize> {
        // advance charges every source edge; closure charges every distinct
        // state and epsilon edge. Owner work is charged at the next loop.
        let mut cost = 0usize;
        for &index in initial {
            cost = cost.checked_add(self.program.states.get(index)?.edges.len())?;
        }
        for &index in states {
            cost = cost
                .checked_add(1)?
                .checked_add(self.program.states.get(index)?.epsilon.len())?;
        }
        Some(cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_lexer_definitions_remain_send_and_sync() {
        fn shared<T: Send + Sync>() {}
        shared::<LexerPrediction>();
        shared::<crate::grammar::Terminal>();
        shared::<crate::grammar::Grammar>();
    }

    fn expression(kind: Operation) -> Expression {
        Expression {
            kind,
            cardinality: '1',
            first: vec![],
            nullable: false,
            decision: None,
            cardinality_decision: None,
        }
    }
    fn literal(text: &str) -> Expression {
        expression(Operation::Literal(text.encode_utf16().collect()))
    }
    fn initial(program: &LexerPrediction) -> Result<BTreeSet<usize>, String> {
        program.closure(program.roots.iter().copied(), &mut None)
    }

    #[test]
    fn failed_work_is_not_cached_and_hits_replay_exact_transition_debits() -> Result<(), String> {
        let program = LexerPrediction::terminal_choices(&[literal("ab"), literal("ac")])?;
        let initial = initial(&program)?;
        let first = Current::Canonical(Cow::Borrowed(&initial));
        let mut cache = Transitions::new(&program);
        let mut cold_work = Some(10_000);
        let states = cache.advance(&program, &first, 97, &mut cold_work)?;
        let cost = 10_000 - cold_work.ok_or("Missing finite work")?;
        assert!(cost > 0);
        for unit in [97, 120] {
            for budget in 0..=cost + 2 {
                let mut expected_work = Some(budget);
                let expected = program
                    .advance(&initial, unit, &mut expected_work)
                    .and_then(|targets| {
                        if targets.is_empty() {
                            Ok(targets)
                        } else {
                            program.closure(targets, &mut expected_work)
                        }
                    });
                for warmed in [false, true] {
                    let mut fresh = Transitions::new(&program);
                    let candidate = if warmed { &mut cache } else { &mut fresh };
                    let mut work = Some(budget);
                    let result = candidate.advance(&program, &first, unit, &mut work);
                    assert_eq!(
                        result.as_ref().map(|s| &**s).map_err(String::as_str),
                        expected.as_ref().map_err(String::as_str)
                    );
                    assert_eq!(work, expected_work);
                    if !warmed && expected.is_err() {
                        assert!(fresh.rows.is_empty());
                    }
                }
            }
        }
        let mut materialized = states;
        materialized.to_mut()?.insert(usize::MAX);
        assert!(
            !cache
                .rows
                .get(&(0, 97))
                .ok_or("Missing transition")?
                .states
                .contains(&usize::MAX)
        );
        let foreign = program.clone();
        assert!(cache.advance(&foreign, &first, 97, &mut None).is_err());
        Ok(())
    }

    #[test]
    fn retained_entry_and_state_caps_fall_back_without_changing_frontiers() -> Result<(), String> {
        for choices in [2, 100] {
            let alternatives: Vec<_> = (0..choices)
                .map(|_| {
                    expression(Operation::Sequence(vec![
                        expression(Operation::Wildcard),
                        literal("a"),
                    ]))
                })
                .collect();
            let program = LexerPrediction::terminal_choices(&alternatives)?;
            let initial = initial(&program)?;
            let first = Current::Canonical(Cow::Borrowed(&initial));
            let mut cache = Transitions::new(&program);
            for unit in 0..256 {
                let targets = program.advance(&initial, unit, &mut None)?;
                let expected = program.closure(targets, &mut None)?;
                let actual = cache.advance(&program, &first, unit, &mut None)?;
                assert_eq!(&*actual, &expected);
                assert!(cache.rows.len() <= MAX_ENTRIES);
                assert!(cache.retained_states <= MAX_STATES);
            }
            assert_eq!(cache.rows.len(), MAX_ENTRIES);
            // Different units share exactly the same successor frontier.
            assert_eq!(cache.frontiers.len(), 1);
        }
        // A valid sub-10K-state program with one large successor closure.
        // Repeating terminal alternatives would hit the compiler's separate
        // construction cap before reaching this retained-state boundary.
        let program = LexerPrediction {
            alternatives: vec![Alternative::Terminal("large".into())],
            roots: vec![0],
            states: (0..=MAX_STATES + 1)
                .map(|index| State {
                    owner: 0,
                    accepting: index == MAX_STATES + 1,
                    epsilon: if index > 0 && index < MAX_STATES + 1 {
                        vec![index + 1]
                    } else {
                        vec![]
                    },
                    eof: vec![],
                    edges: if index == 0 {
                        vec![Edge {
                            low: 97,
                            high: 97,
                            target: 1,
                        }]
                    } else {
                        vec![]
                    },
                })
                .collect(),
        };
        let initial = initial(&program)?;
        let first = Current::Canonical(Cow::Borrowed(&initial));
        let mut cache = Transitions::new(&program);
        let expected = program.closure(program.advance(&initial, 97, &mut None)?, &mut None)?;
        assert!(expected.len() > MAX_STATES);
        assert_eq!(&*cache.advance(&program, &first, 97, &mut None)?, &expected);
        assert!(cache.rows.is_empty());
        assert!(cache.frontiers.is_empty());
        Ok(())
    }

    #[test]
    fn later_edges_distinguish_source_frontiers_and_replay_all_budget_boundaries()
    -> Result<(), String> {
        let program = LexerPrediction::terminal_choices(&[
            literal("abx"),
            literal("aby"),
            literal("acx"),
            literal("acy"),
        ])?;
        let initial = initial(&program)?;
        let first = Current::Canonical(Cow::Borrowed(&initial));
        let mut cache = Transitions::new(&program);
        let after_a = cache.advance(&program, &first, 97, &mut None)?;
        let after_ab = cache.advance(&program, &after_a, 98, &mut None)?;
        let after_ac = cache.advance(&program, &after_a, 99, &mut None)?;
        let mut destinations = Vec::new();
        for current in [&after_ab, &after_ac] {
            let Current::Reused(source) = current else {
                return Err("Expected an interned noninitial frontier".into());
            };
            for unit in [120, 121, 122] {
                let mut work = Some(10_000);
                let result = cache.advance(&program, current, unit, &mut work)?;
                if unit == 120 {
                    destinations.push(result.iter().copied().collect::<Vec<_>>());
                }
                let cost = 10_000 - work.ok_or("Missing finite work")?;
                let key = (source.frontier, unit);
                let saved = Rc::clone(cache.rows.get(&key).ok_or("Missing later transition")?);
                for budget in 0..=cost + 1 {
                    let mut expected_work = Some(budget);
                    let expected =
                        program
                            .advance(current, unit, &mut expected_work)
                            .and_then(|targets| {
                                if targets.is_empty() {
                                    Ok(targets)
                                } else {
                                    program.closure(targets, &mut expected_work)
                                }
                            });
                    for warmed in [true, false] {
                        if !warmed {
                            cache.rows.remove(&key);
                        }
                        let mut work = Some(budget);
                        let actual = cache.advance(&program, current, unit, &mut work);
                        assert_eq!(
                            actual.as_ref().map(|s| &**s).map_err(String::as_str),
                            expected.as_ref().map_err(String::as_str)
                        );
                        assert_eq!(work, expected_work);
                        if !warmed && actual.is_err() {
                            assert!(!cache.rows.contains_key(&key));
                        }
                        cache.rows.insert(key, Rc::clone(&saved));
                    }
                }
            }
        }
        assert_ne!(destinations[0], destinations[1]);
        assert!(cache.rows.keys().any(|&(source, _)| source != 0));
        Ok(())
    }

    fn collect_owners(
        cache: &mut Transitions<'_>,
        program: &LexerPrediction,
        current: &Current<'_>,
        work: &mut Option<usize>,
    ) -> Result<(BTreeSet<usize>, BTreeSet<usize>), String> {
        match cache.owner_sets(program, current, work)? {
            Some(sets) => Ok((sets.owners.clone(), sets.accepting.clone())),
            None => program.frontier_owners(current, work),
        }
    }

    #[test]
    fn owner_hits_and_misses_keep_exact_work_errors_and_frontier_identity() -> Result<(), String> {
        let program = LexerPrediction::terminal_choices(&[
            literal("ab"),
            literal("ac"),
            literal("bd"),
            literal("be"),
        ])?;
        let initial = initial(&program)?;
        let first = Current::Canonical(Cow::Borrowed(&initial));
        let mut warm = Transitions::new(&program);
        let mut distinct = Vec::new();
        for unit in [97, 98] {
            let current = warm.advance(&program, &first, unit, &mut None)?;
            let retained = warm
                .owner_sets(&program, &current, &mut None)?
                .ok_or("Missing retained owner sets")?;
            distinct.push(retained.owners.clone());
            assert!(Rc::ptr_eq(
                &retained,
                &warm
                    .owner_sets(&program, &current, &mut None)?
                    .ok_or("Missing owner hit")?
            ));
            for budget in 0..=current.len() + 1 {
                let mut expected_work = Some(budget);
                let expected = program.frontier_owners(&current, &mut expected_work);
                let mut actual_work = Some(budget);
                let actual = collect_owners(&mut warm, &program, &current, &mut actual_work);
                assert_eq!(actual, expected);
                assert_eq!(actual_work, expected_work);

                let mut cold = Transitions::new(&program);
                let current = cold.advance(&program, &first, unit, &mut None)?;
                let mut work = Some(budget);
                assert_eq!(
                    collect_owners(&mut cold, &program, &current, &mut work),
                    expected
                );
                assert_eq!(work, expected_work);
                if expected.is_err() {
                    assert!(cold.owners.is_empty());
                    assert_eq!(cold.retained_owner_indices, 0);
                }
            }
            let foreign = program.clone();
            assert!(warm.owner_sets(&foreign, &current, &mut None).is_err());
        }
        assert_ne!(distinct[0], distinct[1]);
        let mut untouched = Some(0);
        assert!(warm.owner_sets(&program, &first, &mut untouched)?.is_none());
        assert_eq!(untouched, Some(0));
        Ok(())
    }

    #[test]
    fn owner_index_bound_declines_before_work_and_keeps_canonical_results() -> Result<(), String> {
        // A valid 9,600-state NFA: six different units select disjoint groups
        // of 800 accepting owners. State storage fits, but six owner-set pairs
        // would exceed the independent owner-index bound.
        let program = LexerPrediction {
            alternatives: (0..4800)
                .map(|n| Alternative::Terminal(n.to_string()))
                .collect(),
            roots: (0..4800).map(|n| n * 2).collect(),
            states: (0..9600)
                .map(|index| State {
                    owner: index / 2,
                    accepting: index % 2 == 1,
                    epsilon: vec![],
                    eof: vec![],
                    edges: if index % 2 == 0 {
                        vec![Edge {
                            low: (index / 1600) as u16,
                            high: (index / 1600) as u16,
                            target: index + 1,
                        }]
                    } else {
                        vec![]
                    },
                })
                .collect(),
        };
        let initial = initial(&program)?;
        let first = Current::Canonical(Cow::Borrowed(&initial));
        let mut cache = Transitions::new(&program);
        for unit in 0..6 {
            let current = cache.advance(&program, &first, unit, &mut None)?;
            let mut work = Some(10_000);
            let sets = cache.owner_sets(&program, &current, &mut work)?;
            if unit < 5 {
                let sets = sets.ok_or("Owner admission declined too early")?;
                assert_eq!((sets.owners.len(), sets.accepting.len()), (800, 800));
                assert_eq!(work, Some(9200));
            } else {
                assert!(sets.is_none());
                assert_eq!(work, Some(10_000));
                let actual = collect_owners(&mut cache, &program, &current, &mut work)?;
                let expected = program.frontier_owners(&current, &mut None)?;
                assert_eq!(actual, expected);
                assert_eq!(work, Some(9200));
            }
            assert!(cache.retained_states <= MAX_STATES);
            assert!(cache.retained_owner_indices <= MAX_OWNER_INDICES);
        }
        assert_eq!(cache.owners.len(), 5);
        assert_eq!(cache.retained_owner_indices, 8000);
        Ok(())
    }
}
