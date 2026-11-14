//! Native JSON trust-boundary controls for the public resource adapter.
use serde_json::{Value, json};
fn invoke(mut value: Value) -> Result<Value, String> {
    if value["method"]
        .as_str()
        .is_some_and(|method| method.starts_with("model/cache"))
        && value["runtime_identity"].is_null()
    {
        value["runtime_identity"] = json!("native-json-resource-tests/v1");
    }
    intersect_web::dispatch(&value.to_string())
}
fn fixture() -> Result<(Value, Value, Value), String> {
    let package = intersect_text::compiler::compile_rcore_language(
        include_str!("../../../fixtures/library/library.rcore"),
        include_str!("../../../fixtures/library/library.xtext"),
        include_str!("../../../fixtures/library/library-behavior.rcore"),
    )
    .map_err(|errors| format!("{errors:?}"))?;
    let sources = json!([
        {"uri":"catalog.library","text":include_str!("../../../fixtures/library/catalog.library")},
        {"uri":"loans.library","text":include_str!("../../../fixtures/library/loans.library")},
    ]);
    let input: Vec<intersect_text::Source> =
        serde_json::from_value(sources.clone()).map_err(|error| error.to_string())?;
    let evaluation = intersect_text::evaluate(&package, &input);
    assert!(evaluation.diagnostics.is_empty());
    Ok((json!(package), json!(evaluation.model), sources))
}
#[test]
fn unavailable_models_are_diagnostic_and_snapshot_context_is_rechecked() -> Result<(), String> {
    let (package, model, sources) = fixture()?;
    let selected = json!(["catalog.library"]);
    let files=invoke(json!({"method":"model/export","package":package,"model":model,"format":"xmi","selected":selected}))?["resources"].clone();
    let unloaded = invoke(
        json!({"method":"model/resourceUnload","package":package,"model":model,"selected":selected,"policy":"discard"}),
    )?;
    assert!(
        !unloaded["evaluation"]["diagnostics"]
            .as_array()
            .ok_or("Missing diagnostics")?
            .is_empty()
    );
    assert_eq!(unloaded["evaluation"]["derived"], json!({}));
    let state = invoke(
        json!({"method":"model/resourceSnapshot","package":package,"model":unloaded["evaluation"]["model"]}),
    )?;
    let restored =
        invoke(json!({"method":"model/resourceRestore","package":package,"state":state}))?;
    assert_eq!(restored["evaluation"], unloaded["evaluation"]);
    let mut bad = state;
    bad["context"]["dependencies"]["compiled-language"] = json!("changed implementation");
    assert!(
        invoke(json!({"method":"model/resourceRestore","package":package,"state":bad})).is_err()
    );
    let loaded = invoke(
        json!({"method":"model/resourceReload","package":package,"model":restored["evaluation"]["model"],"selected":selected,"resources":files,"sources":sources}),
    )?;
    assert_eq!(loaded["evaluation"]["diagnostics"], json!([]));
    assert_eq!(
        loaded["evaluation"]["model"]["elements"]
            .as_array()
            .ok_or("Missing elements")?
            .len(),
        6
    );
    Ok(())
}
#[test]
fn cache_sources_and_compiled_implementations_are_independent_expected_inputs() -> Result<(), String>
{
    let (package, model, sources) = fixture()?;
    let cache = invoke(
        json!({"method":"model/cachePrepare","package":package,"model":model,"sources":sources,"selected":["catalog.library","loans.library"]}),
    )?;
    let request = json!({"method":"model/cacheLoad","package":package,"model":model,"sources":sources,"bytes":cache["bytes"]});
    assert_eq!(
        invoke(request.clone())?["evaluation"]["diagnostics"],
        json!([])
    );
    let mut changed = request.clone();
    changed["sources"][0]["text"] = json!("changed source bytes");
    assert!(invoke(changed).is_err());
    let mut corrupt = request;
    corrupt["bytes"][0] = json!(0);
    assert!(invoke(corrupt).is_err());
    Ok(())
}
