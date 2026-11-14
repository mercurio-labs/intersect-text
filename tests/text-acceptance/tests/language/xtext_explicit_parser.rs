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
fn input(base: &str, child: &str) -> BundleCompilation {
    let e = DefinitionSource::new("model.ecore", SourceKind::Ecore, ECORE);
    let mut b = DefinitionSource::new("base.xtext", SourceKind::Xtext, base);
    b.dependencies.push(e.id.clone());
    let mut c = DefinitionSource::new("child.xtext", SourceKind::Xtext, child);
    c.dependencies = vec![e.id.clone(), b.id.clone()];
    let mut s = DefinitionSource::new(
        "model-behavior.rcore",
        SourceKind::RcoreBehavior,
        include_str!("../../../../fixtures/xtext/contextual-overrides/model-behavior.rcore"),
    );
    s.dependencies.push(e.id.clone());
    let result = BundleCompilation {
        ecore: e.id.clone(),
        xtext: Some(c.id.clone()),
        rcore_behavior: s.id.clone(),
        namespace: "urn:intersect:xtext:mini-kerml:contextual-overrides:1".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![e, b, c, s],
            namespaces: vec![],
        },
    };
    intersect_text_acceptance::migration_fixture::migrate_bundle_fixture(result).unwrap()
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_parent_parser_dispatch_and_replay() {
    explicit_parent_parser_dispatch_and_replay_with_profile(Profile::NativeV17);
}
fn explicit_parent_parser_dispatch_and_replay_with_profile(profile: Profile) {
    for spelling in [
        "super",
        "super::Member",
        "OverrideBase::Member",
        "example::OverrideBase::Member",
    ] {
        let c = CHILD.replace("'class' name=Name ';'", spelling);
        let i = input(BASE, &c);
        assert!(compiler::compile_bundle_with_profile(i.clone(), Profile::NativeV16).is_err());
        let p = compiler::compile_bundle_with_profile(i, profile).unwrap();
        let replay = compiler::recompile(&p).unwrap();
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        for p in [&p, &replay] {
            for (text, ok) in [
                ("package p { type Wheel; }", true),
                ("package p { class Wheel; }", false),
                ("package p { type ; }", false),
            ] {
                let r = evaluate(
                    p,
                    &[Source {
                        uri: "parent.model".into(),
                        text: text.into(),
                    }],
                );
                assert_eq!(
                    r.diagnostics.is_empty(),
                    ok,
                    "{spelling}: {:?}",
                    r.diagnostics
                );
                if ok {
                    assert_eq!(r.model.elements.len(), 2);
                    assert_eq!(r.model.elements[1].values["name"], "Wheel");
                } else {
                    assert!(r.model.elements.is_empty());
                }
            }
        }
        assert_eq!(
            p.grammar.as_ref().unwrap().declaration_sources["@example.OverrideBase::Member"]
                .source
                .uri,
            "base.xtext"
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn retained_parent_parser_keeps_contextual_nested_calls() {
    retained_parent_parser_keeps_contextual_nested_calls_with_profile(Profile::NativeV17);
}
fn retained_parent_parser_keeps_contextual_nested_calls_with_profile(profile: Profile) {
    let c = format!("{CHILD} Model returns k::Model: super;");
    let p = compiler::compile_bundle_with_profile(input(BASE, &c), profile).unwrap();
    for (text, ok) in [
        ("package p { class Wheel; }", true),
        ("package p { type Wheel; }", false),
    ] {
        let r = evaluate(
            &p,
            &[Source {
                uri: "context.model".into(),
                text: text.into(),
            }],
        );
        assert_eq!(r.diagnostics.is_empty(), ok, "{:?}", r.diagnostics);
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn parser_explicit_terminal_visibility_and_missing_targets() {
    parser_explicit_terminal_visibility_and_missing_targets_with_profile(Profile::NativeV17);
}
fn parser_explicit_terminal_visibility_and_missing_targets_with_profile(profile: Profile) {
    let c = CHILD
        .replace("name=Name", "name=OverrideBase::ID")
        .to_owned()
        + " terminal ID: '0'..'9'+;";
    let base = format!("{BASE} terminal OTHER: 'Wheel';");
    let base = if profile == Profile::NativeV51 {
        let errors = compiler::compile_bundle_with_profile(input(&base, &c), profile).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.source == "base.xtext"
                    && e.message.contains("Unreachable terminal OTHER")),
            "{errors:?}"
        );
        // Preserve the historical shadowed token control; use a reachable
        // token to test the independent current-profile ancestor-call contract.
        base.replace("terminal OTHER: 'Wheel'", "terminal OTHER: '@Wheel'")
    } else {
        base
    };
    let p = compiler::compile_bundle_with_profile(input(&base, &c), profile).unwrap();
    let order = &p.grammar.as_ref().unwrap().terminal_order;
    assert!(
        order
            .iter()
            .position(|n| n == "@example.OverrideBase::ID")
            .unwrap()
            < order.iter().position(|n| n == "OTHER").unwrap()
    );
    let r = evaluate(
        &p,
        &[Source {
            uri: "terminal.model".into(),
            text: "package 1 { class Wheel; }".into(),
        }],
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(r.model.elements[1].values["name"], "Wheel");
    for spelling in [
        "Missing::Member",
        "OverrideBase::Missing",
        "OverrideChild::Member",
    ] {
        let c = CHILD.replace("'class' name=Name ';'", spelling);
        assert!(
            compiler::compile_bundle_with_profile(input(BASE, &c), profile).is_err(),
            "{spelling}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_inferred_model_fragment_and_source_limits() {
    explicit_inferred_model_fragment_and_source_limits_with_profile(Profile::NativeV17);
}
fn explicit_inferred_model_fragment_and_source_limits_with_profile(profile: Profile) {
    use intersect_text::grammar;
    use intersect_text::grammar::SourceLimits;
    let base = BASE.replace("Model returns k::Model", "Model");
    let child = format!("{CHILD} Model returns k::Model: super;");
    let i = input(&base, &child);
    let p = compiler::compile_bundle_with_profile(i.clone(), profile).unwrap();
    let r = evaluate(
        &p,
        &[Source {
            uri: "inferred.model".into(),
            text: "package p { class Wheel; }".into(),
        }],
    );
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(r.model.elements[0].class, "Model");
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let before = serde_json::to_value(&schema).unwrap();
    assert!(
        grammar::compile_sources_with_limits(
            &i.bundle,
            i.xtext.as_ref().unwrap(),
            &mut schema,
            profile,
            SourceLimits {
                source_bytes: 256_000,
                declarations: 7
            }
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&schema).unwrap(), before);
    assert!(
        grammar::compile_sources_with_limits(
            &i.bundle,
            i.xtext.as_ref().unwrap(),
            &mut schema,
            profile,
            SourceLimits {
                source_bytes: 256_000,
                declarations: 8
            }
        )
        .is_ok()
    );
    let base = BASE
        .replace("'type' name=Name ';'", "'type' Tail")
        .to_owned()
        + " fragment Tail returns k::Type: name=Name ';';";
    for body in [
        "'class' OverrideBase::Tail",
        "'class' name=OverrideBase::Name ';'",
    ] {
        let child = CHILD.replace("'class' name=Name ';'", body);
        let p = compiler::compile_bundle_with_profile(input(&base, &child), profile).unwrap();
        let r = evaluate(
            &p,
            &[Source {
                uri: "fragment.model".into(),
                text: "package p { class Wheel; }".into(),
            }],
        );
        assert!(r.diagnostics.is_empty(), "{body}: {:?}", r.diagnostics);
        assert_eq!(r.model.elements[1].values["name"], "Wheel");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn parent_cardinality_matches_pinned_semantic_rejection() {
    parent_cardinality_matches_pinned_semantic_rejection_with_profile(Profile::NativeV17);
}
fn parent_cardinality_matches_pinned_semantic_rejection_with_profile(profile: Profile) {
    use intersect_text::grammar;
    use intersect_text::grammar::SourceLimits;
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/terminal-overrides/inheritance-observations.json"
    ))
    .unwrap();
    assert_eq!(rows.len(), 4);
    for row in rows {
        let count = row["parents"].as_u64().unwrap() as usize;
        let accepted = row["error_indices"].as_array().unwrap().is_empty();
        assert_eq!(accepted, count <= 1);
        if count == 0 {
            let source = DefinitionSource::new("base.xtext", SourceKind::Xtext, BASE);
            assert!(
                grammar::syntax::parse(&source, SourceLimits::default())
                    .unwrap()
                    .parents
                    .is_empty()
            );
            assert!(
                intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                    ECORE,
                    BASE,
                    include_str!(
                        "../../../../fixtures/xtext/contextual-overrides/model-behavior.rcore"
                    ),
                    profile
                )
                .is_ok()
            );
            continue;
        }
        let parents = std::iter::once("example.OverrideBase".to_owned())
            .chain((1..count).map(|n| format!("example.Parent{n}")))
            .collect::<Vec<_>>()
            .join(", ");
        let child = CHILD.replace("with example.OverrideBase", &format!("with {parents}"));
        let source = DefinitionSource::new("child.xtext", SourceKind::Xtext, &child);
        assert_eq!(
            grammar::syntax::parse(&source, SourceLimits::default())
                .unwrap()
                .parents
                .len(),
            count
        );
        let mut i = input(BASE, &child);
        for n in 1..count {
            let mut parent = DefinitionSource::new(
                format!("parent{n}.xtext"),
                SourceKind::Xtext,
                BASE.replace("example.OverrideBase", &format!("example.Parent{n}")),
            );
            parent.dependencies.push(i.ecore.clone());
            i.bundle
                .sources
                .iter_mut()
                .find(|s| Some(&s.id) == i.xtext.as_ref())
                .unwrap()
                .dependencies
                .push(parent.id.clone());
            i.bundle.sources.push(parent);
        }
        let result = compiler::compile_bundle_with_profile(i, profile);
        assert_eq!(result.is_ok(), accepted);
        if !accepted {
            assert!(
                result
                    .unwrap_err()
                    .iter()
                    .any(|e| e.message.contains("Multiple grammar inheritance"))
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn current_profile_contracts() {
    explicit_parent_parser_dispatch_and_replay_with_profile(Profile::NativeV51);
    retained_parent_parser_keeps_contextual_nested_calls_with_profile(Profile::NativeV51);
    parser_explicit_terminal_visibility_and_missing_targets_with_profile(Profile::NativeV51);
    explicit_inferred_model_fragment_and_source_limits_with_profile(Profile::NativeV51);
    parent_cardinality_matches_pinned_semantic_rejection_with_profile(Profile::NativeV51);
}
