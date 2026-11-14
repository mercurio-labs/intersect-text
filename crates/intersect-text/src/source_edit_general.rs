//! General verified model -> existing text. Syntax matching uses retained
//! semantic/grammar identities, never a language-specific delimiter or parser.
use super::*;
use crate::serialization::{SerializationLimits, TerminalSpelling, default_spelling};

impl SourceEditSession {
    /// Default grammar-driven preservation. Custom language spellings use
    /// `plan_general_with`; every path reparses and verifies the complete graph.
    pub fn plan(
        &self,
        candidate: &Model,
        limits: SerializationLimits,
    ) -> Result<PreparedSourceEdit, String> {
        if let Some(services) = &self.owned_services {
            return services.with_context(&self.language, limits.steps, |context| {
                self.plan_general_using(candidate, limits, Some(context), |value| {
                    services.spelling(self.package(), candidate, value)
                })
            });
        }
        self.plan_general_with(candidate, limits, |value| {
            default_spelling(self.package(), value)
        })
    }

    pub fn plan_general_with(
        &self,
        candidate: &Model,
        limits: SerializationLimits,
        mut spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
    ) -> Result<PreparedSourceEdit, String> {
        if let Some(services) = &self.owned_services {
            return services.with_context(&self.language, limits.steps, |context| {
                self.plan_general_using(candidate, limits, Some(context), &mut spelling)
            });
        }
        self.plan_general_using(candidate, limits, None, &mut spelling)
    }

    pub fn plan_with_context(
        &self,
        candidate: &Model,
        limits: SerializationLimits,
        context: &mut SourceEditContext<'_, '_>,
        spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
    ) -> Result<PreparedSourceEdit, String> {
        self.plan_general_using(candidate, limits, Some(context), spelling)
    }

    pub fn plan_using_context(
        &self,
        candidate: &Model,
        limits: SerializationLimits,
        context: &mut SourceEditContext<'_, '_>,
    ) -> Result<PreparedSourceEdit, String> {
        self.check_context(Some(context))?;
        if context.services.linking.is_some() {
            return Err("A custom scope requires an explicit inverse spelling policy".into());
        }
        let conversions = context.services.conversions;
        self.plan_with_context(candidate, limits, context, |value| {
            crate::serialization::default_spelling_with_conversions(
                self.package(),
                value,
                conversions,
            )
        })
    }

