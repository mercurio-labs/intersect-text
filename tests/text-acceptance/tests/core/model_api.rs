use intersect_model::core::{
    api::{BasicModel, ModelEdit, TransactionalModel},
    frozen::FrozenStore,
    model::Element,
    storage::{self, ModelRead, ModelStore, ObjectHandle, Query, ReferenceView, StorageResult},
};
use intersect_model::definitions::schema::Schema;
use serde_json::{Value, json};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn schema() -> Schema {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .expect("Library fixture")
    .schema
}

// An external domain can supply this typed convenience API without depending on
// a concrete store or selecting whether its caller uses a transaction.
fn add_named<M: ModelEdit + ?Sized>(
    model: &mut M,
    id: &str,
    class: &str,
    name: &str,
) -> StorageResult<()> {
    assert!(model.schema().conforms(class, "Named"));
    model.create(id, class, "library.model")?;
    model.set(id, "name", json!(name))
}

fn library() -> BasicModel {
    populated_library(BasicModel::new(schema()).expect("basic model"))
}

fn populated_library<S: ModelStore>(mut model: BasicModel<S>) -> BasicModel<S> {
    add_named(&mut model, "library", "Library", "Central").unwrap();
    add_named(&mut model, "branch", "Library", "Branch").unwrap();
    add_named(&mut model, "copy", "BookCopy", "Dune").unwrap();
    model.set("library", "copies", json!(["copy"])).unwrap();
    model.validate().unwrap();
    model
}

fn reference_handle(model: &(impl ModelRead + ?Sized)) -> ObjectHandle {
    let mut handle = None;
    assert!(
        model
            .inspect("copy", &mut |view| {
                assert!(
                    view.type_id().is_some(),
                    "canonical descriptor survives facade"
                );
                if let Some(ReferenceView::Resolved(target)) =
                    view.value("library").and_then(|v| v.reference())
                {
                    handle = Some(target);
                }
                Ok(())
            })
            .unwrap()
    );
    handle.expect("canonical resolved reference")
}

#[test]
fn domain_helper_works_in_basic_and_transactional_contexts() {
    let mut basic: BasicModel = BasicModel::new(schema()).unwrap();
    add_named(&mut basic, "basic", "Member", "Basic member").unwrap();
    basic.validate().unwrap();

    let mut managed: TransactionalModel = TransactionalModel::new(basic);
    let result = managed
        .transaction(|edit| {
            add_named(edit, "transactional", "Member", "Transactional member")?;
            assert_eq!(edit.class("transactional")?, Some("Member".into()));
            Ok("committed")
        })
        .unwrap();
    assert_eq!(result, "committed");
    assert_eq!(
        managed.query(Query::Class("Member")).unwrap(),
        vec!["basic", "transactional"]
    );
    let basic = managed.into_basic();
    basic.validate().unwrap();
    let store: FrozenStore = basic.into_store();
    assert!(!store.is_transaction_active());
}

#[test]
fn draft_construction_defers_only_missing_required_values() {
    let mut basic = BasicModel::new(schema()).unwrap();
    basic.create("copy", "BookCopy", "library.model").unwrap();
    assert_eq!(basic.value("copy", "edition").unwrap(), Some(json!(1)));
    add_named(&mut basic, "draft", "Loan", "Draft loan").unwrap();
    assert!(
        basic.validate().is_err(),
        "loan member and copy are required"
    );
    assert!(basic.set("copy", "edition", json!("invalid")).is_err());
    assert_eq!(basic.value("copy", "edition").unwrap(), Some(json!(1)));
    basic.set("copy", "name", json!("Dune")).unwrap();
    add_named(&mut basic, "reader", "Member", "Reader").unwrap();
    basic.set("draft", "member", json!("reader")).unwrap();
    basic.set("draft", "copy", json!("copy")).unwrap();
    basic.validate().unwrap();

    let mut managed = TransactionalModel::new(basic);
    let before = storage::export(&managed).unwrap();
    assert!(
        managed
            .transaction(|edit| edit.create("incomplete", "Loan", "loans.model"))
            .is_err()
    );
    assert_eq!(storage::export(&managed).unwrap(), before);
    managed
        .transaction(|edit| {
            add_named(edit, "member", "Member", "Alice")?;
            add_named(edit, "loan", "Loan", "Loan 1")?;
            edit.set("loan", "member", json!("member"))?;
            edit.set("loan", "copy", json!("copy"))
        })
        .unwrap();
    managed.into_basic().validate().unwrap();
}

