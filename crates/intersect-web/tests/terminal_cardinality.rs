//! Exact full-source cardinality controls; source failures/timeouts remain separate.
use intersect_model::core::storage;
use intersect_model::core::storage::ModelRead;
use intersect_text::grammar::Profile;
use intersect_text::package::CompilationSourcePolicy;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_stored};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/terminal-cardinality-v63/cases.json")).unwrap()
}
fn compilation(case: &Value) -> BundleCompilation {
    serde_json::from_value(case["compilation"].clone()).unwrap()
}
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap_or_else(|error| json!({"error":error}))
}
fn compile(case: &Value) -> Package {
    intersect_text::ordinary::compile_with_optional_source_policy(
        compilation(case),
        Profile::NativeV63,
        &[],
        None,
        None,
        Default::default(),
    )
    .unwrap_or_else(|error| panic!("{}: {error:?}", case["id"]))
}

pub fn complete_thirty_source_graphs_keep_stage_distinctions_and_model_values() {
    let f = fixture();
    let mut counts = (0, 0, 0, 0, 0);
    for c in f["cases"].as_array().unwrap() {
        if c["clean_upstream_generation"] != true {
            assert!(
                intersect_text::ordinary::compile(compilation(c), Profile::NativeV63).is_err(),
                "{}",
                c["id"]
            );
            counts.1 += 1;
            continue;
        }
        let package = compile(c);
        counts.0 += 1;
        assert_eq!(
            package.grammar.as_ref().unwrap().profile,
            Profile::NativeV63
        );
        assert_eq!(json!(package.compilation), c["compilation"]);
        assert_eq!(
            json!(intersect_text::recompile(&package).unwrap()),
            json!(package)
        );
        assert_eq!(
            call(
                json!({"method":"compile_bundle","compilation":c["compilation"],"xtext_profile":Profile::NativeV63})
            )["package"],
            json!(package)
        );
        for r in c["runtime"].as_array().unwrap() {
            let text = r["text"].as_str().unwrap();
            let sources = [Source {
                uri: format!("memory:cardinality-v63/{}.model", c["id"].as_str().unwrap()),
                text: text.into(),
            }];
            let stored =
                evaluate_stored(&package, &sources, EvaluationLimits::workbench()).unwrap();
            let direct = evaluate(&package, &sources);
            assert_eq!(storage::export(&stored.store).unwrap(), direct.model);
            assert_eq!(
                json!(direct),
                call(json!({"method":"evaluate","package":package,"sources":sources}))
            );
            if r["upstream_complete"] != true {
                counts.4 += 1;
                // Original process timeout is unobserved, not an upstream reject.
                continue;
            }
            let accepted = r["upstream_accepted"].as_bool().unwrap();
            assert_eq!(
                direct.diagnostics.is_empty(),
                accepted,
                "{} {text:?}: {:?}",
                c["id"],
                direct.diagnostics
            );
            if accepted {
                counts.2 += 1;
                let expected = &r["positive"]["resource"]["model"][0];
                assert_eq!(direct.model.elements.len(), 1);
                let element = &direct.model.elements[0];
                assert_eq!(element.class, "Model");
                assert_eq!((element.start, element.end), (0, text.len()));
                assert_eq!(element.resource, sources[0].uri);
                for (name, value) in expected["values"].as_object().unwrap() {
                    assert_eq!(
                        stored.store.value(&element.id, name).unwrap(),
                        Some(value.clone())
                    );
                }
                assert_eq!(
                    expected["class"],
                    "urn:intersect:xtext:mini-kerml:terminal-spellings:1#//Model"
                );
            } else {
                counts.3 += 1;
                assert!(
                    direct.model.elements.is_empty(),
                    "Do not publish a rejected upstream partial model"
                );
            }
        }
    }
    assert_eq!(counts, (13, 17, 31, 54, 9));
}

