use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/typed-fragments/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/typed-fragments/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/typed-fragments/model-behavior.rcore");
const TEXT: &str = include_str!("../../../../fixtures/xtext/typed-fragments/example.kerml");

#[test]
fn typed_fragments_mutate_the_caller_and_preserve_pending_reference_ownership() {
    let package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    assert_eq!(package.sources["xtext"], XTEXT);
    assert_eq!(package.grammar.as_ref().unwrap().entry, "Model");
    let result = evaluate(
        &package,
        &[Source {
            uri: "fragments.kerml".into(),
            text: TEXT.into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    for (index, class, name) in [
        (0, "Model", "Demo"),
        (1, "Type", "Vehicle"),
        (2, "Feature", "wheel"),
    ] {
        let element = &result.model.elements[index];
        assert_eq!(element.id, format!("fragments.kerml#{index}"));
        assert_eq!(element.class, class);
        assert_eq!(element.values["name"], json!(name));
    }
    assert_eq!(
        result.model.elements[2].values["target"],
        json!("fragments.kerml#1")
    );
    let mut direct = intersect_model::core::model::Model::default();
    for element in &result.model.elements {
        direct
            .create(
                &package.schema,
                &element.id,
                &element.class,
                "fragments.kerml",
                element.start,
                element.end,
            )
            .unwrap();
    }
    direct
        .edit(
            &package.schema,
            &[
                ("fragments.kerml#0".into(), "name".into(), json!("Demo")),
                (
                    "fragments.kerml#0".into(),
                    "members".into(),
                    json!(["fragments.kerml#1", "fragments.kerml#2"]),
                ),
                ("fragments.kerml#1".into(), "name".into(), json!("Vehicle")),
                ("fragments.kerml#1".into(), "isAbstract".into(), json!(true)),
                ("fragments.kerml#2".into(), "name".into(), json!("wheel")),
                (
                    "fragments.kerml#2".into(),
                    "target".into(),
                    json!("fragments.kerml#1"),
                ),
            ],
        )
        .unwrap();
    for (parsed, constructed) in result.model.elements.iter().zip(&direct.elements) {
        assert_eq!(parsed.values, constructed.values);
        assert_eq!(parsed.type_identity, constructed.type_identity);
        assert_eq!(parsed.assignment_state, constructed.assignment_state);
    }
    assert_eq!(result.references.len(), 1);
    assert_eq!(result.references[0].target, "fragments.kerml#1");
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(
        result.model,
        evaluate(
            &replay,
            &[Source {
                uri: "fragments.kerml".into(),
                text: TEXT.into()
            }]
        )
        .model
    );
}

#[test]
fn fragment_calls_reject_conflicting_caller_continuations_and_invalid_uses() {
    let conflict = XTEXT.replace("name=ID;", "'named' name=ID?;").replace(
        "'type' Identification ';'",
        "'type' Identification name=ID ';'",
    );
    let errors = compile(ECORE, &conflict, RCORE_BEHAVIOR).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("Fragment caller follow conflict")),
        "{errors:?}"
    );
    for grammar in [
        XTEXT.replace(
            "fragment Identification returns k::Element",
            "fragment Identification",
        ),
        XTEXT.replace(
            "fragment Identification returns k::Element",
            "fragment Identification returns k::Feature",
        ),
        XTEXT.replace("name=ID;", "Identification;"),
        XTEXT.replace("'type' Identification", "'type' name=Identification"),
        XTEXT.replace("name=ID;", "Nested;")
            + "\nfragment Nested returns k::Element: Identification;",
    ] {
        assert!(
            compile(ECORE, &grammar, RCORE_BEHAVIOR).is_err(),
            "{grammar}"
        );
    }
    let only_fragments = XTEXT.split("Model returns").next().unwrap();
    assert!(compile(ECORE, only_fragments, RCORE_BEHAVIOR).is_err());
}

#[test]
fn fragment_failure_rolls_back_the_resource_and_a_fresh_parse_recovers() {
    let package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    for text in [
        "package P { type ; }",
        "package P { feature x to ; }",
        "package P { type T; feature x to Missing; }",
    ] {
        let result = evaluate(
            &package,
            &[Source {
                uri: "bad".into(),
                text: text.into(),
            }],
        );
        assert!(!result.diagnostics.is_empty());
        if !text.contains("Missing") {
            assert!(result.model.elements.is_empty());
        }
    }
    assert!(
        evaluate(
            &package,
            &[Source {
                uri: "good".into(),
                text: TEXT.into()
            }]
        )
        .diagnostics
        .is_empty()
    );
}

#[test]
fn nested_fragments_and_fragment_only_callers_preserve_object_and_depth_limits() {
    use intersect_text::{EvaluationLimits, evaluate_with_limits};
    let grammar = XTEXT.replace("fragment Identification returns k::Element: name=ID;",
        "fragment Identification returns k::Element: Name;\nfragment Name returns k::Element: name=ID;");
    let package = compile(ECORE, &grammar, RCORE_BEHAVIOR).unwrap();
    let sources = [Source {
        uri: "nested".into(),
        text: TEXT.into(),
    }];
    let limits = EvaluationLimits {
        elements_per_resource: Some(3),
        nesting: 5,
        ..EvaluationLimits::native()
    };
    let result = evaluate_with_limits(&package, &sources, limits);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    for constrained in [
        EvaluationLimits {
            nesting: 4,
            ..limits
        },
        EvaluationLimits {
            elements_per_resource: Some(2),
            ..limits
        },
    ] {
        let rejected = evaluate_with_limits(&package, &sources, constrained);
        assert!(
            rejected
                .diagnostics
                .iter()
                .any(|d| d.message.contains("budget"))
        );
        assert!(rejected.model.elements.is_empty());
    }
    let grammar = XTEXT.replace(
        "isAbstract?='abstract'? 'type' Identification ';'",
        "Identification",
    );
    let package = compile(ECORE, &grammar, RCORE_BEHAVIOR).unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "only".into(),
            text: "package P { T }".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 2);
    assert_eq!(result.model.elements[1].class, "Type");
    assert_eq!(result.model.elements[1].values["name"], json!("T"));
}

