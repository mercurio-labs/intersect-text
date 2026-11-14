//! Frozen ordinary entry behavior and shared inferred/native counterparts.
use intersect_text::grammar::Profile;
use intersect_text::package::CompilationSourcePolicy;
use intersect_text::{
    EvaluationLimits, Source,
    entry::{EntryLanguage, EntryValue},
};
use serde_json::Value;
fn sources(text: &str) -> [Source; 1] {
    [Source {
        uri: "memory:entry.test".into(),
        text: text.into(),
    }]
}
pub(crate) fn check(language: &EntryLanguage, run: &Value) {
    let input = sources(run["text"].as_str().unwrap());
    let result = language
        .evaluate(
            run["entry"].as_str().unwrap(),
            &input,
            EvaluationLimits::native(),
        )
        .unwrap();
    let parsed = run["exception"].is_null()
        && run["errors"]
            .as_array()
            .is_some_and(|errors| errors.is_empty());
    assert_eq!(
        result.evaluation.diagnostics.is_empty(),
        parsed,
        "{} / {} / {}: {:?}",
        run["case"],
        run["entry"],
        run["text"],
        result.evaluation.diagnostics
    );
    if language
        .entries()
        .contains_key(run["entry"].as_str().unwrap())
    {
        super::parameters::check_resource(
            &result.evaluation,
            "memory:entry.test",
            &input[0].text,
            parsed,
            usize::from(run["accepted"] == true),
        );
    } else {
        // Selection fails before a resource is admitted to the interpreter.
        assert!(result.evaluation.model.resources.is_empty());
    }
    if run["accepted"] == true {
        assert_eq!(
            super::fragment_runtime::model_projection(
                &language.package().schema,
                &result.evaluation
            ),
            run["model"]
        );
        let EntryValue::Object(id) = &result.values["memory:entry.test"] else {
            panic!("Expected object entry")
        };
        assert_eq!(id, &result.evaluation.model.elements[0].id);
    } else if !parsed {
        assert!(result.evaluation.model.elements.is_empty());
        assert!(result.values.is_empty());
    } else {
        assert!(result.evaluation.model.elements.is_empty());
    }
}
pub fn original_named_matrix() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ))
    .unwrap();
    let observed: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/observations.json"
    ))
    .unwrap();
    let mut counts = [0, 0];
    for case in cases["cases"].as_array().unwrap() {
        let runs = observed["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|run| run["case"] == case["id"] && run["entry"] != "resource")
            .collect::<Vec<_>>();
        if runs.is_empty() {
            continue;
        }
        let package = intersect_text::ordinary::compile(
            super::imported_parameters::input(case),
            Profile::NativeV57,
        )
        .unwrap();
        let language =
            EntryLanguage::prepare(&package, CompilationSourcePolicy::default()).unwrap();
        assert_eq!(
            serde_json::to_value(language.package()).unwrap(),
            serde_json::to_value(&package).unwrap()
        );
        for run in runs {
            check(&language, run);
            counts[usize::from(run["accepted"] != true)] += 1;
        }
    }
    assert_eq!(counts, [6, 8]);
}
pub fn entry_contexts_and_returns() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/parameter-entry-contexts/cases.json"
    ))
    .unwrap();
    let observed: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/parameter-entry-contexts/observations.json"
    ))
    .unwrap();
    let mut counts = [0, 0, 0];
    for (case, meta) in cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(observed["metadata"].as_array().unwrap())
    {
        let package = intersect_text::ordinary::compile(
            super::imported_parameters::input(case),
            Profile::NativeV57,
        );
        let package = match package {
            Ok(package) => {
                assert_eq!(
                    meta["all_sources_accepted"], true,
                    "{} wrongly accepted",
                    case["id"]
                );
                counts[0] += 1;
                package
            }
            Err(errors) => {
                // Metadata admission and executable compilation are separate.
                // These two scalar-first graphs fail the independent generator;
                // the native compiler explicitly retains its scalar-root limit.
                if meta["all_sources_accepted"] == true {
                    assert!(matches!(
                        case["id"].as_str(),
                        Some("parameterized_scalar_entry" | "scalar_unassigned_fragment")
                    ));
                    assert!(
                        errors.iter().any(|e| e
                            .message
                            .contains("Scalar parser entry execution is unsupported")),
                        "{errors:?}"
                    );
                } else {
                    assert_eq!(case["id"], "scalar_via_fragment");
                }
                counts[1] += 1;
                continue;
            }
        };
        let language =
            EntryLanguage::prepare(&package, CompilationSourcePolicy::default()).unwrap();
        assert_eq!(
            serde_json::to_value(language.package()).unwrap(),
            serde_json::to_value(&package).unwrap()
        );
        let generation = observed["generation"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == case["id"])
            .unwrap();
        if generation["stage"] != "compiled" {
            continue;
        } // Retain unexecuted upstream generator obstructions.
        let inventory = generation["entry_methods"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                m["method"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("entryRule")
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            language
                .entries()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            inventory,
            "{}: entry inventory",
            case["id"]
        );
        for run in observed["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["case"] == case["id"])
        {
            if run["entry"] == "resource" {
                let input = sources(run["text"].as_str().unwrap());
                let result = intersect_text::evaluate(language.package(), &input);
                let parsed = run["exception"].is_null()
                    && run["errors"].as_array().is_some_and(|e| e.is_empty());
                assert_eq!(
                    result.diagnostics.is_empty(),
                    parsed,
                    "{}: resource",
                    case["id"]
                );
                if run["accepted"] == true {
                    assert_eq!(
                        super::fragment_runtime::model_projection(&package.schema, &result),
                        run["model"]
                    );
                } else {
                    assert!(result.model.elements.is_empty());
                }
            } else {
                check(&language, run);
                counts[2] += 1;
                let raw = observed["direct_entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| {
                        r["case"] == case["id"]
                            && r["entry"] == run["entry"]
                            && r["text"] == run["text"]
                    })
                    .unwrap();
                let result = language
                    .evaluate(
                        run["entry"].as_str().unwrap(),
                        &sources(run["text"].as_str().unwrap()),
                        EvaluationLimits::native(),
                    )
                    .unwrap();
                if raw["value_kind"] == "string"
                    && raw["errors"].as_array().is_some_and(|e| e.is_empty())
                {
                    assert_eq!(
                        result.values["memory:entry.test"],
                        EntryValue::Datatype(raw["value"].as_str().unwrap().into())
                    );
                }
            }
        }
    }
    assert_eq!(counts, [7, 3, 25]);
}
pub fn inferred_counterparts() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-parameters/cases.json"
    ))
    .unwrap();
    let mut count = 0;
    // Additional native counterparts, distinct from original independent named
    // observations. Their default model shapes are already independently frozen.
    for id in [
        "false_call",
        "recursive_context",
        "inherited_parameters",
        "parameterized_entry",
    ] {
        let case = cases["lowercase_counterparts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        let inferred = super::parameters::input(case)
            .compile(Default::default())
            .unwrap();
        let language =
            EntryLanguage::prepare(inferred.package(), CompilationSourcePolicy::default()).unwrap();
        let entry = if id == "parameterized_entry" {
            "Model"
        } else {
            "Node"
        };
        assert!(language.entries().contains_key(entry));
        let result = language
            .evaluate(
                entry,
                &sources(if id == "recursive_context" {
                    "no alpha"
                } else {
                    "alpha"
                }),
                EvaluationLimits::native(),
            )
            .unwrap();
        assert!(
            result.evaluation.diagnostics.is_empty(),
            "{id}: {:?}",
            result.evaluation.diagnostics
        );
        assert!(!result.evaluation.model.elements.is_empty());
        count += 1;
    }
    assert_eq!(count, 4);
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/cases.json"
    ))
    .unwrap();
    let observations: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/observations.json"
    ))
    .unwrap();
    let mut count = 0;
    for run in observations["runs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["entry"] != "resource")
    {
        let case = cases.iter().find(|c| c["id"] == run["case"]).unwrap();
        let inferred = super::composition::input(case)
            .compile(Default::default())
            .unwrap();
        let language =
            EntryLanguage::prepare(inferred.package(), CompilationSourcePolicy::default()).unwrap();
        check(&language, run);
        count += 1;
    }
    assert_eq!(count, 2);
}
pub fn authority_and_budgets() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ))
    .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "false_call")
        .unwrap();
    let package = intersect_text::ordinary::compile(
        super::imported_parameters::input(case),
        Profile::NativeV57,
    )
    .unwrap();
    let policy = CompilationSourcePolicy::default();
    let language = EntryLanguage::prepare(&package, policy).unwrap();
    let span = &language.entries()["Node"].declaration;
    let original = package
        .compilation
        .as_ref()
        .unwrap()
        .bundle
        .source(&span.source)
        .unwrap();
    assert!(original.text[span.start..span.end].starts_with("Node<Flag>"));
    assert!(span.source.uri.contains("false_call"));
    let mut cached = package.clone();
    cached.grammar.as_mut().unwrap().entry = "forged".into();
    cached.sources.clear();
    let rebuilt = EntryLanguage::prepare(&cached, policy).unwrap();
    assert_eq!(
        serde_json::to_value(rebuilt.package()).unwrap(),
        serde_json::to_value(language.package()).unwrap()
    );
    let mut invalid = package.clone();
    invalid.compilation.as_mut().unwrap().bundle.sources[0]
        .text
        .push(' ');
    assert!(EntryLanguage::prepare(&invalid, policy).is_err());
    assert!(
        EntryLanguage::prepare(
            &package,
            CompilationSourcePolicy {
                xtext_bytes: policy.xtext_bytes - 1,
                ..policy
            }
        )
        .is_err()
    );
    assert!(
        language
            .evaluate("Missing", &[], EvaluationLimits::native())
            .is_err()
    );
    let missing = language
        .evaluate("ID", &sources("alpha"), EvaluationLimits::native())
        .unwrap();
    assert!(!missing.evaluation.diagnostics.is_empty());
    assert!(missing.values.is_empty());
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
            nesting: 0,
            ..EvaluationLimits::native()
        },
    ] {
        let rejected = language
            .evaluate("Node", &sources("alpha"), limits)
            .unwrap();
        assert!(!rejected.evaluation.diagnostics.is_empty(), "{limits:?}");
        assert!(rejected.evaluation.model.elements.is_empty());
        assert!(rejected.values.is_empty());
    }
    let bundle = &language.package().compilation.as_ref().unwrap().bundle;
    let own = intersect_text::setting::TextSettings::new(language.package(), bundle, &[]).unwrap();
    let other = intersect_text::setting::TextSettings::new(
        &package,
        &package.compilation.as_ref().unwrap().bundle,
        &[],
    )
    .unwrap();
    let derived = intersect_model::query::PackageDerived(language.package());
    assert!(
        language
            .evaluate_with_context(
                "Node",
                &sources("alpha"),
                EvaluationLimits::native(),
                None,
                Some(&other),
                &derived
            )
            .is_err()
    );
    let accepted = language
        .evaluate_with_context(
            "Node",
            &sources("alpha"),
            EvaluationLimits::native(),
            None,
            Some(&own),
            &derived,
        )
        .unwrap();
    assert!(accepted.evaluation.diagnostics.is_empty());
    assert_eq!(accepted.evaluation.model.elements.len(), 1);
    // The failed invocation and its budgets never alter the prepared language.
    assert_eq!(
        serde_json::to_value(language.package()).unwrap(),
        serde_json::to_value(&package).unwrap()
    );
}
pub fn verify() -> usize {
    original_named_matrix();
    entry_contexts_and_returns();
    inferred_counterparts();
    authority_and_budgets();
    4
}
