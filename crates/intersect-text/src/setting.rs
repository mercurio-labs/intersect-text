//! Handwritten grammar-aware setting selection over checked model services.
//! The model layer owns contract checking, reads, mutation and rollback.
pub use intersect_model::setting::{RcoreProjection, SourceBoundSettings, SourceSettingValues};
use {
    crate::package::Package, crate::preparation::PreparedLanguageDefinition,
    intersect_model::core::query::QueryBudget, intersect_model::core::storage::ComputedEdit,
    intersect_model::core::storage::ModelRead, intersect_model::core::storage::ModelStore,
    intersect_model::definitions::bundle::DefinitionBundle,
    intersect_model::definitions::conversion::ConversionRegistry,
    intersect_model::definitions::preparation::PreparedPhysicalMetadata,
    intersect_model::definitions::schema::FeatureLookup,
    intersect_model::definitions::schema::MemberIdentity,
    intersect_model::setting::CheckedSourceSettings,
    intersect_model::storage::ClassInitializationSelector,
    intersect_model::storage::DraftConstructionContext,
    intersect_model::storage::DraftConstructionSource,
    intersect_model::storage::DraftConstructionStore,
};

pub struct TextSettings<'a> {
    package: crate::package::TextPackageView<'a>,
    bound: CheckedSourceSettings<'a>,
}
impl<'a> TextSettings<'a> {
    pub fn new(
        package: impl Into<crate::package::TextPackageView<'a>>,
        bundle: &'a DefinitionBundle,
        providers: &[&'a dyn SourceSettingValues],
    ) -> Result<Self, String> {
        let package = package.into();
        let grammar = package.grammar.ok_or("Missing text grammar")?;
        Ok(Self {
            package,
            bound: CheckedSourceSettings::bind(
                package.schema,
                bundle,
                &grammar.computed_settings,
                providers,
            )?,
        })
    }
    pub fn belongs_to(&self, package: &Package) -> bool {
        self.belongs_to_definition(package.into())
    }
    pub(crate) fn belongs_to_definition(
        &self,
        package: crate::package::TextPackageView<'_>,
    ) -> bool {
        self.package.same_world(package)
    }
    pub fn supports(&self, class: &str, feature: &str) -> bool {
        self.bound.supports(class, feature)
    }
    pub fn read(
        &self,
        store: &dyn ModelRead,
        id: &str,
        class: &str,
        feature: &str,
        budget: &mut QueryBudget,
    ) -> Result<serde_json::Value, String> {
        self.bound.read(store, id, class, feature, budget)
    }
    pub fn apply(
        &self,
        store: &mut impl ModelStore,
        id: &str,
        target: (&str, &str),
        edit: ComputedEdit,
        budget: &mut QueryBudget,
        conversions: Option<&ConversionRegistry<'_>>,
    ) -> Result<(), String> {
        self.bound
            .apply_checked(store, id, target, edit, budget, conversions)
    }
}

/// A checked grammar selection composed with model-owned source contracts.
pub struct CheckedDraftTextSettings<'a> {
    grammar: &'a crate::grammar::Grammar,
    bound: CheckedSourceSettings<'a>,
}
impl<'a> CheckedDraftTextSettings<'a> {
    pub fn bind(
        definition: &'a PreparedLanguageDefinition<'_>,
        providers: &[&'a dyn SourceSettingValues],
    ) -> Result<Self, String> {
        let grammar = definition.grammar().ok_or("Missing text grammar")?;
        Ok(Self {
            grammar,
            bound: CheckedSourceSettings::bind(
                definition.schema(),
                definition.sources(),
                &grammar.computed_settings,
                providers,
            )?,
        })
    }
    pub fn belongs_to(&self, definition: &PreparedLanguageDefinition<'_>) -> bool {
        self.bound
            .belongs_to(definition.schema(), definition.sources())
            && definition
                .grammar()
                .is_some_and(|grammar| std::ptr::eq(self.grammar, grammar))
    }
    pub fn supports(&self, class: &str, feature: &str) -> bool {
        self.bound.supports(class, feature)
    }
    pub fn admits(
        &self,
        class: &str,
        feature: &str,
        member: &MemberIdentity,
        append: bool,
    ) -> Result<bool, String> {
        self.bound.admits(class, feature, member, append)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        store: &mut impl DraftConstructionStore,
        id: &str,
        target: (&str, &str),
        edit: ComputedEdit,
        budget: &mut QueryBudget,
        conversions: Option<&ConversionRegistry<'_>>,
        construction: &DraftConstructionContext<'_>,
        metadata: &PreparedPhysicalMetadata<'_>,
        elements_per_resource: Option<usize>,
        source: (&str, &str),
    ) -> Result<(), String> {
        self.bound.apply(
            store,
            id,
            target,
            edit,
            budget,
            conversions,
            construction,
            metadata,
            elements_per_resource,
            source,
        )
    }
    pub fn apply_plain(
        &self,
        store: &mut impl ModelStore,
        id: &str,
        target: (&str, &str),
        edit: ComputedEdit,
        budget: &mut QueryBudget,
        conversions: Option<&ConversionRegistry<'_>>,
    ) -> Result<(), String> {
        self.bound
            .apply_plain(store, id, target, edit, budget, conversions)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply_plain_prepared(
        &self,
        store: &mut impl ModelStore,
        id: &str,
        target: (&str, &str),
        edit: ComputedEdit,
        budget: &mut QueryBudget,
        conversions: Option<&ConversionRegistry<'_>>,
        metadata: &PreparedPhysicalMetadata<'_>,
        features: &dyn FeatureLookup,
    ) -> Result<(), String> {
        self.bound.apply_plain_prepared(
            store,
            id,
            target,
            edit,
            budget,
            conversions,
            metadata,
            features,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply_with_source_initializers(
        &self,
        store: &mut impl DraftConstructionStore,
        id: &str,
        target: (&str, &str),
        edit: ComputedEdit,
        budget: &mut QueryBudget,
        conversions: Option<&ConversionRegistry<'_>>,
        construction: &DraftConstructionContext<'_>,
        metadata: &PreparedPhysicalMetadata<'_>,
        elements_per_resource: Option<usize>,
        source: (&str, &str),
        source_lease: &DraftConstructionSource<'_, '_>,
        initializers: &dyn ClassInitializationSelector,
    ) -> Result<(), String> {
        self.bound.apply_with_source_initializers(
            store,
            id,
            target,
            edit,
            budget,
            conversions,
            construction,
            metadata,
            elements_per_resource,
            source,
            source_lease,
            initializers,
        )
    }
}
