//! Handwritten text preparation, source provenance and runtime admission ownership.
use crate::grammar::Grammar;
use crate::package::BundleCompilation;
use crate::package::TextCompilationView;
use intersect_model::definitions::ecore_migration::BundleMigrationReport;
use intersect_model::definitions::model_definition::{BoundModelOrigin, SavedModelOrigin};
pub use intersect_model::definitions::preparation::{
    BuiltinRuleCoverage, CheckedModelOrigin, PreparedModelDefinition, PreparedPersistenceContext,
    PreparedPhysicalMetadata, SemanticRequirement, SemanticRequirementClass,
    SemanticRequirementClasses, SemanticRequirements,
};
use std::borrow::Cow;

/// One compiled structure/grammar with borrowed exact original source authority.
/// No Package conversion, model construction, evaluation or deserialization is
/// provided. Schema/grammar access is structural inspection, not language readiness.
///
/// The public data and prepared-view imports are independently usable:
/// ```
/// use intersect_text::{package::Package, preparation::PreparedLanguageDefinition};
/// let _: Option<&Package> = None;
/// let _: Option<&PreparedLanguageDefinition<'_>> = None;
/// ```
///
/// ```compile_fail
/// use intersect_text::preparation::PreparedLanguageDefinition;
/// use intersect_text::package::Package;
/// fn executable<'a>(value: &'a PreparedLanguageDefinition<'_>) -> &'a Package { value }
/// ```
/// ```compile_fail
/// use intersect_text::preparation::PreparedLanguageDefinition;
/// use intersect_text::package::Package;
/// fn escape(value: PreparedLanguageDefinition<'_>) -> Package { value.into() }
/// ```
pub struct PreparedLanguageDefinition<'inputs> {
    compilation: TextCompilationView<'inputs>,
    origin: &'inputs BundleMigrationReport,
    model: PreparedModelDefinition<'inputs>,
    grammar: Option<Cow<'inputs, Grammar>>,
    runtime_admission: Option<&'inputs crate::grammar::RuntimeGuardedAdmission>,
}
impl PreparedPersistenceContext for PreparedLanguageDefinition<'_> {
    type ContextData<'a>
        = Option<&'a Grammar>
    where
        Self: 'a;
    fn model_preparation(&self) -> &PreparedModelDefinition<'_> {
        &self.model
    }
    fn context_data(&self) -> Self::ContextData<'_> {
        self.grammar()
    }
}
impl<'inputs> std::ops::Deref for PreparedLanguageDefinition<'inputs> {
    type Target = PreparedModelDefinition<'inputs>;
    fn deref(&self) -> &Self::Target {
        &self.model
    }
}
impl<'inputs> PreparedLanguageDefinition<'inputs> {
    pub(crate) fn new(
        compilation: &'inputs BundleCompilation,
        origin: &'inputs BundleMigrationReport,
        model: PreparedModelDefinition<'inputs>,
        grammar: Option<Grammar>,
    ) -> Self {
        Self {
            compilation: compilation.into(),
            origin,
            model,
            grammar: grammar.map(Cow::Owned),
            runtime_admission: None,
        }
    }
    pub fn model(&self) -> &PreparedModelDefinition<'inputs> {
        &self.model
    }
    pub fn grammar(&self) -> Option<&Grammar> {
        self.grammar.as_deref()
    }
    pub fn migration(&self) -> &BundleMigrationReport {
        self.origin
    }
    pub fn selection(&self) -> TextCompilationView<'inputs> {
        self.compilation
    }
    pub fn runtime_admission(&self) -> Option<&crate::grammar::RuntimeGuardedAdmission> {
        self.runtime_admission
    }
}

