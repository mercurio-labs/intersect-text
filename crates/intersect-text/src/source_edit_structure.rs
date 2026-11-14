//! Checked leaf deletion and same-resource movement over existing core edits.
use super::*;
use intersect_model::core::text_document::NodeHandle;
use intersect_model::core::{
    frozen::FrozenStore,
    query::QueryBudget,
    storage::{
        self, ContainmentDestination, DeleteRequest, DeleteSelection, IncomingReferences,
        ReparentRequest, StructuralContext,
    },
};
pub struct MovePlacement<'a> {
    pub snapshot: &'a DocumentSnapshot,
    pub owner: &'a Element,
    pub element: &'a Element,
    pub declaration: &'a str,
    pub before: Option<NodeHandle>,
}

impl SourceEditSession {
    pub fn declaration(&self, id: &str) -> Result<NodeHandle, String> {
        let object = self
            .model()
            .elements
            .iter()
            .find(|e| e.id == id)
            .ok_or("Unknown declaration object")?;
        for (name, field) in self.package().schema.features(&object.class)? {
            if field.containment
                && object
                    .values
                    .get(&name)
                    .is_some_and(|v| !v.is_null() && !v.as_array().is_some_and(Vec::is_empty))
            {
                return Err("Structural source editing currently requires a leaf object".into());
            }
        }
        let snapshot = self.snapshot(&object.resource)?;
        let mut found: Option<NodeId> = None;
        for (index, node) in snapshot.document().nodes().iter().enumerate() {
            if node.kind != NodeKind::Composite || !node.data.direct_semantic_association {
                continue;
            }
            if node
                .data
                .semantic_object
                .and_then(|i| snapshot.semantic_objects().get(i))
                .is_some_and(|e| e.id == id)
            {
                match found {
                    None => found = Some(NodeId(index)),
                    Some(old) => {
                        let old_range = snapshot.document().nodes()[old.0].range;
                        if node.range.start <= old_range.start && old_range.end <= node.range.end {
                            found = Some(NodeId(index));
                        } else if !(old_range.start <= node.range.start
                            && node.range.end <= old_range.end)
                        {
                            return Err("Ambiguous declaration ranges".into());
                        }
                    }
                }
            }
        }
        snapshot
            .document()
            .handle(found.ok_or("Object has no direct declaration node")?)
    }

    pub fn plan_delete(&self, id: &str) -> Result<PreparedSourceEdit, String> {
        self.check_context(None)?;
        let object = self
            .model()
            .elements
            .iter()
            .find(|e| e.id == id)
            .ok_or("Unknown deletion target")?;
        let handle = self.declaration(id)?;
        let snapshot = self.snapshot(&object.resource)?;
        let range = snapshot
            .document()
            .utf8_range(snapshot.document().node(&handle)?.range)?
            .ok_or("Declaration splits a Unicode scalar")?;
        let mut store = FrozenStore::from_materialized(&self.package().schema, self.model())?;
        StructuralContext {
            schema: &self.package().schema,
            conversions: None,
        }
        .delete(
            &mut store,
            DeleteRequest {
                roots: &[id],
                selection: DeleteSelection::Objects,
                incoming: IncomingReferences::Reject,
            },
            &mut QueryBudget::new(self.limits.steps_per_resource.unwrap_or(1_000_000)),
        )?;
        let candidate = storage::export(&store)?;
        let original = &snapshot.document().text()[range.start..range.end];
        let mut replacement = String::new();
        let mut cursor = range.start;
        for node in snapshot
            .document()
            .nodes()
            .iter()
            .filter(|n| n.kind == NodeKind::Leaf && !n.data.hidden)
        {
            let token = snapshot
                .document()
                .utf8_range(node.range)?
                .ok_or("Deletion token splits Unicode")?;
            if token.start < range.start || token.end > range.end {
                continue;
            }
            replacement.push_str(&snapshot.document().text()[cursor..token.start]);
            cursor = token.end;
        }
        replacement.push_str(&snapshot.document().text()[cursor..range.end]);
        let patch = SourcePatch {
            uri: object.resource.clone(),
            owner: id.into(),
            feature: String::new(),
            range,
            output_range: range,
            original: original.into(),
            replacement,
        };
        let prepared = self.verify_patches(
            &candidate,
            vec![patch],
            &BTreeSet::from([object.resource.as_str()]),
            false,
        )?;
        preserve::verify(
            &self.snapshots,
            &prepared.output.analysis.snapshots,
            &prepared.output.patches,
            &BTreeSet::from([id]),
            None,
            false,
        )?;
        Ok(prepared)
    }

