use intersect_text::compiler::compile_bundle;
use intersect_text::package::BundleCompilation;
use intersect_text::{Source, evaluate};
use serde_json::json;
fn input() -> BundleCompilation {
    serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-generics/language-bundle-canonical.json"
    ))
    .unwrap()
}
#[test]
fn generic_bundle_parses_links_derives_and_replays_from_exact_inputs() {
    let package = compile_bundle(input()).unwrap();
    let sources = [
        Source {
            uri: "one.model".into(),
            text: "root { value v; }".into(),
        },
        Source {
            uri: "two.model".into(),
            text: "root { box v; }".into(),
        },
    ];
    let result = evaluate(&package, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.model.validate(&package.schema).unwrap();
    let key = |name| package.schema.language_class_key(name).unwrap();
    let value = result
        .model
        .elements
        .iter()
        .find(|e| e.class == key("Value"))
        .unwrap();
    let boxed = result
        .model
        .elements
        .iter()
        .find(|e| e.class == key("ValueBox"))
        .unwrap();
    assert_eq!(boxed.values["value"], json!(value.id));
    assert_eq!(result.references.len(), 1);
    let first = result
        .model
        .elements
        .iter()
        .find(|e| e.class == key("Root") && e.resource == "one.model")
        .unwrap();
    assert_eq!(result.derived[&first.id]["valueCount"], json!(1));
    let replay = compile_bundle(
        serde_json::from_value(serde_json::to_value(package.compilation.unwrap()).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(evaluate(&replay, &sources)).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    let invalid = evaluate(
        &replay,
        &[Source {
            uri: "bad.model".into(),
            text: "root { box absent; }".into(),
        }],
    );
    assert!(!invalid.diagnostics.is_empty());
}
#[test]
fn generic_bundle_rejects_missing_selection_and_wrong_entry_namespace() {
    let mut request = input();
    request.materialized_types.clear();
    assert!(compile_bundle(request).is_err());
    let mut request = input();
    request.namespace = "urn:missing".into();
    assert!(
        compile_bundle(request).unwrap_err()[0]
            .message
            .contains("Entry namespace")
    );
    let mut request = input();
    let mut stale = request.materialized_types[0].expression().clone();
    if let intersect_model::definitions::generic_types::Type::Classifier { declaration, .. } =
        &mut stale
    {
        declaration.source.sha256 = "0".repeat(64);
    }
    request.materialized_types[0] =
        intersect_model::definitions::generic_types::ClosedType::new(stale)
            .unwrap()
            .into();
    assert!(compile_bundle(request).is_err());
}

#[test]
fn grammar_names_reject_ambiguous_generic_applications() {
    use intersect_model::definitions::generic_types::{ClosedType, Type};
    use intersect_model::definitions::resources::DeclarationId;
    let mut request = input();
    let ty = |index, arguments| Type::Classifier {
        declaration: DeclarationId {
            source: request.ecore.clone(),
            path: vec![index],
        },
        arguments,
        raw: false,
    };
    let other = ty(1, vec![]);
    let other_box = ty(2, vec![other.clone()]);
    request
        .materialized_types
        .push(ClosedType::new(other).unwrap().into());
    request
        .materialized_types
        .push(ClosedType::new(other_box).unwrap().into());
    let package = compile_bundle(request).unwrap();
    assert!(
        package
            .schema
            .language_class_key("Box")
            .unwrap_err()
            .contains("Ambiguous")
    );
    let grammar = package.sources["xtext"].replace("returns g::ValueBox", "returns g::Box");
    assert!(
        intersect_text::grammar::compile(&grammar, &package.schema)
            .unwrap_err()
            .message
            .contains("Ambiguous")
    );
}

#[test]
fn declared_selection_packages_recompile_and_parse_without_changing_overload_groups() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        generic_types::{ClosedType, Type},
    };
    let fixture =
        include_str!("../../../../fixtures/ecore-generics/materialize-operation-selection.ecore");
    for (text, count) in [
        (fixture.to_owned(), 2),
        (fixture.replace("eType=\"#//B\"", "eType=\"#//A\""), 1),
    ] {
        let source = DefinitionSource::new("memory:selection.ecore", SourceKind::Ecore, text);
        let (source, declarations) =
            intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
        let ty = |n| {
            ClosedType::new(Type::Classifier {
                declaration: declarations[&vec![n]].clone(),
                arguments: vec![],
                raw: false,
            })
            .unwrap()
        };
        let requested = vec![ty(0), ty(1), ty(3)];
        let mut grammar = DefinitionSource::new(
            "memory:selection.xtext",
            SourceKind::Xtext,
            "grammar example.Selection import \"urn:g\" as g\nDerived returns g::Derived: 'derived';",
        );
        grammar.dependencies.push(source.id.clone());
        let mut semantics = DefinitionSource::new(
            "memory:selection-behavior.rcore",
            SourceKind::RcoreBehavior,
            r#"rcore 1; package emptyRules version "1";"#,
        );
        semantics.dependencies.push(source.id.clone());
        let request = BundleCompilation {
            ecore: source.id.clone(),
            namespace: "urn:g".into(),
            xtext: Some(grammar.id.clone()),
            rcore_behavior: semantics.id.clone(),
            materialized_types: requested.into_iter().map(Into::into).collect(),
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![source, grammar, semantics],
                namespaces: vec![],
            },
        };
        let package = compile_bundle(request).unwrap();
        let key = package.schema.language_class_key("Derived").unwrap();
        assert_eq!(
            package.schema.resolved_operations(key).unwrap().len(),
            count
        );
        let sources = [Source {
            uri: "one.model".into(),
            text: "derived".into(),
        }];
        let result = evaluate(&package, &sources);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        let replay = compile_bundle(
            serde_json::from_value(
                serde_json::to_value(package.compilation.clone().unwrap()).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(&package).unwrap()
        );
        assert_eq!(
            serde_json::to_value(evaluate(&replay, &sources)).unwrap(),
            serde_json::to_value(result).unwrap()
        );
    }
}

#[test]
fn method_template_package_replays_and_constructs_models() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        generic_resolution::Resolver,
        generic_types::{ClosedType, Type},
    };
    let source = DefinitionSource::new(
        "memory:templates.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/method-templates.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let operation = declarations[&vec![1, 1]].clone();
    let types: Vec<_> = [0, 2]
        .into_iter()
        .map(|n| {
            ClosedType::new(Type::Classifier {
                declaration: declarations[&vec![n]].clone(),
                arguments: vec![],
                raw: false,
            })
            .unwrap()
        })
        .collect();
    let mut grammar = DefinitionSource::new(
        "memory:templates.xtext",
        SourceKind::Xtext,
        "grammar example.Templates import \"urn:templates\" as t\nChild returns t::Child: 'child';",
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:templates-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let request = BundleCompilation {
        ecore: source.id.clone(),
        namespace: "urn:templates".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        materialized_types: types.iter().map(Into::into).collect(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![source, grammar, semantics],
            namespaces: vec![],
        },
    };
    let resolver = Resolver::new(&request.bundle).unwrap();
    let package = compile_bundle(request).unwrap();
    let applied = package
        .schema
        .instantiate_generic_operation(&resolver, &types[1], &operation, &types[..1])
        .unwrap();
    let replay = compile_bundle(
        serde_json::from_value(serde_json::to_value(package.compilation.clone().unwrap()).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&replay).unwrap(),
        serde_json::to_value(&package).unwrap()
    );
    assert_eq!(
        replay
            .schema
            .instantiate_generic_operation(&resolver, &types[1], &operation, &types[..1])
            .unwrap(),
        applied
    );
    let result = evaluate(
        &replay,
        &[Source {
            uri: "one.model".into(),
            text: "child".into(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
}
