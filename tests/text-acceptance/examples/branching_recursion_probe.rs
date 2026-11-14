#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_branching_recursion.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_branching_recursion() -> u32 {
    controls::depth_first_analysis_preserves_recursive_models_and_limits();
    1
}
fn main() {}
