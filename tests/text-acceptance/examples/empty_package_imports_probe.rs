//! Verification-only native/WASM package authority controls.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_empty_package_imports.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_empty_package_imports() -> u32 {
    controls::empty_package_identity_is_retained_but_not_admitted_as_a_grammar_import();
    controls::checked_empty_imports_execute_and_replay_in_both_compiler_paths();
    controls::empty_imports_do_not_authorize_missing_duplicate_unreachable_or_stale_sources();
    controls::current_profile_contracts();
    4
}
fn main() {}
