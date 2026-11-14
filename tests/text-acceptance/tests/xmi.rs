//! Consolidated qualification suite.
#![allow(clippy::duplicate_mod)]

mod stores {
    #[allow(unused_imports)]
    pub use intersect_model_test_support::stores::*;
}

#[path = "xmi/compatibility.rs"]
pub mod xmi_compatibility;

#[path = "xmi/rcore_library.rs"]
pub mod xmi_rcore_library;

#[path = "xmi/unsettable.rs"]
pub mod xmi_unsettable;

#[path = "xmi/upstream.rs"]
pub mod xmi_upstream;

#[path = "xmi/parity/frozen.rs"]
pub mod frozen;
