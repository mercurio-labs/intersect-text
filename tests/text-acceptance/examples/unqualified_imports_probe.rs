//! Verification-only native/WASM TypeRef scope controls.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_unqualified_imports.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_unqualified_imports() -> u32 {
    controls::explicit_types_resolve_across_local_aliased_imports();
    controls::ambiguous_and_missing_unqualified_types_reject();
    controls::inherited_rules_keep_scope_but_child_types_require_local_imports();
    controls::multiple_parent_syntax_is_retained_but_native_admission_rejects();
    controls::current_profile_contracts();
    5
}
fn main() {}
