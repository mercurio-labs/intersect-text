//! One new leaf declaration, appended through language-owned placement policy.
use super::*;
use crate::serialization::{SerializationLimits, TerminalSpelling};
use intersect_model::core::text_document::NodeHandle;

/// A language-owned parser rule for an existing owner's containment collection.
#[derive(Clone, Copy)]
pub struct AppendTarget<'a> {
    pub owner: &'a str,
    pub feature: &'a str,
    pub rule: &'a str,
}

pub struct AppendPlacement<'a> {
    pub snapshot: &'a DocumentSnapshot,
    pub owner: &'a Element,
    pub feature: &'a str,
    pub element: &'a Element,
    pub declaration: &'a str,
}

/// Insert immediately before an actual owner keyword. The checked handle is
/// source/revision bound; arbitrary byte addresses are not accepted.
pub struct SourceInsertion {
    pub before: NodeHandle,
    pub text: String,
}

impl SourceEditSession {
    pub fn plan_append_with(
        &self,
        candidate: &Model,
        target: AppendTarget<'_>,
        render_limits: SerializationLimits,
        spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
        placement: impl FnOnce(AppendPlacement<'_>) -> Result<SourceInsertion, String>,
    ) -> Result<PreparedSourceEdit, String> {
        self.check_context(None)?;
        let (owner, element) = self.append_change(candidate, target)?;
        let changed = BTreeSet::from([owner.resource.as_str()]);
        check_candidate_resources(self.model(), candidate, &changed)?;
        let declaration = crate::serialization::render_fragment(
            self.language.package(),
            candidate,
            target.rule,
            &element.id,
            render_limits,
            spelling,
        )?;
        let snapshot = self.snapshot(&owner.resource)?;
        let insertion = placement(AppendPlacement {
            snapshot,
            owner,
            feature: target.feature,
            element,
            declaration: &declaration,
        })?;
        if insertion.text.is_empty() {
            return Err("Containment insertion text is empty".into());
        }
        let node = snapshot.document().node(&insertion.before)?;
        if node.kind != NodeKind::Leaf
            || node.data.hidden
            || node.range.start == node.range.end
            || !node
                .data
                .origin
                .and_then(|i| snapshot.origins().get(i))
                .is_some_and(|origin| matches!(origin.kind, crate::document::OriginKind::Keyword))
        {
            return Err("Containment insertion anchor must be an actual owner keyword".into());
        }
        let linked = snapshot
            .linked_object(
                &insertion.before,
                self.model(),
                &self.language.package().schema,
            )?
            .ok_or("Insertion anchor has no semantic owner")?;
        if linked.id != owner.id {
            return Err("Insertion anchor belongs to a different semantic owner".into());
        }
        let at = snapshot
            .document()
            .utf8_range(node.range)?
            .ok_or("Insertion anchor splits a Unicode scalar")?
            .start;
        let patch = SourcePatch {
            uri: owner.resource.clone(),
            owner: owner.id.clone(),
            feature: target.feature.into(),
            range: TextRange { start: at, end: at },
            output_range: TextRange { start: at, end: at },
            original: String::new(),
            replacement: insertion.text,
        };
        let prepared = self.verify_patches(candidate, vec![patch], &changed, false)?;
        preserve::verify(
            &self.snapshots,
            &prepared.output.analysis.snapshots,
            &prepared.output.patches,
            &BTreeSet::new(),
            None,
            false,
        )?;
        Ok(prepared)
    }

    fn append_change<'a>(
        &'a self,
        candidate: &'a Model,
        target: AppendTarget<'_>,
    ) -> Result<(&'a Element, &'a Element), String> {
        let baseline = self.model();
        if candidate.schema_revision != baseline.schema_revision
            || candidate.elements.len()
                != baseline
                    .elements
                    .len()
                    .checked_add(1)
                    .ok_or("Element count overflow")?
        {
            return Err(
                "Containment append requires exactly one new object in the same schema".into(),
            );
        }
        let schema = &self.language.package().schema;
        candidate.validate(schema)?;
        let owner = baseline
            .elements
            .iter()
            .find(|e| e.id == target.owner)
            .ok_or("Unknown append owner")?;
        let field = schema
            .effective_feature(&owner.class, target.feature)?
            .feature;
        if !field.reference
            || !field.containment
            || field.upper == 1
            || !field.ordered
            || field.derived
            || field.transient
            || field.volatile
            || !field.changeable
        {
            return Err("Append requires a stored ordered containment collection".into());
        }
        let element = candidate.elements.last().ok_or("Missing appended object")?;
        if baseline.elements.iter().any(|e| e.id == element.id)
            || element.resource != owner.resource
            || element.start != 0
            || element.end != 0
        {
            return Err("Append requires a new source-free object in the owner's resource".into());
        }
        for (name, field) in schema.features(&element.class)? {
            if field.containment
                && element
                    .values
                    .get(&name)
                    .is_some_and(|v| !v.is_null() && !v.as_array().is_some_and(Vec::is_empty))
            {
                return Err("Containment append supports new leaf objects only".into());
            }
        }
        for (before, after) in baseline.elements.iter().zip(&candidate.elements) {
            if before.id != after.id
                || before.class != after.class
                || before.type_identity != after.type_identity
                || before.resource != after.resource
                || before.start != after.start
                || before.end != after.end
                || before.initialization != after.initialization
                || before.assignment_state != after.assignment_state
                || !before.values.keys().eq(after.values.keys())
            {
                return Err("Append changed existing object identity, order or metadata".into());
            }
            for (name, value) in &before.values {
                let next = after.values.get(name).ok_or("Missing candidate value")?;
                if before.id == owner.id && name == target.feature {
                    let old = value
                        .as_array()
                        .ok_or("Append baseline collection is not an array")?;
                    let new = next
                        .as_array()
                        .ok_or("Append candidate collection is not an array")?;
                    if new.len()
                        != old
                            .len()
                            .checked_add(1)
                            .ok_or("Collection length overflow")?
                        || !new.starts_with(old)
                        || new.last().and_then(Value::as_str) != Some(&element.id)
                    {
                        return Err(
                            "Candidate must append the new object without moving existing members"
                                .into(),
                        );
                    }
                } else if value != next {
                    return Err("Append also changed an unrelated stored value".into());
                }
            }
        }
        Ok((owner, element))
    }
}
