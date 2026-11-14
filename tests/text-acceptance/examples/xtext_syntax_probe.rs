//! Verification-only AST importer probe using identical native and WASM assertions.
#[cfg(target_arch = "wasm32")]
#[path = "../../../crates/intersect-text/tests/definitions/xtext_syntax.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_xtext_syntax() -> u32 {
    controls::pinned_xtext_grammar_and_terminals_import();
    controls::complete_headers_and_nonexecuted_constructs_are_retained();
    controls::malformed_syntax_and_source_identity_fail_closed();
    controls::syntax_limits_and_illegal_predicates_reject();
    controls::cached_upstream_syntax_observations();
    controls::syntax_strings_preserve_all_utf16_boundaries();
    controls::annotations_arguments_and_source_envelope_have_spans();
    controls::independent_node_classes_and_lexical_boundaries();
    controls::independent_attributes_containment_and_reference_spellings();
    9
}
fn main() {}
