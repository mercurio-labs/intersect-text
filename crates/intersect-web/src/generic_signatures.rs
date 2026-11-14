//! Shared generic-signature checks. Never activates a runtime language package.
use intersect_model::definitions::{
    bundle::DefinitionBundle, ecore::ImportError, generic_resolution::Resolver,
    resources::DeclarationId,
};
use serde_json::{Value, json};
fn bundle(request: &Value) -> Result<DefinitionBundle, String> {
    let bundle: DefinitionBundle = serde_json::from_value(
        request
            .get("bundle")
            .cloned()
            .ok_or("Missing definition bundle")?,
    )
    .map_err(|e| e.to_string())?;
    if bundle
        .sources
        .iter()
        .any(|source| source.text.len() > 256_000)
    {
        return Err("Definition size budget exceeded".into());
    }
    Ok(bundle)
}
// Catalog/selection failures use the same request-error path as materialization;
// source and conformance failures retain structured ImportError diagnostics.
fn configured_resolver(
    request: &Value,
    bundle: &DefinitionBundle,
) -> Result<Result<Resolver, ImportError>, String> {
    let contracts: Vec<intersect_model::definitions::conversion::ConversionContract> =
        serde_json::from_value(request.get("conversions").cloned().unwrap_or(json!([])))
            .map_err(|e| e.to_string())?;
    let constructors = crate::conversions::constructor_inputs(request, "constructors")?;
    intersect_model::conversion::with_catalog_policies(
        bundle,
        &contracts,
        &[],
        &constructors,
        |registry| {
            Ok(Resolver::new(bundle).and_then(|resolver| resolver.with_conversions(registry)))
        },
    )
}

fn response(result: Result<Value, ImportError>) -> Value {
    match result {
        Ok(report) => {
            json!({"stage":"generic_signature","ok":true,"executable":false,"report":report})
        }
        Err(error) => {
            json!({"stage":"generic_signature","ok":false,"executable":false,"diagnostics":[{"span":error.span,"kind":format!("{:?}",error.kind),"message":error.message}]})
        }
    }
}
pub fn check_all(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    Ok(response(Resolver::new(&bundle).and_then(|resolver| {
        resolver.resolve_all().map(|report| json!(report))
    })))
}
pub fn check(request: &Value, parameter: bool) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let declaration: DeclarationId = serde_json::from_value(
        request
            .get("declaration")
            .cloned()
            .ok_or("Missing exact generic declaration")?,
    )
    .map_err(|e| e.to_string())?;
    Ok(response(Resolver::new(&bundle).and_then(|resolver| {
        if parameter {
            resolver
                .resolve_parameter(&declaration)
                .map(|report| json!(report))
        } else {
            resolver
                .resolve_type(&declaration)
                .map(|report| json!(report))
        }
    })))
}

