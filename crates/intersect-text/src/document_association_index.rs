//! Temporary borrowed identity projection. No object copy, semantic graph,
//! mutation authority or persistent cache. Duplicate rows remain detectable.
use super::{AnalysisLimits, AssociationUsage};
const SLOT: usize = 32;
const _: () = assert!(std::mem::size_of::<(&str, usize)>() <= SLOT);
pub(super) struct IdentityIndex<'a> {
    entries: Vec<(&'a str, usize)>,
}
impl<'a> IdentityIndex<'a> {
    pub(super) fn prepare(
        identities: impl ExactSizeIterator<Item = (&'a str, usize)>,
        usage: &mut AssociationUsage,
        limits: AnalysisLimits,
    ) -> Result<Self, String> {
        let count = identities.len();
        let records = count
            .checked_mul(2)
            .ok_or("Document identity index record overflow")?;
        let bytes = records
            .checked_mul(SLOT)
            .and_then(|n| n.checked_add(48))
            .ok_or("Document identity index storage overflow")?;
        usage.admit(limits, bytes, records)?;
        let mut entries = Vec::new();
        let mut scratch = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| "Document identity index allocation failed")?;
        scratch
            .try_reserve_exact(count)
            .map_err(|_| "Document identity sort allocation failed")?;
        for identity in identities {
            usage.work(limits, 1)?;
            entries.push(identity);
        }
        // Explicit bottom-up merge: every comparison and copied borrowed slot
        // has a fallible precharge; no unobservable standard-library sort work.
        let mut width = 1;
        while width < count {
            scratch.clear();
            let mut start = 0;
            while start < count {
                let middle = start.saturating_add(width).min(count);
                let end = middle.saturating_add(width).min(count);
                let (mut left, mut right) = (start, middle);
                while left < middle || right < end {
                    let take_left = if left == middle {
                        false
                    } else if right == end {
                        true
                    } else {
                        compare(entries[left].0, entries[right].0, usage, limits)?
                            != std::cmp::Ordering::Greater
                    };
                    usage.work(limits, 1)?;
                    if take_left {
                        scratch.push(entries[left]);
                        left += 1;
                    } else {
                        scratch.push(entries[right]);
                        right += 1;
                    }
                }
                start = end;
            }
            std::mem::swap(&mut entries, &mut scratch);
            width = width
                .checked_mul(2)
                .ok_or("Document identity sort width overflow")?;
        }
        Ok(Self { entries })
    }
    pub(super) fn lookup(
        &self,
        id: &str,
        duplicate: &str,
        usage: &mut AssociationUsage,
        limits: AnalysisLimits,
    ) -> Result<Option<usize>, String> {
        let (mut low, mut high) = (0, self.entries.len());
        while low < high {
            let middle = low + (high - low) / 2;
            match compare(self.entries[middle].0, id, usage, limits)? {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => {
                    if (middle > 0
                        && compare(self.entries[middle - 1].0, id, usage, limits)?
                            == std::cmp::Ordering::Equal)
                        || (middle + 1 < self.entries.len()
                            && compare(self.entries[middle + 1].0, id, usage, limits)?
                                == std::cmp::Ordering::Equal)
                    {
                        return Err(duplicate.into());
                    }
                    return Ok(Some(self.entries[middle].1));
                }
            }
        }
        Ok(None)
    }
}
fn compare(
    a: &str,
    b: &str,
    usage: &mut AssociationUsage,
    limits: AnalysisLimits,
) -> Result<std::cmp::Ordering, String> {
    usage.work(limits, 1 + a.len().max(b.len()).div_ceil(64))?;
    Ok(a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_identity_index_preserves_ordinals_and_detects_only_queried_duplicates() {
        let limits = AnalysisLimits::workbench();
        let mut usage = AssociationUsage::default();
        let rows = [("z", 0), ("same", 1), ("a", 2), ("same", 3), ("middle", 4)];
        let index = IdentityIndex::prepare(rows.into_iter(), &mut usage, limits).unwrap();
        assert_eq!(
            index.lookup("a", "duplicate", &mut usage, limits).unwrap(),
            Some(2)
        );
        assert_eq!(
            index
                .lookup("middle", "duplicate", &mut usage, limits)
                .unwrap(),
            Some(4)
        );
        assert_eq!(
            index
                .lookup("missing", "duplicate", &mut usage, limits)
                .unwrap(),
            None
        );
        assert_eq!(
            index
                .lookup("same", "duplicate", &mut usage, limits)
                .unwrap_err(),
            "duplicate"
        );
    }
    #[test]
    fn document_identity_index_bounded_merge_and_short_controls() {
        let rows: Vec<_> = (0..2000)
            .rev()
            .map(|i| format!("exact://source#{i:05}"))
            .collect();
        let limits = AnalysisLimits::workbench();
        let mut usage = AssociationUsage::default();
        let index = IdentityIndex::prepare(
            rows.iter().enumerate().map(|(i, s)| (s.as_str(), i)),
            &mut usage,
            limits,
        )
        .unwrap();
        assert_eq!(
            index
                .lookup("exact://source#00000", "duplicate", &mut usage, limits)
                .unwrap(),
            Some(1999)
        );
        // The old nested scan of 2000 owners by 2000 rows alone costs4m.
        assert!(usage.operations < 200_000);
        assert_eq!(usage.records, 4000);
        for short in [
            AnalysisLimits {
                association_bytes: usage.bytes - 1,
                ..limits
            },
            AnalysisLimits {
                association_records: usage.records - 1,
                ..limits
            },
            AnalysisLimits {
                association_operations: 1,
                ..limits
            },
        ] {
            let mut used = AssociationUsage::default();
            assert!(
                IdentityIndex::prepare(
                    rows.iter().enumerate().map(|(i, s)| (s.as_str(), i)),
                    &mut used,
                    short
                )
                .is_err()
            );
        }
    }
}
