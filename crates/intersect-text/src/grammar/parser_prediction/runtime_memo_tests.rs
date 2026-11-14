// Copyright (c) 2026 Mercurio Labs
// SPDX-License-Identifier: MIT
//! Handwritten tests (not generated) of the runtime prediction memo: what a
//! retained selection depends on, when it is reused, and what it charges.
use super::runtime_memo::{Decision, memoized};
use super::*;
use std::collections::HashMap;

/// Replayable input over fixed symbols with hidden tokens between them (odd
/// positions), a furthest-read mark, and retained nested-decision answers.
#[derive(Default)]
struct Input {
    symbols: Vec<PredictionSymbol>,
    position: usize,
    furthest: usize,
    viability: HashMap<ViabilityKey, bool>,
}

impl Input {
    fn new(words: &[&str]) -> Self {
        Self {
            symbols: words
                .iter()
                .map(|word| PredictionSymbol::Keyword((*word).into()))
                .collect(),
            ..Self::default()
        }
    }
    /// The token index after `n` symbols: every symbol is followed by one
    /// hidden token, so positions do not equal symbol ordinals.
    fn after(n: usize) -> usize {
        n * 2
    }
}

impl PredictionInput for Input {
    fn replay_position(&self) -> Option<usize> {
        Some(self.position)
    }
    fn lookahead_mark(&self) -> Option<usize> {
        Some(self.furthest)
    }
    fn restore_lookahead(&mut self, mark: usize) {
        self.furthest = mark;
    }
    fn viability(&self, key: &ViabilityKey) -> Option<bool> {
        self.viability.get(key).copied()
    }
    fn record_viability(&mut self, keys: Vec<ViabilityKey>, accepted: bool) {
        for key in keys {
            self.viability.insert(key, accepted);
        }
    }
    fn read(&mut self, remaining: &mut Option<usize>) -> Result<Option<PredictionSymbol>, String> {
        charge(remaining)?;
        let index = self.position / 2;
        let symbol = match index.cmp(&self.symbols.len()) {
            std::cmp::Ordering::Less => self.symbols.get(index).cloned(),
            std::cmp::Ordering::Equal => Some(PredictionSymbol::End),
            std::cmp::Ordering::Greater => None,
        };
        if index <= self.symbols.len() {
            self.position += 2;
        }
        self.furthest = self.furthest.max(self.position);
        Ok(symbol)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        Ok(self.position)
    }
    fn rewind(&mut self, checkpoint: usize) -> Result<(), String> {
        self.position = checkpoint;
        Ok(())
    }
}

fn decision<'a>(key: &'a str, returns: &'a [usize]) -> Decision<'a> {
    Decision {
        owner: (1, 2),
        key,
        returns,
        route: 1,
        retain_none: !key.ends_with("choice"),
    }
}

fn keyword(word: &str) -> Option<PredictionSymbol> {
    Some(PredictionSymbol::Keyword(word.into()))
}

/// A prediction that reads `reads` symbols, charges `cost` more work and
/// selects alternative 1 when the first symbol is `a`, else 0.
fn predict(
    reads: usize,
    cost: usize,
) -> impl FnMut(&mut dyn PredictionInput, &mut Option<usize>) -> Result<Option<usize>, String> {
    move |input, work| {
        let mut first = None;
        for _ in 0..reads {
            let symbol = input.read(work)?;
            first.get_or_insert(symbol);
        }
        for _ in 0..cost {
            charge(work)?;
        }
        Ok(Some(usize::from(first.flatten() == keyword("a"))))
    }
}

/// Runs `memoized` and counts the uncached predictions it made.
fn run(
    memo: &RuntimePredictionMemo,
    decision: Decision<'_>,
    input: &mut Input,
    work: &mut Option<usize>,
    calls: &mut usize,
    mut uncached: impl FnMut(
        &mut dyn PredictionInput,
        &mut Option<usize>,
    ) -> Result<Option<usize>, String>,
) -> Result<Option<usize>, String> {
    memoized(memo, decision, input, work, |input, work| {
        *calls += 1;
        uncached(input, work)
    })
}

#[test]
fn same_lookahead_reuses_the_selection_and_charges_its_work() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut first = Input::new(&["a", "b", "c"]);
    let mut work = Some(1_000);
    let selected = run(
        &memo,
        decision("rule/R/repeat", &[7]),
        &mut first,
        &mut work,
        &mut calls,
        predict(2, 10),
    )?;
    assert_eq!((selected, calls, work), (Some(1), 1, Some(1_000 - 12)));
    assert_eq!(first.furthest, Input::after(2));
    // The same two symbols, then different input: the prediction never
    // looked past them, so the selection is reused.
    let mut second = Input::new(&["a", "b", "x", "y"]);
    second.furthest = 1;
    let mut work = Some(1_000);
    let selected = run(
        &memo,
        decision("rule/R/repeat", &[7]),
        &mut second,
        &mut work,
        &mut calls,
        predict(2, 10),
    )?;
    assert_eq!((selected, calls, work), (Some(1), 1, Some(1_000 - 12)));
    // The furthest read is the one the uncached prediction leaves.
    assert_eq!(second.furthest, Input::after(2));
    let stats = memo.stats();
    assert_eq!((stats.hits, stats.misses, stats.inserted), (1, 1, 1));
    Ok(())
}

