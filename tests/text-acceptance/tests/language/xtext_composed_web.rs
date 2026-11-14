use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};

fn verify_composed_web_examples(profile: Profile) {
    for (name, ecore, grammar, rcore_behavior, catalog, loans, count) in [
        (
            "library",
            include_str!("../../../../fixtures/library/library.ecore"),
            include_str!("../../../../fixtures/xtext/composed-web/library.xtext"),
            include_str!("../../../../fixtures/library/library-behavior.rcore"),
            include_str!("../../../../fixtures/xtext/composed-web/catalog.library"),
            include_str!("../../../../fixtures/xtext/composed-web/loans.library"),
            6,
        ),
        (
            "library-strings",
            include_str!("../../../../fixtures/library/library.ecore"),
            include_str!("../../../../fixtures/xtext/composed-web/library-v52.xtext"),
            include_str!("../../../../fixtures/library/library-behavior.rcore"),
            include_str!("../../../../fixtures/xtext/composed-web/catalog-v52.library"),
            include_str!("../../../../fixtures/xtext/composed-web/loans.library"),
            6,
        ),
        (
            "expressions",
            include_str!("../../../../fixtures/xtext/current-actions/model.ecore"),
            include_str!("../../../../fixtures/xtext/composed-web/expressions.xtext"),
            include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore"),
            include_str!("../../../../fixtures/xtext/composed-web/catalog.kerml"),
            include_str!("../../../../fixtures/xtext/composed-web/more.kerml"),
            17,
        ),
    ] {
        if name == "library-strings" && profile != Profile::NativeV52 {
            continue;
        }
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ecore,
                grammar,
                rcore_behavior,
                profile,
            )
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let sources = [
            Source {
                uri: "catalog.model".into(),
                text: catalog.into(),
            },
            Source {
                uri: "more.model".into(),
                text: loans.into(),
            },
        ];
        let result = evaluate(&package, &sources);
        assert!(
            result.diagnostics.is_empty(),
            "{name}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.model.elements.len(), count, "{name}");
        if name.starts_with("library") {
            let named = |name: &str| {
                &result
                    .model
                    .elements
                    .iter()
                    .find(|element| element.values.get("name") == Some(&serde_json::json!(name)))
                    .unwrap()
                    .values
            };
            if name == "library-strings" {
                assert_eq!(named("dune_1")["title"], "Dune\nVolume 1");
                let invalid = [
                    Source {
                        uri: sources[0].uri.clone(),
                        text: catalog.replace(r"\n", r"\q"),
                    },
                    sources[1].clone(),
                ];
                assert!(!evaluate(&package, &invalid).diagnostics.is_empty());
                assert_eq!(evaluate(&package, &sources).model, result.model);
            }
            assert_eq!(named("dune_1")["edition"], 2);
            assert_eq!(named("spare_2")["edition"], 1);
            assert_eq!(named("spare_2")["title"], "untitled");
            assert_eq!(result.references.len(), 2);
        } else {
            let operators: Vec<_> = result
                .model
                .elements
                .iter()
                .filter(|element| element.class == "OperatorExpression")
                .map(|element| element.values["operator"].as_str().unwrap())
                .collect();
            assert_eq!(operators, ["+", "-", "all", "+", "all", "+"]);
            let literals: Vec<_> = result
                .model
                .elements
                .iter()
                .filter(|element| element.class == "Literal")
                .map(|element| element.values["value"].as_i64().unwrap())
                .collect();
            assert_eq!(literals, (1..=9).collect::<Vec<_>>());
        }
        for (index, source) in sources.iter().enumerate() {
            let elements: Vec<_> = result
                .model
                .elements
                .iter()
                .filter(|element| element.id.starts_with(&format!("{}#", source.uri)))
                .collect();
            for (ordinal, element) in elements.iter().enumerate() {
                assert_eq!(element.id, format!("{}#{ordinal}", source.uri));
                assert!(
                    element.start < element.end && element.end <= source.text.len(),
                    "{name}/{index}"
                );
            }
        }
        let negatives: &[&str] = if name.starts_with("library") {
            &[
                "library Bad { copy bad : Title edition ; }",
                "library Bad { loan bad copy Missing to ada_1; }",
                "library Bad { loan a copy dune_1 to ada_1; loan b copy dune_1 to ada_1; }",
            ]
        } else {
            &["Bad = (1 +);", "Bad = all of ();", "Bad = all 1 with ;"]
        };
        for invalid in negatives {
            let invalid_sources = [
                sources[0].clone(),
                Source {
                    uri: sources[1].uri.clone(),
                    text: (*invalid).into(),
                },
            ];
            assert!(
                !evaluate(&package, &invalid_sources).diagnostics.is_empty(),
                "{name}: {invalid}"
            );
            assert_eq!(evaluate(&package, &sources).model, result.model);
        }
        assert!(
            !evaluate_with_limits(
                &package,
                &sources,
                EvaluationLimits {
                    elements_per_resource: Some(1),
                    ..EvaluationLimits::native()
                }
            )
            .diagnostics
            .is_empty()
        );
        assert!(
            !evaluate_with_limits(
                &package,
                &sources,
                EvaluationLimits {
                    steps_per_resource: Some(1),
                    ..EvaluationLimits::native()
                }
            )
            .diagnostics
            .is_empty()
        );
        result.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&compiler::recompile(&package).unwrap(), &sources).model,
            result.model
        );
    }
}

#[test]
fn composed_web_examples_compile_and_execute() {
    verify_composed_web_examples(Profile::NativeV42);
}
#[test]
fn composed_web_examples_remain_cumulative_under_inherited_entry_profile() {
    verify_composed_web_examples(Profile::NativeV43);
}

#[test]
fn composed_web_examples_remain_cumulative_under_package_import_profile() {
    verify_composed_web_examples(Profile::NativeV44);
}

#[test]
fn composed_web_examples_remain_cumulative_under_local_type_scope_profile() {
    verify_composed_web_examples(Profile::NativeV45);
}

#[test]
fn composed_web_examples_remain_cumulative_under_enum_parser_profile() {
    verify_composed_web_examples(Profile::NativeV46);
}

#[test]
fn composed_web_examples_remain_cumulative_under_enum_terminal_profile() {
    verify_composed_web_examples(Profile::NativeV47);
}

#[test]
fn composed_web_examples_remain_cumulative_under_lexical_reachability_profile() {
    verify_composed_web_examples(Profile::NativeV48);
}

#[test]
fn composed_web_examples_remain_cumulative_under_default_hidden_policy_profile() {
    verify_composed_web_examples(Profile::NativeV49);
}

#[test]
fn composed_web_examples_remain_cumulative_under_terminal_parent_profile() {
    verify_composed_web_examples(Profile::NativeV50);
}

#[test]
fn composed_web_examples_remain_cumulative_under_declaration_library_profile() {
    verify_composed_web_examples(Profile::NativeV51);
}

#[test]
fn composed_web_examples_remain_cumulative_under_string_conversion_profile() {
    verify_composed_web_examples(Profile::NativeV52);
}
