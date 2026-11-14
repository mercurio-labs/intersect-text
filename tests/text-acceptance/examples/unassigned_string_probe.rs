#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_unassigned_string.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_unassigned_string() -> u32 {
    controls::string_helpers_preserve_owner_in_objects_fragments_and_actions();
    controls::unsupported_calls_and_nullable_helpers_remain_rejected();
    2
}
fn main() {}
