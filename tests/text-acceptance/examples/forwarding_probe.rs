#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_forwarding.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_forwarding() -> u32 {
    controls::parenthesized_forwarding_keeps_returned_identity_and_spans();
    controls::forwarding_requires_one_typed_object_on_every_path();
    2
}
fn main() {}
