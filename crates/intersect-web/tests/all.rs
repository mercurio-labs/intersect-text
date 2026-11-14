//! Handwritten test-suite root (not generated): merges the intersect-web integration tests into one
//! test binary so the suite builds and links once. Each module is a former test
//! binary of the same name; its tests are named `<module>::<test>`.
//! Modules are `pub` so items a former binary exported stay live, and shared
//! helper files are loaded once per former binary, as before the merge.
#![allow(clippy::duplicate_mod)]

// Compare the complete JSON against its pre-migration snapshot digest.
fn assert_store_abi(response: &serde_json::Value, expected: &str) {
    let json = response.to_string();
    let identity =
        intersect_model::definitions::bundle::SourceId::for_text("memory:store-abi", &json);
    assert_eq!(identity.sha256, expected, "{json}");
}

#[path = "applied_collection_operations.rs"]
pub mod applied_collection_operations;

#[path = "applied_conversions.rs"]
pub mod applied_conversions;

#[path = "applied_model.rs"]
pub mod applied_model;

#[path = "applied_operations.rs"]
pub mod applied_operations;

#[path = "bundle.rs"]
pub mod bundle;

#[path = "collections.rs"]
pub mod collections;

#[path = "constructor_map.rs"]
pub mod constructor_map;

#[path = "constructor_policies.rs"]
pub mod constructor_policies;

#[path = "contextual_first_rule.rs"]
pub mod contextual_first_rule;

#[path = "contextual_inference_compat.rs"]
pub mod contextual_inference_compat;

#[path = "contextual_program_adapter.rs"]
pub mod contextual_program_adapter;

#[path = "conversions.rs"]
pub mod conversions;

#[path = "converted_query_columns.rs"]
pub mod converted_query_columns;

#[path = "diagnostic_chain.rs"]
pub mod diagnostic_chain;

#[path = "diagnostic_identities.rs"]
pub mod diagnostic_identities;

#[path = "durable_document.rs"]
pub mod durable_document;

#[path = "durable_rgb.rs"]
pub mod durable_rgb;

#[path = "ecore_members.rs"]
pub mod ecore_members;

#[path = "ecore_scripting.rs"]
pub mod ecore_scripting;

#[path = "enumerator.rs"]
pub mod enumerator;

#[path = "eobject_construction.rs"]
pub mod eobject_construction;

#[path = "evaluation_limits.rs"]
pub mod evaluation_limits;

#[path = "feature_maps.rs"]
pub mod feature_maps;

#[path = "feature_references.rs"]
pub mod feature_references;

#[path = "function_jobs.rs"]
pub mod function_jobs;

#[path = "generic_reference_context.rs"]
pub mod generic_reference_context;

#[path = "generic_signatures.rs"]
pub mod generic_signatures;

#[path = "grouped_negation.rs"]
pub mod grouped_negation;

#[path = "hidden_scope.rs"]
pub mod hidden_scope;

#[path = "imported_assignment_v66.rs"]
pub mod imported_assignment_v66;

#[path = "inference_replay.rs"]
pub mod inference_replay;

#[path = "inferred_computed_collections.rs"]
pub mod inferred_computed_collections;

#[path = "inferred_computed_contexts.rs"]
pub mod inferred_computed_contexts;

#[path = "inferred_contexts.rs"]
pub mod inferred_contexts;

#[path = "inferred_provider_selection.rs"]
pub mod inferred_provider_selection;

#[path = "inferred_unified_provider_selection.rs"]
pub mod inferred_unified_provider_selection;

#[path = "invocation_error.rs"]
pub mod invocation_error;

#[path = "library_text.rs"]
pub mod library_text;

#[path = "literal_negation.rs"]
pub mod literal_negation;

#[path = "method_provider_bounds.rs"]
pub mod method_provider_bounds;

#[path = "model_text.rs"]
pub mod model_text;

#[path = "named_entries.rs"]
pub mod named_entries;

#[path = "ordinary_parameters.rs"]
pub mod ordinary_parameters;

#[path = "parameter_origins.rs"]
pub mod parameter_origins;

#[path = "prediction_observation.rs"]
pub mod prediction_observation;

#[path = "query.rs"]
pub mod query;

#[path = "rcore_behavior.rs"]
pub mod rcore_behavior;

#[path = "rcore_behavior_functions.rs"]
pub mod rcore_behavior_functions;

#[path = "rcore_behavior_scalar.rs"]
pub mod rcore_behavior_scalar;

#[path = "rcore_direct.rs"]
pub mod rcore_direct;

#[path = "resource_handles.rs"]
pub mod resource_handles;

#[path = "resource_lifecycle.rs"]
pub mod resource_lifecycle;

#[path = "runtime_operation_providers.rs"]
pub mod runtime_operation_providers;

#[path = "runtime_operations.rs"]
pub mod runtime_operations;

#[path = "scripting.rs"]
pub mod scripting;

#[path = "terminal_cardinality.rs"]
pub mod terminal_cardinality;

#[path = "terminal_negation.rs"]
pub mod terminal_negation;

#[path = "tree_iterator.rs"]
pub mod tree_iterator;

#[path = "until_operands.rs"]
pub mod until_operands;

#[path = "wildcard_defaults.rs"]
pub mod wildcard_defaults;
