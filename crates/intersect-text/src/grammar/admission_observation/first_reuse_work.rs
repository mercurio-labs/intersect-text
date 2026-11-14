//! Fixed caller-owned measurements; never semantic work or admission authority.
use super::{CapturedSource, PredictionObservation, RunStage};
use crate::grammar::Kind;
use crate::grammar::Rule;
use intersect_model::definitions::bundle::SourceId;

pub const FIRST_REUSE_CHARGES: usize = 8;
pub const FIRST_REUSE_EVENTS: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum FirstReuseCharge {
    CanonicalVisit,
    AffinityAndState,
    VisitPeakUpdates,
    SourceKeyAndActiveChecks,
    OrderedMap,
    FrameRetentionAndCapacity,
    SummaryInspectionAndClone,
    PublicationAndPeakTransfer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum FirstReuseEvent {
    InventoryKey,
    InventoryRepeat,
    Binding,
    EligibleRecord,
    FirstQuery,
    VisitAttempt,
    VisitSucceeded,
    CalleeEntered,
    Hit,
    Publication,
    MissingSourceOrRange,
    ForeignGraph,
    ActiveCall,
    DepthOrCapacity,
    ClonedSymbols,
    ClonedStringBytes,
}

/// Opt-in, fixed 2x3 diagnostic: body shape then actual map-fee purpose.
/// Shape is not source validity, cache eligibility or semantic authority.
/// Completed fee windows are not claimed successful publications. A partial
/// failure keeps every already-paid unit, without inventing a denied charge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirstMapCostSnapshot {
    pub attempted_operations: [Option<usize>; 6],
    pub completed_fee_windows: [Option<usize>; 6],
    pub paid_units: [Option<usize>; 6],
    pub hits_by_body: [Option<usize>; 2],
    pub finished: bool,
    pub multiple_programs: bool,
    pub overflow: bool,
}
impl FirstMapCostSnapshot {
    const fn new() -> Self {
        Self {
            attempted_operations: [Some(0); 6],
            completed_fee_windows: [Some(0); 6],
            paid_units: [Some(0); 6],
            hits_by_body: [Some(0); 2],
            finished: false,
            multiple_programs: false,
            overflow: false,
        }
    }
    pub fn total_paid_units(&self) -> Option<usize> {
        if self.multiple_programs || self.overflow {
            return None;
        }
        self.paid_units
            .iter()
            .try_fold(0usize, |sum, value| sum.checked_add((*value)?))
    }
}
#[derive(Clone, Copy)]
#[repr(usize)]
enum MapBody {
    OneStepCallFree,
    Other,
}
#[derive(Clone, Copy)]
#[repr(usize)]
pub(in crate::grammar) enum MapOperation {
    CompletedProbe,
    DeclarationSource,
    PublicationInsert,
}

#[derive(Clone, Copy)]
pub(in crate::grammar) enum Window {
    Inventory,
    CompletedHeader,
    CompanionHeader,
    Binding,
    First,
}

