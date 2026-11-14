use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    scalars::Builtin,
};
use serde_json::{Value, json};
fn request(body: &str, arguments: Vec<Value>) -> Value {
    let source = DefinitionSource::new(
        "memory:scalar-behavior.rcore",
        SourceKind::RcoreBehavior,
        format!("rcore 1; package scalar version \"1\"; {body}"),
    );
    json!({"method":"rcore_behavior/evaluateScalar","bundle":{"version":1,"sources":[source],"namespaces":[]},"entry":source.id,"selected":{"source":source.id,"declaration":0},"arguments":arguments})
}
fn run(request: &Value) -> Value {
    intersect_web::dispatch(&request.to_string()).expect("host dispatch")
}
fn scalar(ty: &str, payload: Value) -> Value {
    json!({"ty":{"Scalar":ty},"payload":payload})
}
#[test]
fn every_scalar_has_an_actual_host_parameter_and_return_consumer() {
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
        let argument = scalar(ty, payload);
        let response = run(&request(
            &format!("fn identity(x:{ty})->{ty}=x;"),
            vec![argument.clone()],
        ));
        assert_eq!(response["ok"], true, "{ty}: {response}");
        assert_eq!(response["evaluation"]["value"], argument);
    }
}
#[test]
fn short_circuit_and_nested_conditionals_execute_only_selected_branches() {
    for (body, payload) in [
        ("fn run()->Bool=false && (1 / 0 == 0);", json!(false)),
        ("fn run()->Bool=true || (1 / 0 == 0);", json!(true)),
        (
            "fn run()->Int8=if true { if false { 1 / 0 } else { -(128) } } else { 1 / 0 };",
            json!(-128),
        ),
    ] {
        let result = run(&request(body, vec![]));
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(result["evaluation"]["value"]["payload"], payload);
    }
    let result = run(&request(
        "fn run()->Int8=if false { 1 } else { 1 / 0 };",
        vec![],
    ));
    assert_eq!(result["ok"], false);
    assert_eq!(result["failure"]["kind"], "Arithmetic");
    assert!(
        result["failure"]["span"]["end"].as_u64().unwrap()
            > result["failure"]["span"]["start"].as_u64().unwrap()
    );
}
#[test]
fn fresh_replay_rejects_forged_body_and_source_revision() {
    let mut input = request("fn run()->Int8=42;", vec![]);
    let first = run(&input);
    assert_eq!(first["ok"], true, "{first}");
    input["expected_program"] = first["program"].clone();
    assert_eq!(run(&input), first);
    input["expected_program"]["blocks"][0]["instructions"][0]["kind"]["Constant"]["payload"] =
        json!(43);
    assert_eq!(run(&input)["stage"], "scalar_replay");
    input.as_object_mut().unwrap().remove("expected_program");
    input["selected"]["source"]["sha256"] = json!("0".repeat(64));
    assert_eq!(run(&input)["ok"], false);
}
#[test]
fn execution_budget_and_bad_arguments_reject_then_recover() {
    let mut zero_work =
        serde_json::to_value(intersect_model::rcore_behavior_execution::ExecutionLimits::default())
            .unwrap();
    zero_work["numeric"]["max_work"] = json!(0);
    let mut skipped = request("fn run()->Bool=false && (1+1==2);", vec![]);
    skipped["execution_limits"] = zero_work.clone();
    let skipped_result = run(&skipped);
    assert_eq!(skipped_result["ok"], true, "{skipped_result}");
    assert_eq!(skipped_result["evaluation"]["numeric_work"], 0);
    let mut taken = request("fn run()->Bool=true && (1+1==2);", vec![]);
    taken["execution_limits"] = zero_work;
    assert_eq!(run(&taken)["failure"]["kind"], "ResourceExhausted");
    let mut input = request("fn run(x:Int8)->Int8=x+1;", vec![scalar("Int8", json!(3))]);
    let accepted = run(&input);
    assert_eq!(accepted["evaluation"]["value"]["payload"], 4);
    let mut limits =
        serde_json::to_value(intersect_model::rcore_behavior_execution::ExecutionLimits::default())
            .unwrap();
    limits["max_steps"] = json!(0);
    input["execution_limits"] = limits;
    assert_eq!(run(&input)["failure"]["kind"], "ResourceExhausted");
    input.as_object_mut().unwrap().remove("execution_limits");
    assert_eq!(run(&input), accepted);
    input["arguments"][0]["payload"] = json!(128);
    assert_eq!(run(&input)["failure"]["kind"], "Arguments");
    input["arguments"][0] = scalar("UInt8", json!(3));
    assert_eq!(run(&input)["failure"]["kind"], "Arguments");
}
#[test]
fn decimal_and_ieee_results_are_exact_and_explicit() {
    let result = run(&request("fn run()->Decimal=1 / 8;", vec![]));
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(
        result["evaluation"]["value"]["payload"],
        json!({"kind":"decimal","coefficient":"125","scale":3})
    );
    let bad = run(&request("fn run()->Decimal=1 / 3;", vec![]));
    assert_eq!(bad["ok"], false);
    assert_eq!(bad["failure"]["kind"], "Arithmetic");
    let result = run(&request("fn run()->Float32=0 / 0;", vec![]));
    assert_eq!(
        result["evaluation"]["value"]["payload"],
        json!({"kind":"float32","bits":"7fc00000"})
    );
}
#[test]
fn oversized_request_and_execution_digit_budget_fail_before_work_and_recover() {
    let mut forged = request("fn run()->Int8=1;", vec![]);
    forged["selected"]["verified"] = json!(true);
    assert!(
        intersect_web::dispatch(&forged.to_string())
            .unwrap_err()
            .contains("canonical identity")
    );
    forged["selected"]
        .as_object_mut()
        .unwrap()
        .remove("verified");
    let mut excessive =
        serde_json::to_value(intersect_model::rcore_behavior_execution::ExecutionLimits::default())
            .unwrap();
    excessive["max_steps"] = json!(4294967296_u64);
    forged["execution_limits"] = excessive;
    assert!(
        intersect_web::dispatch(&forged.to_string())
            .unwrap_err()
            .contains("safety ceiling")
    );
    let input = request(
        "fn run(x:BigInt)->BigInt=x;",
        vec![scalar("BigInt", json!({"kind":"bigint","value":"123"}))],
    );
    let accepted = run(&input);
    assert_eq!(accepted["ok"], true);
    let oversized = format!("{}{}", input, " ".repeat(4_194_304));
    assert!(
        intersect_web::dispatch(&oversized)
            .unwrap_err()
            .contains("request byte budget")
    );
    let mut low = input.clone();
    let mut limits =
        serde_json::to_value(intersect_model::rcore_behavior_execution::ExecutionLimits::default())
            .unwrap();
    limits["values"]["max_digits"] = json!(2);
    low["execution_limits"] = limits;
    assert_eq!(run(&low)["failure"]["kind"], "ResourceExhausted");
    assert_eq!(run(&input), accepted);
}
