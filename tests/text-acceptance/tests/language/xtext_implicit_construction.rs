use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/compatibility/graph.ecore");
const INITIALIZERS: &str = r#"{"version":1,"scope":"unique_names","derived":[],"validations":[],"initializers":[{"class":"Node","provider":{"name":"intersect.constant-attributes","version":"1"},"members":["count"],"values":{"count":9}}]}"#;
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/implicit-construction/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "implicit.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn first_assignments_share_construction_defaults_and_replay() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV27
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV28,
        )
        .unwrap();
    let text =
        include_str!("../../../../fixtures/xtext/implicit-construction/example.model").trim();
    let accepted = evaluate(&package, &source(text));
    assert!(
        accepted.diagnostics.is_empty(),
        "{:?}",
        accepted.diagnostics
    );
    assert_eq!(accepted.model.elements.len(), 5);
    assert_eq!(accepted.references.len(), 5);
    assert_eq!(
        accepted.model.elements[0].values["nodes"],
        json!([
            "implicit.model#1",
            "implicit.model#2",
            "implicit.model#3",
            "implicit.model#4"
        ])
    );
    for (index, span) in [
        "implicit a in g",
        "explicit b in g",
        "bool on c in g",
        "ref a d in g",
    ]
    .iter()
    .enumerate()
    {
        let element = &accepted.model.elements[index + 1];
        assert_eq!(element.id, format!("implicit.model#{}", index + 1));
        assert_eq!(&text[element.start..element.end], *span);
        assert_eq!(element.values["count"], 7);
        assert_eq!(element.values["graph"], "implicit.model#0");
    }
    assert_eq!(accepted.model.elements[3].values["active"], true);
    assert_eq!(
        accepted.model.elements[4].values["peer"],
        "implicit.model#1"
    );
    accepted.model.validate(&package.schema).unwrap();
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(evaluate(&replay, &source(text)).model, accepted.model);
    assert_eq!(
        evaluate_with_limits(
            &package,
            &source(text),
            EvaluationLimits {
                elements_per_resource: Some(5),
                ..EvaluationLimits::native()
            }
        )
        .model,
        accepted.model
    );
    for limits in [
        EvaluationLimits {
            elements_per_resource: Some(4),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..EvaluationLimits::native()
        },
    ] {
        let rejected = evaluate_with_limits(&package, &source(text), limits);
        assert!(!rejected.diagnostics.is_empty());
        assert!(rejected.model.elements.is_empty());
        assert_eq!(evaluate(&package, &source(text)).model, accepted.model);
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn implicit_rejection_rolls_back_constructed_objects_and_links() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV28,
        )
        .unwrap();
    for text in [
        "g { implicit }",
        "g { bool off c in g }",
        "g { ref missing d in g }",
        "g { explicit b in g implicit }",
        "g { implicit a in g } trailing",
    ] {
        let rejected = evaluate(&package, &source(text));
        assert!(!rejected.diagnostics.is_empty(), "{text}");
        if !text.contains("missing") {
            assert!(rejected.model.elements.is_empty(), "{text}");
        }
        let recovered = evaluate(&package, &source("g { explicit b in g implicit a in g }"));
        assert!(
            recovered.diagnostics.is_empty(),
            "{:?}",
            recovered.diagnostics
        );
        assert_eq!(recovered.model.elements.len(), 3);
        assert_eq!(recovered.model.elements[2].id, "implicit.model#2");
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn implicit_admission_preserves_concrete_type_and_flow_controls() {
    for ecore in [
        ECORE.replace(
            "name=\"Node\" eSuperTypes",
            "name=\"Node\" abstract=\"true\" eSuperTypes",
        ),
        ECORE.replace(
            "name=\"Node\" eSuperTypes",
            "name=\"Node\" interface=\"true\" eSuperTypes",
        ),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &ecore,
                GRAMMAR,
                RCORE_BEHAVIOR,
                Profile::NativeV28
            )
            .is_err()
        );
    }
    for grammar in [
        GRAMMAR.replace("'implicit' name=ID", "'implicit' absent=ID"),
        GRAMMAR.replace("'implicit' name=ID", "'implicit' name=ID {g::Node}"),
        GRAMMAR
            .replace("'implicit' name=ID", "'empty'")
            .replace("graph=[g::Graph|ID]", "'graph'"),
        GRAMMAR.replace("'implicit' name=ID", "'implicit' name+=ID"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV28
            )
            .is_err(),
            "{grammar}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn implicit_and_explicit_roots_share_initializers() {
    // Establish that the deferred-link initializer boundary predates NativeV28.
    let legacy_grammar = GRAMMAR
        .replace("'implicit'", "{g::Node} 'implicit'")
        .replace("'bool'", "{g::Node} 'bool'")
        .replace("'ref'", "{g::Node} 'ref'");
    let legacy =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &legacy_grammar,
            INITIALIZERS,
            Profile::NativeV27,
        )
        .unwrap();
    let legacy_result = evaluate(&legacy, &source("g { implicit a in g explicit b in g }"));
    assert!(
        legacy_result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Missing reciprocal opposite"))
    );
    // Current initialization validation precedes deferred reciprocal linking.
    // Retain this explicit rejection until that separate lifecycle is supported.
    let graph =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            INITIALIZERS,
            Profile::NativeV28,
        )
        .unwrap();
    let deferred = evaluate(&graph, &source("g { implicit a in g explicit b in g }"));
    assert!(
        deferred
            .diagnostics
            .iter()
            .any(|d| d.message.contains("Missing reciprocal opposite"))
    );
    let grammar = r#"grammar roots hidden(WS) import "urn:intersect:compatibility:graph" as g
Root returns g::Node: 'implicit' name=ID | {g::Node} 'explicit' name=ID;
terminal ID: ('a'..'z')+; terminal WS: ' '+;"#;
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            grammar,
            INITIALIZERS,
            Profile::NativeV28,
        )
        .unwrap();
    for text in ["implicit a", "explicit a"] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        let element = &result.model.elements[0];
        assert_eq!(element.values["count"], 9);
        assert_eq!(element.initialization.as_ref().unwrap().object, element.id);
        assert_eq!(element.id, "implicit.model#0");
        result.model.validate(&package.schema).unwrap();
    }
}
