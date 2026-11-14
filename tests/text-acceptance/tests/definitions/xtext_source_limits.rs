use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
const ECORE: &str = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:x='http://www.w3.org/2001/XMLSchema-instance' name='m' nsURI='urn:limits'><eClassifiers x:type='e:EClass' name='Item'/></e:EPackage>";
fn source(uri: &str, header: &str, start: usize, count: usize) -> DefinitionSource {
    let mut text = format!(
        "{header}\nimport 'urn:limits' as m\nimport 'http://www.eclipse.org/emf/2002/Ecore' as ecore\n"
    );
    for n in start..start + count {
        text.push_str(&format!("R{n} returns m::Item: 'r{n}';\n"));
    }
    DefinitionSource::new(uri, SourceKind::Xtext, text)
}
#[test]
fn exact_composed_budget_counts_shadowed_declarations_and_preserves_sources() {
    let mut parent = source("memory:parent", "grammar limits.Parent hidden(WS)", 0, 140);
    parent.text.push_str("terminal WS: ' '+;\n");
    parent = DefinitionSource::new(parent.id.uri, SourceKind::Xtext, parent.text);
    let mut child = source(
        "memory:child",
        "grammar limits.Child with limits.Parent",
        140,
        140,
    );
    child
        .text
        .push_str("@Override R0 returns m::Item: 'replacement';\n");
    child = DefinitionSource::new(child.id.uri, SourceKind::Xtext, child.text);
    child.dependencies.push(parent.id.clone());
    let mut bundle = DefinitionBundle {
        version: 1,
        sources: vec![parent.clone(), child.clone()],
        namespaces: vec![],
    };
    let base = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
        .unwrap();
    let limits = SourceLimits {
        source_bytes: 256_000,
        declarations: 282,
    };
    let mut schema = base.clone();
    let grammar = grammar::compile_sources_with_limits(
        &bundle,
        &child.id,
        &mut schema,
        Profile::NativeV3,
        limits,
    )
    .unwrap();
    assert_eq!(grammar.rules.len(), 280);
    assert_eq!(grammar.terminals.len(), 1);
    assert_eq!(grammar.shadowed_declarations["R0"].len(), 1);
    assert_eq!(grammar.shadowed_declarations["R0"][0].source, parent.id);
    assert_eq!(grammar.declaration_sources["R0"].source, child.id);
    assert_eq!(grammar.declaration_sources["R1"].source, parent.id);
    assert!(schema.reference_top.is_some());
    for policy in [
        SourceLimits::default(),
        SourceLimits {
            declarations: 281,
            ..limits
        },
    ] {
        let mut denied = base.clone();
        let error = grammar::compile_sources_with_limits(
            &bundle,
            &child.id,
            &mut denied,
            Profile::NativeV3,
            policy,
        )
        .unwrap_err();
        assert_eq!(error.message, "Grammar rule budget exceeded");
        assert_eq!(denied, base);
    }
    assert!(
        grammar::compile_sources(&bundle, &child.id, &mut base.clone(), Profile::NativeV3).is_err()
    );
    bundle.sources.reverse();
    let mut reordered = base.clone();
    let replay = grammar::compile_sources_with_limits(
        &bundle,
        &child.id,
        &mut reordered,
        Profile::NativeV3,
        limits,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&grammar).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    assert_eq!(schema, reordered);
    assert_eq!(bundle.source(&parent.id).unwrap(), &parent);
    assert_eq!(bundle.source(&child.id).unwrap(), &child);
}
#[test]
fn explicit_byte_policy_keeps_default_rejection_and_does_not_grant_syntax() {
    let mut input = source("memory:large", "grammar limits.Large hidden(WS)", 0, 1);
    input
        .text
        .push_str(&format!("terminal WS: ' '+; /*{}*/", "x".repeat(256_000)));
    input = DefinitionSource::new(input.id.uri, SourceKind::Xtext, input.text);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![input.clone()],
        namespaces: vec![],
    };
    let base = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
        .unwrap();
    let limits = SourceLimits {
        source_bytes: input.text.len(),
        declarations: 2,
    };
    let accepted = grammar::compile_sources_with_limits(
        &bundle,
        &input.id,
        &mut base.clone(),
        Profile::NativeV3,
        limits,
    )
    .unwrap();
    assert_eq!(accepted.declaration_sources["R0"].source, input.id);
    for policy in [
        SourceLimits::default(),
        SourceLimits {
            source_bytes: limits.source_bytes - 1,
            ..limits
        },
    ] {
        let mut schema = base.clone();
        let error = grammar::compile_sources_with_limits(
            &bundle,
            &input.id,
            &mut schema,
            Profile::NativeV3,
            policy,
        )
        .unwrap_err();
        assert_eq!(error.message, "Definition size budget exceeded");
        assert_eq!(error.source, input.id.uri);
        assert_eq!(schema, base);
    }
    let error = grammar::compile_sources_with_limits(
        &bundle,
        &input.id,
        &mut base.clone(),
        Profile::NativeV1,
        limits,
    )
    .unwrap_err();
    assert!(error.message.contains("predictive native profile"));
    let mut forged = bundle.clone();
    forged.sources[0].text.push(' ');
    assert!(
        grammar::compile_sources_with_limits(
            &forged,
            &input.id,
            &mut base.clone(),
            Profile::NativeV3,
            SourceLimits {
                source_bytes: limits.source_bytes + 1,
                ..limits
            }
        )
        .is_err()
    );
}
