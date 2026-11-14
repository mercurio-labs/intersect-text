//! Grammar-driven draft capability for any Xtext language.
//!
//! Every concrete class of the definition's schema is constructible, with its
//! stored features at their schema defaults, and every assignment the grammar
//! makes to a stored feature is admitted. The grammar and the metamodel are
//! the authority; there are no per-class tables.
//!
//! Language behaviour that the grammar does not determine comes from an inner
//! capability: value converters (EString and boolean rule conversions),
//! delegated and computed assignments. This is a temporary bridge: those
//! conversions should become Rcore functions in the language package.
//! Containment opposites are reconciled while parsing, as the metamodel
//! defines them. Native initializers and per-value admission tables of the
//! inner capability are not used.
use super::*;
use intersect_model::core::storage::{
    DraftClassContract, DraftDisposition, DraftMemberDisposition,
};

/// A language's conversions of EString datatype rule spellings (Xtext value
/// converters, such as KerML's `Name`, which unquotes `'=='`). `None` keeps the
/// spelling as written.
pub trait StringRuleConversion {
    fn convert(
        &self,
        grammar: &grammar::Grammar,
        rule: &str,
        spelling: &str,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>>;
    /// The value of a terminal token (an Xtext terminal value converter).
    fn convert_terminal(&self, _terminal: &str, _text: &str) -> StorageResult<Option<String>> {
        Ok(None)
    }
}

pub struct OpenDraftCapability<'a, C: DraftCapability> {
    inner: &'a C,
    features: intersect_model::definitions::schema::FeatureIndex<'a>,
    contracts: Vec<DraftClassContract>,
    strings: Option<&'a dyn StringRuleConversion>,
}

impl<'a, C: DraftCapability> OpenDraftCapability<'a, C> {
    pub fn new(
        definition: &'a PreparedLanguageDefinition<'_>,
        inner: &'a C,
    ) -> StorageResult<Self> {
        let schema = definition.schema();
        let sources: Vec<SourceId> = definition
            .sources()
            .sources
            .iter()
            .map(|source| source.id.clone())
            .collect();
        let mut contracts = Vec::new();
        for (class, info) in &schema.classes {
            if info.abstract_type {
                continue;
            }
            let members = schema
                .effective_features(class)?
                .into_iter()
                .filter(|member| !member.feature.derived)
                .map(|member| DraftMemberDisposition {
                    name: member.feature.name.clone(),
                    identity: member.identity,
                    disposition: DraftDisposition::StoredDefault,
                })
                .collect();
            contracts.push(DraftClassContract {
                version: 1,
                schema_revision: schema.revision.clone(),
                class: class.clone(),
                class_identity: schema.identity(class),
                sources: sources.clone(),
                members,
            });
        }
        Ok(Self {
            inner,
            features: intersect_model::definitions::schema::FeatureIndex::new(schema),
            contracts,
            strings: None,
        })
    }
    /// Convert EString datatype rule spellings with the language's own
    /// converters first; spellings they leave alone go to the inner capability.
    pub fn with_string_rules(mut self, strings: &'a dyn StringRuleConversion) -> Self {
        self.strings = Some(strings);
        self
    }
}

/// The inner policy for a language that needs nothing beyond its grammar and
/// metamodel: no value converters beyond Xtext's terminals and EBoolean, and no
/// delegated or computed assignments. `OpenDraftCapability::new(definition,
/// &GrammarPolicy::new(definition))` then constructs whatever the grammar and
/// metamodel describe.
pub struct GrammarPolicy {
    original: Vec<SourceId>,
    converted: Vec<SourceId>,
}

impl GrammarPolicy {
    pub fn new(definition: &PreparedLanguageDefinition<'_>) -> Self {
        let ids = |bundle: &intersect_model::definitions::bundle::DefinitionBundle| {
            bundle
                .sources
                .iter()
                .map(|source| source.id.clone())
                .collect()
        };
        Self {
            original: ids(definition.original_sources()),
            converted: ids(definition.sources()),
        }
    }
}

