use intersect_model::core::{
    frozen::FrozenStore,
    model::Element,
    storage::{self, ModelRead, ModelStore, ObjectHandle, ObjectView, Query, ReferenceView},
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

fn member(schema: &Schema, id: &str) -> Element {
    let mut element =
        Element::new(schema, id, "Member", "resource:members", 3, 12).expect("Member defaults");
    element.values.insert("name".into(), json!(id));
    element
}

fn seeded<S: ModelStore>(mut store: S, schema: &Schema) -> S {
    storage::atomic(&mut store, |store| {
        store.configure_schema(schema)?;
        store.put(member(schema, "member"))
    })
    .expect("initialize store");
    store
}

fn store(schema: &Schema) -> FrozenStore {
    seeded(
        FrozenStore::for_schema(schema).expect("frozen schema"),
        schema,
    )
}

fn live(store: &FrozenStore, handle: &ObjectHandle) -> bool {
    store.inspect_handle(handle, &mut |_| Ok(())).is_ok()
}

fn with_view<T>(
    store: &FrozenStore,
    handle: &ObjectHandle,
    mut read: impl FnMut(ObjectView<'_>) -> T,
) -> T {
    let mut result = None;
    store
        .inspect_handle(handle, &mut |view| {
            result = Some(read(view));
            Ok(())
        })
        .expect("live object view");
    result.expect("callback result")
}

#[test]
fn object_and_schema_handles_are_scoped_to_their_owning_context() {
    let schema = schema();
    let first = store(&schema);
    let second = store(&schema);
    let first_object = first.handle("member").expect("first object");
    let second_object = second.handle("member").expect("second object");
    assert_ne!(first_object, second_object);
    assert!(live(&first, &first_object));
    assert!(!live(&second, &first_object));
    assert!(!live(&first, &second_object));

    let first_schema = first.schema_descriptor().expect("first schema");
    let second_schema = second.schema_descriptor().expect("second schema");
    let first_type = first_schema.type_id("Member").expect("first type");
    let second_type = second_schema.type_id("Member").expect("second type");
    assert_ne!(first_type, second_type);
    assert!(first_schema.type_descriptor(&first_type).is_ok());
    assert!(second_schema.type_descriptor(&first_type).is_err());
    let first_feature = first_schema
        .type_descriptor(&first_type)
        .expect("type descriptor")
        .feature("name")
        .expect("name feature")
        .id();
    assert!(
        second_schema
            .type_descriptor(&second_type)
            .expect("second descriptor")
            .feature_descriptor(&first_feature)
            .is_err()
    );
    with_view(&first, &first_object, |object| {
        assert_eq!(object.type_id(), Some(first_type.clone()));
        let name = object.value_by_feature(&first_feature).unwrap().unwrap();
        assert_eq!(name.as_str(), Some("member"));
        assert!(std::ptr::eq(
            name.as_str().unwrap(),
            object.value("name").unwrap().as_str().unwrap()
        ));
    });
    with_view(&second, &second_object, |object| {
        assert!(object.value_by_feature(&first_feature).is_err());
    });
}

#[test]
fn removed_objects_and_rolled_back_allocations_cannot_reuse_live_handles() {
    let schema = schema();
    let mut store = store(&schema);
    let original = store.handle("member").expect("original handle");
    let deletion = store.begin().expect("begin deletion");
    store.remove("member").expect("remove");
    assert!(!live(&store, &original));
    store.rollback(&deletion).expect("undo deletion");
    assert!(live(&store, &original));

    storage::atomic(&mut store, |store| {
        store.remove("member")?;
        store.put(member(&schema, "member"))
    })
    .expect("replace identity");
    let replacement = store.handle("member").expect("replacement handle");
    assert_ne!(original, replacement);
    assert!(!live(&store, &original));
    assert!(live(&store, &replacement));

    let outer = store.begin().expect("begin allocation");
    store.put(member(&schema, "provisional")).expect("allocate");
    let provisional = store.handle("provisional").expect("provisional handle");
    store.rollback(&outer).expect("rollback allocation");
    storage::atomic(&mut store, |store| {
        store.put(member(&schema, "provisional"))
    })
    .expect("allocate identity again");
    let next = store.handle("provisional").expect("new handle");
    assert_ne!(provisional, next);
    assert!(!live(&store, &provisional));
    assert!(live(&store, &next));
}

#[test]
fn reference_identity_survives_target_unavailability_and_rebinding() {
    let schema = schema();
    let mut store = store(&schema);
    let mut loan =
        Element::new(&schema, "loan", "Loan", "resource:loans", 10, 30).expect("Loan defaults");
    loan.values.insert("member".into(), json!("member"));
    storage::atomic(&mut store, |store| store.put(loan)).expect("store reference");
    let owner = store.handle("loan").expect("loan handle");
    let original = store.handle("member").expect("original member");
    assert!(with_view(&store, &owner, |view| matches!(
        view.value("member").expect("member reference").reference(),
        Some(ReferenceView::Resolved(target)) if target == original
    )));

    storage::atomic(&mut store, |store| store.remove("member")).expect("remove target");
    assert!(with_view(&store, &owner, |view| matches!(
        view.value("member")
            .expect("unavailable reference")
            .reference(),
        Some(ReferenceView::Unavailable { identity: "member" })
    )));
    assert_eq!(
        store
            .get("loan")
            .expect("export loan")
            .expect("loan")
            .values["member"],
        json!("member")
    );

    storage::atomic(&mut store, |store| store.put(member(&schema, "member")))
        .expect("reload identity");
    let reloaded = store.handle("member").expect("reloaded member");
    assert_ne!(original, reloaded);
    assert!(!live(&store, &original));
    assert!(with_view(&store, &owner, |view| matches!(
        view.value("member").expect("rebound reference").reference(),
        Some(ReferenceView::Resolved(target)) if target == reloaded
    )));
    let exported = store.get("loan").expect("export loan").expect("loan");
    assert_eq!(exported.values["member"], json!("member"));
    assert_eq!(exported.resource, "resource:loans");
    assert_eq!((exported.start, exported.end), (10, 30));
}

#[test]
fn absent_null_and_explicit_default_fields_survive_the_compatibility_boundary() {
    let schema = schema();
    let mut store = store(&schema);
    let mut absent = member(&schema, "absent");
    absent.values.remove("name");
    let mut null = member(&schema, "null");
    null.values.insert("name".into(), json!(null));
    let explicit_default =
        Element::new(&schema, "default", "Member", "resource:members", 0, 0).expect("defaults");
    let expected_default = explicit_default.values["name"].clone();
    storage::atomic(&mut store, |store| {
        store.put(absent)?;
        store.put(null)?;
        store.put(explicit_default)
    })
    .expect("store field states");

    let absent = store.handle("absent").expect("absent handle");
    let null = store.handle("null").expect("null handle");
    let explicit_default = store.handle("default").expect("default handle");
    with_view(&store, &absent, |view| {
        assert!(!view.is_set("name"));
        assert!(view.value("name").is_none());
    });
    assert!(with_view(&store, &null, |view| view.is_set("name")));
    assert!(with_view(&store, &explicit_default, |view| view.is_set("name")));
    assert_eq!(
        with_view(&store, &null, |view| view
            .value("name")
            .expect("null value")
            .to_json()),
        json!(null)
    );
    assert_eq!(
        with_view(&store, &explicit_default, |view| view
            .value("name")
            .expect("default value")
            .to_json()),
        expected_default
    );
    assert!(
        !store
            .get("absent")
            .expect("get absent")
            .expect("absent")
            .values
            .contains_key("name")
    );
    assert_eq!(
        store.get("null").expect("get null").expect("null").values["name"],
        json!(null)
    );
}

#[test]
fn canonical_defaults_survive_field_removal_assignment_and_rollback() {
    let schema = schema();
    let mut store = FrozenStore::for_schema(&schema).expect("frozen schema");
    storage::atomic(&mut store, |store| {
        store.create_element(&schema, "member", "Member", "resource:members", 0, 0)
    })
    .expect("canonical construction");
    let handle = store.handle("member").expect("member handle");
    let default = with_view(&store, &handle, |view| {
        view.value("name").expect("effective default").to_json()
    });
    assert!(with_view(&store, &handle, |view| view.is_set("name")));
    let change = store.begin().expect("begin assignment");
    store
        .set_value("member", "name", None)
        .expect("remove field");
    assert!(!with_view(&store, &handle, |view| view.is_set("name")));
    store
        .set_value("member", "name", Some(json!("changed")))
        .expect("assign name");
    assert!(with_view(&store, &handle, |view| view.is_set("name")));
    store.rollback(&change).expect("rollback assignment");
    assert!(with_view(&store, &handle, |view| view.is_set("name")));
    assert_eq!(
        with_view(&store, &handle, |view| view
            .value("name")
            .expect("effective default")
            .to_json()),
        default
    );
}

fn scratch_schema() -> Schema {
    let library = include_str!("../../../../fixtures/library/library.ecore").replace(
        "name=\"Member\" eSuperTypes=\"#//Named\"/>",
        "name=\"Member\" eSuperTypes=\"#//Named\"><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"scratch\" upperBound=\"-1\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString\"/></eClassifiers>",
    );
    intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&library).unwrap()
}

#[test]
fn committed_child_edits_do_not_break_rollback_after_parent_removes_new_object() {
    let schema = scratch_schema();
    for remove in [true, false] {
        let mut store = self::store(&schema);
        let baseline = storage::export(&store).expect("baseline");
        let outer = store.begin().expect("outer");
        let mut temporary = member(&schema, "temporary");
        temporary.values.insert("scratch".into(), json!([]));
        store.put(temporary).expect("new outer object");
        let handle = store.handle("temporary").expect("temporary handle");
        let inner = store.begin().expect("inner");
        store
            .set_value("temporary", "name", Some(json!("inner")))
            .expect("inner field");
        store
            .append_value("temporary", "scratch", json!("item"))
            .expect("inner append");
        store.set_span("temporary", 20, 40).expect("inner span");
        store.commit(&inner).expect("commit child");
        if remove {
            store.remove("temporary").expect("parent removal");
        } else {
            store
                .put(member(&schema, "temporary"))
                .expect("parent replacement without scratch");
        }
        store.rollback(&outer).expect("rollback parent");
        assert_eq!(storage::export(&store).expect("restored store"), baseline);
        assert!(!live(&store, &handle));
    }
}

#[test]
fn independent_collection_edits_index_nested_json_as_one_item() {
    let schema = scratch_schema();
    let mut store = self::store(&schema);
    storage::atomic(&mut store, |store| {
        store.set_value("member", "scratch", Some(json!([])))?;
        store.append_value("member", "scratch", json!(["nested", null]))
    })
    .expect("append nested JSON item");
    let nested = json!(["nested", null]);
    assert_eq!(
        store
            .query(Query::Value("scratch", &nested))
            .expect("nested index"),
        ["member"]
    );
    assert!(
        store
            .query(Query::Value("scratch", &json!("nested")))
            .expect("inner string index")
            .is_empty()
    );
    let transaction = store.begin().expect("begin truncation");
    store
        .truncate_values("member", "scratch", 0)
        .expect("truncate nested item");
    assert!(
        store
            .query(Query::Value("scratch", &nested))
            .expect("removed nested index")
            .is_empty()
    );
    store.rollback(&transaction).expect("rollback truncation");
    assert_eq!(
        store
            .query(Query::Value("scratch", &nested))
            .expect("restored nested index"),
        ["member"]
    );
    let snapshot = storage::export(&store).expect("raw snapshot");
    let restored =
        FrozenStore::from_materialized(&schema, &snapshot).expect("compatibility reload");
    assert_eq!(
        restored
            .query(Query::Value("scratch", &nested))
            .expect("reloaded nested index"),
        ["member"]
    );
}
