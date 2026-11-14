//! Handwritten package replay and exact source scope selector controls.
use intersect_model::definitions::{rcore_behavior::ir_v2::Limits, rcore_scope::compile_scope};
const SOURCE: &str = r#"rcore 1;
package example.scopes version "1";
metamodel "urn:scope-test" prefix m {
  class Namespace {
    ref types: Type [0,*] containment;
    ref properties: Property [0,*] containment;
  }
  class Type { attr name: String [1,1]; }
  class Property {
    ref owner: Namespace [0,1];
    ref targetType: Type [0,1];
  }
}
behavior {
  import model "urn:scope-test" as m;
  bind scope m.Property.targetType = visible_types;
  fn visible_types(owner:m.Property, spelling:String)->Sequence<m.Type> = [];
}
"#;

#[test]
fn package_retains_scope_source_selectors_for_explicit_activation_and_replay() {
    let package = intersect_text::compiler::compile_rcore_with_profile(
        SOURCE,
        None,
        None,
        intersect_text::grammar::Profile::default(),
    )
    .expect("package compiles");
    assert_eq!(package.semantics.scope_bindings.len(), 1);
    let selector = &package.semantics.scope_bindings[0];
    assert_eq!(selector.declaration, 1);
    let inputs = package
        .compilation
        .as_ref()
        .expect("retained source inputs");
    assert_eq!(selector.source, inputs.rcore_behavior);
    compile_scope(
        &inputs.bundle,
        &selector.source,
        selector.declaration,
        &Limits::default(),
    )
    .expect("retained selector recompiles original scope body");
    let replay = intersect_text::compiler::recompile(&package).expect("source replay");
    assert_eq!(
        replay.semantics.scope_bindings,
        package.semantics.scope_bindings
    );
}
