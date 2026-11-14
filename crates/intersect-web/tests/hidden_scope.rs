//! Verification-only controls over original source graphs and shared production APIs.
use intersect_model::core::storage;
use intersect_model::definitions::bundle::SourceId;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy, Package};
use intersect_text::{EvaluationLimits, Source, evaluate_stored, evaluate_with_limits};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/hidden-scope-v64/cases.json")).unwrap()
}
fn inputs(case: &Value) -> BundleCompilation {
    serde_json::from_value(case["compilation"].clone()).unwrap()
}
fn case<'a>(fixture: &'a Value, id: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
}
fn compile(case: &Value) -> Package {
    intersect_text::ordinary::compile_with_optional_source_policy(
        inputs(case),
        Profile::NativeV64,
        &[],
        None,
        None,
        Default::default(),
    )
    .unwrap_or_else(|e| panic!("{}: {e:?}", case["id"]))
}
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap_or_else(|e| json!({"error":e}))
}

pub fn exact_original_source_admission_and_recompile() {
    let fixture = fixture();
    let mut counts = (0, 0);
    for c in fixture["cases"].as_array().unwrap() {
        let compiled = intersect_text::ordinary::compile(inputs(c), Profile::NativeV64);
        assert_eq!(
            compiled.is_ok(),
            c["clean_upstream_generation"].as_bool().unwrap(),
            "{}: {compiled:?}",
            c["id"]
        );
        if let Ok(package) = compiled {
            counts.0 += 1;
            assert_eq!(json!(package.compilation), c["compilation"]);
            assert_eq!(
                json!(intersect_text::recompile(&package).unwrap()),
                json!(package)
            );
            assert_eq!(
                package.grammar.as_ref().unwrap().profile,
                Profile::NativeV64
            );
        } else {
            counts.1 += 1;
        }
    }
    assert_eq!(counts, (22, 29));
}

pub fn source_language_adapter_and_original_profile_boundaries() {
    let fixture = fixture();
    for id in [
        "namespace_model_empty_scope",
        "namespace_model_adds_scope",
        "reference_adds_scope",
        "reference_empty_scope",
    ] {
        let c = case(&fixture, id);
        assert!(intersect_text::ordinary::compile(inputs(c), Profile::NativeV63).is_err());
        let error =
            intersect_text::compiler::compile_bundle_with_profile(inputs(c), Profile::NativeV64)
                .unwrap_err();
        assert!(format!("{error:?}").contains("language-owned authored-source validation"));
        compile(c);
    }
}

pub fn raw_trivia_work_and_token_limits_preserve_published_model() {
    let fixture = fixture();
    for (id, text) in [
        ("namespace_model_adds_scope", " /*😀*/a "),
        ("restore_child_policy", "[a/*😀*/] done"),
        ("prediction_marker_scope", "a/*x*/b!"),
        ("namespace_significant_ws_datatype", "a\r\nb"),
        ("caller_suffix", "direct a b?/*😀*/done"),
        ("reference_adds_scope", "type:é;use:f= /*😀*/é;"),
    ] {
        let package = compile(case(&fixture, id));
        let sources = [Source {
            uri: format!("memory:hidden-budget/{id}.model"),
            text: text.into(),
        }];
        let ordinary = evaluate_with_limits(&package, &sources, EvaluationLimits::workbench());
        assert!(
            ordinary.diagnostics.is_empty(),
            "{id}: {:?}",
            ordinary.diagnostics
        );
        for token_limit in [false, true] {
            let run = |limit| {
                evaluate_with_limits(
                    &package,
                    &sources,
                    EvaluationLimits {
                        steps_per_resource: (!token_limit).then_some(limit),
                        tokens_per_resource: token_limit.then_some(limit),
                        ..EvaluationLimits::workbench()
                    },
                )
            };
            let mut low = 0;
            let mut high = 1;
            while !run(high).diagnostics.is_empty() {
                high *= 2;
                assert!(high < 1_000_000, "{id}");
            }
            while low + 1 < high {
                let mid = (low + high) / 2;
                if run(mid).diagnostics.is_empty() {
                    high = mid;
                } else {
                    low = mid;
                }
            }
            assert_eq!(json!(run(high)), json!(ordinary));
            let denied = run(high - 1);
            assert!(!denied.diagnostics.is_empty());
            assert!(denied.model.elements.is_empty());
            let mut store = evaluate_stored(&package, &sources, EvaluationLimits::workbench())
                .unwrap()
                .store;
            let previous = storage::export(&store).unwrap();
            let result = intersect_text::evaluate_into(
                &package,
                &sources,
                EvaluationLimits {
                    steps_per_resource: (!token_limit).then_some(high - 1),
                    tokens_per_resource: token_limit.then_some(high - 1),
                    ..EvaluationLimits::workbench()
                },
                &mut store,
            )
            .unwrap();
            assert!(!result.diagnostics.is_empty());
            assert_eq!(storage::export(&store).unwrap(), previous);
        }
        assert!(
            evaluate_with_limits(
                &package,
                &sources,
                EvaluationLimits {
                    input_bytes: Some(text.len()),
                    ..EvaluationLimits::workbench()
                }
            )
            .diagnostics
            .is_empty()
        );
        assert!(
            !evaluate_with_limits(
                &package,
                &sources,
                EvaluationLimits {
                    input_bytes: Some(text.len() - 1),
                    ..EvaluationLimits::workbench()
                }
            )
            .diagnostics
            .is_empty()
        );
    }
}

