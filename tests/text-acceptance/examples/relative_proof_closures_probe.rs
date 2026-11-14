#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_relative_proof_closures.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_relative_proof_closures() -> u32 {
    controls::relative_closures_keep_exact_caller_suffixes_and_model_contracts();
    controls::relative_closures_preserve_ambiguity_rejection();
    2
}
fn main() {}
