use intersect_model::core::{
    model::Model,
    transaction::{TransactionContext, TransactionStatus},
};
use serde_json::json;

fn schema() -> intersect_model::definitions::schema::Schema {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .expect("fixture")
    .schema
}
fn context() -> TransactionContext {
    TransactionContext {
        id: "tx-1".into(),
        language_revision: "language-1".into(),
        base_generation: 3,
    }
}

#[test]
fn stage_prepare_commit_is_atomic_and_a_receipt_cannot_be_replayed() {
    let schema = schema();
    let mut model = Model::default();
    let mut transaction = model.begin_transaction(&schema, context()).expect("begin");
    transaction
        .stage(|m| {
            m.create(&schema, "member-1", "Member", "api", 0, 0)?;
            m.set_with_opposite(&schema, "member-1", "name", json!("grace"))
        })
        .expect("stage");
    assert!(model.elements.is_empty());
    assert_eq!(transaction.record().changes[0].kind, "created");
    assert!(transaction.commit(&mut model, "language-1", 3).is_err());
    transaction.prepare(&schema, |_| Ok(())).expect("prepare");
    assert!(transaction.stage(|_| Ok(())).is_err());
    let receipt = transaction
        .commit(&mut model, "language-1", 3)
        .expect("commit");
    assert_eq!(receipt.status, TransactionStatus::Committed);
    assert_eq!(model.elements[0].values["name"], json!("grace"));
    assert!(transaction.commit(&mut model, "language-1", 3).is_err());
    assert!(
        transaction
            .abort(TransactionStatus::Cancelled, "too late".into())
            .is_err()
    );
}

#[test]
fn failed_staging_validation_and_conflicts_keep_the_published_model() {
    let schema = schema();
    let mut model = Model::default();
    let mut tx = model.begin_transaction(&schema, context()).expect("begin");
    assert!(
        tx.stage(|m| {
            m.create(&schema, "bad", "Member", "api", 0, 0)?;
            Err::<(), _>("stop".into())
        })
        .is_err()
    );
    assert!(tx.model().elements.is_empty());
    assert!(model.elements.is_empty());
    assert_eq!(tx.status(), TransactionStatus::RolledBack);
    let mut tx = model.begin_transaction(&schema, context()).expect("begin");
    tx.stage(|m| m.create(&schema, "incomplete", "Member", "api", 0, 0))
        .expect("stage");
    assert!(tx.prepare(&schema, |_| Ok(())).is_err());
    assert_eq!(tx.status(), TransactionStatus::RolledBack);
    assert!(model.elements.is_empty());
    for (revision, generation) in [("new-language", 3), ("language-1", 4)] {
        let mut tx = model.begin_transaction(&schema, context()).expect("begin");
        tx.prepare(&schema, |_| Ok(())).expect("prepare");
        assert!(tx.commit(&mut model, revision, generation).is_err());
        assert_eq!(tx.status(), TransactionStatus::RolledBack);
    }
    let mut tx = model.begin_transaction(&schema, context()).expect("begin");
    tx.prepare(&schema, |_| Ok(())).expect("prepare");
    model
        .create(&schema, "external", "Member", "api", 0, 0)
        .expect("external edit");
    assert!(tx.commit(&mut model, "language-1", 3).is_err());
    assert_eq!(model.elements[0].id, "external");
}

#[test]
fn cancellation_and_language_validation_are_explicit() {
    let schema = schema();
    let model = Model::default();
    let mut tx = model.begin_transaction(&schema, context()).expect("begin");
    tx.abort(TransactionStatus::Cancelled, "cancelled by host".into())
        .expect("cancel");
    assert!(tx.prepare(&schema, |_| Ok(())).is_err());
    assert_eq!(tx.record().reason.as_deref(), Some("cancelled by host"));
    let mut tx = model.begin_transaction(&schema, context()).expect("begin");
    assert!(
        tx.prepare(&schema, |_| Err("language validation failed".into()))
            .is_err()
    );
    assert_eq!(tx.record().status, TransactionStatus::RolledBack);
}
