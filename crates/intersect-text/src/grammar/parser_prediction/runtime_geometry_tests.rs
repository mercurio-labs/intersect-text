// Test-only passive counters; fixed traces compare original semantic callbacks.
#[test]
fn runtime_geometry_is_passive_for_real_first_full_follow_and_errors() -> Result<(), String> {
    use std::cell::{Cell, RefCell};
    type Callback = (u8, usize, usize, Option<usize>);
    struct Input<'a, const OBSERVED: bool> {
        inner: ReplayTokens<'a>,
        callbacks: RefCell<[Option<Callback>; 256]>,
        count: Cell<usize>,
        invalid: Cell<bool>,
        failure: u8,
        phases: usize,
        geometry: [usize; 17],
        max_return: usize,
    }
    const KINDS: [&str; 17] = [
        "decision_enter",
        "completion_enter",
        "initial_row",
        "pending",
        "row",
        "copy_visited",
        "visited_unique",
        "visited_duplicate",
        "pop",
        "copy_pop",
        "follow_fanout",
        "follow_step",
        "copy_follow",
        "copy_edge",
        "push",
        "frontier",
        "accepting",
    ];
    impl<'a, const OBSERVED: bool> Input<'a, OBSERVED> {
        fn new(tokens: &'a [PredictionSymbol], failure: u8) -> Self {
            Self {
                inner: ReplayTokens::new(tokens),
                callbacks: RefCell::new([None; 256]),
                count: Cell::new(0),
                invalid: Cell::new(false),
                failure,
                phases: 0,
                geometry: [0; 17],
                max_return: 0,
            }
        }
        fn note(&self, kind: u8, a: usize, b: usize, work: Option<usize>) {
            let index = self.count.get();
            let Ok(mut values) = self.callbacks.try_borrow_mut() else {
                self.invalid.set(true);
                return;
            };
            let Some(slot) = values.get_mut(index) else {
                self.invalid.set(true);
                return;
            };
            *slot = Some((kind, a, b, work));
            if let Some(next) = index.checked_add(1) {
                self.count.set(next);
            } else {
                self.invalid.set(true);
            }
        }
    }
    impl<const OBSERVED: bool> PredictionInput for Input<'_, OBSERVED> {
        fn runtime_work_diagnostics(&self) -> bool {
            OBSERVED
        }
        fn runtime_work_snapshot(&mut self, _: &'static str, _: Option<usize>) {
            if let Some(value) = self.phases.checked_add(1) {
                self.phases = value;
            } else {
                self.invalid.set(true);
            }
        }
        fn runtime_work_geometry(&mut self, event: RuntimeWorkGeometry) {
            let Some(index) = KINDS.iter().position(|name| *name == event.kind) else {
                self.invalid.set(true);
                return;
            };
            if let Some(value) = self.geometry[index].checked_add(1) {
                self.geometry[index] = value;
            } else {
                self.invalid.set(true);
            }
            self.max_return = self.max_return.max(event.return_depth);
            if event.return_depth > 128 || event.base_depth > 128 {
                self.invalid.set(true);
            }
        }
        fn replay_position(&self) -> Option<usize> {
            self.inner.replay_position()
        }
        fn read(&mut self, work: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
            self.note(0, self.inner.position, 0, *work);
            if self.failure == 1 && self.inner.position == 1 {
                return Err("passive control read failure".into());
            }
            self.inner.read(work)
        }
        fn checkpoint(&self) -> Result<usize, String> {
            self.note(1, self.inner.position, 0, None);
            if self.failure == 2 {
                return Err("passive control checkpoint failure".into());
            }
            self.inner.checkpoint()
        }
        fn rewind(&mut self, value: usize) -> Result<(), String> {
            self.note(2, self.inner.position, value, None);
            if self.failure == 3 {
                return Err("passive control rewind failure".into());
            }
            self.inner.rewind(value)
        }
    }
    let full_program = full_oracle_program(false);
    let mut first_body = keyword("a");
    first_body.first_set_predicated = true;
    let mut first_grammar = grammar(choice(vec![first_body, keyword("b")]), vec![]);
    live_compile(&mut first_grammar);
    let first_program = first_grammar
        .parser_prediction
        .as_ref()
        .ok_or("First program missing")?;
    let follow_grammar = live_component_grammar(3);
    let follow_program = follow_grammar
        .parser_prediction
        .as_ref()
        .ok_or("FOLLOW program missing")?;
    let mut duplicate_program = first_program.clone();
    let state = *duplicate_program
        .decisions
        .get("rule/Entry/choice")
        .and_then(|d| d.entries.first())
        .ok_or("duplicate entry")?;
    let row = duplicate_program
        .states
        .get_mut(state)
        .ok_or("duplicate state")?;
    let edge = row.edges.first().cloned().ok_or("duplicate edge")?;
    row.edges.push(edge); // Same alternative/full configuration reaches one row twice.
    let mut saw_full = false;
    let mut saw_first = false;
    let mut saw_follow = false;
    let mut saw_returns = false;
    let mut saw_duplicate = false;
    for (case, program, decision, text) in [
        ("full", &full_program, "rule/Entry/choice", "a a b b"),
        ("full_fallback", &full_program, "rule/Entry/choice", "a c"),
        ("first", first_program, "rule/Entry/choice", "a"),
        ("follow", follow_program, "rule/Prefix/repeat", "next"),
        ("duplicate", &duplicate_program, "rule/Entry/choice", "a"),
        ("unknown", first_program, "missing/choice", "a"),
    ] {
        let tokens = symbols(text);
        for failure in 0..4 {
            for grant in [0, 9, 100_000] {
                let mut off = Input::<false>::new(&tokens, failure);
                let mut on = Input::<true>::new(&tokens, failure);
                let mut off_work = Some(grant);
                let mut on_work = Some(grant);
                let expected = program.predict_with_replay_input(decision, &mut off, &mut off_work);
                let actual = program.predict_with_replay_input(decision, &mut on, &mut on_work);
                println!(
                    "PASSIVE_RUNTIME_CORE_RAW {}",
                    serde_json::json!({
                    "case":case,"failure":failure,"grant":grant,"off":expected,"on":actual,
                    "off_remaining":off_work,"on_remaining":on_work,"phase_samples":on.phases,
                    "geometry_counts":on.geometry,"max_return_depth":on.max_return})
                );
                assert_eq!(actual, expected);
                assert_eq!(on_work, off_work);
                assert_eq!(on.inner.position, off.inner.position);
                assert_eq!(on.count.get(), off.count.get());
                assert_eq!(*on.callbacks.borrow(), *off.callbacks.borrow());
                assert!(!on.invalid.get() && !off.invalid.get());
                assert_eq!(off.phases, 0);
                assert_eq!(off.geometry, [0; 17]);
                if failure == 0 && grant == 100_000 {
                    saw_full |= case == "full" && on.geometry[1] > 0;
                    saw_first |= case == "first" && on.phases > 0 && actual == Ok(Some(0));
                    saw_follow |= case == "follow" && on.geometry[11] > 0;
                    saw_returns |= on.geometry[14] > 0 && on.geometry[8] > 0 && on.max_return > 0;
                    saw_duplicate |= case == "duplicate" && on.geometry[7] > 0;
                }
            }
        }
    }
    assert!(saw_full && saw_first && saw_follow && saw_returns && saw_duplicate);
    Ok(())
}
