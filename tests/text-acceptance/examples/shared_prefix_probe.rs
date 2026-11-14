#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_shared_prefix.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_shared_prefix() -> u32 {
    controls::shared_prefix_selection_and_rejections();
    controls::nullable_alternative_uses_follow_and_rejects_ambiguity();
    controls::pinned_argument_declarations_with_shared_identifier_prefix();
    3
}
fn main() {}
