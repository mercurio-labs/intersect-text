//! Surviving syntax keeps its original grammar and semantic owner.
use super::*;
pub(super) struct Relocation<'a> {
    pub uri: &'a str,
    pub source: TextRange,
    pub destination: TextRange,
}

pub(super) fn verify(
    before: &[DocumentSnapshot],
    after: &[DocumentSnapshot],
    patches: &[SourcePatch],
    removed: &BTreeSet<&str>,
    relocation: Option<Relocation<'_>>,
    replacements: bool,
) -> Result<(), String> {
    for original in before {
        let uri = &original.document().source().uri;
        let updated = snapshot_for(after, uri)?;
        let mut tokens = BTreeMap::new();
        for (index, node) in updated.document().nodes().iter().enumerate() {
            if node.kind != NodeKind::Leaf || node.data.hidden || node.range.start == node.range.end
            {
                continue;
            }
            let range = updated
                .document()
                .utf8_range(node.range)?
                .ok_or("Updated token splits a Unicode scalar")?;
            if tokens
                .insert(
                    (range.start, range.end),
                    (node.data.origin, semantic_id(updated, NodeId(index))?),
                )
                .is_some()
            {
                return Err("Ambiguous updated source token range".into());
            }
        }
        for (index, node) in original.document().nodes().iter().enumerate() {
            if node.kind != NodeKind::Leaf || node.data.hidden || node.range.start == node.range.end
            {
                continue;
            }
            let owner = semantic_id(original, NodeId(index))?;
            if owner.is_some_and(|id| removed.contains(id)) {
                continue;
            }
            let old = original
                .document()
                .utf8_range(node.range)?
                .ok_or("Original token splits a Unicode scalar")?;
            if replacements && patches.iter().any(|p| &p.uri == uri && p.range == old) {
                continue;
            }
            let range = if let Some(movement) = relocation
                .as_ref()
                .filter(|m| m.uri == uri && m.source.start <= old.start && old.end <= m.source.end)
            {
                TextRange {
                    start: movement.destination.start + old.start - movement.source.start,
                    end: movement.destination.start + old.end - movement.source.start,
                }
            } else {
                shifted(old, uri, patches)?
            };
            if tokens.get(&(range.start, range.end)) != Some(&(node.data.origin, owner)) {
                return Err(
                    "Source edit changed an existing token's grammar or semantic identity".into(),
                );
            }
        }
    }
    Ok(())
}
fn shifted(range: TextRange, uri: &str, patches: &[SourcePatch]) -> Result<TextRange, String> {
    let mut offset = 0i128;
    for patch in patches.iter().filter(|p| p.uri == uri) {
        if patch.range.end <= range.start {
            offset +=
                patch.replacement.len() as i128 - (patch.range.end - patch.range.start) as i128;
        } else if patch.range.start < range.end {
            return Err("Source edit unexpectedly overlaps a surviving token".into());
        }
    }
    Ok(TextRange {
        start: usize::try_from(range.start as i128 + offset)
            .map_err(|_| "Token offset overflow")?,
        end: usize::try_from(range.end as i128 + offset).map_err(|_| "Token offset overflow")?,
    })
}
fn semantic_id(snapshot: &DocumentSnapshot, node: NodeId) -> Result<Option<&str>, String> {
    snapshot
        .semantic_object(node)?
        .map(|i| {
            snapshot
                .semantic_objects()
                .get(i)
                .map(|e| e.id.as_str())
                .ok_or_else(|| "Unknown semantic token owner".to_string())
        })
        .transpose()
}
