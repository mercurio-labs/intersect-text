//! Handwritten ordinary-source frontend. The original compilation is authority;
//! generated parameter programs only feed the existing shared compiler.
use super::*;
use crate::package::{BundleCompilation, CompilationSourcePolicy, Package};

/// Compile exact ordinary inputs with the shared model compiler and the installed
/// Xtext parameter frontend. Earlier profiles retain their original contracts.
pub fn compile(
    compilation: BundleCompilation,
    profile: grammar::Profile,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    compile_with_source_policy(
        compilation,
        profile,
        CompilationSourcePolicy::default(),
        CompilationSourcePolicy::default(),
    )
}

/// Source admission remains bounded by an independent host ceiling. Imported
/// definitions stay sealed and ordinary inputs never become an inference recipe.
pub fn compile_with_source_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    compile_with_context(compilation, profile, &[], None, requested, host)
}

/// Explicit host contracts and datatype conversions are passed to the existing
/// model/behavior compiler; source metadata cannot install executable providers.
pub fn compile_with_context(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    compile_with_context_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        None,
    )
}

/// Explicit compiler search only. All authored validation, source/provider
/// admission, graph, per-context fees and original limits remain shared. A
/// finite search may reach a different real witness/error; success is unproven.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_context_frontier_selected(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    order: grammar::RecursiveFrontierOrder,
) -> std::result::Result<Package, Vec<Diagnostic>> {
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
    compile_with_context_prefix_impl_frontier_selected(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        observation,
        None,
        order,
    )
}

/// Explicit single-pass diagnostic for a real source occurrence. Diagnostic
/// retention consumes that occurrence's original FIRST pool, so this opt-in may
/// deny earlier; it never resets the FOLLOW/recursive pools or grants admission.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_context_prefix_inspected(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: &grammar::SourcePrefixDiagnostic<'_>,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    if !matches!(
        profile,
        grammar::Profile::NativeV77
            | grammar::Profile::NativeV78
            | grammar::Profile::NativeV79
            | grammar::Profile::NativeV80
            | grammar::Profile::NativeV81
            | grammar::Profile::NativeV82
    ) {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Source prefix diagnostics require an explicit source-retaining profile",
        )]);
    }
    compile_with_context_prefix_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        observation,
        Some(prefix),
    )
}
/// Inspect complete ordinary definitions and unmet language behavior without
/// constructing an executable Package. This initial boundary supports durable,
/// unparameterized ordinary sources; other frontends reject explicitly.
#[allow(clippy::too_many_arguments)] // Existing explicit source/provider context plus borrowed original migration provenance.
pub fn prepare_with_context<'inputs>(
    compilation: &'inputs BundleCompilation,
    origin: &'inputs intersect_model::definitions::ecore_migration::BundleMigrationReport,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> std::result::Result<crate::preparation::PreparedLanguageDefinition<'inputs>, Vec<Diagnostic>> {
    let entry = compilation.xtext.as_ref().ok_or_else(|| {
        vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Inspection preparation requires an authoritative Xtext entry",
        )]
    })?;
    if !profile.has_durable_documents() {
        return Err(vec![Diagnostic::at_source(
            entry,
            0,
            1,
            "Inspection preparation requires an explicit durable ordinary profile",
        )]);
    }
    let (_, requires_program) =
        inspect_ordinary_inputs(compilation, entry, profile, requested, host)?;
    if requires_program {
        return Err(vec![Diagnostic::at_source(
            entry,
            0,
            1,
            "Inspection preparation for parameter programs is unsupported",
        )]);
    }
    let validate = |bundle: &DefinitionBundle, entry: &SourceId, limits: SourceLimits| {
        sources::prepare_with_profile_and_conversions(bundle, entry, limits, profile, conversions)
            .map(|_| ())
    };
    compiler::prepare_bundle_with_borrowed_authored_validation(
        compilation,
        origin,
        profile,
        settings,
        conversions,
        requested,
        host,
        &validate,
    )
}

