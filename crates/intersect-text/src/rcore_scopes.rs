//! Source-selected Rcore scope functions over the existing checked linker.
//! No language naming, import, visibility or shadowing algorithm belongs here.
use crate::linking::{ReferenceCandidates, ReferenceRequest};
#[cfg(test)]
use crate::package::Package;
#[cfg(test)]
use intersect_model::source_model_reads::observed_member;
use intersect_model::{
    core::{query::QueryBudget, storage::ModelRead},
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceId, SourceKind},
        ecore::ParseLimits,
        rcore_behavior::ir_v2::Limits,
        rcore_scope::{VerifiedScope, compile_scope_with_source_limits, source_scope_bindings},
        schema::ConformancePreparation,
    },
    rcore_behavior_functions::{ExecutionContext, ExecutionLimits},
    scope_runtime::{self, ScopeRead},
    semantic_bindings::BorrowedBindingWork,
    source_model_reads::SourceModelReads,
};
use std::collections::BTreeMap;

fn retained_behavior_selection<'a>(
    package: crate::package::TextPackageView<'a>,
) -> Result<Option<(&'a DefinitionBundle, Option<&'a SourceId>)>, String> {
    match (package.compilation, package.xtext_inference) {
        (Some(_), Some(_)) => Err("Package has conflicting retained compilation recipes".into()),
        (Some(recipe), None) => Ok(Some((recipe.bundle, Some(recipe.rcore_behavior)))),
        (None, Some(recipe)) => Ok(Some((&recipe.bundle, recipe.behavior.as_ref()))),
        (None, None) => Ok(None),
    }
}

fn selected_behavior_source<'a>(
    bundle: &'a DefinitionBundle,
    selected: &SourceId,
) -> Result<&'a DefinitionSource, String> {
    let mut matches = bundle
        .sources
        .iter()
        .filter(|source| source.id == *selected);
    let source = matches
        .next()
        .ok_or("Retained scope behavior source is missing")?;
    if matches.next().is_some() {
        return Err("Retained scope behavior source is duplicated".into());
    }
    Ok(source)
}

/// Compare cached selectors with fresh syntax from the retained selected entry.
/// Component packages without a retained recipe keep explicit bundle selection.
#[cfg(test)]
fn validate_retained_scope_bindings(
    package: &Package,
    source_limits: ParseLimits,
) -> Result<(), String> {
    validate_retained_scope_bindings_inner(
        crate::package::TextPackageView::from_package(package),
        Some(source_limits),
    )
}

/// Descriptor-only replay for an already retained package at evaluation entry.
/// The existing retained text determines parse storage; this grants no source
/// admission allowance to executable compilation or scope activation.
#[cfg(test)]
fn validate_retained_scope_bindings_for_evaluation(package: &Package) -> Result<(), String> {
    validate_retained_scope_bindings_for_definition(crate::package::TextPackageView::from_package(
        package,
    ))
}

pub(crate) fn validate_retained_scope_bindings_for_definition(
    package: crate::package::TextPackageView<'_>,
) -> Result<(), String> {
    validate_retained_scope_bindings_inner(package, None)
}

fn validate_retained_scope_bindings_inner(
    package: crate::package::TextPackageView<'_>,
    source_limits: Option<ParseLimits>,
) -> Result<(), String> {
    let Some((bundle, selected)) = retained_behavior_selection(package)? else {
        return Ok(());
    };
    let expected = if let Some(selected) = selected {
        let source = selected_behavior_source(bundle, selected)?;
        if source_limits.is_none()
            && package.semantics.scope_bindings.is_empty()
            && !source.text.contains("bind")
        {
            // A scope declaration requires the literal Word token `bind`.
            // Absence proves there are no selectors without constructing the
            // unrelated behavior AST. Comments/strings can only cause replay.
            // Still reject edited text retaining an old scoped source identity.
            if !matches!(source.kind, SourceKind::Rcore | SourceKind::RcoreBehavior) {
                return Err("Scope selectors require an authored Rcore source".into());
            }
            source.validate()?;
            return Ok(());
        }
        let limits = source_limits.unwrap_or(ParseLimits {
            source_bytes: source.text.len().max(ParseLimits::default().source_bytes),
        });
        source_scope_bindings(source, limits)
            .map_err(|errors| format!("Retained Rcore scope selector replay failed: {errors:?}"))?
    } else {
        vec![]
    };
    if expected != package.semantics.scope_bindings {
        return Err(
            "Retained Rcore scope selectors differ from the selected authored entry".into(),
        );
    }
    Ok(())
}

/// Prepared source programs and exact receiver/member dispatch for one package.
/// Construction recompiles retained source. No serialized selector or IR grants
/// executable authority, and invocation never exports a model to JSON.
pub struct RcoreScopes<'a> {
    package: crate::package::TextPackageView<'a>,
    bundle: &'a DefinitionBundle,
    scopes: Vec<(VerifiedScope, String)>,
    dispatch: BTreeMap<(String, String), usize>,
    limits: ExecutionLimits,
}

