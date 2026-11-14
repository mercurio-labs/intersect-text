mod driver;
pub use driver::{Request, run};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use std::cell::RefCell;
    thread_local! {static INPUT:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};static OUTPUT:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};}
    #[unsafe(no_mangle)]
    pub extern "C" fn input_buffer(length: usize) -> *mut u8 {
        if length > 4096 {
            return std::ptr::null_mut();
        }
        INPUT.with(|value| {
            let mut value = value.borrow_mut();
            value.resize(length, 0);
            value.as_mut_ptr()
        })
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn execute() -> usize {
        let result = INPUT.with(|value| {
            serde_json::from_slice::<Request>(&value.borrow())
                .map_err(|e| e.to_string())
                .and_then(run)
        });
        let value = match result {
            Ok(value) => value,
            Err(error) => serde_json::json!({"verification_error":error}),
        };
        let encoded = match serde_json::to_vec(&value) {
            Ok(value) => value,
            Err(_) => b"{\"verification_error\":\"Output serialization failed\"}".to_vec(),
        };
        OUTPUT.with(|output| {
            *output.borrow_mut() = encoded;
            output.borrow().len()
        })
    }
    #[unsafe(no_mangle)]
    pub extern "C" fn output_buffer() -> *const u8 {
        OUTPUT.with(|output| output.borrow().as_ptr())
    }
}