pub fn inherited_context_addendum_keeps_clean_override_warning_admissible() {
    let f: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/context-addendum.json"
    ))
    .unwrap();
    for c in f["cases"].as_array().unwrap() {
        let package = compile(c);
        assert_eq!(json!(package.compilation), c["compilation"]);
        assert_eq!(
            json!(intersect_text::recompile(&package).unwrap()),
            json!(package)
        );
        // Metadata-only scope: original inherited full workflow had no runtime.
    }
}

pub fn raw_core_cannot_admit_v63_without_the_language_source_adapter() {
    let f = fixture();
    let c = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "ecore_aliased_implicit")
        .unwrap();
    let error =
        intersect_text::compiler::compile_bundle_with_profile(compilation(c), Profile::NativeV63)
            .unwrap_err();
    assert!(format!("{error:?}").contains("language-owned authored-source validation"));
    let package = compile(c);
    let inputs = compilation(c);
    let authored = inputs
        .bundle
        .source(inputs.xtext.as_ref().unwrap())
        .unwrap();
    let mut schema = package.schema.clone();
    assert!(
        intersect_text::grammar::compile_with_reference_context_and_profile(
            &authored.text,
            &mut schema,
            Profile::NativeV63,
        )
        .is_err()
    );
    assert!(
        intersect_text::grammar::compile_with_generated_metamodels(
            authored,
            &inputs.bundle,
            &mut schema,
            Profile::NativeV63,
            Default::default(),
        )
        .is_err()
    );
}

pub fn complete_v62_and_default_before_responses_remain_exact() {
    let original: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/original-v62-default-before.json"
    ))
    .unwrap();
    let mut count = 0;
    for pair in original["transcript"].as_array().unwrap() {
        assert_eq!(call(pair["request"].clone()), pair["result"]);
        count += 1;
    }
    let context: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/original-v62-context-before.json"
    ))
    .unwrap();
    for pair in context.as_array().unwrap() {
        assert_eq!(call(pair["request"].clone()), pair["result"]);
        count += 1;
    }
    assert_eq!(count, 100);
}

pub fn same_source_inherited_workflow_addendum_models_have_distinct_qualification() {
    let f: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/context-addendum.json"
    ))
    .unwrap();
    let up: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/inherited-workflow-addendum.json"
    ))
    .unwrap();
    let mut count = (0, 0);
    for c in f["cases"].as_array().unwrap() {
        let package = compile(c);
        for r in up["runtime"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == c["id"])
        {
            let text = r["text"].as_str().unwrap();
            let sources = [Source {
                uri: format!(
                    "memory:cardinality-v63-inherited/{}.model",
                    c["id"].as_str().unwrap()
                ),
                text: text.into(),
            }];
            let stored =
                evaluate_stored(&package, &sources, EvaluationLimits::workbench()).unwrap();
            let result = evaluate(&package, &sources);
            assert_eq!(
                result.diagnostics.is_empty(),
                r["accepted"].as_bool().unwrap()
            );
            assert_eq!(storage::export(&stored.store).unwrap(), result.model);
            if r["accepted"] == true {
                count.0 += 1;
                let element = &result.model.elements[0];
                assert_eq!((element.start, element.end), (0, text.len()));
                for (name, value) in r["positive"]["resource"]["model"][0]["values"]
                    .as_object()
                    .unwrap()
                {
                    assert_eq!(
                        stored.store.value(&element.id, name).unwrap(),
                        Some(value.clone())
                    );
                }
            } else {
                count.1 += 1;
                assert!(result.model.elements.is_empty());
            }
        }
    }
    assert_eq!(count, (2, 8));
}

fn forge_cardinality(value: &mut Value, field: &str) -> bool {
    match value {
        Value::Object(map) => {
            if let Some(decision) = map.get_mut("cardinality_decision") {
                if field == "removed" {
                    *decision = Value::Null;
                } else if field == "root" {
                    decision["roots"][0] = decision["roots"][1].clone();
                } else {
                    map.insert("cardinality".into(), json!("1"));
                }
                return true;
            }
            map.values_mut().any(|v| forge_cardinality(v, field))
        }
        Value::Array(items) => items.iter_mut().any(|v| forge_cardinality(v, field)),
        _ => false,
    }
}

