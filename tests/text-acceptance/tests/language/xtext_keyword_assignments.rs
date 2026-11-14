use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
const XTEXT: &str = r#"grammar keywords hidden(WS) import "urn:intersect:compatibility:graph" as g
Root returns g::Node: name='root' (labels+='all')* (labels+=('one'|'two'))* ';';
terminal WS: ' '+;"#;
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "keywords.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn keyword_values_use_existing_assignment_and_replay() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV26
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV27,
        )
        .unwrap();
    for (text, labels) in [
        ("root;", json!([])),
        (
            "root all all one two one;",
            json!(["all", "all", "one", "two", "one"]),
        ),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        assert_eq!(result.model.elements[0].values["name"], "root");
        assert_eq!(result.model.elements[0].values["labels"], labels);
        assert_eq!(result.model.elements[0].values["count"], 7);
        result.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&compiler::recompile(&package).unwrap(), &source(text)).model,
            result.model
        );
    }
    for text in [
        "root all wrong;",
        "root all one",
        "root all one; trailing",
        "wrong;",
    ] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty());
    }
    let result = evaluate_with_limits(
        &package,
        &source("root all;"),
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..EvaluationLimits::native()
        },
    );
    assert!(!result.diagnostics.is_empty());
    assert!(result.model.elements.is_empty());
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn keyword_assignment_checks_type_and_multiplicity() {
    for replacement in [
        "count='root'",
        "active='root'",
        "peer='root'",
        "name+='root'",
        "labels='root'",
    ] {
        let invalid = XTEXT.replace("name='root'", replacement);
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &invalid,
                RCORE_BEHAVIOR,
                Profile::NativeV27
            )
            .is_err(),
            "{replacement}"
        );
    }
    for flags in ["changeable=\"false\"", "derived=\"true\""] {
        let invalid = ECORE.replace("name=\"labels\"", &format!("name=\"labels\" {flags}"));
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &invalid,
                XTEXT,
                RCORE_BEHAVIOR,
                Profile::NativeV27
            )
            .is_err()
        );
    }
    for rhs in ["('one' 'two')", "('one'|'one')"] {
        let invalid = XTEXT.replace("('one'|'two')", rhs);
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &invalid,
                RCORE_BEHAVIOR,
                Profile::NativeV27
            )
            .is_err(),
            "{rhs}"
        );
    }
}
