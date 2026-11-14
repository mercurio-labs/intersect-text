use super::Store;
use intersect_model::core::{
    model::Element,
    storage::{self, Checkpoint, ModelRead, ModelStore, Query, Row, StorageResult},
};
use intersect_model::definitions::schema::Schema;

/// Exercise the trait's default creation/configuration path, as a persistence
/// adapter would, while retaining an independent injected write failure.
struct FailingStore {
    inner: Store,
    fail_next_put: bool,
}

impl FailingStore {
    fn for_schema(schema: &Schema) -> Self {
        Self {
            inner: crate::stores::store::<Store>(schema).unwrap(),
            fail_next_put: false,
        }
    }
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
    fn put(&mut self, element: Element) -> StorageResult<()> {
        if std::mem::take(&mut self.fail_next_put) {
            return Err("Injected persistence failure".into());
        }
        self.inner.put(element)
    }
    fn remove(&mut self, id: &str) -> StorageResult<()> {
        self.inner.remove(id)
    }
}

fn schema() -> Schema {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .expect("Library fixture")
    .schema
}

fn member(schema: &Schema, id: &str) -> Element {
    Element::new(schema, id, "Member", "resource", 0, 1).expect("valid member")
}

store_test! {
pub fn default_creation_and_shared_insert_require_an_existing_transaction() {
    let schema = schema();
    let mut store = FailingStore::for_schema(&schema);
    let baseline = storage::export(&store).expect("empty model");
    assert!(
        store
            .create_element(&schema, "created", "Member", "resource", 0, 1)
            .is_err()
    );
    assert!(storage::insert(&mut store, &schema, member(&schema, "inserted")).is_err());
    assert_eq!(storage::export(&store).expect("unchanged model"), baseline);
    assert!(!store.is_transaction_active());
}
}

store_test! {
pub fn default_creation_and_shared_insert_reject_bad_requests_before_binding() {
    let mut schema = schema();
    schema.classes.get_mut("Loan").expect("Loan").abstract_type = true;
    let mut store = FailingStore::for_schema(&schema);
    storage::atomic(&mut store, |store| store.put(member(&schema, "existing")))
        .expect("seed unbound raw store");
    let baseline = storage::export(&store).expect("unbound model");
    let outer = store.begin().expect("outer transaction");
    for (id, class) in [
        ("", "Member"),
        ("existing", "Member"),
        ("unknown", "Unknown"),
        ("abstract", "Loan"),
    ] {
        assert!(
            store
                .create_element(&schema, id, class, "resource", 0, 1)
                .is_err()
        );
        assert_eq!(storage::export(&store).expect("unchanged model"), baseline);
        assert!(store.is_transaction_active());
    }
    let mut empty = member(&schema, "empty");
    empty.id.clear();
    let mut wrong_type = member(&schema, "wrong-type");
    wrong_type.type_identity = "another-schema#Member".into();
    for element in [empty, member(&schema, "existing"), wrong_type] {
        assert!(storage::insert(&mut store, &schema, element).is_err());
        assert_eq!(storage::export(&store).expect("unchanged model"), baseline);
        assert!(store.is_transaction_active());
    }
    store
        .commit(&outer)
        .expect("handled errors leave outer transaction usable");
    assert_eq!(
        storage::export(&store).expect("unchanged committed model"),
        baseline
    );
}
}

store_test! {
pub fn failed_put_rolls_back_binding_before_a_caller_commits_the_outer_transaction() {
    let first_schema = schema();
    let mut second_schema = first_schema.clone();
    second_schema.revision.push_str("-different-revision");
    for insert in [false, true] {
        let mut store = FailingStore::for_schema(&first_schema);
        storage::atomic(&mut store, |store| {
            store.put(member(&first_schema, "existing"))
        })
        .expect("seed unbound raw store");
        let baseline = storage::export(&store).expect("unbound model");
        let outer = store.begin().expect("outer transaction");
        store.fail_next_put = true;
        let result = if insert {
            storage::insert(&mut store, &first_schema, member(&first_schema, "failed"))
        } else {
            store.create_element(&first_schema, "failed", "Member", "resource", 0, 1)
        };
        assert_eq!(result, Err("Injected persistence failure".into()));
        assert_eq!(storage::export(&store).expect("restored model"), baseline);
        assert!(store.is_transaction_active());
        store
            .commit(&outer)
            .expect("caller handles failure and commits");
        assert_eq!(
            storage::export(&store).expect("unchanged committed model"),
            baseline
        );

        storage::atomic(&mut store, |store| {
            if insert {
                storage::insert(store, &second_schema, member(&second_schema, "survivor"))
            } else {
                store.create_element(&second_schema, "survivor", "Member", "resource", 0, 1)
            }
        })
        .expect("failure did not bind the first schema");
        assert_eq!(
            store.revision().expect("new revision"),
            second_schema.revision
        );
        assert_eq!(
            store.get("existing").expect("original object"),
            Some(member(&first_schema, "existing"))
        );
        assert!(store.get("failed").expect("failed object").is_none());
        assert!(store.get("survivor").expect("successful object").is_some());
    }
}
}
