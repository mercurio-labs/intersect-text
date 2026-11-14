//! Public native parser controls over a tiny genuine migrated grammar. The
//! original Pilot generated-parser excerpt is evidence only, never executed.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{EvaluationLimits, Source, evaluate_with_limits, ordinary};
use serde_json::{Value, json};
const NS: &str = "urn:intersect:nullable-fragment-current:v1";
const ECORE: &str = r##"<ecore:EPackage xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" name="fragmentCurrent" nsURI="urn:intersect:nullable-fragment-current:v1" nsPrefix="m">
<eClassifiers xsi:type="ecore:EClass" name="Root"><eStructuralFeatures xsi:type="ecore:EReference" name="items" upperBound="-1" eType="#//Base" containment="true"/></eClassifiers>
<eClassifiers xsi:type="ecore:EClass" name="Base" abstract="true"><eStructuralFeatures xsi:type="ecore:EAttribute" name="name" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
<eClassifiers xsi:type="ecore:EClass" name="Feature" eSuperTypes="#//Base"/>
<eClassifiers xsi:type="ecore:EClass" name="Wrapper" eSuperTypes="#//Base"><eStructuralFeatures xsi:type="ecore:EReference" name="child" eType="#//Base" containment="true"/></eClassifiers>
</ecore:EPackage>"##;
const GRAMMAR: &str = r#"grammar regression.NullableFragmentCurrent hidden(WS)
import "urn:intersect:nullable-fragment-current:v1" as m
import "http://www.eclipse.org/emf/2002/Ecore" as ecore
Root returns m::Root: {m::Root} items+=Entry*;
Entry returns m::Base: Plain | Forward | Action | Capture | Datatype | Digits | Choice;
Plain returns m::Feature: Prefix 'feature' Body;
fragment Prefix returns m::Feature: (name=ID)?;
fragment Body returns m::Feature: ';';
Forward returns m::Base: 'forward' Plain;
Action returns m::Base: {m::Feature} SharedPrefix 'action' TailBody;
fragment SharedPrefix returns m::Base: (name=ID)?;
fragment TailBody returns m::Base: ';';
Capture returns m::Base: 'capture' Plain {m::Wrapper.child=current} Tail;
fragment Tail returns m::Base: 'tail' ';';
Datatype returns m::Feature: 'datatype' name=Name Body;
Name returns ecore::EString: ID;
Digits returns m::Feature: 'digits' name=NUMBER Body;
Choice returns m::Feature: 'choose' (Prefix 'bad' | Prefix 'good') Body;
terminal fragment DIGIT: '0'..'9';
terminal NUMBER returns ecore::EString: DIGIT+;
terminal ID returns ecore::EString: ('a'..'z')+;
terminal WS returns ecore::EString: (' '|'\t'|'\r'|'\n')+;
"#;
fn package(grammar: &str) -> Result<Package, String> {
    let model = DefinitionSource::new("memory:fragment-current.ecore", SourceKind::Ecore, ECORE);
    let mut xtext =
        DefinitionSource::new("memory:fragment-current.xtext", SourceKind::Xtext, grammar);
    xtext.dependencies.push(model.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:fragment-current-rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package fragmentCurrent version \"1\";",
    );
    behavior.dependencies.push(model.id.clone());
    let original = BundleCompilation {
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), xtext.clone(), behavior.clone()],
            namespaces: vec![],
        },
        materialized_types: vec![],
        ecore: model.id.clone(),
        xtext: Some(xtext.id),
        rcore_behavior: behavior.id,
        namespace: NS.into(),
    };
    let migration = intersect_model::migration::migrate_ecore_bundle_with_limits(
        original.bundle.clone(),
        &[(model.id, "memory:fragment-current.rcore".into())].into(),
        ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    ordinary::compile(
        intersect_text::migration::remap_compilation(&migration, &original)?,
        Profile::NativeV80,
    )
    .map_err(|e| format!("{e:?}"))
}
fn output(package: &Package, text: &str) -> Value {
    serde_json::to_value(evaluate_with_limits(
        package,
        &[Source {
            uri: "fragment.model".into(),
            text: text.into(),
        }],
        EvaluationLimits::workbench(),
    ))
    .unwrap()
}
fn elements(output: &Value) -> &Vec<Value> {
    output["model"]["elements"].as_array().unwrap()
}
fn feature(rows: &[Value]) -> &Value {
    rows.iter().find(|r| r["class"] == "Feature").unwrap()
}
#[test]
fn bare_feature_constructs_canonical_caller_before_nullable_fragment() -> Result<(), String> {
    let p = package(GRAMMAR)?;
    for (text, start, end) in [("feature;", 0, 8), (" \tfeature;  ", 2, 10)] {
        let result = output(&p, text);
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        let rows = elements(&result);
        assert_eq!(rows.len(), 2, "{result}");
        assert_eq!(rows[0]["class"], "Root");
        assert_eq!(rows[1]["class"], "Feature");
        assert_eq!(rows[1]["id"], "fragment.model#1");
        assert_eq!(rows[1]["start"], start);
        assert_eq!(rows[1]["end"], end);
        assert_eq!(rows[0]["values"]["items"], json!(["fragment.model#1"]));
        assert_eq!(rows[1]["values"]["name"], Value::Null);
    }
    Ok(())
}
#[test]
fn caller_creation_preserves_forwarded_action_and_capture_currents() -> Result<(), String> {
    let p = package(GRAMMAR)?;
    for (text, count, start, end, name) in [
        ("forward feature;", 2, 8, 16, None),
        ("action;", 2, 0, 7, None),
        ("capture feature; tail;", 3, 8, 16, None),
        ("datatype abc;", 2, 0, 13, Some("abc")),
        ("digits 12;", 2, 0, 10, Some("12")),
        ("choose good;", 2, 0, 12, None),
        ("choose named good;", 2, 0, 18, Some("named")),
    ] {
        let result = output(&p, text);
        assert_eq!(result["diagnostics"], json!([]), "{text}: {result}");
        let rows = elements(&result);
        assert_eq!(rows.len(), count, "{text}: {result}");
        assert!(!rows.iter().any(|r| r["class"] == "Base"));
        let f = feature(rows);
        assert_eq!(f["id"], "fragment.model#1");
        assert_eq!(f["start"], start, "{text}");
        assert_eq!(f["end"], end, "{text}");
        assert_eq!(f["values"]["name"], name.map_or(Value::Null, |s| json!(s)));
        if count == 3 {
            let wrapper = &rows[2];
            assert_eq!(wrapper["class"], "Wrapper");
            assert_eq!(wrapper["values"]["child"], f["id"]);
            assert_eq!(wrapper["start"], 0);
            assert_eq!(wrapper["end"], text.len());
            assert_eq!(rows[0]["values"]["items"], json!([wrapper["id"]]));
        }
    }
    Ok(())
}
#[test]
fn rejected_alternative_rolls_back_all_caller_objects_and_spans() -> Result<(), String> {
    let p = package(GRAMMAR)?;
    for text in ["choose nope;", "feature", "capture feature; wrong;"] {
        let result = output(&p, text);
        assert_ne!(result["diagnostics"], json!([]));
        assert!(elements(&result).is_empty(), "{result}");
    }
    let valid = output(&p, "choose good;");
    assert_eq!(valid["diagnostics"], json!([]));
    assert_eq!(elements(&valid).len(), 2);
    assert_eq!(feature(elements(&valid))["id"], "fragment.model#1");
    Ok(())
}
#[test]
fn datatype_fragment_remains_scalar_or_preserves_explicit_admission_refusal() -> Result<(), String>
{
    let text = GRAMMAR.replace(
        "Name returns ecore::EString: ID;",
        "Name returns ecore::EString: Word;\nfragment Word returns ecore::EString: ID;",
    );
    match package(&text) {
        Ok(p) => {
            let result = output(&p, "datatype abc;");
            assert_eq!(result["diagnostics"], json!([]), "{result}");
            assert_eq!(elements(&result).len(), 2);
            assert_eq!(feature(elements(&result))["values"]["name"], "abc");
        }
        Err(error) => assert!(
            error.contains("Datatype parser fragments are unsupported"),
            "Unexpected admission failure, not a datatype-fragment contract: {error}"
        ),
    }
    Ok(())
}
#[test]
fn original_pilot_generated_caller_creation_is_pinned_source_evidence() {
    let excerpt = include_str!(
        "../../../fixtures/xtext-nullable-fragment-current-v1/original-feature-caller.java.txt"
    );
    let pin: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-nullable-fragment-current-v1/provenance.json"
    ))
    .unwrap();
    assert_eq!(
        pin["upstream_commit"],
        "692170b71867353b8f90341e61556f49a5beb0e5"
    );
    assert_eq!(
        SourceId::for_text("evidence:feature-caller", excerpt).sha256,
        pin["excerpt_sha256"]
    );
    assert!(excerpt.contains("current = createModelElement(grammarAccess.getFeatureRule())"));
    assert!(
        excerpt.find("createModelElement").unwrap()
            < excerpt.find("ruleFeaturePrefix(current)").unwrap()
    );
}
