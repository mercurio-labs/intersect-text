//! Additive X05 consumers. Original source graphs, profiles and failures remain.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy, Package};
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/native-cases.json"
    ))
    .unwrap()
}
// Decode only scalar-safe input/outcome fields. The exact upstream token payload
// can contain isolated UTF-16 surrogates and remains intact in observations.json.
#[derive(serde::Deserialize)]
struct OracleGeneration {
    exit_code: i64,
}
#[derive(serde::Deserialize)]
struct OracleRun {
    text: String,
    complete_single_value: Option<bool>,
    lexer_errors: Option<usize>,
    failure: Option<String>,
    timeout_seconds: Option<u64>,
}
#[derive(serde::Deserialize)]
struct OracleObservation {
    id: String,
    generation: Option<OracleGeneration>,
    generation_exit: Option<i64>,
    runtime: Vec<OracleRun>,
}
fn oracle() -> Vec<OracleObservation> {
    let mut original: Vec<OracleObservation> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/observations.json"
    ))
    .unwrap();
    let nested: Vec<OracleObservation> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/terminal-negation/nested-observations.json"
    ))
    .unwrap();
    original.extend(nested);
    original
}
fn cases(f: &Value) -> impl Iterator<Item = &Value> {
    f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .chain(f["nested_companions"].as_array().unwrap().iter())
}
fn case<'a>(f: &'a Value, id: &str) -> &'a Value {
    cases(f).find(|c| c["id"] == id).unwrap()
}
fn inputs(c: &Value) -> BundleCompilation {
    serde_json::from_value(c["compilation"].clone()).unwrap()
}
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn compile(c: &Value) -> Package {
    intersect_text::ordinary::compile_with_optional_source_policy(
        inputs(c),
        Profile::NativeV58,
        &[],
        None,
        None,
        Default::default(),
    )
    .unwrap()
}
fn supported(id: &str) -> bool {
    matches!(
        id,
        "literal"
            | "range"
            | "alternatives"
            | "nested_groups"
            | "fragment"
            | "fragment_alternatives"
            | "fragment_nested"
            | "committed_failure"
            | "negated_repeat"
            | "inherited_override"
            | "explicit_parent"
            | "inherited_program_override"
            | "committed_path"
            | "nested_negated"
            | "negated_fragment"
    )
}
fn source(text: &str) -> Source {
    Source {
        uri: "memory:terminal-negation/value.model".into(),
        text: text.into(),
    }
}
fn assert_value(package: &Package, text: &str, accepted: bool) {
    let source = source(text);
    let result = evaluate(package, std::slice::from_ref(&source));
    assert_eq!(
        result.diagnostics.is_empty(),
        accepted,
        "{text:?}: {:?}",
        result.diagnostics
    );
    let json_result = call(json!({"method":"evaluate","package":package,"sources":[source]}));
    let json_evaluation: intersect_text::Evaluation = serde_json::from_value(json_result).unwrap();
    assert_eq!(json_evaluation.model, result.model);
    assert_eq!(
        json!(json_evaluation.diagnostics),
        json!(result.diagnostics)
    );
    if accepted {
        assert_eq!(result.model.elements.len(), 1);
        let element = &result.model.elements[0];
        assert_eq!(element.id, "memory:terminal-negation/value.model#0");
        assert_eq!(element.class, "Model");
        assert_eq!(element.values["name"], text);
        assert_eq!(element.start, 0);
        assert_eq!(element.end, text.len());
        assert_eq!(&text[element.start..element.end], text);
    } else {
        assert!(
            result.model.elements.is_empty(),
            "partial model for {text:?}"
        );
    }
}

#[test]
fn original_six_v56_observations_remain_exact_and_only_opt_in_negation_advances() {
    let f = fixture();
    assert_eq!(f["original_before"].as_array().unwrap().len(), 6);
    for original in f["original_before"].as_array().unwrap() {
        assert_eq!(
            call(original["request"].clone()),
            original["result"],
            "{}",
            original["id"]
        );
        let mut opt_in = original["request"].clone();
        opt_in["xtext_profile"] = json!(Profile::NativeV58);
        let actual = call(opt_in);
        let id = original["id"].as_str().unwrap();
        assert_eq!(
            actual["ok"],
            matches!(
                id,
                "old_character_negation" | "range_negation" | "fragment_negation"
            )
        );
        if ["alternative_until", "fragment_until", "local_hidden"].contains(&id) {
            assert_eq!(actual, original["result"]);
        }
    }
    for number in 1..=57 {
        let old: Profile =
            serde_json::from_value(json!(format!("intersect.xtext.native.v{number}"))).unwrap();
        assert!(
            intersect_text::ordinary::compile(inputs(case(&f, "range")), old).is_err(),
            "widened {old:?}"
        );
    }
}

