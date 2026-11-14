//! Explicit language-owned raw-source/logical-resource table for a closed input.
//! The table is immutable and grants no model, source-text or semantic authority.
use super::*;
use intersect_model::core::text_document::DocumentResourceAssociation;

/// No normalization or percent decoding occurs. Every mapping is supplied
/// explicitly; a raw encoded path and a raw space path cannot silently alias.
pub struct ClosedSourceResources {
    rows: Vec<DocumentResourceAssociation>,
    retained_bytes: usize,
}
const _: () = assert!(std::mem::size_of::<ClosedSourceResources>() <= 32);
impl ClosedSourceResources {
    /// Portable logical header; the old/default document counters stay exact.
    pub const LOGICAL_HEADER_BYTES: usize = 32;
    pub fn prepare(
        declarations: &[(&str, &str, &str)],
        limits: document::AnalysisLimits,
        budget: &mut QueryBudget,
    ) -> StorageResult<Self> {
        budget.charge()?;
        if declarations.is_empty()
            || declarations.len() > limits.resources
            || declarations.len() > limits.association_records
        {
            return Err("Closed source/resource association count budget exceeded".into());
        }
        let mut result = Self {
            rows: Vec::new(),
            retained_bytes: Self::LOGICAL_HEADER_BYTES,
        };
        for &(uri, digest, resource) in declarations {
            for old in &result.rows {
                budget.charge()?;
                if old.source().uri == uri || old.resource_uri() == resource {
                    return Err("Duplicate explicit source or logical resource association".into());
                }
            }
            let remaining = limits
                .association_bytes
                .checked_sub(result.retained_bytes)
                .ok_or("Closed source/resource association storage budget exceeded")?;
            let row =
                DocumentResourceAssociation::prepare((uri, digest), resource, remaining, budget)?;
            budget.charge()?;
            result
                .rows
                .try_reserve_exact(1)
                .map_err(|_| "Closed source/resource row allocation failed")?;
            result.retained_bytes = result
                .retained_bytes
                .checked_add(row.retained_bytes())
                .ok_or("Closed source/resource association storage overflow")?;
            result.rows.push(row);
        }
        Ok(result)
    }
    /// Read-only association DATA in its existing declaration order. This
    /// borrows the exact admitted rows without allocation or a lookup; it grants
    /// no model, text-correspondence, mutation or semantic-validation authority.
    pub fn associations(&self) -> &[DocumentResourceAssociation] {
        &self.rows
    }
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    /// A missing row keeps the identity URI policy. A row with the same raw URI
    /// and different content never falls back to that policy.
    pub fn association(
        &self,
        source: &SourceId,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<&DocumentResourceAssociation>> {
        DocumentResourceAssociation::select_with_work(&self.rows, source, || budget.charge())
    }
}
