//! Source-backed provider host consumers; native extension evidence, not JVM awards.
use intersect_model::definitions::conversion::ProviderId;
use intersect_model::definitions::setting::ComputedSettingContract;
use intersect_model::definitions::setting::SettingOperation;
use intersect_text::grammar::SourceLimits;
use serde_json::{Value, json};
fn rows() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../fixtures/xtext/inferred-contexts/behavior-entry-explicit-cases.json"
    ))
    .unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}
fn context(row: &Value) -> Value {
    let input: intersect_text::inference::InferenceRequest =
        serde_json::from_value(row["inference"].clone()).unwrap();
    let mut definitions = intersect_text::inference::infer_metamodels(
        &input.bundle,
        &input.grammar,
        SourceLimits::default(),
    )
    .unwrap();
    intersect_text::grammar::prepare_ecore_reference_context(&mut definitions.schema).unwrap();
    let schema = &definitions.schema;
    let class = schema
        .declarations
        .iter()
        .find(|(_, d)| d.namespace == "urn:inferred:settings" && d.name == "Item")
        .unwrap()
        .0
        .clone();
    let contract = ComputedSettingContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class_identity: schema.identity(&class),
        member: schema
            .effective_feature(&class, "computed")
            .unwrap()
            .identity,
        class,
        feature: "computed".into(),
        provider: ProviderId {
            name: "rcore.projection".into(),
            version: "1".into(),
        },
        sources: input.bundle.sources.iter().map(|s| s.id.clone()).collect(),
        operations: vec![SettingOperation::Set],
    };
    let actual = json!({"version":2,"behavior":row["behavior"],"computed_settings":[contract]});
    let frozen: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-computed-host-contexts/contexts.json"
    ))
    .unwrap();
    let selected = &frozen
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == row["id"])
        .unwrap()["inference_context"];
    assert_eq!(&actual, selected);
    selected.clone()
}
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn request(row: &Value, context: &Value) -> Value {
    json!({"method":"compile_inferred_with_context","inference":row["inference"],"inference_context":context})
}
fn compile(row: &Value, context: &Value) -> Value {
    let result = call(request(row, context)).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    result["package"].clone()
}
fn evaluate(p: &Value, entry: Option<&str>, text: &str) -> Value {
    call(json!({"method":"evaluate","package":p,"entry_rule":entry,"sources":[{"uri":"memory:computed-host.model","text":text}]})).unwrap()
}
fn item<'a>(p: &Value, e: &'a Value, count: usize, value: &str) -> &'a Value {
    assert_eq!(e["diagnostics"], json!([]), "{e}");
    let objects = e["model"]["elements"].as_array().unwrap();
    assert_eq!(objects.len(), count);
    let item = objects
        .iter()
        .find(|e| p["schema"]["declarations"][e["class"].as_str().unwrap()]["name"] == "Item")
        .unwrap();
    assert_eq!(item["values"], json!({"backing":value}));
    assert_eq!(
        e["derived"][item["id"].as_str().unwrap()]["computed"],
        value
    );
    if count == 2 {
        let root = objects.iter().find(|e| e["id"] != item["id"]).unwrap();
        assert_eq!(root["values"], json!({"child":item["id"]}));
    }
    item
}
#[test]
fn source_authored_projection_executes_default_named_replay_query_and_script_consumers() {
    let mut responses = Vec::new();
    for row in rows() {
        let context = context(&row);
        let p = compile(&row, &context);
        assert_eq!(p["xtext_inference"]["bundle"], row["inference"]["bundle"]);
        assert_eq!(p["xtext_inference"]["behavior"], row["behavior"]);
        let text = row["model_text"].as_str().unwrap();
        let e = evaluate(&p, None, text);
        let object = item(&p, &e, 2, "alpha");
        item(
            &p,
            &evaluate(&p, Some("Item"), row["named_model_text"].as_str().unwrap()),
            1,
            "alpha",
        );
        let catalog = call(json!({"method":"text/entries","package":p})).unwrap();
        assert!(catalog["entries"]["Item"].is_object());
        let query = json!({"schema_uri":p["schema"]["uri"],"schema_revision":p["schema"]["revision"],"from":object["class"],"select":[{"name":"computed","expression":{"op":"field","path":"computed"}}],"limit":10});
        assert_eq!(
            call(json!({"method":"query/execute","package":p,"model":e["model"],"query":query}))
                .unwrap()["rows"],
            json!([["alpha"]])
        );
        assert_eq!(
            call(json!({"method":"script/check","package":p,"model":e["model"]})).unwrap(),
            e
        );
        let mut forged = p.clone();
        forged["semantics"]["derived"] = json!([]);
        forged["schema"]["classes"] = Value::Null;
        forged["sources"] = json!({});
        assert_eq!(evaluate(&forged, None, text), e);
        let tx = json!({"id":"computed-host","language_revision":"1","base_generation":0});
        call(json!({"method":"script/begin","package":p,"model":e["model"],"revision":"1","context":tx})).unwrap();
        call(json!({"method":"script/set","revision":"1","id":object["id"],"feature":"computed","value":"beta"})).unwrap();
        let next = call(json!({"method":"script/commit","revision":"1"})).unwrap();
        item(&p, &next, 2, "beta");
        let published=call(json!({"method":"script/publish","package":p,"before":e["model"],"current":e["model"],"model":next["model"],"context":tx,"revision":"1","generation":0})).unwrap();
        item(&p, &published, 2, "beta");
        responses.push(json!([next, published]));
        call(json!({"method":"script/begin","package":p,"model":e["model"],"revision":"1","context":tx})).unwrap();
        assert!(call(json!({"method":"script/set","revision":"1","id":object["id"],"feature":"computed","value":false})).is_err());
        assert!(call(json!({"method":"script/commit","revision":"1"})).is_err());
        assert_eq!(evaluate(&p, None, text), e);
    }
    super::assert_store_abi(
        &Value::Array(responses),
        "688c2cc4d3abaf9c06951957e4918642043ebfa28876d37b6575f11b29ecb459",
    );
}
#[test]
fn archives_require_exact_behavior_and_installed_projection_and_verify_ir() {
    for row in rows() {
        let context = context(&row);
        let p = compile(&row, &context);
        let metadata = json!({"package_id":"test.computed-host","release":"1","display_name":"Computed host","description":""});
        let archive=call(json!({"method":"package/export","inference":p["xtext_inference"],"inference_context":context,"metadata":metadata})).unwrap();
        assert_eq!(archive["package"], p);
        let mut files = archive["files"].clone();
        files["manifest.json"] = json!(archive["manifest"].to_string());
        assert!(call(json!({"method":"package/import","files":files})).is_err());
        assert_eq!(call(json!({"method":"package/import_with_context","files":files,"inference_context":context})).unwrap()["package"],p);
        let mut invalid = context.clone();
        invalid["behavior"]["sha256"] = json!("0".repeat(64));
        assert!(call(json!({"method":"package/import_with_context","files":files,"inference_context":invalid})).is_err());
        let mut recipe = p["xtext_inference"].clone();
        recipe["behavior"] = Value::Null;
        assert!(call(json!({"method":"package/export","inference":recipe,"inference_context":context,"metadata":metadata})).is_err());
        files["package.json"] = json!("{}");
        assert!(call(json!({"method":"package/import_with_context","files":files,"inference_context":context})).is_err());
    }
}
#[test]
fn stale_uninstalled_unused_and_conflicting_settings_reject_without_fallback() {
    for row in rows() {
        let context = context(&row);
        let p = compile(&row, &context);
        let before = evaluate(&p, None, row["model_text"].as_str().unwrap());
        for field in ["schema_revision", "class_identity", "class", "feature"] {
            let mut bad = context.clone();
            bad["computed_settings"][0][field] = json!("stale");
            let result = call(request(&row, &bad));
            assert!(
                !result.as_ref().is_ok_and(|r| r["ok"] == true),
                "{field}: {result:?}"
            );
        }
        let mut variants = vec![];
        let mut bad = context.clone();
        bad["version"] = json!(1);
        variants.push(bad);
        let mut bad = context.clone();
        bad["computed_settings"][0]["provider"]["name"] = json!("test.projection");
        variants.push(bad);
        let mut bad = context.clone();
        bad["computed_settings"][0]["provider"]["version"] = json!("2");
        variants.push(bad);
        let mut bad = context.clone();
        bad["computed_settings"][0]["sources"] = json!([]);
        variants.push(bad);
        let mut bad = context.clone();
        bad["computed_settings"][0]["sources"]
            .as_array_mut()
            .unwrap()
            .retain(|s| s != &row["behavior"]);
        variants.push(bad);
        let mut bad = context.clone();
        bad["computed_settings"][0]["operations"] = json!(["append"]);
        variants.push(bad);
        let mut bad = context.clone();
        let c = bad["computed_settings"][0].clone();
        bad["computed_settings"].as_array_mut().unwrap().push(c);
        variants.push(bad);
        let mut bad = context.clone();
        bad["behavior"] = Value::Null;
        variants.push(bad);
        let mut bad = context.clone();
        bad["behavior"] = row["inference"]["grammar"].clone();
        variants.push(bad);
        for bad in variants {
            let r = call(request(&row, &bad));
            assert!(!r.as_ref().is_ok_and(|r| r["ok"] == true), "{bad}: {r:?}");
        }
        let mut no_context = request(&row, &context);
        no_context["method"] = json!("compile_inferred");
        assert!(call(no_context).is_err());
        let mut retained = p.clone();
        retained["grammar"]["computed_settings"] = json!([]);
        assert!(call(json!({"method":"text/entries","package":retained})).is_err());
        assert_eq!(
            evaluate(&p, None, row["model_text"].as_str().unwrap()),
            before
        );
    }
}

