#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_keyword_assignments.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_keyword_assignments() -> u32 {
    controls::keyword_values_use_existing_assignment_and_replay();
    controls::keyword_assignment_checks_type_and_multiplicity();
    2
}
fn main() {}
