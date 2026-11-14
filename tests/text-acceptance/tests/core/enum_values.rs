use intersect_model::core::model::Model;
use intersect_model::definitions::schema::Schema;
use serde_json::{Value, json};

fn source(extra: &str) -> String {
    format!(
        "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='test' nsURI='urn:test'><eClassifiers xsi:type='e:EEnum' name='EInt'><eLiterals name='Primary' value='7' literal='primary'/><eLiterals name='Alias' value='7' literal='alias'/></eClassifiers><eClassifiers xsi:type='e:EClass' name='Item'><eStructuralFeatures xsi:type='e:EAttribute' name='mode' eType='#//EInt' lowerBound='1'/><eStructuralFeatures xsi:type='e:EAttribute' name='selected' eType='#//EInt' defaultValueLiteral='alias'/><eStructuralFeatures xsi:type='e:EAttribute' name='modes' eType='#//EInt' upperBound='-1'/><eStructuralFeatures xsi:type='e:EAttribute' name='count' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EInt'/>{extra}</eClassifiers></e:EPackage>"
    )
}
fn value(schema: &Schema, spelling: &str) -> Value {
    serde_json::to_value(schema.enumerations["EInt"].parse(spelling).unwrap()).unwrap()
}

#[test]
fn enum_defaults_and_alias_lists_round_trip_through_shared_model_validation() {
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&source(""))
            .unwrap();
    let mut model = Model::default();
    model.create(&schema, "item", "Item", "api", 0, 0).unwrap();
    model.validate(&schema).unwrap();
    assert_eq!(model.elements[0].values["mode"], value(&schema, "primary"));
    assert_eq!(
        model.elements[0].values["selected"],
        value(&schema, "alias")
    );
    assert_eq!(model.elements[0].values["count"], json!(0));
    model
        .edit(
            &schema,
            &[(
                "item".into(),
                "modes".into(),
                json!([value(&schema, "primary"), value(&schema, "alias")]),
            )],
        )
        .unwrap();
    let restored: Model = serde_json::from_str(&serde_json::to_string(&model).unwrap()).unwrap();
    restored.validate(&schema).unwrap();
    assert_eq!(restored, model);
    let schema_roundtrip: Schema =
        serde_json::from_str(&serde_json::to_string(&schema).unwrap()).unwrap();
    restored.validate(&schema_roundtrip).unwrap();
}

#[test]
fn bad_enum_assignments_roll_back_entire_edit_batches() {
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&source(""))
            .unwrap();
    let changed = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        &source("").replacen("value='7'", "value='8'", 1),
    )
    .unwrap();
    let mut model = Model::default();
    model.create(&schema, "item", "Item", "api", 0, 0).unwrap();
    let before = model.clone();
    for invalid in [
        json!("primary"),
        json!(7),
        Value::Null,
        value(&changed, "primary"),
        json!({"kind":"enum"}),
    ] {
        assert!(
            model
                .edit(
                    &schema,
                    &[
                        ("item".into(), "count".into(), json!(12)),
                        ("item".into(), "mode".into(), invalid)
                    ]
                )
                .is_err()
        );
        assert_eq!(model, before);
    }
    assert!(
        model
            .edit(
                &schema,
                &[(
                    "item".into(),
                    "modes".into(),
                    json!([value(&schema, "alias"), value(&schema, "alias")])
                )]
            )
            .is_err()
    );
    assert_eq!(model, before);
    assert!(
        model
            .set_with_opposite(&schema, "item", "mode", json!("alias"))
            .is_err()
    );
    assert_eq!(model, before);
    assert!(
        model
            .edit(
                &schema,
                &[("item".into(), "count".into(), value(&schema, "primary"))]
            )
            .is_err()
    );
    assert_eq!(model, before);
}

#[test]
fn enum_defaults_and_feature_kind_errors_reject_at_compilation() {
    let text = source("");
    for invalid in [
        text.replace("defaultValueLiteral='alias'", "defaultValueLiteral='Alias'"),
        text.replace("defaultValueLiteral='alias'", "defaultValueLiteral='7'"),
        text.replace("name='mode' eType='#//EInt'", "name='mode' eType='#//Item'"),
        source("<eStructuralFeatures xsi:type='e:EReference' name='wrong' eType='#//EInt'/>"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&invalid)
                .is_err()
        );
    }
}

#[test]
fn empty_enum_defaults_are_absent_and_required_values_remain_invalid() {
    let text = source("").replace("<eLiterals name='Primary' value='7' literal='primary'/><eLiterals name='Alias' value='7' literal='alias'/>", "").replace(" defaultValueLiteral='alias'", "");
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&text)
            .unwrap();
    let mut model = Model::default();
    model.create(&schema, "item", "Item", "api", 0, 0).unwrap();
    assert_eq!(model.elements[0].values["mode"], Value::Null);
    assert!(
        model
            .validate(&schema)
            .unwrap_err()
            .contains("Multiplicity")
    );
    let optional = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        &text.replace(" lowerBound='1'", ""),
    )
    .unwrap();
    let mut model = Model::default();
    model
        .create(&optional, "item", "Item", "api", 0, 0)
        .unwrap();
    model.validate(&optional).unwrap();
}

#[test]
fn enum_named_builtin_does_not_enter_legacy_terminal_or_derivation_paths() {
    let text = source("");
    let semantics = r#"rcore 1; package emptyRules version "1";"#;
    let grammar = "grammar Test import \"urn:test\" as t Item returns t::Item: 'item' mode=INT;";
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            &text, grammar, semantics
        )
        .is_err()
    );
}

#[test]
fn semantic_state_compares_exact_enum_literal_defaults_not_numeric_aliases() {
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&source(""))
            .unwrap();
    let mut model = Model::default();
    model
        .create(&schema, "item", "Item", "model", 0, 0)
        .unwrap();
    assert!(!model.is_set(&schema, "item", "mode").unwrap());
    assert!(!model.is_set(&schema, "item", "selected").unwrap());
    model
        .edit(
            &schema,
            &[("item".into(), "selected".into(), value(&schema, "primary"))],
        )
        .unwrap();
    assert!(model.is_set(&schema, "item", "selected").unwrap());
    model
        .edit(
            &schema,
            &[("item".into(), "selected".into(), value(&schema, "alias"))],
        )
        .unwrap();
    assert!(!model.is_set(&schema, "item", "selected").unwrap());
}
