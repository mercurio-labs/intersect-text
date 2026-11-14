//! Handwritten declaration-position binding and boolean environments. Both
//! structural inference and executable specialization consume these checks.
use super::*;

fn at(source: &SourceId, node: &Element, message: impl Into<String>) -> Diagnostic {
    Diagnostic::from_span(
        SourceSpan {
            source: source.clone(),
            start: node.start,
            end: node.end,
        },
        message,
    )
}

pub(super) fn condition(
    source: &SourceId,
    parameters: &[Located<String>],
    values: &[bool],
    node: &Element,
) -> Result<bool> {
    match &node.value {
        ElementKind::ConditionLiteral(value) => Ok(*value),
        ElementKind::Parameter(name) => {
            let index = parameters
                .iter()
                .position(|p| p.value == *name)
                .ok_or_else(|| at(source, node, format!("Unknown rule parameter {name}")))?;
            values
                .get(index)
                .copied()
                .ok_or_else(|| at(source, node, "Missing boolean parameter environment"))
        }
        ElementKind::ConditionNot(value) => Ok(!condition(source, parameters, values, value)?),
        ElementKind::Group(value) => condition(source, parameters, values, value),
        ElementKind::ConditionAnd(items) => {
            let mut result = true;
            for item in items {
                result &= condition(source, parameters, values, item)?;
            }
            Ok(result)
        }
        ElementKind::ConditionOr(items) => {
            let mut result = false;
            for item in items {
                result |= condition(source, parameters, values, item)?;
            }
            Ok(result)
        }
        _ => Err(at(source, node, "Expected a boolean parameter condition")),
    }
}

pub(super) fn arguments(
    source: &SourceId,
    parameters: &[Located<String>],
    values: &[bool],
    target: &[Located<String>],
    supplied: Option<&[Located<syntax::Argument>]>,
    call: &Element,
) -> Result<Vec<bool>> {
    if target.is_empty() {
        return if supplied.is_some() {
            Err(at(source, call, "Called rule has no parameters"))
        } else {
            Ok(vec![])
        };
    }
    let supplied = supplied.unwrap_or_default();
    let mut selected = BTreeSet::new();
    let mut result = vec![false; target.len()];
    let mixed = supplied.iter().any(|a| a.value.parameter.is_some())
        && supplied.iter().any(|a| a.value.parameter.is_none());
    let mut out_of_order = false;
    for (position, argument) in supplied.iter().enumerate() {
        let index = if let Some(name) = &argument.value.parameter {
            target
                .iter()
                .position(|p| p.value == name.value)
                .ok_or_else(|| {
                    at(
                        source,
                        call,
                        format!("Unknown called-rule parameter {}", name.value),
                    )
                })?
        } else {
            position
        };
        if index >= target.len() {
            return Err(at(
                source,
                call,
                format!(
                    "Expected {} rule arguments, got {}",
                    target.len(),
                    supplied.len()
                ),
            ));
        }
        if !selected.insert(index) {
            return Err(at(
                source,
                call,
                format!("Duplicate value for parameter {}", target[index].value),
            ));
        }
        out_of_order |= mixed && argument.value.parameter.is_some() && index != position;
        result[index] = condition(source, parameters, values, &argument.value.value)?;
    }
    if selected.len() != target.len() {
        return Err(at(
            source,
            call,
            format!(
                "Expected {} rule arguments, got {}",
                target.len(),
                selected.len()
            ),
        ));
    }
    if out_of_order {
        return Err(at(source, call, "Out of sequence named rule argument"));
    }
    Ok(result)
}

