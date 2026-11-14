#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_mixed_actions.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_mixed_actions() -> u32 {
    controls::mixed_branches_share_construction_and_initializers();
    controls::mixed_action_flow_rejects_invalid_creation_paths();
    controls::simple_action_alternative_without_capture_uses_same_flow();
    3
}
fn main() {}
