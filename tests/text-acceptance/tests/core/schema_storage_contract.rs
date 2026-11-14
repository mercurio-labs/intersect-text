use intersect_model::core::{
    frozen::FrozenStore,
    model::Element,
    storage::{self, ModelRead, ModelStore},
};
use intersect_model::definitions::schema::Schema;

fn schema() -> Schema {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .expect("Library fixture")
    .schema
}

#[test]
fn nested_rollback_preserves_local_descriptors_and_rejects_foreign_descriptors() {
    let schema = schema();
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    let other = FrozenStore::for_schema(&schema).unwrap();
    let type_id = store
        .schema_descriptor()
        .unwrap()
        .type_id("Library")
        .unwrap();
    let other_descriptor = other.schema_descriptor().unwrap();
    let other_type = other_descriptor.type_id("Library").unwrap();
    let foreign = other_descriptor
        .type_descriptor(&other_type)
        .unwrap()
        .feature("members")
        .unwrap()
        .id();
    let outer = store.begin().expect("outer transaction");
    let child = store.begin().expect("child transaction");
    store
        .create_element(&schema, "aborted", "Library", "resource", 0, 1)
        .expect("create child");
    let aborted = store.handle("aborted").expect("aborted handle");
    store.rollback(&child).expect("abort child");
    assert!(store.inspect_handle(&aborted, &mut |_| Ok(())).is_err());
    assert_eq!(
        store.schema_descriptor().unwrap().type_id("Library"),
        Some(type_id.clone())
    );

    store
        .create_element(&schema, "survivor", "Library", "resource", 0, 1)
        .expect("create survivor");
    let feature = store
        .schema_descriptor()
        .unwrap()
        .type_descriptor(&type_id)
        .unwrap()
        .feature("members")
        .unwrap()
        .id();
    let handle = store.handle("survivor").expect("surviving handle");
    store
        .inspect_handle(&handle, &mut |object| {
            assert_eq!(object.type_id(), Some(type_id.clone()));
            assert!(object.value_by_feature(&foreign).is_err());
            assert!(
                object
                    .value_by_feature(&feature)
                    .unwrap()
                    .unwrap()
                    .collection()
                    .unwrap()
                    .is_empty()
            );
            Ok(())
        })
        .expect("inspect surviving object");
    store.commit(&outer).expect("commit survivor");
}

#[test]
fn failed_creation_leaves_empty_and_seeded_typed_stores_unchanged() {
    let mut schema = schema();
    schema
        .classes
        .get_mut("Loan")
        .expect("Loan class")
        .abstract_type = true;
    for seed in [false, true] {
        let mut store = FrozenStore::for_schema(&schema).unwrap();
        if seed {
            storage::atomic(&mut store, |store| {
                store.put(Element::new(
                    &schema, "existing", "Member", "resource", 0, 1,
                )?)
            })
            .expect("seed store");
        }
        let baseline = storage::export(&store).expect("original contents");
        let original_type = store.schema_descriptor().unwrap().type_id("Member");
        let transaction = store.begin().expect("transaction");
        for (id, class) in [("", "Member"), ("missing", "Unknown"), ("abstract", "Loan")] {
            assert!(
                store
                    .create_element(&schema, id, class, "resource", 0, 1)
                    .is_err()
            );
            assert_eq!(
                storage::export(&store).expect("unchanged contents"),
                baseline
            );
            assert_eq!(
                store.schema_descriptor().unwrap().type_id("Member"),
                original_type
            );
        }
        if seed {
            assert!(
                store
                    .create_element(&schema, "existing", "Member", "resource", 0, 1)
                    .is_err()
            );
            assert_eq!(
                storage::export(&store).expect("unchanged contents"),
                baseline
            );
            assert_eq!(
                store.schema_descriptor().unwrap().type_id("Member"),
                original_type
            );
        }
        store
            .commit(&transaction)
            .expect("commit after handled write failures");
        assert_eq!(
            storage::export(&store).expect("unchanged committed contents"),
            baseline
        );
        assert_eq!(
            store.revision().expect("unchanged revision"),
            baseline.schema_revision
        );
    }
}
