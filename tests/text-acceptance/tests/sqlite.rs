//! Consolidated qualification suite.
#![allow(clippy::duplicate_mod)]

mod stores {
    #[allow(unused_imports)]
    pub use intersect_model_test_support::stores::*;
}

#[path = "sqlite/applied_model.rs"]
pub mod sqlite_applied_model;

#[path = "sqlite/assignment_state.rs"]
pub mod sqlite_assignment_state;

#[path = "sqlite/constructor_map.rs"]
pub mod sqlite_constructor_map;

#[path = "sqlite/diagnostic_chain.rs"]
pub mod sqlite_diagnostic_chain;

#[path = "sqlite/enumerator.rs"]
pub mod sqlite_enumerator;

#[path = "sqlite/eobject_construction.rs"]
pub mod sqlite_eobject_construction;

#[path = "sqlite/eobject_grammar.rs"]
pub mod sqlite_eobject_grammar;

#[path = "sqlite/feature_maps.rs"]
pub mod sqlite_feature_maps;

#[path = "sqlite/feature_references.rs"]
pub mod sqlite_feature_references;

#[path = "sqlite/generic_identity.rs"]
pub mod sqlite_generic_identity;

#[path = "sqlite/generic_reference_context.rs"]
pub mod sqlite_generic_reference_context;

#[path = "sqlite/invocation_error.rs"]
pub mod sqlite_invocation_error;

#[path = "sqlite/model_api.rs"]
pub mod sqlite_model_api;

#[path = "sqlite/nominal_collections.rs"]
pub mod sqlite_nominal_collections;

#[path = "sqlite/raw_runtime.rs"]
pub mod sqlite_raw_runtime;

#[path = "sqlite/raw_unbounded.rs"]
pub mod sqlite_raw_unbounded;

#[path = "sqlite/resource_handles.rs"]
pub mod sqlite_resource_handles;

#[path = "sqlite/runtime_lower_bounds.rs"]
pub mod sqlite_runtime_lower_bounds;

#[path = "sqlite/runtime_operations.rs"]
pub mod sqlite_runtime_operations;

#[path = "sqlite/serializable_sequence.rs"]
pub mod sqlite_serializable_sequence;

#[path = "sqlite/tree_iterator.rs"]
pub mod sqlite_tree_iterator;

#[path = "sqlite/wildcard_defaults.rs"]
pub mod sqlite_wildcard_defaults;

#[path = "sqlite/parity/frozen.rs"]
pub mod frozen;
