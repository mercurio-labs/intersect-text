//! Handwritten ordinary text compiler, source admission and runtime owner orchestration.
//! Grammar lowering still calls the transitional model-hosted backend until X6.
use crate::grammar;
use crate::package::{BundleCompilation, CompilationSourcePolicy, Package};
use crate::package::{TextCompilationView, TextPackageView};
use intersect_model::definitions::{
    bundle::SourceKind, model_definition::ModelPreparationMode as SemanticFinish, schema::*,
};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, SourceId},
    model_definition::OwnedModelDefinition,
};
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn compile_bundle(compilation: BundleCompilation) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_context(compilation, None, grammar::Profile::default())
}
pub fn compile_bundle_with_conversions(
    compilation: BundleCompilation,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_context(compilation, Some(conversions), grammar::Profile::default())
}
pub fn compile_bundle_with_profile(
    compilation: BundleCompilation,
    profile: grammar::Profile,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_context(compilation, None, profile)
}
pub fn compile_bundle_with_conversions_and_profile(
    compilation: BundleCompilation,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
    profile: grammar::Profile,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_context(compilation, Some(conversions), profile)
}
fn compile_bundle_context(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        None,
        CompilationSourcePolicy::default(),
        &[],
        CompilationProgramContext::default(),
    )
}
/// Compile caller-requested policy only within an independent host ceiling.
pub fn compile_bundle_with_source_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        None,
        profile,
        Some(requested),
        host,
        &[],
        CompilationProgramContext::default(),
    )
}
/// Compile exact bundle inputs with explicit computed-setting requirements and
/// an independent host source ceiling. Providers remain a language-host concern.
pub fn compile_bundle_with_settings_and_source_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        None,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext::default(),
    )
}
/// Compile under explicit host requirements, conversions and source admission.
pub fn compile_bundle_with_context(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext::default(),
    )
}
/// Compile original ordinary sources with a fresh language-owned validator.
/// The callback is trusted native code, never an archive-selected certificate.
pub fn compile_bundle_with_authored_source_validation(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: grammar::AuthoredSourceValidation,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(&validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
    )
}
/// Compile ordinary sources under a borrowed trusted language policy. The
/// closure may borrow explicit providers but cannot escape into the package.
pub fn compile_bundle_with_borrowed_authored_validation(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
    )
}
/// Prepare inspected definitions, retaining exact original migration authority.
/// No executable Package is constructed or provider installed by this path.
#[allow(clippy::too_many_arguments)] // Same existing source/provider context plus borrowed migration provenance.
pub fn prepare_bundle_with_borrowed_authored_validation<'inputs>(
    compilation: &'inputs BundleCompilation,
    origin: &'inputs intersect_model::definitions::ecore_migration::BundleMigrationReport,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
) -> Result<crate::preparation::PreparedLanguageDefinition<'inputs>, Vec<Diagnostic>> {
    let checked_origin =
        crate::preparation::validate_origin(compilation.into(), origin, requested, host)?;
    let parts = compile_bundle_parts_observed(
        compilation.into(),
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
        None,
        SemanticFinish::Inspection,
    )?;
    Ok(crate::preparation::PreparedLanguageDefinition::new(
        compilation,
        origin,
        parts
            .model
            .into_prepared_definition()
            .with_origin(checked_origin)
            .map_err(|message| vec![Diagnostic::new("migration", 0, 1, message)])?,
        parts.grammar.map(CompiledGrammarOutput::into_metadata),
    ))
}
/// In-process supplemental grammar-phase observation; the package contract is unchanged.
#[allow(clippy::too_many_arguments)] // Preserve source/provider policy and the borrowed validator alongside an explicit observation adapter.
pub fn compile_bundle_with_borrowed_authored_validation_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    observation: &grammar::admission_observation::PredictionObservation,
) -> Result<Package, Vec<Diagnostic>> {
    observation.reset();
    compile_bundle_with_borrowed_authored_validation_measured(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        grammar::admission_observation::AnalysisObservers::phases(observation),
    )
}

/// Borrowed concrete diagnostic adapters; no executable source authority changes.
/// The caller resets once at its top-level boundary; forwarding never resets stages.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_authored_validation_measured(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    observation: grammar::admission_observation::AnalysisObservers<'_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_with_borrowed_authored_validation_work_observed(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        observation.into(),
    )
}

/// Explicit deterministic search order over the SAME checked compiler. No
/// source or serialized metadata selects it; default facades stay historical.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_authored_validation_frontier_selected(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    order: grammar::RecursiveFrontierOrder,
) -> Result<Package, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80 {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Recursive frontier selection requires NativeV80",
        )]);
    }
    if order != grammar::RecursiveFrontierOrder::Historical && compilation.xtext.is_none() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Recursive frontier selection requires an authoritative ordinary Xtext source",
        )]);
    }
    if let Some(observation) = observation {
        observation.reset();
    }
    compile_bundle_admitted_prefix_inspected_frontier_selected(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
        observation,
        None,
        order,
    )
}

/// Explicit diagnostic sibling. Existing compilation and observer holders are
/// unchanged. Only one original FOLLOW FIRST query may retain bounded facts.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_authored_validation_prefix_inspected(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: &grammar::SourcePrefixDiagnostic<'_>,
) -> Result<Package, Vec<Diagnostic>> {
    prefix
        .begin()
        .map_err(|message| vec![Diagnostic::new("bundle", 0, 1, message)])?;
    compile_bundle_admitted_prefix_inspected(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
        observation,
        Some(prefix),
    )
}
/// Additive V76 work sidecar; no source or executable authority changes.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_authored_validation_work_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    observation: grammar::admission_observation::AnalysisWorkObservers<'_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted_observed(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
        Some(observation),
    )
}

