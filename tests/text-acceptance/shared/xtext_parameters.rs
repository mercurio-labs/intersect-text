//! Source-bound parameter inference controls, shared by native and WASM probes.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::SourceLimits;
use intersect_text::inference::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(crate) fn input(case: &Value) -> InferenceRequest {
    let mut sources = vec![];
    let mut ids = BTreeMap::<String, SourceId>::new();
    for source in case["sources"].as_array().unwrap() {
        let name = source["id"].as_str().unwrap();
        let mut definition = DefinitionSource::new(
            format!(
                "memory:parameters/{}/{name}.xtext",
                case["id"].as_str().unwrap()
            ),
            SourceKind::Xtext,
            source["source"].as_str().unwrap(),
        );
        for dependency in source["dependencies"].as_array().unwrap() {
            definition
                .dependencies
                .push(ids[dependency.as_str().unwrap()].clone());
        }
        ids.insert(name.into(), definition.id.clone());
        sources.push(definition);
    }
    InferenceRequest {
        version: 1,
        grammar: ids[case["entry"].as_str().unwrap()].clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources,
            namespaces: vec![],
        },
    }
}

pub fn frozen_inference_matrix() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameters/cases.json"
    ))
    .unwrap();
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameters/observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    let mut counts = [0, 0];
    for variant in [
        "original_obstructions",
        "lowercase_counterparts",
        "additional_controls",
    ] {
        for (case, row) in cases[variant]
            .as_array()
            .unwrap()
            .iter()
            .zip(rows[variant]["metadata"].as_array().unwrap())
        {
            let label = format!("{variant}/{}", case["id"]);
            let request = input(case);
            let actual =
                infer_metamodels(&request.bundle, &request.grammar, SourceLimits::default());
            match actual {
                Ok(report) => {
                    if row["all_sources_accepted"] != true {
                        failures.push(format!("{label}: incorrectly accepted"));
                        continue;
                    }
                    counts[0] += 1;
                    assert_eq!(
                        report.bundle.sources[..request.bundle.sources.len()],
                        request.bundle.sources
                    );
                    assert_eq!(
                        report.grammars.len(),
                        row["sources"].as_array().unwrap().len()
                    );
                    for (grammar, expected) in report
                        .grammars
                        .iter()
                        .zip(row["sources"].as_array().unwrap())
                    {
                        let packages = expected["metamodels"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|p| p["generated"] == true)
                            .map(|p| json!({"alias":p["alias"],"package":p["package"]}))
                            .collect::<Vec<_>>();
                        let expected = json!({"generated_packages":packages});
                        let shape = super::controls::projection(&grammar.metamodels);
                        let oracle = super::controls::expected(&expected);
                        if shape != oracle {
                            failures.push(format!(
                                "{label}: structure differs\nactual {shape}\nexpected {oracle}"
                            ));
                        }
                        super::controls::materialized_projection(
                            &report.schema,
                            &grammar.metamodels,
                            &grammar.generated_sources,
                            &expected,
                        );
                        for binding in grammar.rules.values() {
                            assert_eq!(binding.declaration.source, grammar.source);
                            for parameter in &binding.parameters {
                                let original = report.bundle.source(&grammar.source).unwrap();
                                assert_eq!(
                                    &original.text[parameter.start..parameter.end],
                                    parameter.value
                                );
                            }
                        }
                    }
                }
                Err(error) => {
                    if row["all_sources_accepted"] == true {
                        failures.push(format!("{label}: rejected: {}", error.message));
                    } else {
                        counts[1] += 1;
                        assert!(
                            error.origin.is_some(),
                            "{label}: missing native source ownership"
                        );
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(counts, [48, 26]);
}

pub fn verify() -> usize {
    frozen_inference_matrix();
    frozen_runtime_matrix();
    empty_resources_and_forwarding();
    3
}

pub(crate) fn check_resource(
    result: &intersect_text::Evaluation,
    uri: &str,
    text: &str,
    parsed: bool,
    roots: usize,
) {
    use intersect_model::core::resources::ResourceLoadState;
    assert_eq!(result.model.resources.len(), 1);
    let resource = result.model.resources.get(uri).unwrap();
    assert_eq!(resource.uri, uri);
    assert_eq!(resource.load_state, ResourceLoadState::Loaded);
    assert_eq!(resource.roots.len(), roots);
    if parsed {
        assert!(resource.diagnostics.is_empty());
        let correspondence = resource.source.as_ref().unwrap();
        assert_eq!(correspondence.uri, uri);
        assert_eq!(
            correspondence.revision,
            SourceId::for_text(uri, text).sha256
        );
    } else {
        assert!(resource.source.is_none());
        assert!(!resource.diagnostics.is_empty());
    }
}

pub fn frozen_runtime_matrix() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameters/cases.json"
    ))
    .unwrap();
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameters/observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    let mut counts = [0, 0, 0];
    for variant in [
        "original_obstructions",
        "lowercase_counterparts",
        "additional_controls",
    ] {
        for (case, metadata) in cases[variant]
            .as_array()
            .unwrap()
            .iter()
            .zip(rows[variant]["metadata"].as_array().unwrap())
        {
            if metadata["all_sources_accepted"] != true {
                continue;
            }
            let label = format!("{variant}/{}", case["id"]);
            let request = input(case);
            let language = match request.compile(SourceLimits::default()) {
                Ok(language) => language,
                Err(errors) => {
                    failures.push(format!(
                        "{label}: executable compilation rejected: {errors:?}"
                    ));
                    continue;
                }
            };
            let serialized = serde_json::to_value(language.package()).unwrap();
            let restored =
                intersect_text::recompile(&serde_json::from_value(serialized.clone()).unwrap())
                    .unwrap();
            assert_eq!(
                serde_json::to_value(restored).unwrap(),
                serialized,
                "{label}: source replay differs"
            );
            let archived = super::archives::roundtrip(&language);
            let original = variant == "original_obstructions";
            // Original Java generators failed. These additional native executions
            // use explicitly identified namespace counterparts as semantic controls;
            // they never become original independent-parser observations.
            let reference = if original {
                "lowercase_counterparts"
            } else {
                variant
            };
            for expected in rows[reference]["runs"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["case"] == case["id"])
            {
                let mut package = archived.clone();
                if expected["entry"] != "resource" {
                    package.grammar.as_mut().unwrap().entry =
                        expected["entry"].as_str().unwrap().into();
                }
                let text = expected["text"].as_str().unwrap();
                let result = intersect_text::evaluate(
                    &package,
                    &[intersect_text::Source {
                        uri: "memory:parameter-model.test".into(),
                        text: text.into(),
                    }],
                );
                if result.diagnostics.is_empty() != (expected["accepted"] == true) {
                    failures.push(format!(
                        "{label} / {} / {text:?}: acceptance differs: {:?}",
                        expected["entry"], result.diagnostics
                    ));
                    continue;
                }
                if expected["entry"] == "resource" {
                    check_resource(
                        &result,
                        "memory:parameter-model.test",
                        text,
                        expected["accepted"] == true,
                        if expected["accepted"] == true {
                            expected["resource_contents"].as_u64().unwrap() as usize
                        } else {
                            0
                        },
                    );
                }
                if expected["accepted"] == true {
                    let actual =
                        super::fragment_runtime::model_projection(&package.schema, &result);
                    if actual != expected["model"] {
                        failures.push(format!(
                            "{label} / {text:?}: model differs\nactual {actual}\nexpected {}",
                            expected["model"]
                        ));
                    }
                } else if !result.model.elements.is_empty() {
                    failures.push(format!("{label}: rejected native model was not atomic"));
                }
                if original {
                    counts[2] += 1;
                } else if expected["accepted"] == true {
                    counts[0] += 1;
                } else {
                    counts[1] += 1;
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(counts, [30, 37, 58]);
}

pub fn empty_resources_and_forwarding() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameter-empty/cases.json"
    ))
    .unwrap();
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameter-empty/observations.json"
    ))
    .unwrap();
    let mut counts = [0, 0, 0];
    for case in cases {
        let request = input(&case);
        let language = request.compile(SourceLimits::default()).unwrap();
        let package = super::archives::roundtrip(&language);
        for expected in rows["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["case"] == case["id"])
        {
            let result = intersect_text::evaluate(
                &package,
                &[intersect_text::Source {
                    uri: "memory:parameter-empty.test".into(),
                    text: expected["text"].as_str().unwrap().into(),
                }],
            );
            let syntax_success = expected["errors"].as_array().unwrap().is_empty();
            assert_eq!(
                result.diagnostics.is_empty(),
                syntax_success,
                "{}: {:?}",
                case["id"],
                result.diagnostics
            );
            check_resource(
                &result,
                "memory:parameter-empty.test",
                expected["text"].as_str().unwrap(),
                syntax_success,
                if syntax_success {
                    expected["resource_contents"].as_u64().unwrap() as usize
                } else {
                    0
                },
            );
            if expected["accepted"] == true {
                counts[0] += 1;
                assert_eq!(
                    super::fragment_runtime::model_projection(&package.schema, &result),
                    expected["model"]
                );
            } else {
                assert!(result.model.elements.is_empty());
                counts[if syntax_success { 2 } else { 1 }] += 1;
            }
        }
    }
    assert_eq!(counts, [1, 2, 1]);
}
