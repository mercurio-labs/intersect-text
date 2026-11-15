//! Shared-ledger bookkeeping for the existing resource parser.
//!
//! The default policy is a ZST identity operation. An opted-in draft invocation
//! borrows its original caller ledger; this module creates no allowance and
//! owns no source, grammar, model, semantic or publication authority.

use super::EvaluationLimits;
use intersect_model::core::query::QueryBudget;

pub(super) trait ResourceAccounting {
    // Explicit shared ledgers retain their existing canonical linking path.
    fn reuse_link_metadata(&self) -> bool {
        false
    }
    fn begin(&mut self, limits: EvaluationLimits) -> EvaluationLimits;
    fn finish(
        &mut self,
        resource_work: usize,
        report: &mut Option<&mut usize>,
    ) -> Result<(), String>;
}

pub(super) struct NoResourceAccounting;

impl ResourceAccounting for NoResourceAccounting {
    fn reuse_link_metadata(&self) -> bool {
        true
    }
    fn begin(&mut self, limits: EvaluationLimits) -> EvaluationLimits {
        limits
    }

    fn finish(&mut self, _: usize, _: &mut Option<&mut usize>) -> Result<(), String> {
        Ok(())
    }
}

/// Explicit project accounting, preserving the old per-resource policy.
pub(super) struct SharedResourceAccounting<'a> {
    budget: &'a mut QueryBudget,
    granted: usize,
    work: usize,
}

impl<'a> SharedResourceAccounting<'a> {
    pub(super) fn new(budget: &'a mut QueryBudget) -> Self {
        Self {
            budget,
            granted: 0,
            work: 0,
        }
    }
}

impl ResourceAccounting for SharedResourceAccounting<'_> {
    fn begin(&mut self, mut limits: EvaluationLimits) -> EvaluationLimits {
        self.granted = self
            .budget
            .remaining()
            .min(limits.steps_per_resource.unwrap_or(usize::MAX));
        limits.steps_per_resource = Some(self.granted);
        limits
    }

    fn finish(
        &mut self,
        resource_work: usize,
        report: &mut Option<&mut usize>,
    ) -> Result<(), String> {
        // This bounded loop transfers already reported lex/Machine units to
        // the original ledger. It is bookkeeping, not a new CPU fee formula.
        let admitted = resource_work.min(self.granted);
        match self.work.checked_add(admitted) {
            // One bulk transfer with the same ledger as `admitted` single
            // charges: on exhaustion, exactly the units the budget still had
            // are charged and counted, then the same error is returned.
            Some(_) => {
                let consumed = admitted.min(self.budget.remaining());
                let charged = self.budget.charge_many(admitted);
                self.work += consumed;
                charged?;
            }
            None => {
                for _ in 0..admitted {
                    self.budget.charge()?;
                    self.work = self
                        .work
                        .checked_add(1)
                        .ok_or("Cumulative resource parse work overflow")?;
                }
            }
        }
        if let Some(report) = report.as_deref_mut() {
            *report = self.work;
        }
        if resource_work > self.granted {
            return Err("Resource parse work exceeds its shared caller grant".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_resource_accounting_keeps_link_metadata_uncached() {
        assert!(NoResourceAccounting.reuse_link_metadata());
        let mut budget = QueryBudget::new(7);
        assert!(!SharedResourceAccounting::new(&mut budget).reuse_link_metadata());
        assert_eq!(budget.reads, 0);
        assert_eq!(budget.remaining(), 7);
    }

    /// The previous unit-at-a-time transfer, as the oracle.
    fn reference(
        budget: &mut QueryBudget,
        granted: usize,
        work: &mut usize,
        resource_work: usize,
        report: &mut usize,
    ) -> Result<(), String> {
        for _ in 0..resource_work.min(granted) {
            budget.charge()?;
            *work = work
                .checked_add(1)
                .ok_or("Cumulative resource parse work overflow")?;
        }
        *report = *work;
        if resource_work > granted {
            return Err("Resource parse work exceeds its shared caller grant".into());
        }
        Ok(())
    }

    #[test]
    fn bulk_finish_keeps_the_unit_ledger() {
        for remaining in [0, 1, 7, 50] {
            for granted in [0, 3, 7, 60] {
                for prior in [0, 4] {
                    for resource_work in [0, 1, 6, 7, 8, 59, 61] {
                        let mut old = QueryBudget::new(remaining);
                        let (mut old_work, mut old_report) = (prior, usize::MAX);
                        let old_result = reference(
                            &mut old,
                            granted,
                            &mut old_work,
                            resource_work,
                            &mut old_report,
                        );
                        let mut new = QueryBudget::new(remaining);
                        let mut new_report = usize::MAX;
                        let (new_result, new_work) = {
                            let mut accounting = SharedResourceAccounting {
                                budget: &mut new,
                                granted,
                                work: prior,
                            };
                            let result =
                                accounting.finish(resource_work, &mut Some(&mut new_report));
                            (result, accounting.work)
                        };
                        assert_eq!(new_result, old_result);
                        assert_eq!(new_work, old_work);
                        assert_eq!(new.remaining(), old.remaining());
                        assert_eq!(new.reads, old.reads);
                        assert_eq!(new_report, old_report);
                    }
                }
            }
        }
    }
}
