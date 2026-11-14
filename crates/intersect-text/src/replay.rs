//! Source-authoritative package replay. Inference remains language-owned.
use crate::compiler;
use crate::grammar::SourceLimits;
use crate::inference::InferenceRequest;
use crate::package::{CompilationSourcePolicy, Package, XtextInferenceInputs};
use intersect_model::definitions::schema::Diagnostic;

pub fn recompile(package: &Package) -> Result<Package, Vec<Diagnostic>> {
    recompile_with_source_policy(package, CompilationSourcePolicy::default())
}
/// Rebuild under a host-selected ceiling; retained policy cannot authorize itself.
pub fn recompile_with_source_policy(
    package: &Package,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    recompile_with_context(package, &[], None, host)
}
/// Rebuild original inputs with explicit host contracts and conversions.
pub fn recompile_with_context(
    package: &Package,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    let Some(inputs) = &package.xtext_inference else {
        return compiler::recompile_with_frontend(
            package,
            settings,
            host,
            |compilation, profile, requested, settings| {
                let mut rebuilt = crate::ordinary::compile_with_context(
                    compilation,
                    profile,
                    settings,
                    conversions,
                    requested.unwrap_or_default(),
                    host,
                )?;
                rebuilt.source_policy = requested;
                Ok(rebuilt)
            },
        );
    };
    let failure = |message: &str| vec![Diagnostic::new(&inputs.grammar.uri, 0, 0, message)];
    if !settings.is_empty()
        && package
            .grammar
            .as_ref()
            .map_or(&[][..], |grammar| grammar.computed_settings.as_slice())
            != settings
    {
        return Err(failure(
            "Explicit computed-setting contracts do not match retained requirements",
        ));
    }
    if package.version != 2 {
        return Err(failure("Unsupported package version"));
    }
    if package.compilation.is_some() {
        return Err(failure(
            "Inferred packages cannot select ordinary compilation inputs",
        ));
    }
    let requested = package
        .source_policy
        .ok_or_else(|| failure("Missing inference source policy"))?;
    compile_inputs_with_context(inputs, requested, host, settings, conversions)
}
pub(crate) fn compile_inputs_with_source_policy(
    inputs: &XtextInferenceInputs,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
) -> Result<Package, Vec<Diagnostic>> {
    compile_inputs_with_context(inputs, requested, host, &[], None)
}
/// Rebuild an original inference recipe with caller-owned providers under independent host ceilings.
/// Serialized generated schemas and grammar programs are never replay authority.
pub fn compile_inputs_with_context(
    inputs: &XtextInferenceInputs,
    requested: CompilationSourcePolicy,
    host: CompilationSourcePolicy,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
) -> Result<Package, Vec<Diagnostic>> {
    let failure = |message: &str| vec![Diagnostic::new(&inputs.grammar.uri, 0, 0, message)];
    if !host.admits(requested) {
        return Err(failure(
            "Requested source policy exceeds host admission limits",
        ));
    }
    // The inference API currently has one explicit byte limit for both imported
    // structure and grammar sources. Do not broaden either side while replaying.
    if requested.structure_bytes != requested.xtext_bytes {
        return Err(failure("Inference source byte policies must agree"));
    }
    InferenceRequest {
        version: inputs.version,
        bundle: inputs.bundle.clone(),
        grammar: inputs.grammar.clone(),
    }
    .compile_with_selected_profile(
        SourceLimits {
            source_bytes: requested.xtext_bytes,
            declarations: requested.xtext_declarations,
        },
        inputs.profile,
        settings,
        conversions,
        inputs.behavior.as_ref(),
    )
    .map(|language| language.into_package())
}
