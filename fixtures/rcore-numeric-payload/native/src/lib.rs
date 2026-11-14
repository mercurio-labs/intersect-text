#[path = "../../support/lifecycle.rs"]
pub mod lifecycle;
#[path = "../../support/typed.rs"]
pub mod typed;
pub fn observe() -> serde_json::Value {
    serde_json::json!({"lifecycle":lifecycle::capture(),"typed":typed::capture()})
}
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn qualification() -> u64 {
    let raw = serde_json::to_vec(&observe()).unwrap().into_boxed_slice();
    let result = ((raw.as_ptr() as u64) << 32) | raw.len() as u64;
    std::mem::forget(raw);
    result
}
