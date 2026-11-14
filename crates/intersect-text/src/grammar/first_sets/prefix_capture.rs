//! Explicit transient FIRST diagnostics. No result grants parser/admission authority.
use super::{OccurrenceFacts, Summary, analyze_shared};
use crate::grammar::Expr;
use crate::grammar::Grammar;
use crate::grammar::Rule;
use intersect_model::definitions::bundle::SourceSpan;
use std::collections::{BTreeMap, BTreeSet};

fn pay(remaining: &mut usize) -> Result<(), String> {
    if *remaining == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *remaining -= 1;
    Ok(())
}

// Portable logical copy preflight for this NEW route only. Existing FIRST
// allocations/error strings retain their documented atomic/S09 boundary. One
// unit per 256 UTF-8 payload bytes, rounded up, is not an allocator/RSS bound.
const COPY_CHUNK_BYTES: usize = 256;
const LOGICAL_FRAME_BYTES: usize = 128;
pub(in crate::grammar) fn reserve_copy_bytes(
    bytes: usize,
    remaining: &mut usize,
) -> Result<(), String> {
    let chunks = bytes / COPY_CHUNK_BYTES + usize::from(!bytes.is_multiple_of(COPY_CHUNK_BYTES));
    for _ in 0..chunks {
        pay(remaining)?;
    }
    Ok(())
}
pub(super) fn retain_source_frame<S: super::symbols::Symbols>(
    frames: &mut Vec<super::Frame<'_, S>>,
    remaining: &mut usize,
) -> Result<(), String> {
    // Always pay one logical new frame and the conservative live-stack copy,
    // independently of allocator capacity. Reserve TWO slots before inserting
    // the new return frame and its existing Visit; neither push can reallocate.
    // Existing subsequent FIRST stack operations remain the old S09 boundary.
    reserve_copy_bytes(LOGICAL_FRAME_BYTES, remaining)?;
    let copied = frames
        .len()
        .checked_mul(LOGICAL_FRAME_BYTES)
        .ok_or("Source prefix diagnostic frame size overflow")?;
    reserve_copy_bytes(copied, remaining)?;
    frames
        .try_reserve_exact(2)
        .map_err(|_| "Source prefix diagnostic frame retention failed".into())
}

/// A completed callee derivation in this exact source/grammar. Not a graph proof.
pub struct PrefixCallFact<'g> {
    name: &'g str,
    rule: &'g Rule,
    source: &'g SourceSpan,
    symbols: BTreeSet<String>,
    nullable: bool,
}
impl<'g> PrefixCallFact<'g> {
    pub fn name(&self) -> &'g str {
        self.name
    }
    pub fn source(&self) -> &'g SourceSpan {
        self.source
    }
    pub fn rule(&self) -> &'g Rule {
        self.rule
    }
    pub fn symbols(&self) -> &BTreeSet<String> {
        &self.symbols
    }
    pub fn nullable(&self) -> bool {
        self.nullable
    }
    /// Successful empty-FIRST/nullable source derivation. This is deliberately
    /// not a cached recursion_witness Boolean or permission to skip that worker.
    pub fn complete_empty_derivation(&self) -> bool {
        self.nullable && self.symbols.is_empty()
    }
}

pub(super) struct Capture<'g> {
    grammar: &'g Grammar,
    calls: BTreeMap<&'g str, PrefixCallFact<'g>>,
    peak_calls: usize,
}
impl<'g> Capture<'g> {
    fn new(grammar: &'g Grammar) -> Self {
        Self {
            grammar,
            calls: BTreeMap::new(),
            peak_calls: 0,
        }
    }
    pub(super) fn visit(&mut self, calls: usize) {
        self.peak_calls = self.peak_calls.max(calls);
    }
    pub(super) fn complete_call(
        &mut self,
        name: &str,
        rule: &'g Rule,
        summary: &Summary,
        remaining: &mut usize,
    ) -> Result<(), String> {
        pay(remaining)?;
        let (name, canonical) = self
            .grammar
            .rules
            .get_key_value(name)
            .ok_or("Captured FIRST callee is not a canonical grammar rule")?;
        if !std::ptr::eq(rule, canonical) {
            return Err("Captured FIRST callee belongs to another rule body".into());
        }
        pay(remaining)?;
        let source = self
            .grammar
            .declaration_sources
            .get(name)
            .ok_or("Captured FIRST callee lacks an exact source declaration")?;
        let range = rule
            .body
            .original_range
            .as_ref()
            .ok_or("Captured FIRST callee lacks original body boundaries")?;
        if range.start >= range.end || range.start < source.start || range.end > source.end {
            return Err("Captured FIRST callee escapes its source declaration".into());
        }
        pay(remaining)?;
        if let Some(previous) = self.calls.get(name.as_str()) {
            // Prepay every compared symbol before the ordered-set comparison.
            for _ in 0..previous.symbols.len().max(summary.0.len()) {
                pay(remaining)?;
            }
            if previous.nullable != summary.1 || previous.symbols != summary.0 {
                return Err("Captured FIRST derivations disagree for one immutable callee".into());
            }
            return Ok(());
        }
        for symbol in &summary.0 {
            pay(remaining)?; // Logical set node/header, before retaining it.
            reserve_copy_bytes(symbol.len(), remaining)?;
        }
        pay(remaining)?;
        self.calls.insert(
            name,
            PrefixCallFact {
                name,
                rule,
                source,
                symbols: summary.0.clone(),
                nullable: summary.1,
            },
        );
        Ok(())
    }
}

