//! Consolidated qualification suite.
#![allow(clippy::duplicate_mod)]

mod stores {
    #[allow(unused_imports)]
    pub use intersect_model_test_support::stores::*;
}

#[path = "language/applied_model.rs"]
pub mod language_applied_model;

#[path = "language/applied_provider_bounds.rs"]
pub mod language_applied_provider_bounds;

#[path = "language/class_conversion.rs"]
pub mod language_class_conversion;

#[path = "language/constructor_map.rs"]
pub mod language_constructor_map;

#[path = "language/constructor_wildcards.rs"]
pub mod language_constructor_wildcards;

#[path = "language/conversion.rs"]
pub mod language_conversion;

#[path = "language/datatype_inventory.rs"]
pub mod language_datatype_inventory;

#[path = "language/enumerator_conversion.rs"]
pub mod language_enumerator_conversion;

#[path = "language/eobject_construction.rs"]
pub mod language_eobject_construction;

#[path = "language/generic_conversion.rs"]
pub mod language_generic_conversion;

#[path = "language/generic_feature_matrix.rs"]
pub mod language_generic_feature_matrix;

#[path = "language/generic_feature_provider_matrix.rs"]
pub mod language_generic_feature_provider_matrix;

#[path = "language/generic_package.rs"]
pub mod language_generic_package;

#[path = "language/generic_reference_context.rs"]
pub mod language_generic_reference_context;

#[path = "language/initialization.rs"]
pub mod language_initialization;

#[path = "language/limits.rs"]
pub mod language_limits;

#[path = "language/operation_policy_qualification.rs"]
pub mod language_operation_policy_qualification;

#[path = "language/raw_attribute_applications.rs"]
pub mod language_raw_attribute_applications;

#[path = "language/raw_attribute_text.rs"]
pub mod language_raw_attribute_text;

#[path = "language/raw_runtime.rs"]
pub mod language_raw_runtime;

#[path = "language/raw_top_linking.rs"]
pub mod language_raw_top_linking;

#[path = "language/raw_unbounded.rs"]
pub mod language_raw_unbounded;

#[path = "language/rcore_construction.rs"]
pub mod language_rcore_construction;

#[path = "language/rcore_library_rules.rs"]
pub mod language_rcore_library_rules;

#[path = "language/rcore_provider_contracts.rs"]
pub mod language_rcore_provider_contracts;

#[path = "language/runtime_operation_providers.rs"]
pub mod language_runtime_operation_providers;

#[path = "language/scalar_references.rs"]
pub mod language_scalar_references;

#[path = "language/serializable_sequence.rs"]
pub mod language_serializable_sequence;

#[path = "language/setting_construction.rs"]
pub mod language_setting_construction;

#[path = "language/settings.rs"]
pub mod language_settings;

#[path = "language/source_settings.rs"]
pub mod language_source_settings;

#[path = "language/wildcard_defaults.rs"]
pub mod language_wildcard_defaults;

#[path = "language/wildcard_references.rs"]
pub mod language_wildcard_references;

#[path = "language/workbench.rs"]
pub mod language_workbench;

#[path = "language/xtext_advanced_web.rs"]
pub mod language_xtext_advanced_web;

#[path = "language/xtext_boolean.rs"]
pub mod language_xtext_boolean;

#[path = "language/xtext_branching_recursion.rs"]
pub mod language_xtext_branching_recursion;

#[path = "language/xtext_cached_proof_closures.rs"]
pub mod language_xtext_cached_proof_closures;

#[path = "language/xtext_caller_repeat.rs"]
pub mod language_xtext_caller_repeat;

#[path = "language/xtext_category_overrides.rs"]
pub mod language_xtext_category_overrides;

#[path = "language/xtext_composed_web.rs"]
pub mod language_xtext_composed_web;

#[path = "language/xtext_current.rs"]
pub mod language_xtext_current;

#[path = "language/xtext_current_fragments.rs"]
pub mod language_xtext_current_fragments;

#[path = "language/xtext_datatypes.rs"]
pub mod language_xtext_datatypes;

#[path = "language/xtext_dispatch.rs"]
pub mod language_xtext_dispatch;

#[path = "language/xtext_double.rs"]
pub mod language_xtext_double;

#[path = "language/xtext_empty_package_imports.rs"]
pub mod language_xtext_empty_package_imports;

#[path = "language/xtext_enum_composition.rs"]
pub mod language_xtext_enum_composition;

#[path = "language/xtext_enum_datatype_parsers.rs"]
pub mod language_xtext_enum_datatype_parsers;

#[path = "language/xtext_enums.rs"]
pub mod language_xtext_enums;

