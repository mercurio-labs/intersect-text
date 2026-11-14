use super::Store;
use intersect_model::core::model::Element;
use intersect_model::core::storage::{
    self, Checkpoint, ModelRead, ModelStore, ObjectView, Query, Row, StorageResult,
};
use intersect_model::definitions::schema::Schema;
use intersect_text::package::Package;
use intersect_text::{
    EvaluationLimits, Source, StoredEvaluation, apply_stored_semantics, evaluate, evaluate_into,
    evaluate_stored,
};
use serde_json::{Value, json};

fn package() -> Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap()
}

fn source(uri: &str, text: &str) -> Source {
    Source {
        uri: uri.into(),
        text: text.into(),
    }
}

store_test! {
pub fn compact_evaluation_and_compatibility_export_share_the_same_result() {
    let package = package();
    let sources = vec![
        source(
            "catalog",
            include_str!("../../../../fixtures/library/catalog.library"),
        ),
        source(
            "loans",
            include_str!("../../../../fixtures/library/loans.library"),
        ),
    ];
    let compact = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
    // A newly evaluated store selects the indexes needed by this language.
    // Explicitly supplied stores retain their own policy and must evaluate to
    // the same linked model, defaults, opposites and semantic results.
    let mut all_indexed = crate::stores::store::<Store>(&package.schema).unwrap();
    let all_metadata = evaluate_into(
        &package,
        &sources,
        EvaluationLimits::native(),
        &mut all_indexed,
    )
    .unwrap();
    assert_eq!(
        storage::export(&compact.store).unwrap(),
        storage::export(&all_indexed).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&compact.evaluation).unwrap(),
        serde_json::to_value(&all_metadata).unwrap()
    );
    let owned = evaluate(&package, &sources);
    assert!(compact.evaluation.diagnostics.is_empty());
    assert_eq!(storage::export(&compact.store).unwrap(), owned.model);
    assert_eq!(compact.evaluation.derived, owned.derived);
    assert_eq!(
        serde_json::to_value(&compact.evaluation.references).unwrap(),
        serde_json::to_value(&owned.references).unwrap(),
    );
    // Source buffers are caller-owned and need not stay alive with the model.
    drop(sources);
    assert_eq!(compact.store.id_page(None, 10).unwrap().len(), 6);
}
}

/// Deliberately reject the materializing compatibility API. A consumer that
/// regresses to cloning whole objects must fail even on this small fixture.
struct ProjectionOnly<'a>(&'a dyn ModelRead);
impl ModelRead for ProjectionOnly<'_> {
    fn resource_records(
        &self,
    ) -> Result<
        std::collections::BTreeMap<String, intersect_model::core::resources::ResourceRecord>,
        String,
    > {
        self.0.resource_records()
    }
    fn resource_record(
        &self,
        uri: &str,
    ) -> Result<Option<intersect_model::core::resources::ResourceRecord>, String> {
        self.0.resource_record(uri)
    }

    fn revision(&self) -> StorageResult<String> {
        self.0.revision()
    }
    fn get(&self, _: &str) -> StorageResult<Option<Element>> {
        Err("Full object materialization is forbidden in this control".into())
    }
    fn page(&self, _: Option<u64>, _: usize) -> StorageResult<Vec<Row>> {
        Err("Full object pages are forbidden in this control".into())
    }
    fn class(&self, id: &str) -> StorageResult<Option<String>> {
        self.0.class(id)
    }
    fn query(&self, query: Query<'_>) -> StorageResult<Vec<String>> {
        self.0.query(query)
    }
    fn matches(&self, id: &str, query: Query<'_>) -> StorageResult<bool> {
        self.0.matches(id, query)
    }
    fn id_page(&self, after: Option<u64>, limit: usize) -> StorageResult<Vec<(u64, String)>> {
        self.0.id_page(after, limit)
    }
    fn value(&self, id: &str, field: &str) -> StorageResult<Option<Value>> {
        self.0.value(id, field)
    }
    fn value_len(&self, id: &str, field: &str) -> StorageResult<Option<usize>> {
        self.0.value_len(id, field)
    }
    fn collection_page(
        &self,
        id: &str,
        field: &str,
        offset: usize,
        limit: usize,
    ) -> StorageResult<Vec<Value>> {
        self.0.collection_page(id, field, offset, limit)
    }
    fn inspect(
        &self,
        id: &str,
        f: &mut dyn FnMut(ObjectView<'_>) -> StorageResult<()>,
    ) -> StorageResult<bool> {
        self.0.inspect(id, f)
    }
}