/// Same-worker retained-IR diagnostics; cannot be deserialized into executable
/// proof. The source metadata comes from this immutable Grammar. A caller must
/// still establish its fresh exact source world; this object grants none.
pub struct SourcePrefixInspection<'g> {
    grammar: &'g Grammar,
    owner: &'g str,
    expression: &'g Expr,
    source: &'g SourceSpan,
    path: String,
    symbols: BTreeSet<String>,
    nullable: bool,
    facts: OccurrenceFacts,
    owner_hidden_policy: bool,
    capture: Capture<'g>,
}
impl<'g> SourcePrefixInspection<'g> {
    pub(in crate::grammar) fn into_summary(self) -> (Summary, OccurrenceFacts) {
        ((self.symbols, self.nullable), self.facts)
    }
    pub fn owner(&self) -> &'g str {
        self.owner
    }
    pub fn source(&self) -> &'g SourceSpan {
        self.source
    }
    pub fn expression(&self) -> &'g Expr {
        self.expression
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn symbols(&self) -> &BTreeSet<String> {
        &self.symbols
    }
    pub fn nullable(&self) -> bool {
        self.nullable
    }
    pub fn body_nullable(&self) -> Option<bool> {
        self.facts.body_nullable
    }
    pub fn prefix_hidden_policy(&self) -> bool {
        self.facts.explicit_prefix_hidden_policy || self.owner_hidden_policy
    }
    pub fn maximum_call_depth(&self) -> usize {
        self.capture.peak_calls
    }
    pub fn calls(&self) -> impl Iterator<Item = &PrefixCallFact<'g>> {
        self.capture.calls.values()
    }
    /// Diagnostic arithmetic only. Existing depth checks still execute in the
    /// real classifier; this function grants no permission to omit them.
    pub fn combined_call_depth_with_work(
        &self,
        existing: usize,
        remaining: &mut usize,
    ) -> Result<usize, String> {
        pay(remaining)?;
        let depth = existing
            .checked_add(self.capture.peak_calls)
            .ok_or("Parser prediction call depth exceeded")?;
        if depth > 128 {
            return Err("Parser prediction call depth exceeded".into());
        }
        Ok(depth)
    }
    /// Conservative lexical comparison of two retained FIRST results. This
    /// reports source facts only: graph/return/stop/witness binding is absent.
    pub fn separation_with_work(
        &self,
        other: &Self,
        remaining: &mut usize,
    ) -> Result<PrefixSeparation, String> {
        pay(remaining)?;
        if !std::ptr::eq(self.grammar, other.grammar) {
            return Err("FIRST diagnostics belong to different immutable grammars".into());
        }
        if self.facts.body_nullable != Some(false)
            || other.facts.body_nullable != Some(false)
            || self.symbols.is_empty()
            || other.symbols.is_empty()
            || self.prefix_hidden_policy()
            || other.prefix_hidden_policy()
        {
            return Ok(PrefixSeparation::Unavailable);
        }
        for left in &self.symbols {
            for right in &other.symbols {
                pay(remaining)?;
                if left == right {
                    return Ok(PrefixSeparation::Overlap);
                }
                if let (Some(_), Some(_)) = (
                    left.strip_prefix("literal:"),
                    right.strip_prefix("literal:"),
                ) {
                    continue;
                }
                let intervals = |symbol: &str,
                                 remaining: &mut usize|
                 -> Result<Option<PrefixRanges<'_>>, String> {
                    if let Some(text) = symbol.strip_prefix("literal:") {
                        pay(remaining)?;
                        let Some(unit) = text.encode_utf16().next() else {
                            return Ok(None);
                        };
                        return Ok(Some(PrefixRanges {
                            literal: Some((unit, unit)),
                            terminal: &[],
                        }));
                    }
                    let Some(name) = symbol.strip_prefix("terminal:") else {
                        return Ok(None);
                    };
                    pay(remaining)?;
                    let Some(terminal) = self.grammar.terminals.get(name) else {
                        return Ok(None);
                    };
                    if terminal.fragment || terminal.body.nullable || terminal.body.first.is_empty()
                    {
                        return Ok(None);
                    }
                    // Keep the charged range reads, borrow the prepared ranges:
                    // no diagnostic Vec or second terminal evaluator is created.
                    for _ in 0..terminal.body.first.len() {
                        pay(remaining)?;
                    }
                    Ok(Some(PrefixRanges {
                        literal: None,
                        terminal: &terminal.body.first,
                    }))
                };
                let (Some(left), Some(right)) =
                    (intervals(left, remaining)?, intervals(right, remaining)?)
                else {
                    return Ok(PrefixSeparation::Unavailable);
                };
                for a in left.iter() {
                    for b in right.iter() {
                        pay(remaining)?;
                        if a.0 <= b.1 && b.0 <= a.1 {
                            return Ok(PrefixSeparation::Unavailable);
                        }
                    }
                }
            }
        }
        Ok(PrefixSeparation::DisjointFirstOnly)
    }
}

