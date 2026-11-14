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
pub fn terminal_replaces_datatype_parser_with_one_effective_program() {
    terminal_replaces_datatype_parser_with_one_effective_program_with_profile(Profile::NativeV21);
}
fn terminal_replaces_datatype_parser_with_one_effective_program_with_profile(profile: Profile) {
    let child = format!(
        "{} @Override terminal Name: '@' ('a'..'z'|'A'..'Z')+;",
        CHILD.split("@Override").next().unwrap()
    );
    let i = input(ECORE, BASE, &child, RCORE_BEHAVIOR);
    let base_id = i.bundle.sources[1].id.clone();
    assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV20).is_err());
    let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
    let g = p.grammar.as_ref().unwrap();
    assert!(!g.rules.contains_key("Name"));
    assert!(g.terminals.contains_key("Name"));
    assert_eq!(g.shadowed_declarations["Name"][0].source, base_id);
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(compiler::recompile(&p).unwrap()).unwrap()
    );
    let r = evaluate(&p, &sources("package @p { type @Wheel; }"));
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(r.model.elements[1].values["name"], "@Wheel");
    assert!(
        !evaluate(&p, &sources("package p { type Wheel; }"))
            .diagnostics
            .is_empty()
    );
    let child = child.replace("@Override terminal Name", "terminal Name");
    assert!(
        compiler::compile_bundle_with_profile(input(ECORE, BASE, &child, RCORE_BEHAVIOR), profile)
            .is_ok()
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn incompatible_category_replacements_reject() {
    incompatible_category_replacements_reject_with_profile(Profile::NativeV21);
}
fn incompatible_category_replacements_reject_with_profile(profile: Profile) {
    let header = CHILD.split("@Override").next().unwrap().replace(
        "Entry returns",
        "import \"http://www.eclipse.org/emf/2002/Ecore\" as Ecore Entry returns",
    );
    for replacement in [
        "@Override terminal fragment Name: 'x';",
        "@Override terminal Name returns Ecore::EInt: ('0'..'9')+;",
        "@Override ID returns Ecore::EString: 'x';",
        "@Override terminal Member: 'x';",
    ] {
        let child = format!("{header} {replacement}");
        assert!(
            compiler::compile_bundle_with_profile(
                input(ECORE, BASE, &child, RCORE_BEHAVIOR),
                profile
            )
            .is_err(),
            "{child}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_parent_datatype_survives_terminal_replacement() {
    explicit_parent_datatype_survives_terminal_replacement_with_profile(Profile::NativeV21);
}
fn explicit_parent_datatype_survives_terminal_replacement_with_profile(profile: Profile) {
    let header = CHILD.split("@Override").next().unwrap();
    let child = format!(
        "{header} @Override terminal Name: '@' ('a'..'z'|'A'..'Z')+; @Override Member returns k::Type: 'old' name=super::Name ';';"
    );
    let p =
        compiler::compile_bundle_with_profile(input(ECORE, BASE, &child, RCORE_BEHAVIOR), profile)
            .unwrap();
    let r = evaluate(&p, &sources("package @p { old Wheel; }"));
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(r.model.elements[1].values["name"], "Wheel");
    assert!(
        p.grammar
            .as_ref()
            .unwrap()
            .rules
            .keys()
            .any(|k| k.ends_with("::Name"))
    );
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(compiler::recompile(&p).unwrap()).unwrap()
    );
    let base = BASE.replace(
        "Name returns Ecore::EString: ID;",
        "terminal Name: '@' ('a'..'z'|'A'..'Z')+;",
    );
    let child = format!(
        "{} @Override Name returns Ecore::EString: ID;",
        header.replace(
            "Entry returns",
            "import \"http://www.eclipse.org/emf/2002/Ecore\" as Ecore Entry returns"
        )
    );
    let err =
        compiler::compile_bundle_with_profile(input(ECORE, &base, &child, RCORE_BEHAVIOR), profile)
            .unwrap_err();
    assert!(
        format!("{err:?}").contains("@Override has no inherited parser declaration"),
        "{err:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    terminal_replaces_datatype_parser_with_one_effective_program_with_profile(Profile::NativeV51);
    incompatible_category_replacements_reject_with_profile(Profile::NativeV51);
    explicit_parent_datatype_survives_terminal_replacement_with_profile(Profile::NativeV51);
}
