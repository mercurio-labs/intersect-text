use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
const ECORE: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/model.ecore");
const BASE: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/base.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/contextual-overrides/model-behavior.rcore");
fn input(ecore: &str, base: &str, child: &str, rcore_behavior: &str) -> BundleCompilation {
    let namespace =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ecore)
            .unwrap()
            .uri;
    let e = DefinitionSource::new("model.ecore", SourceKind::Ecore, ecore);
    let mut b = DefinitionSource::new("base.xtext", SourceKind::Xtext, base);
    b.dependencies.push(e.id.clone());
    let mut c = DefinitionSource::new("child.xtext", SourceKind::Xtext, child);
    c.dependencies = vec![e.id.clone(), b.id.clone()];
    let mut s = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        rcore_behavior,
    );
    s.dependencies.push(e.id.clone());
    let result = BundleCompilation {
        ecore: e.id.clone(),
        xtext: Some(c.id.clone()),
        rcore_behavior: s.id.clone(),
        namespace,
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![e, b, c, s],
            namespaces: vec![],
        },
    };
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
fn sources(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "typed.model".into(),
        text: text.into(),
    }]
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn final_parser_and_terminal_contracts() {
    final_parser_and_terminal_contracts_with_profile(Profile::NativeV19);
}
fn final_parser_and_terminal_contracts_with_profile(profile: Profile) {
    for (name, declaration, text, replacement) in [
        (
            "Member",
            "Member returns",
            "package p { type Wheel; }",
            "Member returns k::Type: 'other' name=Name ';';",
        ),
        (
            "ID",
            "terminal ID",
            "package p { type Wheel; }",
            "terminal ID: ('a'..'z')+;",
        ),
    ] {
        let base = BASE.replace(declaration, &format!("@Final {declaration}"));
        let child = CHILD.split("@Override").next().unwrap();
        let i = input(ECORE, &base, child, RCORE_BEHAVIOR);
        assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV18).is_err());
        let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
        assert!(p.grammar.as_ref().unwrap().final_rules.contains(name));
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(compiler::recompile(&p).unwrap()).unwrap()
        );
        let result = evaluate(&p, &sources(text));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements[1].values["name"], "Wheel");
        assert!(
            !evaluate(&p, &sources("package p { other Wheel; }"))
                .diagnostics
                .is_empty()
        );
        for annotation in ["", "@Override", "@Final @Override"] {
            let child = format!("{child} {annotation} {replacement}");
            let err = compiler::compile_bundle_with_profile(
                input(ECORE, &base, &child, RCORE_BEHAVIOR),
                profile,
            )
            .unwrap_err();
            assert!(
                format!("{err:?}").contains(&format!("Cannot override final rule {name}")),
                "{err:?}"
            );
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn final_enum_contract() {
    final_enum_contract_with_profile(Profile::NativeV19);
}
fn final_enum_contract_with_profile(profile: Profile) {
    let e = include_str!("../../../../fixtures/xtext/enum-rules/model.ecore");
    let b = include_str!("../../../../fixtures/xtext/enum-rules/model.xtext").replace(
        "enum VisibilityIndicator",
        "@Final enum VisibilityIndicator",
    );
    let s = include_str!("../../../../fixtures/xtext/enum-rules/model-behavior.rcore");
    let c = r#"grammar example.EnumChild with example.EnumRules import "urn:intersect:xtext:enum-rules:1" as p Entry returns p::Model: Model;"#;
    let p = compiler::compile_bundle_with_profile(input(e, &b, c, s), profile).unwrap();
    assert!(
        p.grammar
            .as_ref()
            .unwrap()
            .final_rules
            .contains("VisibilityIndicator")
    );
    assert!(
        evaluate(&p, &sources("visibility public;"))
            .diagnostics
            .is_empty()
    );
    for annotation in ["", "@Override"] {
        let c = format!(
            "{c} {annotation} enum VisibilityIndicator returns p::VisibilityKind: public='visible';"
        );
        let err = compiler::compile_bundle_with_profile(input(e, &b, &c, s), profile).unwrap_err();
        assert!(
            format!("{err:?}").contains("Cannot override final rule VisibilityIndicator"),
            "{err:?}"
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn final_override_is_transitive_and_may_be_declared_on_replacement() {
    final_override_is_transitive_and_may_be_declared_on_replacement_with_profile(
        Profile::NativeV19,
    );
}
fn final_override_is_transitive_and_may_be_declared_on_replacement_with_profile(profile: Profile) {
    let child = CHILD.replace("@Override", "@Override @Final");
    let i = input(ECORE, BASE, &child, RCORE_BEHAVIOR);
    let p = compiler::compile_bundle_with_profile(i.clone(), profile).unwrap();
    assert!(
        evaluate(&p, &sources("package p { class Wheel; }"))
            .diagnostics
            .is_empty()
    );
    let mut top = DefinitionSource::new(
        "top.xtext",
        SourceKind::Xtext,
        r#"grammar example.Top with example.OverrideChild import "urn:intersect:xtext:mini-kerml:contextual-overrides:1" as k Top returns k::Model: Entry;"#,
    );
    top.dependencies = vec![i.ecore.clone(), i.xtext.clone().unwrap()];
    let mut three = i;
    three.xtext = Some(top.id.clone());
    three.bundle.sources.push(top.clone());
    let p = compiler::compile_bundle_with_profile(three.clone(), profile).unwrap();
    assert!(p.grammar.as_ref().unwrap().final_rules.contains("Member"));
    assert!(
        evaluate(&p, &sources("package p { class Wheel; }"))
            .diagnostics
            .is_empty()
    );
    let mut fourth = DefinitionSource::new(
        "fourth.xtext",
        SourceKind::Xtext,
        r#"grammar example.Fourth with example.Top import "urn:intersect:xtext:mini-kerml:contextual-overrides:1" as k Root returns k::Model: Top; Member returns k::Type: 'other' name=Name ';';"#,
    );
    fourth.dependencies = vec![three.ecore.clone(), top.id];
    three.xtext = Some(fourth.id.clone());
    three.bundle.sources.push(fourth);
    let err = compiler::compile_bundle_with_profile(three, profile).unwrap_err();
    assert!(
        format!("{err:?}").contains("Cannot override final rule Member"),
        "{err:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    final_parser_and_terminal_contracts_with_profile(Profile::NativeV51);
    final_enum_contract_with_profile(Profile::NativeV51);
    final_override_is_transitive_and_may_be_declared_on_replacement_with_profile(
        Profile::NativeV51,
    );
}
