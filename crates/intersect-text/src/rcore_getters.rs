//! Handwritten grammar-aware getter activation over checked model sources.
//! Getter checking, bytecode and execution remain in intersect-model.
use {
    crate::package::Package, crate::preparation::PreparedLanguageDefinition,
    intersect_model::definitions::ecore::ParseLimits,
    intersect_model::definitions::model_definition::CheckedStructure,
    intersect_model::definitions::model_definition::StructureSelection,
    intersect_model::definitions::rcore_behavior::ir_v2::Limits,
    intersect_model::rcore_behavior_functions::ExecutionLimits,
};

pub struct RcoreGetters<'a>(intersect_model::rcore_getters::RcoreGetters<'a>);
impl<'a> RcoreGetters<'a> {
    pub fn from_package(
        package: &'a Package,
        verification: &Limits,
        limits: ExecutionLimits,
        source_limits: ParseLimits,
    ) -> Result<Self, String> {
        if package.xtext_inference.is_some() {
            return Err("Source getter activation for inferred metamodel recipes requires an explicit authored structure selection".into());
        }
        let recipe = package
            .compilation
            .as_ref()
            .ok_or("Source getter activation requires retained compilation sources")?;
        let mut checked = CheckedStructure::prepare(
            StructureSelection {
                bundle: &recipe.bundle,
                entry: &recipe.ecore,
                namespace: &recipe.namespace,
                materialized_types: &recipe.materialized_types,
            },
            source_limits,
        )?;
        if let Some(entry) = &recipe.xtext {
            let grammar = package
                .grammar
                .as_ref()
                .ok_or("Missing retained grammar metadata")?;
            if crate::grammar::source_reference_context_requirement(
                &recipe.bundle,
                entry,
                checked.schema(),
                grammar.profile,
                crate::grammar::SourceLimits {
                    source_bytes: source_limits.source_bytes,
                    declarations: source_limits.source_bytes,
                },
            )
            .map_err(|error| error.message)?
            .is_some()
            {
                checked.enable_eobject_references()?;
            }
        }
        intersect_model::rcore_getters::RcoreGetters::from_checked_structure(
            package.model_definition(),
            checked,
            &recipe.rcore_behavior,
            verification,
            limits,
            source_limits,
        )
        .map(Self)
    }
    pub fn from_prepared(
        definition: &'a PreparedLanguageDefinition<'_>,
        verification: &Limits,
        limits: ExecutionLimits,
        source_limits: ParseLimits,
    ) -> Result<Self, String> {
        intersect_model::rcore_getters::RcoreGetters::from_prepared(
            definition,
            verification,
            limits,
            source_limits,
        )
        .map(Self)
    }
}
impl<'a> std::ops::Deref for RcoreGetters<'a> {
    type Target = intersect_model::rcore_getters::RcoreGetters<'a>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