fn inspect_ordinary_inputs<'a>(
    compilation: impl Into<crate::package::TextCompilationView<'a>>,
    entry: &SourceId,
    profile: grammar::Profile,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> std::result::Result<(SourceLimits, bool), Vec<Diagnostic>> {
    let compilation = compilation.into();
    let failure = |message: &str| vec![Diagnostic::at_source(entry, 0, 1, message)];
    if !host.admits(requested) {
        return Err(failure(
            "Requested source policy exceeds host admission limits",
        ));
    }
    compilation
        .bundle
        .validate()
        .map_err(|error| failure(&error))?;
    for source in &compilation.bundle.sources {
        let limit = match source.kind {
            SourceKind::Rcore => requested.structure_bytes,
            SourceKind::Xtext => requested.xtext_bytes,
            SourceKind::Ecore => {
                return Err(vec![Diagnostic::at_source(
                    &source.id,
                    0,
                    source.text.len(),
                    "Migrate Ecore explicitly before ordinary compilation",
                )]);
            }
            _ => 256_000,
        };
        if source.text.len() > limit {
            return Err(vec![Diagnostic::at_source(
                &source.id,
                0,
                source.text.len(),
                "Definition size budget exceeded",
            )]);
        }
    }
    let limits = SourceLimits {
        source_bytes: if profile.has_terminal_cardinality_decisions() {
            // Per-kind checks above precede any source-world preparation. Side
            // sources have their own preexisting 256k bound, not the Xtext cap.
            requested
                .structure_bytes
                .max(requested.xtext_bytes)
                .max(256_000)
        } else {
            requested.structure_bytes.max(requested.xtext_bytes)
        },
        declarations: requested.xtext_declarations,
    };
    // Preserve the ordinary compiler path when no consumed source needs a
    // parameter program; declaration libraries need no inferred-model context.
    let mut pending = vec![entry.clone()];
    let mut visited = BTreeSet::new();
    let mut requires_program = false;
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let source = compilation
            .bundle
            .source(&id)
            .map_err(|error| failure(&error))?;
        if source.kind != SourceKind::Xtext {
            continue;
        }
        let syntax = syntax::parse(source, limits).map_err(|error| vec![error])?;
        requires_program |= parameter_program::requires(&syntax);
        pending.extend(source.dependencies.iter().cloned());
    }
    Ok((limits, requires_program))
}

/// Observe the first returned grammar-analysis phase without changing admission.
/// The initial observation scope covers ordinary, unparameterized durable profiles.
/// Other frontends preserve their result and report explicit missing coverage.
#[allow(clippy::too_many_arguments)] // Existing provider/source contract plus a separate in-process observation adapter.
pub fn compile_with_context_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: &grammar::admission_observation::PredictionObservation,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    observation.reset();
    compile_with_context_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(grammar::admission_observation::AnalysisObservers::phases(observation).into()),
    )
}

/// Measure actual recursive-analysis work with separate caller-owned counters.
/// Source admission, semantic ceilings and old observer output stay unchanged.
pub fn compile_with_context_measured(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observers: grammar::admission_observation::AnalysisObservers<'_>,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    observers.reset();
    compile_with_context_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(observers.into()),
    )
}

/// Observe every charged unit of the existing V76 parser-analysis pool.
/// Earlier phase/FOLLOW pools and runtime parsing are outside this sidecar.
pub fn compile_with_context_work_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observers: grammar::admission_observation::AnalysisWorkObservers<'_>,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    observers.reset();
    compile_with_context_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(observers),
    )
}

