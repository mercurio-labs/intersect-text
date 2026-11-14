//! Advanced native language and host integration.
//!
//! Ordinary applications use [`crate::compiler`], [`crate::evaluate`] and the
//! document/editing services. Explicit source getter activation and
//! runtime-obligation execution belong here. Runtime-guarded evaluation consumes
//! checked compiler admission and the caller's finite work ledger; it does not
//! make arbitrary grammar profiles executable.
//!
//! Other extension contracts retain their owning modules:
//!
//! | Contract | Owner | Host obligation |
//! | --- | --- | --- |
//! | Linking and source-bound scope | [`crate::linking`], [`crate::rcore_scopes`] | Supply language policy and retain declaration identity |
//! | Conversion, spelling and inverse scope | [`crate::text_services`], [`crate::rule_conversion`] | Make transformations explicit and validate their results |
//! | Computed settings | [`crate::setting`] | Preserve checked ownership and mutation boundaries |
//! | Borrowed compilation and grammar admission | [`crate::preparation`], [`crate::grammar`] | Retain sources, selected profile, providers and resource limits |
//! | Inference and draft execution | [`crate::inference`], [`crate::draft`] | Distinguish mutable recipes from checked execution authority |
//!
//! These remain provisional extension APIs. They do not confer production or
//! cross-alpha compatibility guarantees.

pub use crate::bounded_runtime::{
    RuntimeGuardedEvaluation, evaluate as evaluate_runtime_guarded,
    evaluate_into as evaluate_runtime_guarded_into,
};
pub use crate::rcore_getters::RcoreGetters;
