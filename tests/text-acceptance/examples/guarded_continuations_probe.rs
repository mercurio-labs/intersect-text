#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_guarded_continuations.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_guarded_continuations() -> u32 {
    controls::guarded_optional_prefix_preserves_models_and_old_rejection();
    controls::guards_do_not_justify_ambiguous_continuations();
    2
}
fn main() {}