#[path = "language/xtext_escaped_identifiers.rs"]
pub mod language_xtext_escaped_identifiers;

#[path = "language/xtext_explicit_parser.rs"]
pub mod language_xtext_explicit_parser;

#[path = "language/xtext_final_rules.rs"]
pub mod language_xtext_final_rules;

#[path = "language/xtext_first_depth.rs"]
pub mod language_xtext_first_depth;

#[path = "language/xtext_follow_transfers.rs"]
pub mod language_xtext_follow_transfers;

#[path = "language/xtext_follow_worklist.rs"]
pub mod language_xtext_follow_worklist;

#[path = "language/xtext_forwarded_fragments.rs"]
pub mod language_xtext_forwarded_fragments;

#[path = "language/xtext_forwarding.rs"]
pub mod language_xtext_forwarding;

#[path = "language/xtext_fragments.rs"]
pub mod language_xtext_fragments;

#[path = "language/xtext_grammar_libraries.rs"]
pub mod language_xtext_grammar_libraries;

#[path = "language/xtext_grouped_objects.rs"]
pub mod language_xtext_grouped_objects;

#[path = "language/xtext_guarded_continuations.rs"]
pub mod language_xtext_guarded_continuations;

#[path = "language/xtext_implicit_construction.rs"]
pub mod language_xtext_implicit_construction;

#[path = "language/xtext_inferred_contexts.rs"]
pub mod language_xtext_inferred_contexts;

#[path = "language/xtext_inheritance.rs"]
pub mod language_xtext_inheritance;

#[path = "language/xtext_keyword_assignments.rs"]
pub mod language_xtext_keyword_assignments;

#[path = "language/xtext_lexical_policy.rs"]
pub mod language_xtext_lexical_policy;

#[path = "language/xtext_lexical_prediction.rs"]
pub mod language_xtext_lexical_prediction;

#[path = "language/xtext_metamodel_imports.rs"]
pub mod language_xtext_metamodel_imports;

#[path = "language/xtext_mixed_actions.rs"]
pub mod language_xtext_mixed_actions;

#[path = "language/xtext_nullable_continuations.rs"]
pub mod language_xtext_nullable_continuations;

#[path = "language/xtext_overrides.rs"]
pub mod language_xtext_overrides;

#[path = "language/xtext_predicates.rs"]
pub mod language_xtext_predicates;

#[path = "language/xtext_projected_prediction.rs"]
pub mod language_xtext_projected_prediction;

#[path = "language/xtext_punctuation.rs"]
pub mod language_xtext_punctuation;

#[path = "language/xtext_recursive_fragments.rs"]
pub mod language_xtext_recursive_fragments;

#[path = "language/xtext_recursive_predicates.rs"]
pub mod language_xtext_recursive_predicates;

#[path = "language/xtext_relative_proof_closures.rs"]
pub mod language_xtext_relative_proof_closures;

#[path = "language/xtext_repeat_follow.rs"]
pub mod language_xtext_repeat_follow;

#[path = "language/xtext_rule_converters.rs"]
pub mod language_xtext_rule_converters;

#[path = "language/xtext_shared_prefix.rs"]
pub mod language_xtext_shared_prefix;

#[path = "language/xtext_shared_recursion.rs"]
pub mod language_xtext_shared_recursion;

#[path = "language/xtext_source_policy.rs"]
pub mod language_xtext_source_policy;

#[path = "language/xtext_terminal_decisions.rs"]
pub mod language_xtext_terminal_decisions;

#[path = "language/xtext_terminal_overrides.rs"]
pub mod language_xtext_terminal_overrides;

#[path = "language/xtext_terminal_parents.rs"]
pub mod language_xtext_terminal_parents;

#[path = "language/xtext_terminals.rs"]
pub mod language_xtext_terminals;

#[path = "language/xtext_typed_overrides.rs"]
pub mod language_xtext_typed_overrides;

#[path = "language/xtext_unassigned_string.rs"]
pub mod language_xtext_unassigned_string;

#[path = "language/xtext_unqualified_imports.rs"]
pub mod language_xtext_unqualified_imports;

#[path = "language/xtext_validated_syntax.rs"]
pub mod language_xtext_validated_syntax;

#[path = "language/parity/frozen.rs"]
pub mod frozen;

// Portable carriers share their native unit controls with the existing suite.
#[allow(dead_code)] // The executable ABI is exercised by explicit portable probe runs.
#[path = "../examples/xtext_metamodel_inference_probe.rs"]
pub mod portable_xtext_metamodel_inference_probe;

#[path = "language/generated_bindings.rs"]
pub mod generated_bindings;