fn walk(
    engine: &Engine<'_>,
    parameters: &[Located<String>],
    values: &[bool],
    element: &Element,
) -> Result<()> {
    match &element.value {
        ElementKind::Call {
            rule,
            arguments: supplied,
        } => {
            if let Some(target) = engine.binding(&rule.value) {
                arguments(
                    engine.current_source(),
                    parameters,
                    values,
                    &target.parameters,
                    supplied.as_deref(),
                    element,
                )?;
            } else if supplied.is_some() {
                return Err(engine.at(element, "Missing called-rule parameter declaration"));
            }
        }
        ElementKind::Guard {
            condition: expression,
            elements,
        } => {
            condition(engine.current_source(), parameters, values, expression)?;
            for element in elements {
                walk(engine, parameters, values, element)?;
            }
        }
        ElementKind::Sequence(items)
        | ElementKind::Alternatives(items)
        | ElementKind::Unordered(items) => {
            for item in items {
                walk(engine, parameters, values, item)?;
            }
        }
        ElementKind::Group(item)
        | ElementKind::Cardinality { element: item, .. }
        | ElementKind::Predicate { element: item, .. }
        | ElementKind::Assignment { value: item, .. }
        | ElementKind::Negated(item)
        | ElementKind::Until(item) => walk(engine, parameters, values, item)?,
        ElementKind::CrossReference {
            spelling: Some(item),
            ..
        } => walk(engine, parameters, values, item)?,
        ElementKind::Range { left, right } => {
            walk(engine, parameters, values, left)?;
            walk(engine, parameters, values, right)?;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn check_rule(engine: &Engine<'_>, rule: &Located<syntax::Rule>) -> Result<()> {
    let parameters = rule.value.parameters.as_deref().unwrap_or_default();
    if rule.value.parameters.as_ref().is_some_and(Vec::is_empty) {
        return Err(engine.at(rule, "Empty parameter declarations are not valid Xtext"));
    }
    if let Some(prior) = engine
        .inherited
        .and_then(|p| p.effective_rules().get(&rule.value.name.value))
        && !parameters
            .iter()
            .map(|p| &p.value)
            .eq(prior.parameters.iter().map(|p| &p.value))
    {
        return Err(engine.at(
            rule,
            "Parameter list is incompatible with the inherited rule",
        ));
    }
    walk(
        engine,
        parameters,
        &vec![false; parameters.len()],
        &rule.value.body,
    )
}

// Pinned Xtext validates authored construction order before parameter guards are
// flattened. A false guard therefore cannot hide an invalid second simple action.
// This is source admission only; executable object flow stays in the shared model
// grammar/compiler and interpreter.
pub(super) fn check_authored_actions(
    engine: &Engine<'_>,
    rule: &Located<syntax::Rule>,
) -> Result<()> {
    fn current(engine: &Engine<'_>, node: &Element, created: bool, many: bool) -> Result<bool> {
        match &node.value {
            ElementKind::Sequence(items)
            | ElementKind::Guard {
                elements: items, ..
            } => {
                let mut output = created;
                for item in items {
                    output = current(engine, item, output, many)?;
                }
                Ok(output)
            }
            ElementKind::Alternatives(items) => {
                let mut output = created;
                for item in items {
                    output |= current(engine, item, created, many)?;
                }
                Ok(output)
            }
            ElementKind::Unordered(items) => {
                let mut output = created;
                for item in items {
                    output &= current(engine, item, created, many)?;
                }
                Ok(output)
            }
            ElementKind::Group(item) | ElementKind::Predicate { element: item, .. } => {
                current(engine, item, created, many)
            }
            ElementKind::Cardinality { element, operator } => current(
                engine,
                element,
                created,
                many || matches!(operator, '*' | '+'),
            ),
            ElementKind::Assignment { .. } => Ok(true),
            ElementKind::Action { feature, .. } => {
                if feature.is_none() && (created || many) {
                    return Err(engine.at(node, "An unassigned action is not allowed when current was already created or the action is repeated"));
                }
                Ok(true)
            }
            ElementKind::Call { rule, .. } => {
                let constructs = engine.binding(&rule.value).is_some_and(|binding| {
                    matches!(binding.kind, RuleKind::Parser | RuleKind::Fragment)
                        && (binding.wildcard
                            || binding
                                .target
                                .as_ref()
                                .is_some_and(|target| engine.class(target)))
                });
                Ok(created || constructs)
            }
            _ => Ok(created),
        }
    }
    if matches!(rule.value.kind, RuleKind::Parser | RuleKind::Fragment)
        && (rule.value.wildcard || engine.class(&engine.rule_type(&rule.value.name)?))
    {
        current(
            engine,
            &rule.value.body,
            rule.value.kind == RuleKind::Fragment,
            false,
        )?;
    }
    Ok(())
}
