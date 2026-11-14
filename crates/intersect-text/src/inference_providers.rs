//! Authored provider selection over shared inference and contextual compilation.
//! Candidates describe structure only; selected compilation checks executable semantics.
use super::{
    InferenceRequest, InferredLanguage, infer_metamodels, infer_metamodels_with_conversions,
};
use crate::grammar::SourceLimits;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::conversion::ConversionRegistry;
use intersect_model::definitions::conversion::ProviderId;
use intersect_model::definitions::schema::Diagnostic;
use intersect_model::definitions::schema::MemberIdentity;
use intersect_model::definitions::schema::Schema;
use intersect_model::definitions::setting::ComputedSettingContract;
use intersect_model::definitions::setting::SettingOperation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

type SelectionResult<T> = Result<T, Vec<Diagnostic>>;
const MAX_SETTINGS: usize = 16;
const MAX_CANDIDATES: usize = 4096;
/// Original authored identity and spelling, never an internal runtime class key.
/// This initial profile supports direct nongeneric Rcore members only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredSettingSelector {
    pub source: SourceId,
    pub namespace: String,
    pub class: String,
    pub feature: String,
    pub provider: ProviderId,
    pub operations: Vec<SettingOperation>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderSelection {
    pub version: u32,
    pub behavior: SourceId,
    pub computed_settings: Vec<AuthoredSettingSelector>,
}
#[derive(Clone, Debug, Serialize)]
pub struct UnsupportedSetting {
    pub source: SourceId,
    pub namespace: String,
    pub class: String,
    pub feature: String,
    pub reason: String,
}
/// This report does not prove a projection provider exists or will compile.
#[derive(Clone, Debug, Serialize)]
pub struct CandidateReport {
    pub boundary: &'static str,
    pub candidates: Vec<AuthoredSettingSelector>,
    pub unsupported: Vec<UnsupportedSetting>,
}
/// In-process checked result. Deserialized selectors/contracts cannot construct it.
pub struct SelectedLanguage {
    selection: ProviderSelection,
    computed_settings: Vec<ComputedSettingContract>,
    language: InferredLanguage,
}
impl SelectedLanguage {
    pub fn selection(&self) -> &ProviderSelection {
        &self.selection
    }
    pub fn computed_settings(&self) -> &[ComputedSettingContract] {
        &self.computed_settings
    }
    pub fn language(&self) -> &InferredLanguage {
        &self.language
    }
    pub fn into_language(self) -> InferredLanguage {
        self.language
    }
}
fn error(request: &InferenceRequest, message: impl Into<String>) -> Vec<Diagnostic> {
    vec![Diagnostic::new(&request.grammar.uri, 0, 1, message).with_context(&request.grammar)]
}
fn prepared_schema(
    request: &InferenceRequest,
    limits: SourceLimits,
    conversions: Option<&ConversionRegistry<'_>>,
) -> SelectionResult<Schema> {
    if request.version != 1 {
        return Err(error(request, "Unsupported inference request version"));
    }
    let definitions = match conversions {
        Some(registry) => {
            infer_metamodels_with_conversions(&request.bundle, &request.grammar, limits, registry)
        }
        None => infer_metamodels(&request.bundle, &request.grammar, limits),
    }
    .map_err(|e| vec![e])?;
    let mut schema = definitions.schema;
    crate::grammar::prepare_ecore_reference_context(&mut schema).map_err(|e| error(request, e))?;
    Ok(schema)
}
fn resolve<'a>(
    request: &InferenceRequest,
    schema: &'a Schema,
    selector: &AuthoredSettingSelector,
) -> Result<(&'a str, MemberIdentity), String> {
    if !request
        .bundle
        .sources
        .iter()
        .any(|source| source.id == selector.source && source.kind == SourceKind::Rcore)
    {
        return Err("Provider selectors require an original authored Rcore source".into());
    }
    let class = schema.class_key(&selector.source, &selector.namespace, &selector.class)?;
    let declaration = schema
        .declarations
        .get(class)
        .ok_or("Missing authored receiver declaration")?;
    let generic = schema.instantiations.contains_key(class)
        || schema
            .generic_signatures
            .as_ref()
            .is_some_and(|signatures| {
                signatures.parameters.iter().any(|parameter| {
                    let id = &parameter.syntax.declaration;
                    id.source == declaration.source
                        && id
                            .path
                            .split_last()
                            .is_some_and(|(_, parent)| parent == declaration.path)
                })
            });
    if generic {
        return Err(
            "Provider selectors do not support generic receiver declarations or materializations"
                .into(),
        );
    }
    let member = schema.effective_feature(class, &selector.feature)?;
    let MemberIdentity::Source { declaration: field } = &member.identity else {
        return Err("Provider selectors require a direct authored member identity".into());
    };
    if member.owner != class
        || field.source != selector.source
        || field
            .path
            .split_last()
            .is_none_or(|(_, parent)| parent != declaration.path)
    {
        return Err("Provider selectors support direct authored members only; inherited members require a separate origin contract".into());
    }
    if !member.feature.derived || !member.feature.changeable {
        return Err("Provider selectors require a changeable derived member".into());
    }
    Ok((class, member.identity))
}
/// Discover structural choices from freshly inferred sources. Provider execution,
/// behavior reachability and grammar usage are checked only by `compile_selected`.
pub fn candidates(
    request: &InferenceRequest,
    limits: SourceLimits,
    conversions: Option<&ConversionRegistry<'_>>,
) -> SelectionResult<CandidateReport> {
    let schema = prepared_schema(request, limits, conversions)?;
    let mut report = CandidateReport {
        boundary: "authored_direct_nongeneric_rcore",
        candidates: vec![],
        unsupported: vec![],
    };
    for (class, declaration) in &schema.declarations {
        if !schema.classes.contains_key(class) {
            continue;
        }
        let members = schema
            .effective_features(class)
            .map_err(|e| error(request, e))?;
        for member in members {
            if !member.feature.derived || !member.feature.changeable {
                continue;
            }
            if report.candidates.len() + report.unsupported.len() >= MAX_CANDIDATES {
                return Err(error(request, "Provider candidate budget exceeded (4096)"));
            }
            let mut operations = vec![SettingOperation::Set];
            if !matches!(member.feature.upper, 0 | 1) {
                operations.push(SettingOperation::Append);
            }
            operations.push(SettingOperation::Unset);
            let selector = AuthoredSettingSelector {
                source: declaration.source.clone(),
                namespace: declaration.namespace.clone(),
                class: declaration.name.clone(),
                feature: member.feature.name.clone(),
                provider: ProviderId {
                    name: "rcore.projection".into(),
                    version: "1".into(),
                },
                operations,
            };
            match resolve(request, &schema, &selector) {
                Ok(_) => report.candidates.push(selector),
                Err(reason) => report.unsupported.push(UnsupportedSetting {
                    source: selector.source,
                    namespace: selector.namespace,
                    class: selector.class,
                    feature: selector.feature,
                    reason,
                }),
            }
        }
    }
    Ok(report)
}
/// Resolve authored selectors, compile through the existing contextual frontend,
/// and validate only the installed source-backed Rcore projection adapter.
///
/// At most sixteen selected contracts are accepted. A successful full compile is
/// followed by one shared compiler run per omitted contract to reject unused
/// selectors. This bounded synchronous helper makes no cooperative S09 claim.
pub fn compile_selected(
    request: &InferenceRequest,
    selection: &ProviderSelection,
    limits: SourceLimits,
    conversions: Option<&ConversionRegistry<'_>>,
) -> SelectionResult<SelectedLanguage> {
    if selection.version != 1
        || selection.computed_settings.is_empty()
        || selection.computed_settings.len() > MAX_SETTINGS
    {
        return Err(error(
            request,
            "Provider selection version 1 requires 1..=16 authored settings",
        ));
    }
    let schema = prepared_schema(request, limits, conversions)?;
    let mut selected = BTreeSet::new();
    let mut contracts = Vec::new();
    for selector in &selection.computed_settings {
        if [&selector.namespace, &selector.class, &selector.feature]
            .iter()
            .any(|value| value.is_empty() || value.len() > 1024)
        {
            return Err(error(
                request,
                "Provider selector names must contain 1..=1024 bytes",
            ));
        }
        if selector.provider.name != "rcore.projection" || selector.provider.version != "1" {
            return Err(error(
                request,
                "Unsupported inferred computed setting provider",
            ));
        }
        let (class, member) = resolve(request, &schema, selector).map_err(|e| error(request, e))?;
        if !selected.insert((class.to_owned(), selector.feature.clone())) {
            return Err(error(
                request,
                "Duplicate authored provider receiver/member selector",
            ));
        }
        let contract = ComputedSettingContract {
            version: 1,
            schema_revision: schema.revision.clone(),
            class: class.into(),
            class_identity: schema.identity(class),
            feature: selector.feature.clone(),
            member,
            provider: selector.provider.clone(),
            sources: request
                .bundle
                .sources
                .iter()
                .map(|s| s.id.clone())
                .collect(),
            operations: selector.operations.clone(),
        };
        contract
            .validate(&schema, &request.bundle)
            .map_err(|e| error(request, e))?;
        contracts.push(contract);
    }
    let language =
        request.compile_with_context(limits, &contracts, conversions, Some(&selection.behavior))?;
    for contract in &contracts {
        intersect_model::setting::RcoreProjection::new(
            language.package(),
            &request.bundle,
            contract,
        )
        .map_err(|e| error(request, e))?;
    }
    for index in 0..contracts.len() {
        let omitted = contracts
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, c)| c.clone())
            .collect::<Vec<_>>();
        if request
            .compile_with_context(limits, &omitted, conversions, Some(&selection.behavior))
            .is_ok()
        {
            return Err(error(
                request,
                format!(
                    "Unused authored provider selector {}::{}.{}",
                    selection.computed_settings[index].namespace,
                    selection.computed_settings[index].class,
                    selection.computed_settings[index].feature
                ),
            ));
        }
    }
    Ok(SelectedLanguage {
        selection: selection.clone(),
        computed_settings: contracts,
        language,
    })
}