control_test! {
pub fn structural_validation_and_semantics_use_projections_without_materializing_objects() {
    let package = package();
    let sources = vec![
        source(
            "catalog",
            include_str!("../../../../fixtures/library/catalog.library"),
        ),
        source(
            "loans",
            include_str!("../../../../fixtures/library/loans.library"),
        ),
    ];
    let mut compact = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
    assert!(compact.evaluation.diagnostics.is_empty());
    let expected_derived = json!({
        "catalog#0": { "copyCount": 2 },
        "catalog#1": { "available": false },
        "catalog#2": { "available": true },
        "loans#0": { "copyCount": 0 },
    });
    let projection = ProjectionOnly(&compact.store);
    storage::validate(&projection, &package.schema).unwrap();
    let mut result = StoredEvaluation::default();
    apply_stored_semantics(&package, &projection, &mut result).unwrap();
    assert!(result.diagnostics.is_empty());
    assert_eq!(
        serde_json::to_value(&result.derived).unwrap(),
        expected_derived
    );

    // A second loan is structurally valid but violates the language rule. Both
    // positive and negative semantic paths must stay on the projection API.
    storage::atomic(&mut compact.store, |store| {
        store.create_element(&package.schema, "loans#2", "Loan", "loans", 0, 1)?;
        for (field, value) in [
            ("name", json!("second")),
            ("copy", json!("catalog#1")),
            ("member", json!("catalog#3")),
        ] {
            store.set_value("loans#2", field, Some(value))?;
        }
        store.append_value("loans#0", "loans", json!("loans#2"))
    })
    .unwrap();
    let projection = ProjectionOnly(&compact.store);
    storage::validate(&projection, &package.schema).unwrap();
    let mut rejected = StoredEvaluation::default();
    apply_stored_semantics(&package, &projection, &mut rejected).unwrap();
    assert_eq!(
        serde_json::to_value(&rejected.derived).unwrap(),
        expected_derived
    );
    assert_eq!(rejected.diagnostics.len(), 2);
    assert!(rejected.diagnostics.iter().all(|d| d.source == "loans"
        && d.message == "A book copy cannot have more than one active loan."));

    storage::atomic(&mut compact.store, |store| {
        store.set_value("catalog#2", "edition", Some(json!("invalid")))
    })
    .unwrap();
    let invalid = storage::validate(&ProjectionOnly(&compact.store), &package.schema).unwrap_err();
    assert!(
        invalid.contains("Invalid EInt value for BookCopy.edition"),
        "{invalid}"
    );
}
}

control_test! {
pub fn ordered_reference_slots_resolve_independently_across_resources() {
    let ecore = include_str!("../../../../fixtures/library/library.ecore").replace(
        "name=\"copy\" lowerBound=\"1\"",
        "name=\"copy\" lowerBound=\"1\" upperBound=\"-1\"",
    );
    let grammar = include_str!("../../../../fixtures/library/library.xtext").replace(
        "copy=[lib::BookCopy|ID]",
        "copy+=[lib::BookCopy|ID] (',' copy+=[lib::BookCopy|ID])*",
    );
    let behavior = include_str!("../../../../fixtures/library/library-behavior.rcore").replace(
        "size(incoming::<m.`Loan`>(self.`copy`, \"copy\")) <= 1",
        "forAll(self.copy, |copy:m.BookCopy| size(incoming::<m.Loan>(copy, \"copy\")) <= 1)",
    );
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &ecore, &grammar, &behavior,
    )
    .unwrap();
    let sources = vec![
        source(
            "loan",
            "library Loans { loan both copy second,first to reader; }",
        ),
        source(
            "catalog",
            "library Catalog { copy first \"First\"; copy second \"Second\"; member reader; }",
        ),
    ];
    let result = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_eq!(
        result.store.value("loan#1", "copy").unwrap(),
        Some(serde_json::json!(["catalog#2", "catalog#1"])),
    );
    assert_eq!(result.evaluation.references.len(), 3);
    // The collection rule retains the same per-copy loan limit.
    let mut conflicting = sources.clone();
    conflicting[0].text = "library Loans { loan both copy second,first to reader; loan duplicate copy first to reader; }".into();
    let invalid = evaluate_stored(&package, &conflicting, EvaluationLimits::native()).unwrap();
    assert_eq!(invalid.evaluation.diagnostics.len(), 2);
    assert!(invalid.evaluation.diagnostics.iter().all(
        |diagnostic| diagnostic.message == "A book copy cannot have more than one active loan."
    ));
    let nonunique = ecore.replace("name=\"copy\"", "name=\"copy\" unique=\"false\"");
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            &nonunique, &grammar, &behavior,
        )
        .unwrap_err()
        .iter()
        .any(|diagnostic| diagnostic.message.contains("requires a unique reference"))
    );
    // An arbitrary target expression must not be translated into this bounded rule.
    let mismatched = behavior.replace(
        "incoming::<m.Loan>(copy,",
        "incoming::<m.Loan>(if true { copy } else { copy },",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            &ecore,
            &grammar,
            &mismatched,
        )
        .unwrap_err()
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Validation must count"))
    );
}
}