#[test]
fn legacy_rules_keep_their_wire_shape_and_fragment_entry_execution_rejects() {
    let library = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let wire = serde_json::to_value(&library.grammar).unwrap();
    for rule in wire["rules"].as_object().unwrap().values() {
        assert!(rule.get("fragment").is_none());
    }
    let mut package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    package.grammar.as_mut().unwrap().entry = "Identification".into();
    let result = evaluate(
        &package,
        &[Source {
            uri: "entry".into(),
            text: "Name".into(),
        }],
    );
    assert!(result.model.elements.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("requires a caller object"))
    );
}

#[test]
fn fragment_follow_analysis_reaches_nested_suffixes_and_repeated_call_sites() {
    for (identification, caller, extra) in [
        (
            "('named' name=ID?)",
            "'type' Identification name=ID ';'",
            "",
        ),
        (
            "'named' name=ID?",
            "'type' (Identification name=ID)* ';'",
            "",
        ),
        (
            "'named' Tail",
            "'type' Identification name=ID ';'",
            "fragment Tail returns k::Element: name=ID?;",
        ),
    ] {
        let grammar = XTEXT
            .replace(
                "fragment Identification returns k::Element: name=ID;",
                &format!("fragment Identification returns k::Element: {identification};"),
            )
            .replace("isAbstract?='abstract'? 'type' Identification ';'", caller)
            + extra;
        let errors = compile(ECORE, &grammar, RCORE_BEHAVIOR).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Fragment caller follow conflict")),
            "{errors:?}"
        );
    }
    let grammar = XTEXT.replace(
        "fragment Identification returns k::Element: name=ID;",
        "fragment Identification returns k::Element: 'named' name=ID?;",
    );
    let package = compile(ECORE, &grammar, RCORE_BEHAVIOR).unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "optional".into(),
            text: "package P { type named; }".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 2);
    assert!(
        !result.model.elements[1]
            .assignment_state
            .as_ref()
            .unwrap()
            .explicitly_set("name")
    );
    let cycle = XTEXT.replace(
        "fragment Identification returns k::Element: name=ID;",
        "fragment Identification returns k::Element: Model;",
    );
    let errors = compile(ECORE, &cycle, RCORE_BEHAVIOR).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("Recursive parser fragment")),
        "{errors:?}"
    );
}

#[test]
fn invalid_caller_feature_metadata_cannot_bypass_inline_grammar_checks() {
    // Synthetic invalid-schema control, not an Ecore redefinition qualification:
    // a caller feature lacking declaration metadata must fail just as inline text.
    let package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    let mut schema = package.schema.clone();
    let mut field = schema.features("Type").unwrap()["name"].clone();
    field.changeable = false;
    let class = schema.classes.get_mut("Type").unwrap();
    class.features.insert("name".into(), field);
    class.feature_order.push("name".into());
    let delegated = XTEXT.replace(
        "isAbstract?='abstract'? 'type' Identification ';'",
        "Identification",
    );
    let inline = delegated.replace(
        "Type returns k::Type: Identification;",
        "Type returns k::Type: name=ID;",
    );
    let direct_error =
        intersect_text::grammar::compile_with_profile(&inline, &schema, Profile::NativeV1)
            .unwrap_err();
    let fragment_error =
        intersect_text::grammar::compile_with_profile(&delegated, &schema, Profile::NativeV1)
            .unwrap_err();
    assert_eq!(fragment_error.message, direct_error.message);
    assert_eq!(fragment_error.source, "xtext");
}

// These newly introduced capabilities explicitly select the native Xtext contract.
fn compile(
    ecore: &str,
    xtext: &str,
    rcore_behavior: &str,
) -> Result<intersect_text::package::Package, Vec<intersect_model::definitions::schema::Diagnostic>>
{
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ecore,
        xtext,
        rcore_behavior,
        Profile::NativeV1,
    )
}
