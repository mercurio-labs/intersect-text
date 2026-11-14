#[path = "../shared/xtext_inferred_contexts.rs"]
mod controls;
fn main() {
    println!(
        "{} native inferred provider-context groups passed; full qualification remains open.",
        controls::verify()
    );
}
#[unsafe(no_mangle)]
pub extern "C" fn verify_inferred_contexts() -> u32 {
    controls::verify() as u32
}
