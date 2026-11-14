//! Verification-only native/WASM shared assertions.
#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_typed_overrides.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_typed_overrides() -> u32 {
    controls::covariant_model_overrides_use_shared_conformance();
    controls::typed_fragment_overrides_keep_exact_return_identity();
    controls::datatype_overrides_keep_exact_scalar_type();
    controls::enum_overrides_keep_literal_declaration_identity();
    controls::enum_override_rejects_distinct_classifier_with_same_literals();
    controls::current_profile_contracts();
    6
}
fn main() {}
