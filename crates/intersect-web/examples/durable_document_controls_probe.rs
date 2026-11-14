//! Verification-only; identical public controls on native and zero-import WASM.
#[path = "../tests/durable_document.rs"]
mod controls;
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    controls::verify_all();
    println!("durable_document_controls_pass");
}
#[cfg(target_arch = "wasm32")]
fn main() {}
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_durable_document() -> u32 {
    controls::verify_all();
    1
}
