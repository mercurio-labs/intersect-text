//! Source-loaded full-generation controls; recovered ANTLR artifacts stay distinct.
use intersect_model::core::storage;
use intersect_model::core::storage::ModelRead;
use intersect_model::definitions::bundle::SourceId;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_into, evaluate_stored};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/native-cases.json"
    ))
    .unwrap()
}
fn positives() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/positive-resources.json"
    ))
    .unwrap()
}
#[derive(serde::Deserialize)]
struct Accepted {
    accepted: bool,
}
#[derive(serde::Deserialize)]
struct Observation {
    resource: Accepted,
    parser: Accepted,
}
#[derive(serde::Deserialize)]
struct Run {
    text: String,
    observation: Observation,
}
#[derive(serde::Deserialize)]
struct Oracle {
    id: String,
    runtime: Vec<Run>,
}
fn oracle() -> Vec<Oracle> {
    // Unselected raw token/partial strings include isolated UTF16 units; keep
    // their bytes intact, decoding only scalar-safe request/acceptance fields.
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/observations.json"
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
fn compile(c: &Value) -> Package {
    intersect_text::ordinary::compile_with_optional_source_policy(
        inputs(c),
        Profile::NativeV62,
        &[],
        None,
        None,
        Default::default(),
    )
    .unwrap()
}
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap_or_else(|error| json!({"error":error}))
}
fn source(id: &str, text: &str) -> Source {
    Source {
        uri: format!("memory:grouped-negation/{id}.model"),
        text: text.into(),
    }
}

#[test]
fn every_frozen_legacy_request_remains_exact() {
    let full: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/original7-before-v61.json"
    ))
    .unwrap();
    let rows = full["transcript"].as_array().unwrap();
    assert_eq!(rows.len(), 181);
    for row in rows {
        assert_eq!(call(row["request"].clone()), row["result"]);
    }
    let old: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/all-legacy-before-v61.json"
    ))
    .unwrap();
    assert_eq!(old.as_array().unwrap().len(), 305);
    for row in old.as_array().unwrap() {
        assert_eq!(
            call(row["request"].clone()),
            row["result"],
            "{} v{}",
            row["id"],
            row["version"]
        );
    }
}

#[test]
fn default_inference_retains_its_complete_original_host_results() {
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/contextual-first-rule/native/inferred-before-v60.json"
    ))
    .unwrap();
    fn replay(value: &Value) -> usize {
        if let (Some(request), Some(result)) = (value.get("request"), value.get("result")) {
            assert_eq!(call(request.clone()), *result);
            return 1;
        }
        match value {
            Value::Object(items) => items.values().map(replay).sum(),
            Value::Array(items) => items.iter().map(replay).sum(),
            _ => 0,
        }
    }
    assert_eq!(replay(&before), 4);
}

#[test]
fn ordinary_parameter_specialization_keeps_v62_and_original_source_replay() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/native-cases.json"
    ))
    .unwrap();
    let mut compilation: BundleCompilation =
        serde_json::from_value(f["parameterized"]["compilation"].clone()).unwrap();
    let entry = compilation.xtext.clone().unwrap();
    let authored = compilation
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == entry)
        .unwrap();
    assert_eq!(authored.text.matches("!('a'..'z')").count(), 1);
    authored.text = authored.text.replace("!('a'..'z')", "!('ab'|'x')");
    authored.id = SourceId::for_text(
        "memory:grouped-negation-parameterization/model.xtext",
        &authored.text,
    );
    compilation.xtext = Some(authored.id.clone());
    let package =
        intersect_text::ordinary::compile(compilation.clone(), Profile::NativeV62).unwrap();
    assert_eq!(
        package.grammar.as_ref().unwrap().profile,
        Profile::NativeV62
    );
    assert_eq!(json!(package.compilation), json!(compilation));
    assert_eq!(
        json!(intersect_text::recompile(&package).unwrap()),
        json!(package)
    );
    for text in ["a", "c"] {
        assert!(
            evaluate(&package, &[source("parameterized", text)])
                .diagnostics
                .is_empty()
        );
    }
    for text in ["x", "ab", "off"] {
        assert!(
            !evaluate(&package, &[source("parameterized", text)])
                .diagnostics
                .is_empty()
        );
    }
}

