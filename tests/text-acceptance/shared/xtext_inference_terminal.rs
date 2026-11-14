//! Exact terminal/scalar library graphs through the shared model consumers.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::SourceLimits;
use intersect_text::inference::*;
use serde_json::{Value, json};
fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-terminal-parents/cases.json"
    ))
    .unwrap()
}
pub fn frozen_matrix() {
    let cases = cases();
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-terminal-parents/observations.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 21);
    assert_eq!(cases.len(), rows.len());
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
                        .filter(|d| d["generated"] == true)
                        .map(|d| json!({"alias":d["alias"],"package":d["package"]}))
                        .collect::<Vec<_>>();
                    let expected_packages = json!({"generated_packages":packages});
                    assert_eq!(
                        super::controls::projection(&grammar.metamodels),
                        super::controls::expected(&expected_packages),
                        "{} / {}",
                        case["id"],
                        grammar.name
                    );
                    super::controls::materialized_projection(
                        &report.schema,
                        &grammar.metamodels,
                        &grammar.generated_sources,
                        &expected_packages,
                    );
                    for rule in expected["rules"].as_array().unwrap() {
                        let binding = &grammar.rules[rule["name"].as_str().unwrap()];
                        assert_eq!(
                            json!(binding.target.as_ref().unwrap().identity()),
                            rule["type"]
                        );
                        assert_eq!(binding.declaration.source, grammar.source);
                        assert!(binding.declaration.text(&report.bundle).is_ok());
                    }
                    if grammar.name == "org.eclipse.xtext.common.Terminals" {
                        assert_eq!(
                            grammar.source.sha256,
                            "3e3d37770a731b0d5270b456743fc70331aa7aaf1c99afa6728896c64bbe401a"
                        );
                        assert!(grammar.generated_sources.is_empty());
                        assert!(grammar.metamodels.is_empty());
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
    assert_eq!(accepted, 16);
}
fn execute(language: &InferredLanguage, text: &str) -> intersect_text::Evaluation {
    let result = intersect_text::evaluate(
        language.package(),
        &[intersect_text::Source {
            uri: "memory:terminal-model.test".into(),
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
        ("authored_explicit_types", "Alpha 7", "name", json!("Alpha")),
        (
            "common_scalars",
            "Alpha 7 \"Hello\\nworld\"",
            "title",
            json!("Hello\nworld"),
        ),
        ("common_no_local_import", "Alpha", "name", json!("Alpha")),
        (
            "common_references",
            "node Alpha node Beta use Alpha use Alpha",
            "nodes",
            json!(2),
        ),
        (
            "common_datatype_wrapper",
            "\"Alpha\"",
            "title",
            json!("\"Alpha\""),
        ),
        ("common_ws_override", "model~Alpha", "name", json!("Alpha")),
        ("common_hidden_policy", "Alpha", "name", json!("Alpha")),
        ("common_id_override", "@alpha", "name", json!("@alpha")),
        (
            "common_explicit_super",
            "model Alpha @beta",
            "original",
            json!("Alpha"),
        ),
        ("three_level_terminal", "word", "word", json!("word")),
        ("three_level_generated", "Alpha", "name", json!("Alpha")),
        ("common_enumeration", "red", "shade", Value::Null),
        ("terminal_fragments", "alpha", "name", json!("alpha")),
        ("custom_quoted", "\"alpha\"", "title", json!("\"alpha\"")),
        (
            "three_level_scalar",
            "Alpha.Beta",
            "name",
            json!("Alpha.Beta"),
        ),
        ("three_level_fragments", "42", "count", json!(42)),
    ];
    let cases = cases();
    let mut failures = vec![];
    for (name, text, feature, expected) in scenarios {
        let case = cases.iter().find(|c| c["id"] == name).unwrap();
        let request = super::composition::input(case);
        let language = match request.compile(SourceLimits::default()) {
            Ok(value) => value,
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
            .find(|e| e.values.contains_key(feature))
            .unwrap();
        if name == "common_references" {
            assert_eq!(object.values[feature].as_array().unwrap().len(), 2);
            assert_eq!(
                object.values["uses"],
                json!([object.values["nodes"][0], object.values["nodes"][0]])
            );
        } else if name == "common_enumeration" {
            let value: intersect_model::definitions::enums::EnumValue =
                serde_json::from_value(object.values[feature].clone()).unwrap();
            let enumeration = language
                .package()
                .schema
                .enumerations
                .values()
                .find(|e| e.name() == "Shade")
                .unwrap();
            assert_eq!(
                value,
                enumeration
                    .value_by_id(&enumeration.by_name("RED").unwrap().declaration)
                    .unwrap()
            );
        } else {
            assert_eq!(object.values[feature], expected, "{name}");
        }
        if name == "common_scalars" {
            assert_eq!(object.values["name"], "Alpha");
            assert_eq!(object.values["count"], 7);
        }
        if name == "common_explicit_super" {
            assert_eq!(object.values["custom"], "@beta");
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

        for grammar in &language.definitions().grammars {
            if grammar.generated_sources.is_empty() {
                let mut standalone = request.clone();
                standalone.grammar = grammar.source.clone();
                assert!(
                    standalone.compile(SourceLimits::default()).is_err(),
                    "Library must not invent a model entry: {}",
                    grammar.name
                );
            }
        }
        let mut missing = request.clone();
        missing
            .bundle
            .sources
            .iter_mut()
            .find(|s| s.id == missing.grammar)
            .unwrap()
            .dependencies
            .retain(|id| request.bundle.source(id).unwrap().kind != SourceKind::Xtext);
        assert!(missing.compile(SourceLimits::default()).is_err());
        if name == "common_hidden_policy" {
            let invalid = intersect_text::evaluate(
                language.package(),
                &[intersect_text::Source {
                    uri: "memory:invalid.test".into(),
                    text: "/* comment */ Alpha".into(),
                }],
            );
            assert!(!invalid.diagnostics.is_empty());
            assert!(invalid.model.elements.is_empty());
        }
        assert_eq!(result.model, execute(&language, text).model);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let common = cases
        .iter()
        .find(|c| c["id"] == "common_no_local_import")
        .unwrap();
    let request = super::composition::input(common);
    assert!(
        request
            .compile(SourceLimits {
                source_bytes: 256_000,
                declarations: 7
            })
            .is_err()
    );
    let mut large = request.clone();
    let old = large.bundle.sources[0].id.clone();
    let mut parent = DefinitionSource::new(
        old.uri.clone(),
        SourceKind::Xtext,
        format!(
            "{}\n/*{}*/",
            large.bundle.sources[0].text,
            "x".repeat(260_000)
        ),
    );
    parent.dependencies = large.bundle.sources[0].dependencies.clone();
    for source in &mut large.bundle.sources {
        for dependency in &mut source.dependencies {
            if *dependency == old {
                *dependency = parent.id.clone();
            }
        }
    }
    large.bundle.sources[0] = parent;
    assert!(large.compile(SourceLimits::default()).is_err());
    let limits = SourceLimits {
        source_bytes: large.bundle.sources[0].text.len() + 1024,
        declarations: 128,
    };
    let language = large.compile(limits).unwrap();
    assert_eq!(execute(&language, "Alpha").model.elements.len(), 1);
    assert!(language.recompile(limits).is_ok());
    assert!(language.recompile(SourceLimits::default()).is_err());
    let case = cases
        .iter()
        .find(|c| c["id"] == "terminal_fragments")
        .unwrap();
    let mut invalid = super::composition::input(case);
    let index = invalid
        .bundle
        .sources
        .iter()
        .position(|s| s.id == invalid.grammar)
        .unwrap();
    let source = &invalid.bundle.sources[index];
    let mut replacement = DefinitionSource::new(
        source.id.uri.clone(),
        SourceKind::Xtext,
        source.text.replace("name=ID", "name=LETTER"),
    );
    replacement.dependencies = source.dependencies.clone();
    invalid.grammar = replacement.id.clone();
    invalid.bundle.sources[index] = replacement;
    assert!(invalid.compile(SourceLimits::default()).is_err());
}
pub fn verify() -> usize {
    frozen_matrix();
    execution_and_replay();
    2
}
