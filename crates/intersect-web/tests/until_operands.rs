//! Source-loaded full generated-language Until controls over the shared native engine.
use intersect_model::core::storage;
use intersect_model::core::storage::ModelRead;
use intersect_model::definitions::bundle::SourceId;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_into, evaluate_stored};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until/native-cases.json"
    ))
    .unwrap()
}
fn observations() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until/native-observations.json"
    ))
    .unwrap()
}
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn inputs(case: &Value) -> BundleCompilation {
    serde_json::from_value(case["compilation"].clone()).unwrap()
}
fn compiled(
    case: &Value,
    profile: Profile,
) -> Result<Package, Vec<intersect_model::definitions::schema::Diagnostic>> {
    intersect_text::ordinary::compile_with_optional_source_policy(
        inputs(case),
        profile,
        &[],
        None,
        None,
        Default::default(),
    )
}
#[test]
fn full_generated_original_graphs_drive_native_until_and_ordinary_rule_boundaries() {
    let fixture = fixture();
    let observations = observations();
    let mut failures = vec![];
    let mut graphs = 0;
    let mut evaluated = 0;
    for expected in observations["cases"].as_array().unwrap() {
        if expected["upstream_generated"] != true {
            assert!(expected["runtime"].as_array().unwrap().is_empty());
            continue;
        }
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == expected["id"])
            .unwrap();
        let package = match compiled(case, Profile::NativeV60) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("{} compile: {e:?}", case["id"]));
                continue;
            }
        };
        graphs += 1;
        assert_eq!(
            json!(package.compilation.as_ref().unwrap()),
            case["compilation"]
        );
        assert_eq!(
            call(
                json!({"method":"compile_bundle","compilation":case["compilation"],"xtext_profile":Profile::NativeV60})
            )["package"],
            json!(package)
        );
        for run in expected["runtime"].as_array().unwrap() {
            evaluated += 1;
            let text = run["text"].as_str().unwrap();
            let source = Source {
                uri: format!("memory:until/{}.model", case["id"].as_str().unwrap()),
                text: text.into(),
            };
            let result = evaluate(&package, std::slice::from_ref(&source));
            let accepted = result.diagnostics.is_empty();
            if accepted != run["resource_accepted"].as_bool().unwrap() {
                failures.push(format!(
                    "{} {text:?} expectedaccepted={} got={:?}",
                    case["id"], run["resource_accepted"], result.diagnostics
                ));
                continue;
            }
            assert_eq!(
                json!(result),
                call(json!({"method":"evaluate","package":package,"sources":[source]}))
            );
            if accepted {
                let stored = evaluate_stored(
                    &package,
                    std::slice::from_ref(&source),
                    EvaluationLimits::native(),
                )
                .unwrap();
                assert_eq!(storage::export(&stored.store).unwrap(), result.model);
                assert_eq!(result.model.elements.len(), 1);
                let element = &result.model.elements[0];
                assert_eq!(element.start, 0);
                assert_eq!(element.end, text.len());
                for (feature, value) in run["resource_model"][0]["values"].as_object().unwrap() {
                    assert_eq!(
                        stored.store.value(&element.id, feature).unwrap(),
                        Some(value.clone())
                    );
                }
            } else {
                assert!(
                    result.model.elements.is_empty(),
                    "Native invalid resources remain atomic; upstream partials retained in oracle"
                );
            }
            if run["first_token_name"] == "RULE_VALUE" {
                let grammar = package.grammar.as_ref().unwrap();
                let terminal = &grammar.terminals["VALUE"];
                let end = terminal.match_len(text);
                let units = run["first_token_utf16_end"].as_u64().unwrap() as usize;
                let byte_end = text
                    .char_indices()
                    .find_map(|(at, _)| (text[..at].encode_utf16().count() == units).then_some(at))
                    .unwrap_or(text.len());
                if end != Ok(byte_end) {
                    failures.push(format!(
                        "{} {text:?} firstVALUEend expected{byte_end} got{end:?}",
                        case["id"]
                    ));
                }
            }
        }
    }
    assert_eq!(graphs, 20, "compilefailures: {failures:#?}");
    assert_eq!(evaluated, 268);
    assert!(
        failures.is_empty(),
        "fullsource disagreements: {failures:#?}"
    );
}
#[test]
fn complete_frozen_v59_call_responses_remain_unchanged() {
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until/before-calls-v59.json"
    ))
    .unwrap();
    for row in before["cases"].as_array().unwrap() {
        assert_eq!(
            call(row["compile"]["request"].clone()),
            row["compile"]["result"]
        );
        for input in row["inputs"].as_array().unwrap() {
            if input.get("evaluate").is_some() {
                assert_eq!(
                    call(input["evaluate"]["request"].clone()),
                    input["evaluate"]["result"]
                );
            }
        }
    }
    let full7: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until/before-full7-v59.json"
    ))
    .unwrap();
    for row in full7["cases"].as_array().unwrap() {
        assert_eq!(
            call(row["compile"]["request"].clone()),
            row["compile"]["result"]
        );
        for run in row["evaluations"].as_array().unwrap() {
            assert_eq!(call(run["request"].clone()), run["result"]);
        }
    }
}

