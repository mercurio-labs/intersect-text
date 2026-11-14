use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/caller-repeat/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "caller.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn caller_suffixes_resolve_fragment_repeat_decisions() {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV31,
    )
    .unwrap_err();
    assert!(format!("{old:?}").contains("Fragment caller follow conflict"));
    for cardinality in ["*", "?", "+"] {
        let grammar = GRAMMAR.replace(")*", &format!("){cardinality}"));
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV32,
            )
            .unwrap();
        for (prefix, suffix) in [("one", "c"), ("two", "d")] {
            for count in 0..=2 {
                let text = format!("{prefix} start {}a {suffix}", "a b ".repeat(count));
                let result = evaluate(&package, &source(&text));
                if (cardinality == "+" && count == 0) || (cardinality == "?" && count == 2) {
                    assert!(!result.diagnostics.is_empty());
                    assert!(result.model.elements.is_empty());
                    continue;
                }
                assert!(
                    result.diagnostics.is_empty(),
                    "{text}: {:?}",
                    result.diagnostics
                );
                assert_eq!(result.model.elements.len(), 1);
                let root = &result.model.elements[0];
                assert_eq!(root.id, "caller.model#0");
                assert_eq!(root.values["name"], "start");
                assert_eq!(root.values["labels"], serde_json::json!(vec!["b"; count]));
                assert_eq!((root.start, root.end), (0, text.len()));
                result.model.validate(&package.schema).unwrap();
                assert_eq!(
                    evaluate(&compiler::recompile(&package).unwrap(), &source(&text)).model,
                    result.model
                );
            }
        }
        for text in [
            "one start a d",
            "two start a c",
            "one start a b a",
            "one start a c trailing",
        ] {
            let result = evaluate(&package, &source(text));
            assert!(!result.diagnostics.is_empty());
            assert!(result.model.elements.is_empty());
        }
        let text = "one start a b a c";
        let limited = evaluate_with_limits(
            &package,
            &source(text),
            EvaluationLimits {
                steps_per_resource: Some(1),
                ..EvaluationLimits::native()
            },
        );
        assert!(!limited.diagnostics.is_empty());
        assert!(limited.model.elements.is_empty());
        assert!(evaluate(&package, &source(text)).diagnostics.is_empty());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn ambiguous_caller_partition_and_nonprogress_remain_rejected() {
    let ambiguous = GRAMMAR.replace(
        "'two' Part 'a' 'd'",
        "'two' Part ('a' labels+='b')* 'a' 'd'",
    );
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &ambiguous,
            RCORE_BEHAVIOR,
            Profile::NativeV32,
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("Ambiguous parser prediction"),
        "{error:?}"
    );
    let nullable = GRAMMAR.replace("('a' labels+='b')*", "('a'?)*");
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &nullable,
            RCORE_BEHAVIOR,
            Profile::NativeV32,
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("Nullable repetitions"),
        "{error:?}"
    );
}
