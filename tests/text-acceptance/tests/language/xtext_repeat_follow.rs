use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/repeat-follow/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "repeat.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn repeat_and_skip_use_following_tokens_without_speculative_writes() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV29
        )
        .is_err()
    );
    for cardinality in ["*", "?", "+"] {
        let grammar = GRAMMAR.replace(")*", &format!("){cardinality}"));
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV30,
            )
            .unwrap();
        for (text, count) in [
            ("a c", 0),
            ("a b a c", 1),
            (
                include_str!("../../../../fixtures/xtext/repeat-follow/example.model").trim(),
                2,
            ),
        ] {
            let result = evaluate(&package, &source(text));
            if (cardinality == "+" && count == 0) || (cardinality == "?" && count == 2) {
                assert!(!result.diagnostics.is_empty());
                assert!(result.model.elements.is_empty());
                continue;
            }
            assert!(
                result.diagnostics.is_empty(),
                "{cardinality} {text}: {:?}",
                result.diagnostics
            );
            assert_eq!(result.model.elements.len(), 1);
            let root = &result.model.elements[0];
            assert_eq!(root.id, "repeat.model#0");
            assert_eq!(root.values["name"], "c");
            assert_eq!(root.values["labels"], serde_json::json!(vec!["b"; count]));
            assert_eq!(&text[root.start..root.end], text);
            result.model.validate(&package.schema).unwrap();
            assert_eq!(
                evaluate(&compiler::recompile(&package).unwrap(), &source(text)).model,
                result.model
            );
        }
        for text in ["a", "a b a", "a b a d", "a c trailing"] {
            let rejected = evaluate(&package, &source(text));
            assert!(!rejected.diagnostics.is_empty());
            assert!(rejected.model.elements.is_empty());
        }
        let limited = evaluate_with_limits(
            &package,
            &source("a b a c"),
            EvaluationLimits {
                steps_per_resource: Some(1),
                ..EvaluationLimits::native()
            },
        );
        assert!(!limited.diagnostics.is_empty());
        assert!(limited.model.elements.is_empty());
        assert!(
            evaluate(&package, &source("a b a c"))
                .diagnostics
                .is_empty()
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn ambiguous_or_nonprogressing_repeats_remain_rejected() {
    for grammar in [
        GRAMMAR.replace(
            "('a' labels+='b')* 'a' name='c'",
            "('a' labels+='b')* ('a' labels+='b')* name='c'",
        ),
        GRAMMAR.replace("('a' labels+='b')*", "('a'?)*"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV30
            )
            .is_err(),
            "{grammar}"
        );
    }
}
