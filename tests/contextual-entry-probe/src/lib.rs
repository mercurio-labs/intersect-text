//! Verification-only WASM host. No public application protocol or domain semantics.
#[path = "../../../crates/intersect-text/tests/support/first_rule_proof_gap_observations.rs"]
mod observations;
use std::sync::Mutex;
static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
#[unsafe(no_mangle)]
pub extern "C" fn run_controls() -> usize {
    let result = match observations::suite() {
        Ok(value) => value,
        Err(error) => serde_json::json!({"error":error}),
    };
    let bytes = match serde_json::to_vec(&result) {
        Ok(bytes) => bytes,
        Err(_) => return 0,
    };
    match OUTPUT.lock() {
        Ok(mut output) => {
            *output = bytes;
            output.len()
        }
        Err(_) => 0,
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn output_buffer() -> *const u8 {
    match OUTPUT.lock() {
        Ok(output) => output.as_ptr(),
        Err(_) => std::ptr::null(),
    }
}