fn named<'a>(f: &'a Value, id: &str) -> &'a Value {
    f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
}
fn authored(c: &Value, body: &str) -> BundleCompilation {
    let mut input = inputs(c);
    let selected = input.xtext.clone().unwrap();
    let source = input
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == selected)
        .unwrap();
    let prefix = source
        .text
        .split("terminal VALUE returns ecore::EString:")
        .next()
        .unwrap();
    source.text = format!("{prefix}terminal VALUE returns ecore::EString: {body};\n");
    source.id = SourceId::for_text(&selected.uri, &source.text);
    input.xtext = Some(source.id.clone());
    input
}
#[test]
fn source_replay_preserves_ordinary_inherited_and_explicit_parent_authority() {
    let f = fixture();
    let o = observations();
    for c in f["cases"].as_array().unwrap() {
        let expected = named(&o, c["id"].as_str().unwrap());
        if expected["upstream_generated"] != true {
            continue;
        }
        let package = compiled(c, Profile::NativeV60).unwrap();
        assert_eq!(
            json!(intersect_text::recompile(&package).unwrap()),
            json!(package)
        );
        let input = inputs(c);
        let grammar = package.grammar.as_ref().unwrap();
        for declaration in grammar.declaration_sources.values() {
            assert!(
                input
                    .bundle
                    .sources
                    .iter()
                    .any(|s| s.id == declaration.source)
            );
        }
        for source in &input.bundle.sources {
            assert_eq!(source.id, SourceId::for_text(&source.id.uri, &source.text));
        }
        let mut stale = input;
        let selected = stale.xtext.clone().unwrap();
        stale
            .bundle
            .sources
            .iter_mut()
            .find(|s| s.id == selected)
            .unwrap()
            .text
            .push(' ');
        assert!(intersect_text::ordinary::compile(stale, Profile::NativeV60).is_err());
    }
}
fn until_node(value: &mut Value) -> Option<&mut Value> {
    if value.get("UntilOperand").is_some() {
        return value.get_mut("UntilOperand");
    }
    match value {
        Value::Object(fields) => {
            for v in fields.values_mut() {
                if let Some(found) = until_node(v) {
                    return Some(found);
                }
            }
        }
        Value::Array(items) => {
            for v in items {
                if let Some(found) = until_node(v) {
                    return Some(found);
                }
            }
        }
        _ => {}
    }
    None
}
#[test]
fn source_rebuilt_archives_reject_rehashed_operand_and_executable_decision_forgery() {
    let f = fixture();
    for id in [
        "direct_until_suffix_c",
        "valid_explicit_parent_operand",
        "fragment_companion",
    ] {
        let c = named(&f, id);
        let package = compiled(c, Profile::NativeV60).unwrap();
        let exported = call(
            json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV60,"metadata":{"package_id":"finite.until","release":"1","display_name":"Finite Until","description":"Full generated source consumer"}}),
        );
        let mut files = exported["files"].as_object().unwrap().clone();
        files.insert(
            "manifest.json".into(),
            json!(exported["manifest"].to_string()),
        );
        for field in ["operand", "decision"] {
            let mut forged = files.clone();
            let mut cached: Value =
                serde_json::from_str(forged["package.json"].as_str().unwrap()).unwrap();
            let node = until_node(&mut cached["grammar"]).unwrap();
            if field == "operand" {
                node["operand"]["kind"] = json!({"Literal":[97,98]});
            } else {
                let old = node["decision"]["states"][0]["accepting"]
                    .as_bool()
                    .unwrap();
                node["decision"]["states"][0]["accepting"] = json!(!old);
            }
            assert_eq!(cached["compilation"], c["compilation"]);
            let text = cached.to_string();
            let mut manifest: Value =
                serde_json::from_str(forged["manifest.json"].as_str().unwrap()).unwrap();
            manifest["files"]["package.json"] =
                json!({"bytes":text.len(),"sha256":SourceId::for_text("irrelevant",&text).sha256});
            forged.insert("package.json".into(), json!(text));
            forged.insert("manifest.json".into(), json!(manifest.to_string()));
            let error = intersect_web::dispatch(
                &json!({"method":"package/import","files":forged}).to_string(),
            )
            .unwrap_err();
            assert!(
                error.contains(
                    "Archived compiled package differs from freshly compiled definitions"
                ),
                "{id}/{field}:{error}"
            );
            assert_eq!(
                intersect_text::distribution::import_files(
                    serde_json::from_value(json!(forged)).unwrap()
                )
                .unwrap_err(),
                error
            );
        }
        let loaded = call(json!({"method":"package/import","files":files}));
        assert_eq!(loaded["package"], json!(package));
    }
}
#[test]
fn unproved_domains_explicitly_reject_and_all_until_literals_use_new_execution_path() {
    let f = fixture();
    let c = named(&f, "direct_until_suffix_c");
    for body in [
        "'@' ->'ab'*",
        "'@' (->'ab')*",
        "'@' (->'ab')?",
        "'@' BLOCK*; terminal fragment BLOCK: ->'ab'",
        "'@' BLOCK?; terminal fragment BLOCK: ->'ab'",
        "'@' ->''",
        "'@' ->EOF",
        "'@' ->('a'?)",
        "'@' ->('a'*)",
        "'@' ->(->'a')",
        "'@' ->.",
        "'@' ->('a'..'z')",
        "'@' ->END; terminal fragment END: END",
    ] {
        assert!(
            intersect_text::ordinary::compile(authored(c, body), Profile::NativeV60).is_err(),
            "{body}"
        );
    }
    let input = authored(c, "'@' ->'ab' 'c'");
    let package = intersect_text::ordinary::compile(input, Profile::NativeV60).unwrap();
    let mut serialized = json!(package);
    let until = until_node(&mut serialized["grammar"]).unwrap();
    assert_eq!(until["operand"]["kind"], json!({"Literal":[97,98]}));
    assert!(until["decision"]["states"].as_array().unwrap().len() > 2);
    let huge = authored(c, &format!("'@' ->'{}'", "a".repeat(12_000)));
    let failures = intersect_text::ordinary::compile(huge, Profile::NativeV60).unwrap_err();
    assert!(
        failures
            .iter()
            .any(|e| e.message.contains("prediction budget exceeded")),
        "{failures:?}"
    );
}
#[test]
fn actual_until_failure_and_resource_limits_leave_existing_model_unchanged() {
    let f = fixture();
    let package = compiled(named(&f, "direct_until_suffix_c"), Profile::NativeV60).unwrap();
    let source = |text: &str| Source {
        uri: "memory:until/atomic.model".into(),
        text: text.into(),
    };
    let valid = [source("@abc")];
    let mut store = evaluate_stored(&package, &valid, EvaluationLimits::native())
        .unwrap()
        .store;
    let previous = storage::export(&store).unwrap();
    for text in [
        "@ab",
        "@abx",
        "@abc@abc",
        "@xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ] {
        let result = evaluate_into(
            &package,
            &[source(text)],
            EvaluationLimits::native(),
            &mut store,
        )
        .unwrap();
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert_eq!(storage::export(&store).unwrap(), previous);
    }
    for limits in [
        EvaluationLimits {
            resources: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            input_bytes: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            elements_per_resource: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            tokens_per_resource: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            steps_per_resource: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            steps_per_resource: Some(100),
            ..EvaluationLimits::native()
        },
    ] {
        let failed = intersect_text::evaluate_with_limits(&package, &valid, limits);
        assert!(!failed.diagnostics.is_empty());
        assert!(failed.model.elements.is_empty());
        assert_eq!(storage::export(&store).unwrap(), previous);
    }
    assert_eq!(
        store.value(&previous.elements[0].id, "name").unwrap(),
        Some(json!("@abc"))
    );
}
#[test]
fn source_parameter_programs_keep_v60_until_and_legacy_inferred_contracts() {
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/native-cases.json"
    ))
    .unwrap();
    let input = authored(&old["parameterized"], "'@' ->('ab'|'abc') 'c'");
    let package = intersect_text::ordinary::compile(input.clone(), Profile::NativeV60).unwrap();
    assert_eq!(
        package.grammar.as_ref().unwrap().profile,
        Profile::NativeV60
    );
    assert!(package.xtext_inference.is_none());
    assert_eq!(json!(package.compilation), json!(input));
    assert_eq!(
        json!(intersect_text::recompile(&package).unwrap()),
        json!(package)
    );
    for (text, accepted) in [
        ("@abc", true),
        ("@xabc", true),
        ("@ab", false),
        ("off", false),
    ] {
        let result = evaluate(
            &package,
            &[Source {
                uri: "memory:until/parameter.model".into(),
                text: text.into(),
            }],
        );
        assert_eq!(
            result.diagnostics.is_empty(),
            accepted,
            "{text}:{:?}",
            result.diagnostics
        );
    }
}
#[test]
fn original_failed_first_parser_graphs_are_explicit_admission_gaps_not_upstream_success() {
    let f = fixture();
    let observations = observations();
    for id in ["inherited_operand_override", "explicit_parent_operand"] {
        let expected = named(&observations, id);
        assert_eq!(expected["upstream_generated"], false);
        assert!(expected["runtime"].as_array().unwrap().is_empty());
        let native = compiled(named(&f, id), Profile::NativeV60);
        assert!(
            native.is_ok(),
            "Frozen known admission discrepancy: native accepts these original inherited parser-entry graphs while full Xtext generation rejects; no semantic award"
        );
    }
}