#[allow(clippy::too_many_arguments)] // Keep the existing compilation body shared with the opt-in observer.
fn compile_with_context_impl(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    compile_with_context_prefix_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        observation,
        None,
    )
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
fn compile_with_context_prefix_impl(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    compile_with_context_prefix_impl_frontier_selected(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        observation,
        prefix,
        grammar::RecursiveFrontierOrder::Historical,
    )
}
#[allow(clippy::too_many_arguments)] // Same worker plus explicit transient search order.
fn compile_with_context_prefix_impl_frontier_selected(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    compile_with_context_prefix_impl_frontier_selected_pool(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        observation,
        prefix,
        order,
        None,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // One transient caller-owned diagnostic; same worker.
fn compile_with_context_prefix_impl_frontier_selected_pool(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: Option<grammar::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&grammar::SourcePrefixDiagnostic<'_>>,
    order: grammar::RecursiveFrontierOrder,
    pool: Option<&grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
    // Explicit analysis grant; `None` keeps every existing route unchanged.
    // Only the `*_and_budget` facades pass `Some`, always with no prefix/pool
    // and the historical order.
    work_budget: Option<grammar::GrammarAnalysisWorkBudget>,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    let plain = |compilation| match work_budget {
        Some(work_budget) => compiler::compile_bundle_with_context_and_budget(
            compilation,
            profile,
            settings,
            conversions,
            requested,
            host,
            None,
            None,
            work_budget,
        ),
        None => compiler::compile_bundle_with_context(
            compilation,
            profile,
            settings,
            conversions,
            requested,
            host,
        ),
    };
    let Some(entry) = compilation.xtext.as_ref() else {
        if let Some(observation) = observation {
            observation.not_instrumented(grammar::admission_observation::Coverage::NoXtext);
        }
        return plain(compilation);
    };
    if !profile.has_constant_guards() {
        if let Some(observation) = observation {
            observation.not_instrumented(grammar::admission_observation::Coverage::LegacyFrontend);
        }
        return plain(compilation);
    }
    let failure = |message: &str| vec![Diagnostic::at_source(entry, 0, 1, message)];
    let (limits, requires_program) =
        inspect_ordinary_inputs(&compilation, entry, profile, requested, host)?;
    if order != grammar::RecursiveFrontierOrder::Historical && requires_program {
        return Err(failure(
            "Recursive frontier selection requires unchanged ordinary sources",
        ));
    }
    if pool.is_some() && requires_program {
        return Err(failure(
            "Compiler pool diagnostics require an unchanged ordinary imported grammar",
        ));
    }
    if prefix.is_some() && requires_program {
        return Err(failure(
            "Source prefix diagnostics require an unchanged ordinary imported grammar",
        ));
    }
    if !requires_program && profile.has_durable_documents() {
        let validate = |bundle: &DefinitionBundle, entry: &SourceId, limits: SourceLimits| {
            sources::prepare_with_profile_and_conversions(
                bundle,
                entry,
                limits,
                profile,
                conversions,
            )
            .map(|_| ())
        };
        return if let Some(work_budget) = work_budget {
            compiler::compile_bundle_with_context_and_budget(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                Some(&validate),
                observation,
                work_budget,
            )
        } else if let Some(pool) = pool {
            compiler::compile_bundle_with_borrowed_authored_validation_pool_observed(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                &validate,
                observation,
                order,
                pool,
            )
        } else if order != grammar::RecursiveFrontierOrder::Historical {
            compiler::compile_bundle_with_borrowed_authored_validation_frontier_selected(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                &validate,
                observation,
                order,
            )
        } else if let Some(prefix) = prefix {
            compiler::compile_bundle_with_borrowed_authored_validation_prefix_inspected(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                &validate,
                observation,
                prefix,
            )
        } else if let Some(observation) = observation {
            compiler::compile_bundle_with_borrowed_authored_validation_work_observed(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                &validate,
                observation,
            )
        } else {
            compiler::compile_bundle_with_borrowed_authored_validation(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                &validate,
            )
        };
    }
    if let Some(observation) = observation {
        observation.not_instrumented(if requires_program {
            grammar::admission_observation::Coverage::SourceProgramFrontend
        } else {
            grammar::admission_observation::Coverage::LegacyFrontend
        });
    }
    if !requires_program && profile.has_terminal_cardinality_decisions() {
        let validate: grammar::AuthoredSourceValidation = if profile.has_durable_documents() {
            validate_document_authored_program
        } else if profile.has_scoped_hidden_tokens() {
            validate_hidden_authored_program
        } else {
            validate_cardinality_authored_program
        };
        return match work_budget {
            Some(work_budget) => compiler::compile_bundle_with_context_and_budget(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                Some(&validate),
                None,
                work_budget,
            ),
            None => compiler::compile_bundle_with_authored_source_validation(
                compilation,
                profile,
                settings,
                conversions,
                requested,
                host,
                validate,
            ),
        };
    }
    if !requires_program {
        return plain(compilation);
    }
    // Authored classification must be checked before specialization can omit or
    // rename declarations. Inference v1 continues to use its legacy policy.
    let prepared = if profile.has_durable_documents() {
        sources::prepare_with_profile_and_conversions(
            &compilation.bundle,
            entry,
            limits,
            profile,
            conversions,
        )
    } else if profile.has_contextual_first_rule() {
        sources::prepare_with_profile(&compilation.bundle, entry, limits, profile)
    } else {
        sources::prepare(&compilation.bundle, entry, limits)
    }
    .map_err(|e| vec![e])?;
    let PreparedInference::Library(library) = prepared else {
        return Err(failure(
            "Ordinary compilation requires imported metamodels; use the inference frontend for generate declarations",
        ));
    };
    let program =
        parameter_program::prepare_imported(&compilation.bundle, entry, &library.grammars, limits)
            .map_err(|e| vec![e])?;
    let Some(program) = program else {
        return plain(compilation);
    };
    if work_budget.is_some() {
        return Err(failure(
            "Explicit grammar-analysis budgets require unchanged ordinary imported sources",
        ));
    }
    let authored = compilation
        .bundle
        .source(entry)
        .map_err(|e| failure(&e))?
        .text
        .clone();
    let mut derived = compilation.clone();
    derived.bundle = program.bundle.clone();
    derived.xtext = Some(program.entry.clone());
    let source_program = grammar::SourceProgram {
        bindings: &program.bindings,
        producers: &program.producers,
    };
    let validate = |bundle: &DefinitionBundle, entry: &SourceId, limits: SourceLimits| {
        sources::prepare_with_profile_and_conversions(bundle, entry, limits, profile, conversions)
            .map(|_| ())
    };
    let mut package = if profile.has_durable_documents() {
        compiler::compile_bundle_with_borrowed_source_program_validation(
            derived,
            profile,
            settings,
            conversions,
            requested,
            host,
            &grammar::BorrowedAuthoredSourceProgramAdapter {
                program: &source_program,
                validate_authored: &validate,
            },
        )
    } else if profile.has_contextual_first_rule() {
        compiler::compile_bundle_with_source_program_and_authored_validation(
            derived,
            profile,
            settings,
            conversions,
            requested,
            host,
            &grammar::AuthoredSourceProgramAdapter {
                program: &source_program,
                validate_authored: if profile.has_durable_documents() {
                    validate_document_authored_program
                } else if profile.has_scoped_hidden_tokens() {
                    validate_hidden_authored_program
                } else if profile.has_terminal_cardinality_decisions() {
                    validate_cardinality_authored_program
                } else {
                    validate_authored_program
                },
            },
        )
    } else {
        compiler::compile_bundle_with_source_program(
            derived,
            profile,
            settings,
            conversions,
            requested,
            host,
            &source_program,
        )
    }
    .map_err(|errors| {
        errors
            .into_iter()
            .map(|error| emit::project_diagnostic(error, &program.origins))
            .collect::<Vec<_>>()
    })?;
    if let Some(grammar) = &mut package.grammar {
        grammar.parameterized_rules = program.parameterized_rules;
        grammar.parameterized_rules.remove(&grammar.entry);
    }
    for source in program.generated {
        package
            .sources
            .insert(format!("generated:xtext:{}", source.id.uri), source.text);
    }
    package.sources.insert("xtext".into(), authored);
    package.compilation = Some(compilation);
    package.xtext_inference = None;
    Ok(package)
}

/// `compile_with_context` with an explicit finite grammar-analysis work grant
/// instead of `GrammarAnalysisWorkBudget::default()`. Source/depth/storage limits
/// and routes are unchanged; `LEGACY_10K` is explicit regression scope. Imported
/// grammars that need a generated parameter program reject explicitly.
pub fn compile_with_context_and_budget(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    compile_with_context_prefix_impl_frontier_selected_pool(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        None,
        None,
        grammar::RecursiveFrontierOrder::Historical,
        None,
        Some(work_budget),
    )
}

/// `compile_with_context_work_observed` with an explicit finite grammar-analysis
/// work grant. Observers are reset once; they never supply the grant.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_context_work_observed_and_budget(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observers: grammar::admission_observation::AnalysisWorkObservers<'_>,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    observers.reset();
    compile_with_context_prefix_impl_frontier_selected_pool(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(observers),
        None,
        grammar::RecursiveFrontierOrder::Historical,
        None,
        Some(work_budget),
    )
}

/// Preserve an optional retained policy while rebuilding under an explicit host
/// ceiling. `None` retains the legacy default-policy wire contract.
pub fn compile_with_optional_source_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: Option<CompilationSourcePolicy>,
    host: CompilationSourcePolicy,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    let mut package = compile_with_context(
        compilation,
        profile,
        settings,
        conversions,
        requested.unwrap_or_default(),
        host,
    )?;
    package.source_policy = requested;
    Ok(package)
}

