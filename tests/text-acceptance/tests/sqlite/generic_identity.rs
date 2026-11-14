#![cfg(not(target_arch = "wasm32"))]
use intersect_model::core::{
    model::Element,
    storage::{self, ModelStore},
};
use intersect_model::definitions::generic_types::{ClosedType, Type};
use intersect_model::sqlite::SqliteStore;
use intersect_text::package::BundleCompilation;
use serde_json::json;
#[path = "../core/common/generic_schema.rs"]
mod generic_schema;

#[test]
fn instantiated_identity_and_reference_rollback_survive_sqlite_reopen() {
    let schema = generic_schema::schema();
    let path = std::env::temp_dir().join(format!(
        "intersect-generic-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let expected;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        let checkpoint = store.begin().unwrap();
        for (id, class) in [("a", "box_value"), ("b", "box_other"), ("use", "Use")] {
            storage::insert(
                &mut store,
                &schema,
                Element::new(&schema, id, class, "memory:model", 0, 0).unwrap(),
            )
            .unwrap();
        }
        store.commit(&checkpoint).unwrap();
        storage::edit(
            &mut store,
            &schema,
            &[("use".into(), "value".into(), json!("a"))],
        )
        .unwrap();
        expected = storage::export(&store).unwrap();
        assert!(
            storage::edit(
                &mut store,
                &schema,
                &[("use".into(), "value".into(), json!("b"))]
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
        let mut forged =
            Element::new(&schema, "forged", "box_value", "memory:model", 0, 0).unwrap();
        forged.type_identity = schema.identity("box_other");
        assert!(storage::insert(&mut store, &schema, forged).is_err());
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        storage::validate(&store, &schema).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
        assert_ne!(
            expected.elements[0].type_identity,
            expected.elements[1].type_identity
        );
        let mut changed = schema.clone();
        let mut expression = changed.instantiations["box_value"].expression().clone();
        if let Type::Classifier { arguments, .. } = &mut expression
            && let Type::Classifier { declaration, .. } = &mut arguments[0]
        {
            declaration.source.sha256 = "1".repeat(64);
        }
        changed.instantiations.insert(
            "box_value".into(),
            ClosedType::new(expression).unwrap().into(),
        );
        assert_eq!(changed.revision, schema.revision);
        assert!(storage::validate(&store, &changed).is_err());
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn compiled_generic_package_and_instances_survive_real_database_reopen() {
    let package = intersect_text::compiler::compile_bundle(
        serde_json::from_str(include_str!(
            "../../../../fixtures/ecore-generics/language-bundle-canonical.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let schema = &package.schema;
    let boxed = schema.language_class_key("Box").unwrap();
    let value = schema.language_class_key("Value").unwrap();
    let root = schema.language_class_key("Root").unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-compiled-generic-{}.sqlite",
        std::process::id()
    ));
    let expected;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        let checkpoint = store.begin().unwrap();
        for (id, class) in [("b", boxed), ("v", value), ("r", root)] {
            storage::insert(
                &mut store,
                schema,
                Element::new(schema, id, class, "memory:generic", 0, 0).unwrap(),
            )
            .unwrap();
        }
        store.commit(&checkpoint).unwrap();
        storage::edit(
            &mut store,
            schema,
            &[("b".into(), "value".into(), json!("v"))],
        )
        .unwrap();
        expected = storage::export(&store).unwrap();
        assert!(
            expected
                .elements
                .iter()
                .find(|e| e.id == "b")
                .unwrap()
                .type_identity
                .starts_with("intersect:generic:1:")
        );
        assert!(
            storage::edit(
                &mut store,
                schema,
                &[("b".into(), "value".into(), json!("r"))]
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        let restored = intersect_text::compiler::compile_bundle(
            serde_json::from_value(
                serde_json::to_value(package.compilation.as_ref().unwrap()).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        storage::validate(&store, &restored.schema).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn method_template_application_is_rechecked_after_database_and_package_reopen() {
    use intersect_model::definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        generic_resolution::Resolver,
    };
    use intersect_text::compiler::compile_bundle;
    let source = DefinitionSource::new(
        "memory:templates.ecore",
        SourceKind::Ecore,
        include_str!("../../../../fixtures/ecore-generics/method-templates.ecore"),
    );
    let (source, declarations) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let operation = declarations[&vec![1, 1]].clone();
    let types: Vec<_> = [0, 2, 3]
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
    let path = std::env::temp_dir().join(format!(
        "intersect-template-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    // SQLite stores instances. Retained compilation inputs are a separate package file.
    let package_path = path.with_extension("package.json");
    let (expected_model, expected_template, expected_application);
    {
        let resolver = Resolver::new(&request.bundle).unwrap();
        let package = compile_bundle(request).unwrap();
        let schema = &package.schema;
        let key = schema.language_class_key("Child").unwrap();
        expected_template = schema.operation_template(key, &operation).unwrap().clone();
        expected_application = schema
            .instantiate_generic_operation(&resolver, &types[1], &operation, &types[..1])
            .unwrap();
        let mut store = SqliteStore::open(&path).unwrap();
        let checkpoint = store.begin().unwrap();
        storage::insert(
            &mut store,
            schema,
            Element::new(schema, "child", key, "memory:model", 0, 5).unwrap(),
        )
        .unwrap();
        store.commit(&checkpoint).unwrap();
        expected_model = storage::export(&store).unwrap();
        std::fs::write(
            &package_path,
            serde_json::to_vec(package.compilation.as_ref().unwrap()).unwrap(),
        )
        .unwrap();
    }
    {
        let request: BundleCompilation =
            serde_json::from_slice(&std::fs::read(&package_path).unwrap()).unwrap();
        let resolver = Resolver::new(&request.bundle).unwrap();
        let package = compile_bundle(request).unwrap();
        let schema = &package.schema;
        let key = schema.language_class_key("Child").unwrap();
        let store = SqliteStore::open(&path).unwrap();
        storage::validate(&store, schema).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected_model);
        assert_eq!(
            schema.operation_template(key, &operation).unwrap(),
            &expected_template
        );
        assert_eq!(
            schema
                .instantiate_generic_operation(&resolver, &types[1], &operation, &types[..1])
                .unwrap(),
            expected_application
        );
        // Reopening cannot bypass arity, bounds, or exact operation source identity.
        for arguments in [vec![], vec![types[2].clone()], vec![types[0].clone(); 2]] {
            assert!(
                schema
                    .instantiate_generic_operation(&resolver, &types[1], &operation, &arguments)
                    .is_err()
            );
        }
        let mut stale = operation.clone();
        stale.source.sha256 = "0".repeat(64);
        assert!(
            schema
                .instantiate_generic_operation(&resolver, &types[1], &stale, &types[..1])
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected_model);
    }
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(package_path).unwrap();
}
