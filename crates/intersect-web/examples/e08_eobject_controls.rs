//! Shared source fixtures for native/browser acceptance, not a product API.
#[path = "../../../tests/text-acceptance/tests/language/common/eobject_construction.rs"]
mod eobject;
#[path = "../../../tests/text-acceptance/tests/language/common/operation_policy_qualification.rs"]
mod policy;
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let policies:Vec<_>=[false,true].into_iter().map(|generic| {let p=policy::fixture(generic);json!({"compilation":p.input,"conversions":p.conversions,"receiver":p.receiver,"base":p.base,"child":p.child})}).collect();
    println!(
        "{}",
        serde_json::to_string(
            &json!({"eobject":{"compilation":eobject::fixture(),"source":eobject::source()},"policies":policies})
        )?
    );
    Ok(())
}
