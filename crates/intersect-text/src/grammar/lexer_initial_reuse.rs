//! Scan-local reuse of canonical initial sets, with unchanged canonical debits.
//! Unlike explicitly admitted prepared facts, this does not reduce logical work.
use super::*;

pub(super) type InitialFrontier<'a> = (
    &'a BTreeSet<usize>,
    &'a BTreeSet<usize>,
    &'a BTreeSet<usize>,
);

struct Initial {
    states: BTreeSet<usize>,
    owners: BTreeSet<usize>,
    accepting: BTreeSet<usize>,
    work: usize,
}

/// Borrows one exact immutable program for one scan; retains no input results.
pub(in crate::grammar) struct ReusableLexicalInitial<'program> {
    program: &'program LexerPrediction,
    initial: Option<Initial>,
    transitions: Transitions<'program>,
}

impl<'program> ReusableLexicalInitial<'program> {
    pub(in crate::grammar) fn new(program: &'program LexerPrediction) -> Self {
        Self {
            program,
            initial: None,
            transitions: Transitions::new(program),
        }
    }

    pub(in crate::grammar) fn program(&self) -> &'program LexerPrediction {
        self.program
    }

    fn seed(&mut self, work: &mut Option<usize>) -> Result<(), String> {
        if let Some(initial) = &self.initial {
            // The canonical worker has no observer here. Replay every logical
            // debit, including exhaustion to zero, without allocating sets.
            if let Some(remaining) = work {
                if *remaining < initial.work {
                    *remaining = 0;
                    return Err(STEP_BUDGET_EXCEEDED.into());
                }
                *remaining -= initial.work;
            }
        } else {
            let states = self
                .program
                .closure(self.program.roots.iter().copied(), work)?;
            let (owners, accepting) = self.program.frontier_owners(&states, work)?;
            // Closure pops each unique state once and charges every epsilon
            // edge. Owner collection then charges each state once more.
            let mut count = states.len();
            for &index in &states {
                let state = self
                    .program
                    .states
                    .get(index)
                    .ok_or("Invalid lexical prediction state")?;
                count = count
                    .checked_add(1)
                    .and_then(|n| n.checked_add(state.epsilon.len()))
                    .ok_or("Lexical initial work overflow")?;
            }
            self.initial = Some(Initial {
                states,
                owners,
                accepting,
                work: count,
            });
        }
        Ok(())
    }

    /// Predict over the source's UTF-16 units, also reporting how many leading bytes of `input`
    /// the decision consumed when those are ASCII and the end of the input
    /// was not reached: the decision is then a function of that prefix.
    pub(in crate::grammar) fn predict_document_observed(
        &mut self,
        input: &str,
        work: &mut Option<usize>,
    ) -> Result<(LexicalDecision, Option<usize>), String> {
        let mut consumed = 0usize;
        let mut ended = false;
        let mut units = input.encode_utf16();
        let decision = self.predict_units(
            std::iter::from_fn(|| {
                let unit = units.next();
                match unit {
                    Some(_) => consumed += 1,
                    None => ended = true,
                }
                unit
            }),
            work,
            false,
        )?;
        let observed = (!ended
            && input
                .as_bytes()
                .get(..consumed)
                .is_some_and(<[u8]>::is_ascii))
        .then_some(consumed);
        Ok((decision, observed))
    }

    pub(in crate::grammar) fn predict(
        &mut self,
        units: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        nongreedy: bool,
    ) -> Result<usize, String> {
        self.predict_units(units, work, nongreedy)?.selected()
    }

    fn predict_units(
        &mut self,
        units: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        nongreedy: bool,
    ) -> Result<LexicalDecision, String> {
        let program = self.program;
        self.seed(work)?;
        let initial = self
            .initial
            .as_ref()
            .ok_or("Missing canonical lexical initial")?;
        let first = (&initial.states, &initial.owners, &initial.accepting);
        program.predict_units_with_frontier::<false>(
            units,
            work,
            nongreedy,
            None,
            Some(first),
            Some(&mut self.transitions),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn fixtures() -> Result<Vec<LexerPrediction>, String> {
        let mut letters = expression(Operation::Range(97, 122));
        letters.cardinality = '+';
        let mut keyword = LexerPrediction::terminal_choices(&[literal("if"), letters])?;
        keyword.alternatives[0] = Alternative::Keyword("if".into());
        let mut invalid_later = LexerPrediction::terminal_choices(&[literal("ab"), literal("ac")])?;
        for state in &mut invalid_later.states {
            for edge in &mut state.edges {
                if edge.low == 98 {
                    edge.target = usize::MAX;
                }
            }
        }
        Ok(vec![
            keyword,
            LexerPrediction::terminal_choices(&[
                literal("abx"),
                literal("aby"),
                literal("acx"),
                literal("acy"),
            ])?,
            invalid_later,
            LexerPrediction::terminal_choices(&[
                expression(Operation::Sequence(vec![
                    literal("a"),
                    expression(Operation::Eof),
                ])),
                literal("ab"),
            ])?,
            LexerPrediction::terminal_choices(&[
                expression(Operation::Eof),
                literal("a"),
                literal("ab"),
            ])?,
            LexerPrediction::terminal_choices(&[
                expression(Operation::Until(vec![33])),
                literal("!a"),
            ])?,
            LexerPrediction::terminal_choices(&[
                literal("😀"),
                expression(Operation::NotCharacters(vec![(120, 120)])),
            ])?,
            LexerPrediction::terminal_choices(&[literal("a"), literal("a")])?,
            LexerPrediction {
                alternatives: vec![],
                states: vec![],
                roots: vec![],
            },
        ])
    }

    #[test]
    fn cold_and_reused_initials_match_all_decisions_errors_and_budget_boundaries()
    -> Result<(), String> {
        let inputs: Vec<Vec<u16>> = [
            "", "a", "ab", "ac", "abx", "aby", "acx", "acy", "abz", "if", "iff", "x", "!", "!a",
            "!a!", "abc ", "😀", "\r\n",
        ]
        .into_iter()
        .map(|s| s.encode_utf16().collect())
        .chain([
            vec![0],
            vec![0xd800],
            vec![0xdc00],
            vec![0xffff],
            vec![0xd800, 97],
        ])
        .collect();
        for program in fixtures()? {
            let mut reused = ReusableLexicalInitial::new(&program);
            reused.seed(&mut None)?;
            for units in &inputs {
                for nongreedy in [false, true] {
                    let mut full = Some(10000);
                    let _ = program.predict_units_with_policy(
                        units.iter().copied(),
                        &mut full,
                        nongreedy,
                    );
                    let used = 10000 - full.ok_or("Lost finite work")?;
                    for budget in 0..=used + 1 {
                        let mut expected_work = Some(budget);
                        let expected = program.predict_units_with_policy(
                            units.iter().copied(),
                            &mut expected_work,
                            nongreedy,
                        );
                        let mut cold = ReusableLexicalInitial::new(&program);
                        for cache in [&mut cold, &mut reused] {
                            let mut actual_work = Some(budget);
                            let actual = cache.predict_units(
                                units.iter().copied(),
                                &mut actual_work,
                                nongreedy,
                            );
                            assert_eq!(
                                format!("{actual:?}"),
                                format!("{expected:?}"),
                                "{units:?}, budget {budget}, nongreedy {nongreedy}"
                            );
                            assert_eq!(actual_work, expected_work);
                        }
                    }
                    let expected = program.predict_units_with_policy(
                        units.iter().copied(),
                        &mut None,
                        nongreedy,
                    );
                    let actual = reused.predict_units(units.iter().copied(), &mut None, nongreedy);
                    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn transition_reuse_keeps_exact_unit_reads_and_early_commitment() -> Result<(), String> {
        use std::cell::Cell;
        let mut programs = fixtures()?;
        programs.push(LexerPrediction::terminal_choices(&[literal("a")])?);
        for program in programs {
            let mut cache = ReusableLexicalInitial::new(&program);
            for input in [
                "",
                "ab",
                "abx",
                "acy",
                "abz",
                "iff",
                "!a!",
                "?",
                "\u{1f600}",
            ] {
                for nongreedy in [false, true] {
                    for budget in [0, 1, 16, 64, 10000] {
                        let expected_reads = Cell::new(0);
                        let actual_reads = Cell::new(0);
                        let mut expected_work = Some(budget);
                        let mut actual_work = Some(budget);
                        let expected = program.predict_units_with_policy(
                            input
                                .encode_utf16()
                                .inspect(|_| expected_reads.set(expected_reads.get() + 1)),
                            &mut expected_work,
                            nongreedy,
                        );
                        let actual = cache.predict_units(
                            input
                                .encode_utf16()
                                .inspect(|_| actual_reads.set(actual_reads.get() + 1)),
                            &mut actual_work,
                            nongreedy,
                        );
                        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
                        assert_eq!(actual_work, expected_work);
                        assert_eq!(actual_reads.get(), expected_reads.get());
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn partial_or_invalid_initial_work_never_publishes_reusable_sets() -> Result<(), String> {
        let program = LexerPrediction::terminal_choices(&[literal("ab"), literal("ac")])?;
        let mut complete = ReusableLexicalInitial::new(&program);
        complete.seed(&mut None)?;
        let cost = complete.initial.as_ref().ok_or("Missing initial")?.work;
        for budget in 0..cost {
            let mut cache = ReusableLexicalInitial::new(&program);
            let mut work = Some(budget);
            assert!(cache.seed(&mut work).is_err());
            assert_eq!(work, Some(0));
            assert!(cache.initial.is_none());
            cache.seed(&mut None)?;
            assert!(cache.initial.is_some());
        }
        let mut invalid = program.clone();
        invalid.roots.push(usize::MAX);
        let mut cache = ReusableLexicalInitial::new(&invalid);
        assert!(cache.seed(&mut None).is_err());
        assert!(cache.initial.is_none());
        Ok(())
    }

    #[test]
    fn scanner_keeps_token_offsets_and_remaining_work_across_reused_initials() -> Result<(), String>
    {
        let mut program = LexerPrediction::terminal_choices(&[literal("a"), literal("b")])?;
        program.alternatives = vec![
            Alternative::Keyword("a".into()),
            Alternative::Keyword("b".into()),
        ];
        for source in [" a b a", " a b ?"] {
            for budget in 0..=100 {
                let mut expected_work = Some(budget);
                let mut expected_error = None;
                for start in [1, 3, 5] {
                    match program.predict_units_with_policy(
                        source[start..].encode_utf16(),
                        &mut expected_work,
                        false,
                    ) {
                        Ok(LexicalDecision::Selected(_)) => {}
                        Ok(LexicalDecision::Unselected { .. }) => {
                            expected_error = Some((start, "No viable lexical alternative".into()));
                            break;
                        }
                        Err(e) => {
                            expected_error = Some((start, e));
                            break;
                        }
                    }
                }
                let mut actual_work = Some(budget);
                let actual = crate::grammar::lex_borrowed_with_prediction(
                    source,
                    None,
                    None,
                    None,
                    None,
                    Some(&program),
                    &mut actual_work,
                );
                assert_eq!(actual_work, expected_work);
                match (actual, expected_error) {
                    (Err(actual), Some(expected)) => assert_eq!(actual, expected),
                    (Ok(tokens), None) => assert_eq!(
                        tokens
                            .iter()
                            .map(|t| (t.text.as_ref(), t.start, t.end, t.lexical_alternative))
                            .collect::<Vec<_>>(),
                        vec![
                            ("a", 1, 2, Some(0)),
                            ("b", 3, 4, Some(1)),
                            ("a", 5, 6, Some(0))
                        ]
                    ),
                    _ => return Err("Scanner outcome differs".into()),
                }
            }
        }
        for source in ["", " \t\n"] {
            let mut work = Some(0);
            assert!(
                crate::grammar::lex_borrowed_with_prediction(
                    source,
                    None,
                    None,
                    None,
                    None,
                    Some(&program),
                    &mut work
                )
                .map_err(|(_, e)| e)?
                .is_empty()
            );
            assert_eq!(work, Some(0));
        }
        Ok(())
    }
}
