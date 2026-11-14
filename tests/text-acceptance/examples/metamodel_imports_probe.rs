//! Verification-only shared native/WASM assertions.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_metamodel_imports.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_metamodel_imports() -> u32 {
    controls::multiple_packages_and_aliases_execute_with_exact_identity();
    controls::missing_imports_and_duplicate_namespaces_reject();
    controls::shared_alias_ambiguity_rejects_without_first_match();
    controls::pinned_ecore_alias_collision_uses_complete_inventory();
    controls::inherited_rules_keep_declaring_metamodel_context();
    controls::current_profile_contracts();
    6
}
fn main() {}