#[test]
fn separate_full_generated_literal_suffix_graph_skips_failed_early_delimiters_and_keeps_old_v59() {
    let c: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until-literal-suffix/native-case.json"
    ))
    .unwrap();
    let observed: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until-literal-suffix/native-observations.json"
    ))
    .unwrap();
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-until-literal-suffix/before-v59.json"
    ))
    .unwrap();
    assert_eq!(
        call(before["compile"]["request"].clone()),
        before["compile"]["result"]
    );
    for run in before["evaluations"].as_array().unwrap() {
        assert_eq!(call(run["request"].clone()), run["result"]);
    }
    let package = compiled(&c, Profile::NativeV60).unwrap();
    assert_eq!(
        json!(package.compilation.as_ref().unwrap()),
        c["compilation"]
    );
    assert_eq!(
        json!(intersect_text::recompile(&package).unwrap()),
        json!(package)
    );
    assert_eq!(
        c["compilation"]["xtext"]["sha256"],
        "f80f72ea0daf718e83753ef24cdf3072348dfdb352d006eaf027be06b3df10b3"
    );
    assert_eq!(observed["runtime"].as_array().unwrap().len(), 7);
    let mut accepted = 0;
    let mut partials = 0;
    for run in observed["runtime"].as_array().unwrap() {
        let text = run["text"].as_str().unwrap();
        let sources = [Source {
            uri: "memory:until/literal-suffix.model".into(),
            text: text.into(),
        }];
        let result = evaluate(&package, &sources);
        let expected = run["resource"]["accepted"].as_bool().unwrap();
        assert_eq!(
            result.diagnostics.is_empty(),
            expected,
            "{text}:{:?}",
            result.diagnostics
        );
        assert_eq!(
            json!(result),
            call(json!({"method":"evaluate","package":package,"sources":sources}))
        );
        if expected {
            accepted += 1;
            let stored = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
            let element = &result.model.elements[0];
            assert_eq!(element.start, 0);
            assert_eq!(element.end, text.len());
            for (feature, value) in run["resource"]["model"][0]["values"].as_object().unwrap() {
                assert_eq!(
                    stored.store.value(&element.id, feature).unwrap(),
                    Some(value.clone())
                );
            }
        } else {
            assert!(result.model.elements.is_empty());
            if run["resource"]["model"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
            {
                partials += 1;
                assert_eq!(run["resource"]["model"][0]["values"]["name"], "@abc");
            }
        }
        if run["first_token"]["name"] == "RULE_VALUE" {
            assert_eq!(
                package.grammar.as_ref().unwrap().terminals["VALUE"]
                    .match_len(text)
                    .unwrap(),
                run["first_token"]["utf16_stop"].as_u64().unwrap() as usize + 1
            );
        }
    }
    assert_eq!((accepted, partials), (4, 2));
}
