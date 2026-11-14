use intersect_model::core::{
    frozen::FrozenStore,
    model::Element,
    storage::{self, ModelRead, ModelStore, ReferenceView},
};
use intersect_model::definitions::schema::Schema;
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

fn member(schema: &Schema, name: &str) -> Element {
    let mut member =
        Element::new(schema, "member", "Member", "members", 0, 1).expect("Member defaults");
    member.values.insert("name".into(), json!(name));
    member
}

fn store(schema: &Schema) -> FrozenStore {
    let mut store = FrozenStore::for_schema(schema).expect("frozen schema");
    storage::atomic(&mut store, |store| {
        store.configure_schema(schema)?;
        store.put(member(schema, "original"))?;
        store.create_element(schema, "loan", "Loan", "loans", 0, 1)?;
        store.set_value("loan", "member", Some(json!("member")))
    })
    .expect("seed reference and target");
    store
}

#[test]
fn nested_watermarks_restore_identity_bindings_and_preserve_only_committed_incarnations() {
    let schema = schema();
    for commit_child in [false, true] {
        for commit_outer in [false, true] {
            let mut store = store(&schema);
            let original = store.handle("member").expect("original handle");
            let loan = store.handle("loan").expect("loan handle");
            let outer = store.begin().expect("outer transaction");
            store.remove("member").expect("remove original");
            store
                .put(member(&schema, "outer"))
                .expect("outer incarnation");
            let outer_member = store.handle("member").expect("outer handle");

            let child = store.begin().expect("child transaction");
            store.remove("member").expect("remove outer incarnation");
            store
                .create_element(&schema, "member", "Member", "members", 2, 3)
                .expect("child incarnation");
            store
                .set_value("member", "name", Some(json!("child")))
                .expect("child name");
            let child_member = store.handle("member").expect("child handle");
            if commit_child {
                store.commit(&child).expect("commit child");
            } else {
                store.rollback(&child).expect("rollback child");
            }
            let after_child = if commit_child {
                &child_member
            } else {
                &outer_member
            };
            assert_eq!(store.handle("member").as_ref(), Some(after_child));
            store
                .inspect_handle(&loan, &mut |view| {
                    assert!(matches!(
                        view.value("member").expect("member reference").reference(),
                        Some(ReferenceView::Resolved(target)) if &target == after_child
                    ));
                    Ok(())
                })
                .expect("loan view");

            if commit_outer {
                store.commit(&outer).expect("commit outer");
            } else {
                store.rollback(&outer).expect("rollback outer");
            }
            let survivor = if commit_outer { after_child } else { &original };
            assert_eq!(store.handle("member").as_ref(), Some(survivor));
            for handle in [&original, &outer_member, &child_member] {
                assert_eq!(
                    store.inspect_handle(handle, &mut |_| Ok(())).is_ok(),
                    handle == survivor
                );
            }
            assert_eq!(
                store.value("member", "name").expect("surviving value"),
                Some(json!(if !commit_outer {
                    "original"
                } else if commit_child {
                    "child"
                } else {
                    "outer"
                }))
            );
            store
                .inspect_handle(&loan, &mut |view| {
                    assert!(matches!(
                        view.value("member").expect("member reference").reference(),
                        Some(ReferenceView::Resolved(target)) if &target == survivor
                    ));
                    Ok(())
                })
                .expect("loan view");
        }
    }
}

#[test]
fn committed_child_deletions_cannot_resurrect_parent_allocations_on_rollback() {
    let schema = schema();
    let mut store = store(&schema);
    let baseline = storage::export(&store).expect("baseline");
    let outer = store.begin().expect("outer transaction");
    store
        .create_element(&schema, "temporary", "Member", "members", 0, 1)
        .expect("parent allocation");
    let first = store.handle("temporary").expect("first incarnation");
    let child = store.begin().expect("child transaction");
    store
        .set_value("temporary", "name", Some(json!("changed")))
        .expect("child edit");
    store.remove("temporary").expect("child deletion");
    let grandchild = store.begin().expect("grandchild transaction");
    store
        .create_element(&schema, "temporary", "Member", "members", 2, 3)
        .expect("grandchild allocation");
    let second = store.handle("temporary").expect("second incarnation");
    store.commit(&grandchild).expect("commit grandchild");
    store.commit(&child).expect("commit child");
    store.rollback(&outer).expect("rollback all allocations");
    assert_eq!(storage::export(&store).expect("restored model"), baseline);
    assert!(store.inspect_handle(&first, &mut |_| Ok(())).is_err());
    assert!(store.inspect_handle(&second, &mut |_| Ok(())).is_err());

    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "temporary", "Member", "members", 4, 5)
    })
    .expect("later allocation");
    let later = store.handle("temporary").expect("later incarnation");
    assert_ne!(later, first);
    assert_ne!(later, second);
    assert!(store.inspect_handle(&first, &mut |_| Ok(())).is_err());
    assert!(store.inspect_handle(&second, &mut |_| Ok(())).is_err());
}

#[test]
fn invalid_checkpoint_leaves_allocations_and_history_intact() {
    let schema = schema();
    let mut store = store(&schema);
    let baseline = storage::export(&store).expect("baseline");
    let outer = store.begin().expect("outer transaction");
    store
        .set_value("member", "name", Some(json!("outer")))
        .expect("edit original");
    store
        .create_element(&schema, "outer", "Member", "members", 0, 1)
        .expect("outer allocation");
    let child = store.begin().expect("child transaction");
    store
        .create_element(&schema, "child", "Member", "members", 0, 1)
        .expect("child allocation");
    let outer_handle = store.handle("outer").expect("outer handle");
    let child_handle = store.handle("child").expect("child handle");
    assert!(store.rollback(&outer).is_err());
    assert!(store.commit(&outer).is_err());
    assert!(store.inspect_handle(&outer_handle, &mut |_| Ok(())).is_ok());
    assert!(store.inspect_handle(&child_handle, &mut |_| Ok(())).is_ok());
    store.rollback(&child).expect("valid child rollback");
    assert!(store.inspect_handle(&outer_handle, &mut |_| Ok(())).is_ok());
    assert!(
        store
            .inspect_handle(&child_handle, &mut |_| Ok(()))
            .is_err()
    );
    store.rollback(&outer).expect("valid outer rollback");
    assert!(
        store
            .inspect_handle(&outer_handle, &mut |_| Ok(()))
            .is_err()
    );
    assert_eq!(storage::export(&store).expect("restored model"), baseline);
}

#[test]
fn atomic_failure_and_unwind_remove_allocations_before_restoring_old_identity() {
    let schema = schema();
    for unwind in [false, true] {
        let mut store = store(&schema);
        let baseline = storage::export(&store).expect("baseline");
        let original = store.handle("member").expect("original handle");
        let mut provisional = None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            storage::atomic(&mut store, |store| {
                store.remove("member")?;
                store.put(member(&schema, "provisional"))?;
                provisional = store.handle("member");
                if unwind {
                    panic!("injected interruption");
                }
                Err::<(), _>("injected write failure".into())
            })
        }));
        if unwind {
            assert!(result.is_err());
        } else {
            assert_eq!(
                result.expect("returned normally"),
                Err("injected write failure".into())
            );
        }
        assert_eq!(storage::export(&store).expect("restored model"), baseline);
        assert!(store.inspect_handle(&original, &mut |_| Ok(())).is_ok());
        assert!(
            store
                .inspect_handle(&provisional.expect("provisional handle"), &mut |_| Ok(()))
                .is_err()
        );
    }
}
