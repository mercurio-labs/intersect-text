#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_grouped_objects.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_grouped_objects() -> u32 {
    controls::scalar_and_many_grouped_assignments_preserve_containment();
    controls::grouped_rhs_types_cardinality_and_writability_stay_checked();
    2
}
fn main() {}
