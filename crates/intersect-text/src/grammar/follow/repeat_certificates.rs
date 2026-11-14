//! Temporary, analyzer-owned evidence for one nonconflicting source repeat.
//! Nothing in this module is deserialized or installed as package authority.
use super::{Tokens, charge};
use crate::grammar::Expr;
use crate::grammar::Grammar;
use crate::grammar::Kind;
use crate::grammar::first_sets::OccurrenceFacts;
use crate::grammar::overlaps;
use intersect_model::definitions::bundle::SourceId;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::grammar) struct GraphBinding {
    pub start: usize,
    pub body: usize,
    pub stop: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub(in crate::grammar) struct SourceRepeatCertificate {
    pub source: SourceId,
    pub declaration: Range<usize>,
    pub owner: String,
    pub path: String,
    pub occurrence: Range<usize>,
    pub operand: Range<usize>,
    pub callee: String,
    pub callee_source: SourceId,
    pub suffix_path: String,
    pub suffix: Range<usize>,
    pub literal: String,
    pub graph: Option<GraphBinding>,
}

// This proves ONLY the already-computed nonnullable occurrence lemma.
// It deliberately has no suffix/head/nonconflict fields or authority.
#[derive(Clone, Debug)]
pub(in crate::grammar) struct NonnullableOccurrence {
    pub source: SourceId,
    pub declaration: Range<usize>,
    pub owner: String,
    pub path: String,
    pub recovery_key: String,
    pub occurrence: Range<usize>,
    pub operand: Range<usize>,
    pub callee: String,
    pub callee_source: SourceId,
    pub graph: Option<GraphBinding>,
}

#[derive(Clone, Debug, Default)]
pub(in crate::grammar) struct RepeatCertificates {
    pub entries: BTreeMap<String, SourceRepeatCertificate>,
    pub nonnullable: BTreeMap<String, NonnullableOccurrence>,
    pub bound: usize,
    pub consumed: usize,
    pub binding_work: usize,
    pub consumption_work: usize,
}

pub(in crate::grammar) fn candidate(expr: &Expr) -> Option<&str> {
    if expr.cardinality != '*' || expr.first_set_predicated {
        return None;
    }
    let Kind::SyntacticPredicate(value) = &expr.kind else {
        return None;
    };
    if value.cardinality != '1' || value.first_set_predicated {
        return None;
    }
    match &value.kind {
        Kind::Call(name) => Some(name),
        _ => None,
    }
}

// New probes conservatively pay comparison height times key chunks before
// reading the map. These do not alter legacy certificate fees.
pub(in crate::grammar) fn lemma_lookup_work(len: usize, bytes: usize) -> Result<usize, String> {
    let levels = usize::BITS as usize - len.max(1).leading_zeros() as usize;
    levels
        .checked_mul(2)
        .and_then(|n| n.checked_mul(1 + bytes.div_ceil(64)))
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| "Nonnullable lookup work overflow".into())
}
pub(in crate::grammar) fn lemma_identity_work(a: &SourceId, b: &SourceId) -> Result<usize, String> {
    let bytes = a
        .uri
        .len()
        .checked_add(a.sha256.len())
        .and_then(|n| n.checked_add(b.uri.len()))
        .and_then(|n| n.checked_add(b.sha256.len()))
        .ok_or("Nonnullable identity work overflow")?;
    Ok(1 + bytes.div_ceil(64))
}