#[test]
fn exact_oracle_graphs_execute_model_values_spans_resources_and_source_replay() {
    let f = fixture();
    let observations = oracle();
    for c in cases(&f) {
        let id = c["id"].as_str().unwrap();
        if !supported(id) {
            continue;
        }
        let observed = observations.iter().find(|o| o.id == id).unwrap();
        assert_eq!(
            observed
                .generation
                .as_ref()
                .map(|g| g.exit_code)
                .or(observed.generation_exit)
                .unwrap(),
            0,
            "{id}"
        );
        let package = compile(c);
        assert_eq!(
            package.grammar.as_ref().unwrap().profile,
            Profile::NativeV58
        );
        assert_eq!(
            json!(package.compilation.as_ref().unwrap()),
            json!(inputs(c))
        );
        let transported = call(
            json!({"method":"compile_bundle","compilation":c["compilation"],"xtext_profile":Profile::NativeV58}),
        );
        assert_eq!(transported["ok"], true, "{id}: {transported}");
        assert_eq!(transported["package"], json!(package));
        let replay = intersect_text::recompile(&package).unwrap();
        assert_eq!(json!(replay), json!(package));
        let mut positives = Vec::new();
        for run in &observed.runtime {
            let text = run.text.as_str();
            if run.timeout_seconds.is_some() {
                // Timeout is inconclusive upstream execution, not oracle rejection.
                // Preserve it and separately require the native negative to terminate.
                assert_value(&package, text, false);
                continue;
            }
            let accepted = run.complete_single_value == Some(true)
                && run.lexer_errors == Some(0)
                && run.failure.is_none();
            assert_value(&package, text, accepted);
            assert_eq!(
                evaluate(&package, &[source(text)]).model,
                evaluate(&replay, &[source(text)]).model
            );
            if accepted {
                positives.push(text);
            }
        }
        if positives.len() >= 2 {
            let sources: Vec<_> = positives[..2]
                .iter()
                .enumerate()
                .map(|(index, text)| Source {
                    uri: format!("memory:{id}/{index}.model"),
                    text: (*text).into(),
                })
                .collect();
            let result = evaluate(&package, &sources);
            assert!(
                result.diagnostics.is_empty(),
                "{id}: {:?}",
                result.diagnostics
            );
            assert_eq!(result.model.elements.len(), 2);
            for (index, element) in result.model.elements.iter().enumerate() {
                assert_eq!(element.id, format!("memory:{id}/{index}.model#0"));
                assert_eq!(element.values["name"], positives[index]);
                assert_eq!(element.end, positives[index].len());
            }
        }
    }
}

#[test]
fn unproven_domains_reject_and_keep_successful_upstream_stages_as_disagreements() {
    let f = fixture();
    let observations = oracle();
    for c in cases(&f) {
        let id = c["id"].as_str().unwrap();
        if supported(id) {
            continue;
        }
        let errors = intersect_text::ordinary::compile(inputs(c), Profile::NativeV58).unwrap_err();
        assert!(!errors.is_empty(), "{id}");
        let actual = call(
            json!({"method":"compile_bundle","compilation":c["compilation"],"xtext_profile":Profile::NativeV58}),
        );
        assert_eq!(actual["ok"], false, "{id}: {actual}");
        assert_eq!(actual["diagnostics"], json!(errors));
        if ["multichar", "astral_literal"].contains(&id) {
            let observed = observations.iter().find(|o| o.id == id).unwrap();
            assert_eq!(
                observed
                    .generation
                    .as_ref()
                    .map(|g| g.exit_code)
                    .or(observed.generation_exit)
                    .unwrap(),
                0
            );
            assert!(
                observed
                    .runtime
                    .iter()
                    .any(|r| r.complete_single_value == Some(true))
            );
            assert!(
                errors.iter().any(|e| e.message.contains("one UTF-16 unit")),
                "{errors:?}"
            );
        }
    }
}

