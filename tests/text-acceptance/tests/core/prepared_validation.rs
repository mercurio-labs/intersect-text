//! Full audits must keep checking model values when schema metadata is prepared.
use super::Store;
use intersect_model::core::{
    api::{BasicModel, ModelEdit, TransactionalModel},
    model::{Element, Model},
    storage::{self, ModelRead, ModelView, Query},
};
use intersect_model::definitions::schema::Schema;
use serde_json::{Value, json};

fn schema() -> Schema {
    let source = include_str!("../../../../fixtures/library/library.ecore").replace(
        "<eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"title\"",
        "<eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"tags\" upperBound=\"2\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString\"/>\n    <eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"title\"",
    );
    intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&source).unwrap()
}

fn add(model: &mut Model, schema: &Schema, id: &str, class: &str) {
    let mut element = Element::new(schema, id, class, "memory:prepared", 0, 0).unwrap();
    element.values.insert("name".into(), json!(id));
    model.elements.push(element);
}

fn set(model: &mut Model, id: &str, feature: &str, value: Value) {
    model
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .unwrap()
        .values
        .insert(feature.into(), value);
}

fn populated(schema: &Schema, copies: usize) -> Model {
    let mut model = Model {
        schema_revision: schema.revision.clone(),
        resources: Default::default(),
        elements: Vec::new(),
    };
    for i in 0..copies {
        add(&mut model, schema, &format!("copy-{i}"), "BookCopy");
    }
    for (id, class) in [
        ("library", "Library"),
        ("branch", "Library"),
        ("reader", "Member"),
        ("loan", "Loan"),
    ] {
        add(&mut model, schema, id, class);
    }
    set(&mut model, "library", "copies", json!(["copy-0"]));
    set(&mut model, "copy-0", "library", json!("library"));
    set(&mut model, "library", "members", json!(["reader"]));
    set(&mut model, "library", "loans", json!(["loan"]));
    set(&mut model, "loan", "copy", json!("copy-0"));
    set(&mut model, "loan", "member", json!("reader"));
    model.resources =
        intersect_model::core::resources::records_for_elements(schema, &model.elements).unwrap();
    model
}

// Adapter parity is checked alongside explicit expected structural predicates;
// neither adapter's result alone is treated as the correctness oracle.
fn audit_both(model: &Model, schema: &Schema, draft: bool) -> Result<(), String> {
    let view = ModelView::new(model).unwrap();
    let audit = |read: &dyn ModelRead| {
        if draft {
            storage::validate_draft(read, schema)
        } else {
            storage::validate(read, schema)
        }
    };
    let actual = audit(&view);
    match crate::stores::from_model::<Store>(schema, model) {
        Ok(store) => {
            assert_eq!(actual, audit(&store));
            assert_eq!(storage::export(&store).unwrap(), *model);
        }
        Err(error) => assert!(
            actual.is_err(),
            "Typed import rejected a valid model: {error}"
        ),
    }
    actual
}

store_test! {
pub fn later_objects_of_an_already_checked_class_keep_all_value_checks() {
    let schema = schema();
    // The malformed object is beyond two 128-object identity pages, and its
    // class metadata has already been used for many valid objects.
    let original = populated(&schema, 257);
    audit_both(&original, &schema, false).unwrap();
    for (feature, value, expected) in [
        (
            "edition",
            json!("wrong"),
            "Invalid EInt value for BookCopy.edition",
        ),
        ("edition", json!([1]), "Singular feature cannot hold a list"),
        ("tags", json!(null), "Multiple feature requires a list"),
        ("tags", json!(["x", "x"]), "Duplicate value: BookCopy.tags"),
        (
            "tags",
            json!(["x", "y", "z"]),
            "Multiplicity violation: BookCopy.tags",
        ),
        (
            "tags",
            json!([1]),
            "Invalid EString value for BookCopy.tags",
        ),
        ("available", json!(true), "Derived values cannot be stored"),
        ("unknown", json!(true), "Unknown feature BookCopy.unknown"),
    ] {
        let mut malformed = original.clone();
        set(&mut malformed, "copy-256", feature, value);
        for draft in [true, false] {
            let error = audit_both(&malformed, &schema, draft).unwrap_err();
            assert!(
                error.contains(expected),
                "{feature}, draft={draft}: {error}"
            );
        }
        audit_both(&original, &schema, false).unwrap();
    }
}
}

