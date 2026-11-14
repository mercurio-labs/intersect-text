use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/typed-fragments/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/typed-fragments/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/typed-fragments/model-behavior.rcore");

fn definitions() -> (String, String) {
    let schema = ECORE.replace(
        "<eClassifiers xsi:type=\"ecore:EClass\" name=\"Type\" eSuperTypes=\"#//Element\">",
        "<eClassifiers xsi:type=\"ecore:EClass\" name=\"Type\" eSuperTypes=\"#//Element\"><eStructuralFeatures xsi:type=\"ecore:EReference\" name=\"children\" upperBound=\"-1\" containment=\"true\" eType=\"#//Type\"/>",
    );
    let grammar = XTEXT
        .replace("with org.eclipse.xtext.common.Terminals", "hidden(WS)")
        .replace("'type' Identification ';'", "'type' Identification Body")
        + "\nterminal ID: ('a'..'z' | 'A'..'Z')+; terminal WS: ' '+;\nfragment Body returns k::Type: '{' children+=Type* '}';";
    (schema, grammar)
}

#[test]
fn productive_fragment_cycle_restores_each_caller_and_replays() {
    let (schema, grammar) = definitions();
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &schema,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV10
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &schema,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV11,
        )
        .unwrap();
    let sources = [Source {
        uri: "nested.kerml".into(),
        text: "package P { type A { type B { type C {} } type D {} } }".into(),
    }];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 5);
    assert_eq!(
        result.model.elements[1].values["children"],
        json!(["nested.kerml#2", "nested.kerml#4"])
    );
    assert_eq!(
        result.model.elements[2].values["children"],
        json!(["nested.kerml#3"])
    );
    assert_eq!(
        result
            .model
            .elements
            .iter()
            .map(|e| e.values["name"].clone())
            .collect::<Vec<_>>(),
        vec![json!("P"), json!("A"), json!("B"), json!("C"), json!("D")]
    );
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(evaluate(&replay, &sources).model, result.model);
    let bad = evaluate(
        &package,
        &[Source {
            uri: "bad.kerml".into(),
            text: "package P { type A { type B {}".into(),
        }],
    );
    assert!(!bad.diagnostics.is_empty());
    assert!(bad.model.elements.is_empty());
    assert_eq!(evaluate(&package, &sources).model, result.model);
}

#[test]
fn nonprogressing_fragment_cycles_still_reject() {
    let (schema, grammar) = definitions();
    for body in ["Body", "Body?", "Body '{' '}'", "('prefix')? Body"] {
        let invalid = grammar.replace("'{' children+=Type* '}'", body);
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &schema,
                &invalid,
                RCORE_BEHAVIOR,
                Profile::NativeV11
            )
            .is_err(),
            "{body}"
        );
    }
}

#[test]
fn recursive_fragment_limits_reject_atomically_and_recover() {
    use intersect_text::{EvaluationLimits, evaluate_with_limits};
    let (schema, grammar) = definitions();
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &schema,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV11,
        )
        .unwrap();
    let sources = [Source {
        uri: "limits.kerml".into(),
        text: "package P { type A { type B { type C {} } type D {} } }".into(),
    }];
    let expected = evaluate(&package, &sources);
    let minimum = (1..32)
        .find(|&nesting| {
            evaluate_with_limits(
                &package,
                &sources,
                EvaluationLimits {
                    nesting,
                    ..EvaluationLimits::native()
                },
            )
            .diagnostics
            .is_empty()
        })
        .unwrap();
    assert_eq!(minimum, 8);
    let minimum_steps = 2899;
    let step_limited = EvaluationLimits {
        steps_per_resource: Some(minimum_steps),
        ..EvaluationLimits::native()
    };
    assert_eq!(
        evaluate_with_limits(&package, &sources, step_limited).model,
        expected.model
    );
    let exhausted = evaluate_with_limits(
        &package,
        &sources,
        EvaluationLimits {
            steps_per_resource: Some(minimum_steps - 1),
            ..step_limited
        },
    );
    assert!(
        exhausted
            .diagnostics
            .iter()
            .any(|d| d.message.contains("budget"))
    );
    assert!(exhausted.model.elements.is_empty());

    let limits = EvaluationLimits {
        nesting: minimum,
        elements_per_resource: Some(5),
        ..EvaluationLimits::native()
    };
    assert_eq!(
        evaluate_with_limits(&package, &sources, limits).model,
        expected.model
    );
    for constrained in [
        EvaluationLimits {
            nesting: minimum - 1,
            ..limits
        },
        EvaluationLimits {
            elements_per_resource: Some(4),
            ..limits
        },
    ] {
        let rejected = evaluate_with_limits(&package, &sources, constrained);
        assert!(
            rejected
                .diagnostics
                .iter()
                .any(|d| d.message.contains("budget")),
            "{:?}",
            rejected.diagnostics
        );
        assert!(rejected.model.elements.is_empty());
        assert_eq!(
            evaluate_with_limits(&package, &sources, limits).model,
            expected.model
        );
    }
}

