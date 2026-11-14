//! Handwritten adapters for text recipes over the model migration service.
//! Rewriting retained selections grants no compilation or origin authority.
use crate::package::BundleCompilation;
use intersect_model::migration::BundleMigrationReport;

/// Carry an ordinary text recipe across an explicit whole-bundle migration.
/// Exact source and nested type rewriting use the same service as model recipes.
/// Native completion provenance and replay verification remain separate inputs.
pub fn remap_compilation(
    report: &BundleMigrationReport,
    original: &BundleCompilation,
) -> Result<BundleCompilation, String> {
    let mapping = report.selection_remapping(&original.bundle)?;
    let materialized_types = mapping.materialized_types(&original.materialized_types)?;
    Ok(BundleCompilation {
        bundle: mapping.converted().clone(),
        ecore: mapping.source_id(&original.ecore)?,
        namespace: original.namespace.clone(),
        xtext: original
            .xtext
            .as_ref()
            .map(|id| mapping.source_id(id))
            .transpose()?,
        rcore_behavior: mapping.source_id(&original.rcore_behavior)?,
        materialized_types,
    })
}
