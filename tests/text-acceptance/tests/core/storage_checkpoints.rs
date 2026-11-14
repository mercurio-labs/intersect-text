use super::Store;
use intersect_model::core::{
    model::Element,
    storage::{self, Checkpoint, ModelRead, ModelStore, Query, Row, StorageResult},
};

store_test! {
pub fn checkpoints_reject_foreign_completed_and_non_lifo_tokens_without_mutation() {
    let mut first = crate::stores::store::<Store>(&crate::stores::empty_schema().unwrap()).unwrap();
    let mut second = crate::stores::store::<Store>(&crate::stores::empty_schema().unwrap()).unwrap();
    let first_token = first.begin().unwrap();
    let second_token = second.begin().unwrap();
    first.set_revision("first").unwrap();
    second.set_revision("second").unwrap();
    assert_ne!(first_token, second_token);
    assert!(first.commit(&second_token).is_err());
    assert!(first.rollback(&second_token).is_err());
    assert!(second.commit(&first_token).is_err());
    assert!(second.rollback(&first_token).is_err());
    assert_eq!(first.revision().unwrap(), "first");
    assert_eq!(second.revision().unwrap(), "second");

    let child = first.begin().unwrap();
    first.set_revision("child").unwrap();
    assert!(first.commit(&first_token).is_err());
    assert!(first.rollback(&first_token).is_err());
    assert_eq!(first.revision().unwrap(), "child");
    first.commit(&child).unwrap();
    assert!(first.commit(&child).is_err());
    assert!(first.rollback(&child.clone()).is_err());
    first.rollback(&first_token).unwrap();
    assert!(first.commit(&first_token).is_err());
    assert!(first.rollback(&first_token).is_err());
    assert_eq!(first.revision().unwrap(), "");
    assert!(!first.is_transaction_active());
    second.rollback(&second_token).unwrap();
    assert_eq!(second.revision().unwrap(), "");
}
}

struct FailingCompletion {
    inner: Store,
    last_checkpoint: Option<Checkpoint>,
    fail_rollback: bool,
    commits: usize,
    rollbacks: usize,
}

impl Default for FailingCompletion {
    fn default() -> Self {
        Self {
            inner: crate::stores::store::<Store>(&crate::stores::empty_schema().unwrap()).unwrap(),
            last_checkpoint: None,
            fail_rollback: false,
            commits: 0,
            rollbacks: 0,
        }
    }
}

impl ModelRead for FailingCompletion {
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

impl ModelStore for FailingCompletion {
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
        let checkpoint = self.inner.begin()?;
        self.last_checkpoint = Some(checkpoint.clone());
        Ok(checkpoint)
    }
    fn commit(&mut self, _checkpoint: &Checkpoint) -> StorageResult<()> {
        self.commits += 1;
        Err("injected commit failure".into())
    }
    fn rollback(&mut self, checkpoint: &Checkpoint) -> StorageResult<()> {
        self.rollbacks += 1;
        if self.fail_rollback {
            Err("injected rollback failure".into())
        } else {
            self.inner.rollback(checkpoint)
        }
    }
    fn set_revision(&mut self, revision: &str) -> StorageResult<()> {
        self.inner.set_revision(revision)
    }
    fn put(&mut self, element: Element) -> StorageResult<()> {
        self.inner.put(element)
    }
    fn remove(&mut self, id: &str) -> StorageResult<()> {
        self.inner.remove(id)
    }
}

store_test! {
pub fn atomic_reports_recovery_failures_after_callback_or_commit_errors() {
    for callback_fails in [false, true] {
        for rollback_fails in [false, true] {
            let mut store = FailingCompletion {
                fail_rollback: rollback_fails,
                ..FailingCompletion::default()
            };
            let result = storage::atomic(&mut store, |store| {
                store.set_revision("uncommitted")?;
                if callback_fails {
                    Err("injected callback failure".into())
                } else {
                    Ok(())
                }
            });
            let original = if callback_fails {
                "injected callback failure"
            } else {
                "injected commit failure"
            };
            let expected = if rollback_fails {
                format!("{original}; rollback failed: injected rollback failure")
            } else {
                original.into()
            };
            assert_eq!(result.unwrap_err(), expected);
            assert_eq!(store.commits, usize::from(!callback_fails));
            assert_eq!(store.rollbacks, 1);
            assert_eq!(store.is_transaction_active(), rollback_fails);
            if rollback_fails {
                assert_eq!(store.revision().unwrap(), "uncommitted");
                let checkpoint = store.last_checkpoint.clone().unwrap();
                store.fail_rollback = false;
                store.rollback(&checkpoint).unwrap();
            }
            assert_eq!(store.revision().unwrap(), "");
            assert!(!store.is_transaction_active());
        }
    }
}
}
