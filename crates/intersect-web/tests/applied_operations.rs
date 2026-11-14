use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use serde_json::{Value, json};
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
#[test]
fn operations_reconstruct_bound_context_and_reject_wrong_nested_values() {
    let source = DefinitionSource::new(
        "memory:applied-operations",
        SourceKind::Ecore,
        include_str!("../../../fixtures/ecore-generics/applied-provider-operations.ecore"),
    );
    let (source, ids) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| json!(ids[&vec![n]]);
    let ty = |n, args: Vec<Value>| json!({"kind":"classifier","declaration":id(n),"arguments":args,"raw":false});
    let rgb = json!({"datatype":id(0),"provider":{"name":"example.rgb","version":"1"},"serializable":true});
    let mut alias = rgb.clone();
    alias["datatype"] = id(1);
    let applied = json!({"application":ty(2,vec![ty(1,vec![])]),"conversion":{"datatype":id(2),
        "provider":{"name":"ecore.list","version":"1"},"serializable":false,
        "members":[{"nullable":false,"value":{"kind":"converted","contract":alias}}]}});
    let wrapper = json!({"datatype":id(4),"provider":{"name":"ecore.list","version":"1"},"serializable":false,
        "members":[{"nullable":false,"value":{"kind":"applied","contract":applied}}]});
    let feature = json!(ids[&vec![6, 0]]);
    let map = json!({"datatype":id(7),"provider":{"name":"ecore.feature-map","version":"1"},"serializable":false,
        "declarations":[feature],"members":[{"nullable":false,"value":{"kind":"converted","contract":wrapper}}]});
    let mut entry = map.clone();
    entry["datatype"] = id(8);
    entry["provider"]["name"] = json!("ecore.feature-entry");
    let mut grammar = DefinitionSource::new(
        "memory:operations.xtext",
        SourceKind::Xtext,
        "grammar example.Operations import \"urn:colors\" as c\nOnlyWrapped returns c::OnlyWrapped: 'holder';",
    );
    grammar.dependencies.push(source.id.clone());
    let mut rcore_behavior = DefinitionSource::new(
        "memory:operations-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    rcore_behavior.dependencies.push(source.id.clone());
    let nested = json!({"application":ty(3,vec![applied["application"].clone()]),"conversion":{
        "datatype":id(3),"provider":{"name":"ecore.list","version":"1"},"serializable":false,
        "members":[{"nullable":false,"value":{"kind":"applied","contract":applied}}]}});
    let compiled=call(json!({"method":"compile_bundle","compilation":{
        "bundle":{"version":1,"sources":[source,grammar,rcore_behavior],"namespaces":[]},"ecore":source.id,"namespace":"urn:colors",
        "xtext":grammar.id,"rcore_behavior":rcore_behavior.id,"materialized_types":[ty(5,vec![]),ty(6,vec![]),ty(9,vec![])]},
        "conversions":[map,wrapper,entry,alias,rgb],"applied_conversions":[applied,nested]})).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let value = json!({"kind":"converted_application_v1","application":applied["application"],"converted":{
        "kind":"converted","datatype":id(2),"provider":applied["conversion"]["provider"],"value":{"items":[{
        "kind":"converted","datatype":id(1),"provider":alias["provider"],"value":{"red":1,"green":2,"blue":3}}]}}});
    let direct=call(json!({"method":"collection/step","package":package,"application":applied["application"],"operation":{"kind":"empty"}})).unwrap();
    let built=call(json!({"method":"collection/step","package":package,"application":applied["application"],
        "collection":direct["collection"],"operation":{"kind":"insert","index":0,"item":value["converted"]["value"]["items"][0]}})).unwrap();
    assert_eq!(built["collection"], value);
    let collection = |operation: Value, collection: Option<&Value>| {
        let mut request = json!({"method":"collection/step","package":package,"datatype":id(4),"operation":operation});
        if let Some(value) = collection {
            request["collection"] = value.clone();
        }
        call(request)
    };
    let empty = collection(json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    let inserted = collection(
        json!({"kind":"insert","index":0,"item":value}),
        Some(&empty),
    )
    .unwrap()["collection"]
        .clone();
    let replaced = collection(
        json!({"kind":"set","index":0,"item":value}),
        Some(&inserted),
    )
    .unwrap();
    assert_eq!(replaced["collection"], inserted);
    let mut wrong = value.clone();
    wrong["converted"]["value"]["items"][0]["datatype"] = id(0);
    assert!(
        collection(
            json!({"kind":"insert","index":1,"item":wrong}),
            Some(&inserted)
        )
        .is_err()
    );
    assert_eq!(
        collection(
            json!({"kind":"set","index":0,"item":value}),
            Some(&inserted)
        )
        .unwrap()["collection"],
        inserted
    );
    let step = |datatype: Value, operation: Value, input: Option<&Value>| {
        let mut request = json!({"method":"feature-map/step","package":package,"datatype":datatype,"operation":operation});
        if let Some(value) = input {
            request["value"] = value.clone();
        }
        call(request)
    };
    let entry = step(
        id(8),
        json!({"kind":"entry","feature":feature,"value":inserted}),
        None,
    )
    .unwrap()["value"]
        .clone();
    let empty = step(id(7), json!({"kind":"empty"}), None).unwrap()["value"].clone();
    let mapping = step(
        id(7),
        json!({"kind":"insert","index":0,"entry":entry["value"]}),
        Some(&empty),
    )
    .unwrap()["value"]
        .clone();
    let replaced = step(
        id(7),
        json!({"kind":"set","index":0,"entry":entry["value"]}),
        Some(&mapping),
    )
    .unwrap();
    assert_eq!(replaced["value"], mapping);
    let mut bad = entry["value"].clone();
    bad["value"]["value"]["items"][0] = wrong;
    assert!(
        step(
            id(7),
            json!({"kind":"set","index":0,"entry":bad}),
            Some(&mapping)
        )
        .is_err()
    );
    assert_eq!(
        step(
            id(7),
            json!({"kind":"values","feature":feature}),
            Some(&mapping)
        )
        .unwrap()["result"],
        json!([inserted])
    );
}
