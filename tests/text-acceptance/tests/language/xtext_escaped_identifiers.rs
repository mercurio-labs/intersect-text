//! Authored syntax controls against the pinned Xtext ID spelling contract.
use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/terminal-spellings/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/terminal-spellings/model-behavior.rcore");
const GRAMMAR: &str = r#"grammar ^audit.^grammar hidden() import "urn:intersect:xtext:mini-kerml:terminal-spellings:1" as ^k ^Model returns ^k::^Model: ^name=^VALUE; terminal ^VALUE: ^EOF; terminal fragment ^EOF: '@' 'a';"#;
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn escaped_names_preserve_source_and_execute() {
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            GRAMMAR,
            RCORE_BEHAVIOR,
            Profile::NativeV13,
        )
        .unwrap();
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(replay.sources["xtext"], GRAMMAR);
    for package in [&package, &replay] {
        let result = evaluate(
            package,
            &[Source {
                uri: "escaped.model".into(),
                text: "@a".into(),
            }],
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        assert_eq!(result.model.elements[0].values["name"], "@a");
        let bad = evaluate(
            package,
            &[Source {
                uri: "escaped.model".into(),
                text: "@".into(),
            }],
        );
        assert!(!bad.diagnostics.is_empty());
        assert!(bad.model.elements.is_empty());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn malformed_escapes_reject() {
    for invalid in [
        "^ name",
        "^/*gap*/name",
        "^^name",
        "^1name",
        "^'name'",
        "^é",
    ] {
        let text = GRAMMAR.replace("^name=", &format!("{invalid}="));
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &text,
                RCORE_BEHAVIOR,
                Profile::NativeV13
            )
            .is_err(),
            "{invalid}"
        );
    }
}
