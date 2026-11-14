//! Generic retained-model host inputs. Definition compilation, mutation and text
//! semantics stay in their shared Rust services; JSON requests are not authority.
use intersect_model::{
    core::{
        frozen::FrozenStore,
        query::QueryBudget,
        storage::{
            self, DeleteRequest, DeleteSelection, IncomingReferences, ModelStore, StructuralContext,
        },
    },
    definitions::conversion::{
        AppliedConversionContract, ConstructorContract, ConversionContract, ConversionRegistry,
    },
};
use intersect_text::package::BundleCompilation;
use intersect_text::{
    document::{AnalysisLimits, DocumentLanguage},
    grammar::Profile,
    serialization::SerializationLimits,
    source_edit::{PreparedSourceEdit, SourceEditSession},
};
use serde::Deserialize;
use serde_json::Value;

use intersect_text::text_services::{TextServiceProvider, TextServices};
use std::{rc::Rc, sync::Arc};

struct RegisteredConversions(ConversionRegistry<'static>);
impl TextServiceProvider for RegisteredConversions {
    fn conversions(&self) -> Option<&ConversionRegistry<'_>> {
        Some(&self.0)
    }
}
pub(super) struct PreparedLanguage {
    pub language: Arc<DocumentLanguage>,
    pub services: Option<TextServices>,
}
pub(super) fn language(
    request: &Value,
    retained: Option<&SourceEditSession>,
) -> Result<PreparedLanguage, String> {
    let package;
    let services;
    let package = if let Some(retained) = retained {
        services = retained.installed_services().cloned();
        retained.package()
    } else {
        let inputs: BundleCompilation = serde_json::from_value(
            request
                .get("compilation")
                .cloned()
                .ok_or("Missing definition compilation bundle")?,
        )
        .map_err(|e| e.to_string())?;
        let mut contracts: Vec<ConversionContract> = serde_json::from_value(
            request
                .get("conversions")
                .cloned()
                .unwrap_or(serde_json::json!([])),
        )
        .map_err(|e| e.to_string())?;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Binding {
            fragment: String,
            provider: intersect_model::definitions::conversion::ProviderId,
            serializable: bool,
        }
        let bindings: Vec<Binding> = serde_json::from_value(
            request
                .get("conversion_bindings")
                .cloned()
                .unwrap_or(serde_json::json!([])),
        )
        .map_err(|e| e.to_string())?;
        if !bindings.is_empty() {
            let resources =
                intersect_model::definitions::resources::ResourceSet::new(&inputs.bundle)
                    .map_err(|e| e.to_string())?;
            for binding in bindings {
                contracts.push(ConversionContract {
                    datatype: resources
                        .resolve(&inputs.ecore, &binding.fragment)
                        .map_err(|e| e.to_string())?,
                    provider: binding.provider,
                    serializable: binding.serializable,
                    declarations: vec![],
                    members: vec![],
                });
            }
        }
        let applications: Vec<AppliedConversionContract> = serde_json::from_value(
            request
                .get("applied_conversions")
                .cloned()
                .unwrap_or(serde_json::json!([])),
        )
        .map_err(|e| e.to_string())?;
        let constructors: Vec<ConstructorContract> = serde_json::from_value(
            request
                .get("constructors")
                .cloned()
                .unwrap_or(serde_json::json!([])),
        )
        .map_err(|e| e.to_string())?;
        services = if contracts.is_empty() && applications.is_empty() && constructors.is_empty() {
            None
        } else {
            Some(TextServices::new(Rc::new(RegisteredConversions(
                intersect_model::conversion::build_catalog_policies(
                    &inputs.bundle,
                    &contracts,
                    &applications,
                    &constructors,
                )?,
            ))))
        };
        package = intersect_text::ordinary::compile_with_context(
            inputs,
            Profile::NativeV82,
            &[],
            services.as_ref().and_then(TextServices::conversions),
            Default::default(),
            Default::default(),
        )
        .map_err(|errors| format!("Definition compilation failed: {errors:?}"))?;
        if package.schema.conversion_contracts.len() != contracts.len()
            || package.schema.applied_conversion_contracts.len() != applications.len()
            || package.schema.constructor_contracts.len() != constructors.len()
        {
            return Err("Unused conversion provider selection".into());
        }
        &package
    };
    let language = DocumentLanguage::prepare_with_context(
        package,
        &[],
        services.as_ref().and_then(TextServices::conversions),
        Default::default(),
        AnalysisLimits::workbench(),
    )
    .map_err(|errors| format!("Definition preparation failed: {errors:?}"))?;
    Ok(PreparedLanguage {
        language: Arc::new(language),
        services,
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Creation {
    id: String,
    class: String,
    resource: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Relocation {
    id: String,
    resource: String,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Transaction {
        #[serde(default)]
        creates: Vec<Creation>,
        #[serde(default)]
        changes: Vec<(String, String, Value)>,
        #[serde(default)]
        deletes: Vec<String>,
        #[serde(default)]
        relocates: Vec<Relocation>,
        #[serde(default)]
        remove_resources: Vec<String>,
    },
}
pub(super) fn plan(
    session: &SourceEditSession,
    action: Value,
    limits: Option<&Value>,
) -> Result<PreparedSourceEdit, String> {
    let Action::Transaction {
        creates,
        changes,
        deletes,
        relocates,
        remove_resources,
    } = serde_json::from_value(action).map_err(|e| format!("Invalid model transaction: {e}"))?;
    let limits: SerializationLimits = limits
        .map(|v| serde_json::from_value(v.clone()).map_err(|e| e.to_string()))
        .transpose()?
        .unwrap_or_default();
    let schema = &session.package().schema;
    let mut candidate = session.model().clone();
    for creation in creates {
        candidate.create(
            schema,
            &creation.id,
            &creation.class,
            &creation.resource,
            0,
            0,
        )?;
    }
    if let Some(conversions) = session.conversion_registry() {
        candidate.edit_with_conversions(schema, &changes, conversions)?;
    } else {
        candidate.edit(schema, &changes)?;
    }
    if !deletes.is_empty() || !relocates.is_empty() || !remove_resources.is_empty() {
        let mut work = QueryBudget::new(limits.steps);
        let mut store = FrozenStore::from_materialized(schema, &candidate)?;
        let context = StructuralContext {
            schema,
            conversions: session.conversion_registry(),
        };
        for movement in relocates {
            context.relocate_subtree(&mut store, &movement.id, &movement.resource, &mut work)?;
        }
        let roots: Vec<_> = deletes.iter().map(String::as_str).collect();
        if !roots.is_empty() {
            StructuralContext {
                schema,
                conversions: session.conversion_registry(),
            }
            .delete(
                &mut store,
                DeleteRequest {
                    roots: &roots,
                    selection: DeleteSelection::Subtrees,
                    incoming: IncomingReferences::Reject,
                },
                &mut work,
            )?;
        }
        for resource in remove_resources {
            storage::atomic(&mut store, |store| store.remove_resource_record(&resource))?;
        }
        candidate = storage::export(&store)?;
    }
    session.plan(&candidate, limits)
}