// Hardwired native callback: re-resolve the exact authored graph on every call.
// No serialized approval, function name or cached classification is consulted.
/// Fresh contextual admission for an exact original ordinary source program.
/// This is a trusted native compiler adapter, not serialized approval or an
/// executable-entry guarantee. It always resolves under explicit NativeV61.
pub fn validate_authored_program(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
) -> Result<()> {
    sources::prepare_with_profile(bundle, entry, limits, grammar::Profile::NativeV61).map(|_| ())
}

/// Fresh V63 source/type admission. Old public V61 policy remains unchanged.
pub fn validate_cardinality_authored_program(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
) -> Result<()> {
    sources::prepare_with_profile(bundle, entry, limits, grammar::Profile::NativeV63).map(|_| ())
}

/// Fresh V64 hidden-scope admission. Earlier fixed public helpers stay unchanged.
pub fn validate_hidden_authored_program(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
) -> Result<()> {
    sources::prepare_with_profile(bundle, entry, limits, grammar::Profile::NativeV64).map(|_| ())
}

/// Explicit document-profile source validation; fixed V61/V63/V64 helpers stay
/// immutable and cannot acquire new admission through a serialized context.
pub fn validate_document_authored_program(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
) -> Result<()> {
    sources::prepare_with_profile(bundle, entry, limits, grammar::Profile::NativeV65).map(|_| ())
}

