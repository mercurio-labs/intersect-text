//! Fixed passive attribution for one caller-selected static decision.
//! No graph/result/cache authority, semantic debit, allocation or retained input.
use super::{CapturedSource, CapturedText, RecursiveAnalysisCounters, RunStage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedStaticSnapshot {
    pub decision: CapturedText<256>,
    pub wrapper_rule: CapturedText<128>,
    pub child_rule: CapturedText<128>,
    /// start, stop, first owned internal state, exclusive last internal state.
    pub wrapper_states: Option<[usize; 4]>,
    pub child_states: Option<[usize; 4]>,
    pub stage: Option<RunStage>,
    pub source: Option<CapturedSource>,
    pub capture_unavailable: bool,
    pub entry_remaining: Option<usize>,
    pub exit_remaining: Option<usize>,
    pub decision_work: Option<usize>,
    /// Includes real queue sharing and partial errors while a wrapper row runs.
    pub wrapper_row_work: usize,
    pub wrapper_rows: usize,
    pub wrapper_call_attempts: usize,
    pub wrapper_uncached_entries: usize,
    /// Exact nested Call window including its already-granted transition unit.
    /// Includes witness/depth work; never call this removable wrapper work.
    pub required_call_work: usize,
    pub required_return_work: usize,
    pub child_template_work: usize,
    pub child_template_lookups: usize,
    pub child_ready_lookups: usize,
    pub child_build_lookups: usize,
    pub child_replays: usize,
    pub child_emitted_rows: usize,
    /// Wrapper row work less SAME required Call/return and child-template work.
    /// An upper bound on this narrow candidate's available saving, NOT net gain.
    pub residual_wrapper_upper_bound: Option<usize>,
    pub finished: bool,
    pub returned_error: bool,
    pub unsupported_route: bool,
    pub overflowed: bool,
    pub inconsistent: bool,
}
#[derive(Clone, Copy)]
pub(super) struct SelectedStatic {
    snapshot: SelectedStaticSnapshot,
    supported: bool,
    active: bool,
    row: Option<(usize, bool)>,
    call: Option<(usize, bool)>,
}
const _: () = assert!(std::mem::size_of::<SelectedStatic>() <= 2048);
fn add(value: &mut usize, amount: usize, overflow: &mut bool) {
    match value.checked_add(amount) {
        Some(next) => *value = next,
        None => *overflow = true,
    }
}
impl SelectedStatic {
    fn new(
        decision: CapturedText<256>,
        wrapper_rule: CapturedText<128>,
        child_rule: CapturedText<128>,
    ) -> Self {
        Self {
            snapshot: SelectedStaticSnapshot {
                decision,
                wrapper_rule,
                child_rule,
                wrapper_states: None,
                child_states: None,
                stage: None,
                source: None,
                capture_unavailable: true,
                entry_remaining: None,
                exit_remaining: None,
                decision_work: None,
                wrapper_row_work: 0,
                wrapper_rows: 0,
                wrapper_call_attempts: 0,
                wrapper_uncached_entries: 0,
                required_call_work: 0,
                required_return_work: 0,
                child_template_work: 0,
                child_template_lookups: 0,
                child_ready_lookups: 0,
                child_build_lookups: 0,
                child_replays: 0,
                child_emitted_rows: 0,
                residual_wrapper_upper_bound: None,
                finished: false,
                returned_error: false,
                unsupported_route: false,
                overflowed: false,
                inconsistent: false,
            },
            supported: false,
            active: false,
            row: None,
            call: None,
        }
    }
    fn wrapper_owns(&self, state: usize) -> bool {
        self.snapshot
            .wrapper_states
            .is_some_and(|[start, stop, first, last]| {
                state == start || state == stop || (first..last).contains(&state)
            })
    }
    fn end_call(&mut self, remaining: usize) {
        if let Some((before, wrapper)) = self.call.take() {
            if let Some(work) = before.checked_sub(remaining) {
                if wrapper {
                    add(
                        &mut self.snapshot.required_call_work,
                        work,
                        &mut self.snapshot.overflowed,
                    );
                }
            } else {
                self.snapshot.inconsistent = true;
            }
        }
    }
    fn end_row(&mut self, remaining: usize) {
        self.end_call(remaining);
        if let Some((before, returned)) = self.row.take() {
            if let Some(work) = before.checked_sub(remaining) {
                add(
                    &mut self.snapshot.wrapper_row_work,
                    work,
                    &mut self.snapshot.overflowed,
                );
                if returned {
                    add(
                        &mut self.snapshot.required_return_work,
                        work,
                        &mut self.snapshot.overflowed,
                    );
                }
            } else {
                self.snapshot.inconsistent = true;
            }
        }
    }
}
impl RecursiveAnalysisCounters {
    /// Test-only accounting keeps the pre-existing counter holder separately bounded.
    #[cfg(test)]
    pub(crate) const SELECTED_STATIC_HOLDER_BYTES: usize =
        std::mem::size_of::<std::cell::Cell<Option<SelectedStatic>>>();

    /// Finite caller-owned observation, not a compiler policy or allowance.
    /// The caller supplies names; the library contains no language-specific IDs.
    pub fn with_selected_static(
        decision: &str,
        wrapper: &str,
        child: &str,
    ) -> Result<Self, String> {
        let result = Self::new();
        result.selected_static.set(Some(SelectedStatic::new(
            CapturedText::copy(decision).ok_or("Selected static decision exceeds256 bytes")?,
            CapturedText::copy(wrapper).ok_or("Selected static wrapper exceeds128 bytes")?,
            CapturedText::copy(child).ok_or("Selected static child exceeds128 bytes")?,
        )));
        Ok(result)
    }
    pub fn selected_static_snapshot(&self) -> Option<SelectedStaticSnapshot> {
        self.selected_static.get().map(|state| state.snapshot)
    }
    pub(super) fn reset_selected_static(&self) {
        if let Some(state) = self.selected_static.get() {
            self.selected_static.set(Some(SelectedStatic::new(
                state.snapshot.decision,
                state.snapshot.wrapper_rule,
                state.snapshot.child_rule,
            )));
        }
    }
    fn selected_update(&self, visit: impl FnOnce(&mut SelectedStatic)) {
        if let Some(mut state) = self.selected_static.get() {
            visit(&mut state);
            self.selected_static.set(Some(state));
        }
    }
    pub(crate) fn selected_static_prepare(&self, supported: bool) {
        self.selected_update(|state| {
            if !state.snapshot.finished {
                state.supported = supported;
                state.snapshot.wrapper_states = None;
                state.snapshot.child_states = None;
            }
        });
    }
    pub(crate) fn selected_static_rule(&self, name: &str, states: [usize; 4]) {
        self.selected_update(|state| {
            if state.snapshot.finished {
                return;
            }
            if state.snapshot.wrapper_rule.as_str() == Ok(name) {
                state.snapshot.wrapper_states = Some(states);
            }
            if state.snapshot.child_rule.as_str() == Ok(name) {
                state.snapshot.child_states = Some(states);
            }
        });
    }
    pub(crate) fn selected_static_begin(&self, path: &str, remaining: usize) {
        self.selected_update(|state| {
            if state.snapshot.decision.as_str() != Ok(path) || state.snapshot.finished {
                return;
            }
            if state.active {
                state.snapshot.inconsistent = true;
                return;
            }
            state.snapshot.unsupported_route = !state.supported;
            if !state.supported {
                return;
            }
            state.active = true;
            state.snapshot.stage = self.state.get().current_program.map(|p| p.stage);
            // Borrow no new input and perform no second source resolution.
            // The existing decision observer already selected this source.
            if let Some(current) = self.state.get().current {
                state.snapshot.source = current.source;
                state.snapshot.capture_unavailable = current.capture_unavailable;
            }
            state.snapshot.entry_remaining = Some(remaining);
        });
    }
    pub(crate) fn selected_static_end(&self, remaining: usize, error: bool) {
        self.selected_update(|state| {
            if !state.active {
                return;
            }
            state.end_row(remaining); // preserves partial failing Call/row work
            state.snapshot.exit_remaining = Some(remaining);
            state.snapshot.decision_work = state
                .snapshot
                .entry_remaining
                .and_then(|n| n.checked_sub(remaining));
            state.snapshot.inconsistent |= state.snapshot.decision_work.is_none();
            state.snapshot.returned_error = error;
            state.snapshot.finished = true;
            state.snapshot.residual_wrapper_upper_bound = state
                .snapshot
                .wrapper_row_work
                .checked_sub(state.snapshot.required_call_work)
                .and_then(|n| n.checked_sub(state.snapshot.required_return_work))
                .and_then(|n| n.checked_sub(state.snapshot.child_template_work));
            if state.snapshot.overflowed
                || state.snapshot.inconsistent
                || state.snapshot.wrapper_states.is_none()
                || state.snapshot.child_states.is_none()
            {
                state.snapshot.residual_wrapper_upper_bound = None;
            }
            state.active = false;
        });
    }
    pub(crate) fn selected_static_row(&self, index: usize, remaining: usize) {
        self.selected_update(|state| {
            if !state.active {
                return;
            }
            state.end_row(remaining);
            if state.wrapper_owns(index) {
                add(
                    &mut state.snapshot.wrapper_rows,
                    1,
                    &mut state.snapshot.overflowed,
                );
                let returned = state
                    .snapshot
                    .wrapper_states
                    .is_some_and(|ids| index == ids[1]);
                state.row = Some((remaining, returned));
            }
        });
    }
    pub(crate) fn selected_static_row_end(&self, remaining: usize) {
        self.selected_update(|state| {
            if state.active {
                state.end_row(remaining);
            }
        });
    }
    pub(crate) fn selected_static_call_begin(&self, target: usize, remaining: usize) {
        self.selected_update(|state| {
            if !state.active {
                return;
            }
            if state.call.is_some() {
                state.snapshot.inconsistent = true;
            }
            if state
                .snapshot
                .wrapper_states
                .is_some_and(|ids| target == ids[0])
            {
                add(
                    &mut state.snapshot.wrapper_call_attempts,
                    1,
                    &mut state.snapshot.overflowed,
                );
            }
            let wrapper = state.row.is_some();
            // The canonical transition unit immediately before this Call arm
            // has already succeeded and is included in its required effects.
            if wrapper {
                add(
                    &mut state.snapshot.required_call_work,
                    1,
                    &mut state.snapshot.overflowed,
                );
            }
            state.call = Some((remaining, wrapper));
        });
    }
    pub(crate) fn selected_static_call_end(&self, remaining: usize) {
        self.selected_update(|state| {
            if state.active {
                state.end_call(remaining);
            }
        });
    }
    pub(crate) fn selected_static_template_lookup(
        &self,
        target: usize,
        ready: bool,
        building: bool,
    ) {
        self.selected_update(|state| {
            if !state.active
                || state
                    .snapshot
                    .child_states
                    .is_none_or(|ids| target != ids[0])
            {
                return;
            }
            add(
                &mut state.snapshot.child_template_lookups,
                1,
                &mut state.snapshot.overflowed,
            );
            if ready {
                add(
                    &mut state.snapshot.child_ready_lookups,
                    1,
                    &mut state.snapshot.overflowed,
                );
            }
            if building {
                add(
                    &mut state.snapshot.child_build_lookups,
                    1,
                    &mut state.snapshot.overflowed,
                );
            }
        });
    }
    pub(crate) fn selected_static_template_row(&self, target: usize) {
        self.selected_update(|state| {
            if state.active
                && state
                    .snapshot
                    .child_states
                    .is_some_and(|ids| target == ids[0])
            {
                add(
                    &mut state.snapshot.child_emitted_rows,
                    1,
                    &mut state.snapshot.overflowed,
                );
            }
        });
    }
    pub(crate) fn selected_static_template_end(
        &self,
        target: usize,
        before: usize,
        after: usize,
        replayed: Option<bool>,
    ) {
        self.selected_update(|state| {
            if !state.active {
                return;
            }
            if state
                .snapshot
                .wrapper_states
                .is_some_and(|ids| target == ids[0])
                && replayed == Some(false)
            {
                add(
                    &mut state.snapshot.wrapper_uncached_entries,
                    1,
                    &mut state.snapshot.overflowed,
                );
            }
            if state
                .snapshot
                .child_states
                .is_some_and(|ids| target == ids[0])
            {
                // Only subtract child costs nested IN an attributed wrapper row.
                if state.row.is_some() {
                    if let Some(work) = before.checked_sub(after) {
                        add(
                            &mut state.snapshot.child_template_work,
                            work,
                            &mut state.snapshot.overflowed,
                        );
                    } else {
                        state.snapshot.inconsistent = true;
                    }
                }
                if replayed == Some(true) {
                    add(
                        &mut state.snapshot.child_replays,
                        1,
                        &mut state.snapshot.overflowed,
                    );
                }
            }
        });
    }
}