/// Shared lifetime owner for inspection-only runtime-prepared metadata. Arc
/// ownership is host lifetime management, not a compiler-pool allocation claim.
/// The borrowed view cannot outlive this facade or expose its private backing.
#[derive(Clone)]
pub struct SharedDraftRuntimeLanguageDefinition {
    owner: std::sync::Arc<crate::compiler::DraftRuntimeCompilation>,
}
impl SharedDraftRuntimeLanguageDefinition {
    pub fn new(owner: std::sync::Arc<crate::compiler::DraftRuntimeCompilation>) -> Self {
        Self { owner }
    }
    /// Replay original migration once and retain its binding to this actual
    /// checked model source allocation. No raw metadata supplies this binding.
    pub fn bind_origin(
        self,
        origin: BundleMigrationReport,
        host: crate::package::CompilationSourcePolicy,
    ) -> Result<
        SourceBoundDraftRuntimeDefinition,
        Vec<intersect_model::definitions::schema::Diagnostic>,
    > {
        let (package, _) = self.owner.prepared_parts();
        let requested = package
            .source_policy
            .copied()
            .ok_or_else(|| failure("Missing retained source policy"))?;
        let compilation = package
            .compilation
            .ok_or_else(|| failure("Missing retained compilation"))?;
        validate_origin_header(compilation, &origin, requested, host)?;
        let origin = BoundModelOrigin::replay(
            self.owner.model_owner(),
            origin,
            intersect_model::definitions::ecore::ParseLimits {
                source_bytes: requested.structure_bytes,
            },
        )
        .map_err(|message| failure(&message))?;
        Ok(SourceBoundDraftRuntimeDefinition {
            shared: self,
            origin,
            requested,
        })
    }
    /// Consume the native migrator's completion token. Exact original policy,
    /// converted-world and source-owner checks stay in force without replay.
    pub fn bind_completed_origin(
        self,
        completed: intersect_model::definitions::ecore_migration::CompletedBundleMigration,
        host: crate::package::CompilationSourcePolicy,
    ) -> Result<
        SourceBoundDraftRuntimeDefinition,
        Vec<intersect_model::definitions::schema::Diagnostic>,
    > {
        let (package, _) = self.owner.prepared_parts();
        let requested = package
            .source_policy
            .copied()
            .ok_or_else(|| failure("Draft runtime definition has no retained source policy"))?;
        let compilation = package
            .compilation
            .ok_or_else(|| failure("Draft runtime definition has no retained compilation"))?;
        validate_origin_header(compilation, completed.report(), requested, host)?;
        let origin = BoundModelOrigin::from_completed(self.owner.model_owner(), completed)
            .map_err(|message| failure(&message))?;
        Ok(SourceBoundDraftRuntimeDefinition {
            shared: self,
            origin,
            requested,
        })
    }
    pub fn prepare<'view>(
        &'view self,
        origin: &'view BundleMigrationReport,
        host: crate::package::CompilationSourcePolicy,
    ) -> Result<
        PreparedLanguageDefinition<'view>,
        Vec<intersect_model::definitions::schema::Diagnostic>,
    > {
        let (package, _) = self.owner.prepared_parts();
        let requested = package
            .source_policy
            .copied()
            .ok_or_else(|| failure("Draft runtime definition has no retained source policy"))?;
        let compilation = package
            .compilation
            .ok_or_else(|| failure("Draft runtime definition has no retained compilation"))?;
        let checked = validate_origin(compilation, origin, requested, host)?;
        let model = self
            .owner
            .model_owner()
            .prepared_definition()
            .with_origin(checked)
            .map_err(|message| failure(&message))?;
        self.borrow_model(origin, model)
    }
    fn borrow_model<'view>(
        &'view self,
        origin: &'view BundleMigrationReport,
        model: PreparedModelDefinition<'view>,
    ) -> Result<
        PreparedLanguageDefinition<'view>,
        Vec<intersect_model::definitions::schema::Diagnostic>,
    > {
        let (package, admission) = self.owner.prepared_parts();
        let compilation = package
            .compilation
            .ok_or_else(|| failure("Missing retained compilation"))?;
        Ok(PreparedLanguageDefinition {
            compilation,
            origin,
            model,
            grammar: package.grammar.map(Cow::Borrowed),
            runtime_admission: Some(admission),
        })
    }
}
fn failure(message: &str) -> Vec<intersect_model::definitions::schema::Diagnostic> {
    vec![intersect_model::definitions::schema::Diagnostic::new(
        "draft runtime preparation",
        0,
        1,
        message,
    )]
}

