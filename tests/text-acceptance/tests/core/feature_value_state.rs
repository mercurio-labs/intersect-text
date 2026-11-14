//! E09 state transitions across every built-in scalar codec and collection kind.
//! Codec lexical conformance is independently tested elsewhere; these controls
//! verify that checked model state and rollback consume those values consistently.
use super::Store;
use intersect_model::core::{
    api::ModelEdit,
    model::Model,
    storage::{self, CollectionEdit, ModelRead},
};
use intersect_model::definitions::scalars::Builtin;
use serde_json::{Value, json};

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

const CASES: &[(&str, &str)] = &[
    ("EDate", "1970-01-02"),
    ("EByteArray", "00FF"),
    ("EChar", "65"),
    ("ECharacterObject", "65"),
    ("EBigDecimal", "1.00"),
    ("EBigInteger", "9007199254740993"),
    ("EString", "sample"),
    ("EBoolean", "true"),
    ("EByte", "42"),
    ("EShort", "42"),
    ("EInt", "42"),
    ("ELong", "9007199254740993"),
    ("EBooleanObject", "true"),
    ("EByteObject", "42"),
    ("EShortObject", "42"),
    ("EIntegerObject", "42"),
    ("ELongObject", "9007199254740993"),
    ("EFloat", "1.25"),
    ("EDouble", "1.25"),
    ("EFloatObject", "1.25"),
    ("EDoubleObject", "1.25"),
];

store_test! {
pub fn all_scalar_families_distinguish_defaults_assignment_reset_and_collection_history() {
    assert_eq!(CASES.len(), 21);
    for (name, lexical) in CASES {
        let codec = Builtin::named(name).unwrap();
        let value = codec.parse(lexical).unwrap();
        let mut fields = format!(
            r#"<eStructuralFeatures x:type="e:EAttribute" name="ordinary" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//{name}"/>
<eStructuralFeatures x:type="e:EAttribute" name="history" unsettable="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//{name}"/>
<eStructuralFeatures x:type="e:EAttribute" name="literal" unsettable="true" defaultValueLiteral="{lexical}" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//{name}"/>"#
        );
        for (index, (ordered, unique)) in
            [(true, true), (true, false), (false, true), (false, false)]
                .into_iter()
                .enumerate()
        {
            fields += &format!(
                r#"<eStructuralFeatures x:type="e:EAttribute" name="list{index}" unsettable="true" ordered="{ordered}" unique="{unique}" upperBound="3" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//{name}"/>"#
            );
        }
        let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&format!(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="state" nsURI="urn:scalar-state"><eClassifiers x:type="e:EClass" name="Item">{fields}</eClassifiers></e:EPackage>"#)).unwrap();
        let mut model = crate::stores::basic::<Store>(schema.clone()).unwrap();
        model.create("a", "Item", "model").unwrap();
        let fresh = storage::export(&model).unwrap();
        assert_eq!(
            model.value("a", "ordinary").unwrap(),
            Some(codec.default_value()),
            "{name}"
        );
        assert_eq!(
            model.value("a", "literal").unwrap(),
            Some(value.clone()),
            "{name}"
        );
        for field in ["ordinary", "history", "literal"] {
            assert!(!model.is_set("a", field).unwrap(), "{name}.{field}");
        }
        model.set("a", "ordinary", codec.default_value()).unwrap();
        model.set("a", "history", codec.default_value()).unwrap();
        model.set("a", "literal", value.clone()).unwrap();
        assert!(!model.is_set("a", "ordinary").unwrap());
        assert!(model.is_set("a", "history").unwrap());
        assert!(model.is_set("a", "literal").unwrap());
        if codec.nullable() {
            model.set("a", "literal", Value::Null).unwrap();
            assert_eq!(model.value("a", "literal").unwrap(), Some(Value::Null));
            assert!(model.is_set("a", "literal").unwrap());
        }
        for field in ["ordinary", "history", "literal"] {
            model.unset("a", field).unwrap();
        }
        assert_same_model_content(storage::export(&model).unwrap(), &fresh);
        for (index, unique) in [true, false, true, false].into_iter().enumerate() {
            let field = format!("list{index}");
            assert!(!model.is_set("a", &field).unwrap());
            assert!(
                !model
                    .edit_collection("a", &field, CollectionEdit::Clear)
                    .unwrap()
            );
            assert!(model.is_set("a", &field).unwrap());
            model.unset("a", &field).unwrap();
            assert!(
                model
                    .edit_collection("a", &field, CollectionEdit::Add(value.clone()))
                    .unwrap()
            );
            assert_eq!(
                model
                    .edit_collection("a", &field, CollectionEdit::Add(value.clone()))
                    .unwrap(),
                !unique
            );
            let copies = if unique { 1 } else { 2 };
            assert_eq!(
                model.value("a", &field).unwrap(),
                Some(Value::Array(vec![value.clone(); copies]))
            );
            for _ in 0..copies {
                assert!(
                    model
                        .edit_collection("a", &field, CollectionEdit::Remove(value.clone()))
                        .unwrap()
                );
            }
            assert!(model.is_set("a", &field).unwrap());
            let persisted = storage::export(&model).unwrap();
            let wire = serde_json::to_string(&persisted).unwrap();
            let reloaded = crate::stores::from_model::<Store>(&schema, &serde_json::from_str(&wire).unwrap()).unwrap();
            assert!(storage::is_set(&reloaded, &schema, "a", &field).unwrap());
            assert_eq!(storage::export(&reloaded).unwrap(), persisted);
            model.unset("a", &field).unwrap();
            assert_same_model_content(storage::export(&model).unwrap(), &fresh);
        }
        let before_failure = storage::export(&model).unwrap();
        assert!(
            model
                .set_many(&[
                    ("a", "history", value),
                    ("a", "ordinary", json!({"invalid":true}))
                ])
                .is_err()
        );
        assert_eq!(
            storage::export(&model).unwrap(),
            before_failure,
            "{name} late failure"
        );
        model.validate().unwrap();
    }
}
}

store_test! {
pub fn absent_and_explicit_empty_literals_keep_distinct_defaults_and_history() {
    for datatype in ["EString", "EByteArray"] {
        let text = format!(
            r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="empty" nsURI="urn:empty">
<eClassifiers x:type="e:EClass" name="Item">
<eStructuralFeatures x:type="e:EAttribute" name="absent" unsettable="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//{datatype}"/>
<eStructuralFeatures x:type="e:EAttribute" name="literal" unsettable="true" defaultValueLiteral="" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//{datatype}"/>
</eClassifiers></e:EPackage>"#
        );
        let schema =
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&text)
                .unwrap();
        let fields = schema.features("Item").unwrap();
        assert!(fields["absent"].default.is_null(), "{datatype}");
        assert!(!fields["literal"].default.is_null(), "{datatype}");
        let empty = fields["literal"].default.clone();
        let mut model = crate::stores::basic::<Store>(schema).unwrap();
        model.create("a", "Item", "model").unwrap();
        assert_eq!(model.value("a", "literal").unwrap(), Some(empty.clone()));
        assert!(!model.is_set("a", "literal").unwrap());
        model.set("a", "literal", empty.clone()).unwrap();
        assert!(model.is_set("a", "literal").unwrap());
        model.set("a", "literal", Value::Null).unwrap();
        assert!(model.is_set("a", "literal").unwrap());
        model.unset("a", "literal").unwrap();
        assert_eq!(model.value("a", "literal").unwrap(), Some(empty));
        assert!(!model.is_set("a", "literal").unwrap());
        model.validate().unwrap();
    }
}
}
