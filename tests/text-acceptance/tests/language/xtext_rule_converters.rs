use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::{Value, json};
const ECORE: &str = include_str!("../../../../fixtures/xtext/rule-converters/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/rule-converters/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/rule-converters/model-behavior.rcore");
fn sources(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "model.kerml".into(),
        text: text.into(),
    }]
}
fn binding(package: &Package) -> Value {
    let g = package.grammar.as_ref().unwrap();
    json!({"version":1,"scope":"unique_names","derived":[],"validations":[],"rule_conversions":[{
 "context":g.name,"rule":"Nonunique","declaration":g.declaration_sources["Nonunique"],
 "datatype":"http://www.eclipse.org/emf/2002/Ecore#//EBoolean",
 "provider":{"name":"intersect.xtext.keyword-boolean","version":"1"},"keywords":{"nonunique":false}}]})
}
fn package(ecore: &str, grammar: &str) -> Package {
    let plain =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ecore,
            grammar,
            RCORE_BEHAVIOR,
            Profile::NativeV8,
        )
        .unwrap();
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ecore,
        grammar,
        &binding(&plain).to_string(),
        Profile::NativeV8,
    )
    .unwrap()
}
#[test]
fn exact_nonunique_rule_converts_false_and_preserves_true_default() {
    let p = package(ECORE, XTEXT);
    for (text, expected) in [("feature F;", true), ("feature F nonunique;", false)] {
        let r = evaluate(&p, &sources(text));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert_eq!(r.model.elements.len(), 1);
        assert_eq!(r.model.elements[0].values["isUnique"], expected);
        assert_eq!(r.model.elements[0].start, 0);
        assert_eq!(r.model.elements[0].end, text.len());
    }
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV7
        )
        .is_err()
    );
}
#[test]
fn ordinary_boolean_and_both_modifier_orders_keep_separate_meaning() {
    let ecore=ECORE.replace("</eClassifiers>",r#"<eStructuralFeatures xsi:type="ecore:EAttribute" name="isOrdered" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBoolean"/><eStructuralFeatures xsi:type="ecore:EAttribute" name="flag" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBoolean"/></eClassifiers>"#);
    let evidence: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/rule-converters/provenance.json"
    ))
    .unwrap();
    let grammar=XTEXT.replace("(isUnique=Nonunique)?","((isOrdered?='ordered' (isUnique=Nonunique)?) | (isUnique=Nonunique (isOrdered?='ordered')?))? ('flag' flag=BooleanValue)?")+evidence["rules"][1]["text"].as_str().unwrap();
    let p = package(&ecore, &grammar);
    for (text, unique, ordered, flag) in [
        ("feature F;", true, false, false),
        ("feature F ordered flag true;", true, true, true),
        (
            "feature F ordered nonunique flag false;",
            false,
            true,
            false,
        ),
        ("feature F nonunique ordered flag true;", false, true, true),
    ] {
        let r = evaluate(&p, &sources(text));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let values = &r.model.elements[0].values;
        assert_eq!(values["isUnique"], unique);
        assert_eq!(values["isOrdered"], ordered);
        assert_eq!(values["flag"], flag);
    }
}
#[test]
fn converters_require_exact_source_context_type_provider_and_unique_binding() {
    let plain =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV8,
        )
        .unwrap();
    let good = binding(&plain);
    for (path, value) in [
        ("/context", json!("foreign")),
        ("/rule", json!("Model")),
        ("/declaration/source/sha256", json!("0".repeat(64))),
        ("/declaration/source/uri", json!("elsewhere")),
        ("/declaration/start", json!(0)),
        ("/datatype", json!("EString")),
        ("/provider/name", json!("other")),
        ("/provider/version", json!("2")),
        ("/keywords", json!({})),
    ] {
        let mut bad = good.clone();
        *bad["rule_conversions"][0].pointer_mut(path).unwrap() = value;
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                XTEXT,
                &bad.to_string(),
                Profile::NativeV8
            )
            .is_err(),
            "{path}"
        );
    }
    let mut bad = good.clone();
    let duplicate = bad["rule_conversions"][0].clone();
    bad["rule_conversions"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            &bad.to_string(),
            Profile::NativeV8
        )
        .is_err()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
            ECORE,
            &good.to_string()
        )
        .is_err()
    );
    let r = evaluate(&plain, &sources("feature F nonunique;"));
    assert!(!r.diagnostics.is_empty());
    assert!(r.model.elements.is_empty());
    assert_eq!(r.diagnostics[0].start, 10);
    let changed = XTEXT.replace("'nonunique'", "'other'");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &changed,
            &good.to_string(),
            Profile::NativeV8
        )
        .is_err()
    );
    let wrong = ECORE.replace("#//EBoolean", "#//EString");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &wrong,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV8
        )
        .is_err()
    );
}
#[test]
fn conversion_failures_roll_back_only_failed_resource_and_exact_budgets_hold() {
    let p = package(ECORE, XTEXT);
    let mut s = sources("feature F nonunique;");
    s.push(Source {
        uri: "bad.kerml".into(),
        text: "feature G wrong;".into(),
    });
    let r = evaluate(&p, &s);
    assert!(!r.diagnostics.is_empty());
    assert_eq!(r.model.elements.len(), 1);
    assert_eq!(r.model.elements[0].values["isUnique"], false);
    let mut low = 0;
    let mut high = 10000;
    while low < high {
        let mid = (low + high) / 2;
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(mid);
        if evaluate_with_limits(&p, &sources("feature F nonunique;"), limits)
            .diagnostics
            .is_empty()
        {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(low > 0 && low < 10000);
    for budget in [low - 1, low] {
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(budget);
        let r = evaluate_with_limits(&p, &sources("feature F nonunique;"), limits);
        assert_eq!(r.diagnostics.is_empty(), budget == low);
        assert_eq!(r.model.elements.len(), usize::from(budget == low));
    }
    let mut limits = EvaluationLimits::native();
    limits.elements_per_resource = Some(0);
    assert!(
        evaluate_with_limits(&p, &sources("feature F nonunique;"), limits)
            .model
            .elements
            .is_empty()
    );
}

#[test]
fn inherited_rule_identity_and_selected_bundle_uri_survive_source_replay() {
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    for inherited in [false, true] {
        let ecore = DefinitionSource::new("model.ecore", SourceKind::Ecore, ECORE);
        let mut base = DefinitionSource::new("base.xtext", SourceKind::Xtext, XTEXT);
        base.dependencies.push(ecore.id.clone());
        let mut child = DefinitionSource::new(
            "child.xtext",
            SourceKind::Xtext,
            r#"grammar evidence.Child with evidence.RuleConverters hidden(WS)
import "urn:intersect:xtext:rule-converters:1" as p
Child returns p::Model: Model;"#,
        );
        child.dependencies = vec![ecore.id.clone(), base.id.clone()];
        let mut rcore_behavior = DefinitionSource::new(
            "behavior-behavior.rcore",
            SourceKind::RcoreBehavior,
            RCORE_BEHAVIOR,
        );
        rcore_behavior.dependencies.push(ecore.id.clone());
        let selected = if inherited {
            child.id.clone()
        } else {
            base.id.clone()
        };
        let mut bundle = DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), base.clone(), rcore_behavior.clone()],
            namespaces: vec![],
        };
        if inherited {
            bundle.sources.push(child);
        }
        let input = BundleCompilation {
            materialized_types: vec![],
            bundle,
            ecore: ecore.id.clone(),
            namespace: "urn:intersect:xtext:rule-converters:1".into(),
            xtext: Some(selected),
            rcore_behavior: rcore_behavior.id.clone(),
        };
        let mut input =
            intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(input).unwrap();
        let plain =
            compiler::compile_bundle_with_profile(input.clone(), Profile::NativeV8).unwrap();
        assert_eq!(
            plain.grammar.as_ref().unwrap().declaration_sources["Nonunique"].source,
            base.id
        );
        let mut policy = DefinitionSource::new(
            "behavior-behavior.rcore",
            SourceKind::RcoreBehavior,
            intersect_text_acceptance::migration_fixture::canonical_behavior_fixture(
                &binding(&plain).to_string(),
                "urn:intersect:xtext:rule-converters:1",
            )
            .unwrap(),
        );
        policy.dependencies.push(input.ecore.clone());
        input.bundle.sources.retain(|s| s.id != rcore_behavior.id);
        input.rcore_behavior = policy.id.clone();
        input.bundle.sources.push(policy);
        let p = compiler::compile_bundle_with_profile(input, Profile::NativeV8).unwrap();
        let replay = compiler::recompile(&p).unwrap();
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        let r = evaluate(&replay, &sources("feature F nonunique;"));
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert_eq!(r.model.elements[0].values["isUnique"], false);
    }
}
#[test]
fn valid_but_unmapped_spelling_is_a_conversion_failure_not_a_true_default() {
    let plain =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV8,
        )
        .unwrap();
    let mut policy = binding(&plain);
    policy["rule_conversions"][0]["keywords"] = json!({"other":false});
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        XTEXT,
        &policy.to_string(),
        Profile::NativeV8,
    )
    .unwrap();
    let r = evaluate(&p, &sources("feature F;"));
    assert!(r.diagnostics.is_empty());
    assert_eq!(r.model.elements[0].values["isUnique"], true);
    let r = evaluate(&p, &sources("feature F nonunique;"));
    assert!(!r.diagnostics.is_empty());
    assert!(r.model.elements.is_empty());
    assert_eq!(r.diagnostics[0].start, 10);
}
