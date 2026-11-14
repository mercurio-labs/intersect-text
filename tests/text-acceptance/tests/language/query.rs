use super::Store;
use intersect_model::core::{
    query::*,
    storage::{self, ModelRead, ModelStore},
};
use intersect_model::query::PackageDerived;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate_into};
use serde_json::{Value, json};

// Handwritten markers for this bounded fixture; not generated language support.
macro_rules! entity {
    ($name:ident) => {
        enum $name {}
        impl Entity for $name {
            const CLASS: &'static str = stringify!($name);
            const SCHEMA_URI: &'static str = "urn:intersect:example:library:1";
        }
    };
}
entity!(BookCopy);
entity!(Loan);
entity!(Library);
entity!(Named);
fn fixture() -> (Package, Store) {
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let sources = [
        Source {
            uri: "catalog.library".into(),
            text: include_str!("../../../../fixtures/library/catalog.library").into(),
        },
        Source {
            uri: "loans.library".into(),
            text: include_str!("../../../../fixtures/library/loans.library").into(),
        },
    ];
    let mut store = crate::stores::store::<Store>(&package.schema).unwrap();
    let result = evaluate_into(&package, &sources, EvaluationLimits::native(), &mut store).unwrap();
    assert!(result.diagnostics.is_empty());
    (package, store)
}
fn plan(package: &Package, from: &str, columns: &[&str]) -> QueryPlan {
    QueryPlan {
        schema_uri: package.schema.uri.clone(),
        schema_revision: package.schema.revision.clone(),
        from: from.into(),
        filter: None,
        select: columns
            .iter()
            .map(|p| Column {
                name: (*p).into(),
                expression: Expression::Field { path: (*p).into() },
            })
            .collect(),
        order_by: vec![],
        limit: 50,
    }
}
fn run(plan: &QueryPlan, package: &Package, store: &dyn ModelRead) -> QueryOutput {
    plan.execute(
        &package.schema,
        store,
        &Parameters::new(),
        &PackageDerived(package),
        QueryLimits::default(),
    )
    .unwrap()
}

