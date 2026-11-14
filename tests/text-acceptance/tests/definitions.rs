//! Consolidated qualification suite.
#![allow(clippy::duplicate_mod)]

#[path = "definitions/bundle.rs"]
pub mod definitions_bundle;

#[path = "definitions/distribution.rs"]
pub mod definitions_distribution;

#[path = "definitions/ecore_migration.rs"]
pub mod definitions_ecore_migration;

#[path = "definitions/ecore_package.rs"]
pub mod definitions_ecore_package;

#[path = "definitions/package_source_policy.rs"]
pub mod definitions_package_source_policy;

#[path = "definitions/xtext_inference.rs"]
pub mod definitions_xtext_inference;

#[path = "definitions/xtext_overrides.rs"]
pub mod definitions_xtext_overrides;

#[path = "definitions/xtext_profiles.rs"]
pub mod definitions_xtext_profiles;

#[path = "definitions/xtext_punctuation.rs"]
pub mod definitions_xtext_punctuation;

#[path = "definitions/xtext_source_limits.rs"]
pub mod definitions_xtext_source_limits;

#[path = "definitions/xtext_sources.rs"]
pub mod definitions_xtext_sources;
