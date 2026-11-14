//! Verification-only entry point sharing native and WASM assertions.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_explicit_parser.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_explicit_parser() -> u32 {
    controls::explicit_parent_parser_dispatch_and_replay();
    controls::retained_parent_parser_keeps_contextual_nested_calls();
    controls::parser_explicit_terminal_visibility_and_missing_targets();
    controls::explicit_inferred_model_fragment_and_source_limits();
    controls::parent_cardinality_matches_pinned_semantic_rejection();
    controls::current_profile_contracts();
    6
}
fn main() {}
