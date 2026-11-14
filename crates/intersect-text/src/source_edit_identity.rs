//! Stable model IDs over fresh authoritative parse results. No syntax is guessed.
use super::*;
use std::collections::BTreeMap;

pub(super) fn reconcile_using(
    language: &DocumentLanguage,
    candidate: &Model,
    evaluation: &mut Evaluation,
    snapshots: &mut [DocumentSnapshot],
    services: Option<&crate::document::DocumentServices<'_, '_>>,
    steps: usize,
) -> Result<BTreeMap<String, String>, String> {
    let package = language.package();
    let schema = &package.schema;
    // The exact route retains old source associations, including unchanged
    // transient syntax. Otherwise compare only the declared persistence view.
    let mut projection = None;
    let identities =
        match crate::serialization::correspondence(candidate, &evaluation.model, schema) {
            Ok(identities) => identities,
            Err(_) => {
                let left = crate::serialization::SerializationState::new(
                    package, candidate, services, steps,
                )?;
                let right = crate::serialization::SerializationState::new(
                    package,
                    &evaluation.model,
                    services,
                    steps.saturating_sub(left.steps),
                )?;
                let identities = crate::serialization::persistent_correspondence(
                    candidate,
                    &evaluation.model,
                    schema,
                    &left,
                    &right,
                )?;
                projection = Some(left);
                identities
            }
        };
    let inverse: BTreeMap<_, _> = identities
        .iter()
        .map(|(model, parsed)| (parsed.clone(), model.clone()))
        .collect();
    evaluation
        .model
        .elements
        .retain(|element| inverse.contains_key(&element.id));
    for element in &mut evaluation.model.elements {
        let stable = mapped(&element.id, &inverse)?;
        for (name, value) in &mut element.values {
            if let Some(state) = &projection {
                let member = state.member(stable, name)?;
                if member.whole || !member.omitted.is_empty() {
                    continue;
                }
            }
            if schema
                .effective_feature(&element.class, name)?
                .feature
                .reference
            {
                *value = references(value, &inverse)?;
            }
        }
        if let Some(receipt) = &mut element.initialization {
            receipt.object = mapped(&receipt.object, &inverse)?.into();
        }
        element.id = mapped(&element.id, &inverse)?.into();
        if let Some(state) = &projection {
            let original = candidate
                .elements
                .iter()
                .find(|original| original.id == element.id)
                .ok_or("Missing transient-state owner")?;
            for (name, value) in &original.values {
                let member = state.member(&element.id, name)?;
                if member.whole || !member.omitted.is_empty() {
                    element.values.insert(name.clone(), value.clone());
                }
            }
            element.assignment_state = original.assignment_state.clone();
        }
    }
    for resource in evaluation.model.resources.values_mut() {
        resource.roots = Arc::new(
            resource
                .roots
                .iter()
                .map(|id| mapped(id, &inverse).map(str::to_owned))
                .collect::<Result<_, _>>()?,
        );
    }
    for reference in &mut evaluation.references {
        reference.target = mapped(&reference.target, &inverse)?.into();
    }
    // Derived outputs are recomputed after final resource reconciliation by the
    // shared semantic service, so ID-sensitive values are never guessed here.
    // A model's element vector order is storage order, independent of parse order.
    let mut elements: BTreeMap<_, _> = std::mem::take(&mut evaluation.model.elements)
        .into_iter()
        .map(|e| (e.id.clone(), e))
        .collect();
    evaluation.model.elements = candidate
        .elements
        .iter()
        .map(|e| {
            if projection
                .as_ref()
                .is_some_and(|state| !state.retained.contains(&e.id))
            {
                Ok(e.clone())
            } else {
                elements
                    .remove(&e.id)
                    .ok_or_else(|| "Missing reconciled model object".to_string())
            }
        })
        .collect::<Result<_, _>>()?;
    for snapshot in snapshots {
        language.reconcile_snapshot_ids(snapshot, &inverse)?;
    }
    Ok(identities)
}

fn mapped<'a>(id: &str, inverse: &'a BTreeMap<String, String>) -> Result<&'a str, String> {
    inverse
        .get(id)
        .map(String::as_str)
        .ok_or_else(|| "Missing source/model identity correspondence".into())
}

fn references(value: &Value, inverse: &BTreeMap<String, String>) -> Result<Value, String> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::String(id) => Ok(Value::String(mapped(id, inverse)?.into())),
        Value::Array(items) => items.iter().map(|v| references(v, inverse)).collect(),
        _ => {
            Err("Unsupported reference representation during source identity reconciliation".into())
        }
    }
}
