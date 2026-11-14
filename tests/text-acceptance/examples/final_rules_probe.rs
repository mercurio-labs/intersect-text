//! Verification-only shared native/WASM assertions.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_final_rules.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_final_rules() -> u32 {
    controls::final_parser_and_terminal_contracts();
    controls::final_enum_contract();
    controls::final_override_is_transitive_and_may_be_declared_on_replacement();
    controls::current_profile_contracts();
    4
}
fn main() {}