store_test! {
pub fn typed_projection_matches_serialized_query_and_uses_language_derivation() {
    let (package, store) = fixture();
    let b = Binding::<BookCopy>::new(&package.schema).unwrap();
    let query = b
        .query()
        .filter(b.field::<Option<bool>>("available").unwrap().eq(true))
        .select((
            b.field::<String>("$id").unwrap(),
            b.field::<String>("name").unwrap(),
            b.field::<Option<String>>("title").unwrap(),
            b.field::<Option<i64>>("edition").unwrap(),
        ));
    let typed = query
        .execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default(),
        )
        .unwrap();
    assert_eq!(
        typed.rows,
        vec![(
            "catalog.library#2".into(),
            "earthsea".into(),
            Some("A Wizard of Earthsea".into()),
            Some(2)
        )]
    );
    let serialized = serde_json::to_value(query.plan().unwrap()).unwrap();
    let dynamic: QueryPlan = serde_json::from_value(serialized).unwrap();
    assert_eq!(
        run(&dynamic, &package, &store).rows,
        vec![vec![
            json!("catalog.library#2"),
            json!("earthsea"),
            json!("A Wizard of Earthsea"),
            json!(2)
        ]]
    );
}
}
store_test! {
pub fn parameters_navigate_references_and_select_native_tuples() {
    let (package, store) = fixture();
    let b = Binding::<Loan>::new(&package.schema).unwrap();
    let parameter = Parameter::<String>::new("borrower");
    let query = b
        .query()
        .filter(
            b.field::<String>("member.name")
                .unwrap()
                .eq_parameter(&parameter),
        )
        .select((
            b.field::<String>("name").unwrap(),
            b.field::<Option<String>>("copy.title").unwrap(),
        ));
    let mut parameters = Parameters::new();
    parameter.bind(&mut parameters, "ada".into());
    let result = query
        .execute(
            &package.schema,
            &store,
            &parameters,
            &PackageDerived(&package),
            QueryLimits::default(),
        )
        .unwrap();
    assert_eq!(result.rows, vec![("first".into(), Some("Dune".into()))]);
    assert!(
        query
            .execute(
                &package.schema,
                &store,
                &Parameters::new(),
                &PackageDerived(&package),
                QueryLimits::default()
            )
            .is_err()
    );
    parameters.insert("borrower".into(), json!(4));
    assert!(
        query
            .execute(
                &package.schema,
                &store,
                &parameters,
                &PackageDerived(&package),
                QueryLimits::default()
            )
            .is_err()
    );
}
}
store_test! {
pub fn collection_quantifiers_preserve_empty_collection_semantics() {
    let (package, store) = fixture();
    let library = Binding::<Library>::new(&package.schema).unwrap();
    let copy = Binding::<BookCopy>::new(&package.schema).unwrap();
    let copies = library
        .field::<Vec<ReferenceId<BookCopy>>>("copies")
        .unwrap();
    let available = copy.field::<Option<bool>>("available").unwrap();
    let any = library
        .query()
        .filter(copies.any(available.eq(true)))
        .select((
            library.field::<String>("name").unwrap(),
            library.count("copies").unwrap(),
        ));
    let all = library
        .query()
        .filter(copies.all(available.eq(true)))
        .select(library.field::<String>("name").unwrap());
    assert_eq!(
        any.execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default()
        )
        .unwrap()
        .rows,
        vec![("Catalog".into(), 2)]
    );
    assert_eq!(
        all.execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default()
        )
        .unwrap()
        .rows,
        vec!["Circulation"]
    );
}
}
store_test! {
pub fn binding_rejects_wrong_types_cardinality_namespace_and_navigation() {
    let (package, _) = fixture();
    let b = Binding::<BookCopy>::new(&package.schema).unwrap();
    assert!(b.field::<String>("edition").is_err());
    assert!(b.field::<String>("title").is_err());
    assert!(b.field::<Option<String>>("title.name").is_err());
    assert!(b.field::<String>("library.name").is_err());
    assert!(b.field::<Option<String>>("library.name").is_ok());
    assert!(b.field::<String>("absent").is_err());
    let library = Binding::<Library>::new(&package.schema).unwrap();
    assert!(
        library
            .field::<Vec<Vec<ReferenceId<BookCopy>>>>("copies")
            .is_err()
    );
    assert!(
        library
            .field::<Vec<Option<ReferenceId<BookCopy>>>>("copies")
            .is_err()
    );
    let mut other = package.schema.clone();
    other.uri = "urn:other".into();
    assert!(Binding::<BookCopy>::new(&other).is_err());
}
}
store_test! {
pub fn inherited_selection_and_stable_ordering_report_more_rows() {
    let (package, store) = fixture();
    let b = Binding::<Named>::new(&package.schema).unwrap();
    let name = b.field::<String>("name").unwrap();
    let q = b
        .query()
        .select(name.clone())
        .order_by(name.ascending())
        .limit(3);
    let output = q
        .execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default(),
        )
        .unwrap();
    assert_eq!(output.rows, vec!["Catalog", "Circulation", "ada"]);
    assert!(output.has_more);
    let all = run(&plan(&package, "Named", &["name"]), &package, &store);
    assert_eq!(all.rows.len(), 6);
    assert!(!all.has_more);
}
}
store_test! {
pub fn optional_navigation_missing_values_and_dangling_targets_are_distinct() {
    let (package, mut store) = fixture();
    storage::atomic(&mut store, |s| {
        let mut e = s.get("catalog.library#2")?.ok_or("Missing test object")?;
        e.values.insert("library".into(), Value::Null);
        e.values.remove("title");
        s.put(e)
    })
    .unwrap();
    let q = plan(&package, "BookCopy", &["title", "library.name"]);
    assert_eq!(
        run(&q, &package, &store).rows[1],
        vec![Value::Null, Value::Null]
    );
    let copy = Binding::<BookCopy>::new(&package.schema).unwrap();
    let absent = copy
        .query()
        .filter(copy.field::<Option<String>>("title").unwrap().is_none())
        .select(copy.field::<String>("name").unwrap());
    assert_eq!(
        absent
            .execute(
                &package.schema,
                &store,
                &Parameters::new(),
                &PackageDerived(&package),
                QueryLimits::default()
            )
            .unwrap()
            .rows,
        vec!["earthsea"]
    );
    storage::atomic(&mut store, |s| {
        let mut e = s.get("catalog.library#2")?.ok_or("Missing test object")?;
        e.values.insert("library".into(), json!("missing"));
        s.put(e)
    })
    .unwrap();
    assert!(
        q.execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default()
        )
        .unwrap_err()
        .contains("Unavailable reference")
    );
}
}
store_test! {
pub fn prepared_query_observes_edits_and_rollback_without_stale_derived_values() {
    let (package, mut store) = fixture();
    let b = Binding::<BookCopy>::new(&package.schema).unwrap();
    let q = b
        .query()
        .filter(b.field::<Option<bool>>("available").unwrap().eq(true))
        .select(b.field::<String>("name").unwrap());
    let params = Parameters::new();
    let providers = PackageDerived(&package);
    let prepared = q.prepare(&package.schema, &params, &providers).unwrap();
    assert_eq!(
        prepared
            .execute(&store, QueryLimits::default())
            .unwrap()
            .rows,
        vec!["earthsea"]
    );
    let checkpoint = store.begin().unwrap();
    storage::edit(
        &mut store,
        &package.schema,
        &[(
            "loans.library#1".into(),
            "copy".into(),
            json!("catalog.library#2"),
        )],
    )
    .unwrap();
    assert_eq!(
        prepared
            .execute(&store, QueryLimits::default())
            .unwrap()
            .rows,
        vec!["dune"]
    );
    store.rollback(&checkpoint).unwrap();
    assert_eq!(
        prepared
            .execute(&store, QueryLimits::default())
            .unwrap()
            .rows,
        vec!["earthsea"]
    );
}
}
store_test! {
pub fn validation_precedes_execution_even_for_empty_inputs() {
    let (package, _) = fixture();
    let mut store = crate::stores::store::<Store>(&package.schema).unwrap();
    storage::atomic(&mut store, |s| s.set_revision(&package.schema.revision)).unwrap();
    for column in ["missing", "copies.name"] {
        assert!(
            plan(&package, "Library", &[column])
                .execute(
                    &package.schema,
                    &store,
                    &Parameters::new(),
                    &PackageDerived(&package),
                    QueryLimits::default()
                )
                .is_err()
        );
    }
    assert!(
        plan(&package, "BookCopy", &["available"])
            .execute(
                &package.schema,
                &store,
                &Parameters::new(),
                &NoDerivedValues,
                QueryLimits::default()
            )
            .unwrap_err()
            .contains("Unsupported derived")
    );
    let mut q = plan(&package, "BookCopy", &["name"]);
    q.filter = Some(Predicate::Compare {
        left: Expression::Field {
            path: "edition".into(),
        },
        comparison: Comparison::Eq,
        right: Expression::Literal {
            value: json!("two"),
        },
    });
    assert!(
        q.execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default()
        )
        .is_err()
    );
}
}
store_test! {
pub fn revision_budgets_and_unknown_wire_operators_are_explicit_errors() {
    let (package, store) = fixture();
    let mut q = plan(&package, "BookCopy", &["name"]);
    let budget = QueryLimits {
        max_steps: 1,
        max_results: 50,
    };
    assert!(
        q.execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            budget
        )
        .unwrap_err()
        .contains("budget exhausted")
    );
    q.schema_revision = "old".into();
    assert!(
        q.validate(
            &package.schema,
            &Parameters::new(),
            &PackageDerived(&package)
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<Predicate>(json!({"op":"sparql", "text":"SELECT *"})).is_err()
    );
    let b = Binding::<BookCopy>::new(&package.schema).unwrap();
    let mut other = package.schema.clone();
    other.revision = "new".into();
    let foreign = Binding::<BookCopy>::new(&other).unwrap();
    assert!(
        b.query()
            .select(foreign.field::<String>("name").unwrap())
            .plan()
            .is_err()
    );
}
}
store_test! {
pub fn indexed_filter_plan_and_boolean_predicates_preserve_results() {
    let (package, store) = fixture();
    let b = Binding::<BookCopy>::new(&package.schema).unwrap();
    let name = b.field::<String>("name").unwrap();
    let edition = b.field::<Option<i64>>("edition").unwrap();
    let q = b
        .query()
        .filter(name.eq("earthsea".into()).and(edition.ge(2)))
        .select(name.clone());
    let result = q
        .execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default(),
        )
        .unwrap();
    assert_eq!(result.rows, vec!["earthsea"]);
    assert_eq!(result.stats.candidates, 1);
    assert!(result.stats.access.starts_with("feature lookup"));
    let q = b
        .query()
        .filter(!(name.eq("earthsea".into()).or(edition.lt(1))))
        .select(name);
    assert_eq!(
        q.execute(
            &package.schema,
            &store,
            &Parameters::new(),
            &PackageDerived(&package),
            QueryLimits::default()
        )
        .unwrap()
        .rows,
        vec!["dune"]
    );
}
}

