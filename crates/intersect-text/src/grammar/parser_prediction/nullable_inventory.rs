//! Compiler-local MAY shortlist; the canonical qualifier still supplies every answer.
//! No serialized graph field, public factory, new ledger or nullable capability.
use super::{
    AdjacentWorkObservation, GraphBinding, ParserPrediction, RepeatCertificates,
    RuntimeGuardedAdmission, analysis_charge, repeat_certificates::GraphRepeatContext,
};
use crate::grammar::AnalysisObservation;
use crate::grammar::Expr;
use crate::grammar::Grammar;
use crate::grammar::Rule;
use std::cmp::Ordering;

const HEADER: usize = 192;
const MAX_PATHS: usize = 4096;
const SLOT: usize = 32;

/// Keep disabled/linear ownership without constructing or dropping a Builder.
pub(super) enum Collection<'g, 'a> {
    Existing(&'a mut RuntimeGuardedAdmission),
    Indexed(Builder<'g, 'a>),
}
impl<'g, 'a> Collection<'g, 'a> {
    pub(super) fn new(
        grammar: &'g Grammar,
        admission: &'a mut RuntimeGuardedAdmission,
        work: &mut usize,
    ) -> Result<Self, String> {
        if admission.inventory_enabled() {
            Ok(Self::Indexed(Builder::new(grammar, admission, work)?))
        } else {
            Ok(Self::Existing(admission))
        }
    }
    pub(super) fn builder(&mut self) -> Option<&mut Builder<'g, 'a>> {
        match self {
            Self::Indexed(builder) => Some(builder),
            Self::Existing(_) => None,
        }
    }
    #[cfg(test)]
    pub(super) fn collect(
        self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
        observation: Option<AnalysisObservation<'_>>,
    ) -> Result<(), String> {
        self.collect_with_certificates(grammar, program, work, observation, None)
    }
    pub(super) fn collect_with_certificates(
        self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
        observation: Option<AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
    ) -> Result<(), String> {
        match self {
            Self::Existing(admission) => admission.collect_with_inventory_and_certificates(
                grammar,
                program,
                work,
                observation,
                None,
                certificates,
            ),
            Self::Indexed(builder) => builder.finish(program, work)?.collect_with_certificates(
                grammar,
                program,
                work,
                observation,
                certificates,
            ),
        }
    }
    pub(super) fn collect_with_guard_facts(
        self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
        observation: Option<AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
        prepared: Option<&super::guard_facts::GuardFacts<'_>>,
    ) -> Result<(), String> {
        if prepared.is_none() {
            return self.collect_with_certificates(
                grammar,
                program,
                work,
                observation,
                certificates,
            );
        }

        match self {
            Self::Existing(admission) => admission.collect_with_inventory_certificates_and_guards(
                grammar,
                program,
                work,
                observation,
                None,
                certificates,
                prepared,
            ),
            Self::Indexed(builder) => builder.finish(program, work)?.collect_with_guard_facts(
                grammar,
                program,
                work,
                observation,
                certificates,
                prepared,
            ),
        }
    }
}

/// One original admission borrow remains live through build, finish and collect.
pub(super) struct Builder<'g, 'a> {
    grammar: &'g Grammar,
    admission: Option<&'a mut RuntimeGuardedAdmission>,
    paths: Vec<String>,
    guaranteed: usize,
    fallback: bool,
    failed: bool,
}
pub(super) struct Finished<'g, 'p, 'a> {
    grammar: &'g Grammar,
    program: &'p ParserPrediction,
    admission: &'a mut RuntimeGuardedAdmission,
    paths: Option<Vec<String>>,
    consumed: bool,
}
const _: () = {
    assert!(
        std::mem::size_of::<Builder<'_, '_>>() + std::mem::size_of::<Finished<'_, '_, '_>>()
            <= HEADER
    );
    // Phase-specific overlap, not a claim about whole compiler stack/RSS.
    assert!(
        std::mem::size_of::<Collection<'_, '_>>()
            + std::mem::size_of::<Finished<'_, '_, '_>>()
            + SLOT
            <= HEADER
    );
    assert!(
        std::mem::size_of::<Collection<'_, '_>>()
            + std::mem::size_of::<GraphBuildContext<'_, '_, '_>>()
            + std::mem::size_of::<GraphRepeatContext<'_, '_>>()
            <= HEADER
    );
    assert!(
        std::mem::size_of::<Builder<'_, '_>>()
            + std::mem::size_of::<Finished<'_, '_, '_>>()
            + std::mem::size_of::<String>()
            <= HEADER
    );
    assert!(
        std::mem::size_of::<Builder<'_, '_>>()
            + std::mem::size_of::<GraphBuildContext<'_, '_, '_>>()
            + std::mem::size_of::<GraphRepeatContext<'_, '_>>()
            <= HEADER
    );
    assert!(std::mem::size_of::<String>() <= SLOT);
};