#[test]
fn invalid_edits_restore_objects_opposites_and_indexes() {
    let mut model = library();
    let before = storage::export(&model).unwrap();
    for (id, feature, value) in [
        ("copy", "edition", json!("wrong type")),
        ("copy", "edition", json!([1, 2])),
        ("copy", "available", json!(true)),
        ("copy", "unknown", json!(true)),
        ("library", "copies", json!(["copy", "copy"])),
        ("library", "members", json!(["copy"])),
        ("library", "members", json!(["missing"])),
        // Opposite maintenance mutates copy.library before reaching missing.
        ("library", "copies", json!(["copy", "missing"])),
        // Displace the old owner, then fail on the next target.
        ("branch", "copies", json!(["copy", "missing"])),
    ] {
        assert!(model.set(id, feature, value).is_err(), "{id}.{feature}");
        assert_eq!(storage::export(&model).unwrap(), before);
        assert_eq!(
            model
                .query(Query::Value("library", &json!("library")))
                .unwrap(),
            vec!["copy"]
        );
        assert!(
            model
                .query(Query::Value("library", &json!("branch")))
                .unwrap()
                .is_empty()
        );
    }
    assert!(model.create("copy", "BookCopy", "other.model").is_err());
    assert!(model.create("abstract", "Named", "other.model").is_err());
    assert!(model.create("unknown", "Unknown", "other.model").is_err());
    assert!(model.create("", "Member", "other.model").is_err());
    assert_eq!(storage::export(&model).unwrap(), before);
}

#[test]
fn relationship_batches_commit_together_or_restore_both_sides() {
    let mut model = library();
    let before = storage::export(&model).unwrap();
    assert!(
        model
            .set_many(&[
                ("branch", "copies", json!(["copy"])),
                ("copy", "edition", json!(false)),
            ])
            .is_err()
    );
    assert_eq!(storage::export(&model).unwrap(), before);
    model
        .set_many(&[("branch", "copies", json!(["copy"]))])
        .unwrap();
    assert_eq!(model.value("library", "copies").unwrap(), Some(json!([])));
    assert_eq!(
        model.value("copy", "library").unwrap(),
        Some(json!("branch"))
    );
    assert_eq!(
        model.query(Query::Value("copies", &json!("copy"))).unwrap(),
        vec!["branch"]
    );
    model.validate().unwrap();
}

#[test]
fn operation_error_can_be_caught_without_leaking_its_changes() {
    let mut managed = TransactionalModel::new(library());
    managed
        .transaction(|edit| {
            edit.set("copy", "title", json!("First edition"))?;
            assert!(
                edit.set_many(&[
                    ("branch", "copies", json!(["copy"])),
                    ("copy", "edition", json!(false)),
                ])
                .is_err()
            );
            assert_eq!(edit.value("copy", "library")?, Some(json!("library")));
            edit.set("copy", "edition", json!(2))
        })
        .unwrap();
    assert_eq!(
        managed.value("copy", "title").unwrap(),
        Some(json!("First edition"))
    );
    assert_eq!(managed.value("copy", "edition").unwrap(), Some(json!(2)));
    assert_eq!(
        managed.value("copy", "library").unwrap(),
        Some(json!("library"))
    );
}