pub fn rehashed_archives_cannot_authorize_cached_cardinality_decisions() {
    use intersect_model::definitions::bundle::SourceId;
    let f = fixture();
    for id in [
        "inline_star_follow_typed",
        "nullable_fragment_disjoint_follow",
        "nullable_fragment_optional_local_eot",
    ] {
        let c = f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        let package = compile(c);
        let exported = call(
            json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV63,"metadata":{"package_id":"cardinality.source","release":"1","display_name":"Cardinality","description":"Source authoritative cardinality controls"}}),
        );
        assert_eq!(exported["package"], json!(package));
        let mut files = exported["files"].as_object().unwrap().clone();
        files.insert(
            "manifest.json".into(),
            json!(exported["manifest"].to_string()),
        );
        assert_eq!(
            call(json!({"method":"package/import","files":files}))["package"],
            json!(package)
        );
        for field in ["removed", "root", "cardinality"] {
            let mut forged = files.clone();
            let mut cached: Value =
                serde_json::from_str(forged["package.json"].as_str().unwrap()).unwrap();
            assert!(forge_cardinality(
                &mut cached["grammar"]["terminals"]["VALUE"]["body"],
                field
            ));
            assert_eq!(cached["compilation"], c["compilation"]);
            let text = cached.to_string();
            let mut manifest: Value =
                serde_json::from_str(forged["manifest.json"].as_str().unwrap()).unwrap();
            manifest["files"]["package.json"] =
                json!({"bytes":text.len(),"sha256":SourceId::for_text("ignored",&text).sha256});
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
                "{error}"
            );
        }
    }
}

pub fn exact_and_one_short_execution_limits_preserve_previous_publication() {
    let f = fixture();
    for (id, text) in [
        ("inline_optional_follow_typed", "aab"),
        ("inline_star_follow_typed", "aaaab"),
        ("inline_plus_follow_typed", "aab"),
        ("nullable_fragment_disjoint_follow", "aab"),
        ("nullable_fragment_optional_local_eot", "aab"),
    ] {
        let c = f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        let package = compile(c);
        let sources = [Source {
            uri: format!("memory:cardinality-budget/{id}.model"),
            text: text.into(),
        }];
        let run = |steps| {
            intersect_text::evaluate_with_limits(
                &package,
                &sources,
                EvaluationLimits {
                    steps_per_resource: Some(steps),
                    ..EvaluationLimits::workbench()
                },
            )
        };
        let mut low = 0;
        let mut high = 1;
        while !run(high).diagnostics.is_empty() {
            high *= 2;
            assert!(high < 1_000_000);
        }
        while low + 1 < high {
            let mid = (low + high) / 2;
            if run(mid).diagnostics.is_empty() {
                high = mid;
            } else {
                low = mid;
            }
        }
        assert!(run(high).diagnostics.is_empty());
        assert!(!run(high - 1).diagnostics.is_empty());
        let failed = run(high - 1);
        assert!(failed.model.elements.is_empty());
        let mut store = evaluate_stored(&package, &sources, EvaluationLimits::workbench())
            .unwrap()
            .store;
        let previous = storage::export(&store).unwrap();
        let result = intersect_text::evaluate_into(
            &package,
            &sources,
            EvaluationLimits {
                steps_per_resource: Some(high - 1),
                ..EvaluationLimits::workbench()
            },
            &mut store,
        )
        .unwrap();
        assert!(!result.diagnostics.is_empty());
        assert_eq!(storage::export(&store).unwrap(), previous);
    }
}