/// A single actual collection window. None means unmeasured or overflowed.
/// Units are logical successful debits, not CPU, allocations or visited states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirstReuseWorkSnapshot {
    pub stage: RunStage,
    pub source: Option<CapturedSource>,
    pub capture_unavailable: bool,
    pub entry_remaining: usize,
    pub exit_remaining: Option<usize>,
    pub precollect_work: Option<usize>,
    pub collect_work: Option<usize>,
    pub inventory_work: Option<usize>,
    /// Completed::new is outside every FIRST query, counted here only.
    pub completed_header_work: Option<usize>,
    pub companion_header_work: Option<usize>,
    pub binding_inclusive_work: Option<usize>,
    pub first_inclusive_work: Option<usize>,
    pub first_units: [Option<usize>; FIRST_REUSE_CHARGES],
    pub events: [Option<usize>; FIRST_REUSE_EVENTS],
    /// Existing cumulative logical cache quota, not heap size or RSS.
    pub retained_cache_bytes: Option<usize>,
    pub last_syntax_depth: Option<usize>,
    pub last_call_depth: Option<usize>,
    pub finished: bool,
    pub returned_error: bool,
    pub overflow: bool,
    pub multiple_programs: bool,
}
impl FirstReuseWorkSnapshot {
    pub fn binding_exclusive_first(&self) -> Option<usize> {
        if self.multiple_programs {
            return None;
        }
        self.binding_inclusive_work?
            .checked_sub(self.first_inclusive_work?)
    }
    pub fn collector_outer_work(&self) -> Option<usize> {
        if self.multiple_programs {
            return None;
        }
        self.collect_work?
            .checked_sub(self.inventory_work?)?
            .checked_sub(self.completed_header_work?)?
            .checked_sub(self.companion_header_work?)?
            .checked_sub(self.binding_inclusive_work?)
    }
    pub fn first_units_total(&self) -> Option<usize> {
        if self.multiple_programs {
            return None;
        }
        self.first_units
            .iter()
            .try_fold(0usize, |n, v| n.checked_add((*v)?))
    }
}
fn add(slot: &mut Option<usize>, value: usize, overflow: &mut bool) {
    let Some(old) = *slot else {
        return;
    };
    *slot = old.checked_add(value);
    if slot.is_none() {
        *overflow = true;
    }
}
impl PredictionObservation {
    /// Configure before the observed compile. Its reset preserves this opt-in
    /// while clearing all prior measurements. Existing new/default stays OFF.
    pub fn enable_first_map_cost_diagnostic(&self) -> bool {
        if self.first_reuse.get().is_some() {
            return false;
        }
        self.first_map_cost_enabled.set(true);
        true
    }
    pub fn first_map_cost(&self) -> Option<FirstMapCostSnapshot> {
        self.first_map_cost.get()
    }
    fn map_cost_update(&self, change: impl FnOnce(&mut FirstMapCostSnapshot)) {
        if let Some(mut value) = self.first_map_cost.get() {
            if !value.finished && !value.multiple_programs {
                change(&mut value);
            }
            self.first_map_cost.set(Some(value));
        }
    }
    pub fn first_reuse_work(&self) -> Option<FirstReuseWorkSnapshot> {
        self.first_reuse.get()
    }
    #[cfg(test)]
    pub(in crate::grammar) fn reuse_begin(
        &self,
        stage: RunStage,
        source: &SourceId,
        remaining: usize,
    ) {
        self.reuse_begin_with_initial(stage, source, 10_000, remaining);
    }
    pub(in crate::grammar) fn reuse_begin_with_initial(
        &self,
        stage: RunStage,
        source: &SourceId,
        initial_work: usize,
        remaining: usize,
    ) {
        if let Some(mut old) = self.first_reuse.get() {
            old.multiple_programs = true;
            self.first_reuse.set(Some(old));
            if let Some(mut map) = self.first_map_cost.get() {
                map.multiple_programs = true;
                self.first_map_cost.set(Some(map));
            }
            return;
        }
        if self.first_map_cost_enabled.get() {
            self.first_map_cost.set(Some(FirstMapCostSnapshot::new()));
        }
        let captured = CapturedSource::copy(source);
        self.first_reuse.set(Some(FirstReuseWorkSnapshot {
            stage,
            source: captured,
            capture_unavailable: captured.is_none(),
            entry_remaining: remaining,
            exit_remaining: None,
            precollect_work: initial_work.checked_sub(remaining),
            collect_work: None,
            inventory_work: Some(0),
            completed_header_work: Some(0),
            companion_header_work: Some(0),
            binding_inclusive_work: Some(0),
            first_inclusive_work: Some(0),
            first_units: [Some(0); FIRST_REUSE_CHARGES],
            events: [Some(0); FIRST_REUSE_EVENTS],
            retained_cache_bytes: None,
            last_syntax_depth: None,
            last_call_depth: None,
            finished: false,
            returned_error: false,
            overflow: remaining > initial_work,
            multiple_programs: false,
        }));
    }
    fn reuse_update(&self, change: impl FnOnce(&mut FirstReuseWorkSnapshot)) {
        if let Some(mut value) = self.first_reuse.get() {
            if !value.multiple_programs && !value.finished {
                change(&mut value);
            }
            self.first_reuse.set(Some(value));
        }
    }
    pub(in crate::grammar) fn reuse_event(&self, event: FirstReuseEvent, amount: usize) {
        self.reuse_update(|s| add(&mut s.events[event as usize], amount, &mut s.overflow));
    }
    pub(in crate::grammar) fn reuse_charge(&self, charge: FirstReuseCharge) {
        self.reuse_update(|s| add(&mut s.first_units[charge as usize], 1, &mut s.overflow));
    }
    pub(in crate::grammar) fn reuse_visit(&self, syntax: usize, calls: usize) {
        self.reuse_update(|s| {
            s.last_syntax_depth = Some(syntax);
            s.last_call_depth = Some(calls);
        });
        self.reuse_event(FirstReuseEvent::VisitAttempt, 1);
    }
    pub(in crate::grammar) fn reuse_retained(&self, bytes: usize) {
        self.reuse_update(|s| s.retained_cache_bytes = Some(bytes));
    }
    pub(in crate::grammar) fn reuse_window(&self, window: Window, before: usize, after: usize) {
        self.reuse_update(|s| {
            let slot = match window {
                Window::Inventory => &mut s.inventory_work,
                Window::CompletedHeader => &mut s.completed_header_work,
                Window::CompanionHeader => &mut s.companion_header_work,
                Window::Binding => &mut s.binding_inclusive_work,
                Window::First => &mut s.first_inclusive_work,
            };
            match before.checked_sub(after) {
                Some(delta) => add(slot, delta, &mut s.overflow),
                None => {
                    *slot = None;
                    s.overflow = true;
                }
            }
        });
    }
    pub(in crate::grammar) fn reuse_end(&self, remaining: usize, error: bool) {
        self.reuse_update(|s| {
            s.exit_remaining = Some(remaining);
            s.collect_work = s.entry_remaining.checked_sub(remaining);
            s.overflow |= s.collect_work.is_none();
            s.returned_error = error;
            s.finished = true;
        });
        self.map_cost_update(|s| s.finished = true);
    }
}