impl<'a> RcoreScopes<'a> {
    /// Prepare a package's exact source-authored bindings using an explicit bundle.
    pub fn from_bundle(
        package: impl Into<crate::package::TextPackageView<'a>>,
        bundle: &'a DefinitionBundle,
        verification: &Limits,
        limits: ExecutionLimits,
    ) -> Result<Self, String> {
        Self::from_bundle_with_source_limits(
            package,
            bundle,
            verification,
            limits,
            ParseLimits::default(),
        )
    }

    /// The same preparation with an explicit host-owned source admission ceiling.
    pub fn from_bundle_with_source_limits(
        package: impl Into<crate::package::TextPackageView<'a>>,
        bundle: &'a DefinitionBundle,
        verification: &Limits,
        limits: ExecutionLimits,
        source_limits: ParseLimits,
    ) -> Result<Self, String> {
        let package = package.into();
        validate_retained_scope_bindings_inner(package, Some(source_limits))?;
        if package.semantics.scope_bindings.is_empty() {
            return Err("Package has no source-authored Rcore scope bindings".into());
        }
        let mut scopes = Vec::new();
        let mut dispatch = BTreeMap::new();
        for selector in &package.semantics.scope_bindings {
            let scope = compile_scope_with_source_limits(
                bundle,
                &selector.source,
                selector.declaration,
                verification,
                source_limits,
            )
            .map_err(|errors| format!("Rcore scope compilation failed: {errors:?}"))?;
            let receiver = package.schema.class_key_for_type(scope.receiver())?;
            let target = package.schema.class_key_for_type(scope.target())?;
            let selected = package.schema.feature_by_id(receiver, scope.member())?;
            if !selected.feature.reference
                || selected.feature.containment
                || selected.feature.target != target
            {
                return Err(
                    "Rcore scope target differs from the package reference contract".into(),
                );
            }
            let index = scopes.len();
            for actual in package.schema.classes.keys() {
                if !package.schema.conforms(actual, receiver) {
                    continue;
                }
                let member = package.schema.feature_by_id(actual, scope.member())?;
                let key = (actual.clone(), member.feature.name.clone());
                if dispatch.insert(key, index).is_some() {
                    return Err(
                        "Conflicting Rcore scope bindings for one effective reference".into(),
                    );
                }
            }
            scopes.push((scope, target.to_owned()));
        }
        Ok(Self {
            package,
            bundle,
            scopes,
            dispatch,
            limits,
        })
    }

    /// Rebuild source bindings from a package's retained compilation recipe.
    pub fn from_package(
        package: impl Into<crate::package::TextPackageView<'a>>,
        verification: &Limits,
        limits: ExecutionLimits,
    ) -> Result<Self, String> {
        let package = package.into();
        let compilation = package
            .compilation
            .as_ref()
            .ok_or("Rcore scope activation requires retained compilation sources")?;
        Self::from_bundle(package, compilation.bundle, verification, limits)
    }

    pub fn package(&self) -> crate::package::TextPackageView<'a> {
        self.package
    }

    /// Execute over either ordinary or explicitly checked pending observations.
    /// Empty results are final: this service never falls back to global names.
    pub fn resolve_with(
        &self,
        reads: &dyn ScopeRead,
        reference: &ReferenceRequest<'_>,
        budget: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        let work = BorrowedBindingWork::bind(budget);
        let observation = work.observer();
        reads.check(&observation)?;
        let actual = reads.class(reference.owner, &observation)?;
        let index = self
            .dispatch
            .get(&(actual.clone(), reference.feature.to_owned()))
            .ok_or_else(|| format!("No Rcore scope binding for {actual}.{}", reference.feature))?;
        let (scope, target) = &self.scopes[*index];
        let mut conformance =
            ConformancePreparation::new_observed(reference.target_class, target, &observation)
                .map_err(|error| error.to_string())?;
        while !conformance
            .advance_observed(self.package.schema, &observation)
            .map_err(|error| error.to_string())?
        {}
        if !conformance.finish()? {
            return Err("Pending reference target differs from its Rcore scope contract".into());
        }
        let context = ExecutionContext::new(self.bundle);
        scope_runtime::evaluate(
            scope,
            &context,
            self.package.schema,
            reads,
            reference.owner,
            reference.spelling,
            &self.limits,
            &work,
        )
        .map(|result| result.into_identities())
        .map_err(|error| error.to_string())
    }
}

impl ReferenceCandidates for RcoreScopes<'_> {
    fn rcore_scopes(&self) -> Option<&RcoreScopes<'_>> {
        Some(self)
    }
    fn validate_definition(
        &self,
        package: crate::package::TextPackageView<'_>,
    ) -> Result<(), String> {
        if !self.package.same_world(package) {
            return Err("Rcore scope services belong to a different compiled package".into());
        }
        Ok(())
    }
    fn candidates(
        &self,
        store: &dyn ModelRead,
        reference: &ReferenceRequest<'_>,
        work: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        let reads = SourceModelReads::new(store, self.package.schema);
        self.resolve_with(&reads, reference, work)
    }
}

#[cfg(test)]
#[path = "rcore_scopes_tests.rs"]
mod tests;
