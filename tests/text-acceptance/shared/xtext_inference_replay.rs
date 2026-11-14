//! Independent source authority and host ceiling controls for inferred packages.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::SourceLimits;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy, Package};
use serde_json::{Value, json};
fn execute(package: &Package) -> intersect_text::Evaluation {
    intersect_text::evaluate(
        package,
        &[intersect_text::Source {
            uri: "memory:replay.test".into(),
            text: "alpha".into(),
        }],
    )
}
pub fn source_authority() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/cases.json"
    ))
    .unwrap();
    let request = super::composition::input(&cases[0]);
    let language = request.compile(SourceLimits::default()).unwrap();
    let serialized = serde_json::to_value(language.package()).unwrap();
    assert_eq!(
        serialized["xtext_inference"],
        serde_json::to_value(&request).unwrap()
    );
    assert!(serialized.get("compilation").is_none());
    let package: Package = serde_json::from_value(serialized.clone()).unwrap();
    let replay = intersect_text::recompile(&package).unwrap();
    assert!(replay.grammar.is_some());
    assert_eq!(serde_json::to_value(&replay).unwrap(), serialized);
    assert!(
        intersect_text::compiler::recompile(&package).unwrap_err()[0]
            .message
            .contains("intersect-text")
    );
    let expected = execute(&replay);
    assert!(expected.diagnostics.is_empty());
    assert_eq!(expected.model.elements[0].values["name"], "alpha");

    let mut forged = package.clone();
    forged.schema.uri = "urn:forged-cache".into();
    forged.grammar = None;
    forged.sources.clear();
    let rebuilt = intersect_text::recompile(&forged).unwrap();
    assert_eq!(serde_json::to_value(&rebuilt).unwrap(), serialized);
    assert_eq!(execute(&rebuilt).model, expected.model);

    let mut negatives = vec![];
    let mut invalid = package.clone();
    invalid.version = 99;
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.xtext_inference.as_mut().unwrap().version = 99;
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid
        .xtext_inference
        .as_mut()
        .unwrap()
        .bundle
        .sources
        .clear();
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.xtext_inference.as_mut().unwrap().bundle.sources[0]
        .text
        .push(' ');
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.xtext_inference.as_mut().unwrap().grammar.sha256 = "0".repeat(64);
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.xtext_inference = None;
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.source_policy = None;
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.source_policy.as_mut().unwrap().xtext_bytes += 1;
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.source_policy.as_mut().unwrap().structure_bytes -= 1;
    negatives.push(invalid);
    let mut invalid = package.clone();
    invalid.source_policy.as_mut().unwrap().xtext_declarations = 0;
    negatives.push(invalid);
    let mut invalid = package.clone();
    let definitions = language.definitions();
    invalid.compilation = Some(BundleCompilation {
        materialized_types: definitions.materialized_types.clone(),
        bundle: definitions.bundle.clone(),
        ecore: definitions.entry_source.clone(),
        namespace: definitions.namespace.clone(),
        xtext: None,
        rcore_behavior: definitions.entry_source.clone(),
    });
    let mut legacy = invalid.clone();
    legacy.xtext_inference = None;
    assert!(intersect_text::compiler::recompile(&legacy).is_err());
    negatives.push(legacy);
    negatives.push(invalid);
    for invalid in negatives {
        assert!(intersect_text::recompile(&invalid).is_err());
        assert_eq!(
            execute(&intersect_text::recompile(&package).unwrap()).model,
            expected.model
        );
    }
    let mut invalid = serialized.clone();
    invalid["xtext_inference"]["schema"] = json!({});
    assert!(serde_json::from_value::<Package>(invalid).is_err());

    // A valid edited source revision is the authority, even when cached IR still
    // describes the previous name. Preserve exact IDs rather than patching IR.
    let mut changed = package.clone();
    let inputs = changed.xtext_inference.as_mut().unwrap();
    let original = &inputs.bundle.sources[0];
    let mut source = DefinitionSource::new(
        original.id.uri.clone(),
        SourceKind::Xtext,
        original.text.replace("name=ID", "title=ID"),
    );
    source.dependencies = original.dependencies.clone();
    inputs.grammar = source.id.clone();
    inputs.bundle.sources[0] = source;
    let changed = intersect_text::recompile(&changed).unwrap();
    let changed_result = execute(&changed);
    assert!(changed_result.diagnostics.is_empty());
    assert_eq!(changed_result.model.elements[0].values["title"], "alpha");
    assert!(!changed_result.model.elements[0].values.contains_key("name"));
    assert_ne!(changed.schema.revision, package.schema.revision);
    assert_eq!(execute(&replay).model, expected.model);
    assert!(
        intersect_text::recompile_with_source_policy(
            &package,
            CompilationSourcePolicy {
                structure_bytes: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
pub fn verify() -> usize {
    source_authority();
    1
}