#[test]
fn clean_full_generated_graphs_drive_native_json_and_stored_values() {
    let f = fixture();
    let positives = positives();
    let oracle = oracle();
    assert_eq!(oracle.iter().map(|r| r.runtime.len()).sum::<usize>(), 40);
    let mut observed = 0;
    let mut accepted = 0;
    for id in ["grouped_multichar", "fragment_multichar", "mixed_group"] {
        let c = case(&f, id);
        let package = compile(c);
        assert_eq!(
            package.grammar.as_ref().unwrap().profile,
            Profile::NativeV62
        );
        assert_eq!(json!(package.compilation), c["compilation"]);
        assert_eq!(
            json!(intersect_text::recompile(&package).unwrap()),
            json!(package)
        );
        assert_eq!(
            call(
                json!({"method":"compile_bundle","compilation":c["compilation"],"xtext_profile":Profile::NativeV62})
            )["package"],
            json!(package)
        );
        let grammar = package.grammar.as_ref().unwrap();
        let terminal = json!(grammar.terminals["VALUE"]);
        let negated = &terminal["body"]["kind"]["Negated"];
        assert_eq!(
            negated["excluded"],
            if id == "mixed_group" {
                json!([[120, 120]])
            } else {
                json!([])
            }
        );
        if id == "fragment_multichar" {
            assert_eq!(negated["operand"]["kind"]["Call"]["name"], "PAIR");
            assert_eq!(
                negated["operand"]["kind"]["Call"]["body"]["kind"]["Literal"],
                json!([97, 98])
            );
            assert_eq!(
                grammar.terminals["PAIR"].identity,
                "audit.FragmentMultiNegation::PAIR"
            );
        } else {
            assert!(negated["operand"]["kind"]["Sequence"].is_array());
        }
        let span = &grammar.declaration_sources["VALUE"];
        let original = inputs(c);
        let authored = original
            .bundle
            .sources
            .iter()
            .find(|s| s.id == span.source)
            .unwrap();
        assert!(authored.text[span.start..span.end].starts_with("terminal VALUE"));
        for run in &oracle.iter().find(|r| r.id == id).unwrap().runtime {
            observed += 1;
            assert_eq!(
                run.observation.resource.accepted,
                run.observation.parser.accepted
            );
            let sources = [source(id, &run.text)];
            let stored = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
            let direct = evaluate(&package, &sources);
            assert_eq!(
                json!(direct),
                call(json!({"method":"evaluate","package":package,"sources":sources}))
            );
            assert_eq!(storage::export(&stored.store).unwrap(), direct.model);
            assert_eq!(
                direct.diagnostics.is_empty(),
                run.observation.resource.accepted,
                "{id} {:?}",
                run.text
            );
            if run.observation.resource.accepted {
                accepted += 1;
                let expected = positives
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p["id"] == id && p["text"] == run.text)
                    .unwrap();
                assert_eq!(direct.model.elements.len(), 1);
                let element = &direct.model.elements[0];
                assert_eq!(element.class, "Model");
                assert_eq!(element.resource, sources[0].uri);
                assert_eq!(element.start, 0);
                assert_eq!(element.end, run.text.len());
                assert_eq!(
                    expected["resource"]["model"][0]["class"],
                    "urn:intersect:xtext:mini-kerml:terminal-spellings:1#//Model"
                );
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
                        Some(value.clone())
                    );
                }
                assert_eq!(expected["resource"]["leaves"][0]["utf16_start"], 0);
                assert_eq!(
                    expected["resource"]["leaves"][0]["utf16_end"],
                    run.text.encode_utf16().count()
                );
            } else {
                assert!(
                    direct.model.elements.is_empty(),
                    "Do not publish an upstream rejected partial model"
                );
            }
        }
    }
    assert_eq!((observed, accepted), (27, 11));
}

