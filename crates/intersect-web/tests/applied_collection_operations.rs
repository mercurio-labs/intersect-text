#[path = "../../../tests/text-acceptance/tests/language/common/applied_model.rs"]
mod common;
use serde_json::{Value, json};
fn call(request: Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
#[test]
fn applied_list_map_and_nested_operations_keep_domains_and_publish_checked_values() {
    let (input, contracts) = common::input();
    let compiled = call(
        json!({"method":"compile_bundle","compilation":input,"applied_conversions":contracts}),
    )
    .unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let step = |n: usize, operation: Value, collection: Option<&Value>| {
        let mut request = json!({"method":"collection/step","package":package,"application":contracts[n].application,"operation":operation});
        if let Some(value) = collection {
            request["collection"] = value.clone();
        }
        call(request)
    };
    let a = common::nominal(&input, 0, json!("a"));
    let b = common::nominal(&input, 0, json!("b"));
    let number = common::nominal(&input, 1, json!(42));
    let empty = step(0, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    assert_eq!(empty["kind"], "converted_application_v1");
    let one =
        step(0, json!({"kind":"insert","index":0,"item":a}), Some(&empty)).unwrap()["collection"]
            .clone();
    let two =
        step(0, json!({"kind":"insert","index":1,"item":b}), Some(&one)).unwrap()["collection"]
            .clone();
    let moved = step(
        0,
        json!({"kind":"move","new_position":0,"old_position":1}),
        Some(&two),
    )
    .unwrap();
    assert_eq!(moved["result"], json!({"present":true,"value":b}));
    let changed = step(
        0,
        json!({"kind":"set","index":0,"item":a}),
        Some(&moved["collection"]),
    )
    .unwrap();
    assert_eq!(changed["result"]["value"], b);
    assert_eq!(
        step(
            0,
            json!({"kind":"remove_at","index":0}),
            Some(&changed["collection"])
        )
        .unwrap()["result"]["value"],
        a
    );
    assert_eq!(
        step(0, json!({"kind":"clear"}), Some(&two)).unwrap()["collection"],
        empty
    );
    for operation in [
        json!({"kind":"insert","index":99,"item":a}),
        json!({"kind":"set","index":0,"item":number}),
        json!({"kind":"get","key":a}),
    ] {
        assert!(step(0, operation, Some(&one)).is_err());
    }
    assert!(step(1, json!({"kind":"clear"}), Some(&one)).is_err());
    let numeric = step(1, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    let numeric = step(
        1,
        json!({"kind":"insert","index":0,"item":number}),
        Some(&numeric),
    )
    .unwrap()["collection"]
        .clone();
    let nested = step(2, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    let nested = step(
        2,
        json!({"kind":"insert","index":0,"item":one}),
        Some(&nested),
    )
    .unwrap()["collection"]
        .clone();
    assert!(
        step(
            2,
            json!({"kind":"set","index":0,"item":numeric}),
            Some(&nested)
        )
        .is_err()
    );
    let map = step(3, json!({"kind":"empty"}), None).unwrap()["collection"].clone();
    let inserted = step(3, json!({"kind":"put","key":a,"item":number}), Some(&map)).unwrap();
    assert_eq!(inserted["result"], json!({"present":false,"value":null}));
    let map = inserted["collection"].clone();
    assert_eq!(
        step(3, json!({"kind":"get","key":a}), Some(&map)).unwrap()["result"]["value"],
        number
    );
    assert_eq!(
        step(3, json!({"kind":"put","key":a,"item":number}), Some(&map)).unwrap()["result"]["value"],
        number
    );
    assert_eq!(
        step(3, json!({"kind":"remove_key","key":a}), Some(&map)).unwrap()["result"]["value"],
        number
    );
    assert_eq!(
        step(3, json!({"kind":"get","key":b}), Some(&map)).unwrap()["result"]["present"],
        false
    );
    assert!(
        step(
            3,
            json!({"kind":"put","key":number,"item":number}),
            Some(&map)
        )
        .is_err()
    );
    assert!(step(3, json!({"kind":"insert","index":0,"item":a}), Some(&map)).is_err());
    assert!(call(json!({"method":"collection/step","package":package,"application":contracts[0].application,"datatype":contracts[0].conversion.datatype,"operation":{"kind":"empty"}})).is_err());
    let mut stale = json!(contracts[0].application);
    stale["declaration"]["source"]["sha256"] = json!("0".repeat(64));
    assert!(call(json!({"method":"collection/step","package":package,"application":stale,"operation":{"kind":"empty"}})).is_err());
    let mut wrong_provider = two.clone();
    wrong_provider["converted"]["provider"]["version"] = json!("wrong");
    assert!(step(0, json!({"kind":"clear"}), Some(&wrong_provider)).is_err());
    let mut stale_package = package.clone();
    stale_package["schema"]["applied_conversion_contracts"][0]["conversion"]["provider"]["version"] =
        json!("wrong");
    assert!(call(json!({"method":"collection/step","package":stale_package,"application":contracts[0].application,"operation":{"kind":"empty"}})).is_err());
    let mut unselected = json!(contracts[0].application);
    unselected["arguments"] = json!([contracts[3].application]);
    let missing=call(json!({"method":"collection/step","package":package,"application":unselected,"operation":{"kind":"empty"}})).unwrap_err();
    assert!(
        missing.contains("Missing applied collection provider contract"),
        "{missing}"
    );
    let evaluated=call(json!({"method":"evaluate","package":package,"sources":[{"uri":"one.model","text":"holder sample"}]})).unwrap();
    let script = |method: &str, args: Value| {
        let mut args = args;
        args["method"] = json!(method);
        args["revision"] = json!("applied-operations:1");
        call(args)
    };
    script(
        "script/begin",
        json!({"package":package,"model":evaluated["model"]}),
    )
    .unwrap();
    let id = &evaluated["model"]["elements"][0]["id"];
    for (feature, value) in [
        ("texts", &two),
        ("numbers", &numeric),
        ("nested", &nested),
        ("mapping", &map),
    ] {
        script(
            "script/set",
            json!({"id":id,"feature":feature,"value":value}),
        )
        .unwrap();
    }
    let prepared = script("script/commit", json!({})).unwrap();
    assert_eq!(prepared["model"]["elements"][0]["values"]["texts"], two);
    assert_eq!(prepared["model"]["elements"][0]["values"]["mapping"], map);
    assert_eq!(
        step(0, json!({"kind":"clear"}), Some(&two)).unwrap()["collection"],
        empty
    );
}