pub fn used_sources_obey_exact_requested_xtext_byte_admission() {
    let fixture = fixture();
    for id in [
        "namespace_model_adds_scope",
        "namespace_inherited_terminal_override",
        "namespace_inherited_rule_override",
        "reference_adds_scope",
    ] {
        let c = case(&fixture, id);
        let inputs = inputs(c);
        let max = inputs
            .bundle
            .sources
            .iter()
            .filter(|s| s.kind == intersect_model::definitions::bundle::SourceKind::Xtext)
            .map(|s| s.text.len())
            .max()
            .unwrap();
        let requested = CompilationSourcePolicy {
            xtext_bytes: max,
            structure_bytes: max + 100_000,
            ..Default::default()
        };
        assert!(
            intersect_text::ordinary::compile_with_source_policy(
                inputs.clone(),
                Profile::NativeV64,
                requested,
                Default::default()
            )
            .is_ok(),
            "{id}"
        );
        let errors = intersect_text::ordinary::compile_with_source_policy(
            inputs.clone(),
            Profile::NativeV64,
            CompilationSourcePolicy {
                xtext_bytes: max - 1,
                ..requested
            },
            Default::default(),
        )
        .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message == "Definition size budget exceeded"
                    && inputs.bundle.sources.iter().any(|s| s.id.uri == e.source
                        && s.kind == intersect_model::definitions::bundle::SourceKind::Xtext
                        && s.text.len() == max)),
            "{id}: {errors:?}"
        );
    }
}

pub fn source_and_archive_caches_cannot_authorize_hidden_policy() {
    let fixture = fixture();
    for id in [
        "namespace_model_adds_scope",
        "namespace_model_empty_scope",
        "namespace_inherited_terminal_override",
    ] {
        let c = case(&fixture, id);
        let package = compile(c);
        let exported = call(
            json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV64,"metadata":{"package_id":"hidden.authority","release":"1","display_name":"Hidden authority","description":"Source authority controls"}}),
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
        for field in ["grammar", "rule", "terminal"] {
            let mut forged = files.clone();
            let mut cached: Value =
                serde_json::from_str(forged["package.json"].as_str().unwrap()).unwrap();
            match field {
                "grammar" => cached["grammar"]["hidden"] = json!(["ID"]),
                "rule" => {
                    cached["grammar"]["rules"]["Model"]["hidden"] =
                        if cached["grammar"]["rules"]["Model"]["hidden"] == json!([]) {
                            json!(["WS"])
                        } else {
                            json!([])
                        }
                }
                _ => cached["grammar"]["terminals"]["WS"]["identity"] = json!("forged.hidden::WS"),
            }
            assert_ne!(cached, json!(package));
            assert_eq!(cached["compilation"], c["compilation"]);
            assert_eq!(
                call(
                    json!({"method":"evaluate","package":cached,"sources":[{"uri":"memory:forged.model","text":"a"}]})
                ),
                call(
                    json!({"method":"evaluate","package":package,"sources":[{"uri":"memory:forged.model","text":"a"}]})
                ),
                "Web evaluation must rebuild exact original definitions instead of trusting forged cached policy",
            );
            let text = cached.to_string();
            let mut manifest: Value =
                serde_json::from_str(forged["manifest.json"].as_str().unwrap()).unwrap();
            manifest["files"]["package.json"] =
                json!({"bytes":text.len(),"sha256":SourceId::for_text("ignored",&text).sha256});
            forged.insert("package.json".into(), json!(text));
            forged.insert("manifest.json".into(), json!(manifest.to_string()));
            let error = call(json!({"method":"package/import","files":forged}));
            assert!(
                error["error"].as_str().unwrap().contains(
                    "Archived compiled package differs from freshly compiled definitions"
                ),
                "{id} {field}: {error}"
            );
        }
    }
}

pub fn verify_all() {
    exact_original_source_admission_and_recompile();
    source_language_adapter_and_original_profile_boundaries();
    raw_trivia_work_and_token_limits_preserve_published_model();
    used_sources_obey_exact_requested_xtext_byte_admission();
    source_and_archive_caches_cannot_authorize_hidden_policy();
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_admission() {
        super::exact_original_source_admission_and_recompile();
    }
    #[test]
    fn language_and_profile_authority() {
        super::source_language_adapter_and_original_profile_boundaries();
    }
    #[test]
    fn raw_trivia_budgets_and_rollback() {
        super::raw_trivia_work_and_token_limits_preserve_published_model();
    }
    #[test]
    fn source_byte_admission() {
        super::used_sources_obey_exact_requested_xtext_byte_admission();
    }
    #[test]
    fn archive_source_authority() {
        super::source_and_archive_caches_cannot_authorize_hidden_policy();
    }
}