pub fn substitute(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let declaration: DeclarationId = serde_json::from_value(
        request
            .get("declaration")
            .cloned()
            .ok_or("Missing exact generic declaration")?,
    )
    .map_err(|e| e.to_string())?;
    let selections =
        serde_json::from_value::<Vec<intersect_model::definitions::generic_types::SourceBinding>>(
            request
                .get("bindings")
                .cloned()
                .ok_or("Missing source bindings")?,
        )
        .map_err(|e| e.to_string())?;
    let mut result = response(Resolver::new(&bundle).and_then(|resolver| {
        resolver
            .substitute_type(&declaration, &selections)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_substitution");
    result["bounds_verified"] = json!(false);
    Ok(result)
}

pub fn inheritance(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let declaration: DeclarationId = serde_json::from_value(
        request
            .get("declaration")
            .cloned()
            .ok_or("Missing exact generic declaration")?,
    )
    .map_err(|e| e.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .inherited_types(&declaration)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_inheritance");
    result["bounds_verified"] = json!(false);
    Ok(result)
}

pub fn application_bounds(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let declaration: DeclarationId = serde_json::from_value(
        request
            .get("declaration")
            .cloned()
            .ok_or("Missing exact generic declaration")?,
    )
    .map_err(|e| e.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .application_bounds(&declaration)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_application_bounds");
    result["profile"] = json!("nominal_with_raw_warnings");
    if result["ok"] == true {
        result["ok"] = result["report"]["valid"].clone();
    }
    Ok(result)
}

pub fn member_types(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let declaration: DeclarationId = serde_json::from_value(
        request
            .get("declaration")
            .cloned()
            .ok_or("Missing exact generic declaration")?,
    )
    .map_err(|e| e.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .member_types(&declaration)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_member_types");
    result["selection_verified"] = json!(false);
    Ok(result)
}

pub fn runtime_member_types(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let receiver = serde_json::from_value(
        request
            .get("receiver")
            .cloned()
            .ok_or("Missing runtime receiver")?,
    )
    .map_err(|error| error.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .runtime_member_projections(&receiver)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("runtime_member_types");
    result["selection_verified"] = json!(false);
    Ok(result)
}

fn runtime_operation_application(request: &Value) -> Result<RuntimeOperationApplication, String> {
    // Preserve the prior required-field checks and sorted JSON field order while
    // deserializing borrowed values. No intermediate argument tree is cloned.
    let receiver = request.get("receiver").ok_or("Missing runtime receiver")?;
    let operation = request
        .get("operation")
        .ok_or("Missing exact operation declaration")?;
    let arguments = request
        .get("arguments")
        .ok_or("Missing operation type arguments")?;
    let owner = request.get("owner").unwrap_or(&Value::Null);
    let fields = [
        ("arguments", arguments),
        ("operation", operation),
        ("owner", owner),
        ("receiver", receiver),
    ];
    serde::Deserialize::deserialize(
        serde::de::value::MapDeserializer::<_, serde_json::Error>::new(fields.into_iter()),
    )
    .map_err(|error| error.to_string())
}
pub fn runtime_operation_types(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let application = runtime_operation_application(request)?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .instantiate_runtime_operation_in_owner(
                &application.receiver,
                &application.operation,
                application.owner.as_ref(),
                &application.arguments,
            )
            .map(|report| json!(report))
    }));
    result["stage"] = json!("runtime_operation_types");
    result["selection_verified"] = json!(false);
    Ok(result)
}

pub fn requested_member_types(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let receiver: intersect_model::definitions::generic_types::ClosedType = serde_json::from_value(
        request
            .get("receiver")
            .cloned()
            .ok_or("Missing requested closed receiver")?,
    )
    .map_err(|e| e.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .instantiate_members(&receiver)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_requested_members");
    result["selection_verified"] = json!(false);
    Ok(result)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Instance {
    id: String,
    #[serde(rename = "type")]
    ty: intersect_model::definitions::generic_types::RuntimeType,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationApplication {
    receiver: intersect_model::definitions::generic_types::ClosedType,
    operation: DeclarationId,
    arguments: Vec<intersect_model::definitions::generic_types::ClosedType>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeOperationApplication {
    receiver: intersect_model::definitions::generic_types::RuntimeType,
    operation: DeclarationId,
    arguments: Vec<intersect_model::definitions::generic_types::RuntimeType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner: Option<intersect_model::definitions::generic_types::Type>,
}
pub fn materialize(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    if request.get("conversions").is_some()
        || request.get("applied_conversions").is_some()
        || request.get("constructors").is_some()
    {
        let contracts: Vec<intersect_model::definitions::conversion::ConversionContract> =
            serde_json::from_value(request.get("conversions").cloned().unwrap_or(json!([])))
                .map_err(|e| e.to_string())?;
        let applications: Vec<intersect_model::definitions::conversion::AppliedConversionContract> =
            serde_json::from_value(
                request
                    .get("applied_conversions")
                    .cloned()
                    .unwrap_or(json!([])),
            )
            .map_err(|e| e.to_string())?;
        let constructors = crate::conversions::constructor_inputs(request, "constructors")?;
        return intersect_model::conversion::with_catalog_policies(
            &bundle,
            &contracts,
            &applications,
            &constructors,
            |registry| materialize_context(request, &bundle, Some(registry)),
        );
    }
    materialize_context(request, &bundle, None)
}
fn materialize_context(
    request: &Value,
    bundle: &DefinitionBundle,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
) -> Result<Value, String> {
    let types =
        serde_json::from_value::<Vec<intersect_model::definitions::generic_types::RuntimeType>>(
            request
                .get("types")
                .cloned()
                .ok_or("Missing materialization types")?,
        )
        .map_err(|e| e.to_string())?;
    let compiled = match (request.get("entry"), request.get("namespace")) {
        (None, None) => intersect_model::definitions::compiler::materialize_runtime_types(
            bundle,
            &types,
            conversions,
        ),
        (Some(entry), Some(namespace)) => {
            let entry = serde_json::from_value(entry.clone()).map_err(|error| error.to_string())?;
            let namespace = namespace
                .as_str()
                .ok_or("Materialization namespace must be a string")?;
            intersect_model::definitions::compiler::materialize_runtime_bundle(
                bundle,
                &entry,
                namespace,
                &types,
                conversions,
            )
        }
        _ => return Err("Selected materialization requires both entry and namespace".into()),
    };
    let schema = match compiled {
        Ok(schema) => schema,
        Err(error) => {
            return Ok(
                json!({"ok":false,"stage":"generic_schema","diagnostics":[{"span":error.span,"message":error.message}]}),
            );
        }
    };
    let applications: Vec<OperationApplication> = serde_json::from_value(
        request
            .get("operation_applications")
            .cloned()
            .unwrap_or(json!([])),
    )
    .map_err(|error| error.to_string())?;
    let runtime_applications: Vec<RuntimeOperationApplication> = serde_json::from_value(
        request
            .get("runtime_operation_applications")
            .cloned()
            .unwrap_or(json!([])),
    )
    .map_err(|error| error.to_string())?;
    let mut runtime_applied = Vec::new();
    let mut applied = Vec::new();
    let mut method_conversions = Vec::new();
    let mut method_constructors = Vec::new();
    if !applications.is_empty() || !runtime_applications.is_empty() {
        let resolver = Resolver::new(bundle).map_err(|error| error.to_string())?;
        let resolver = match conversions {
            Some(registry) => resolver
                .with_conversions(registry)
                .map_err(|error| error.to_string())?,
            None => resolver,
        };
        for application in &applications {
            match schema.instantiate_generic_operation(
                &resolver,
                &application.receiver,
                &application.operation,
                &application.arguments,
            ) {
                Ok(report) => applied.push(json!({"executable":false,"report":report})),
                Err(error) => {
                    return Ok(
                        json!({"ok":false,"stage":"generic_schema","diagnostics":[{"span":error.span,"message":error.message}]}),
                    );
                }
            }
        }
        for application in &runtime_applications {
            match schema.instantiate_runtime_operation(
                &resolver,
                &application.receiver,
                &application.operation,
                application.owner.as_ref(),
                &application.arguments,
            ) {
                Ok(report) => runtime_applied.push(json!({"executable":false,"report":report})),
                Err(error) => {
                    return Ok(json!({"ok":false,"stage":"generic_schema",
                    "diagnostics":[{"span":error.span,"message":error.message}]}));
                }
            }
        }
        method_constructors = resolver.consumed_constructor_contracts();
        if let Some(registry) = conversions {
            let consumed = resolver.consumed_relation_datatypes();
            method_conversions = registry
                .registered_contracts()?
                .into_iter()
                .filter(|contract| consumed.contains(&contract.datatype))
                .collect();
        }
    }
    // Method checks consume policies independently of the materialized schema.
    let required_conversions = required_selections(
        request,
        "conversions",
        &schema.conversion_contracts,
        &method_conversions,
        "Unused conversion provider selection",
    )?;
    let required_constructors = required_selections(
        request,
        "constructors",
        &schema.constructor_contracts,
        &method_constructors,
        "Unused constructor provider selection",
    )?;
    let required_applications = required_selections(
        request,
        "applied_conversions",
        &schema.applied_conversion_contracts,
        &[],
        "Unused applied provider selection",
    )?;
    let instances: Vec<Instance> =
        serde_json::from_value(request.get("instances").cloned().unwrap_or(json!([])))
            .map_err(|e| e.to_string())?;
    let edits: Vec<(String, String, Value)> =
        serde_json::from_value(request.get("edits").cloned().unwrap_or(json!([])))
            .map_err(|e| e.to_string())?;
    let mut model = intersect_model::core::model::Model::default();
    let preview = (|| -> Result<(), String> {
        for instance in instances {
            let key = schema.class_key_for_type(&instance.ty)?;
            model.create(&schema, &instance.id, key, "memory:materialized", 0, 0)?;
        }
        if let Some(registry) = conversions {
            if !edits.is_empty() {
                model.edit_with_conversions(&schema, &edits, registry)?;
            }
            model.validate_with_conversions(&schema, registry)
        } else {
            if !edits.is_empty() {
                model.edit(&schema, &edits)?;
            }
            model.validate(&schema)
        }
    })();
    match preview {
        Ok(()) => {
            let profile = if schema.runtime_warnings.is_empty() {
                "explicit_closed_set"
            } else {
                "explicit_runtime_set"
            };
            let mut result = json!({"ok":true,"stage":"generic_schema","profile":profile,"schema":schema,"model":model});
            if request.get("operation_applications").is_some()
                || request.get("runtime_operation_applications").is_some()
            {
                if request.get("operation_applications").is_some() {
                    result["operation_applications"] = json!(applied);
                }
                if request.get("runtime_operation_applications").is_some() {
                    result["runtime_operation_applications"] = json!(runtime_applied);
                }
                result["operation_dependencies"] = json!({
                    "conversions": method_conversions, "constructors": method_constructors,
                });
                // Re-enter normal source and provider validation; reports are not trusted inputs.
                result["operation_replay"] = json!({
                    "method": "ecore/materialize", "bundle": bundle, "types": types,
                    "operation_applications": applications,
                    "conversions": required_conversions,
                    "constructors": required_constructors,
                    "applied_conversions": required_applications,
                });
                if request.get("runtime_operation_applications").is_some() {
                    result["operation_replay"]["runtime_operation_applications"] =
                        json!(runtime_applications);
                }
                if let (Some(entry), Some(namespace)) =
                    (request.get("entry"), request.get("namespace"))
                {
                    result["operation_replay"]["entry"] = entry.clone();
                    result["operation_replay"]["namespace"] = namespace.clone();
                }
            }
            Ok(result)
        }
        Err(message) => {
            Ok(json!({"ok":false,"stage":"generic_schema","diagnostics":[{"message":message}]}))
        }
    }
}

// Compare typed contracts (including defaulted fields), and canonicalize replay selections.
fn required_selections<T>(
    request: &Value,
    field: &str,
    schema: &[T],
    method: &[T],
    unused: &str,
) -> Result<Vec<T>, String>
where
    T: Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned,
{
    let selected: Vec<T> = serde_json::from_value(request.get(field).cloned().unwrap_or(json!([])))
        .map_err(|error| error.to_string())?;
    let mut required = schema.to_vec();
    for contract in method {
        if !required.contains(contract) {
            required.push(contract.clone());
        }
    }
    if selected.len() != required.len() || selected.iter().any(|c| !required.contains(c)) {
        return Err(unused.into());
    }
    let mut keyed = required
        .into_iter()
        .map(|contract| serde_json::to_string(&contract).map(|key| (key, contract)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(keyed.into_iter().map(|(_, contract)| contract).collect())
}

pub fn operation_types(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let receiver = serde_json::from_value(
        request
            .get("receiver")
            .cloned()
            .ok_or("Missing closed receiver")?,
    )
    .map_err(|e| e.to_string())?;
    let operation = serde_json::from_value(
        request
            .get("operation")
            .cloned()
            .ok_or("Missing exact operation declaration")?,
    )
    .map_err(|e| e.to_string())?;
    let arguments: Vec<intersect_model::definitions::generic_types::ClosedType> =
        serde_json::from_value(
            request
                .get("arguments")
                .cloned()
                .ok_or("Missing operation type arguments")?,
        )
        .map_err(|e| e.to_string())?;
    let owner: Option<intersect_model::definitions::generic_types::Type> = request
        .get("owner")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| error.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .instantiate_operation_in_owner(&receiver, &operation, owner.as_ref(), &arguments)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_operation_types");
    result["selection_verified"] = json!(false);
    Ok(result)
}

pub fn operation_override(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let operation = serde_json::from_value(
        request
            .get("operation")
            .cloned()
            .ok_or("Missing exact operation declaration")?,
    )
    .map_err(|e| e.to_string())?;
    let base = serde_json::from_value(
        request
            .get("base")
            .cloned()
            .ok_or("Missing exact base operation declaration")?,
    )
    .map_err(|e| e.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .operation_override(&operation, &base)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_operation_override");
    result["profile"] = json!("declared_erasure");
    result["selection_verified"] = json!(false);
    Ok(result)
}

pub fn operation_selection(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let receiver = serde_json::from_value(
        request
            .get("receiver")
            .cloned()
            .ok_or("Missing closed receiver")?,
    )
    .map_err(|e| e.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .resolved_generic_operations(&receiver)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_operation_selection");
    result["profile"] = json!("declared_erasure");
    result["selection_verified"] = result["ok"].clone();
    Ok(result)
}

/// Validate an explicit datatype application without installing a codec or activating a package.
pub fn datatype_application(request: &Value) -> Result<Value, String> {
    let bundle = bundle(request)?;
    let application: intersect_model::definitions::generic_types::ClosedType =
        serde_json::from_value(
            request
                .get("application")
                .cloned()
                .ok_or("Missing requested datatype application")?,
        )
        .map_err(|error| error.to_string())?;
    let mut result = response(configured_resolver(request, &bundle)?.and_then(|resolver| {
        resolver
            .instantiate_datatype(&application)
            .map(|report| json!(report))
    }));
    result["stage"] = json!("generic_datatype_application");
    Ok(result)
}

#[cfg(test)]
mod runtime_operation_input_controls {
    use super::*;
    fn original(request: &Value) -> Result<RuntimeOperationApplication, String> {
        serde_json::from_value(json!({"receiver":request.get("receiver").ok_or("Missing runtime receiver")?,"operation":request.get("operation").ok_or("Missing exact operation declaration")?,"arguments":request.get("arguments").ok_or("Missing operation type arguments")?,"owner":request.get("owner")})).map_err(|error|error.to_string())
    }
    #[test]
    fn borrowed_runtime_operation_input_preserves_valid_data_and_error_order() {
        let declaration =
            json!({"source":{"uri":"memory:input","sha256":"a".repeat(64)},"path":[0]});
        let ty = json!({"kind":"classifier","declaration":declaration,"arguments":[],"raw":false});
        let good = json!({"receiver":ty,"operation":declaration,"arguments":[ty]});
        for variant in 0..10 {
            let mut input = good.clone();
            match variant {
                0 => {}
                1 => {
                    input["owner"] = ty.clone();
                }
                2 => {
                    input["owner"] = Value::Null;
                }
                3 => {
                    input.as_object_mut().unwrap().remove("receiver");
                }
                4 => {
                    input.as_object_mut().unwrap().remove("operation");
                }
                5 => {
                    input.as_object_mut().unwrap().remove("arguments");
                }
                6 => {
                    input["receiver"] = json!("bad");
                }
                7 => {
                    input["arguments"] = json!(["bad"]);
                }
                8 => {
                    input["owner"] = json!("bad");
                }
                _ => {
                    input["receiver"] = json!("bad");
                    input["arguments"] = json!(["bad"]);
                }
            }
            let old = original(&input).map(|value| serde_json::to_value(value).unwrap());
            let new = runtime_operation_application(&input)
                .map(|value| serde_json::to_value(value).unwrap());
            assert_eq!(new, old, "variant {variant}");
        }
    }
    #[test]
    fn runtime_method_budget_phase_profile_is_diagnostic_only() {
        let formals = |n| {
            (0..n)
                .map(|i| format!("<eTypeParameters name=\"T{i}\"/>"))
                .collect::<String>()
        };
        let leaves = (0..128)
            .map(|i| format!("<eClassifiers x:type=\"e:EClass\" name=\"Leaf{i}\"/>"))
            .collect::<String>();
        let text = format!(
            "<e:EPackage xmlns:e=\"http://www.eclipse.org/emf/2002/Ecore\" xmlns:x=\"http://www.w3.org/2001/XMLSchema-instance\" name=\"budget\" nsURI=\"urn:runtime-budget\"><eClassifiers x:type=\"e:EClass\" name=\"Wide\">{}</eClassifiers>{leaves}<eClassifiers x:type=\"e:EClass\" name=\"Methods\"><eOperations name=\"over\">{}</eOperations><eOperations name=\"small\">{}</eOperations></eClassifiers></e:EPackage>",
            formals(128),
            formals(128),
            formals(1)
        );
        let source = intersect_model::definitions::bundle::DefinitionSource::new(
            "memory:runtime-budget",
            intersect_model::definitions::bundle::SourceKind::Ecore,
            text,
        );
        let id = |path| DeclarationId {
            source: source.id.clone(),
            path,
        };
        let ty = |n, args: Vec<Value>| json!({"kind":"classifier","declaration":id(vec![n]),"arguments":args,"raw":false});
        let actuals = (0..128)
            .map(|i| ty(0, (0..128).map(|_| ty(i + 1, vec![])).collect()))
            .collect::<Vec<_>>();
        let request = json!({"bundle":{"version":1,"sources":[source],"namespaces":[]},"receiver":ty(129,vec![]),"operation":id(vec![129,0]),"arguments":actuals});
        let start = std::time::Instant::now();
        let bundle = bundle(&request).unwrap();
        let bundle_ms = start.elapsed().as_millis();
        let start = std::time::Instant::now();
        let application = runtime_operation_application(&request).unwrap();
        let input_ms = start.elapsed().as_millis();
        let start = std::time::Instant::now();
        let resolver = configured_resolver(&request, &bundle).unwrap().unwrap();
        let resolver_ms = start.elapsed().as_millis();
        let start = std::time::Instant::now();
        let result = resolver.instantiate_runtime_operation_in_owner(
            &application.receiver,
            &application.operation,
            application.owner.as_ref(),
            &application.arguments,
        );
        let operation_ms = start.elapsed().as_millis();
        let error = result.unwrap_err();
        assert_eq!(
            error.kind,
            intersect_model::definitions::ecore::ErrorKind::ResourceLimit
        );
        assert!(error.message.contains("budget"));
        println!(
            "DIAGNOSTIC_ONLY bundle_ms={bundle_ms} input_ms={input_ms} resolver_ms={resolver_ms} operation_ms={operation_ms}"
        );
    }
}
