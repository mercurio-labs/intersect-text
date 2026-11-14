use super::*;
use intersect_model::core::{query::DerivedValues, storage::ModelView};
use intersect_model::semantic_bindings::derived::BoundDerived;
use intersect_text::compiler;
use intersect_text::package::Package;
#[path = "../../../fixtures/library/library-implementations.rs"]
mod native_library;

const STRUCTURE: &str = include_str!("../../../fixtures/library/library-implementations.rcore");
const SCRIPT: &str = include_str!("../../../fixtures/library/library-implementations.rhai");

fn draft(script: &str) -> Value {
    let mut request = json!({"project":true,"structure":STRUCTURE,"behavior":""});
    let sources = compile_request(&request).unwrap();
    request["implementations"] = json!(
        sources
            .contracts
            .iter()
            .map(|c| json!({"contract":c.id(),"backend":"rhai","source":script}))
            .collect::<Vec<_>>()
    );
    request
}
fn compile_request_for(script: &str) -> Value {
    let bindings = draft(script);
    let sources = compile_request(&bindings).unwrap();
    json!({"method":"compile_rcore", "rcore":sources.model_source.text,
        "xtext":include_str!("../../../fixtures/library/library.xtext"),
        "rcore_behavior":sources.model_behavior,"bindings":bindings,"xtext_profile":"intersect.workbench.v1"})
}
fn dispatch(request: Value) -> Value {
    crate::dispatch(&request.to_string()).unwrap()
}
fn compile(script: &str) -> Value {
    let result = dispatch(compile_request_for(script));
    assert_eq!(result["ok"], true, "{result}");
    result["package"].clone()
}
fn evaluate(package: &Value) -> Value {
    dispatch(json!({"method":"evaluate","package":package,"sources":[
        {"uri":"catalog.library","text":include_str!("../../../fixtures/library/catalog.library")},
        {"uri":"loans.library","text":include_str!("../../../fixtures/library/loans.library")}
    ]}))
}
fn id(evaluation: &Value, name: &str) -> String {
    evaluation["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["values"]["name"] == name)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}

#[test]
fn library_external_properties_evaluate_query_and_recompute_after_edits() {
    let package = compile(SCRIPT);
    let evaluation = evaluate(&package);
    assert_eq!(evaluation["diagnostics"], json!([]), "{evaluation}");
    let loan = id(&evaluation, "first");
    let copy = id(&evaluation, "earthsea");
    assert_eq!(
        evaluation["derived"][&loan]["overdueFee"],
        json!({"kind":"int64","value":"100"})
    );
    assert_eq!(
        evaluation["derived"][&copy]["copyLabel"],
        "earthsea (edition 2)"
    );
    let query = dispatch(
        json!({"method":"query/execute","package":package,"model":evaluation["model"],"query":{
            "schema_uri":package["schema"]["uri"],"schema_revision":package["schema"]["revision"],"from":"Loan",
            "select":[{"name":"fee","expression":{"op":"field","path":"overdueFee"}}],"limit":10
        }}),
    );
    assert_eq!(query["rows"][0][0]["value"], "100");
    dispatch(
        json!({"method":"script/begin","package":package,"model":evaluation["model"],"revision":"test"}),
    );
    assert_eq!(
        dispatch(json!({"method":"script/get","revision":"test","id":loan,"feature":"overdueFee"}))
            ["value"],
        "100"
    );
    dispatch(
        json!({"method":"script/set","revision":"test","id":loan,"feature":"daysLate","value":{"kind":"int64","value":"7"}}),
    );
    assert_eq!(
        dispatch(json!({"method":"script/get","revision":"test","id":loan,"feature":"overdueFee"}))
            ["value"],
        "175"
    );
    dispatch(
        json!({"method":"script/set","revision":"test","id":copy,"feature":"edition","value":3}),
    );
    assert_eq!(
        dispatch(json!({"method":"script/get","revision":"test","id":copy,"feature":"copyLabel"})),
        "earthsea (edition 3)"
    );
    let changed = compile(&SCRIPT.replace("daysLate * 25", "daysLate * 50"));
    assert_eq!(
        evaluate(&changed)["derived"][&loan]["overdueFee"]["value"],
        "200"
    );
    let rejected = dispatch(compile_request_for("fn overdueFee(n) { n }"));
    assert_eq!(rejected["ok"], false);
    assert_eq!(
        evaluate(&changed)["derived"][&loan]["overdueFee"]["value"],
        "200"
    );
}

#[test]
fn native_and_rhai_providers_share_derived_reads_and_reject_wrong_sources() {
    let request = draft(SCRIPT);
    let sources = compile_request(&request).unwrap();
    let package: Package = serde_json::from_value(compile(SCRIPT)).unwrap();
    let evaluated = evaluate(&json!(package));
    let model = serde_json::from_value(evaluated["model"].clone()).unwrap();
    let view = ModelView::new(&model).unwrap();
    let mut registry = Registry::default();
    for contract in &sources.contracts {
        if contract.descriptor().name == "overdueFee" {
            registry
                .install(
                    contract,
                    Rc::new(NativeProvider::new(
                        contract,
                        "library-native-fee-v1",
                        |call: &Call<'_>, args: &[ValueWire]| {
                            call.result(native_library::overdue_fee(
                                call,
                                i64::from_wire(&args[0])?,
                            )?)
                        },
                    )),
                )
                .unwrap();
        } else {
            registry
                .install(
                    contract,
                    Rc::new(RhaiProvider::compile(contract, "library.rhai", SCRIPT).unwrap()),
                )
                .unwrap();
        }
    }
    let bound = BoundDerived::new(&package, &sources, &registry, Budget::new(100_000)).unwrap();
    let mut budget = intersect_model::core::query::QueryBudget::new(10_000);
    assert_eq!(
        bound
            .read(&view, &id(&evaluated, "first"), "overdueFee", &mut budget)
            .unwrap()["value"],
        "100"
    );
    assert_eq!(
        bound
            .read(&view, &id(&evaluated, "earthsea"), "copyLabel", &mut budget)
            .unwrap(),
        "earthsea (edition 2)"
    );
    let missing = Registry::default();
    assert!(BoundDerived::new(&package, &sources, &missing, Budget::new(100_000)).is_err());
    let foreign = compile_sources(STRUCTURE, "").unwrap();
    assert!(
        BoundDerived::new(&package, &foreign, &registry, Budget::new(100_000))
            .err()
            .unwrap()
            .contains("different model or behavior sources")
    );
    let mut changed = package.clone();
    changed.sources.get_mut("rcore_behavior").unwrap().push(' ');
    assert!(BoundDerived::new(&changed, &sources, &registry, Budget::new(100_000)).is_err());
    let exhausted = BoundDerived::new(&package, &sources, &registry, Budget::new(1)).unwrap();
    assert!(
        exhausted
            .read(&view, &id(&evaluated, "first"), "overdueFee", &mut budget)
            .unwrap_err()
            .contains("budget")
    );
}

#[test]
fn external_derived_type_errors_nested_calls_and_execution_errors_are_explicit() {
    let request = compile_request_for(SCRIPT);
    let behavior = request["rcore_behavior"].as_str().unwrap();
    for invalid in [
        behavior.replace("overdueFee(self.daysLate)", "overdueFee(self.name)"),
        behavior.replace(
            "overdueFee(self.daysLate)",
            "overdueFee(overdueFee(self.daysLate))",
        ),
    ] {
        assert!(
            compiler::compile_rcore_with_profile(
                request["rcore"].as_str().unwrap(),
                Some(request["xtext"].as_str().unwrap()),
                Some(&invalid),
                intersect_text::grammar::Profile::WorkbenchV1
            )
            .is_err()
        );
    }
    for body in ["\"wrong type\"", "loop {}", "throw \"Fee unavailable\""] {
        let package = compile(&SCRIPT.replace("daysLate * 25", body));
        let result = evaluate(&package);
        assert!(
            !result["diagnostics"].as_array().unwrap().is_empty(),
            "{result}"
        );
    }
}
