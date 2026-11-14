//! Compiler-invocation borrows of already completed canonical FIRST results.
//! These supply only false nullability for the EXACT unquantified First Call.
//! No serialized field, source binding, symbol-set answer or standalone cache.
use super::{RuntimeGuardedAdmission, analysis_charge};
use crate::grammar::Expr;
use crate::grammar::Grammar;
use crate::grammar::Kind;

const HEADER: usize = 64;
const SLOT: usize = 16;

pub(super) struct GuardFacts<'g> {
    grammar: &'g Grammar,
    entries: Vec<&'g Expr>,
    guaranteed: usize,
    admitted: bool,
    failed: bool,
}
const _: () = {
    assert!(std::mem::size_of::<Option<GuardFacts<'_>>>() <= HEADER);
    assert!(std::mem::size_of::<&Expr>() <= SLOT);
};

impl<'g> GuardFacts<'g> {
    pub(super) fn new(grammar: &'g Grammar) -> Self {
        Self {
            grammar,
            entries: Vec::new(),
            guaranteed: 0,
            admitted: false,
            failed: false,
        }
    }

    // Called only immediately after the existing canonical FIRST returns Ok.
    // Retain only the shape where summary.1 IS the later unquantified answer.
    pub(super) fn record(
        &mut self,
        expr: &'g Expr,
        nullable: bool,
        admission: &mut RuntimeGuardedAdmission,
        work: &mut usize,
    ) -> Result<(), String> {
        let result: Result<(), String> = (|| {
            if self.failed {
                return Err("Prepared First guard facts already failed".into());
            }
            analysis_charge(work)?; // inspect the already-returned nullable scalar
            if nullable {
                return Ok(());
            }
            for _ in 0..3 {
                analysis_charge(work)?;
            }
            if expr.cardinality != '1'
                || !expr.first_set_predicated
                || !matches!(expr.kind, Kind::Call(_))
            {
                return Ok(());
            }
            if !self.admitted {
                admission.retain_nonnullable_lemma(HEADER, work)?;
                self.admitted = true;
            }
            if self.entries.len() == self.guaranteed {
                let capacity = if self.guaranteed == 0 {
                    4
                } else {
                    self.guaranteed
                        .checked_mul(2)
                        .ok_or("Prepared First guard capacity overflow")?
                };
                let bytes = capacity
                    .checked_mul(SLOT)
                    .ok_or("Prepared First guard bytes overflow")?;
                // Full replacement capacity, including spare slots, is cumulative.
                // allocator.capacity() never grants an uncharged logical slot.
                admission.retain_nonnullable_lemma(bytes, work)?;
                analysis_charge(work)?; // actual replacement allocation
                analysis_charge(work)?; // prepay this allocation's eventual deallocation
                for _ in 0..self
                    .entries
                    .len()
                    .checked_mul(SLOT)
                    .ok_or("Prepared First guard move overflow")?
                    .div_ceil(64)
                {
                    analysis_charge(work)?;
                }
                let mut replacement = Vec::new();
                replacement
                    .try_reserve_exact(capacity)
                    .map_err(|_| "Prepared First guard allocation failed")?;
                replacement.extend(self.entries.iter().copied());
                self.entries = replacement;
                self.guaranteed = capacity;
            }
            analysis_charge(work)?; // actual slot write
            self.entries.push(expr);
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
            admission.invalidate_inventory();
        }
        result
    }

    pub(super) fn nonnullable(
        &self,
        grammar: &Grammar,
        expr: &Expr,
        work: &mut usize,
    ) -> Result<bool, String> {
        if self.failed {
            return Err("Prepared First guard facts already failed".into());
        }
        if self.entries.is_empty() {
            return Ok(false);
        }
        analysis_charge(work)?; // exact invocation grammar identity
        if !std::ptr::eq(self.grammar, grammar) {
            return Ok(false);
        }
        for stored in &self.entries {
            analysis_charge(work)?; // bounds, pointer read and exact expression identity
            if std::ptr::eq(*stored, expr) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
#[path = "guard_facts_tests.rs"]
mod tests;
