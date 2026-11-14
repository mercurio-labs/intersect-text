//! Verification-only entry point using identical native and WASM controls.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_escaped_identifiers.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_escaped_identifiers() -> u32 {
    controls::escaped_names_preserve_source_and_execute();
    controls::malformed_escapes_reject();
    2
}
fn main() {}
