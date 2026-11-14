#[path = "common/raw_runtime.rs"]
mod common;
use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
#[test]
fn raw_package_constructs_from_text_and_recompiles_exact_inputs() {
    let input = common::input();
    let package = compile_bundle(input.clone()).unwrap();
    let source = [Source {
        uri: "memory:model".into(),
        text: "raw".into(),
    }];
    let result = evaluate(&package, &source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    result.model.validate(&package.schema).unwrap();
    let key = package
        .schema
        .class_key_for_type(&input.materialized_types[0])
        .unwrap();
    assert_eq!(result.model.elements[0].class, key);
    assert_eq!(
        package.schema.identity(key),
        input.materialized_types[0].identity()
    );
    assert!(!package.schema.runtime_warnings.is_empty());
    let replay: BundleCompilation = serde_json::from_value(
        serde_json::to_value(package.compilation.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
    let rebuilt = compile_bundle(replay).unwrap();
    assert_eq!(
        serde_json::to_value(evaluate(&rebuilt, &source)).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    assert_eq!(rebuilt.schema, package.schema);
}
