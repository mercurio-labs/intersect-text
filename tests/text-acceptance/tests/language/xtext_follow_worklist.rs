use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/follow-worklist/model.xtext");
const TEXT: &str = include_str!("../../../../fixtures/xtext/follow-worklist/example.model");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "follow.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn changed_callers_preserve_model_and_old_profile_budget() {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV30,
    );
    assert!(
        old.is_err(),
        "previous profile must retain its budget boundary"
    );
    assert!(format!("{:?}", old.err()).contains("Grammar prediction budget exceeded"));
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV31,
        )
        .unwrap();
    let result = evaluate(&package, &source(TEXT));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    let root = &result.model.elements[0];
    assert_eq!(root.id, "follow.model#0");
    assert_eq!(root.values["name"], "alice");
    assert_eq!(root.start, 0);
    assert_eq!(root.end, TEXT.len());
    result.model.validate(&package.schema).unwrap();
    assert_eq!(
        evaluate(&compiler::recompile(&package).unwrap(), &source(TEXT)).model,
        result.model
    );
    for text in [
        TEXT.replacen("x ", "", 1),
        format!("{TEXT} trailing"),
        "x".into(),
    ] {
        let rejected = evaluate(&package, &source(&text));
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
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn ambiguous_fragment_continuations_remain_rejected() {
    let grammar = r#"grammar ambiguous hidden(WS) import "urn:intersect:compatibility:graph" as g
        Root returns g::Node: Part 'x';
        fragment Part returns g::Node: name='a' 'x'?;
        terminal WS: ' '+;"#;
    for profile in [Profile::NativeV30, Profile::NativeV31] {
        let error =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                grammar,
                RCORE_BEHAVIOR,
                profile,
            )
            .unwrap_err();
        assert!(
            format!("{error:?}").contains("Fragment caller follow conflict"),
            "{error:?}"
        );
    }
}