    pub fn plan_move_with(
        &self,
        id: &str,
        destination: ContainmentDestination<'_>,
        placement: impl FnOnce(MovePlacement<'_>) -> Result<SourceInsertion, String>,
    ) -> Result<PreparedSourceEdit, String> {
        self.check_context(None)?;
        let object = self
            .model()
            .elements
            .iter()
            .find(|e| e.id == id)
            .ok_or("Unknown move target")?;
        let owner = self
            .model()
            .elements
            .iter()
            .find(|e| e.id == destination.owner)
            .ok_or("Unknown move owner")?;
        if owner.resource != object.resource {
            return Err("Source move profile requires the same logical resource".into());
        }
        let handle = self.declaration(id)?;
        let snapshot = self.snapshot(&object.resource)?;
        let range = snapshot
            .document()
            .utf8_range(snapshot.document().node(&handle)?.range)?
            .ok_or("Declaration splits Unicode")?;
        let declaration = &snapshot.document().text()[range.start..range.end];
        let mut store = FrozenStore::from_materialized(&self.package().schema, self.model())?;
        let moved = StructuralContext {
            schema: &self.package().schema,
            conversions: None,
        }
        .reparent(
            &mut store,
            ReparentRequest {
                object: id,
                destination: Some(destination),
            },
            &mut QueryBudget::new(self.limits.steps_per_resource.unwrap_or(1_000_000)),
        )?;
        if !moved {
            return self.plan_with(self.model(), |_| Err("No-op requested spelling".into()));
        }
        let candidate = storage::export(&store)?;
        let target = candidate
            .elements
            .iter()
            .find(|e| e.id == owner.id)
            .ok_or("Missing moved owner")?;
        let values = target
            .values
            .get(destination.feature)
            .and_then(Value::as_array)
            .ok_or("Move requires an ordered collection")?;
        let index = values
            .iter()
            .position(|v| v.as_str() == Some(id))
            .ok_or("Moved object absent from destination")?;
        let before = values
            .get(index + 1)
            .and_then(Value::as_str)
            .map(|id| self.declaration(id))
            .transpose()?;
        let insertion = placement(MovePlacement {
            snapshot,
            owner,
            element: object,
            declaration,
            before,
        })?;
        let node = snapshot.document().node(&insertion.before)?;
        let at = snapshot
            .document()
            .utf8_range(node.range)?
            .ok_or("Move anchor splits Unicode")?
            .start;
        if range.start <= at && at <= range.end {
            return Err("Move anchor lies inside its declaration".into());
        }
        let prefix = insertion
            .text
            .find(declaration)
            .ok_or("Move placement must retain the exact declaration")?;
        if !insertion.text[..prefix].chars().all(char::is_whitespace)
            || !insertion.text[prefix + declaration.len()..]
                .chars()
                .all(char::is_whitespace)
        {
            return Err(
                "Move placement may add whitespace only around the exact declaration".into(),
            );
        }
        let mut patches = vec![
            SourcePatch {
                uri: object.resource.clone(),
                owner: id.into(),
                feature: String::new(),
                range,
                output_range: range,
                original: declaration.into(),
                replacement: String::new(),
            },
            SourcePatch {
                uri: object.resource.clone(),
                owner: owner.id.clone(),
                feature: destination.feature.into(),
                range: TextRange { start: at, end: at },
                output_range: TextRange { start: at, end: at },
                original: String::new(),
                replacement: insertion.text,
            },
        ];
        patches.sort_by_key(|p| p.range.start);
        let prepared = self.verify_patches(
            &candidate,
            patches,
            &BTreeSet::from([object.resource.as_str()]),
            false,
        )?;
        let inserted = prepared
            .output
            .patches
            .iter()
            .find(|p| p.range.start == p.range.end)
            .ok_or("Missing move insertion")?;
        let relocated = TextRange {
            start: inserted.output_range.start + prefix,
            end: inserted.output_range.start + prefix + declaration.len(),
        };
        preserve::verify(
            &self.snapshots,
            &prepared.output.analysis.snapshots,
            &prepared.output.patches,
            &BTreeSet::new(),
            Some(preserve::Relocation {
                uri: &object.resource,
                source: range,
                destination: relocated,
            }),
            false,
        )?;
        Ok(prepared)
    }
}
