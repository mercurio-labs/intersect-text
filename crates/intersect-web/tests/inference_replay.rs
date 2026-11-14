use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/cases.json"
    ))
    .unwrap()
}
fn inputs(case: &Value) -> Value {
    let mut ids = BTreeMap::<String, intersect_model::definitions::bundle::SourceId>::new();
    let mut sources = vec![];
    for source in case["sources"].as_array().unwrap() {
        let mut definition = DefinitionSource::new(
            format!(
                "memory:host-inference/{}/{}.xtext",
                case["id"].as_str().unwrap(),
                source["id"].as_str().unwrap()
            ),
            SourceKind::Xtext,
            source["source"].as_str().unwrap(),
        );
        definition.dependencies = source["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| ids[id.as_str().unwrap()].clone())
            .collect();
        ids.insert(
            source["id"].as_str().unwrap().to_owned(),
            definition.id.clone(),
        );
        sources.push(definition);
    }
    json!({"version":1,"bundle":DefinitionBundle{version:1,sources,namespaces:vec![]},"grammar":ids[case["entry"].as_str().unwrap()]})
}
fn invoke(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string())
        .unwrap_or_else(|e| panic!("{}: {e}", request["method"]))
}
fn compile(inputs: &Value) -> Value {
    let result = invoke(json!({"method":"compile_inferred","inference":inputs}));
    assert_eq!(result["ok"], true, "{result}");
    result["package"].clone()
}
fn evaluate(package: &Value, text: &str) -> Value {
    invoke(
        json!({"method":"evaluate","package":package,"sources":[{"uri":"memory:host-model.test","text":text}]}),
    )
}
fn projection(package: &Value, model: &Value) -> Value {
    fn uri(package: &Value, class: &Value) -> String {
        let d = &package["schema"]["declarations"][class.as_str().unwrap()];
        format!(
            "{}#//{}",
            d["namespace"].as_str().unwrap(),
            d["name"].as_str().unwrap()
        )
    }
    fn value(package: &Value, model: &Value, v: &Value) -> Value {
        if let Some(id) = v.as_str()
            && let Some((i, e)) = model["elements"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .find(|(_, e)| e["id"] == id)
        {
            return json!({"object":i,"class":uri(package,&e["class"])});
        }
        if let Some(values) = v.as_array() {
            return json!(
                values
                    .iter()
                    .map(|v| value(package, model, v))
                    .collect::<Vec<_>>()
            );
        }
        v.clone()
    }
    json!(model["elements"].as_array().unwrap().iter().map(|e|json!({"class":uri(package,&e["class"]),"values":e["values"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),value(package,model,v))).collect::<serde_json::Map<_,_>>()})).collect::<Vec<_>>())
}
#[test]
fn generated_oracle_models_and_rejections_cross_serialized_host_requests() {
    let observations: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/observations.json"
    ))
    .unwrap();
    let mut counts = (0, 0);
    for case in cases() {
        let input = inputs(&case);
        let package = compile(&input);
        assert_eq!(package["xtext_inference"], input);
        assert!(package.get("compilation").is_none());
        let package: Value = serde_json::from_str(&package.to_string()).unwrap();
        let archive = invoke(
            json!({"method":"package/export","inference":input,"metadata":{"package_id":"inference.host","release":"1","display_name":"Host inferred archive","description":""}}),
        );
        assert_eq!(archive["package"], package);
        let mut files = archive["files"].clone();
        files["manifest.json"] = json!(archive["manifest"].to_string());
        let loaded = invoke(json!({"method":"package/import","files":files}));
        assert_eq!(loaded["package"], package);
        let compiled = invoke(json!({"method":"compile_inferred","inference":input}));
        assert_eq!(loaded["keywords"], compiled["keywords"]);
        if case["id"] == "wildcard_basic" {
            assert_eq!(loaded["keywords"], json!(["m"]));
        }
        let package = loaded["package"].clone();
        for run in observations["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["case"] == case["id"] && r["entry"] == "resource")
        {
            let result = evaluate(&package, run["text"].as_str().unwrap());
            assert_eq!(
                result["diagnostics"].as_array().unwrap().is_empty(),
                run["accepted"] == true,
                "{}: {result}",
                case["id"]
            );
            if run["accepted"] == true {
                counts.0 += 1;
                assert_eq!(projection(&package, &result["model"]), run["model"]);
            } else {
                counts.1 += 1;
                assert_eq!(result["model"]["elements"], json!([]));
            }
        }
    }
    assert_eq!(counts, (7, 4));
}
#[test]
fn original_inputs_and_host_policy_control_replay_after_failures() {
    let input = inputs(&cases()[0]);
    let package = compile(&input);
    let expected = evaluate(&package, "alpha");
    let mut forged = package.clone();
    forged["grammar"] = Value::Null;
    forged["schema"]["uri"] = json!("urn:forged");
    forged["sources"] = json!({});
    assert_eq!(evaluate(&forged, "alpha"), expected);
    let mut invalids = vec![];
    let mut invalid = package.clone();
    invalid["xtext_inference"]["version"] = json!(99);
    invalids.push(invalid);
    let mut invalid = package.clone();
    invalid["xtext_inference"]["schema"] = json!({});
    invalids.push(invalid);
    let mut invalid = package.clone();
    invalid["xtext_inference"]["bundle"]["sources"][0]["text"] = json!("changed without identity");
    invalids.push(invalid);
    let mut invalid = package.clone();
    invalid["xtext_inference"]["grammar"]["sha256"] = json!("0".repeat(64));
    invalids.push(invalid);
    let mut invalid = package.clone();
    invalid["source_policy"]["xtext_bytes"] = json!(256001);
    invalids.push(invalid);
    let mut invalid = package.clone();
    invalid.as_object_mut().unwrap().remove("xtext_inference");
    invalids.push(invalid);
    for invalid in invalids {
        assert!(intersect_web::dispatch(&json!({"method":"evaluate","package":invalid,"sources":[{"uri":"memory:host-model.test","text":"alpha"}]}).to_string()).is_err());
        assert_eq!(evaluate(&package, "alpha"), expected);
    }
    let mut invalid = input.clone();
    invalid["version"] = json!(99);
    assert_eq!(
        invoke(json!({"method":"compile_inferred","inference":invalid}))["ok"],
        false
    );
    for (name, value) in [
        ("conversions", json!([])),
        ("bindings", json!({})),
        ("xtext_profile", json!("native.v56")),
    ] {
        let mut request = json!({"method":"compile_inferred","inference":input});
        request[name] = value;
        assert!(intersect_web::dispatch(&request.to_string()).is_err());
    }
    assert_eq!(evaluate(&package, "alpha"), expected);
}
#[test]
fn inferred_models_share_query_and_xmi_services_after_replay() {
    let package = compile(&inputs(&cases()[0]));
    let evaluated = evaluate(&package, "alpha");
    let model = &evaluated["model"];
    let result = invoke(
        json!({"method":"query/execute","package":package,"model":model,"query":{"schema_uri":package["schema"]["uri"],"schema_revision":package["schema"]["revision"],"from":model["elements"][0]["class"],"select":[{"name":"name","expression":{"op":"field","path":"name"}}],"limit":10}}),
    );
    assert_eq!(result["rows"], json!([["alpha"]]));
    let exported =
        invoke(json!({"method":"model/export","package":package,"format":"xmi","model":model}));
    let imported = invoke(
        json!({"method":"model/import","package":package,"format":"xmi","resources":exported["resources"]}),
    );
    assert_eq!(imported["diagnostics"], json!([]));
    assert_eq!(
        projection(&package, &imported["model"]),
        projection(&package, model)
    );
}

