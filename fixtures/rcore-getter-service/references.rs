//! Shared model-only controls for source reference getters, native and WASM.
use intersect_model::{
    api::ModelEdit, compile_rcore_at, core::query::QueryBudget, rcore::ExecutionLimits,
};
use serde_json::json;

pub const SOURCE: &str = include_str!("references.rcore");

pub fn verify() -> Result<(), String> {
    run().map_err(|e| e.to_string())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let package = compile_rcore_at("memory:reference-getters.rcore", SOURCE)?;
    let mut model = package.new_model()?;
    model.create("a", "Node", "memory:objects")?;
    model.create("b", "Child", "memory:objects")?;
    model.create("c", "Node", "memory:objects")?;
    model.set("b", "label", json!("chosen"))?;
    model.set("a", "peers", json!(["c", "b", "a"]))?;
    model.set("a", "repeats", json!(["b", "a", "b"]))?;
    let limits = ExecutionLimits::default();
    for (feature, expected) in [
        ("selected", json!("b")),
        ("optional", json!(null)),
        ("required", json!("a")),
        ("ordered", json!(["c", "b", "a"])),
        ("occurrences", json!(["b", "a", "b"])),
    ] {
        let mut measured = QueryBudget::new(10_000_000);
        assert_eq!(
            package.read_derived(&model, "a", feature, &mut measured, &limits)?,
            expected
        );
        let mut exact = QueryBudget::new(7 + measured.reads);
        exact.charge_many(7)?;
        assert_eq!(
            package.read_derived(&model, "a", feature, &mut exact, &limits)?,
            expected
        );
        assert_eq!(exact.remaining(), 0);
        let mut short = QueryBudget::new(measured.reads - 1);
        assert!(
            package
                .read_derived(&model, "a", feature, &mut short, &limits)
                .is_err()
        );
        assert!(
            package
                .read_derived(&model, "a", feature, &mut short, &limits)
                .is_err()
        );
    }
    model.set("a", "next", json!("c"))?;
    model.set("b", "label", json!(null))?;
    model.set("c", "label", json!("chosen"))?;
    let mut work = QueryBudget::new(10_000_000);
    assert_eq!(
        package.read_derived(&model, "a", "optional", &mut work, &limits)?,
        json!("c")
    );
    assert_eq!(
        package.read_derived(&model, "a", "selected", &mut work, &limits)?,
        json!("c")
    );
    let saved = package.save(&model)?;
    let (reloaded, restored) = intersect_model::rcore::RcorePackage::reload(&saved)?;
    assert_eq!(
        reloaded.read_derived(&restored, "a", "ordered", &mut work, &limits)?,
        json!(["c", "b", "a"])
    );
    let bounded = ExecutionLimits {
        max_model_elements: 1,
        ..ExecutionLimits::default()
    };
    assert!(
        package
            .read_derived(&model, "a", "selected", &mut work, &bounded)
            .is_err()
    );
    assert!(
        package
            .read_derived(&model, "missing", "required", &mut work, &limits)
            .is_err()
    );
    Ok(())
}
