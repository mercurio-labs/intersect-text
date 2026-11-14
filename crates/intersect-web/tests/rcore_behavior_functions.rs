use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    rcore_behavior::ir_v2::Limits,
};
use intersect_model::rcore_behavior_functions::ExecutionLimits;
use serde_json::{Value, json};
fn input(body: &str) -> Value {
    let source = DefinitionSource::new(
        "memory:functions-behavior.rcore",
        SourceKind::RcoreBehavior,
        format!("rcore 1; package functions version \"1\"; {body}"),
    );
    json!({"method":"rcore_behavior/evaluateFunction","bundle":{"version":1,"sources":[source],"namespaces":[]},"entry":source.id,"selection":{"symbol":{"source":source.id,"declaration":0},"types":[],"arguments":[]},"arguments":[]})
}
fn run(request: &Value) -> Value {
    intersect_web::dispatch(&request.to_string()).expect("host request")
}
fn payload(result: &Value) -> &Value {
    &result["evaluation"]["value"]["data"]["Scalar"]["payload"]
}
#[test]
fn source_calls_escaping_closures_and_generic_specializations_execute() {
    for (body, want) in [
        (
            "fn run()->Int8=twice(21); fn twice(x:Int8)->Int8=x+x;",
            json!(42),
        ),
        (
            "fn run()->Int8={let f=make(40);f(2)}; fn make(x:Int8)->(Int8)->Int8=|y:Int8|x+y;",
            json!(42),
        ),
        (
            "fn run()->Int8=keep::<Int8,Bool>(42); fn keep<T,U>(x:T)->T=x;",
            json!(42),
        ),
        (
            "fn run()->Int8={let n:Int8=40;let f=|x:Int8|n+x;{let n=0;f(2)}};",
            json!(42),
        ),
        (
            "fn run()->Int8=if false {fail(0)} else {42};fn fail(n:Int8)->Int8=1/n;",
            json!(42),
        ),
        (
            "fn run()->Int8={let value:Int8?=Some(42);match value {Some(x)=>id(x),None=>0}};fn id<T>(x:T)->T=x;",
            json!(42),
        ),
    ] {
        let result = run(&input(body));
        assert_eq!(result["ok"], true, "{body}: {result}");
        assert_eq!(payload(&result), &want, "{body}");
    }
}
#[test]
fn compile_inspection_drives_exact_argument_types_and_fresh_replay() {
    let mut request = input("fn id<T>(x:T)->T=x;");
    request["method"] = json!("rcore_behavior/compileFunction");
    request.as_object_mut().unwrap().remove("arguments");
    request["selection"]["types"] = json!([{"Scalar":"Int8"}]);
    request["selection"]["arguments"] = json!([0]);
    let compiled = run(&request);
    assert_eq!(compiled["ok"], true, "{compiled}");
    let program = &compiled["program"];
    let entry = program["entry"].as_u64().unwrap() as usize;
    assert_eq!(compiled["effects"][entry], "pure");
    let parameter = program["functions"][entry]["parameters"][0].clone();
    request["method"] = json!("rcore_behavior/evaluateFunction");
    request["arguments"] =
        json!([{"ty":parameter,"data":{"Scalar":{"ty":{"Scalar":"Int8"},"payload":42}}}]);
    request["expected_program"] = program.clone();
    let result = run(&request);
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(payload(&result), &json!(42));
    request["expected_program"]["entry"] = json!(900);
    let rejected = run(&request);
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["stage"], "function_replay");
    request.as_object_mut().unwrap().remove("expected_program");
    request["arguments"][0]["data"]["Scalar"]["payload"] = json!(128);
    assert_eq!(run(&request)["ok"], false);
}
#[test]
fn function_host_limits_strict_inputs_and_recovery() {
    let mut request = input("fn run()->Int8=42;");
    let mut execution = serde_json::to_value(ExecutionLimits::default()).unwrap();
    execution["scalar"]["max_steps"] = json!(0);
    request["execution_limits"] = execution;
    let limited = run(&request);
    assert_eq!(limited["ok"], false, "{limited}");
    assert_eq!(limited["failure"]["kind"], "ResourceExhausted");
    request.as_object_mut().unwrap().remove("execution_limits");
    assert_eq!(run(&request)["ok"], true);
    request["selection"]["symbol"]["unexpected"] = json!(true);
    assert!(intersect_web::dispatch(&request.to_string()).is_err());
    request["selection"]["symbol"]
        .as_object_mut()
        .unwrap()
        .remove("unexpected");
    let mut verification = serde_json::to_value(Limits::default()).unwrap();
    verification["max_types"] = json!(u64::MAX);
    request["verification_limits"] = verification;
    assert!(
        intersect_web::dispatch(&request.to_string())
            .unwrap_err()
            .contains("host safety ceiling")
    );
    request
        .as_object_mut()
        .unwrap()
        .remove("verification_limits");
    request["untrusted_proof"] = json!({});
    assert!(intersect_web::dispatch(&request.to_string()).is_err());
}
#[test]
fn recursive_calls_and_returned_executable_wires_have_explicit_boundaries() {
    for body in ["fn run()->Int8=run();", "fn run()->(Int8)->Int8=|x:Int8|x;"] {
        let result = run(&input(body));
        assert_eq!(result["ok"], false, "{result}");
        assert_eq!(
            result["failure"]["kind"],
            if body.contains("|x:Int8|") {
                "Unsupported"
            } else {
                "ResourceExhausted"
            },
            "{result}"
        );
    }
    let result = run(&input("fn run()->Int8=fail(0);fn fail(x:Int8)->Int8=1/x;"));
    assert_eq!(result["failure"]["kind"], "Arithmetic", "{result}");
    assert_eq!(result["failure"]["trace"].as_array().unwrap().len(), 2);
}

