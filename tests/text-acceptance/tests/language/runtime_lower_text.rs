use super::Store;
#[path = "common/runtime_lower_text.rs"]
mod common;
use intersect_model::core::storage::{self};
use intersect_text::compiler::compile_bundle;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate_into};
use serde_json::json;
store_test! {
pub fn raw_and_concrete_lower_domains_link_text_and_replay_atomically() {
    let package = compile_bundle(common::fixture()).unwrap();
    let mut store = crate::stores::store::<Store>(&package.schema).unwrap();
    let result = evaluate_into(
        &package,
        &[common::source()],
        EvaluationLimits::native(),
        &mut store,
    )
    .unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let before = storage::export(&store).unwrap();
    for text in [
        "model { narrow N raw R N }",
        "model { bad B raw R B }",
        "model { bad B closed C B }",
    ] {
        let result = evaluate_into(
            &package,
            &[Source {
                uri: common::source().uri,
                text: text.into(),
            }],
            EvaluationLimits::native(),
            &mut store,
        )
        .unwrap();
        assert!(!result.diagnostics.is_empty());
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    let retained: Package = serde_json::from_value(json!(package)).unwrap();
    let rebuilt = compile_bundle(retained.compilation.clone().unwrap()).unwrap();
    assert_eq!(json!(rebuilt), json!(retained));
    let mut fresh = crate::stores::store::<Store>(&package.schema).unwrap();
    let replay = evaluate_into(
        &rebuilt,
        &[common::source()],
        EvaluationLimits::native(),
        &mut fresh,
    )
    .unwrap();
    assert!(replay.diagnostics.is_empty());
    assert_eq!(storage::export(&fresh).unwrap(), before);
}
}
