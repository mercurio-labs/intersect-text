//! Native source fixtures for private grammar-backend controls.
//! Public frontend controls stay beside their compiler.
use super::*;
use intersect_model::definitions::declarations::DeclarationCollection;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_model::definitions::model_definition::CompiledStructure;
use intersect_model::definitions::model_definition::ModelCompilation;
use intersect_model::definitions::model_definition::ModelPreparationMode;
use intersect_model::definitions::model_definition::PhysicalInputCapture;

/// Preserve the original Ecore fixture bytes while using public model services.
/// Grammar inspection consumes the migrated structure; this grants no admission.
pub(crate) fn inspect_ecore_schema(source: &str) -> Result<Schema, String> {
    use intersect_model::definitions::{
        bundle::{
            BUNDLE_VERSION, DefinitionBundle, DefinitionSource, NamespaceBinding, SourceKind,
        },
        compiler, rcore,
    };
    let report = intersect_model::migration::migrate_ecore(
        DefinitionSource::new("memory:ecore", SourceKind::Ecore, source),
        "memory:grammar-fixture.rcore",
    )
    .map_err(|error| error.to_string())?;
    let converted = report.converted.ok_or_else(|| {
        format!(
            "Grammar fixture migration was incomplete: {:?}",
            report.issues
        )
    })?;
    let namespace =
        rcore::declared_namespace(&converted).map_err(|errors| format!("{errors:?}"))?;
    let entry = converted.id.clone();
    let bundle = DefinitionBundle {
        version: BUNDLE_VERSION,
        sources: vec![converted],
        namespaces: vec![NamespaceBinding {
            namespace: namespace.clone(),
            source: entry.clone(),
        }],
    };
    compiler::compile_structure_bundle(&bundle, &entry, &namespace)
        .map_err(|error| error.to_string())
}

pub(crate) fn runtime(
    input: &ModelCompilation,
    syntax: &intersect_model::definitions::bundle::SourceId,
    work_budget: GrammarAnalysisWorkBudget,
    validate: &AuthoredSourceValidator<'_>,
) -> Result<CompiledRuntimeGrammar, String> {
    let mut model = CompiledStructure::prepare(
        input.structure_selection(),
        DeclarationCollection::Classifiers,
        PhysicalInputCapture::None,
        None,
        ParseLimits::default(),
    )
    .map_err(|error| error.to_string())?
    .check_behavior(&input.behavior, None, ParseLimits::default())
    .map_err(|errors| format!("{errors:?}"))?;
    let limits = SourceLimits {
        source_bytes: 256_000,
        declarations: 128,
    };
    let imports = PackageImports {
        collect_classifiers: true,
        validate_authored: Some(validate),
        authored_source_limits: Some(limits),
        work_budget,
        ..PackageImports::default()
    };
    let grammar = compile_runtime_sources_with_package_imports(
        &input.bundle,
        syntax,
        &mut model,
        Profile::NativeV80,
        limits,
        &[],
        imports,
        None,
        None,
        RecursiveFrontierOrder::Historical,
        None,
        RuntimeGrammarRequest {
            policy: RuntimeGuardedPolicy::InputCheckedCompletionScopes,
            work_budget,
            call_depth: DEFAULT_CALL_DEPTH,
        },
    )
    .map_err(|error| match error {
        RuntimeGrammarError::Diagnostic(error) => format!("{error:?}"),
        RuntimeGrammarError::NoObligations => {
            "No productive linear whole-Full runtime obligation was admitted".into()
        }
        RuntimeGrammarError::Publication(message) => message,
    })?;
    // Preserve behavior checking/lowering from the former whole-compiler fixture.
    model
        .finish(
            ParseLimits::default(),
            false,
            ModelPreparationMode::Strict,
            &[],
        )
        .map_err(|errors| format!("{errors:?}"))?;
    Ok(grammar)
}