#[test]
fn imported_failure_trace_preserves_definition_and_actual_call_site_revisions() {
    let helper = DefinitionSource::new(
        "memory:helper-behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package helper version \"1\"; pub fn fail(x:Int8)->Int8=1/x;",
    );
    let mut entry = DefinitionSource::new(
        "memory:caller-behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package caller version \"1\"; import package helper version \"1\" as h; fn run()->Int8=h.fail(0);",
    );
    entry.dependencies.push(helper.id.clone());
    let request = json!({"method":"rcore_behavior/evaluateFunction","bundle":{"version":1,"sources":[entry,helper],"namespaces":[]},"entry":entry.id,"selection":{"symbol":{"source":entry.id,"declaration":1},"types":[],"arguments":[]},"arguments":[]});
    let result = run(&request);
    assert_eq!(result["failure"]["kind"], "Arithmetic", "{result}");
    let trace = result["failure"]["trace"].as_array().unwrap();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0]["definition"]["source"], json!(helper.id));
    assert_eq!(trace[0]["call_site"]["source"], json!(entry.id));
    assert_eq!(trace[1]["definition"]["source"], json!(entry.id));
    assert!(trace[1]["call_site"].is_null());
    let mut replay = request.clone();
    replay["bundle"]["sources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    replay["expected_program"] = result["program"].clone();
    assert_eq!(run(&replay), result);
    replay["bundle"]["sources"][1]["dependencies"] = json!([]);
    assert_eq!(run(&replay)["ok"], false);
}

#[test]
fn every_scalar_passes_through_a_real_captured_function_value() {
    use intersect_model::definitions::scalars::Builtin;
    let cases = [
        ("Bool", json!(true)),
        ("String", json!("e\u{301}😀")),
        ("Bytes", Builtin::EByteArray.parse("00ABFF").unwrap()),
        ("Char16", Builtin::EChar.parse("55296").unwrap()),
        (
            "DateInstant",
            Builtin::EDate.parse("1970-01-01T00:00:00.000Z").unwrap(),
        ),
        ("Unit", json!({"kind":"unit"})),
        ("Int8", json!(-128)),
        ("Int16", json!(-32768)),
        ("Int32", json!(-2147483648_i64)),
        (
            "Int64",
            Builtin::ELong.parse("-9223372036854775808").unwrap(),
        ),
        ("UInt8", json!(255)),
        ("UInt16", json!(65535)),
        ("UInt32", json!(4294967295_u64)),
        (
            "UInt64",
            json!({"kind":"uint64","value":"18446744073709551615"}),
        ),
        (
            "BigInt",
            Builtin::EBigInteger
                .parse("123456789012345678901234567890")
                .unwrap(),
        ),
        (
            "Decimal",
            Builtin::EBigDecimal
                .parse("123456789012345678901.2300")
                .unwrap(),
        ),
        ("Float32", json!({"kind":"float32","bits":"7fc01234"})),
        (
            "Float64",
            json!({"kind":"float64","bits":"8000000000000000"}),
        ),
    ];
    for (ty, payload) in cases {
        let mut request = input(&format!(
            "fn run(x:{ty})->{ty}={{let f=|dummy:Int8|x;f(0)}};"
        ));
        request["method"] = json!("rcore_behavior/compileFunction");
        request.as_object_mut().unwrap().remove("arguments");
        let compiled = run(&request);
        assert_eq!(compiled["ok"], true, "{ty}: {compiled}");
        let entry = compiled["program"]["entry"].as_u64().unwrap() as usize;
        let parameter = compiled["program"]["functions"][entry]["parameters"][0].clone();
        let value =
            json!({"ty":parameter,"data":{"Scalar":{"ty":{"Scalar":ty},"payload":payload}}});
        request["method"] = json!("rcore_behavior/evaluateFunction");
        request["arguments"] = json!([value]);
        let result = run(&request);
        assert_eq!(result["ok"], true, "{ty}: {result}");
        assert_eq!(result["evaluation"]["value"], value, "{ty}");
    }
}