/// An immutable compiled owner paired with its once-verified migration origin.
/// Borrowed views preserve the existing inspection-only authority. There is no
/// raw-schema constructor, model store or executable promotion, and no
/// deserialization except the single host-cache restore of a
/// `SavedSourceBoundDefinition` through `load_host_cache` (see there).
pub struct SourceBoundDraftRuntimeDefinition {
    shared: SharedDraftRuntimeLanguageDefinition,
    origin: BoundModelOrigin,
    requested: crate::package::CompilationSourcePolicy,
}
impl SourceBoundDraftRuntimeDefinition {
    /// Borrow the same immutable definition after checking the current host limit.
    /// Origin was replayed or sealed by the native migrator at binding, or was
    /// restored from this host's own cache by `load_host_cache`; no path
    /// permits a mutable origin to enter this immutable owner.
    pub fn prepare(
        &self,
        host: crate::package::CompilationSourcePolicy,
    ) -> Result<PreparedLanguageDefinition<'_>, Vec<intersect_model::definitions::schema::Diagnostic>>
    {
        if !host.admits(self.requested) {
            return Err(vec![intersect_model::definitions::schema::Diagnostic::new(
                "draft runtime preparation",
                0,
                1,
                "Requested source policy exceeds host admission limits",
            )]);
        }
        let model = self
            .origin
            .prepare(self.shared.owner.model_owner())
            .map_err(|message| failure(&message))?;
        self.shared.borrow_model(self.origin.report(), model)
    }
    /// The requested source policy retained at binding (or restored by
    /// `load_host_cache`). `prepare` only requires that a host ADMITS it; a
    /// language pack loader that must reproduce a fresh preparation requires
    /// it to EQUAL its recipe policy.
    pub fn requested(&self) -> crate::package::CompilationSourcePolicy {
        self.requested
    }
    /// Replay the bound migration origin through the existing complete
    /// replay (`SharedDraftRuntimeLanguageDefinition::prepare`, the check
    /// `bind_origin` runs): the original sources are migrated again and the
    /// result must EQUAL the bound report. No second implementation; this is
    /// the oracle `load_host_cache` skips, offered for a host's revalidating
    /// (test/CI) load of its own cache.
    pub fn replay_origin(
        &self,
        host: crate::package::CompilationSourcePolicy,
    ) -> Result<(), Vec<intersect_model::definitions::schema::Diagnostic>> {
        self.shared.prepare(self.origin.report(), host).map(|_| ())
    }
    // Handwritten host-cache boundary (language-pack step 4). Not generated.
    /// Persist this source-bound definition for a HOST-BUILT language pack:
    /// the compiled draft (`DraftRuntimeCompilation::save`), the bound
    /// migration origin and the retained requested source policy. Nothing is
    /// borrowed out of the owner; the package and origin are copied as data.
    pub fn save(&self) -> Result<SavedSourceBoundDefinition, String> {
        Ok(SavedSourceBoundDefinition {
            compilation: self.shared.owner.save()?,
            origin: self.origin.save_host_cache(),
            requested: self.requested,
        })
    }
    /// Restore a definition previously produced by `save` in this host,
    /// WITHOUT recompiling and WITHOUT replaying the source migration.
    ///
    /// Checks that run, in order:
    /// 1. `DraftRuntimeCompilation::load_host_cache(compilation, mode)`: the
    ///    step-3 package/admission checks (`mode` applies only here).
    /// 2. The loaded package retains a compilation and a source policy, and
    ///    the saved requested policy EQUALS the retained one.
    /// 3. `validate_origin_header`, exactly as `bind_completed_origin` runs it:
    ///    `host` admits the requested policy, the original bundle validates
    ///    (source identities match their texts), every original source fits
    ///    the requested size limits, the report version is 1, and the
    ///    report's converted bundle EQUALS the loaded compilation's bundle.
    /// 4. The loaded compilation's converted bundle validates, which keeps the
    ///    `CompletedCompiledSources` invariant (a validated canonical bundle)
    ///    that compilation established before the save.
    /// 5. `validate_saved_report_envelope`: the saved reports have exactly the
    ///    structure the native migrator gives a completed migration, checked
    ///    without parsing. The converted bundle follows the original position
    ///    by position; pass-through sources keep identity, kind and text; each
    ///    Ecore original has exactly one version-1 report, in bundle order,
    ///    whose original EQUALS it and whose converted source EQUALS the Rcore
    ///    source at that position (text = the report's draft, a distinct new
    ///    URI); dependencies and namespaces are remapped through that position
    ///    map; no issue blocks conversion; and every correspondence row names
    ///    exactly that report's original and converted sources.
    /// 6. `borrow_bound_origin`, the same owner/source association
    ///    `bind_origin` and `bind_completed_origin` run before retaining.
    ///
    /// Skipped: the native migration replay (`validate_origin`'s
    /// `migrate_ecore_bundle_with_limits` and its equality with the report)
    /// and the consumed `CompletedBundleMigration` token, which cannot be
    /// serialized. So the correspondence rows' declaration paths and spans,
    /// the non-blocking issues, and the derivation of each draft (and hence
    /// each converted source) from its original are trusted saved data in
    /// BOTH modes; their report-to-bundle mapping and completeness are not
    /// (step 5). That is covered by the trust contract: the
    /// saved origin came from `save` on a definition this host bound through
    /// either the full replay or the native token, and the caller matched
    /// the pack on compiler identity and section digests. Replaying the
    /// origin through `SharedDraftRuntimeLanguageDefinition::bind_origin`, or
    /// rebuilding the pack from its sources, remains the oracle.
    ///
    /// Trust: the `VerifiedExecutable::load_trusted` contract. This is a
    /// convention, not cryptography; never pass model, network or browser
    /// input. Like `bind_completed_origin`, the host limit is also re-checked
    /// on every `prepare`.
    ///
    /// As for `DraftRuntimeCompilation::load_host_cache`, completed physical
    /// inputs are not restored, so physical metadata, draft construction and
    /// parse views take their cold path. Results are the same, but under a
    /// tight finite observation/reservation budget a loaded definition can
    /// exhaust where a fresh one succeeds. Budget consumption is not part of
    /// the host-cache equivalence guarantee.
    pub fn load_host_cache(
        saved: SavedSourceBoundDefinition,
        host: crate::package::CompilationSourcePolicy,
        mode: crate::grammar::RuntimeAdmissionLoadMode,
    ) -> Result<Self, Vec<intersect_model::definitions::schema::Diagnostic>> {
        let fail = |message: String| {
            vec![intersect_model::definitions::schema::Diagnostic::new(
                "source-bound host cache",
                0,
                1,
                message,
            )]
        };
        let SavedSourceBoundDefinition {
            compilation,
            origin,
            requested,
        } = saved;
        let owner = crate::compiler::DraftRuntimeCompilation::load_host_cache(compilation, mode)?;
        let (package, _) = owner.prepared_parts();
        let compilation = package
            .compilation
            .ok_or_else(|| fail("Loaded draft has no retained compilation".into()))?;
        let retained = package
            .source_policy
            .copied()
            .ok_or_else(|| fail("Loaded draft has no retained source policy".into()))?;
        if retained != requested {
            return Err(fail(
                "Saved requested source policy differs from the loaded draft".into(),
            ));
        }
        validate_origin_header(compilation, origin.report(), requested, host)?;
        let origin = origin.load_host_cache(owner.model_owner()).map_err(fail)?;
        let shared = SharedDraftRuntimeLanguageDefinition::new(std::sync::Arc::new(owner));
        let model = origin
            .prepare(shared.owner.model_owner())
            .map_err(|message| failure(&message))?;
        shared.borrow_model(origin.report(), model)?;
        Ok(Self {
            shared,
            origin,
            requested,
        })
    }
}

