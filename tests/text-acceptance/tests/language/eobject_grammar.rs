use super::Store;
use intersect_text::package::Package;
#[path = "common/eobject_grammar.rs"]
mod common;
use intersect_model::core::storage::{self};
use intersect_model::definitions::compiler::materialize_runtime_types;
use intersect_model::definitions::schema::EOBJECT_REFERENCE_TOP;
use intersect_model::definitions::schema::eobject_reference_top_declaration;
use intersect_text::compiler::compile_bundle;
use intersect_text::grammar;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_into};
use serde_json::json;

control_test! {
pub fn explicit_pinned_references_link_unrelated_raw_and_applied_objects_and_replay() {
    let input = common::fixture();
    let package = compile_bundle(input).unwrap();
    let result = evaluate(&package, &[common::source()]);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.model.validate(&package.schema).unwrap();
    let root = &result.model.elements[0];
    assert_eq!(root.values["any"].as_array().unwrap().len(), 5);
    let raw = result
        .model
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&json!("R")))
        .unwrap();
    let applied = result
        .model
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&json!("A")))
        .unwrap();
    assert!(package.schema.instantiations[&raw.class].has_raw());
    assert!(!package.schema.instantiations[&applied.class].has_raw());
    assert_ne!(raw.class, applied.class);
    let local = intersect_text::grammar::type_names::reference_key(&package.schema, &package.schema.uri, "EObject")
        .unwrap();
    assert_ne!(local, EOBJECT_REFERENCE_TOP);
    assert_eq!(
        intersect_text::grammar::type_names::reference_key(&package.schema, "http://www.eclipse.org/emf/2002/Ecore", "EObject")
            .unwrap(),
        EOBJECT_REFERENCE_TOP
    );
    let retained: Package = serde_json::from_value(json!(package)).unwrap();
    let rebuilt = compile_bundle(retained.compilation.clone().unwrap()).unwrap();
    assert_eq!(json!(retained), json!(rebuilt));
    assert_eq!(
        json!(evaluate(&rebuilt, &[common::source()])),
        json!(result)
    );
}
}

store_test! {
pub fn candidate_filtering_preserves_feature_domains_and_failed_imports_roll_back() {
    let package = compile_bundle(common::fixture()).unwrap();
    let mut store = crate::stores::store::<Store>(&package.schema).unwrap();
    let source = Source {
        uri: "memory:filtered".into(),
        text: "node Shared other Shared bound B target Shared narrow Shared".into(),
    };
    let result =
        evaluate_into(&package, &[source], EvaluationLimits::native(), &mut store).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let before = storage::export(&store).unwrap();
    let node = before
        .elements
        .iter()
        .find(|e| e.class == package.schema.language_class_key("Node").unwrap())
        .unwrap();
    let root = before
        .elements
        .iter()
        .find(|e| e.class == package.schema.language_class_key("Root").unwrap())
        .unwrap();
    assert_eq!(root.values["narrow"], json!(node.id));
    let bound = before
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&json!("B")))
        .unwrap();
    assert_eq!(bound.values["target"], json!(node.id));
    for (text, spelling, diagnostic) in [
        ("node Shared other Shared any Shared", "Shared", "Ambiguous"),
        ("node N any Missing", "Missing", "Unresolved"),
        ("other O narrow O", "O", "Unresolved"),
        ("node N user N", "N", "Unresolved"),
        ("other O bound B target O", "O", "Unresolved"),
    ] {
        let result = evaluate_into(
            &package,
            &[Source {
                uri: "memory:bad".into(),
                text: text.into(),
            }],
            EvaluationLimits::native(),
            &mut store,
        )
        .unwrap();
        let error = result
            .diagnostics
            .iter()
            .find(|error| error.message.contains(diagnostic))
            .unwrap();
        let start = text.rfind(spelling).unwrap();
        assert_eq!((error.start, error.end), (start, start + spelling.len()));
        assert_eq!(storage::export(&store).unwrap(), before);
    }
}
}