    fn plan_general_using(
        &self,
        candidate: &Model,
        limits: SerializationLimits,
        mut context: Option<&mut SourceEditContext<'_, '_>>,
        mut spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
    ) -> Result<PreparedSourceEdit, String> {
        self.check_context(context.as_deref())?;
        check_candidate(self, candidate, context.as_deref())?;

        if candidate == self.model() {
            return self.verify_patches_using(candidate, vec![], &BTreeSet::new(), false, context);
        }
        let mut work = limits.steps;
        let hints = self
            .language
            .serialization_hints(&self.snapshots, self.model(), &mut work)?;
        let custom_unassigned = context
            .as_ref()
            .is_some_and(|context| context.services.unassigned.is_some());
        let custom_scope = context
            .as_ref()
            .is_some_and(|context| context.services.linking.is_some());
        let mut generate = |retain_custom| {
            crate::serialization::serialize(
                &self.language,
                self.limits,
                limits,
                candidate,
                Some(&hints),
                context.as_deref_mut().map(|context| &mut context.services),
                |value| {
                    if (!custom_scope || retain_custom || value.target.is_none())
                        && let Some(text) = retained_spelling(self, &value, custom_scope)?
                    {
                        return Ok(text);
                    }
                    spelling(value)
                },
            )
        };
        // First keep source-bound custom references. Their old spelling is only
        // a candidate: changed visibility must still reproduce every target.
        // If it fails, the installed inverse policy supplies fresh spellings.
        let mut generated = match generate(true) {
            Ok(output) => output,
            Err(_) if custom_scope => generate(false)?,
            Err(error) => return Err(error),
        };
        // Scalar edits can retain authored cross-feature order without imposing
        // the canonical order. Failure means a broader syntax edit is necessary.
        if !custom_unassigned
            && let Ok(prepared) =
                self.plan_scalar_using(candidate, context.as_deref_mut(), |value| {
                    let snapshot = snapshot_for(
                        &generated.analysis.snapshots,
                        &candidate
                            .elements
                            .iter()
                            .find(|e| e.id == value.owner)
                            .ok_or("Missing scalar owner")?
                            .resource,
                    )?;
                    let parsed = generated
                        .identities
                        .get(value.owner)
                        .ok_or("Missing generated identity")?;
                    let range = occurrence(snapshot, parsed, value.feature)?;
                    Ok(snapshot.document().text()[range.start..range.end].into())
                })
        {
            return Ok(prepared);
        }
        let mut evaluation = accepted(&mut generated.analysis)?;
        identity::reconcile_using(
            &self.language,
            candidate,
            &mut evaluation,
            &mut generated.analysis.snapshots,
            context.as_ref().map(|context| &context.services),
            limits.steps,
        )?;
        let mut policy = MergePolicy {
            output_bytes: limits.output_bytes,
            reuse_unassigned: !custom_unassigned,
            keep_unchanged: !custom_scope && !custom_unassigned,
            prefer_empty: true,
            filter_trivia: false,
        };
        let sources = merge_sources(
            self,
            &generated.analysis.snapshots,
            &evaluation.model,
            candidate,
            policy,
            &mut work,
        )?;
        match self.verify_general_sources(candidate, sources, context.as_deref_mut()) {
            Ok(prepared) => Ok(prepared),
            Err(_) => {
                // Empty authored gaps are preferred, but a changed token can
                // require a separator. Retry with verified generated gaps.
                policy.prefer_empty = false;
                let sources = merge_sources(
                    self,
                    &generated.analysis.snapshots,
                    &evaluation.model,
                    candidate,
                    policy,
                    &mut work,
                )?;
                match self.verify_general_sources(candidate, sources, context.as_deref_mut()) {
                    Ok(prepared) => Ok(prepared),
                    Err(_) => {
                        // A moved object can enter a different hidden() context.
                        // Reuse legal trivia and move comments to the nearest
                        // legal boundary before sacrificing incompatible layout.
                        policy.filter_trivia = true;
                        let sources = merge_sources(
                            self,
                            &generated.analysis.snapshots,
                            &evaluation.model,
                            candidate,
                            policy,
                            &mut work,
                        )?;
                        self.verify_general_sources(candidate, sources, context)
                    }
                }
            }
        }
    }
    fn verify_general_sources(
        &self,
        candidate: &Model,
        sources: Vec<Source>,
        context: Option<&mut SourceEditContext<'_, '_>>,
    ) -> Result<PreparedSourceEdit, String> {
        let mut patches = Vec::new();
        let mut changed = BTreeSet::new();
        let uris: BTreeSet<_> = self
            .model()
            .resources
            .keys()
            .chain(candidate.resources.keys())
            .map(String::as_str)
            .collect();
        for uri in uris {
            let before = self
                .sources
                .iter()
                .find(|s| s.uri == uri)
                .map(|s| s.text.as_str());
            let after = sources
                .iter()
                .find(|s| s.uri == uri)
                .map(|s| s.text.as_str());
            if let Some(patch) = difference(uri, before.unwrap_or(""), after.unwrap_or("")) {
                patches.push(patch);
            }
            if before != after || self.model().resources.get(uri) != candidate.resources.get(uri) {
                changed.insert(uri);
            }
        }
        self.verify_sources_using(candidate, sources, patches, &changed, false, context)
    }
}

fn unchanged_resource(
    session: &SourceEditSession,
    candidate: &Model,
    uri: &str,
) -> Result<bool, String> {
    let Some(before) = session.model().resources.get(uri) else {
        return Ok(false);
    };
    let Some(after) = candidate.resources.get(uri) else {
        return Ok(false);
    };
    if before.roots != after.roots {
        return Ok(false);
    }
    let old: BTreeMap<_, _> = session
        .model()
        .elements
        .iter()
        .filter(|e| e.resource == uri)
        .map(|e| (&e.id, e))
        .collect();
    let new: BTreeMap<_, _> = candidate
        .elements
        .iter()
        .filter(|e| e.resource == uri)
        .map(|e| (&e.id, e))
        .collect();
    if old != new {
        return Ok(false);
    }
    for reference in session
        .evaluation
        .references
        .iter()
        .filter(|reference| reference.uri == uri)
    {
        let before = session
            .model()
            .elements
            .iter()
            .find(|e| e.id == reference.target)
            .ok_or("Missing original reference target")?;
        let Some(after) = candidate.elements.iter().find(|e| e.id == reference.target) else {
            return Ok(false);
        };
        if before.values.get("name") != after.values.get("name") {
            return Ok(false);
        }
    }
    Ok(true)
}

