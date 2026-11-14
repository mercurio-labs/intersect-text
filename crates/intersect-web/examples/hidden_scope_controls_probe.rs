//! Verification-only; identical controls execute natively and as zero-import WASM.
#[path = "../tests/hidden_scope.rs"]
mod controls;
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    controls::verify_all();
    println!("hidden_scope_controls_pass");
}
#[cfg(target_arch = "wasm32")]
fn main() {}
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_hidden_scope() -> u32 {
    controls::verify_all();
    1
}