#[test]
fn default_context_preserves_partial_resources_and_reports_execution_errors_as_diagnostics() {
    for row in rows() {
        let selected = context(&row);
        let p = compile(&row, &selected);
        let sources = json!([{"uri":"memory:computed-host.model","text":row["model_text"]},{"uri":"memory:bad.model","text":"!"}]);
        let result = call(json!({"method":"evaluate","package":p,"sources":sources})).unwrap();
        assert_eq!(result["diagnostics"].as_array().unwrap().len(), 1);
        assert_eq!(result["diagnostics"][0]["source"], "memory:bad.model");
        assert_eq!(result["model"]["elements"].as_array().unwrap().len(), 2);
        let package: intersect_text::package::Package = serde_json::from_value(p).unwrap();
        let bundle = &package.xtext_inference.as_ref().unwrap().bundle;
        let contract = &package.grammar.as_ref().unwrap().computed_settings[0];
        let provider =
            intersect_model::setting::RcoreProjection::new(&package, bundle, contract).unwrap();
        let settings =
            intersect_text::setting::TextSettings::new(&package, bundle, &[&provider]).unwrap();
        let sources: Vec<intersect_text::Source> = serde_json::from_value(sources).unwrap();
        let native = intersect_text::evaluate_with_context(
            &package,
            &sources,
            intersect_text::EvaluationLimits::workbench(),
            None,
            Some(&settings),
            &intersect_model::query::PackageDerived(&package),
        );
        assert_eq!(serde_json::to_value(native).unwrap(), result);
        let missing = intersect_text::evaluate_with_context(
            &package,
            &sources,
            intersect_text::EvaluationLimits::workbench(),
            None,
            None,
            &intersect_model::query::PackageDerived(&package),
        );
        assert!(!missing.diagnostics.is_empty());
        assert!(missing.model.elements.is_empty());
    }
}
