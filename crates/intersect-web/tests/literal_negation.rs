//! Full generated-language evidence consumers; legacy profiles remain frozen.
use intersect_model::core::storage;
use intersect_model::core::storage::ModelRead;
use intersect_model::definitions::bundle::SourceId;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_into, evaluate_stored};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-lexical/native-cases.json"
    ))
    .unwrap()
}
fn before() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-lexical/before-v58.json"
    ))
    .unwrap()
}
fn positives() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-lexical/positive-resources.json"
    ))
    .unwrap()
}
// Raw error/token/partial strings can contain isolated UTF-16 units. Decode only
// scalar-safe request and outcome fields; preserve the full oracle bytes intact.
#[derive(serde::Deserialize)]
struct Accepted {
    accepted: bool,
}
#[derive(serde::Deserialize)]
struct Observed {
    resource: Accepted,
    parser: Accepted,
}
#[derive(serde::Deserialize)]
struct Run {
    text: String,
    observation: Observed,
}
#[derive(serde::Deserialize)]
struct Oracle {
    id: String,
    runtime: Vec<Run>,
}
fn oracle() -> Vec<Oracle> {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/full-generated-lexical/observations.json"
    ))
    .unwrap()
}
fn case<'a>(f: &'a Value, id: &str) -> &'a Value {
    f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
}
fn inputs(c: &Value) -> BundleCompilation {
    serde_json::from_value(c["compilation"].clone()).unwrap()
}
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn compile(c: &Value, profile: Profile) -> Package {
    intersect_text::ordinary::compile_with_optional_source_policy(
        inputs(c),
        profile,
        &[],
        None,
        None,
        Default::default(),
    )
    .unwrap()
}
fn source(id: &str, text: &str) -> Source {
    Source {
        uri: format!("memory:literal-negation/{id}.model"),
        text: text.into(),
    }
}
fn assert_value(package: &Package, id: &str, text: &str, accepted: bool, positive: &Value) {
    let sources = [source(id, text)];
    let stored = evaluate_stored(package, &sources, EvaluationLimits::native()).unwrap();
    assert_eq!(
        stored.evaluation.diagnostics.is_empty(),
        accepted,
        "{id} {text:?}: {:?}",
        stored.evaluation.diagnostics
    );
    let result = evaluate(package, &sources);
    assert_eq!(
        json!(result),
        call(json!({"method":"evaluate","package":package,"sources":sources}))
    );
    assert_eq!(storage::export(&stored.store).unwrap(), result.model);
    if accepted {
        let expected = positive
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id && r["text"] == text)
            .unwrap();
        let element = &result.model.elements[0];
        assert_eq!(result.model.elements.len(), 1);
        assert_eq!(element.id, format!("{}#0", sources[0].uri));
        assert_eq!(element.resource, sources[0].uri);
        assert_eq!(element.class, "Model");
        assert_eq!(element.start, 0);
        assert_eq!(element.end, text.len());
        assert_eq!(
            package.schema.declarations["Model"].namespace,
            "urn:intersect:xtext:mini-kerml:terminal-spellings:1"
        );
        for (feature, value) in expected["resource"]["model"][0]["values"]
            .as_object()
            .unwrap()
        {
            assert_eq!(
                stored.store.value(&element.id, feature).unwrap(),
                Some(value.clone()),
                "observable feature {feature}"
            );
        }
        let leaf = &expected["resource"]["leaves"][0];
        assert_eq!(leaf["utf16_start"], 0);
        assert_eq!(leaf["utf16_end"], text.encode_utf16().count());
    } else {
        assert!(
            result.model.elements.is_empty(),
            "native must not publish upstream rejected partial model"
        );
    }
}

#[test]
fn complete_v58_and_all_older_literal_rejections_remain_exact() {
    let b = before();
    assert_eq!(
        b["baseline_wasm_sha256"],
        "f7a8cc16c414c47953641394bcd65427a193d2b89ac76a7cb17fc44800221062"
    );
    for row in b["v58"].as_array().unwrap() {
        assert_eq!(call(row["request"].clone()), row["result"], "{}", row["id"]);
        for run in row["model_runs"].as_array().unwrap() {
            assert_eq!(call(run["request"].clone()), run["result"]);
        }
    }
    assert_eq!(b["old_profiles"].as_array().unwrap().len(), 116);
    for row in b["old_profiles"].as_array().unwrap() {
        assert_eq!(
            call(row["request"].clone()),
            row["result"],
            "{} v{}",
            row["id"],
            row["profile"]
        );
    }
}

