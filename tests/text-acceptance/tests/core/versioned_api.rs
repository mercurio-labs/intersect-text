//! Handwritten controls for the opt-in versioned model API.
use intersect_model::core::{
    api::{BasicModel, ModelEdit, VersionedModel},
    model::Model,
    storage::{self, ModelRead, ModelStore, ObjectHandle, Query, ReferenceView, StorageResult},
};
use intersect_model::definitions::schema::Schema;
use serde_json::json;
use std::panic::{AssertUnwindSafe, catch_unwind};

// Successful write strategies can issue different resource revision tokens.
// Compare every other field here; rollback and snapshot assertions remain exact.
fn assert_same_model_content(mut actual: Model, expected: &Model) {
    let mut expected = expected.clone();
    for record in actual.resources.values_mut() {
        record.content_revision = 0;
    }
    for record in expected.resources.values_mut() {
        record.content_revision = 0;
    }
    assert_eq!(actual, expected);
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

// The same domain helper accepts basic, transactional, or versioned edit access.
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
    let mut model = BasicModel::new(schema()).unwrap();
    add_named(&mut model, "library", "Library", "Central").unwrap();
    add_named(&mut model, "branch", "Library", "Branch").unwrap();
    add_named(&mut model, "copy", "BookCopy", "Dune").unwrap();
    model.set("library", "copies", json!(["copy"])).unwrap();
    let schema = model.schema().clone();
    let mut store = model.into_store();
    storage::atomic(&mut store, |store| store.set_span("copy", 4, 9)).unwrap();
    let model = BasicModel::from_store(schema, store).unwrap();
    model.validate().unwrap();
    model
}

fn reference_handle(model: &(impl ModelRead + ?Sized)) -> ObjectHandle {
    let mut handle = None;
    assert!(
        model
            .inspect("copy", &mut |view| {
                assert!(view.type_id().is_some());
                if let Some(ReferenceView::Resolved(target)) =
                    view.value("library").and_then(|v| v.reference())
                {
                    handle = Some(target);
                }
                Ok(())
            })
            .unwrap()
    );
    handle.expect("unchanged canonical reference context")
}

#[test]
fn shared_domain_helper_and_pinned_snapshots_keep_siblings_isolated() {
    let basic = library();
    let original_handle = reference_handle(&basic);
    let original_schema = basic.schema().revision.clone();
    let mut model = VersionedModel::new(basic).unwrap();
    let baseline = model.snapshot().unwrap();
    let baseline_version = baseline.version();
    let mut first = model.fork(&baseline).unwrap();
    let mut sibling = model.fork(&baseline).unwrap();
    assert_eq!(first.base_version(), baseline_version);
    assert_eq!(sibling.base_version(), baseline_version);
    assert_eq!(reference_handle(&baseline), original_handle);

    add_named(&mut first, "reader", "Member", "Alice").unwrap();
    first.set("copy", "edition", json!(2)).unwrap();
    sibling.set("copy", "edition", json!(3)).unwrap();
    assert_eq!(first.value("copy", "edition").unwrap(), Some(json!(2)));
    assert_eq!(sibling.value("copy", "edition").unwrap(), Some(json!(3)));
    assert_eq!(baseline.value("copy", "edition").unwrap(), Some(json!(1)));
    assert_eq!(sibling.class("reader").unwrap(), None);
    assert_eq!(baseline.class("reader").unwrap(), None);
    assert_eq!(reference_handle(&first), original_handle);
    first.validate().unwrap();

    let published = model.commit(first).unwrap();
    let current = model.snapshot().unwrap();
    assert_eq!(current.version(), published);
    assert_ne!(current.version(), baseline_version);
    assert_eq!(current.class("reader").unwrap(), Some("Member".into()));
    assert_eq!(current.value("copy", "edition").unwrap(), Some(json!(2)));
    assert_eq!(baseline.value("copy", "edition").unwrap(), Some(json!(1)));
    assert_eq!(sibling.value("copy", "edition").unwrap(), Some(json!(3)));
    assert_eq!(reference_handle(&current), original_handle);
    assert_eq!(baseline.revision().unwrap(), original_schema);
    assert_eq!(current.revision().unwrap(), original_schema);
    assert_eq!(sibling.revision().unwrap(), original_schema);

    let original = baseline.get("copy").unwrap().unwrap();
    let changed = current.get("copy").unwrap().unwrap();
    assert_eq!(changed.id, original.id);
    assert_eq!(changed.type_identity, original.type_identity);
    assert_eq!(changed.resource, original.resource);
    assert_eq!((changed.start, changed.end), (4, 9));
    assert!(model.commit(sibling).is_err());
    assert_eq!(model.snapshot().unwrap().version(), published);
}

