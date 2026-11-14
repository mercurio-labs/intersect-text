//! Handwritten NativeV82 lowering. The generated fragment DAG shares every
//! authored member once and tracks remaining members through subset states.
//! No permutation copies, domain algorithms or additional runtime interpreter.
use super::{Datatype, Expr, Grammar, Kind, Rule};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};
const PREFIX: &str = "@intersect.unordered.";
pub(super) fn generated(name: &str) -> bool {
    name.contains(PREFIX)
}
pub(super) fn namespace(name: &str) -> String {
    format!("{:x}", Sha256::digest(name.as_bytes()))
}
#[derive(Default)]
pub(super) struct ParserState {
    pub namespace: String,
    pub pending: Vec<Pending>,
}
pub(super) struct Pending {
    prefix: String,
    range: Range<usize>,
    pub members: Vec<Expr>,
}
fn expression(kind: Kind, range: &Range<usize>) -> Expr {
    Expr {
        original_range: Some(range.clone()),
        first_set_predicated: false,
        prediction_path: None,
        kind,
        cardinality: '1',
    }
}
impl Pending {
    pub fn new(namespace: &str, range: Range<usize>, members: Vec<Expr>) -> Self {
        Self {
            prefix: format!("{PREFIX}{namespace}.g{}", range.start),
            range,
            members,
        }
    }
    fn states(&self) -> Result<usize, String> {
        let shift = u32::try_from(self.members.len())
            .map_err(|_| "Unordered planning exceeds definition capacity")?;
        1usize
            .checked_shl(shift)
            .ok_or_else(|| "Unordered planning exceeds definition capacity".into())
    }
    fn state(&self, mask: usize) -> String {
        format!("{}.state.{mask}", self.prefix)
    }
    fn member(&self, index: usize) -> String {
        format!("{}.member.{index}", self.prefix)
    }
    pub fn entry(&self) -> Result<Expr, String> {
        Ok(expression(
            Kind::Call(
                self.state(
                    self.states()?
                        .checked_sub(1)
                        .ok_or("Missing unordered state")?,
                ),
            ),
            &self.range,
        ))
    }
}
fn fragment(parent: &Rule, body: Expr) -> Rule {
    Rule {
        hidden: None,
        enum_literals: BTreeMap::new(),
        inferred_return: false,
        override_declared: false,
        datatype: parent.datatype.map(|_| Datatype::EString),
        nominal_return: None,
        fragment: true,
        wildcard: parent.wildcard,
        class: parent.class.clone(),
        body,
    }
}
// Remove the empty path from a nullable member before choosing it. Empty
// members are skipped by the subset-state exit. Nested groups stay contiguous.
fn nullable(expr: &Expr, groups: &BTreeMap<String, (bool, String)>) -> bool {
    matches!(expr.cardinality, '?' | '*')
        || match &expr.kind {
            Kind::Sequence(items) => items.iter().all(|item| nullable(item, groups)),
            Kind::Alternative(items) => items.iter().any(|item| nullable(item, groups)),
            Kind::Call(name) => groups.get(name).is_some_and(|(nullable, _)| *nullable),
            Kind::Assign { value, .. } | Kind::BooleanAssign { value, .. } => {
                nullable(value, groups)
            }
            _ => false,
        }
}
fn copied(expr: &Expr, budget: &mut usize) -> Result<Expr, String> {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        *budget = budget
            .checked_sub(1)
            .ok_or("Unordered expression budget exceeded")?;
        match &expr.kind {
            Kind::Sequence(items) | Kind::Alternative(items) => pending.extend(items),
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => pending.push(value),
            _ => {}
        }
    }
    Ok(expr.clone())
}
fn nonempty(
    expr: &Expr,
    groups: &BTreeMap<String, (bool, String)>,
    budget: &mut usize,
) -> Result<Expr, String> {
    let mut result = copied(expr, budget)?;
    result.cardinality = match result.cardinality {
        '?' => '1',
        '*' => '+',
        other => other,
    };
    result.kind = match &expr.kind {
        Kind::Call(name) if groups.get(name).is_some_and(|(nullable, _)| *nullable) => Kind::Call(
            groups
                .get(name)
                .ok_or("Missing nested unordered group")?
                .1
                .clone(),
        ),
        Kind::Sequence(items) if items.first().is_some_and(|item| nullable(item, groups)) => {
            let mut branches = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let mut sequence = vec![nonempty(item, groups, budget)?];
                for tail in &items[index + 1..] {
                    sequence.push(copied(tail, budget)?);
                }
                branches.push(expression(
                    Kind::Sequence(sequence),
                    expr.original_range
                        .as_ref()
                        .ok_or("Missing authored unordered sequence range")?,
                ));
                if !nullable(item, groups) {
                    break;
                }
            }
            Kind::Alternative(branches)
        }
        Kind::Alternative(items) if items.iter().any(|item| nullable(item, groups)) => {
            Kind::Alternative(
                items
                    .iter()
                    .map(|item| nonempty(item, groups, budget))
                    .collect::<Result<_, _>>()?,
            )
        }
        Kind::Assign {
            feature,
            many,
            value,
        } if nullable(value, groups) => Kind::Assign {
            feature: feature.clone(),
            many: *many,
            value: Box::new(nonempty(value, groups, budget)?),
        },
        Kind::BooleanAssign { feature, value } if nullable(value, groups) => Kind::BooleanAssign {
            feature: feature.clone(),
            value: Box::new(nonempty(value, groups, budget)?),
        },
        other => other.clone(),
    };
    Ok(result)
}
pub(super) fn lower(
    pending: Vec<Pending>,
    parent: &Rule,
    remaining: usize,
) -> Result<Vec<(String, Rule)>, String> {
    let mut needed = 0usize;
    for group in &pending {
        needed = needed
            .checked_add(group.states()?)
            .and_then(|n| n.checked_add(group.members.len() + 1))
            .ok_or("Unordered generated declaration budget overflow")?;
        if needed > remaining {
            return Err("Unordered generated declaration budget exceeded".into());
        }
    }
    let mut rules = Vec::new();
    rules
        .try_reserve(needed)
        .map_err(|_| "Unordered rule allocation failed")?;
    let mut groups = BTreeMap::new();
    let mut expression_budget = remaining.saturating_mul(64);
    for mut group in pending {
        let optional: Vec<_> = group
            .members
            .iter()
            .map(|member| nullable(member, &groups))
            .collect();
        let full = group
            .states()?
            .checked_sub(1)
            .ok_or("Missing unordered state")?;
        let required = format!("{}.required", group.prefix);
        for mask in 0..group.states()? {
            let mut branches = Vec::new();
            for index in 0..group.members.len() {
                let bit = 1usize
                    .checked_shl(
                        u32::try_from(index).map_err(|_| "Unordered member capacity exceeded")?,
                    )
                    .ok_or("Unordered member capacity exceeded")?;
                if mask & bit == 0 {
                    continue;
                }
                branches.push(expression(
                    Kind::Sequence(vec![
                        expression(Kind::Call(group.member(index)), &group.range),
                        expression(Kind::Call(group.state(mask ^ bit)), &group.range),
                    ]),
                    &group.range,
                ));
            }
            if mask == full {
                let body = expression(Kind::Alternative(branches.clone()), &group.range);
                rules.push((
                    required.clone(),
                    fragment(parent, copied(&body, &mut expression_budget)?),
                ));
            }
            if mask != 0
                && optional
                    .iter()
                    .enumerate()
                    .all(|(index, optional)| mask & (1usize << index) == 0 || *optional)
            {
                branches.push(expression(Kind::Sequence(Vec::new()), &group.range));
            }
            let body = match branches.len() {
                0 => expression(Kind::Sequence(Vec::new()), &group.range),
                1 => branches.pop().ok_or("Missing unordered branch")?,
                _ => expression(Kind::Alternative(branches), &group.range),
            };
            rules.push((group.state(mask), fragment(parent, body)));
        }
        for (index, mut member) in std::mem::take(&mut group.members).into_iter().enumerate() {
            // Choosing an optional/repeated member must consume one occurrence;
            // omission belongs to the remaining-state exit, not another branch.
            member = nonempty(&member, &groups, &mut expression_budget)?;
            rules.push((group.member(index), fragment(parent, member)));
        }
        groups.insert(group.state(full), (optional.iter().all(|v| *v), required));
    }
    // Prune unused generated states: shared prediction requires actual caller
    // FOLLOW context, including for a nullable nested group's ordinary entry.
    let lookup: BTreeMap<_, _> = rules
        .iter()
        .map(|(name, rule)| (name.as_str(), rule))
        .collect();
    let mut called = BTreeSet::new();
    let mut expressions = vec![&parent.body];
    while let Some(expr) = expressions.pop() {
        match &expr.kind {
            Kind::Call(name) => {
                if called.insert(name.clone())
                    && let Some(rule) = lookup.get(name.as_str())
                {
                    expressions.push(&rule.body);
                }
            }
            Kind::Sequence(items) | Kind::Alternative(items) => expressions.extend(items),
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => expressions.push(value),
            _ => {}
        }
    }
    rules.retain(|(name, _)| called.contains(name));
    Ok(rules)
}

