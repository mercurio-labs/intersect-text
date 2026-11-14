//! Optional bounded bookkeeping of the existing recursive-analysis work bucket.
use super::{CapturedSource, CapturedText, Coverage, PredictionObservation, RunStage};
use crate::grammar::Grammar;
use intersect_model::definitions::bundle::SourceId;
use std::cell::Cell;
#[path = "selected_static.rs"]
mod selected_static;
pub use selected_static::SelectedStaticSnapshot;

#[derive(Clone, Copy)]
pub struct AnalysisObservers<'a> {
    pub phases: &'a PredictionObservation,
    pub recursive_work: Option<&'a RecursiveAnalysisCounters>,
}
impl<'a> AnalysisObservers<'a> {
    pub const fn phases(phases: &'a PredictionObservation) -> Self {
        Self {
            phases,
            recursive_work: None,
        }
    }
    pub fn reset(self) {
        self.phases.reset();
        if let Some(work) = self.recursive_work {
            work.reset();
        }
    }
    pub fn not_instrumented(self, reason: Coverage) {
        self.phases.not_instrumented(reason);
        if let Some(work) = self.recursive_work {
            work.coverage(reason);
        }
    }
    pub(crate) fn analysis_reached(self) {
        self.phases.analysis_reached();
        if let Some(work) = self.recursive_work {
            work.coverage(Coverage::CoreSourceAnalysis);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[repr(usize)]
pub enum RecursiveWorkCategory {
    FrontierPop,
    ClosureVisit,
    ClosureReservation,
    CallerlessFollowEdge,
    TransitionEdge,
    WitnessState,
    WitnessEdge,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum ScheduleSite {
    Seed,
    Return,
    Follow,
    Successor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(crate) enum ScheduleEvent {
    Absent,
    Retained,
    Duplicate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct RecursiveWorkCounts {
    pub granted: [u64; 7],
    pub denied: [u64; 7],
    /// Four existing scheduling sites; absent candidates, retained, duplicates.
    pub scheduling: [[u64; 3]; 4],
    pub legacy_unique_pops: u64,
    pub legacy_duplicate_pops: u64,
    pub closure_pops: u64,
    pub single_alternative_frontiers: u64,
    pub visited_frontiers: u64,
    pub witness_hits: u64,
    pub witness_misses: u64,
    pub witness_miss_true: u64,
    pub witness_miss_false: u64,
    pub witness_miss_errors: u64,
    pub witness_miss_granted_work: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct RecursiveStageCounts {
    pub program_invocations: u64,
    pub programs_completed: u64,
    pub decisions_started: u64,
    pub completed_true: u64,
    pub completed_false: u64,
    pub failed: u64,
    pub skipped_metadata: u64,
    pub skipped_without_entry_guard: u64,
    pub work: RecursiveWorkCounts,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecursiveDecisionCounts {
    pub program_ordinal: u64,
    pub decision_ordinal: u64,
    pub stage: RunStage,
    pub source: Option<CapturedSource>,
    pub owner_rule: Option<CapturedText<128>>,
    pub declaration_span: Option<(usize, usize)>,
    pub decision: Option<CapturedText<256>>,
    pub capture_unavailable: bool,
    pub failed: bool,
    pub completed: Option<bool>,
    pub work: RecursiveWorkCounts,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecursiveProgramCounts {
    pub ordinal: u64,
    pub stage: RunStage,
    pub initial: usize,
    pub remaining: usize,
    pub granted: [u64; 7],
    pub denied: [u64; 7],
    pub completed: bool,
    pub conserved: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecursiveCounterSnapshot {
    pub coverage: Coverage,
    /// Parent, TerminalParent, Executable; these are aggregate totals.
    pub stages: [RecursiveStageCounts; 3],
    pub current_program: Option<RecursiveProgramCounts>,
    pub last_program: Option<RecursiveProgramCounts>,
    pub current: Option<RecursiveDecisionCounts>,
    pub heaviest_completed: [Option<RecursiveDecisionCounts>; 2],
    pub overflowed: bool,
    pub inconsistent: bool,
}
impl Default for RecursiveCounterSnapshot {
    fn default() -> Self {
        Self {
            coverage: Coverage::NotReached,
            stages: [RecursiveStageCounts::default(); 3],
            current_program: None,
            last_program: None,
            current: None,
            heaviest_completed: [None; 2],
            overflowed: false,
            inconsistent: false,
        }
    }
}
pub struct RecursiveAnalysisCounters {
    state: Cell<RecursiveCounterSnapshot>,
    selected_static: Cell<Option<selected_static::SelectedStatic>>,
}
impl Default for RecursiveAnalysisCounters {
    fn default() -> Self {
        Self::new()
    }
}
fn stage_index(stage: RunStage) -> usize {
    match stage {
        RunStage::Parent => 0,
        RunStage::TerminalParent => 1,
        RunStage::Executable => 2,
    }
}
fn add(value: &mut u64, amount: u64, overflow: &mut bool) {
    match value.checked_add(amount) {
        Some(next) => *value = next,
        None => {
            *value = u64::MAX;
            *overflow = true;
        }
    }
}
impl RecursiveAnalysisCounters {
    pub fn new() -> Self {
        Self {
            state: Cell::new(RecursiveCounterSnapshot::default()),
            selected_static: Cell::new(None),
        }
    }
    pub fn reset(&self) {
        self.state.set(RecursiveCounterSnapshot::default());
        self.reset_selected_static();
    }
    pub fn snapshot(&self) -> RecursiveCounterSnapshot {
        self.state.get()
    }
    fn update(&self, visit: impl FnOnce(&mut RecursiveCounterSnapshot)) {
        let mut state = self.state.get();
        visit(&mut state);
        self.state.set(state);
    }
    pub(crate) fn coverage(&self, coverage: Coverage) {
        self.update(|s| s.coverage = coverage);
    }
    pub(crate) fn begin_program(&self, stage: RunStage, initial: usize) {
        self.update(|s| {
            if s.current_program.is_some() {
                s.inconsistent = true;
            }
            let ordinal = s
                .stages
                .iter()
                .try_fold(0u64, |sum, row| sum.checked_add(row.program_invocations));
            let ordinal = match ordinal {
                Some(value) => value,
                None => {
                    s.overflowed = true;
                    u64::MAX
                }
            };
            add(
                &mut s.stages[stage_index(stage)].program_invocations,
                1,
                &mut s.overflowed,
            );
            s.current_program = Some(RecursiveProgramCounts {
                ordinal,
                stage,
                initial,
                remaining: initial,
                granted: [0; 7],
                denied: [0; 7],
                completed: false,
                conserved: true,
            });
            s.current = None;
        });
    }
    pub(crate) fn end_program(&self, remaining: usize, completed: bool) {
        self.update(|s| {
            if let Some(mut program) = s.current_program.take() {
                program.remaining = remaining;
                program.completed = completed;
                let sum = program
                    .granted
                    .iter()
                    .try_fold(0u64, |a, b| a.checked_add(*b));
                if sum.is_none() {
                    s.overflowed = true;
                }
                program.conserved = !s.overflowed
                    && match (
                        program
                            .initial
                            .checked_sub(remaining)
                            .and_then(|used| u64::try_from(used).ok()),
                        sum,
                    ) {
                        (Some(used), Some(granted)) => used == granted,
                        _ => false,
                    };
                if completed {
                    add(
                        &mut s.stages[stage_index(program.stage)].programs_completed,
                        1,
                        &mut s.overflowed,
                    );
                }
                s.last_program = Some(program);
            } else {
                s.inconsistent = true;
            }
        });
    }
    pub(crate) fn skip(&self, metadata: bool) {
        self.update(|s| {
            if let Some(program) = s.current_program {
                let stage = &mut s.stages[stage_index(program.stage)];
                add(
                    if metadata {
                        &mut stage.skipped_metadata
                    } else {
                        &mut stage.skipped_without_entry_guard
                    },
                    1,
                    &mut s.overflowed,
                );
            }
        });
    }
    pub(crate) fn begin_decision(&self, grammar: &Grammar, entry: &SourceId, path: &str) {
        self.update(|s| {
            if let Some(program) = s.current_program {
                let stage = &mut s.stages[stage_index(program.stage)];
                let ordinal = stage.decisions_started;
                add(&mut stage.decisions_started, 1, &mut s.overflowed);
                let origin = path.split('/').nth(1).and_then(|name| {
                    grammar
                        .declaration_sources
                        .get(name)
                        .map(|span| (name, span))
                });
                let (source, owner_rule, declaration_span) = match origin {
                    Some((name, span)) => (
                        CapturedSource::copy(&span.source),
                        CapturedText::copy(name),
                        Some((span.start, span.end)),
                    ),
                    None => (CapturedSource::copy(entry), None, None),
                };
                let decision = CapturedText::copy(path);
                let capture_unavailable =
                    source.is_none() || owner_rule.is_none() || decision.is_none();
                s.current = Some(RecursiveDecisionCounts {
                    program_ordinal: program.ordinal,
                    decision_ordinal: ordinal,
                    stage: program.stage,
                    source,
                    owner_rule,
                    declaration_span,
                    decision,
                    capture_unavailable,
                    failed: false,
                    completed: None,
                    work: RecursiveWorkCounts::default(),
                });
            }
        });
    }
    pub(crate) fn end_decision(&self, result: &Result<bool, String>) {
        self.update(|s| {
            if let Some(mut current) = s.current.take() {
                let stage = &mut s.stages[stage_index(current.stage)];
                match result {
                    Ok(value) => {
                        add(
                            if *value {
                                &mut stage.completed_true
                            } else {
                                &mut stage.completed_false
                            },
                            1,
                            &mut s.overflowed,
                        );
                        current.completed = Some(*value);
                        let cost = |row: &RecursiveDecisionCounts| {
                            row.work
                                .granted
                                .iter()
                                .fold(0u64, |a, b| a.saturating_add(*b))
                        };
                        for slot in &mut s.heaviest_completed {
                            if slot.as_ref().is_none_or(|row| cost(&current) > cost(row)) {
                                match slot.replace(current) {
                                    Some(previous) => current = previous,
                                    None => return,
                                }
                            }
                        }
                    }
                    Err(_) => {
                        add(&mut stage.failed, 1, &mut s.overflowed);
                        current.failed = true;
                        s.current = Some(current);
                    }
                }
            }
        });
    }
    fn work(&self, visit: impl Fn(&mut RecursiveWorkCounts, &mut bool)) {
        self.update(|s| {
            let Some(program) = s.current_program else {
                return;
            };
            visit(
                &mut s.stages[stage_index(program.stage)].work,
                &mut s.overflowed,
            );
            if let Some(current) = s.current.as_mut() {
                visit(&mut current.work, &mut s.overflowed);
            }
        });
    }
    pub(crate) fn charge(&self, kind: RecursiveWorkCategory, granted: bool) {
        self.update(|s| {
            let Some(program) = s.current_program.as_mut() else {
                return;
            };
            add(
                if granted {
                    &mut program.granted[kind as usize]
                } else {
                    &mut program.denied[kind as usize]
                },
                1,
                &mut s.overflowed,
            );
        });
        self.work(|row, overflow| {
            add(
                if granted {
                    &mut row.granted[kind as usize]
                } else {
                    &mut row.denied[kind as usize]
                },
                1,
                overflow,
            )
        });
    }
    pub(crate) fn scheduling(&self, site: ScheduleSite, event: ScheduleEvent) {
        self.work(|row, o| add(&mut row.scheduling[site as usize][event as usize], 1, o));
    }
    pub(crate) fn closure_pop(&self) {
        self.work(|row, o| add(&mut row.closure_pops, 1, o));
    }
    pub(crate) fn legacy_key(&self, unique: bool) {
        self.work(|row, o| {
            add(
                if unique {
                    &mut row.legacy_unique_pops
                } else {
                    &mut row.legacy_duplicate_pops
                },
                1,
                o,
            )
        });
    }
    pub(crate) fn frontier_skip(&self, repeated: bool) {
        self.work(|row, o| {
            add(
                if repeated {
                    &mut row.visited_frontiers
                } else {
                    &mut row.single_alternative_frontiers
                },
                1,
                o,
            )
        });
    }
    pub(crate) fn witness_hit(&self) {
        self.work(|row, o| add(&mut row.witness_hits, 1, o));
    }
    pub(crate) fn witness_miss(&self) {
        self.work(|row, o| add(&mut row.witness_misses, 1, o));
    }
    pub(crate) fn witness_result(&self, result: &Result<bool, String>, used: usize) {
        self.work(|row, o| {
            add(
                match result {
                    Ok(true) => &mut row.witness_miss_true,
                    Ok(false) => &mut row.witness_miss_false,
                    Err(_) => &mut row.witness_miss_errors,
                },
                1,
                o,
            );
            match u64::try_from(used) {
                Ok(used) => add(&mut row.witness_miss_granted_work, used, o),
                Err(_) => *o = true,
            }
        });
    }
}

#[cfg(test)]
mod tests;
