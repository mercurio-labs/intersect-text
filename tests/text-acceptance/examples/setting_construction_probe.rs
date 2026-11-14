//! Verification-only WASM entry point for the same construction assertions run natively.
//! This does not install a provider in the production web runtime.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/setting_construction.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/source_settings.rs"]
mod source_controls;

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_setting_construction() -> u32 {
    controls::construction_plan_matches_direct_checked_graph_and_preserves_spans();
    controls::construction_failure_and_every_work_boundary_roll_back_all_objects();
    controls::append_uses_existing_package_projection_and_checks_final_collection();
    controls::write_only_append_constructs_a_graph_without_storing_the_computed_feature();
    controls::construction_is_shared_by_basic_transactions_and_versioned_forks();
    controls::construction_during_linking_preserves_strict_reference_boundaries();
    source_controls::exact_contract_executes_and_every_budget_cutoff_rolls_back();
    source_controls::stale_forged_and_incomplete_contracts_never_invoke_provider();
    source_controls::changing_provider_contract_rolls_back_planning_and_readback();
    source_controls::inherited_member_keeps_receiver_and_declaration_identities_distinct();
    source_controls::grammar_admission_requires_exact_sources_and_keeps_execution_closed();
    source_controls::assigned_current_admission_uses_the_same_exact_member_contract();
    source_controls::source_bound_linking_policy_preserves_strict_publication_and_rollback();
    source_controls::explicit_text_provider_executes_and_missing_or_drifting_selection_rejects();
    source_controls::computed_current_append_builds_nested_tree_and_rejects_without_leaks();
    source_controls::composed_settings_bind_inherited_sources_and_execute_shared_provider();
    source_controls::package_settings_recompile_requires_explicit_matching_contracts();
    source_controls::computed_contracts_bind_after_explicit_ecore_context_preparation();
    18
}

fn main() {}