#[test]
fn inferred_archive_export_rejects_ambiguous_compiler_options_and_recovers() {
    let input = inputs(&cases()[0]);
    let request = json!({"method":"package/export","inference":input,"metadata":{"package_id":"inference.host","release":"1","display_name":"Inferred archive","description":""}});
    let good = invoke(request.clone());
    for option in [
        "compilation",
        "conversions",
        "applied_conversions",
        "constructors",
        "bindings",
        "xtext_profile",
    ] {
        let mut invalid = request.clone();
        invalid[option] = json!({});
        assert!(intersect_web::dispatch(&invalid.to_string()).is_err());
        assert_eq!(invoke(request.clone()), good);
    }
    let mut files = good["files"].clone();
    files["manifest.json"] = json!(good["manifest"].to_string());
    let mut invalid = files.clone();
    invalid["package.json"] = json!("self-inconsistent cached IR");
    assert!(
        intersect_web::dispatch(&json!({"method":"package/import","files":invalid}).to_string())
            .is_err()
    );
    let loaded = invoke(json!({"method":"package/import","files":files}));
    assert_eq!(
        evaluate(&loaded["package"], "alpha")["model"]["elements"][0]["values"]["name"],
        "alpha"
    );
}

#[test]
fn inferred_generated_previews_are_source_bound_outputs_without_a_replay_recipe() {
    let input = inputs(&cases()[0]);
    let result = invoke(json!({"method":"compile_inferred","inference":input}));
    assert_eq!(result["ok"], true);
    assert_eq!(result["package"]["xtext_inference"], input);
    assert!(result["package"].get("compilation").is_none());
    let report = &result["generated_definitions"];
    let sources = report["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 1);
    assert!(!report["origins"].as_array().unwrap().is_empty());
    for value in sources {
        let source: DefinitionSource = serde_json::from_value(value.clone()).unwrap();
        let reconstructed =
            DefinitionSource::new(source.id.uri.clone(), source.kind, source.text.clone());
        assert_eq!(source.id, reconstructed.id);
        assert_eq!(report["entry"], value["id"]);
        assert_eq!(result["package"]["sources"]["ecore"], source.text);
    }
}

#[test]
fn inferred_structure_previews_resolve_exact_canonical_context_without_cached_authority() {
    for case in cases() {
        let input = inputs(&case);
        let compiled = invoke(json!({"method":"compile_inferred","inference":input}));
        for source in compiled["generated_definitions"]["sources"]
            .as_array()
            .unwrap()
        {
            let preview = invoke(
                json!({"method":"inferred/structurePreview","inference":input,"generated_source":source["id"]}),
            );
            assert_eq!(preview["ok"], true);
            assert_eq!(preview["lowering"]["original"], *source);
            assert_ne!(preview["lowering"]["generated"]["id"], source["id"]);
            assert_eq!(preview["document"]["warnings"], json!([]));
            if case["id"] == "typed_basic" {
                assert_eq!(
                    preview["document"]["namespace"],
                    "urn:inference:fragments:typed_basic"
                );
                let names = preview["document"]["classes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| c["name"].as_str().unwrap())
                    .collect::<Vec<_>>();
                assert!(names.contains(&"Model") && names.contains(&"Fields"));
                let model = preview["document"]["classes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["name"] == "Model")
                    .unwrap();
                assert_eq!(model["super_types"], json!(["Fields"]));
            }
        }
        let invalid = json!({"method":"inferred/structurePreview","inference":input,"generated_source":{"uri":"memory:forged.rcore","sha256":"0".repeat(64)}});
        assert!(intersect_web::dispatch(&invalid.to_string()).is_err());
    }
}

fn parameter_case(name: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameters/cases.json"
    ))
    .unwrap();
    cases["lowercase_counterparts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == name)
        .unwrap()
        .clone()
}

#[test]
fn parameter_host_replay_uses_original_sources_and_ignores_cached_specializations() {
    let case = parameter_case("recursive_context");
    let input = inputs(&case);
    let package = compile(&input);
    assert_eq!(package["grammar"]["profile"], "intersect.xtext.native.v57");
    assert_eq!(package["xtext_inference"], input);
    assert!(package.get("compilation").is_none());
    let text = "yes alpha(no beta(yes gamma))";
    let expected = evaluate(&package, text);
    assert_eq!(expected["diagnostics"], json!([]));
    assert_eq!(expected["model"]["elements"].as_array().unwrap().len(), 4);
    let mut cached = package.clone();
    cached["grammar"] = Value::Null;
    cached["schema"]["uri"] = json!("urn:forged-specialization");
    cached["sources"] = json!({"generated:xtext:intersect:parameter-program:forged":"corrupt"});
    assert_eq!(evaluate(&cached, text), expected);
    let mut tampered = package.clone();
    tampered["xtext_inference"]["bundle"]["sources"][0]["text"] =
        json!("changed without a new source identity");
    assert!(intersect_web::dispatch(&json!({"method":"evaluate","package":tampered,"sources":[{"uri":"memory:host-model.test","text":text}]}).to_string()).is_err());
    assert_eq!(evaluate(&package, text), expected);
}

#[test]
fn parameter_host_archives_bind_actual_runtime_and_reject_forged_profiles() {
    let input = inputs(&parameter_case("true_call"));
    let exported = invoke(
        json!({"method":"package/export","inference":input,"metadata":{"package_id":"parameter.host","release":"1","display_name":"Host parameters","description":""}}),
    );
    assert_eq!(
        exported["manifest"]["runtime"]["profile"],
        "intersect.xtext.native.v57"
    );
    let mut files = exported["files"].clone();
    files["manifest.json"] = json!(exported["manifest"].to_string());
    assert_eq!(
        serde_json::from_str::<Value>(files["definitions/inference.json"].as_str().unwrap())
            .unwrap(),
        input
    );
    let loaded = invoke(json!({"method":"package/import","files":files}));
    assert_eq!(loaded["package"], exported["package"]);
    let expected = evaluate(&loaded["package"], "alpha");
    assert_eq!(expected["diagnostics"], json!([]));
    let mut forged = files.clone();
    let mut manifest = exported["manifest"].clone();
    manifest["runtime"]["profile"] = json!("intersect.xtext.native.v56");
    forged["manifest.json"] = json!(manifest.to_string());
    let error =
        intersect_web::dispatch(&json!({"method":"package/import","files":forged}).to_string())
            .unwrap_err();
    assert!(error.contains("runtime requirements differ"), "{error}");
    assert_eq!(
        invoke(json!({"method":"package/import","files":files}))["package"],
        loaded["package"]
    );
    assert_eq!(evaluate(&loaded["package"], "alpha"), expected);
}
