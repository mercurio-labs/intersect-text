//! Verification-only shared enum terminal controls.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_enum_datatype_parsers.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_enum_terminals() -> u32 {
    controls::enum_and_parser_overrides_preserve_nominal_values();
    controls::enum_parser_uses_lexical_conversion_and_retained_parent_calls();
    controls::enum_parser_category_rejections_remain_explicit();
    controls::enum_terminals_preserve_nominal_override_values();
    controls::enum_terminal_categories_and_types_reject_invalid_overrides();
    controls::enum_terminal_parent_calls_and_parser_calls_share_codec();
    controls::retained_parent_partial_overlap_keeps_child_token_priority();
    controls::unreachable_retained_terminal_rejects_at_admission();
    controls::reachable_terminal_overlap_preserves_priority_and_values();
    controls::lexical_reachability_proves_union_and_reports_inconclusive_budget();
    controls::absent_hidden_policy_is_an_explicit_native_v48_gap();
    controls::default_hidden_policy_matches_pinned_flattening();
    controls::default_hidden_policy_preserves_missing_parent_and_invalid_token_rejections();
    controls::current_hidden_policy_contracts();
    14
}
fn main() {}
