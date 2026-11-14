use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/parser-dispatch/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/parser-dispatch/model-behavior.rcore");
fn grammar() -> String {
    include_str!("../../../../fixtures/xtext/parser-dispatch/model.xtext")
        .replace("with org.eclipse.xtext.common.Terminals", "hidden(WS)")
        .replace("Type | Feature;", "Type | Feature | '(' Member ')';")
        + "terminal ID: ('a'..'z' | 'A'..'Z')+; terminal WS: ' '+;"
}
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "forward.kerml".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn parenthesized_forwarding_keeps_returned_identity_and_spans() {
    let grammar = grammar();
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV24
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV25,
        )
        .unwrap();
    assert_eq!(package.sources["xtext"], grammar);
    let text = "package Demo { ((abstract type Vehicle;)) (feature wheel;) }";
    let sources = source(text);
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    for (i, class) in ["Model", "Type", "Feature"].iter().enumerate() {
        assert_eq!(result.model.elements[i].class, *class);
        assert_eq!(result.model.elements[i].id, format!("forward.kerml#{i}"));
    }
    assert_eq!(
        result.model.elements[0].values["members"],
        json!(["forward.kerml#1", "forward.kerml#2"])
    );
    assert_eq!(result.model.elements[1].values["isAbstract"], true);
    for (index, expected) in [(1, "abstract type Vehicle;"), (2, "feature wheel;")] {
        let element = &result.model.elements[index];
        assert_eq!(&text[element.start..element.end], expected);
    }
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(evaluate(&replay, &sources).model, result.model);
    let unwrapped = evaluate(
        &package,
        &source("package Demo { abstract type Vehicle; feature wheel; }"),
    );
    assert!(unwrapped.diagnostics.is_empty());
    for (actual, plain) in result.model.elements.iter().zip(&unwrapped.model.elements) {
        assert_eq!(actual.id, plain.id);
        assert_eq!(actual.values, plain.values);
        assert_eq!(actual.type_identity, plain.type_identity);
        assert_eq!(actual.assignment_state, plain.assignment_state);
    }
    for text in [
        "package Demo { () }",
        "package Demo { ((type A;) }",
        "package Demo { (type A; feature b;) }",
        "package Demo { type A; (feature b; }",
        "package Demo { (type A;) } trailing",
    ] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty());
    }
    for limits in [
        EvaluationLimits {
            elements_per_resource: Some(2),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            nesting: 2,
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..EvaluationLimits::native()
        },
    ] {
        let result = evaluate_with_limits(&package, &sources, limits);
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty());
    }
    assert_eq!(evaluate(&package, &sources).model, result.model);
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn forwarding_requires_one_typed_object_on_every_path() {
    let base = grammar();
    for body in [
        "Type | Feature | '(' Member? ')'",
        "Type | Feature | '(' Member Member ')'",
        "Type | Feature | '(' ')'",
        "Type | Feature | '(' Model ')'",
        "Type | Feature | '(' ID ')'",
        "Type | Feature | '(' Member* ')'",
        "Type | Feature | Member",
        "Type | Feature | '(' name=ID Member ')'",
    ] {
        let invalid = base.replace("Type | Feature | '(' Member ')'", body);
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &invalid,
                RCORE_BEHAVIOR,
                Profile::NativeV25
            )
            .is_err(),
            "{body}"
        );
    }
}