store_test! {
pub fn computed_set_state_reads_current_store_and_rollback_without_cached_values() {
    let (package, mut store) = fixture();
    let state = |store: &Store, id: &str, feature: &str| {
        storage::is_set_with_derived(
            store,
            &package.schema,
            id,
            feature,
            &PackageDerived(&package),
            &mut QueryBudget::new(100),
        )
        .unwrap()
    };
    assert!(state(&store, "catalog.library#0", "copyCount"));
    assert!(!state(&store, "loans.library#0", "copyCount"));
    assert!(!state(&store, "catalog.library#1", "available"));
    assert!(state(&store, "catalog.library#2", "available"));
    assert!(!state(&store, "catalog.library#1", "edition"));
    let original = storage::export(&store).unwrap();
    let checkpoint = store.begin().unwrap();
    storage::edit(
        &mut store,
        &package.schema,
        &[(
            "loans.library#1".into(),
            "copy".into(),
            json!("catalog.library#2"),
        )],
    )
    .unwrap();
    assert!(state(&store, "catalog.library#1", "available"));
    assert!(!state(&store, "catalog.library#2", "available"));
    // The store rejects attempts to cache a derived value as physical data.
    assert!(store
        .set_value("catalog.library#1", "available", Some(json!(false)))
        .is_err());
    assert!(state(&store, "catalog.library#1", "available"));
    store.rollback(&checkpoint).unwrap();
    assert_eq!(storage::export(&store).unwrap(), original);
    assert!(!state(&store, "catalog.library#1", "available"));
    assert!(state(&store, "catalog.library#2", "available"));
}
}

