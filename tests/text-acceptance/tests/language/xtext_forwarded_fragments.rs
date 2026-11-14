use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/forwarded-fragments/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "forward.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn forwarded_objects_keep_identity_through_fragment_writes() {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV34,
    )
    .unwrap_err();
    assert!(format!("{old:?}").contains("Unassigned rule calls"));
    let wrapped =
        GRAMMAR.replace("(Plus | All) Suffix", "Marker (Plus | All) Suffix") + "\nMarker: 'wrap';";
    for (grammar, marker) in [(GRAMMAR.to_owned(), ""), (wrapped, "wrap ")] {
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV35,
            )
            .unwrap();
        for (constructor, operator) in [("plus +", "+"), ("all all", "all")] {
            let body = format!("{marker}{constructor}(1,2)");
            let text = format!("Demo = {body};");
            let result = evaluate(&package, &source(&text));
            assert!(
                result.diagnostics.is_empty(),
                "{text}: {:?}",
                result.diagnostics
            );
            assert_eq!(result.model.elements.len(), 4);
            for (index, element) in result.model.elements.iter().enumerate() {
                assert_eq!(element.id, format!("forward.model#{index}"));
            }
            let current = &result.model.elements[1];
            assert_eq!(current.values["operator"], operator);
            assert_eq!(
                current.values["operand"],
                serde_json::json!(["forward.model#2", "forward.model#3"])
            );
            assert_eq!(result.model.elements[0].values["value"], current.id);
            assert_eq!(result.model.elements[2].values["value"], 1);
            assert_eq!(result.model.elements[3].values["value"], 2);
            assert_eq!(&text[current.start..current.end], body);
            assert_eq!(
                (result.model.elements[0].start, result.model.elements[0].end),
                (0, text.len())
            );
            result.model.validate(&package.schema).unwrap();
            assert_eq!(
                evaluate(&compiler::recompile(&package).unwrap(), &source(&text)).model,
                result.model
            );
            assert_eq!(
                evaluate_with_limits(
                    &package,
                    &source(&text),
                    EvaluationLimits {
                        elements_per_resource: Some(4),
                        ..EvaluationLimits::native()
                    }
                )
                .model,
                result.model
            );
            for limits in [
                EvaluationLimits {
                    elements_per_resource: Some(3),
                    ..EvaluationLimits::native()
                },
                EvaluationLimits {
                    steps_per_resource: Some(1),
                    ..EvaluationLimits::native()
                },
            ] {
                let rejected = evaluate_with_limits(&package, &source(&text), limits);
                assert!(!rejected.diagnostics.is_empty());
                assert!(rejected.model.elements.is_empty());
            }
            assert_eq!(evaluate(&package, &source(&text)).model, result.model);
        }
        for suffix in ["plus +(1,);", "all all(1,2)", "plus +(1,2) trailing;"] {
            let text = format!("Demo = {marker}{suffix}");
            let rejected = evaluate(&package, &source(&text));
            assert!(!rejected.diagnostics.is_empty());
            assert!(rejected.model.elements.is_empty());
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn missing_owner_replacement_and_fragment_type_mismatch_reject() {
    for (body, diagnostic) in [
        (
            "Suffix (Plus | All)",
            "Fragment call requires a current object",
        ),
        ("(Plus | All) Suffix Plus", "Unassigned object replacement"),
        ("(Plus | All)+ Suffix", "Unassigned object replacement"),
        (
            "(Plus | All | Literal) Suffix",
            "Fragment caller type mismatch",
        ),
        ("(Plus | All | 'empty')", "may finish without an object"),
        (
            "(Plus | All) operand+=Suffix",
            "Parser fragments cannot be assigned",
        ),
    ] {
        let grammar = GRAMMAR.replace("(Plus | All) Suffix", body);
        let error =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &grammar,
                RCORE_BEHAVIOR,
                Profile::NativeV35,
            )
            .unwrap_err();
        assert!(
            format!("{error:?}").contains(diagnostic),
            "{body}: {error:?}"
        );
    }
}
