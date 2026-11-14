use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
const GRAMMAR: &str = include_str!("../../../../fixtures/xtext/guarded-continuations/model.xtext");
fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "guarded.model".into(),
        text: text.into(),
    }]
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn guarded_optional_prefix_preserves_models_and_old_rejection() {
    for profile in [Profile::NativeV40, Profile::NativeV41, Profile::NativeV42] {
        guarded_optional_prefix_preserves_models_and_old_rejection_for(profile);
    }
}
fn guarded_optional_prefix_preserves_models_and_old_rejection_for(profile: Profile) {
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        GRAMMAR,
        RCORE_BEHAVIOR,
        Profile::NativeV39,
    )
    .unwrap_err();
    assert!(
        format!("{old:?}").contains("Guarded shared-prefix proof remains unsupported"),
        "{old:?}"
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            profile,
        )
        .unwrap();
    for (text, header, first, second) in [
        (
            include_str!("../../../../fixtures/xtext/guarded-continuations/example.model"),
            Some("a"),
            "a",
            "b",
        ),
        ("b from b to a", Some("b"), "b", "a"),
        ("a to b", None, "a", "b"),
        ("b to a", None, "b", "a"),
    ] {
        let result = evaluate(&package, &source(text));
        assert!(
            result.diagnostics.is_empty(),
            "{text}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.model.elements.len(), 3);
        let root = &result.model.elements[0];
        assert_eq!(root.id, "guarded.model#0");
        assert_eq!((root.start, root.end), (0, text.len()));
        assert_eq!(
            root.values["operand"],
            serde_json::json!(["guarded.model#1", "guarded.model#2"])
        );
        if let Some(header) = header {
            assert_eq!(root.values["operator"], header);
        } else {
            assert!(
                root.values
                    .get("operator")
                    .is_none_or(serde_json::Value::is_null)
            );
        }
        for (index, spelling) in [(1, first), (2, second)] {
            let child = &result.model.elements[index];
            assert_eq!(child.id, format!("guarded.model#{index}"));
            assert_eq!(child.values["operator"], spelling);
            assert_eq!(&text[child.start..child.end], spelling);
        }
        result.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&compiler::recompile(&package).unwrap(), &source(text)).model,
            result.model
        );
        assert_eq!(
            evaluate_with_limits(
                &package,
                &source(text),
                EvaluationLimits {
                    elements_per_resource: Some(3),
                    ..EvaluationLimits::native()
                }
            )
            .model,
            result.model
        );
        for limits in [
            EvaluationLimits {
                elements_per_resource: Some(2),
                ..EvaluationLimits::native()
            },
            EvaluationLimits {
                steps_per_resource: Some(1),
                ..EvaluationLimits::native()
            },
        ] {
            let failed = evaluate_with_limits(&package, &source(text), limits);
            assert!(!failed.diagnostics.is_empty());
            assert!(failed.model.elements.is_empty());
        }
        assert_eq!(evaluate(&package, &source(text)).model, result.model);
    }
    for text in [
        "a from b",
        "c to b",
        "a from a to b trailing",
        "a from from a to b",
    ] {
        let failed = evaluate(&package, &source(text));
        assert!(!failed.diagnostics.is_empty(), "{text}");
        assert!(failed.model.elements.is_empty());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn guards_do_not_justify_ambiguous_continuations() {
    for profile in [Profile::NativeV40, Profile::NativeV41, Profile::NativeV42] {
        guards_do_not_justify_ambiguous_continuations_for(profile);
    }
}
fn guards_do_not_justify_ambiguous_continuations_for(profile: Profile) {
    let ambiguous = GRAMMAR
        .replace(
            "(Prefix 'from')? operand+=Atom 'to' operand+=Atom",
            "(Prefix)? operand+=Atom",
        )
        .replace("operator=('a'|'b');", "operator='a' 'a'*;");
    let rejected =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &ambiguous,
            RCORE_BEHAVIOR,
            profile,
        )
        .unwrap_err();
    assert!(
        format!("{rejected:?}").contains("Ambiguous parser prediction"),
        "{rejected:?}"
    );
}
