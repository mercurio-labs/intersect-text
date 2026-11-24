//! JSON host adapter. The WASM ABI owns input/output buffers; all semantics are native.
use intersect_text::Source;
use intersect_text::compiler;
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
mod applied_conversions;
mod collections;
mod conversions;
mod definition;
mod diagnostic_chain;
mod documents;
mod feature_maps;
pub mod function_jobs;
mod generic_signatures;
mod inferred;
mod inferred_context;
mod interchange;
mod library_text;
mod model_text;
mod named_entries;
use intersect_model_test_support::native;
mod query;
mod rcore_behavior_functions;
mod rcore_behavior_scalar;
mod rcore_operations;
mod resource_handles;
mod resource_lifecycle;
mod scripting;
mod semantic_bindings;
mod tree_iterator;

pub fn dispatch(input: &str) -> Result<Value, String> {
    // Oversized source-execution requests are rejected before constructing a JSON tree or
    // cloning source/value payloads. Ignored fields are scanned without retaining
    // their contents; unrelated host methods retain their existing size policy.
    if input.len() > 4_000_000 {
        #[derive(serde::Deserialize)]
        struct MethodProbe {
            method: String,
        }
        let probe: MethodProbe = serde_json::from_str(input).map_err(|e| e.to_string())?;
        if probe.method.starts_with("document/") {
            return Err("Document request byte budget exceeded".into());
        }
        if probe.method.starts_with("model/resource") || probe.method.starts_with("model/cache") {
            return Err("Resource lifecycle request byte budget exceeded".into());
        }
        if probe.method.starts_with("bindings/") {
            return Err("Binding request byte budget exceeded".into());
        }
        if probe.method == "rcore_behavior/evaluateScalar" {
            return Err("Scalar request byte budget exceeded".into());
        }
        if matches!(
            probe.method.as_str(),
            "rcore_behavior/compileFunction"
                | "rcore_behavior/evaluateFunction"
                | "rcore/compileOperation"
                | "rcore/invokeOperation"
        ) {
            return Err("Function request byte budget exceeded".into());
        }
    }
    let request: Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let string = |name: &str| {
        request
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Missing {name}"))
    };
    let method = string("method")?;
    if method.starts_with("library/") || method.starts_with("modelText/") {
        return library_text::dispatch(method, &request);
    }
    if method == "package/export"
        && request
            .get("inference")
            .is_some_and(|value| !value.is_null())
    {
        return if request
            .get("inference_context")
            .is_some_and(|v| !v.is_null())
        {
            inferred_context::export(&request)
        } else {
            inferred::export(&request)
        };
    }
    if method == "package/export"
        && (request.get("conversions").is_some()
            || request.get("applied_conversions").is_some()
            || request.get("constructors").is_some())
    {
        return conversions::export(&request);
    }
    if matches!(method, "compile_rcore" | "compile_bundle")
        && (request.get("conversions").is_some()
            || request.get("applied_conversions").is_some()
            || request.get("constructors").is_some())
    {
        return match conversions::compile(&request) {
            Ok(result) => Ok(result),
            Err(message) => Ok(
                json!({"ok":false,"diagnostics":[{"source":"ecore","start":0,"end":0,"message":message}]}),
            ),
        };
    }
    if method == "evaluate"
        && request
            .get("entry_rule")
            .is_some_and(|value| !value.is_null())
    {
        return named_entries::evaluate(&request);
    }
    if method == "evaluate" && !request["package"]["xtext_inference"].is_null() {
        return inferred_context::evaluate(&request);
    }
    if method == "evaluate"
        && [
            "conversion_contracts",
            "applied_conversion_contracts",
            "constructor_contracts",
        ]
        .iter()
        .any(|field| {
            request["package"]["schema"][*field]
                .as_array()
                .is_some_and(|contracts| !contracts.is_empty())
        })
    {
        return conversions::evaluate(&request);
    }
    match string("method")? {
        "text/entries" => named_entries::catalog(&request),
        "document/analyze" => documents::analyze(&request),
        "document/link" => documents::linked_object(&request),
        "bindings/inspect"
        | "bindings/activate"
        | "bindings/invoke"
        | "bindings/stubs"
        | "bindings/projectInspect" => semantic_bindings::handle(&request),
        "feature-map/step" => feature_maps::step(&request),
        "feature-map/reference-bind" => feature_maps::reference(&request, true),
        "feature-map/reference-read" => feature_maps::reference(&request, false),
        "collection/step" => collections::step(&request),
        "collection/reference-bind" => collections::reference(&request, true),
        "collection/reference-read" => collections::reference(&request, false),
        "conversion/constructor" => conversions::constructor(&request),
        "conversion/applied" => applied_conversions::access(&request),
        "iterator/step" => tree_iterator::step(&request),
        "resource/read" | "resource/resolve" => resource_handles::access(&request),
        "diagnostic/collect" => diagnostic_chain::collect(&request),
        "ecore/materialize" => generic_signatures::materialize(&request),
        "ecore/generic-datatype-application" => generic_signatures::datatype_application(&request),
        "ecore/generic-operation-override" => generic_signatures::operation_override(&request),
        "ecore/generic-operation-selection" => generic_signatures::operation_selection(&request),
        "ecore/generic-operation-types" => generic_signatures::operation_types(&request),
        "ecore/runtime-operation-types" => generic_signatures::runtime_operation_types(&request),
        "ecore/runtime-member-types" => generic_signatures::runtime_member_types(&request),
        "ecore/generic-instantiate-members" => generic_signatures::requested_member_types(&request),
        "ecore/generic-member-types" => generic_signatures::member_types(&request),
        "ecore/generic-application-bounds" => generic_signatures::application_bounds(&request),
        "ecore/generic-inheritance" => generic_signatures::inheritance(&request),
        "ecore/generic-substitute" => generic_signatures::substitute(&request),
        "ecore/generic-signatures" => generic_signatures::check_all(&request),
        "ecore/generic-type" => generic_signatures::check(&request, false),
        "ecore/generic-parameter" => generic_signatures::check(&request, true),
        "ecore/members" => {
            let bundle: intersect_model::definitions::bundle::DefinitionBundle =
                serde_json::from_value(
                    request
                        .get("bundle")
                        .cloned()
                        .ok_or("Missing definition bundle")?,
                )
                .map_err(|e| format!("Invalid definition bundle: {e}"))?;
            let entry = serde_json::from_value(
                request
                    .get("entry")
                    .cloned()
                    .ok_or("Missing entry source")?,
            )
            .map_err(|e| format!("Invalid entry source: {e}"))?;
            if bundle
                .sources
                .iter()
                .any(|source| source.text.len() > 256_000)
            {
                return Err("Definition size budget exceeded".into());
            }
            match intersect_model::definitions::ecore_document::inspect_members(
                &bundle,
                &entry,
                string("namespace")?,
                string("class")?,
            ) {
                Ok(inspection) => Ok(
                    json!({"stage":"effective_members", "ok":true, "operations_executable":false,
                    "revision":inspection.revision, "members":inspection.members}),
                ),
                Err(error) => Ok(
                    json!({"stage":"effective_members", "ok":false, "operations_executable":false,
                    "diagnostics":[{"span":error.span, "message":error.message, "kind":format!("{:?}", error.kind)}]}),
                ),
            }
        }
        "rcore_behavior/evaluateScalar" => rcore_behavior_scalar::evaluate(&request),
        "rcore/compileOperation" | "rcore/invokeOperation" => rcore_operations::handle(&request),
        "rcore_behavior/compileFunction" | "rcore_behavior/evaluateFunction" => {
            rcore_behavior_functions::handle(&request)
        }
        "rcore_behavior/bind" | "rcore_behavior/check" => {
            let bundle: intersect_model::definitions::bundle::DefinitionBundle =
                serde_json::from_value(
                    request
                        .get("bundle")
                        .cloned()
                        .ok_or("Missing definition bundle")?,
                )
                .map_err(|e| format!("Invalid definition bundle: {e}"))?;
            let entry = serde_json::from_value(
                request
                    .get("entry")
                    .cloned()
                    .ok_or("Missing entry source")?,
            )
            .map_err(|e| format!("Invalid entry source: {e}"))?;
            if bundle.sources.iter().any(|s| s.text.len() > 256_000) {
                return Err("Definition size budget exceeded".into());
            }
            if string("method")? == "rcore_behavior/check" {
                let result = if ["conversions", "applied_conversions", "constructors"]
                    .iter()
                    .any(|field| request.get(*field).is_some())
                {
                    let contracts: Vec<
                        intersect_model::definitions::conversion::ConversionContract,
                    > = serde_json::from_value(
                        request.get("conversions").cloned().unwrap_or(json!([])),
                    )
                    .map_err(|error| format!("Invalid conversion selection: {error}"))?;
                    let applications: Vec<
                        intersect_model::definitions::conversion::AppliedConversionContract,
                    > = serde_json::from_value(
                        request
                            .get("applied_conversions")
                            .cloned()
                            .unwrap_or(json!([])),
                    )
                    .map_err(|error| format!("Invalid applied conversion selection: {error}"))?;
                    let constructors = conversions::constructor_inputs(&request, "constructors")?;
                    intersect_model::conversion::with_catalog_policies(
                        &bundle,
                        &contracts,
                        &applications,
                        &constructors,
                        |registry| {
                            Ok(
                                intersect_model::definitions::rcore_behavior::checking::check_with_conversions(
                                    &bundle, &entry, registry,
                                ),
                            )
                        },
                    )?
                } else {
                    intersect_model::definitions::rcore_behavior::checking::check(&bundle, &entry)
                };
                return match result {
                    Ok(typed) => {
                        Ok(json!({"stage":"typing", "ok":true, "executable":false, "typed":typed}))
                    }
                    Err(diagnostics) => Ok(
                        json!({"stage":"typing", "ok":false, "executable":false, "diagnostics":diagnostics}),
                    ),
                };
            }
            match intersect_model::definitions::rcore_behavior::bindings::bind(&bundle, &entry) {
                Ok(resolved) => Ok(
                    json!({"stage":"resolution", "ok":true, "executable":false, "resolved":resolved}),
                ),
                Err(diagnostics) => Ok(
                    json!({"stage":"resolution", "ok":false, "executable":false, "diagnostics":diagnostics}),
                ),
            }
        }
        "ecore/migrate" => {
            let source = intersect_model::definitions::bundle::DefinitionSource::new(
                string("uri")?,
                intersect_model::definitions::bundle::SourceKind::Ecore,
                string("text")?,
            );
            let report = intersect_model::migration::migrate_ecore(source, string("target_uri")?)
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok":report.converted.is_some(),"report":report}))
        }
        "ecore/migrateBundle" => {
            use intersect_model::definitions::bundle::{DefinitionBundle, SourceId};
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Target {
                source: SourceId,
                uri: String,
            }
            let bundle: DefinitionBundle =
                serde_json::from_value(request["bundle"].clone()).map_err(|e| e.to_string())?;
            let targets: Vec<Target> =
                serde_json::from_value(request["targets"].clone()).map_err(|e| e.to_string())?;
            let mut destinations = std::collections::BTreeMap::new();
            for target in targets {
                if destinations.insert(target.source, target.uri).is_some() {
                    return Err("Duplicate migration target".into());
                }
            }
            let report = intersect_model::migration::migrate_ecore_bundle(bundle, &destinations)
                .map_err(|e| e.to_string())?;
            Ok(json!({"ok":report.converted.is_some(),"report":report}))
        }
        "ecore/migrateCompilation" => {
            use intersect_model::definitions::bundle::SourceId;
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Target {
                source: SourceId,
                uri: String,
            }
            let compilation: BundleCompilation =
                serde_json::from_value(request["compilation"].clone())
                    .map_err(|e| e.to_string())?;
            let targets: Vec<Target> =
                serde_json::from_value(request["targets"].clone()).map_err(|e| e.to_string())?;
            let mut destinations = std::collections::BTreeMap::new();
            for target in targets {
                if destinations.insert(target.source, target.uri).is_some() {
                    return Err("Duplicate migration target".into());
                }
            }
            let report = intersect_model::migration::migrate_ecore_bundle(
                compilation.bundle.clone(),
                &destinations,
            )
            .map_err(|e| e.to_string())?;
            let converted = report
                .converted
                .as_ref()
                .map(|_| intersect_text::migration::remap_compilation(&report, &compilation))
                .transpose()?;
            Ok(json!({"ok":converted.is_some(),"report":report,"compilation":converted}))
        }
        "rcore/inspectStructure" => {
            use intersect_model::definitions::bundle::{
                DefinitionBundle, DefinitionSource, SourceKind,
            };
            let source = DefinitionSource::new(string("uri")?, SourceKind::Rcore, string("text")?);
            let bundle = DefinitionBundle {
                version: 1,
                sources: vec![source.clone()],
                namespaces: vec![],
            };
            match intersect_model::definitions::rcore::compile_structure(&bundle, &source.id) {
                Ok(structure) => Ok(
                    json!({"ok":true, "source":source, "schema":structure.schema,
                    "namespace":structure.namespace, "classifiers":structure.classifiers, "operations":structure.operations}),
                ),
                Err(diagnostics) => Ok(json!({"ok":false,"diagnostics":diagnostics})),
            }
        }
        "compile_rcore" => {
            let behavior = request
                .get("rcore_behavior")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty());
            let text = request
                .get("xtext")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty());
            match compiler::compile_rcore_with_profile(
                string("rcore")?,
                text,
                behavior,
                compiler_profile(&request)?,
            ) {
                Ok(package) => {
                    match semantic_bindings::install_for_package(&package, request.get("bindings"))
                    {
                        Ok(()) => Ok(
                            json!({"ok":true,"keywords":intersect_text::keywords(&package),"package":package}),
                        ),
                        Err(message) => Ok(
                            json!({"ok":false,"diagnostics":[{"source":"rcore_behavior","start":0,"end":0,"message":message}]}),
                        ),
                    }
                }
                Err(diagnostics) => Ok(json!({"ok":false,"diagnostics":diagnostics})),
            }
        }
        "rcore/lowerStructure" => {
            if string("text")?.len() > 256_000 {
                return Err("Definition size budget exceeded".into());
            }
            let source = intersect_model::definitions::bundle::DefinitionSource::new(
                string("uri")?,
                intersect_model::definitions::bundle::SourceKind::Rcore,
                string("text")?,
            );
            match intersect_model::definitions::rcore::lower_structure(
                source,
                string("generated_uri")?,
            ) {
                Ok(lowering) => {
                    Ok(json!({"ok":true,"stage":"rcore_structure","lowering":lowering}))
                }
                Err(diagnostics) => {
                    Ok(json!({"ok":false,"stage":"rcore_structure","diagnostics":diagnostics}))
                }
            }
        }
        "rcore_behavior/parse" => {
            let text = string("text")?;
            if text.len() > 256_000 {
                return Err("Definition size budget exceeded".into());
            }
            let source = intersect_model::definitions::bundle::DefinitionSource::new(
                string("uri")?,
                intersect_model::definitions::bundle::SourceKind::RcoreBehavior,
                text,
            );
            let document = intersect_model::definitions::rcore_behavior::parse(source);
            Ok(
                json!({"stage":"syntax", "ok":document.diagnostics.is_empty(), "executable":false, "document":document}),
            )
        }
        "inferred/structurePreview" => inferred::structure_preview(&request),
        "inferred/structurePreviewWithContext" => inferred_context::structure_preview(&request),
        "inferred/providerCandidates" => inferred_context::provider_candidates(&request),
        "inferred/selectProviders" => inferred_context::select_providers(&request),
        "compile_inferred" | "compile_inferred_with_origins" => {
            let inputs = inferred::inputs(&request)?;
            Ok(inferred::compiled_response(
                inputs.compile(intersect_text::grammar::SourceLimits::default()),
                method == "compile_inferred_with_origins",
            ))
        }
        "compile_inferred_with_context" => inferred_context::compile(&request),
        "package/import_with_context" => inferred_context::import(&request),
        "compile_bundle" => {
            let compilation = serde_json::from_value(
                request
                    .get("compilation")
                    .cloned()
                    .ok_or("Missing bundle compilation inputs")?,
            )
            .map_err(|e| format!("Invalid bundle compilation inputs: {e}"))?;
            match intersect_text::ordinary::compile_with_optional_source_policy(
                compilation,
                compiler_profile(&request)?,
                &[],
                None,
                None,
                Default::default(),
            ) {
                Ok(package) => Ok(
                    json!({"ok":true,"keywords":intersect_text::keywords(&package),"package":package}),
                ),
                Err(diagnostics) => Ok(json!({"ok":false,"diagnostics":diagnostics})),
            }
        }
        "evaluate" => {
            // Packages cross a trust boundary: rebuild from retained definitions instead
            // of trusting arbitrary serialized executable programs from the host.
            let package = scripting::package(&request)?;
            if package.grammar.is_none() {
                return Ok(json!(scripting::inspect(
                    &package,
                    intersect_model::core::model::Model {
                        schema_revision: package.schema.revision.clone(),
                        resources: Default::default(),
                        elements: vec![],
                    },
                )?));
            }
            let sources: Vec<Source> = serde_json::from_value(
                request
                    .get("sources")
                    .cloned()
                    .ok_or("Missing model sources")?,
            )
            .map_err(|e| e.to_string())?;
            Ok(json!(intersect_text::evaluate_with_provider(
                &package,
                &sources,
                intersect_text::EvaluationLimits::workbench(),
                &semantic_bindings::Derived::new(&package),
            )))
        }
        "package/import_with_conversions" => conversions::import(&request),
        "package/import_with_execution" | "package/export_with_execution" => {
            execution_archive(&request, method == "package/import_with_execution")
        }
        "package/import" => {
            let imported = intersect_text::distribution::import_files(archive_files(&request)?)?;
            let keywords = intersect_text::keywords(&imported.package);
            let mut response = serde_json::to_value(imported).map_err(|e| e.to_string())?;
            response["keywords"] = json!(keywords);
            Ok(response)
        }
        "package/export" => {
            let metadata = serde_json::from_value(
                request
                    .get("metadata")
                    .cloned()
                    .ok_or("Missing package metadata")?,
            )
            .map_err(|e| format!("Invalid package metadata: {e}"))?;
            let distribution = if let Some(compilation) = request.get("compilation") {
                let compilation = serde_json::from_value(compilation.clone())
                    .map_err(|e| format!("Invalid bundle compilation inputs: {e}"))?;
                intersect_text::distribution::build_bundle_with_profile(
                    compilation,
                    metadata,
                    compiler_profile(&request)?,
                )?
            } else {
                return Err("Package export requires retained compilation inputs".into());
            };
            serde_json::to_value(distribution).map_err(|e| e.to_string())
        }
        "query/execute" => query::execute(&request),
        "query/execute_with_conversions" => query::execute_with_conversions(&request),
        method if method.starts_with("definition/") => definition::dispatch(method, &request),
        method if method.starts_with("model/") => interchange::dispatch(method, &request),
        method if method.starts_with("script/") => scripting::dispatch(method, &request),
        _ => Err("Unknown method".into()),
    }
}

