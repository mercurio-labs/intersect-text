#[path = "../shared/xtext_inference_archives.rs"]
mod archives;
#[path = "../shared/xtext_inference_composition.rs"]
mod composition;
#[path = "../shared/xtext_metamodel_inference.rs"]
mod controls;
#[path = "../shared/xtext_fragment_runtime.rs"]
mod fragment_runtime;
#[path = "../shared/xtext_inference_fragments.rs"]
mod fragments;
#[path = "../shared/xtext_inference_replay.rs"]
mod package_replay;
#[path = "../shared/xtext_parameters.rs"]
mod parameters;
#[path = "../shared/xtext_inference_terminal.rs"]
mod terminal;
fn main() {
    println!(
        "{} inferred metamodel/native execution/replay groups passed; full X03 remains open.",
        controls::verify()
            + composition::verify()
            + terminal::verify()
            + fragments::verify()
            + fragment_runtime::verify()
            + package_replay::verify()
            + archives::verify()
    );
}
#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn verify_xtext_metamodel_inference() -> u32 {
    (controls::verify()
        + composition::verify()
        + terminal::verify()
        + fragments::verify()
        + fragment_runtime::verify()
        + package_replay::verify()
        + archives::verify()) as u32
}
#[cfg(test)]
#[test]
fn frozen_native_inference_matrix() {
    controls::frozen_matrix();
}
#[cfg(test)]
#[test]
fn inferred_model_execution_and_replay() {
    controls::source_execution_and_replay();
}
#[cfg(test)]
#[test]
fn explicit_source_policy_survives_inference() {
    controls::larger_source_policy();
}

#[cfg(test)]
#[test]
fn frozen_composed_inference_matrix() {
    composition::frozen_matrix();
}

#[cfg(test)]
#[test]
fn composed_model_execution_and_source_replay() {
    composition::execution_and_replay();
}

#[cfg(test)]
#[test]
fn frozen_terminal_library_matrix() {
    terminal::frozen_matrix();
}
#[cfg(test)]
#[test]
fn terminal_library_models_and_source_replay() {
    terminal::execution_and_replay();
}

#[cfg(test)]
#[test]
fn frozen_parser_fragment_matrix() {
    fragments::frozen_matrix();
}

#[cfg(test)]
#[test]
fn parser_fragment_models_and_source_replay() {
    fragments::execution_and_replay();
}

#[cfg(test)]
#[test]
fn generated_fragment_parser_outcomes_and_source_replay() {
    fragment_runtime::execution_and_replay();
}

#[cfg(test)]
#[test]
fn inferred_package_replay_keeps_its_text_grammar() {
    package_replay::source_authority();
}

#[cfg(test)]
#[test]
fn inferred_source_archives_rebuild_and_enforce_integrity_and_host_policy() {
    archives::integrity_and_policy();
}

#[cfg(test)]
#[test]
fn frozen_parameter_inference_matrix() {
    parameters::frozen_inference_matrix();
}

#[cfg(test)]
#[test]
fn frozen_parameter_runtime_matrix() {
    parameters::frozen_runtime_matrix();
}

#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn verify_xtext_parameters() -> u32 {
    parameters::verify() as u32
}

#[cfg(test)]
#[test]
fn parameter_empty_resources_and_forwarding() {
    parameters::empty_resources_and_forwarding();
}

#[path = "../shared/xtext_imported_parameters.rs"]
mod imported_parameters;
#[cfg(test)]
#[test]
fn frozen_ordinary_parameter_default_resources() {
    imported_parameters::default_resources();
}
#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn verify_xtext_imported_parameter_defaults() -> u32 {
    imported_parameters::default_resources();
    imported_parameters::authority_and_admission();
    2
}

#[cfg(test)]
#[test]
fn ordinary_parameter_source_authority_and_admission() {
    imported_parameters::authority_and_admission();
}

#[path = "../shared/xtext_named_entries.rs"]
mod named_entries;
#[cfg(test)]
#[test]
fn original_named_entry_contexts() {
    named_entries::original_named_matrix();
}
#[cfg(test)]
#[test]
fn supplemental_entry_contexts_and_scalar_returns() {
    named_entries::entry_contexts_and_returns();
}
#[cfg(test)]
#[test]
fn inferred_named_entry_counterparts() {
    named_entries::inferred_counterparts();
}
#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn verify_xtext_named_entries() -> u32 {
    named_entries::verify() as u32
}

#[cfg(test)]
#[test]
fn named_entry_authority_and_budgets() {
    named_entries::authority_and_budgets();
}

#[path = "../shared/xtext_ordinary_archives.rs"]
mod ordinary_archives;
#[cfg(test)]
#[test]
fn ordinary_parameter_archives_rebuild_frozen_sources_and_models() {
    ordinary_archives::frozen_roundtrips();
}
#[cfg(test)]
#[test]
fn ordinary_archives_preserve_historical_bytes_and_rebuild_through_both_importers() {
    ordinary_archives::integrity_and_legacy();
}
#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn verify_xtext_ordinary_archives() -> u32 {
    ordinary_archives::verify() as u32
}

#[path = "../shared/xtext_guard_sequences.rs"]
mod guard_sequences;
#[cfg(test)]
#[test]
fn ordinary_guard_sequences_and_source_replay() {
    guard_sequences::ordinary_resources_and_replay();
}
#[cfg(test)]
#[test]
fn inferred_guard_sequences_and_source_replay() {
    guard_sequences::inferred_resources_and_replay();
}
#[cfg_attr(not(test), unsafe(no_mangle))]
pub extern "C" fn verify_xtext_guard_sequences() -> u32 {
    guard_sequences::ordinary_resources_and_replay();
    guard_sequences::inferred_resources_and_replay();
    2
}
