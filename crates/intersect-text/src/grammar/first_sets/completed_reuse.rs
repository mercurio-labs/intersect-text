//! One strict-collection borrow of completed canonical FIRST derivations.
//! No executable/serialized proof, cross-grammar cache or runtime authority.
use super::{Summary, symbols::Symbols};
use crate::grammar::Grammar;
use crate::grammar::Kind;
use crate::grammar::Rule;
use crate::grammar::admission_observation::FirstMapOperation as MapOperation;
use crate::grammar::admission_observation::FirstReuseCharge as Charge;
use crate::grammar::admission_observation::FirstReuseEvent as Event;
use crate::grammar::admission_observation::FirstReuseMeter as Meter;
use crate::grammar::admission_observation::PredictionObservation;
use intersect_model::definitions::bundle::SourceSpan;
use std::collections::{BTreeMap, BTreeSet};

const LIMIT: usize = 1_048_576;
const HEADER: usize = 128;
const ENTRY: usize = 192;
const FRAME: usize = 128;
const SYMBOL: usize = 64;
const MAX_ENTRIES: usize = 4096;

fn pay(work: &mut usize, meter: Meter<'_>) -> Result<(), String> {
    if *work == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *work -= 1;
    meter.paid();
    Ok(())
}
fn bytes(bytes: usize, work: &mut usize, meter: Meter<'_>) -> Result<(), String> {
    for _ in 0..bytes.div_ceil(64) {
        pay(work, meter)?;
    }
    Ok(())
}
struct Entry<'g, S: Symbols> {
    rule: &'g Rule,
    source: &'g SourceSpan,
    summary: Summary<S>,
    calls: usize,
    syntax: usize,
}
struct Active<'g> {
    name: &'g str,
    rule: &'g Rule,
    source: &'g SourceSpan,
    base: usize,
    calls: usize,
    syntax: usize,
}
pub(super) enum Call<S: Symbols> {
    Hit(Summary<S>),
    Entered,
    // Traverse the same body without a cache return, paying its real worker slot.
    Uncached,
    Fallback,
}

pub(in crate::grammar) type Completed<'g> = Storage<'g, BTreeSet<String>>;
pub(in crate::grammar) type NullabilityCompleted<'g> = Storage<'g, ()>;

