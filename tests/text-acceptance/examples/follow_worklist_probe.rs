#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_follow_worklist.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_follow_worklist() -> u32 {
    controls::changed_callers_preserve_model_and_old_profile_budget();
    controls::ambiguous_fragment_continuations_remain_rejected();
    2
}
fn main() {}