#[test]
fn old_snapshots_can_be_explored_but_cannot_replace_a_newer_head() {
    let mut model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    let mut update = model.fork(&baseline).unwrap();
    update.set("copy", "edition", json!(2)).unwrap();
    model.commit(update).unwrap();
    let head = model.snapshot().unwrap();
    let mut historical = model.fork(&baseline).unwrap();
    assert_eq!(historical.base_version(), baseline.version());
    assert_eq!(historical.value("copy", "edition").unwrap(), Some(json!(1)));
    historical.set("copy", "edition", json!(9)).unwrap();
    let mut called = false;
    assert!(
        model
            .commit_with(historical, |_| {
                called = true;
                Ok(())
            })
            .is_err()
    );
    assert!(
        !called,
        "stale forks must be rejected before domain execution"
    );
    assert_eq!(model.snapshot().unwrap().version(), head.version());
    assert_eq!(
        storage::export(&model.snapshot().unwrap()).unwrap(),
        storage::export(&head).unwrap()
    );
}

#[test]
fn instance_scopes_reject_foreign_snapshots_and_foreign_commits() {
    let mut first = VersionedModel::new(library()).unwrap();
    let second = VersionedModel::new(library()).unwrap();
    let first_snapshot = first.snapshot().unwrap();
    let second_snapshot = second.snapshot().unwrap();
    assert_eq!(
        first_snapshot.revision().unwrap(),
        second_snapshot.revision().unwrap()
    );
    assert_ne!(first_snapshot.version(), second_snapshot.version());
    assert!(first.fork(&second_snapshot).is_err());
    assert!(second.fork(&first_snapshot).is_err());
    let mut foreign = second.fork(&second_snapshot).unwrap();
    foreign.set("copy", "edition", json!(2)).unwrap();
    let mut called = false;
    assert!(
        first
            .commit_with(foreign, |_| {
                called = true;
                Ok(())
            })
            .is_err()
    );
    assert!(
        !called,
        "foreign forks must be rejected before domain execution"
    );
    assert_eq!(
        first.snapshot().unwrap().version(),
        first_snapshot.version()
    );
    assert_eq!(
        storage::export(&first.snapshot().unwrap()).unwrap(),
        storage::export(&first_snapshot).unwrap()
    );
}