fn check_candidate(
    session: &SourceEditSession,
    candidate: &Model,
    context: Option<&SourceEditContext<'_, '_>>,
) -> Result<(), String> {
    match context.and_then(|context| context.services.conversions) {
        Some(registry) => {
            candidate.validate_with_conversions(&session.package().schema, registry)?
        }
        None => candidate.validate(&session.package().schema)?,
    }
    if candidate.schema_revision != session.model().schema_revision {
        return Err("General source editing requires the same schema".into());
    }
    for old in &session.model().elements {
        if let Some(next) = candidate.elements.iter().find(|e| e.id == old.id)
            && (old.class != next.class
                || old.type_identity != next.type_identity
                || old.initialization != next.initialization
                || old.start != next.start
                || old.end != next.end)
        {
            return Err("General source edit changed unrelated existing object metadata".into());
        }
    }
    for (uri, old) in &session.model().resources {
        let Some(next) = candidate.resources.get(uri) else {
            continue;
        };
        if next.uri != old.uri
            || next.load_state != old.load_state
            || next.diagnostics != old.diagnostics
            || next.content_revision < old.content_revision
            || (next.source.is_some() && next.source != old.source)
        {
            return Err("General source edit changed unrelated resource metadata".into());
        }
    }
    Ok(())
}

fn retained_spelling(
    session: &SourceEditSession,
    value: &TerminalSpelling<'_>,
    custom_scope: bool,
) -> Result<Option<String>, String> {
    let Some(before) = session
        .model()
        .elements
        .iter()
        .find(|e| e.id == value.owner.id)
    else {
        return Ok(None);
    };
    let Some(stored) = before.values.get(value.feature).or_else(|| {
        session
            .evaluation
            .derived
            .get(&before.id)
            .and_then(|values| values.get(value.feature))
    }) else {
        return Ok(None);
    };
    // A reference spelling can be reused only if its target's name is unchanged
    // under the explicit built-in unique-name scope. Custom inverse scopes own
    // this decision and receive the normal spelling request instead.
    if let Some(target) = value.target {
        let old = session.model().elements.iter().find(|e| e.id == target.id);
        let same = if custom_scope {
            old.is_some_and(|old| old.values == target.values && old.resource == target.resource)
        } else {
            session.package().semantics.scope == "unique_names"
                && old.is_some_and(|old| old.values.get("name") == target.values.get("name"))
        };
        if !same {
            return Ok(None);
        }
    }
    let index = if let Some(items) = stored.as_array() {
        if items.get(value.index) == Some(value.value) {
            Some(value.index)
        } else {
            items.iter().position(|v| v == value.value)
        }
    } else {
        (stored == value.value).then_some(0)
    };
    let unchanged = index.is_some();
    let index = index.unwrap_or(0);
    let snapshot = session.snapshot(&before.resource)?;
    let Some(feature) = snapshot
        .feature_nodes()
        .iter()
        .filter(|f| f.owner == before.id && f.feature == value.feature)
        .nth(index)
    else {
        return Ok(None);
    };
    let node = snapshot
        .document()
        .nodes()
        .get(feature.node.0)
        .ok_or("Missing retained feature node")?;
    let range = snapshot
        .document()
        .utf8_range(node.range)?
        .ok_or("Retained spelling splits Unicode")?;
    let original = &snapshot.document().text()[range.start..range.end];
    if unchanged {
        return Ok(Some(original.into()));
    }
    if value.terminal == "STRING"
        && original.starts_with('\'')
        && let Some(text) = value.value.as_str()
    {
        let mut quoted = String::from("'");
        for c in text.chars() {
            match c {
                '\'' => quoted.push_str("\\'"),
                '\\' => quoted.push_str("\\\\"),
                '\n' => quoted.push_str("\\n"),
                '\r' => quoted.push_str("\\r"),
                '\t' => quoted.push_str("\\t"),
                c => quoted.push(c),
            }
        }
        quoted.push('\'');
        return Ok(Some(quoted));
    }
    Ok(None)
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    scope: String,
    owner: Option<String>,
    origin: Option<usize>,
    occurrence: usize,
}
struct Token {
    key: Key,
    range: TextRange,
    assigned: bool,
    hidden: Option<Vec<String>>,
}
fn charge(work: &mut usize) -> Result<(), String> {
    *work = work
        .checked_sub(1)
        .ok_or("Source preservation work budget exceeded")?;
    Ok(())
}
fn tokens(
    language: &DocumentLanguage,
    snapshot: &DocumentSnapshot,
    model: &Model,
    schema: &intersect_model::definitions::schema::Schema,
    work: &mut usize,
) -> Result<Vec<Token>, String> {
    let mut counts = BTreeMap::new();
    let mut result = Vec::new();
    for (index, node) in snapshot.document().nodes().iter().enumerate() {
        charge(work)?;
        if node.kind != NodeKind::Leaf || node.data.hidden || node.range.start == node.range.end {
            continue;
        }
        let owner = snapshot.serialization_owner(model, schema, NodeId(index), work)?;
        let count = counts.entry((owner.clone(), node.data.origin)).or_insert(0);
        let key = Key {
            scope: if owner.is_none() {
                snapshot.document().source().uri.clone()
            } else {
                String::new()
            },
            owner,
            origin: node.data.origin,
            occurrence: *count,
        };
        *count += 1;
        let range = snapshot
            .document()
            .utf8_range(node.range)?
            .ok_or("Syntax token splits Unicode")?;
        let mut assigned = false;
        for feature in snapshot
            .feature_nodes()
            .iter()
            .filter(|f| Some(&f.owner) == key.owner.as_ref())
        {
            charge(work)?;
            let feature_range = snapshot
                .document()
                .nodes()
                .get(feature.node.0)
                .ok_or("Missing feature syntax")?
                .range;
            if feature_range.start <= node.range.start && node.range.end <= feature_range.end {
                assigned = true;
                break;
            }
        }
        result.push(Token {
            key,
            range,
            assigned,
            hidden: language
                .serialization_hidden(snapshot, NodeId(index), work)?
                .map(<[String]>::to_vec),
        });
    }
    Ok(result)
}

