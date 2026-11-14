mod observe;
static OUTPUT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
fn output() -> &'static Vec<u8> {
    OUTPUT.get_or_init(|| serde_json::to_vec(&observe::observe()).unwrap())
}
#[unsafe(no_mangle)]
pub extern "C" fn qualify() -> usize {
    output().len()
}
#[unsafe(no_mangle)]
pub extern "C" fn result_ptr() -> *const u8 {
    output().as_ptr()
}
