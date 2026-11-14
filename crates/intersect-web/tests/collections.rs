use serde_json::{Value, json};
const RCORE: &str = include_str!("../../../fixtures/collections/collections-structure.rcore");
fn request() -> Value {
    json!({"method":"compile_rcore", "rcore":RCORE, "xtext":include_str!("../../../fixtures/collections/collections.xtext"), "rcore_behavior":include_str!("../../../fixtures/collections/collections-behavior.rcore"), "conversions":serde_json::from_str::<Value>(include_str!("../../../fixtures/collections/conversions-rcore.json")).unwrap()})
}
fn dispatch(request: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn converted(contract: &Value, value: Value) -> Value {
    json!({"kind":"converted","datatype":contract["datatype"],"provider":contract["provider"],"value":value})
}
#[test]
fn collection_catalog_compiles_defaults_operations_nested_edits_and_export() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let contracts = &input["conversions"];
    let operation = &package["schema"]["classes"]["Catalog"]["operations"][0];
    assert_eq!(
        operation["result"]["target"],
        json!({"kind":"datatype","name":"__standard_EEList"})
    );
    assert_eq!(
        operation["parameters"][0]["value"]["target"],
        json!({"kind":"datatype","name":"__standard_EMap"})
    );
    let evaluated = dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"sample.collections","text":"catalog example"}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    let id = &model["elements"][0]["id"];
    for feature in ["list", "map", "localList", "localMap"] {
        assert!(model["elements"][0]["values"][feature].is_null());
    }
    let list = converted(&contracts[0], json!({"items":[1,2,1]}));
    let map = converted(
        &contracts[1],
        json!({"entries":[{"key":"nested","value":list}]}),
    );
    let count = json!({"kind":"datatype","datatype":contracts[3]["members"][1]["value"]["declaration"],"value":7});
    let local_map = converted(
        &contracts[3],
        json!({"entries":[{"key":"count","value":count}]}),
    );
    dispatch(
        &json!({"method":"script/begin","package":package,"model":model,"revision":"collections"}),
    )
    .unwrap();
    for (feature, value) in [
        ("list", list.clone()),
        ("map", map.clone()),
        ("localMap", local_map),
    ] {
        dispatch(&json!({"method":"script/set","revision":"collections","id":id,"feature":feature,"value":value})).unwrap();
    }
    let saved =
        dispatch(&json!({"method":"script/commit","revision":"collections"})).unwrap()["model"]
            .clone();
    let mut bad_nested = map.clone();
    bad_nested["value"]["entries"][0]["value"]["provider"]["version"] = json!("2");
    let duplicate = converted(
        &contracts[1],
        json!({"entries":[{"key":"a","value":null},{"key":"a","value":null}]}),
    );
    for bad in [bad_nested, duplicate, list] {
        dispatch(
            &json!({"method":"script/begin","package":package,"model":saved,"revision":"bad"}),
        )
        .unwrap();
        assert!(
            dispatch(
                &json!({"method":"script/set","revision":"bad","id":id,"feature":"map","value":bad})
            )
            .is_err()
        );
        assert!(dispatch(&json!({"method":"script/commit","revision":"bad"})).is_err());
    }
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.collections","release":"1","display_name":"Collections","description":"Exact collection policies"});
    let exported = dispatch(&export).unwrap();
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    assert_eq!(
        loaded["schema"]["conversion_contracts"],
        package["schema"]["conversion_contracts"]
    );
    dispatch(&json!({"method":"script/begin","package":loaded,"model":saved,"revision":"reload"}))
        .unwrap();
    dispatch(&json!({"method":"script/commit","revision":"reload"})).unwrap();
}
#[test]
fn collection_catalog_rejects_missing_wrong_nested_and_member_contracts() {
    let input = request();
    let mut cases = vec![];
    let mut missing = input.clone();
    missing["conversions"].as_array_mut().unwrap().remove(0);
    cases.push(missing);
    let mut version = input.clone();
    version["conversions"][1]["members"][1]["value"]["contract"]["provider"]["version"] =
        json!("2");
    cases.push(version);
    let mut arity = input.clone();
    arity["conversions"][0]["members"] = json!([]);
    cases.push(arity);
    let mut stale = input.clone();
    stale["conversions"][2]["members"][0]["value"]["declaration"]["source"]["sha256"] =
        json!("0".repeat(64));
    cases.push(stale);
    let mut wrong = input.clone();
    wrong["conversions"][0]["provider"]["name"] = json!("ecore.map");
    cases.push(wrong);
    for bad in cases {
        let result = dispatch(&bad);
        assert!(
            result.as_ref().map_or(true, |value| value["ok"] == false),
            "{result:?}"
        );
    }
}

