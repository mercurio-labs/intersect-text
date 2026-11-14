//! Checked source subtype branches over live borrowed and owning model inputs.
use intersect_model::{
    api::ModelEdit,
    compile_rcore_at,
    core::query::QueryBudget,
    rcore::{DataWire, ExecutionLimits, RcorePackage},
};
use serde_json::json;
pub const SOURCE: &str = include_str!("type_views.rcore");
pub fn verify() -> Result<(), String> {
    run().map_err(|error| error.to_string())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut package = compile_rcore_at("memory:type-views.rcore", SOURCE)?;
    package.bind_operation("Node", "narrowed", "narrowed")?;
    let mut model = package.new_model()?;
    for (id, class) in [
        ("a", "Node"),
        ("b", "Leaf"),
        ("c", "LeafChild"),
        ("d", "Sibling"),
    ] {
        model.create(id, class, "memory:objects")?;
        model.set(id, "raw", json!(format!("raw-{id}")))?;
    }
    model.set("b", "detail", json!("chosen"))?;
    model.set("c", "detail", json!("child"))?;
    model.set("a", "next", json!("b"))?;
    model.set("a", "peers", json!(["d", "c", "b"]))?;
    let limits = ExecutionLimits::default();
    for (owner, feature, expected) in [
        ("a", "narrow", json!(null)),
        ("b", "narrow", json!("b")),
        ("c", "narrow", json!("c")),
        ("d", "narrow", json!(null)),
        ("a", "label", json!("raw-a")),
        ("b", "label", json!("chosen")),
        ("c", "label", json!("child")),
        ("d", "label", json!("raw-d")),
        ("a", "nextLabel", json!("chosen")),
        ("a", "selected", json!("b")),
    ] {
        let mut measured = QueryBudget::new(10_000_000);
        assert_eq!(
            package.read_derived(&model, owner, feature, &mut measured, &limits)?,
            expected
        );
        let mut exact = QueryBudget::new(7 + measured.reads);
        exact.charge_many(7)?;
        assert_eq!(
            package.read_derived(&model, owner, feature, &mut exact, &limits)?,
            expected
        );
        assert_eq!(exact.remaining(), 0);
        let mut short = QueryBudget::new(7 + measured.reads - 1);
        short.charge_many(7)?;
        assert!(
            package
                .read_derived(&model, owner, feature, &mut short, &limits)
                .is_err()
        );
        assert_eq!(short.remaining(), 0);
        assert!(
            package
                .read_derived(&model, owner, feature, &mut short, &limits)
                .is_err()
        );
    }
    let result = package.invoke(&model, "a", "narrowed", &[], &limits)?;
    assert!(
        matches!(result.value.data, DataWire::Some(value) if matches!(&value.data, DataWire::Model(payload) if payload["element"] == "b"))
    );
    let mut work = QueryBudget::new(10_000_000);
    model.set("a", "next", json!("d"))?;
    assert!(matches!(
        package
            .invoke(&model, "a", "narrowed", &[], &limits)?
            .value
            .data,
        DataWire::None
    ));
    assert_eq!(
        package.read_derived(&model, "a", "nextLabel", &mut work, &limits)?,
        json!(null)
    );
    model.set("a", "next", json!("c"))?;
    model.set("c", "detail", json!("chosen"))?;
    assert_eq!(
        package.read_derived(&model, "a", "selected", &mut work, &limits)?,
        json!("c")
    );
    model.set("a", "next", json!(null))?;
    assert!(matches!(
        package
            .invoke(&model, "a", "narrowed", &[], &limits)?
            .value
            .data,
        DataWire::None
    ));
    let saved = package.save(&model)?;
    let (reloaded, restored) = RcorePackage::reload(&saved)?;
    assert_eq!(
        reloaded.read_derived(&restored, "c", "label", &mut work, &limits)?,
        json!("chosen")
    );
    assert_eq!(
        reloaded.read_derived(&restored, "a", "selected", &mut work, &limits)?,
        json!("c")
    );
    Ok(())
}