#[test]
fn rejected_operations_restore_existing_private_edits_and_opposites() {
    let model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    let mut candidate = model.fork(&baseline).unwrap();
    candidate
        .set("copy", "title", json!("Private draft"))
        .unwrap();
    let before = storage::export(&candidate).unwrap();
    assert!(
        candidate
            .set_many(&[
                ("branch", "copies", json!(["copy"])),
                ("copy", "edition", json!(false)),
            ])
            .is_err()
    );
    assert_eq!(storage::export(&candidate).unwrap(), before);
    for (id, feature, value) in [
        ("copy", "edition", json!("wrong type")),
        ("copy", "edition", json!([1, 2])),
        ("copy", "available", json!(true)),
        ("copy", "unknown", json!(true)),
        ("library", "copies", json!(["copy", "copy"])),
        ("library", "members", json!(["copy"])),
        ("library", "members", json!(["missing"])),
        ("library", "copies", json!(["copy", "missing"])),
        ("branch", "copies", json!(["copy", "missing"])),
    ] {
        assert!(candidate.set(id, feature, value).is_err(), "{id}.{feature}");
        assert_eq!(storage::export(&candidate).unwrap(), before);
    }
    for (id, class) in [
        ("copy", "BookCopy"),
        ("abstract", "Named"),
        ("unknown", "Unknown"),
        ("", "Member"),
    ] {
        assert!(candidate.create(id, class, "other.model").is_err());
        assert_eq!(storage::export(&candidate).unwrap(), before);
    }
    assert_eq!(
        candidate
            .query(Query::Value("library", &json!("library")))
            .unwrap(),
        vec!["copy"]
    );
    assert!(
        candidate
            .query(Query::Value("library", &json!("branch")))
            .unwrap()
            .is_empty()
    );
    candidate.set("copy", "edition", json!(2)).unwrap();
    candidate.validate().unwrap();
    assert_eq!(baseline.value("copy", "title").unwrap(), Some(json!(null)));
}

#[test]
fn incomplete_drafts_are_editable_and_full_structure_is_required_at_commit() {
    let mut model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    let mut incomplete = model.fork(&baseline).unwrap();
    add_named(&mut incomplete, "loan", "Loan", "Draft loan").unwrap();
    assert!(incomplete.validate().is_err());
    assert!(model.commit(incomplete).is_err());
    assert_eq!(model.snapshot().unwrap().version(), baseline.version());
    assert_eq!(model.snapshot().unwrap().class("loan").unwrap(), None);

    let mut complete = model.fork(&baseline).unwrap();
    add_named(&mut complete, "loan", "Loan", "Completed loan").unwrap();
    add_named(&mut complete, "reader", "Member", "Alice").unwrap();
    complete.set("loan", "member", json!("reader")).unwrap();
    complete.set("loan", "copy", json!("copy")).unwrap();
    complete.validate().unwrap();
    model.commit(complete).unwrap();
    assert_eq!(
        model.snapshot().unwrap().class("loan").unwrap(),
        Some("Loan".into())
    );
    assert_eq!(baseline.class("loan").unwrap(), None);
}

#[test]
fn domain_rejection_and_unwind_leave_the_head_usable() {
    let mut model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    let before = storage::export(&baseline).unwrap();
    let mut rejected = model.fork(&baseline).unwrap();
    rejected.set("copy", "edition", json!(2)).unwrap();
    let error = model.commit_with(rejected, |candidate| {
        assert_eq!(candidate.value("copy", "edition")?, Some(json!(2)));
        Err("library policy rejects this edition".into())
    });
    assert_eq!(error.unwrap_err(), "library policy rejects this edition");
    assert_eq!(model.snapshot().unwrap().version(), baseline.version());
    assert_eq!(storage::export(&model.snapshot().unwrap()).unwrap(), before);

    let mut panicking = model.fork(&baseline).unwrap();
    panicking.set("copy", "edition", json!(3)).unwrap();
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = model.commit_with(panicking, |candidate| {
            assert_eq!(candidate.value("copy", "edition")?, Some(json!(3)));
            panic!("injected application validator panic");
        });
    }));
    assert!(panic.is_err());
    assert_eq!(model.snapshot().unwrap().version(), baseline.version());
    assert_eq!(storage::export(&model.snapshot().unwrap()).unwrap(), before);

    let mut accepted = model.fork(&baseline).unwrap();
    accepted.set("copy", "edition", json!(4)).unwrap();
    model
        .commit_with(accepted, |candidate| {
            assert_eq!(candidate.value("copy", "edition")?, Some(json!(4)));
            Ok(())
        })
        .unwrap();
    assert_eq!(
        model.snapshot().unwrap().value("copy", "edition").unwrap(),
        Some(json!(4))
    );
}

