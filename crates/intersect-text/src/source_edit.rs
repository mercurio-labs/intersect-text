//! Verified source-preserving edits over the shared document parser.
//!
//! [`SourceEditSession::plan`] uses the retained grammar and semantic identities
//! to preserve existing text while verifying a candidate model. General plans
//! support representable structural and cross-resource changes; ambiguous or
//! unrepresentable syntax is rejected. Language packages provide custom spelling
//! and scope services through explicit contexts, without a second parser.
//!
//! The scalar `plan_with` and leaf append/delete/move helpers have narrower
//! contracts; their restrictions do not describe the general planner. Preparing
//! a result reparses, links, validates and compares the complete proposed graph.
//! Hosts own editor/disk publication and must perform their revision checks.
use crate::{
    Evaluation, EvaluationLimits, Source,
    document::{DocumentAnalysisSet, DocumentLanguage, DocumentSnapshot},
};
use intersect_model::core::{
    model::{Element, Model},
    resources::ResourceRecord,
    text_document::{NodeId, NodeKind, TextDocument, TextRange},
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

type ScalarChange<'a> = (&'a Element, &'a Element, &'a str);

#[path = "source_edit_append.rs"]
mod append;
#[path = "source_edit_general.rs"]
mod general;
#[path = "source_edit_identity.rs"]
mod identity;
#[path = "source_edit_preserve.rs"]
mod preserve;
#[path = "source_edit_structure.rs"]
mod structure;
pub use append::{AppendPlacement, AppendTarget, SourceInsertion};
pub use structure::MovePlacement;

/// A language-owned spelling decision for an already matched scalar token.
pub struct ScalarSpelling<'a> {
    pub owner: &'a str,
    pub feature: &'a str,
    pub original: &'a str,
    pub before: &'a Value,
    pub after: &'a Value,
    pub before_target: Option<&'a Element>,
    pub target: Option<&'a Element>,
}

/// One exact UTF-8 byte replacement. Ranges belong to the original/output text,
/// respectively; they are not UTF-16 editor coordinates.
#[derive(Clone, Debug, Serialize)]
pub struct SourcePatch {
    pub uri: String,
    pub owner: String,
    pub feature: String,
    pub range: TextRange,
    pub output_range: TextRange,
    pub original: String,
    pub replacement: String,
}

/// Immutable, fully accepted baseline and its prepared language authority.
pub struct SourceEditSession {
    owner: Arc<()>,
    language: Arc<DocumentLanguage>,
    context_owner: Option<Arc<()>>,
    owned_services: Option<crate::text_services::TextServices>,
    sources: Vec<Source>,
    snapshots: Vec<DocumentSnapshot>,
    evaluation: Evaluation,
    limits: EvaluationLimits,
    identities: BTreeMap<String, String>,
}

/// A borrowed, immutable service selection with private session authority.
/// Its mutable forward-linking budget remains caller-owned and is not reset
/// between parsing/generation attempts. This capability is never deserialized.
pub struct SourceEditContext<'a, 'providers> {
    pub(crate) owner: Arc<()>,
    pub(crate) services: crate::document::DocumentServices<'a, 'providers>,
}
impl<'a, 'providers> SourceEditContext<'a, 'providers> {
    pub fn new(services: crate::document::DocumentServices<'a, 'providers>) -> Self {
        Self {
            owner: Arc::new(()),
            services,
        }
    }
    /// Explicit host request boundary; changing the allowance does not replace
    /// the selected scope implementation or its session authority.
    pub fn reset_linking_budget(&mut self, steps: usize) -> Result<(), String> {
        let linking = self
            .services
            .linking
            .as_deref_mut()
            .ok_or("No explicit linking context")?;
        *linking.work = intersect_model::core::query::QueryBudget::new(steps);
        Ok(())
    }
}

/// A verified proposal tied to exact original source bytes and model state.
/// There is deliberately no deserialization or unchecked construction API.
pub struct PreparedSourceEdit {
    owner: Arc<()>,
    baseline_sources: Vec<Source>,
    baseline_model: Model,
    output: SourceEditOutput,
}

