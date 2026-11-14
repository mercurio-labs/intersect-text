#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_cached_proof_closures.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_guarded_continuations.rs"]
mod guards;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_cached_proof_closures() -> u32 {
    controls::closure_reuse_preserves_decisions_models_and_limits();
    controls::short_forms_match_uncached_proof_execution();
    guards::guarded_optional_prefix_preserves_models_and_old_rejection();
    guards::guards_do_not_justify_ambiguous_continuations();
    4
}
fn main() {}
