use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar;
use intersect_text::grammar::Profile;
const ECORE: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/model.ecore");
const BASE: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/base.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/contextual-overrides/model.xtext");
fn inputs(base: &str, child: &str) -> (DefinitionBundle, DefinitionSource) {
    let base = DefinitionSource::new("base.xtext", SourceKind::Xtext, base);
    let mut child = DefinitionSource::new("model.xtext", SourceKind::Xtext, child);
    child.dependencies.push(base.id.clone());
    (
        DefinitionBundle {
            version: 1,
            sources: vec![child.clone(), base],
            namespaces: vec![],
        },
        child,
    )
}
#[test]
fn contextual_selection_retains_annotation_and_shadowed_source_span() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, child) = inputs(BASE, CHILD);
    let grammar =
        grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV3).unwrap();
    assert_eq!(grammar.entry, "Entry");
    assert!(grammar.rules["Member"].override_declared);
    assert_eq!(grammar.declaration_sources["Member"].source, child.id);
    let replaced = &grammar.shadowed_declarations["Member"][0];
    assert_eq!(replaced.source.uri, "base.xtext");
    assert!(replaced.text(&bundle).unwrap().contains("'type'"));
    assert!(
        grammar.declaration_sources["Member"]
            .text(&bundle)
            .unwrap()
            .starts_with("@Override")
    );
    bundle.sources.reverse();
    let reordered =
        grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV3).unwrap();
    assert_eq!(
        serde_json::to_value(grammar).unwrap(),
        serde_json::to_value(reordered).unwrap()
    );
}
#[test]
fn old_profile_and_orphan_unknown_or_terminal_annotations_reject() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (bundle, child) = inputs(BASE, CHILD);
    assert!(grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2).is_err());
    for text in [
        CHILD.replace("Member returns", "Missing returns"),
        CHILD.replace("@Override", "@Final"),
        CHILD.replace("@Override", "@Override @Override"),
        CHILD.replace("@Override\nMember", "@Override\nterminal Member"),
    ] {
        let (bundle, child) = inputs(BASE, &text);
        assert!(
            grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV3).is_err(),
            "{text}"
        );
    }
    assert!(
        grammar::compile_with_profile(
            &BASE.replace("Member returns", "@Override Member returns"),
            &schema,
            Profile::NativeV3
        )
        .unwrap_err()
        .message
        .contains("exact inherited")
    );
}
#[test]
fn unsupported_override_contracts_and_shadowed_semantic_errors_reject() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    for text in [
        CHILD.replace("Member returns k::Type", "Member returns k::Feature"),
        CHILD.replace("Member returns", "fragment Member returns"),
        CHILD.replace("Member returns k::Type", "Member returns k::Model"),
    ] {
        let (bundle, child) = inputs(BASE, &text);
        assert!(
            grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV3)
                .unwrap_err()
                .message
                .contains("same model return type")
        );
    }
    let (bundle, child) = inputs(
        &BASE.replace("'type' name=Name", "'type' absent=Name"),
        CHILD,
    );
    let before = serde_json::to_value(&schema).unwrap();
    let error =
        grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV3).unwrap_err();
    assert_eq!(error.source, "base.xtext");
    assert!(error.start > 0);
    assert_eq!(before, serde_json::to_value(schema).unwrap());
}
#[test]
fn unannotated_and_three_level_overrides_keep_all_declarations() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, middle) = inputs(BASE, &CHILD.replace("@Override", ""));
    let mut top = DefinitionSource::new(
        "top.xtext",
        SourceKind::Xtext,
        CHILD
            .replace(
                "example.OverrideChild with example.OverrideBase",
                "example.OverrideTop with example.OverrideChild",
            )
            .replace("Entry returns", "TopEntry returns")
            .replace("'class'", "'struct'"),
    );
    top.dependencies.push(middle.id.clone());
    bundle.sources.push(top.clone());
    let grammar =
        grammar::compile_sources(&bundle, &top.id, &mut schema, Profile::NativeV3).unwrap();
    assert_eq!(
        grammar.shadowed_declarations["Member"]
            .iter()
            .map(|span| span.source.uri.as_str())
            .collect::<Vec<_>>(),
        ["model.xtext", "base.xtext"]
    );
    assert_eq!(grammar.declaration_sources["Member"].source, top.id);
}
#[test]
fn shadowed_declarations_do_not_evade_the_aggregate_budget() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let mut base = BASE.to_owned();
    for n in 0..120 {
        base.push_str(&format!(
            "\nExtra{n} returns k::Type: 'x{n}' name=Name ';';"
        ));
    }
    let mut child = CHILD.to_owned();
    for n in 0..2 {
        child.push_str(&format!(
            "\nExtra{n} returns k::Type: 'y{n}' name=Name ';';"
        ));
    }
    let (bundle, child) = inputs(&base, &child);
    assert!(
        grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV3)
            .unwrap_err()
            .message
            .contains("rule budget")
    );
}
