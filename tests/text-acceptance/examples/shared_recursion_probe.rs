#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_shared_recursion.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_shared_recursion() -> u32 {
    controls::shared_recursion_preserves_models_and_earlier_budget_boundary();
    controls::short_shared_recursion_matches_existing_predicate_policy();
    controls::retained_independent_recursive_oracle_remains_satisfied();
    3
}
fn main() {}
