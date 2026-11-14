use serde_json::{Value, json};
fn rows() -> Vec<Value> {
    let plan: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-host-contexts/plan.json"
    ))
    .unwrap();
    let contexts: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-host-contexts/contexts.json"
    ))
    .unwrap();
    plan["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let mut row = row.clone();
            row["inference_context"] = contexts
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == row["id"])
                .unwrap()["inference_context"]
                .clone();
            row
        })
        .collect()
}
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn request(row: &Value) -> Value {
    json!({"method":"compile_inferred_with_context","inference":row["inference"],"inference_context":row["inference_context"]})
}
fn compile(row: &Value) -> Value {
    let result = call(request(row)).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    result["package"].clone()
}
fn evaluate(package: &Value, entry: Option<&str>) -> Value {
    call(json!({"method":"evaluate","package":package,"entry_rule":entry,"sources":[{"uri":"memory:host-context.model","text":"paint"}]})).unwrap()
}
fn verify(package: &Value, evaluation: &Value) -> Value {
    assert_eq!(evaluation["diagnostics"], json!([]), "{evaluation}");
    let elements = evaluation["model"]["elements"].as_array().unwrap();
    let paint = elements
        .iter()
        .find(|e| {
            package["schema"]["declarations"][e["class"].as_str().unwrap()]["name"] == "Paint"
        })
        .unwrap();
    assert_eq!(
        package["schema"]["declarations"][paint["class"].as_str().unwrap()]["namespace"],
        "urn:inferred:paint"
    );
    let contract = &package["schema"]["conversion_contracts"][0];
    let expected = intersect_model::definitions::conversion::ConvertedValue::Converted {
        datatype: serde_json::from_value(contract["datatype"].clone()).unwrap(),
        provider: serde_json::from_value(contract["provider"].clone()).unwrap(),
        value: json!({"red":18,"green":58,"blue":188}),
    };
    assert_eq!(paint["values"], json!({"color":expected}));
    if elements.len() == 2 {
        let model = elements.iter().find(|e| e["id"] != paint["id"]).unwrap();
        assert_eq!(model["values"], json!({"child":paint["id"]}));
    }
    paint.clone()
}
#[test]
fn installed_catalog_executes_actual_defaults_through_replay_entries_queries_and_scripts() {
    for row in rows() {
        let input: intersect_text::inference::InferenceRequest =
            serde_json::from_value(row["inference"].clone()).unwrap();
        let identity = intersect_model::definitions::resources::ResourceSet::new(&input.bundle)
            .unwrap()
            .resolve(&input.bundle.sources[0].id, "#//RGB")
            .unwrap();
        assert_eq!(
            json!(identity),
            row["inference_context"]["conversions"][0]["datatype"]
        );
        let package = compile(&row);
        assert_eq!(package["xtext_inference"], row["inference"]);
        let evaluated = evaluate(&package, None);
        let paint = verify(&package, &evaluated);
        assert_eq!(evaluated["model"]["elements"].as_array().unwrap().len(), 2);
        let named = evaluate(&package, Some("Paint"));
        verify(&package, &named);
        assert_eq!(named["model"]["elements"].as_array().unwrap().len(), 1);
        let catalog = call(json!({"method":"text/entries","package":package})).unwrap();
        assert!(catalog["entries"].get("Paint").is_some());
        let root = evaluated["model"]["elements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] != paint["id"])
            .unwrap();
        let query = json!({"schema_uri":package["schema"]["uri"],"schema_revision":package["schema"]["revision"],"from":root["class"],"select":[{"name":"child","expression":{"op":"field","path":"child"}}],"limit":10});
        assert_eq!(call(json!({"method":"query/execute","package":package,"model":evaluated["model"],"query":query})).unwrap()["rows"],json!([[paint["id"]]]));
        assert_eq!(
            call(json!({"method":"script/check","package":package,"model":evaluated["model"]}))
                .unwrap(),
            evaluated
        );
        // The actual shared codec/query limitations remain explicit obligations.
        let unsupported = call(json!({"method":"model/export","package":package,"model":evaluated["model"],"format":"xmi"})).unwrap_err();
        assert_eq!(
            unsupported,
            "Explicit conversion registry required for custom datatype"
        );
        let mut nominal = query.clone();
        nominal["from"] = paint["class"].clone();
        nominal["select"][0]["expression"]["path"] = json!("color");
        assert!(call(json!({"method":"query/execute","package":package,"model":evaluated["model"],"query":nominal})).unwrap_err().contains("Unsupported query datatype"));
        let mut forged = package.clone();
        forged["schema"]["classes"] = Value::Null;
        forged["grammar"] = json!({"forged":true});
        forged["sources"] = json!({});
        assert_eq!(evaluate(&forged, None), evaluated);
        for field in [
            "conversion_contracts",
            "applied_conversion_contracts",
            "constructor_contracts",
        ] {
            assert!(package["schema"][field].is_array() || package["schema"][field].is_null());
        }
    }
}
#[test]
fn explicit_context_archives_rebuild_original_inputs_and_check_all_cached_ir() {
    for row in rows() {
        let package = compile(&row);
        let mut export = request(&row);
        export["method"] = json!("package/export");
        export["metadata"] = json!({"package_id":"test.inferred-context","release":"1","display_name":"Host context","description":""});
        let archive = call(export).unwrap();
        assert_eq!(archive["package"], package);
        let mut files = archive["files"].clone();
        files["manifest.json"] = json!(archive["manifest"].to_string());
        assert!(call(json!({"method":"package/import","files":files})).is_err());
        let import = json!({"method":"package/import_with_context","files":files,"inference_context":row["inference_context"]});
        let imported = call(import.clone()).unwrap();
        assert_eq!(imported["package"], package);
        verify(&package, &evaluate(&imported["package"], None));
        let mut bad = import.clone();
        bad["files"]["package.json"] = json!("forged IR");
        assert!(call(bad).is_err());
        let mut forged = import.clone();
        let mut cached: Value =
            serde_json::from_str(forged["files"]["package.json"].as_str().unwrap()).unwrap();
        cached["grammar"]["entry"] = json!("FORGED");
        let text = cached.to_string();
        let mut manifest: Value =
            serde_json::from_str(forged["files"]["manifest.json"].as_str().unwrap()).unwrap();
        manifest["files"]["package.json"] = json!({"sha256":intersect_model::definitions::bundle::SourceId::for_text("irrelevant",&text).sha256,"bytes":text.len()});
        forged["files"]["package.json"] = json!(text);
        forged["files"]["manifest.json"] = json!(manifest.to_string());
        assert!(call(forged).is_err());
        let mut bad = import.clone();
        bad["inference_context"]["conversions"] = json!([]);
        assert!(call(bad).is_err());
        let mut bad = import.clone();
        bad["inference_context"]["conversions"][0]["provider"]["version"] = json!("stale");
        assert!(call(bad).is_err());
        assert_eq!(call(import).unwrap()["package"], package);
    }
}
#[test]
fn legacy_boundaries_and_bad_contexts_reject_without_accepting_cached_authority() {
    for row in rows() {
        let good = request(&row);
        let package = compile(&row);
        let expected = evaluate(&package, None);
        let mut legacy = good.clone();
        legacy["method"] = json!("compile_inferred");
        assert!(call(legacy).is_err());
        let mut invalid = good.clone();
        invalid.as_object_mut().unwrap().remove("inference_context");
        assert!(call(invalid).is_err());
        for context in [
            Value::Null,
            json!({"version":2}),
            json!({"version":1,"extra":true}),
            json!({"version":1,"computed_settings":[]}),
            json!({"version":1,"conversions":{}}),
        ] {
            let mut bad = good.clone();
            bad["inference_context"] = context;
            assert!(call(bad).is_err());
        }
        let mut missing = good.clone();
        missing["inference_context"]["conversions"] = json!([]);
        assert_eq!(call(missing).unwrap()["ok"], false);
        for field in ["name", "version"] {
            let mut bad = good.clone();
            bad["inference_context"]["conversions"][0]["provider"][field] = json!("unknown");
            assert!(call(bad).is_err());
        }
        let mut unused = good.clone();
        let mut contract = unused["inference_context"]["conversions"][0].clone();
        contract["datatype"] = json!(
            intersect_model::definitions::compiler::standard_datatype("EJavaObject")
                .unwrap()
                .declaration()
        );
        unused["inference_context"]["conversions"]
            .as_array_mut()
            .unwrap()
            .push(contract);
        assert!(
            call(unused)
                .unwrap_err()
                .contains("Unused conversion provider selection")
        );
        let mut bad = good.clone();
        bad["inference_context"]["conversions"][0]["datatype"]["source"]["sha256"] =
            json!("0".repeat(64));
        assert!(call(bad).is_err());
        let mut duplicate = good.clone();
        let contract = duplicate["inference_context"]["conversions"][0].clone();
        duplicate["inference_context"]["conversions"]
            .as_array_mut()
            .unwrap()
            .push(contract);
        assert!(call(duplicate).is_err());
        let mut bad = package.clone();
        bad["source_policy"]["xtext_bytes"] = json!(256001);
        assert!(call(json!({"method":"evaluate","package":bad,"sources":[]})).is_err());
        let mut bad = package.clone();
        bad["xtext_inference"]["bundle"]["sources"][0]["text"] = json!("forged");
        assert!(call(json!({"method":"text/entries","package":bad})).is_err());
        let mut bad = package.clone();
        bad["compilation"] = json!({});
        assert!(call(json!({"method":"text/entries","package":bad})).is_err());
        assert_eq!(evaluate(&package, None), expected);
    }
}

#[test]
fn inferred_catalog_transactions_publish_valid_values_and_reject_invalid_payloads() {
    for row in rows() {
        let package = compile(&row);
        let before = evaluate(&package, None);
        let paint = verify(&package, &before);
        let context = json!({"id":"inferred-paint","language_revision":"1","base_generation":0});
        call(json!({"method":"script/begin","package":package,"model":before["model"],"revision":"1","context":context})).unwrap();
        let mut color = paint["values"]["color"].clone();
        color["value"]["red"] = json!(255);
        call(json!({"method":"script/set","revision":"1","id":paint["id"],"feature":"color","value":color})).unwrap();
        let prepared = call(json!({"method":"script/commit","revision":"1"})).unwrap();
        let published=call(json!({"method":"script/publish","package":package,"before":before["model"],"model":prepared["model"],"current":before["model"],"context":context,"revision":"1","generation":0})).unwrap();
        let changed = published["model"]["elements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == paint["id"])
            .unwrap();
        assert_eq!(changed["values"]["color"], color);
        call(json!({"method":"script/begin","package":package,"model":before["model"],"revision":"1","context":context})).unwrap();
        color["value"]["red"] = json!(256);
        assert!(call(json!({"method":"script/set","revision":"1","id":paint["id"],"feature":"color","value":color})).is_err());
        assert!(call(json!({"method":"script/commit","revision":"1"})).is_err());
        assert_eq!(evaluate(&package, None), before);
    }
}
