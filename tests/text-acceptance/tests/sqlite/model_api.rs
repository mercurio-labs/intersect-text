#![cfg(not(target_arch = "wasm32"))]

use intersect_model::core::{
    api::{BasicModel, ModelEdit, TransactionalModel},
    frozen::FrozenStore,
    model::Model,
    storage::{self, ModelRead, ModelStore, Query, StorageResult},
};
use intersect_model::definitions::schema::Schema;
use intersect_model::sqlite::SqliteStore;
use serde_json::json;

fn schema() -> Schema {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .expect("Library fixture")
    .schema
}

fn add_named(model: &mut impl ModelEdit, id: &str, class: &str) -> StorageResult<()> {
    model.create(id, class, "library.model")?;
    model.set(id, "name", json!(id))
}

fn exercise(schema: &Schema, store: impl ModelStore) -> Model {
    let mut basic = BasicModel::from_store(schema.clone(), store).unwrap();
    add_named(&mut basic, "library", "Library").unwrap();
    add_named(&mut basic, "branch", "Library").unwrap();
    add_named(&mut basic, "copy", "BookCopy").unwrap();
    basic.set("library", "copies", json!(["copy"])).unwrap();
    basic.validate().unwrap();
    let initial = storage::export(&basic).unwrap();
    assert!(
        basic
            .set_many(&[
                ("branch", "copies", json!(["copy"])),
                ("copy", "edition", json!("invalid")),
            ])
            .is_err()
    );
    assert_eq!(storage::export(&basic).unwrap(), initial);
    assert_eq!(
        basic.query(Query::Value("copies", &json!("copy"))).unwrap(),
        vec!["library"]
    );

    let mut managed = TransactionalModel::new(basic);
    let abort: StorageResult<()> = managed.transaction(|edit| {
        add_named(edit, "temporary", "Member")?;
        edit.set("copy", "title", json!("Discarded"))?;
        Err("cancelled".into())
    });
    assert_eq!(abort.unwrap_err(), "cancelled");
    assert_eq!(storage::export(&managed).unwrap(), initial);
    assert!(
        managed
            .transaction(|edit| edit.create("incomplete", "Loan", "loans.model"))
            .is_err()
    );
    assert_eq!(storage::export(&managed).unwrap(), initial);
    assert!(
        managed
            .transaction_with(
                |edit| edit.set("copy", "edition", json!(2)),
                |_| Err("domain rejects edition".into()),
            )
            .is_err()
    );
    assert_eq!(storage::export(&managed).unwrap(), initial);

    managed
        .transaction(|edit| {
            add_named(edit, "member", "Member")?;
            assert!(edit.set("copy", "edition", json!("invalid")).is_err());
            edit.set_many(&[
                ("branch", "copies", json!(["copy"])),
                ("copy", "edition", json!(3)),
            ])?;
            assert_eq!(edit.value("copy", "library")?, Some(json!("branch")));
            assert_eq!(
                edit.collection_page("branch", "copies", 0, 1)?,
                vec![json!("copy")]
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(
        managed
            .query(Query::Value("library", &json!("branch")))
            .unwrap(),
        vec!["copy"]
    );
    assert_eq!(
        managed.query(Query::Value("edition", &json!(3))).unwrap(),
        vec!["copy"]
    );
    assert_eq!(managed.value_len("branch", "copies").unwrap(), Some(1));
    assert!(
        managed
            .inspect("copy", &mut |view| {
                assert_eq!(view.class(), "BookCopy");
                assert_eq!(view.value("edition").and_then(|v| v.as_i64()), Some(3));
                Ok(())
            })
            .unwrap()
    );
    let basic = managed.into_basic();
    basic.validate().unwrap();
    let final_model = storage::export(&basic).unwrap();
    let store = basic.into_store();
    assert!(!store.is_transaction_active());
    assert_eq!(storage::export(&store).unwrap(), final_model);
    final_model
}

#[test]
fn basic_and_transactional_facades_have_frozen_and_sqlite_parity() {
    let schema = schema();
    let frozen = exercise(&schema, FrozenStore::for_schema(&schema).unwrap());
    let sqlite = exercise(&schema, SqliteStore::in_memory().unwrap());
    assert_eq!(semantic_model(sqlite), semantic_model(frozen));
}

#[test]
fn external_writes_still_trigger_full_validation_and_atomic_rejection() {
    struct TemporaryDatabase(std::path::PathBuf);
    impl Drop for TemporaryDatabase {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    let path = std::env::temp_dir().join(format!(
        "intersect-api-external-write-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let reserved = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    // Declared before the connections so cleanup also runs after they close
    // when an assertion unwinds on Windows. Only own a newly reserved file.
    let database = TemporaryDatabase(path);
    drop(reserved);
    let store = SqliteStore::open(&database.0).unwrap();
    assert!(!store.supports_incremental_validation());
    let mut model = BasicModel::from_store(schema(), store).unwrap();
    add_named(&mut model, "edited", "BookCopy").unwrap();
    add_named(&mut model, "unrelated", "BookCopy").unwrap();
    model.validate().unwrap();

    let mut external = SqliteStore::open(&database.0).unwrap();
    assert!(!external.supports_incremental_validation());
    storage::atomic(&mut external, |store| {
        store.set_value(
            "unrelated",
            "edition",
            Some(json!("invalid external value")),
        )
    })
    .unwrap();
    let corrupted = storage::export(&model).unwrap();
    let expected = storage::validate_draft(&external, model.schema()).unwrap_err();

    // This scalar edit would otherwise qualify for local validation. Its own
    // write must roll back when the unrelated external corruption is detected.
    assert_eq!(
        model.set("edited", "title", json!("rejected")).unwrap_err(),
        expected
    );
    assert_eq!(storage::export(&model).unwrap(), corrupted);
    assert_eq!(model.set_many(&[]).unwrap_err(), expected);
    assert_eq!(storage::export(&model).unwrap(), corrupted);

    storage::atomic(&mut external, |store| {
        store.set_value("unrelated", "edition", Some(json!(1)))
    })
    .unwrap();
    model.set("edited", "title", json!("accepted")).unwrap();
    model.validate().unwrap();
    assert_eq!(
        external.value("edited", "title").unwrap(),
        Some(json!("accepted"))
    );
}

// Resource content revisions are adapter-local deterministic tokens, not a
// portable edit counter. Preserve every other field in semantic comparisons.
fn semantic_model(
    mut model: intersect_model::core::model::Model,
) -> intersect_model::core::model::Model {
    for record in model.resources.values_mut() {
        record.content_revision = 0;
    }
    model
}