/// Opaque in-memory history, bound to one prepared language/session. Serialized
/// model observations cannot be used to construct a checkpoint.
#[derive(Clone)]
pub struct SourceEditCheckpoint {
    owner: Arc<()>,
    sources: Vec<Source>,
    model: Model,
}

/// Owned observations after successful baseline checking. The contained analysis
/// has a valid model and source-bound snapshots. Mutating these public DTOs later
/// does not retain the preparation guarantee or authorize host publication.
#[derive(Debug)]
pub struct SourceEditOutput {
    pub sources: Vec<Source>,
    pub patches: Vec<SourcePatch>,
    pub analysis: DocumentAnalysisSet,
    /// Stable candidate ID -> actual parser ID, including original parse-stage
    /// observations. The accepted model and semantic node links use stable IDs.
    pub identities: BTreeMap<String, String>,
}

impl SourceEditSession {
    /// Analyze every source atomically. Recovered/invalid documents never become
    /// an editable baseline; external conversion/setting providers are not retained.
    pub fn new(
        language: DocumentLanguage,
        sources: &[Source],
        limits: EvaluationLimits,
    ) -> Result<Self, String> {
        Self::new_using(Arc::new(language), sources, limits, None)
    }

    pub fn new_shared(
        language: Arc<DocumentLanguage>,
        sources: &[Source],
        limits: EvaluationLimits,
    ) -> Result<Self, String> {
        Self::new_using(language, sources, limits, None)
    }

    pub fn installed_services(&self) -> Option<&crate::text_services::TextServices> {
        self.owned_services.as_ref()
    }

    pub fn conversion_registry(
        &self,
    ) -> Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>> {
        self.owned_services
            .as_ref()
            .and_then(|services| services.conversions())
    }

    pub fn new_with_context(
        language: Arc<DocumentLanguage>,
        sources: &[Source],
        limits: EvaluationLimits,
        context: &mut SourceEditContext<'_, '_>,
    ) -> Result<Self, String> {
        Self::new_using(language, sources, limits, Some(context))
    }

    pub fn new_with_services(
        language: Arc<DocumentLanguage>,
        sources: &[Source],
        limits: EvaluationLimits,
        services: crate::text_services::TextServices,
    ) -> Result<Self, String> {
        let mut session = services.with_context(
            &language,
            limits.steps_per_resource.unwrap_or(usize::MAX),
            |context| Self::new_using(Arc::clone(&language), sources, limits, Some(context)),
        )?;
        session.owned_services = Some(services);
        Ok(session)
    }

    fn new_using(
        language: Arc<DocumentLanguage>,
        sources: &[Source],
        limits: EvaluationLimits,
        context: Option<&mut SourceEditContext<'_, '_>>,
    ) -> Result<Self, String> {
        let context_owner = context.as_ref().map(|context| Arc::clone(&context.owner));
        let mut analysis = match context {
            Some(context) => {
                language.analyze_sources_with_services(sources, limits, &mut context.services)?
            }
            None => language.analyze_sources(sources, None, limits)?,
        };
        let evaluation = accepted(&mut analysis)?;
        let identities = evaluation
            .model
            .elements
            .iter()
            .map(|e| (e.id.clone(), e.id.clone()))
            .collect();
        Ok(Self {
            owner: Arc::new(()),
            language,
            context_owner,
            owned_services: None,
            sources: sources.to_vec(),
            snapshots: analysis.snapshots,
            evaluation,
            limits,
            identities,
        })
    }

    fn check_context(&self, context: Option<&SourceEditContext<'_, '_>>) -> Result<(), String> {
        match (&self.context_owner, context) {
            (None, None) => Ok(()),
            (Some(owner), Some(context)) if Arc::ptr_eq(owner, &context.owner) => Ok(()),
            _ => Err("Source edit requires its original native language-service context".into()),
        }
    }

    pub fn model(&self) -> &Model {
        &self.evaluation.model
    }
    pub fn evaluation(&self) -> &Evaluation {
        &self.evaluation
    }