store_test! {
pub fn full_audits_continue_to_check_reference_targets_opposites_and_ownership() {
    let schema = schema();
    let original = populated(&schema, 129);
    audit_both(&original, &schema, false).unwrap();
    for (id, feature, value, expected) in [
        (
            "loan",
            "copy",
            json!("missing"),
            "Missing reference target missing",
        ),
        (
            "loan",
            "copy",
            json!("reader"),
            "Reference type mismatch Loan.copy",
        ),
        ("loan", "copy", json!(123), "Reference must be an identity"),
        (
            "copy-0",
            "library",
            json!(null),
            "Missing reciprocal opposite BookCopy/library",
        ),
        (
            "branch",
            "members",
            json!(["reader"]),
            "Multiple containment owners",
        ),
    ] {
        let mut malformed = original.clone();
        set(&mut malformed, id, feature, value);
        for draft in [true, false] {
            let error = audit_both(&malformed, &schema, draft).unwrap_err();
            assert_eq!(error, expected, "{id}.{feature}, draft={draft}");
        }
    }
}
}

store_test! {
pub fn full_audits_keep_containment_cycle_detection_after_field_checks() {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        r##"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="tree" nsURI="urn:test:prepared-tree" nsPrefix="tree">
          <eClassifiers xsi:type="ecore:EClass" name="Node">
            <eStructuralFeatures xsi:type="ecore:EReference" name="children" upperBound="-1" containment="true" eType="#//Node"/>
          </eClassifiers>
        </ecore:EPackage>"##,
    ).unwrap();
    let mut model = Model::default();
    for id in ["a", "b", "c"] {
        model
            .create(&schema, id, "Node", "memory:tree", 0, 0)
            .unwrap();
    }
    set(&mut model, "a", "children", json!(["b"]));
    set(&mut model, "b", "children", json!(["c"]));
    model.resources =
        intersect_model::core::resources::records_for_elements(&schema, &model.elements).unwrap();
    audit_both(&model, &schema, false).unwrap();
    set(&mut model, "c", "children", json!(["a"]));
    model.resources =
        intersect_model::core::resources::records_for_elements(&schema, &model.elements).unwrap();
    for draft in [true, false] {
        assert_eq!(
            audit_both(&model, &schema, draft).unwrap_err(),
            "Containment cycle"
        );
    }
    set(&mut model, "c", "children", json!([]));
    model.resources =
        intersect_model::core::resources::records_for_elements(&schema, &model.elements).unwrap();
    audit_both(&model, &schema, false).unwrap();
}
}

store_test! {
pub fn raw_schema_changes_with_the_same_revision_are_seen_on_the_next_audit() {
    let mut schema = schema();
    // Exercise the legacy scalar contract directly, independently of the
    // imported runtime-domain projection that has its own integrity checks.
    schema.runtime_feature_domains.clear();
    let revision = schema.revision.clone();
    let mut model = populated(&schema, 1);
    audit_both(&model, &schema, false).unwrap();

    set(&mut model, "reader", "name", json!(null));
    schema
        .classes
        .get_mut("Named")
        .unwrap()
        .features
        .get_mut("name")
        .unwrap()
        .lower = 0;
    audit_both(&model, &schema, false).unwrap();
    schema
        .classes
        .get_mut("Named")
        .unwrap()
        .features
        .get_mut("name")
        .unwrap()
        .lower = 1;
    assert_eq!(
        audit_both(&model, &schema, false).unwrap_err(),
        "Multiplicity violation: Member.name"
    );
    audit_both(&model, &schema, true).unwrap();
    set(&mut model, "reader", "name", json!("Reader"));

    set(&mut model, "copy-0", "tags", json!(["same", "same"]));
    schema
        .classes
        .get_mut("BookCopy")
        .unwrap()
        .features
        .get_mut("tags")
        .unwrap()
        .unique = false;
    audit_both(&model, &schema, false).unwrap();
    schema
        .classes
        .get_mut("BookCopy")
        .unwrap()
        .features
        .get_mut("tags")
        .unwrap()
        .unique = true;
    assert_eq!(
        audit_both(&model, &schema, false).unwrap_err(),
        "Duplicate value: BookCopy.tags"
    );
    set(&mut model, "copy-0", "tags", json!([]));

    let edition = schema
        .classes
        .get_mut("BookCopy")
        .unwrap()
        .features
        .get_mut("edition")
        .unwrap();
    edition.datatype = None;
    edition.target = "EString".into();
    let error = audit_both(&model, &schema, false).unwrap_err();
    assert!(
        error.contains("Invalid EString value for BookCopy.edition"),
        "{error}"
    );
    set(&mut model, "copy-0", "edition", json!("revised type"));
    audit_both(&model, &schema, false).unwrap();
    assert_eq!(schema.revision, revision);
}
}

