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
pub fn covariant_model_overrides_use_shared_conformance() {
    covariant_model_overrides_use_shared_conformance_with_profile(Profile::NativeV18);
}
fn covariant_model_overrides_use_shared_conformance_with_profile(profile: Profile) {
    let ecore=ECORE.replace("</ecore:EPackage>",r##"<eClassifiers xsi:type="ecore:EClass" name="SpecialType" eSuperTypes="#//Type"/></ecore:EPackage>"##);
    let child = CHILD.replace("Member returns k::Type", "Member returns k::SpecialType");
    let i = input(&ecore, BASE, &child, RCORE_BEHAVIOR);
    assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV17).is_err());
    let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
    let replay = compiler::recompile(&p).unwrap();
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    let r = evaluate(&p, &sources("package p { class Wheel; }"));
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(r.model.elements[1].class, "SpecialType");
    assert_eq!(r.model.elements[1].values["name"], "Wheel");
    for (base, child) in [
        (BASE.to_owned(), CHILD.replace("k::Type", "k::Feature")),
        (
            BASE.replace("Member returns k::Type", "Member returns k::SpecialType"),
            CHILD.to_owned(),
        ),
    ] {
        assert!(
            compiler::compile_bundle_with_profile(
                input(&ecore, &base, &child, RCORE_BEHAVIOR),
                profile
            )
            .is_err()
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn typed_fragment_overrides_keep_exact_return_identity() {
    typed_fragment_overrides_keep_exact_return_identity_with_profile(Profile::NativeV18);
}
fn typed_fragment_overrides_keep_exact_return_identity_with_profile(profile: Profile) {
    let base = BASE
        .replace("'type' name=Name ';'", "'type' Tail")
        .to_owned()
        + " fragment Tail returns k::Type: name=Name ';';";
    let header = CHILD.split("@Override").next().unwrap();
    let child = format!("{header} @Override fragment Tail returns k::Type: 'tag' name=Name ';';");
    let i = input(ECORE, &base, &child, RCORE_BEHAVIOR);
    assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV17).is_err());
    let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
    for (text, ok) in [
        ("package p { type tag Wheel; }", true),
        ("package p { type Wheel; }", false),
    ] {
        let r = evaluate(&p, &sources(text));
        assert_eq!(r.diagnostics.is_empty(), ok, "{:?}", r.diagnostics);
        if ok {
            assert_eq!(r.model.elements[1].values["name"], "Wheel");
        } else {
            assert!(r.model.elements.is_empty());
        }
    }
    for child in [
        child.replace("fragment Tail", "Tail"),
        child.replace("Tail returns k::Type", "Tail returns k::Element"),
    ] {
        assert!(
            compiler::compile_bundle_with_profile(
                input(ECORE, &base, &child, RCORE_BEHAVIOR),
                profile
            )
            .is_err()
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn datatype_overrides_keep_exact_scalar_type() {
    datatype_overrides_keep_exact_scalar_type_with_profile(Profile::NativeV18);
}
fn datatype_overrides_keep_exact_scalar_type_with_profile(profile: Profile) {
    let child = CHILD.replace(
        "Entry returns",
        "import \"http://www.eclipse.org/emf/2002/Ecore\" as Ecore Entry returns",
    );
    let child = format!("{child} @Override Name returns Ecore::EString: 'label' ID;");
    let i = input(ECORE, BASE, &child, RCORE_BEHAVIOR);
    assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV17).is_err());
    let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
    let r = evaluate(&p, &sources("package label p { class label Wheel; }"));
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(r.model.elements[1].values["name"], "label Wheel");
    let bad = child.replace("Ecore::EString: 'label' ID", "Ecore::EInt: INT");
    assert!(
        compiler::compile_bundle_with_profile(input(ECORE, BASE, &bad, RCORE_BEHAVIOR), profile)
            .is_err()
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_overrides_keep_literal_declaration_identity() {
    enum_overrides_keep_literal_declaration_identity_with_profile(Profile::NativeV18);
}
fn enum_overrides_keep_literal_declaration_identity_with_profile(profile: Profile) {
    let e = include_str!("../../../../fixtures/xtext/enum-rules/model.ecore");
    let b = include_str!("../../../../fixtures/xtext/enum-rules/model.xtext");
    let s = include_str!("../../../../fixtures/xtext/enum-rules/model-behavior.rcore");
    let c = r#"grammar example.EnumChild with example.EnumRules import "urn:intersect:xtext:enum-rules:1" as p Entry returns p::Model: Model; @Override enum VisibilityIndicator returns p::VisibilityKind: public='visible';"#;
    let i = input(e, b, c, s);
    assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV17).is_err());
    let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
    let r = evaluate(&p, &sources("visibility visible;"));
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let enumeration = &p.schema.enumerations["VisibilityKind"];
    let literal = enumeration.by_name("public").unwrap();
    assert_eq!(
        r.model.elements[0].values["visibility"],
        serde_json::to_value(enumeration.value_by_id(&literal.declaration).unwrap()).unwrap()
    );
    assert!(
        !evaluate(&p, &sources("visibility public;"))
            .diagnostics
            .is_empty()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn enum_override_rejects_distinct_classifier_with_same_literals() {
    enum_override_rejects_distinct_classifier_with_same_literals_with_profile(Profile::NativeV18);
}
fn enum_override_rejects_distinct_classifier_with_same_literals_with_profile(profile: Profile) {
    let e = include_str!("../../../../fixtures/xtext/enum-rules/model.ecore");
    let b = include_str!("../../../../fixtures/xtext/enum-rules/model.xtext");
    let s = include_str!("../../../../fixtures/xtext/enum-rules/model-behavior.rcore");
    let e = e.replace("</ecore:EPackage>", r#"<eClassifiers xsi:type="ecore:EEnum" name="OtherVisibility"><eLiterals name="public" value="2"/></eClassifiers></ecore:EPackage>"#);
    let c = r#"grammar example.EnumChild with example.EnumRules import "urn:intersect:xtext:enum-rules:1" as p Entry returns p::Model: Model; @Override enum VisibilityIndicator returns p::OtherVisibility: public='visible';"#;
    let error = compiler::compile_bundle_with_profile(input(&e, b, c, s), profile).unwrap_err();
    assert!(
        format!("{error:?}").contains("Parser override must preserve"),
        "{error:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    covariant_model_overrides_use_shared_conformance_with_profile(Profile::NativeV51);
    typed_fragment_overrides_keep_exact_return_identity_with_profile(Profile::NativeV51);
    datatype_overrides_keep_exact_scalar_type_with_profile(Profile::NativeV51);
    enum_overrides_keep_literal_declaration_identity_with_profile(Profile::NativeV51);
    enum_override_rejects_distinct_classifier_with_same_literals_with_profile(Profile::NativeV51);
}