pub fn declaration_and_byte_admission_boundaries_are_exact() {
    use intersect_text::package::CompilationSourcePolicy;
    let f = fixture();
    let c = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "ecore_aliased_implicit")
        .unwrap();
    let inputs = compilation(c);
    let source = inputs
        .bundle
        .source(inputs.xtext.as_ref().unwrap())
        .unwrap();
    let exact = CompilationSourcePolicy {
        xtext_bytes: source.text.len(),
        xtext_declarations: 2,
        ..Default::default()
    };
    let compile = |policy| {
        intersect_text::ordinary::compile_with_source_policy(
            inputs.clone(),
            Profile::NativeV63,
            policy,
            Default::default(),
        )
    };
    let admitted = compile(exact);
    assert!(admitted.is_ok(), "{admitted:?}");
    assert!(
        compile(CompilationSourcePolicy {
            xtext_bytes: exact.xtext_bytes - 1,
            ..exact
        })
        .is_err()
    );
    assert!(
        compile(CompilationSourcePolicy {
            xtext_declarations: 1,
            ..exact
        })
        .is_err()
    );
}

pub fn every_actual_first_value_endpoint_has_exact_utf8_boundary_and_charged_work() {
    let f = fixture();
    let context: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/context-addendum.json"
    ))
    .unwrap();
    let endpoints: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/first-value-endpoints.json"
    ))
    .unwrap();
    let mut count = 0;
    for r in endpoints["rows"].as_array().unwrap() {
        let c = context["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == r["id"])
            .or_else(|| {
                f["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"] == r["id"])
            })
            .unwrap();
        let package = compile(c);
        let text = r["text"].as_str().unwrap();
        let units = r["first_value_utf16_end"].as_u64().unwrap() as usize;
        let byte_end = text
            .char_indices()
            .map(|(at, _)| at)
            .chain(std::iter::once(text.len()))
            .find(|&at| text[..at].encode_utf16().count() == units)
            .expect("Observed endpoint must be scalar boundary");
        let terminal = &package.grammar.as_ref().unwrap().terminals["VALUE"];
        assert_eq!(
            terminal.match_len(text),
            Ok(byte_end),
            "{} {text:?}",
            r["id"]
        );
        let mut work = Some(1_000_000);
        assert_eq!(terminal.match_len_with_work(text, &mut work), Ok(byte_end));
        let used = 1_000_000 - work.unwrap();
        if used != 0 {
            let mut exact = Some(used);
            assert_eq!(terminal.match_len_with_work(text, &mut exact), Ok(byte_end));
            assert_eq!(exact, Some(0));
            let mut short = Some(used - 1);
            assert!(
                terminal
                    .match_len_with_work(text, &mut short)
                    .unwrap_err()
                    .contains("budget")
            );
        }
        count += 1;
    }
    assert_eq!(count, 45);
}