    pub fn checkpoint(&self) -> SourceEditCheckpoint {
        SourceEditCheckpoint {
            owner: Arc::clone(&self.owner),
            sources: self.sources.clone(),
            model: self.model().clone(),
        }
    }

    pub fn plan_restore(
        &self,
        checkpoint: &SourceEditCheckpoint,
    ) -> Result<PreparedSourceEdit, String> {
        if let Some(services) = &self.owned_services {
            return services.with_context(
                &self.language,
                self.limits.steps_per_resource.unwrap_or(usize::MAX),
                |context| self.plan_restore_using(checkpoint, Some(context)),
            );
        }
        self.plan_restore_using(checkpoint, None)
    }
    pub fn plan_restore_with_context(
        &self,
        checkpoint: &SourceEditCheckpoint,
        context: &mut SourceEditContext<'_, '_>,
    ) -> Result<PreparedSourceEdit, String> {
        self.plan_restore_using(checkpoint, Some(context))
    }
    fn plan_restore_using(
        &self,
        checkpoint: &SourceEditCheckpoint,
        context: Option<&mut SourceEditContext<'_, '_>>,
    ) -> Result<PreparedSourceEdit, String> {
        self.check_context(context.as_deref())?;
        if !Arc::ptr_eq(&self.owner, &checkpoint.owner) {
            return Err("History belongs to another language/session".into());
        }
        let mut patches = Vec::new();
        let mut changed = BTreeSet::new();
        for source in &self.sources {
            let restored = checkpoint
                .sources
                .iter()
                .find(|s| s.uri == source.uri)
                .map_or("", |s| s.text.as_str());
            if let Some(patch) = general::difference(&source.uri, &source.text, restored) {
                patches.push(patch);
            }
            if source.text != restored
                || self.model().resources.get(&source.uri)
                    != checkpoint.model.resources.get(&source.uri)
            {
                changed.insert(source.uri.as_str());
            }
        }
        for source in &checkpoint.sources {
            if !self.sources.iter().any(|s| s.uri == source.uri) {
                changed.insert(source.uri.as_str());
                if let Some(patch) = general::difference(&source.uri, "", &source.text) {
                    patches.push(patch);
                }
            }
        }
        self.verify_sources_using(
            &checkpoint.model,
            checkpoint.sources.clone(),
            patches,
            &changed,
            false,
            context,
        )
    }

    pub fn package(&self) -> &crate::package::Package {
        self.language.package()
    }

    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    pub fn snapshot(&self, uri: &str) -> Result<&DocumentSnapshot, String> {
        snapshot_for(&self.snapshots, uri)
    }

    pub fn parsed_identities(&self) -> &BTreeMap<String, String> {
        &self.identities
    }

    /// Adopt a proposal from this exact session after checking its still-current
    /// baseline. Keeps the prepared language, refreshed source maps and monotone
    /// resource revisions for subsequent edits. Does not publish to a host.
    pub fn adopt(&mut self, prepared: PreparedSourceEdit) -> Result<Vec<SourcePatch>, String> {
        if !Arc::ptr_eq(&self.owner, &prepared.owner) {
            return Err("Source edit belongs to a different prepared session".into());
        }
        let mut output = prepared.finish(&self.sources, self.model())?;
        let evaluation = accepted(&mut output.analysis)?;
        self.sources = output.sources;
        self.snapshots = output.analysis.snapshots;
        self.evaluation = evaluation;
        self.identities = output.identities;
        Ok(output.patches)
    }

