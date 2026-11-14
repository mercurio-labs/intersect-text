//! Handwritten lossless document observations; no semantic graph or global cache.
//! Pools exist only while one document is captured. Arc values keep exact JSON
//! observations alive after the lookup index is discarded.
use super::{AnalysisLimits, AssociationUsage};
use intersect_model::definitions::{
    conversion::InvocationContext, metadata_limits::TraversalBudget,
    retained_storage::RETAINED_NODE_BYTES,
};
use intersect_model::storage::ValueView;
use serde::{Serialize, Serializer, ser::SerializeMap};
use serde_json::Value;
use std::{borrow::Borrow, cell::RefCell, collections::BTreeMap, io::Write, sync::Arc};

// Portable logical upper allowances for inline/index slots, independent of
// pointer width. Allocator capacity/RSS is not this accounting contract.
const FIELD: usize = 128;
const INDEX: usize = 128;
const ARC: usize = 16;
const BUCKET_ENTRY: usize = 48;
const POOL: usize = 64;
const _: () = assert!(std::mem::size_of::<(String, Arc<Value>)>() <= FIELD);
const _: () = assert!(std::mem::size_of::<(u64, Vec<PoolEntry>)>() <= INDEX);
const _: () = assert!(std::mem::size_of::<ValuePool>() <= POOL);
const _: () = assert!(std::mem::size_of::<PoolEntry>() <= BUCKET_ENTRY);

