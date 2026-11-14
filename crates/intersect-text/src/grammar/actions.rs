//! Shared action admission and bounded current-object flow analysis.
use super::{Expr, Grammar, Kind, Rule};
use intersect_model::definitions::schema::Schema;
use std::collections::BTreeSet;

pub(super) fn leading(expr: &Expr) -> Option<&str> {
    if expr.cardinality != '1' {
        return None;
    }
    match &expr.kind {
        Kind::Action(class) => Some(class),
        Kind::Sequence(items) => items.first().and_then(leading),
        _ => None,
    }
}

// Assigned values and fragment bodies retain their existing ownership paths.
// This detects only direct unassigned object calls in the enclosing rule.
pub(super) fn has_unassigned_object_call(expr: &Expr, grammar: &Grammar) -> bool {
    match &expr.kind {
        Kind::SyntacticPredicate(value) => has_unassigned_object_call(value, grammar),
        Kind::Call(name) => grammar
            .rules
            .get(name)
            .is_some_and(|rule| !rule.fragment && rule.datatype.is_none()),
        Kind::Sequence(items) | Kind::Alternative(items) => items
            .iter()
            .any(|item| has_unassigned_object_call(item, grammar)),
        _ => false,
    }
}
pub(super) fn simple_count(expr: &Expr) -> usize {
    match &expr.kind {
        Kind::Action(_) => 1,
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => simple_count(value),
        Kind::Sequence(items) | Kind::Alternative(items) => items.iter().map(simple_count).sum(),
        _ => 0,
    }
}

pub(super) fn check(rule: &Rule, schema: &Schema, grammar: &Grammar) -> Result<(), String> {
    if rule.uses_current_flow(grammar) {
        return check_current(rule, schema, grammar);
    }
    let actions = simple_count(&rule.body);
    if actions == 0 {
        return Ok(());
    }
    if rule.fragment || rule.datatype.is_some() {
        return Err("Simple actions are unsupported in fragments or datatype rules".into());
    }
    if actions != 1 || leading(&rule.body).is_none() {
        return Err(
            "Only one unconditional leading simple action is supported per object rule".into(),
        );
    }
    let selected = rule.construction_class();
    let class = schema.classes.get(selected).ok_or("Unknown action class")?;
    if class.abstract_type || class.interface_type {
        return Err("A simple action requires a concrete non-interface class".into());
    }
    if !schema.conforms(selected, &rule.class) {
        return Err(format!(
            "Action type {selected} does not conform to rule return type {}",
            rule.class
        ));
    }
    Ok(())
}

pub(super) fn has_capture(expr: &Expr) -> bool {
    match &expr.kind {
        Kind::Capture { .. } => true,
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => has_capture(value),
        Kind::Sequence(items) | Kind::Alternative(items) => items.iter().any(has_capture),
        _ => false,
    }
}