    /// Prepare the smallest supported token replacements and verify the complete
    /// proposed graph by authoritative reanalysis. Callback errors and unsupported
    /// edits leave this session unchanged. No-op plans never call the callback.
    pub fn plan_with(
        &self,
        candidate: &Model,
        mut spelling: impl FnMut(ScalarSpelling<'_>) -> Result<String, String>,
    ) -> Result<PreparedSourceEdit, String> {
        if let Some(services) = &self.owned_services {
            return services.with_context(
                &self.language,
                self.limits.steps_per_resource.unwrap_or(usize::MAX),
                |context| self.plan_scalar_using(candidate, Some(context), &mut spelling),
            );
        }
        self.plan_scalar_using(candidate, None, &mut spelling)
    }
    fn plan_scalar_using(
        &self,
        candidate: &Model,
        context: Option<&mut SourceEditContext<'_, '_>>,
        mut spelling: impl FnMut(ScalarSpelling<'_>) -> Result<String, String>,
    ) -> Result<PreparedSourceEdit, String> {
        let changes = self.changes_using(candidate, context.as_deref())?;

        let mut changed_resources: BTreeSet<_> = changes
            .iter()
            .map(|(before, _, _)| before.resource.as_str())
            .collect();
        check_candidate_resources(self.model(), candidate, &changed_resources)?;
        let changed_objects: BTreeSet<_> = changes
            .iter()
            .map(|(before, _, _)| before.id.as_str())
            .collect();
        let mut patches = Vec::with_capacity(changes.len());
        for (before, after, feature) in changes {
            let snapshot = snapshot_for(&self.snapshots, &before.resource)?;
            let range = occurrence(snapshot, &before.id, feature)?;
            let original = snapshot
                .document()
                .text()
                .get(range.start..range.end)
                .ok_or("Scalar occurrence is not on UTF-8 source boundaries")?;
            let replacement = spelling(ScalarSpelling {
                owner: &before.id,
                feature,
                original,
                before: before
                    .values
                    .get(feature)
                    .ok_or("Missing baseline scalar")?,
                after: after
                    .values
                    .get(feature)
                    .ok_or("Missing candidate scalar")?,
                before_target: reference_target(self.package(), self.model(), before, feature)?,
                target: reference_target(self.package(), candidate, after, feature)?,
            })?;
            if replacement.is_empty() {
                return Err("Scalar replacement must contain one significant token".into());
            }
            patches.push(SourcePatch {
                uri: before.resource.clone(),
                owner: before.id.clone(),
                feature: feature.into(),
                range,
                output_range: range,
                original: original.into(),
                replacement,
            });
        }
        // Naming/scoping policy is language-owned: ask it whether incoming
        // spellings need to change for any edited object, without guessing which
        // stored attribute constitutes the language's name.
        for reference in &self.evaluation.references {
            if !changed_objects.contains(reference.target.as_str())
                || patches.iter().any(|p| {
                    p.uri == reference.uri
                        && p.range.start == reference.start
                        && p.range.end == reference.end
                })
            {
                continue;
            }
            let snapshot = self.snapshot(&reference.uri)?;
            let mut occurrences = snapshot.feature_nodes().iter().filter(|f| {
                snapshot
                    .document()
                    .nodes()
                    .get(f.node.0)
                    .and_then(|n| snapshot.document().utf8_range(n.range).ok().flatten())
                    == Some(TextRange {
                        start: reference.start,
                        end: reference.end,
                    })
            });
            let feature = occurrences
                .next()
                .ok_or("Incoming reference has no feature occurrence")?;
            if occurrences.next().is_some() {
                return Err("Ambiguous incoming reference occurrence".into());
            }
            let before = self
                .model()
                .elements
                .iter()
                .find(|e| e.id == feature.owner)
                .ok_or("Missing reference owner")?;
            let after = candidate
                .elements
                .iter()
                .find(|e| e.id == feature.owner)
                .ok_or("Missing candidate reference owner")?;
            let range = occurrence(snapshot, &before.id, &feature.feature)?;
            let original = &snapshot.document().text()[range.start..range.end];
            let replacement = spelling(ScalarSpelling {
                owner: &before.id,
                feature: &feature.feature,
                original,
                before: before
                    .values
                    .get(&feature.feature)
                    .ok_or("Missing reference value")?,
                after: after
                    .values
                    .get(&feature.feature)
                    .ok_or("Missing candidate reference value")?,
                before_target: reference_target(
                    self.package(),
                    self.model(),
                    before,
                    &feature.feature,
                )?,
                target: reference_target(self.package(), candidate, after, &feature.feature)?,
            })?;
            if replacement != original {
                changed_resources.insert(
                    self.sources
                        .iter()
                        .find(|s| s.uri == reference.uri)
                        .ok_or("Missing reference resource")?
                        .uri
                        .as_str(),
                );
                patches.push(SourcePatch {
                    uri: reference.uri.clone(),
                    owner: before.id.clone(),
                    feature: feature.feature.clone(),
                    range,
                    output_range: range,
                    original: original.into(),
                    replacement,
                });
            }
        }
        patches.sort_by(|a, b| a.uri.cmp(&b.uri).then(a.range.start.cmp(&b.range.start)));
        let prepared =
            self.verify_patches_using(candidate, patches, &changed_resources, true, context)?;
        preserve::verify(
            &self.snapshots,
            &prepared.output.analysis.snapshots,
            &prepared.output.patches,
            &BTreeSet::new(),
            None,
            true,
        )?;
        Ok(prepared)
    }

    fn verify_patches(
        &self,
        candidate: &Model,
        patches: Vec<SourcePatch>,
        changed_resources: &BTreeSet<&str>,
        scalar_tokens: bool,
    ) -> Result<PreparedSourceEdit, String> {
        self.verify_patches_using(candidate, patches, changed_resources, scalar_tokens, None)
    }
    fn verify_patches_using(
        &self,
        candidate: &Model,
        mut patches: Vec<SourcePatch>,
        changed_resources: &BTreeSet<&str>,
        scalar_tokens: bool,
        context: Option<&mut SourceEditContext<'_, '_>>,
    ) -> Result<PreparedSourceEdit, String> {
        self.check_context(context.as_deref())?;
        let sources = apply_patches(&self.sources, &mut patches, self.limits)?;
        self.verify_sources_using(
            candidate,
            sources,
            patches,
            changed_resources,
            scalar_tokens,
            context,
        )
    }

    fn verify_sources_using(
        &self,
        candidate: &Model,
        sources: Vec<Source>,
        patches: Vec<SourcePatch>,
        changed_resources: &BTreeSet<&str>,
        scalar_tokens: bool,
        mut context: Option<&mut SourceEditContext<'_, '_>>,
    ) -> Result<PreparedSourceEdit, String> {
        self.check_context(context.as_deref())?;
        let mut analysis = match context.as_deref_mut() {
            Some(context) => self.language.analyze_sources_with_services(
                &sources,
                self.limits,
                &mut context.services,
            )?,
            None => self.language.analyze_sources(&sources, None, self.limits)?,
        };
        let mut evaluation = accepted(&mut analysis)?;
        let identities = identity::reconcile_using(
            &self.language,
            candidate,
            &mut evaluation,
            &mut analysis.snapshots,
            context.as_ref().map(|context| &context.services),
            self.limits.steps_per_resource.unwrap_or(usize::MAX),
        )?;
        check_reparsed(candidate, &evaluation.model)?;
        // A callback must emit exactly the new scalar token, not additional
        // trivia, optional clauses or declaration text with coincidentally equal values.
        for patch in patches.iter().filter(|_| scalar_tokens) {
            let snapshot = snapshot_for(&analysis.snapshots, &patch.uri)?;
            if occurrence(snapshot, &patch.owner, &patch.feature)? != patch.output_range {
                return Err("Replacement is not exactly one source-bound scalar token".into());
            }
        }
        reconcile_resources(
            self.model(),
            candidate,
            &mut evaluation.model,
            changed_resources,
        )?;
        let mut snapshots = Vec::with_capacity(analysis.snapshots.len());
        for mut snapshot in analysis.snapshots {
            let record = evaluation
                .model
                .resources
                .get(&snapshot.document().source().uri)
                .ok_or("Reparsed document resource is missing")?;
            snapshot.document = snapshot.document.bind_resource(record)?;
            snapshots.push(snapshot);
        }
        analysis.snapshots = snapshots;
        match context
            .as_ref()
            .and_then(|context| context.services.conversions)
        {
            Some(registry) => evaluation
                .model
                .validate_with_conversions(&self.package().schema, registry)?,
            None => evaluation.model.validate(&self.package().schema)?,
        }
        crate::apply_semantics_with_provider(
            self.package(),
            &mut evaluation,
            context
                .as_ref()
                .and_then(|context| context.services.derived)
                .unwrap_or(&intersect_model::query::PackageDerived(self.package())),
        );
        if !evaluation.diagnostics.is_empty() {
            return Err(format!(
                "Reconciled source model failed semantic validation: {:?}",
                evaluation.diagnostics
            ));
        }
        analysis.valid_model = Some(evaluation);
        Ok(PreparedSourceEdit {
            owner: Arc::clone(&self.owner),
            baseline_sources: self.sources.clone(),
            baseline_model: self.model().clone(),
            output: SourceEditOutput {
                sources,
                patches,
                analysis,
                identities,
            },
        })
    }

    fn changes_using<'a>(
        &'a self,
        candidate: &'a Model,
        context: Option<&SourceEditContext<'_, '_>>,
    ) -> Result<Vec<ScalarChange<'a>>, String> {
        self.check_context(context)?;
        let baseline = self.model();
        if candidate.schema_revision != baseline.schema_revision
            || candidate.elements.len() != baseline.elements.len()
        {
            return Err("Source edits do not support schema or structural changes".into());
        }
        let schema = &self.language.package().schema;
        match context.and_then(|context| context.services.conversions) {
            Some(registry) => candidate.validate_with_conversions(schema, registry)?,
            None => candidate.validate(schema)?,
        }
        let mut changes = Vec::new();
        for (before, after) in baseline.elements.iter().zip(&candidate.elements) {
            if before.id != after.id
                || before.class != after.class
                || before.type_identity != after.type_identity
                || before.resource != after.resource
                || before.start != after.start
                || before.end != after.end
                || before.initialization != after.initialization
            {
                return Err(
                    "Source edits do not support object identity, structure or metadata changes"
                        .into(),
                );
            }
            if before.assignment_state != after.assignment_state {
                return Err("Source edits do not support assignment-state changes".into());
            }
            if !before.values.keys().eq(after.values.keys()) {
                return Err(
                    "Source edits do not support adding or removing stored features".into(),
                );
            }
            for (name, value) in &before.values {
                let next = after.values.get(name).ok_or("Missing candidate feature")?;
                if value == next {
                    continue;
                }
                let effective = schema.effective_feature(&before.class, name)?;
                let field = effective.feature;
                if field.containment
                    || field.upper != 1
                    || field.derived
                    || field.transient
                    || field.volatile
                    || !field.changeable
                    || value.is_array()
                    || value.is_object()
                    || next.is_array()
                    || next.is_object()
                {
                    return Err(format!(
                        "Source edit requires a stored scalar attribute: {}.{name}",
                        before.id
                    ));
                }
                changes.push((before, after, name.as_str()));
            }
        }
        Ok(changes)
    }
}

