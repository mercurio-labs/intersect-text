//! EString inference and shared admission for nonnullable, acyclic scalar rules.
use super::{Datatype, Expr, Grammar, Kind};
use intersect_model::definitions::schema::Schema;

pub(super) fn check(expr: &Expr, schema: &Schema, grammar: &Grammar) -> Result<(), String> {
    match &expr.kind {
        Kind::SyntacticPredicate(value) => check(value, schema, grammar),
        Kind::Literal(_) => super::check_expr(expr, "", schema, grammar, false),
        Kind::Call(name)
            if ["ID", "STRING", "INT"].contains(&name.as_str())
                || grammar.terminals.contains_key(name) =>
        {
            Ok(())
        }
        Kind::Call(name)
            if grammar
                .rules
                .get(name)
                .is_some_and(|rule| rule.datatype == Some(Datatype::EString)) =>
        {
            Ok(())
        }
        Kind::Call(name)
            if grammar.profile.has_durable_documents()
                && grammar.rules.get(name).is_some_and(|rule| {
                    rule.datatype == Some(Datatype::Nominal)
                        && schema.datatypes.get(&rule.class).is_some_and(|datatype| {
                            datatype.parameters().is_empty()
                                && datatype.application().is_none()
                                && datatype.serializable()
                                && rule.nominal_return.as_ref() == Some(datatype.declaration())
                        })
                }) =>
        {
            Ok(())
        }
        Kind::Call(name)
            if matches!(
                grammar.profile,
                super::Profile::NativeV46
                    | super::Profile::NativeV47
                    | super::Profile::NativeV48
                    | super::Profile::NativeV49
                    | super::Profile::NativeV50
                    | super::Profile::NativeV51
                    | super::Profile::NativeV52
                    | super::Profile::NativeV53
                    | super::Profile::NativeV54
                    | super::Profile::NativeV55
                    | super::Profile::NativeV56
                    | super::Profile::NativeV57
                    | super::Profile::NativeV58
                    | super::Profile::NativeV59
                    | super::Profile::NativeV60
                    | super::Profile::NativeV61
                    | super::Profile::NativeV62
                    | super::Profile::NativeV63
                    | super::Profile::NativeV64
                    | super::Profile::NativeV65
                    | super::Profile::NativeV66
                    | super::Profile::NativeV67
                    | super::Profile::NativeV68
                    | super::Profile::NativeV69
                    | super::Profile::NativeV70
                    | super::Profile::NativeV71
                    | super::Profile::NativeV72
                    | super::Profile::NativeV73
                    | super::Profile::NativeV74
                    | super::Profile::NativeV75
                    | super::Profile::NativeV76
                    | super::Profile::NativeV77
                    | super::Profile::NativeV78
                    | super::Profile::NativeV79
                    | super::Profile::NativeV80
                    | super::Profile::NativeV81
                    | super::Profile::NativeV82
            ) && grammar
                .rules
                .get(name)
                .is_some_and(|rule| rule.datatype == Some(Datatype::Enum)) =>
        {
            Ok(())
        }
        Kind::Sequence(items) | Kind::Alternative(items) => {
            for item in items {
                check(item, schema, grammar)?;
            }
            Ok(())
        }
        _ => Err(
            "Datatype parser bodies require literals, terminals or EString datatype calls".into(),
        ),
    }
}

/// Classify the complete effective graph before existing admission/execution.
/// Greatest fixed point excludes model operations; syntax reachability then
/// distinguishes scalar cycles from call-only cycles. Execution still rejects
/// recursive/nullable datatypes through the existing shared checks.
pub(super) fn infer(grammar: &mut Grammar, schema: &Schema) -> Result<(), super::AnalysisError> {
    use std::collections::BTreeSet;
    if !grammar.profile.has_return_inference() {
        return Ok(());
    }
    let mut candidates: BTreeSet<String> = grammar
        .rules
        .iter()
        .filter(|(_, rule)| rule.inferred_return || rule.datatype == Some(Datatype::EString))
        .map(|(name, _)| name.clone())
        .collect();
    loop {
        let next: BTreeSet<_> = candidates
            .iter()
            .filter(|name| {
                let rule = &grammar.rules[*name];
                !rule.inferred_return || scalar_shape(&rule.body, &candidates, grammar)
            })
            .cloned()
            .collect();
        if next == candidates {
            break;
        }
        candidates = next;
    }
    let mut concrete = BTreeSet::new();
    loop {
        let next: BTreeSet<_> = candidates
            .iter()
            .filter(|name| {
                let rule = &grammar.rules[*name];
                !rule.inferred_return || has_syntax(&rule.body, &concrete, grammar)
            })
            .cloned()
            .collect();
        if next == concrete {
            break;
        }
        concrete = next;
    }
    for (name, rule) in &mut grammar.rules {
        if !rule.inferred_return {
            continue;
        }
        if concrete.contains(name) {
            if ["ID", "STRING", "INT"].contains(&name.as_str()) {
                return Err(super::AnalysisError {
                    declaration: Some(name.clone()),
                    producer_declaration: None,
                    message: "Datatype rules shadowing common terminals are unsupported".into(),
                });
            }
            rule.datatype = Some(Datatype::EString);
            rule.class.clear();
        } else {
            rule.datatype = None;
            let declaration_name = if grammar.profile.has_explicit_parser_calls() {
                name.strip_prefix('@')
                    .and_then(|n| n.rsplit_once("::"))
                    .map_or(name.as_str(), |(_, local)| local)
            } else {
                name.as_str()
            };
            rule.class = super::type_names::rule_class_key(schema, &schema.uri, declaration_name)
                .map_err(|message| super::AnalysisError {
                    declaration: Some(name.clone()),
                    producer_declaration: None,
                    message,
                })?
                .to_owned();
        }
    }
    Ok(())
}
fn terminal(name: &str, grammar: &Grammar) -> bool {
    ["ID", "STRING", "INT"].contains(&name) || grammar.terminals.contains_key(name)
}
fn scalar_shape(
    expr: &Expr,
    candidates: &std::collections::BTreeSet<String>,
    grammar: &Grammar,
) -> bool {
    match &expr.kind {
        Kind::SyntacticPredicate(value) => scalar_shape(value, candidates, grammar),
        Kind::Literal(_) => true,
        Kind::Call(name) => terminal(name, grammar) || candidates.contains(name),
        Kind::Sequence(items) | Kind::Alternative(items) => {
            items.iter().all(|e| scalar_shape(e, candidates, grammar))
        }
        _ => false,
    }
}
fn has_syntax(
    expr: &Expr,
    concrete: &std::collections::BTreeSet<String>,
    grammar: &Grammar,
) -> bool {
    match &expr.kind {
        Kind::SyntacticPredicate(value) => has_syntax(value, concrete, grammar),
        Kind::Literal(_) => true,
        Kind::Call(name) => terminal(name, grammar) || concrete.contains(name),
        Kind::Sequence(items) | Kind::Alternative(items) => {
            items.iter().any(|e| has_syntax(e, concrete, grammar))
        }
        _ => false,
    }
}
