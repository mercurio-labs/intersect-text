//! Exact closed-graph correspondence for canonical text verification. Source
//! locations, lifecycle metadata and construction evidence are not model values.
use intersect_model::{core::model::Model, definitions::schema::Schema};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn correspondence(
    expected: &Model,
    actual: &Model,
    schema: &Schema,
) -> Result<BTreeMap<String, String>, String> {
    compare(expected, actual, schema, None)
}
pub(crate) fn persistent_correspondence(
    expected: &Model,
    actual: &Model,
    schema: &Schema,
    left: &super::state::State,
    right: &super::state::State,
) -> Result<BTreeMap<String, String>, String> {
    compare(expected, actual, schema, Some((left, right)))
}
fn selected<'a>(
    state: Option<&super::state::State>,
    object: &intersect_model::core::model::Element,
    name: &str,
    value: &'a Value,
) -> Result<Option<std::borrow::Cow<'a, Value>>, String> {
    let Some(state) = state else {
        return Ok(Some(std::borrow::Cow::Borrowed(value)));
    };
    let member = state.member(&object.id, name)?;
    Ok(if member.whole {
        None
    } else if member.omitted.is_empty() {
        Some(std::borrow::Cow::Borrowed(value))
    } else {
        Some(std::borrow::Cow::Owned(member.value(value)))
    })
}
fn compare(
    expected: &Model,
    actual: &Model,
    schema: &Schema,
    states: Option<(&super::state::State, &super::state::State)>,
) -> Result<BTreeMap<String, String>, String> {
    if expected.schema_revision != actual.schema_revision
        || (states.is_none() && expected.elements.len() != actual.elements.len())
        || !expected.resources.keys().eq(actual.resources.keys())
    {
        return Err("Generated text changed model structure or schema identity".into());
    }
    let original: BTreeMap<_, _> = expected
        .elements
        .iter()
        .map(|object| (object.id.as_str(), object))
        .collect();
    let parsed: BTreeMap<_, _> = actual
        .elements
        .iter()
        .map(|object| (object.id.as_str(), object))
        .collect();
    let mut pending = Vec::new();
    for (uri, resource) in &expected.resources {
        let output = actual
            .resources
            .get(uri)
            .ok_or("Generated resource is missing")?;
        if resource.uri != output.uri
            || resource.roots.len() != output.roots.len()
            || resource.load_state != output.load_state
            || resource.diagnostics != output.diagnostics
        {
            return Err("Generated text changed resource roots or state".into());
        }
        pending.extend(
            resource
                .roots
                .iter()
                .cloned()
                .zip(output.roots.iter().cloned()),
        );
    }
    let mut identities = BTreeMap::new();
    let mut covered = BTreeSet::new();
    while let Some((before, after)) = pending.pop() {
        if states.is_some() && identities.get(&before) == Some(&after) {
            continue;
        }
        if identities.insert(before.clone(), after.clone()).is_some()
            || !covered.insert(after.clone())
        {
            return Err("Generated containment correspondence is not one-to-one".into());
        }
        let left = *original
            .get(before.as_str())
            .ok_or("Original contained object is missing")?;
        let right = *parsed
            .get(after.as_str())
            .ok_or("Generated contained object is missing")?;
        if left.class != right.class
            || left.type_identity != right.type_identity
            || left.resource != right.resource
            || !left.values.keys().eq(right.values.keys())
        {
            return Err(format!(
                "Generated text changed object type, resource or stored features for {before}"
            ));
        }
        let same_assignment = match states {
            Some((l, r)) => {
                let ignored = l.ignored(&left.id);
                l.assignment_state(left, schema, &ignored)?
                    == r.assignment_state(right, schema, &ignored)?
            }
            None => left.assignment_state == right.assignment_state,
        };
        if !same_assignment {
            return Err(format!(
                "Generated text changed persistent assignment state for {before}"
            ));
        }
        for (name, field) in schema.features(&left.class)? {
            if !field.containment
                || (field.derived
                    && !states.is_some_and(|(l, _)| {
                        l.computed.contains_key(&(before.clone(), name.clone()))
                    }))
            {
                continue;
            }
            let l = selected(
                states.map(|(l, _)| l),
                left,
                &name,
                states
                    .map_or_else(|| left.values.get(&name), |(l, _)| l.value(left, &name))
                    .ok_or("Original containment value is missing")?,
            )?;
            let Some(l) = l else {
                continue;
            };
            let r = selected(
                states.map(|(_, r)| r),
                right,
                &name,
                states
                    .map_or_else(|| right.values.get(&name), |(_, r)| r.value(right, &name))
                    .ok_or("Generated containment value is missing")?,
            )?;
            let lt = targets(&l)?;
            let rt = r.as_deref().map(targets).transpose()?.unwrap_or_default();
            if lt.len() != rt.len() {
                return Err(format!(
                    "Generated text changed persistent containment {}.{name}",
                    left.id
                ));
            }
            pending.extend(lt.into_iter().cloned().zip(rt.into_iter().cloned()));
        }
    }
    let (expected_count, actual_count) = states
        .map_or((original.len(), parsed.len()), |(left, right)| {
            (left.retained.len(), right.retained.len())
        });
    if identities.len() != expected_count || covered.len() != actual_count {
        return Err("Generated correspondence did not cover every persistent object".into());
    }
    for (before, after) in &identities {
        let left = *original
            .get(before.as_str())
            .ok_or("Missing original mapped object")?;
        let right = *parsed
            .get(after.as_str())
            .ok_or("Missing generated mapped object")?;
        let mut values: BTreeMap<&str, &Value> =
            left.values.iter().map(|(k, v)| (k.as_str(), v)).collect();
        if let Some((state, _)) = states {
            values.extend(
                state
                    .computed
                    .iter()
                    .filter(|((id, _), _)| id == before)
                    .map(|((_, name), value)| (name.as_str(), value)),
            );
        }
        for (name, value) in values {
            let field = schema.effective_feature(&left.class, name)?.feature;
            let Some(lvalue) = selected(states.map(|(l, _)| l), left, name, value)? else {
                continue;
            };
            let expected_value = if field.reference {
                mapped(&lvalue, &identities)?
            } else {
                lvalue.into_owned()
            };
            let actual_value = states
                .map_or_else(|| right.values.get(name), |(_, r)| r.value(right, name))
                .map(|value| selected(states.map(|(_, r)| r), right, name, value))
                .transpose()?
                .flatten();
            if actual_value.as_deref() != Some(&expected_value) {
                return Err(format!(
                    "Generated text does not reproduce stored state for {before}.{name}: expected {expected_value}, got {:?}",
                    actual_value.as_deref()
                ));
            }
        }
    }
    Ok(identities)
}

fn targets(value: &Value) -> Result<Vec<&String>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(id) => Ok(vec![id]),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::String(id) => Ok(id),
                _ => Err("Containment collection contains a non-ID value".into()),
            })
            .collect(),
        _ => Err("Containment value is not an ID or ID collection".into()),
    }
}

fn mapped(value: &Value, identities: &BTreeMap<String, String>) -> Result<Value, String> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::String(id) => identities
            .get(id)
            .cloned()
            .map(Value::String)
            .ok_or_else(|| "Canonical references require a closed model graph".into()),
        Value::Array(items) => items.iter().map(|v| mapped(v, identities)).collect(),
        _ => Err("Reference value has an unsupported representation".into()),
    }
}