pub fn inherited_source_caps_precede_fresh_parsing_with_both_relative_world_limits() {
    use intersect_model::definitions::bundle::SourceId;
    let f: Value = serde_json::from_str(include_str!(
        "fixtures/terminal-cardinality-v63/context-addendum.json"
    ))
    .unwrap();
    let mut inputs = compilation(&f["cases"][0]);
    let base = inputs
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.text.starts_with("grammar audit.ReturnIdentityBase "))
        .unwrap();
    let old = base.id.clone();
    base.text.push_str(&" ".repeat(2048));
    base.id = SourceId::for_text("memory:cardinality-budget/long-base.xtext", &base.text);
    let selected = base.id.clone();
    let bytes = base.text.len();
    for source in &mut inputs.bundle.sources {
        for id in &mut source.dependencies {
            if *id == old {
                *id = selected.clone();
            }
        }
    }
    let structure = inputs.bundle.source(&inputs.ecore).unwrap().text.len();
    let entry_bytes = inputs
        .bundle
        .source(inputs.xtext.as_ref().unwrap())
        .unwrap()
        .text
        .len();
    assert!(bytes > entry_bytes);
    for structure_bytes in [structure, 256_000] {
        let exact = CompilationSourcePolicy {
            structure_bytes,
            xtext_bytes: bytes,
            ..Default::default()
        };
        assert!(
            intersect_text::ordinary::compile_with_source_policy(
                inputs.clone(),
                Profile::NativeV63,
                exact,
                Default::default()
            )
            .is_ok()
        );
        for cap in [bytes - 1, entry_bytes] {
            let error = intersect_text::ordinary::compile_with_source_policy(
                inputs.clone(),
                Profile::NativeV63,
                CompilationSourcePolicy {
                    xtext_bytes: cap,
                    ..exact
                },
                Default::default(),
            )
            .unwrap_err();
            assert!(
                error
                    .iter()
                    .any(|e| e.source == selected.uri
                        && e.message == "Definition size budget exceeded"),
                "{error:?}"
            );
        }
    }
    let base = inputs
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == selected)
        .unwrap();
    base.text
        .push_str("terminal BROKEN returns missing::Type;\n");
    base.id = SourceId::for_text(
        "memory:cardinality-budget/invalid-long-base.xtext",
        &base.text,
    );
    let invalid = base.id.clone();
    let invalid_bytes = base.text.len();
    for source in &mut inputs.bundle.sources {
        for id in &mut source.dependencies {
            if *id == selected {
                *id = invalid.clone();
            }
        }
    }
    let error = intersect_text::ordinary::compile_with_source_policy(
        inputs.clone(),
        Profile::NativeV63,
        CompilationSourcePolicy {
            xtext_bytes: bytes - 1,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap_err();
    assert!(
        error
            .iter()
            .any(|e| e.source == invalid.uri && e.message == "Definition size budget exceeded"),
        "{error:?}"
    );
    let error = intersect_text::ordinary::compile_with_source_policy(
        inputs,
        Profile::NativeV63,
        CompilationSourcePolicy {
            xtext_bytes: invalid_bytes,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap_err();
    assert!(
        error
            .iter()
            .any(|e| e.source == invalid.uri && !e.message.contains("budget")),
        "{error:?}"
    );
}

pub fn behavior_keeps_its_own_existing_ceiling_under_narrow_grammar_and_structure_caps() {
    use intersect_model::definitions::bundle::SourceId;
    let f = fixture();
    let c = f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "ecore_aliased_implicit")
        .unwrap();
    let mut inputs = compilation(c);
    let requested = CompilationSourcePolicy {
        structure_bytes: inputs.bundle.source(&inputs.ecore).unwrap().text.len(),
        xtext_bytes: inputs
            .bundle
            .source(inputs.xtext.as_ref().unwrap())
            .unwrap()
            .text
            .len(),
        ..Default::default()
    };
    let behavior = inputs
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == inputs.rcore_behavior)
        .unwrap();
    behavior.text.push_str(&" ".repeat(2048));
    behavior.id = SourceId::for_text(
        "memory:cardinality-source-policy/native-only-padded-behavior.rcore",
        &behavior.text,
    );
    inputs.rcore_behavior = behavior.id.clone();
    for profile in [Profile::NativeV62, Profile::NativeV63] {
        assert!(
            intersect_text::ordinary::compile_with_source_policy(
                inputs.clone(),
                profile,
                requested,
                Default::default()
            )
            .is_ok()
        );
    }
    let behavior = inputs
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == inputs.rcore_behavior)
        .unwrap();
    behavior
        .text
        .push_str(&" ".repeat(256_001 - behavior.text.len()));
    behavior.id = SourceId::for_text(
        "memory:cardinality-source-policy/native-only-over-cap-behavior.rcore",
        &behavior.text,
    );
    inputs.rcore_behavior = behavior.id.clone();
    for profile in [Profile::NativeV62, Profile::NativeV63] {
        let error = intersect_text::ordinary::compile_with_source_policy(
            inputs.clone(),
            profile,
            requested,
            Default::default(),
        )
        .unwrap_err();
        assert!(error.iter().any(|e| e.source == inputs.rcore_behavior.uri
            && e.message == "Definition size budget exceeded"));
    }
}

