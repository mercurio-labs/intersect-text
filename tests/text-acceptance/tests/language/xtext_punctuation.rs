//! X-D04 acceptance controls: initially red at explicit keyword admission.
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/punctuation-keywords/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/punctuation-keywords/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/punctuation-keywords/model-behavior.rcore");
const TEXT: &str = include_str!("../../../../fixtures/xtext/punctuation-keywords/example.kerml");
fn package(xtext: &str) -> Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        xtext,
        RCORE_BEHAVIOR,
        Profile::NativeV1,
    )
    .unwrap()
}
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "punctuation.kerml".into(),
        text: text.into(),
    }]
}
#[test]
fn punctuation_keywords_preserve_exact_source_spans_and_boolean_state() {
    let p = package(XTEXT);
    let result = evaluate(&p, &source(TEXT));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 5);
    for (index, class, name) in [
        (0, "Model", "Demo"),
        (1, "Import", "Parts"),
        (2, "Import", "Units"),
        (3, "Specialization", "Wheel"),
        (4, "Specialization", "front"),
    ] {
        let object = &result.model.elements[index];
        assert_eq!(object.id, format!("punctuation.kerml#{index}"));
        assert_eq!(object.class, class);
        assert_eq!(object.values["name"], name);
        assert_eq!(object.type_identity, p.schema.identity(&object.class));
        let declaration = &p.schema.declarations[&object.class];
        assert_eq!(
            declaration.namespace,
            "urn:intersect:xtext:mini-kerml:punctuation:1"
        );
        assert_eq!(declaration.source.uri, "memory:model.rcore");
    }
    for (index, spelling) in [
        (1, "import Parts::**;"),
        (2, "import Units::*;"),
        (3, "specialize Wheel :> Parts;"),
        (4, "redefine front :>> Wheel;"),
    ] {
        let object = &result.model.elements[index];
        assert_eq!(&TEXT[object.start..object.end], spelling);
    }
    assert_eq!(
        result.model.elements[0].values["members"],
        json!([
            "punctuation.kerml#1",
            "punctuation.kerml#2",
            "punctuation.kerml#3",
            "punctuation.kerml#4"
        ])
    );
    for (index, field, value, set) in [
        (1, "recursive", true, true),
        (1, "all", false, false),
        (2, "recursive", false, false),
        (2, "all", true, true),
        (3, "redefinition", false, false),
        (4, "redefinition", true, true),
    ] {
        let object = &result.model.elements[index];
        assert_eq!(object.values[field], value);
        assert_eq!(
            object
                .assignment_state
                .as_ref()
                .unwrap()
                .explicitly_set(field),
            set
        );
    }
    assert_eq!(result.model.elements[3].values["targetName"], "Parts");
    assert_eq!(result.model.elements[4].values["targetName"], "Wheel");
    let replay = compiler::recompile(&p).unwrap();
    assert_eq!(replay.sources, p.sources);
    assert_eq!(evaluate(&replay, &source(TEXT)).model, result.model);
    let mut direct = intersect_model::core::model::Model::default();
    for object in &result.model.elements {
        direct
            .create(
                &p.schema,
                &object.id,
                &object.class,
                "punctuation.kerml",
                object.start,
                object.end,
            )
            .unwrap();
    }
    direct
        .edit(
            &p.schema,
            &[
                ("punctuation.kerml#0".into(), "name".into(), json!("Demo")),
                (
                    "punctuation.kerml#0".into(),
                    "members".into(),
                    json!([
                        "punctuation.kerml#1",
                        "punctuation.kerml#2",
                        "punctuation.kerml#3",
                        "punctuation.kerml#4"
                    ]),
                ),
                ("punctuation.kerml#1".into(), "name".into(), json!("Parts")),
                (
                    "punctuation.kerml#1".into(),
                    "recursive".into(),
                    json!(true),
                ),
                ("punctuation.kerml#2".into(), "name".into(), json!("Units")),
                ("punctuation.kerml#2".into(), "all".into(), json!(true)),
                ("punctuation.kerml#3".into(), "name".into(), json!("Wheel")),
                (
                    "punctuation.kerml#3".into(),
                    "targetName".into(),
                    json!("Parts"),
                ),
                ("punctuation.kerml#4".into(), "name".into(), json!("front")),
                (
                    "punctuation.kerml#4".into(),
                    "targetName".into(),
                    json!("Wheel"),
                ),
                (
                    "punctuation.kerml#4".into(),
                    "redefinition".into(),
                    json!(true),
                ),
            ],
        )
        .unwrap();
    for (a, b) in result.model.elements.iter().zip(&direct.elements) {
        assert_eq!(a.values, b.values);
        assert_eq!(a.type_identity, b.type_identity);
        assert_eq!(a.assignment_state, b.assignment_state);
    }
}
#[test]
fn punctuation_does_not_join_across_trivia_or_split_a_longer_keyword() {
    let p = package(XTEXT);
    for body in [
        "import Parts: :**;",
        "import Parts:/*gap*/:**;",
        "import Parts:: * *;",
        "import Parts::;",
        "import Parts:;",
        "specialize Wheel : > Parts;",
        "specialize Wheel :>> Parts;",
        "redefine Wheel :> > Parts;",
        "redefine Wheel :> Parts;",
        "import import::*;",
    ] {
        let result = evaluate(&p, &source(&format!("package Bad {{ {body} }}")));
        assert!(!result.diagnostics.is_empty(), "{body}");
        assert!(result.model.elements.is_empty(), "{body}");
    }
    let good = evaluate(
        &p,
        &source("package P { /*::***/ import Parts /*gap*/ :: /*gap*/ **; }"),
    );
    assert!(good.diagnostics.is_empty(), "{:?}", good.diagnostics);
    assert_eq!(good.model.elements[1].values["recursive"], true);
    // STRING remains one quoted token even when its value spells active punctuation.
    let quoted = package(&XTEXT.replace("'import' name=ID", "'import' name=STRING"));
    let result = evaluate(&quoted, &source("package P { import \"::**:>>\"; }"));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[1].values["name"], "::**:>>");
}
#[test]
fn keyword_tokens_keep_inclusive_existing_budget_policy() {
    let p = package(XTEXT);
    let exact = EvaluationLimits {
        tokens_per_resource: Some(24),
        elements_per_resource: Some(5),
        ..EvaluationLimits::native()
    };
    let result = evaluate_with_limits(&p, &source(TEXT), exact);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    for limits in [
        EvaluationLimits {
            tokens_per_resource: Some(23),
            ..exact
        },
        EvaluationLimits {
            elements_per_resource: Some(4),
            ..exact
        },
    ] {
        let rejected = evaluate_with_limits(&p, &source(TEXT), limits);
        assert!(rejected.model.elements.is_empty());
        assert!(
            rejected
                .diagnostics
                .iter()
                .any(|d| d.message.contains("budget"))
        );
    }
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR
        )
        .is_err()
    );
}
#[test]
fn unsupported_keyword_trivia_and_comment_delimiters_reject_explicitly() {
    for keyword in ["", "a b", "/*", "//", "x::y"] {
        let changed = XTEXT.replace("'::'", &format!("'{keyword}'"));
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &changed,
                RCORE_BEHAVIOR,
                Profile::NativeV1
            )
            .is_err(),
            "{keyword}"
        );
    }
}
