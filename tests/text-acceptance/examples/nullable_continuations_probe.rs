#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_nullable_continuations.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_nullable_continuations() -> u32 {
    controls::nested_nullable_fragments_keep_prediction_and_model_contracts();
    controls::nullable_wrappers_do_not_hide_ambiguity_or_nonprogress();
    2
}
fn main() {}