#[test]
fn original_full_generated_graphs_drive_real_native_json_and_stored_feature_reads() {
    let f = fixture();
    let positive = positives();
    let rows = oracle();
    assert_eq!(rows.iter().map(|c| c.runtime.len()).sum::<usize>(), 80);
    for id in ["nested_groups", "multichar", "astral_literal"] {
        let c = case(&f, id);
        let package = compile(c, Profile::NativeV59);
        assert_eq!(
            json!(package),
            call(
                json!({"method":"compile_bundle","compilation":c["compilation"],"xtext_profile":Profile::NativeV59})
            )["package"]
        );
        assert_eq!(
            json!(package.compilation.as_ref().unwrap()),
            c["compilation"]
        );
        assert_eq!(
            json!(intersect_text::recompile(&package).unwrap()),
            json!(package)
        );
        let grammar = package.grammar.as_ref().unwrap();
        assert_eq!(grammar.profile, Profile::NativeV59);
        let declaration = &grammar.declaration_sources["VALUE"];
        let original = inputs(c);
        let source = original
            .bundle
            .sources
            .iter()
            .find(|s| s.id == declaration.source)
            .unwrap();
        assert!(source.text[declaration.start..declaration.end].starts_with("terminal VALUE"));
        if id != "nested_groups" {
            let negated = &json!(grammar.terminals["VALUE"])["body"]["kind"]["Negated"];
            let expected = if id == "multichar" {
                vec![97_u16, 98]
            } else {
                vec![0xd83c, 0xdf0d]
            };
            assert_eq!(negated["operand"]["kind"]["Literal"], json!(expected));
            assert_eq!(negated["excluded"], json!([]));
        } else {
            assert_eq!(
                json!(grammar.terminals),
                json!(compile(c, Profile::NativeV58).grammar.unwrap().terminals)
            );
        }
        for run in &rows.iter().find(|r| r.id == id).unwrap().runtime {
            assert_eq!(
                run.observation.resource.accepted,
                run.observation.parser.accepted
            );
            assert_value(
                &package,
                id,
                &run.text,
                run.observation.resource.accepted,
                &positive,
            );
        }
    }
    // Valid full-generated until grammars remain real unsupported gaps.
    for id in [
        "alternatives_companion",
        "fragment_companion",
        "abc_or_b",
        "ab_or_abc",
    ] {
        let errors = intersect_text::ordinary::compile(inputs(case(&f, id)), Profile::NativeV59)
            .unwrap_err();
        assert!(errors.iter().any(|e| {
            e.message
                .contains("Terminal until requires a nonempty literal delimiter")
        }));
        assert!(
            rows.iter()
                .find(|r| r.id == id)
                .unwrap()
                .runtime
                .iter()
                .any(|r| r.observation.resource.accepted)
        );
    }
}