impl PreparedSourceEdit {
    pub fn parsed_identities(&self) -> &BTreeMap<String, String> {
        &self.output.identities
    }
    pub fn patches(&self) -> &[SourcePatch] {
        &self.output.patches
    }

    pub fn sources(&self) -> &[Source] {
        &self.output.sources
    }

    pub fn analysis(&self) -> &DocumentAnalysisSet {
        &self.output.analysis
    }

    /// Check the host's still-current exact baseline before handing off this
    /// result. Hosts must keep checking and publication in their atomic boundary.
    pub fn finish(
        self,
        current_sources: &[Source],
        current_model: &Model,
    ) -> Result<SourceEditOutput, String> {
        if !same_sources(&self.baseline_sources, current_sources)
            || current_model != &self.baseline_model
        {
            return Err("Stale source edit: source bytes or baseline model changed".into());
        }
        Ok(self.output)
    }
}

fn accepted(analysis: &mut DocumentAnalysisSet) -> Result<Evaluation, String> {
    if !analysis.diagnostics.is_empty() || analysis.recovered.is_some() {
        return Err(format!(
            "Source edit requires fully accepted documents: {:?}",
            analysis.diagnostics
        ));
    }
    analysis
        .valid_model
        .take()
        .ok_or_else(|| "Source edit requires a valid model".into())
}