pub fn parameter_program_rebuilds_original_source_under_v63_admission() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/native-cases.json"
    ))
    .unwrap();
    let inputs: BundleCompilation =
        serde_json::from_value(f["parameterized"]["compilation"].clone()).unwrap();
    let package = intersect_text::ordinary::compile(inputs.clone(), Profile::NativeV63).unwrap();
    assert_eq!(json!(package.compilation), json!(inputs));
    assert_eq!(
        json!(intersect_text::recompile(&package).unwrap()),
        json!(package)
    );
    for text in ["A", "_"] {
        assert!(
            evaluate(
                &package,
                &[Source {
                    uri: "memory:cardinality-v63/parameter.model".into(),
                    text: text.into()
                }]
            )
            .diagnostics
            .is_empty()
        );
    }
}

pub fn verify_all() {
    complete_thirty_source_graphs_keep_stage_distinctions_and_model_values();
    inherited_context_addendum_keeps_clean_override_warning_admissible();
    raw_core_cannot_admit_v63_without_the_language_source_adapter();
    complete_v62_and_default_before_responses_remain_exact();
    same_source_inherited_workflow_addendum_models_have_distinct_qualification();
    rehashed_archives_cannot_authorize_cached_cardinality_decisions();
    exact_and_one_short_execution_limits_preserve_previous_publication();
    declaration_and_byte_admission_boundaries_are_exact();
    every_actual_first_value_endpoint_has_exact_utf8_boundary_and_charged_work();
    inherited_source_caps_precede_fresh_parsing_with_both_relative_world_limits();
    behavior_keeps_its_own_existing_ceiling_under_narrow_grammar_and_structure_caps();
    parameter_program_rebuilds_original_source_under_v63_admission();
}

#[cfg(test)]
mod native_controls {
    #[test]
    fn complete_thirty_source_graphs_keep_stage_distinctions_and_model_values() {
        super::complete_thirty_source_graphs_keep_stage_distinctions_and_model_values();
    }
    #[test]
    fn inherited_context_addendum_keeps_clean_override_warning_admissible() {
        super::inherited_context_addendum_keeps_clean_override_warning_admissible();
    }
    #[test]
    fn raw_core_cannot_admit_v63_without_the_language_source_adapter() {
        super::raw_core_cannot_admit_v63_without_the_language_source_adapter();
    }
    #[test]
    fn complete_v62_and_default_before_responses_remain_exact() {
        super::complete_v62_and_default_before_responses_remain_exact();
    }
    #[test]
    fn same_source_inherited_workflow_addendum_models_have_distinct_qualification() {
        super::same_source_inherited_workflow_addendum_models_have_distinct_qualification();
    }
    #[test]
    fn rehashed_archives_cannot_authorize_cached_cardinality_decisions() {
        super::rehashed_archives_cannot_authorize_cached_cardinality_decisions();
    }
    #[test]
    fn exact_and_one_short_execution_limits_preserve_previous_publication() {
        super::exact_and_one_short_execution_limits_preserve_previous_publication();
    }
    #[test]
    fn declaration_and_byte_admission_boundaries_are_exact() {
        super::declaration_and_byte_admission_boundaries_are_exact();
    }
    #[test]
    fn every_actual_first_value_endpoint_has_exact_utf8_boundary_and_charged_work() {
        super::every_actual_first_value_endpoint_has_exact_utf8_boundary_and_charged_work();
    }
    #[test]
    fn inherited_source_caps_precede_fresh_parsing_with_both_relative_world_limits() {
        super::inherited_source_caps_precede_fresh_parsing_with_both_relative_world_limits();
    }
    #[test]
    fn behavior_keeps_its_own_existing_ceiling_under_narrow_grammar_and_structure_caps() {
        super::behavior_keeps_its_own_existing_ceiling_under_narrow_grammar_and_structure_caps();
    }
    #[test]
    fn parameter_program_rebuilds_original_source_under_v63_admission() {
        super::parameter_program_rebuilds_original_source_under_v63_admission();
    }
}