store_test! {
pub fn preparation_preserves_revision_identity_and_abstract_error_precedence() {
    let mut schema = schema();
    let original = populated(&schema, 1);
    // An unused malformed class fails effective-feature preparation, but that
    // preparation historically occurs after revision and object identity checks.
    schema.runtime_feature_domains.clear();
    let mut malformed = schema.classes["Member"].clone();
    malformed.name = "Broken".into();
    malformed.parents = vec!["MissingBase".into()];
    schema.classes.insert("Broken".into(), malformed);
    schema.validate_instantiations().unwrap();
    let preparation_error = schema.features("Broken").unwrap_err();

    let mut wrong_revision = original.clone();
    wrong_revision.schema_revision = "wrong revision".into();
    assert_eq!(
        audit_both(&wrong_revision, &schema, false).unwrap_err(),
        "Model belongs to a different schema revision"
    );

    let mut wrong_identity = original.clone();
    wrong_identity.elements[0].type_identity = "wrong identity".into();
    assert_eq!(
        audit_both(&wrong_identity, &schema, false).unwrap_err(),
        "Model type identity does not match schema"
    );

    let mut abstract_instance = original.clone();
    abstract_instance.elements[0].class = "Named".into();
    abstract_instance.elements[0].type_identity = schema.identity("Named");
    assert_eq!(
        audit_both(&abstract_instance, &schema, false).unwrap_err(),
        "Abstract instance"
    );
    assert_eq!(
        audit_both(&original, &schema, false).unwrap_err(),
        preparation_error
    );
}
}

store_test! {
pub fn final_audit_rejects_late_missing_required_values_and_rolls_back_relationship_edits() {
    let schema = schema();
    let snapshot = populated(&schema, 129);
    let store = crate::stores::from_model::<Store>(&schema, &snapshot).unwrap();
    let mut model = TransactionalModel::new(BasicModel::from_store(schema, store).unwrap());
    model.validate().unwrap();
    let mut callback_completed = false;
    let error = model
        .transaction(|edit| {
            edit.set_many(&[
                ("library", "copies", json!([])),
                ("branch", "copies", json!(["copy-0"])),
                ("copy-128", "edition", json!(7)),
            ])?;
            edit.create("late-loan", "Loan", "memory:prepared")?;
            edit.set_many(&[
                ("late-loan", "name", json!("Late loan")),
                ("late-loan", "copy", json!("copy-128")),
            ])?;
            // The missing member is permitted during draft construction. The
            // transaction's final full audit must reject it after callback success.
            callback_completed = true;
            Ok(())
        })
        .unwrap_err();
    assert!(callback_completed);
    assert_eq!(error, "Multiplicity violation: Loan.member");
    assert_eq!(storage::export(&model).unwrap(), snapshot);
    assert!(model.class("late-loan").unwrap().is_none());
    assert_eq!(
        model
            .query(Query::Value("library", &json!("library")))
            .unwrap(),
        vec!["copy-0"]
    );
    assert!(
        model
            .query(Query::Value("library", &json!("branch")))
            .unwrap()
            .is_empty()
    );
    model.validate().unwrap();
    model
        .transaction(|edit| edit.set("copy-128", "edition", json!(2)))
        .unwrap();
    assert_eq!(model.value("copy-128", "edition").unwrap(), Some(json!(2)));
}
}