#[test]
fn callback_and_domain_errors_rollback_the_whole_transaction() {
    let mut managed = TransactionalModel::new(library());
    let before = storage::export(&managed).unwrap();
    let callback: StorageResult<()> = managed.transaction(|edit| {
        add_named(edit, "temporary", "Member", "Temporary")?;
        edit.set("copy", "title", json!("Uncommitted"))?;
        Err("cancelled domain operation".into())
    });
    assert_eq!(callback.unwrap_err(), "cancelled domain operation");
    assert_eq!(storage::export(&managed).unwrap(), before);

    let domain = managed.transaction_with(
        |edit| edit.set("copy", "edition", json!(2)),
        |candidate| {
            assert_eq!(candidate.value("copy", "edition")?, Some(json!(2)));
            Err("library policy rejects this edition".into())
        },
    );
    assert_eq!(domain.unwrap_err(), "library policy rejects this edition");
    assert_eq!(storage::export(&managed).unwrap(), before);
    assert!(
        managed
            .query(Query::Value("edition", &json!(2)))
            .unwrap()
            .is_empty()
    );
    managed
        .transaction_with(
            |edit| edit.set("copy", "edition", json!(3)),
            |candidate| {
                assert_eq!(candidate.value("copy", "edition")?, Some(json!(3)));
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(managed.value("copy", "edition").unwrap(), Some(json!(3)));
}

#[test]
fn callback_unwind_restores_model_and_allows_a_later_transaction() {
    let mut managed = TransactionalModel::new(library());
    let before = storage::export(&managed).unwrap();
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _: StorageResult<()> = managed.transaction(|edit| {
            edit.set("copy", "title", json!("Uncommitted"))?;
            add_named(edit, "temporary", "Member", "Temporary")?;
            panic!("injected application panic");
        });
    }));
    assert!(panic.is_err());
    assert_eq!(storage::export(&managed).unwrap(), before);
    managed
        .transaction(|edit| edit.set("copy", "title", json!("Recovered")))
        .unwrap();
    assert!(!managed.into_basic().into_store().is_transaction_active());
}

#[test]
fn read_facades_preserve_canonical_context_and_portable_projections() {
    let first = library();
    let second = library();
    let first_handle = reference_handle(&first);
    assert_ne!(first_handle, reference_handle(&second));
    assert_eq!(first.value_len("library", "copies").unwrap(), Some(1));
    assert_eq!(
        first.collection_page("library", "copies", 0, 1).unwrap(),
        vec![json!("copy")]
    );
    assert_eq!(first.id_page(None, 1).unwrap().len(), 1);
    assert!(first.matches("copy", Query::Class("BookCopy")).unwrap());

    let mut managed = TransactionalModel::new(first);
    assert_eq!(reference_handle(&managed), first_handle);
    managed
        .transaction(|edit| {
            assert_eq!(reference_handle(edit), first_handle);
            assert_eq!(
                edit.collection_page("library", "copies", 0, 1)?,
                vec![json!("copy")]
            );
            assert_eq!(edit.value_len("library", "copies")?, Some(1));
            Ok(())
        })
        .unwrap();
    let store = managed.into_basic().into_store();
    store
        .inspect_handle(&first_handle, &mut |view| {
            assert_eq!(view.id(), "library");
            Ok(())
        })
        .unwrap();
}

#[test]
fn wrapping_raw_storage_rejects_open_transactions_invalid_values_and_revisions() {
    let schema = schema();
    let mut active = FrozenStore::for_schema(&schema).unwrap();
    active.begin().unwrap();
    assert!(BasicModel::from_store(schema.clone(), active).is_err());

    let mut invalid = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut invalid, |store| {
        store.configure_schema(&schema)?;
        let mut element = Element::new(&schema, "member", "Member", "raw.model", 0, 0)?;
        element.values.insert("name".into(), json!(12));
        store.put(element)
    })
    .unwrap();
    assert!(BasicModel::from_store(schema.clone(), invalid).is_err());

    let mut foreign = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut foreign, |store| store.set_revision("different-schema")).unwrap();
    assert!(BasicModel::from_store(schema.clone(), foreign).is_err());

    let mut draft = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut draft, |store| {
        store.create_element(&schema, "loan", "Loan", "raw.model", 4, 9)
    })
    .unwrap();
    let mut basic = BasicModel::from_store(schema, draft).unwrap();
    assert!(basic.validate().is_err());
    basic
        .set("loan", "name", Value::String("Completed".into()))
        .unwrap();
    add_named(&mut basic, "member", "Member", "Reader").unwrap();
    add_named(&mut basic, "copy", "BookCopy", "Book").unwrap();
    basic.set("loan", "member", json!("member")).unwrap();
    basic.set("loan", "copy", json!("copy")).unwrap();
    basic.validate().unwrap();
    let element = basic.get("loan").unwrap().unwrap();
    assert_eq!((element.start, element.end), (4, 9));
}