fn reference_target<'a>(
    package: &crate::package::Package,
    model: &'a Model,
    owner: &Element,
    feature: &str,
) -> Result<Option<&'a Element>, String> {
    if !package
        .schema
        .effective_feature(&owner.class, feature)?
        .feature
        .reference
    {
        return Ok(None);
    }
    let Some(id) = owner.values.get(feature).and_then(Value::as_str) else {
        return Ok(None);
    };
    model
        .elements
        .iter()
        .find(|e| e.id == id)
        .map(Some)
        .ok_or_else(|| "Reference target is outside the current model".into())
}

fn same_sources(a: &[Source], b: &[Source]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.uri == b.uri && a.text == b.text)
}

fn snapshot_for<'a>(
    snapshots: &'a [DocumentSnapshot],
    uri: &str,
) -> Result<&'a DocumentSnapshot, String> {
    let mut found = snapshots
        .iter()
        .filter(|s| s.document().source().uri == uri);
    let snapshot = found.next().ok_or("Scalar has no source document")?;
    if found.next().is_some() {
        return Err("Scalar source document is ambiguous".into());
    }
    Ok(snapshot)
}

fn occurrence(
    snapshot: &DocumentSnapshot,
    owner: &str,
    feature: &str,
) -> Result<TextRange, String> {
    let mut found = snapshot
        .feature_nodes()
        .iter()
        .filter(|f| f.owner == owner && f.feature == feature);
    let matched = found
        .next()
        .ok_or("Scalar feature has no existing source occurrence")?;
    if found.next().is_some() {
        return Err("Scalar feature has ambiguous repeated source occurrences".into());
    }
    let document = snapshot.document();
    let root = document
        .nodes()
        .get(matched.node.0)
        .ok_or("Unknown scalar feature node")?;
    let mut significant = None;
    for (index, node) in document.nodes().iter().enumerate() {
        if !descendant(document, NodeId(index), matched.node)? || node.range.start == node.range.end
        {
            continue;
        }
        if node.data.hidden
            && node.range.start < root.range.end
            && root.range.start < node.range.end
        {
            return Err("Scalar feature contains internal trivia".into());
        }
        if node.kind == NodeKind::Leaf
            && !node.data.hidden
            && significant.replace(node.range).is_some()
        {
            return Err("Scalar feature requires exactly one significant source token".into());
        }
    }
    if significant != Some(root.range) || root.range.start == root.range.end {
        return Err("Scalar feature requires exactly one significant source token".into());
    }
    document
        .utf8_range(root.range)?
        .ok_or_else(|| "Scalar source range splits a Unicode scalar".into())
}

