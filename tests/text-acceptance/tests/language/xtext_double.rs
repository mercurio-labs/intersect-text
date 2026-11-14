use intersect_text::grammar::Datatype;
use intersect_text::grammar::Profile;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::{Value, json};
const ECORE: &str = include_str!("../../../../fixtures/xtext/double-datatypes/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/double-datatypes/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/double-datatypes/model-behavior.rcore");
fn package() -> Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
        Profile::NativeV10,
    )
    .unwrap()
}
fn sources(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "numbers.kerml".into(),
        text: text.into(),
    }]
}
#[test]
fn pinned_real_value_uses_independent_converter_bits_including_overflow_and_subnormals() {
    let package = package();
    assert_eq!(
        package.grammar.as_ref().unwrap().rules["RealValue"].datatype,
        Some(Datatype::EDouble)
    );
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/double-datatypes/observations.json"
    ))
    .unwrap();
    // These fourteen authored grammar inputs have independently observed Xtext converter results.
    for row in &rows[..14] {
        let text = format!("values {}", row["text"].as_str().unwrap());
        let result = evaluate(&package, &sources(&text));
        assert!(
            result.diagnostics.is_empty(),
            "{text}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.model.elements.len(), 2);
        assert_eq!(
            result.model.elements[1].values["value"],
            json!({"kind":"float64","bits":row["bits"]})
        );
    }
    let wire = serde_json::to_string(&package).unwrap();
    let replay: Package = serde_json::from_str(&wire).unwrap();
    assert_eq!(
        serde_json::to_value(evaluate(&package, &sources("values .5"))).unwrap(),
        serde_json::to_value(evaluate(&replay, &sources("values .5"))).unwrap()
    );
}
#[test]
fn grammar_and_conversion_rejections_preserve_other_resources_and_recover() {
    let package = package();
    for spelling in [
        "12",
        "NaN",
        "Infinity",
        "-0.0",
        "0x1p-1074",
        ". 5",
        "1. 0",
        "1e+",
        "1.0e 3",
    ] {
        let mut inputs = sources("values .5");
        inputs.push(Source {
            uri: "bad.kerml".into(),
            text: format!("values {spelling}"),
        });
        let result = evaluate(&package, &inputs);
        assert!(!result.diagnostics.is_empty(), "{spelling}");
        assert_eq!(result.model.elements.len(), 2, "{spelling}");
        assert_eq!(
            result.model.elements[1].values["value"]["bits"],
            "3fe0000000000000"
        );
    }
    assert!(
        evaluate(&package, &sources("values .5"))
            .diagnostics
            .is_empty()
    );
}
#[test]
fn double_rules_are_opt_in_and_do_not_erase_feature_contracts() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV9
        )
        .is_err()
    );
    for schema in [
        ECORE.replace("#//EDouble", "#//EString"),
        ECORE.replace("#//EDouble", "#//EFloat"),
        ECORE.replace("name=\"value\"", "name=\"value\" derived=\"true\""),
        ECORE.replace("name=\"value\"", "name=\"value\" changeable=\"false\""),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                &schema,
                XTEXT,
                RCORE_BEHAVIOR,
                Profile::NativeV10
            )
            .is_err()
        );
    }
    let fragment = XTEXT.replace("RealValue returns", "fragment RealValue returns");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &fragment,
            RCORE_BEHAVIOR,
            Profile::NativeV10
        )
        .is_err()
    );
}
#[test]
fn exact_step_and_element_limits_remain_atomic() {
    let package = package();
    let inputs = sources("values .5 1e309");
    let mut low = 0;
    let mut high = 100_000;
    while low < high {
        let mid = low + (high - low) / 2;
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(mid);
        if evaluate_with_limits(&package, &inputs, limits)
            .diagnostics
            .is_empty()
        {
            high = mid
        } else {
            low = mid + 1
        }
    }
    assert!(low > 0 && low < 100_000);
    for budget in [low - 1, low] {
        let mut limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(budget);
        let result = evaluate_with_limits(&package, &inputs, limits);
        assert_eq!(result.diagnostics.is_empty(), budget == low);
        assert_eq!(
            result.model.elements.len(),
            if budget == low { 3 } else { 0 }
        );
    }
    for count in [2, 3] {
        let mut limits = EvaluationLimits::native();
        limits.elements_per_resource = Some(count);
        let result = evaluate_with_limits(&package, &inputs, limits);
        assert_eq!(result.diagnostics.is_empty(), count == 3);
        assert_eq!(result.model.elements.len(), if count == 3 { 3 } else { 0 });
    }
}

#[test]
fn authoritative_codec_matches_all_independent_xtext_converter_observations() {
    use intersect_model::definitions::scalars::Builtin;
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/double-datatypes/observations.json"
    ))
    .unwrap();
    assert_eq!(rows.len(), 25);
    for row in rows {
        let value = Builtin::EDouble.parse(row["text"].as_str().unwrap());
        assert_eq!(
            value.is_ok(),
            row["accepted"].as_bool().unwrap(),
            "{}",
            row["text"]
        );
        if let Ok(value) = value {
            assert_eq!(value["bits"], row["bits"], "{}", row["text"]);
        }
    }
}