/// Ordinary (not runtime-guarded) compilation with an explicit finite
/// grammar-analysis work grant. With `validate_authored` and `observation` both
/// `None` this is the `compile_bundle_with_context` route; with a validator it is
/// the authored-validation route, and observers behave as in
/// `compile_bundle_with_borrowed_authored_validation_work_observed` (the caller
/// resets them). Every existing facade keeps `GrammarAnalysisWorkBudget::default()`;
/// source/depth/storage limits are unchanged and `LEGACY_10K` is explicit
/// regression scope. Source programs are not accepted here.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_context_and_budget(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: impl Into<Option<CompilationSourcePolicy>>,
    host: CompilationSourcePolicy,
    validate_authored: Option<&grammar::AuthoredSourceValidator<'_>>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted_observed(
        compilation,
        conversions,
        profile,
        requested.into(),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored,
            imported_models: false,
            work_budget,
        },
        observation,
    )
}

pub fn compile_bundle_with_borrowed_source_program_validation(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    adapter: &grammar::BorrowedAuthoredSourceProgramAdapter<'_, '_, '_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: Some(adapter.program),
            validate_authored: Some(adapter.validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
    )
}
/// Compile an explicit in-process language program using the ordinary model and
/// behavior pipeline. Core does not infer or specialize a language on its own.
pub fn compile_bundle_with_source_program(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    program: &grammar::SourceProgram<'_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: Some(program),
            validate_authored: None,
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
    )
}
/// Native language adapter with fresh authored-producer validation. This does
/// not change the existing SourceProgram contract or install source-selected code.
pub fn compile_bundle_with_source_program_and_authored_validation(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    adapter: &grammar::AuthoredSourceProgramAdapter<'_, '_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: Some(adapter.program),
            validate_authored: Some(&adapter.validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
    )
}
/// Compile behavior against exact imported model declarations in an admitted source world.
/// Language frontends must retain their explicit behavior selection for source replay.
pub fn compile_bundle_with_imported_semantics(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: None,
            imported_models: true,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
    )
}
#[derive(Default)]
struct CompilationProgramContext<'a, 'b> {
    program: Option<&'a grammar::SourceProgram<'b>>,
    validate_authored: Option<&'a grammar::AuthoredSourceValidator<'a>>,
    imported_models: bool,
    /// Grammar-analysis grant when no runtime admission supplies one. Every
    /// existing facade keeps `GrammarAnalysisWorkBudget::default()`.
    work_budget: grammar::GrammarAnalysisWorkBudget,
}
fn compile_bundle_admitted(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted_observed(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // Observation is transient and never changes existing compiler context layouts.
fn compile_bundle_admitted_observed(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted_prefix_inspected(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        None,
    )
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
fn compile_bundle_admitted_prefix_inspected(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted_prefix_inspected_frontier_selected(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        prefix,
        grammar::RecursiveFrontierOrder::Historical,
    )
}
#[allow(clippy::too_many_arguments)] // Same worker plus explicit transient search order.
fn compile_bundle_admitted_prefix_inspected_frontier_selected(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted_prefix_inspected_frontier_selected_pool(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        prefix,
        order,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // One transient caller-owned diagnostic; same worker.
fn compile_bundle_admitted_prefix_inspected_frontier_selected_pool(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
    pool: Option<&grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_admitted_prediction_policy(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        prefix,
        order,
        pool,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn compile_bundle_admitted_prediction_policy(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
    pool: Option<&grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
    runtime: Option<grammar::RuntimeGrammarRequest>,
) -> Result<Package, Vec<Diagnostic>> {
    let parts = compile_bundle_parts_prediction_policy(
        (&compilation).into(),
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        SemanticFinish::Strict,
        prefix,
        order,
        pool,
        runtime,
    )?;
    let metadata = parts.model.into_metadata();
    Ok(Package {
        xtext_inference: None,
        source_policy: requested,
        version: 2,
        schema: metadata.schema,
        grammar: parts.grammar.map(CompiledGrammarOutput::into_metadata),
        semantics: metadata.semantics,
        sources: parts.sources,
        compilation: Some(compilation),
    })
}

// Ordinary compilation keeps its existing inline grammar. Boxing that variant
// would add an allocation to every ordinary compiler path solely for layout.
#[allow(clippy::large_enum_variant)]
enum CompiledGrammarOutput {
    Metadata(grammar::Grammar),
    Runtime(grammar::CompiledRuntimeGrammar),
}
impl CompiledGrammarOutput {
    fn grammar(&self) -> &grammar::Grammar {
        match self {
            Self::Metadata(grammar) => grammar,
            Self::Runtime(owner) => owner.grammar(),
        }
    }
    fn into_metadata(self) -> grammar::Grammar {
        match self {
            Self::Metadata(grammar) => grammar,
            Self::Runtime(owner) => owner.into_metadata(),
        }
    }
    fn into_runtime(self) -> Result<grammar::CompiledRuntimeGrammar, String> {
        match self {
            Self::Runtime(owner) => Ok(owner),
            Self::Metadata(_) => Err("Missing fresh runtime grammar owner".into()),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TextCompilationSelection {
    materialized_types: Vec<intersect_model::definitions::generic_types::RuntimeType>,
    ecore: SourceId,
    namespace: String,
    xtext: Option<SourceId>,
    rcore_behavior: SourceId,
}
impl TextCompilationSelection {
    fn split(compilation: BundleCompilation) -> (Self, Arc<DefinitionBundle>) {
        let BundleCompilation {
            materialized_types,
            bundle,
            ecore,
            namespace,
            xtext,
            rcore_behavior,
        } = compilation;
        (
            Self {
                materialized_types,
                ecore,
                namespace,
                xtext,
                rcore_behavior,
            },
            Arc::new(bundle),
        )
    }
    fn borrow<'a>(&'a self, bundle: &'a DefinitionBundle) -> TextCompilationView<'a> {
        TextCompilationView {
            materialized_types: &self.materialized_types,
            bundle,
            ecore: &self.ecore,
            namespace: &self.namespace,
            xtext: self.xtext.as_ref(),
            rcore_behavior: &self.rcore_behavior,
        }
    }
}

struct CompiledDefinitionParts<'inputs> {
    model: intersect_model::definitions::model_definition::CompiledModelDefinition<'inputs>,
    grammar: Option<CompiledGrammarOutput>,
    sources: BTreeMap<String, String>,
}

#[allow(clippy::too_many_arguments)] // Transient shared source/provider/observation context; no runtime holder change.
fn compile_bundle_parts_observed<'inputs>(
    compilation: TextCompilationView<'inputs>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    finish: SemanticFinish,
) -> Result<CompiledDefinitionParts<'inputs>, Vec<Diagnostic>> {
    compile_bundle_parts_prefix_inspected(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        finish,
        None,
    )
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
fn compile_bundle_parts_prefix_inspected<'inputs>(
    compilation: TextCompilationView<'inputs>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    finish: SemanticFinish,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
) -> Result<CompiledDefinitionParts<'inputs>, Vec<Diagnostic>> {
    compile_bundle_parts_prefix_inspected_frontier_selected(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        finish,
        prefix,
        grammar::RecursiveFrontierOrder::Historical,
    )
}
#[allow(clippy::too_many_arguments)] // Same worker plus explicit transient search order.
fn compile_bundle_parts_prefix_inspected_frontier_selected<'inputs>(
    compilation: TextCompilationView<'inputs>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    finish: SemanticFinish,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
) -> Result<CompiledDefinitionParts<'inputs>, Vec<Diagnostic>> {
    compile_bundle_parts_prefix_inspected_frontier_selected_pool(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        finish,
        prefix,
        order,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // One transient caller-owned diagnostic; same worker.
fn compile_bundle_parts_prefix_inspected_frontier_selected_pool<'inputs>(
    compilation: TextCompilationView<'inputs>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    finish: SemanticFinish,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
    pool: Option<&grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
) -> Result<CompiledDefinitionParts<'inputs>, Vec<Diagnostic>> {
    compile_bundle_parts_prediction_policy(
        compilation,
        conversions,
        profile,
        requested,
        host,
        settings,
        context,
        observation,
        finish,
        prefix,
        order,
        pool,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn compile_bundle_parts_prediction_policy<'inputs>(
    compilation: TextCompilationView<'inputs>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: CompilationProgramContext<'_, '_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    finish: SemanticFinish,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
    pool: Option<&grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
    runtime: Option<grammar::RuntimeGrammarRequest>,
) -> Result<CompiledDefinitionParts<'inputs>, Vec<Diagnostic>> {
    let CompilationProgramContext {
        program,
        validate_authored,
        imported_models,
        work_budget,
    } = context;
    let failure = |source: &str, message: String| vec![Diagnostic::new(source, 0, 0, message)];
    let policy = requested.unwrap_or_default();
    if program.is_some() && compilation.xtext.is_none() {
        return Err(failure(
            "bundle",
            "A grammar program requires an authoritative Xtext source".into(),
        ));
    }
    if !host.admits(policy) {
        return Err(failure(
            "bundle",
            "Requested source policy exceeds host admission limits".into(),
        ));
    }
    let byte_limit = |kind: SourceKind| match kind {
        SourceKind::Rcore => policy.structure_bytes,
        SourceKind::Xtext => policy.xtext_bytes,
        _ => 256_000,
    };
    compilation
        .bundle
        .validate()
        .map_err(|e| failure("bundle", e))?;
    let source = |id: &intersect_model::definitions::bundle::SourceId, kind: SourceKind| {
        let source = compilation
            .bundle
            .source(id)
            .map_err(|e| failure(&id.uri, e))?;
        if source.kind != kind {
            return Err(failure(&id.uri, "Incorrect selected source kind".into()));
        }
        if source.text.len() > byte_limit(source.kind) {
            return Err(failure(&id.uri, "Definition size budget exceeded".into()));
        }
        Ok(source)
    };
    let model_kind = compilation
        .bundle
        .source(compilation.ecore)
        .map_err(|e| failure(&compilation.ecore.uri, e))?
        .kind;
    if model_kind != SourceKind::Rcore {
        return Err(failure(
            &compilation.ecore.uri,
            "Selected model source must be Rcore; migrate Ecore explicitly before compilation"
                .into(),
        ));
    }
    if let Some(imported) = compilation
        .bundle
        .sources
        .iter()
        .find(|source| source.kind == SourceKind::Ecore)
    {
        return Err(failure(
            &imported.id.uri,
            "Ecore sources are migration inputs; convert the complete bundle to Rcore before compilation".into(),
        ));
    }
    let ecore = source(compilation.ecore, model_kind)?;
    let xtext = compilation
        .xtext
        .as_ref()
        .map(|id| source(id, SourceKind::Xtext))
        .transpose()?;
    let behavior_kind = compilation
        .bundle
        .source(compilation.rcore_behavior)
        .map_err(|e| failure(&compilation.rcore_behavior.uri, e))?
        .kind;
    if !matches!(behavior_kind, SourceKind::RcoreBehavior | SourceKind::Rcore) {
        return Err(failure(
            &compilation.rcore_behavior.uri,
            "Selected behavior source must be Rcore".into(),
        ));
    }
    let rcore_behavior = source(compilation.rcore_behavior, behavior_kind)?;
    // Explicit imported-model frontends may select behavior already reachable
    // from the model's exact source world. Do not rewrite authored dependencies:
    // they are part of schema identity and remain source-replay authority.
    let mut reachable = std::collections::BTreeSet::new();
    if imported_models {
        let mut pending = vec![ecore.id.clone()];
        while let Some(id) = pending.pop() {
            if reachable.insert(id.clone()) {
                pending.extend(
                    compilation
                        .bundle
                        .source(&id)
                        .map_err(|e| failure(&id.uri, e))?
                        .dependencies
                        .iter()
                        .cloned(),
                );
            }
        }
    }
    for selected in xtext.into_iter().chain(std::iter::once(rcore_behavior)) {
        let imported_behavior =
            imported_models && selected.id == rcore_behavior.id && reachable.contains(&selected.id);
        if selected.id != ecore.id
            && !selected.dependencies.contains(&ecore.id)
            && !imported_behavior
        {
            return Err(failure(
                &selected.id.uri,
                "Selected language source must declare its exact Rcore dependency".into(),
            ));
        }
    }
    let mut package_imports = grammar::PackageImports {
        collect_classifiers: matches!(
            profile,
            grammar::Profile::NativeV45
                | grammar::Profile::NativeV46
                | grammar::Profile::NativeV47
                | grammar::Profile::NativeV48
                | grammar::Profile::NativeV49
                | grammar::Profile::NativeV50
                | grammar::Profile::NativeV51
                | grammar::Profile::NativeV52
                | grammar::Profile::NativeV53
                | grammar::Profile::NativeV54
                | grammar::Profile::NativeV55
                | grammar::Profile::NativeV56
                | grammar::Profile::NativeV57
                | grammar::Profile::NativeV58
                | grammar::Profile::NativeV59
                | grammar::Profile::NativeV60
                | grammar::Profile::NativeV61
                | grammar::Profile::NativeV62
                | grammar::Profile::NativeV63
                | grammar::Profile::NativeV64
                | grammar::Profile::NativeV65
                | grammar::Profile::NativeV66
                | grammar::Profile::NativeV67
                | grammar::Profile::NativeV68
                | grammar::Profile::NativeV69
                | grammar::Profile::NativeV70
                | grammar::Profile::NativeV71
                | grammar::Profile::NativeV72
                | grammar::Profile::NativeV73
                | grammar::Profile::NativeV74
                | grammar::Profile::NativeV75
                | grammar::Profile::NativeV76
                | grammar::Profile::NativeV77
                | grammar::Profile::NativeV78
                | grammar::Profile::NativeV79
                | grammar::Profile::NativeV80
                | grammar::Profile::NativeV81
                | grammar::Profile::NativeV82
        ),
        ..Default::default()
    };
    let checked_imports = matches!(
        profile,
        grammar::Profile::NativeV44
            | grammar::Profile::NativeV45
            | grammar::Profile::NativeV46
            | grammar::Profile::NativeV47
            | grammar::Profile::NativeV48
            | grammar::Profile::NativeV49
            | grammar::Profile::NativeV50
            | grammar::Profile::NativeV51
            | grammar::Profile::NativeV52
            | grammar::Profile::NativeV53
            | grammar::Profile::NativeV54
            | grammar::Profile::NativeV55
            | grammar::Profile::NativeV56
            | grammar::Profile::NativeV57
            | grammar::Profile::NativeV58
            | grammar::Profile::NativeV59
            | grammar::Profile::NativeV60
            | grammar::Profile::NativeV61
            | grammar::Profile::NativeV62
            | grammar::Profile::NativeV63
            | grammar::Profile::NativeV64
            | grammar::Profile::NativeV65
            | grammar::Profile::NativeV66
            | grammar::Profile::NativeV67
            | grammar::Profile::NativeV68
            | grammar::Profile::NativeV69
            | grammar::Profile::NativeV70
            | grammar::Profile::NativeV71
            | grammar::Profile::NativeV72
            | grammar::Profile::NativeV73
            | grammar::Profile::NativeV74
            | grammar::Profile::NativeV75
            | grammar::Profile::NativeV76
            | grammar::Profile::NativeV77
            | grammar::Profile::NativeV78
            | grammar::Profile::NativeV79
            | grammar::Profile::NativeV80
            | grammar::Profile::NativeV81
            | grammar::Profile::NativeV82
    );
    package_imports.validate_authored = validate_authored;
    package_imports.work_budget = work_budget;
    if profile.has_terminal_cardinality_decisions() {
        package_imports.authored_source_limits = Some(grammar::SourceLimits {
            // Other source kinds retain their existing independent 256k cap.
            source_bytes: policy.structure_bytes.max(policy.xtext_bytes).max(256_000),
            declarations: policy.xtext_declarations,
        });
    }
    // Only the private inspection runtime owner retains these actual ordinary
    // compiler outputs. Strict/public/materialized paths preserve old behavior.
    let capture_inputs = matches!(finish, SemanticFinish::Inspection) && runtime.is_some();
    let mut compiled = intersect_model::definitions::model_definition::CompiledStructure::prepare(
        intersect_model::definitions::model_definition::StructureSelection {
            bundle: compilation.bundle,
            entry: compilation.ecore,
            namespace: compilation.namespace,
            materialized_types: compilation.materialized_types,
        },
        if !checked_imports {
            intersect_model::definitions::declarations::DeclarationCollection::None
        } else if package_imports.collect_classifiers {
            intersect_model::definitions::declarations::DeclarationCollection::Classifiers
        } else {
            intersect_model::definitions::declarations::DeclarationCollection::Packages
        },
        if capture_inputs {
            intersect_model::definitions::model_definition::PhysicalInputCapture::Retain
        } else {
            intersect_model::definitions::model_definition::PhysicalInputCapture::None
        },
        conversions,
        intersect_model::definitions::ecore::ParseLimits {
            source_bytes: policy.structure_bytes,
        },
    )
    .map_err(|error| {
        vec![
            Diagnostic::new(
                &error.span.source.uri,
                error.span.start,
                error.span.end,
                error.to_string(),
            )
            .with_span(error.span.clone()),
        ]
    })?;
    if runtime.is_none() {
        package_imports.declarations = compiled.take_declarations();
    }
    for source in &compilation.bundle.sources {
        if source.text.len() > byte_limit(source.kind) {
            return Err(failure(
                &source.id.uri,
                "Definition size budget exceeded".into(),
            ));
        }
    }
    let mut compiled = compiled.check_behavior(
        &rcore_behavior.id,
        conversions,
        intersect_model::definitions::ecore::ParseLimits {
            source_bytes: policy.structure_bytes,
        },
    )?;
    let Some(xtext) = xtext else {
        if !settings.is_empty() {
            return Err(failure(
                "bundle",
                "Text settings require an Xtext grammar".into(),
            ));
        }
        let model = compiled.finish(
            intersect_model::definitions::ecore::ParseLimits {
                source_bytes: byte_limit(behavior_kind),
            },
            imported_models,
            finish,
            settings,
        )?;
        crate::rule_conversion::validate(None, &model.semantics().rule_conversions)
            .map_err(|e| failure(&rcore_behavior.id.uri, e))?;
        let sources = if matches!(finish, SemanticFinish::Strict) {
            [
                ("ecore".into(), ecore.text.clone()),
                ("rcore_behavior".into(), rcore_behavior.text.clone()),
            ]
            .into()
        } else {
            BTreeMap::new()
        };
        return Ok(CompiledDefinitionParts {
            model,
            grammar: None,
            sources,
        });
    };
    let composed = xtext.dependencies.iter().any(|id| {
        compilation
            .bundle
            .source(id)
            .is_ok_and(|source| source.kind == SourceKind::Xtext)
    });
    let (model, grammar) = if program.is_some()
        || composed
        || !settings.is_empty()
        || matches!(
            profile,
            grammar::Profile::NativeV50
                | grammar::Profile::NativeV51
                | grammar::Profile::NativeV52
                | grammar::Profile::NativeV53
                | grammar::Profile::NativeV54
                | grammar::Profile::NativeV55
                | grammar::Profile::NativeV56
                | grammar::Profile::NativeV57
                | grammar::Profile::NativeV58
                | grammar::Profile::NativeV59
                | grammar::Profile::NativeV60
                | grammar::Profile::NativeV61
                | grammar::Profile::NativeV62
                | grammar::Profile::NativeV63
                | grammar::Profile::NativeV64
                | grammar::Profile::NativeV65
                | grammar::Profile::NativeV66
                | grammar::Profile::NativeV67
                | grammar::Profile::NativeV68
                | grammar::Profile::NativeV69
                | grammar::Profile::NativeV70
                | grammar::Profile::NativeV71
                | grammar::Profile::NativeV72
                | grammar::Profile::NativeV73
                | grammar::Profile::NativeV74
                | grammar::Profile::NativeV75
                | grammar::Profile::NativeV76
                | grammar::Profile::NativeV77
                | grammar::Profile::NativeV78
                | grammar::Profile::NativeV79
                | grammar::Profile::NativeV80
                | grammar::Profile::NativeV81
                | grammar::Profile::NativeV82
        ) {
        let limits = grammar::SourceLimits {
            source_bytes: policy.xtext_bytes,
            declarations: policy.xtext_declarations,
        };
        let grammar = if let Some(program) = program {
            if let Some(validate_authored) = validate_authored {
                grammar::compile_source_program_with_borrowed_authored_limits(
                    compilation.bundle,
                    &xtext.id,
                    &mut compiled,
                    profile,
                    limits,
                    settings,
                    &grammar::BorrowedAuthoredSourceProgramAdapter {
                        program,
                        validate_authored,
                    },
                    if profile.has_terminal_cardinality_decisions() {
                        grammar::SourceLimits {
                            source_bytes: policy
                                .structure_bytes
                                .max(policy.xtext_bytes)
                                .max(256_000),
                            declarations: policy.xtext_declarations,
                        }
                    } else {
                        limits
                    },
                )
            } else {
                grammar::compile_source_program(
                    compilation.bundle,
                    &xtext.id,
                    &mut compiled,
                    profile,
                    limits,
                    settings,
                    program,
                )
            }.map(CompiledGrammarOutput::Metadata)
        } else {
            if let Some(request) = runtime {
                grammar::compile_runtime_sources_with_package_imports(
                    compilation.bundle,
                    &xtext.id,
                    &mut compiled,
                    profile,
                    limits,
                    settings,
                    package_imports,
                    observation,
                    prefix,
                    order,
                    pool,
                    request,
                ).map(CompiledGrammarOutput::Runtime).map_err(|error| match error {
                    grammar::RuntimeGrammarError::Diagnostic(error) => error,
                    grammar::RuntimeGrammarError::NoObligations => Diagnostic::new("bundle", 0, 1,
                        if matches!(finish, SemanticFinish::Inspection) {
                            "No bounded runtime obligation was admitted for draft preparation"
                        } else { "No productive linear whole-Full runtime obligation was admitted" }),
                    grammar::RuntimeGrammarError::Publication(message) => Diagnostic::new("bundle", 0, 1, message),
                })
            } else if order != grammar::RecursiveFrontierOrder::Historical || pool.is_some() {
                grammar::compile_sources_with_package_imports_prefix_inspected_frontier_selected_pool(
                    compilation.bundle, &xtext.id, &mut compiled, profile, limits,
                    settings, package_imports, observation, prefix, order, pool,
                ).map(CompiledGrammarOutput::Metadata)
            } else {
                grammar::compile_sources_with_package_imports_prefix_inspected(
                    compilation.bundle,
                    &xtext.id,
                    &mut compiled,
                    profile,
                    limits,
                    settings,
                    package_imports,
                    observation,
                    prefix,
                ).map(CompiledGrammarOutput::Metadata)
            }
        }
        .map_err(|error| vec![error])?;
        let grammar_sources = grammar
            .grammar()
            .declaration_sources
            .values()
            .chain(grammar.grammar().shadowed_declarations.values().flatten())
            .map(|span| &span.source)
            .collect::<std::collections::BTreeSet<_>>();
        for id in grammar_sources {
            let consumed = source(id, SourceKind::Xtext)?;
            if !consumed.dependencies.contains(&ecore.id) {
                return Err(failure(
                    &id.uri,
                    "Consumed grammar source must declare its exact Ecore dependency".into(),
                ));
            }
        }
        let model = compiled
            .finish(
                intersect_model::definitions::ecore::ParseLimits {
                    source_bytes: byte_limit(behavior_kind),
                },
                imported_models,
                finish,
                settings,
            )
            .map_err(|mut errors| {
                for error in &mut errors {
                    error.source = rcore_behavior.id.uri.clone();
                }
                errors
            })?;
        (model, grammar)
    } else {
        (|| {
            let mut grammar = grammar::compile_with_reference_context_and_package_imports(
                &xtext.text,
                &mut compiled,
                profile,
                policy.xtext_declarations,
                package_imports,
            )
            .map_err(|error| vec![error])?;
            if profile.has_rule_conversions() {
                for span in grammar.declaration_sources.values_mut() {
                    span.source = xtext.id.clone();
                }
            }
            let model = compiled.finish(
                intersect_model::definitions::ecore::ParseLimits {
                    source_bytes: byte_limit(behavior_kind),
                },
                imported_models,
                finish,
                settings,
            )?;
            Ok::<_, Vec<Diagnostic>>((model, CompiledGrammarOutput::Metadata(grammar)))
        })()
        .map_err(|mut errors| {
            for error in &mut errors {
                error.source = if error.source == "xtext" {
                    xtext.id.uri.clone()
                } else {
                    rcore_behavior.id.uri.clone()
                };
            }
            errors
        })?
    };
    crate::rule_conversion::validate(Some(grammar.grammar()), &model.semantics().rule_conversions)
        .map_err(|e| vec![Diagnostic::new(&rcore_behavior.id.uri, 0, 1, e)])?;
    let sources = if matches!(finish, SemanticFinish::Strict) {
        [
            ("ecore".into(), ecore.text.clone()),
            ("xtext".into(), xtext.text.clone()),
            ("rcore_behavior".into(), rcore_behavior.text.clone()),
        ]
        .into()
    } else {
        BTreeMap::new()
    };
    Ok(CompiledDefinitionParts {
        model,
        grammar: Some(grammar),
        sources,
    })
}

#[cfg(test)]
#[test]
fn authored_context_layout_after_compiler() {
    println!(
        "authored_context_compiler_layout={{\"context\":{},\"outer_holder\":{}}}",
        std::mem::size_of::<CompilationProgramContext<'_, '_>>(),
        std::mem::size_of::<
            intersect_model::definitions::compiler::OwnedRuntimeSourceArrayPreparation,
        >()
    );
    assert!(
        std::mem::size_of::<
            intersect_model::definitions::compiler::OwnedRuntimeSourceArrayPreparation,
        >() + 128
            <= 2048
    );
}

/// Explicit diagnostic sibling over the same borrowed authored-source worker.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_authored_validation_pool_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    order: grammar::RecursiveFrontierOrder,
    pool: &grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic,
) -> Result<Package, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80
        || !matches!(
            order,
            grammar::RecursiveFrontierOrder::Historical
                | grammar::RecursiveFrontierOrder::BreadthFirstTransparentPorts
                | grammar::RecursiveFrontierOrder::BreadthFirstComposedTransparentPorts
                | grammar::RecursiveFrontierOrder::BreadthFirstSharedCallPorts
        )
        || compilation.xtext.is_none()
    {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Compiler pool diagnostics require a supported explicit NativeV80 source path",
        )]);
    }
    if let Some(observation) = observation {
        observation.reset();
    }
    compile_bundle_admitted_prefix_inspected_frontier_selected_pool(
        compilation,
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
        observation,
        None,
        order,
        Some(pool),
    )
}