#[test]
fn relationships_queries_and_collection_projections_read_current_overlays() {
    let mut model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    let mut candidate = model.fork(&baseline).unwrap();
    add_named(&mut candidate, "new-copy", "BookCopy", "Foundation").unwrap();
    candidate.set("new-copy", "edition", json!(2)).unwrap();
    candidate.set("copy", "edition", json!(2)).unwrap();
    candidate
        .set_many(&[("branch", "copies", json!(["new-copy", "copy"]))])
        .unwrap();
    assert_eq!(
        candidate.value("copy", "library").unwrap(),
        Some(json!("branch"))
    );
    assert_eq!(
        candidate.value("new-copy", "library").unwrap(),
        Some(json!("branch"))
    );
    assert_eq!(candidate.value_len("branch", "copies").unwrap(), Some(2));
    assert_eq!(
        candidate.collection_page("branch", "copies", 0, 1).unwrap(),
        vec![json!("new-copy")]
    );
    assert_eq!(
        candidate.collection_page("branch", "copies", 1, 9).unwrap(),
        vec![json!("copy")]
    );
    assert!(
        candidate
            .collection_page("branch", "copies", 2, 1)
            .unwrap()
            .is_empty()
    );
    assert!(
        candidate
            .collection_page("branch", "copies", 0, 0)
            .unwrap()
            .is_empty()
    );
    assert_eq!(candidate.value_len("copy", "edition").unwrap(), None);
    assert_eq!(candidate.value_len("missing", "copies").unwrap(), None);
    assert!(candidate.collection_page("copy", "edition", 0, 1).is_err());
    assert!(
        candidate
            .collection_page("missing", "copies", 0, 1)
            .is_err()
    );

    // Results follow object insertion order, independently of collection order.
    assert_eq!(
        candidate.query(Query::Class("BookCopy")).unwrap(),
        vec!["copy", "new-copy"]
    );
    assert_eq!(
        candidate.query(Query::Value("edition", &json!(2))).unwrap(),
        vec!["copy", "new-copy"]
    );
    assert!(
        candidate
            .query(Query::Value("edition", &json!(1)))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        candidate
            .query(Query::Value("library", &json!("branch")))
            .unwrap(),
        vec!["copy", "new-copy"]
    );
    assert!(
        candidate
            .query(Query::Value("library", &json!("library")))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        candidate
            .query(Query::Value("copies", &json!("copy")))
            .unwrap(),
        vec!["branch"]
    );
    assert_eq!(
        candidate.query(Query::Resource("library.model")).unwrap(),
        vec!["library", "branch", "copy", "new-copy"]
    );
    assert!(
        candidate
            .matches("new-copy", Query::Class("BookCopy"))
            .unwrap()
    );
    assert!(
        !candidate
            .matches("missing", Query::Class("BookCopy"))
            .unwrap()
    );
    assert!(
        !candidate
            .matches("copy", Query::Value("edition", &json!(1)))
            .unwrap()
    );

    assert!(
        candidate
            .inspect("branch", &mut |view| {
                let values = view.value("copies").unwrap().collection().unwrap();
                assert_eq!(values.len(), 2);
                assert_eq!(values.get(0).unwrap().as_str(), Some("new-copy"));
                assert_eq!(values.get(1).unwrap().as_str(), Some("copy"));
                assert!(values.get(2).is_none());
                assert_eq!(view.to_element(), candidate.get("branch")?.unwrap());
                Ok(())
            })
            .unwrap()
    );
    let mut called = false;
    assert!(
        !candidate
            .inspect("missing", &mut |_| {
                called = true;
                Ok(())
            })
            .unwrap()
    );
    assert!(!called);
    candidate.validate().unwrap();
    model.commit(candidate).unwrap();
    let published = model.snapshot().unwrap();
    assert_eq!(
        published.collection_page("branch", "copies", 0, 9).unwrap(),
        vec![json!("new-copy"), json!("copy")]
    );
    assert_eq!(
        baseline.value("library", "copies").unwrap(),
        Some(json!(["copy"]))
    );
    assert_eq!(baseline.class("new-copy").unwrap(), None);
}