pub(in crate::grammar) struct Storage<'g, S: Symbols> {
    grammar: &'g Grammar,
    entries: BTreeMap<&'g str, Entry<'g, S>>,
    active: Vec<Active<'g>>,
    // Guaranteed by our own successful reservation, never Vec::capacity().
    active_capacity: usize,
    retained: usize,
    failed: bool,
    #[cfg(test)]
    pub(super) visits: usize,
    #[cfg(test)]
    pub(super) hits: usize,
    #[cfg(test)]
    pub(super) completions: usize,
    #[cfg(test)]
    pub(super) publications: usize,
    #[cfg(test)]
    test_limit: usize,
    #[cfg(test)]
    force_memoize_one_step: bool,
}
impl<'g, S: Symbols> Storage<'g, S> {
    pub(in crate::grammar) fn observe_retained(&self, observation: &PredictionObservation) {
        observation.reuse_retained(self.retained);
    }
    pub(in crate::grammar) fn new(grammar: &'g Grammar, work: &mut usize) -> Result<Self, String> {
        const {
            // The same prepaid HEADER also covers the query-local worker bound.
            assert!(std::mem::size_of::<Storage<'_, S>>() + std::mem::size_of::<usize>() <= HEADER);
            assert!(std::mem::size_of::<Entry<'_, S>>() <= ENTRY);
            assert!(std::mem::size_of::<Active<'_>>() <= FRAME);
            assert!(std::mem::size_of::<super::Frame<'_, S>>() <= S::WORKER_FRAME_BYTES);
        }
        let meter = Meter::new(None, Charge::AffinityAndState);
        pay(work, meter)?;
        bytes(HEADER, work, meter)?;
        Ok(Self {
            grammar,
            entries: BTreeMap::new(),
            active: Vec::new(),
            active_capacity: 0,
            retained: HEADER,
            failed: false,
            #[cfg(test)]
            visits: 0,
            #[cfg(test)]
            hits: 0,
            #[cfg(test)]
            completions: 0,
            #[cfg(test)]
            publications: 0,
            #[cfg(test)]
            test_limit: LIMIT,
            #[cfg(test)]
            force_memoize_one_step: false,
        })
    }
    #[cfg(test)]
    pub(in crate::grammar) fn with_limit(
        grammar: &'g Grammar,
        work: &mut usize,
        limit: usize,
    ) -> Result<Self, String> {
        if !(HEADER..=LIMIT).contains(&limit) {
            return Err("Completed FIRST retention bound exceeded".into());
        }
        let mut value = Self::new(grammar, work)?;
        value.test_limit = limit;
        Ok(value)
    }
    // Test-only same-worker old policy for complete real cost comparison.
    #[cfg(test)]
    pub(in crate::grammar) fn memoize_one_step_for_test(&mut self) {
        self.force_memoize_one_step = true;
    }
    fn memoize_one_step(&self) -> bool {
        #[cfg(test)]
        if self.force_memoize_one_step {
            return true;
        }
        S::MEMOIZE_ONE_STEP
    }
    fn limit(&self) -> usize {
        #[cfg(test)]
        {
            self.test_limit
        }
        #[cfg(not(test))]
        {
            LIMIT
        }
    }
    #[cfg(test)]
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained
    }
    pub(super) fn belongs(&self, grammar: &Grammar, work: &mut usize) -> Result<bool, String> {
        self.belongs_meter(grammar, work, Meter::new(None, Charge::AffinityAndState))
    }
    pub(super) fn belongs_observed(
        &self,
        grammar: &Grammar,
        work: &mut usize,
        observation: &PredictionObservation,
    ) -> Result<bool, String> {
        self.belongs_meter(
            grammar,
            work,
            Meter::new(Some(observation), Charge::AffinityAndState),
        )
    }
    fn belongs_meter(
        &self,
        grammar: &Grammar,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<bool, String> {
        // Fixed entry identity/state and eventual fail-sticky cleanup reservation.
        for _ in 0..3 {
            pay(work, meter)?;
        }
        Ok(std::ptr::eq(self.grammar, grammar))
    }
    pub(super) fn ready(&self) -> Result<(), String> {
        if self.failed || !self.active.is_empty() {
            return Err("Completed FIRST collection is failed or active".into());
        }
        Ok(())
    }
    pub(super) fn fail(&mut self) {
        self.failed = true;
    }
    // Canonical Visit debit/check has ALREADY executed, even on a later hit.
    pub(super) fn visit(
        &mut self,
        syntax: usize,
        calls: usize,
        work: &mut usize,
    ) -> Result<(), String> {
        self.visit_meter(
            syntax,
            calls,
            work,
            Meter::new(None, Charge::VisitPeakUpdates),
        )
    }
    pub(super) fn visit_observed(
        &mut self,
        syntax: usize,
        calls: usize,
        work: &mut usize,
        observation: &PredictionObservation,
    ) -> Result<(), String> {
        self.visit_meter(
            syntax,
            calls,
            work,
            Meter::new(Some(observation), Charge::VisitPeakUpdates),
        )
    }
    fn visit_meter(
        &mut self,
        syntax: usize,
        calls: usize,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<(), String> {
        pay(work, meter)?;
        if let Some(active) = self.active.last_mut() {
            pay(work, meter)?;
            active.calls = active.calls.max(calls);
            active.syntax = active.syntax.max(syntax);
        }
        #[cfg(test)]
        {
            self.visits += 1;
        }
        Ok(())
    }
    fn map_cost(
        entries: usize,
        name: &str,
        work: &mut usize,
        meter: Meter<'_>,
        operation: MapOperation,
    ) -> Result<(), String> {
        let meter = meter.category(Charge::OrderedMap).map(operation);
        meter.map_started();
        // Same chosen logical ordered-map fee as transparent_ports::map_work:
        // 1 + bit_length(entries+1), not a CPU/comparison/RSS bound. Key reads
        // are additionally paid in portable64-byte chunks at each fee unit.
        let mut levels = entries
            .checked_add(1)
            .ok_or("Completed FIRST map size overflow")?;
        pay(work, meter)?;
        bytes(name.len(), work, meter)?;
        while levels != 0 {
            pay(work, meter)?;
            bytes(name.len(), work, meter)?;
            levels /= 2;
        }
        meter.map_finished();
        Ok(())
    }
    fn lookup_cost(
        &self,
        name: &str,
        work: &mut usize,
        meter: Meter<'_>,
        operation: MapOperation,
    ) -> Result<(), String> {
        Self::map_cost(self.entries.len(), name, work, meter, operation)
    }
    #[cfg(test)]
    pub(super) fn entry_count(&self) -> usize {
        self.entries.len()
    }
    fn reserve(&mut self, amount: usize, work: &mut usize, meter: Meter<'_>) -> Result<(), String> {
        let next = self
            .retained
            .checked_add(amount)
            .filter(|n| *n <= self.limit())
            .ok_or("Completed FIRST retention bound exceeded")?;
        pay(work, meter)?;
        bytes(amount, work, meter)?;
        self.retained = next;
        meter.retained(self.retained);
        Ok(())
    }
    // One fixed planning debit even when a previously prepaid bound fits.
    // Every newly guaranteed slot is retained cumulatively IN ADDITION to the
    // unchanged per-call FRAME retention. This conservative double accounting
    // covers spare storage without refunds or relying on allocator capacity.
    // A bound is published only AFTER the actual Vec reservation succeeds.
    fn plan_capacity(
        &mut self,
        length: usize,
        additional: usize,
        guaranteed: usize,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<Option<usize>, String> {
        self.plan_capacity_for(length, additional, guaranteed, FRAME, work, meter)
    }
    fn plan_capacity_for(
        &mut self,
        length: usize,
        additional: usize,
        guaranteed: usize,
        frame_bytes: usize,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<Option<usize>, String> {
        pay(work, meter)?;
        let required = length
            .checked_add(additional)
            .ok_or("Completed FIRST frame capacity overflow")?;
        if required <= guaranteed {
            return Ok(None);
        }
        let target = required
            .checked_next_power_of_two()
            .filter(|n| *n <= LIMIT / FRAME)
            .ok_or("Completed FIRST frame capacity bound exceeded")?;
        let growth = target
            .checked_sub(guaranteed)
            .and_then(|n| n.checked_mul(frame_bytes))
            .ok_or("Completed FIRST frame capacity overflow")?;
        self.reserve(growth, work, meter)?;
        // Conservatively charge transfer of all live frames on actual planned
        // growth, even if the allocator could keep its existing allocation.
        bytes(
            length
                .checked_mul(frame_bytes)
                .ok_or("Completed FIRST frame size overflow")?,
            work,
            meter,
        )?;
        Ok(Some(target))
    }
    fn summary_bytes(
        summary: &Summary<S>,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<usize, String> {
        let meter = meter.category(Charge::SummaryInspectionAndClone);
        let mut total = 0usize;
        summary.0.each(|value| {
            pay(work, meter)?;
            total = total
                .checked_add(SYMBOL)
                .and_then(|n| n.checked_add(value.len()))
                .ok_or_else(|| String::from("Completed FIRST symbol size overflow"))?;
            Ok(())
        })?;
        Ok(total)
    }
    pub(super) fn enter(
        &mut self,
        resolved: super::resolved_call::Resolved<'g>,
        calls: usize,
        work: &mut usize,
    ) -> Result<Call<S>, String> {
        self.enter_meter(
            resolved,
            calls,
            work,
            Meter::new(None, Charge::SourceKeyAndActiveChecks),
        )
    }
    pub(super) fn enter_observed(
        &mut self,
        resolved: super::resolved_call::Resolved<'g>,
        calls: usize,
        work: &mut usize,
        observation: &PredictionObservation,
    ) -> Result<Call<S>, String> {
        self.enter_meter(
            resolved,
            calls,
            work,
            Meter::new(Some(observation), Charge::SourceKeyAndActiveChecks),
        )
    }
    fn enter_meter(
        &mut self,
        resolved: super::resolved_call::Resolved<'g>,
        calls: usize,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<Call<S>, String> {
        // The private token comes from the real canonical worker lookup. Its
        // fields cannot be constructed by a caller; validate exact graph owner.
        pay(work, meter)?;
        let Some((name, rule)) = resolved.for_grammar(self.grammar) else {
            meter.event(Event::ForeignGraph, 1);
            return Ok(Call::Fallback);
        };
        let meter = meter.for_rule(rule);
        // Every actual completed-map/declaration lookup retains its old fee.
        // A completed entry's source was borrowed from THIS immutable Grammar
        // at the first miss and published only after the canonical body. This
        // borrow cannot change; use it without repeating the declaration read.
        // Full/default keeps its literal map/body route. Unit pays the new
        // single-tag eligibility read; no traversal or nullable answer occurs.
        let uncached = if self.memoize_one_step() {
            false
        } else {
            pay(work, meter)?;
            matches!(
                &rule.body.kind,
                Kind::Literal(_) | Kind::Action(_) | Kind::Capture { .. } | Kind::Reference(_)
            )
        };
        let completed = if uncached {
            None
        } else {
            self.lookup_cost(name, work, meter, MapOperation::CompletedProbe)?;
            self.entries.get(name)
        };
        let source = match completed {
            Some(entry) => entry.source,
            None => {
                Self::map_cost(
                    self.grammar.declaration_sources.len(),
                    name,
                    work,
                    meter,
                    MapOperation::DeclarationSource,
                )?;
                let Some(source) = self.grammar.declaration_sources.get(name) else {
                    meter.event(Event::MissingSourceOrRange, 1);
                    return Ok(Call::Fallback);
                };
                source
            }
        };
        // Full FIRST keeps the original validation route and debits. Unit
        // may reuse ONLY a published entry's immutable source/body validity.
        // Entry is created from a validated Active after complete pops it;
        // enter pushes Active only on a missing entry. Thus a published rule
        // cannot also be Active, including when a depth hit takes fallback.
        // The same-grammar canonical token and paid map probe above still run;
        // the common depth/peak/clone/quota checks below also remain unchanged.
        let published_unit = completed.is_some_and(|entry| S::full(&entry.summary).is_none());
        if !published_unit {
            pay(work, meter)?;
            let Some(range) = rule.body.original_range.as_ref() else {
                meter.event(Event::MissingSourceOrRange, 1);
                return Ok(Call::Fallback);
            };
            if range.start >= range.end || range.start < source.start || range.end > source.end {
                meter.event(Event::MissingSourceOrRange, 1);
                return Ok(Call::Fallback);
            }
            for active in &self.active {
                pay(work, meter)?;
                if std::ptr::eq(active.rule, rule) {
                    meter.event(Event::ActiveCall, 1);
                    return Ok(Call::Fallback);
                }
            }
        }
        if uncached {
            // Source/range/active checks above remain real and paid. No cache
            // entry, Active or CompletedCallReturn is retained for this body.
            // Canonical Visit/cardinality still execute at calls+1; parent
            // peaks are updated by those visits, not a fabricated summary.
            if calls.checked_add(1).is_none_or(|n| n > 64) {
                meter.event(Event::DepthOrCapacity, 1);
            }
            return Ok(Call::Uncached);
        }
        if let Some(entry) = completed {
            pay(work, meter)?;
            let peak = calls
                .checked_add(1)
                .and_then(|base| base.checked_add(entry.calls));
            if std::ptr::eq(entry.rule, rule)
                && std::ptr::eq(entry.source, source)
                && peak.is_some_and(|peak| peak <= 64)
                && entry.syntax <= 64
            {
                let peak = peak.ok_or("Completed FIRST call depth overflow")?;
                let copy = Self::summary_bytes(&entry.summary, work, meter)?;
                // Account output clones cumulatively too: transferring a clone
                // into the canonical working Summary does not refund storage.
                // The total cache/frame/entry/output allowance is ONE LIMIT.
                let next = self
                    .retained
                    .checked_add(copy)
                    .filter(|n| *n <= self.limit())
                    .ok_or("Completed FIRST output retention bound exceeded")?;
                pay(work, meter.category(Charge::SummaryInspectionAndClone))?;
                bytes(
                    copy,
                    work,
                    meter.category(Charge::SummaryInspectionAndClone),
                )?;
                self.retained = next;
                meter.retained(self.retained);
                let output = entry.summary.clone();
                meter.cloned(entry.summary.0.symbol_count(), copy);
                if let Some(active) = self.active.last_mut() {
                    pay(work, meter.category(Charge::PublicationAndPeakTransfer))?;
                    active.calls = active.calls.max(peak);
                    active.syntax = active.syntax.max(entry.syntax);
                }
                #[cfg(test)]
                {
                    self.hits += 1;
                }
                meter.event(Event::Hit, 1);
                meter.retained(self.retained);
                return Ok(Call::Hit(output));
            }
            meter.event(Event::DepthOrCapacity, 1);
            // Insufficient depth evidence traverses the original body instead;
            // no invented early depth error or replacement recursive allowance.
            return Ok(Call::Fallback);
        }
        let Some(base) = calls.checked_add(1).filter(|n| *n <= 64) else {
            meter.event(Event::DepthOrCapacity, 1);
            return Ok(Call::Fallback);
        };
        if self.active.len() >= 64 || self.entries.len() >= MAX_ENTRIES {
            meter.event(Event::DepthOrCapacity, 1);
            return Ok(Call::Fallback);
        }
        self.reserve(
            FRAME,
            work,
            meter.category(Charge::FrameRetentionAndCapacity),
        )?;
        if let Some(target) = self.plan_capacity(
            self.active.len(),
            1,
            self.active_capacity,
            work,
            meter.category(Charge::FrameRetentionAndCapacity),
        )? {
            self.active
                .try_reserve_exact(target - self.active.len())
                .map_err(|_| "Completed FIRST frame retention failed")?;
            self.active_capacity = target;
        }
        self.active.push(Active {
            name,
            rule,
            source,
            base,
            calls: base,
            syntax: 0,
        });
        meter.event(Event::CalleeEntered, 1);
        meter.retained(self.retained);
        Ok(Call::Entered)
    }
    pub(super) fn return_frame(
        &mut self,
        frames: &mut Vec<super::Frame<'_, S>>,
        guaranteed: &mut usize,
        work: &mut usize,
    ) -> Result<(), String> {
        self.return_frame_meter(
            frames,
            guaranteed,
            work,
            Meter::new(None, Charge::FrameRetentionAndCapacity),
        )
    }
    pub(super) fn return_frame_observed(
        &mut self,
        frames: &mut Vec<super::Frame<'_, S>>,
        guaranteed: &mut usize,
        work: &mut usize,
        observation: &PredictionObservation,
    ) -> Result<(), String> {
        self.return_frame_meter(
            frames,
            guaranteed,
            work,
            Meter::new(Some(observation), Charge::FrameRetentionAndCapacity),
        )
    }
    fn return_frame_meter(
        &mut self,
        frames: &mut Vec<super::Frame<'_, S>>,
        guaranteed: &mut usize,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<(), String> {
        self.worker_frames_meter(frames, guaranteed, 2, work, meter)
    }
    pub(super) fn uncached_frame(
        &mut self,
        frames: &mut Vec<super::Frame<'_, S>>,
        guaranteed: &mut usize,
        work: &mut usize,
        observation: Option<&PredictionObservation>,
    ) -> Result<(), String> {
        self.worker_frames_meter(
            frames,
            guaranteed,
            1,
            work,
            Meter::new(observation, Charge::FrameRetentionAndCapacity),
        )
    }
    fn worker_frames_meter(
        &mut self,
        frames: &mut Vec<super::Frame<'_, S>>,
        guaranteed: &mut usize,
        additional: usize,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<(), String> {
        // The real body Visit still reserves/retains/moves worker storage from
        // the same quota and balance, even though no cache return is pushed.
        self.reserve(S::WORKER_FRAME_BYTES, work, meter)?;
        if let Some(target) = self.plan_capacity_for(
            frames.len(),
            additional,
            *guaranteed,
            S::WORKER_FRAME_BYTES,
            work,
            meter,
        )? {
            frames
                .try_reserve_exact(target - frames.len())
                .map_err(|_| "Completed FIRST worker frame retention failed")?;
            *guaranteed = target;
        }
        Ok(())
    }
    // Invoked after the entire callee body (including its cardinality) and
    // before caller Call/ScalarReference cardinality is applied by the worker.
    pub(super) fn complete(
        &mut self,
        rule: &Rule,
        summary: &Summary<S>,
        work: &mut usize,
    ) -> Result<(), String> {
        self.complete_meter(
            rule,
            summary,
            work,
            Meter::new(None, Charge::PublicationAndPeakTransfer),
        )
    }
    pub(super) fn complete_observed(
        &mut self,
        rule: &Rule,
        summary: &Summary<S>,
        work: &mut usize,
        observation: &PredictionObservation,
    ) -> Result<(), String> {
        self.complete_meter(
            rule,
            summary,
            work,
            Meter::new(Some(observation), Charge::PublicationAndPeakTransfer),
        )
    }
    fn complete_meter(
        &mut self,
        rule: &Rule,
        summary: &Summary<S>,
        work: &mut usize,
        meter: Meter<'_>,
    ) -> Result<(), String> {
        #[cfg(test)]
        {
            self.completions += 1;
        }
        let meter = meter.for_rule(rule);
        pay(work, meter)?;
        let active = self
            .active
            .last()
            .ok_or("Missing completed FIRST source frame")?;
        if !std::ptr::eq(active.rule, rule) {
            return Err("Completed FIRST return differs from active body".into());
        }
        let relative = active
            .calls
            .checked_sub(active.base)
            .ok_or("Completed FIRST depth underflow")?;
        if self.entries.len() >= MAX_ENTRIES {
            return Err("Completed FIRST entry bound exceeded".into());
        }
        let amount = ENTRY
            .checked_add(Self::summary_bytes(summary, work, meter)?)
            .ok_or("Completed FIRST entry size overflow")?;
        self.reserve(
            amount,
            work,
            meter.category(Charge::SummaryInspectionAndClone),
        )?;
        self.lookup_cost(
            self.active
                .last()
                .ok_or("Missing FIRST publication frame")?
                .name,
            work,
            meter,
            MapOperation::PublicationInsert,
        )?;
        pay(work, meter)?;
        let active = self.active.pop().ok_or("Missing completed FIRST frame")?;
        if let Some(parent) = self.active.last_mut() {
            pay(work, meter)?;
            parent.calls = parent.calls.max(active.calls);
            parent.syntax = parent.syntax.max(active.syntax);
        }
        self.entries.insert(
            active.name,
            Entry {
                rule: active.rule,
                source: active.source,
                summary: summary.clone(),
                calls: relative,
                syntax: active.syntax,
            },
        );
        meter.event(Event::Publication, 1);
        if let Some(copy) = amount.checked_sub(ENTRY) {
            meter.cloned(summary.0.symbol_count(), copy);
        }
        meter.retained(self.retained);
        #[cfg(test)]
        {
            self.publications += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "completed_capacity_tests.rs"]
mod capacity_tests;
