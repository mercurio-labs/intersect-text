//! Frozen source-loaded fragment inference through the native definition path.
use intersect_text::grammar::SourceLimits;
use intersect_text::inference::*;
use serde_json::{Value, json};
fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragments/cases.json"
    ))
    .unwrap()
}
pub fn frozen_matrix() {
    let cases = cases();
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragments/observations.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 45);
    assert_eq!(rows.len(), cases.len());
    let mut accepted = 0;
    let mut failures = vec![];
    for (case, row) in cases.iter().zip(&rows) {
        let request = super::composition::input(case);
        match infer_metamodels(&request.bundle, &request.grammar, SourceLimits::default()) {
            Ok(report) => {
                if row["all_sources_accepted"] != true {
                    failures.push(format!("{} incorrectly accepted", case["id"]));
                    continue;
                }
                accepted += 1;
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
                    assert_eq!(grammar.name, expected["grammar"]);
                    let packages = expected["metamodels"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|p| p["generated"] == true)
                        .map(|p| json!({"alias":p["alias"],"package":p["package"]}))
                        .collect::<Vec<_>>();
                    let expected_packages = json!({"generated_packages":packages});
                    let actual = super::controls::projection(&grammar.metamodels);
                    let expected_projection = super::controls::expected(&expected_packages);
                    if actual != expected_projection {
                        failures.push(format!(
                            "{} / {} differs\nactual {actual}\nexpected {expected_projection}",
                            case["id"], grammar.name
                        ));
                    } else {
                        super::controls::materialized_projection(
                            &report.schema,
                            &grammar.metamodels,
                            &grammar.generated_sources,
                            &expected_packages,
                        );
                    }
                    for rule in expected["rules"].as_array().unwrap() {
                        let binding = &grammar.rules[rule["name"].as_str().unwrap()];
                        assert_eq!(
                            json!(binding.target.as_ref().map(TypeId::identity)),
                            rule["type"],
                            "{} / {}",
                            case["id"],
                            rule["name"]
                        );
                        assert_eq!(json!(binding.wildcard), rule["wildcard"]);
                        assert_eq!(
                            binding.kind == intersect_text::grammar::syntax::RuleKind::Fragment,
                            rule["fragment"] == true
                        );
                        assert_eq!(binding.declaration.source, grammar.source);
                        assert!(binding.declaration.text(&report.bundle).is_ok());
                    }
                    if case["id"] == "inherited_wildcard" && grammar.source == request.grammar {
                        let feature = &grammar.metamodels[0].classifiers[0].features[0];
                        assert_eq!(feature.name, "name");
                        assert!(
                            feature
                                .origins
                                .iter()
                                .all(|o| o.span.source == report.grammars[0].source)
                        );
                    }
                    for classifier in &grammar.metamodels {
                        for class in &classifier.classifiers {
                            for origin in &class.origins {
                                assert!(origin.span.text(&report.bundle).is_ok());
                            }
                            for feature in &class.features {
                                for origin in &feature.origins {
                                    assert!(origin.span.text(&report.bundle).is_ok());
                                }
                            }
                        }
                    }
                }
            }
            Err(error) => {
                if row["all_sources_accepted"] == true {
                    failures.push(format!("{} rejected: {}", case["id"], error.message));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(accepted, 28);
}

fn execute(language: &InferredLanguage, text: &str) -> intersect_text::Evaluation {
    let result = intersect_text::evaluate(
        language.package(),
        &[intersect_text::Source {
            uri: "memory:fragment-model.test".into(),
            text: text.into(),
        }],
    );
    assert!(
        result.diagnostics.is_empty(),
        "{text}: {:?}",
        result.diagnostics
    );
    result
}
pub fn execution_and_replay() {
    let scenarios = [
        ("typed_basic", "alpha", "name", json!("alpha")),
        ("typed_explicit", "alpha", "name", json!("alpha")),
        ("typed_optional", "m alpha", "name", json!("alpha")),
        ("typed_nested", "alpha", "name", json!("alpha")),
        (
            "typed_multiple_callers",
            "a alpha b beta",
            "name",
            json!("alpha"),
        ),
        ("typed_enum", "a", "mode", Value::Null),
        (
            "typed_references",
            "item alpha item beta to alpha",
            "name",
            json!("alpha"),
        ),
        (
            "typed_recursive",
            "node alpha child node beta",
            "name",
            json!("beta"),
        ),
        (
            "typed_call_then_assignment",
            "alpha tag beta",
            "tag",
            json!("beta"),
        ),
        (
            "typed_assignment_then_call",
            "tag beta alpha",
            "tag",
            json!("beta"),
        ),
        ("wildcard_basic", "m alpha", "name", json!("alpha")),
        (
            "wildcard_multiple_callers",
            "a alpha b beta",
            "name",
            json!("alpha"),
        ),
        ("wildcard_nested", "alpha", "name", json!("alpha")),
        ("wildcard_override", "alpha tag beta", "tag", json!("beta")),
        ("typed_wildcard_call", "alpha", "name", json!("alpha")),
        ("wildcard_optional", "m alpha", "name", json!("alpha")),
        (
            "wildcard_many",
            "m alpha beta",
            "names",
            json!(["alpha", "beta"]),
        ),
        (
            "wildcard_references",
            "item alpha item beta to alpha",
            "name",
            json!("alpha"),
        ),
        ("inherited_typed", "m alpha", "name", json!("alpha")),
        ("inherited_wildcard", "m alpha", "name", json!("alpha")),
        ("typed_override_same_type", "alpha", "name", json!("alpha")),
        ("three_level_wildcard", "last alpha", "name", json!("alpha")),
        ("three_level_typed", "last alpha", "name", json!("alpha")),
        (
            "typed_datatype_unassigned",
            "x alpha",
            "name",
            json!("alpha"),
        ),
        ("typed_imported_return", "alpha", "name", json!("alpha")),
        ("wildcard_unused_valid", "alpha", "name", json!("alpha")),
        (
            "typed_shared_return",
            "alpha then beta",
            "tag",
            json!("beta"),
        ),
    ];
    assert_eq!(scenarios.len(), 27);
    let cases = cases();
    let mut failures = vec![];
    for (name, text, feature, expected) in scenarios {
        let case = cases.iter().find(|c| c["id"] == name).unwrap();
        let request = super::composition::input(case);
        let language = match request.compile(SourceLimits::default()) {
            Ok(language) => language,
            Err(errors) => {
                failures.push(format!("{name}: {errors:?}"));
                continue;
            }
        };
        let result = execute(&language, text);
        super::archives::model_roundtrip(&language, text, &result);
        let object = result
            .model
            .elements
            .iter()
            .find(|o| o.values.contains_key(feature))
            .unwrap();
        if name == "typed_enum" {
            let value: intersect_model::definitions::enums::EnumValue =
                serde_json::from_value(object.values[feature].clone()).unwrap();
            let enumeration = language
                .package()
                .schema
                .enumerations
                .values()
                .find(|e| e.name() == "Mode")
                .unwrap();
            assert_eq!(
                value,
                enumeration
                    .value_by_id(&enumeration.by_name("A").unwrap().declaration)
                    .unwrap()
            );
        } else {
            assert_eq!(object.values[feature], expected, "{name}");
        }
        if name.ends_with("multiple_callers") {
            let beta = result
                .model
                .elements
                .iter()
                .find(|o| o.values.get("name") == Some(&json!("beta")))
                .unwrap();
            assert_ne!(object.class, beta.class);
            assert_eq!(result.model.elements.len(), 3);
        }
        if name.ends_with("references") {
            let beta = result
                .model
                .elements
                .iter()
                .find(|o| o.values.get("name") == Some(&json!("beta")))
                .unwrap();
            assert_eq!(beta.values["target"], json!(object.id));
        }
        if matches!(name, "typed_optional" | "wildcard_optional") {
            let omitted = execute(&language, "m");
            assert_eq!(omitted.model.elements.len(), 1);
            assert_eq!(
                omitted.model.elements[0].values.get("name"),
                Some(&Value::Null),
                "{name}: {:?}",
                omitted.model.elements[0].values
            );
        }
        let replay: InferenceRequest =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        let replay = replay.compile(SourceLimits::default()).unwrap();
        assert_eq!(
            serde_json::to_value(language.package()).unwrap(),
            serde_json::to_value(replay.package()).unwrap()
        );
        assert_eq!(result.model, execute(&replay, text).model);
        let restored =
            serde_json::from_value(serde_json::to_value(language.package()).unwrap()).unwrap();
        let restored = intersect_text::recompile(&restored).unwrap();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(language.package()).unwrap()
        );
        let restored_result = intersect_text::evaluate(
            &restored,
            &[intersect_text::Source {
                uri: result.model.elements[0].resource.clone(),
                text: text.into(),
            }],
        );
        assert!(
            restored_result.diagnostics.is_empty(),
            "{name}: {:?}",
            restored_result.diagnostics
        );
        assert_eq!(result.model, restored_result.model);

        let invalid = intersect_text::evaluate(
            language.package(),
            &[intersect_text::Source {
                uri: "memory:fragment-invalid.test".into(),
                text: format!("{text} !"),
            }],
        );
        assert!(!invalid.diagnostics.is_empty());
        assert!(invalid.model.elements.is_empty());
        assert_eq!(result.model, execute(&language, text).model);
    }
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragments/observations.json"
    ))
    .unwrap();
    for (case, row) in cases.iter().zip(rows) {
        if row["all_sources_accepted"] != true {
            assert!(
                super::composition::input(case)
                    .compile(SourceLimits::default())
                    .is_err(),
                "{}",
                case["id"]
            );
        }
    }
    // Upstream inference keeps Model unrelated to Common. Compile the generic
    // caller and reject its missing feature at runtime without publishing data.
    let case = cases
        .iter()
        .find(|c| c["id"] == "wildcard_typed_call")
        .unwrap();
    let language = super::composition::input(case)
        .compile(SourceLimits::default())
        .unwrap();
    let result = intersect_text::evaluate(
        language.package(),
        &[intersect_text::Source {
            uri: "memory:fragment-missing-feature.test".into(),
            text: "alpha".into(),
        }],
    );
    assert!(!result.diagnostics.is_empty());
    assert!(result.model.elements.is_empty());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
pub fn verify() -> usize {
    frozen_matrix();
    execution_and_replay();
    2
}
