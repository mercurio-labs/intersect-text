//! Verification-only native/WASM shared assertions.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_terminal_overrides.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_terminal_overrides() -> u32 {
    controls::inherited_calls_select_overridden_terminals_and_fragments();
    controls::incompatible_or_orphan_overrides_and_shadowed_errors_reject();
    controls::three_level_sources_and_aggregate_limits_remain_exact();
    controls::independent_contextual_binding_observations();
    controls::explicit_call_spellings_have_independent_scope_evidence();
    controls::explicit_ancestor_terminal_programs_execute();
    controls::explicit_ancestor_failures_restore_schema();
    controls::explicit_three_level_context_and_named_super();
    controls::current_profile_contracts();
    9
}
fn main() {}
