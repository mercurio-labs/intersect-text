use intersect_model::definitions::scalars::Builtin;
use intersect_text::compiler;
use intersect_text::{Source, evaluate};

enum Library {}
impl intersect_model::core::query::Entity for Library {
    const CLASS: &'static str = "Library";
    const SCHEMA_URI: &'static str = "urn:intersect:example:library:1";
}

fn package(
    text: &str,
) -> Result<intersect_text::package::Package, Vec<intersect_model::definitions::schema::Diagnostic>>
{
    compiler::compile_rcore_project(
        text,
        include_str!("../../../../fixtures/library/library.xtext"),
    )
}

#[test]
fn typed_library_rules_preserve_shared_evaluation_and_replay() {
    let text = include_str!("../../../../fixtures/library/library-project.rcore");
    let package = package(text).unwrap();
    let structural =
        compiler::compile_rcore_with_profile(text, None, None, Default::default()).unwrap();
    assert!(structural.grammar.is_none());
    assert_eq!(
        serde_json::to_value(&structural.semantics).unwrap(),
        serde_json::to_value(&package.semantics).unwrap()
    );
    let legacy = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    for loans in [
        "loan first copy dune to ada;",
        "loan first copy dune to ada; loan second copy dune to ada;",
        "",
    ] {
        let sources = [Source {
            uri: "memory:catalog".into(),
            text: format!(
                "library Catalog {{ copy dune \"Dune\"; copy earthsea \"Earthsea\"; member ada; {loans} }}"
            ),
        }];
        let result = evaluate(&package, &sources);
        let old = evaluate(&legacy, &sources);
        let mut expected = old.derived.clone();
        for values in expected.values_mut() {
            if let Some(count) = values.get_mut("copyCount") {
                *count = Builtin::ELong.parse(&count.to_string()).unwrap();
            }
        }
        assert_eq!(result.derived, expected);
        assert_eq!(result.diagnostics, old.diagnostics);
        assert_eq!(
            serde_json::to_value(&result.references).unwrap(),
            serde_json::to_value(&old.references).unwrap()
        );
        let root = result
            .model
            .elements
            .iter()
            .find(|e| e.class == "Library")
            .unwrap();
        assert_eq!(
            package.schema.classes["Library"].features["copyCount"].target,
            "ELong"
        );
        assert_eq!(
            result.derived[&root.id]["copyCount"],
            serde_json::json!({"kind":"int64","value":"2"})
        );
        let view = intersect_model::core::storage::ModelView::new(&result.model).unwrap();
        let binding =
            intersect_model::core::query::Binding::<Library>::new(&package.schema).unwrap();
        let query = binding
            .query()
            .select(binding.field::<i64>("copyCount").unwrap());
        let output = query
            .execute(
                &package.schema,
                &view,
                &intersect_model::core::query::Parameters::new(),
                &intersect_model::query::PackageDerived(&package),
                intersect_model::core::query::QueryLimits::default(),
            )
            .unwrap();
        assert_eq!(output.rows, [2]);
        assert_eq!(
            query
                .plan()
                .unwrap()
                .execute(
                    &package.schema,
                    &view,
                    &intersect_model::core::query::Parameters::new(),
                    &intersect_model::query::PackageDerived(&package),
                    intersect_model::core::query::QueryLimits::default()
                )
                .unwrap()
                .rows,
            [vec![serde_json::json!({"kind":"int64","value":"2"})]]
        );
        assert_eq!(
            result.diagnostics.len(),
            2 * usize::from(loans.contains("second"))
        );
        let replay = compiler::recompile(&package).unwrap();
        assert_eq!(evaluate(&replay, &sources).derived, result.derived);
    }
    let dynamic = evaluate(
        &package,
        &[Source {
            uri: "memory:branch.library".into(),
            text: "library Branch { member grace; }".into(),
        }],
    );
    assert!(dynamic.diagnostics.is_empty(), "{:?}", dynamic.diagnostics);
    let branch = dynamic
        .model
        .elements
        .iter()
        .find(|e| e.class == "Library")
        .unwrap();
    assert_eq!(
        dynamic.derived[&branch.id]["copyCount"],
        serde_json::json!({"kind":"int64","value":"0"})
    );
}

#[test]
fn typed_activation_rejects_unimplemented_rules_and_type_errors_at_authored_offsets() {
    let text = include_str!("../../../../fixtures/library/library-project.rcore");
    for (replacement, expected) in [
        (
            "1 + size(self.copies)",
            "Derivation requires a supported builtin or a direct external call",
        ),
        ("true", "Type mismatch"),
    ] {
        let changed = text.replace("size(self.copies)", replacement);
        let errors = package(&changed).unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains(expected)),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .all(|e| e.start >= changed.find("behavior {").unwrap())
        );
    }
}
