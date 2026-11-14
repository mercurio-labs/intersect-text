//! Verification-only entry point; identical Rust assertions run natively and in WASM.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_terminal_decisions.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_terminal_decisions() -> u32 {
    controls::shared_prefixes_nested_groups_and_range_boundaries();
    controls::nullable_branch_and_committed_failure_controls();
    controls::old_profiles_and_unsupported_nullable_repetition_stay_rejected();
    controls::wildcard_and_eof_preserve_boundaries_and_reject_trailing_input();
    controls::every_internal_prediction_budget_cutoff_is_reported();
    controls::cached_independent_lexer_observations_match();
    controls::saved_example_and_language_limits_use_the_shared_path();
    7
}
fn main() {}
