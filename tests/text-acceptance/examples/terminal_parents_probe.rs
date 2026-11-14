#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_terminal_parents.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_terminal_parents() -> u32 {
    controls::pinned_terminal_parent_executes_with_exact_identity();
    controls::terminal_parent_can_pass_through_an_intermediate_grammar();
    controls::terminal_parent_does_not_hide_invalid_or_unexecutable_sources();
    controls::current_profile_contracts();
    controls::exact_parent_dependencies_reject_ambiguity_and_cycles_without_schema_changes();
    controls::parent_dependency_diagnostics_select_the_exact_reference_bytes();
    controls::pinned_string_converter_observations_expose_native_v51_gap();
    controls::default_string_conversion_matches_oracle_with_explicit_model_boundary();
    8
}
fn main() {}
