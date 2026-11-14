//! Verification-only entry point; the same assertions run natively and in WASM.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_validated_syntax.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_validated_syntax() -> u32 {
    controls::unicode_definition_literals_execute_and_replay_exactly();
    controls::syntax_invalid_inputs_cannot_publish_a_package();
    controls::imported_syntax_is_not_unimplemented_execution();
    controls::inherited_sources_keep_identity_limits_and_rollback();
    4
}
fn main() {}
