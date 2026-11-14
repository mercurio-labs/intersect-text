//! Verification-only shared enum parser controls.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_enum_datatype_parsers.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_enum_datatype_parsers() -> u32 {
    controls::enum_and_parser_overrides_preserve_nominal_values();
    controls::enum_parser_uses_lexical_conversion_and_retained_parent_calls();
    controls::enum_parser_category_rejections_remain_explicit();
    3
}
fn main() {}
