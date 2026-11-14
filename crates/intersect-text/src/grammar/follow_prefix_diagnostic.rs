//! Explicit bounded diagnostic for ONE already-required FOLLOW FIRST query.
//! A source observation is never a parser certificate or semantic authority.
use super::first_sets::{OccurrenceFacts, prefix_capture::SourcePrefixInspection};
use super::{Expr, Grammar};
use std::cell::Cell;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefixDiagnosticText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}
impl<const N: usize> PrefixDiagnosticText<N> {
    fn copy(value: &str) -> Option<Self> {
        if value.len() > N {
            return None;
        }
        let mut result = Self {
            bytes: [0; N],
            len: value.len(),
        };
        result.bytes[..value.len()].copy_from_slice(value.as_bytes());
        Some(result)
    }
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.bytes[..self.len])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixDiagnosticState {
    Complete,
    Unavailable,
    Denied,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixDiagnosticAbsence {
    None,
    SourceBinding,
    RequestedCalleeNotTraversed,
    FixedCaptureCapacity,
    FirstFailure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefixDiagnosticSource {
    pub uri: PrefixDiagnosticText<512>,
    pub sha256: PrefixDiagnosticText<64>,
    pub declaration: (usize, usize),
    pub body: (usize, usize),
}
/// All arrays are fixed. Missing/overlong identities are never truncated into
/// identities. `Unavailable` preserves an explicit reason and capture flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePrefixDiagnosticRecord {
    pub state: PrefixDiagnosticState,
    pub absence: PrefixDiagnosticAbsence,
    pub owner: Option<PrefixDiagnosticText<128>>,
    pub path: Option<PrefixDiagnosticText<256>>,
    pub source: Option<PrefixDiagnosticSource>,
    pub nullable: Option<bool>,
    pub body_nullable: Option<bool>,
    pub prefix_hidden_policy: Option<bool>,
    pub maximum_source_call_depth: Option<usize>,
    pub first: [Option<PrefixDiagnosticText<256>>; 16],
    pub first_count: usize,
    pub callee: Option<PrefixDiagnosticText<128>>,
    pub callee_source: Option<PrefixDiagnosticSource>,
    pub callee_nullable: Option<bool>,
    pub callee_first: [Option<PrefixDiagnosticText<256>>; 16],
    pub callee_first_count: usize,
    pub original_first_remaining: usize,
    pub error: Option<PrefixDiagnosticText<256>>,
    pub capture_unavailable: bool,
}
impl SourcePrefixDiagnosticRecord {
    fn empty(remaining: usize) -> Self {
        Self {
            state: PrefixDiagnosticState::Unavailable,
            absence: PrefixDiagnosticAbsence::SourceBinding,
            owner: None,
            path: None,
            source: None,
            nullable: None,
            body_nullable: None,
            prefix_hidden_policy: None,
            maximum_source_call_depth: None,
            first: [None; 16],
            first_count: 0,
            callee: None,
            callee_source: None,
            callee_nullable: None,
            callee_first: [None; 16],
            callee_first_count: 0,
            original_first_remaining: remaining,
            error: None,
            capture_unavailable: false,
        }
    }
}

/// Caller-owned, non-Sync and invocation-local. It captures at most ONE selected
/// query and ONE requested callee. `record()==None` means the selected occurrence
/// was not reached. No observer or work authority is stored in old holders.
pub struct SourcePrefixDiagnostic<'a> {
    owner: &'a str,
    path: &'a str,
    callee: &'a str,
    captured: Cell<bool>,
    started: Cell<bool>,
    record: Cell<Option<SourcePrefixDiagnosticRecord>>,
}
impl<'a> SourcePrefixDiagnostic<'a> {
    /// `path` is the exact FOLLOW occurrence path, without `/repeat`.
    pub fn new(owner: &'a str, path: &'a str, callee: &'a str) -> Self {
        Self {
            owner,
            path,
            callee,
            captured: Cell::new(false),
            started: Cell::new(false),
            record: Cell::new(None),
        }
    }
    pub fn record(&self) -> Option<SourcePrefixDiagnosticRecord> {
        self.record.get()
    }
    pub fn begin(&self) -> Result<(), String> {
        if self.started.replace(true) {
            return Err(
                "Source prefix diagnostic belongs to a previous compilation invocation".into(),
            );
        }
        Ok(())
    }
    pub(super) fn selects(&self, owner: Option<&str>, path: &str) -> bool {
        owner == Some(self.owner) && path == self.path && !self.captured.get()
    }
    fn pay(work: &mut usize) -> Result<(), String> {
        if *work == 0 {
            return Err("Grammar prediction budget exceeded".into());
        }
        *work -= 1;
        Ok(())
    }
    fn text<const N: usize>(
        value: &str,
        work: &mut usize,
    ) -> Result<Option<PrefixDiagnosticText<N>>, String> {
        Self::pay(work)?;
        super::first_sets::prefix_capture::reserve_copy_bytes(value.len(), work)?;
        Ok(PrefixDiagnosticText::copy(value))
    }
    fn source(
        source: &intersect_model::definitions::bundle::SourceSpan,
        body: Option<&std::ops::Range<usize>>,
        work: &mut usize,
    ) -> Result<Option<PrefixDiagnosticSource>, String> {
        Self::pay(work)?;
        let uri = Self::text(&source.source.uri, work)?;
        let sha256 = Self::text(&source.source.sha256, work)?;
        Ok(match (uri, sha256, body) {
            (Some(uri), Some(sha256), Some(body)) => Some(PrefixDiagnosticSource {
                uri,
                sha256,
                declaration: (source.start, source.end),
                body: (body.start, body.end),
            }),
            _ => None,
        })
    }
    fn symbols(
        values: &BTreeSet<String>,
        work: &mut usize,
    ) -> Result<([Option<PrefixDiagnosticText<256>>; 16], bool), String> {
        let mut result = [None; 16];
        let mut unavailable = values.len() > result.len();
        for (slot, value) in result.iter_mut().zip(values) {
            *slot = Self::text(value, work)?;
            unavailable |= slot.is_none();
        }
        Ok((result, unavailable))
    }
    fn capture(
        &self,
        facts: &SourcePrefixInspection<'_>,
        work: &mut usize,
    ) -> Result<SourcePrefixDiagnosticRecord, String> {
        Self::pay(work)?;
        let mut record = SourcePrefixDiagnosticRecord::empty(*work);
        record.owner = Self::text(facts.owner(), work)?;
        record.path = Self::text(facts.path(), work)?;
        record.source = Self::source(
            facts.source(),
            facts.expression().original_range.as_ref(),
            work,
        )?;
        record.nullable = Some(facts.nullable());
        record.body_nullable = facts.body_nullable();
        record.prefix_hidden_policy = Some(facts.prefix_hidden_policy());
        record.maximum_source_call_depth = Some(facts.maximum_call_depth());
        let (first, unavailable) = Self::symbols(facts.symbols(), work)?;
        record.first = first;
        record.first_count = facts.symbols().len();
        record.capture_unavailable = unavailable
            || record.owner.is_none()
            || record.path.is_none()
            || record.source.is_none();
        for callee in facts.calls() {
            Self::pay(work)?; // Each actual completed callee comparison.
            if callee.name() != self.callee {
                continue;
            }
            record.callee = Self::text(callee.name(), work)?;
            record.callee_source = Self::source(
                callee.source(),
                callee.rule().body.original_range.as_ref(),
                work,
            )?;
            record.callee_nullable = Some(callee.nullable());
            let (first, unavailable) = Self::symbols(callee.symbols(), work)?;
            record.callee_first = first;
            record.callee_first_count = callee.symbols().len();
            record.capture_unavailable |=
                unavailable || record.callee.is_none() || record.callee_source.is_none();
            break;
        }
        record.absence = if record.capture_unavailable {
            PrefixDiagnosticAbsence::FixedCaptureCapacity
        } else if record.callee.is_none() {
            PrefixDiagnosticAbsence::RequestedCalleeNotTraversed
        } else {
            PrefixDiagnosticAbsence::None
        };
        record.state = if record.absence == PrefixDiagnosticAbsence::None {
            PrefixDiagnosticState::Complete
        } else {
            PrefixDiagnosticState::Unavailable
        };
        record.original_first_remaining = *work;
        Ok(record)
    }
    /// Called INSTEAD OF the existing FIRST query, with its same original pool.
    /// No second query, allowance, source clone, semantic cache or graph grant.
    pub(super) fn analyze(
        &self,
        grammar: &Grammar,
        expr: &Expr,
        owner: Option<&str>,
        path: &str,
        remaining: &mut usize,
    ) -> Result<((BTreeSet<String>, bool), OccurrenceFacts), String> {
        let result: Result<_, String> = (|| {
            Self::pay(remaining)?;
            if owner != Some(self.owner) || path != self.path {
                return Err(
                    "Selected FIRST diagnostic occurrence differs from caller binding".into(),
                );
            }
            let facts = grammar.inspect_source_prefix_with_work(path, remaining)?;
            Self::pay(remaining)?;
            if facts.owner() != self.owner || !std::ptr::eq(facts.expression(), expr) {
                return Err(
                    "Selected FIRST diagnostic expression differs from canonical source".into(),
                );
            }
            let record = self.capture(&facts, remaining)?;
            self.record.set(Some(record));
            self.captured.set(true);
            Ok(facts.into_summary())
        })();
        if let Err(error) = &result {
            // One fixed error record is observational cleanup, as in existing
            // phase observation. It allocates nothing, cannot fail, and never
            // replaces the actual FIRST/denial error or creates work credit.
            let mut record = SourcePrefixDiagnosticRecord::empty(*remaining);
            record.state = if error == "Grammar prediction budget exceeded" && *remaining == 0 {
                PrefixDiagnosticState::Denied
            } else {
                PrefixDiagnosticState::Unavailable
            };
            record.absence = PrefixDiagnosticAbsence::FirstFailure;
            record.error = PrefixDiagnosticText::copy(error);
            record.capture_unavailable = record.error.is_none();
            self.record.set(Some(record));
            self.captured.set(true);
        }
        result
    }
}

#[cfg(test)]
#[path = "follow_prefix_diagnostic_tests.rs"]
mod tests;