/// A borrowed passive label passed on the stack, never retained by a cache.
#[derive(Clone, Copy)]
pub(in crate::grammar) struct Meter<'a> {
    observer: Option<&'a PredictionObservation>,
    charge: FirstReuseCharge,
    body: Option<MapBody>,
    map_operation: Option<MapOperation>,
}
impl<'a> Meter<'a> {
    pub(in crate::grammar) const fn new(
        observer: Option<&'a PredictionObservation>,
        charge: FirstReuseCharge,
    ) -> Self {
        Self {
            observer,
            charge,
            body: None,
            map_operation: None,
        }
    }
    pub(in crate::grammar) const fn category(self, charge: FirstReuseCharge) -> Self {
        Self {
            observer: self.observer,
            charge,
            body: self.body,
            map_operation: self.map_operation,
        }
    }
    pub(in crate::grammar) fn for_rule(mut self, rule: &Rule) -> Self {
        // Do not inspect Rule.kind at all on the ordinary or diagnostic-OFF path.
        // The canonical caller already owns this Rule; inspect one tag only.
        // No lookup, body traversal, source validation or eligibility proof.
        if self.observer.is_some_and(|o| {
            o.first_map_cost_enabled.get()
                && o.first_map_cost
                    .get()
                    .is_some_and(|s| !s.finished && !s.multiple_programs)
        }) {
            self.body = Some(match &rule.body.kind {
                Kind::Literal(_) | Kind::Action(_) | Kind::Capture { .. } | Kind::Reference(_) => {
                    MapBody::OneStepCallFree
                }
                _ => MapBody::Other,
            });
        }
        self
    }
    pub(in crate::grammar) const fn map(mut self, operation: MapOperation) -> Self {
        self.map_operation = Some(operation);
        self
    }
    fn map_cell(self) -> Option<usize> {
        Some((self.body? as usize) * 3 + self.map_operation? as usize)
    }
    pub(in crate::grammar) fn map_started(self) {
        if let (Some(observer), Some(cell)) = (self.observer, self.map_cell()) {
            observer
                .map_cost_update(|s| add(&mut s.attempted_operations[cell], 1, &mut s.overflow));
        }
    }
    pub(in crate::grammar) fn map_finished(self) {
        if let (Some(observer), Some(cell)) = (self.observer, self.map_cell()) {
            observer
                .map_cost_update(|s| add(&mut s.completed_fee_windows[cell], 1, &mut s.overflow));
        }
    }
    pub(in crate::grammar) fn paid(self) {
        if let Some(observer) = self.observer {
            observer.reuse_charge(self.charge);
            if self.charge == FirstReuseCharge::OrderedMap
                && let Some(cell) = self.map_cell()
            {
                observer.map_cost_update(|s| add(&mut s.paid_units[cell], 1, &mut s.overflow));
            }
        }
    }
    pub(in crate::grammar) fn event(self, event: FirstReuseEvent, n: usize) {
        if let Some(observer) = self.observer {
            observer.reuse_event(event, n);
            if event == FirstReuseEvent::Hit
                && let Some(body) = self.body
            {
                observer.map_cost_update(|s| {
                    add(&mut s.hits_by_body[body as usize], n, &mut s.overflow)
                });
            }
        }
    }
    pub(in crate::grammar) fn cloned(self, symbols: usize, logical_bytes: usize) {
        if self.observer.is_none() {
            return;
        }
        self.event(FirstReuseEvent::ClonedSymbols, symbols);
        match symbols
            .checked_mul(64)
            .and_then(|nodes| logical_bytes.checked_sub(nodes))
        {
            Some(bytes) => self.event(FirstReuseEvent::ClonedStringBytes, bytes),
            None => {
                if let Some(observer) = self.observer {
                    observer.reuse_update(|s| {
                        s.events[FirstReuseEvent::ClonedStringBytes as usize] = None;
                        s.overflow = true;
                    });
                }
            }
        }
    }
    pub(in crate::grammar) fn retained(self, bytes: usize) {
        if let Some(observer) = self.observer {
            observer.reuse_retained(bytes);
        }
    }
}

