//! Checked nonderived scalar assignment routing for explicitly selected native
//! language gateways. This selects metadata; existing callers own the one write.
use super::DraftCapability;
use serde_json::Value;
use {
    crate::preparation::PreparedLanguageDefinition, intersect_model::core::query::QueryBudget,
    intersect_model::definitions::schema::EffectiveFeature,
    intersect_model::definitions::schema::MemberIdentity,
    intersect_model::storage::DraftDisposition, intersect_model::storage::ModelRead,
    intersect_model::storage::StorageResult,
};
fn scalar(field: &intersect_model::definitions::schema::Feature) -> bool {
    !field.reference
        && !field.containment
        && !field.derived
        && field.changeable
        && !field.unsettable
        && field.opposite.is_none()
        && field.upper == 1
}
fn source(member: &MemberIdentity, definition: &PreparedLanguageDefinition<'_>) -> bool {
    matches!(member, MemberIdentity::Source { declaration }
        if definition.sources().sources.iter().any(|s| s.id == declaration.source))
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn target<C: DraftCapability>(
    definition: &PreparedLanguageDefinition<'_>,
    capability: &C,
    store: &dyn ModelRead,
    id: &str,
    feature: &str,
    append: bool,
    value: &Value,
    budget: &mut QueryBudget,
) -> StorageResult<Option<String>> {
    if !capability.has_delegated_assignments() {
        return Ok(None);
    }
    budget.charge()?;
    let schema = definition.schema();
    let mut header = None;
    if !store.inspect(id, &mut |row| {
        budget.charge()?;
        if row.id() != id || header.is_some() {
            return Err("Delegated assignment actual-row selection is ambiguous".into());
        }
        if row.type_identity() != schema.identity(row.class()) {
            return Err("Delegated assignment actual receiver identity changed".into());
        }
        header = Some(row.class().to_owned());
        Ok(())
    })? {
        return Err("Missing delegated assignment actual receiver".into());
    }
    let actual = header.ok_or("Missing delegated assignment receiver header")?;
    budget.charge()?;
    let original = schema.effective_feature(&actual, feature)?;
    let Some(target_id) =
        capability.delegated_assignment_target(&actual, &original.identity, append, budget)?
    else {
        return Ok(None);
    };
    for (held, rows) in [
        (
            capability.original_sources(),
            &definition.original_sources().sources,
        ),
        (
            capability.converted_sources(),
            &definition.sources().sources,
        ),
    ] {
        budget.charge()?;
        if held.len() != rows.len() {
            return Err("Delegated assignment capability source owner differs".into());
        }
        for (held, row) in held.iter().zip(rows) {
            budget.charge()?;
            if held != &row.id {
                return Err("Delegated assignment capability source owner differs".into());
            }
        }
    }
    let declaration = schema
        .declarations
        .get(&actual)
        .ok_or("Missing delegated assignment receiver source declaration")?;
    budget.charge()?;
    if declaration.name != actual
        || declaration.path.is_empty()
        || !definition
            .sources()
            .sources
            .iter()
            .any(|s| s.id == declaration.source)
    {
        return Err("Foreign delegated assignment receiver source declaration".into());
    }
    if append || !scalar(original.feature) || !source(&original.identity, definition) {
        return Err(
            "Delegated assignment requires a canonical singular writable source attribute".into(),
        );
    }
    let mut selected: Option<EffectiveFeature<'_>> = None;
    for field in schema.effective_features(&actual)? {
        budget.charge()?;
        if &field.identity == target_id {
            if selected.is_some() {
                return Err("Ambiguous delegated assignment target identity".into());
            }
            selected = Some(field);
        }
    }
    let selected = selected.ok_or("Foreign delegated assignment target identity")?;
    if selected.identity == original.identity
        || !scalar(selected.feature)
        || selected.feature.volatile
        || selected.feature.transient
        || !source(&selected.identity, definition)
        || selected.feature.target != original.feature.target
        || selected.feature.datatype != original.feature.datatype
        || selected.feature.lower != original.feature.lower
    {
        return Err(
            "Delegated assignment target differs from canonical writable scalar shape".into(),
        );
    }
    let mut classes = capability.contracts().iter().filter(|c| c.class == actual);
    let class = classes
        .next()
        .ok_or("Missing delegated assignment actual constructor contract")?;
    if classes.next().is_some() {
        return Err("Ambiguous delegated assignment actual constructor contract".into());
    }
    budget.charge()?;
    if class.version != 1
        || class.schema_revision != schema.revision
        || class.class_identity != schema.identity(&actual)
        || class.sources != capability.converted_sources()
    {
        return Err("Delegated assignment constructor/source identity differs".into());
    }
    for (member, disposition) in [
        (&original, DraftDisposition::DelegatedAbsent),
        (&selected, DraftDisposition::StoredDefault),
    ] {
        budget.charge()?;
        let mut entries = class
            .members
            .iter()
            .filter(|m| m.name == member.feature.name);
        let entry = entries
            .next()
            .ok_or("Missing delegated assignment constructor member")?;
        if entries.next().is_some()
            || entry.identity != member.identity
            || entry.disposition != disposition
        {
            return Err("Delegated assignment constructor disposition differs".into());
        }
    }
    budget.charge()?;
    let mut verified = false;
    let found = store.inspect(id, &mut |row| {
        budget.charge()?;
        if row.id() != id || verified {
            return Err("Delegated assignment second actual-row selection is ambiguous".into());
        }
        verified = true;
        if row.class() != actual || row.type_identity() != class.class_identity {
            return Err("Delegated assignment receiver changed during selection".into());
        }
        if row.value(feature).is_some() {
            return Err("Raw delegated assignment alias shadow is forbidden".into());
        }
        if row.value(&selected.feature.name).is_none() {
            return Err("Delegated assignment stored target carrier is absent".into());
        }
        Ok(())
    })?;
    if !found || !verified {
        return Err("Delegated assignment receiver disappeared during selection".into());
    }
    for member in [&original, &selected] {
        budget.charge()?;
        if !capability.permits_assignment(&actual, &member.identity, false) {
            return Err("Unsupported delegated assignment source or target capability".into());
        }
        if C::NEEDS_VALUE_ADMISSION {
            capability.validate_assignment_value(
                &actual,
                &member.identity,
                false,
                value,
                budget,
            )?;
        }
    }
    budget.charge()?;
    Ok(Some(selected.feature.name.clone()))
}
