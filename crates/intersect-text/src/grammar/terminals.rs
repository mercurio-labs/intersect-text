//! Native terminal programs. Recognition follows Mercurio's committed FIRST-based
//! matcher contract (MIT); provenance and remaining arbitration are documented in
//! docs/xtext-day07-design.md. No language-specific names or generated Pilot tables.
use super::{Grammar, Kind, Parser};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Terminal {
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub override_declared: bool,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub fragment: bool,
    pub datatype: super::Datatype,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_return: Option<intersect_model::definitions::resources::DeclarationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enumeration: Option<String>,
    pub identity: String,
    pub start: usize,
    pub end: usize,
    pub body: Expression,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expression {
    pub kind: Operation,
    pub cardinality: char,
    pub first: Vec<(u16, u16)>,
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<super::LexerPrediction>,
    /// Source-regenerated greedy enter/exit decision, independently scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardinality_decision: Option<super::LexerPrediction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    Literal(Vec<u16>),
    Range(u16, u16),
    Wildcard,
    Eof,
    NotCharacters(Vec<(u16, u16)>),
    /// Authored operand retained after resolution. Older profiles prove a
    /// one-unit domain; V62 separately projects eligible source lexer sets.
    /// Neither path substitutes FIRST for the resolved operand.
    Negated {
        operand: Box<Expression>,
        excluded: Vec<(u16, u16)>,
    },
    Until(Vec<u16>),
    /// Original source operand and source-regenerated rule-local loop decision.
    /// The delimiter language is not flattened into a string or lookup table.
    UntilOperand {
        operand: Box<Expression>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision: Option<super::LexerPrediction>,
    },
    Call {
        name: String,
        body: Option<Box<Expression>>,
    },
    Sequence(Vec<Expression>),
    Alternative(Vec<Expression>),
}
fn expression(kind: Operation) -> Expression {
    Expression {
        kind,
        cardinality: '1',
        first: Vec::new(),
        nullable: false,
        decision: None,
        cardinality_decision: None,
    }
}
fn parse_expression(p: &mut Parser, profile: super::Profile) -> Result<Expression, String> {
    p.depth += 1;
    if p.depth > 64 {
        return Err("Grammar nesting budget exceeded".into());
    }
    let mut choices = vec![parse_sequence(p, profile)?];
    while p.eat("|") {
        choices.push(parse_sequence(p, profile)?);
    }
    p.depth -= 1;
    Ok(if choices.len() == 1 {
        choices.remove(0)
    } else {
        expression(Operation::Alternative(choices))
    })
}
fn parse_sequence(p: &mut Parser, profile: super::Profile) -> Result<Expression, String> {
    let mut items = Vec::new();
    while p.pos < p.tokens.len() && !p.peek("|") && !p.peek(")") && !p.peek(";") {
        let mut item = parse_element(p, profile)?;
        for cardinality in ['?', '*', '+'] {
            if p.eat(&cardinality.to_string()) {
                item.cardinality = cardinality;
                break;
            }
        }
        items.push(item);
    }
    if items.is_empty() {
        return Err("Empty terminal branch is unsupported".into());
    }
    Ok(if items.len() == 1 {
        items.remove(0)
    } else {
        expression(Operation::Sequence(items))
    })
}
fn parse_element(p: &mut Parser, profile: super::Profile) -> Result<Expression, String> {
    let token = p.take()?;
    let escaped = !token.quoted && token.text == "^";
    let token = p.identifier_token(token)?;
    let item = if token.quoted {
        let units: Vec<_> = token.text.encode_utf16().collect();
        if p.peek(".")
            && p.tokens
                .get(p.pos + 1)
                .is_some_and(|t| !t.quoted && t.text == ".")
        {
            p.require(".")?;
            p.require(".")?;
            let right = p.take()?;
            let right: Vec<_> = if right.quoted {
                right.text.encode_utf16().collect()
            } else {
                return Err("Terminal range requires quoted characters".into());
            };
            if units.len() != 1 || right.len() != 1 || units[0] > right[0] {
                return Err("Invalid UTF-16 terminal character range".into());
            }
            expression(Operation::Range(units[0], right[0]))
        } else {
            expression(Operation::Literal(units))
        }
    } else if token.text == "!" && p.hidden.is_some() && profile.has_terminal_operand_negation() {
        p.depth += 1;
        if p.depth > 64 {
            return Err("Grammar nesting budget exceeded".into());
        }
        let operand = parse_element(p, profile)?;
        p.depth -= 1;
        expression(Operation::Negated {
            operand: Box::new(operand),
            excluded: Vec::new(),
        })
    } else if token.text == "!" && p.hidden.is_some() {
        let grouped = p.eat("(");
        let mut excluded = Vec::new();
        loop {
            let value = p.take()?;
            let units: Vec<_> = value.text.encode_utf16().collect();
            if !value.quoted || units.len() != 1 {
                return Err("Terminal negation requires single UTF-16 characters".into());
            }
            excluded.push((units[0], units[0]));
            if !grouped || !p.eat("|") {
                break;
            }
        }
        if grouped {
            p.require(")")?;
        }
        expression(Operation::NotCharacters(merged(excluded)))
    } else if token.text == "-" && p.hidden.is_some() {
        p.require(">")?;
        if profile.has_until_operands() {
            p.depth += 1;
            if p.depth > 64 {
                return Err("Grammar nesting budget exceeded".into());
            }
            let operand = parse_element(p, profile)?;
            p.depth -= 1;
            return Ok(expression(Operation::UntilOperand {
                operand: Box::new(operand),
                decision: None,
            }));
        }
        let value = p.take()?;
        if !value.quoted || value.text.is_empty() {
            return Err("Terminal until requires a nonempty literal delimiter".into());
        }
        expression(Operation::Until(value.text.encode_utf16().collect()))
    } else if token.text == "." && p.hidden.is_some() {
        expression(Operation::Wildcard)
    } else if !escaped && token.text == "EOF" && p.hidden.is_some() {
        expression(Operation::Eof)
    } else if token.text == "(" {
        let inner = parse_expression(p, profile)?;
        p.require(")")?;
        expression(Operation::Sequence(vec![inner]))
    } else if p.hidden.is_some()
        && token
            .text
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
    {
        expression(Operation::Call {
            name: p.rule_name_tail(token.text)?,
            body: None,
        })
    } else {
        return Err(format!("Unsupported terminal construct '{}'", token.text));
    };
    Ok(item)
}
fn merged(mut ranges: Vec<(u16, u16)>) -> Vec<(u16, u16)> {
    ranges.sort_unstable();
    let mut result: Vec<(u16, u16)> = Vec::new();
    for (low, high) in ranges {
        if let Some(last) = result.last_mut()
            && u32::from(low) <= u32::from(last.1) + 1
        {
            last.1 = last.1.max(high);
            continue;
        }
        result.push((low, high));
    }
    result
}
/// Complement a normalized set in the UTF-16 code-unit domain. EOF is separate.
fn complement(excluded: &[(u16, u16)]) -> Vec<(u16, u16)> {
    let mut result = Vec::new();
    let mut cursor = 0u32;
    for &(low, high) in excluded {
        if cursor < u32::from(low) {
            result.push((cursor as u16, low - 1));
        }
        cursor = u32::from(high) + 1;
    }
    if cursor <= u32::from(u16::MAX) {
        result.push((cursor as u16, u16::MAX));
    }
    result
}
/// Prove the entire resolved operand language consumes exactly one code unit.
/// FIRST alone is insufficient: e.g. 'ab' has FIRST('a') but width two.
fn unit_domain(expr: &Expression, budget: &mut usize) -> Result<Vec<(u16, u16)>, String> {
    if *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    if expr.cardinality != '1' {
        return Err("Terminal negation operand must match exactly one UTF-16 unit; cardinalities are unsupported".into());
    }
    match &expr.kind {
        Operation::Literal(text) if text.len() == 1 => Ok(vec![(text[0], text[0])]),
        Operation::Range(low, high) => Ok(vec![(*low, *high)]),
        Operation::Wildcard => Ok(vec![(0, u16::MAX)]),
        Operation::NotCharacters(excluded) | Operation::Negated { excluded, .. } => {
            Ok(complement(excluded))
        }
        Operation::Call { name, body } => unit_domain(
            body.as_ref().ok_or_else(|| format!("Unresolved terminal call {name}"))?,
            budget,
        ),
        Operation::Alternative(items) => {
            let mut domain = Vec::new();
            for item in items {
                domain.extend(unit_domain(item, budget)?);
            }
            Ok(merged(domain))
        }
        Operation::Sequence(items) if items.len() == 1 => unit_domain(&items[0], budget),
        Operation::Eof => Err("Terminal negation cannot contain EOF".into()),
        Operation::Literal(_) => Err("Terminal negation operand must match exactly one UTF-16 unit; empty and multi-unit literals are unsupported".into()),
        Operation::Until(_) | Operation::UntilOperand { .. } | Operation::Sequence(_) => Err("Terminal negation operand is not a proved one-UTF-16-unit domain".into()),
    }
}
/// A source lexer-set projection is distinct from a one-unit language proof.
/// Pinned clean generated lexers ignore multi-unit keywords in a negated set.
/// Keep source structure and qualification boundaries explicit rather than
/// interpreting such a keyword as its first character.
struct NegationProjection {
    excluded: Vec<(u16, u16)>,
    // Keep source provenance even when an inner negation becomes a unit set.
    multi_keyword: bool,
    // Only an unwrapped literal or literal-only fragment call chain qualifies
    // a newly multi-unit callee body. A mixed fragment body is not an inline
    // caller alternative: pinned ANTLR reports a generator error for that form.
    literal_chain: bool,
}
impl NegationProjection {
    fn unit(excluded: Vec<(u16, u16)>) -> Self {
        Self {
            excluded,
            multi_keyword: false,
            literal_chain: false,
        }
    }
}
fn projected_negation_domain(
    expr: &Expression,
    budget: &mut usize,
    terminals: &BTreeMap<String, Terminal>,
) -> Result<NegationProjection, String> {
    if *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    if expr.cardinality != '1' {
        return Err("Terminal negation set projection requires cardinality one".into());
    }
    match &expr.kind {
        Operation::Literal(units) if !units.is_empty() => Ok(NegationProjection {
            excluded: if units.len() == 1 {
                vec![(units[0], units[0])]
            } else {
                Vec::new()
            },
            multi_keyword: units.len() > 1,
            literal_chain: true,
        }),
        Operation::Range(low, high) => Ok(NegationProjection::unit(vec![(*low, *high)])),
        Operation::Wildcard => Ok(NegationProjection::unit(vec![(0, u16::MAX)])),
        Operation::NotCharacters(excluded) => {
            Ok(NegationProjection::unit(complement(excluded)))
        }
        Operation::Negated { operand, .. } => {
            let inner = projected_negation_domain(operand, budget, terminals)?;
            Ok(NegationProjection {
                excluded: complement(&inner.excluded),
                multi_keyword: inner.multi_keyword,
                literal_chain: false,
            })
        }
        Operation::Sequence(items) if items.len() == 1 => {
            let mut result = projected_negation_domain(&items[0], budget, terminals)?;
            // Operand grouping is transparent. New grouped callee bodies need
            // their own clean full-source proof before becoming call chains.
            result.literal_chain = false;
            Ok(result)
        }
        Operation::Alternative(items) if !items.is_empty() => {
            let mut result = NegationProjection::unit(Vec::new());
            for item in items {
                let child = projected_negation_domain(item, budget, terminals)?;
                result.excluded.extend(child.excluded);
                result.multi_keyword |= child.multi_keyword;
            }
            result.excluded = merged(result.excluded);
            Ok(result)
        }
        Operation::Call { name, body } => {
            let body = body.as_ref().ok_or_else(|| format!("Unresolved terminal call {name}"))?;
            let result = projected_negation_domain(body, budget, terminals)?;
            if result.multi_keyword {
                let target = terminals.get(name).ok_or_else(|| format!("Unknown terminal call {name}"))?;
                if !target.fragment || !result.literal_chain {
                    return Err("Multi-unit terminal negation calls require literal-only fragment chains; mixed and grouped callee bodies remain unsupported".into());
                }
            }
            // Resolution already selected the actual effective or explicit-
            // parent declaration. Keep its Call/body and Terminal identity;
            // owner-name equality is not a source-scoping proof.
            Ok(result)
        }
        Operation::Eof => Err("Terminal negation cannot contain EOF".into()),
        Operation::Sequence(_) => Err("General multi-element terminal negation sequences remain unsupported; pinned ANTLR recovery is not a lexer-set proof".into()),
        Operation::Literal(_) | Operation::Alternative(_) | Operation::Until(_) | Operation::UntilOperand { .. } => Err("Terminal negation operand is not an eligible nonempty lexer-set program".into()),
    }
}
fn overlap(left: &[(u16, u16)], right: &[(u16, u16)]) -> bool {
    left.iter()
        .any(|(a, b)| right.iter().any(|(c, d)| a <= d && c <= b))
}
fn analyze(
    expr: &mut Expression,
    budget: &mut usize,
    profile: super::Profile,
    terminals: &BTreeMap<String, Terminal>,
) -> Result<(), String> {
    if *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    let (first, nullable) = match &mut expr.kind {
        Operation::Literal(text) => (
            text.first().map(|c| vec![(*c, *c)]).unwrap_or_default(),
            text.is_empty(),
        ),
        Operation::Range(low, high) => (vec![(*low, *high)], false),
        Operation::Wildcard | Operation::Eof if !profile.has_terminal_decisions() => {
            return Err("Terminal wildcard and EOF require NativeV13".into());
        }
        Operation::Wildcard => (vec![(0, u16::MAX)], false),
        Operation::Eof => (Vec::new(), true),
        Operation::NotCharacters(excluded) => {
            let mut first = Vec::new();
            let mut cursor = 0u32;
            for &(low, high) in excluded.iter() {
                if cursor < u32::from(low) {
                    first.push((cursor as u16, low - 1));
                }
                cursor = u32::from(high) + 1;
            }
            if cursor <= u32::from(u16::MAX) {
                first.push((cursor as u16, u16::MAX));
            }
            (first, false)
        }
        Operation::Negated { operand, excluded } => {
            if !profile.has_terminal_operand_negation() {
                return Err("Resolved terminal-negation operands require NativeV58".into());
            }
            analyze(operand, budget, profile, terminals)?;
            // Pinned full generated Xtext languages lower a direct multi-unit
            // keyword under negation to an empty exclusion set. This is not
            // the one-unit operand proof. Preserve that path independently;
            // V62's explicitly eligible groups/calls use a separate projector.
            *excluded = if profile.has_multi_unit_literal_negation()
                && operand.cardinality == '1'
                && matches!(&operand.kind, Operation::Literal(units) if units.len() > 1)
            {
                // Preserve the previously supported direct-literal path.
                Vec::new()
            } else if profile.has_projected_terminal_negation() {
                projected_negation_domain(operand, budget, terminals)?.excluded
            } else {
                unit_domain(operand, budget)?
            };
            let first = complement(excluded);
            if first.is_empty() {
                return Err("Terminal negation excludes every UTF-16 unit".into());
            }
            (first, false)
        }
        Operation::Until(_) => (vec![(0, u16::MAX)], false),
        Operation::UntilOperand { operand, .. } => {
            if !profile.has_until_operands() {
                return Err("Terminal Until operands require NativeV60".into());
            }
            analyze(operand, budget, profile, terminals)?;
            finite_until_operand(operand, budget)?;
            (vec![(0, u16::MAX)], false)
        }
        Operation::Call { name, body } => {
            let body = body
                .as_mut()
                .ok_or_else(|| format!("Unresolved terminal call {name}"))?;
            analyze(body, budget, profile, terminals)?;
            (body.first.clone(), body.nullable)
        }
        Operation::Alternative(items) => {
            let mut first = Vec::new();
            let mut nullable = false;
            let mut needs_decision = false;
            for item in items.iter_mut() {
                analyze(item, budget, profile, terminals)?;
                needs_decision |= item.nullable || overlap(&first, &item.first);
                nullable |= item.nullable;
                if !profile.has_terminal_decisions()
                    && (item.nullable || overlap(&first, &item.first))
                {
                    return Err("Overlapping or nullable terminal alternatives require unsupported prediction".into());
                }
                first.extend_from_slice(&item.first);
            }
            if needs_decision {
                expr.decision = Some(super::LexerPrediction::terminal_choices(items)?);
            }
            (merged(first), nullable)
        }
        Operation::Sequence(items) => {
            for item in items.iter_mut() {
                analyze(item, budget, profile, terminals)?;
            }
            for (index, item) in items.iter().enumerate() {
                if !profile.has_terminal_cardinality_decisions()
                    && (item.nullable || matches!(item.cardinality, '*' | '+'))
                {
                    for following in &items[index + 1..] {
                        if overlap(&item.first, &following.first) {
                            return Err("Terminal repetition/optional follow conflict requires unsupported prediction".into());
                        }
                        if !following.nullable {
                            break;
                        }
                    }
                }
            }
            let mut first = Vec::new();
            let mut nullable = true;
            for item in items {
                first.extend_from_slice(&item.first);
                if !item.nullable {
                    nullable = false;
                    break;
                }
            }
            (merged(first), nullable)
        }
    };
    if profile.has_until_operands()
        && !profile.has_terminal_cardinality_decisions()
        && expr.cardinality != '1'
        && contains_until(expr)
    {
        return Err(
            "Terminal Until execution repetitions and optionals are unsupported in NativeV60"
                .into(),
        );
    }
    if nullable && matches!(expr.cardinality, '*' | '+') {
        return Err("Nullable terminal repetitions are unsupported".into());
    }
    expr.first = first;
    expr.nullable = nullable || matches!(expr.cardinality, '?' | '*');
    Ok(())
}
/// The resolved tree already has shared expansion and depth ceilings. Inspect
/// the whole wrapper, not just an Until node: cardinality may belong to a group
/// or a fragment call whose subtree contains the loop.
fn contains_until(expr: &Expression) -> bool {
    match &expr.kind {
        Operation::UntilOperand { .. } => true,
        Operation::Sequence(items) | Operation::Alternative(items) => {
            items.iter().any(contains_until)
        }
        Operation::Call {
            body: Some(body), ..
        }
        | Operation::Negated { operand: body, .. } => contains_until(body),
        _ => false,
    }
}
/// Bounded initial Until domain: finite nonempty literal programs, preserving
/// grouping and call boundaries. Nullable/repeated/EOF/nested Until and other
/// operand kinds remain explicit rather than being approximated from FIRST.
fn finite_until_operand(expr: &Expression, budget: &mut usize) -> Result<(), String> {
    if *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    if expr.cardinality != '1' || expr.nullable {
        return Err(
            "Terminal Until operand requires a proved nonempty finite literal program".into(),
        );
    }
    match &expr.kind {
        Operation::Literal(units) if !units.is_empty() => Ok(()),
        Operation::Sequence(items) | Operation::Alternative(items) if !items.is_empty() => {
            for item in items {
                finite_until_operand(item, budget)?;
            }
            Ok(())
        }
        Operation::Call {
            body: Some(body), ..
        } => finite_until_operand(body, budget),
        _ => Err("Terminal Until operand requires a proved nonempty finite literal program".into()),
    }
}
fn has_decisions(expr: &Expression, profile: super::Profile) -> bool {
    (profile.has_terminal_cardinality_decisions() && expr.cardinality != '1')
        || expr.decision.is_some()
        || match &expr.kind {
            Operation::UntilOperand { .. } => true,
            Operation::Sequence(items) | Operation::Alternative(items) => {
                items.iter().any(|item| has_decisions(item, profile))
            }
            Operation::Call {
                body: Some(body), ..
            } => has_decisions(body, profile),
            _ => false,
        }
}
fn contextual_decisions(
    expr: &mut Expression,
    mut tail: Vec<Expression>,
    budget: &mut usize,
    profile: super::Profile,
) -> Result<(), String> {
    if !has_decisions(expr, profile) {
        return Ok(());
    }
    if *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    // Keep the original body and containing-rule tail. The recognizer commits
    // after this greedy enter/exit prediction; it never retries a shorter body.
    let mut once = expr.clone();
    once.cardinality = '1';
    once.cardinality_decision = None;
    let repeated = if matches!(expr.cardinality, '*' | '+') {
        let mut repeated = once.clone();
        repeated.cardinality = '*';
        Some(repeated)
    } else {
        None
    };
    if profile.has_terminal_cardinality_decisions() && expr.cardinality != '1' {
        let mut entering = vec![once];
        if let Some(repeated) = &repeated {
            entering.push(repeated.clone());
        }
        entering.extend(tail.iter().cloned());
        expr.cardinality_decision = Some(super::LexerPrediction::terminal_choices(&[
            expression(Operation::Sequence(entering)),
            expression(Operation::Sequence(tail.clone())),
        ])?);
    }
    if let Some(repeated) = repeated {
        tail.insert(0, repeated);
    }
    match &mut expr.kind {
        Operation::Alternative(items) => {
            if expr.decision.is_some() {
                let choices: Vec<_> = items
                    .iter()
                    .map(|item| {
                        let mut sequence = vec![item.clone()];
                        sequence.extend(tail.iter().cloned());
                        expression(Operation::Sequence(sequence))
                    })
                    .collect();
                expr.decision = Some(super::LexerPrediction::terminal_choices(&choices)?);
            }
            for item in items {
                contextual_decisions(item, tail.clone(), budget, profile)?;
            }
        }
        Operation::Sequence(items) => {
            // Traverse left to right so the suffix copies have no contextual
            // decisions yet. The prediction compiler consumes only their IR.
            for index in 0..items.len() {
                if !has_decisions(&items[index], profile) {
                    continue;
                }
                let mut following = items[index + 1..].to_vec();
                following.extend(tail.iter().cloned());
                contextual_decisions(&mut items[index], following, budget, profile)?;
            }
        }
        Operation::Call {
            body: Some(body), ..
        } => {
            // A terminal fragment's own decisions end at that rule's EOT.
            // Token arbitration still expands calls in its independent NFA.
            let following = if profile.has_until_operands() {
                Vec::new()
            } else {
                tail
            };
            contextual_decisions(body, following, budget, profile)?;
        }
        Operation::UntilOperand { operand, decision } => {
            let mut repeated = expression(Operation::Wildcard);
            repeated.cardinality = '*';
            let mut continuing = vec![
                expression(Operation::Wildcard),
                repeated,
                (**operand).clone(),
            ];
            continuing.extend(tail.iter().cloned());
            let mut exiting = vec![(**operand).clone()];
            exiting.extend(tail.iter().cloned());
            *decision = Some(super::LexerPrediction::terminal_choices(&[
                expression(Operation::Sequence(continuing)),
                expression(Operation::Sequence(exiting)),
            ])?);
            contextual_decisions(operand, tail, budget, profile)?;
        }
        _ => {}
    }
    Ok(())
}
pub(super) fn parse(
    p: &mut Parser,
    identity: String,
    start: usize,
    fragment: bool,
    profile: super::Profile,
) -> Result<Terminal, String> {
    let body = parse_expression(p, profile)?;
    p.require(";")?;
    let end = p
        .tokens
        .get(p.pos.saturating_sub(1))
        .map_or(start, |token| token.end);
    Ok(Terminal {
        override_declared: false,
        fragment,
        datatype: super::Datatype::EString,
        nominal_return: None,
        enumeration: None,
        identity,
        start,
        end,
        body,
    })
}
/// Resolve calls with a shared expansion ceiling. Retain each call name and each
/// declaration identity; never flatten source documents or introduce runtime I/O.
pub(super) fn prepare(
    terminals: &mut BTreeMap<String, Terminal>,
    profile: super::Profile,
) -> Result<(), super::AnalysisError> {
    fn resolve(
        expr: &mut Expression,
        sources: &BTreeMap<String, Terminal>,
        stack: &mut Vec<String>,
        budget: &mut usize,
        depth: usize,
    ) -> Result<(), String> {
        if *budget == 0 {
            return Err("Grammar prediction budget exceeded".into());
        }
        *budget -= 1;
        if depth > 64 {
            return Err("Terminal call nesting budget exceeded".into());
        }
        match &mut expr.kind {
            Operation::Call { name, body } => {
                if stack.contains(name) {
                    return Err(format!("Cyclic terminal call {name}"));
                }
                let target = sources
                    .get(name)
                    .ok_or_else(|| format!("Unknown terminal call {name}"))?;
                let mut resolved = target.body.clone();
                stack.push(name.clone());
                resolve(&mut resolved, sources, stack, budget, depth + 1)?;
                stack.pop();
                *body = Some(Box::new(resolved));
            }
            Operation::Sequence(items) | Operation::Alternative(items) => {
                for item in items {
                    resolve(item, sources, stack, budget, depth + 1)?;
                }
            }
            Operation::Negated { operand, .. } | Operation::UntilOperand { operand, .. } => {
                resolve(operand, sources, stack, budget, depth + 1)?;
            }
            _ => {}
        }
        Ok(())
    }
    let sources = terminals.clone();
    let mut budget = 10_000;
    for (name, terminal) in terminals {
        resolve(
            &mut terminal.body,
            &sources,
            &mut vec![name.clone()],
            &mut budget,
            0,
        )?;
        analyze(&mut terminal.body, &mut 10_000, profile, &sources)?;
        if terminal.body.nullable
            && !(profile.has_terminal_cardinality_decisions() && terminal.fragment)
        {
            return Err(super::AnalysisError {
                declaration: None,
                producer_declaration: Some(name.clone()),
                message: "Nullable terminals are unsupported".into(),
            });
        }
        if profile.has_terminal_decisions() {
            contextual_decisions(&mut terminal.body, Vec::new(), &mut 10_000, profile)?;
        }
    }
    Ok(())
}
/// Token arbitration is presently admitted only for disjoint terminal starts
/// outside the inherited identifier, number, quote, whitespace and comment starts.
/// Incomplete selected bodies commit an error; no successful-shorter fallback.
pub(super) fn check(grammar: &Grammar) -> Result<(), String> {
    let mut starts = Vec::new();
    for terminal in grammar
        .terminals
        .values()
        .filter(|terminal| !terminal.fragment)
    {
        for &(low, high) in &terminal.body.first {
            if grammar.hidden.is_none()
                && (high > 127
                    || (low..=high).any(|c| {
                        char::from_u32(u32::from(c)).is_none_or(|c| {
                            c.is_alphanumeric()
                                || c.is_whitespace()
                                || matches!(c, '_' | '\'' | '"' | '/')
                        })
                    }))
            {
                return Err("Terminal start conflicts with inherited lexical policy; explicit lexical prediction is not yet supported".into());
            }
        }
        if !grammar.profile.has_lexical_prediction() && overlap(&starts, &terminal.body.first) {
            return Err(
                "Overlapping terminal starts require unsupported lexical prediction".into(),
            );
        }
        starts.extend_from_slice(&terminal.body.first);
    }
    fn keywords(expr: &super::Expr, starts: &[(u16, u16)]) -> Result<(), String> {
        match &expr.kind {
            Kind::Literal(text) => {
                if text
                    .encode_utf16()
                    .next()
                    .is_some_and(|c| starts.iter().any(|(a, b)| *a <= c && c <= *b))
                {
                    return Err("Terminal start conflicts with a grammar keyword".into());
                }
            }
            Kind::Sequence(items) | Kind::Alternative(items) => {
                for item in items {
                    keywords(item, starts)?;
                }
            }
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => keywords(value, starts)?,
            _ => {}
        }
        Ok(())
    }
    if let Some(hidden) = &grammar.hidden {
        for name in hidden {
            if grammar.terminals.get(name).is_none_or(|t| t.fragment) {
                return Err(format!("Unknown or fragment hidden terminal {name}"));
            }
        }
        fn check_policy(
            expr: &super::Expr,
            grammar: &Grammar,
            hidden: &[String],
        ) -> Result<(), String> {
            let terminal = |name: &str| -> Result<(), String> {
                if grammar.terminals.get(name).is_none_or(|t| t.fragment)
                    || hidden.iter().any(|h| h == name)
                {
                    return Err(format!(
                        "Parser requires a visible declared terminal {name}"
                    ));
                }
                Ok(())
            };
            match &expr.kind {
                Kind::Call(name) if !grammar.rules.contains_key(name) => terminal(name)?,
                Kind::Reference(_) => terminal("ID")?,
                Kind::Literal(text) if !grammar.profile.has_lexical_prediction() => {
                    if let Some((name, target)) = selected(&grammar.terminals, text)
                        && (hidden.contains(name) || target.match_len(text)? != text.len())
                    {
                        return Err(
                            "Keyword and terminal overlap requires unsupported lexical prediction"
                                .into(),
                        );
                    }
                }
                Kind::Sequence(items) | Kind::Alternative(items) => {
                    for item in items {
                        check_policy(item, grammar, hidden)?;
                    }
                }
                Kind::Assign { value, .. }
                | Kind::BooleanAssign { value, .. }
                | Kind::SyntacticPredicate(value) => check_policy(value, grammar, hidden)?,
                _ => {}
            }
            Ok(())
        }
        for rule in grammar.rules.values() {
            if grammar.profile.has_scoped_hidden_tokens() {
                if let Some(policy) = &rule.hidden {
                    for name in policy {
                        if grammar
                            .terminals
                            .get(name)
                            .is_none_or(|terminal| terminal.fragment)
                        {
                            return Err(format!("Unknown or fragment hidden terminal {name}"));
                        }
                    }
                }
                // Visibility depends on the actual caller and nested rule scope.
                // Validate declarations here; the retained cursor owns consumption.
                check_policy(&rule.body, grammar, &[])?;
            } else {
                check_policy(&rule.body, grammar, hidden)?;
            }
        }
    } else {
        for rule in grammar.rules.values() {
            keywords(&rule.body, &starts)?;
        }
    }
    Ok(())
}
impl Terminal {
    pub fn starts(&self, input: &str) -> bool {
        input
            .encode_utf16()
            .next()
            .is_some_and(|c| self.body.first.iter().any(|(a, b)| *a <= c && c <= *b))
    }
    pub fn match_len(&self, input: &str) -> Result<usize, String> {
        self.match_len_with_work(input, &mut None)
    }
    pub fn match_len_with_work(
        &self,
        input: &str,
        work: &mut Option<usize>,
    ) -> Result<usize, String> {
        let mut matcher = Matcher {
            remaining: input.encode_utf16(),
            units: Vec::new(),
            work: *work,
            error: None,
            observe_failure: false,
            failure: None,
            prediction: ColdPrediction,
        };
        let result = matcher.matched(&self.body, 0);
        *work = matcher.work;
        if let Some(error) = matcher.error {
            return Err(error);
        }
        let end =
            result.ok_or_else(|| format!("Incomplete or invalid terminal {}", self.identity))?;
        utf8_endpoint(input, end)
    }
    /// Diagnostic observation over this exact committed recognizer. It does
    /// not retry an alternative or turn a failed body into a matched token.
    pub fn match_document(
        &self,
        input: &str,
        work: &mut Option<usize>,
    ) -> Result<DocumentMatch, String> {
        let mut matcher = Matcher {
            remaining: input.encode_utf16(),
            units: Vec::new(),
            work: *work,
            error: None,
            observe_failure: true,
            failure: None,
            prediction: ColdPrediction,
        };
        let result = matcher.matched(&self.body, 0);
        *work = matcher.work;
        if let Some(error) = matcher.error
            && (error != "No viable lexical alternative" || matcher.failure.is_none())
        {
            return Err(error);
        }
        if let Some(end) = result {
            return Ok(DocumentMatch::Matched(utf8_endpoint(input, end)?));
        }
        let failure = matcher.failure.ok_or_else(|| {
            format!(
                "No qualified document failure observation for terminal {}",
                self.identity
            )
        })?;
        let end = failure
            .utf16_offset
            .checked_add(usize::from(failure.actual.is_some()))
            .ok_or("Terminal failure position overflow")?;
        let consumed_bytes = utf8_endpoint(input, end)?;
        if consumed_bytes == 0 {
            return Err("Non-consuming committed terminal failure".into());
        }
        Ok(DocumentMatch::Failed {
            consumed_bytes,
            failure,
        })
    }
    /// Exact source-unit observation over the same committed interpreter.
    /// It does not establish scalar model String support.
    pub fn match_document_units(
        &self,
        input: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
    ) -> Result<DocumentUnitMatch, String> {
        self.match_document_units_mode(input, work, ColdPrediction)
    }
    pub(super) fn match_document_units_prepared(
        &self,
        input: impl Iterator<Item = u16>,
        registry: Option<&super::lexer_prediction::PreparedTerminalInitials<'_>>,
        work: &mut Option<usize>,
    ) -> Result<DocumentUnitMatch, String> {
        match registry {
            Some(registry) => self.match_document_units_with_initials(input, registry, work),
            None => self.match_document_units(input, work),
        }
    }
    pub(super) fn match_document_units_with_initials(
        &self,
        input: impl Iterator<Item = u16>,
        registry: &super::lexer_prediction::PreparedTerminalInitials<'_>,
        work: &mut Option<usize>,
    ) -> Result<DocumentUnitMatch, String> {
        if registry.selects(self, work)? {
            self.match_document_units_mode(input, work, PreparedPrediction(registry))
        } else {
            self.match_document_units_mode(input, work, ColdPrediction)
        }
    }
    fn match_document_units_mode<P: LocalPrediction>(
        &self,
        input: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        prediction: P,
    ) -> Result<DocumentUnitMatch, String> {
        let mut matcher = Matcher {
            remaining: input,
            units: Vec::new(),
            work: *work,
            error: None,
            observe_failure: true,
            failure: None,
            prediction,
        };
        let result = matcher.matched(&self.body, 0);
        *work = matcher.work;
        if let Some(error) = matcher.error
            && (error != "No viable lexical alternative" || matcher.failure.is_none())
        {
            return Err(error);
        }
        if let Some(end) = result {
            return Ok(DocumentUnitMatch::Matched(end));
        }
        let failure = matcher.failure.ok_or_else(|| {
            format!(
                "No qualified document failure observation for terminal {}",
                self.identity
            )
        })?;
        let consumed_units = failure
            .utf16_offset
            .checked_add(usize::from(failure.actual.is_some()))
            .ok_or("Terminal failure position overflow")?;
        if consumed_units == 0 {
            return Err("Non-consuming committed terminal failure".into());
        }
        Ok(DocumentUnitMatch::Failed {
            consumed_units,
            failure,
        })
    }
}
fn utf8_endpoint(input: &str, end: usize) -> Result<usize, String> {
    let mut units = 0;
    for (byte, ch) in input.char_indices() {
        units += ch.len_utf16();
        if units == end {
            return Ok(byte + ch.len_utf8());
        }
        if units > end {
            break;
        }
    }
    Err("Terminal result does not end on a complete UTF-8 character boundary".into())
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct TerminalFailure {
    pub utf16_offset: usize,
    pub expected: Option<u16>,
    pub early_exit: bool,
    pub actual: Option<u16>,
}
pub enum DocumentMatch {
    Matched(usize),
    Failed {
        consumed_bytes: usize,
        failure: TerminalFailure,
    },
}
pub enum DocumentUnitMatch {
    Matched(usize),
    Failed {
        consumed_units: usize,
        failure: TerminalFailure,
    },
}

fn literal_entry(expr: &Expression, depth: usize) -> Option<u16> {
    if depth > 64 || expr.cardinality != '1' {
        return None;
    }
    match &expr.kind {
        Operation::Literal(units) => units.first().copied(),
        Operation::Sequence(items) if items.len() == 1 => literal_entry(&items[0], depth + 1),
        Operation::Call {
            body: Some(body), ..
        } => literal_entry(body, depth + 1),
        _ => None,
    }
}
trait LocalPrediction: Copy {
    fn predict(
        self,
        program: &super::LexerPrediction,
        units: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        nongreedy: bool,
    ) -> Result<usize, String>;
}
#[derive(Clone, Copy)]
struct ColdPrediction;
const _: () = assert!(std::mem::size_of::<ColdPrediction>() == 0);
impl LocalPrediction for ColdPrediction {
    fn predict(
        self,
        program: &super::LexerPrediction,
        units: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        nongreedy: bool,
    ) -> Result<usize, String> {
        if nongreedy {
            program.predict_nongreedy_units(units, work)
        } else {
            program.predict_units(units, work)
        }
    }
}
#[derive(Clone, Copy)]
struct PreparedPrediction<'context, 'program>(
    &'context super::lexer_prediction::PreparedTerminalInitials<'program>,
);
impl LocalPrediction for PreparedPrediction<'_, '_> {
    fn predict(
        self,
        program: &super::LexerPrediction,
        units: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        nongreedy: bool,
    ) -> Result<usize, String> {
        self.0.predict(program, units, work, nongreedy)
    }
}
struct Matcher<I: Iterator<Item = u16>, P: LocalPrediction = ColdPrediction> {
    remaining: I,
    units: Vec<u16>,
    work: Option<usize>,
    error: Option<String>,
    observe_failure: bool,
    failure: Option<TerminalFailure>,
    prediction: P,
}
impl<I: Iterator<Item = u16>, P: LocalPrediction> Matcher<I, P> {
    fn unit(&mut self, offset: usize) -> Option<u16> {
        while self.units.len() <= offset {
            self.units.push(self.remaining.next()?);
        }
        self.units.get(offset).copied()
    }
    fn starts(&mut self, expr: &Expression, offset: usize) -> bool {
        self.unit(offset)
            .is_some_and(|c| expr.first.iter().any(|(a, b)| *a <= c && c <= *b))
    }
    fn select_cardinality(
        &mut self,
        decision: &super::LexerPrediction,
        offset: usize,
    ) -> Option<usize> {
        let mut cursor = offset;
        let mut work = self.work;
        let prediction = self.prediction;
        let units = std::iter::from_fn(|| {
            let unit = self.unit(cursor);
            cursor += 1;
            unit
        });
        let selected = prediction.predict(decision, units, &mut work, false);
        self.work = work;
        match selected {
            Ok(selected) => Some(selected),
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }
    fn matched(&mut self, expr: &Expression, offset: usize) -> Option<usize> {
        if let Some(decision) = &expr.cardinality_decision {
            match expr.cardinality {
                '?' => {
                    return match self.select_cardinality(decision, offset)? {
                        0 => self.expression_body(expr, offset),
                        1 => Some(offset),
                        _ => None,
                    };
                }
                '*' | '+' => {
                    let mut cursor = offset;
                    let mut count = 0usize;
                    loop {
                        match self.select_cardinality(decision, cursor)? {
                            0 => {
                                let end = self.expression_body(expr, cursor)?;
                                if end <= cursor {
                                    return None;
                                }
                                cursor = end;
                                count += 1;
                            }
                            1 => {
                                if count == 0 && expr.cardinality == '+' {
                                    if self.observe_failure {
                                        self.failure = Some(TerminalFailure {
                                            utf16_offset: cursor,
                                            expected: None,
                                            early_exit: true,
                                            actual: self.unit(cursor),
                                        });
                                    }
                                    return None;
                                }
                                return Some(cursor);
                            }
                            _ => return None,
                        }
                    }
                }
                _ => return None,
            }
        }
        match expr.cardinality {
            '1' => self.expression_body(expr, offset),
            '?' => {
                if self.starts(expr, offset) {
                    self.expression_body(expr, offset)
                } else {
                    Some(offset)
                }
            }
            '*' | '+' => {
                let mut cursor = offset;
                while self.starts(expr, cursor) {
                    let end = self.expression_body(expr, cursor)?;
                    if end <= cursor {
                        return None;
                    }
                    cursor = end;
                }
                (cursor > offset || expr.cardinality == '*').then_some(cursor)
            }
            _ => None,
        }
    }
    fn expression_body(&mut self, expr: &Expression, offset: usize) -> Option<usize> {
        if let (Some(decision), Operation::Alternative(items)) = (&expr.decision, &expr.kind) {
            let mut cursor = offset;
            let mut work = self.work;
            let prediction = self.prediction;
            let units = std::iter::from_fn(|| {
                let unit = self.unit(cursor);
                cursor += 1;
                unit
            });
            let selected = prediction.predict(decision, units, &mut work, false);
            self.work = work;
            let selected = match selected {
                Ok(selected) => selected,
                Err(error) => {
                    self.error = Some(error);
                    return None;
                }
            };
            return self.matched(items.get(selected)?, offset);
        }
        self.body(&expr.kind, offset)
    }
    fn body(&mut self, kind: &Operation, offset: usize) -> Option<usize> {
        match kind {
            Operation::Wildcard => self.unit(offset).map(|_| offset + 1),
            Operation::Eof => self.unit(offset).is_none().then_some(offset),
            Operation::Literal(text) => {
                for (index, expected) in text.iter().enumerate() {
                    let actual = self.unit(offset + index);
                    if actual != Some(*expected) {
                        if self.observe_failure {
                            self.failure = Some(TerminalFailure {
                                utf16_offset: offset + index,
                                expected: Some(*expected),
                                early_exit: false,
                                actual,
                            });
                        }
                        return None;
                    }
                }
                Some(offset + text.len())
            }
            Operation::NotCharacters(excluded) | Operation::Negated { excluded, .. } => self
                .unit(offset)
                .filter(|c| !excluded.iter().any(|(low, high)| low <= c && c <= high))
                .map(|_| offset + 1),
            Operation::Until(delimiter) => {
                // Linear delimiter search also bounds adversarial repeated prefixes.
                let mut failure = vec![0; delimiter.len()];
                for i in 1..delimiter.len() {
                    let mut matched = failure[i - 1];
                    while matched > 0 && delimiter[i] != delimiter[matched] {
                        matched = failure[matched - 1];
                    }
                    if delimiter[i] == delimiter[matched] {
                        matched += 1;
                    }
                    failure[i] = matched;
                }
                let mut cursor = offset;
                let mut matched = 0;
                while let Some(unit) = self.unit(cursor) {
                    while matched > 0 && delimiter[matched] != unit {
                        matched = failure[matched - 1];
                    }
                    if delimiter.get(matched) == Some(&unit) {
                        matched += 1;
                    }
                    cursor += 1;
                    if matched == delimiter.len() {
                        return Some(cursor);
                    }
                }
                None
            }
            Operation::UntilOperand { operand, decision } => {
                let decision = decision.as_ref()?;
                let mut cursor = offset;
                loop {
                    let mut lookahead = cursor;
                    let mut work = self.work;
                    let prediction = self.prediction;
                    let units = std::iter::from_fn(|| {
                        let unit = self.unit(lookahead);
                        lookahead += 1;
                        unit
                    });
                    let selected = prediction.predict(decision, units, &mut work, true);
                    self.work = work;
                    match selected {
                        Ok(1) => return self.matched(operand, cursor),
                        Ok(0) => {
                            self.unit(cursor)?;
                            cursor += 1;
                        }
                        Ok(_) => return None,
                        Err(error) => {
                            if self.observe_failure
                                && error == "No viable lexical alternative"
                                && self.unit(cursor).is_none()
                                && let Some(expected) = literal_entry(operand, 0)
                            {
                                self.failure = Some(TerminalFailure {
                                    utf16_offset: cursor,
                                    expected: Some(expected),
                                    early_exit: false,
                                    actual: None,
                                });
                            }
                            self.error = Some(error);
                            return None;
                        }
                    }
                }
            }
            Operation::Call { body, .. } => self.matched(body.as_ref()?, offset),
            Operation::Range(low, high) => self
                .unit(offset)
                .filter(|c| low <= c && c <= high)
                .map(|_| offset + 1),
            Operation::Sequence(items) => {
                let mut cursor = offset;
                for item in items {
                    cursor = self.matched(item, cursor)?;
                }
                Some(cursor)
            }
            Operation::Alternative(items) => {
                let selected = items.iter().find(|item| self.starts(item, offset))?;
                self.matched(selected, offset)
            }
        }
    }
}
/// Determine the one admitted terminal at a source position, never a fallback
/// among complete recognitions. The compiler proves disjoint FIRST sets.
pub(super) fn selected<'a>(
    terminals: &'a BTreeMap<String, Terminal>,
    source: &str,
) -> Option<(&'a String, &'a Terminal)> {
    terminals
        .iter()
        .find(|(_, terminal)| !terminal.fragment && terminal.starts(source))
}

#[cfg(test)]
mod source_projection_budget_tests {
    use super::*;

    #[test]
    fn projection_budget_exact_and_one_short_cover_every_retained_operand_node() {
        // Exact shape of the clean source operand !('ab'|'x'): transparent
        // group, alternative, multi-unit keyword and singleton keyword.
        let operand = expression(Operation::Sequence(vec![expression(
            Operation::Alternative(vec![
                expression(Operation::Literal(vec![97, 98])),
                expression(Operation::Literal(vec![120])),
            ]),
        )]));
        let terminals = BTreeMap::new();
        let mut exact = 4;
        let result = projected_negation_domain(&operand, &mut exact, &terminals).unwrap();
        assert_eq!(exact, 0);
        assert_eq!(result.excluded, vec![(120, 120)]);
        assert!(result.multi_keyword);
        let mut short = 3;
        let error = projected_negation_domain(&operand, &mut short, &terminals)
            .err()
            .unwrap();
        assert_eq!(error, "Grammar prediction budget exceeded");
        assert_eq!(short, 0);
    }

    #[test]
    fn empty_exclusion_prefix_cannot_skip_unsupported_suffix_or_general_sequence() {
        let terminals = BTreeMap::new();
        let first = expression(Operation::Literal(vec![97, 98]));
        let alternative = expression(Operation::Alternative(vec![
            first.clone(),
            expression(Operation::Eof),
        ]));
        let error = projected_negation_domain(&alternative, &mut 10, &terminals)
            .err()
            .unwrap();
        assert_eq!(error, "Terminal negation cannot contain EOF");
        let sequence = expression(Operation::Sequence(vec![
            first,
            expression(Operation::Literal(vec![120])),
        ]));
        let error = projected_negation_domain(&sequence, &mut 10, &terminals)
            .err()
            .unwrap();
        assert!(error.contains("General multi-element"));
    }

    fn fragment(name: &str, body: Expression) -> Terminal {
        Terminal {
            override_declared: false,
            fragment: true,
            datatype: crate::grammar::Datatype::EString,
            nominal_return: None,
            enumeration: None,
            identity: format!("audit.Projection::{name}"),
            start: 0,
            end: 0,
            body,
        }
    }

    #[test]
    fn literal_only_call_chain_has_exact_and_one_short_retained_node_budgets() {
        // Synthetic private layout/control fixture; actual source and callee
        // identities are independently checked by the full-source consumers.
        let literal = expression(Operation::Literal(vec![97, 98]));
        let inner = expression(Operation::Call {
            name: "INNER".into(),
            body: Some(Box::new(literal.clone())),
        });
        let outer = expression(Operation::Call {
            name: "PAIR".into(),
            body: Some(Box::new(inner.clone())),
        });
        let terminals = BTreeMap::from([
            ("INNER".into(), fragment("INNER", literal)),
            ("PAIR".into(), fragment("PAIR", inner)),
        ]);
        let mut exact = 3;
        let result = projected_negation_domain(&outer, &mut exact, &terminals).unwrap();
        assert_eq!(exact, 0);
        assert!(result.excluded.is_empty());
        assert!(result.multi_keyword && result.literal_chain);
        let mut short = 2;
        let error = projected_negation_domain(&outer, &mut short, &terminals)
            .err()
            .unwrap();
        assert_eq!(short, 0);
        assert_eq!(error, "Grammar prediction budget exceeded");
    }

    #[test]
    fn nested_complement_visits_full_original_operand_at_exact_and_short_budget() {
        let grouped = expression(Operation::Sequence(vec![expression(
            Operation::Alternative(vec![
                expression(Operation::Literal(vec![97, 98])),
                expression(Operation::Literal(vec![120])),
            ]),
        )]));
        let inner = expression(Operation::Negated {
            operand: Box::new(grouped),
            // Deliberately stale derived data: the whole-tree projector must
            // derive this domain from the retained operand, not this cache.
            excluded: vec![(97, 97)],
        });
        let terminals = BTreeMap::new();
        let mut exact = 5;
        let result = projected_negation_domain(&inner, &mut exact, &terminals).unwrap();
        assert_eq!(exact, 0);
        assert_eq!(result.excluded, vec![(0, 119), (121, u16::MAX)]);
        assert!(result.multi_keyword);
        assert!(!result.literal_chain);
        let mut short = 4;
        let error = projected_negation_domain(&inner, &mut short, &terminals)
            .err()
            .unwrap();
        assert_eq!(short, 0);
        assert_eq!(error, "Grammar prediction budget exceeded");
    }
}
