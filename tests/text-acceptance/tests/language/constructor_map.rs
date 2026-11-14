#[path = "common/constructor_map.rs"]
mod common;
use intersect_model::conversion::with_catalog_policies;
use intersect_model::core::model::Model;
use intersect_text::compiler::compile_bundle_with_conversions;
use serde_json::json;
#[test]
fn map_alias_bounds_keep_ordered_arguments_and_nominal_nested_values() {
    for wrapped in [false, true] {
        let f = common::fixture(wrapped);
        with_catalog_policies(&f.input.bundle,&f.ordinary,&f.applications,&f.constructors,|registry| {
            let package=compile_bundle_with_conversions(f.input.clone(),registry).map_err(|e|format!("{e:?}"))?;
            let schema=&package.schema;
            assert_eq!(schema.constructor_contracts.len(),2);
            assert!(schema.datatypes.values().all(|d|d.declaration()!=f.constructors[0].datatype.declaration()));
            if wrapped {assert!(schema.applied_conversion_contracts.is_empty());}
            let mut model=Model::default();
            let class=schema.class_key_for_type(&f.input.materialized_types[0])?;
            model.create(schema,"h",class,"memory:model",0,0)?;
            model.edit_with_conversions(schema,&[("h".into(),f.field.into(),f.value.clone())],registry)?;
            if !wrapped {
                model.edit_with_conversions(schema,&[("h".into(),"mapping".into(),f.map.clone())],registry)?;
                let duplicate=json!({"entries":[f.map["converted"]["value"]["entries"][0],f.map["converted"]["value"]["entries"][0]]});
                assert!(registry.applied().bind(&f.applications[0].application,duplicate).is_err());
            }
            let before=serde_json::to_value(&model).unwrap();
            let mut wrong=f.value.clone();
            let bounded=if wrapped {&mut wrong["value"]["items"][0]} else {&mut wrong};
            let unrelated = f.constructors[0].datatype.declaration();
            bounded["converted"]["value"]["items"][0]["application"]["declaration"]=json!(unrelated);
            bounded["converted"]["value"]["items"][0]["converted"]["datatype"]=json!(unrelated);
            assert!(model.edit_with_conversions(schema,&[("h".into(),"name".into(),json!("rollback")),("h".into(),f.field.into(),wrong)],registry).is_err());
            assert_eq!(serde_json::to_value(&model).unwrap(),before);
            model.validate_with_conversions(schema,registry)?;
            Ok(())
        }).unwrap();
        assert!(
            with_catalog_policies(
                &f.input.bundle,
                &f.ordinary,
                &f.applications,
                &f.constructors[..1],
                |_| Ok(())
            )
            .is_err()
        );
    }
    let f = common::fixture(false);
    let mut policies = f.constructors.clone();
    let list_id = intersect_model::definitions::resources::ResourceSet::new(&f.input.bundle)
        .unwrap()
        .resolve(&f.input.ecore, "#//Bounded")
        .unwrap();
    policies.push(
        intersect_model::conversion::constructor_contract(
            &f.input.bundle,
            &list_id,
            intersect_model::definitions::conversion::ProviderId {
                name: "ecore.list.constructor".into(),
                version: "1".into(),
            },
        )
        .unwrap(),
    );
    with_catalog_policies(&f.input.bundle, &[], &[], &policies, |registry| {
        assert_eq!(
            registry
                .constructors()
                .constructor_relation(policies[0].datatype.declaration(), &list_id)?,
            Some(intersect_model::definitions::conversion::ClassifierRelation::Unrelated)
        );
        Ok(())
    })
    .unwrap();
    let mut wire = serde_json::to_value(&f.applications).unwrap();
    wire[0]["application"]["arguments"]
        .as_array_mut()
        .unwrap()
        .reverse();
    wire[0]["conversion"]["members"]
        .as_array_mut()
        .unwrap()
        .reverse();
    wire[1]["application"]["arguments"][0] = wire[0]["application"].clone();
    wire[1]["conversion"]["members"][0]["value"]["contract"] = wire[0].clone();
    let swapped: Vec<intersect_model::definitions::conversion::AppliedConversionContract> =
        serde_json::from_value(wire).unwrap();
    assert!(
        with_catalog_policies(&f.input.bundle, &[], &swapped, &f.constructors, |_| Ok(()))
            .unwrap_err()
            .contains("violates generic bounds")
    );
}
