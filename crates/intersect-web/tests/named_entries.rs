use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::Profile;
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[path = "../../../tests/text-acceptance/shared/xtext_imported_parameter_inputs.rs"]
mod imported_inputs;
const URI: &str = "memory:entry-host.model";
fn call(value: Value) -> Result<Value, String> {
    intersect_web::dispatch(&value.to_string())
}
fn read(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}
fn compiled(case: &Value) -> Value {
    let value=call(json!({"method":"compile_bundle","compilation":imported_inputs::input(case),"xtext_profile":Profile::NativeV57})).unwrap();
    assert_eq!(value["ok"], true, "{value}");
    value["package"].clone()
}
fn inferred(case: &Value) -> Value {
    let mut ids = BTreeMap::<String, intersect_model::definitions::bundle::SourceId>::new();
    let mut sources = vec![];
    for source in case["sources"].as_array().unwrap() {
        let mut value = DefinitionSource::new(
            format!(
                "memory:entry-host/{}/{}.xtext",
                case["id"].as_str().unwrap(),
                source["id"].as_str().unwrap()
            ),
            SourceKind::Xtext,
            source["source"].as_str().unwrap(),
        );
        value.dependencies = source["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| ids[d.as_str().unwrap()].clone())
            .collect();
        ids.insert(source["id"].as_str().unwrap().into(), value.id.clone());
        sources.push(value);
    }
    let request = intersect_text::inference::InferenceRequest {
        version: 1,
        grammar: ids[case["entry"].as_str().unwrap()].clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources,
            namespaces: vec![],
        },
    };
    let result = call(json!({"method":"compile_inferred","inference":request})).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    result["package"].clone()
}
fn projection(package: &Value, model: &Value) -> Value {
    let elements = model["elements"].as_array().unwrap();
    let class = |key: &Value| {
        let d = &package["schema"]["declarations"][key.as_str().unwrap()];
        format!(
            "{}#//{}",
            d["namespace"].as_str().unwrap(),
            d["name"].as_str().unwrap()
        )
    };
    fn value(original: &Value, elements: &[Value], class: &impl Fn(&Value) -> String) -> Value {
        if let Some(index) = elements.iter().position(|e| e["id"] == *original) {
            return json!({"object":index,"class":class(&elements[index]["class"])});
        }
        if let Some(items) = original.as_array() {
            return Value::Array(items.iter().map(|v| value(v, elements, class)).collect());
        }
        original.clone()
    }
    Value::Array(elements.iter().map(|e|json!({"class":class(&e["class"]),"values":e["values"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),value(v,elements,&class))).collect::<serde_json::Map<_,_>>()})).collect())
}
fn check(package: &Value, run: &Value, raw: Option<&Value>) {
    let request =
        json!({"method":"evaluate","package":package,"sources":[{"uri":URI,"text":run["text"]}]});
    let mut selected = request;
    if run["entry"] != "resource" {
        selected["entry_rule"] = run["entry"].clone();
    }
    let result = call(selected).unwrap();
    let parsed = run["exception"].is_null() && run["errors"].as_array().unwrap().is_empty();
    assert_eq!(
        result["diagnostics"].as_array().unwrap().is_empty(),
        parsed,
        "{} / {} / {}: {result}",
        run["case"],
        run["entry"],
        run["text"]
    );
    if run["accepted"] == true {
        assert_eq!(projection(package, &result["model"]), run["model"]);
    } else {
        assert!(result["model"]["elements"].as_array().unwrap().is_empty());
    }
    if run["entry"] == "resource" {
        assert!(result.get("entry_values").is_none());
        return;
    }
    if !parsed {
        assert_eq!(result["entry_values"], json!({}));
    } else if run["accepted"] == true {
        assert_eq!(
            result["entry_values"][URI],
            json!({"kind":"object","value":result["model"]["elements"][0]["id"]})
        );
    }
    if let Some(raw) = raw
        && raw["value_kind"] == "string"
        && parsed
    {
        assert_eq!(
            result["entry_values"][URI],
            json!({"kind":"datatype","value":raw["value"]})
        );
    }
}
#[test]
fn original_ordinary_named_observations_use_json_source_authority() {
    let cases = read(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ));
    let observed = read(include_str!(
        "../../../fixtures/xtext/imported-parameters/observations.json"
    ));
    let mut counts = [0, 0];
    for case in cases["cases"].as_array().unwrap() {
        let runs = observed["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["case"] == case["id"] && r["entry"] != "resource")
            .collect::<Vec<_>>();
        if runs.is_empty() {
            continue;
        }
        let package = compiled(case);
        let catalog = call(json!({"method":"text/entries","package":package})).unwrap();
        assert_eq!(catalog["default_entry"], package["grammar"]["entry"]);
        for run in runs {
            check(&package, run, None);
            counts[usize::from(run["accepted"] != true)] += 1;
        }
    }
    assert_eq!(counts, [6, 8]);
}
#[test]
fn supplemental_catalogs_scalar_results_and_unexecuted_boundaries() {
    let cases = read(include_str!(
        "../../../fixtures/xtext/parameter-entry-contexts/cases.json"
    ));
    let observed = read(include_str!(
        "../../../fixtures/xtext/parameter-entry-contexts/observations.json"
    ));
    let mut counts = [0, 0, 0, 0];
    for case in cases["cases"].as_array().unwrap() {
        let stage = observed["generation"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == case["id"])
            .unwrap();
        let built=call(json!({"method":"compile_bundle","compilation":imported_inputs::input(case),"xtext_profile":Profile::NativeV57})).unwrap();
        if stage["stage"] != "compiled" {
            assert_eq!(built["ok"], false);
            counts[1] += 1;
            continue;
        }
        assert_eq!(built["ok"], true, "{built}");
        let package = &built["package"];
        counts[0] += 1;
        let catalog = call(json!({"method":"text/entries","package":package})).unwrap();
        let names = stage["entry_methods"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                r["method"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("entryRule")
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            catalog["entries"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            names
        );
        for rule in catalog["entries"].as_object().unwrap().values() {
            let id = &rule["declaration"]["source"];
            let source = package["compilation"]["bundle"]["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == *id)
                .unwrap();
            let text = source["text"].as_str().unwrap();
            let start = rule["declaration"]["start"].as_u64().unwrap() as usize;
            let end = rule["declaration"]["end"].as_u64().unwrap() as usize;
            assert!(!text[start..end].is_empty());
        }
        for run in observed["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["case"] == case["id"])
        {
            let raw = observed["direct_entries"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| {
                    r["case"] == run["case"]
                        && r["entry"] == run["entry"]
                        && r["text"] == run["text"]
                });
            check(package, run, raw);
            counts[2] += 1;
            if raw.is_some_and(|r| {
                r["value_kind"] == "string" && r["errors"].as_array().unwrap().is_empty()
            }) {
                counts[3] += 1;
            }
        }
    }
    assert_eq!(counts, [7, 3, 38, 3]);
}
#[test]
fn inferred_named_contexts_keep_independent_rejections_and_native_counterparts() {
    let cases = read(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/cases.json"
    ));
    let observed = read(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/observations.json"
    ));
    let mut count = 0;
    for run in observed["runs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["entry"] != "resource")
    {
        let case = cases
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == run["case"])
            .unwrap();
        check(&inferred(case), run, None);
        count += 1;
    }
    assert_eq!(count, 2);
    let cases = read(include_str!(
        "../../../fixtures/xtext/inference-parameters/cases.json"
    ));
    for id in [
        "false_call",
        "recursive_context",
        "inherited_parameters",
        "parameterized_entry",
    ] {
        let case = cases["lowercase_counterparts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        let package = inferred(case);
        let entry = if id == "parameterized_entry" {
            "Model"
        } else {
            "Node"
        };
        let text = if id == "recursive_context" {
            "no alpha"
        } else {
            "alpha"
        };
        let catalog = call(json!({"method":"text/entries","package":package})).unwrap();
        assert!(catalog["entries"].get(entry).is_some());
        let result=call(json!({"method":"evaluate","package":package,"entry_rule":entry,"sources":[{"uri":URI,"text":text}]})).unwrap();
        assert_eq!(result["diagnostics"], json!([]));
        assert!(!result["model"]["elements"].as_array().unwrap().is_empty());
        assert_eq!(result["entry_values"][URI]["kind"], "object");
    }
}
#[test]
fn cached_programs_cannot_supply_entries_and_multi_source_parsing_is_atomic_per_resource() {
    let cases = read(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ));
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "false_call")
        .unwrap();
    let package = compiled(case);
    let catalog = call(json!({"method":"text/entries","package":package})).unwrap();
    let mut forged = package.clone();
    forged["grammar"]["entry"] = json!("FORGED");
    forged["sources"] = json!({});
    assert_eq!(
        call(json!({"method":"text/entries","package":forged})).unwrap(),
        catalog
    );
    for edit in 0..3 {
        let mut bad = package.clone();
        match edit {
            0 => bad["version"] = json!(0),
            1 => bad["compilation"]["xtext"] = Value::Null,
            _ => bad["compilation"]["bundle"]["sources"][0]["text"] = json!("wrong source bytes"),
        };
        assert!(call(json!({"method":"text/entries","package":bad})).is_err());
    }
    let mut bad = package.clone();
    bad["source_policy"] = json!(intersect_text::package::CompilationSourcePolicy::default());
    bad["source_policy"]["xtext_bytes"] = json!(256001);
    assert!(call(json!({"method":"text/entries","package":bad})).is_err());
    assert!(
        call(json!({"method":"evaluate","package":package,"entry_rule":false,"sources":[]}))
            .is_err()
    );
    let sources = json!([{"uri":"memory:a","text":"alpha"},{"uri":"memory:b","text":"beta"}]);
    let result =
        call(json!({"method":"evaluate","package":package,"entry_rule":"Node","sources":sources}))
            .unwrap();
    assert_eq!(result["diagnostics"], json!([]));
    assert_eq!(result["entry_values"].as_object().unwrap().len(), 2);
    let result=call(json!({"method":"evaluate","package":package,"entry_rule":"Node","sources":[{"uri":"memory:a","text":"alpha"},{"uri":"memory:b","text":"alpha beta"}]})).unwrap();
    assert!(!result["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(
        result["entry_values"],
        json!({"memory:a":{"kind":"object","value":"memory:a#0"}})
    );
    assert_eq!(result["model"]["elements"].as_array().unwrap().len(), 1);
    assert_eq!(result["model"]["elements"][0]["resource"], "memory:a");
    assert_eq!(
        result["model"]["resources"]["memory:a"]["roots"],
        json!(["memory:a#0"])
    );
    assert!(result["model"]["resources"]["memory:a"]["source"].is_object());
    assert_eq!(result["model"]["resources"]["memory:b"]["roots"], json!([]));
    assert!(result["model"]["resources"]["memory:b"]["source"].is_null());
    let result = call(
        json!({"method":"evaluate","package":package,"entry_rule":"FORGED","sources":sources}),
    )
    .unwrap();
    assert_eq!(result["model"]["resources"], json!({}));
    assert_eq!(result["entry_values"], json!({}));
    assert_eq!(
        call(json!({"method":"text/entries","package":package})).unwrap(),
        catalog
    );
}