/// Explicit compiler-minted runtime obligations; deserialization is not authority.
pub struct RuntimeGuardedCompilation {
    owner: DraftRuntimeCompilation,
}
impl RuntimeGuardedCompilation {
    pub fn parts(&self) -> (TextPackageView<'_>, &grammar::RuntimeGuardedAdmission) {
        self.owner.prepared_parts()
    }
}
/// Opt-in narrow whole-Full runtime admission over the same authored worker.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_runtime_guarded_validation(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
) -> Result<RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_bundle_with_borrowed_runtime_guarded_validation_policy(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        grammar::RuntimeGuardedPolicy::LinearFullRepeats,
    )
}
/// Explicit proof policy over the SAME authored compiler and native worker.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_runtime_guarded_validation_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
) -> Result<RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_bundle_runtime_guarded_policy_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        policy,
        None,
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
/// Borrowed bounded phase diagnostics. The top-level host resets once before
/// input inspection; this forwarding entry never resets an existing failure.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_runtime_guarded_validation_policy_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
    observation: &grammar::admission_observation::PredictionObservation,
) -> Result<RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_bundle_runtime_guarded_policy_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        policy,
        Some(grammar::admission_observation::AnalysisObservers::phases(observation).into()),
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
/// Existing phase/work observers on the SAME runtime-guarded compiler.
/// Diagnostic state supplies no budget, result, graph or admission authority.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_runtime_guarded_validation_policy_work_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
    observation: grammar::admission_observation::AnalysisObservers<'_>,
) -> Result<RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_bundle_runtime_guarded_policy_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        policy,
        Some(observation.into()),
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
/// Select a finite per-existing-pool analysis grant without changing grammar,
/// source, depth, storage or semantic policy. Existing runtime entrypoints use
/// GrammarAnalysisWorkBudget::default(); LEGACY_10K is explicit regression scope.
#[allow(clippy::too_many_arguments)]
pub fn compile_bundle_with_borrowed_runtime_guarded_validation_policy_and_budget(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
    work_budget: grammar::GrammarAnalysisWorkBudget,
    observation: Option<grammar::admission_observation::AnalysisObservers<'_>>,
) -> Result<RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_bundle_runtime_guarded_policy_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        policy,
        observation.map(Into::into),
        work_budget,
    )
}
#[allow(clippy::too_many_arguments)]
fn compile_bundle_runtime_guarded_policy_impl(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> Result<RuntimeGuardedCompilation, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80 || compilation.xtext.is_none() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Runtime guarded admission requires explicit NativeV80 ordinary sources",
        )]);
    }
    let owner = compile_runtime_owner(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        policy,
        observation,
        work_budget,
        grammar::DEFAULT_CALL_DEPTH,
        SemanticFinish::Strict,
    )?;
    Ok(RuntimeGuardedCompilation { owner })
}