type Flow = BTreeSet<Option<String>>;
fn check_current(rule: &Rule, schema: &Schema, grammar: &Grammar) -> Result<(), String> {
    if !grammar.profile.has_current_actions() {
        return Err("Assigned-current actions require NativeV5".into());
    }
    if rule.fragment || rule.datatype.is_some() {
        return Err("Assigned-current actions require an object parser rule".into());
    }
    let mut budget = 10_000;
    let output = flow(
        &rule.body,
        &[None].into(),
        rule,
        schema,
        grammar,
        &mut budget,
    )?;
    if output.contains(&None) && !grammar.profile.has_constant_guards() {
        return Err("Current-action rule may finish without an object".into());
    }
    Ok(())
}
fn flow(
    expr: &Expr,
    input: &Flow,
    rule: &Rule,
    schema: &Schema,
    grammar: &Grammar,
    budget: &mut usize,
) -> Result<Flow, String> {
    let mut output = flow_once(expr, input, rule, schema, grammar, budget)?;
    if ['*', '+'].contains(&expr.cardinality) {
        loop {
            let mut next = output.clone();
            next.extend(flow_once(expr, &output, rule, schema, grammar, budget)?);
            if next == output {
                break;
            }
            output = next;
        }
    }
    if ['?', '*'].contains(&expr.cardinality) {
        output.extend(input.iter().cloned());
    }
    Ok(output)
}
fn flow_once(
    expr: &Expr,
    input: &Flow,
    rule: &Rule,
    schema: &Schema,
    grammar: &Grammar,
    budget: &mut usize,
) -> Result<Flow, String> {
    if *budget == 0 {
        return Err("Grammar current-object analysis budget exceeded".into());
    }
    *budget -= 1;
    match &expr.kind {
        Kind::SyntacticPredicate(value) => flow(value, input, rule, schema, grammar, budget),
        Kind::Sequence(items) => {
            let mut current = input.clone();
            for item in items {
                current = flow(item, &current, rule, schema, grammar, budget)?;
            }
            Ok(current)
        }
        Kind::Alternative(items) => {
            let mut output = Flow::new();
            for item in items {
                output.extend(flow(item, input, rule, schema, grammar, budget)?);
            }
            Ok(output)
        }
        Kind::Call(name) => {
            if super::is_unassigned_string_call(name, grammar) {
                return Ok(input.clone());
            }
            let target = grammar
                .rules
                .get(name)
                .ok_or_else(|| format!("Unknown object rule {name}"))?;
            if target.fragment
                && matches!(
                    grammar.profile,
                    super::Profile::NativeV29
                        | super::Profile::NativeV30
                        | super::Profile::NativeV31
                        | super::Profile::NativeV32
                        | super::Profile::NativeV33
                        | super::Profile::NativeV34
                        | super::Profile::NativeV35
                        | super::Profile::NativeV36
                        | super::Profile::NativeV37
                        | super::Profile::NativeV38
                        | super::Profile::NativeV39
                        | super::Profile::NativeV40
                        | super::Profile::NativeV41
                        | super::Profile::NativeV42
                        | super::Profile::NativeV43
                        | super::Profile::NativeV44
                        | super::Profile::NativeV45
                        | super::Profile::NativeV46
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
                )
            {
                let mut checked = BTreeSet::new();
                for owner in input {
                    let owner = owner
                        .as_deref()
                        .ok_or("Fragment call requires a current object")?;
                    super::check_expr(expr, owner, schema, grammar, false)?;
                    super::check_fragment_context(
                        expr,
                        owner,
                        grammar,
                        schema,
                        &mut checked,
                        budget,
                    )?;
                }
                return Ok(input.clone());
            }
            if target.fragment || target.datatype.is_some() {
                return Err("Current-object delegation requires an object rule".into());
            }
            if input.iter().any(Option::is_some) {
                return Err(
                    "Unassigned object replacement after construction is unsupported".into(),
                );
            }
            if !schema.conforms(&target.class, &rule.class) {
                return Err("Current-object delegation return type mismatch".into());
            }
            Ok([Some(target.class.clone())].into())
        }
        Kind::Capture {
            class,
            feature,
            many,
        } => {
            let target = schema.classes.get(class).ok_or("Unknown capture class")?;
            if target.abstract_type || target.interface_type {
                return Err("Capture requires a concrete non-interface class".into());
            }
            if !schema.conforms(class, &rule.class) {
                return Err("Capture class does not conform to rule return type".into());
            }
            let field = super::writable_feature(schema, class, feature, *many, grammar)?;
            if !field.reference
                || !(field.containment
                    || field.derived
                        && matches!(
                            grammar.profile,
                            super::Profile::NativeV22
                                | super::Profile::NativeV23
                                | super::Profile::NativeV24
                                | super::Profile::NativeV25
                                | super::Profile::NativeV26
                                | super::Profile::NativeV27
                                | super::Profile::NativeV28
                                | super::Profile::NativeV29
                                | super::Profile::NativeV30
                                | super::Profile::NativeV31
                                | super::Profile::NativeV32
                                | super::Profile::NativeV33
                                | super::Profile::NativeV34
                                | super::Profile::NativeV35
                                | super::Profile::NativeV36
                                | super::Profile::NativeV37
                                | super::Profile::NativeV38
                                | super::Profile::NativeV39
                                | super::Profile::NativeV40
                                | super::Profile::NativeV41
                                | super::Profile::NativeV42
                                | super::Profile::NativeV43
                                | super::Profile::NativeV44
                                | super::Profile::NativeV45
                                | super::Profile::NativeV46
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
                        ))
            {
                return Err("Current capture requires a containment reference".into());
            }
            for owner in input {
                let owner = owner.as_ref().ok_or("Capture requires a current object")?;
                if !schema.conforms(owner, &field.target) {
                    return Err("Capture current-type/feature binding mismatch".into());
                }
            }
            Ok([Some(class.clone())].into())
        }
        Kind::Action(class)
            if matches!(
                grammar.profile,
                super::Profile::NativeV26
                    | super::Profile::NativeV27
                    | super::Profile::NativeV28
                    | super::Profile::NativeV29
                    | super::Profile::NativeV30
                    | super::Profile::NativeV31
                    | super::Profile::NativeV32
                    | super::Profile::NativeV33
                    | super::Profile::NativeV34
                    | super::Profile::NativeV35
                    | super::Profile::NativeV36
                    | super::Profile::NativeV37
                    | super::Profile::NativeV38
                    | super::Profile::NativeV39
                    | super::Profile::NativeV40
                    | super::Profile::NativeV41
                    | super::Profile::NativeV42
                    | super::Profile::NativeV43
                    | super::Profile::NativeV44
                    | super::Profile::NativeV45
                    | super::Profile::NativeV46
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
            ) =>
        {
            if input.iter().any(Option::is_some) {
                return Err("Simple action replacement after construction is unsupported".into());
            }
            let target = schema.classes.get(class).ok_or("Unknown action class")?;
            if target.abstract_type || target.interface_type {
                return Err("A simple action requires a concrete non-interface class".into());
            }
            if !schema.conforms(class, &rule.class) {
                return Err("Simple action does not conform to rule return type".into());
            }
            Ok([Some(class.clone())].into())
        }
        Kind::Action(_) => Err("Simple actions mixed with current captures are unsupported".into()),
        Kind::Assign { .. } | Kind::BooleanAssign { .. }
            if matches!(
                grammar.profile,
                super::Profile::NativeV28
                    | super::Profile::NativeV29
                    | super::Profile::NativeV30
                    | super::Profile::NativeV31
                    | super::Profile::NativeV32
                    | super::Profile::NativeV33
                    | super::Profile::NativeV34
                    | super::Profile::NativeV35
                    | super::Profile::NativeV36
                    | super::Profile::NativeV37
                    | super::Profile::NativeV38
                    | super::Profile::NativeV39
                    | super::Profile::NativeV40
                    | super::Profile::NativeV41
                    | super::Profile::NativeV42
                    | super::Profile::NativeV43
                    | super::Profile::NativeV44
                    | super::Profile::NativeV45
                    | super::Profile::NativeV46
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
            ) =>
        {
            let mut output = Flow::new();
            for owner in input {
                let class = if let Some(class) = owner.as_deref() {
                    class
                } else {
                    let target = schema
                        .classes
                        .get(&rule.class)
                        .ok_or("Unknown implicit construction class")?;
                    if target.abstract_type || target.interface_type {
                        return Err(
                            "Implicit construction requires a concrete non-interface class".into(),
                        );
                    }
                    &rule.class
                };
                super::check_expr(expr, class, schema, grammar, false)?;
                output.insert(Some(class.to_owned()));
            }
            Ok(output)
        }
        Kind::Literal(_) => {
            super::check_expr(expr, "", schema, grammar, false)?;
            Ok(input.clone())
        }
        _ => {
            for owner in input {
                super::check_expr(
                    expr,
                    owner
                        .as_deref()
                        .ok_or("Assignment requires a current object")?,
                    schema,
                    grammar,
                    false,
                )?;
            }
            Ok(input.clone())
        }
    }
}
