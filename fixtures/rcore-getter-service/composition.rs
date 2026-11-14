//! Source getter calls share one VM, live read authority and original budget.
use intersect_model::{
    api::ModelEdit, compile_rcore_at, core::query::QueryBudget, rcore::ExecutionLimits,
};
use serde_json::json;
pub const SOURCE: &str = include_str!("composition.rcore");
pub fn verify() -> Result<(), String> {
    run().map_err(|error| error.to_string())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let package = compile_rcore_at("memory:composition.rcore", SOURCE)?;
    let mut model = package.new_model()?;
    model.create("a", "Child", "memory:objects")?;
    model.create("b", "Node", "memory:objects")?;
    model.set("a", "raw", json!("A"))?;
    model.set("b", "raw", json!("chosen"))?;
    model.set("a", "next", json!("b"))?;
    model.set("a", "peers", json!(["a", "b"]))?;
    let limits = ExecutionLimits::default();
    for (feature, expected) in [
        ("decorated", json!("~A")),
        ("peerLabel", json!("~chosen")),
        ("selected", json!("b")),
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
    let mut work = QueryBudget::new(10_000_000);
    model.set("b", "raw", json!("changed"))?;
    assert_eq!(
        package.read_derived(&model, "a", "peerLabel", &mut work, &limits)?,
        json!("~changed")
    );
    assert_eq!(
        package.read_derived(&model, "a", "selected", &mut work, &limits)?,
        json!(null)
    );
    model.set("a", "next", json!(null))?;
    assert_eq!(
        package.read_derived(&model, "a", "peerLabel", &mut work, &limits)?,
        json!(null)
    );
    let saved = package.save(&model)?;
    let (reloaded, restored) = intersect_model::rcore::RcorePackage::reload(&saved)?;
    assert_eq!(
        reloaded.read_derived(&restored, "a", "decorated", &mut work, &limits)?,
        json!("~A")
    );
    Ok(())
}