/// Immutable, sorted raw values with BTreeMap-compatible reads and identical
/// JSON map serialization. This replaces the public field's concrete Rust map
/// type; consumers that need an owned mutable map must request an observed export.
#[derive(Debug, Default)]
pub struct ParsedValues(BTreeMap<String, Arc<Value>>);
impl ParsedValues {
    pub(super) fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
    {
        self.0.get(key).map(Arc::as_ref)
    }
    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        String: Borrow<Q>,
    {
        self.0.contains_key(key)
    }
    pub fn get_key_value<Q: Ord + ?Sized>(&self, key: &Q) -> Option<(&String, &Value)>
    where
        String: Borrow<Q>,
    {
        self.0.get_key_value(key).map(|(k, v)| (k, v.as_ref()))
    }
    pub fn iter(&self) -> ParsedValuesIter<'_> {
        ParsedValuesIter(self.0.iter())
    }
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &String> + ExactSizeIterator {
        self.0.keys()
    }
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Value> + ExactSizeIterator {
        self.0.values().map(Arc::as_ref)
    }
    /// No implicit materialization on read or serialization. The caller pays
    /// for a complete independent map, even when internal values are shared.
    pub fn to_owned_map_observed(
        &self,
        observation: &InvocationContext<'_>,
    ) -> Result<BTreeMap<String, Value>, String> {
        observation.checkpoint()?;
        observation.retain_storage(intersect_model::logical_size!(BTreeMap<String, Value>, 24))?;
        let mut out = BTreeMap::new();
        for (name, value) in self.iter() {
            observation.work(1 + name.len().div_ceil(64))?;
            let mut traversal = TraversalBudget::new(usize::MAX, 64, usize::MAX);
            let payload = traversal.inspect_with_work(&value, &|units| observation.work(units))?;
            let nodes = traversal.visited_nodes();
            let bytes = nodes
                .checked_mul(RETAINED_NODE_BYTES)
                .and_then(|n| n.checked_add(payload))
                .and_then(|n| n.checked_add(FIELD))
                .and_then(|n| n.checked_add(name.len()))
                .ok_or("Document raw map export storage overflow")?;
            observation.retain_storage(bytes)?;
            observation.work(
                nodes
                    .checked_add(payload.div_ceil(64))
                    .and_then(|n| n.checked_add(name.len().div_ceil(64)))
                    .and_then(|n| n.checked_add(1))
                    .ok_or("Document raw map export work overflow")?,
            )?;
            observation.checkpoint()?;
            let height = usize::BITS as usize - out.len().max(1).leading_zeros() as usize;
            observation.work(
                height
                    .checked_mul(12)
                    .and_then(|n| n.checked_mul(1 + name.len().div_ceil(64)))
                    .ok_or("Document raw map insertion work overflow")?,
            )?;
            out.insert(name.clone(), value.clone());
        }
        observation.checkpoint()?;
        Ok(out)
    }
    pub(super) fn capture(
        &mut self,
        name: &str,
        value: ValueView<'_>,
        pool: &mut ValuePool,
        usage: &mut AssociationUsage,
        limits: AnalysisLimits,
    ) -> Result<(), String> {
        // Preserve previous last-write-wins map semantics, including duplicate
        // names. Charge every attempted slot/key rather than refund replacements.
        usage.admit(
            limits,
            FIELD
                .checked_add(name.len())
                .ok_or("Document raw field storage overflow")?,
            1,
        )?;
        probe(self.0.len(), name.len(), usage, limits)?;
        // Precharge the final field write before interning can retain a value.
        // A later field-work refusal must not leave an unused pool entry.
        usage.work(limits, 1 + name.len().div_ceil(64))?;
        let shared = pool.intern(value, usage, limits)?;
        self.0.insert(name.to_owned(), shared);
        Ok(())
    }
}
/// Borrowed sorted values. Iteration exposes existing raw JSON references;
/// it never clones/materializes maps or values.
#[derive(Clone, Debug)]
pub struct ParsedValuesIter<'a>(std::collections::btree_map::Iter<'a, String, Arc<Value>>);
impl<'a> Iterator for ParsedValuesIter<'a> {
    type Item = (&'a String, &'a Value);
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(name, value)| (name, value.as_ref()))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}
impl DoubleEndedIterator for ParsedValuesIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0
            .next_back()
            .map(|(name, value)| (name, value.as_ref()))
    }
}
impl ExactSizeIterator for ParsedValuesIter<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
}
impl std::iter::FusedIterator for ParsedValuesIter<'_> {}
impl<'a> IntoIterator for &'a ParsedValues {
    type Item = (&'a String, &'a Value);
    type IntoIter = ParsedValuesIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl Serialize for ParsedValues {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (name, value) in self.iter() {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}
impl<Q: Ord + ?Sized> std::ops::Index<&Q> for ParsedValues
where
    String: Borrow<Q>,
{
    type Output = Value;
    fn index(&self, key: &Q) -> &Value {
        self.0[key].as_ref()
    }
}

fn probe(
    count: usize,
    length: usize,
    usage: &mut AssociationUsage,
    limits: AnalysisLimits,
) -> Result<(), String> {
    // Conservative comparison count for BTree lookup+insertion, charging each
    // comparison's complete possible key length before calling the container.
    let height = usize::BITS as usize - count.max(1).leading_zeros() as usize;
    let units = height
        .checked_mul(12)
        .and_then(|n| n.checked_mul(1 + length.div_ceil(64)))
        .ok_or("Document raw value lookup work overflow")?;
    usage.work(limits, units)
}

struct PoolEntry {
    encoding: Vec<u8>,
    value: Arc<Value>,
}
// Deliberately coarse deterministic index, never equality/semantic authority.
// Fixed16-work preflight covers length/min, two bounded8-byte copies/word loads,
// two rotations, multiplication and xor. It never scans the whole encoded key.
fn fingerprint(
    bytes: &[u8],
    usage: &mut AssociationUsage,
    limits: AnalysisLimits,
) -> Result<u64, String> {
    usage.work(limits, 16)?;
    let count = bytes.len().min(8);
    let mut first = [0u8; 8];
    let mut last = [0u8; 8];
    first[..count].copy_from_slice(&bytes[..count]);
    last[..count].copy_from_slice(&bytes[bytes.len() - count..]);
    let length = u64::try_from(bytes.len()).map_err(|_| "Document fingerprint length overflow")?;
    Ok(u64::from_le_bytes(first).rotate_left(13)
        ^ u64::from_le_bytes(last).rotate_right(7)
        ^ length.wrapping_mul(0x9e3779b97f4a7c15))
}
fn fingerprint_probe(
    count: usize,
    usage: &mut AssociationUsage,
    limits: AnalysisLimits,
) -> Result<(), String> {
    let height = usize::BITS as usize - count.max(1).leading_zeros() as usize;
    // Same conservative tree height/comparison count as the byte-key index;
    // each comparison is now exactly one u64 comparison, not a variable byte scan.
    usage.work(
        limits,
        height
            .checked_mul(12)
            .ok_or("Document fingerprint lookup overflow")?,
    )
}
pub(super) struct ValuePool {
    values: BTreeMap<u64, Vec<PoolEntry>>,
    scratch: Vec<u8>,
    scratch_paid: usize,
}
impl ValuePool {
    pub(super) fn new(
        usage: &mut AssociationUsage,
        limits: AnalysisLimits,
    ) -> Result<Self, String> {
        usage.admit(limits, POOL, 1)?;
        Ok(Self {
            values: BTreeMap::new(),
            scratch: Vec::new(),
            scratch_paid: 0,
        })
    }
    #[cfg(test)]
    fn retained_value_count(&self) -> usize {
        self.values.values().map(Vec::len).sum()
    }
    fn intern(
        &mut self,
        value: ValueView<'_>,
        usage: &mut AssociationUsage,
        limits: AnalysisLimits,
    ) -> Result<Arc<Value>, String> {
        // Trusted boundary: only the model's sealed ValueView may enter. Its
        // Serialize/to_json arms are source-equivalent for Null/Default, Json,
        // Bool/Number/Text/Other, identities/unavailable identities, unresolved
        // spelling, Lists, References and MixedReferences. A duplicate hit does
        // not invoke to_json or accept arbitrary caller conversion callbacks.
        // The unique-value stream check additionally guards integration drift.
        // Inspect the borrowed raw value. Its payload may already be retained;
        // do not incorrectly apply remaining *unique storage* to duplicate walks.
        let mut traversal =
            TraversalBudget::new(limits.association_operations, 64, limits.association_bytes);
        let payload = {
            let usage = RefCell::new(&mut *usage);
            traversal.inspect_with_work(&value, &|n| usage.borrow_mut().work(limits, n))?
        };
        let nodes = traversal.visited_nodes();
        // Pay the serializer traversal before performing the counting walk.
        usage.work(
            limits,
            nodes
                .checked_add(payload.div_ceil(64))
                .ok_or("Document raw serialization work overflow")?,
        )?;
        let mut count = CountWriter {
            bytes: 0,
            limit: limits.association_bytes,
            usage,
            limits,
        };
        serde_json::to_writer(&mut count, &value)
            .map_err(|e| format!("Document raw value encoding: {e}"))?;
        let length = count.bytes;
        if length > self.scratch_paid {
            usage.admit(limits, length - self.scratch_paid, 0)?;
            self.scratch_paid = length;
        }
        self.scratch.clear();
        self.scratch
            .try_reserve_exact(length)
            .map_err(|_| "Document raw encoding allocation failed")?;
        usage.work(
            limits,
            nodes
                .checked_add(payload.div_ceil(64))
                .ok_or("Document raw serialization work overflow")?,
        )?;
        let mut output = BoundedWriter {
            output: &mut self.scratch,
            limit: length,
            usage,
            limits,
        };
        serde_json::to_writer(&mut output, &value)
            .map_err(|e| format!("Document raw value encoding: {e}"))?;
        if self.scratch.len() != length {
            return Err("Document raw value encoding changed".into());
        }
        let fingerprint = fingerprint(&self.scratch, usage, limits)?;
        fingerprint_probe(self.values.len(), usage, limits)?;
        let (bucket_len, bucket_capacity) = if let Some(bucket) = self.values.get(&fingerprint) {
            for entry in bucket {
                // Exact byte equality remains mandatory on every hit. Length
                // mismatch is a constant check; equal lengths pay the full scan
                // before comparing, including deliberately adversarial collisions.
                usage.work(
                    limits,
                    1 + if entry.encoding.len() == length {
                        length.div_ceil(64)
                    } else {
                        0
                    },
                )?;
                if entry.encoding == self.scratch {
                    usage.work(limits, 1)?;
                    return Ok(Arc::clone(&entry.value));
                }
            }
            (bucket.len(), bucket.capacity())
        } else {
            (0, 0)
        };
        if bucket_capacity == 0 {
            usage.admit(limits, INDEX, 1)?;
        }
        // All entry slots/bucket storage, including temporarily concurrent old
        // and new allocations, are admitted cumulatively before reserve/moves.
        let mut prepared_bucket = if bucket_len == bucket_capacity {
            let capacity = bucket_capacity
                .max(1)
                .checked_mul(if bucket_capacity == 0 { 1 } else { 2 })
                .ok_or("Document collision bucket capacity overflow")?;
            let bytes = capacity
                .checked_mul(BUCKET_ENTRY)
                .and_then(|n| n.checked_add(24))
                .ok_or("Document collision bucket storage overflow")?;
            usage.admit(limits, bytes, capacity)?;
            usage.work(limits, 1 + bucket_len)?;
            let mut next = Vec::new();
            next.try_reserve_exact(capacity)
                .map_err(|_| "Document collision bucket allocation failed")?;
            if next.capacity() != capacity {
                return Err("Document collision bucket capacity differs from admission".into());
            }
            Some(next)
        } else {
            None
        };
        let bytes = nodes
            .checked_mul(RETAINED_NODE_BYTES)
            .and_then(|n| n.checked_add(payload))
            .and_then(|n| n.checked_add(ARC))
            .and_then(|n| n.checked_add(length))
            .ok_or("Document shared raw value storage overflow")?;
        usage.admit(limits, bytes, nodes)?;
        // to_json clones this exact already-bounded value. Compare the resulting
        // stream with the borrowed encoding without parsing or allocating again.
        usage.work(
            limits,
            nodes
                .checked_add(payload.div_ceil(64))
                .and_then(|n| n.checked_mul(2))
                .ok_or("Document raw clone work overflow")?,
        )?;
        let owned = value.to_json();
        let mut verify = EqualWriter {
            expected: &self.scratch,
            position: 0,
            usage,
            limits,
        };
        serde_json::to_writer(&mut verify, &owned)
            .map_err(|e| format!("Document raw value correspondence: {e}"))?;
        if verify.position != length {
            return Err("Document raw value correspondence length changed".into());
        }
        usage.work(limits, 3 + length.div_ceil(64))?;
        let shared = Arc::new(owned);
        let entry = PoolEntry {
            encoding: self.scratch.clone(),
            value: Arc::clone(&shared),
        };
        if let Some(mut bucket) = prepared_bucket.take() {
            if bucket_capacity == 0 {
                bucket.push(entry);
                self.values.insert(fingerprint, bucket);
            } else {
                let previous = self
                    .values
                    .get_mut(&fingerprint)
                    .ok_or("Document collision bucket disappeared")?;
                bucket.append(previous);
                bucket.push(entry);
                *previous = bucket;
            }
        } else {
            self.values
                .get_mut(&fingerprint)
                .ok_or("Document collision bucket disappeared")?
                .push(entry);
        }
        Ok(shared)
    }
}
fn io_error(error: impl Into<String>) -> std::io::Error {
    std::io::Error::other(error.into())
}
struct CountWriter<'a> {
    bytes: usize,
    limit: usize,
    usage: &'a mut AssociationUsage,
    limits: AnalysisLimits,
}
impl Write for CountWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.usage
            .work(self.limits, 1 + bytes.len().div_ceil(64))
            .map_err(io_error)?;
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.limit)
            .ok_or_else(|| io_error("Document raw encoding exceeds association byte limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct BoundedWriter<'a> {
    output: &'a mut Vec<u8>,
    limit: usize,
    usage: &'a mut AssociationUsage,
    limits: AnalysisLimits,
}
impl Write for BoundedWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.usage
            .work(self.limits, 1 + bytes.len().div_ceil(64))
            .map_err(io_error)?;
        if self
            .output
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.limit)
        {
            return Err(io_error("Document raw encoding changed after counting"));
        }
        self.output.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct EqualWriter<'a> {
    expected: &'a [u8],
    position: usize,
    usage: &'a mut AssociationUsage,
    limits: AnalysisLimits,
}
impl Write for EqualWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.usage
            .work(self.limits, 1 + bytes.len().div_ceil(64))
            .map_err(io_error)?;
        let end = self
            .position
            .checked_add(bytes.len())
            .ok_or_else(|| io_error("Document raw comparison overflow"))?;
        if self.expected.get(self.position..end) != Some(bytes) {
            return Err(io_error(
                "Document borrowed and owned raw representations differ",
            ));
        }
        self.position = end;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "document_values_tests.rs"]
mod tests;
