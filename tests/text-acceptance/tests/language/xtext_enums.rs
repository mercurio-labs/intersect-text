use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/enum-rules/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/enum-rules/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/enum-rules/model-behavior.rcore");
fn source(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "enum.model".into(),
        text: text.into(),
    }]
}
#[test]
fn enum_keywords_bind_literal_identity_instead_of_alternative_order() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV7,
        )
        .unwrap();
    let enumeration = &package.schema.enumerations["VisibilityKind"];
    for (text, name) in [
        ("visibility public;", "public"),
        ("visibility [;", "private"),
        ("visibility protected;", "protected"),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        let element = &result.model.elements[0];
        assert_eq!(element.id, "enum.model#0");
        assert_eq!((element.start, element.end), (0, text.len()));
        let literal = enumeration.by_name(name).unwrap();
        let expected = enumeration.value_by_id(&literal.declaration).unwrap();
        assert_eq!(
            element.values["visibility"],
            serde_json::to_value(expected).unwrap()
        );
    }
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV6
        )
        .is_err()
    );
}
#[test]
fn enum_aliases_preserve_distinct_ids_despite_equal_numeric_values() {
    let ecore = ECORE.replace("</eClassifiers>\n</ecore:EPackage>", "<eLiterals name=\"publicAlias\" value=\"2\" literal=\"PUBLIC_ALIAS\"/></eClassifiers>\n</ecore:EPackage>");
    assert_ne!(ecore, ECORE);
    let grammar = XTEXT.replace(
        "protected='protected';",
        "protected='protected' | publicAlias='alias' | public='+';",
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &ecore,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV7,
        )
        .unwrap();
    let enumeration = &package.schema.enumerations["VisibilityKind"];
    for (spelling, name) in [
        ("alias", "publicAlias"),
        ("+", "public"),
        ("public", "public"),
    ] {
        let result = evaluate(&package, &source(&format!("visibility {spelling};")));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.model.elements[0].values["visibility"],
            serde_json::to_value(
                enumeration
                    .value_by_id(&enumeration.by_name(name).unwrap().declaration)
                    .unwrap()
            )
            .unwrap()
        );
    }
}
#[test]
fn enum_rules_reject_invalid_binding_and_syntax_without_model_leaks() {
    for grammar in [
        XTEXT.replace("public='public'", "missing='public'"),
        XTEXT.replace("p::VisibilityKind", "p::Model"),
        XTEXT.replace("private='['", "private='public'"),
        XTEXT.replace("private='['", "private=''"),
        XTEXT.replace("private='['", "private=ID"),
        XTEXT.replace("private='['", "private='['?"),
        XTEXT.replace(
            "enum VisibilityIndicator",
            "@Override enum VisibilityIndicator",
        ),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV7
            )
            .is_err(),
            "{grammar}"
        );
    }
    let shadow = XTEXT
        .lines()
        .filter(|line| !line.starts_with("terminal ID:"))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("VisibilityIndicator", "ID");
    let diagnostics =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &shadow,
            RCORE_BEHAVIOR,
            Profile::NativeV7,
        )
        .unwrap_err();
    assert!(
        diagnostics[0]
            .message
            .contains("shadowing common terminals")
    );
    let wrong = ECORE.replace(
        "eType=\"#//VisibilityKind\"",
        "eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString\"",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &wrong,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV7
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV7,
        )
        .unwrap();
    for text in [
        "visibility private;",
        "visibility missing;",
        "visibility public",
        "visibility [; extra",
    ] {
        let result = evaluate(&package, &source(text));
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(result.model.elements.is_empty(), "{text}");
    }
}
#[test]
fn enum_defaults_implicit_keyword_names_and_resource_budgets_stay_exact() {
    use intersect_text::{EvaluationLimits, evaluate_with_limits};
    let grammar = XTEXT
        .replace(
            "visibility=VisibilityIndicator",
            "(visibility=VisibilityIndicator)?",
        )
        .replace("public='public'", "public");
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV7,
        )
        .unwrap();
    for (text, index) in [
        ("visibility;", 0),
        ("visibility public;", 2),
        ("visibility [;", 0),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.model.elements[0].values["visibility"]["literal"]["path"],
            serde_json::json!([1, index])
        );
        let mut limits = EvaluationLimits::native();
        limits.elements_per_resource = Some(0);
        let failed = evaluate_with_limits(&package, &source(text), limits);
        assert!(!failed.diagnostics.is_empty());
        assert!(failed.model.elements.is_empty());
    }
    let sources = vec![
        Source {
            uri: "good.model".into(),
            text: "visibility public;".into(),
        },
        Source {
            uri: "bad.model".into(),
            text: "visibility missing;".into(),
        },
    ];
    let result = evaluate(&package, &sources);
    assert!(!result.diagnostics.is_empty());
    assert_eq!(result.model.elements.len(), 1);
    assert_eq!(result.model.elements[0].id, "good.model#0");
    let mut low = 0;
    let mut high = 100_000;
    while low < high {
        let mid = low + (high - low) / 2;
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(mid);
        if evaluate_with_limits(&package, &source("visibility public;"), limits)
            .diagnostics
            .is_empty()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(low > 0 && low < 100_000);
    for budget in [low - 1, low] {
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(budget);
        let result = evaluate_with_limits(&package, &source("visibility public;"), limits);
        assert_eq!(result.diagnostics.is_empty(), budget == low);
        assert_eq!(result.model.elements.len(), usize::from(budget == low));
    }
}
#[test]
fn exact_pinned_enum_fragments_parse_against_separately_authored_teaching_schema() {
    // Exact upstream grammar text; the metamodel remains the authored fixture,
    // so this proves fragment consumption, not actual pinned KerML semantics.
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/enum-rules/provenance.json"
    ))
    .unwrap();
    let start = XTEXT.find("enum VisibilityIndicator").unwrap();
    let end = XTEXT[start..].find(';').unwrap() + start + 1;
    for (index, spelling, literal_index) in [
        (0, "[", 0),
        (1, "private", 0),
        (1, "public", 2),
        (2, "in", 0),
        (2, "out", 2),
        (2, "inout", 1),
    ] {
        let row = &provenance["rules"][index];
        let rule_name = row["name"].as_str().unwrap();
        let fragment = row["text"].as_str().unwrap();
        let grammar = format!(
            "{}{}{}",
            XTEXT[..start]
                .replace("as p", "as SysML")
                .replace("p::Model", "SysML::Model")
                .replace("VisibilityIndicator", rule_name),
            fragment,
            &XTEXT[end..]
        );
        assert!(grammar.contains(fragment));
        let ecore = if index == 2 {
            ECORE
                .replace("VisibilityKind", "FeatureDirectionKind")
                .replace("name=\"private\"", "name=\"in\"")
                .replace("name=\"protected\"", "name=\"inout\"")
                .replace("name=\"public\"", "name=\"out\"")
        } else {
            ECORE.to_owned()
        };
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &ecore,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV7,
            )
            .unwrap();
        let result = evaluate(&package, &source(&format!("visibility {spelling};")));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.model.elements[0].values["visibility"]["literal"]["path"],
            serde_json::json!([1, literal_index])
        );
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &ecore,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV6
            )
            .is_err()
        );
    }
}
