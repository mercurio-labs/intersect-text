//! Consolidated qualification suite.
#![allow(clippy::duplicate_mod)]

mod stores {
    #[allow(unused_imports)]
    pub use intersect_model_test_support::stores::*;
}

#[path = "core/enum_values.rs"]
pub mod core_enum_values;

#[path = "core/index_policy.rs"]
pub mod core_index_policy;

#[path = "core/model_api.rs"]
pub mod core_model_api;

#[path = "core/model_identity_contract.rs"]
pub mod core_model_identity_contract;

#[path = "core/model_observation.rs"]
pub mod core_model_observation;

#[path = "core/reference_collections.rs"]
pub mod core_reference_collections;

#[path = "core/schema_storage_contract.rs"]
pub mod core_schema_storage_contract;

#[path = "core/transactions.rs"]
pub mod core_transactions;

#[path = "core/versioned_api.rs"]
pub mod core_versioned_api;

#[path = "core/watermark_transactions.rs"]
pub mod core_watermark_transactions;

#[path = "core/parity/frozen.rs"]
pub mod frozen;