fn descendant(document: &TextDocument, node: NodeId, ancestor: NodeId) -> Result<bool, String> {
    let mut next = Some(node);
    let mut remaining = document.nodes().len();
    while let Some(id) = next {
        if remaining == 0 {
            return Err("Cyclic scalar source node ancestry".into());
        }
        remaining -= 1;
        if id == ancestor {
            return Ok(true);
        }
        next = document
            .nodes()
            .get(id.0)
            .ok_or("Unknown source node ancestor")?
            .parent;
    }
    Ok(false)
}

fn check_candidate_resources(
    baseline: &Model,
    candidate: &Model,
    changed: &BTreeSet<&str>,
) -> Result<(), String> {
    if !baseline.resources.keys().eq(candidate.resources.keys()) {
        return Err("Source edits do not support resource metadata changes".into());
    }
    for (uri, before) in &baseline.resources {
        let after = candidate
            .resources
            .get(uri)
            .ok_or("Missing candidate resource")?;
        if !changed.contains(uri.as_str()) {
            if before != after {
                return Err("Source edits do not support metadata-only resource changes".into());
            }
        } else if before.uri != after.uri
            || before.roots != after.roots
            || before.load_state != after.load_state
            || before.diagnostics != after.diagnostics
            || after.content_revision < before.content_revision
            || (after.source.is_some() && after.source != before.source)
        {
            return Err("Source edit candidate changed unrelated resource metadata".into());
        }
    }
    Ok(())
}

