//! Public native/WASM consumer: no application adapters or replacement semantics.
//!
//! Language entry points remain available to an independent crate:
//! ```
//! use intersect_text::{compiler, evaluate, Source, grammar::Profile};
//! ```
//! Explicit host extensions remain available without exposing their modules:
//! ```
//! use intersect_text::extensions::{RcoreGetters, RuntimeGuardedEvaluation,
//!     evaluate_runtime_guarded, evaluate_runtime_guarded_into};
//! ```
//! Activation and runtime-obligation implementation paths are private:
//! ```compile_fail
//! use intersect_text::rcore_getters::RcoreGetters;
//! ```
//! ```compile_fail
//! use intersect_text::bounded_runtime::evaluate;
//! ```
//! Internal scanner, replay and spelling helpers are not extension contracts:
//! ```compile_fail
//! use intersect_text::grammar::lex_borrowed_with_policy;
//! ```
//! ```compile_fail
//! use intersect_text::rcore_scopes::validate_retained_scope_bindings;
//! ```
//! ```compile_fail
//! use intersect_text::serialization::default_spelling_with_conversions;
//! ```
//! Cursor state and internal lexer preparation cannot be used as host APIs:
//! ```compile_fail
//! use intersect_text::grammar::token_cursor::TokenCursor;
//! ```
//! ```compile_fail
//! use intersect_text::grammar::lex_document_with_prepared_initial;
//! ```
//! ```compile_fail
//! use intersect_text::grammar::type_names::rule_class_key;
//! ```
//! Generated rule naming is internal to compilation and editing:
//! ```compile_fail
//! use intersect_text::grammar::is_generated_unordered_rule;
//! ```
//! Model inspection helpers are imported directly from Model:
//! ```compile_fail
//! use intersect_text::compiler::inspect_declarations_with_limits;
//! ```
//! Decoding a DTO cannot produce a checked preparation owner:
//! ```compile_fail
//! let _ = serde_json::from_str::<intersect_text::preparation::PreparedLanguageDefinition<'static>>("{}");
//! ```
use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    grammar::Profile,
    package::{BundleCompilation, Package},
};

mod inference;
mod ordinary;
mod profiles;

const STRUCTURE: &str = include_str!("../../../fixtures/xtext/model-to-text/machine.rcore");
const GRAMMAR: &str = include_str!("../../../fixtures/xtext/model-to-text/machine.xtext");
const TERMINALS: &str = include_str!(
    "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
);

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn diagnostics(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

fn inputs(grammar: &str) -> BundleCompilation {
    let model = DefinitionSource::new("memory:machine.rcore", SourceKind::Rcore, STRUCTURE);
    let mut common = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        TERMINALS,
    );
    common.dependencies.push(model.id.clone());
    let mut text = DefinitionSource::new("memory:machine.xtext", SourceKind::Xtext, grammar);
    text.dependencies = vec![model.id.clone(), common.id.clone()];
    BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:general-text".into(),
        xtext: Some(text.id.clone()),
        rcore_behavior: model.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, text, common],
            namespaces: vec![],
        },
    }
}

fn prepare(package: &Package) -> Result<DocumentLanguage, String> {
    DocumentLanguage::prepare(package, Default::default(), AnalysisLimits::workbench())
        .map_err(diagnostics)
}

/// Ordinary compilation, linking, generation, preserving edits and rejection/recovery.
pub fn verify_ordinary() -> Result<(), String> {
    ordinary::verify()
}

/// Native inference and retained-source replay, independent of generated metadata.
pub fn verify_inference() -> Result<(), String> {
    inference::verify()
}

#[unsafe(no_mangle)]
pub extern "C" fn verify_ordinary_boundary() -> u32 {
    u32::from(verify_ordinary().is_err())
}

#[unsafe(no_mangle)]
pub extern "C" fn verify_inference_boundary() -> u32 {
    u32::from(verify_inference().is_err())
}

#[unsafe(no_mangle)]
pub extern "C" fn verify_profiles_boundary() -> u32 {
    u32::from(profiles::verify().is_err())
}

#[cfg(test)]
mod tests {
    #[test]
    fn selected_profiles_keep_admission_and_wire_identity() -> Result<(), String> {
        super::profiles::verify()
    }
    #[test]
    fn ordinary_public_api_executes_without_application_adapters() -> Result<(), String> {
        super::verify_ordinary()
    }
    #[test]
    fn inference_replays_original_sources_without_application_adapters() -> Result<(), String> {
        super::verify_inference()
    }
}