fn authored_body(c: &Value, body: &str) -> BundleCompilation {
    let mut input = c["compilation"].clone();
    let uri = input["xtext"]["uri"].as_str().unwrap().to_owned();
    let source = input["bundle"]["sources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["id"]["uri"] == uri)
        .unwrap();
    let original = source["text"].as_str().unwrap();
    let prefix = original
        .split("terminal VALUE returns ecore::EString:")
        .next()
        .unwrap();
    let text = format!("{prefix}terminal VALUE returns ecore::EString: {body};\n");
    let identity = SourceId::for_text(&uri, &text);
    source["text"] = json!(text);
    source["id"] = json!(identity);
    input["xtext"] = json!(identity);
    serde_json::from_value(input).unwrap()
}
#[test]
fn grouping_fragments_nullable_eof_and_unproved_domains_do_not_gain_literal_authority() {
    let f = fixture();
    let c = case(&f, "multichar");
    // Separate authored negative controls, not assertions that upstream rejects.
    for body in [
        "!('ab')",
        "!F; terminal fragment F: 'ab'",
        "!''",
        "!EOF",
        "!('a' 'b')",
        "!('a'?)",
        "!.",
    ] {
        let errors = intersect_text::ordinary::compile(authored_body(c, body), Profile::NativeV59)
            .unwrap_err();
        assert!(!errors.is_empty(), "{body}");
    }
    for id in ["multichar", "astral_literal"] {
        let mut stale = inputs(case(&f, id));
        let selected = stale.xtext.clone().unwrap();
        let source = stale
            .bundle
            .sources
            .iter_mut()
            .find(|s| s.id == selected)
            .unwrap();
        source.text.push(' ');
        assert!(intersect_text::ordinary::compile(stale, Profile::NativeV59).is_err());
    }
}

#[test]
fn archive_rebuild_rejects_rehashed_operand_and_exclusion_authority_and_allows_retry() {
    let f = fixture();
    let positive = positives();
    for id in ["multichar", "astral_literal"] {
        let c = case(&f, id);
        let package = compile(c, Profile::NativeV59);
        let exported = call(
            json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV59,"metadata":{"package_id":"literal.negation","release":"1","display_name":"Literal negation","description":"Full generated-language source consumer"}}),
        );
        assert_eq!(exported["package"], json!(package));
        let mut files = exported["files"].as_object().unwrap().clone();
        files.insert(
            "manifest.json".into(),
            json!(exported["manifest"].to_string()),
        );
        for field in ["excluded", "operand"] {
            let mut forged = files.clone();
            let mut cached: Value =
                serde_json::from_str(forged["package.json"].as_str().unwrap()).unwrap();
            let negated = &mut cached["grammar"]["terminals"]["VALUE"]["body"]["kind"]["Negated"];
            if field == "excluded" {
                negated["excluded"] = json!([[97, 97]]);
            } else {
                negated["operand"]["kind"] = json!({"Literal":[99,100]});
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
                "{error}"
            );
            let native_error = intersect_text::distribution::import_files(
                serde_json::from_value(json!(forged)).unwrap(),
            )
            .unwrap_err();
            assert_eq!(native_error, error);
            assert_value(&package, id, "a", true, &positive);
        }
        let loaded = call(json!({"method":"package/import","files":files}));
        assert_eq!(loaded["package"], json!(package));
        let native = intersect_text::distribution::import_files(
            serde_json::from_value(json!(files)).unwrap(),
        )
        .unwrap();
        assert_eq!(json!(native.package), json!(package));
    }
}

#[test]
fn rejected_partial_and_surrogate_inputs_roll_back_existing_model_and_keep_limits() {
    let f = fixture();
    for id in ["multichar", "astral_literal"] {
        let package = compile(case(&f, id), Profile::NativeV59);
        let valid = [source(id, "a")];
        let mut store = evaluate_stored(&package, &valid, EvaluationLimits::native())
            .unwrap()
            .store;
        let previous = storage::export(&store).unwrap();
        for text in ["ab", "🌍", ""] {
            let failed = evaluate_into(
                &package,
                &[source(id, text)],
                EvaluationLimits::native(),
                &mut store,
            )
            .unwrap();
            assert!(!failed.diagnostics.is_empty());
            assert_eq!(storage::export(&store).unwrap(), previous);
        }
        for limit in [
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
        ] {
            let result = intersect_text::evaluate_with_limits(&package, &valid, limit);
            assert!(!result.diagnostics.is_empty());
            assert_eq!(result.model.elements.len(), 0);
        }
        assert_eq!(
            store.value(&previous.elements[0].id, "name").unwrap(),
            Some(json!("a"))
        );
    }
}

#[test]
fn ordinary_parameter_specialization_retains_v59_and_original_source_replay() {
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/native-cases.json"
    ))
    .unwrap();
    let c = &old["parameterized"];
    let package = compile(c, Profile::NativeV59);
    assert_eq!(
        package.grammar.as_ref().unwrap().profile,
        Profile::NativeV59
    );
    assert!(package.xtext_inference.is_none());
    assert!(
        package
            .sources
            .keys()
            .any(|s| s.starts_with("generated:xtext:"))
    );
    assert_eq!(
        json!(package.compilation.as_ref().unwrap()),
        c["compilation"]
    );
    assert_eq!(
        json!(intersect_text::recompile(&package).unwrap()),
        json!(package)
    );
    for text in c["positive"].as_array().unwrap() {
        assert!(
            evaluate(&package, &[source("parameter", text.as_str().unwrap())])
                .diagnostics
                .is_empty()
        );
    }
    for text in c["negative"].as_array().unwrap() {
        let result = evaluate(&package, &[source("parameter", text.as_str().unwrap())]);
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty());
    }
}
