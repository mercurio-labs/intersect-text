#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_forwarded_fragments.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_forwarded_fragments() -> u32 {
    controls::forwarded_objects_keep_identity_through_fragment_writes();
    controls::missing_owner_replacement_and_fragment_type_mismatch_reject();
    2
}
fn main() {}
