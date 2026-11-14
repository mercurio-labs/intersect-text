#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_repeat_follow.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_repeat_follow() -> u32 {
    controls::repeat_and_skip_use_following_tokens_without_speculative_writes();
    controls::ambiguous_or_nonprogressing_repeats_remain_rejected();
    2
}
fn main() {}
