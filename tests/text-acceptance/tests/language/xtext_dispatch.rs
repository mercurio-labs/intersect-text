use intersect_model::core::model::Model;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;

const ECORE: &str = include_str!("../../../../fixtures/xtext/parser-dispatch/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/parser-dispatch/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/parser-dispatch/model-behavior.rcore");
const TEXT: &str = include_str!("../../../../fixtures/xtext/parser-dispatch/example.kerml");

#[test]
fn subtype_dispatch_has_no_phantom_objects_and_retains_exact_source_identity() {
    let package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    assert_eq!(package.sources["xtext"], XTEXT);
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    let sources = [Source {
        uri: "dispatch.kerml".into(),
        text: TEXT.into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model, evaluate(&replay, &sources).model);
    assert_eq!(result.model.elements.len(), 3);
    for (index, class) in ["Model", "Type", "Feature"].into_iter().enumerate() {
        let element = &result.model.elements[index];
        assert_eq!(element.id, format!("dispatch.kerml#{index}"));
        assert_eq!(element.class, class);
        assert_eq!(
            element.type_identity,
            package.schema.identity(&element.class)
        );
        let declaration = &package.schema.declarations[&element.class];
        assert_eq!(
            declaration.namespace,
            "urn:intersect:xtext:mini-kerml:dispatch:1"
        );
        assert_eq!(declaration.source.uri, "memory:model.rcore");
    }
    assert_eq!(
        result.model.elements[0].values["members"],
        json!(["dispatch.kerml#1", "dispatch.kerml#2"])
    );
    assert_eq!(result.model.elements[1].values["isAbstract"], json!(true));
    for (index, spelling) in [(1, "abstract type Vehicle;"), (2, "feature wheel;")] {
        let element = &result.model.elements[index];
        assert_eq!(&TEXT[element.start..element.end], spelling);
    }
    // Independent construction path: the text dispatcher must not manufacture
    // a fourth Element or substitute its abstract identity for a concrete type.
    let mut direct = Model::default();
    for element in &result.model.elements {
        direct
            .create(
                &package.schema,
                &element.id,
                &element.class,
                "dispatch.kerml",
                element.start,
                element.end,
            )
            .unwrap();
    }
    direct
        .edit(
            &package.schema,
            &[
                ("dispatch.kerml#0".into(), "name".into(), json!("Demo")),
                (
                    "dispatch.kerml#0".into(),
                    "members".into(),
                    json!(["dispatch.kerml#1", "dispatch.kerml#2"]),
                ),
                ("dispatch.kerml#1".into(), "name".into(), json!("Vehicle")),
                ("dispatch.kerml#1".into(), "isAbstract".into(), json!(true)),
                ("dispatch.kerml#2".into(), "name".into(), json!("wheel")),
            ],
        )
        .unwrap();
    for (parsed, constructed) in result.model.elements.iter().zip(&direct.elements) {
        assert_eq!(parsed.values, constructed.values);
        assert_eq!(parsed.type_identity, constructed.type_identity);
        assert_eq!(parsed.assignment_state, constructed.assignment_state);
    }
    let exact = EvaluationLimits {
        elements_per_resource: Some(3),
        ..EvaluationLimits::native()
    };
    let bounded = evaluate_with_limits(&package, &sources, exact);
    assert!(bounded.diagnostics.is_empty(), "{:?}", bounded.diagnostics);
    assert_eq!(bounded.model, result.model);
    let rejected = evaluate_with_limits(
        &package,
        &sources,
        EvaluationLimits {
            elements_per_resource: Some(2),
            ..exact
        },
    );
    assert!(
        rejected
            .diagnostics
            .iter()
            .any(|d| d.message.contains("budget"))
    );
    assert!(rejected.model.elements.is_empty());
}

#[test]
fn single_grouped_chained_and_entry_delegates_return_the_actual_child() {
    for body in ["Type", "(Type)", "((Type))", "Alias"] {
        let grammar = format!(
            "{}\nAlias returns k::Element: Type;",
            XTEXT.replace("Type | Feature", body)
        );
        let package = compile(ECORE, &grammar, RCORE_BEHAVIOR).unwrap();
        let result = evaluate(
            &package,
            &[Source {
                uri: "alias".into(),
                text: "package P { type T; }".into(),
            }],
        );
        assert!(
            result.diagnostics.is_empty(),
            "{body}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.model.elements.len(), 2);
        assert_eq!(result.model.elements[1].class, "Type");
        assert_eq!(result.model.elements[1].id, "alias#1");
    }
    let grammar = XTEXT.replace(
        "Model returns k::Model:",
        "Root returns k::Model: Model;\nModel returns k::Model:",
    );
    let package = compile(ECORE, &grammar, RCORE_BEHAVIOR).unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "entry".into(),
            text: TEXT.into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    assert_eq!(result.model.elements[0].id, "entry#0");
}

#[test]
fn unsupported_dispatch_shapes_reject_without_weakening_existing_prediction() {
    for body in [
        "ID",
        "Type?",
        "Type*",
        "Type+",
        "Type | Type",
        "Type | Feature?",
        "Member",
        "'prefix' Type",
        "Type 'suffix'",
        "name=ID Type",
        "Type | name=ID",
        "Model",
    ] {
        assert!(
            compile(
                ECORE,
                &XTEXT.replace("Type | Feature", body),
                RCORE_BEHAVIOR
            )
            .is_err(),
            "{body}"
        );
    }
    let nullable = XTEXT.replace("isAbstract?='abstract'? 'type' name=ID ';'", "name=ID?");
    assert!(compile(ECORE, &nullable, RCORE_BEHAVIOR).is_err());
    let cycle = format!(
        "{}\nAlias returns k::Element: Member;",
        XTEXT.replace("Type | Feature", "Alias")
    );
    assert!(compile(ECORE, &cycle, RCORE_BEHAVIOR).is_err());
}

#[test]
fn malformed_subtype_models_publish_no_partial_objects() {
    let package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    for text in [
        "package P { abstract type ; }",
        "package P { feature ; }",
        "package P { type T; feature U }",
        "package P { type T; unknown U; }",
    ] {
        let result = evaluate(
            &package,
            &[Source {
                uri: "bad".into(),
                text: text.into(),
            }],
        );
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty(), "{text}");
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