#[test]
fn cumulative_snapshots_preserve_new_objects_pagination_and_later_reversions() {
    let mut model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    let base_rows = baseline.id_page(None, 100).unwrap();
    let mut first = model.fork(&baseline).unwrap();
    add_named(&mut first, "reader-z", "Member", "First").unwrap();
    add_named(&mut first, "reader-a", "Member", "Second").unwrap();
    first.set("copy", "edition", json!(2)).unwrap();
    model.commit(first).unwrap();
    let middle = model.snapshot().unwrap();
    let middle_rows = middle.id_page(None, 100).unwrap();
    assert_eq!(&middle_rows[..base_rows.len()], base_rows);
    assert_eq!(
        middle_rows
            .iter()
            .map(|(_, id)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["library", "branch", "copy", "reader-z", "reader-a"]
    );
    assert!(middle_rows.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(
        middle.id_page(Some(base_rows[2].0), 1).unwrap(),
        middle_rows[3..4]
    );
    assert_eq!(
        middle.id_page(Some(middle_rows[3].0), 100).unwrap(),
        middle_rows[4..]
    );
    assert!(middle.id_page(None, 0).unwrap().is_empty());
    assert!(middle.id_page(Some(u64::MAX), 1).unwrap().is_empty());
    assert_eq!(
        middle
            .page(None, 100)
            .unwrap()
            .into_iter()
            .map(|(row, element)| (row, element.id))
            .collect::<Vec<_>>(),
        middle_rows
    );

    let mut second = model.fork(&middle).unwrap();
    second.set("reader-z", "name", json!("Updated")).unwrap();
    second.set("copy", "edition", json!(1)).unwrap();
    model.commit(second).unwrap();
    let final_snapshot = model.snapshot().unwrap();
    assert_eq!(final_snapshot.id_page(None, 100).unwrap(), middle_rows);
    assert_eq!(
        final_snapshot.query(Query::Class("Member")).unwrap(),
        vec!["reader-z", "reader-a"]
    );
    assert_eq!(
        final_snapshot.value("reader-z", "name").unwrap(),
        Some(json!("Updated"))
    );
    assert_eq!(
        middle.value("reader-z", "name").unwrap(),
        Some(json!("First"))
    );
    assert_eq!(
        final_snapshot.value("copy", "edition").unwrap(),
        Some(json!(1))
    );
    assert_eq!(middle.value("copy", "edition").unwrap(), Some(json!(2)));
    assert_eq!(baseline.class("reader-z").unwrap(), None);
    assert!(
        final_snapshot
            .query(Query::Value("edition", &json!(2)))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn repeated_overwrites_keep_current_values_and_old_snapshots_pinned() {
    let mut model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    for edition in 2..102 {
        let snapshot = model.snapshot().unwrap();
        let mut candidate = model.fork(&snapshot).unwrap();
        candidate.set("copy", "edition", json!(edition)).unwrap();
        candidate
            .set("copy", "title", json!(format!("edition {edition}")))
            .unwrap();
        candidate.set("copy", "title", json!("Current")).unwrap();
        model.commit(candidate).unwrap();
        assert_eq!(
            snapshot.value("copy", "edition").unwrap(),
            Some(json!(edition - 1))
        );
    }
    let current = model.snapshot().unwrap();
    assert_eq!(current.value("copy", "edition").unwrap(), Some(json!(101)));
    assert_eq!(
        current.value("copy", "title").unwrap(),
        Some(json!("Current"))
    );
    assert_eq!(baseline.value("copy", "edition").unwrap(), Some(json!(1)));
    assert_eq!(baseline.value("copy", "title").unwrap(), Some(json!(null)));
    assert!(
        current
            .query(Query::Value("edition", &json!(100)))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        current.query(Query::Value("edition", &json!(101))).unwrap(),
        vec!["copy"]
    );
}

#[test]
fn reverting_the_only_override_restores_the_root_value() {
    let mut model = VersionedModel::new(library()).unwrap();
    let baseline = model.snapshot().unwrap();
    let original_snapshot = storage::export(&baseline).unwrap();
    let mut update = model.fork(&baseline).unwrap();
    update.set("copy", "edition", json!(2)).unwrap();
    model.commit(update).unwrap();
    let changed = model.snapshot().unwrap();
    let mut revert = model.fork(&changed).unwrap();
    revert.set("copy", "edition", json!(1)).unwrap();
    assert_eq!(revert.value("copy", "edition").unwrap(), Some(json!(1)));
    model.commit(revert).unwrap();
    let restored = model.snapshot().unwrap();
    assert_same_model_content(storage::export(&restored).unwrap(), &original_snapshot);
    assert_eq!(storage::export(&baseline).unwrap(), original_snapshot);
    assert_eq!(changed.value("copy", "edition").unwrap(), Some(json!(2)));
}

#[test]
fn unchanged_large_field_survives_edited_siblings() {
    fn title(model: &impl ModelRead) -> (usize, String) {
        let mut result = None;
        model
            .inspect("copy", &mut |view| {
                let title = view.value("title").unwrap().as_str().unwrap();
                result = Some((title.as_ptr() as usize, title.to_owned()));
                Ok(())
            })
            .unwrap();
        result.unwrap()
    }
    let payload = "shared payload ".repeat(4096);
    let mut basic = library();
    basic.set("copy", "title", json!(payload)).unwrap();
    let original = title(&basic);
    let mut model = VersionedModel::new(basic).unwrap();
    let baseline = model.snapshot().unwrap();
    assert_eq!(title(&baseline), original);
    let mut siblings = Vec::new();
    for edition in 2..34 {
        let mut candidate = model.fork(&baseline).unwrap();
        candidate.set("copy", "edition", json!(edition)).unwrap();
        assert_eq!(title(&candidate).1, payload);
        siblings.push(candidate);
    }
    for candidate in &siblings {
        assert_eq!(title(candidate).1, payload);
    }
    model.commit(siblings.pop().unwrap()).unwrap();
    assert_eq!(title(&model.snapshot().unwrap()).1, payload);
    assert_eq!(title(&baseline), original);
}

#[test]
fn bounded_candidates_flow_through_facades_and_preserve_snapshot_membership() {
    fn names(read: &dyn ModelRead, wanted: &str) -> Vec<String> {
        let value = json!(wanted);
        let query = Query::Value("name", &value);
        let mut after = None;
        let mut result = Vec::new();
        loop {
            let batch = read.query_candidates(query, after, 1).unwrap();
            for (_, id) in batch.rows {
                if batch.exact || read.matches(&id, query).unwrap() {
                    result.push(id);
                }
            }
            if batch.exhausted {
                return result;
            }
            assert!(batch.after > after);
            after = batch.after;
        }
    }
    let basic = library();
    assert_eq!(names(&basic, "Central"), ["library"]);
    let mut tx = intersect_model::core::api::TransactionalModel::new(basic);
    let rejected: StorageResult<()> = tx.transaction(|edit| {
        edit.set("branch", "name", json!("Central"))?;
        assert_eq!(names(edit, "Central"), ["library", "branch"]);
        Err("veto".into())
    });
    assert!(rejected.is_err());
    assert_eq!(names(&tx, "Central"), ["library"]);
    let mut model = VersionedModel::new(library()).unwrap();
    let before = model.snapshot().unwrap();
    let mut fork = model.fork(&before).unwrap();
    fork.set("branch", "name", json!("Central")).unwrap();
    add_named(&mut fork, "new", "Library", "Central").unwrap();
    assert_eq!(names(&fork, "Central"), ["library", "branch", "new"]);
    assert_eq!(names(&before, "Central"), ["library"]);
    model.commit(fork).unwrap();
    assert_eq!(
        names(&model.snapshot().unwrap(), "Central"),
        ["library", "branch", "new"]
    );
    assert_eq!(names(&before, "Central"), ["library"]);
}
