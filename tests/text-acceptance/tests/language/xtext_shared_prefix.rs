use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t">
<eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="items" containment="true" upperBound="-1" eType="#//Item"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Item" abstract="true"><eStructuralFeatures x:type="e:EAttribute" name="name" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Left" eSuperTypes="#//Item"><eStructuralFeatures x:type="e:EAttribute" name="value" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EInt"/></eClassifiers>
<eClassifiers x:type="e:EClass" name="Right" eSuperTypes="#//Item"><eStructuralFeatures x:type="e:EAttribute" name="text" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
</e:EPackage>"##;
const XTEXT: &str = r#"grammar t hidden(WS) import "urn:t" as t import "http://www.eclipse.org/emf/2002/Ecore" as Ecore
Root returns t::Root: 'root' items+=Entry*;
Entry returns t::Item: Left | Right;
Left returns t::Left: 'item' name=ID '=' value=INT ';';
Right returns t::Right: 'item' name=ID ':' text=ID ';';
terminal ID: ('a'..'z')+;
terminal INT returns Ecore::EInt: ('0'..'9')+;
terminal WS: ' '+;"#;
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "prefix.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn shared_prefix_selection_and_rejections() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV23
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV24,
        )
        .unwrap();
    let sources = source("root item a = 12; item b : hello;");
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 3);
    assert_eq!(result.model.elements[1].class, "Left");
    assert_eq!(result.model.elements[1].values["value"], 12);
    assert_eq!(result.model.elements[2].class, "Right");
    assert_eq!(result.model.elements[2].values["text"], "hello");
    for (index, element) in result.model.elements.iter().enumerate() {
        assert_eq!(element.id, format!("prefix.model#{index}"));
    }
    let replay = compiler::recompile(&package).unwrap();
    assert_eq!(evaluate(&replay, &sources).model, result.model);
    for text in [
        "root item a = 2147483648;",
        "root item a = 'wrong';",
        "root item a : 12;",
        "root item a = 12; item b : ;",
        "root item a",
    ] {
        let rejected = evaluate(&package, &source(text));
        assert!(!rejected.diagnostics.is_empty(), "{text}");
        assert!(rejected.model.elements.is_empty(), "{text}");
    }
    let rejected = evaluate_with_limits(
        &package,
        &sources,
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..EvaluationLimits::native()
        },
    );
    assert!(!rejected.diagnostics.is_empty());
    assert!(rejected.model.elements.is_empty());
    let ambiguous = XTEXT.replace("':' text=ID", "'=' text=INT");
    // The duplicate syntax must reject independently of feature type validation.
    let ambiguous = ambiguous
        .replace("text=INT", "value=INT")
        .replace("Right returns t::Right", "Right returns t::Left");
    let errors =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &ambiguous,
            RCORE_BEHAVIOR,
            Profile::NativeV24,
        )
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("Ambiguous parser prediction")),
        "{errors:?}"
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn nullable_alternative_uses_follow_and_rejects_ambiguity() {
    let text = r#"grammar t hidden(WS) import "urn:t" as t Root returns t::Root: ('x'? | 'y') 'end'; terminal WS: ' '+;"#;
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            text,
            RCORE_BEHAVIOR,
            Profile::NativeV23
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            text,
            RCORE_BEHAVIOR,
            Profile::NativeV24,
        )
        .unwrap();
    for text in ["end", "x end", "y end"] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
    }
    for text in ["x", "x y end", "y"] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty());
    }
    for body in ["('x'? | 'y'?) 'end'", "('x' | 'x') 'end'", "('x'?)* 'end'"] {
        let invalid = text.replace("('x'? | 'y') 'end'", body);
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &invalid,
                RCORE_BEHAVIOR,
                Profile::NativeV24
            )
            .is_err(),
            "{body}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn pinned_argument_declarations_with_shared_identifier_prefix() {
    let ecore = include_str!("../../../../fixtures/xtext/recursive-fragments/model.ecore");
    let original = include_str!("../../../../fixtures/xtext/recursive-fragments/model.xtext");
    let rcore_behavior =
        include_str!("../../../../fixtures/xtext/recursive-fragments/model-behavior.rcore");
    // Change only authored teaching context; exact pinned Argument declarations stay intact.
    let grammar = original.replace("'atom' name=ID", "name=ID");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ecore,
            &grammar,
            rcore_behavior,
            Profile::NativeV23
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ecore,
            &grammar,
            rcore_behavior,
            Profile::NativeV24,
        )
        .unwrap();
    for text in [
        "model call f(a)",
        "model a call f(a = b)",
        "model a call f(a = call g(b))",
    ] {
        let result = evaluate(&package, &source(text));
        assert!(
            result.diagnostics.is_empty(),
            "{text}: {:?}",
            result.diagnostics
        );
        assert!(
            result
                .model
                .elements
                .iter()
                .any(|element| element.class == "ParameterMembership")
        );
        assert!(
            result
                .model
                .elements
                .iter()
                .any(|element| element.class == "FeatureValue")
        );
        if text.contains('=') {
            let relation = result
                .model
                .elements
                .iter()
                .find(|element| element.class == "Redefinition")
                .unwrap();
            let target = relation.values["redefinedFeature"].as_str().unwrap();
            assert_eq!(
                result
                    .model
                    .elements
                    .iter()
                    .find(|element| element.id == target)
                    .unwrap()
                    .values["name"],
                "a"
            );
        }
    }
    for text in [
        "model call f(a =)",
        "model call f(a,)",
        "model a call f(a = b, c)",
    ] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty());
    }
}