fn companion(c: &Value, id: &str, operand: &str, declarations: &str) -> BundleCompilation {
    let mut compilation = inputs(c);
    let source = compilation
        .bundle
        .sources
        .iter_mut()
        .find(|s| Some(&s.id) == compilation.xtext.as_ref())
        .unwrap();
    assert_eq!(source.text.matches("!('ab')").count(), 1);
    source.text = source.text.replace("!('ab')", operand);
    source.text.push_str(declarations);
    source.id = SourceId::for_text(
        format!("memory:grouped-negation-negative/{id}.xtext"),
        &source.text,
    );
    compilation.xtext = Some(source.id.clone());
    compilation
}

#[test]
fn recovered_generator_cases_and_unqualified_operand_shapes_stay_explicit() {
    let f = fixture();
    for (id, message) in [
        ("original_sequence", "General multi-element"),
        ("fragment_mixed", "literal-only fragment chains"),
    ] {
        let errors = intersect_text::ordinary::compile(inputs(case(&f, id)), Profile::NativeV62)
            .unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains(message)),
            "{errors:?}"
        );
    }
    let base = case(&f, "grouped_multichar");
    for (id, operand, extra, message) in [
        (
            "unsupported_suffix",
            "!('ab' EOF)",
            "",
            "General multi-element",
        ),
        ("repeated", "!('ab'?)", "", "cardinality one"),
        ("eof", "!EOF", "", "cannot contain EOF"),
        (
            "nonfragment",
            "!PAIR",
            "\nterminal PAIR: 'ab';\n",
            "literal-only fragment chains",
        ),
        (
            "grouped_callee",
            "!PAIR",
            "\nterminal fragment PAIR: ('ab');\n",
            "literal-only fragment chains",
        ),
        (
            "new_negated_callee",
            "!PAIR",
            "\nterminal fragment PAIR: !('ab'|'x');\n",
            "literal-only fragment chains",
        ),
        (
            "call_then_eof",
            "!(PAIR|EOF)",
            "\nterminal fragment PAIR: 'ab';\n",
            "cannot contain EOF",
        ),
        ("unknown", "!MISSING", "", "Unknown terminal call"),
        (
            "cycle",
            "!PAIR",
            "\nterminal fragment PAIR: OTHER;\nterminal fragment OTHER: PAIR;\n",
            "Cyclic terminal call",
        ),
    ] {
        let inputs = companion(base, id, operand, extra);
        let errors = intersect_text::ordinary::compile(inputs, Profile::NativeV62).unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains(message)),
            "{id}: {errors:?}"
        );
    }
}

#[test]
fn effective_source_links_admit_the_exact_clean_inherited_parent_program() {
    // This exact request was accepted by the initial private candidate. Retained
    // declaration IDs alone cannot authorize execution in a child grammar.
    let request: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/inherited-parent-before-correction.json"
    ))
    .unwrap();
    // Attempt4's exact rejection and earlier incorrect admission are frozen
    // separately. This graph now has its own clean full-source oracle replay.
    let admitted = call(request.clone());
    assert_eq!(admitted["ok"], true, "{admitted}");
    let child_package: Package = serde_json::from_value(admitted["package"].clone()).unwrap();
    assert!(
        evaluate(&child_package, &[source("qualified_inherited", "x")])
            .diagnostics
            .is_empty()
    );

    // The exact standalone parent remains independently source-authoritative;
    // admitting the child must not replace the parent's original declarations.
    let mut parent: BundleCompilation =
        serde_json::from_value(request["compilation"].clone()).unwrap();
    let child = parent.xtext.take().unwrap();
    parent.bundle.sources.retain(|s| s.id != child);
    let base = parent
        .bundle
        .sources
        .iter()
        .find(|s| s.id.uri.ends_with("/base.xtext"))
        .unwrap()
        .id
        .clone();
    parent.xtext = Some(base);
    let package = intersect_text::ordinary::compile(parent, Profile::NativeV62).unwrap();
    let result = evaluate(&package, &[source("qualified_parent_standalone", "a")]);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[0].values["name"], json!("a"));
}

