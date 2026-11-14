//! Exact source-world preparation for language-owned inference. No source
//! rewriting or alternate parser: syntax and executable composition stay shared.
use super::*;

pub(super) fn infer(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
) -> Result<InferredMetamodels> {
    infer_with_conversions(bundle, entry, limits, None)
}
pub(super) fn infer_with_conversions(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
) -> Result<InferredMetamodels> {
    prepare_with_conversions(
        bundle,
        entry,
        limits,
        conversions,
        SourceValidation::default(),
    )?
    .into_model()
}

pub(super) fn prepare(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
) -> Result<PreparedInference> {
    prepare_with_conversions(bundle, entry, limits, None, SourceValidation::default())
}
pub(super) fn prepare_with_profile(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
    profile: grammar::Profile,
) -> Result<PreparedInference> {
    prepare_with_profile_and_conversions(bundle, entry, limits, profile, None)
}
pub(super) fn prepare_with_profile_and_conversions(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
    profile: grammar::Profile,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
) -> Result<PreparedInference> {
    prepare_with_conversions(
        bundle,
        entry,
        limits,
        conversions,
        SourceValidation {
            contextual_first_rule: profile.has_contextual_first_rule(),
            scoped_terminal_returns: profile.has_terminal_cardinality_decisions(),
            scoped_hidden_tokens: profile.has_scoped_hidden_tokens(),
            durable_documents: profile.has_durable_documents(),
            imported_assignment_compatibility: profile.has_imported_assignment_compatibility(),
            unordered_groups: profile.has_unordered_groups(),
        },
    )
}
fn prepare_with_conversions(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    validation: SourceValidation,
) -> Result<PreparedInference> {
    bundle
        .validate()
        .map_err(|e| Diagnostic::new(&entry.uri, 0, 1, e).with_context(entry))?;
    let mut active = BTreeSet::new();
    visit(bundle, entry, limits, &mut active, conversions, validation)
}
fn closure(bundle: &DefinitionBundle, entry: &SourceId) -> Result<DefinitionBundle> {
    let mut selected = BTreeSet::new();
    let mut pending = vec![entry.clone()];
    // Namespace selections are explicit constraints on this source world. Keep
    // their targets so a missing declared dependency rejects instead of silently
    // selecting another same-namespace source after projection.
    pending.extend(
        bundle
            .namespaces
            .iter()
            .map(|binding| binding.source.clone()),
    );
    while let Some(id) = pending.pop() {
        if selected.insert(id.clone()) {
            pending.extend(
                bundle
                    .source(&id)
                    .map_err(|e| Diagnostic::new(&id.uri, 0, 1, e).with_context(&id))?
                    .dependencies
                    .iter()
                    .cloned(),
            );
        }
    }
    Ok(DefinitionBundle {
        version: bundle.version,
        sources: bundle
            .sources
            .iter()
            .filter(|s| selected.contains(&s.id))
            .cloned()
            .collect(),
        namespaces: bundle
            .namespaces
            .iter()
            .filter(|n| selected.contains(&n.source))
            .cloned()
            .collect(),
    })
}
fn visit(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
    active: &mut BTreeSet<SourceId>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    validation: SourceValidation,
) -> Result<PreparedInference> {
    let fail = |message: &str| Diagnostic::at_source(entry, 0, 1, message);
    if active.len() >= 64 || !active.insert(entry.clone()) {
        return Err(fail("Cyclic or over-deep inference grammar inheritance"));
    }
    let source = bundle
        .source(entry)
        .map_err(|e| fail(&e).with_context(entry))?;
    let syntax = syntax::parse(source, limits)?;
    if validation.durable_documents {
        let tail = syntax.name.value.rsplit('.').next().unwrap_or("");
        if !tail.chars().next().is_some_and(char::is_uppercase) {
            return Err(Diagnostic::at_source(
                entry,
                syntax.name.start,
                syntax.name.end,
                "The last element should start with an upper case letter.",
            ));
        }
        if !syntax.name.value.contains('.') {
            return Err(Diagnostic::at_source(
                entry,
                syntax.name.start,
                syntax.name.end,
                "You must use a namespace.",
            ));
        }
    }
    if syntax.parents.len() > 1 {
        return Err(fail("Pinned Xtext permits only one parent grammar"));
    }
    let parent = if let Some(name) = syntax.parents.first() {
        let mut selected = None;
        for id in &source.dependencies {
            let candidate = bundle.source(id).map_err(|e| fail(&e))?;
            if candidate.kind != SourceKind::Xtext {
                continue;
            }
            let candidate_syntax = syntax::parse(candidate, limits)?;
            if candidate_syntax.name.value == name.value && selected.replace(id.clone()).is_some() {
                return Err(fail("Ambiguous exact parent grammar dependency"));
            }
        }
        let id = selected.ok_or_else(|| fail("Missing exact parent grammar dependency"))?;
        let selected = closure(bundle, &id)?;
        let parent = visit(&selected, &id, limits, active, conversions, validation)?;
        if parent
            .grammars()
            .iter()
            .any(|g| g.name == syntax.name.value)
        {
            return Err(fail("Duplicate grammar identity in inference chain"));
        }
        Some(parent)
    } else {
        None
    };
    let mut world = bundle.clone();
    if let Some(parent) = &parent {
        for grammar in parent.grammars() {
            for generated in &grammar.generated_sources {
                if let Some(existing) = world.sources.iter().find(|s| s.id.uri == generated.id.uri)
                {
                    if existing != generated {
                        return Err(fail("Inherited generated source identity collision"));
                    }
                } else {
                    world.sources.push(generated.clone());
                }
            }
        }
        for binding in &parent.bundle().namespaces {
            if let Some(existing) = world
                .namespaces
                .iter()
                .find(|b| b.namespace == binding.namespace)
            {
                if existing != binding {
                    return Err(fail("Inherited generated namespace selection conflicts"));
                }
            } else {
                world.namespaces.push(binding.clone());
            }
        }
    }
    let result = infer_local(
        &world,
        entry,
        limits,
        parent.as_ref(),
        conversions,
        validation,
    )?;
    active.remove(entry);
    Ok(result)
}
