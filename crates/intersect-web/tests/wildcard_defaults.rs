#[path = "../../../tests/text-acceptance/tests/language/common/wildcard_defaults.rs"]
mod common;
use serde_json::{Value, json};
fn dispatch(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
#[test]
fn default_consumers_rebuild_exact_policies_and_reject_forged_package_context() {
    for diamond in [false, true] {
        let fixture = common::fixture(diamond);
        assert!(fixture.selected < fixture.applications.len());
        let request = json!({"method":"compile_bundle", "compilation":fixture.input,
            "applied_conversions":fixture.applications,"constructors":fixture.constructors});
        let compiled = dispatch(request.clone());
        assert_eq!(compiled["ok"], true, "{compiled}");
        let package = &compiled["package"];
        let sources = json!([{"uri":"memory:defaults.model","text":"root Demo"}]);
        let evaluated = dispatch(json!({"method":"evaluate","package":package,"sources":sources}));
        assert_eq!(evaluated["diagnostics"], json!([]), "{evaluated}");
        let values = &evaluated["model"]["elements"][0]["values"]["values"];
        assert_eq!(
            values["application"],
            json!(fixture.applications[fixture.selected].application)
        );
        let replay = dispatch(
            json!({"method":"compile_bundle", "compilation":package["compilation"],
            "conversions":package["schema"].get("conversion_contracts").cloned().unwrap_or(json!([])),
            "applied_conversions":package["schema"].get("applied_conversion_contracts").cloned().unwrap_or(json!([])),
            "constructors":package["schema"].get("constructor_contracts").cloned().unwrap_or(json!([]))}),
        );
        assert_eq!(replay["ok"], true, "{replay}");
        assert_eq!(replay["package"], *package);
        assert_eq!(
            dispatch(json!({"method":"evaluate","package":replay["package"],"sources":sources})),
            evaluated
        );
        if !diamond {
            let mut missing = request.clone();
            missing["constructors"] = json!([]);
            assert_eq!(dispatch(missing)["ok"], false);
            let mut stale = package.clone();
            stale["schema"]["constructor_contracts"][0]["provider"]["version"] = json!("2");
            assert!(
                intersect_web::dispatch(
                    &json!({"method":"evaluate","package":stale,"sources":sources}).to_string()
                )
                .is_err()
            );
        }
    }
}