struct PrefixRanges<'a> {
    literal: Option<(u16, u16)>,
    terminal: &'a [(u16, u16)],
}
impl PrefixRanges<'_> {
    fn iter(&self) -> impl Iterator<Item = &(u16, u16)> {
        self.literal.iter().chain(self.terminal.iter())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixSeparation {
    DisjointFirstOnly,
    Overlap,
    Unavailable,
}

impl Grammar {
    /// Inspect one real source occurrence using the SAME FIRST worker. The
    /// supplied existing pool pays source binding, original FIRST visits and
    /// new diagnostic retention. No allowance is created or reset here.
    /// This never constructs a parser certificate or enables a profile.
    pub fn inspect_source_prefix_with_work(
        &self,
        path: &str,
        remaining: &mut usize,
    ) -> Result<SourcePrefixInspection<'_>, String> {
        // The supplied pool can be at most one standard per-pool grant. Since
        // 1e5cd75b the default analysis grant is REALISTIC, not the historical
        // 10k pool, so the bound follows the current default grant.
        if *remaining > crate::grammar::GrammarAnalysisWorkBudget::default().units() {
            return Err("Source prefix inspection requires the original bounded work pool".into());
        }
        let (owner, rule, expression) =
            super::super::nullable_repetition::prefix_source_occurrence_with_work(
                self, path, remaining,
            )?
            .ok_or("Source prefix inspection needs an explicit source-retaining profile")?;
        pay(remaining)?;
        let source = self
            .declaration_sources
            .get(owner)
            .ok_or("FIRST inspection owner lacks exact source identity")?;
        pay(remaining)?;
        let range = expression
            .original_range
            .as_ref()
            .ok_or("FIRST inspection lacks original expression boundaries")?;
        if range.start >= range.end || range.start < source.start || range.end > source.end {
            return Err("FIRST inspection occurrence escapes its original declaration".into());
        }
        let mut facts = OccurrenceFacts::default();
        let mut capture = Capture::new(self);
        let (symbols, nullable) = analyze_shared(
            expression,
            self,
            0,
            remaining,
            Some(&mut facts),
            Some(&mut capture),
        )?;
        let owner_hidden_policy = rule.hidden.is_some();
        pay(remaining)?;
        reserve_copy_bytes(path.len(), remaining)?;
        Ok(SourcePrefixInspection {
            grammar: self,
            owner,
            expression,
            source,
            path: path.into(),
            symbols,
            nullable,
            facts,
            owner_hidden_policy,
            capture,
        })
    }
}

#[cfg(test)]
#[path = "prefix_capture_tests.rs"]
mod tests;
