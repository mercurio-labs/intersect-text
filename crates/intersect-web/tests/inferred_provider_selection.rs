//! Authored native provider selection; independent frozen source inputs retained.
use intersect_model::definitions::bundle::SourceId;
use intersect_text::grammar::SourceLimits;
use intersect_text::inference::{
    InferenceRequest,
    providers::{ProviderSelection, candidates, compile_selected},
};
use serde_json::{Value, json};
fn plan() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-provider-selection/plan.json"
    ))
    .unwrap()
}
fn before() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-provider-selection/before.json"
    ))
    .unwrap()
}
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn request(row: &Value) -> Value {
    json!({"method":"inferred/selectProviders","inference":row["inference"],"provider_selection":row["provider_selection"]})
}
fn select(row: &Value) -> Value {
    let result = call(request(row)).unwrap();
    assert_eq!(result["ok"], true, "{result}");
    result
}
fn evaluation(result: &Value, row: &Value) -> Value {
    call(json!({"method":"evaluate","package":result["package"],"sources":[{"uri":"memory:provider-selection.model","text":row["model_text"]}]})).unwrap()
}
fn verify_values(result: &Value, e: &Value, row: &Value) {
    assert_eq!(e["diagnostics"], json!([]), "{e}");
    assert_eq!(e["model"]["elements"].as_array().unwrap().len(), 2);
    let p = &result["package"];
    let item = e["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| p["schema"]["declarations"][e["class"].as_str().unwrap()]["name"] == "Item")
        .unwrap();
    let value = if row["id"].as_str().unwrap().starts_with("collection") {
        json!(["alpha", "beta"])
    } else {
        json!("alpha")
    };
    assert_eq!(item["values"], json!({"backing":value}));
    assert_eq!(
        e["derived"][item["id"].as_str().unwrap()]["computed"],
        value
    );
}
#[test]
fn authored_selectors_create_exact_existing_contracts_and_execute_shared_native_values() {
    let baseline = before();
    for row in plan()["cases"].as_array().unwrap() {
        let input: InferenceRequest = serde_json::from_value(row["inference"].clone()).unwrap();
        let selection: ProviderSelection =
            serde_json::from_value(row["provider_selection"].clone()).unwrap();
        let report = candidates(&input, SourceLimits::default(), None).unwrap();
        assert_eq!(report.boundary, "authored_direct_nongeneric_rcore");
        assert!(
            report
                .candidates
                .iter()
                .any(|c| c.source == selection.computed_settings[0].source
                    && c.class == "Item"
                    && c.feature == "computed")
        );
        let native = compile_selected(&input, &selection, SourceLimits::default(), None).unwrap();
        assert_eq!(
            json!({"version":2,"behavior":native.selection().behavior,"computed_settings":native.computed_settings()}),
            row["expected_context"]
        );
        let result = select(row);
        assert_eq!(result["inference_context"], row["expected_context"]);
        let mut with_catalog = request(row);
        with_catalog["inference_context"] = json!({"version":1,"conversions":[]});
        assert_eq!(
            call(with_catalog).unwrap()["inference_context"],
            row["expected_context"]
        );
        assert_eq!(result["provider_selection"], row["provider_selection"]);
        assert_eq!(result["package"], json!(native.language().package()));
        assert_eq!(
            result["package"]["xtext_inference"]["bundle"],
            row["inference"]["bundle"]
        );
        let e = evaluation(&result, row);
        verify_values(&result, &e, row);
        let old = baseline["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == row["id"])
            .unwrap();
        assert_eq!(e, old["observations"][5]["response"]);
        let package = native.language().package();
        let contracts = native.computed_settings();
        let provider =
            intersect_model::setting::RcoreProjection::new(package, &input.bundle, &contracts[0])
                .unwrap();
        let settings =
            intersect_text::setting::TextSettings::new(package, &input.bundle, &[&provider])
                .unwrap();
        let executed = intersect_text::evaluate_with_context(
            package,
            &[intersect_text::Source {
                uri: "memory:provider-selection.model".into(),
                text: row["model_text"].as_str().unwrap().into(),
            }],
            intersect_text::EvaluationLimits::workbench(),
            None,
            Some(&settings),
            &intersect_model::query::PackageDerived(package),
        );
        assert_eq!(json!(executed), e);
    }
}
#[test]
fn contextual_generated_preview_rebuilds_source_and_preserves_legacy_failures() {
    let baseline = before();
    for row in plan()["cases"].as_array().unwrap() {
        let result = select(row);
        let source = &result["generated_definitions"]["entry"];
        let request = json!({"method":"inferred/structurePreviewWithContext","inference":row["inference"],"inference_context":result["inference_context"],"generated_source":source});
        let preview = call(request.clone()).unwrap();
        assert_eq!(preview["ok"], true);
        assert_eq!(preview["lowering"]["original"]["id"], *source);
        let model = preview["document"]["classes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "Model")
            .unwrap();
        let child = model["features"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == "child")
            .unwrap();
        assert_eq!(child["containment"], true);
        assert!(child["type"].as_str().unwrap().contains("Item"));
        let mut forged = request.clone();
        forged["package"] = json!({"schema":null,"grammar":null});
        forged["generated_definitions"] = json!({"sources":[]});
        assert_eq!(call(forged).unwrap(), preview);
        let mut stale = request.clone();
        stale["generated_source"]["sha256"] = json!("0".repeat(64));
        assert!(call(stale).unwrap_err().contains("exact freshly generated"));
        let mut bad = request.clone();
        bad["inference_context"]["computed_settings"][0]["schema_revision"] = json!("stale");
        assert!(call(bad).is_err());
        let old = baseline["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == row["id"])
            .unwrap();
        assert_eq!(
            call(old["observations"][3]["request"].clone()).unwrap_err(),
            old["observations"][3]["response"]["error"]
                .as_str()
                .unwrap()
        );
        assert_eq!(
            call(old["observations"][4]["request"].clone()).unwrap(),
            old["observations"][4]["response"]
        );
    }
}
#[test]
fn selectors_reject_stale_missing_duplicates_origins_providers_operations_and_unused_members() {
    for row in plan()["cases"].as_array().unwrap() {
        let mut variants = vec![];
        for field in ["namespace", "class", "feature"] {
            let mut bad = request(row);
            bad["provider_selection"]["computed_settings"][0][field] = json!("missing");
            variants.push(bad);
        }
        let mut bad = request(row);
        bad["provider_selection"]["computed_settings"][0]["source"]["sha256"] =
            json!("0".repeat(64));
        variants.push(bad);
        let mut bad = request(row);
        bad["provider_selection"]["behavior"]["sha256"] = json!("0".repeat(64));
        variants.push(bad);
        let mut bad = request(row);
        bad["provider_selection"]["computed_settings"][0]["provider"]["version"] = json!("2");
        variants.push(bad);
        let mut bad = request(row);
        bad["provider_selection"]["computed_settings"][0]["operations"] = json!([]);
        variants.push(bad);
        let mut bad = request(row);
        let duplicate = bad["provider_selection"]["computed_settings"][0].clone();
        bad["provider_selection"]["computed_settings"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        variants.push(bad);
        let mut bad = request(row);
        bad["provider_selection"]["computed_settings"] = json!([]);
        variants.push(bad);
        let result = select(row);
        let baseline = evaluation(&result, row);
        let mut missing_required = request(row);
        missing_required["provider_selection"]["computed_settings"][0]["operations"] =
            if row["id"].as_str().unwrap().starts_with("collection") {
                json!(["set", "unset"])
            } else {
                json!(["unset"])
            };
        let omitted = call(missing_required.clone()).unwrap();
        assert_eq!(omitted["ok"], false, "{omitted}");
        assert!(
            omitted["diagnostics"]
                .to_string()
                .contains("Cannot assign derived feature"),
            "{omitted}"
        );
        let input: InferenceRequest = serde_json::from_value(row["inference"].clone()).unwrap();
        let selection: ProviderSelection =
            serde_json::from_value(missing_required["provider_selection"].clone()).unwrap();
        assert!(compile_selected(&input, &selection, SourceLimits::default(), None).is_err());
        assert_eq!(evaluation(&result, row), baseline);
        assert_eq!(select(row), result);
        let mut bad = request(row);
        bad["provider_selection"]["computed_settings"][0]["source"] =
            result["generated_definitions"]["entry"].clone();
        variants.push(bad);
        let mut bad = request(row);
        bad["provider_selection"]["computed_settings"][0]["member"] = json!({});
        variants.push(bad);
        let mut bad = request(row);
        bad["inference_context"] = row["expected_context"].clone();
        variants.push(bad);
        for bad in variants {
            let response = call(bad.clone());
            assert!(
                !response.as_ref().is_ok_and(|value| value["ok"] == true),
                "{bad}: {response:?}"
            );
        }
        let input: InferenceRequest = serde_json::from_value(row["inference"].clone()).unwrap();
        let mut selection: ProviderSelection =
            serde_json::from_value(row["provider_selection"].clone()).unwrap();
        selection.computed_settings = vec![selection.computed_settings[0].clone(); 17];
        assert!(compile_selected(&input, &selection, SourceLimits::default(), None).is_err());
    }
    for row in plan()["negative_cases"].as_array().unwrap() {
        let input: InferenceRequest = serde_json::from_value(row["inference"].clone()).unwrap();
        let selection: ProviderSelection =
            serde_json::from_value(row["provider_selection"].clone()).unwrap();
        assert!(compile_selected(&input, &selection, SourceLimits::default(), None).is_err());
        let expected = row["expected_message"].as_str().unwrap();
        let result = match call(request(row)) {
            Ok(result) => result,
            Err(error) if row["id"] == "ambiguous" => {
                assert_eq!(
                    error,
                    "InvalidDefinition: Duplicate classifier/package name"
                );
                continue;
            }
            Err(error) => panic!("{row}: {error}"),
        };
        assert_eq!(result["ok"], false, "{row}: {result}");
        if !expected.is_empty() {
            assert!(
                result["diagnostics"]
                    .to_string()
                    .to_ascii_lowercase()
                    .contains(&expected.to_ascii_lowercase()),
                "{result}"
            );
        }
    }
    let plan = plan();
    let inherited = plan["negative_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "inherited")
        .unwrap();
    let report =
        call(json!({"method":"inferred/providerCandidates","inference":inherited["inference"]}))
            .unwrap();
    assert_eq!(report["ok"], true);
    assert!(report["unsupported"].as_array().unwrap().iter().any(
        |c| c["class"] == "Child" && c["reason"].as_str().unwrap().contains("direct authored")
    ));
}
#[test]
fn changed_authored_sources_require_fresh_native_resolution_instead_of_contract_rehash() {
    for row in plan()["cases"].as_array().unwrap() {
        let mut input: InferenceRequest = serde_json::from_value(row["inference"].clone()).unwrap();
        let mut selection: ProviderSelection =
            serde_json::from_value(row["provider_selection"].clone()).unwrap();
        let old = selection.computed_settings[0].source.clone();
        let source = input
            .bundle
            .sources
            .iter_mut()
            .find(|s| s.id == old)
            .unwrap();
        source.text.push_str("// current authored source café\n");
        source.id = SourceId::for_text(&source.id.uri, &source.text);
        let fresh = source.id.clone();
        for source in &mut input.bundle.sources {
            for dependency in &mut source.dependencies {
                if *dependency == old {
                    *dependency = fresh.clone();
                }
            }
        }
        for binding in &mut input.bundle.namespaces {
            if binding.source == old {
                binding.source = fresh.clone();
            }
        }
        assert!(compile_selected(&input, &selection, SourceLimits::default(), None).is_err());
        selection.computed_settings[0].source = fresh;
        let selected = compile_selected(&input, &selection, SourceLimits::default(), None).unwrap();
        assert_ne!(
            json!(selected.computed_settings()),
            row["expected_context"]["computed_settings"]
        );
        let old_contracts = serde_json::from_value::<
            Vec<intersect_model::definitions::setting::ComputedSettingContract>,
        >(row["expected_context"]["computed_settings"].clone())
        .unwrap();
        assert!(
            input
                .compile_with_context(
                    SourceLimits::default(),
                    &old_contracts,
                    None,
                    Some(&selection.behavior)
                )
                .is_err()
        );
        let result=call(json!({"method":"inferred/selectProviders","inference":input,"provider_selection":selection})).unwrap();
        assert_eq!(result["ok"], true, "{result}");
        verify_values(&result, &evaluation(&result, row), row);
    }
}
#[test]
fn catalog_compile_stays_valid_while_custom_datatype_structure_projection_remains_explicit() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-host-contexts/plan.json"
    ))
    .unwrap();
    let contexts: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-host-contexts/contexts.json"
    ))
    .unwrap();
    for row in original["cases"].as_array().unwrap() {
        let context = &contexts.iter().find(|c| c["id"] == row["id"]).unwrap()["inference_context"];
        let compiled=call(json!({"method":"compile_inferred_with_context","inference":row["inference"],"inference_context":context})).unwrap();
        assert_eq!(compiled["ok"], true);
        let request = json!({"method":"inferred/structurePreviewWithContext","inference":row["inference"],"inference_context":context,"generated_source":compiled["generated_definitions"]["entry"]});
        let unsupported = call(request.clone()).unwrap_err();
        assert!(
            unsupported.contains("Cannot project inferred resource context"),
            "{unsupported}"
        );
        assert!(
            unsupported.contains("Unsupported datatype RGB")
                && unsupported.contains("an explicit provider is required"),
            "{unsupported}"
        );
        assert!(
            unsupported.contains(
                row["inference"]["bundle"]["sources"][0]["id"]["uri"]
                    .as_str()
                    .unwrap()
            ),
            "{unsupported}"
        );
        let mut old = request;
        old["method"] = json!("inferred/structurePreview");
        assert!(call(old).unwrap_err().contains("explicit context method"));
    }
}
