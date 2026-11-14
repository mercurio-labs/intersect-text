#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_current_fragments.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_current_fragments() -> u32 {
    controls::fragments_preserve_captured_and_explicit_current_objects();
    controls::fragments_require_compatible_existing_owners_and_writable_members();
    controls::fragments_check_actual_action_class_feature_contracts();
    3
}
fn main() {}