impl RepeatCertificates {
    pub(in crate::grammar) fn record_nonnullable(
        &mut self,
        expr: &Expr,
        grammar: &Grammar,
        facts: OccurrenceFacts,
        (owner, path): (Option<&str>, &str),
        budget: &mut usize,
        runtime: &mut super::super::RuntimeGuardedAdmission,
    ) -> Result<(), String> {
        let Some(callee) = candidate(expr) else {
            return Ok(());
        };
        if facts.body_nullable != Some(false) || facts.explicit_prefix_hidden_policy {
            return Ok(());
        }
        let Some(owner) = owner else {
            return Ok(());
        };
        for _ in 0..lemma_lookup_work(grammar.rules.len(), owner.len())? {
            charge(budget)?;
        }
        let Some(rule) = grammar.rules.get(owner) else {
            return Ok(());
        };
        if rule.hidden.is_some() {
            return Ok(());
        }
        for _ in 0..lemma_lookup_work(grammar.declaration_sources.len(), owner.len())? {
            charge(budget)?;
        }
        let Some(origin) = grammar.declaration_sources.get(owner) else {
            return Ok(());
        };
        for _ in 0..lemma_lookup_work(grammar.declaration_sources.len(), callee.len())? {
            charge(budget)?;
        }
        let Some(callee_origin) = grammar.declaration_sources.get(callee) else {
            return Ok(());
        };
        let Kind::SyntacticPredicate(value) = &expr.kind else {
            return Ok(());
        };
        let (Some(occurrence), Some(operand)) = (&expr.original_range, &value.original_range)
        else {
            return Ok(());
        };
        for range in [occurrence, operand] {
            charge(budget)?;
            if range.start >= range.end || range.start < origin.start || range.end > origin.end {
                return Ok(());
            }
        }
        // Each retained BTree record/node/header is conservatively fixed512;
        // every string clone/key and source identity payload is additional.
        // Both maps remain separate; this map cannot satisfy nonconflict.
        const RECORD: usize = 512;
        const _: () = assert!(std::mem::size_of::<NonnullableOccurrence>() <= RECORD);
        let key_len = path
            .len()
            .checked_add(7)
            .ok_or("Nonnullable path size overflow")?;
        let levels = usize::BITS as usize - self.nonnullable.len().max(1).leading_zeros() as usize;
        let probes = levels
            .checked_mul(2)
            .and_then(|n| n.checked_mul(1 + key_len.div_ceil(64)))
            .ok_or("Nonnullable retention lookup overflow")?;
        for _ in 0..probes {
            charge(budget)?;
        }
        // Reserve BEFORE constructing the key or cloning any source data.
        let recovery_len = path
            .len()
            .checked_add(9)
            .ok_or("Nonnullable recovery size overflow")?;
        // Graph binding borrows the pre-existing legacy key; no second
        // temporary key allocation or additional temporary allowance exists.
        let mut bytes = RECORD;
        for size in [
            key_len,
            recovery_len,
            path.len(),
            owner.len(),
            callee.len(),
            origin.source.uri.len(),
            origin.source.sha256.len(),
            callee_origin.source.uri.len(),
            callee_origin.source.sha256.len(),
        ] {
            bytes = bytes
                .checked_add(size)
                .ok_or("Nonnullable retention size overflow")?;
        }
        runtime.retain_nonnullable_lemma(bytes, budget)?;
        let key = format!("{path}/repeat");
        if self.nonnullable.contains_key(&key) {
            return Err("Duplicate nonnullable source occurrence".into());
        }
        // New insertion/search as well as duplicate probe are paid separately.
        for _ in 0..probes {
            charge(budget)?;
        }
        self.nonnullable.insert(
            key,
            NonnullableOccurrence {
                source: origin.source.clone(),
                declaration: origin.start..origin.end,
                owner: owner.into(),
                path: path.into(),
                recovery_key: format!("{path}/recovery"),
                occurrence: occurrence.clone(),
                operand: operand.clone(),
                callee: callee.into(),
                callee_source: callee_origin.source.clone(),
                graph: None,
            },
        );
        Ok(())
    }
    pub(super) fn record(
        &mut self,
        expr: &Expr,
        grammar: &Grammar,
        (head, facts, outer): (&BTreeSet<String>, OccurrenceFacts, &Tokens),
        (owner, path): (Option<&str>, &str),
        continuation: Option<(&[Expr], usize, &str)>,
        budget: &mut usize,
    ) -> Result<(), String> {
        let Some(callee) = candidate(expr) else {
            return Ok(());
        };
        if facts.body_nullable != Some(false)
            || facts.explicit_prefix_hidden_policy
            || head.is_empty()
        {
            return Ok(());
        }
        let Some((items, next, parent)) = continuation else {
            return Ok(());
        };
        charge(budget)?;
        let Some(suffix) = items.get(next) else {
            return Ok(());
        };
        let Kind::Literal(literal) = &suffix.kind else {
            return Ok(());
        };
        if suffix.cardinality != '1' || suffix.first_set_predicated {
            return Ok(());
        }
        let mut units = literal.encode_utf16();
        let Some(unit) = units.next() else {
            return Ok(());
        };
        if units.next().is_some() {
            return Ok(());
        }
        if outer.caller
            || outer.symbols.len() != 1
            || outer
                .symbols
                .iter()
                .next()
                .is_none_or(|symbol| symbol.strip_prefix("literal:") != Some(literal))
        {
            return Ok(());
        }
        // Pay for the bounded set query before it reads any symbol. Existing
        // overlap semantics, including INT/numeric collision, remain authority.
        for _ in 0..head.len() {
            charge(budget)?;
        }
        charge(budget)?;
        if overlaps(head, &outer.symbols) {
            return Ok(());
        }
        let mut symbols = head.iter();
        for _ in 0..head.len() {
            charge(budget)?;
            let symbol = symbols.next().ok_or("Missing source repeat FIRST symbol")?;
            if symbol.starts_with("literal:") {
                continue;
            }
            let Some(name) = symbol.strip_prefix("terminal:") else {
                return Ok(());
            };
            charge(budget)?;
            let Some(terminal) = grammar.terminals.get(name) else {
                return Ok(());
            };
            if terminal.fragment || terminal.body.nullable || terminal.body.first.is_empty() {
                return Ok(());
            }
            let mut ranges = terminal.body.first.iter();
            for _ in 0..terminal.body.first.len() {
                charge(budget)?;
                let (low, high) = ranges
                    .next()
                    .ok_or("Missing prepared terminal FIRST interval")?;
                if *low <= unit && unit <= *high {
                    return Ok(());
                }
            }
        }
        let Some(owner) = owner else {
            return Ok(());
        };
        charge(budget)?;
        let Some(rule) = grammar.rules.get(owner) else {
            return Ok(());
        };
        if rule.hidden.is_some() {
            return Ok(());
        }
        charge(budget)?;
        let Some(origin) = grammar.declaration_sources.get(owner) else {
            return Ok(());
        };
        charge(budget)?;
        let Some(callee_origin) = grammar.declaration_sources.get(callee) else {
            return Ok(());
        };
        let Kind::SyntacticPredicate(value) = &expr.kind else {
            return Ok(());
        };
        let (Some(occurrence), Some(operand), Some(suffix_range)) = (
            &expr.original_range,
            &value.original_range,
            &suffix.original_range,
        ) else {
            return Ok(());
        };
        for range in [occurrence, operand, suffix_range] {
            charge(budget)?;
            if range.start > range.end || range.start < origin.start || range.end > origin.end {
                return Ok(());
            }
        }
        charge(budget)?;
        let key = format!("{path}/repeat");
        if self.entries.contains_key(&key) {
            return Err("Duplicate source repeat certificate occurrence".into());
        }
        // One owned bounded record/key; this is not an allocator-byte bound.
        charge(budget)?;
        self.entries.insert(
            key,
            SourceRepeatCertificate {
                source: origin.source.clone(),
                declaration: origin.start..origin.end,
                owner: owner.into(),
                path: path.into(),
                occurrence: occurrence.clone(),
                operand: operand.clone(),
                callee: callee.into(),
                callee_source: callee_origin.source.clone(),
                suffix_path: format!("{parent}/s{next}"),
                suffix: suffix_range.clone(),
                literal: literal.clone(),
                graph: None,
            },
        );
        Ok(())
    }
}