control_test! {
pub fn resource_failure_rolls_back_partial_objects_but_retains_other_editor_resources() {
    let package = package();
    let sources = vec![
        source("bad", "library Bad { member partial; member broken }"),
        source("good", "library Good { member retained; }"),
    ];
    let compact = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
    assert_eq!(compact.evaluation.diagnostics.len(), 1);
    assert!(
        compact
            .store
            .query(Query::Resource("bad"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        compact.store.query(Query::Resource("good")).unwrap(),
        vec!["good#0", "good#1"]
    );

    let mut store = compact.store;
    let before = storage::export(&store).unwrap();
    let rejected =
        evaluate_into(&package, &sources, EvaluationLimits::native(), &mut store).unwrap();
    assert!(!rejected.diagnostics.is_empty());
    assert_eq!(storage::export(&store).unwrap(), before);
}
}

struct FailingStore {
    inner: Store,
    fail_members: bool,
}
impl ModelRead for FailingStore {
    fn resource_records(
        &self,
    ) -> Result<
        std::collections::BTreeMap<String, intersect_model::core::resources::ResourceRecord>,
        String,
    > {
        self.inner.resource_records()
    }
    fn resource_record(
        &self,
        uri: &str,
    ) -> Result<Option<intersect_model::core::resources::ResourceRecord>, String> {
        self.inner.resource_record(uri)
    }

    fn revision(&self) -> StorageResult<String> {
        self.inner.revision()
    }
    fn get(&self, id: &str) -> StorageResult<Option<Element>> {
        self.inner.get(id)
    }
    fn class(&self, id: &str) -> StorageResult<Option<String>> {
        self.inner.class(id)
    }
    fn page(&self, after: Option<u64>, limit: usize) -> StorageResult<Vec<Row>> {
        self.inner.page(after, limit)
    }
    fn query(&self, query: Query<'_>) -> StorageResult<Vec<String>> {
        self.inner.query(query)
    }
    fn matches(&self, id: &str, query: Query<'_>) -> StorageResult<bool> {
        self.inner.matches(id, query)
    }
}
impl ModelStore for FailingStore {
    fn set_resource_record(
        &mut self,
        record: intersect_model::core::resources::ResourceRecord,
    ) -> Result<(), String> {
        self.inner.set_resource_record(record)
    }
    fn remove_resource_record(&mut self, uri: &str) -> Result<(), String> {
        self.inner.remove_resource_record(uri)
    }

    fn is_transaction_active(&self) -> bool {
        self.inner.is_transaction_active()
    }
    fn begin(&mut self) -> StorageResult<Checkpoint> {
        self.inner.begin()
    }
    fn commit(&mut self, checkpoint: &Checkpoint) -> StorageResult<()> {
        self.inner.commit(checkpoint)
    }
    fn rollback(&mut self, checkpoint: &Checkpoint) -> StorageResult<()> {
        self.inner.rollback(checkpoint)
    }
    fn set_revision(&mut self, revision: &str) -> StorageResult<()> {
        self.inner.set_revision(revision)
    }
    fn configure_schema(&mut self, schema: &Schema) -> StorageResult<()> {
        self.inner.configure_schema(schema)
    }
    fn put(&mut self, element: Element) -> StorageResult<()> {
        if self.fail_members && element.class == "Member" {
            return Err("Injected storage write failure".into());
        }
        self.inner.put(element)
    }
    fn remove(&mut self, id: &str) -> StorageResult<()> {
        self.inner.remove(id)
    }
}

store_test! {
pub fn storage_failure_inside_optional_alternative_is_not_a_grammar_miss() {
    let package = package();
    let mut store = FailingStore {
        inner: crate::stores::store::<Store>(&package.schema).unwrap(),
        fail_members: false,
    };
    evaluate_into(
        &package,
        &[source("old", "library Existing {}")],
        EvaluationLimits::native(),
        &mut store,
    )
    .unwrap();
    let before = storage::export(&store).unwrap();
    store.fail_members = true;
    let error = evaluate_into(
        &package,
        &[source("new", "library New { member entry; }")],
        EvaluationLimits::native(),
        &mut store,
    )
    .unwrap_err();
    assert!(error.contains("Injected storage write failure"), "{error}");
    assert_eq!(storage::export(&store).unwrap(), before);
}
}
