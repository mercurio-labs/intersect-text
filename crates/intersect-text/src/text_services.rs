//! Owning native service selection over the shared borrowed invocation APIs.
//! Language packages implement policies; this adapter installs no domain logic.
use crate::package::Package;
use crate::setting::TextSettings;
use crate::{
    document::{DocumentLanguage, DocumentServices},
    linking::{LinkingContext, ReferenceCandidates},
    serialization::{TerminalSpelling, default_spelling_with_conversions},
    source_edit::SourceEditContext,
};
use intersect_model::{
    core::{model::Model, query::QueryBudget},
    definitions::conversion::ConversionRegistry,
    query::DerivedValues,
};
use std::{rc::Rc, sync::Arc};

pub trait TextServiceProvider {
    fn conversions(&self) -> Option<&ConversionRegistry<'_>> {
        None
    }
    fn scope(&self) -> Option<&dyn ReferenceCandidates> {
        None
    }
    fn derived(&self) -> Option<&dyn DerivedValues> {
        None
    }
    fn entry(&self) -> Option<&str> {
        None
    }
    fn formatter(&self) -> Option<&dyn crate::serialization::TextFormatter> {
        None
    }
    fn unassigned(&self) -> Option<&dyn crate::serialization::UnassignedText> {
        None
    }
    fn transient(&self) -> Option<&dyn crate::serialization::TransientValues> {
        None
    }
    fn settings<'a>(&'a self, _package: &'a Package) -> Result<Option<TextSettings<'a>>, String> {
        Ok(None)
    }
    fn spelling(
        &self,
        package: &Package,
        _model: &Model,
        value: TerminalSpelling<'_>,
    ) -> Result<String, String> {
        if self.scope().is_some() && value.target.is_some() {
            return Err("A custom scope requires an explicit inverse spelling policy".into());
        }
        default_spelling_with_conversions(package, value, self.conversions())
    }
}

/// Clones share the exact installed provider selection and its opaque identity.
/// Metadata or deserialized service names cannot construct this capability.
#[derive(Clone)]
pub struct TextServices {
    pub(crate) owner: Arc<()>,
    provider: Rc<dyn TextServiceProvider>,
}
impl TextServices {
    pub fn new(provider: Rc<dyn TextServiceProvider>) -> Self {
        Self {
            owner: Arc::new(()),
            provider,
        }
    }
    pub fn conversions(&self) -> Option<&ConversionRegistry<'_>> {
        self.provider.conversions()
    }
    pub(crate) fn spelling(
        &self,
        package: &Package,
        model: &Model,
        value: TerminalSpelling<'_>,
    ) -> Result<String, String> {
        self.provider.spelling(package, model, value)
    }
    pub(crate) fn with_context<T>(
        &self,
        language: &DocumentLanguage,
        linking_steps: usize,
        execute: impl FnOnce(&mut SourceEditContext<'_, '_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let settings = self.provider.settings(language.package())?;
        let mut work = QueryBudget::new(linking_steps);
        let mut linking = self.provider.scope().map(|resolver| LinkingContext {
            resolver,
            work: &mut work,
        });
        let mut context = SourceEditContext {
            owner: Arc::clone(&self.owner),
            services: DocumentServices {
                entry: self.provider.entry(),
                conversions: self.provider.conversions(),
                settings: settings.as_ref(),
                derived: self.provider.derived(),
                linking: linking.as_mut(),
                formatter: self.provider.formatter(),
                unassigned: self.provider.unassigned(),
                transient: self.provider.transient(),
            },
        };
        execute(&mut context)
    }
}