#[test]
fn inherited_operands_and_explicit_parent_calls_retain_distinct_original_identities() {
    let f = fixture();
    for id in [
        "inherited_override",
        "inherited_program_override",
        "explicit_parent",
    ] {
        let c = case(&f, id);
        let package = compile(c);
        let grammar = package.grammar.as_ref().unwrap();
        let value = &grammar.terminals["VALUE"];
        let retained = json!(value.body);
        let operand = &retained["kind"]["Negated"]["operand"];
        assert!(operand["kind"]["Call"]["body"].is_object());
        if id == "explicit_parent" {
            assert_eq!(
                operand["kind"]["Call"]["name"],
                "@audit.NegationBaseExplicit::LETTER"
            );
            assert_eq!(
                grammar.terminals["@audit.NegationBaseExplicit::LETTER"].identity,
                "audit.NegationBaseExplicit::LETTER"
            );
            assert_value(&package, "a", false);
            assert_value(&package, "b", true);
        } else {
            assert_eq!(operand["kind"]["Call"]["name"], "LETTER");
            assert_value(&package, "a", true);
            assert_value(&package, "b", false);
        }
        let span = &grammar.declaration_sources["VALUE"];
        let original = package
            .compilation
            .as_ref()
            .unwrap()
            .bundle
            .source(&span.source)
            .unwrap();
        assert!(original.text[span.start..span.end].starts_with("terminal VALUE"));
        assert!(
            value
                .identity
                .starts_with(if id == "inherited_program_override" {
                    "audit.NegationProgramBase::"
                } else {
                    "audit.NegationChild"
                })
        );
    }
}

#[test]
fn original_compiler_and_evaluation_limits_reject_without_partial_models() {
    let f = fixture();
    let c = case(&f, "negated_repeat");
    let package = compile(c);
    let source = source("ABC");
    let baseline = evaluate(&package, std::slice::from_ref(&source));
    assert!(baseline.diagnostics.is_empty());
    for limits in [
        EvaluationLimits {
            resources: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            input_bytes: Some(2),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            tokens_per_resource: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            elements_per_resource: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            steps_per_resource: Some(0),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            nesting: 0,
            ..EvaluationLimits::native()
        },
    ] {
        let failed = evaluate_with_limits(&package, std::slice::from_ref(&source), limits);
        assert!(!failed.diagnostics.is_empty(), "{limits:?}");
        assert!(failed.model.elements.is_empty());
    }
    let compilation = inputs(c);
    for policy in [
        CompilationSourcePolicy {
            xtext_bytes: 1,
            ..Default::default()
        },
        CompilationSourcePolicy {
            xtext_declarations: 1,
            ..Default::default()
        },
    ] {
        assert!(
            intersect_text::ordinary::compile_with_source_policy(
                compilation.clone(),
                Profile::NativeV58,
                policy,
                Default::default()
            )
            .is_err()
        );
    }
    let mut nested = inputs(case(&f, "range"));
    let old = nested.xtext.clone().unwrap();
    let selected = nested
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == old)
        .unwrap();
    let text = selected.text.replace(
        "!('a'..'z')",
        &format!("!{}'a'{}", "(".repeat(65), ")".repeat(65)),
    );
    let dependencies = selected.dependencies.clone();
    *selected = DefinitionSource::new(&old.uri, SourceKind::Xtext, text);
    selected.dependencies = dependencies;
    nested.xtext = Some(selected.id.clone());
    let errors = intersect_text::ordinary::compile(nested, Profile::NativeV58).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("nesting budget exceeded")),
        "{errors:?}"
    );
    assert_eq!(evaluate(&package, &[source]).model, baseline.model);
}

#[test]
fn ordinary_parameter_specialization_keeps_v58_and_archive_rebuild_uses_original_sources() {
    let f = fixture();
    let c = &f["parameterized"];
    let package = compile(c);
    assert_eq!(
        package.grammar.as_ref().unwrap().profile,
        Profile::NativeV58
    );
    assert!(package.xtext_inference.is_none());
    assert!(
        package
            .sources
            .keys()
            .any(|k| k.starts_with("generated:xtext:"))
    );
    assert_eq!(
        json!(package.compilation.as_ref().unwrap()),
        json!(inputs(c))
    );
    assert_eq!(
        json!(intersect_text::recompile(&package).unwrap()),
        json!(package)
    );
    for text in c["positive"].as_array().unwrap() {
        assert_value(&package, text.as_str().unwrap(), true);
    }
    for text in c["negative"].as_array().unwrap() {
        assert_value(&package, text.as_str().unwrap(), false);
    }
    assert!(intersect_text::ordinary::compile(inputs(c), Profile::NativeV57).is_err());
    let exported = call(
        json!({"method":"package/export","compilation":c["compilation"],"xtext_profile":Profile::NativeV58,"metadata":{"package_id":"terminal.negation","release":"1","display_name":"Terminal negation","description":"Authored X05 consumer"}}),
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
        assert!(negated.is_object());
        if field == "excluded" {
            negated["excluded"] = json!([]);
        } else {
            negated["operand"]["kind"] = json!({"Literal":[98]});
        }
        assert_eq!(
            cached["compilation"],
            json!(package.compilation.as_ref().unwrap())
        );
        let text = cached.to_string();
        let mut manifest: Value =
            serde_json::from_str(forged["manifest.json"].as_str().unwrap()).unwrap();
        manifest["files"]["package.json"] = json!({"bytes":text.len(),"sha256":intersect_model::definitions::bundle::SourceId::for_text("irrelevant",&text).sha256});
        forged.insert("package.json".into(), json!(text));
        forged.insert("manifest.json".into(), json!(manifest.to_string()));
        let error =
            intersect_web::dispatch(&json!({"method":"package/import","files":forged}).to_string())
                .unwrap_err();
        assert!(
            error.contains("Archived compiled package differs from freshly compiled definitions"),
            "{field}: {error}"
        );
        let native_files = serde_json::from_value(json!(forged)).unwrap();
        let native_error = intersect_text::distribution::import_files(native_files).unwrap_err();
        assert_eq!(native_error, error);
        assert_value(&package, "A", true);
    }
    let loaded = call(json!({"method":"package/import","files":files}));
    assert_eq!(loaded["package"], json!(package));
    assert_value(
        &serde_json::from_value(loaded["package"].clone()).unwrap(),
        "A",
        true,
    );
}

