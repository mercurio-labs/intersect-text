//! Ordinary imported-source controls against frozen independent observations.
use intersect_text::grammar::Profile;
use serde_json::Value;
use std::collections::BTreeMap;
#[path = "xtext_imported_parameter_inputs.rs"]
mod inputs;
pub(crate) use inputs::input;

pub fn default_resources() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ))
    .unwrap();
    let expected: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/observations.json"
    ))
    .unwrap();
    let mut failures = vec![];
    let mut counts = [0usize; 6];
    for (case, metadata) in cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["metadata"].as_array().unwrap())
    {
        assert_eq!(case["id"], metadata["id"]);
        let original = input(case);
        let package = match intersect_text::ordinary::compile(original.clone(), Profile::NativeV57)
        {
            Ok(package) => {
                if metadata["all_sources_accepted"] != true {
                    failures.push(format!("{}: incorrectly accepted", case["id"]));
                    continue;
                }
                counts[0] += 1;
                package
            }
            Err(errors) => {
                if metadata["all_sources_accepted"] == true {
                    failures.push(format!("{}: incorrectly rejected: {errors:?}", case["id"]));
                } else {
                    counts[1] += 1;
                }
                continue;
            }
        };
        assert_eq!(
            serde_json::to_value(&package.compilation).unwrap(),
            serde_json::to_value(Some(&original)).unwrap()
        );
        assert!(package.xtext_inference.is_none());
        assert_eq!(
            package.sources["xtext"],
            original
                .bundle
                .source(original.xtext.as_ref().unwrap())
                .unwrap()
                .text
        );
        let serialized = serde_json::to_value(&package).unwrap();
        let restored =
            intersect_text::recompile(&serde_json::from_value(serialized.clone()).unwrap())
                .unwrap();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serialized,
            "{}: ordinary source replay",
            case["id"]
        );
        for run in expected["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|run| run["case"] == case["id"] && run["entry"] == "resource")
        {
            let text = run["text"].as_str().unwrap();
            let result = intersect_text::evaluate(
                &package,
                &[intersect_text::Source {
                    uri: "memory:ordinary-model.test".into(),
                    text: text.into(),
                }],
            );
            let parsed = run["errors"]
                .as_array()
                .is_some_and(|errors| errors.is_empty())
                && run["exception"].is_null();
            if result.diagnostics.is_empty() != parsed {
                failures.push(format!(
                    "{} / {text:?}: acceptance differs: {:?}",
                    case["id"], result.diagnostics
                ));
                continue;
            }
            super::parameters::check_resource(
                &result,
                "memory:ordinary-model.test",
                text,
                parsed,
                if parsed {
                    run["resource_contents"].as_u64().unwrap() as usize
                } else {
                    0
                },
            );
            if parsed {
                let actual = super::fragment_runtime::model_projection(&package.schema, &result);
                if (run["accepted"] == true && actual != run["model"])
                    || (run["accepted"] != true && !result.model.elements.is_empty())
                {
                    failures.push(format!(
                        "{} / {text:?}: model differs\nactual {actual}\nexpected {}",
                        case["id"], run["model"]
                    ));
                }
                if run["accepted"] == true {
                    counts[2] += 1;
                } else {
                    counts[5] += 1;
                }
            } else {
                assert!(result.model.elements.is_empty());
                counts[3] += 1;
            }
        }
        counts[4] += expected["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|run| run["case"] == case["id"] && run["entry"] != "resource")
            .count();
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    println!("ordinary default resources: {counts:?}; named entries remain separately unconsumed");
    assert_eq!(counts, [31, 17, 34, 42, 14, 1]);
}

pub fn authority_and_admission() {
    use intersect_text::compiler;
    use intersect_text::package::CompilationSourcePolicy;
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ))
    .unwrap();
    let original = input(&cases["cases"][0]);
    let policy = CompilationSourcePolicy::default();
    let package = intersect_text::ordinary::compile(original.clone(), Profile::NativeV57).unwrap();
    let expected = serde_json::to_value(&package).unwrap();
    // Cached generated output cannot replace the authored ordinary recipe.
    let mut cached = package.clone();
    cached.sources.clear();
    cached.grammar.as_mut().unwrap().entry = "not_an_authored_entry".into();
    assert_eq!(
        serde_json::to_value(intersect_text::recompile(&cached).unwrap()).unwrap(),
        expected
    );
    for corrupt in 0..4 {
        let mut invalid = package.clone();
        match corrupt {
            0 => invalid.version = 0,
            1 => invalid.compilation = None,
            2 => invalid.compilation.as_mut().unwrap().xtext = None,
            _ => invalid.compilation.as_mut().unwrap().bundle.sources[0]
                .text
                .push(' '),
        }
        assert!(
            intersect_text::recompile(&invalid).is_err(),
            "invalid replay authority {corrupt}"
        );
    }
    let lower_host = CompilationSourcePolicy {
        xtext_declarations: policy.xtext_declarations - 1,
        ..policy
    };
    assert!(intersect_text::recompile_with_source_policy(&package, lower_host).is_err());
    for requested in [
        CompilationSourcePolicy {
            xtext_declarations: 1,
            ..policy
        },
        CompilationSourcePolicy {
            xtext_bytes: 1,
            ..policy
        },
        CompilationSourcePolicy {
            structure_bytes: 1,
            ..policy
        },
    ] {
        assert!(
            intersect_text::ordinary::compile_with_source_policy(
                original.clone(),
                Profile::NativeV57,
                requested,
                policy
            )
            .is_err()
        );
    }
    assert!(
        intersect_text::ordinary::compile_with_source_policy(
            original.clone(),
            Profile::NativeV57,
            policy,
            lower_host
        )
        .is_err()
    );
    // Installing the frontend does not broaden an earlier syntax profile.
    assert!(intersect_text::ordinary::compile(original.clone(), Profile::NativeV56).is_err());
    assert!(compiler::compile_bundle_with_profile(original.clone(), Profile::NativeV57).is_err());
    // The shared program seam must never silently ignore unused inputs.
    let mut structure = original;
    structure.xtext = None;
    assert!(
        compiler::compile_bundle_with_source_program(
            structure,
            Profile::NativeV57,
            &[],
            None,
            policy,
            policy,
            &intersect_text::grammar::SourceProgram {
                bindings: &[],
                producers: &BTreeMap::new()
            }
        )
        .is_err()
    );
}
