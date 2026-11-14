//! Browser host policy and JSON transport. Contracts, checks, reads and execution
//! are supplied by the shared definitions/core/language crates.
use intersect_model::definitions::{
    bundle::DefinitionBundle,
    compiler::materialize_runtime_bundle,
    conversion::{ConversionRegistry, ReferenceContext},
    rcore_behavior::{
        ir_v2::{DataWire, TypeNode, ValueWire},
        types::Scalar,
    },
    semantic_bindings::{SourceContracts, compile_sources, compile_sources_with_model_uri},
};
use intersect_model::{
    model_reference::ReferenceDomain,
    rcore_behavior_functions::{ExecutionContext, ExecutionLimits},
    semantic_bindings::{Budget, Call, NativeProvider, NativeValue, Registry, rhai::RhaiProvider},
};
use intersect_model_test_support::contract as fixture;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
mod generated {
    include!(concat!(env!("OUT_DIR"), "/twice.rs"));
    include!(concat!(env!("OUT_DIR"), "/doubled.rs"));
}
struct Active {
    sources: SourceContracts,
    registry: Registry,
}
thread_local! { static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) }; }
#[cfg(test)]
#[path = "semantic_bindings_derived_tests.rs"]
mod derived_tests;
fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing {name}"))
}
fn inspect(sources: &SourceContracts) -> Result<Value, String> {
    let (_, native, _) = fixture::contracts()?;
    Ok(
        json!({"ok":true,"contracts":sources.contracts.iter().map(|c|json!({
        "descriptor":c.descriptor(),"stub":c.rhai_stub(),"properties":c.model_properties(),"native":native.iter().any(|n|n.id()==c.id())
    })).collect::<Vec<_>>()}),
    )
}
fn compile_request(request: &Value) -> Result<SourceContracts, String> {
    let structure = string(request, "structure")?;
    let behavior = string(request, "behavior")?;
    if request["project"].as_bool() == Some(true) {
        // The workbench compiler identifies its authored Rcore as memory:model.rcore.
        // Retain that exact identity instead of relabeling nominal model values.
        compile_sources_with_model_uri(structure, behavior, "memory:model.rcore")
    } else {
        compile_sources(structure, behavior)
    }
}
pub fn handle(request: &Value) -> Result<Value, String> {
    let outcome = match string(request, "method")? {
        "bindings/projectInspect" => project_inspect(request),
        "bindings/stubs" => stubs(request),
        "bindings/inspect" => compile_request(request).and_then(|s| inspect(&s)),
        "bindings/activate" => activate(request),
        "bindings/invoke" => ACTIVE.with(|active| {
            let state = active
                .try_borrow()
                .map_err(|_| "Binding invocation is already active")?;
            invoke(
                state
                    .as_ref()
                    .ok_or("Activate an implementation package first")?,
                request,
            )
        }),
        _ => Err("Unknown bindings method".into()),
    };
    Ok(outcome.unwrap_or_else(|message| json!({"ok":false,"message":message})))
}
fn project_inspect(request: &Value) -> Result<Value, String> {
    use intersect_model::definitions::{
        bundle::{DefinitionSource, SourceKind},
        rcore_behavior::ast::Declaration,
    };
    let structure = string(request, "structure")?;
    if structure.len() > 262_144 || string(request, "behavior")?.len() > 262_144 {
        return Err("Binding definition exceeds 256 KiB".into());
    }
    let separate = string(request, "behavior")?;
    let source = if separate.trim().is_empty() {
        DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, structure)
    } else {
        DefinitionSource::new(
            "memory:bindings/behavior.rcore",
            SourceKind::RcoreBehavior,
            separate,
        )
    };
    let document = intersect_model::definitions::rcore_behavior::parse(source);
    let functions = document
        .declarations
        .iter()
        .filter(|d| matches!(d.value, Declaration::Function { .. }))
        .collect::<Vec<_>>();
    // The file workflow is opt-in through external declarations. Leave ordinary
    // typed rule/function diagnostics with the existing package compiler.
    if !functions
        .iter()
        .any(|d| matches!(d.value, Declaration::Function { external: true, .. }))
    {
        return Ok(json!({"ok":true,"contracts":[]}));
    }
    if !document.diagnostics.is_empty() {
        return Err(format!("{:?}", document.diagnostics));
    }
    let sources = compile_request(request)?;
    let mut result = inspect(&sources)?;
    result["model_behavior"] = json!(sources.model_behavior);
    Ok(result)
}
fn stubs(request: &Value) -> Result<Value, String> {
    let sources = compile_request(request)?;
    let mut source = string(request, "source")?.to_owned();
    let functions = RhaiProvider::functions(&source).map_err(|e| e.to_string())?;
    let mut names = std::collections::BTreeSet::new();
    let mut added = 0;
    for contract in sources.contracts.iter().filter(|c| c.descriptor().external) {
        let name = contract.script_name();
        if !names.insert(name.clone()) {
            return Err(format!(
                "External overloads are unsupported in a Rhai file: {name}"
            ));
        }
        if let Some((_, count)) = functions.iter().find(|(n, _)| n == &name) {
            if *count != contract.descriptor().parameters.len() {
                return Err(format!(
                    "Signature changed for {name}; update its existing implementation parameters"
                ));
            }
        } else {
            if !source.is_empty() && !source.ends_with('\n') {
                source.push('\n');
            }
            source.push_str(&contract.rhai_stub());
            added += 1;
        }
    }
    // Reject identifiers unsupported by Rhai, without publishing a partial file.
    RhaiProvider::functions(&source).map_err(|e| e.to_string())?;
    Ok(json!({"ok":true,"source":source,"added":added,"contracts":inspect(&sources)?["contracts"]}))
}
fn prepare(request: &Value) -> Result<(Active, Value), String> {
    let sources = compile_request(request)?;
    let selections = request["implementations"]
        .as_array()
        .ok_or("Missing implementations")?;
    if selections.len() != sources.contracts.len() {
        return Err("Select exactly one implementation per contract".into());
    }
    let (_, native, _) = fixture::contracts()?;
    let mut registry = Registry::default();
    for selection in selections {
        let id = string(selection, "contract")?;
        let c = sources
            .contracts
            .iter()
            .find(|c| c.id() == id)
            .ok_or("Stale or unknown contract; inspect current sources")?;
        match string(selection, "backend")? {
            "rcore" => registry.install_rcore(c).map_err(|e| e.to_string())?,
            "rhai" => registry
                .install(
                    c,
                    Rc::new(
                        RhaiProvider::compile(
                            c,
                            "memory:behavior.rhai",
                            string(selection, "source")?,
                        )
                        .map_err(|e| e.to_string())?,
                    ),
                )
                .map_err(|e| e.to_string())?,
            backend @ ("rust-generated" | "rust-dynamic") => {
                let index = native
                    .iter()
                    .position(|n| n.id() == c.id())
                    .ok_or("Rust example requires the exact compiled fixture contract")?;
                if backend == "rust-generated" {
                    match index {
                        0 => generated::register_twice(&mut registry, c),
                        _ => generated::register_doubled(&mut registry, c),
                    }
                    .map_err(|e| e.to_string())?;
                } else {
                    registry
                        .install(
                            c,
                            Rc::new(NativeProvider::new(
                                c,
                                "manual-example-v1",
                                move |call: &Call<'_>, args: &[ValueWire]| {
                                    let value = if index == 0 {
                                        crate::native::twice(call, i64::from_wire(&args[0])?)?
                                    } else {
                                        crate::native::doubled(call, call.model(0)?)?
                                    };
                                    call.result(value)
                                },
                            )),
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
            _ => return Err("Unsupported implementation backend".into()),
        }
    }
    // All compilation/registration succeeds before publishing this replacement.
    let mut result = inspect(&sources)?;
    result["implementations"] = json!(
        sources
            .contracts
            .iter()
            .map(|c| registry.pin(c).map(|p| p.identity().clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    );
    Ok((Active { sources, registry }, result))
}
fn activate(request: &Value) -> Result<Value, String> {
    let (next, result) = prepare(request)?;
    ACTIVE.with(|active| -> Result<(), String> {
        *active
            .try_borrow_mut()
            .map_err(|_| "Binding invocation is already active")? = Some(next);
        Ok(())
    })?;
    Ok(result)
}

/// Publish only after the model compiler and all provider checks succeed.
pub fn install_for_package(
    package: &intersect_text::package::Package,
    request: Option<&Value>,
) -> Result<(), String> {
    let next = if let Some(request) = request {
        let (next, _) = prepare(request)?;
        intersect_model::semantic_bindings::derived::BoundDerived::new(
            package,
            &next.sources,
            &next.registry,
            Budget::new(100_000),
        )?;
        Some(next)
    } else {
        if package.semantics.derived.iter().any(|r| {
            matches!(
                r.expression,
                intersect_model::definitions::schema::Expression::External { .. }
            )
        }) {
            return Err("External derived properties require compiled implementations".into());
        }
        None
    };
    ACTIVE.with(|active| -> Result<(), String> {
        *active
            .try_borrow_mut()
            .map_err(|_| "Binding invocation is already active")? = next;
        Ok(())
    })
}

/// The host selects a registry explicitly; semantics remain in intersect-model.
pub struct Derived<'a> {
    package: &'a intersect_text::package::Package,
    budget: Budget,
}
impl<'a> Derived<'a> {
    pub fn new(package: &'a intersect_text::package::Package) -> Self {
        Self {
            package,
            budget: Budget::new(100_000),
        }
    }
}
impl intersect_model::core::query::DerivedValues for Derived<'_> {
    fn schema_revision(&self) -> Option<&str> {
        Some(&self.package.schema.revision)
    }
    fn supports(&self, class: &str, feature: &str) -> bool {
        intersect_model::query::PackageDerived(self.package).supports(class, feature)
    }
    fn explicit_set_state(
        &self,
        store: &dyn intersect_model::core::storage::ModelRead,
        id: &str,
        feature: &str,
        budget: &mut intersect_model::core::query::QueryBudget,
    ) -> Result<bool, String> {
        intersect_model::query::PackageDerived(self.package)
            .explicit_set_state(store, id, feature, budget)
    }
    fn read(
        &self,
        store: &dyn intersect_model::core::storage::ModelRead,
        id: &str,
        feature: &str,
        budget: &mut intersect_model::core::query::QueryBudget,
    ) -> Result<Value, String> {
        let class = store.class(id)?.ok_or("Missing derived object")?;
        let external = self.package.semantics.derived.iter().any(|r| {
            r.feature == feature
                && self.package.schema.conforms(&class, &r.class)
                && matches!(
                    r.expression,
                    intersect_model::definitions::schema::Expression::External { .. }
                )
        });
        if !external {
            return intersect_model::query::PackageDerived(self.package)
                .read(store, id, feature, budget);
        }
        ACTIVE.with(|active| {
            let state = active
                .try_borrow()
                .map_err(|_| "Binding invocation is already active")?;
            let state = state
                .as_ref()
                .ok_or("External derived properties require compiled implementations")?;
            intersect_model::semantic_bindings::derived::BoundDerived::new(
                self.package,
                &state.sources,
                &state.registry,
                self.budget.clone(),
            )?
            .read(store, id, feature, budget)
        })
    }
}

fn invoke(active: &Active, request: &Value) -> Result<Value, String> {
    let c = active
        .sources
        .contracts
        .iter()
        .find(|c| Some(c.id()) == request["contract"].as_str())
        .ok_or("Contract is not in the active package")?;
    let raw = request["arguments"]
        .as_array()
        .ok_or("Arguments must be an array")?;
    if raw.len() != c.descriptor().parameters.len() {
        return Err("Wrong argument count".into());
    }
    let mut references = Vec::new();
    let mut arguments = Vec::new();
    for (input, ty) in raw.iter().zip(&c.descriptor().parameters) {
        arguments.push(match c.descriptor().types.get(ty.0 as usize) {
            Some(TypeNode::Scalar(Scalar::Int64)) => input
                .as_str()
                .ok_or("Int64 input must be a decimal string")?
                .parse::<i64>()
                .map_err(|e| e.to_string())?
                .into_wire(*ty)
                .map_err(|e| e.to_string())?,
            Some(TypeNode::Scalar(Scalar::Bool)) => input
                .as_bool()
                .ok_or("Expected Bool")?
                .into_wire(*ty)
                .map_err(|e| e.to_string())?,
            Some(TypeNode::Scalar(Scalar::String)) => input
                .as_str()
                .ok_or("Expected String")?
                .to_owned()
                .into_wire(*ty)
                .map_err(|e| e.to_string())?,
            Some(TypeNode::Model(receiver)) => {
                let source = &active.sources.model_source;
                let bundle = DefinitionBundle {
                    version: 1,
                    sources: vec![source.clone()],
                    namespaces: vec![],
                };
                let schema = materialize_runtime_bundle(
                    &bundle,
                    &source.id,
                    &active.sources.namespace,
                    &active.sources.receiver_types,
                    None,
                )
                .map_err(|e| e.to_string())?;
                let mut model = intersect_model::core::model::Model::default();
                let elements = request["model"]
                    .as_array()
                    .ok_or("Expected model elements array")?;
                if elements.len() > 256 {
                    return Err("Model input exceeds 256 elements".into());
                }
                for element in elements {
                    let class = string(element, "class")?;
                    let model_type = active
                        .sources
                        .model_classes
                        .get(class)
                        .ok_or("Unknown model class")?;
                    model.create(
                        &schema,
                        string(element, "id")?,
                        schema.class_key_for_type(model_type)?,
                        "memory:bindings-model",
                        0,
                        0,
                    )?;
                }
                let mut edits = Vec::new();
                for element in elements {
                    for (feature, value) in element["values"]
                        .as_object()
                        .ok_or("Expected model values object")?
                    {
                        edits.push((
                            string(element, "id")?.to_owned(),
                            feature.clone(),
                            value.clone(),
                        ));
                    }
                }
                model.edit(&schema, &edits)?;
                model.validate(&schema)?;
                let reference = ReferenceContext {
                    bundle,
                    entry: source.id.clone(),
                    namespace: active.sources.namespace.clone(),
                    conversions: vec![],
                    materialized_types: active.sources.receiver_types.clone(),
                    applied_conversions: vec![],
                    constructors: vec![],
                    model: serde_json::to_value(model).map_err(|e| e.to_string())?,
                };
                let payload = ReferenceDomain::new_runtime(
                    receiver.clone(),
                    reference.clone(),
                    &ConversionRegistry::default(),
                )?
                .bind(input.as_str().ok_or("Expected model element id")?)?;
                // Every model argument uses this request's same immutable snapshot.
                // The evaluator requires each context identity exactly once.
                if references.is_empty() {
                    references.push(reference);
                }
                ValueWire {
                    ty: *ty,
                    data: DataWire::Model(payload),
                }
            }
            _ => {
                return Err(
                    "This browser input profile supports Int64, Bool, String and model arguments"
                        .into(),
                );
            }
        });
    }
    let work = request
        .get("budget")
        .and_then(Value::as_u64)
        .unwrap_or(100_000)
        .min(1_000_000);
    let result = active.registry.pin(c).map_err(|e| e.to_string())?.invoke(
        &arguments,
        &ExecutionContext {
            bundle: &active.sources.bundle,
            conversions: None,
            references: &references,
        },
        &ExecutionLimits::default(),
        Budget::new(work),
    );
    Ok(match result {
        Ok(result) => json!({"ok":true,"result":result}),
        Err(error) => json!({"ok":false,"diagnostic":error}),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_properties_preserve_inheritance_types_defaults_and_receiver_identity() {
        let mut request = json!({"method":"bindings/inspect", "behavior":"", "structure":r#"
rcore 1; package properties version "1";
metamodel "urn:properties" prefix p {
  abstract class Named { attr name: String [1,1]; }
  class Meter extends Named {
    attr value: Int64 [1,1];
    attr narrow: Int32 [0,1] default -2147483648;
    attr enabled: Bool [0,1] default false;
    attr note: String [0,1];
    attr computed: Int64 [1,1] derived;
    attr items: Int64 [0,*];
    ref other: Other [0,1];
  }
  class Other { attr value: String [1,1]; }
}
behavior {
  import model "urn:properties" as m;
  extern fn inspect(a:m.Meter,b:m.Other)->String;
}
"#});
        let inspected = handle(&request).unwrap();
        assert_eq!(inspected["ok"], true, "{inspected}");
        let bindings = inspected["contracts"][0]["properties"].as_array().unwrap();
        let meter = bindings.iter().find(|b| b["parameter"] == 0).unwrap();
        assert_eq!(meter["class_name"], "Meter");
        let properties = meter["properties"].as_array().unwrap();
        let property = |name: &str| properties.iter().find(|p| p["name"] == name).unwrap();
        assert_eq!(property("name")["scalar"], "String");
        assert_eq!(property("name")["optional"], false);
        assert_eq!(property("narrow")["scalar"], "Int32");
        assert_eq!(property("note")["optional"], true);
        for name in ["computed", "items", "other"] {
            assert!(property(name)["unsupported"].is_string());
        }
        let contract = inspected["contracts"][0]["descriptor"]["id"].clone();
        request["method"] = json!("bindings/activate");
        let script = r#"fn inspect(a,b) {
            a.name + ":" + a.value + ":" + a.narrow + ":" + a.enabled + ":" + (a.note == ()) + ":" + b.value
        }"#;
        request["implementations"] =
            json!([{"contract":contract,"backend":"rhai","source":script}]);
        assert_eq!(handle(&request).unwrap()["ok"], true);
        let mut query = json!({"method":"bindings/invoke","contract":contract,"arguments":["meter","other"],"model":[
            {"id":"meter","class":"Meter","values":{"name":"meter","value":{"kind":"int64","value":"9223372036854775807"}}},
            {"id":"other","class":"Other","values":{"value":"text"}}
        ]});
        let result = handle(&query).unwrap();
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(
            result["result"]["evaluation"]["value"]["data"]["Scalar"]["payload"],
            "meter:9223372036854775807:-2147483648:false:true:text"
        );
        query["model"][0]["values"]["value"] =
            json!({"kind":"int64","value":"-9223372036854775808"});
        let minimum = handle(&query).unwrap();
        assert_eq!(minimum["ok"], true, "{minimum}");
        assert_eq!(
            minimum["result"]["evaluation"]["value"]["data"]["Scalar"]["payload"],
            "meter:-9223372036854775808:-2147483648:false:true:text"
        );
        // The same property spelling must dispatch by checked receiver type.
        // Explicit failures retain the adapter's read-only and stored-only scope.
        for (body, message) in [
            ("a.missing", "missing"),
            ("a.computed", "stored attributes"),
            ("a.other", "Reference traversal"),
            ("a.items", "Collection properties"),
            ("a.value = 3; a.name", "value"),
            ("b.name", "not declared"),
        ] {
            request["implementations"][0]["source"] =
                json!(format!("fn inspect(a,b) {{ {body} }}"));
            assert_eq!(handle(&request).unwrap()["ok"], true);
            let rejected = handle(&query).unwrap();
            assert_eq!(rejected["ok"], false, "{rejected}");
            assert!(
                rejected["diagnostic"]["message"]
                    .as_str()
                    .unwrap()
                    .contains(message),
                "{rejected}"
            );
        }
    }
    #[test]
    fn library_project_preserves_workbench_enum_identity_for_rhai_queries() {
        let mut request = json!({
            "method":"bindings/inspect", "project":true, "behavior":"",
            "structure":include_str!("../../../fixtures/library/library-implementations.rcore")
        });
        let sources = compile_request(&request).unwrap();
        let schema = intersect_model::definitions::rcore::compile_structure(
            &sources.bundle,
            &sources.model_source.id,
        )
        .unwrap()
        .schema;
        let mut model = intersect_model::core::model::Model::default();
        model
            .create(&schema, "copy", "BookCopy", "memory:catalog", 0, 0)
            .unwrap();
        model
            .edit(
                &schema,
                &[
                    ("copy".into(), "name".into(), json!("earthsea")),
                    ("copy".into(), "edition".into(), json!(2)),
                ],
            )
            .unwrap();
        let inspected = handle(&request).unwrap();
        assert_eq!(inspected["ok"], true, "{inspected}");
        request["method"] = json!("bindings/activate");
        request["implementations"] =
            json!(inspected["contracts"].as_array().unwrap().iter().map(|c| json!({
            "contract":c["descriptor"]["id"], "backend":"rhai",
            "source":include_str!("../../../fixtures/library/library-implementations.rhai")
        })).collect::<Vec<_>>());
        let active = handle(&request).unwrap();
        assert_eq!(active["ok"], true, "{active}");
        let contract = inspected["contracts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["descriptor"]["name"] == "copyLabel")
            .unwrap()["descriptor"]["id"]
            .clone();
        let mut query = json!({"method":"bindings/invoke", "contract":contract, "arguments":["copy"], "model":model.elements});
        let result = handle(&query).unwrap();
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(
            result["result"]["evaluation"]["value"]["data"]["Scalar"]["payload"],
            "earthsea (edition 2)"
        );
        // The alignment must not make nominal values from a foreign source valid.
        let foreign = compile_sources(request["structure"].as_str().unwrap(), "").unwrap();
        let foreign_schema = materialize_runtime_bundle(
            &foreign.bundle,
            &foreign.model_source.id,
            &foreign.namespace,
            &foreign.receiver_types,
            None,
        )
        .unwrap();
        let class = foreign_schema
            .class_key_for_type(&foreign.model_classes["BookCopy"])
            .unwrap();
        query["model"][0]["values"]["condition"] =
            foreign_schema.features(class).unwrap()["condition"]
                .default
                .clone();
        let rejected = handle(&query).unwrap();
        assert_eq!(rejected["ok"], false, "{rejected}");
        assert!(
            rejected["message"]
                .as_str()
                .unwrap()
                .contains("another declaration or revision")
        );
    }
    #[test]
    fn project_binding_workflow_only_handles_projects_with_external_declarations() {
        let mut request = draft();
        request["method"] = json!("bindings/projectInspect");
        request["behavior"] =
            json!("rcore 1; package existing version \"1\"; fn bad()->Int64 { true }");
        // The ordinary compiler must retain its original source/type diagnostic.
        let inspected = handle(&request).unwrap();
        assert_eq!(inspected["contracts"], json!([]));
        request["structure"] = json!(include_str!(
            "../../../fixtures/rcore/bindings/external-project.rcore"
        ));
        request["behavior"] = json!("");
        let inspected = handle(&request).unwrap();
        assert_eq!(inspected["contracts"].as_array().unwrap().len(), 2);
        assert!(
            inspected["model_behavior"]
                .as_str()
                .unwrap()
                .contains("extern fn")
        );
    }
    #[test]
    fn external_signatures_generate_preserved_stubs_and_execute_without_reference_bodies() {
        let mut request = draft();
        request["behavior"] = json!(
            "rcore 1; package external.test version \"1\"; extern fn twice(value:Int64)->Int64;"
        );
        request["method"] = json!("bindings/stubs");
        request["source"] = json!("");
        let stubs = handle(&request).unwrap();
        assert_eq!(stubs["ok"], true, "{stubs}");
        assert!(
            stubs["source"]
                .as_str()
                .unwrap()
                .contains("fn twice(value)")
        );
        assert_eq!(stubs["contracts"][0]["descriptor"]["external"], true);
        request["source"] = json!("fn twice(value) { value * 2 }");
        let preserved = handle(&request).unwrap();
        assert_eq!(preserved["source"], request["source"]);
        assert_eq!(preserved["added"], 0);
        request["source"] = json!("// fn twice(value) {}\n");
        assert_eq!(handle(&request).unwrap()["added"], 1);
        request["source"] = json!("fn twice(a,b) {a+b}");
        assert!(
            handle(&request).unwrap()["message"]
                .as_str()
                .unwrap()
                .contains("Signature changed")
        );
        request["source"] = json!("fn twice(value) { value * 2 }");
        let id = stubs["contracts"][0]["descriptor"]["id"].clone();
        request["method"] = json!("bindings/activate");
        request["implementations"] = json!([{"contract":id,"backend":"rcore"}]);
        assert_eq!(handle(&request).unwrap()["ok"], false);
        request["implementations"] =
            json!([{"contract":id,"backend":"rhai","source":request["source"]}]);
        let active = handle(&request).unwrap();
        assert_eq!(active["ok"], true, "{active}");
        let run =
            handle(&json!({"method":"bindings/invoke","contract":id,"arguments":["21"]})).unwrap();
        assert_eq!(run["ok"], true, "{run}");
        assert_eq!(
            run["result"]["evaluation"]["value"]["data"]["Scalar"]["payload"]["value"],
            "42"
        );
    }
    fn draft() -> Value {
        json!({"method":"bindings/inspect","structure":include_str!("../../../fixtures/rcore/bindings/model.rcore"),"behavior":include_str!("../../../fixtures/rcore/bindings/behavior.rcore")})
    }
    #[test]
    fn browser_host_runs_every_backend_and_keeps_last_successful_package() {
        let mut request = draft();
        let inspected = handle(&request).unwrap();
        assert_eq!(inspected["ok"], true, "{inspected}");
        assert_eq!(inspected["contracts"].as_array().unwrap().len(), 2);
        let free = inspected["contracts"][0]["descriptor"]["id"].clone();
        let method = inspected["contracts"][1]["descriptor"]["id"].clone();
        for backend in ["rcore", "rust-generated", "rust-dynamic", "rhai"] {
            request["method"] = json!("bindings/activate");
            request["implementations"] = json!([
                {"contract":free,"backend":backend,"source":include_str!("../../../fixtures/rcore/bindings/behavior.rhai")},
                {"contract":method,"backend":"rhai","source":include_str!("../../../fixtures/rcore/bindings/behavior.rhai")}
            ]);
            let result = handle(&request).unwrap();
            assert_eq!(result["ok"], true, "{result}");
            let run = json!({"method":"bindings/invoke","contract":free,"arguments":["21"]});
            let result = handle(&run).unwrap();
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(
                result["result"]["evaluation"]["value"]["data"]["Scalar"]["payload"]["value"], "42",
                "{result}"
            );
            let run_model = json!({"method":"bindings/invoke","contract":method,"arguments":["meter"],"model":[{"id":"meter","class":"Meter","values":{"value":{"kind":"int64","value":"21"}}}]});
            let result = handle(&run_model).unwrap();
            assert_eq!(result["ok"], true, "{result}");
            request["implementations"][0]["backend"] = json!("rhai");
            request["implementations"][0]["source"] = json!("broken source");
            assert_eq!(handle(&request).unwrap()["ok"], false);
            assert_eq!(handle(&run).unwrap()["ok"], true);
        }
    }

    #[test]
    fn multiple_model_arguments_share_one_admitted_context() {
        let mut request = draft();
        request["behavior"] = json!(
            "rcore 1; package pair version \"1\"; import model \"urn:intersect:bindings\" as m; extern fn sum(a:m.Meter,b:m.Meter)->Int64;"
        );
        let inspected = handle(&request).unwrap();
        assert_eq!(inspected["ok"], true, "{inspected}");
        let contract = inspected["contracts"][0]["descriptor"]["id"].clone();
        request["method"] = json!("bindings/activate");
        request["implementations"] = json!([{"contract":contract,"backend":"rhai","source":"fn sum(a,b) { a.value + b.value }"}]);
        let activated = handle(&request).unwrap();
        assert_eq!(activated["ok"], true, "{activated}");
        let result = handle(&json!({"method":"bindings/invoke","contract":contract,"arguments":["left","right"],"model":[
            {"id":"left","class":"Meter","values":{"value":{"kind":"int64","value":"5"}}},
            {"id":"right","class":"Meter","values":{"value":{"kind":"int64","value":"8"}}}
        ]})).unwrap();
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(
            result["result"]["evaluation"]["value"]["data"]["Scalar"]["payload"]["value"],
            "13"
        );
        assert_eq!(
            result["result"]["evaluation"]["consumed"]["model_contexts"],
            1
        );
    }
}