fn archive_files(request: &Value) -> Result<std::collections::BTreeMap<String, String>, String> {
    serde_json::from_value(
        request
            .get("files")
            .cloned()
            .ok_or("Missing extracted archive files")?,
    )
    .map_err(|e| format!("Invalid archive files: {e}"))
}

fn execution_archive(request: &Value, include_keywords: bool) -> Result<Value, String> {
    use intersect_text::distribution::{LanguageArchive, import_archive};

    if request.as_object().is_some_and(|fields| {
        fields
            .keys()
            .any(|key| !matches!(key.as_str(), "method" | "files"))
    }) {
        return Err("Execution archive requests take only the retained archive files".into());
    }
    let files = archive_files(request)?;
    let manifest = files
        .get("manifest.json")
        .cloned()
        .ok_or("Missing manifest.json")?;
    let LanguageArchive::Execution(archive) = import_archive(files)? else {
        return Err("Execution archive requests require a format-3 language archive".into());
    };
    let keywords = include_keywords.then(|| intersect_text::keywords(&archive.package));
    let mut response = serde_json::to_value(archive).map_err(|e| e.to_string())?;
    // Keep the original wire bytes in the transport file map for an exact re-export.
    response["files"]["manifest.json"] = json!(manifest);
    if let Some(keywords) = keywords {
        response["keywords"] = json!(keywords);
    }
    Ok(response)
}