#[test]
fn selected_token_failure_never_falls_back_and_lexical_work_keeps_original_fees() {
    use intersect_text::grammar;
    use intersect_text::grammar::LexicalAlternative;
    let f = fixture();
    let package = compile(case(&f, "committed_path"));
    let grammar = package.grammar.as_ref().unwrap();
    for text in ["ad", "ac"] {
        let decision = grammar.lexical_prediction.as_ref().unwrap();
        let selected = decision.predict(text, &mut None).unwrap();
        assert!(
            matches!(decision.alternative(selected).unwrap(),LexicalAlternative::Terminal(name) if name=="VALUE")
        );
        let rejected = grammar::lex_borrowed_with_prediction(
            text,
            None,
            None,
            Some(&grammar.terminals),
            grammar.hidden.as_deref(),
            Some(decision),
            &mut None,
        )
        .unwrap_err();
        assert_eq!(rejected.0, 0);
        assert!(
            rejected
                .1
                .contains("Incomplete or invalid terminal audit.CommittedPath::VALUE"),
            "{rejected:?}"
        );
        let host = call(json!({"method":"evaluate","package":package,"sources":[source(text)]}));
        assert!(host["diagnostics"].as_array().unwrap().iter().any(|d| {
            d["message"]
                .as_str()
                .unwrap()
                .contains("Incomplete or invalid terminal audit.CommittedPath::VALUE")
        }));
    }
    let mut remaining = Some(10000);
    let tokens = grammar::lex_borrowed_with_prediction(
        "adc",
        None,
        None,
        Some(&grammar.terminals),
        grammar.hidden.as_deref(),
        grammar.lexical_prediction.as_ref(),
        &mut remaining,
    )
    .unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].text, "adc");
    let used = 10000 - remaining.unwrap();
    assert!(used > 0);
    for budget in 0..used {
        let error = grammar::lex_borrowed_with_prediction(
            "adc",
            None,
            None,
            Some(&grammar.terminals),
            grammar.hidden.as_deref(),
            grammar.lexical_prediction.as_ref(),
            &mut Some(budget),
        )
        .unwrap_err();
        assert!(
            error.1.contains("step budget exceeded"),
            "{budget}: {error:?}"
        );
    }
    assert!(
        grammar::lex_borrowed_with_prediction(
            "adc",
            None,
            None,
            Some(&grammar.terminals),
            grammar.hidden.as_deref(),
            grammar.lexical_prediction.as_ref(),
            &mut Some(used)
        )
        .is_ok()
    );
    let old =
        intersect_text::ordinary::compile(inputs(case(&f, "literal")), Profile::NativeV57).unwrap();
    let new = compile(case(&f, "literal"));
    for text in ["b", "\n"] {
        let mut costs = Vec::new();
        for package in [&old, &new] {
            let grammar = package.grammar.as_ref().unwrap();
            let mut remaining = Some(10000);
            grammar::lex_borrowed_with_prediction(
                text,
                None,
                None,
                Some(&grammar.terminals),
                grammar.hidden.as_deref(),
                grammar.lexical_prediction.as_ref(),
                &mut remaining,
            )
            .unwrap();
            costs.push(remaining.unwrap());
        }
        assert_eq!(
            costs[0], costs[1],
            "changed lexical logical fee for {text:?}"
        );
    }
}
