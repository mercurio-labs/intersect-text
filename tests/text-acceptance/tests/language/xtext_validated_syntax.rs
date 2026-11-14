//! NativeV14 uses validated syntax admission plus the existing semantic/runtime path.
use intersect_model::definitions::bundle::BUNDLE_VERSION;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/terminal-spellings/model.ecore");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/terminal-spellings/model-behavior.rcore");
fn source(body: &str) -> String {
    format!(
        r#"grammar audit.Terminals hidden() import "urn:intersect:xtext:mini-kerml:terminal-spellings:1" as k Model returns k::Model: name=VALUE; terminal VALUE: {body};"#
    )
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn unicode_definition_literals_execute_and_replay_exactly() {
    let text = source(r"'\u0040' '\u03b1'");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &text,
            RCORE_BEHAVIOR,
            Profile::NativeV13
        )
        .is_err()
    );
    let package =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &text,
            RCORE_BEHAVIOR,
            Profile::NativeV14,
        )
        .unwrap();
    assert_eq!(
        package.grammar.as_ref().unwrap().profile,
        Profile::NativeV14
    );
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(replay.sources["xtext"], text);
    for p in [&package, &replay] {
        let good = evaluate(
            p,
            &[Source {
                uri: "syntax.model".into(),
                text: "@α".into(),
            }],
        );
        assert!(good.diagnostics.is_empty(), "{:?}", good.diagnostics);
        assert_eq!(good.model.elements[0].values["name"], "@α");
        for text in ["@", "@a", "@αx"] {
            let bad = evaluate(
                p,
                &[Source {
                    uri: "syntax.model".into(),
                    text: text.into(),
                }],
            );
            assert!(!bad.diagnostics.is_empty());
            assert!(bad.model.elements.is_empty());
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn syntax_invalid_inputs_cannot_publish_a_package() {
    let valid = source("'@' 'a'");
    for text in [
        format!("{valid} // trailing\r"),
        valid.replace("grammar audit", "grammar\u{00a0}audit"),
        valid.replace("audit.Terminals", "terminal"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &text,
                RCORE_BEHAVIOR,
                Profile::NativeV13
            )
            .is_ok(),
            "frozen profile control"
        );
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &text,
                RCORE_BEHAVIOR,
                Profile::NativeV14
            )
            .is_err(),
            "{text}"
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn imported_syntax_is_not_unimplemented_execution() {
    for text in [
        source(r"'\uD800'"),
        source("'@'").replace("Model returns", "Model<P> returns"),
        source("'@'").replace("name=VALUE", "name=VALUE & 'end'"),
    ] {
        let definition = DefinitionSource::new("unsupported.xtext", SourceKind::Xtext, &text);
        assert!(grammar::syntax::parse(&definition, SourceLimits::default()).is_ok());
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &text,
                RCORE_BEHAVIOR,
                Profile::NativeV14
            )
            .is_err()
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn inherited_sources_keep_identity_limits_and_rollback() {
    let ecore = include_str!("../../../../fixtures/xtext/inherited-grammar/model.ecore");
    let base = include_str!("../../../../fixtures/xtext/inherited-grammar/base.xtext")
        .replace("urn:", r"\u0075rn:");
    let base = DefinitionSource::new("base.xtext", SourceKind::Xtext, base);
    let mut child = DefinitionSource::new(
        "child.xtext",
        SourceKind::Xtext,
        include_str!("../../../../fixtures/xtext/inherited-grammar/model.xtext"),
    );
    child.dependencies.push(base.id.clone());
    let bundle = DefinitionBundle {
        version: BUNDLE_VERSION,
        sources: vec![base.clone(), child.clone()],
        namespaces: vec![],
    };
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ecore)
            .unwrap();
    let before = serde_json::to_value(&schema).unwrap();
    let grammar = grammar::compile_sources_with_limits(
        &bundle,
        &child.id,
        &mut schema,
        Profile::NativeV14,
        SourceLimits {
            source_bytes: base.text.len().max(child.text.len()),
            declarations: 5,
        },
    )
    .unwrap();
    assert_eq!(grammar.declaration_sources["Type"].source, base.id);
    assert_eq!(grammar.declaration_sources["Model"].source, child.id);
    assert!(
        grammar.declaration_sources["Type"]
            .text(&bundle)
            .unwrap()
            .starts_with("Type returns")
    );
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ecore)
            .unwrap();
    assert!(
        grammar::compile_sources_with_limits(
            &bundle,
            &child.id,
            &mut schema,
            Profile::NativeV14,
            SourceLimits {
                source_bytes: base.text.len().max(child.text.len()),
                declarations: 4
            }
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&schema).unwrap(), before);
    let bad = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        format!("{} // trailing\r", base.text),
    );
    child.dependencies = vec![bad.id.clone()];
    let bad_bundle = DefinitionBundle {
        version: BUNDLE_VERSION,
        sources: vec![bad, child.clone()],
        namespaces: vec![],
    };
    let error = grammar::compile_sources(&bad_bundle, &child.id, &mut schema, Profile::NativeV14)
        .unwrap_err();
    assert_eq!(error.source, "base.xtext");
    assert_eq!(serde_json::to_value(&schema).unwrap(), before);
}