#[test]
fn other_lookahead_context_or_decision_predicts_uncached() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut work = Some(1_000);
    run(
        &memo,
        decision("rule/R/repeat", &[7]),
        &mut Input::new(&["a", "b"]),
        &mut work,
        &mut calls,
        predict(2, 0),
    )?;
    // A different second symbol, caller returns, decision, or grammar.
    let mut other = Input::new(&["a", "c"]);
    other.furthest = 1;
    let selected = run(
        &memo,
        decision("rule/R/repeat", &[7]),
        &mut other,
        &mut work,
        &mut calls,
        predict(2, 0),
    )?;
    assert_eq!((selected, calls), (Some(1), 2));
    // The probe's reads do not survive a miss: furthest is the uncached one.
    assert_eq!(other.furthest, Input::after(2));
    run(
        &memo,
        decision("rule/R/repeat", &[8]),
        &mut Input::new(&["a", "b"]),
        &mut work,
        &mut calls,
        predict(2, 0),
    )?;
    run(
        &memo,
        decision("rule/S/repeat", &[7]),
        &mut Input::new(&["a", "b"]),
        &mut work,
        &mut calls,
        predict(2, 0),
    )?;
    let mut foreign = decision("rule/R/repeat", &[7]);
    foreign.owner = (3, 2);
    run(
        &memo,
        foreign,
        &mut Input::new(&["a", "b"]),
        &mut work,
        &mut calls,
        predict(2, 0),
    )?;
    assert_eq!(calls, 5);
    Ok(())
}

#[test]
fn end_of_input_and_reads_past_it_are_part_of_the_lookahead() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut work = Some(1_000);
    // Reads `a`, End, then past the end.
    run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        predict(3, 0),
    )?;
    run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        predict(3, 0),
    )?;
    assert_eq!(calls, 1);
    run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a", "b"]),
        &mut work,
        &mut calls,
        predict(3, 0),
    )?;
    assert_eq!(calls, 2);
    Ok(())
}

#[test]
fn errors_and_failed_choices_are_not_retained() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut work = Some(1_000);
    let fail = |input: &mut dyn PredictionInput, work: &mut Option<usize>| {
        input.read(work)?;
        Err::<Option<usize>, String>("no".into())
    };
    for _ in 0..2 {
        assert!(
            run(
                &memo,
                decision("rule/R/repeat", &[]),
                &mut Input::new(&["a"]),
                &mut work,
                &mut calls,
                fail
            )
            .is_err()
        );
    }
    let none = |input: &mut dyn PredictionInput, work: &mut Option<usize>| {
        input.read(work)?;
        Ok(None)
    };
    for _ in 0..2 {
        run(
            &memo,
            decision("rule/R/choice", &[]),
            &mut Input::new(&["a"]),
            &mut work,
            &mut calls,
            none,
        )?;
    }
    assert_eq!(calls, 4);
    // A repeat that exits (`None`) is retained.
    for _ in 0..2 {
        run(
            &memo,
            decision("rule/R/repeat", &[]),
            &mut Input::new(&["b"]),
            &mut work,
            &mut calls,
            none,
        )?;
    }
    assert_eq!(calls, 5);
    Ok(())
}

fn viability_key(position: usize) -> ViabilityKey {
    ViabilityKey {
        position,
        stop: 9,
        rows: vec![(4, vec![7], true, false)],
    }
}

/// Reads one symbol, then asks the input whether the rows were already
/// carried to the end; without an answer it "carries" them itself by reading
/// to the end and records the verdict.
fn viable(
    input: &mut dyn PredictionInput,
    work: &mut Option<usize>,
) -> Result<Option<usize>, String> {
    input.read(work)?;
    let key = viability_key(input.replay_position().ok_or("position")?);
    match input.viability(&key) {
        Some(accepted) => Ok(accepted.then_some(0)),
        None => {
            while input.read(work)?.is_some() {}
            input.record_viability(vec![key], true);
            Ok(Some(0))
        }
    }
}

