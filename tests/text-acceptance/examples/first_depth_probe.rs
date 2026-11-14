#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_first_depth.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_first_depth() -> u32 {
    controls::deep_rule_first_sets_execute_with_separate_depth_bounds();
    1
}
fn main() {}
