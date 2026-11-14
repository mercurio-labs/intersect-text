#[path = "../../../tests/text-acceptance/tests/language/common/constructor_map.rs"]
mod common;
use serde_json::{Value, json};
fn dispatch(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
#[test]
fn map_constructor_packages_rebuild_for_model_and_map_operations() {
    for wrapped in [false, true] {
        let f = common::fixture(wrapped);
        let request = json!({"method":"compile_bundle","compilation":f.input,"conversions":f.ordinary,"applied_conversions":f.applications,"constructors":f.constructors});
        let compiled = dispatch(request.clone());
        assert_eq!(compiled["ok"], true, "{compiled}");
        let package = &compiled["package"];
        assert_eq!(
            package["schema"]["constructor_contracts"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let result = dispatch(
            json!({"method":"ecore/materialize","bundle":f.input.bundle,"types":f.input.materialized_types,"conversions":f.ordinary,"applied_conversions":f.applications,"constructors":f.constructors,"instances":[{"id":"h","type":f.input.materialized_types[0]}],"edits":[["h",f.field,f.value]]}),
        );
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(result["model"]["elements"][0]["values"][f.field], f.value);
        if !wrapped {
            let application = &f.applications[0].application;
            let empty = dispatch(
                json!({"method":"collection/step","package":package,"application":application,"operation":{"kind":"empty"}}),
            );
            let key = &f.map["converted"]["value"]["entries"][0]["key"];
            let value = &f.map["converted"]["value"]["entries"][0]["value"];
            let put = dispatch(
                json!({"method":"collection/step","package":package,"application":application,"collection":empty["collection"],"operation":{"kind":"put","key":key,"item":value}}),
            );
            assert_eq!(put["collection"], f.map);
            let get = dispatch(
                json!({"method":"collection/step","package":package,"application":application,"collection":put["collection"],"operation":{"kind":"get","key":key}}),
            );
            assert_eq!(get["result"]["value"], *value);
            let removed = dispatch(
                json!({"method":"collection/step","package":package,"application":application,"collection":put["collection"],"operation":{"kind":"remove_key","key":key}}),
            );
            assert_eq!(removed["collection"], empty["collection"]);
        }
        let evaluated = dispatch(
            json!({"method":"evaluate","package":package,"sources":[{"uri":"memory:model","text":"holder one"}]}),
        );
        assert_eq!(evaluated["model"]["elements"].as_array().unwrap().len(), 1);
        let mut missing = request.clone();
        missing["constructors"] = json!([]);
        assert_eq!(dispatch(missing)["ok"], false);
    }
}