#[test]
fn source_rebuilt_archives_reject_rehashed_exclusions_operands_and_call_identity() {
    let f = fixture();
    for id in ["grouped_multichar", "fragment_multichar", "mixed_group"] {
        let c = case(&f, id);
        let package = compile(c);
        let exported = call(
            json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV62,"metadata":{"package_id":"grouped.negation","release":"1","display_name":"Grouped negation","description":"Full source-generated oracle consumer"}}),
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
        for field in ["excluded", "operand", "call_identity"] {
            if field == "call_identity" && id != "fragment_multichar" {
                continue;
            }
            let mut forged = files.clone();
            let mut cached: Value =
                serde_json::from_str(forged["package.json"].as_str().unwrap()).unwrap();
            let node = &mut cached["grammar"]["terminals"]["VALUE"]["body"]["kind"]["Negated"];
            match field {
                "excluded" => node["excluded"] = json!([[99, 99]]),
                "operand" => node["operand"]["kind"] = json!({"Literal":[99,100]}),
                _ => node["operand"]["kind"]["Call"]["name"] = json!("FORGED"),
            }
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
        assert!(
            evaluate(&package, &[source(id, "a")])
                .diagnostics
                .is_empty()
        );
    }
}

#[test]
fn failed_partial_or_surrogate_inputs_preserve_published_model_and_host_limits() {
    let f = fixture();
    for id in ["grouped_multichar", "fragment_multichar", "mixed_group"] {
        let package = compile(case(&f, id));
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
        ] {
            let failed = intersect_text::evaluate_with_limits(&package, &valid, limits);
            assert!(!failed.diagnostics.is_empty());
            assert!(failed.model.elements.is_empty());
        }
        assert_eq!(
            store.value(&previous.elements[0].id, "name").unwrap(),
            Some(json!("a"))
        );
    }
}

#[test]
fn original_nested_one_unit_programs_keep_their_exact_source_and_evaluation() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/native-cases.json"
    ))
    .unwrap();
    let companions = original["nested_companions"].as_array().unwrap();
    assert_eq!(companions.len(), 2);
    for c in companions {
        let old = intersect_text::ordinary::compile(inputs(c), Profile::NativeV61).unwrap();
        let new = compile(c);
        assert_eq!(json!(old.compilation), json!(new.compilation));
        for text in ["a", "b", "c"] {
            let sources = [source(c["id"].as_str().unwrap(), text)];
            let old_result = evaluate(&old, &sources);
            let new_result = evaluate(&new, &sources);
            assert_eq!(json!(new_result), json!(old_result));
            assert_eq!(new_result.diagnostics.is_empty(), text == "a");
        }
    }
}

fn expanded_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/expanded-native-cases.json"
    ))
    .unwrap()
}

fn expanded_oracle() -> Vec<Oracle> {
    let mut rows: Vec<Oracle> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/combinations-record-observations.json"
    ))
    .unwrap();
    rows.extend(serde_json::from_str::<Vec<Oracle>>(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/exact-negatives-record-observations.json"
    ))
    .unwrap());
    rows
}