fn apply_patches(
    sources: &[Source],
    patches: &mut [SourcePatch],
    limits: EvaluationLimits,
) -> Result<Vec<Source>, String> {
    let mut total = 0usize;
    let mut output = Vec::with_capacity(sources.len());
    for source in sources {
        let mut length = source.text.len();
        for patch in patches.iter().filter(|p| p.uri == source.uri) {
            length = length
                .checked_sub(patch.range.end - patch.range.start)
                .and_then(|n| n.checked_add(patch.replacement.len()))
                .ok_or("Source edit output length overflow")?;
        }
        total = total
            .checked_add(length)
            .ok_or("Source edit aggregate length overflow")?;
        if limits.input_bytes.is_some_and(|limit| total > limit) {
            return Err("Source edit output exceeds input byte budget".into());
        }
        let mut text = String::new();
        text.try_reserve_exact(length)
            .map_err(|_| "Source edit output allocation failed")?;
        let mut cursor = 0;
        for patch in patches.iter_mut().filter(|p| p.uri == source.uri) {
            if patch.range.start < cursor {
                return Err("Overlapping scalar source edits".into());
            }
            text.push_str(
                source
                    .text
                    .get(cursor..patch.range.start)
                    .ok_or("Invalid scalar patch start")?,
            );
            if source.text.get(patch.range.start..patch.range.end) != Some(patch.original.as_str())
            {
                return Err("Scalar patch does not match original source bytes".into());
            }
            patch.output_range.start = text.len();
            text.push_str(&patch.replacement);
            patch.output_range.end = text.len();
            cursor = patch.range.end;
        }
        text.push_str(
            source
                .text
                .get(cursor..)
                .ok_or("Invalid scalar patch end")?,
        );
        output.push(Source {
            uri: source.uri.clone(),
            text,
        });
    }
    Ok(output)
}

fn check_reparsed(candidate: &Model, reparsed: &Model) -> Result<(), String> {
    if candidate.schema_revision != reparsed.schema_revision
        || candidate.elements.len() != reparsed.elements.len()
        || !candidate.resources.keys().eq(reparsed.resources.keys())
    {
        return Err("Serialized source does not reproduce the candidate model structure".into());
    }
    for (expected, actual) in candidate.elements.iter().zip(&reparsed.elements) {
        if expected.id != actual.id
            || expected.class != actual.class
            || expected.type_identity != actual.type_identity
            || expected.resource != actual.resource
            || expected.values != actual.values
            || expected.assignment_state != actual.assignment_state
            || expected.initialization != actual.initialization
        {
            return Err(format!(
                "Serialized source does not reproduce candidate state for {}",
                expected.id
            ));
        }
    }
    for (uri, expected) in &candidate.resources {
        let actual = reparsed
            .resources
            .get(uri)
            .ok_or("Missing reparsed resource")?;
        if expected.uri != actual.uri
            || expected.roots != actual.roots
            || expected.load_state != actual.load_state
            || expected.diagnostics != actual.diagnostics
        {
            return Err("Serialized source does not reproduce candidate resource state".into());
        }
    }
    Ok(())
}

fn reconcile_resources(
    baseline: &Model,
    candidate: &Model,
    reparsed: &mut Model,
    changed: &BTreeSet<&str>,
) -> Result<(), String> {
    for (uri, actual) in &mut reparsed.resources {
        let expected = candidate
            .resources
            .get(uri)
            .ok_or("Missing candidate resource")?;
        let before = baseline.resources.get(uri);
        if changed.contains(uri.as_str()) || before.is_none() {
            let revision = before
                .map_or(0, |r| r.content_revision)
                .max(expected.content_revision)
                .checked_add(1)
                .ok_or("Source edit resource revision exhausted")?;
            *actual = ResourceRecord {
                content_revision: revision,
                source: actual.source.clone(),
                ..expected.clone()
            };
        } else {
            *actual = before.ok_or("Missing original resource")?.clone();
        }
    }
    Ok(())
}
