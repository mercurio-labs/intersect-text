use intersect_model::core::model::Model;
use intersect_text::grammar::Kind;
use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
use serde_json::json;

const ECORE: &str = include_str!("../../../../fixtures/xtext/boolean-keywords/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/boolean-keywords/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/boolean-keywords/model-behavior.rcore");

#[test]
fn presence_preserves_defaults_assignment_state_source_and_direct_model_values() {
    let package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    assert_eq!(package.sources["xtext"], XTEXT);
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    let text = include_str!("../../../../fixtures/xtext/boolean-keywords/example.kerml");
    let sources = [Source {
        uri: "example.kerml".into(),
        text: text.into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model, evaluate(&replay, &sources).model);
    assert_eq!(result.model.elements.len(), 4);
    for (index, name, abstract_value, explicit_standard) in [
        (1, "Vehicle", true, false),
        (2, "Wheel", false, false),
        (3, "Scalar", false, true),
    ] {
        let element = &result.model.elements[index];
        assert_eq!(element.id, format!("example.kerml#{index}"));
        assert_eq!(
            element.type_identity,
            package.schema.identity(&element.class)
        );
        let declaration = &package.schema.declarations[&element.class];
        assert_eq!(
            declaration.namespace,
            "urn:intersect:xtext:mini-kerml:boolean:1"
        );
        assert_eq!(declaration.source.uri, "memory:model.rcore");
        assert_eq!(element.values["isAbstract"], json!(abstract_value));
        assert_eq!(element.values["isStandard"], json!(true));
        let spelling = &text[element.start..element.end];
        assert!(spelling.ends_with(';') && spelling.contains(name));
        assert_eq!(spelling.starts_with("abstract "), abstract_value);
        let state = element.assignment_state.as_ref().unwrap();
        assert_eq!(state.explicitly_set("isAbstract"), abstract_value);
        assert_eq!(state.explicitly_set("isStandard"), explicit_standard);
        let mut direct = Model::default();
        direct
            .create(&package.schema, "direct", "Type", "direct", 0, 0)
            .unwrap();
        let mut edits = vec![("direct".into(), "name".into(), json!(name))];
        if abstract_value {
            edits.push(("direct".into(), "isAbstract".into(), json!(true)));
        }
        if explicit_standard {
            edits.push(("direct".into(), "isStandard".into(), json!(true)));
        }
        direct.edit(&package.schema, &edits).unwrap();
        assert_eq!(element.values, direct.elements[0].values);
        assert_eq!(
            element.assignment_state,
            direct.elements[0].assignment_state
        );
    }
    let body = &package.grammar.as_ref().unwrap().rules["Type"].body;
    assert!(
        matches!(&body.kind, Kind::Sequence(items) if matches!(items[0].kind, Kind::BooleanAssign { .. }))
    );
}

#[test]
fn rejects_invalid_boolean_targets_and_unsupported_rhs_without_relaxing_prediction() {
    for changed in [
        ECORE.replace(
            "name=\"isAbstract\" unsettable=\"true\"",
            "name=\"isAbstract\" unsettable=\"true\" upperBound=\"-1\"",
        ),
        ECORE.replace(
            "name=\"isAbstract\" unsettable=\"true\"",
            "name=\"isAbstract\" unsettable=\"true\" derived=\"true\"",
        ),
        ECORE.replace(
            "name=\"isAbstract\" unsettable=\"true\"",
            "name=\"isAbstract\" unsettable=\"true\" changeable=\"false\"",
        ),
        ECORE.replace("#//EBoolean", "#//EString").replace(
            "defaultValueLiteral=\"true\"",
            "defaultValueLiteral=\"text\"",
        ),
        ECORE
            .replace(
                "xsi:type=\"ecore:EAttribute\" name=\"isAbstract\"",
                "xsi:type=\"ecore:EReference\" name=\"isAbstract\"",
            )
            .replace(
                "name=\"isAbstract\" unsettable=\"true\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBoolean\"",
                "name=\"isAbstract\" unsettable=\"true\" eType=\"#//Type\"",
            )
            .replace(" defaultValueLiteral=\"true\"", ""),
    ] {
        let errors = compile(&changed, XTEXT, RCORE_BEHAVIOR).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.source == "memory:language.xtext" && e.message.contains("Boolean assignment")),
            "{errors:?}"
        );
    }
    for (rhs, message) in [
        ("ID", "keyword RHS"),
        ("('abstract'|'virtual')", "keyword RHS"),
        ("[k::Type|ID]", "keyword RHS"),
        ("''", "Empty keywords"),
        ("'two words'", "single identifier"),
    ] {
        let errors = compile(
            ECORE,
            &XTEXT.replace("?='abstract'", &format!("?={rhs}")),
            RCORE_BEHAVIOR,
        )
        .unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains(message)),
            "{errors:?}"
        );
    }
    assert!(
        compile(
            ECORE,
            &XTEXT.replace("'type' name", "'abstract' name"),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert!(
        compile(
            ECORE,
            &XTEXT.replace("isAbstract?=", "missing?="),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert!(
        compile(
            ECORE,
            &XTEXT.replace("isAbstract?=", "isAbstract?"),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    for operator in ["isAbstract? =", "isAbstract?/* split */=", "isAbstract?\n="] {
        assert!(
            compile(
                ECORE,
                &XTEXT.replace("isAbstract?=", operator),
                RCORE_BEHAVIOR
            )
            .is_err()
        );
    }
    assert!(
        compile(
            ECORE,
            &XTEXT.replace("isAbstract?='abstract'?", "(isAbstract ?= 'abstract')?"),
            RCORE_BEHAVIOR
        )
        .is_ok()
    );
}

#[test]
fn failed_models_publish_no_partial_objects_and_keywords_are_reserved() {
    let package = compile(ECORE, XTEXT, RCORE_BEHAVIOR).unwrap();
    for text in [
        "package P { abstract type ; }",
        "package P { abstract type T }",
        "package P { type abstract; }",
        "package P { abstract abstract type T; }",
    ] {
        let result = evaluate(
            &package,
            &[Source {
                uri: "bad".into(),
                text: text.into(),
            }],
        );
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty(), "{text}");
    }
}

#[test]
fn ordinary_boolean_fields_use_the_same_defaults_and_writes() {
    let package = compile(
        &ECORE.replace(" unsettable=\"true\"", ""),
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "ordinary".into(),
            text: "package P { abstract type T; type U; }".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[1].values["isAbstract"], json!(true));
    assert_eq!(result.model.elements[2].values["isAbstract"], json!(false));
    assert_eq!(result.model.elements[2].values["isStandard"], json!(true));
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
