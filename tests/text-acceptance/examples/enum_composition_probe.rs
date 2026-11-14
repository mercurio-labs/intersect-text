//! Verification-only shared native/WASM controls.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_enum_composition.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_enum_composition() -> u32 {
    controls::enum_override_preserves_nominal_values_and_source_identity();
    controls::enum_override_rejects_nominal_mismatch_final_and_orphan();
    controls::explicit_parent_enum_retains_its_spelling_and_literal_identity();
    controls::declaration_only_children_inherit_entry_but_standalone_grammars_reject();
    4
}
fn main() {}