/// Observe the existing Native80 compiler pool; no allowance, policy or parser
/// is installed. Source preparation and authored validation remain canonical.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_context_pool_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: grammar::admission_observation::AnalysisWorkObservers<'_>,
    pool: &grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80 || compilation.xtext.is_none() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Compiler pool diagnostics require NativeV80 and an authoritative ordinary Xtext source",
        )]);
    }
    compile_with_context_prefix_impl_frontier_selected_pool(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(observation),
        None,
        grammar::RecursiveFrontierOrder::BreadthFirstTransparentPorts,
        Some(pool),
        None,
    )
}

/// Explicit completed-component policy over the same Native80 compiler and
/// original pool. Only graph proofs are reused; semantic effects remain canonical.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_context_composed_ports_pool_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: grammar::admission_observation::AnalysisWorkObservers<'_>,
    pool: &grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80 || compilation.xtext.is_none() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Compiler pool diagnostics require NativeV80 and an authoritative ordinary Xtext source",
        )]);
    }
    compile_with_context_prefix_impl_frontier_selected_pool(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(observation),
        None,
        grammar::RecursiveFrontierOrder::BreadthFirstComposedTransparentPorts,
        Some(pool),
        None,
    )
}

/// Explicit physical full-call-prefix sharing over the same Native80 compiler
/// and original pool; all predicate/return/caller checks stay canonical.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_context_shared_call_ports_pool_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: grammar::admission_observation::AnalysisWorkObservers<'_>,
    pool: &grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80 || compilation.xtext.is_none() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Compiler pool diagnostics require NativeV80 and an authoritative ordinary Xtext source",
        )]);
    }
    compile_with_context_prefix_impl_frontier_selected_pool(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(observation),
        None,
        grammar::RecursiveFrontierOrder::BreadthFirstSharedCallPorts,
        Some(pool),
        None,
    )
}

