//! Shared native/WASM consumer of the production JSON dispatcher and exact source fixtures.
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
fn call(request: &Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap_or_else(|error| panic!("{error}"))
}
fn node_text(text: &str, range: &Value) -> String {
    let units: Vec<_> = text.encode_utf16().collect();
    String::from_utf16(
        &units[range["start"].as_u64().unwrap() as usize..range["end"].as_u64().unwrap() as usize],
    )
    .unwrap()
}
fn node_checks(snapshot: &Value, expected: &Value, text: &str, package: &Value, bindings: &Value) {
    let nodes = snapshot["document"]["nodes"].as_array().unwrap();
    let wanted = expected["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), wanted.len(), "complete node count: {text}");
    for (index, (actual, wanted)) in nodes.iter().zip(wanted).enumerate() {
        assert_eq!(actual["kind"], wanted["kind"], "kind {index}: {text}");
        assert_eq!(actual["parent"], wanted["parent"], "parent {index}: {text}");
        let children: Vec<_> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node["parent"] == json!(index))
            .map(|(id, _)| id)
            .collect();
        assert_eq!(
            json!(children),
            wanted["children"],
            "ordered edges {index}: {text}"
        );
        assert_eq!(
            actual["range"],
            json!({"start":wanted["utf16_start"],"end":wanted["utf16_end"]})
        );
        assert_eq!(
            actual["total_range"],
            json!({"start":wanted["total_utf16_start"],"end":wanted["total_utf16_end"]})
        );
        assert_eq!(node_text(text, &actual["total_range"]), wanted["text"]);
        if wanted["kind"] == "leaf" {
            assert_eq!(actual["data"]["hidden"], wanted["hidden"]);
        }
        assert_eq!(
            actual["data"]["direct_semantic_association"],
            wanted["direct_semantic_association"]
        );
        let mut owner = Some(index);
        let mut effective = Value::Null;
        let mut visited = 0;
        while let Some(at) = owner {
            visited += 1;
            assert!(visited <= nodes.len(), "acyclic retained parent chain");
            if !nodes[at]["data"]["semantic_object"].is_null() {
                effective = nodes[at]["data"]["semantic_object"].clone();
                break;
            }
            owner = nodes[at]["parent"].as_u64().map(|id| id as usize);
        }
        assert_eq!(
            effective, wanted["semantic_object"],
            "effective semantic object {index}: {text}"
        );
        match actual["data"]["syntax_error"].as_u64() {
            Some(id) => {
                let issue = &snapshot["syntax_issues"][id as usize];
                assert_eq!(
                    issue["message"], wanted["syntax_error"]["message"],
                    "error {index}: {text}"
                );
                assert_eq!(issue["issue_code"], wanted["syntax_error"]["issue_code"]);
                assert_eq!(issue["issue_data"], wanted["syntax_error"]["issue_data"]);
            }
            None => assert!(
                wanted["syntax_error"].is_null(),
                "missing attachment {index}: {text}"
            ),
        }
        let Some(origin_index) = actual["data"]["origin"].as_u64() else {
            assert!(
                wanted["grammar"].is_null(),
                "unmapped grammar identity {index}: {text}"
            );
            continue;
        };
        let origin = &snapshot["origins"][origin_index as usize];
        let kind = match origin["kind"].as_str().unwrap() {
            "parser_rule" => "ParserRule",
            "rule_call" => "RuleCall",
            "terminal_rule" => "TerminalRule",
            "keyword" => "Keyword",
            other => panic!("Unexpected {other}"),
        };
        assert_eq!(kind, wanted["grammar"]["kind"]);
        let span = if origin["expression"].is_null() {
            &origin["declaration"]
        } else {
            &origin["expression"]
        };
        assert_eq!(span["source"], wanted["grammar"]["authored_owner_source"]);
        let matches: Vec<_> = bindings
            .as_array()
            .unwrap()
            .iter()
            .filter(|binding| binding["generated_raw_uri"] == wanted["grammar"]["raw_uri"])
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "exact generated-to-authored EObject correspondence"
        );
        let bound = matches[0];
        assert_eq!(span["source"], bound["authored_source"]);
        assert_eq!(kind, bound["kind"]);
        assert_eq!(bound["authored_proxy"], false);
        assert_eq!(bound["generated_proxy"], false);
        let sources = package["compilation"]["bundle"]["sources"]
            .as_array()
            .unwrap();
        let source = sources
            .iter()
            .find(|source| source["id"] == span["source"])
            .expect("same exact authored SourceId");
        let original = source["text"].as_str().unwrap();
        let start = span["start"].as_u64().unwrap() as usize;
        let end = span["end"].as_u64().unwrap() as usize;
        assert_eq!(
            original[..start].encode_utf16().count() as u64,
            bound["authored_node"]["utf16_start"].as_u64().unwrap()
        );
        assert_eq!(
            original[..end].encode_utf16().count() as u64,
            bound["authored_node"]["utf16_end"].as_u64().unwrap()
        );
        if let Some(target) = origin["target"].as_object() {
            let target = &Value::Object(target.clone());
            let expected = &bound["authored_target"];
            assert_eq!(target["source"], expected["source"]);
            assert_eq!(expected["proxy"], false);
            let source = sources
                .iter()
                .find(|source| source["id"] == target["source"])
                .unwrap();
            let original = source["text"].as_str().unwrap();
            assert_eq!(
                original[..target["start"].as_u64().unwrap() as usize]
                    .encode_utf16()
                    .count() as u64,
                expected["node"]["utf16_start"].as_u64().unwrap()
            );
            assert_eq!(
                original[..target["end"].as_u64().unwrap() as usize]
                    .encode_utf16()
                    .count() as u64,
                expected["node"]["utf16_end"].as_u64().unwrap()
            );
        } else {
            assert!(bound["authored_target"].is_null());
        }
    }
}
pub fn verify_typed_rgb_documents() {
    verify_committed_provider_invocations();
    let data: Value = serde_json::from_str(include_str!("fixtures/durable_rgb.json")).unwrap();
    for case in data["cases"].as_array().unwrap() {
        let built = call(&case["compile_request"]);
        assert_eq!(built["ok"], true, "{built}");
        let package = &built["package"];
        let datatype = &case["compile_request"]["conversions"][0]["datatype"];
        for upstream in case["upstream"].as_array().unwrap() {
            let text = upstream["text"].as_str().unwrap();
            let request = json!({"method":"document/analyze","package":package,"sources":[{"uri":"memory:typed-provider-control/model","text":text}]});
            let analysis = call(&request);
            assert!(analysis["snapshots"].is_array(), "{analysis}");
            let snapshot = &analysis["snapshots"][0];
            node_checks(
                snapshot,
                &upstream["resource_node_snapshot"],
                text,
                package,
                &case["source_loaded_bindings"],
            );
            // The same default IParser nodes correspond to the actual native pre-link snapshot.
            // Link/validation diagnostics remain separately present in the whole analysis.
            node_checks(
                snapshot,
                &upstream["parser_node_snapshot"],
                text,
                package,
                &case["source_loaded_bindings"],
            );
            let object_count = upstream["resource_node_snapshot"]["model_objects"]
                .as_u64()
                .unwrap() as usize;
            let native = if analysis["valid_model"].is_null() {
                &analysis["recovered"]["objects"]
            } else {
                &analysis["valid_model"]["model"]["elements"]
            };
            assert_eq!(
                native.as_array().unwrap().len(),
                object_count,
                "complete model {text}"
            );
            let parsed = &snapshot["parse_stage"]["objects"];
            assert_eq!(
                parsed, native,
                "complete actual pre-link objects remain separate but equal for this source cohort"
            );
            assert!(
                snapshot["parse_stage"]["references"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                analysis["diagnostics"].as_array().unwrap().len(),
                upstream["resource"]["errors"].as_array().unwrap().len()
            );
            for (actual, expected) in analysis["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .zip(upstream["resource"]["errors"].as_array().unwrap())
            {
                assert_eq!(actual["message"], expected["message"]);
            }
            for (actual, expected) in snapshot["syntax_issues"]
                .as_array()
                .unwrap()
                .iter()
                .zip(upstream["parser"]["errors"].as_array().unwrap())
            {
                assert_eq!(actual["message"], expected["message"]);
                assert_eq!(
                    actual["range"],
                    json!({"start":expected["utf16_start"],"end":expected["utf16_end"]})
                );
            }

            assert_eq!(parsed.as_array().unwrap().len(), object_count);
            for object in native.as_array().unwrap() {
                assert_eq!(object["class"], "Paint");
                let declaration = &package["schema"]["declarations"]["Paint"];
                let source = &declaration["source"];
                let identity = format!(
                    "{}@{}#{}",
                    source["uri"].as_str().unwrap(),
                    source["sha256"].as_str().unwrap(),
                    declaration["path"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|part| part.as_u64().unwrap().to_string())
                        .collect::<Vec<_>>()
                        .join("/")
                );
                assert_eq!(object["type_identity"], identity);
                assert_eq!(object["values"].as_object().unwrap().len(), 1);
                let value = &object["values"]["color"];
                let wanted = &upstream["resource"]["model"][0]["values"]["color"];
                if wanted.is_null() {
                    assert!(value.is_null());
                } else {
                    assert_eq!(value["kind"], "converted");
                    assert_eq!(&value["datatype"], datatype);
                    assert_eq!(
                        value["provider"],
                        json!({"name":"example.rgb","version":"1"})
                    );
                    assert_eq!(&value["value"], wanted);
                }
            }
            let issues = upstream["resource"]["errors"].as_array().unwrap();
            assert_eq!(
                analysis["valid_model"].is_null(),
                !issues.is_empty(),
                "no recovered model may be published: {text}"
            );
            for change in ["missing", "stale", "wrong"] {
                let mut forged = request.clone();
                let contract = &mut forged["package"]["schema"]["conversion_contracts"];
                match change {
                    "missing" => *contract = json!([]),
                    "stale" => contract[0]["provider"]["version"] = json!("2"),
                    _ => contract[0]["provider"]["name"] = json!("other.rgb"),
                }
                assert!(
                    intersect_web::dispatch(&forged.to_string()).is_err(),
                    "{change}"
                );
            }
            let mut forged = request.clone();
            for terminal in forged["package"]["grammar"]["terminals"]
                .as_object_mut()
                .unwrap()
                .values_mut()
            {
                if !terminal["nominal_return"].is_null() {
                    terminal["nominal_return"]["path"] = json!([0]);
                }
            }
            for rule in forged["package"]["grammar"]["rules"]
                .as_object_mut()
                .unwrap()
                .values_mut()
            {
                if !rule["nominal_return"].is_null() {
                    rule["nominal_return"]["path"] = json!([0]);
                }
            }
            if let Ok(rebuilt) = intersect_web::dispatch(&forged.to_string()) {
                assert_eq!(
                    rebuilt, analysis,
                    "serialized nominal marker is never authority"
                );
            }
        }
    }
}
struct CountedRgb {
    inner: intersect_model::conversion::RgbConverter,
    parses: std::cell::Cell<usize>,
}
impl intersect_model::definitions::conversion::ConversionProvider for CountedRgb {
    fn contract(&self) -> &intersect_model::definitions::conversion::ConversionContract {
        self.inner.contract()
    }
    fn classifier_relation(
        &self,
        target: &intersect_model::definitions::conversion::ConversionContract,
    ) -> Result<Option<intersect_model::definitions::conversion::ClassifierRelation>, String> {
        self.inner.classifier_relation(target)
    }
    fn parse(&self, text: &str) -> Result<Value, String> {
        self.parses.set(self.parses.get() + 1);
        self.inner.parse(text)
    }
    fn format(&self, value: &Value) -> Result<String, String> {
        self.inner.format(value)
    }
    fn validate(&self, value: &Value) -> Result<(), String> {
        self.inner.validate(value)
    }
    fn default_value(&self) -> Result<Option<Value>, String> {
        self.inner.default_value()
    }
}
fn verify_committed_provider_invocations() {
    use intersect_model::definitions::conversion::{ConversionContract, ConversionRegistry};
    let data: Value = serde_json::from_str(include_str!("fixtures/durable_rgb.json")).unwrap();
    for case in data["cases"].as_array().unwrap() {
        let contract: ConversionContract =
            serde_json::from_value(case["compile_request"]["conversions"][0].clone()).unwrap();
        let provider = CountedRgb {
            inner: intersect_model::conversion::RgbConverter::new(contract),
            parses: std::cell::Cell::new(0),
        };
        let mut registry = ConversionRegistry::default();
        registry
            .register(
                &intersect_model::definitions::compiler::standard_datatype("EJavaObject").unwrap(),
                &provider,
            )
            .unwrap();
        let inputs: BundleCompilation =
            serde_json::from_value(case["compile_request"]["compilation"].clone()).unwrap();
        let package = intersect_text::ordinary::compile_with_context(
            inputs,
            intersect_text::grammar::Profile::NativeV65,
            &[],
            Some(&registry),
            Default::default(),
            Default::default(),
        )
        .unwrap();
        let language = intersect_text::document::DocumentLanguage::prepare_with_context(
            &package,
            &[],
            Some(&registry),
            Default::default(),
            intersect_text::document::AnalysisLimits::workbench(),
        )
        .unwrap();
        assert_eq!(
            provider.parses.get(),
            0,
            "prediction/preparation must not execute conversion"
        );
        for (text, expected) in [
            ("paint #010203", 1),
            ("paint #bad", 1),
            ("paint #", 0),
            ("paint", 0),
        ] {
            provider.parses.set(0);
            let source = intersect_text::Source {
                uri: "memory:committed-provider/model".into(),
                text: text.into(),
            };
            let analysis = language
                .analyze_with_context(
                    &source,
                    None,
                    intersect_text::EvaluationLimits::native(),
                    Some(&registry),
                    None,
                    &intersect_model::query::PackageDerived(&package),
                )
                .unwrap();
            assert_eq!(
                provider.parses.get(),
                expected,
                "one committed converter call, no prediction/backtracking: {text}"
            );
            if text == "paint #bad" {
                assert!(analysis.valid_model.is_none());
                assert!(analysis.recovered.is_some());
                assert_eq!(
                    serde_json::to_value(&analysis.snapshot).unwrap()["syntax_issues"]
                        .as_array()
                        .unwrap()
                        .len(),
                    1
                );
            }
        }
    }
}
#[cfg(test)]
mod controls {
    #[test]
    fn exact_provider_models_nodes_and_fresh_authority() {
        super::verify_typed_rgb_documents();
    }
}