/// Inspection-only definition with a compiler-minted bounded prediction owner.
/// Missing imported providers remain explicit requirements. This type grants no
/// executable Package, general evaluator, semantic getter or operation authority.
///
/// ```compile_fail
/// use {
/// intersect_text::compiler::DraftRuntimeCompilation,
/// intersect_text::package::Package,
/// };
/// fn executable(owner: &DraftRuntimeCompilation) -> &Package { owner.parts().0 }
/// ```
/// ```compile_fail
/// use {
/// intersect_text::compiler::DraftRuntimeCompilation,
/// intersect_text::compiler::RuntimeGuardedCompilation,
/// };
/// fn upgrade(owner: DraftRuntimeCompilation) -> RuntimeGuardedCompilation { owner.into() }
/// ```
/// ```compile_fail
/// use {
/// intersect_text::compiler::DraftRuntimeCompilation,
/// };
/// fn unknown_operation(owner: &DraftRuntimeCompilation) { owner.invoke("uninstalled"); }
/// ```
pub struct DraftRuntimeCompilation {
    model: OwnedModelDefinition,
    grammar: grammar::CompiledRuntimeGrammar,
    selection: TextCompilationSelection,
    requested: CompilationSourcePolicy,
    sources: BTreeMap<String, String>,
}
impl DraftRuntimeCompilation {
    pub(crate) fn prepared_parts(
        &self,
    ) -> (TextPackageView<'_>, &grammar::RuntimeGuardedAdmission) {
        (
            TextPackageView {
                version: 2,
                xtext_inference: None,
                source_policy: Some(&self.requested),
                schema: self.model.schema(),
                semantics: self.model.semantics(),
                grammar: Some(self.grammar.grammar()),
                sources: &self.sources,
                compilation: Some(self.selection.borrow(self.model.sources())),
            },
            self.grammar.admission(),
        )
    }
    pub(crate) fn model_owner(&self) -> &OwnedModelDefinition {
        &self.model
    }
    /// Export only this native host's cache data; optional prepared inputs are discarded.
    pub fn save(&self) -> Result<SavedDraftRuntimeCompilation, String> {
        Ok(SavedDraftRuntimeCompilation {
            version: 1,
            model: self.model.save_host_cache(),
            grammar: self.grammar.save()?,
            selection: self.selection.clone(),
            source_policy: Some(self.requested),
            sources: self.sources.clone(),
        })
    }
    /// Restore this host's previous compiler output, matched on compiler identity
    /// and section digests. Never pass model, network or browser input. The saved
    /// requirement set stays trusted in both modes; source recompilation remains
    /// the oracle. Completed physical inputs remain absent after a cache load.
    pub fn load_host_cache(
        saved: SavedDraftRuntimeCompilation,
        mode: grammar::RuntimeAdmissionLoadMode,
    ) -> Result<Self, Vec<Diagnostic>> {
        let fail =
            |message: String| vec![Diagnostic::new("draft runtime host cache", 0, 1, message)];
        let SavedDraftRuntimeCompilation {
            version,
            model,
            grammar,
            selection,
            source_policy,
            sources,
        } = saved;
        if version != 1 {
            return Err(fail(
                "Unsupported saved draft cache version; rebuild from sources".into(),
            ));
        }
        let requested = source_policy.ok_or_else(|| {
            fail("Saved draft package is not an ordinary retained compilation".into())
        })?;
        let grammar =
            grammar::CompiledRuntimeGrammar::load_host_cache(grammar, mode).map_err(fail)?;
        let model = model.load_host_cache().map_err(fail)?;
        let selected = model.structure_selection();
        if selected.entry != &selection.ecore
            || selected.namespace != selection.namespace
            || selected.materialized_types != selection.materialized_types
            || model.behavior_source() != &selection.rcore_behavior
            || selection.xtext.is_none()
        {
            return Err(fail(
                "Saved draft selection differs from its model owner".into(),
            ));
        }
        Ok(Self {
            model,
            grammar,
            selection,
            requested,
            sources,
        })
    }
}

