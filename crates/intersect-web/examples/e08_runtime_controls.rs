//! Native fixture export for shared browser integration controls; no product API.
#[path = "../../../tests/text-acceptance/tests/language/common/eobject_grammar.rs"]
mod eobject;
#[path = "../../../tests/text-acceptance/tests/language/common/runtime_lower_text.rs"]
mod lower;
#[path = "../../../tests/text-acceptance/tests/language/common/runtime_operation_providers.rs"]
mod providers;
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let p = providers::fixture();
    println!(
        "{}",
        serde_json::to_string(&json!({
         "eobject":{"compilation":eobject::fixture(),"source":eobject::source()},
         "lower":{"compilation":lower::fixture(),"source":lower::source()},
         "providers":{"compilation":p.input,"conversions":p.conversions,"constructors":p.constructors,"receiver":p.receiver,"operations":p.operations,"arguments":p.arguments}
        }))?
    );
    Ok(())
}