#[test]
fn recursive_fragments_preserve_scalar_cycle_and_caller_rejections() {
    let (schema, grammar) = definitions();
    let scalar = format!("{}\nRaw returns ecore::EString: 'x' Raw?;", grammar.replace("import \"urn:intersect:xtext:mini-kerml:fragments:1\" as k", "import \"urn:intersect:xtext:mini-kerml:fragments:1\" as k\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore"));
    let errors =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &schema,
            &scalar,
            RCORE_BEHAVIOR,
            Profile::NativeV11,
        )
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("Recursive datatype")),
        "{errors:?}"
    );
    let wrong = grammar.replace(
        "fragment Body returns k::Type",
        "fragment Body returns k::Feature",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &schema,
            &wrong,
            RCORE_BEHAVIOR,
            Profile::NativeV11
        )
        .is_err()
    );
}

#[test]
fn exact_pinned_arguments_preserve_nested_ownership_and_named_links() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            include_str!("../../../../fixtures/xtext/recursive-fragments/model.ecore"),
            include_str!("../../../../fixtures/xtext/recursive-fragments/model.xtext"),
            include_str!("../../../../fixtures/xtext/recursive-fragments/model-behavior.rcore"),
            Profile::NativeV11,
        )
        .unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "arguments".into(),
            text: include_str!("../../../../fixtures/xtext/recursive-fragments/example.kerml")
                .into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 14);
    assert_eq!(
        result.model.elements[1].values["ownedRelationship"],
        json!(["arguments#2", "arguments#6"])
    );
    assert_eq!(
        result.model.elements[9].values["ownedRelationship"],
        json!(["arguments#10"])
    );
    for (owner, feature, target) in [
        (2, "ownedRelatedElement", 3),
        (3, "ownedRelationship", 4),
        (4, "ownedRelatedElement", 5),
        (6, "ownedRelatedElement", 7),
        (7, "ownedRelationship", 8),
        (8, "ownedRelatedElement", 9),
        (10, "ownedRelatedElement", 11),
        (11, "ownedRelationship", 12),
        (12, "ownedRelatedElement", 13),
    ] {
        assert_eq!(
            result.model.elements[owner].values[feature],
            json!([format!("arguments#{target}")])
        );
    }
    let named = evaluate(
        &package,
        &[Source {
            uri: "named".into(),
            text: "model atom target call n(target=atom v)".into(),
        }],
    );
    assert!(named.diagnostics.is_empty(), "{:?}", named.diagnostics);
    let redef = named
        .model
        .elements
        .iter()
        .find(|e| e.class == "Redefinition")
        .unwrap();
    assert_eq!(redef.values["redefinedFeature"], json!("named#1"));
    assert_eq!(named.references.len(), 1);
    assert_eq!(named.references[0].target, "named#1");
    for text in [
        "model call f(atom a,)",
        "model call f(missing=atom a)",
        "model call f(atom a, target=atom b)",
        "model call f(atom a",
    ] {
        let rejected = evaluate(
            &package,
            &[Source {
                uri: "bad".into(),
                text: text.into(),
            }],
        );
        assert!(!rejected.diagnostics.is_empty(), "{text}");
    }
}

#[test]
fn direct_right_recursive_fragment_keeps_one_caller() {
    let (schema, grammar) = definitions();
    let grammar = grammar.replace(
        "fragment Body returns k::Type: '{' children+=Type* '}'",
        "fragment Body returns k::Type: 'again' Body | '{' children+=Type* '}'",
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &schema,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV11,
        )
        .unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "direct".into(),
            text: "package P { type A again again { type B {} } }".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    assert_eq!(
        result.model.elements[1].values["children"],
        json!(["direct#2"])
    );
}
