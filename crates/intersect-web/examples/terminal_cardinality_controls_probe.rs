//! Verification-only; execute identical actual native and zero-import WASM controls.
#[path = "../tests/terminal_cardinality.rs"]
mod controls;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    controls::verify_all();
    println!("terminal_cardinality_controls_pass");
}
#[cfg(target_arch = "wasm32")]
fn main() {}
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_terminal_cardinality() -> u32 {
    controls::verify_all();
    1
}
