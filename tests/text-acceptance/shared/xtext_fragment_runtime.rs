//! Frozen generated Xtext parser outcomes through the shared native/WASM path.
use intersect_model::definitions::schema::Schema;
use intersect_text::grammar::SourceLimits;
use intersect_text::{Evaluation, Source, inference::InferenceRequest};
use serde_json::{Value, json};

fn evaluate(package: &intersect_text::package::Package, text: &str) -> Evaluation {
    intersect_text::evaluate(
        package,
        &[Source {
            uri: "memory:fragment-runtime.test".into(),
            text: text.into(),
        }],
    )
}
fn class_uri(schema: &Schema, key: &str) -> String {
    let declaration = &schema.declarations[key];
    format!("{}#//{}", declaration.namespace, declaration.name)
}
pub(crate) fn model_projection(schema: &Schema, result: &Evaluation) -> Value {
    fn value(schema: &Schema, result: &Evaluation, original: &Value) -> Value {
        if let Some(id) = original.as_str()
            && let Some((index, object)) = result
                .model
                .elements
                .iter()
                .enumerate()
                .find(|(_, e)| e.id == id)
        {
            return json!({"object":index,"class":class_uri(schema, &object.class)});
        }
        if let Some(items) = original.as_array() {
            return Value::Array(items.iter().map(|v| value(schema, result, v)).collect());
        }
        original.clone()
    }
    Value::Array(result.model.elements.iter().map(|object| json!({
        "class":class_uri(schema, &object.class),
        "values":object.values.iter().map(|(key,v)| (key.clone(),value(schema,result,v))).collect::<serde_json::Map<_,_>>()
    })).collect())
}
pub fn execution_and_replay() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/cases.json"
    ))
    .unwrap();
    let observation: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/observations.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 10);
    let mut successes = 0;
    let mut rejections = 0;
    let mut named_probes = 0;
    for (case, expected) in cases
        .iter()
        .zip(observation["metadata"].as_array().unwrap())
    {
        assert_eq!(case["id"], expected["id"]);
        let request = super::composition::input(case);
        let language = request
            .compile(SourceLimits::default())
            .unwrap_or_else(|errors| panic!("{}: {errors:?}", case["id"]));
        for (grammar, source) in language
            .definitions()
            .grammars
            .iter()
            .zip(expected["sources"].as_array().unwrap())
        {
            let packages = source["metamodels"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["generated"] == true)
                .map(|p| json!({"alias":p["alias"],"package":p["package"]}))
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
                &language.package().schema,
                &grammar.metamodels,
                &grammar.generated_sources,
                &expected_packages,
            );
        }
        let replay: InferenceRequest =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        let replay = replay.compile(SourceLimits::default()).unwrap();
        assert_eq!(
            serde_json::to_value(language.package()).unwrap(),
            serde_json::to_value(replay.package()).unwrap()
        );
        let serialized = serde_json::to_value(language.package()).unwrap();
        assert!(serialized.get("compilation").is_none());
        let restored = serde_json::from_value(serialized.clone()).unwrap();
        let restored = intersect_text::recompile(&restored).unwrap();
        assert_eq!(serde_json::to_value(&restored).unwrap(), serialized);
        let archived = super::archives::roundtrip(&language);
        for run in observation["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|run| run["case"] == case["id"])
        {
            if run["entry"] != "resource" {
                // The public evaluator has only a default source entry. These
                // two upstream emitted-entry probes make no native API claim.
                named_probes += 1;
                continue;
            }
            let text = run["text"].as_str().unwrap();
            let result = evaluate(language.package(), text);
            let archived_result = evaluate(&archived, text);
            assert_eq!(result.model, archived_result.model);
            assert_eq!(result.diagnostics, archived_result.diagnostics);
            let repeated = evaluate(replay.package(), text);
            let package_replay = evaluate(&restored, text);
            assert_eq!(
                result.model, package_replay.model,
                "{} / {text}",
                case["id"]
            );
            assert_eq!(result.diagnostics, package_replay.diagnostics);
            assert_eq!(result.model, repeated.model, "{} / {text}", case["id"]);
            assert_eq!(
                result.diagnostics.is_empty(),
                run["accepted"] == true,
                "{} / {text}: {:?}",
                case["id"],
                result.diagnostics
            );
            assert_eq!(result.diagnostics, repeated.diagnostics);
            if run["accepted"] == true {
                successes += 1;
                assert_eq!(
                    model_projection(&language.package().schema, &result),
                    run["model"],
                    "{} / {text}",
                    case["id"]
                );
                let invalid = evaluate(language.package(), &format!("{text} !"));
                assert!(!invalid.diagnostics.is_empty());
                assert!(invalid.model.elements.is_empty());
                assert_eq!(result.model, evaluate(language.package(), text).model);
            } else {
                rejections += 1;
                // Xtext may retain a partial EObject on conversion failure.
                // Native ingestion intentionally rejects the complete source.
                assert!(result.model.elements.is_empty(), "{} / {text}", case["id"]);
            }
        }
    }
    assert_eq!((successes, rejections, named_probes), (7, 4, 2));
}
pub fn verify() -> usize {
    execution_and_replay();
    1
}