/// Explicit selected frontier with the existing whole-pool diagnostic. This
/// general entry avoids adding another facade for each transient policy.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_context_frontier_pool_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    observation: grammar::admission_observation::AnalysisWorkObservers<'_>,
    order: grammar::RecursiveFrontierOrder,
    pool: &grammar::admission_observation::pool_blocks::CompilerPoolDiagnostic,
) -> std::result::Result<Package, Vec<Diagnostic>> {
    if profile != grammar::Profile::NativeV80 || compilation.xtext.is_none() {
        return Err(vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Compiler pool diagnostics require NativeV80 and an authoritative ordinary Xtext source",
        )]);
    }
    compile_with_context_prefix_impl_frontier_selected_pool(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        Some(observation),
        None,
        order,
        Some(pool),
        None,
    )
}

/// Explicit native productive-linear Full-repeat runtime admission.
/// Does not catch/retry a failed ordinary compile or grant static disjointness.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_runtime_guarded_decisions(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_with_runtime_guarded_policy(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        grammar::RuntimeGuardedPolicy::LinearFullRepeats,
    )
}
/// Explicit bounded proof policy; unsupported scope/budget errors propagate.
pub fn compile_with_runtime_guarded_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_runtime_guarded_policy_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        policy,
        None,
    )
}
/// Observe the first returned runtime candidate error without changing proof
/// policy, result or semantic limits. Reset once before any input inspection.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_runtime_guarded_policy_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    observation: &grammar::admission_observation::PredictionObservation,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    observation.reset();
    compile_runtime_guarded_policy_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        policy,
        Some(observation),
    )
}
/// Existing bounded work/phase observers on the SAME original compiler route.
/// Reset observations once; work capture grants no semantic allowance.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_runtime_guarded_policy_work_observed(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    observation: grammar::admission_observation::AnalysisObservers<'_>,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    observation.reset();
    compile_runtime_guarded_policy_work_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        policy,
        Some(observation),
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
/// Same ordinary compiler with an explicit finite grammar-analysis work grant.
/// Source/depth/storage limits and runtime commitment policy are unchanged.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_runtime_guarded_policy_and_budget(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_runtime_guarded_policy_work_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        policy,
        None,
        work_budget,
    )
}