fn retained_call_sources(value: &Value, package: &Package, metadata: &Value) -> usize {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|v| retained_call_sources(v, package, metadata))
            .sum(),
        Value::Object(values) => {
            let mut count = 0;
            if let Some(call) = values.get("Call") {
                let name = call["name"].as_str().unwrap();
                let target = &package.grammar.as_ref().unwrap().terminals[name];
                assert!(target.fragment);
                let (grammar, rule) = target.identity.rsplit_once("::").unwrap();
                let compilation = package.compilation.as_ref().unwrap();
                let authored: Vec<_> = compilation
                    .bundle
                    .sources
                    .iter()
                    .filter(|s| {
                        s.text.lines().next().is_some_and(|line| {
                            line.split_whitespace().next() == Some("grammar")
                                && line.split_whitespace().nth(1) == Some(grammar)
                        })
                    })
                    .collect();
                assert_eq!(authored.len(), 1, "{}", target.identity);
                let source = authored[0];
                assert_eq!(source.id, SourceId::for_text(&source.id.uri, &source.text));
                let declaration = &source.text[target.start..target.end];
                assert!(declaration.starts_with("terminal fragment "));
                assert!(declaration.contains(rule));
                assert!(
                    metadata["linking"].as_array().unwrap().iter().any(|s| {
                        s["calls"].as_array().unwrap().iter().any(|c| {
                            c["target"] == target.identity
                                && (!name.starts_with('@') || c["explicit"] == true)
                        })
                    }),
                    "No exact source-linked oracle target for {name}: {}",
                    target.identity
                );
                assert!(call["body"].is_object());
                count += 1;
            }
            count
                + values
                    .values()
                    .map(|v| retained_call_sources(v, package, metadata))
                    .sum::<usize>()
        }
        _ => 0,
    }
}

#[test]
fn exact_clean_compositions_and_original_negative_sources_execute_complete_models() {
    let f = expanded_fixture();
    let oracle = expanded_oracle();
    let positives: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/expanded-positive-resources.json"
    ))
    .unwrap();
    let metadata: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/grouped-terminal-negation/expanded-source-metadata.json"
    ))
    .unwrap();
    assert_eq!(f["cases"].as_array().unwrap().len(), 10);
    let (mut inputs_seen, mut accepted, mut calls) = (0, 0, 0);
    for c in f["cases"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let package = compile(c);
        assert_eq!(json!(package.compilation), c["compilation"]);
        assert_eq!(
            json!(intersect_text::recompile(&package).unwrap()),
            json!(package)
        );
        assert_eq!(
            call(
                json!({"method":"compile_bundle","compilation":c["compilation"],"xtext_profile":Profile::NativeV62})
            )["package"],
            json!(package)
        );
        let grammar = package.grammar.as_ref().unwrap();
        let terminal = json!(grammar.terminals["VALUE"]);
        let operand = &terminal["body"]["kind"]["Negated"]["operand"];
        let expected_metadata = &metadata
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap()["metadata"];
        calls += retained_call_sources(operand, &package, expected_metadata);
        let span = &grammar.declaration_sources["VALUE"];
        let authored = package
            .compilation
            .as_ref()
            .unwrap()
            .bundle
            .source(&span.source)
            .unwrap();
        assert!(authored.text[span.start..span.end].starts_with("terminal VALUE"));
        if id == "explicit_parent_multi" {
            assert_eq!(
                operand["kind"]["Call"]["name"],
                "@audit.ExplicitMultiBase::PAIR"
            );
            assert_eq!(
                grammar.terminals["@audit.ExplicitMultiBase::PAIR"].identity,
                "audit.ExplicitMultiBase::PAIR"
            );
            assert_eq!(
                grammar.terminals["PAIR"].identity,
                "audit.ExplicitMultiChild::PAIR"
            );
        } else if id == "inherited_program_override" {
            assert_eq!(
                grammar.terminals["VALUE"].identity,
                "audit.MultiProgramBase::VALUE"
            );
            assert_eq!(
                grammar.terminals["PAIR"].identity,
                "audit.MultiProgramChild::PAIR"
            );
            assert_eq!(
                operand["kind"]["Call"]["body"]["kind"]["Literal"],
                json!([120])
            );
        } else if id == "inherited_multi_fragment" {
            assert_eq!(
                grammar.terminals["PAIR"].identity,
                "audit.MultiFragmentBase::PAIR"
            );
        }
        let runs = &oracle.iter().find(|r| r.id == id).unwrap().runtime;
        let valid = runs
            .iter()
            .find(|r| r.observation.resource.accepted)
            .unwrap();
        let mut published = evaluate_stored(
            &package,
            &[source(id, &valid.text)],
            EvaluationLimits::native(),
        )
        .unwrap()
        .store;
        let previous = storage::export(&published).unwrap();
        for run in runs {
            inputs_seen += 1;
            let sources = [source(id, &run.text)];
            let direct = evaluate(&package, &sources);
            let stored = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
            assert_eq!(storage::export(&stored.store).unwrap(), direct.model);
            assert_eq!(
                json!(direct),
                call(json!({"method":"evaluate","package":package,"sources":sources}))
            );
            assert_eq!(
                direct.diagnostics.is_empty(),
                run.observation.resource.accepted,
                "{id} {:?}",
                run.text
            );
            assert_eq!(
                run.observation.resource.accepted,
                run.observation.parser.accepted
            );
            if run.observation.resource.accepted {
                accepted += 1;
                let expected = positives
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["id"] == id && v["text"] == run.text)
                    .unwrap();
                assert_eq!(direct.model.elements.len(), 1);
                let element = &direct.model.elements[0];
                assert_eq!(element.id, format!("{}#0", sources[0].uri));
                assert_eq!(element.class, "Model");
                assert_eq!(element.resource, sources[0].uri);
                assert_eq!((element.start, element.end), (0, run.text.len()));
                assert_eq!(
                    package.schema.declarations["Model"].namespace,
                    "urn:intersect:xtext:mini-kerml:terminal-spellings:1"
                );
                assert_eq!(
                    expected["resource"]["model"][0]["class"],
                    "urn:intersect:xtext:mini-kerml:terminal-spellings:1#//Model"
                );
                for (feature, value) in expected["resource"]["model"][0]["values"]
                    .as_object()
                    .unwrap()
                {
                    assert_eq!(
                        stored.store.value(&element.id, feature).unwrap(),
                        Some(value.clone())
                    );
                }
                assert_eq!(expected["resource"]["leaves"][0]["utf16_start"], 0);
                assert_eq!(
                    expected["resource"]["leaves"][0]["utf16_end"],
                    run.text.encode_utf16().count()
                );
            } else {
                assert!(
                    direct.model.elements.is_empty(),
                    "Rejected upstream partials stay separate"
                );
                let failed = evaluate_into(
                    &package,
                    &sources,
                    EvaluationLimits::native(),
                    &mut published,
                )
                .unwrap();
                assert!(!failed.diagnostics.is_empty());
                assert_eq!(storage::export(&published).unwrap(), previous);
            }
        }
    }
    assert_eq!((inputs_seen, accepted), (90, 31));
    assert!(calls >= 10);
}

