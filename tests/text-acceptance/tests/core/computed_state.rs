//! Shared validation of language-owned computed values; no domain algorithm in core.
use super::Store;
use intersect_model::core::{
    query::{DerivedValues, QueryBudget},
    storage::{self, ModelRead, ModelStore},
};
use intersect_model::definitions::schema::Schema;
use serde_json::{Value, json};

fn fixture() -> (Schema, Store) {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="p" nsURI="urn:computed">
<eClassifiers x:type="e:EClass" name="Item">
<eStructuralFeatures x:type="e:EAttribute" name="words" derived="true" upperBound="2" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EAttribute" name="bag" derived="true" unique="false" upperBound="2" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EReference" name="link" derived="true" eType="#//Target"/>
<eStructuralFeatures x:type="e:EReference" name="links" derived="true" upperBound="2" eType="#//Target"/>
<eStructuralFeatures x:type="e:EReference" name="required" derived="true" lowerBound="1" eType="#//Target"/>
</eClassifiers>
<eClassifiers x:type="e:EClass" name="Target"/>
<eClassifiers x:type="e:EClass" name="Child" eSuperTypes="#//Target"/>
<eClassifiers x:type="e:EClass" name="Other"/>
</e:EPackage>"##).unwrap();
    let mut store = crate::stores::store::<Store>(&schema).unwrap();
    storage::atomic(&mut store, |store| {
        for (id, class) in [
            ("a", "Item"),
            ("b", "Target"),
            ("c", "Child"),
            ("d", "Other"),
        ] {
            store.create_element(&schema, id, class, "model", 0, 0)?;
        }
        Ok(())
    })
    .unwrap();
    (schema, store)
}
struct Response<'a> {
    schema: &'a Schema,
    value: Value,
}
impl DerivedValues for Response<'_> {
    fn schema_revision(&self) -> Option<&str> {
        Some(&self.schema.revision)
    }
    fn supports(&self, class: &str, _: &str) -> bool {
        class == "Item"
    }
    fn read(
        &self,
        _: &dyn ModelRead,
        _: &str,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<Value, String> {
        budget.charge()?;
        Ok(self.value.clone())
    }
}
fn state(
    schema: &Schema,
    store: &dyn ModelRead,
    name: &str,
    value: Value,
    steps: usize,
) -> Result<bool, String> {
    storage::is_set_with_derived(
        store,
        schema,
        "a",
        name,
        &Response { schema, value },
        &mut QueryBudget::new(steps),
    )
}
store_test! {
pub fn computed_collections_and_references_share_default_shape_and_nominal_rules() {
    let (schema, store) = fixture();
    let original = storage::export(&store).unwrap();
    for (name, value, expected) in [
        ("words", json!([]), false),
        ("words", json!(["a", "b"]), true),
        ("bag", json!(["a", "a"]), true),
        ("link", Value::Null, false),
        ("link", json!("b"), true),
        ("link", json!("c"), true),
        ("links", json!([]), false),
        ("links", json!(["b", "c"]), true),
        ("required", json!("c"), true),
    ] {
        assert_eq!(
            state(&schema, &store, name, value, 100).unwrap(),
            expected,
            "{name}"
        );
    }
    assert_eq!(storage::export(&store).unwrap(), original);
}
}
store_test! {
pub fn malformed_computed_values_reject_without_state_or_partial_results() {
    let (schema, store) = fixture();
    let original = storage::export(&store).unwrap();
    for (name, value) in [
        ("words", Value::Null),
        ("words", json!("a")),
        ("words", json!(["a", "a"])),
        ("words", json!(["a", "b", "c"])),
        ("words", json!([null])),
        ("words", json!([1])),
        ("words", json!([["nested"]])),
        ("link", json!([])),
        ("link", json!(1)),
        ("link", json!("missing")),
        ("link", json!("d")),
        ("links", json!(["b", "b"])),
        ("links", json!(["b", "missing"])),
        ("links", json!(["b", "d"])),
        ("links", json!(["b", null])),
        ("required", Value::Null),
    ] {
        assert!(
            state(&schema, &store, name, value.clone(), 100).is_err(),
            "{name}: {value}"
        );
    }
    // Two uniqueness comparisons and two retained-value releases are charged.
    for steps in 0..8 {
        assert!(
            state(&schema, &store, "words", json!(["a", "b"]), steps)
                .unwrap_err()
                .contains("budget")
        );
    }
    assert!(state(&schema, &store, "words", json!(["a", "b"]), 8).unwrap());
    assert_eq!(storage::export(&store).unwrap(), original);
}
}
store_test! {
pub fn computed_reference_rejects_foreign_target_identity() {
    let (schema, mut store) = fixture();
    let original = storage::export(&store).unwrap();
    let mut forged = original.clone();
    let target = forged.elements.iter_mut().find(|element| element.id == "b").unwrap();
    target.type_identity = "foreign".into();
    assert!(storage::atomic(&mut store, |store| store.put(target.clone())).is_err());
    assert_eq!(storage::export(&store).unwrap(), original);
    let raw = storage::ModelView::new(&forged).unwrap();
    assert!(
        state(&schema, &raw, "link", json!("b"), 100)
            .unwrap_err()
            .contains("foreign")
    );
}
}
