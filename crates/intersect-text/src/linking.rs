//! Language-neutral installation point for language-owned scoping policies.
//! This module implements no KerML/SysML name, namespace or import algorithm.
use crate::package::Package;
use intersect_model::core::{
    query::QueryBudget,
    storage::{ModelRead, Query},
};
use serde_json::Value;

#[path = "linking_conformance.rs"]
mod conformance;

/// Exact pending text reference; source coordinates retain the parser's meaning.
#[derive(Clone, Copy, Debug)]
pub struct ReferenceRequest<'a> {
    pub owner: &'a str,
    pub feature: &'a str,
    pub spelling: &'a str,
    pub target_class: &'a str,
    pub uri: &'a str,
    pub start: usize,
    pub end: usize,
}

/// Read-only language policy. Validate exact schema/source contracts before use.
/// Return candidates in deterministic order; distinct paths to one visible object
/// must be deduplicated by the policy. The shared linker performs final type and
/// reference checks. Returning no candidates never invokes a global fallback.
/// Unknown semantics must return an error instead of fabricating a candidate.
pub trait ReferenceCandidates {
    /// A freshly source-compiled scope selection, when required by the package.
    /// The shared worker invokes this verified selection directly; a legacy
    /// name provider cannot silently replace a source-authored binding.
    fn rcore_scopes(&self) -> Option<&crate::rcore_scopes::RcoreScopes<'_>> {
        None
    }
    /// Validate this actual borrowed definition before any parser/store work.
    fn validate_definition(
        &self,
        package: crate::package::TextPackageView<'_>,
    ) -> Result<(), String>;
    /// Compatibility entry point for callers retaining an ordinary package DTO.
    fn validate_package(&self, package: &Package) -> Result<(), String> {
        self.validate_definition(package.into())
    }
    fn candidates(
        &self,
        store: &dyn ModelRead,
        reference: &ReferenceRequest<'_>,
        work: &mut QueryBudget,
    ) -> Result<Vec<String>, String>;
}

/// Transient caller-owned context; never serialized or retained in the parser.
/// Core charges one unit per reference and returned candidate. A policy charges
/// its own query/traversal work to this same budget, without refunds. This logical
/// budget does not qualify physical payload copies, arbitrary provider bodies,
/// cancellation, storage cursor work or the remaining S09 obligations.
pub struct LinkingContext<'a> {
    pub resolver: &'a dyn ReferenceCandidates,
    pub work: &'a mut QueryBudget,
}

/// Explicit adapter for the existing global unique-name lookup. Old APIs keep
/// their exact original path, without constructing this adapter or new budgets.
pub struct UniqueNames;
impl ReferenceCandidates for UniqueNames {
    fn validate_definition(
        &self,
        package: crate::package::TextPackageView<'_>,
    ) -> Result<(), String> {
        if package.semantics.scope != "unique_names" {
            return Err("UniqueNames requires the package's unique_names scope".into());
        }
        Ok(())
    }
    fn candidates(
        &self,
        store: &dyn ModelRead,
        reference: &ReferenceRequest<'_>,
        work: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        work.charge()?;
        store.query(Query::Value(
            "name",
            &Value::String(reference.spelling.to_owned()),
        ))
    }
}

