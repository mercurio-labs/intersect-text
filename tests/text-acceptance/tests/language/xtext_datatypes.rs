use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/qualified-names/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/qualified-names/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/qualified-names/model-behavior.rcore");
fn compile(grammar: &str) -> intersect_text::package::Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        grammar,
        RCORE_BEHAVIOR,
        Profile::NativeV1,
    )
    .unwrap()
}
fn source(uri: &str, text: &str) -> Source {
    Source {
        uri: uri.into(),
        text: text.into(),
    }
}
#[test]
fn qualified_attributes_and_references_share_scalar_execution_without_objects() {
    let package = compile(XTEXT);
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert_eq!(package.sources["xtext"], XTEXT);
    let sources = [
        source(
            "example.kerml",
            include_str!("../../../../fixtures/xtext/qualified-names/example.kerml"),
        ),
        source(
            "more.kerml",
            include_str!("../../../../fixtures/xtext/qualified-names/more.kerml"),
        ),
    ];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 6);
    for (index, id, class, name) in [
        (0, "example.kerml#0", "Model", "Demo"),
        (1, "example.kerml#1", "Type", "Parts::Wheel"),
        (2, "example.kerml#2", "Feature", "front"),
        (3, "example.kerml#3", "Feature", "distance"),
        (4, "more.kerml#0", "Model", "Library"),
        (5, "more.kerml#1", "Type", "Units::Length"),
    ] {
        let element = &result.model.elements[index];
        assert_eq!(element.id, id);
        assert_eq!(element.class, class);
        assert_eq!(element.values["name"], json!(name));
    }
    assert_eq!(
        result.model.elements[2].values["target"],
        json!("example.kerml#1")
    );
    assert_eq!(
        result.model.elements[3].values["target"],
        json!("more.kerml#1")
    );
    assert_eq!(result.references.len(), 2);
    for (reference, spelling, target) in [
        (&result.references[0], "Parts::Wheel", "example.kerml#1"),
        (&result.references[1], "Units::Length", "more.kerml#1"),
    ] {
        assert_eq!(reference.uri, "example.kerml");
        assert_eq!(&sources[0].text[reference.start..reference.end], spelling);
        assert_eq!(reference.target, target);
    }
    let element = &result.model.elements[1];
    assert_eq!(
        &sources[0].text[element.start..element.end],
        "type Parts::Wheel;"
    );
    let mut direct = intersect_model::core::model::Model::default();
    for e in &result.model.elements {
        direct
            .create(
                &package.schema,
                &e.id,
                &e.class,
                &e.resource,
                e.start,
                e.end,
            )
            .unwrap();
    }
    direct
        .edit(
            &package.schema,
            &[
                ("example.kerml#0".into(), "name".into(), json!("Demo")),
                (
                    "example.kerml#0".into(),
                    "members".into(),
                    json!(["example.kerml#1", "example.kerml#2", "example.kerml#3"]),
                ),
                (
                    "example.kerml#1".into(),
                    "name".into(),
                    json!("Parts::Wheel"),
                ),
                ("example.kerml#2".into(), "name".into(), json!("front")),
                (
                    "example.kerml#2".into(),
                    "target".into(),
                    json!("example.kerml#1"),
                ),
                ("example.kerml#3".into(), "name".into(), json!("distance")),
                (
                    "example.kerml#3".into(),
                    "target".into(),
                    json!("more.kerml#1"),
                ),
                ("more.kerml#0".into(), "name".into(), json!("Library")),
                (
                    "more.kerml#0".into(),
                    "members".into(),
                    json!(["more.kerml#1"]),
                ),
                ("more.kerml#1".into(), "name".into(), json!("Units::Length")),
            ],
        )
        .unwrap();
    for (actual, expected) in result.model.elements.iter().zip(direct.elements.iter()) {
        assert_eq!(actual.values, expected.values);
        assert_eq!(actual.assignment_state, expected.assignment_state);
        assert_eq!(actual.type_identity, expected.type_identity);
    }
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(
        serde_json::to_value(&replay).unwrap(),
        serde_json::to_value(&package).unwrap()
    );
    assert_eq!(evaluate(&replay, &sources).model, result.model);
    let renamed = compile(
        &XTEXT
            .replace("QualifiedName", "Spelling")
            .replace(" as ecore", " as scalar")
            .replace("ecore::", "scalar::"),
    );
    assert_eq!(
        evaluate(&renamed, &sources)
            .model
            .elements
            .iter()
            .map(|e| &e.values)
            .collect::<Vec<_>>(),
        result
            .model
            .elements
            .iter()
            .map(|e| &e.values)
            .collect::<Vec<_>>()
    );
}
#[test]
fn scalar_conversion_matches_raw_and_nested_pinned_token_observations() {
    for (body, text, expected) in [
        (
            "STRING | INT",
            r#"package Demo { type "B\tC"; }"#,
            r#""B\tC""#,
        ),
        (
            "STRING | INT",
            "package Demo { type 999999999999999999999999; }",
            "999999999999999999999999",
        ),
        (
            "STRING '::' ID",
            r#"package Demo { type "😀"::A; }"#,
            r#""😀"::A"#,
        ),
        (
            "ID ('::' ID)*",
            "package Demo { type P/*gap*/:: A; }",
            "P :: A",
        ),
    ] {
        let grammar = XTEXT.replace("ID ('::' ID)*", body);
        let result = evaluate(&compile(&grammar), &[source("test", text)]);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 2);
        assert_eq!(result.model.elements[1].values["name"], json!(expected));
    }
    let nested = XTEXT.replace("ID ('::' ID)*", "Inner '.'")
        + "\nInner returns ecore::EString: ID ('::' ID)*;";
    let result = evaluate(
        &compile(&nested),
        &[source("test", "package Demo { type A/*gap*/::B.; }")],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[1].values["name"], json!("A ::B ."));
}
#[test]
fn malformed_scalar_input_rolls_back_and_exact_reference_spelling_remains_significant() {
    let package = compile(XTEXT);
    for text in [
        "package Demo { type P::; }",
        "package Demo { type P: :A; }",
        "package Demo { type package; }",
        "package Demo { type P::A; feature f : P::; }",
    ] {
        let result = evaluate(&package, &[source("bad", text)]);
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty(), "{text}");
    }
    let matched = evaluate(
        &package,
        &[source(
            "good",
            "package Demo { type P/*gap*/::A; feature f : P /*other*/::A; }",
        )],
    );
    assert!(matched.diagnostics.is_empty(), "{:?}", matched.diagnostics);
    assert_eq!(matched.model.elements[2].values["target"], json!("good#1"));
    let mismatch = evaluate(
        &package,
        &[source(
            "bad",
            "package Demo { type P ::A; feature f : P::A; }",
        )],
    );
    assert!(!mismatch.diagnostics.is_empty());
}
#[test]
fn invalid_scalar_grammar_contracts_are_explicitly_rejected() {
    for grammar in [
        XTEXT.replace("returns ecore::EString", "returns ecore::EInt"),
        XTEXT.replace("QualifiedName", "ID"),
        XTEXT.replace(
            "Model returns k::Model:",
            "First returns ecore::EString: ID;\nModel returns k::Model:",
        ),
        XTEXT.replace("QualifiedName returns", "fragment QualifiedName returns"),
        XTEXT.replace("ID ('::' ID)*", "ID?"),
        XTEXT.replace("ID ('::' ID)*", "QualifiedName"),
        XTEXT.replace("ID ('::' ID)*", "ID QualifiedName?"),
        XTEXT.replace("ID ('::' ID)*", "Type"),
        XTEXT.replace("ID ('::' ID)*", "name=ID"),
        XTEXT.replace("[k::Type|QualifiedName]", "[k::Type|Type]"),
        XTEXT.replace("name=QualifiedName", "isAbstract=QualifiedName"),
        XTEXT.replace(
            "name=QualifiedName ';'",
            "name=QualifiedName '::' name=ID ';'",
        ),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV1
            )
            .is_err(),
            "{grammar}"
        );
    }
}

#[test]
fn scalar_calls_preserve_token_object_and_nesting_budget_boundaries() {
    use intersect_text::{EvaluationLimits, evaluate_with_limits};
    let package = compile(XTEXT);
    let sources = [source(
        "exact",
        "package Demo { type Parts::Wheel; feature front : Parts::Wheel; }",
    )];
    let exact = EvaluationLimits {
        tokens_per_resource: Some(16),
        elements_per_resource: Some(3),
        nesting: 4,
        ..EvaluationLimits::native()
    };
    let result = evaluate_with_limits(&package, &sources, exact);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    for limits in [
        EvaluationLimits {
            tokens_per_resource: Some(15),
            ..exact
        },
        EvaluationLimits {
            elements_per_resource: Some(2),
            ..exact
        },
        EvaluationLimits {
            nesting: 3,
            ..exact
        },
    ] {
        let rejected = evaluate_with_limits(&package, &sources, limits);
        assert!(!rejected.diagnostics.is_empty());
        assert!(rejected.model.elements.is_empty());
    }
}
