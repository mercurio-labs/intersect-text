//! Shared native/WASM source getter controls; no whole-language qualification.
use intersect_model::{
    api::ModelEdit,
    compile_rcore_at,
    core::query::QueryBudget,
    rcore::{ExecutionLimits, RcorePackage},
};
use serde_json::json;

const SOURCE: &str = r#"rcore 1;
package getter.controls version "1";
metamodel "urn:getter-controls" prefix m {
  class Base { attr declaredName: String [0,1]; }
  class Element extends Base {
    attr name: String [0,1] derived;
    attr named: Bool [1,1] derived;
    attr answer: Int64 [1,1] derived;
  }
  class Child extends Element {}
}
behavior {
  import model "urn:getter-controls" as m;
  derive m.Element.name(self) -> Option<String> = effective_name(self);
  fn effective_name(self:m.Element) -> Option<String> =
    match self.declaredName { Some(name) => if name == "P" { Some("mapped") } else { Some(name) }, None => None };
  derive m.Element.named(self) -> Bool = has_name(self);
  fn has_name(self:m.Element) -> Bool = match self.declaredName { Some(name) => true, None => false };
  derive m.Element.answer(self) -> Int64 = answer(self);
  fn answer(self:m.Element) -> Int64 = 42;
}
"#;

pub fn package_installs_source_getters_and_rebuilds_them_on_reload()
-> Result<(), Box<dyn std::error::Error>> {
    let package = compile_rcore_at("memory:getters.rcore", SOURCE)?;
    let mut model = package.new_model()?;
    model.create("item", "Child", "memory:objects")?;
    let limits = ExecutionLimits::default();
    let mut work = QueryBudget::new(1_000_000);
    assert_eq!(
        package.read_derived(&model, "item", "name", &mut work, &limits)?,
        json!(null)
    );
    assert_eq!(
        package.read_derived(&model, "item", "named", &mut work, &limits)?,
        json!(false)
    );
    assert_eq!(
        package.read_derived(&model, "item", "answer", &mut work, &limits)?,
        json!({"kind":"int64","value":"42"})
    );
    model.set("item", "declaredName", json!("P"))?;
    let saved = package.save(&model)?;
    assert_eq!(
        package.read_derived(&model, "item", "name", &mut work, &limits)?,
        json!("mapped")
    );
    let evaluation = package.evaluate_model_rules(&model)?;
    assert_eq!(evaluation.derived["item"]["name"], json!("mapped"));
    assert_eq!(evaluation.derived["item"]["named"], json!(true));
    assert_eq!(saved, package.save(&model)?);
    let (reloaded, mut restored) = RcorePackage::reload(&saved)?;
    assert_eq!(saved, reloaded.save(&restored)?);
    assert_eq!(
        reloaded.read_derived(&restored, "item", "name", &mut work, &limits)?,
        json!("mapped")
    );
    restored.set("item", "declaredName", json!("changed"))?;
    assert_eq!(
        reloaded.read_derived(&restored, "item", "name", &mut work, &limits)?,
        json!("changed")
    );
    let changed_source = SOURCE.replace("Some(\"mapped\")", "None");
    let changed_package = compile_rcore_at("memory:getters.rcore", &changed_source)?;
    let mut changed_model = changed_package.new_model()?;
    changed_model.create("item", "Child", "memory:objects")?;
    changed_model.set("item", "declaredName", json!("P"))?;
    assert_eq!(
        changed_package.read_derived(&changed_model, "item", "name", &mut work, &limits)?,
        json!(null)
    );
    assert!(
        changed_package
            .read_derived(&model, "item", "name", &mut work, &limits)
            .is_err()
    );
    Ok(())
}

pub fn source_getter_reads_keep_the_callers_exact_remaining_allowance()
-> Result<(), Box<dyn std::error::Error>> {
    let package = compile_rcore_at("memory:getters.rcore", SOURCE)?;
    let mut model = package.new_model()?;
    model.create("item", "Element", "memory:objects")?;
    model.set("item", "declaredName", json!("P"))?;
    let limits = ExecutionLimits::default();
    let mut measured = QueryBudget::new(1_000_000);
    let expected = package.read_derived(&model, "item", "name", &mut measured, &limits)?;
    let mut exact = QueryBudget::new(7 + measured.reads);
    exact.charge_many(7)?;
    assert_eq!(
        package.read_derived(&model, "item", "name", &mut exact, &limits)?,
        expected
    );
    assert_eq!(exact.remaining(), 0);
    for limit in [0, measured.reads - 1] {
        let mut short = QueryBudget::new(limit);
        assert!(
            package
                .read_derived(&model, "item", "name", &mut short, &limits)
                .is_err()
        );
        assert!(
            package
                .read_derived(&model, "item", "name", &mut short, &limits)
                .is_err()
        );
        assert_eq!(short.remaining(), 0);
    }
    assert!(
        package
            .read_derived(&model, "missing", "name", &mut measured, &limits)
            .is_err()
    );
    assert!(
        package
            .read_derived(&model, "item", "missing", &mut measured, &limits)
            .is_err()
    );
    assert_eq!(
        package.read_derived(&model, "item", "name", &mut measured, &limits)?,
        expected
    );
    Ok(())
}

pub fn unsupported_getter_receiver_and_assignment_contracts_reject_at_loading() {
    let wrong_receiver = SOURCE.replace(
        "fn effective_name(self:m.Element)",
        "fn effective_name(self:m.Base)",
    );
    assert!(compile_rcore_at("memory:getters.rcore", &wrong_receiver).is_err());
    let assignment_history = SOURCE.replace(
        "attr name: String [0,1] derived;",
        "attr name: String [0,1] derived unsettable;",
    );
    assert!(compile_rcore_at("memory:getters.rcore", &assignment_history).is_err());
}

/// Run exactly the same source controls in the isolated model-only consumer.
pub fn verify() -> Result<(), String> {
    package_installs_source_getters_and_rebuilds_them_on_reload().map_err(|e| e.to_string())?;
    source_getter_reads_keep_the_callers_exact_remaining_allowance().map_err(|e| e.to_string())?;
    unsupported_getter_receiver_and_assignment_contracts_reject_at_loading();
    Ok(())
}