/// Hidden leaves supply comment boundaries. No language-specific delimiter is
/// recognized here: non-whitespace hidden tokens on a semantic token's line
/// belong to that preceding token. A following line's indentation stays leading.
fn inline_trivia(
    snapshot: &DocumentSnapshot,
    items: &[Token],
    work: &mut usize,
) -> Result<BTreeMap<usize, usize>, String> {
    let text = snapshot.document().text();
    let mut result = BTreeMap::new();
    for node in snapshot.document().nodes() {
        charge(work)?;
        if node.kind != NodeKind::Leaf || !node.data.hidden || node.range.start == node.range.end {
            continue;
        }
        let range = snapshot
            .document()
            .utf8_range(node.range)?
            .ok_or("Hidden syntax splits Unicode")?;
        let mut comment = false;
        for ch in text[range.start..range.end].chars() {
            charge(work)?;
            comment |= !ch.is_whitespace();
        }
        if !comment {
            continue;
        }
        let index = items.partition_point(|token| token.range.end <= range.start);
        if index == 0 {
            continue;
        }
        let start = items[index - 1].range.end;
        let boundary = items
            .get(index)
            .map_or(text.len(), |token| token.range.start);
        if range.end > boundary {
            return Err("Hidden syntax overlaps a semantic token".into());
        }
        let mut same_line = true;
        for ch in text[start..range.start].chars() {
            charge(work)?;
            same_line &= ch != '\n' && ch != '\r';
        }
        if !same_line {
            continue;
        }
        let mut end = range.end;
        // A single-line hidden rule often includes its own line ending. Otherwise
        // carry the first following ending, so the comment cannot eat new syntax.
        if !text[range.start..range.end].ends_with(['\n', '\r']) {
            let mut chars = text[end..boundary].char_indices().peekable();
            while let Some((offset, ch)) = chars.next() {
                charge(work)?;
                if !ch.is_whitespace() {
                    break;
                }
                end = range.end + offset + ch.len_utf8();
                if ch == '\n' {
                    break;
                }
                if ch == '\r' {
                    if let Some((offset, '\n')) = chars.peek().copied() {
                        charge(work)?;
                        end = range.end + offset + 1;
                    }
                    break;
                }
            }
        }
        result
            .entry(index)
            .and_modify(|previous: &mut usize| *previous = (*previous).max(end))
            .or_insert(end);
    }
    Ok(result)
}