// One shared worker. Compatibility policy preserves its exact callback/fee order.
pub(crate) trait PendingPolicy<S: crate::storage::ModelStore> {
    fn before_owner(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn candidates(
        &mut self,
        store: &S,
        request: &ReferenceRequest<'_>,
        owner: &str,
    ) -> Result<Vec<String>, String>;
    fn before_candidate(&mut self) -> Result<(), String>;
}
pub(crate) struct LegacyPending<'a, 'b> {
    pub(crate) context: Option<&'a mut LinkingContext<'b>>,
}
impl<S: crate::storage::ModelStore> PendingPolicy<S> for LegacyPending<'_, '_> {
    fn candidates(
        &mut self,
        store: &S,
        request: &ReferenceRequest<'_>,
        _: &str,
    ) -> Result<Vec<String>, String> {
        if let Some(context) = self.context.as_deref_mut() {
            context.work.charge()?;
            if let Some(scopes) = context.resolver.rcore_scopes() {
                scopes.candidates(store, request, context.work)
            } else {
                context.resolver.candidates(store, request, context.work)
            }
        } else {
            store.query(Query::Value(
                "name",
                &Value::String(request.spelling.to_owned()),
            ))
        }
    }
    fn before_candidate(&mut self) -> Result<(), String> {
        if let Some(context) = self.context.as_deref_mut() {
            context.work.charge()?;
        }
        Ok(())
    }
}
pub(crate) fn link_pending<S: crate::storage::ModelStore, P: PendingPolicy<S>>(
    schema: &intersect_model::definitions::schema::Schema,
    pending: Vec<crate::Pending>,
    store: &mut S,
    policy: &mut P,
    output: &mut crate::StoredEvaluation,
    reuse_metadata: bool,
) -> Result<(), String> {
    let mut access = OrdinaryPendingAccess { store, policy };
    link_pending_with_access(schema, pending, &mut access, output, reuse_metadata)
}

// The scheduling/decision body below is shared, including duplicates, diagnostics
// and output order. The specialized adapter cannot masquerade as ModelStore.
pub(crate) trait PendingAccess {
    fn before_owner(&mut self) -> Result<(), String>;
    fn class(&mut self, object: &str) -> Result<Option<String>, String>;
    fn candidates(
        &mut self,
        request: &ReferenceRequest<'_>,
        actual: &str,
    ) -> Result<Vec<String>, String>;
    fn before_candidate(&mut self) -> Result<(), String>;
    fn resolve(
        &mut self,
        pending: &crate::Pending,
        actual: &str,
        target: &str,
    ) -> Result<(), String>;
}
struct OrdinaryPendingAccess<'a, S: crate::storage::ModelStore, P: PendingPolicy<S>> {
    store: &'a mut S,
    policy: &'a mut P,
}
impl<S: crate::storage::ModelStore, P: PendingPolicy<S>> PendingAccess
    for OrdinaryPendingAccess<'_, S, P>
{
    fn before_owner(&mut self) -> Result<(), String> {
        self.policy.before_owner()
    }
    fn class(&mut self, object: &str) -> Result<Option<String>, String> {
        self.store.class(object)
    }
    fn candidates(
        &mut self,
        request: &ReferenceRequest<'_>,
        actual: &str,
    ) -> Result<Vec<String>, String> {
        self.policy.candidates(self.store, request, actual)
    }
    fn before_candidate(&mut self) -> Result<(), String> {
        self.policy.before_candidate()
    }
    fn resolve(&mut self, pending: &crate::Pending, _: &str, target: &str) -> Result<(), String> {
        self.store
            .resolve_reference(&pending.owner, &pending.field, pending.index, target)
    }
}
pub(crate) fn link_pending_with_access<A: PendingAccess>(
    schema: &intersect_model::definitions::schema::Schema,
    pending: Vec<crate::Pending>,
    access: &mut A,
    output: &mut crate::StoredEvaluation,
    reuse_metadata: bool,
) -> Result<(), String> {
    let mut metadata = reuse_metadata.then(|| conformance::ConformanceCache::new(schema));
    for reference in pending.into_iter().filter(|reference| reference.active) {
        let mut targets = Vec::new();
        let mut unresolved = None;
        access.before_owner()?;
        let owner_class = access
            .class(&reference.owner)?
            .ok_or("Missing reference owner")?;
        let candidates = access.candidates(
            &ReferenceRequest {
                owner: &reference.owner,
                feature: &reference.field,
                spelling: &reference.spelling,
                target_class: &reference.target,
                uri: &reference.uri,
                start: reference.start,
                end: reference.end,
            },
            &owner_class,
        )?;
        for id in candidates {
            access.before_candidate()?;
            let class = access.class(&id)?.ok_or("Missing indexed name target")?;
            let matches_target = match &mut metadata {
                Some(cache) => cache.class_matches(&class, &reference.target),
                None => schema.conforms(&class, &reference.target),
            };
            if matches_target {
                let conforms = match &mut metadata {
                    Some(cache) => cache.reference_matches(&owner_class, &reference.field, &class),
                    None => schema.reference_conforms(&owner_class, &reference.field, &class),
                };
                match conforms {
                    Ok(true) => targets.push(id),
                    Ok(false) => {}
                    Err(message) => unresolved = Some(message),
                }
            }
        }
        if let Some(message) = unresolved {
            output.diagnostics.push(crate::Diagnostic::new(
                &reference.uri,
                reference.start,
                reference.end,
                message,
            ));
            continue;
        }
        if targets.len() != 1 {
            output.diagnostics.push(crate::Diagnostic::new(
                &reference.uri,
                reference.start,
                reference.end,
                format!(
                    "{} reference '{}' ({})",
                    if targets.is_empty() {
                        "Unresolved"
                    } else {
                        "Ambiguous"
                    },
                    reference.spelling,
                    reference.target
                ),
            ));
            continue;
        }
        let target = &targets[0];
        access.resolve(&reference, &owner_class, target)?;
        output.references.push(crate::Reference {
            uri: reference.uri.to_string(),
            start: reference.start,
            end: reference.end,
            target: target.clone(),
        });
    }
    Ok(())
}
