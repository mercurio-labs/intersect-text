use super::Store;
use intersect_model::core::storage::{self, ModelRead, ModelStore};
use intersect_model::definitions::dynamic::SchemaBuilder;
use serde_json::{Value, json};

store_test! {
pub fn same_object_links_between_distinct_references_match_independent_emf() {
    let observations: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-structure/reference-identity.json"
    ))
    .unwrap();
    let mut checked = 0;
    for case in observations["cases"].as_array().unwrap() {
        let many = case["many"] == true;
        let unsettable = case["unsettable"] == true;
        let mut builder = SchemaBuilder::new("identity", "urn:identity");
        let node = builder.class("Node").unwrap();
        let out = node
            .reference("out", &node)
            .bounds(0, if many { -1 } else { 1 })
            .unsettable(unsettable)
            .build()
            .unwrap();
        let back = node
            .reference("back", &node)
            .unsettable(unsettable)
            .optional()
            .unwrap();
        builder.opposites(&out, &back).unwrap();
        let dynamic = builder.build().unwrap();
        let schema = dynamic.as_schema();
        let mut store = crate::stores::store::<Store>(schema).unwrap();
        storage::atomic(&mut store, |s| {
            for id in ["a", "b"] {
                s.create_element(schema, id, "Node", "model", 0, 0)?;
            }
            Ok(())
        })
        .unwrap();
        for step in case["steps"].as_array().unwrap() {
            let edit = match step["operation"].as_str().unwrap() {
                "fresh" => None,
                "bind-other" => Some(("a", "b")),
                "bind-self" => Some(("a", "a")),
                "displace-self" => Some(("b", "a")),
                other => panic!("Unknown operation {other}"),
            };
            if let Some((id, target)) = edit {
                storage::edit(
                    &mut store,
                    schema,
                    &[(
                        id.into(),
                        "out".into(),
                        if many { json!([target]) } else { json!(target) },
                    )],
                )
                .unwrap();
            }
            let mut states = serde_json::Map::new();
            for id in ["a", "b"] {
                states.insert(id.into(),json!({"out":store.value(id,"out").unwrap(),"back":store.value(id,"back").unwrap(),"out_set":storage::is_set(&store,schema,id,"out").unwrap(),"back_set":storage::is_set(&store,schema,id,"back").unwrap()}));
            }
            assert_eq!(Value::Object(states), step["objects"], "{case}");
            checked += 1;
        }
    }
    assert_eq!(checked, 16);
}
}