/// New requests use the default profile when omitted; retained packages require an explicit profile.
fn compiler_profile(request: &Value) -> Result<intersect_text::grammar::Profile, String> {
    request
        .get("xtext_profile")
        .map_or(Ok(Default::default()), |value| {
            serde_json::from_value(value.clone()).map_err(|e| format!("Invalid Xtext profile: {e}"))
        })
}
fn retained_profile(retained: &Value) -> Result<intersect_text::grammar::Profile, String> {
    let value = retained["grammar"]
        .get("profile")
        .ok_or("Retained grammar requires an explicit Xtext profile")?;
    serde_json::from_value(value.clone())
        .map_err(|e| format!("Invalid retained Xtext profile: {e}"))
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-exports"))]
mod wasm {
    use super::*;
    use std::cell::RefCell;
    thread_local! { static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) }; static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) }; }
    thread_local! { static HOST: RefCell<function_jobs::FunctionJobHost> = RefCell::new(function_jobs::FunctionJobHost::default()); }
    /// Allocate a bounded input region owned by this instance until the next call.
    #[unsafe(no_mangle)]
    pub extern "C" fn input_buffer(length: usize) -> *mut u8 {
        if length > 4_000_000 {
            return std::ptr::null_mut();
        }
        INPUT.with(|input| {
            let mut input = input.borrow_mut();
            input.resize(length, 0);
            input.as_mut_ptr()
        })
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn execute() -> usize {
        let response = INPUT.with(|input| {
            let input = input.borrow();
            std::str::from_utf8(&input)
                .map_err(|e| e.to_string())
                .and_then(|request| HOST.with(|host| host.borrow_mut().dispatch(request)))
        });
        let value = response.unwrap_or_else(|message| json!({"error":message}));
        OUTPUT.with(|output| {
            let mut output = output.borrow_mut();
            *output = value.to_string().into_bytes();
            output.len()
        })
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn output_buffer() -> *const u8 {
        OUTPUT.with(|output| output.borrow().as_ptr())
    }
}
