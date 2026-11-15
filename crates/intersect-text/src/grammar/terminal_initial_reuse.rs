//! One scan's lazy initial sets for the exact borrowed terminal decision programs.
//! This is canonical work replay, not prepared-fact admission or input memoization.
use super::super::lexer_prediction::ReusableLexicalInitial;
use super::*;
use rustc_hash::FxHashMap;
use std::cell::RefCell;

#[derive(Default)]
pub(in crate::grammar) struct ReusableTerminalPredictions<'program> {
    initials: RefCell<FxHashMap<usize, ReusableLexicalInitial<'program>>>,
    /// Unbudgeted greedy selections that read exactly one unit, by program
    /// address and that unit.
    single_unit: RefCell<FxHashMap<(usize, u16), usize>>,
}

#[derive(Clone, Copy)]
pub(super) struct ReusedPrediction<'context, 'program>(
    pub(super) &'context ReusableTerminalPredictions<'program>,
);

impl<'program> LocalPrediction<'program> for ReusedPrediction<'_, 'program> {
    fn predict(
        self,
        program: &'program super::super::LexerPrediction,
        units: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        nongreedy: bool,
    ) -> Result<usize, String> {
        let mut initials = self
            .0
            .initials
            .try_borrow_mut()
            .map_err(|_| "Overlapping terminal prediction reuse")?;
        // Addresses are private lookup keys only. Each holder keeps its exact
        // immutable program borrowed for this registry's complete lifetime.
        initials
            .entry(program as *const _ as usize)
            .or_insert_with(|| ReusableLexicalInitial::new(program))
            .predict(units, work, nongreedy)
    }
    const RECALLS: bool = true;
    fn recall(self, program: &'program super::super::LexerPrediction, unit: u16) -> Option<usize> {
        self.0
            .single_unit
            .try_borrow()
            .ok()?
            .get(&(program as *const _ as usize, unit))
            .copied()
    }
    fn remember(
        self,
        program: &'program super::super::LexerPrediction,
        unit: u16,
        selected: usize,
    ) {
        if let Ok(mut known) = self.0.single_unit.try_borrow_mut() {
            known.insert((program as *const _ as usize, unit), selected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr(kind: Operation) -> Expression {
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
        expr(Operation::Literal(text.encode_utf16().collect()))
    }
    fn terminal(name: &str, body: Expression) -> Terminal {
        Terminal {
            override_declared: false,
            fragment: false,
            datatype: crate::grammar::Datatype::EString,
            nominal_return: None,
            enumeration: None,
            identity: name.into(),
            start: 0,
            end: 1,
            body,
        }
    }
    fn fixture() -> Result<BTreeMap<String, Terminal>, String> {
        let letter = || {
            expr(Operation::Alternative(vec![
                expr(Operation::Range(97, 122)),
                expr(Operation::Range(65, 90)),
                literal("_"),
            ]))
        };
        let mut caret = literal("^");
        caret.cardinality = '?';
        let mut tail = expr(Operation::Alternative(vec![
            letter(),
            expr(Operation::Range(48, 57)),
        ]));
        tail.cardinality = '*';
        let mut repeated = literal("x");
        repeated.cardinality = '+';
        let mut map = BTreeMap::from([
            (
                "WORD".into(),
                terminal(
                    "WORD",
                    expr(Operation::Sequence(vec![caret, letter(), tail])),
                ),
            ),
            (
                "PAIR".into(),
                terminal(
                    "PAIR",
                    expr(Operation::Alternative(vec![literal("ab"), literal("ac")])),
                ),
            ),
            (
                "BLOCK".into(),
                terminal(
                    "BLOCK",
                    expr(Operation::Sequence(vec![
                        literal("/*"),
                        expr(Operation::UntilOperand {
                            operand: Box::new(literal("*/")),
                            decision: None,
                        }),
                    ])),
                ),
            ),
            (
                "END".into(),
                terminal(
                    "END",
                    expr(Operation::Sequence(vec![repeated, expr(Operation::Eof)])),
                ),
            ),
            (
                "UNICODE".into(),
                terminal(
                    "UNICODE",
                    expr(Operation::Alternative(vec![literal("😀"), literal("😁")])),
                ),
            ),
            (
                "UNIT".into(),
                terminal("UNIT", expr(Operation::Literal(vec![0xd800]))),
            ),
            ("FIXED".into(), terminal("FIXED", literal("="))),
        ]);
        prepare(&mut map, crate::grammar::Profile::NativeV82).map_err(|e| e.message)?;
        Ok(map)
    }
    fn shown(result: Result<DocumentUnitMatch, String>) -> Result<serde_json::Value, String> {
        result.map(|v| match v {
            DocumentUnitMatch::Matched(end) => serde_json::json!({"end":end}),
            DocumentUnitMatch::Failed {
                consumed_units,
                failure,
            } => serde_json::json!({"consumed":consumed_units,"failure":failure}),
        })
    }

    #[test]
    fn cold_and_reused_terminal_bodies_keep_whole_results_and_all_work_boundaries()
    -> Result<(), String> {
        let map = fixture()?;
        let reused = ReusableTerminalPredictions::default();
        let inputs = [
            "",
            "abc12",
            "^abc_1",
            "_",
            "ab",
            "ac",
            "ad",
            "xxxx",
            "xxx!",
            "/*a*/",
            "/*😀*/",
            "/*unclosed",
            "😀",
            "😁x",
            "=",
            "?",
            "\r\n",
        ];
        for terminal in map.values() {
            for input in inputs {
                let mut full = Some(10000);
                let _ = terminal.match_len_with_work(input, &mut full);
                let used = 10000 - full.ok_or("Lost finite work")?;
                // Warm the registry independently of the boundary under test.
                let _ = terminal.match_len_reusing(input, &reused, &mut None);
                for budget in 0..=used + 1 {
                    let mut expected_work = Some(budget);
                    let expected = terminal.match_len_with_work(input, &mut expected_work);
                    let cold = ReusableTerminalPredictions::default();
                    for registry in [&cold, &reused] {
                        let mut actual_work = Some(budget);
                        let actual = terminal.match_len_reusing(input, registry, &mut actual_work);
                        assert_eq!(
                            actual, expected,
                            "{} {input:?} budget {budget}",
                            terminal.identity
                        );
                        assert_eq!(actual_work, expected_work);
                    }
                }
            }
        }
        assert!(!reused.initials.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn reused_document_observation_preserves_source_units_failures_and_remaining_work()
    -> Result<(), String> {
        let map = fixture()?;
        let reused = ReusableTerminalPredictions::default();
        let inputs: Vec<Vec<u16>> = [
            "", "abc", "ab", "ad", "xxx", "xxx!", "/*😀*/", "/*bad", "😀", "=",
        ]
        .into_iter()
        .map(|s| s.encode_utf16().collect())
        .chain([vec![0xd800], vec![0xdc00], vec![0xffff], vec![0xd800, 97]])
        .collect();
        for terminal in map.values() {
            for input in &inputs {
                let mut full = Some(10000);
                let _ = terminal.match_document_units(input.iter().copied(), &mut full);
                let used = 10000 - full.ok_or("Lost finite work")?;
                for budget in 0..=used + 1 {
                    let mut expected_work = Some(budget);
                    let expected = shown(
                        terminal.match_document_units(input.iter().copied(), &mut expected_work),
                    );
                    let mut actual_work = Some(budget);
                    let actual = shown(terminal.match_document_units_mode(
                        input.iter().copied(),
                        &mut actual_work,
                        ReusedPrediction(&reused),
                    ));
                    assert_eq!(
                        actual, expected,
                        "{} {input:?} budget {budget}",
                        terminal.identity
                    );
                    assert_eq!(actual_work, expected_work);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn same_named_cloned_programs_remain_distinct_and_literal_matches_stay_lazy()
    -> Result<(), String> {
        let map = fixture()?;
        let word = map.get("WORD").ok_or("Missing word")?;
        let clone = word.clone();
        let reused = ReusableTerminalPredictions::default();
        assert_eq!(
            map.get("FIXED")
                .ok_or("Missing literal")?
                .match_len_reusing("=", &reused, &mut Some(0))?,
            1
        );
        assert!(reused.initials.borrow().is_empty());
        assert_eq!(word.match_len_reusing("abc12", &reused, &mut None)?, 5);
        let original_programs = reused.initials.borrow().len();
        assert!(original_programs > 0);
        assert_eq!(clone.match_len_reusing("abc12", &reused, &mut None)?, 5);
        assert_eq!(reused.initials.borrow().len(), 2 * original_programs);
        Ok(())
    }

    // The original buffered source reader, over the same committed matcher.
    fn buffered_match<'program, P: LocalPrediction<'program>>(
        terminal: &'program Terminal,
        input: &str,
        work: &mut Option<usize>,
        prediction: P,
    ) -> Result<usize, String> {
        let mut matcher = Matcher {
            input: BufferedUnits::new(input.encode_utf16()),
            work: *work,
            error: None,
            observe_failure: false,
            failure: None,
            prediction,
            program: std::marker::PhantomData,
        };
        let result = matcher.matched(&terminal.body, 0);
        *work = matcher.work;
        if let Some(error) = matcher.error {
            return Err(error);
        }
        let end = result
            .ok_or_else(|| format!("Incomplete or invalid terminal {}", terminal.identity))?;
        utf8_endpoint(input, end)
    }

    #[test]
    fn borrowed_string_units_keep_canonical_recognition_errors_and_every_work_boundary()
    -> Result<(), String> {
        let mut map = fixture()?;
        map.extend([
            ("EMPTY".into(), terminal("EMPTY", literal(""))),
            (
                "HIGH".into(),
                terminal("HIGH", expr(Operation::Literal(vec![0xd83d]))),
            ),
            ("MIXED".into(), terminal("MIXED", literal("a😀z"))),
            (
                "PARTIAL".into(),
                terminal(
                    "PARTIAL",
                    expr(Operation::Sequence(vec![
                        literal("a"),
                        expr(Operation::Wildcard),
                    ])),
                ),
            ),
        ]);
        let inputs = [
            "",
            "abc12",
            "^abc_1",
            "_",
            "ab",
            "ac",
            "ad",
            "xxx",
            "xxx!",
            "/*a*/",
            "/*😀*/",
            "/*unclosed",
            "😀",
            "😁x",
            "a😀z",
            "aéz",
            "=",
            "?",
            "\r\n",
            "\0",
        ];
        let reused = ReusableTerminalPredictions::default();
        for terminal in map.values() {
            for input in inputs {
                let mut full = Some(10000);
                let _ = buffered_match(terminal, input, &mut full, ColdPrediction);
                let used = 10000 - full.ok_or("Lost finite work")?;
                let _ = terminal.match_len_reusing(input, &reused, &mut None);
                for budget in 0..=used + 1 {
                    let mut expected_work = Some(budget);
                    let expected =
                        buffered_match(terminal, input, &mut expected_work, ColdPrediction);
                    let mut actual_work = Some(budget);
                    let actual = terminal.match_len_with_work(input, &mut actual_work);
                    assert_eq!(
                        (actual, actual_work),
                        (expected.clone(), expected_work),
                        "{} {input:?} at {budget}",
                        terminal.identity
                    );
                    let mut canonical_work = Some(budget);
                    let canonical = buffered_match(
                        terminal,
                        input,
                        &mut canonical_work,
                        ReusedPrediction(&reused),
                    );
                    let mut reused_work = Some(budget);
                    let actual = terminal.match_len_reusing(input, &reused, &mut reused_work);
                    assert_eq!((actual, reused_work), (canonical, canonical_work));
                    assert_eq!(canonical_work, expected_work);
                }
            }
        }
        Ok(())
    }
}