impl DraftPhysicalDefaults for GrammarPolicy {
    fn contracts(&self) -> &[DraftClassContract] {
        &[]
    }
}

impl DraftCapability for GrammarPolicy {
    fn original_sources(&self) -> &[SourceId] {
        &self.original
    }
    fn converted_sources(&self) -> &[SourceId] {
        &self.converted
    }
    /// `OpenDraftCapability` admits stored features itself; nothing else is.
    fn permits_assignment(&self, _: &str, _: &MemberIdentity, _: bool) -> bool {
        false
    }
}

impl<C: DraftCapability> DraftPhysicalDefaults for OpenDraftCapability<'_, C> {
    fn contracts(&self) -> &[DraftClassContract] {
        &self.contracts
    }
}

impl<C: DraftCapability> DraftCapability for OpenDraftCapability<'_, C> {
    const RECONCILE_PARSED_CONTAINMENT: bool = true;
    const HAS_BOOLEAN_RULE_CONVERSIONS: bool = C::HAS_BOOLEAN_RULE_CONVERSIONS;
    // Spellings no converter claims keep their text (`Ok(None)`).
    const HAS_STRING_RULE_CONVERSIONS: bool = true;
    fn has_delegated_assignments(&self) -> bool {
        self.inner.has_delegated_assignments()
    }
    fn delegated_assignment_target<'b>(
        &'b self,
        receiver: &str,
        member: &MemberIdentity,
        append: bool,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<&'b MemberIdentity>> {
        self.inner
            .delegated_assignment_target(receiver, member, append, budget)
    }
    /// A computed assignment is reached only when the checked text settings
    /// admit the grammar's exact contract and provider for it; like stored
    /// assignments, the open capability admits what the grammar declares.
    fn permits_computed_assignment(&self, _: &str, _: &MemberIdentity, _: bool) -> bool {
        true
    }
    fn permits_computed_assignment_observed(
        &self,
        _: &str,
        _: &MemberIdentity,
        _: bool,
        budget: &mut QueryBudget,
    ) -> StorageResult<bool> {
        budget.charge()?;
        Ok(true)
    }
    fn boolean_rule_conversions(
        &self,
    ) -> &[intersect_model::definitions::rule_conversion::RuleConversion] {
        self.inner.boolean_rule_conversions()
    }
    fn convert_string_rule(
        &self,
        grammar: &grammar::Grammar,
        rule: &str,
        spelling: &str,
        budget: &mut QueryBudget,
    ) -> StorageResult<Option<String>> {
        if let Some(strings) = self.strings
            && let Some(converted) = strings.convert(grammar, rule, spelling, budget)?
        {
            return Ok(Some(converted));
        }
        if C::HAS_STRING_RULE_CONVERSIONS {
            self.inner
                .convert_string_rule(grammar, rule, spelling, budget)
        } else {
            Ok(None)
        }
    }
    fn convert_terminal(&self, terminal: &str, text: &str) -> StorageResult<Option<String>> {
        match self.strings {
            Some(strings) => strings.convert_terminal(terminal, text),
            None => self.inner.convert_terminal(terminal, text),
        }
    }
    fn original_sources(&self) -> &[SourceId] {
        self.inner.original_sources()
    }
    fn converted_sources(&self) -> &[SourceId] {
        self.inner.converted_sources()
    }
    /// Any stored feature the grammar assigns, singly or by append as its
    /// multiplicity allows. Derived features defer to the inner capability.
    fn permits_assignment(&self, receiver: &str, member: &MemberIdentity, append: bool) -> bool {
        match self.features.feature_by_id(receiver, member) {
            Ok(found) if !found.feature.derived => !append || found.feature.upper != 1,
            _ => self.inner.permits_assignment(receiver, member, append),
        }
    }
}
