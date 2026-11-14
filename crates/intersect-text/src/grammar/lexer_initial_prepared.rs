//! Explicitly paid immutable first-frontier facts over the actual lexer NFA.
//! No input token, body result, source, model, profile or publication authority.
use super::*;

const HEADER_BYTES: usize = 128;
const RECORD_BYTES: usize = 32;
const ADMISSION_QUANTUM: usize = 64;

#[path = "lexer_initial_first_steps.rs"]
mod first_steps;
#[path = "lexer_terminal_initials.rs"]
pub(super) mod terminal_initials;

pub struct PreparedLexicalInitial<'program> {
    program: &'program LexerPrediction,
    initial: BTreeSet<usize>,
    owners: BTreeSet<usize>,
    accepting: BTreeSet<usize>,
    logical_bytes: usize,
    first_steps: Option<Vec<first_steps::FirstStep>>,
    terminal_initials: Option<Box<terminal_initials::PreparedTerminalInitials<'program>>>,
}
impl<'program> PreparedLexicalInitial<'program> {
    pub fn prepare(
        program: &'program LexerPrediction,
        work: &mut Option<usize>,
        storage_limit: usize,
    ) -> Result<Self, String> {
        Self::prepare_observed(program, work, storage_limit, None)
    }
    /// SAME canonical initial/first-step/terminal preparation, charged through
    /// the original observer as well as a finite local clamp. No input results.
    pub fn prepare_lexical_facts_observed<'selection>(
        program: &'program LexerPrediction,
        terminals: &'program BTreeMap<String, super::super::terminals::Terminal>,
        names: impl IntoIterator<Item = &'selection str>,
        work: &mut Option<usize>,
        storage_limit: usize,
        observation: &InvocationContext<'_>,
    ) -> Result<Self, String> {
        // Internal ceilings only narrow this explicit operation. They do not
        // replace/reset the original observer or grant a second invocation pool.
        const WORK_LIMIT: usize = 100_000;
        const FACT_LIMIT: usize = 1_048_576;
        let result: Result<Self, String> = (|| {
            if !observation.has_observer() {
                return Err("Observed lexical preparation requires an original observer".into());
            }
            let outer_remaining =
                work.ok_or("Lexical initial preparation requires a finite work pool")?;
            observation.checkpoint()?;
            let limit = outer_remaining.min(WORK_LIMIT);
            let mut local = Some(limit);
            let storage_limit = storage_limit.min(FACT_LIMIT);
            let attempt: Result<Self, String> = (|| {
                let initial =
                    Self::prepare_observed(program, &mut local, storage_limit, Some(observation))?;
                let initial = initial.with_first_steps_observed(
                    &mut local,
                    storage_limit,
                    Some(observation),
                )?;
                let result = initial.with_terminal_initials_observed(
                    terminals,
                    names,
                    &mut local,
                    storage_limit,
                    Some(observation),
                )?;
                observation.checkpoint()?;
                Ok(result)
            })();
            // Propagate every completed local debit on success OR failure. The
            // observer was already precharged before each canonical work unit.
            let local_remaining =
                local.ok_or("Observed lexical preparation lost its finite pool")?;
            let consumed = limit
                .checked_sub(local_remaining)
                .ok_or("Observed lexical preparation balance increased")?;
            *work = Some(
                outer_remaining
                    .checked_sub(consumed)
                    .ok_or("Observed lexical preparation balance underflow")?,
            );
            attempt
        })();
        if let Err(error) = &result {
            observation.failed_result(error);
        }
        result
    }
    fn prepare_observed(
        program: &'program LexerPrediction,
        work: &mut Option<usize>,
        storage_limit: usize,
        observation: Option<&InvocationContext<'_>>,
    ) -> Result<Self, String> {
        if work.is_none() {
            return Err("Lexical initial preparation requires a finite work pool".into());
        }
        LexerPrediction::consume_observed(work, observation)?;
        let initial = program.closure_observed(program.roots.iter().copied(), work, observation)?;
        let (owners, accepting) = program.frontier_owners_observed(&initial, work, observation)?;
        let records = initial
            .len()
            .checked_add(owners.len())
            .and_then(|n| n.checked_add(accepting.len()))
            .ok_or("Lexical initial preparation record overflow")?;
        let logical_bytes = records
            .checked_mul(RECORD_BYTES)
            .and_then(|n| n.checked_add(HEADER_BYTES))
            .ok_or("Lexical initial preparation storage overflow")?;
        if logical_bytes > storage_limit {
            return Err("Prepared lexical initial storage budget exceeded".into());
        }
        for _ in 0..logical_bytes.div_ceil(ADMISSION_QUANTUM) {
            LexerPrediction::consume_observed(work, observation)?;
        }
        // Traversal sets remain working temporaries until this admission; no
        // physical peak/RSS bound is claimed. No fact holder publishes on denial.
        if let Some(observation) = observation {
            observation.retain_storage(logical_bytes)?;
        }
        Ok(Self {
            program,
            initial,
            owners,
            accepting,
            logical_bytes,
            first_steps: None,
            terminal_initials: None,
        })
    }
    pub fn logical_bytes(&self) -> usize {
        self.logical_bytes
    }
    pub(crate) fn check(
        &self,
        program: &LexerPrediction,
        work: &mut Option<usize>,
    ) -> Result<(), String> {
        // Every invocation pays its binding before consulting prepared facts.
        if work.is_none() {
            return Err("Prepared lexical prediction requires a finite work pool".into());
        }
        LexerPrediction::consume(work)?;
        if !std::ptr::eq(self.program, program) {
            return Err(
                "Prepared lexical initial differs from the exact live lexical program".into(),
            );
        }
        Ok(())
    }
    #[allow(clippy::type_complexity)]
    pub(super) fn seed(
        &self,
        program: &LexerPrediction,
        work: &mut Option<usize>,
    ) -> Result<(&BTreeSet<usize>, &BTreeSet<usize>, &BTreeSet<usize>), String> {
        self.check(program, work)?;
        LexerPrediction::consume(work)?;
        Ok((&self.initial, &self.owners, &self.accepting))
    }
}
const _: () = assert!(std::mem::size_of::<PreparedLexicalInitial<'static>>() <= HEADER_BYTES);

#[cfg(test)]
#[path = "lexer_initial_prepared_tests.rs"]
mod tests;

#[cfg(test)]
mod observed_tests {
    use super::*;
    use crate::grammar;
    use crate::grammar::terminals;
    use crate::grammar::terminals::Expression;
    use crate::grammar::terminals::Operation;
    use crate::grammar::terminals::Terminal;
    use intersect_model::definitions::conversion::InvocationObserver;
    use intersect_model::definitions::conversion::ProviderId;
    use std::{cell::Cell, rc::Rc};

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
            datatype: grammar::Datatype::EString,
            nominal_return: None,
            enumeration: None,
            identity: name.into(),
            start: 0,
            end: 1,
            body,
        }
    }
    fn fixture() -> Result<(BTreeMap<String, Terminal>, LexerPrediction), String> {
        let mut word = expr(Operation::Range(97, 122));
        word.cardinality = '+';
        let mut space = literal(" ");
        space.cardinality = '+';
        let block = expr(Operation::Sequence(vec![
            literal("/*"),
            expr(Operation::UntilOperand {
                operand: Box::new(literal("*/")),
                decision: None,
            }),
        ]));
        let mut repeated = literal("x");
        repeated.cardinality = '+';
        let end = expr(Operation::Sequence(vec![repeated, expr(Operation::Eof)]));
        let mut map = BTreeMap::from([
            ("WORD".into(), terminal("WORD", word)),
            ("SPACE".into(), terminal("SPACE", space)),
            ("BLOCK".into(), terminal("BLOCK", block)),
            ("END".into(), terminal("END", end)),
            ("FIXED".into(), terminal("FIXED", literal("="))),
        ]);
        terminals::prepare(&mut map, grammar::Profile::NativeV67).map_err(|e| e.message)?;
        let expressions = std::iter::once(literal("if"))
            .chain(map.values().map(|t| t.body.clone()))
            .collect::<Vec<_>>();
        let mut program = LexerPrediction::terminal_choices(&expressions)?;
        program.alternatives[0] = Alternative::Keyword("if".into());
        for (index, name) in map.keys().enumerate() {
            program.alternatives[index + 1] = Alternative::Terminal(name.clone());
        }
        Ok((map, program))
    }
    const NAMES: [&str; 4] = ["WORD", "SPACE", "BLOCK", "END"];
    const POOL: usize = 200_000;
    const CAP: usize = 1_048_576;
    struct Meter {
        work: Cell<usize>,
        storage: Cell<usize>,
        invocations: Cell<usize>,
        failures: Cell<usize>,
        failed: Cell<bool>,
        cancel_at: Cell<Option<usize>>,
        work_limit: usize,
        storage_limit: usize,
    }
    impl Meter {
        fn new(work_limit: usize, storage_limit: usize) -> Self {
            Self {
                work: Cell::new(0),
                storage: Cell::new(0),
                invocations: Cell::new(0),
                failures: Cell::new(0),
                failed: Cell::new(false),
                cancel_at: Cell::new(None),
                work_limit,
                storage_limit,
            }
        }
    }
    impl InvocationObserver for Meter {
        fn before_invoke(&self, _: &ProviderId) -> Result<(), String> {
            self.checkpoint()?;
            self.invocations.set(self.invocations.get() + 1);
            Ok(())
        }
        fn checkpoint(&self) -> Result<(), String> {
            if self.failed.get() {
                return Err("latched observed preparation failure".into());
            }
            if self
                .cancel_at
                .get()
                .is_some_and(|limit| self.work.get() >= limit)
            {
                return Err("cancelled observed preparation".into());
            }
            Ok(())
        }
        fn work(&self, units: usize) -> Result<(), String> {
            self.checkpoint()?;
            let next = self
                .work
                .get()
                .checked_add(units)
                .ok_or("observer work overflow")?;
            if next > self.work_limit {
                return Err("observer work denied".into());
            }
            self.work.set(next);
            Ok(())
        }
        fn retain_storage(&self, bytes: usize) -> Result<(), String> {
            self.checkpoint()?;
            let next = self
                .storage
                .get()
                .checked_add(bytes)
                .ok_or("observer storage overflow")?;
            if next > self.storage_limit {
                return Err("observer storage denied".into());
            }
            self.storage.set(next);
            Ok(())
        }
        fn failed_result(&self, _: &str) {
            self.failed.set(true);
            self.failures.set(self.failures.get() + 1);
        }
    }
    fn old<'a>(
        program: &'a LexerPrediction,
        map: &'a BTreeMap<String, Terminal>,
        names: impl IntoIterator<Item = &'static str>,
        work: &mut Option<usize>,
        cap: usize,
    ) -> Result<PreparedLexicalInitial<'a>, String> {
        PreparedLexicalInitial::prepare(program, work, cap)?
            .with_first_steps(work, cap)?
            .with_terminal_initials(map, names, work, cap)
    }
    #[allow(clippy::type_complexity)]
    fn shown(
        result: Result<
            (
                Vec<grammar::Token<std::borrow::Cow<'_, str>>>,
                Vec<grammar::LexicalIssue>,
                Vec<grammar::DocumentTokenRange>,
            ),
            (usize, String),
        >,
    ) -> Result<serde_json::Value, (usize, String)> {
        result.map(|(tokens, issues, ranges)| serde_json::json!({
            "tokens": tokens.iter().map(|t| serde_json::json!({"text":t.text,"start":t.start,
                "end":t.end,"quoted":t.quoted,"alternative":t.lexical_alternative})).collect::<Vec<_>>(),
            "issues":issues,"ranges":ranges}))
    }
    #[test]
    fn observed_lexical_facts_same_work_storage_and_whole_canonical_results() -> Result<(), String>
    {
        let (map, program) = fixture()?;
        let mut plain_work = Some(POOL);
        let plain = old(&program, &map, NAMES, &mut plain_work, CAP)?;
        let meter = Rc::new(Meter::new(POOL, CAP));
        let context = InvocationContext::new(meter.clone());
        let mut observed_work = Some(POOL);
        let observed = PreparedLexicalInitial::prepare_lexical_facts_observed(
            &program,
            &map,
            NAMES,
            &mut observed_work,
            CAP,
            &context,
        )?;
        assert_eq!(plain_work, observed_work);
        assert_eq!(
            meter.work.get(),
            POOL - observed_work.ok_or("finite pool lost")?
        );
        assert_eq!(meter.storage.get(), observed.logical_bytes());
        assert_eq!(plain.logical_bytes(), observed.logical_bytes());
        assert_eq!(
            plain.first_step_intervals(),
            observed.first_step_intervals()
        );
        assert_eq!(
            plain.terminal_initial_programs(),
            observed.terminal_initial_programs()
        );
        assert!(observed.terminal_initial_programs() >= NAMES.len());
        assert_eq!((meter.invocations.get(), meter.failures.get()), (0, 0));
        for text in [
            "",
            "if",
            "iff",
            "identifier",
            "  ",
            "=",
            "xxx",
            "xy",
            "/*a*/",
            "/*😀*/",
            "/*unclosed",
            "😀",
            "abc=def",
        ] {
            let cold = shown(grammar::lex_document(
                text,
                Some(100),
                &map,
                &program,
                &mut Some(POOL),
            ));
            let plain_result = shown(grammar::lex_document_with_prepared_initial(
                text,
                Some(100),
                &map,
                &program,
                &plain,
                &mut Some(POOL),
            ));
            let observed_result = shown(grammar::lex_document_with_prepared_initial(
                text,
                Some(100),
                &map,
                &program,
                &observed,
                &mut Some(POOL),
            ));
            assert_eq!(cold, plain_result, "cold/prepared {text:?}");
            assert_eq!(plain_result, observed_result, "OFF/ON {text:?}");
        }
        Ok(())
    }
    #[test]
    fn observed_lexical_facts_exact_short_denial_cancel_and_sticky_retry() -> Result<(), String> {
        let (map, program) = fixture()?;
        let mut work = Some(POOL);
        let plain = old(&program, &map, NAMES, &mut work, CAP)?;
        let used = POOL - work.ok_or("finite pool lost")?;
        let bytes = plain.logical_bytes();
        assert!(used > 5 && used < 100_000);
        for allowance in [0, used - 1, used] {
            let meter = Rc::new(Meter::new(POOL, CAP));
            let context = InvocationContext::new(meter.clone());
            let mut local = Some(allowance);
            let result = PreparedLexicalInitial::prepare_lexical_facts_observed(
                &program, &map, NAMES, &mut local, bytes, &context,
            );
            assert_eq!(result.is_ok(), allowance == used);
            assert_eq!(
                meter.work.get(),
                allowance - local.ok_or("finite pool lost")?
            );
            assert_eq!(meter.failed.get(), allowance != used);
        }
        for cap in [bytes - 1, bytes] {
            let meter = Rc::new(Meter::new(POOL, CAP));
            let result = PreparedLexicalInitial::prepare_lexical_facts_observed(
                &program,
                &map,
                NAMES,
                &mut Some(POOL),
                cap,
                &InvocationContext::new(meter.clone()),
            );
            assert_eq!(result.is_ok(), cap == bytes);
            assert!(meter.storage.get() <= cap);
        }
        for (work_cap, storage_cap) in [(0, CAP), (POOL, 0)] {
            let meter = Rc::new(Meter::new(work_cap, storage_cap));
            let mut local = Some(POOL);
            assert!(
                PreparedLexicalInitial::prepare_lexical_facts_observed(
                    &program,
                    &map,
                    NAMES,
                    &mut local,
                    CAP,
                    &InvocationContext::new(meter.clone())
                )
                .is_err()
            );
            assert_eq!(meter.work.get(), POOL - local.ok_or("finite pool lost")?);
            assert_eq!(meter.storage.get(), 0);
            assert!(meter.failed.get());
        }
        let meter = Rc::new(Meter::new(POOL, CAP));
        meter.cancel_at.set(Some(5));
        let context = InvocationContext::new(meter.clone());
        let mut local = Some(POOL);
        assert!(
            PreparedLexicalInitial::prepare_lexical_facts_observed(
                &program, &map, NAMES, &mut local, CAP, &context
            )
            .is_err()
        );
        assert_eq!(meter.work.get(), 5);
        assert_eq!(local, Some(POOL - 5));
        let retained = meter.storage.get();
        meter.cancel_at.set(None);
        assert!(
            PreparedLexicalInitial::prepare_lexical_facts_observed(
                &program, &map, NAMES, &mut local, CAP, &context
            )
            .is_err()
        );
        assert_eq!((meter.work.get(), meter.storage.get()), (5, retained));
        let fresh = Rc::new(Meter::new(POOL, CAP));
        assert!(
            PreparedLexicalInitial::prepare_lexical_facts_observed(
                &program,
                &map,
                NAMES,
                &mut None,
                CAP,
                &InvocationContext::new(fresh.clone())
            )
            .is_err()
        );
        assert_eq!(fresh.work.get(), 0);
        assert!(
            PreparedLexicalInitial::prepare_lexical_facts_observed(
                &program,
                &map,
                NAMES,
                &mut Some(POOL),
                CAP,
                &InvocationContext::default()
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn observed_lexical_facts_selection_errors_and_foreign_identity() -> Result<(), String> {
        let (map, program) = fixture()?;
        for names in [vec![], vec!["UNKNOWN"], vec!["WORD", "WORD"], vec!["FIXED"]] {
            let mut plain_work = Some(POOL);
            let expected = old(&program, &map, names.clone(), &mut plain_work, CAP).err();
            let mut observed_work = Some(POOL);
            let meter = Rc::new(Meter::new(POOL, CAP));
            let actual = PreparedLexicalInitial::prepare_lexical_facts_observed(
                &program,
                &map,
                names,
                &mut observed_work,
                CAP,
                &InvocationContext::new(meter.clone()),
            )
            .err();
            assert!(expected.is_some());
            assert_eq!(actual, expected);
            assert_eq!(plain_work, observed_work);
            assert_eq!(
                meter.work.get(),
                POOL - observed_work.ok_or("finite pool lost")?
            );
            assert!(meter.failed.get());
        }
        let meter = Rc::new(Meter::new(POOL, CAP));
        let prepared = PreparedLexicalInitial::prepare_lexical_facts_observed(
            &program,
            &map,
            NAMES,
            &mut Some(POOL),
            CAP,
            &InvocationContext::new(meter),
        )?;
        let foreign_program = program.clone();
        let foreign_map = map.clone();
        assert_eq!(
            serde_json::to_value(&program).map_err(|e| e.to_string())?,
            serde_json::to_value(&foreign_program).map_err(|e| e.to_string())?
        );
        assert_eq!(
            serde_json::to_value(&map).map_err(|e| e.to_string())?,
            serde_json::to_value(&foreign_map).map_err(|e| e.to_string())?
        );
        let reads = Cell::new(0);
        let input = std::iter::from_fn(|| {
            reads.set(reads.get() + 1);
            Some(97)
        });
        assert!(
            foreign_program
                .predict_units_with_initial::<true>(input, &mut Some(POOL), false, Some(&prepared))
                .is_err()
        );
        assert_eq!(reads.get(), 0);
        for text in ["", "identifier", "😀"] {
            assert!(
                grammar::lex_document_with_prepared_initial(
                    text,
                    Some(100),
                    &foreign_map,
                    &program,
                    &prepared,
                    &mut Some(POOL)
                )
                .is_err()
            );
        }
        Ok(())
    }
}
