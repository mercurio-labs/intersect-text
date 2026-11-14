//! Frozen independent guard outcomes through the ordinary/inferred shared runtime.
use intersect_text::grammar::Profile;
use intersect_text::package::Package;
use serde_json::Value;

fn fixtures() -> (Value, Value) {
    (
        serde_json::from_str(include_str!(
            "../../../fixtures/xtext/guard-sequences/cases.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../fixtures/xtext/guard-sequences/observations.json"
        ))
        .unwrap(),
    )
}
fn resources(package: &Package, id: &str, observations: &Value) -> [usize; 2] {
    let mut counts = [0, 0];
    for run in observations["runs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["case"] == id)
    {
        let uri = "memory:guard-sequences.model";
        let text = run["text"].as_str().unwrap();
        assert_eq!(run["entry"], "resource");
        let result = intersect_text::evaluate(
            package,
            &[intersect_text::Source {
                uri: uri.into(),
                text: text.into(),
            }],
        );
        let accepted = run["accepted"].as_bool().unwrap();
        assert_eq!(
            result.diagnostics.is_empty(),
            accepted,
            "{id} / {text:?}: {:?}",
            result.diagnostics
        );
        super::parameters::check_resource(&result, uri, text, accepted, usize::from(accepted));
        if accepted {
            assert_eq!(
                super::fragment_runtime::model_projection(&package.schema, &result),
                run["model"],
                "{id} / {text:?}"
            );
            counts[0] += 1;
        } else {
            // Upstream may retain partial models. Native rejection is atomic.
            assert!(result.model.elements.is_empty(), "{id} / {text:?}");
            counts[1] += 1;
        }
    }
    counts
}
fn replay(package: &Package) -> Package {
    let serialized = serde_json::to_value(package).unwrap();
    let cached = serde_json::from_value(serialized.clone()).unwrap();
    let restored = intersect_text::recompile(&cached).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), serialized);
    restored
}

pub fn ordinary_resources_and_replay() {
    let (cases, expected) = fixtures();
    let mut counts = [0, 0, 0, 0, 0];
    for (case, generated) in cases["ordinary"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["ordinary"]["generation"].as_array().unwrap())
    {
        assert_eq!(case["id"], generated["id"]);
        let input = super::imported_parameters::input(case);
        let compiled = intersect_text::ordinary::compile(input.clone(), Profile::NativeV57);
        match generated["stage"].as_str().unwrap() {
            "metadata_rejected" => {
                let errors = compiled
                    .err()
                    .unwrap_or_else(|| panic!("{}: accepted rejected metadata", case["id"]));
                let expected_source = input.xtext.as_ref().unwrap();
                let error = &errors[0];
                assert_eq!(&error.source, &expected_source.uri);
                assert!(matches!(error.origin.as_deref(), Some(
                    intersect_model::definitions::schema::DiagnosticOrigin::Source { source, span: Some(span) }
                ) if source == expected_source && span.source == *source && span.start == error.start && span.end == error.end));
                if case["id"] == "empty_sequence" {
                    assert!(error.message.contains("unassigned action"));
                    let text = &input.bundle.source(expected_source).unwrap().text;
                    let second_action = text.rfind("{model::Model}").unwrap();
                    assert_eq!(error.start, second_action);
                    assert_eq!(error.end, second_action + "{model::Model}".len());
                    assert_eq!(&text[error.start..error.end], "{model::Model}");
                }
                counts[1] += 1;
                counts[4] += case["runs"].as_array().unwrap().len();
            }
            "generator_failed" => {
                // No JVM requests executed here: do not turn upstream generation
                // failure into a runtime acceptance/rejection qualification.
                counts[4] += case["runs"].as_array().unwrap().len();
                if let Ok(package) = compiled {
                    let grammar = package.grammar.as_ref().unwrap();
                    assert!(
                        matches!(&grammar.rules[&grammar.entry].body.kind,
                        intersect_text::grammar::Kind::Alternative(items) if items.is_empty()),
                        "{}: rule without a retained body became executable epsilon",
                        case["id"]
                    );
                }
            }
            "compiled" => {
                let package =
                    compiled.unwrap_or_else(|errors| panic!("{}: {errors:?}", case["id"]));
                assert_eq!(
                    serde_json::to_value(&package.compilation).unwrap(),
                    serde_json::to_value(Some(&input)).unwrap()
                );
                assert!(package.xtext_inference.is_none());
                let restored = replay(&package);
                let actual = resources(
                    &restored,
                    case["id"].as_str().unwrap(),
                    &expected["ordinary"],
                );
                counts[0] += 1;
                counts[2] += actual[0];
                counts[3] += actual[1];
            }
            stage => panic!("Unknown frozen stage {stage}"),
        }
    }
    assert_eq!(counts, [12, 2, 14, 89, 25]);
}

pub fn inferred_resources_and_replay() {
    let (cases, expected) = fixtures();
    let mut counts = [0, 0, 0];
    for case in cases["inferred"].as_array().unwrap() {
        let request = super::parameters::input(case);
        let language = request
            .compile(Default::default())
            .unwrap_or_else(|errors| panic!("{}: {errors:?}", case["id"]));
        assert_eq!(
            language.definitions().bundle.sources[..request.bundle.sources.len()],
            request.bundle.sources
        );
        let restored = replay(language.package());
        let actual = resources(
            &restored,
            case["id"].as_str().unwrap(),
            &expected["inferred"],
        );
        counts[0] += 1;
        counts[1] += actual[0];
        counts[2] += actual[1];
    }
    assert_eq!(counts, [4, 6, 26]);
}
