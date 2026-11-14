#[cfg(target_arch = "wasm32")]
#[path = "../tests/language/xtext_grammar_libraries.rs"]
mod controls;
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn verify_grammar_libraries() -> u32 {
    controls::enum_datatype_and_fragment_libraries_execute_in_child_context();
    controls::libraries_do_not_gain_standalone_executable_entries();
    controls::invalid_shadowed_library_declarations_still_reject();
    3
}
fn main() {}
