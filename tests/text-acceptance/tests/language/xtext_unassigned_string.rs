use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/unassigned-string/model.xtext");
const TEXT: &str = include_str!("../../../../fixtures/xtext/unassigned-string/example.model");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "helper.model".into(),
        text: text.into(),
    }]
}
fn typed(grammar: &str) -> String {
    grammar
        .replace(
            "Root returns",
            "import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nRoot returns",
        )
        .replace("Prefix:", "Prefix returns ecore::EString:")
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn string_helpers_preserve_owner_in_objects_fragments_and_actions() {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV32,
    )
    .unwrap_err();
    assert!(format!("{old:?}").contains("Unassigned rule calls"));
    let fragment = GRAMMAR.replace(
        "Root returns g::Node: Prefix",
        "Root returns g::Node: Body;\nfragment Body returns g::Node: Prefix",
    );
    let action = GRAMMAR.replace("Prefix name=ID", "Prefix {g::Node} name=ID");
    for grammar in [GRAMMAR.to_owned(), typed(GRAMMAR), fragment, action] {
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV33,
            )
            .unwrap();
        let result = evaluate(&package, &source(TEXT));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        let root = &result.model.elements[0];
        assert_eq!(root.id, "helper.model#0");
        assert_eq!(root.values["name"], "alice");
        assert_eq!(root.values["labels"], serde_json::json!(["bob", "carol"]));
        assert_eq!((root.start, root.end), (0, TEXT.len()));
        result.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&compiler::recompile(&package).unwrap(), &source(TEXT)).model,
            result.model
        );
        for text in [
            "begin alice",
            "begin alice references",
            "begin alice wrong bob",
            "begin alice references bob trailing",
        ] {
            let rejected = evaluate(&package, &source(text));
            assert!(!rejected.diagnostics.is_empty());
            assert!(rejected.model.elements.is_empty());
        }
        let limited = evaluate_with_limits(
            &package,
            &source(TEXT),
            EvaluationLimits {
                steps_per_resource: Some(1),
                ..EvaluationLimits::native()
            },
        );
        assert!(!limited.diagnostics.is_empty());
        assert!(limited.model.elements.is_empty());
        assert_eq!(evaluate(&package, &source(TEXT)).model, result.model);
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn unsupported_calls_and_nullable_helpers_remain_rejected() {
    let object = GRAMMAR.replace("Prefix: 'begin';", "Prefix returns g::Node: name=ID;");
    let numeric = typed(GRAMMAR).replace(
        "Prefix returns ecore::EString: 'begin';",
        "Prefix returns ecore::EDouble: INT '.' INT;\nterminal INT: ('0'..'9')+;",
    );
    for grammar in [object, numeric] {
        let error =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV33,
            )
            .unwrap_err();
        assert!(
            format!("{error:?}").contains("Unassigned rule calls"),
            "{error:?}"
        );
    }
    let nullable = GRAMMAR.replace("Prefix: 'begin';", "Prefix: 'begin'?;");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &nullable,
            RCORE_BEHAVIOR,
            Profile::NativeV33
        )
        .is_err()
    );
}
