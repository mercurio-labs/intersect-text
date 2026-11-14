use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
fn input(grammar: &str) -> BundleCompilation {
    input_case(grammar, false)
}
fn input_case(grammar: &str, conflict: bool) -> BundleCompilation {
    input_named(grammar, conflict, "Thing")
}
fn input_named(grammar: &str, conflict: bool, class: &str) -> BundleCompilation {
    let b = DefinitionSource::new(
        "file:///imports/b.ecore",
        SourceKind::Ecore,
        r##"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="b" nsURI="urn:b"><eClassifiers xsi:type="ecore:EClass" name="Thing"><eStructuralFeatures xsi:type="ecore:EAttribute" name="name" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers></ecore:EPackage>"##.replace("Thing", class),
    );
    let b = if conflict {
        DefinitionSource::new(
            "file:///imports/b.ecore",
            SourceKind::Ecore,
            b.text.replace(
                "</ecore:EPackage>",
                r#"<eClassifiers xsi:type="ecore:EClass" name="Model"/></ecore:EPackage>"#,
            ),
        )
    } else {
        b
    };
    let mut a = DefinitionSource::new(
        "file:///imports/a.ecore",
        SourceKind::Ecore,
        r##"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="a" nsURI="urn:a"><eClassifiers xsi:type="ecore:EClass" name="Model"><eStructuralFeatures xsi:type="ecore:EReference" name="items" upperBound="-1" containment="true" eType="b.ecore#//Thing"/></eClassifiers></ecore:EPackage>"##.replace("Thing", class),
    );
    a.dependencies.push(b.id.clone());
    let mut x = DefinitionSource::new("model.xtext", SourceKind::Xtext, grammar);
    x.dependencies = vec![a.id.clone(), b.id.clone()];
    let mut s = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    s.dependencies.push(a.id.clone());
    let input = BundleCompilation {
        ecore: a.id.clone(),
        xtext: Some(x.id.clone()),
        rcore_behavior: s.id.clone(),
        namespace: "urn:a".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![a, b, x, s],
            namespaces: vec![],
        },
    };
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(input).unwrap()
}
const GRAMMAR: &str = r#"grammar example.Imports hidden(WS)
import "urn:a" as p
import "urn:b" as q
Model returns p::Model: items+=Item*;
Item returns q::Thing: 'thing' name=ID ';';
terminal ID: ('a'..'z'|'A'..'Z')+;
terminal WS: (' '|'\t'|'\r'|'\n')+;
"#;
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn multiple_packages_and_aliases_execute_with_exact_identity() {
    multiple_packages_and_aliases_execute_with_exact_identity_with_profile(Profile::NativeV20);
}
fn multiple_packages_and_aliases_execute_with_exact_identity_with_profile(profile: Profile) {
    for grammar in [
        GRAMMAR.to_owned(),
        GRAMMAR
            .replace("as q", "as p")
            .replace("q::Thing", "p::Thing"),
        GRAMMAR
            .replace(" as p", "")
            .replace(" as q", "")
            .replace("p::Model", "Model")
            .replace("q::Thing", "Thing"),
    ] {
        let i = input(&grammar);
        assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV19).is_err());
        let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
        let r = evaluate(
            &p,
            &[Source {
                uri: "example.model".into(),
                text: "thing Wheel;".into(),
            }],
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert_eq!(r.model.elements.len(), 2);
        let element = &r.model.elements[1];
        assert_eq!(element.values["name"], "Wheel");
        let identity = &p.schema.declarations[&element.class];
        assert_eq!(identity.namespace, "urn:b");
        assert_eq!(identity.source.uri, "file:///imports/b.ecore.rcore");
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(compiler::recompile(&p).unwrap()).unwrap()
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn missing_imports_and_duplicate_namespaces_reject() {
    missing_imports_and_duplicate_namespaces_reject_with_profile(Profile::NativeV20);
}
fn missing_imports_and_duplicate_namespaces_reject_with_profile(profile: Profile) {
    for grammar in [
        GRAMMAR.replace("urn:b", "urn:missing"),
        GRAMMAR.replace("q::Thing", "missing::Thing"),
        GRAMMAR.replace("q::Thing", "q::Missing"),
        GRAMMAR.replace(
            "import \"urn:b\" as q",
            "import \"urn:b\" as q import \"urn:b\" as other",
        ),
    ] {
        assert!(
            compiler::compile_bundle_with_profile(input(&grammar), profile).is_err(),
            "{grammar}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn shared_alias_ambiguity_rejects_without_first_match() {
    shared_alias_ambiguity_rejects_without_first_match_with_profile(Profile::NativeV20);
}
fn shared_alias_ambiguity_rejects_without_first_match_with_profile(profile: Profile) {
    let grammar = GRAMMAR
        .replace("as q", "as p")
        .replace("q::Thing", "p::Thing");
    let err =
        compiler::compile_bundle_with_profile(input_case(&grammar, true), profile).unwrap_err();
    assert!(
        format!("{err:?}").contains("Ambiguous classifier p::Model"),
        "{err:?}"
    );
    let grammar = GRAMMAR.replace("p::Model", "Model");
    if profile == Profile::NativeV20 {
        assert!(compiler::compile_bundle_with_profile(input(&grammar), profile).is_err());
    } else {
        // V45 admitted unqualified TypeRefs across local aliased imports.
        assert!(compiler::compile_bundle_with_profile(input(&grammar), profile).is_ok());
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn pinned_ecore_alias_collision_uses_complete_inventory() {
    pinned_ecore_alias_collision_uses_complete_inventory_with_profile(Profile::NativeV20);
}
fn pinned_ecore_alias_collision_uses_complete_inventory_with_profile(profile: Profile) {
    let g = GRAMMAR.replace("q::Thing", "q::EClass").replace(
        "import \"urn:b\" as q",
        "import \"urn:b\" as q import \"http://www.eclipse.org/emf/2002/Ecore\" as q",
    );
    let err = compiler::compile_bundle_with_profile(input_named(&g, false, "EClass"), profile)
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("Ambiguous classifier q::EClass"),
        "{err:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn inherited_rules_keep_declaring_metamodel_context() {
    inherited_rules_keep_declaring_metamodel_context_with_profile(Profile::NativeV20);
}
fn inherited_rules_keep_declaring_metamodel_context_with_profile(profile: Profile) {
    let base = input(&GRAMMAR.replace("items+=Item*", "items+=Item+"));
    let child_text = r#"grammar example.Child with example.Imports import "urn:a" as root Entry returns root::Model: Model;"#;
    let compose = |text: &str| {
        let mut i = base.clone();
        let mut child = DefinitionSource::new("child.xtext", SourceKind::Xtext, text);
        child.dependencies = vec![i.ecore.clone(), i.xtext.clone().unwrap()];
        i.xtext = Some(child.id.clone());
        i.bundle.sources.push(child);
        i
    };
    let p = compiler::compile_bundle_with_profile(compose(child_text), profile).unwrap();
    let r = evaluate(
        &p,
        &[Source {
            uri: "inherited.model".into(),
            text: "thing Wheel;".into(),
        }],
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(
        p.schema.declarations[&r.model.elements[1].class].namespace,
        "urn:b"
    );
    let grammar = p.grammar.as_ref().unwrap();
    assert_eq!(
        Some(&grammar.declaration_sources["Item"].source),
        base.xtext.as_ref()
    );
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(compiler::recompile(&p).unwrap()).unwrap()
    );
    for (suffix, message) in [
        (
            "Item returns q::Thing: 'child' name=ID ';';",
            "Unknown metamodel alias q",
        ),
        (
            "Item returns root::Thing: 'child' name=ID ';';",
            "Unknown rule class Thing",
        ),
    ] {
        let error = compiler::compile_bundle_with_profile(
            compose(&format!("{child_text} {suffix}")),
            profile,
        )
        .unwrap_err();
        assert!(format!("{error:?}").contains(message), "{error:?}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    multiple_packages_and_aliases_execute_with_exact_identity_with_profile(Profile::NativeV51);
    missing_imports_and_duplicate_namespaces_reject_with_profile(Profile::NativeV51);
    shared_alias_ambiguity_rejects_without_first_match_with_profile(Profile::NativeV51);
    pinned_ecore_alias_collision_uses_complete_inventory_with_profile(Profile::NativeV51);
    inherited_rules_keep_declaring_metamodel_context_with_profile(Profile::NativeV51);
}
