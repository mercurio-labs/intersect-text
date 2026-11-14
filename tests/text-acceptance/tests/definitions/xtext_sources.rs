use intersect_model::definitions::bundle::BUNDLE_VERSION;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::Profile;
use intersect_text::grammar::compile_sources;
const ECORE: &str = include_str!("../../../../fixtures/xtext/inherited-grammar/model.ecore");
const BASE: &str = include_str!("../../../../fixtures/xtext/inherited-grammar/base.xtext");
const CHILD: &str = include_str!("../../../../fixtures/xtext/inherited-grammar/model.xtext");
fn inputs() -> (DefinitionBundle, DefinitionSource) {
    let base = DefinitionSource::new("base.xtext", SourceKind::Xtext, BASE);
    let mut child = DefinitionSource::new("model.xtext", SourceKind::Xtext, CHILD);
    child.dependencies.push(base.id.clone());
    (
        DefinitionBundle {
            version: BUNDLE_VERSION,
            sources: vec![child.clone(), base],
            namespaces: vec![],
        },
        child,
    )
}
#[test]
fn inherited_declarations_keep_entry_hidden_policy_and_exact_source_spans() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, child) = inputs();
    let grammar = compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2).unwrap();
    assert_eq!(grammar.entry, "Model");
    assert_eq!(
        grammar.hidden.as_deref(),
        Some(["WS".to_owned()].as_slice())
    );
    assert_eq!(grammar.rules.len(), 3);
    assert_eq!(grammar.terminals.len(), 2);
    assert_eq!(grammar.declaration_sources["Type"].source.uri, "base.xtext");
    assert_eq!(grammar.declaration_sources["Model"].source, child.id);
    assert!(
        grammar.declaration_sources["Type"]
            .text(&bundle)
            .unwrap()
            .starts_with("Type returns")
    );
    let before = serde_json::to_value(&grammar).unwrap();
    bundle.sources.reverse();
    let reordered = compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2).unwrap();
    assert_eq!(before, serde_json::to_value(reordered).unwrap());
}
#[test]
fn missing_parent_and_legacy_profiles_reject() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, child) = inputs();
    bundle.sources[0].dependencies.clear();
    let error = compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2).unwrap_err();
    assert_eq!(error.source, "model.xtext");
    assert!(error.message.contains("Missing exact parent"));
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV1)
            .unwrap_err()
            .message
            .contains("predictive native")
    );
}
#[test]
fn overrides_and_ambiguous_parent_identities_reject() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, mut child) = inputs();
    let duplicate = DefinitionSource::new("other-base.xtext", SourceKind::Xtext, BASE);
    child.dependencies.push(duplicate.id.clone());
    bundle.sources[0] = child.clone();
    bundle.sources.push(duplicate);
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("Ambiguous parent")
    );
    let (mut bundle, old) = inputs();
    let mut child = DefinitionSource::new(
        "model.xtext",
        SourceKind::Xtext,
        format!("{CHILD}\nType returns k::Type: 'type' name=Name ';';"),
    );
    child.dependencies = old.dependencies;
    bundle.sources[0] = child.clone();
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("override")
    );
}
#[test]
fn parent_parse_diagnostics_retain_parent_source_and_offset() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, mut child) = inputs();
    let base = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        BASE.replace("name=Name", "name=@"),
    );
    child.dependencies = vec![base.id.clone()];
    bundle.sources = vec![child.clone(), base];
    let error = compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2).unwrap_err();
    assert_eq!(error.source, "base.xtext");
    assert!(error.start > 0);
}

#[test]
fn inherited_semantic_errors_keep_parent_declaration_and_restore_context() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let before = serde_json::to_value(&schema).unwrap();
    let (mut bundle, mut child) = inputs();
    let base = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        BASE.replace("name=Name", "absent=Name"),
    );
    child.dependencies = vec![base.id.clone()];
    bundle.sources = vec![child.clone(), base.clone()];
    let diagnostic =
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2).unwrap_err();
    assert_eq!(diagnostic.source, "base.xtext");
    assert!(base.text[diagnostic.start..diagnostic.end].starts_with("Type returns"));
    assert_eq!(before, serde_json::to_value(schema).unwrap());
}

#[test]
fn cyclic_and_duplicate_ancestor_grammar_identities_reject() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (_, mut child) = inputs();
    let mut base = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        BASE.replace(
            "example.Base hidden",
            "example.Base with example.Model hidden",
        ),
    );
    child.dependencies = vec![base.id.clone()];
    base.dependencies = vec![child.id.clone()];
    let mut bundle = DefinitionBundle {
        version: BUNDLE_VERSION,
        sources: vec![child.clone(), base],
        namespaces: vec![],
    };
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("Cyclic")
    );
    let root = DefinitionSource::new(
        "root.xtext",
        SourceKind::Xtext,
        BASE.replace("grammar example.Base", "grammar example.Model"),
    );
    let mut middle = DefinitionSource::new(
        "middle.xtext",
        SourceKind::Xtext,
        CHILD
            .replace(
                "example.Model with example.Base",
                "example.Base with example.Model",
            )
            .replace("Model returns", "Middle returns"),
    );
    middle.dependencies = vec![root.id.clone()];
    child.dependencies = vec![middle.id.clone()];
    bundle.sources = vec![child.clone(), middle, root];
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("Duplicate grammar identity")
    );
}
#[test]
fn exact_source_kind_digest_size_and_aggregate_rule_limits_reject() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, child) = inputs();
    bundle.sources[0].kind = SourceKind::Ecore;
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("must be Xtext")
    );
    let (mut bundle, child) = inputs();
    bundle.sources[1].text.push(' ');
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("digest mismatch")
    );
    let (_, mut child) = inputs();
    let huge = DefinitionSource::new(
        "huge.xtext",
        SourceKind::Xtext,
        format!("{}/*{}*/", BASE, "x".repeat(256_000)),
    );
    child.dependencies = vec![huge.id.clone()];
    let mut bundle = DefinitionBundle {
        version: BUNDLE_VERSION,
        sources: vec![child.clone(), huge],
        namespaces: vec![],
    };
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("size budget")
    );
    let mut text = BASE.to_owned();
    for n in 0..124 {
        text.push_str(&format!("\nExtra{n} returns k::Type: 'extra{n}';"));
    }
    let base = DefinitionSource::new("base.xtext", SourceKind::Xtext, text);
    child.dependencies = vec![base.id.clone()];
    bundle.sources = vec![child.clone(), base];
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("rule budget")
    );
}

#[test]
fn multiple_parent_syntax_rejects_explicitly() {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let (mut bundle, original) = inputs();
    let mut child = DefinitionSource::new(
        "model.xtext",
        SourceKind::Xtext,
        CHILD.replace("with example.Base", "with example.Base, example.Other"),
    );
    child.dependencies = original.dependencies;
    bundle.sources[0] = child.clone();
    assert!(
        compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV2)
            .unwrap_err()
            .message
            .contains("Multiple grammar inheritance")
    );
}
