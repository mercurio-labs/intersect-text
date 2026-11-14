#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_projected_prediction.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_projected_prediction() -> u32 {
    controls::projected_proof_constructs_models_with_exact_sources_replay_and_recovery();
    controls::projected_proof_keeps_ambiguity_and_left_recursion_explicit();
    controls::projected_profile_keeps_v52_string_conversion_and_scalar_boundaries();
    controls::projected_token_masks_above_64_bits_preserve_existing_models();
    controls::nary_certificates_construct_all_branches_and_preserve_replay_and_recovery();
    controls::nary_certificates_reject_ambiguity_and_left_recursion_then_recover();
    controls::filtered_repeat_constructs_literal_models_and_preserves_replay_recovery();
    controls::filtered_repeat_rejects_ambiguity_and_recursion_then_recovers();
    controls::filtered_profile_preserves_prior_projected_and_nary_models();
    controls::bounded_discovery_executes_literal_models_with_exact_sources_and_recovery();
    controls::bounded_discovery_keeps_nullable_family_and_rejects_unfenced_overlaps();
    11
}
fn main() {}