store_test! {
pub fn computed_set_state_rejects_missing_stale_invalid_and_exhausted_providers() {
    let (package, store) = fixture();
    let original = storage::export(&store).unwrap();
    let query = |provider: &dyn DerivedValues, steps| {
        storage::is_set_with_derived(
            &store,
            &package.schema,
            "catalog.library#0",
            "copyCount",
            provider,
            &mut QueryBudget::new(steps),
        )
    };
    assert!(
        query(&NoDerivedValues, 100)
            .unwrap_err()
            .contains("provider")
    );
    let mut stale = package.clone();
    stale.schema.revision.push('x');
    assert!(
        query(&PackageDerived(&stale), 100)
            .unwrap_err()
            .contains("revision")
    );
    let mut missing = package.clone();
    missing.semantics.derived.clear();
    assert!(
        query(&PackageDerived(&missing), 100)
            .unwrap_err()
            .contains("No semantic provider")
    );
    for steps in [0, 1, 2] {
        assert!(
            query(&PackageDerived(&package), steps)
                .unwrap_err()
                .contains("budget")
        );
    }
    struct Invalid<'a>(&'a str);
    impl DerivedValues for Invalid<'_> {
        fn schema_revision(&self) -> Option<&str> {
            Some(self.0)
        }
        fn supports(&self, _: &str, _: &str) -> bool {
            true
        }
        fn read(
            &self,
            _: &dyn ModelRead,
            _: &str,
            _: &str,
            _: &mut QueryBudget,
        ) -> Result<Value, String> {
            Ok(json!("not an integer"))
        }
    }
    assert!(query(&Invalid(&package.schema.revision), 100).is_err());
    for foreign_revision in [false, true] {
        let mut invalid_model = original.clone();
        if foreign_revision {
            invalid_model.schema_revision.push('x');
        } else {
            invalid_model
                .elements
                .iter_mut()
                .find(|e| e.id == "catalog.library#0")
                .unwrap()
                .type_identity = "foreign".into();
        }
        let view = storage::ModelView::new(&invalid_model).unwrap();
        let error = storage::is_set_with_derived(
            &view,
            &package.schema,
            "catalog.library#0",
            "copyCount",
            &PackageDerived(&package),
            &mut QueryBudget::new(100),
        )
        .unwrap_err();
        assert!(error.contains(if foreign_revision {
            "revision"
        } else {
            "identity"
        }));
    }
    assert_eq!(storage::export(&store).unwrap(), original);
    assert!(query(&PackageDerived(&package), 100).unwrap());
    for (id, feature) in [("missing", "copyCount"), ("catalog.library#0", "missing")] {
        assert!(
            storage::is_set_with_derived(
                &store,
                &package.schema,
                id,
                feature,
                &PackageDerived(&package),
                &mut QueryBudget::new(100)
            )
            .is_err()
        );
    }
}
}