/// Reset the same observers once; the explicit policy, never the observer,
/// supplies the finite grammar-analysis grant.
#[allow(clippy::too_many_arguments)]
pub fn compile_with_runtime_guarded_policy_work_observed_and_budget(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    observation: grammar::admission_observation::AnalysisObservers<'_>,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    observation.reset();
    compile_runtime_guarded_policy_work_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        policy,
        Some(observation),
        work_budget,
    )
}
#[allow(clippy::too_many_arguments)]
fn compile_runtime_guarded_policy_impl(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<&grammar::admission_observation::PredictionObservation>,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    compile_runtime_guarded_policy_work_impl(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        policy,
        observation.map(grammar::admission_observation::AnalysisObservers::phases),
        grammar::GrammarAnalysisWorkBudget::default(),
    )
}
#[allow(clippy::too_many_arguments)]
fn compile_runtime_guarded_policy_work_impl(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<grammar::admission_observation::AnalysisObservers<'_>>,
    work_budget: grammar::GrammarAnalysisWorkBudget,
) -> std::result::Result<compiler::RuntimeGuardedCompilation, Vec<Diagnostic>> {
    let entry = compilation.xtext.as_ref().ok_or_else(|| {
        vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Runtime guarded admission requires an authoritative Xtext source",
        )]
    })?;
    if profile != grammar::Profile::NativeV80 {
        return Err(vec![Diagnostic::at_source(
            entry,
            0,
            1,
            "Runtime guarded admission requires NativeV80",
        )]);
    }
    let (_, requires_program) =
        inspect_ordinary_inputs(&compilation, entry, profile, requested, host)?;
    if requires_program {
        return Err(vec![Diagnostic::at_source(
            entry,
            0,
            1,
            "Runtime guarded admission requires unchanged ordinary imported sources",
        )]);
    }
    let validate = |bundle: &DefinitionBundle, entry: &SourceId, limits: SourceLimits| {
        sources::prepare_with_profile_and_conversions(bundle, entry, limits, profile, conversions)
            .map(|_| ())
    };
    compiler::compile_bundle_with_borrowed_runtime_guarded_validation_policy_and_budget(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        &validate,
        policy,
        work_budget,
        observation,
    )
}

/// Explicit DRAFT runtime preparation. Missing imported semantic providers are
/// requirements, not installed behavior. This never returns an executable Package
/// or the strict RuntimeGuardedCompilation accepted by bounded_runtime::evaluate.
#[allow(clippy::too_many_arguments)]
pub fn prepare_with_runtime_guarded_policy(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<&grammar::admission_observation::PredictionObservation>,
) -> std::result::Result<compiler::DraftRuntimeCompilation, Vec<Diagnostic>> {
    prepare_with_runtime_guarded_policy_and_call_depth(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        policy,
        observation,
        grammar::DEFAULT_CALL_DEPTH,
    )
}
/// `prepare_with_runtime_guarded_policy` with the prediction program's
/// combined call depth (`grammar::DEFAULT_CALL_DEPTH` unless the language's
/// nesting needs more; at most `grammar::MAX_CALL_DEPTH`).
#[allow(clippy::too_many_arguments)]
pub fn prepare_with_runtime_guarded_policy_and_call_depth(
    compilation: BundleCompilation,
    profile: grammar::Profile,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    policy: grammar::RuntimeGuardedPolicy,
    observation: Option<&grammar::admission_observation::PredictionObservation>,
    call_depth: usize,
) -> std::result::Result<compiler::DraftRuntimeCompilation, Vec<Diagnostic>> {
    if let Some(observation) = observation {
        observation.reset();
    }
    let entry = compilation.xtext.as_ref().ok_or_else(|| {
        vec![Diagnostic::new(
            "bundle",
            0,
            1,
            "Draft runtime preparation requires an authoritative Xtext source",
        )]
    })?;
    if profile != grammar::Profile::NativeV80 {
        return Err(vec![Diagnostic::at_source(
            entry,
            0,
            1,
            "Draft runtime preparation requires NativeV80",
        )]);
    }
    let (_, requires_program) =
        inspect_ordinary_inputs(&compilation, entry, profile, requested, host)?;
    if requires_program {
        return Err(vec![Diagnostic::at_source(
            entry,
            0,
            1,
            "Draft runtime preparation requires unchanged ordinary imported sources",
        )]);
    }
    let validate = |bundle: &DefinitionBundle, entry: &SourceId, limits: SourceLimits| {
        sources::prepare_with_profile_and_conversions(bundle, entry, limits, profile, conversions)
            .map(|_| ())
    };
    compiler::prepare_bundle_with_borrowed_runtime_guarded_validation_policy_and_call_depth(
        compilation,
        profile,
        settings,
        conversions,
        requested,
        host,
        &validate,
        policy,
        observation,
        call_depth,
    )
}