/// The persisted form of one `DraftRuntimeCompilation` inside a host-built
/// language pack. Private fields; `DraftRuntimeCompilation::save` produces
/// it, but deserialization can build one from any bytes. Only
/// `DraftRuntimeCompilation::load_host_cache` turns it into runtime authority,
/// under the trusted-host contract documented there.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedDraftRuntimeCompilation {
    version: u32,
    model: intersect_model::definitions::model_definition::SavedModelDefinition,
    grammar: grammar::SavedRuntimeGrammar,
    selection: TextCompilationSelection,
    source_policy: Option<CompilationSourcePolicy>,
    sources: BTreeMap<String, String>,
}

/// Additive DRAFT preparation over the SAME ordinary compiler, source checks,
/// limits, grammar builder and runtime admission. Inspection permits only absent
/// providers; supplied duplicate/invalid behavior still fails in the old worker.
/// The observation is borrowed and is reset only by the top-level host adapter.
#[allow(clippy::too_many_arguments)]
pub fn prepare_bundle_with_borrowed_runtime_guarded_validation_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<&grammar::admission_observation::PredictionObservation>,
) -> Result<DraftRuntimeCompilation, Vec<Diagnostic>> {
    prepare_bundle_with_borrowed_runtime_guarded_validation_policy_and_call_depth(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        policy,
        observation,
        grammar::DEFAULT_CALL_DEPTH,
    )
}
/// `prepare_bundle_with_borrowed_runtime_guarded_validation_policy` with the
/// prediction program's combined call depth, for grammars whose nesting needs
/// more than `DEFAULT_CALL_DEPTH` (at most `MAX_CALL_DEPTH`).
#[allow(clippy::too_many_arguments)]
pub fn prepare_bundle_with_borrowed_runtime_guarded_validation_policy_and_call_depth(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<&grammar::admission_observation::PredictionObservation>,
    call_depth: usize,
) -> Result<DraftRuntimeCompilation, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80 || compilation.xtext.is_none() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Draft runtime preparation requires explicit NativeV80 ordinary sources",
        )]);
    }
    compile_runtime_owner(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        validate_authored,
        policy,
        observation
            .map(|value| grammar::admission_observation::AnalysisObservers::phases(value).into()),
        grammar::GrammarAnalysisWorkBudget::default(),
        call_depth,
        SemanticFinish::Inspection,
    )
}