/// Upstream excludes actions and unassigned non-fragment object calls inside
/// unordered members. Syntax-only generated transitions remain explicit here.
pub(super) fn check(grammar: &Grammar, mut work: usize) -> Result<(), String> {
    fn debit(work: &mut usize) -> Result<(), String> {
        *work = work
            .checked_sub(1)
            .ok_or("Unordered validation work budget exceeded")?;
        Ok(())
    }
    fn contains_group(
        expression: &Expr,
        grammar: &Grammar,
        work: &mut usize,
    ) -> Result<bool, String> {
        let mut expressions = vec![expression];
        let mut visited = BTreeSet::new();
        while let Some(expression) = expressions.pop() {
            debit(work)?;
            match &expression.kind {
                Kind::Call(name) => {
                    if generated(name) {
                        return Ok(true);
                    }
                    if visited.insert(name)
                        && let Some(rule) = grammar.rules.get(name)
                    {
                        expressions.push(&rule.body);
                    }
                }
                Kind::Assign { value, .. }
                | Kind::BooleanAssign { value, .. }
                | Kind::SyntacticPredicate(value) => expressions.push(value),
                Kind::Sequence(items) | Kind::Alternative(items) => expressions.extend(items),
                _ => {}
            }
        }
        Ok(false)
    }
    for (name, rule) in &grammar.rules {
        let mut pending = vec![(&rule.body, false)];
        while let Some((expression, assigned)) = pending.pop() {
            debit(&mut work)?;
            if (expression.first_set_predicated
                || matches!(expression.kind, Kind::SyntacticPredicate(_)))
                && contains_group(expression, grammar, &mut work)?
            {
                return Err("Unordered groups cannot occur within syntax predicates".into());
            }
            match &expression.kind {
                Kind::Action(_) | Kind::Capture { .. } if generated(name) => {
                    return Err("Actions cannot occur inside unordered groups".into());
                }
                Kind::Call(target) if generated(name) && !assigned => {
                    if grammar
                        .rules
                        .get(target)
                        .is_some_and(|rule| !rule.fragment && rule.datatype.is_none())
                    {
                        return Err(
                            "Unassigned object calls cannot occur inside unordered groups".into(),
                        );
                    }
                }
                Kind::Assign { value, .. } | Kind::BooleanAssign { value, .. } => {
                    pending.push((value, true))
                }
                Kind::SyntacticPredicate(value) => pending.push((value, assigned)),
                Kind::Sequence(items) | Kind::Alternative(items) => {
                    pending.extend(items.iter().map(|item| (item, assigned)))
                }
                _ => {}
            }
        }
    }
    Ok(())
}
