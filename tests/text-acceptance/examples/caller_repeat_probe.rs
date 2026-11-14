#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_caller_repeat.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_caller_repeat() -> u32 {
    controls::caller_suffixes_resolve_fragment_repeat_decisions();
    controls::ambiguous_caller_partition_and_nonprogress_remain_rejected();
    2
}
fn main() {}