#[allow(clippy::too_many_arguments)]
fn compile_runtime_owner(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    validate_authored: &grammar::AuthoredSourceValidator<'_>,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    work_budget: grammar::GrammarAnalysisWorkBudget,
    call_depth: usize,
    finish: SemanticFinish,
) -> Result<DraftRuntimeCompilation, Vec<Diagnostic>> {
    let fail = |message: String| vec![Diagnostic::new("bundle", 0, 1, message)];
    let (selection, sources) = TextCompilationSelection::split(compilation);
    let parts = compile_bundle_parts_prediction_policy(
        selection.borrow(&sources),
        conversions,
        profile,
        Some(requested),
        host,
        settings,
        CompilationProgramContext {
            program: None,
            validate_authored: Some(validate_authored),
            imported_models: false,
            work_budget: grammar::GrammarAnalysisWorkBudget::default(),
        },
        observation,
        finish,
        None,
        grammar::RecursiveFrontierOrder::Historical,
        None,
        Some(grammar::RuntimeGrammarRequest {
            policy,
            work_budget,
            call_depth,
        }),
    )?;
    let CompiledDefinitionParts {
        model,
        grammar,
        sources: source_texts,
    } = parts;
    let grammar = grammar
        .ok_or_else(|| fail("Missing runtime grammar".into()))?
        .into_runtime()
        .map_err(fail)?;
    let model = model.into_owned(Arc::clone(&sources)).map_err(fail)?;
    Ok(DraftRuntimeCompilation {
        model,
        grammar,
        selection,
        requested,
        sources: source_texts,
    })
}
