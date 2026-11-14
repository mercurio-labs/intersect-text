//! Verification-only shared native/WASM assertions.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_category_overrides.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_category_overrides() -> u32 {
    controls::terminal_replaces_datatype_parser_with_one_effective_program();
    controls::incompatible_category_replacements_reject();
    controls::explicit_parent_datatype_survives_terminal_replacement();
    controls::current_profile_contracts();
    4
}
fn main() {}
