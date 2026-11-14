//! Handwritten native alternative to the Rhai overdueFee implementation.
//! Register this for overdueFee and retain Rhai for copyLabel to mix providers.
use intersect_model::semantic_bindings::{Call, Error, ErrorKind, Result};

pub fn overdue_fee(call: &Call<'_>, days_late: i64) -> Result<i64> {
    call.budget.charge(1)?;
    days_late
        .max(0)
        .checked_mul(25)
        .ok_or_else(|| Error::new(ErrorKind::Semantic, "Overdue fee exceeds Int64"))
}