#[test]
fn collection_steps_validate_arguments_and_preserve_optional_results() {
    let input = request();
    let package = dispatch(&input).unwrap()["package"].clone();
    let list_type = &input["conversions"][0]["datatype"];
    let map_type = &input["conversions"][1]["datatype"];
    let step = |datatype: &Value, operation: Value, collection: Option<&Value>| {
        let mut request = json!({"method":"collection/step","package":package,"datatype":datatype,"operation":operation});
        if let Some(value) = collection {
            request["collection"] = value.clone();
        }
        dispatch(&request)
    };
    let mut list = step(list_type, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    for (operation, expected) in [
        (json!({"kind":"insert","index":0,"item":1}), json!([1])),
        (json!({"kind":"insert","index":1,"item":2}), json!([1, 2])),
        (
            json!({"kind":"move","new_position":0,"old_position":1}),
            json!([2, 1]),
        ),
        (json!({"kind":"set","index":0,"item":3}), json!([3, 1])),
        (json!({"kind":"remove_at","index":1}), json!([3])),
    ] {
        let result = step(list_type, operation, Some(&list)).unwrap();
        list = result["collection"].clone();
        assert_eq!(list["value"]["items"], expected);
    }
    let mut map = step(map_type, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    let result = step(
        map_type,
        json!({"kind":"put","key":"a","item":null}),
        Some(&map),
    )
    .unwrap();
    assert_eq!(result["result"], json!({"present":false,"value":null}));
    map = result["collection"].clone();
    assert_eq!(
        step(map_type, json!({"kind":"get","key":"a"}), Some(&map)).unwrap()["result"],
        json!({"present":true,"value":null})
    );
    assert_eq!(
        step(map_type, json!({"kind":"get","key":"missing"}), Some(&map)).unwrap()["result"]["present"],
        false
    );
    let result = step(
        map_type,
        json!({"kind":"put","key":"a","item":list}),
        Some(&map),
    )
    .unwrap();
    assert_eq!(result["result"], json!({"present":true,"value":null}));
    map = result["collection"].clone();
    let result = step(map_type, json!({"kind":"remove_key","key":"a"}), Some(&map)).unwrap();
    assert_eq!(result["result"], json!({"present":true,"value":list}));
    assert_eq!(
        step(map_type, json!({"kind":"clear"}), Some(&map)).unwrap()["collection"]["value"],
        json!({"entries":[]})
    );
    for operation in [
        json!({"kind":"empty"}),
        json!({"kind":"clear","extra":1}),
        json!({"kind":"insert","index":-1,"item":2}),
        json!({"kind":"insert","index":0}),
        json!({"kind":"insert","index":99,"item":1}),
        json!({"kind":"get","key":1}),
        json!({"kind":"unknown"}),
    ] {
        assert!(step(list_type, operation, Some(&list)).is_err());
    }
    assert!(step(map_type, json!({"kind":"put","key":"a"}), Some(&map)).is_err());
    assert!(step(list_type, json!({"kind":"clear"}), None).is_err());
    assert!(step(map_type, json!({"kind":"clear"}), Some(&list)).is_err());
    let mut stale = list.clone();
    stale["provider"]["version"] = json!("2");
    assert!(step(list_type, json!({"kind":"clear"}), Some(&stale)).is_err());
    let mut unknown = list_type.clone();
    unknown["source"]["sha256"] = json!("0".repeat(64));
    assert!(step(&unknown, json!({"kind":"empty"}), None).is_err());
}

#[test]
fn rcore_behavior_collection_reads_keep_standard_and_declared_types_distinct() {
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    let input = request();
    for (body, ok) in [
        (
            "fn same(a: c.Catalog,b: c.Catalog) -> Bool = a.list == b.list;",
            true,
        ),
        (
            "fn wrong(a: c.Catalog) -> Bool = a.list == a.localList;",
            false,
        ),
        ("fn wrong(a: c.Catalog) -> Bool = a.list == a.map;", false),
    ] {
        let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, RCORE);
        let mut semantic = DefinitionSource::new(
            "checks-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!(
                "rcore 1; package checks version \"1\"; import model \"urn:intersect:collections\" as c; {body}"
            ),
        );
        semantic.dependencies.push(model.id.clone());
        let checked = dispatch(&json!({"method":"rcore_behavior/check","entry":semantic.id,"bundle":{"version":1,"sources":[model,semantic],"namespaces":[]},"conversions":input["conversions"]})).unwrap();
        assert_eq!(checked["ok"], ok, "{checked}");
        assert_eq!(checked["executable"], false);
    }
}
