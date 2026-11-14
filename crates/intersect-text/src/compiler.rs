//! Handwritten ordinary source assembly and replay. No deserialized IR grants execution.
//! Ordinary text orchestration belongs to this package; model services stay below it.
use crate::grammar;
use crate::package::{BundleCompilation, CompilationSourcePolicy, Package};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    schema::*,
};
// Model compilation stays available internally without exporting every present
// and future Model compiler helper through the Text API.
/// Compatibility path used by downstream language packages. New Model-only
/// callers should import this from `intersect_model::definitions::compiler`.
pub use intersect_model::definitions::compiler::compile_structure_bundle_with_limits;
pub(crate) use intersect_model::definitions::compiler::*;
#[path = "compiler_ordinary.rs"]
mod ordinary_compiler;
pub use ordinary_compiler::*;

#[allow(clippy::too_many_arguments)]
fn compile_default_context(
    compilation: BundleCompilation,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> Result<Package, Vec<Diagnostic>> {
    compile_bundle_with_context_and_budget(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        None,
        None,
        work_budget,
    )
}

/// Compile a text language over an authored Rcore metamodel through the shared
/// bundle compiler. Sources are retained exactly; no generated Ecore is involved.
pub fn compile_rcore_language(
    rcore: &str,
    xtext: &str,
    behavior: &str,
) -> Result<Package, Vec<Diagnostic>> {
    compile_rcore_inputs(
        rcore,
        Some(xtext),
        Some(behavior),
        None,
        grammar::Profile::default(),
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
/// Compile structure and behavior from the same authored Rcore source.
pub fn compile_rcore_project(rcore: &str, xtext: &str) -> Result<Package, Vec<Diagnostic>> {
    compile_rcore_inputs(
        rcore,
        Some(xtext),
        None,
        None,
        grammar::Profile::default(),
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
/// Compile a structural Rcore package with optional text syntax and explicit
/// behavior source. Exact sources and selected host contracts are retained.
pub fn compile_rcore_with_profile(
    rcore: &str,
    xtext: Option<&str>,
    behavior: Option<&str>,
    profile: grammar::Profile,
) -> Result<Package, Vec<Diagnostic>> {
    compile_rcore_inputs(
        rcore,
        xtext,
        behavior,
        None,
        profile,
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
/// `compile_rcore_with_profile` with an explicit finite grammar-analysis work
/// grant instead of `GrammarAnalysisWorkBudget::default()`. Source, depth and
/// storage limits are unchanged; `LEGACY_10K` is explicit regression scope.
pub fn compile_rcore_with_profile_and_budget(
    rcore: &str,
    xtext: Option<&str>,
    behavior: Option<&str>,
    profile: grammar::Profile,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> Result<Package, Vec<Diagnostic>> {
    compile_rcore_inputs(rcore, xtext, behavior, None, profile, work_budget)
}
/// Compile authored Rcore with an explicit datatype/provider registry.
pub fn compile_rcore_with_conversions_and_profile(
    rcore: &str,
    xtext: Option<&str>,
    behavior: Option<&str>,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
    profile: grammar::Profile,
) -> Result<Package, Vec<Diagnostic>> {
    compile_rcore_inputs(
        rcore,
        xtext,
        behavior,
        Some(conversions),
        profile,
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
fn compile_rcore_inputs(
    rcore: &str,
    xtext: Option<&str>,
    behavior: Option<&str>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    profile: grammar::Profile,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> Result<Package, Vec<Diagnostic>> {
    let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, rcore);
    let namespace =
        intersect_model::definitions::rcore::declared_namespace(&model).map_err(|errors| {
            errors
                .into_iter()
                .map(|e| Diagnostic::from_span(e.span, e.message))
                .collect::<Vec<_>>()
        })?;
    let mut sources = vec![model.clone()];
    let grammar = xtext.map(|text| {
        let mut source = DefinitionSource::new("memory:language.xtext", SourceKind::Xtext, text);
        source.dependencies.push(model.id.clone());
        let id = source.id.clone();
        sources.push(source);
        id
    });
    let semantics = if let Some(behavior) = behavior {
        let mut source =
            DefinitionSource::new("memory:behavior.rcore", SourceKind::RcoreBehavior, behavior);
        source.dependencies.push(model.id.clone());
        let id = source.id.clone();
        sources.push(source);
        id
    } else {
        model.id.clone()
    };
    // Same route as `compile_bundle_context`, with the caller's analysis grant.
    compile_default_context(
        BundleCompilation {
            materialized_types: vec![],
            ecore: model.id.clone(),
            namespace,
            xtext: grammar,
            rcore_behavior: semantics,
            bundle: intersect_model::definitions::bundle::DefinitionBundle {
                version: 1,
                sources,
                namespaces: vec![],
            },
        },
        conversions,
        profile,
        None,
        CompilationSourcePolicy::default(),
        &[],
        work_budget,
    )
}
/// Rebuild executable definitions crossing an adapter boundary from exact sources.
/// Serialized schema, grammar and semantic programs are never executed directly.
pub fn recompile(package: &Package) -> Result<Package, Vec<Diagnostic>> {
    recompile_with_source_policy(package, CompilationSourcePolicy::default())
}
/// Rebuild with an explicit host ceiling; retained policy never grants itself admission.
pub fn recompile_with_source_policy(
    package: &Package,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    recompile_with_settings_and_source_policy(package, &[], host)
}
/// Rebuild with host-selected setting requirements; serialized metadata cannot
/// select a provider or authorize a different contract by itself.
pub fn recompile_with_settings_and_source_policy(
    package: &Package,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    recompile_with_frontend(
        package,
        settings,
        host,
        |compilation, profile, requested, retained| {
            compile_default_context(
                compilation,
                None,
                profile,
                requested,
                host,
                retained,
                grammar::GrammarAnalysisWorkBudget::default(),
            )
        },
    )
}

/// Validate retained ordinary inputs before invoking an installed language
/// frontend. Source policy and contracts remain controlled by the host.
pub fn recompile_with_frontend(
    package: &Package,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    host: CompilationSourcePolicy,
    frontend: impl FnOnce(
        BundleCompilation,
        grammar::Profile,
        Option<CompilationSourcePolicy>,
        &[intersect_model::definitions::setting::ComputedSettingContract],
    ) -> Result<Package, Vec<Diagnostic>>,
) -> Result<Package, Vec<Diagnostic>> {
    if package.xtext_inference.is_some() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            0,
            "Inferred Xtext packages require the intersect-text source replay frontend",
        )]);
    }
    let retained = package
        .grammar
        .as_ref()
        .map_or(&[][..], |g| g.computed_settings.as_slice());
    if !settings.is_empty() && retained != settings {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            0,
            "Explicit computed-setting contracts do not match retained requirements",
        )]);
    }
    if !host.admits(package.source_policy.unwrap_or_default()) {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            0,
            "Requested source policy exceeds host admission limits",
        )]);
    }
    if package.version != 2 {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            0,
            "Unsupported package version",
        )]);
    }
    let compilation = package.compilation.clone().ok_or_else(|| {
        vec![Diagnostic::new(
            "bundle",
            0,
            0,
            "Missing retained compilation inputs",
        )]
    })?;
    if compilation.xtext.is_none()
        && (package.grammar.is_some() || package.sources.contains_key("xtext"))
    {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            0,
            "Text package is missing authoritative grammar inputs; inferred packages require their original source request",
        )]);
    }
    frontend(
        compilation,
        package
            .grammar
            .as_ref()
            .map_or(grammar::Profile::default(), |g| g.profile),
        package.source_policy,
        settings,
    )
}

/// Extract retained sources after source recompilation matches every cached field.
/// This replaces the grammar-dependent DefinitionBundle::from_package helper.
pub fn bundle_from_package(package: &Package) -> Result<DefinitionBundle, String> {
    let rebuilt = recompile(package)
        .map_err(|errors| format!("Definition compilation failed: {errors:?}"))?;
    if serde_json::to_value(&rebuilt).map_err(|error| error.to_string())?
        != serde_json::to_value(package).map_err(|error| error.to_string())?
    {
        return Err("Compiled metadata differs from retained source".into());
    }
    package
        .compilation
        .as_ref()
        .map(|recipe| recipe.bundle.clone())
        .ok_or_else(|| "Missing retained compilation inputs".into())
}