control_test! {
pub fn import_identity_and_rule_construction_boundaries_are_explicit() {
    let package = compile_bundle(common::fixture()).unwrap();
    let source = &package.sources["xtext"];
    for text in [
        source.replace("http://www.eclipse.org/emf/2002/Ecore", "urn:fake-ecore"),
        source.replace("[e::EObject", "[missing::EObject"),
        source.replace("as e", "as m"),
        source.replace("[e::EObject", "[e::EClass"),
        source.replace("[e::EObject", "[__ecore_eobject_reference_top"),
        source.replace("User returns m::EObject", "User returns e::EObject"),
    ] {
        assert!(grammar::compile(&text, &package.schema).is_err(), "{text}");
    }
    let mut stale = package.schema.clone();
    stale.reference_top.as_mut().unwrap().source.sha256 = "stale".into();
    let before = stale.clone();
    assert!(grammar::compile_with_reference_context(source, &mut stale).is_err());
    assert_eq!(stale, before);
    // Unqualified EObject still selects the local source class.
    let local = source.replace("[m::EObject", "[EObject");
    let compiled = grammar::compile(&local, &package.schema).unwrap();
    assert_eq!(json!(compiled), json!(package.grammar));
}
}

control_test! {
pub fn grammar_only_top_context_is_hashed_once_and_failed_preparation_is_atomic() {
    let xml = include_str!("../../../../fixtures/ecore-generics/eobject-grammar.ecore").replace(
        "eType=\"e:EClass http://www.eclipse.org/emf/2002/Ecore#//EObject\"",
        "eType=\"#//Node\"",
    );
    let input = common::from_text(&xml);
    let mut schema =
        materialize_runtime_types(&input.bundle, &input.materialized_types, None).unwrap();
    assert!(schema.reference_top.is_none());
    let source = &input
        .bundle
        .sources
        .iter()
        .find(|s| Some(&s.id) == input.xtext.as_ref())
        .unwrap()
        .text;
    let original = schema.clone();
    let mut failed = schema.clone();
    assert!(
        grammar::compile_with_reference_context(
            &source.replace("Root returns m::Root", "Root returns e::EObject"),
            &mut failed
        )
        .is_err()
    );
    assert_eq!(failed, original);
    let first = grammar::compile_with_reference_context(source, &mut schema).unwrap();
    assert_eq!(
        schema.reference_top,
        Some(eobject_reference_top_declaration().unwrap())
    );
    assert_ne!(schema.revision, original.revision);
    let once = schema.clone();
    let second = grammar::compile_with_reference_context(source, &mut schema).unwrap();
    assert_eq!(schema, once);
    assert_eq!(json!(first), json!(second));
    let package = compile_bundle(input).unwrap();
    let result = evaluate(
        &package,
        &[Source {
            uri: "memory:narrow-top".into(),
            text: "node N any N".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}
}

control_test! {
pub fn unqualified_return_types_preserve_the_rule_delimiter_and_exact_class_binding() {
    let package = compile_bundle(common::fixture()).unwrap();
    let qualified = &package.sources["xtext"];
    let unqualified = qualified.replace("returns m::", "returns ");
    let grammar = grammar::compile(&unqualified, &package.schema).unwrap();
    assert_eq!(json!(grammar), json!(package.grammar));
    let mut unqualified_package = package.clone();
    unqualified_package.grammar = Some(grammar);
    let outcome = evaluate(&unqualified_package, &[common::source()]);
    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    assert_eq!(
        json!(outcome),
        json!(evaluate(&package, &[common::source()]))
    );
    let unknown = qualified.replace("returns m::Node", "returns unknown::Node");
    assert!(grammar::compile(&unknown, &package.schema).is_err());
    let mut foreign = package.schema.clone();
    let key = foreign.language_class_key("Node").unwrap().to_owned();
    foreign.declarations.get_mut(&key).unwrap().namespace = "urn:foreign".into();
    assert!(intersect_text::grammar::type_names::reference_key(&foreign, &foreign.uri, &key).is_err());
    assert!(
        grammar::compile(
            &qualified.replace("returns m::Node", &format!("returns m::{key}")),
            &foreign
        )
        .is_err()
    );
}
}
