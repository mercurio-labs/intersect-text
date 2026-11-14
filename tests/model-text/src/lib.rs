//! Test-only raw WASM consumer. It executes the identical assertion controls
//! as the native integration target; it provides no model/language algorithms.
//! Panics in imported test assertions fail the corresponding host control.
#[cfg(target_arch = "wasm32")]
#[path = "../../../crates/intersect-text/tests/general_text.rs"]
mod cases;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::cases;
    type Case = (&'static str, fn());
    static CASES: &[Case] = &[
        ("native_v82_unordered_modifiers_enforce_membership_and_preserve_authored_order",cases::native_v82_unordered_modifiers_enforce_membership_and_preserve_authored_order),
        ("native_v82_unordered_repetitions_are_contiguous_and_empty_entries_survive",cases::native_v82_unordered_repetitions_are_contiguous_and_empty_entries_survive),
        ("native_v82_nested_unordered_groups_preserve_inner_members_as_a_unit",cases::native_v82_nested_unordered_groups_preserve_inner_members_as_a_unit),
        ("native_v82_nullable_unordered_members_skip_empty_paths_without_reordering_groups",cases::native_v82_nullable_unordered_members_skip_empty_paths_without_reordering_groups),
        ("native_v82_unordered_containment_references_and_limits_use_shared_services",cases::native_v82_unordered_containment_references_and_limits_use_shared_services),
        ("native_v82_unordered_inferred_metamodel_replays_original_source",cases::native_v82_unordered_inferred_metamodel_replays_original_source),

        ("native_v81_builtin_terminal_and_parser_returns_share_all_scalar_codecs", cases::native_v81_builtin_terminal_and_parser_returns_share_all_scalar_codecs),
        ("grammar_defined_hidden_separators_generate_and_preserve_without_whitespace_assumptions", cases::grammar_defined_hidden_separators_generate_and_preserve_without_whitespace_assumptions),
        ("empty_object_entries_generate_and_retain_resources_through_root_lifecycle", cases::empty_object_entries_generate_and_retain_resources_through_root_lifecycle),
        ("negated_terminal_witnesses_include_unicode_complements", cases::negated_terminal_witnesses_include_unicode_complements),
        ("moving_syntax_into_a_different_hidden_context_retains_comments_where_legal", cases::moving_syntax_into_a_different_hidden_context_retains_comments_where_legal),
        ("transient_attributes_and_subtrees_preserve_runtime_state_through_text_and_history",cases::transient_attributes_and_subtrees_preserve_runtime_state_through_text_and_history),
        ("custom_transient_members_share_persistent_graph_verification_and_stable_handles",cases::custom_transient_members_share_persistent_graph_verification_and_stable_handles),
        ("empty_gap_preservation_uses_the_parser_to_insert_required_separators",cases::empty_gap_preservation_uses_the_parser_to_insert_required_separators),
        ("unassigned_text_uses_model_values_and_retained_source_without_bypassing_verification",cases::unassigned_text_uses_model_values_and_retained_source_without_bypassing_verification),
        ("independent_actions_references_and_alternatives_generate_without_callbacks",cases::independent_actions_references_and_alternatives_generate_without_callbacks),
        ("generic_scalar_and_rename_keep_authored_order_quotes_comments_and_crlf",cases::generic_scalar_and_rename_keep_authored_order_quotes_comments_and_crlf),
        ("generic_subtree_creation_deletion_reordering_and_optional_fields_keep_comments",cases::generic_subtree_creation_deletion_reordering_and_optional_fields_keep_comments),
        ("expression_capture_actions_and_recursive_delegation_round_trip",cases::expression_capture_actions_and_recursive_delegation_round_trip),
        ("datatype_enum_and_locally_significant_fragment_round_trip",cases::datatype_enum_and_locally_significant_fragment_round_trip),
        ("general_plans_are_stale_checked_and_search_is_finite",cases::general_plans_are_stale_checked_and_search_is_finite),
        ("duplicate_collection_values_keep_distinct_spellings_and_changed_strings_keep_quotes",cases::duplicate_collection_values_keep_distinct_spellings_and_changed_strings_keep_quotes),
        ("generic_cross_resource_rename_preserves_unrelated_bytes_and_ids",cases::generic_cross_resource_rename_preserves_unrelated_bytes_and_ids),
        ("grouped_containment_and_keyword_alternatives_share_grammar_inversion",cases::grouped_containment_and_keyword_alternatives_share_grammar_inversion),
        ("explicit_forward_and_inverse_scope_services_share_verification",cases::explicit_forward_and_inverse_scope_services_share_verification),
        ("selected_serialization_entry_is_verified_with_the_same_parser_context",cases::selected_serialization_entry_is_verified_with_the_same_parser_context),
        ("preserving_sessions_require_their_original_scope_context_through_history",cases::preserving_sessions_require_their_original_scope_context_through_history),
        ("custom_nominal_conversion_is_shared_by_canonical_and_preserving_contexts",cases::custom_nominal_conversion_is_shared_by_canonical_and_preserving_contexts),
        ("owning_native_services_retain_scope_through_edits_and_history",cases::owning_native_services_retain_scope_through_edits_and_history),
        ("cross_resource_subtree_moves_preserve_comments_and_stable_handles",cases::cross_resource_subtree_moves_preserve_comments_and_stable_handles),
        ("resource_creation_deletion_and_root_replacement_restore_exact_sources",cases::resource_creation_deletion_and_root_replacement_restore_exact_sources),
        ("custom_formatting_is_verified_and_preservation_keeps_existing_layout",cases::custom_formatting_is_verified_and_preservation_keeps_existing_layout),
        ("grammar_assigned_computed_values_use_the_installed_inverse_getter",cases::grammar_assigned_computed_values_use_the_installed_inverse_getter),
    ];
    #[unsafe(no_mangle)]
    pub extern "C" fn intersect_model_text_case_count() -> u32 {
        CASES.len() as u32
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn intersect_model_text_case_name(index: u32) -> u64 {
        let Some((name, _)) = CASES.get(index as usize) else {
            return 0;
        };
        ((name.len() as u64) << 32) | (name.as_ptr() as u32 as u64)
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn intersect_model_text_run_case(index: u32) -> i32 {
        let Some((_, execute)) = CASES.get(index as usize) else {
            return 1;
        };
        execute();
        0
    }
}