// Handwritten language-pack facade (step 6a). Not generated.
pub use {
    crate::preparation::SavedSourceBoundDefinition,
    crate::preparation::SourceBoundDraftRuntimeDefinition,
};

/// The arguments a fresh `prepare_with_runtime_guarded_policy_and_call_depth`
/// preparation received that a pack load must reproduce. The NativeV80 profile,
/// grammar/program call-depth agreement and Rcore-only converted sources are
/// already enforced by `SourceBoundDraftRuntimeDefinition::load_host_cache`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinaryPackExpectation {
    /// Requested source policy; the restored one must EQUAL it (admission by
    /// the host is not enough: a smaller saved policy would load tighter).
    pub requested: CompilationSourcePolicy,
    pub policy: grammar::RuntimeGuardedPolicy,
    pub call_depth: usize,
}

/// Restore a source-bound definition from a HOST-BUILT language pack and
/// re-establish the ordinary-input admission a fresh preparation performs.
///
/// Order: `SourceBoundDraftRuntimeDefinition::load_host_cache(saved, host,
/// mode)`; the restored requested policy EQUALS `expected.requested`; the
/// converted selection has an Xtext entry and passes the same
/// `inspect_ordinary_inputs` a fresh preparation runs (host admission, bundle
/// validation, per-kind converted sizes, no Ecore, the Xtext closure parses
/// within the declaration limit and needs no parameter program); the
/// restored runtime admission has `expected.policy` and `expected.call_depth`.
/// In `RuntimeAdmissionLoadMode::Revalidate` the bound migration origin is also
/// replayed (`SourceBoundDraftRuntimeDefinition::replay_origin`).
///
/// Trust: the `VerifiedExecutable::load_trusted` convention (the host's own
/// output; not cryptography). Compilation and choice bindings stay trusted
/// saved data in both modes; rebuilding from sources remains the oracle.
pub fn load_from_pack(
    saved: SavedSourceBoundDefinition,
    host: CompilationSourcePolicy,
    mode: grammar::RuntimeAdmissionLoadMode,
    expected: OrdinaryPackExpectation,
) -> std::result::Result<SourceBoundDraftRuntimeDefinition, Vec<Diagnostic>> {
    let failure = |message: &str| vec![Diagnostic::new("language pack", 0, 1, message)];
    let bound = SourceBoundDraftRuntimeDefinition::load_host_cache(saved, host, mode)?;
    if bound.requested() != expected.requested {
        return Err(failure(
            "Language pack requested source policy differs from its recipe",
        ));
    }
    {
        let definition = bound.prepare(host)?;
        let selection = definition.selection();
        let entry = selection
            .xtext
            .as_ref()
            .ok_or_else(|| failure("Language pack definition has no Xtext entry"))?;
        let (_, requires_program) = inspect_ordinary_inputs(
            selection,
            entry,
            grammar::Profile::NativeV80,
            expected.requested,
            host,
        )?;
        if requires_program {
            return Err(vec![Diagnostic::at_source(
                entry,
                0,
                1,
                "Draft runtime preparation requires unchanged ordinary imported sources",
            )]);
        }
        let admission = definition
            .runtime_admission()
            .ok_or_else(|| failure("Language pack definition has no runtime admission"))?;
        if admission.policy() != expected.policy {
            return Err(failure(
                "Language pack runtime-guarded policy differs from its recipe",
            ));
        }
        if admission.call_depth() != expected.call_depth {
            return Err(failure(
                "Language pack prediction call depth differs from its recipe",
            ));
        }
    }
    if mode == grammar::RuntimeAdmissionLoadMode::Revalidate {
        bound.replay_origin(host)?;
    }
    Ok(bound)
}