#[test]
fn an_answer_from_the_input_memo_is_required_on_reuse() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut work = Some(1_000);
    // The answer is given by the input, not produced by this prediction.
    let mut first = Input::new(&["a", "b", "c"]);
    first.viability.insert(viability_key(Input::after(1)), true);
    run(
        &memo,
        decision("rule/R/choice", &[]),
        &mut first,
        &mut work,
        &mut calls,
        viable,
    )?;
    assert_eq!(calls, 1);
    // Same symbol and the same answer at the same symbol ordinal, although
    // hidden tokens move the absolute position: reused.
    let mut second = Input::new(&["q", "q", "a", "z"]);
    second.position = 4;
    second
        .viability
        .insert(viability_key(4 + Input::after(1)), true);
    run(
        &memo,
        decision("rule/R/choice", &[]),
        &mut second,
        &mut work,
        &mut calls,
        viable,
    )?;
    assert_eq!(calls, 1);
    // Without the answer the uncached prediction runs.
    run(
        &memo,
        decision("rule/R/choice", &[]),
        &mut Input::new(&["a", "z"]),
        &mut work,
        &mut calls,
        viable,
    )?;
    assert_eq!(calls, 2);
    // A different answer is not the retained one either.
    let mut rejected = Input::new(&["a"]);
    rejected
        .viability
        .insert(viability_key(Input::after(1)), false);
    let selected = run(
        &memo,
        decision("rule/R/choice", &[]),
        &mut rejected,
        &mut work,
        &mut calls,
        viable,
    )?;
    assert_eq!((selected, calls), (None, 3));
    Ok(())
}

#[test]
fn an_answer_the_prediction_produced_itself_is_not_a_requirement() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut work = Some(1_000);
    // Carries the rows to the end itself (reading `a b End`) and records it.
    run(
        &memo,
        decision("rule/R/choice", &[]),
        &mut Input::new(&["a", "b"]),
        &mut work,
        &mut calls,
        |input, work| {
            let selected = viable(input, work)?;
            // Asking again is answered by its own record.
            let key = viability_key(Input::after(1));
            assert_eq!(input.viability(&key), Some(true));
            Ok(selected)
        },
    )?;
    // Reused for the same complete lookahead without any input answer.
    run(
        &memo,
        decision("rule/R/choice", &[]),
        &mut Input::new(&["a", "b"]),
        &mut work,
        &mut calls,
        viable,
    )?;
    assert_eq!(calls, 1);
    Ok(())
}

#[test]
fn verify_mode_predicts_uncached_and_reports_a_difference() -> Result<(), String> {
    let memo = RuntimePredictionMemo::new(RuntimePredictionMemoMode::Verify);
    let mut calls = 0;
    let mut work = Some(1_000);
    run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        predict(1, 0),
    )?;
    run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        predict(1, 0),
    )?;
    assert_eq!((calls, memo.stats().verified), (2, 1));
    // An uncached prediction that disagrees with the retained selection.
    let disagree = |input: &mut dyn PredictionInput, work: &mut Option<usize>| {
        input.read(work)?;
        Ok(Some(5))
    };
    let error = run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        disagree,
    );
    assert!(error.is_err_and(|e| e.contains("Runtime prediction memo selected")));
    // Off never uses the memo.
    memo.set_mode(RuntimePredictionMemoMode::Off);
    run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        predict(1, 0),
    )?;
    assert_eq!(calls, 4);
    Ok(())
}

#[test]
fn a_hit_needs_the_recorded_work_and_otherwise_predicts_uncached() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut work = Some(1_000);
    run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        predict(1, 50),
    )?;
    // Less than the 51 recorded units: the uncached prediction runs and
    // exhausts the pool exactly as it would without the memo.
    let mut work = Some(20);
    let result = run(
        &memo,
        decision("rule/R/repeat", &[]),
        &mut Input::new(&["a"]),
        &mut work,
        &mut calls,
        predict(1, 50),
    );
    assert!(result.is_err_and(|e| e.contains("budget")));
    assert_eq!((calls, memo.stats().budget_fallbacks), (2, 1));
    Ok(())
}

#[test]
fn the_memo_stops_growing_at_its_bound() -> Result<(), String> {
    let memo = RuntimePredictionMemo::with_capacity(RuntimePredictionMemoMode::On, 0);
    let mut calls = 0;
    let mut work = Some(1_000);
    for _ in 0..2 {
        run(
            &memo,
            decision("rule/R/repeat", &[]),
            &mut Input::new(&["a"]),
            &mut work,
            &mut calls,
            predict(1, 0),
        )?;
    }
    let stats = memo.stats();
    assert_eq!((calls, stats.over_capacity, stats.bytes), (2, 2, 0));
    Ok(())
}

#[test]
fn lookahead_longer_than_the_bound_is_not_retained() -> Result<(), String> {
    let memo = RuntimePredictionMemo::default();
    let mut calls = 0;
    let mut work = Some(10_000);
    let words = vec!["w"; 200];
    for _ in 0..2 {
        run(
            &memo,
            decision("rule/R/repeat", &[]),
            &mut Input::new(&words),
            &mut work,
            &mut calls,
            predict(150, 0),
        )?;
    }
    assert_eq!((calls, memo.stats().uncacheable), (2, 2));
    Ok(())
}