#[derive(Clone, Copy)]
struct MergePolicy {
    output_bytes: usize,
    reuse_unassigned: bool,
    keep_unchanged: bool,
    prefer_empty: bool,
    filter_trivia: bool,
}
fn merge_sources(
    session: &SourceEditSession,
    after: &[DocumentSnapshot],
    new_model: &Model,
    candidate: &Model,
    policy: MergePolicy,
    work: &mut usize,
) -> Result<Vec<Source>, String> {
    let before = &session.snapshots;
    let old_model = session.model();
    let schema = &session.package().schema;
    let limit = policy.output_bytes;
    let mut old = BTreeMap::new();
    let mut new = BTreeMap::new();
    for snapshot in before {
        old.insert(
            snapshot.document().source().uri.as_str(),
            (
                snapshot,
                tokens(&session.language, snapshot, old_model, schema, work)?,
            ),
        );
    }
    for snapshot in after {
        new.insert(
            snapshot.document().source().uri.as_str(),
            (
                snapshot,
                tokens(&session.language, snapshot, new_model, schema, work)?,
            ),
        );
    }
    let mut new_keys = BTreeMap::new();
    let mut old_keys = BTreeMap::new();
    for (uri, (_, items)) in &new {
        for (i, token) in items.iter().enumerate() {
            new_keys.insert(token.key.clone(), (*uri, i));
        }
    }
    for (uri, (_, items)) in &old {
        for (i, token) in items.iter().enumerate() {
            old_keys.insert(token.key.clone(), (*uri, i, token));
        }
    }
    let mut old_positions: BTreeMap<_, _> = old_keys
        .iter()
        .map(|(key, (uri, index, _))| (key.clone(), (*uri, *index)))
        .collect();
    // A changed grammar alternative can replace a leaf while retaining its
    // enclosing object and neighboring syntax. Associate replacement layout
    // between stable anchors; this never changes semantic object identities or
    // authorizes reuse of the replaced token's old spelling.
    let mut aliases = Vec::new();
    for (uri, (_, items)) in &old {
        let Some((_, replacement)) = new.get(uri) else {
            continue;
        };
        // Creating/removing an optional entry object changes ownership of the
        // surrounding unassigned header syntax. Reuse layout by exact grammar
        // origin only; no object identity or assigned value is transferred.
        let empty_before = old_model
            .resources
            .get(*uri)
            .is_some_and(|record| record.roots.is_empty());
        let empty_after = new_model
            .resources
            .get(*uri)
            .is_some_and(|record| record.roots.is_empty());
        if empty_before != empty_after {
            for (old_index, token) in items
                .iter()
                .enumerate()
                .filter(|(_, token)| !token.assigned)
            {
                for (new_index, after) in replacement.iter().enumerate() {
                    charge(work)?;
                    if !after.assigned
                        && token.key.origin == after.key.origin
                        && token.key.occurrence == after.key.occurrence
                    {
                        aliases.push((
                            token.key.clone(),
                            after.key.clone(),
                            *uri,
                            old_index,
                            new_index,
                        ));
                        break;
                    }
                }
            }
        }
        let mut old_begin = 0;
        let mut new_begin = 0;
        for index in 0..=items.len() {
            charge(work)?;
            let boundary = if let Some(token) = items.get(index) {
                new_keys
                    .get(&token.key)
                    .filter(|(source, _)| source == uri)
                    .map(|(_, index)| *index)
            } else {
                Some(replacement.len())
            };
            let Some(boundary) = boundary else {
                continue;
            };
            if boundary >= new_begin {
                let left = &items[old_begin..index];
                let right = &replacement[new_begin..boundary];
                *work = work
                    .checked_sub(left.len().saturating_add(right.len()))
                    .ok_or("Replacement layout association work budget exceeded")?;
                if let Some(owner) = left.first().and_then(|token| token.key.owner.as_ref())
                    && !right.is_empty()
                    && left.iter().all(|token| {
                        token.key.owner.as_ref() == Some(owner)
                            && !new_keys.contains_key(&token.key)
                    })
                    && right.iter().all(|token| {
                        token.key.owner.as_ref() == Some(owner)
                            && !old_keys.contains_key(&token.key)
                    })
                {
                    for (offset, (before, after)) in left.iter().zip(right).enumerate() {
                        aliases.push((
                            before.key.clone(),
                            after.key.clone(),
                            *uri,
                            old_begin + offset,
                            new_begin + offset,
                        ));
                    }
                }
            }
            old_begin = index + 1;
            new_begin = boundary + 1;
        }
    }
    for (before, after, uri, old_index, new_index) in aliases {
        new_keys.insert(before, (uri, new_index));
        old_positions.insert(after, (uri, old_index));
    }
    let mut gaps: BTreeMap<(&str, usize), Vec<(&str, TextRange)>> = BTreeMap::new();
    let mut suffixes: BTreeMap<(&str, usize), Vec<(&str, TextRange)>> = BTreeMap::new();
    for (uri, (snapshot, items)) in &old {
        let inline = inline_trivia(snapshot, items, work)?;
        // Leading trivia follows its next surviving syntax token, including a
        // resource move. Inline comments follow their preceding surviving token.
        // Other trailing trivia stays in its resource; when the whole
        // root moves to a new resource it follows that root. Explicitly deleted
        // resources with no surviving syntax have no destination for their text.
        let mut anchor = if let Some((_, current)) = new.get(uri) {
            Some((*uri, current.len()))
        } else {
            items
                .iter()
                .rev()
                .find_map(|t| new_keys.get(&t.key))
                .map(|(uri, _)| (*uri, new[uri].1.len()))
        };
        for i in (0..=items.len()).rev() {
            charge(work)?;
            if let Some(destination) = items.get(i).and_then(|t| new_keys.get(&t.key)) {
                anchor = Some(*destination);
            }
            let mut start = if i == 0 { 0 } else { items[i - 1].range.end };
            let end = items
                .get(i)
                .map_or(snapshot.document().text().len(), |t| t.range.start);
            if let Some(inline_end) = inline.get(&i)
                && let Some(destination) = i
                    .checked_sub(1)
                    .and_then(|previous| new_keys.get(&items[previous].key))
            {
                suffixes.entry(*destination).or_default().push((
                    *uri,
                    TextRange {
                        start,
                        end: *inline_end,
                    },
                ));
                start = *inline_end;
            }
            if let Some(anchor) = anchor {
                gaps.entry(anchor)
                    .or_default()
                    .push((*uri, TextRange { start, end }));
            }
        }
    }
    if policy.filter_trivia {
        filter_trivia(
            &session.language,
            &old,
            &new,
            &mut gaps,
            &mut suffixes,
            work,
        )?;
    }
    let mut result = Vec::new();
    let mut total = 0usize;
    for (uri, (snapshot, items)) in &new {
        let text = snapshot.document().text();
        let mut output = String::new();
        let mut previous = 0;
        let ending = old
            .get(uri)
            .map(|(s, _)| s.document().text())
            .or_else(|| {
                items.iter().find_map(|t| {
                    old_keys
                        .get(&t.key)
                        .map(|(uri, _, _)| old[uri].0.document().text())
                })
            })
            .map_or(
                "\n",
                |text| if text.contains("\r\n") { "\r\n" } else { "\n" },
            );
        for (i, token) in items.iter().enumerate() {
            charge(work)?;
            let mut hidden = false;
            let ranges = gaps.remove(&(*uri, i));
            let anchored = ranges.is_some();
            if let Some(ranges) = ranges {
                for (source, range) in ranges.into_iter().rev() {
                    hidden |= range.start != range.end;
                    push(
                        &mut output,
                        &old[source].0.document().text()[range.start..range.end],
                        limit.saturating_sub(total),
                    )?;
                }
            }
            let adjacent = match old_positions.get(&token.key) {
                Some((source, 0)) => i == 0 && source == uri,
                Some((source, index)) => {
                    i > 0
                        && old_positions.get(&items[i - 1].key).is_some_and(
                            |(previous_source, previous_index)| {
                                source == previous_source && *previous_index + 1 == *index
                            },
                        )
                }
                None => false,
            };
            if !(hidden || adjacent || policy.prefer_empty && anchored) {
                let gap = &text[previous..token.range.start];
                if ending == "\r\n" {
                    push(
                        &mut output,
                        &gap.replace('\n', "\r\n"),
                        limit.saturating_sub(total),
                    )?;
                } else {
                    push(&mut output, gap, limit.saturating_sub(total))?;
                }
            }
            let spelling = if let Some((source, _, old_token)) = old_keys
                .get(&token.key)
                .filter(|(_, _, old)| policy.reuse_unassigned && !old.assigned && !token.assigned)
            {
                &old[source].0.document().text()[old_token.range.start..old_token.range.end]
            } else {
                &text[token.range.start..token.range.end]
            };
            push(&mut output, spelling, limit.saturating_sub(total))?;
            if let Some(ranges) = suffixes.remove(&(*uri, i)) {
                for (source, range) in ranges {
                    push(
                        &mut output,
                        &old[source].0.document().text()[range.start..range.end],
                        limit.saturating_sub(total),
                    )?;
                }
            }
            previous = token.range.end;
        }
        if let Some(ranges) = gaps.remove(&(*uri, items.len())) {
            for (source, range) in ranges.into_iter().rev() {
                push(
                    &mut output,
                    &old[source].0.document().text()[range.start..range.end],
                    limit.saturating_sub(total),
                )?;
            }
        } else {
            push(&mut output, &text[previous..], limit.saturating_sub(total))?;
        }
        total = total
            .checked_add(output.len())
            .ok_or("Preserved source length overflow")?;
        result.push(Source {
            uri: (*uri).into(),
            text: output,
        });
    }
    if policy.keep_unchanged {
        for source in &mut result {
            if unchanged_resource(session, candidate, &source.uri)? {
                source.text = session
                    .sources
                    .iter()
                    .find(|old| old.uri == source.uri)
                    .ok_or("Missing unchanged resource source")?
                    .text
                    .clone();
            }
        }
    }
    result.sort_by_key(|source| {
        before
            .iter()
            .position(|snapshot| snapshot.document().source().uri == source.uri)
            .unwrap_or(
                before.len()
                    + after
                        .iter()
                        .position(|snapshot| snapshot.document().source().uri == source.uri)
                        .unwrap_or(after.len()),
            )
    });
    if result
        .iter()
        .try_fold(0usize, |total, source| total.checked_add(source.text.len()))
        .is_none_or(|total| total > limit)
    {
        return Err("Preserved sources exceed output byte budget".into());
    }
    Ok(result)
}
type SyntaxSources<'a> = BTreeMap<&'a str, (&'a DocumentSnapshot, Vec<Token>)>;
type Trivia<'a> = BTreeMap<(&'a str, usize), Vec<(&'a str, TextRange)>>;
fn accepts_trivia(
    grammar: &crate::grammar::Grammar,
    hidden: Option<&[String]>,
    text: &str,
    work: &mut usize,
) -> Result<bool, String> {
    let Some(hidden) = hidden else {
        return Ok(true);
    };
    let mut offset = 0;
    while offset < text.len() {
        charge(work)?;
        let mut longest = 0;
        for name in hidden {
            charge(work)?;
            if let Some(terminal) = grammar.terminals.get(name) {
                let mut remaining = Some(*work);
                let matched = terminal.match_len_with_work(&text[offset..], &mut remaining);
                *work = remaining.ok_or("Lost trivia work budget")?;
                if *work == 0 {
                    return Err("Trivia matching work budget exceeded".into());
                }
                if let Ok(length) = matched {
                    longest = longest.max(length);
                }
            }
        }
        if longest == 0 {
            return Ok(false);
        }
        offset += longest;
    }
    Ok(true)
}
fn filter_trivia<'a>(
    language: &DocumentLanguage,
    old: &SyntaxSources<'a>,
    new: &SyntaxSources<'a>,
    gaps: &mut Trivia<'a>,
    suffixes: &mut Trivia<'a>,
    work: &mut usize,
) -> Result<(), String> {
    let grammar = language
        .package()
        .grammar
        .as_ref()
        .ok_or("Missing trivia grammar")?;
    let hidden_at = |uri: &str, index: usize| {
        new[uri]
            .1
            .get(index)
            .map_or(grammar.hidden.as_deref(), |token| token.hidden.as_deref())
    };
    let mut relocated = Trivia::new();
    for (suffix, selected) in [(false, &mut *gaps), (true, &mut *suffixes)] {
        let original = std::mem::take(selected);
        for ((uri, index), ranges) in original {
            let boundary = index + usize::from(suffix);
            for (source, range) in ranges {
                charge(work)?;
                let snapshot = old[source].0;
                let text = &snapshot.document().text()[range.start..range.end];
                if accepts_trivia(grammar, hidden_at(uri, boundary), text, work)? {
                    selected
                        .entry((uri, index))
                        .or_default()
                        .push((source, range));
                    continue;
                }
                for node in snapshot.document().nodes() {
                    charge(work)?;
                    if node.kind != NodeKind::Leaf || !node.data.hidden {
                        continue;
                    }
                    let node_range = snapshot
                        .document()
                        .utf8_range(node.range)?
                        .ok_or("Trivia splits Unicode")?;
                    let fragment = TextRange {
                        start: range.start.max(node_range.start),
                        end: range.end.min(node_range.end),
                    };
                    if fragment.start >= fragment.end {
                        continue;
                    }
                    let text = &snapshot.document().text()[fragment.start..fragment.end];
                    if accepts_trivia(grammar, hidden_at(uri, boundary), text, work)? {
                        selected
                            .entry((uri, index))
                            .or_default()
                            .push((source, fragment));
                        continue;
                    }
                    let mut comment = false;
                    for character in text.chars() {
                        charge(work)?;
                        comment |= !character.is_whitespace();
                    }
                    if !comment {
                        continue;
                    }
                    let tokens = &new[uri].1;
                    let owner = tokens.get(index).and_then(|token| token.key.owner.as_ref());
                    let mut destination = None;
                    for at in 0..=tokens.len() {
                        charge(work)?;
                        let candidate_owner =
                            tokens.get(at).and_then(|token| token.key.owner.as_ref());
                        let rank = (
                            owner.is_none() || candidate_owner != owner,
                            at.abs_diff(boundary),
                            at,
                        );
                        if destination.as_ref().is_none_or(|(old, _)| rank < *old)
                            && accepts_trivia(grammar, hidden_at(uri, at), text, work)?
                        {
                            destination = Some((rank, at));
                        }
                    }
                    let Some((_, at)) = destination else {
                        return Err(format!(
                            "No legal hidden context in {uri} can retain a source comment"
                        ));
                    };
                    relocated
                        .entry((uri, at))
                        .or_default()
                        .push((source, fragment));
                }
            }
        }
    }
    for (destination, ranges) in relocated {
        gaps.entry(destination).or_default().extend(ranges);
    }
    // Gaps are emitted in reverse insertion order; preserve original byte order
    // even when a rejected gap has been split into several hidden leaves.
    for ranges in gaps.values_mut() {
        ranges.sort_by(|a, b| (b.0, b.1.start).cmp(&(a.0, a.1.start)));
    }
    for ranges in suffixes.values_mut() {
        ranges.sort_by(|a, b| (a.0, a.1.start).cmp(&(b.0, b.1.start)));
    }
    Ok(())
}

fn push(output: &mut String, text: &str, limit: usize) -> Result<(), String> {
    if output
        .len()
        .checked_add(text.len())
        .is_none_or(|n| n > limit)
    {
        return Err("Preserved source exceeds output byte budget".into());
    }
    output
        .try_reserve(text.len())
        .map_err(|_| "Preserved source allocation failed")?;
    output.push_str(text);
    Ok(())
}
pub(super) fn difference(uri: &str, before: &str, after: &str) -> Option<SourcePatch> {
    if before == after {
        return None;
    }
    let start = before
        .chars()
        .zip(after.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    let suffix = before[start..]
        .chars()
        .rev()
        .zip(after[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    let range = TextRange {
        start,
        end: before.len() - suffix,
    };
    Some(SourcePatch {
        uri: uri.into(),
        owner: String::new(),
        feature: String::new(),
        range,
        output_range: TextRange {
            start,
            end: after.len() - suffix,
        },
        original: before[start..before.len() - suffix].into(),
        replacement: after[start..after.len() - suffix].into(),
    })
}
