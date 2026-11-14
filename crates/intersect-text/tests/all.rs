//! Handwritten test-suite root (not generated): merges the intersect-text integration tests into one
//! test binary so the suite builds and links once. Each module is a former test
//! binary of the same name; its tests are named `<module>::<test>`.
//! Modules are `pub` so items a former binary exported stay live, and shared
//! helper files are loaded once per former binary, as before the merge.
#![allow(clippy::duplicate_mod)]

#[path = "call_free_relative_v75.rs"]
pub mod call_free_relative_v75;

#[path = "contextual_first_rule_proof_gaps.rs"]
pub mod contextual_first_rule_proof_gaps;

#[path = "follow_frontier_v72.rs"]
pub mod follow_frontier_v72;

#[path = "frozen_text_catalog.rs"]
pub mod frozen_text_catalog;

#[path = "full_predicate_v68_imported.rs"]
pub mod full_predicate_v68_imported;

#[path = "general_text.rs"]
pub mod general_text;

#[path = "geometric_overflow_v78.rs"]
pub mod geometric_overflow_v78;

#[path = "model_rule_service.rs"]
pub mod model_rule_service;

#[path = "nonconflicting_repeat_v74.rs"]
pub mod nonconflicting_repeat_v74;

#[path = "nullable_fragment_current.rs"]
pub mod nullable_fragment_current;

#[path = "nullable_full_repeat_v77.rs"]
pub mod nullable_full_repeat_v77;

#[path = "old_path_guard.rs"]
pub mod old_path_guard;

#[path = "owned_rule_spans_v71.rs"]
pub mod owned_rule_spans_v71;

#[path = "prediction_v67.rs"]
pub mod prediction_v67;

#[path = "prediction_v68.rs"]
pub mod prediction_v68;

#[path = "prediction_v68_legacy_consumers.rs"]
pub mod prediction_v68_legacy_consumers;

#[path = "preparation.rs"]
pub mod preparation;

#[path = "rcore_getters.rs"]
pub mod rcore_getters;

#[path = "rcore_language.rs"]
pub mod rcore_language;

#[path = "recursive_closure_v73.rs"]
pub mod recursive_closure_v73;

#[path = "recursive_work_observation.rs"]
pub mod recursive_work_observation;

#[path = "required_object_call_v71.rs"]
pub mod required_object_call_v71;

#[path = "runtime_draft_preparation.rs"]
pub mod runtime_draft_preparation;

#[path = "source_bound_host_cache.rs"]
pub mod source_bound_host_cache;

#[path = "source_identity.rs"]
pub mod source_identity;

#[path = "whole_iteration_v76.rs"]
pub mod whole_iteration_v76;

#[path = "release_features.rs"]
pub mod release_features;

mod package_scopes;

#[path = "ordinary_analysis_budget.rs"]
pub mod ordinary_analysis_budget;

#[path = "source_bound_draft_origin.rs"]
pub mod source_bound_draft_origin;

#[path = "definitions/lexer.rs"]
pub mod definitions_lexer;
#[path = "definitions/xtext_syntax.rs"]
pub mod definitions_xtext_syntax;
