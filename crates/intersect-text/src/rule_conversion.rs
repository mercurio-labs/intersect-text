//! Shared language-package execution of explicit scalar rule policy.
use crate::grammar::{Datatype, Grammar};
pub use intersect_model::definitions::rule_conversion::RuleConversion;
use intersect_model::definitions::scalars::Builtin;
use serde_json::Value;
use std::collections::BTreeSet;

/// Validate actual grammar identities and the language-owned keyword-Boolean policy.
pub fn validate(grammar: Option<&Grammar>, bindings: &[RuleConversion]) -> Result<(), String> {
    if bindings.is_empty() {
        return Ok(());
    }
    let grammar = grammar.ok_or("Rule conversions require a text grammar")?;
    if !grammar.profile.has_rule_conversions() {
        return Err("Rule conversions require NativeV8".into());
    }
    if bindings.len() > grammar.rules.len() {
        return Err("Too many rule conversion bindings".into());
    }
    let mut seen = BTreeSet::new();
    for binding in bindings {
        if !seen.insert(&binding.rule) {
            return Err("Duplicate rule converter binding".into());
        }
        if binding.context != grammar.name
            || grammar.declaration_sources.get(&binding.rule) != Some(&binding.declaration)
        {
            return Err("Rule converter source/context identity mismatch".into());
        }
        if grammar.rules.get(&binding.rule).and_then(|r| r.datatype) != Some(Datatype::EBoolean)
            || binding.datatype != "http://www.eclipse.org/emf/2002/Ecore#//EBoolean"
        {
            return Err("Rule converter return type mismatch".into());
        }
        if binding.provider.name != "intersect.xtext.keyword-boolean"
            || binding.provider.version != "1"
        {
            return Err("Unsupported rule conversion provider".into());
        }
        if binding.keywords.is_empty()
            || binding.keywords.len() > 128
            || binding.keywords.keys().any(|s| s.is_empty())
        {
            return Err("Keyword Boolean conversion requires 1..128 nonempty spellings".into());
        }
    }
    Ok(())
}

/// Default Xtext STRING conversion. Lossless decoding is shared with syntax
/// import; the model's current scalar-string boundary is enforced explicitly.
pub(crate) fn string(spelling: &str) -> Result<String, String> {
    let value = crate::grammar::syntax::decode_string_literal(spelling)?;
    value.to_scalar_string().map_err(|_| {
        "STRING contains an unpaired UTF-16 surrogate unsupported by the model string representation".into()
    })
}

pub(crate) fn boolean_definition(
    package: crate::package::TextPackageView<'_>,
    rule: &str,
    spelling: &str,
) -> Result<Value, String> {
    boolean_with_bindings(
        package.grammar,
        &package.semantics.rule_conversions,
        rule,
        spelling,
        true,
    )
}
fn boolean_with_bindings(
    grammar: Option<&crate::grammar::Grammar>,
    bindings: &[RuleConversion],
    rule: &str,
    spelling: &str,
    allow_builtin: bool,
) -> Result<Value, String> {
    validate(grammar, bindings)?;
    if let Some(binding) = bindings.iter().find(|b| b.rule == rule) {
        return binding
            .keywords
            .get(spelling)
            .copied()
            .map(Value::Bool)
            .ok_or_else(|| format!("Unsupported spelling for Boolean rule {rule}"));
    }
    if allow_builtin {
        Builtin::EBoolean.parse(spelling)
    } else {
        Err(format!(
            "Missing explicit draft Boolean rule conversion: {rule}"
        ))
    }
}

pub(crate) fn bound_boolean(
    grammar: Option<&crate::grammar::Grammar>,
    bindings: &[RuleConversion],
    rule: &str,
    spelling: &str,
    budget: &mut intersect_model::core::query::QueryBudget,
) -> Result<Value, String> {
    // Same binding/key/provider precharges as the existing Package path.
    // The caller has separately charged the capability accessor first.
    for binding in bindings {
        budget.charge()?;
        for _ in binding.keywords.keys() {
            budget.charge()?;
        }
    }
    budget.charge()?;
    boolean_with_bindings(grammar, bindings, rule, spelling, false)
}
