use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/nullable-continuations/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "nullable.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn nested_nullable_fragments_keep_prediction_and_model_contracts() {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV33,
    )
    .unwrap_err();
    assert!(format!("{old:?}").contains("follow conflict"));
    let alternative = GRAMMAR.replace(
        "Prefix returns g::Node: Detail;",
        "Prefix returns g::Node: Detail | 'q' labels+='q';",
    );
    for grammar in [GRAMMAR.to_owned(), alternative] {
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV34,
            )
            .unwrap();
        for (prefix, suffix) in [("one", "c"), ("two", "d")] {
            for count in 0..=2 {
                let text = format!("{prefix} {}a {suffix} alice", "a b ".repeat(count));
                let result = evaluate(&package, &source(&text));
                assert!(
                    result.diagnostics.is_empty(),
                    "{text}: {:?}",
                    result.diagnostics
                );
                assert_eq!(result.model.elements.len(), 1);
                let root = &result.model.elements[0];
                assert_eq!(root.id, "nullable.model#0");
                assert_eq!(root.values["name"], "alice");
                assert_eq!(root.values["labels"], serde_json::json!(vec!["b"; count]));
                assert_eq!((root.start, root.end), (0, text.len()));
                result.model.validate(&package.schema).unwrap();
                assert_eq!(
                    evaluate(&compiler::recompile(&package).unwrap(), &source(&text)).model,
                    result.model
                );
            }
        }
        if grammar.contains("'q'") {
            let result = evaluate(&package, &source("one q q a c alice"));
            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(
                result.model.elements[0].values["labels"],
                serde_json::json!(["q"])
            );
        }
        for text in [
            "one a d alice",
            "two a c alice",
            "one a b a",
            "one a c alice trailing",
        ] {
            let rejected = evaluate(&package, &source(text));
            assert!(!rejected.diagnostics.is_empty());
            assert!(rejected.model.elements.is_empty());
        }
        let text = include_str!("../../../../fixtures/xtext/nullable-continuations/example.model");
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
pub fn nullable_wrappers_do_not_hide_ambiguity_or_nonprogress() {
    let ambiguous = GRAMMAR.replace(
        "'two' Prefix 'a' 'd'",
        "'two' Prefix ('a' labels+='b')* 'a' 'd'",
    );
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &ambiguous,
            RCORE_BEHAVIOR,
            Profile::NativeV34,
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("Ambiguous parser prediction"),
        "{error:?}"
    );
    let cycle = GRAMMAR.replace(
        "Detail returns g::Node: ('a' labels+='b')*;",
        "Detail returns g::Node: Prefix;",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &cycle,
            RCORE_BEHAVIOR,
            Profile::NativeV34
        )
        .is_err()
    );
    let nullable_repeat = GRAMMAR.replace("'one' Prefix", "'one' Prefix*");
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &nullable_repeat,
            RCORE_BEHAVIOR,
            Profile::NativeV34,
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("Nullable repetitions"),
        "{error:?}"
    );
    // With no fragments, aggregate nullable wrappers still require FOLLOW proofs.
    let plain = r#"grammar plain hidden(WS) import "urn:intersect:compatibility:graph" as g
        Root returns g::Node: (('a' labels+='b')*) 'a' 'c' name=ID;
        terminal ID: ('a'..'z')+; terminal WS: ' '+;"#;
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            plain,
            RCORE_BEHAVIOR,
            Profile::NativeV34,
        )
        .unwrap();
    assert!(
        evaluate(&package, &source("a b a c alice"))
            .diagnostics
            .is_empty()
    );
    let empty = evaluate(&package, &source("a c alice"));
    assert!(empty.diagnostics.is_empty(), "{:?}", empty.diagnostics);
    assert_eq!(
        empty.model.elements[0].values["labels"],
        serde_json::json!([])
    );
    let ambiguous = plain.replace(" 'a' 'c' name", " ('a' labels+='b')* 'a' 'c' name");
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &ambiguous,
            RCORE_BEHAVIOR,
            Profile::NativeV34,
        )
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("Ambiguous parser prediction"),
        "{error:?}"
    );
}
