#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_implicit_construction.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_implicit_construction() -> u32 {
    controls::first_assignments_share_construction_defaults_and_replay();
    controls::implicit_rejection_rolls_back_constructed_objects_and_links();
    controls::implicit_admission_preserves_concrete_type_and_flow_controls();
    controls::implicit_and_explicit_roots_share_initializers();
    4
}
fn main() {}
