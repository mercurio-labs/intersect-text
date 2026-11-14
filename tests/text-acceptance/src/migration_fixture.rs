//! Text compilation adapters over the shared native Model fixture helpers.
use intersect_model::definitions::{conversion::ConversionRegistry, schema::Diagnostic};
pub use intersect_model_test_support::migration_fixture::*;
use intersect_text::{
    compiler,
    grammar::Profile,
    package::{BundleCompilation, Package},
};
type Result = std::result::Result<Package, Vec<Diagnostic>>;

pub fn migrate_bundle_fixture(
    input: BundleCompilation,
) -> std::result::Result<BundleCompilation, Vec<Diagnostic>> {
    use intersect_model::definitions::bundle::SourceKind;
    let targets = input
        .bundle
        .sources
        .iter()
        .filter(|source| source.kind == SourceKind::Ecore)
        .map(|source| (source.id.clone(), format!("{}.rcore", source.id.uri)))
        .collect();
    let report = intersect_model::migration::migrate_ecore_bundle(input.bundle.clone(), &targets)
        .map_err(|error| {
        vec![Diagnostic::new(
            &error.span.source.uri,
            error.span.start,
            error.span.end,
            error.message,
        )]
    })?;
    intersect_text::migration::remap_compilation(&report, &input)
        .map_err(|message| vec![Diagnostic::new("bundle", 0, 0, message)])
}

fn converted(ecore: &str) -> std::result::Result<String, Vec<Diagnostic>> {
    crate::migrate_model_fixture(ecore)
        .map_err(|message| vec![Diagnostic::new("ecore", 0, ecore.len(), message)])
}

fn inputs(ecore: &str, behavior: &str) -> std::result::Result<(String, String), Vec<Diagnostic>> {
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    let rcore = converted(ecore)?;
    let source = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, &rcore);
    let namespace =
        intersect_model::definitions::rcore::declared_namespace(&source).map_err(|errors| {
            errors
                .into_iter()
                .map(|e| Diagnostic::new(&e.span.source.uri, e.span.start, e.span.end, e.message))
                .collect::<Vec<_>>()
        })?;
    Ok((rcore, canonical_behavior_fixture(behavior, &namespace)?))
}

pub fn compile_migrated_fixture(ecore: &str, xtext: &str, behavior: &str) -> Result {
    let (rcore, behavior) = inputs(ecore, behavior)?;
    compiler::compile_rcore_language(&rcore, xtext, &behavior)
}

pub fn compile_migrated_fixture_with_profile(
    ecore: &str,
    xtext: &str,
    behavior: &str,
    profile: Profile,
) -> Result {
    let (rcore, behavior) = inputs(ecore, behavior)?;
    compiler::compile_rcore_with_profile(&rcore, Some(xtext), Some(&behavior), profile)
}

pub fn compile_migrated_fixture_with_profile_and_budget(
    ecore: &str,
    xtext: &str,
    behavior: &str,
    profile: Profile,
    work_budget: intersect_text::grammar::GrammarAnalysisWorkBudget,
) -> Result {
    let (rcore, behavior) = inputs(ecore, behavior)?;
    compiler::compile_rcore_with_profile_and_budget(
        &rcore,
        Some(xtext),
        Some(&behavior),
        profile,
        work_budget,
    )
}

pub fn compile_migrated_fixture_with_conversions(
    ecore: &str,
    xtext: &str,
    behavior: &str,
    conversions: &ConversionRegistry<'_>,
) -> Result {
    compile_migrated_fixture_with_conversions_and_profile(
        ecore,
        xtext,
        behavior,
        conversions,
        Profile::default(),
    )
}

pub fn compile_migrated_fixture_with_conversions_and_profile(
    ecore: &str,
    xtext: &str,
    behavior: &str,
    conversions: &ConversionRegistry<'_>,
    profile: Profile,
) -> Result {
    let (rcore, behavior) = inputs(ecore, behavior)?;
    compiler::compile_rcore_with_conversions_and_profile(
        &rcore,
        Some(xtext),
        Some(&behavior),
        conversions,
        profile,
    )
}

pub fn compile_migrated_structure_fixture(ecore: &str, behavior: &str) -> Result {
    let (rcore, behavior) = inputs(ecore, behavior)?;
    compiler::compile_rcore_with_profile(
        &rcore,
        None,
        (!behavior.trim().is_empty()).then_some(behavior.as_str()),
        Profile::default(),
    )
}
