//! Authored deep rule-composition controls; no general prediction qualification.
use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
const ECORE: &str = r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#;
const RCORE_BEHAVIOR: &str = r#"rcore 1; package emptyRules version "1";"#;
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn deep_rule_first_sets_execute_with_separate_depth_bounds() {
    let mut text =
        String::from(r#"grammar t hidden() import "urn:t" as t Entry returns t::Root: R0;"#);
    for index in 0..40 {
        text.push_str(&format!(
            "R{index} returns t::Root: R{} | Leaf{index}; Leaf{index} returns t::Root: 'k{index}';",
            index + 1
        ));
    }
    text.push_str("R40 returns t::Root: 'end';");
    let old = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        &text,
        RCORE_BEHAVIOR,
        Profile::NativeV22,
    )
    .unwrap_err();
    assert!(
        old.iter()
            .any(|error| error.message.contains("prediction budget")),
        "{old:?}"
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &text,
            RCORE_BEHAVIOR,
            Profile::NativeV23,
        )
        .unwrap();
    for source in ["end", "k0", "k12", "k39"] {
        let result = evaluate(
            &package,
            &[Source {
                uri: "deep".into(),
                text: source.into(),
            }],
        );
        assert!(
            result.diagnostics.is_empty(),
            "{source}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.model.elements.len(), 1);
    }
    for source in ["missing", "end k0", ""] {
        let result = evaluate(
            &package,
            &[Source {
                uri: "deep".into(),
                text: source.into(),
            }],
        );
        assert!(!result.diagnostics.is_empty());
        assert!(result.model.elements.is_empty());
    }
    let recursive = text.replace("R40 returns t::Root: 'end';", "R40 returns t::Root: Entry;");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &recursive,
            RCORE_BEHAVIOR,
            Profile::NativeV23
        )
        .is_err()
    );
    let ambiguous = text.replace("'k0'", "'end'");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &ambiguous,
            RCORE_BEHAVIOR,
            Profile::NativeV23
        )
        .is_err()
    );
}
