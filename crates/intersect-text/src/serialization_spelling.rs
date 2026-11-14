//! Grammar/definition-derived defaults. Domain conversion and inverse scoping
//! remain explicit extension points; defaults never invent language names.
use super::{TerminalSpelling, *};
use intersect_model::definitions::{enums::EnumValue, scalars::Builtin};

pub fn default_spelling(
    package: &crate::package::Package,
    spelling: TerminalSpelling<'_>,
) -> Result<String, String> {
    default_spelling_with_conversions(package, spelling, None)
}

pub(crate) fn default_spelling_with_conversions(
    package: &crate::package::Package,
    spelling: TerminalSpelling<'_>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
) -> Result<String, String> {
    let grammar = package
        .grammar
        .as_ref()
        .ok_or("Spelling requires a grammar")?;
    let value = if let Some(target) = spelling.target {
        if package.semantics.scope != "unique_names" {
            return Err("This scope requires an explicit inverse reference spelling policy".into());
        }
        target
            .values
            .get("name")
            .ok_or("Unique-name reference target has no name")?
    } else {
        spelling.value
    };
    if let Some(rule) = grammar.rules.get(spelling.terminal) {
        if !rule.enum_literals.is_empty() {
            let enumeration: EnumValue = serde_json::from_value(value.clone())
                .map_err(|e| format!("Invalid enum serialization value: {e}"))?;
            return rule
                .enum_literals
                .iter()
                .find(|(_, v)| **v == enumeration)
                .map(|(text, _)| text.clone())
                .ok_or_else(|| "Enum value is absent from the selected grammar rule".into());
        }
        if let Some(binding) = package
            .semantics
            .rule_conversions
            .iter()
            .find(|b| b.rule == spelling.terminal)
        {
            return binding
                .keywords
                .iter()
                .find(|(_, v)| Some(**v) == value.as_bool())
                .map(|(text, _)| text.clone())
                .ok_or_else(|| "No inverse Boolean rule spelling".into());
        }
    }
    let field = package
        .schema
        .effective_feature(&spelling.owner.class, spelling.feature)?
        .feature;
    if !field.reference
        && let Some(enumeration) = package.schema.enumerations.get(&field.target)
    {
        let value = serde_json::from_value(value.clone())
            .map_err(|e| format!("Invalid enum value: {e}"))?;
        return enumeration.format(&value).map(str::to_owned);
    }
    let raw = if !field.reference
        && let Some(datatype) = package.schema.datatypes.get(&field.target)
    {
        datatype.format_json(value, conversions)?
    } else {
        match value {
            Value::String(text) => text.clone(),
            Value::Number(_) | Value::Bool(_) => value.to_string(),
            _ => Builtin::named(&field.target)
                .ok_or("Structured value requires a serialization codec")?
                .format(value)?
                .ok_or("Null values have no terminal spelling")?,
        }
    };
    if spelling.terminal == "STRING" {
        return quote(&raw);
    }
    if spelling.terminal == "ID" && keyword(grammar, &raw) {
        return Ok(format!("^{raw}"));
    }
    if let Some(terminal) = grammar.terminals.get(spelling.terminal) {
        if terminal.match_len(&raw).is_ok_and(|n| n == raw.len()) && !raw.is_empty() {
            return Ok(raw);
        }
        let quoted = quote(&raw)?;
        if terminal.match_len(&quoted).is_ok_and(|n| n == quoted.len()) {
            return Ok(quoted);
        }
        return Err(format!(
            "Value has no default spelling for terminal {}",
            spelling.terminal
        ));
    }
    Ok(raw)
}

fn quote(text: &str) -> Result<String, String> {
    serde_json::to_string(text).map_err(|e| e.to_string())
}
fn keyword(grammar: &Grammar, text: &str) -> bool {
    fn contains(expr: &Expr, text: &str) -> bool {
        match &expr.kind {
            Kind::Literal(value) => value == text,
            Kind::Sequence(items) | Kind::Alternative(items) => {
                items.iter().any(|e| contains(e, text))
            }
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => contains(value, text),
            _ => false,
        }
    }
    grammar.rules.values().any(|r| contains(&r.body, text))
}
