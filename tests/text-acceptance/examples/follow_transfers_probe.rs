#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_follow_transfers.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_follow_transfers() -> u32 {
    controls::transfers_preserve_models_and_old_profile_budget();
    controls::ambiguous_fragment_continuations_remain_rejected();
    controls::grouped_assignment_capability_is_inherited();
    3
}
fn main() {}