/// The persisted form of one `SourceBoundDraftRuntimeDefinition` inside a
/// host-built language pack. Private fields; `SourceBoundDraftRuntimeDefinition::
/// save` produces it, but deserialization can build one from any bytes. That
/// grants nothing: only `SourceBoundDraftRuntimeDefinition::load_host_cache`
/// turns it into a bound definition, under the trusted-host contract there.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSourceBoundDefinition {
    compilation: crate::compiler::SavedDraftRuntimeCompilation,
    origin: SavedModelOrigin,
    requested: crate::package::CompilationSourcePolicy,
}
/// Compiled ordinary definition data for this native host's own language-pack
/// cache. Deserialization grants no fresh source or grammar verification.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedOrdinaryDefinition {
    compiler: String,
    compilation: BundleCompilation,
    origin: BundleMigrationReport,
    model: intersect_model::definitions::model_definition::SavedModelDefinition,
    grammar: Option<Grammar>,
}
impl SavedOrdinaryDefinition {
    /// Restore only this host's own output after matching compiler/recipe
    /// identities and section digests. Model metadata and migration derivation
    /// stay trusted saved data; fresh compilation/replay is the oracle.
    /// Never pass model, network or browser inputs.
    pub fn prepare_host_cache(
        &mut self,
        profile: crate::grammar::Profile,
        policy: crate::package::CompilationSourcePolicy,
    ) -> Result<PreparedLanguageDefinition<'_>, Vec<intersect_model::definitions::schema::Diagnostic>>
    {
        let fail = |message: String| {
            vec![intersect_model::definitions::schema::Diagnostic::new(
                "ordinary host cache",
                0,
                1,
                message,
            )]
        };
        if self.compiler != crate::compiler_identity() {
            return Err(fail(
                "Ordinary definition was built by another compiler".into(),
            ));
        }
        check_ordinary_prediction(self.grammar.as_ref(), profile).map_err(fail)?;
        validate_origin_header((&self.compilation).into(), &self.origin, policy, policy)?;
        let model = self.model.prepare_host_cache(&self.origin).map_err(fail)?;
        let selection = model.structure_selection();
        if selection.bundle != &self.compilation.bundle
            || selection.entry != &self.compilation.ecore
            || selection.namespace != self.compilation.namespace
            || selection.materialized_types != self.compilation.materialized_types
            || model.behavior_source() != &self.compilation.rcore_behavior
        {
            return Err(fail(
                "Saved ordinary selection differs from its model owner".into(),
            ));
        }
        if let Some(prediction) = self
            .grammar
            .as_mut()
            .and_then(|g| g.parser_prediction.as_mut())
        {
            prediction.restore_host_cache_transients(profile);
        }
        Ok(PreparedLanguageDefinition {
            compilation: (&self.compilation).into(),
            origin: &self.origin,
            model,
            grammar: self.grammar.as_ref().map(Cow::Borrowed),
            runtime_admission: None,
        })
    }
}
fn check_ordinary_prediction(
    grammar: Option<&Grammar>,
    profile: crate::grammar::Profile,
) -> Result<(), String> {
    let grammar = grammar.ok_or("Ordinary host cache requires an Xtext grammar")?;
    if grammar.profile != profile || !grammar.profile.has_durable_documents() {
        return Err("Ordinary host cache requires its original durable grammar profile".into());
    }
    let prediction = grammar
        .parser_prediction
        .as_ref()
        .ok_or("Ordinary host cache requires compiled parser prediction")?;
    if prediction.requires_runtime_capability() {
        return Err("Ordinary host cache cannot restore guarded parser prediction".into());
    }
    Ok(())
}
impl PreparedLanguageDefinition<'_> {
    /// Save ordinary compiled prediction and neutral model data for this host's
    /// trusted cache. Guarded definitions retain their source-bound cache path.
    pub fn save_ordinary_host_cache(&self) -> Result<SavedOrdinaryDefinition, String> {
        if self.runtime_admission.is_some() {
            return Err("Guarded definitions require the source-bound runtime cache".into());
        }
        let grammar = self
            .grammar()
            .ok_or("Ordinary host cache requires an Xtext grammar")?;
        check_ordinary_prediction(Some(grammar), grammar.profile)?;
        Ok(SavedOrdinaryDefinition {
            compiler: crate::compiler_identity().into(),
            compilation: self.compilation.to_owned(),
            origin: self.origin.clone(),
            model: self.model.save_host_cache(),
            grammar: self.grammar.as_deref().cloned(),
        })
    }
}
// Host-cache load only (language-pack step 4). A structural envelope over the
// saved per-resource reports; NOT a migration replay and no parsing. It
// requires exactly the shape `migrate_ecore_bundle_with_limits` produces for a
// completed migration: the converted bundle is the original bundle position
// by position, pass-through sources keep their identity, kind and text, each
// Ecore original has exactly one report in bundle order whose converted
// source EQUALS the Rcore source at that position (text = its draft, a new
// distinct URI), dependencies and namespaces are remapped through that same
// position map, no issue blocks conversion, and every correspondence row
// names exactly that report's original and converted sources. The rows'
// paths and spans, drafts' derivation from the originals, and non-blocking
// issues stay trusted saved data (see `load_host_cache`).
fn validate_origin_header(
    compilation: TextCompilationView<'_>,
    origin: &BundleMigrationReport,
    requested: crate::package::CompilationSourcePolicy,
    host: crate::package::CompilationSourcePolicy,
) -> Result<(), Vec<intersect_model::definitions::schema::Diagnostic>> {
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::schema::Diagnostic;
    let fail = |message: String| vec![Diagnostic::new("migration", 0, 1, message)];
    if !host.admits(requested) {
        return Err(fail(
            "Requested source policy exceeds host admission limits".into(),
        ));
    }
    origin.original.validate().map_err(fail)?;
    for source in &origin.original.sources {
        let limit = match source.kind {
            SourceKind::Ecore | SourceKind::Rcore => requested.structure_bytes,
            SourceKind::Xtext => requested.xtext_bytes,
            _ => 256_000,
        };
        if source.text.len() > limit {
            return Err(fail("Definition size budget exceeded".into()));
        }
    }
    if origin.version != 1 || origin.converted.as_ref() != Some(compilation.bundle) {
        return Err(fail(
            "Preparation requires the exact converted migration bundle".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_origin<'inputs>(
    compilation: TextCompilationView<'inputs>,
    origin: &'inputs BundleMigrationReport,
    requested: crate::package::CompilationSourcePolicy,
    host: crate::package::CompilationSourcePolicy,
) -> Result<CheckedModelOrigin<'inputs>, Vec<intersect_model::definitions::schema::Diagnostic>> {
    validate_origin_header(compilation, origin, requested, host)?;
    CheckedModelOrigin::check(
        compilation.bundle,
        origin,
        intersect_model::definitions::ecore::ParseLimits {
            source_bytes: requested.structure_bytes,
        },
    )
    .map_err(|message| {
        vec![intersect_model::definitions::schema::Diagnostic::new(
            "migration",
            0,
            1,
            message,
        )]
    })
}

#[cfg(test)]
#[path = "preparation_host_cache_tests.rs"]
mod host_cache_tests;