fn bytes(work: &mut usize, bytes: usize) -> Result<(), String> {
    for _ in 0..bytes.div_ceil(64) {
        analysis_charge(work)?;
    }
    Ok(())
}
impl<'g, 'a> Builder<'g, 'a> {
    pub(super) fn new(
        grammar: &'g Grammar,
        admission: &'a mut RuntimeGuardedAdmission,
        work: &mut usize,
    ) -> Result<Self, String> {
        admission.retain_inventory(HEADER, work)?;
        Ok(Self {
            grammar,
            admission: Some(admission),
            paths: Vec::new(),
            guaranteed: 0,
            fallback: false,
            failed: false,
        })
    }
    fn retain(&mut self, amount: usize, work: &mut usize) -> Result<(), String> {
        self.admission
            .as_deref_mut()
            .ok_or("Nullable inventory lost its original companion")?
            .retain_inventory(amount, work)
    }
    fn grow(&mut self, work: &mut usize) -> Result<(), String> {
        if self.paths.len() < self.guaranteed {
            return Ok(());
        }
        let capacity = if self.guaranteed == 0 {
            4
        } else {
            self.guaranteed
                .checked_mul(2)
                .filter(|value| *value <= MAX_PATHS)
                .ok_or("Nullable inventory path count bound exceeded")?
        };
        let slot = SLOT;
        // Admit the FULL replacement buffer, with no refund of old buffers.
        self.retain(
            capacity
                .checked_mul(slot)
                .ok_or("Nullable inventory size overflow")?,
            work,
        )?;
        // Actual reserve moves live String handles, never their heap strings.
        bytes(
            work,
            self.paths
                .len()
                .checked_mul(slot)
                .ok_or("Nullable inventory move overflow")?,
        )?;
        self.paths
            .try_reserve_exact(capacity - self.paths.len())
            .map_err(|_| "Nullable inventory allocation failed")?;
        self.guaranteed = capacity;
        Ok(())
    }
    fn record(
        &mut self,
        owner: &str,
        rule: &Rule,
        expr: &Expr,
        path: &str,
        work: &mut usize,
    ) -> Result<(), String> {
        if self.failed {
            return Err("Nullable inventory builder has failed".into());
        }
        let result = self.record_inner(owner, rule, expr, path, work);
        if result.is_err() {
            self.failed = true;
            if let Some(admission) = self.admission.as_deref_mut() {
                admission.invalidate_inventory();
            }
        }
        result
    }
    fn record_inner(
        &mut self,
        owner: &str,
        rule: &Rule,
        expr: &Expr,
        path: &str,
        work: &mut usize,
    ) -> Result<(), String> {
        if self.fallback {
            return Ok(());
        }
        // Bounded structural inspection + every byte potentially searched for '/'.
        analysis_charge(work)?;
        bytes(work, owner.len())?;
        if owner.contains('/') {
            self.fallback = true;
            return Ok(());
        }
        if !super::super::nullable_repetition::inventory_may_qualify(rule, expr) {
            return Ok(());
        }
        let size = path
            .len()
            .checked_add("/repeat".len())
            .ok_or("Nullable inventory key size overflow")?;
        self.grow(work)?;
        self.retain(size, work)?;
        let mut key = String::new();
        key.try_reserve_exact(size)
            .map_err(|_| "Nullable inventory key allocation failed")?;
        key.push_str(path);
        key.push_str("/repeat");
        self.paths.push(key);
        Ok(())
    }
    fn compare(&self, left: usize, right: usize, work: &mut usize) -> Result<Ordering, String> {
        analysis_charge(work)?;
        bytes(work, self.paths[left].len().min(self.paths[right].len()))?;
        Ok(self.paths[left].cmp(&self.paths[right]))
    }
    fn swap(&mut self, left: usize, right: usize, work: &mut usize) -> Result<(), String> {
        analysis_charge(work)?;
        self.paths.swap(left, right);
        Ok(())
    }
    fn sift(
        &mut self,
        offset: usize,
        mut root: usize,
        end: usize,
        work: &mut usize,
    ) -> Result<(), String> {
        loop {
            analysis_charge(work)?; // Actual bounded heap step, including final leaf.
            let left = root
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or("Nullable inventory heap index overflow")?;
            if left >= end {
                return Ok(());
            }
            let right = left
                .checked_add(1)
                .ok_or("Nullable inventory heap index overflow")?;
            let mut greatest = left;
            if right < end && self.compare(offset + left, offset + right, work)? == Ordering::Less {
                greatest = right;
            }
            if self.compare(offset + root, offset + greatest, work)? != Ordering::Less {
                return Ok(());
            }
            self.swap(offset + root, offset + greatest, work)?;
            root = greatest;
        }
    }
    // Compare just the canonical owner prefix without allocating or copying.
    // A byte pair is prepaid before reading; no semantic callback is invoked.
    // None is an unexpected spelling, never authority to omit a key.
    fn same_owner(
        &self,
        left: usize,
        right: usize,
        work: &mut usize,
    ) -> Result<Option<bool>, String> {
        analysis_charge(work)?;
        let a = self.paths[left].as_bytes();
        let b = self.paths[right].as_bytes();
        let mut index = 0usize;
        loop {
            if index.is_multiple_of(32) {
                analysis_charge(work)?; // up to64 actual bytes, two per position
            }
            let (Some(&x), Some(&y)) = (a.get(index), b.get(index)) else {
                return Ok(None);
            };
            if index < 5 {
                if x != b"rule/"[index] || y != b"rule/"[index] {
                    return Ok(None);
                }
            } else if index == 5 && (x == b'/' || y == b'/') {
                return Ok(None); // empty owner
            } else if x != y {
                return Ok(Some(false));
            } else if x == b'/' {
                return Ok(Some(true));
            }
            index += 1;
        }
    }
    fn heap_block(&mut self, offset: usize, end: usize, work: &mut usize) -> Result<(), String> {
        // Private callers supply 0<=offset<=end<=paths.len(): the complete
        // global range, or the exact range found by the bounded block walk.
        let length = end - offset;
        // Every local index plus offset is strictly below end<=paths.len().
        for root in (0..length / 2).rev() {
            self.sift(offset, root, length, work)?;
        }
        for end in (1..length).rev() {
            self.swap(offset, offset + end, work)?;
            self.sift(offset, 0, end, work)?;
        }
        Ok(())
    }
    fn order_owner_blocks(&mut self, work: &mut usize) -> Result<bool, String> {
        let length = self.paths.len();
        // Same endpoint owners need no block walk: the literal global worker is
        // also the conservative fallback for an unexpected intervening owner.
        if self.same_owner(0, length - 1, work)? != Some(false) {
            return Ok(false);
        }
        let mut start = 0;
        while start < length {
            analysis_charge(work)?; // actual fixed block range/placement metadata
            if self.same_owner(start, start, work)? != Some(true) {
                return Ok(false);
            }
            let mut end = start + 1;
            while end < length {
                match self.same_owner(start, end, work)? {
                    Some(true) => end += 1,
                    Some(false) => break,
                    None => return Ok(false),
                }
            }
            self.heap_block(start, end, work)?;
            // Owner order alone is insufficient (A.B sorts before A/). Prove
            // the real full-key boundary AFTER sorting each complete block.
            // On any overlap/disorder, globally sort ALL rows with spent work.
            if start != 0 && self.compare(start - 1, start, work)? != Ordering::Less {
                return Ok(false);
            }
            start = end;
        }
        Ok(true)
    }
    fn order(&mut self, work: &mut usize) -> Result<(), String> {
        self.order_with_owner_blocks::<true>(work)
    }
    fn order_with_owner_blocks<const BLOCKS: bool>(
        &mut self,
        work: &mut usize,
    ) -> Result<(), String> {
        // In-place fallible heapsort: paid steps/comparisons/swaps, O(n log n).
        let length = self.paths.len();
        if length < 2 {
            return Ok(());
        }
        if !BLOCKS || !self.order_owner_blocks(work)? {
            self.heap_block(0, length, work)?;
        }
        // Pay final ordered/duplicate validation; no hidden unpriced scan.
        for index in 1..length {
            match self.compare(index - 1, index, work)? {
                Ordering::Less => {}
                Ordering::Equal => {
                    self.fallback = true;
                    return Ok(());
                }
                Ordering::Greater => {
                    return Err("Nullable inventory order validation failed".into());
                }
            }
        }
        Ok(())
    }
    /// Only the compiler calls this after its whole canonical graph succeeds.
    pub(super) fn finish<'p>(
        mut self,
        program: &'p ParserPrediction,
        work: &mut usize,
    ) -> Result<Finished<'g, 'p, 'a>, String> {
        if self.failed {
            return Err("Nullable inventory builder has failed".into());
        }
        analysis_charge(work)?;
        if !self.fallback {
            self.order(work)?;
        }
        let paths = if !self.fallback {
            Some(std::mem::take(&mut self.paths))
        } else {
            None
        };
        let admission = self
            .admission
            .take()
            .ok_or("Nullable inventory has already completed")?;
        Ok(Finished {
            grammar: self.grammar,
            program,
            admission,
            paths,
            consumed: false,
        })
    }
}
impl Drop for Builder<'_, '_> {
    fn drop(&mut self) {
        if let Some(admission) = self.admission.as_deref_mut() {
            // Error/unwinding cannot leave a reusable partially paid companion.
            admission.invalidate_inventory();
        }
    }
}
impl Finished<'_, '_, '_> {
    #[cfg(test)]
    pub(super) fn collect(
        self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
        observation: Option<AnalysisObservation<'_>>,
    ) -> Result<(), String> {
        self.collect_with_certificates(grammar, program, work, observation, None)
    }
    pub(super) fn collect_with_certificates(
        mut self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
        observation: Option<AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
    ) -> Result<(), String> {
        let result = (|| {
            let paths = if let Some(paths) = &self.paths {
                analysis_charge(work)?;
                if std::ptr::eq(self.grammar, grammar) && std::ptr::eq(self.program, program) {
                    Some(paths.as_slice())
                } else {
                    None // Foreign identity is no authority: use the whole old scan.
                }
            } else {
                None
            };
            match paths {
                Some(paths) => self.admission.collect_with_inventory_and_certificates(
                    grammar,
                    program,
                    work,
                    observation,
                    Some(paths),
                    certificates,
                ),
                None => self.admission.collect_with_inventory_and_certificates(
                    grammar,
                    program,
                    work,
                    observation,
                    None,
                    certificates,
                ),
            }
        })();
        if result.is_err() {
            self.admission.invalidate_inventory();
        }
        self.consumed = true;
        result
    }
    pub(super) fn collect_with_guard_facts(
        mut self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
        observation: Option<AnalysisObservation<'_>>,
        certificates: Option<&mut RepeatCertificates>,
        prepared: Option<&super::guard_facts::GuardFacts<'_>>,
    ) -> Result<(), String> {
        if prepared.is_none() {
            return self.collect_with_certificates(
                grammar,
                program,
                work,
                observation,
                certificates,
            );
        }

        let result = (|| {
            let paths = if let Some(paths) = &self.paths {
                analysis_charge(work)?;
                if std::ptr::eq(self.grammar, grammar) && std::ptr::eq(self.program, program) {
                    Some(paths.as_slice())
                } else {
                    None // Foreign identity is no authority: use the whole old scan.
                }
            } else {
                None
            };
            match paths {
                Some(paths) => self
                    .admission
                    .collect_with_inventory_certificates_and_guards(
                        grammar,
                        program,
                        work,
                        observation,
                        Some(paths),
                        certificates,
                        prepared,
                    ),
                None => self
                    .admission
                    .collect_with_inventory_certificates_and_guards(
                        grammar,
                        program,
                        work,
                        observation,
                        None,
                        certificates,
                        prepared,
                    ),
            }
        })();
        if result.is_err() {
            self.admission.invalidate_inventory();
        }
        self.consumed = true;
        result
    }
}
impl Drop for Finished<'_, '_, '_> {
    fn drop(&mut self) {
        if !self.consumed {
            self.admission.invalidate_inventory();
        }
    }
}

/// Same original pool; the original certificate worker receives it sequentially.
pub(super) struct GraphBuildContext<'g, 'p, 'a> {
    pub grammar: &'g Grammar,
    pub owner: &'g str,
    pub rule: &'g Rule,
    pub certificates: Option<&'p mut RepeatCertificates>,
    pub inventory: Option<&'p mut Builder<'g, 'a>>,
    pub remaining: &'p mut usize,
    pub exact: Option<&'p AdjacentWorkObservation>,
}
impl GraphBuildContext<'_, '_, '_> {
    pub(super) fn bind(
        &mut self,
        expr: &Expr,
        path: &str,
        binding: GraphBinding,
        continuation: Option<(&[Expr], usize, &str)>,
    ) -> Result<(), String> {
        // Every repeat, before the original certificate's candidate early return.
        if let Some(inventory) = self.inventory.as_deref_mut() {
            inventory.record(self.owner, self.rule, expr, path, self.remaining)?;
        }
        if let Some(certificates) = self.certificates.as_deref_mut() {
            GraphRepeatContext {
                grammar: self.grammar,
                owner: self.owner,
                certificates,
                remaining: self.remaining,
                exact: self.exact,
            }
            .bind(expr, path, binding, continuation)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