// Bounded diagnostic stack storage; not a semantic/cache memory allowance.
const _: () = assert!(std::mem::size_of::<FirstReuseWorkSnapshot>() <= 2048);
// Additional fixed caller-owned diagnostics only; no semantic/cache allowance.
const _: () = assert!(std::mem::size_of::<FirstMapCostSnapshot>() <= 512);
const _: () = assert!(
    std::mem::size_of::<Option<FirstMapCostSnapshot>>()
        + std::mem::size_of::<bool>()
        + std::mem::align_of::<Option<FirstMapCostSnapshot>>()
        <= 512
);

#[cfg(test)]
mod map_cost_diagnostic_tests {
    use super::*;

    #[test]
    fn opt_in_reset_multiple_programs_and_overflow_never_change_old_counters() -> Result<(), String>
    {
        let source = SourceId::for_text("memory:map-cost-lifecycle", "x");
        let off = PredictionObservation::new();
        assert!(off.first_map_cost().is_none());
        off.reuse_begin(RunStage::Executable, &source, 10_000);
        assert!(off.first_map_cost().is_none());
        assert!(!off.enable_first_map_cost_diagnostic());
        let on = PredictionObservation::new();
        assert!(on.enable_first_map_cost_diagnostic());
        on.reset();
        assert!(on.first_map_cost().is_none());
        on.reuse_begin(RunStage::Executable, &source, 10_000);
        let rule: Rule = serde_json::from_value(serde_json::json!({
            "class":"Leaf","body":{"kind":{"Literal":"x"},"cardinality":"1"}
        }))
        .map_err(|e| e.to_string())?;
        let mut seeded = on.first_map_cost().ok_or("Missing enabled scope")?;
        seeded.paid_units[0] = Some(usize::MAX);
        on.first_map_cost.set(Some(seeded));
        let meter = Meter::new(Some(&on), FirstReuseCharge::OrderedMap)
            .for_rule(&rule)
            .map(MapOperation::CompletedProbe);
        meter.map_started();
        meter.paid();
        meter.map_finished();
        let map = on.first_map_cost().ok_or("Missing overflow capture")?;
        println!("MAP_COST_COUNTER_OVERFLOW_ONLY {:?}", map);
        assert!(map.overflow);
        assert_eq!(map.paid_units[0], None);
        assert_eq!(map.total_paid_units(), None);
        let old = on.first_reuse_work().ok_or("Missing original counters")?;
        assert!(!old.overflow);
        assert_eq!(
            old.first_units[FirstReuseCharge::OrderedMap as usize],
            Some(1)
        );
        // Only the bounded diagnostic counter was seeded; this is not a
        // grammar run or a synthetic claim about an actual successful debit.
        on.reset();
        assert!(on.first_reuse_work().is_none() && on.first_map_cost().is_none());
        on.reuse_begin(RunStage::Executable, &source, 10_000);
        assert_eq!(
            on.first_map_cost().and_then(|s| s.total_paid_units()),
            Some(0)
        );
        on.reuse_end(10_000, false);
        on.reuse_begin(RunStage::Executable, &source, 10_000);
        let map = on.first_map_cost().ok_or("Missing multi-program flag")?;
        assert!(map.multiple_programs);
        assert_eq!(map.total_paid_units(), None);
        assert!(on.first_reuse_work().is_some_and(|s| s.multiple_programs));
        Ok(())
    }
}