fn forge_first_call(value: &mut Value, body: bool) -> bool {
    match value {
        Value::Object(fields) => {
            if let Some(call) = fields.get_mut("Call") {
                if body {
                    call["body"]["kind"] = json!({"Literal":[99,100]});
                } else {
                    call["name"] = json!("FORGED");
                }
                return true;
            }
            fields.values_mut().any(|v| forge_first_call(v, body))
        }
        Value::Array(values) => values.iter_mut().any(|v| forge_first_call(v, body)),
        _ => false,
    }
}

#[test]
fn expanded_archives_rebuild_exact_sources_and_reject_nested_or_parent_authority_tampering() {
    let f = expanded_fixture();
    for c in f["cases"].as_array().unwrap() {
        let package = compile(c);
        let exported = call(
            json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV62,"metadata":{"package_id":"grouped.negation.expanded","release":"1","display_name":"Expanded negation","description":"Exact clean full-source compositions"}}),
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
        for field in ["excluded", "operand", "call_identity", "call_body"] {
            let mut forged = files.clone();
            let mut cached: Value =
                serde_json::from_str(forged["package.json"].as_str().unwrap()).unwrap();
            let node = &mut cached["grammar"]["terminals"]["VALUE"]["body"]["kind"]["Negated"];
            match field {
                "excluded" => node["excluded"] = json!([[99, 99]]),
                "operand" => node["operand"]["kind"] = json!({"Literal":[99,100]}),
                _ => {
                    if !forge_first_call(&mut node["operand"], field == "call_body") {
                        continue;
                    }
                }
            }
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
        assert_eq!(
            call(json!({"method":"package/import","files":files}))["package"],
            json!(package)
        );
    }
}
