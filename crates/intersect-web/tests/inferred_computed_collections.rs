//! Source-backed collection provider consumers; native extension evidence only.
use intersect_model::definitions::conversion::ProviderId;
use intersect_model::definitions::setting::ComputedSettingContract;
use intersect_model::definitions::setting::SettingOperation;
use intersect_text::grammar::SourceLimits;
use serde_json::{Value, json};
fn rows() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../fixtures/xtext/inferred-computed-collection-contexts/plan.json"
    ))
    .unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}
fn derived_context(row: &Value) -> Value {
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
        .find(|(_, d)| d.namespace == "urn:inferred:collections" && d.name == "Item")
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
        operations: vec![
            SettingOperation::Set,
            SettingOperation::Append,
            SettingOperation::Unset,
        ],
    };
    json!({"version":2,"behavior":row["behavior"],"computed_settings":[contract]})
}
fn context(row: &Value) -> Value {
    let frozen: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-computed-collection-contexts/contexts.json"
    ))
    .unwrap();
    let selected = &frozen.iter().find(|c| c["id"] == row["id"]).unwrap()["inference_context"];
    assert_eq!(&derived_context(row), selected);
    selected.clone()
}
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn compile(row: &Value, selected: &Value) -> Value {
    let result = call(json!({"method":"compile_inferred_with_context","inference":row["inference"],"inference_context":selected})).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    result["package"].clone()
}
fn evaluate(package: &Value, row: &Value, entry: Option<&str>) -> Value {
    call(json!({"method":"evaluate","package":package,"entry_rule":entry,"sources":[{"uri":"memory:computed-collection.model","text":row["model_text"]}]})).unwrap()
}
fn item<'a>(
    package: &Value,
    evaluation: &'a Value,
    values: Value,
    count: usize,
    assigned: bool,
) -> &'a Value {
    assert_eq!(evaluation["diagnostics"], json!([]), "{evaluation}");
    let objects = evaluation["model"]["elements"].as_array().unwrap();
    assert_eq!(objects.len(), count);
    let item = objects
        .iter()
        .find(|e| package["schema"]["declarations"][e["class"].as_str().unwrap()]["name"] == "Item")
        .unwrap();
    assert_eq!(item["values"], json!({"backing":values}));
    assert_eq!(
        evaluation["derived"][item["id"].as_str().unwrap()]["computed"],
        values
    );
    assert_eq!(
        item["assignment_state"],
        json!({"version":1,"set_features":if assigned { vec!["backing"] } else { vec![] }})
    );
    if count == 2 {
        let root = objects.iter().find(|e| e["id"] != item["id"]).unwrap();
        assert_eq!(root["values"], json!({"child":item["id"]}));
    }
    item
}
fn begin(package: &Value, evaluation: &Value) {
    call(json!({"method":"script/begin","package":package,"model":evaluation["model"],"revision":"1"})).unwrap();
}
fn edit(method: &str, id: &Value, extra: Value) -> Result<Value, String> {
    let mut request = json!({"method":method,"revision":"1","id":id,"feature":"computed"});
    request
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    call(request)
}
fn commit() -> Result<Value, String> {
    call(json!({"method":"script/commit","revision":"1"}))
}
#[test]
fn original_collection_sources_execute_default_named_and_shared_native_provider_paths() {
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-computed-collection-contexts/before.json"
    ))
    .unwrap();
    for row in rows() {
        let selected = context(&row);
        let package = compile(&row, &selected);
        let evaluation = evaluate(&package, &row, None);
        assert_eq!(
            package["xtext_inference"]["bundle"],
            row["inference"]["bundle"]
        );
        assert_eq!(package["xtext_inference"]["behavior"], row["behavior"]);
        item(&package, &evaluation, json!(["alpha", "beta"]), 2, true);
        if row["id"] == "plain" {
            item(
                &package,
                &evaluate(&package, &row, Some("Item")),
                json!(["alpha", "beta"]),
                1,
                true,
            );
        }
        let old = before["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == row["id"])
            .unwrap();
        for observation in old["observations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["request"]["method"] == "evaluate")
        {
            assert_eq!(
                call(observation["request"].clone()).unwrap(),
                observation["response"]
            );
        }
        let typed: intersect_text::package::Package =
            serde_json::from_value(package.clone()).unwrap();
        let bundle = &typed.xtext_inference.as_ref().unwrap().bundle;
        let contract = &typed.grammar.as_ref().unwrap().computed_settings[0];
        let provider =
            intersect_model::setting::RcoreProjection::new(&typed, bundle, contract).unwrap();
        let settings =
            intersect_text::setting::TextSettings::new(&typed, bundle, &[&provider]).unwrap();
        let native = intersect_text::evaluate_with_context(
            &typed,
            &[intersect_text::Source {
                uri: "memory:computed-collection.model".into(),
                text: row["model_text"].as_str().unwrap().into(),
            }],
            intersect_text::EvaluationLimits::workbench(),
            None,
            Some(&settings),
            &intersect_model::query::PackageDerived(&typed),
        );
        assert_eq!(serde_json::to_value(native).unwrap(), evaluation);
        let mut forged = package.clone();
        forged["semantics"]["derived"] = json!([]);
        forged["schema"]["classes"] = Value::Null;
        assert_eq!(evaluate(&forged, &row, None), evaluation);
    }
    for old in before["shape_conflicts"].as_array().unwrap() {
        assert_eq!(call(old["request"].clone()).unwrap(), old["response"]);
    }
}
#[test]
fn script_collection_append_unset_and_explicit_empty_state_use_live_source_projection() {
    for row in rows() {
        let package = compile(&row, &context(&row));
        let evaluation = evaluate(&package, &row, None);
        let id = item(&package, &evaluation, json!(["alpha", "beta"]), 2, true)["id"].clone();
        begin(&package, &evaluation);
        assert_eq!(
            edit(
                "script/collection",
                &id,
                json!({"operation":"add","value":"gamma"})
            )
            .unwrap(),
            true
        );
        assert_eq!(
            edit("script/get", &id, json!({})).unwrap(),
            json!(["alpha", "beta", "gamma"])
        );
        assert_eq!(edit("script/is_set", &id, json!({})).unwrap(), true);
        let prepared = commit().unwrap();
        item(
            &package,
            &prepared,
            json!(["alpha", "beta", "gamma"]),
            2,
            true,
        );
        let published = call(json!({"method":"script/publish","package":package,"before":evaluation["model"],"current":evaluation["model"],"model":prepared["model"],"revision":"1","generation":0,"context":{"id":"script:1","language_revision":"1","base_generation":0}})).unwrap();
        item(
            &package,
            &published,
            json!(["alpha", "beta", "gamma"]),
            2,
            true,
        );
        begin(&package, &evaluation);
        edit("script/unset", &id, json!({})).unwrap();
        assert_eq!(edit("script/get", &id, json!({})).unwrap(), json!([]));
        assert_eq!(edit("script/is_set", &id, json!({})).unwrap(), false);
        let unset = commit().unwrap();
        item(&package, &unset, json!([]), 2, false);
        begin(&package, &unset);
        edit(
            "script/collection",
            &id,
            json!({"operation":"add","value":"delta"}),
        )
        .unwrap();
        item(&package, &commit().unwrap(), json!(["delta"]), 2, true);
        begin(&package, &evaluation);
        edit("script/set", &id, json!({"value":[]})).unwrap();
        assert_eq!(edit("script/is_set", &id, json!({})).unwrap(), true);
        item(&package, &commit().unwrap(), json!([]), 2, true);
    }
}
#[test]
fn undeclared_operations_and_invalid_collection_writes_abort_without_fallback() {
    for row in rows() {
        let selected = context(&row);
        let package = compile(&row, &selected);
        let evaluation = evaluate(&package, &row, None);
        let id = item(&package, &evaluation, json!(["alpha", "beta"]), 2, true)["id"].clone();
        for value in [json!(false), Value::Null, json!(["gamma"])] {
            begin(&package, &evaluation);
            assert!(
                edit(
                    "script/collection",
                    &id,
                    json!({"operation":"add","value":value})
                )
                .is_err()
            );
            assert!(commit().is_err());
            assert_eq!(evaluate(&package, &row, None), evaluation);
        }
        begin(&package, &evaluation);
        edit(
            "script/collection",
            &id,
            json!({"operation":"add","value":"alpha"}),
        )
        .unwrap();
        item(
            &package,
            &commit().unwrap(),
            json!(["alpha", "beta", "alpha"]),
            2,
            true,
        );
        begin(&package, &evaluation);
        edit(
            "script/collection",
            &id,
            json!({"operation":"add","value":"gamma"}),
        )
        .unwrap();
        assert!(
            edit(
                "script/collection",
                &id,
                json!({"operation":"add","value":"delta"})
            )
            .is_err()
        );
        assert!(commit().is_err());
        begin(&package, &evaluation);
        assert!(
            edit(
                "script/set",
                &id,
                json!({"value":["alpha","beta","gamma","delta"]})
            )
            .is_err()
        );
        assert!(commit().is_err());
        for operation in ["clear", "remove", "insert", "replace", "remove_at", "move"] {
            begin(&package, &evaluation);
            let error = edit(
                "script/collection",
                &id,
                json!({"operation":operation,"value":"alpha","index":0,"from":0,"to":1}),
            )
            .unwrap_err();
            assert!(error.contains("computed collection"), "{error}");
            assert!(commit().is_err());
        }
        let mut narrow = selected.clone();
        narrow["computed_settings"][0]["operations"] = json!(["append"]);
        let restricted = compile(&row, &narrow);
        for method in ["script/set", "script/unset"] {
            begin(&restricted, &evaluation);
            let error = edit(method, &id, json!({"value":["gamma"]})).unwrap_err();
            assert!(error.contains("operation is not declared"), "{error}");
            assert!(commit().is_err());
        }
        narrow["computed_settings"][0]["operations"] = json!(["set", "unset"]);
        assert_eq!(call(json!({"method":"compile_inferred_with_context","inference":row["inference"],"inference_context":narrow})).unwrap()["ok"], false);
        begin(&package, &evaluation);
        edit(
            "script/collection",
            &id,
            json!({"operation":"add","value":"delta"}),
        )
        .unwrap();
        item(
            &package,
            &commit().unwrap(),
            json!(["alpha", "beta", "delta"]),
            2,
            true,
        );
    }
}
#[test]
fn collection_archive_replay_recreates_provider_and_rechecks_contracts() {
    for row in rows() {
        let selected = context(&row);
        let package = compile(&row, &selected);
        let archive = call(json!({"method":"package/export","inference":package["xtext_inference"],"inference_context":selected,"metadata":{"package_id":"test.computed-collections","release":"1","display_name":"Computed collections","description":""}})).unwrap();
        assert_eq!(archive["package"], package);
        let mut files = archive["files"].clone();
        files["manifest.json"] = json!(archive["manifest"].to_string());
        assert!(call(json!({"method":"package/import","files":files})).is_err());
        let loaded = call(json!({"method":"package/import_with_context","files":files,"inference_context":selected})).unwrap()["package"].clone();
        assert_eq!(loaded, package);
        let evaluation = evaluate(&loaded, &row, None);
        let id = item(&loaded, &evaluation, json!(["alpha", "beta"]), 2, true)["id"].clone();
        begin(&loaded, &evaluation);
        edit(
            "script/collection",
            &id,
            json!({"operation":"add","value":"gamma"}),
        )
        .unwrap();
        item(
            &loaded,
            &commit().unwrap(),
            json!(["alpha", "beta", "gamma"]),
            2,
            true,
        );
        let mut invalid = selected.clone();
        invalid["computed_settings"][0]["sources"] = json!([]);
        assert!(call(json!({"method":"package/import_with_context","files":files,"inference_context":invalid})).is_err());
        assert_eq!(call(json!({"method":"package/import_with_context","files":files,"inference_context":selected})).unwrap()["package"], package);
    }
}
