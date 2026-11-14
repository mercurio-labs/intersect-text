use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use serde_json::{Value, json};
fn dispatch(request: &Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn input() -> Value {
    let source = DefinitionSource::new(
        "memory:method-providers",
        SourceKind::Ecore,
        include_str!("../../../fixtures/ecore-generics/method-provider-bounds.ecore"),
    );
    let (source, ids) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |path: &[usize]| json!(ids[path]);
    let ty = |n, args: Vec<Value>| json!({"kind":"classifier","declaration":id(&[n]),"arguments":args,"raw":false});
    let bundle = json!({"version":1,"sources":[source],"namespaces":[]});
    let constructors=[2,3].map(|n|dispatch(&json!({"method":"conversion/constructor","bundle":bundle,"datatype":id(&[n]),"provider":{"name":"ecore.list.constructor","version":"1"}}))["contract"].clone());
    let conversions=[5,6].map(|n|json!({"datatype":id(&[n]),"provider":{"name":"example.rgb","version":"1"},"serializable":true}));
    json!({"method":"ecore/materialize","bundle":bundle,"types":[ty(7,vec![])],"conversions":conversions,"constructors":constructors,"operation_applications":[{"receiver":ty(7,vec![]),"operation":id(&[7,1]),"arguments":[ty(3,vec![ty(0,vec![])])]},{"receiver":ty(7,vec![]),"operation":id(&[7,2]),"arguments":[ty(6,vec![])]}],"instances":[{"id":"o","type":ty(7,vec![])}],"edits":[["o","name","checked"]]})
}
#[test]
fn method_only_dependencies_replay_without_becoming_schema_dependencies() {
    let request = input();
    let result = dispatch(&request);
    assert_eq!(result["ok"], true, "{result}");
    assert!(result["schema"].get("constructor_contracts").is_none());
    assert!(result["schema"].get("conversion_contracts").is_none());
    assert_eq!(result["model"]["elements"][0]["values"]["name"], "checked");
    assert_eq!(
        result["operation_dependencies"]["constructors"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        result["operation_dependencies"]["conversions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for (index, application) in request["operation_applications"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let standalone = dispatch(
            &json!({"method":"ecore/generic-operation-types","bundle":request["bundle"],"conversions":request["conversions"],"constructors":request["constructors"],"receiver":application["receiver"],"operation":application["operation"],"arguments":application["arguments"]}),
        );
        assert_eq!(
            result["operation_applications"][index]["report"],
            standalone["report"]
        );
        assert_eq!(result["operation_applications"][index]["executable"], false);
    }
    let replay: Value =
        serde_json::from_str(&serde_json::to_string(&result["operation_replay"]).unwrap()).unwrap();
    let replayed = dispatch(&replay);
    for field in [
        "schema",
        "operation_applications",
        "operation_dependencies",
        "operation_replay",
    ] {
        assert_eq!(replayed[field], result[field]);
    }
    let mut reversed = request.clone();
    reversed["constructors"].as_array_mut().unwrap().reverse();
    reversed["conversions"].as_array_mut().unwrap().reverse();
    assert_eq!(dispatch(&reversed), result);
    let source: DefinitionSource =
        serde_json::from_value(request["bundle"]["sources"][0].clone()).unwrap();
    let unused_type =
        intersect_text_acceptance::migration_fixture::named_declaration_fixture(&source, "Unused");
    let number_type =
        intersect_text_acceptance::migration_fixture::named_declaration_fixture(&source, "Number");
    let mut unused = request.clone();
    let extra = dispatch(
        &json!({"method":"conversion/constructor","bundle":request["bundle"],"datatype":unused_type,"provider":{"name":"ecore.list.constructor","version":"1"}}),
    );
    unused["constructors"]
        .as_array_mut()
        .unwrap()
        .push(extra["contract"].clone());
    assert!(
        intersect_web::dispatch(&unused.to_string())
            .unwrap_err()
            .contains("Unused constructor")
    );
    for field in ["constructors", "conversions"] {
        let mut missing = replay.clone();
        missing[field] = json!([]);
        assert_eq!(dispatch(&missing)["ok"], false);
        let mut stale = replay.clone();
        stale[field][0]["provider"]["version"] = "stale".into();
        assert!(intersect_web::dispatch(&stale.to_string()).is_err());
    }
    let mut bad = replay.clone();
    bad["operation_applications"][0]["arguments"][0]["arguments"][0]["declaration"] =
        json!(number_type);
    assert_eq!(dispatch(&bad)["ok"], false);
    let mut stale = replay;
    stale["operation_applications"][0]["operation"]["source"]["sha256"] = "0".repeat(64).into();
    assert_eq!(dispatch(&stale)["ok"], false);
}
