//! Handwritten grammar contracts, native Xtext compilation and prediction backend.
//! Source-bound admission is separate from serializable inspection data.
mod analysis_work_budget;
pub use analysis_work_budget::GrammarAnalysisWorkBudget;

mod actions;
mod explicit_calls;
mod first_sets;
mod follow_prefix_diagnostic;
pub use first_sets::prefix_capture::{PrefixCallFact, PrefixSeparation, SourcePrefixInspection};
pub use follow_prefix_diagnostic::{
    PrefixDiagnosticAbsence, PrefixDiagnosticSource, PrefixDiagnosticState, PrefixDiagnosticText,
    SourcePrefixDiagnostic, SourcePrefixDiagnosticRecord,
};
mod sources;
pub(crate) use sources::compile_sources_with_package_imports_prediction_policy;
pub use sources::{
    compile_sources_with_package_imports_prefix_inspected,
    compile_sources_with_package_imports_prefix_inspected_frontier_selected_pool,
};
pub mod admission_observation;
pub mod syntax;
pub use sources::{
    SourceLimits, compile_sources, compile_sources_with_limits,
    compile_sources_with_settings_and_limits, source_reference_context_requirement,
};
mod unordered;
/// Naming observation over freshly source-rebuilt scaffolding, never authority.
pub(crate) fn is_generated_unordered_rule(name: &str) -> bool {
    unordered::generated(name)
}
mod datatype;
mod enum_rules;
pub mod first_rule;
mod follow;
#[cfg(test)]
mod fragment_call_observation_tests;
#[cfg(test)]
mod fragment_call_summaries_tests;
mod nullable_repetition;
pub use nullable_repetition::{
    QualifiedNullableDecision, QualifiedNullableFullRepeat, QualifiedNullableKeywordOptional,
};
mod keywords;
mod lexer_prediction;
mod parser_prediction;
pub(crate) mod token_cursor;
pub mod type_names;
pub use parser_prediction::{
    CompletionVerification, DEFAULT_CALL_DEPTH, MAX_CALL_DEPTH, ParserPrediction, PredictionSymbol,
};
pub use parser_prediction::{
    DeferredNestedDecision, InitialPreparation, LiveCallerPrediction, NestedDecisionKey,
    PortRuntimePredictionSource, PredictionInput, PreparedLiveRuntimePredictionContext,
    PreparedRuntimePredictionContext, RecursiveFrontierOrder, RepeatedRuntimePredictionSource,
    RuntimeAdmissionLoadMode, RuntimeGuardedAdmission, RuntimeGuardedPolicy, RuntimePredictionMemo,
    RuntimePredictionMemoMode, RuntimePredictionMemoStats, RuntimePredictionSource,
    RuntimeWorkGeometry, SavedRuntimeAdmission, ViabilityKey,
};
mod terminals;
use intersect_model::definitions::model_definition::ReferenceContextTransaction;
use intersect_model::definitions::model_definition::StructureContext;
use intersect_model::definitions::schema::Diagnostic;
use intersect_model::definitions::schema::Schema;
pub use keywords::PunctuationKeywords;
pub use lexer_prediction::PreparedLexicalInitial;
pub use lexer_prediction::{Alternative as LexicalAlternative, LexerPrediction};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
/// Checked compiler context; never reconstructed from a reduced runtime schema.
#[derive(Default)]
pub struct PackageImports<'policy> {
    /// Explicit generated-source frontend only; ordinary profiles keep rejecting generate.
    pub(crate) inferred_headers: bool,
    /// Fresh language inference supplied by exact grammar source, never a reduced schema.
    pub(crate) inferred_rules: BTreeMap<String, BTreeMap<String, (String, String)>>,
    /// Exact producer identities checked by the in-process source-program bridge.
    pub(crate) setting_sources: BTreeMap<
        intersect_model::definitions::bundle::SourceId,
        intersect_model::definitions::bundle::SourceId,
    >,
    pub declarations: intersect_model::definitions::declarations::DeclarationInventory,
    pub collect_classifiers: bool,
    /// Fresh native language policy; never serialized as an approval.
    pub validate_authored: Option<&'policy AuthoredSourceValidator<'policy>>,
    /// Independent admitted source-world bound; executable grammar limits stay exact.
    pub authored_source_limits: Option<SourceLimits>,
    /// Explicit grammar-analysis work grant copied onto every parsed grammar.
    /// Defaults to `GrammarAnalysisWorkBudget::default()`.
    pub work_budget: GrammarAnalysisWorkBudget,
}
#[cfg(test)]
#[path = "grammar/backend_test_support.rs"]
pub(crate) mod backend_test_support;
pub use terminals::{Expression as TerminalExpression, Operation as TerminalOperation, Terminal};

/// Explicit language-capability contract. Retained grammars require this field.
/// New compilation requests default to the bounded workbench capability policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    #[default]
    #[serde(rename = "intersect.workbench.v1")]
    WorkbenchV1,
    #[serde(rename = "intersect.xtext.native.v1")]
    NativeV1,
    #[serde(rename = "intersect.xtext.native.v2")]
    NativeV2,
    #[serde(rename = "intersect.xtext.native.v3")]
    NativeV3,
    #[serde(rename = "intersect.xtext.native.v4")]
    NativeV4,
    #[serde(rename = "intersect.xtext.native.v5")]
    NativeV5,
    #[serde(rename = "intersect.xtext.native.v6")]
    NativeV6,
    #[serde(rename = "intersect.xtext.native.v7")]
    NativeV7,
    #[serde(rename = "intersect.xtext.native.v8")]
    NativeV8,
    #[serde(rename = "intersect.xtext.native.v9")]
    NativeV9,
    #[serde(rename = "intersect.xtext.native.v10")]
    NativeV10,
    #[serde(rename = "intersect.xtext.native.v11")]
    NativeV11,
    #[serde(rename = "intersect.xtext.native.v12")]
    NativeV12,
    #[serde(rename = "intersect.xtext.native.v13")]
    NativeV13,
    #[serde(rename = "intersect.xtext.native.v14")]
    NativeV14,
    #[serde(rename = "intersect.xtext.native.v15")]
    NativeV15,
    #[serde(rename = "intersect.xtext.native.v16")]
    NativeV16,
    #[serde(rename = "intersect.xtext.native.v17")]
    NativeV17,
    #[serde(rename = "intersect.xtext.native.v18")]
    NativeV18,
    #[serde(rename = "intersect.xtext.native.v19")]
    NativeV19,
    #[serde(rename = "intersect.xtext.native.v20")]
    NativeV20,
    #[serde(rename = "intersect.xtext.native.v21")]
    NativeV21,
    #[serde(rename = "intersect.xtext.native.v22")]
    NativeV22,
    #[serde(rename = "intersect.xtext.native.v23")]
    NativeV23,
    #[serde(rename = "intersect.xtext.native.v24")]
    NativeV24,
    #[serde(rename = "intersect.xtext.native.v25")]
    NativeV25,
    #[serde(rename = "intersect.xtext.native.v26")]
    NativeV26,
    #[serde(rename = "intersect.xtext.native.v27")]
    NativeV27,
    #[serde(rename = "intersect.xtext.native.v28")]
    NativeV28,
    #[serde(rename = "intersect.xtext.native.v29")]
    NativeV29,
    #[serde(rename = "intersect.xtext.native.v30")]
    NativeV30,
    #[serde(rename = "intersect.xtext.native.v31")]
    NativeV31,
    #[serde(rename = "intersect.xtext.native.v32")]
    NativeV32,
    #[serde(rename = "intersect.xtext.native.v33")]
    NativeV33,
    #[serde(rename = "intersect.xtext.native.v34")]
    NativeV34,
    #[serde(rename = "intersect.xtext.native.v35")]
    NativeV35,
    #[serde(rename = "intersect.xtext.native.v36")]
    NativeV36,
    #[serde(rename = "intersect.xtext.native.v37")]
    NativeV37,
    #[serde(rename = "intersect.xtext.native.v38")]
    NativeV38,
    #[serde(rename = "intersect.xtext.native.v39")]
    NativeV39,
    #[serde(rename = "intersect.xtext.native.v40")]
    NativeV40,
    #[serde(rename = "intersect.xtext.native.v41")]
    NativeV41,
    #[serde(rename = "intersect.xtext.native.v42")]
    NativeV42,
    #[serde(rename = "intersect.xtext.native.v43")]
    NativeV43,
    #[serde(rename = "intersect.xtext.native.v44")]
    NativeV44,
    #[serde(rename = "intersect.xtext.native.v45")]
    NativeV45,
    #[serde(rename = "intersect.xtext.native.v46")]
    NativeV46,
    #[serde(rename = "intersect.xtext.native.v47")]
    NativeV47,
    #[serde(rename = "intersect.xtext.native.v48")]
    NativeV48,
    #[serde(rename = "intersect.xtext.native.v49")]
    NativeV49,
    #[serde(rename = "intersect.xtext.native.v50")]
    NativeV50,
    #[serde(rename = "intersect.xtext.native.v51")]
    NativeV51,
    #[serde(rename = "intersect.xtext.native.v52")]
    NativeV52,
    #[serde(rename = "intersect.xtext.native.v53")]
    NativeV53,
    /// Retained pair certificates for n-ary significant-token prediction.
    #[serde(rename = "intersect.xtext.native.v54")]
    NativeV54,
    /// Conservative FIRST filters over exact grouped caller continuations.
    #[serde(rename = "intersect.xtext.native.v55")]
    NativeV55,
    /// Bounded initial binary FIRST discovery preserves exact fallback work.
    #[serde(rename = "intersect.xtext.native.v56")]
    NativeV56,
    /// Constant-guard programs emitted by native parameter specialization.
    #[serde(rename = "intersect.xtext.native.v57")]
    NativeV57,
    /// Resolved one-UTF-16-unit terminal-negation operands.
    #[serde(rename = "intersect.xtext.native.v58")]
    NativeV58,
    /// Pinned full-generated semantics for direct multi-unit literal negation.
    #[serde(rename = "intersect.xtext.native.v59")]
    NativeV59,
    /// Rule-local finite Until operands and nongreedy lexer decisions.
    #[serde(rename = "intersect.xtext.native.v60")]
    NativeV60,
    /// Contextual authored first-rule validation over resolved used grammars.
    #[serde(rename = "intersect.xtext.native.v61")]
    NativeV61,
    /// Pinned lexer-set projection for grouped and literal-fragment negation.
    #[serde(rename = "intersect.xtext.native.v62")]
    NativeV62,
    #[serde(rename = "intersect.xtext.native.v63")]
    NativeV63,
    #[serde(rename = "intersect.xtext.native.v64")]
    NativeV64,
    #[serde(rename = "intersect.xtext.native.v65")]
    NativeV65,
    /// Source-authoritative imported assignment compatibility.
    #[serde(rename = "intersect.xtext.native.v66")]
    NativeV66,
    /// Analyze recursive commitment only at executable choice/repeat decisions.
    #[serde(rename = "intersect.xtext.native.v67")]
    NativeV67,
    /// Full authored syntactic predicates over the shared bounded predictor.
    #[serde(rename = "intersect.xtext.native.v68")]
    NativeV68,
    /// Exact computed Set contracts for writable derived Boolean assignments.
    #[serde(rename = "intersect.xtext.native.v69")]
    NativeV69,
    /// Lazy class-independent fragment call summaries under unchanged limits.
    #[serde(rename = "intersect.xtext.native.v70")]
    NativeV70,
    /// Required nullable object calls remain pending until graph certification.
    #[serde(rename = "intersect.xtext.native.v71")]
    NativeV71,
    /// Reuse symbolic FOLLOW frontiers under the unchanged shared work ceiling.
    #[serde(rename = "intersect.xtext.native.v72")]
    NativeV72,
    /// Reserve unique FIFO closure rows once during recursive classification.
    #[serde(rename = "intersect.xtext.native.v73")]
    NativeV73,
    #[serde(rename = "intersect.xtext.native.v74")]
    NativeV74,
    #[serde(rename = "intersect.xtext.native.v75")]
    NativeV75,
    #[serde(rename = "intersect.xtext.native.v76")]
    NativeV76,
    /// Qualified productive nullable whole-Full iteration, explicit only.
    #[serde(rename = "intersect.xtext.native.v77")]
    NativeV77,
    /// Geometric overflow capacity in the same adjacent-context FIFO.
    /// Includes productive nullable whole-Full iteration from V77.
    #[serde(rename = "intersect.xtext.native.v78")]
    NativeV78,
    /// Resolve one immutable source occurrence for both nullable qualifiers.
    #[serde(rename = "intersect.xtext.native.v79")]
    NativeV79,
    /// Reuse completed same-callee witness facts across compatible depth bounds.
    #[serde(rename = "intersect.xtext.native.v80")]
    NativeV80,
    /// Uniform shared builtin scalar returns for ordinary source languages.
    #[serde(rename = "intersect.xtext.native.v81")]
    NativeV81,
    /// Bounded unordered groups lowered into shared parser/serializer operations.
    #[serde(rename = "intersect.xtext.native.v82")]
    NativeV82,
}
impl Profile {
    pub fn has_builtin_scalar_returns(self) -> bool {
        matches!(self, Self::NativeV81 | Self::NativeV82)
    }
    pub fn has_unordered_groups(self) -> bool {
        matches!(self, Self::NativeV82)
    }
    pub fn has_symbolic_follow_validation(self) -> bool {
        matches!(
            self,
            Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_required_object_call_certificates(self) -> bool {
        matches!(
            self,
            Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_fragment_call_summaries(self) -> bool {
        matches!(
            self,
            Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_computed_boolean_settings(self) -> bool {
        matches!(
            self,
            Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_full_syntactic_predicates(self) -> bool {
        matches!(
            self,
            Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_executable_recursive_analysis(self) -> bool {
        matches!(
            self,
            Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_imported_assignment_compatibility(self) -> bool {
        matches!(
            self,
            Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_durable_documents(self) -> bool {
        matches!(
            self,
            Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_scoped_hidden_tokens(self) -> bool {
        matches!(
            self,
            Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_terminal_cardinality_decisions(self) -> bool {
        matches!(
            self,
            Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_contextual_first_rule(self) -> bool {
        matches!(
            self,
            Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_projected_terminal_negation(self) -> bool {
        matches!(
            self,
            Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_until_operands(self) -> bool {
        matches!(
            self,
            Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_multi_unit_literal_negation(self) -> bool {
        matches!(
            self,
            Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_terminal_operand_negation(self) -> bool {
        matches!(
            self,
            Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_constant_guards(self) -> bool {
        matches!(
            self,
            Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn is_workbench(&self) -> bool {
        *self == Self::WorkbenchV1
    }
    pub fn has_lexical_prediction(self) -> bool {
        matches!(
            self,
            Self::NativeV2
                | Self::NativeV3
                | Self::NativeV4
                | Self::NativeV5
                | Self::NativeV6
                | Self::NativeV7
                | Self::NativeV8
                | Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_parser_overrides(self) -> bool {
        matches!(
            self,
            Self::NativeV3
                | Self::NativeV4
                | Self::NativeV5
                | Self::NativeV6
                | Self::NativeV7
                | Self::NativeV8
                | Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_return_inference(self) -> bool {
        matches!(
            self,
            Self::NativeV4
                | Self::NativeV5
                | Self::NativeV6
                | Self::NativeV7
                | Self::NativeV8
                | Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_current_actions(self) -> bool {
        matches!(
            self,
            Self::NativeV5
                | Self::NativeV6
                | Self::NativeV7
                | Self::NativeV8
                | Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_parser_prediction(self) -> bool {
        matches!(
            self,
            Self::NativeV6
                | Self::NativeV7
                | Self::NativeV8
                | Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_enum_rules(self) -> bool {
        matches!(
            self,
            Self::NativeV7
                | Self::NativeV8
                | Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_rule_conversions(self) -> bool {
        matches!(
            self,
            Self::NativeV8
                | Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_recursive_predicates(self) -> bool {
        matches!(
            self,
            Self::NativeV9
                | Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_double_rules(self) -> bool {
        matches!(
            self,
            Self::NativeV10
                | Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_recursive_fragments(self) -> bool {
        matches!(
            self,
            Self::NativeV11
                | Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_computed_settings(self) -> bool {
        matches!(
            self,
            Self::NativeV12
                | Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_terminal_decisions(self) -> bool {
        matches!(
            self,
            Self::NativeV13
                | Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_validated_syntax(self) -> bool {
        matches!(
            self,
            Self::NativeV14
                | Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_terminal_overrides(self) -> bool {
        matches!(
            self,
            Self::NativeV15
                | Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_explicit_terminal_calls(self) -> bool {
        matches!(
            self,
            Self::NativeV16
                | Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_explicit_parser_calls(self) -> bool {
        matches!(
            self,
            Self::NativeV17
                | Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn has_typed_overrides(self) -> bool {
        matches!(
            self,
            Self::NativeV18
                | Self::NativeV19
                | Self::NativeV20
                | Self::NativeV21
                | Self::NativeV22
                | Self::NativeV23
                | Self::NativeV24
                | Self::NativeV25
                | Self::NativeV26
                | Self::NativeV27
                | Self::NativeV28
                | Self::NativeV29
                | Self::NativeV30
                | Self::NativeV31
                | Self::NativeV32
                | Self::NativeV33
                | Self::NativeV34
                | Self::NativeV35
                | Self::NativeV36
                | Self::NativeV37
                | Self::NativeV38
                | Self::NativeV39
                | Self::NativeV40
                | Self::NativeV41
                | Self::NativeV42
                | Self::NativeV43
                | Self::NativeV44
                | Self::NativeV45
                | Self::NativeV46
                | Self::NativeV47
                | Self::NativeV48
                | Self::NativeV49
                | Self::NativeV50
                | Self::NativeV51
                | Self::NativeV52
                | Self::NativeV53
                | Self::NativeV54
                | Self::NativeV55
                | Self::NativeV56
                | Self::NativeV57
                | Self::NativeV58
                | Self::NativeV59
                | Self::NativeV60
                | Self::NativeV61
                | Self::NativeV62
                | Self::NativeV63
                | Self::NativeV64
                | Self::NativeV65
                | Self::NativeV66
                | Self::NativeV67
                | Self::NativeV68
                | Self::NativeV69
                | Self::NativeV70
                | Self::NativeV71
                | Self::NativeV72
                | Self::NativeV73
                | Self::NativeV74
                | Self::NativeV75
                | Self::NativeV76
                | Self::NativeV77
                | Self::NativeV78
                | Self::NativeV79
                | Self::NativeV80
                | Self::NativeV81
                | Self::NativeV82
        )
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkbenchV1 => "intersect.workbench.v1",
            Self::NativeV1 => "intersect.xtext.native.v1",
            Self::NativeV2 => "intersect.xtext.native.v2",
            Self::NativeV3 => "intersect.xtext.native.v3",
            Self::NativeV4 => "intersect.xtext.native.v4",
            Self::NativeV5 => "intersect.xtext.native.v5",
            Self::NativeV6 => "intersect.xtext.native.v6",
            Self::NativeV7 => "intersect.xtext.native.v7",
            Self::NativeV8 => "intersect.xtext.native.v8",
            Self::NativeV9 => "intersect.xtext.native.v9",
            Self::NativeV10 => "intersect.xtext.native.v10",
            Self::NativeV11 => "intersect.xtext.native.v11",
            Self::NativeV12 => "intersect.xtext.native.v12",
            Self::NativeV13 => "intersect.xtext.native.v13",
            Self::NativeV14 => "intersect.xtext.native.v14",
            Self::NativeV15 => "intersect.xtext.native.v15",
            Self::NativeV16 => "intersect.xtext.native.v16",
            Self::NativeV17 => "intersect.xtext.native.v17",
            Self::NativeV18 => "intersect.xtext.native.v18",
            Self::NativeV19 => "intersect.xtext.native.v19",
            Self::NativeV20 => "intersect.xtext.native.v20",
            Self::NativeV21 => "intersect.xtext.native.v21",
            Self::NativeV22 => "intersect.xtext.native.v22",
            Self::NativeV23 => "intersect.xtext.native.v23",
            Self::NativeV24 => "intersect.xtext.native.v24",
            Self::NativeV25 => "intersect.xtext.native.v25",
            Self::NativeV26 => "intersect.xtext.native.v26",
            Self::NativeV27 => "intersect.xtext.native.v27",
            Self::NativeV28 => "intersect.xtext.native.v28",
            Self::NativeV29 => "intersect.xtext.native.v29",
            Self::NativeV30 => "intersect.xtext.native.v30",
            Self::NativeV31 => "intersect.xtext.native.v31",
            Self::NativeV32 => "intersect.xtext.native.v32",
            Self::NativeV33 => "intersect.xtext.native.v33",
            Self::NativeV34 => "intersect.xtext.native.v34",
            Self::NativeV35 => "intersect.xtext.native.v35",
            Self::NativeV36 => "intersect.xtext.native.v36",
            Self::NativeV37 => "intersect.xtext.native.v37",
            Self::NativeV38 => "intersect.xtext.native.v38",
            Self::NativeV39 => "intersect.xtext.native.v39",
            Self::NativeV40 => "intersect.xtext.native.v40",
            Self::NativeV41 => "intersect.xtext.native.v41",
            Self::NativeV42 => "intersect.xtext.native.v42",
            Self::NativeV43 => "intersect.xtext.native.v43",
            Self::NativeV44 => "intersect.xtext.native.v44",
            Self::NativeV45 => "intersect.xtext.native.v45",
            Self::NativeV46 => "intersect.xtext.native.v46",
            Self::NativeV47 => "intersect.xtext.native.v47",
            Self::NativeV48 => "intersect.xtext.native.v48",
            Self::NativeV49 => "intersect.xtext.native.v49",
            Self::NativeV50 => "intersect.xtext.native.v50",
            Self::NativeV51 => "intersect.xtext.native.v51",
            Self::NativeV52 => "intersect.xtext.native.v52",
            Self::NativeV53 => "intersect.xtext.native.v53",
            Self::NativeV54 => "intersect.xtext.native.v54",
            Self::NativeV55 => "intersect.xtext.native.v55",
            Self::NativeV56 => "intersect.xtext.native.v56",
            Self::NativeV57 => "intersect.xtext.native.v57",
            Self::NativeV58 => "intersect.xtext.native.v58",
            Self::NativeV59 => "intersect.xtext.native.v59",
            Self::NativeV60 => "intersect.xtext.native.v60",
            Self::NativeV61 => "intersect.xtext.native.v61",
            Self::NativeV62 => "intersect.xtext.native.v62",
            Self::NativeV63 => "intersect.xtext.native.v63",
            Self::NativeV64 => "intersect.xtext.native.v64",
            Self::NativeV65 => "intersect.xtext.native.v65",
            Self::NativeV66 => "intersect.xtext.native.v66",
            Self::NativeV67 => "intersect.xtext.native.v67",
            Self::NativeV68 => "intersect.xtext.native.v68",
            Self::NativeV69 => "intersect.xtext.native.v69",
            Self::NativeV70 => "intersect.xtext.native.v70",
            Self::NativeV71 => "intersect.xtext.native.v71",
            Self::NativeV72 => "intersect.xtext.native.v72",
            Self::NativeV73 => "intersect.xtext.native.v73",
            Self::NativeV74 => "intersect.xtext.native.v74",
            Self::NativeV75 => "intersect.xtext.native.v75",
            Self::NativeV76 => "intersect.xtext.native.v76",
            Self::NativeV77 => "intersect.xtext.native.v77",
            Self::NativeV78 => "intersect.xtext.native.v78",
            Self::NativeV79 => "intersect.xtext.native.v79",
            Self::NativeV80 => "intersect.xtext.native.v80",
            Self::NativeV81 => "intersect.xtext.native.v81",
            Self::NativeV82 => "intersect.xtext.native.v82",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grammar {
    /// Call depth for the prediction program compiled from this grammar,
    /// when a guarded runtime admission asks for one; the compiled program
    /// carries it. Not part of the grammar's own data.
    #[serde(skip)]
    pub(crate) call_depth: Option<usize>,
    /// Grammar-analysis work grant for the compilation that built this grammar
    /// when no guarded runtime admission supplies one. Handwritten transient
    /// compiler context like `call_depth`; not part of the grammar's own data.
    #[serde(skip)]
    pub(crate) analysis_work_budget: GrammarAnalysisWorkBudget,
    /// Original parameterized declarations; named no-argument entries stay explicit.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub parameterized_rules: BTreeSet<String>,
    /// Effective final declarations; exact identities remain in declaration_sources.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub final_rules: BTreeSet<String>,
    /// Required executable contracts; serialization does not install providers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub computed_settings: Vec<intersect_model::definitions::setting::ComputedSettingContract>,
    pub profile: Profile,
    pub name: String,
    pub entry: String,
    pub rules: BTreeMap<String, Rule>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub terminals: BTreeMap<String, Terminal>,
    /// Explicit grammar-defined lexical policy; None preserves the inherited contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lexical_prediction: Option<LexerPrediction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parser_prediction: Option<ParserPrediction>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub declaration_sources: BTreeMap<String, intersect_model::definitions::bundle::SourceSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terminal_order: Vec<String>,
    /// Replaced declarations retain exact identities and count toward admission limits.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shadowed_declarations:
        BTreeMap<String, Vec<intersect_model::definitions::bundle::SourceSpan>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Datatype {
    EString,
    EInt,
    EBoolean,
    EDouble,
    Enum,
    /// Explicit converted datatype; exact identity is retained independently.
    Nominal,
    /// NativeV81: one of the already-shared Ecore scalar codecs.
    Builtin(intersect_model::definitions::scalars::Builtin),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    /// Explicit contextual hidden terminals; None inherits the actual caller policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<Vec<String>>,
    /// Keyword spellings bound to exact imported enum literal identities.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub enum_literals: BTreeMap<String, intersect_model::definitions::enums::EnumValue>,
    /// Return type omitted in a profile that performs upstream-style inference.
    #[serde(default, skip_serializing_if = "is_false")]
    pub inferred_return: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub override_declared: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datatype: Option<Datatype>,
    /// Retained checked nominal return; observation alone is never provider authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_return: Option<intersect_model::definitions::resources::DeclarationId>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub fragment: bool,
    /// A caller-owned fragment with no declared return classifier.
    #[serde(default, skip_serializing_if = "is_false")]
    pub wildcard: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub class: String,
    pub body: Expr,
}
fn is_false(value: &bool) -> bool {
    !value
}
impl Rule {
    pub fn has_current_actions(&self) -> bool {
        actions::has_capture(&self.body)
    }
    /// Rules whose alternatives construct or forward the current object lazily.
    pub(crate) fn uses_current_flow(&self, grammar: &Grammar) -> bool {
        self.has_current_actions()
            || (matches!(
                grammar.profile,
                Profile::NativeV35
                    | Profile::NativeV36
                    | Profile::NativeV37
                    | Profile::NativeV38
                    | Profile::NativeV39
                    | Profile::NativeV40
                    | Profile::NativeV41
                    | Profile::NativeV42
                    | Profile::NativeV43
                    | Profile::NativeV44
                    | Profile::NativeV45
                    | Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) && !self.fragment
                && self.datatype.is_none()
                && !self.is_dispatch(grammar)
                && actions::has_unassigned_object_call(&self.body, grammar))
            || (matches!(
                grammar.profile,
                Profile::NativeV26
                    | Profile::NativeV27
                    | Profile::NativeV28
                    | Profile::NativeV29
                    | Profile::NativeV30
                    | Profile::NativeV31
                    | Profile::NativeV32
                    | Profile::NativeV33
                    | Profile::NativeV34
                    | Profile::NativeV35
                    | Profile::NativeV36
                    | Profile::NativeV37
                    | Profile::NativeV38
                    | Profile::NativeV39
                    | Profile::NativeV40
                    | Profile::NativeV41
                    | Profile::NativeV42
                    | Profile::NativeV43
                    | Profile::NativeV44
                    | Profile::NativeV45
                    | Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) && actions::simple_count(&self.body) > 0
                && actions::leading(&self.body).is_none())
    }
    /// Construction class selected by an admitted leading simple action.
    /// Both assignment admission and execution use this same selection.
    pub fn construction_class(&self) -> &str {
        actions::leading(&self.body).unwrap_or(&self.class)
    }

    /// Pure unassigned parser dispatch, derived from the retained grammar body.
    /// Compilation separately checks target types, nullability and prediction.
    pub(crate) fn is_dispatch(&self, grammar: &Grammar) -> bool {
        !self.fragment && self.datatype.is_none() && dispatch_shape(&self.body, grammar)
    }
}
fn dispatch_shape(expr: &Expr, grammar: &Grammar) -> bool {
    if matches!(
        grammar.profile,
        Profile::NativeV25
            | Profile::NativeV26
            | Profile::NativeV27
            | Profile::NativeV28
            | Profile::NativeV29
            | Profile::NativeV30
            | Profile::NativeV31
            | Profile::NativeV32
            | Profile::NativeV33
            | Profile::NativeV34
            | Profile::NativeV35
            | Profile::NativeV36
            | Profile::NativeV37
            | Profile::NativeV38
            | Profile::NativeV39
            | Profile::NativeV40
            | Profile::NativeV41
            | Profile::NativeV42
            | Profile::NativeV43
            | Profile::NativeV44
            | Profile::NativeV45
            | Profile::NativeV46
            | Profile::NativeV47
            | Profile::NativeV48
            | Profile::NativeV49
            | Profile::NativeV50
            | Profile::NativeV51
            | Profile::NativeV52
            | Profile::NativeV53
            | Profile::NativeV54
            | Profile::NativeV55
            | Profile::NativeV56
            | Profile::NativeV57
            | Profile::NativeV58
            | Profile::NativeV59
            | Profile::NativeV60
            | Profile::NativeV61
            | Profile::NativeV62
            | Profile::NativeV63
            | Profile::NativeV64
            | Profile::NativeV65
            | Profile::NativeV66
            | Profile::NativeV67
            | Profile::NativeV68
            | Profile::NativeV69
            | Profile::NativeV70
            | Profile::NativeV71
            | Profile::NativeV72
            | Profile::NativeV73
            | Profile::NativeV74
            | Profile::NativeV75
            | Profile::NativeV76
            | Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) {
        return forwarding_count(expr, grammar) == Some(1);
    }
    if expr.cardinality != '1' {
        return false;
    }
    match &expr.kind {
        Kind::SyntacticPredicate(value) => dispatch_shape(value, grammar),
        Kind::Call(name) => grammar
            .rules
            .get(name)
            .is_some_and(|rule| !rule.fragment && rule.datatype.is_none()),
        Kind::Sequence(items) => items.len() == 1 && dispatch_shape(&items[0], grammar),
        Kind::Alternative(items) => {
            !items.is_empty() && items.iter().all(|item| dispatch_shape(item, grammar))
        }
        _ => false,
    }
}
// Exactly one object-producing call on every path; surrounding keywords do
// not create an owner. Optional/repeated forwarding and writes remain excluded.
fn forwarding_count(expr: &Expr, grammar: &Grammar) -> Option<usize> {
    if expr.cardinality != '1' {
        return None;
    }
    match &expr.kind {
        Kind::SyntacticPredicate(value) => forwarding_count(value, grammar),
        Kind::Literal(_) => Some(0),
        Kind::Call(name) => grammar
            .rules
            .get(name)
            .filter(|rule| !rule.fragment && rule.datatype.is_none())
            .map(|_| 1),
        Kind::Sequence(items) => {
            let mut count = 0;
            for item in items {
                count += forwarding_count(item, grammar)?;
                if count > 1 {
                    return None;
                }
            }
            Some(count)
        }
        Kind::Alternative(items) => {
            let count = forwarding_count(items.first()?, grammar)?;
            items
                .iter()
                .all(|item| forwarding_count(item, grammar) == Some(count))
                .then_some(count)
        }
        _ => None,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expr {
    /// Exact original parser input bytes for this lowered expression. Source
    /// identity comes from its retained declaration, including generated sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_range: Option<std::ops::Range<usize>>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub first_set_predicated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prediction_path: Option<String>,
    pub kind: Kind,
    pub cardinality: char,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    /// Explicit simple action, bound to the exact imported schema class key.
    Action(String),
    /// Construct a new current object and capture the previous current value.
    Capture {
        class: String,
        feature: String,
        many: bool,
    },
    Literal(String),
    Call(String),
    Reference(String),
    ScalarReference {
        target: String,
        rule: String,
    },
    Assign {
        feature: String,
        many: bool,
        value: Box<Expr>,
    },
    /// Presence of a matched keyword sets a single Boolean attribute to true.
    BooleanAssign {
        feature: String,
        value: Box<Expr>,
    },
    Sequence(Vec<Expr>),
    Alternative(Vec<Expr>),
    /// Authored `=>` operand boundary. Cardinality belongs to the outer Expr.
    /// Prediction probes this one shared operand graph; normal parsing consumes
    /// the same operand exactly once after the enclosing decision commits.
    SyntacticPredicate(Box<Expr>),
}
#[derive(Clone, Debug)]
pub struct Token<T = String> {
    pub text: T,
    pub start: usize,
    pub end: usize,
    pub quoted: bool,
    /// Selected alternative in the grammar-owned prediction program, when present.
    pub lexical_alternative: Option<usize>,
}

pub fn lex(source: &str) -> Result<Vec<Token>, (usize, String)> {
    lex_with_limit(source, Some(50_000))
}

/// Shared lexer with a caller-selected token ceiling; `None` disables the ceiling.
/// Definition compilation continues to use the bounded `lex` entry point.
pub fn lex_with_limit(
    source: &str,
    token_limit: Option<usize>,
) -> Result<Vec<Token>, (usize, String)> {
    lex_borrowed(source, token_limit).map(|tokens| {
        tokens
            .into_iter()
            .map(|t| Token {
                text: t.text.into_owned(),
                start: t.start,
                end: t.end,
                quoted: t.quoted,
                lexical_alternative: t.lexical_alternative,
            })
            .collect()
    })
}

/// Source-backed tokens; only escaped strings allocate decoded text.
/// The owning lexer is a compatibility adapter over this implementation.
pub fn lex_borrowed(
    source: &str,
    token_limit: Option<usize>,
) -> Result<Vec<Token<Cow<'_, str>>>, (usize, String)> {
    lex_borrowed_with_keywords(source, token_limit, None)
}

/// Shared lexer with optional native punctuation matching. Definition and legacy
/// consumers pass no keyword matcher and retain their existing lexical contract.
pub fn lex_borrowed_with_keywords<'a>(
    source: &'a str,
    token_limit: Option<usize>,
    keywords: Option<&PunctuationKeywords>,
) -> Result<Vec<Token<Cow<'a, str>>>, (usize, String)> {
    lex_borrowed_with_terminals(source, token_limit, keywords, None)
}

/// Model lexer with admitted native terminal programs. Existing consumers pass
/// no programs and retain the exact inherited lexer contract.
pub fn lex_borrowed_with_terminals<'a>(
    source: &'a str,
    token_limit: Option<usize>,
    keywords: Option<&PunctuationKeywords>,
    terminals: Option<&BTreeMap<String, Terminal>>,
) -> Result<Vec<Token<Cow<'a, str>>>, (usize, String)> {
    lex_borrowed_with_policy(source, token_limit, keywords, terminals, None)
}

/// One scanner for inherited and explicit grammar-defined lexical policies.
fn lex_borrowed_with_policy<'a>(
    source: &'a str,
    token_limit: Option<usize>,
    keywords: Option<&PunctuationKeywords>,
    terminals: Option<&BTreeMap<String, Terminal>>,
    hidden: Option<&[String]>,
) -> Result<Vec<Token<Cow<'a, str>>>, (usize, String)> {
    lex_borrowed_with_prediction(
        source,
        token_limit,
        keywords,
        terminals,
        hidden,
        None,
        &mut None,
    )
}

/// Shared scanner with grammar-generated prediction and the caller's remaining
/// execution budget. The selected alternative stays attached to each token.
pub fn lex_borrowed_with_prediction<'a>(
    source: &'a str,
    token_limit: Option<usize>,
    keywords: Option<&PunctuationKeywords>,
    terminals: Option<&BTreeMap<String, Terminal>>,
    hidden: Option<&[String]>,
    prediction: Option<&LexerPrediction>,
    work: &mut Option<usize>,
) -> Result<Vec<Token<Cow<'a, str>>>, (usize, String)> {
    scan(
        source,
        token_limit,
        LexicalPolicy {
            keywords,
            terminals,
            hidden,
            prediction,
            syntax: false,
        },
        work,
    )
    .map(|scanned| scanned.tokens)
}

/// Scan-local outcomes of unbudgeted lexeme selections, keyed by the exact
/// ASCII source prefix that alone determined them. Prediction and terminal
/// recognition read their input left to right and are functions of what they
/// read, so the same prefix elsewhere selects the same alternative and end.
#[derive(Default)]
struct LexemeMemo {
    by_first: BTreeMap<u8, Vec<MemoizedLexeme>>,
}
struct MemoizedLexeme {
    prefix: Box<[u8]>,
    alternative: usize,
    length: usize,
    hits: usize,
}
impl LexemeMemo {
    const MAX_PREFIX: usize = 24;
    const MAX_PER_FIRST: usize = 16;
    fn get(&mut self, rest: &[u8]) -> Option<(usize, usize)> {
        let known = self.by_first.get_mut(rest.first()?)?;
        let found = known
            .iter_mut()
            .find(|lexeme| rest.starts_with(&lexeme.prefix))?;
        found.hits = found.hits.saturating_add(1);
        Some((found.alternative, found.length))
    }
    fn insert(&mut self, prefix: &[u8], alternative: usize, length: usize) {
        let Some(first) = prefix.first() else {
            return;
        };
        if prefix.len() > Self::MAX_PREFIX || length == 0 || length > prefix.len() {
            return;
        }
        let known = self.by_first.entry(*first).or_default();
        let lexeme = MemoizedLexeme {
            prefix: prefix.into(),
            alternative,
            length,
            hits: 0,
        };
        if known.len() < Self::MAX_PER_FIRST {
            known.push(lexeme);
        } else if let Some(unused) = known.iter_mut().rev().find(|known| known.hits == 0) {
            // Keep outcomes that recur; unique spellings give way.
            *unused = lexeme;
        }
    }
}

fn predicted_lexeme<'program>(
    source: &str,
    start: usize,
    terminals: Option<&'program BTreeMap<String, Terminal>>,
    initial: &mut lexer_prediction::ReusableLexicalInitial<'_>,
    terminal_initials: &terminals::ReusableTerminalPredictions<'program>,
    memo: &mut LexemeMemo,
    work: &mut Option<usize>,
) -> Result<Option<(usize, usize)>, (usize, String)> {
    let prediction = initial.program();
    // A step budget must receive every canonical debit.
    let memoizable = work.is_none();
    if memoizable && let Some((index, length)) = memo.get(&source.as_bytes()[start..]) {
        return Ok(Some((index, start + length)));
    }
    let (selected, predicted) = initial
        .predict_document_observed(&source[start..], work)
        .map_err(|e| (start, e))?;
    let lexer_prediction::LexicalDecision::Selected(index) = selected else {
        return Ok(None);
    };
    let (end, matched) = match prediction.alternative(index).map_err(|e| (start, e))? {
        LexicalAlternative::Terminal(name) => {
            let (length, observed) = terminals
                .and_then(|v| v.get(name))
                .ok_or_else(|| (start, "Missing predicted terminal".into()))?
                .match_len_reusing_observed(&source[start..], terminal_initials, work)
                .map_err(|e| (start, e))?;
            (start + length, observed)
        }
        LexicalAlternative::Keyword(text) => {
            if !source[start..].starts_with(text) {
                return Err((start, format!("Expected keyword {text}")));
            }
            (start + text.len(), Some(text.len()))
        }
    };
    if memoizable && let (Some(predicted), Some(matched)) = (predicted, matched) {
        let observed = predicted.max(matched).max(end - start);
        if let Some(prefix) = source.as_bytes().get(start..start + observed)
            && prefix.is_ascii()
        {
            memo.insert(prefix, index, end - start);
        }
    }
    Ok(Some((index, end)))
}
#[derive(Debug, Serialize)]
pub struct LexicalIssue {
    pub token: usize,
    pub utf16_start: usize,
    pub utf16_end: usize,
    pub utf16_lookahead: usize,
    pub unit: Option<u16>,
    pub expected: Option<u16>,
    pub selected_alternative: Option<usize>,
    #[serde(skip_serializing_if = "is_false")]
    pub early_exit: bool,
}
#[derive(Debug, Serialize)]
pub struct DocumentTokenRange {
    pub utf16_start: usize,
    pub utf16_end: usize,
    pub utf8: Option<std::ops::Range<usize>>,
}
/// Same lexical predictor and committed terminal recognizer over the actual
/// source units. Interior scalar boundaries remain source-unit observations.
#[allow(clippy::type_complexity)]
pub(crate) fn lex_document<'a>(
    source: &'a str,
    token_limit: Option<usize>,
    terminals: &BTreeMap<String, Terminal>,
    prediction: &LexerPrediction,
    work: &mut Option<usize>,
) -> Result<
    (
        Vec<Token<Cow<'a, str>>>,
        Vec<LexicalIssue>,
        Vec<DocumentTokenRange>,
    ),
    (usize, String),
> {
    lex_document_mode::<false>(source, token_limit, terminals, prediction, None, work)
}

/// Same lexical/document worker, using explicitly paid immutable first facts.
#[allow(clippy::type_complexity)]
pub(crate) fn lex_document_with_prepared_initial<'a>(
    source: &'a str,
    token_limit: Option<usize>,
    terminals: &BTreeMap<String, Terminal>,
    prediction: &LexerPrediction,
    prepared: &PreparedLexicalInitial<'_>,
    work: &mut Option<usize>,
) -> Result<
    (
        Vec<Token<Cow<'a, str>>>,
        Vec<LexicalIssue>,
        Vec<DocumentTokenRange>,
    ),
    (usize, String),
> {
    prepared
        .check(prediction, work)
        .map_err(|error| (0, error))?;
    prepared
        .check_terminals(terminals, work)
        .map_err(|error| (0, error))?;
    lex_document_mode::<true>(
        source,
        token_limit,
        terminals,
        prediction,
        Some(prepared),
        work,
    )
}

#[allow(clippy::type_complexity)]
fn lex_document_mode<'a, const PREPARED: bool>(
    source: &'a str,
    token_limit: Option<usize>,
    terminals: &BTreeMap<String, Terminal>,
    prediction: &LexerPrediction,
    prepared: Option<&PreparedLexicalInitial<'_>>,
    work: &mut Option<usize>,
) -> Result<
    (
        Vec<Token<Cow<'a, str>>>,
        Vec<LexicalIssue>,
        Vec<DocumentTokenRange>,
    ),
    (usize, String),
> {
    let mut tokens = Vec::new();
    let mut issues = Vec::new();
    let mut ranges = Vec::new();
    let mut input = source.encode_utf16();
    let mut unit_at = 0usize;
    let mut byte_at = 0usize;
    let mut interior = false;
    while input.clone().next().is_some() {
        if token_limit.is_some_and(|limit| tokens.len() >= limit) {
            return Err((byte_at, "Token budget exceeded".into()));
        }
        let decision = if PREPARED {
            prediction.predict_document_units_with_prepared_initial(
                input.clone(),
                prepared.ok_or_else(|| {
                    (byte_at, "Missing prepared lexical document frontier".into())
                })?,
                work,
            )
        } else {
            prediction.predict_document_units(input.clone(), work)
        }
        .map_err(|e| (byte_at, e))?;
        let (index, consumed, failure) = match decision {
            lexer_prediction::LexicalDecision::Selected(index) => {
                match prediction.alternative(index).map_err(|e| (byte_at, e))? {
                    LexicalAlternative::Keyword(text) => {
                        let mut actual = input.clone();
                        if !text.encode_utf16().all(|unit| actual.next() == Some(unit)) {
                            return Err((
                                byte_at,
                                "Predicted keyword does not match original source units".into(),
                            ));
                        }
                        (Some(index), text.encode_utf16().count(), None)
                    }
                    LexicalAlternative::Terminal(name) => match terminals
                        .get(name)
                        .ok_or_else(|| (byte_at, "Missing predicted terminal".into()))?
                        .match_document_units_prepared(
                            input.clone(),
                            if PREPARED {
                                prepared.and_then(PreparedLexicalInitial::terminal_registry)
                            } else {
                                None
                            },
                            work,
                        )
                        .map_err(|e| (byte_at, e))?
                    {
                        terminals::DocumentUnitMatch::Matched(end) => (Some(index), end, None),
                        terminals::DocumentUnitMatch::Failed {
                            consumed_units,
                            failure,
                        } => (
                            None,
                            consumed_units,
                            Some((
                                failure.utf16_offset,
                                failure.actual,
                                failure.expected,
                                Some(index),
                                failure.early_exit,
                            )),
                        ),
                    },
                }
            }
            lexer_prediction::LexicalDecision::Unselected {
                utf16_lookahead,
                unit,
            } => (None, 1, Some((utf16_lookahead, unit, None, None, false))),
        };
        if consumed == 0 {
            return Err((byte_at, "Non-consuming document lexical record".into()));
        }
        let unit_start = unit_at;
        let byte_start = byte_at;
        let start_interior = interior;
        let source_units = input.clone().take(consumed);
        for _ in 0..consumed {
            input.next().ok_or_else(|| {
                (
                    byte_at,
                    "Document lexical result escapes source units".into(),
                )
            })?;
            let ch = source
                .get(byte_at..)
                .and_then(|s| s.chars().next())
                .ok_or_else(|| (byte_at, "Missing original source scalar".into()))?;
            if interior {
                byte_at += ch.len_utf8();
                interior = false
            } else if ch.len_utf16() == 2 {
                interior = true
            } else {
                byte_at += ch.len_utf8()
            }
            unit_at = unit_at
                .checked_add(1)
                .ok_or_else(|| (byte_at, "Document source position overflow".into()))?;
        }
        let end_cover = if interior {
            byte_at
                + source[byte_at..]
                    .chars()
                    .next()
                    .ok_or_else(|| (byte_at, "Missing interior scalar".into()))?
                    .len_utf8()
        } else {
            byte_at
        };
        let exact_bytes = (!start_interior && !interior).then_some(byte_start..byte_at);
        let text = if let Some(range) = &exact_bytes {
            Cow::Borrowed(&source[range.clone()])
        } else {
            // Display only. The exact units are retained by the source range;
            // this escaped rendering cannot authorize a scalar semantic value.
            Cow::Owned(
                source_units
                    .map(|u| format!("\\u{u:04X}"))
                    .collect::<String>(),
            )
        };
        if let Some((utf16_lookahead, unit, expected, selected_alternative, early_exit)) = failure {
            issues.push(LexicalIssue {
                token: tokens.len(),
                utf16_start: unit_start,
                utf16_end: unit_at,
                utf16_lookahead,
                unit,
                expected,
                selected_alternative,
                early_exit,
            });
        }
        tokens.push(Token {
            text,
            start: byte_start,
            end: end_cover,
            quoted: false,
            lexical_alternative: index,
        });
        ranges.push(DocumentTokenRange {
            utf16_start: unit_start,
            utf16_end: unit_at,
            utf8: exact_bytes,
        });
    }
    Ok((tokens, issues, ranges))
}

struct Scanned<'a> {
    tokens: Vec<Token<Cow<'a, str>>>,
    strings: BTreeMap<usize, Vec<u16>>,
}
struct SyntaxTokens {
    tokens: Vec<Token>,
    strings: BTreeMap<usize, Vec<u16>>,
}
/// One escape decoder, with a lossless syntax representation and scalar model representation.
enum DecodedString {
    Scalar(String),
    Units(Vec<u16>),
}
impl DecodedString {
    fn push_str(&mut self, text: &str) {
        match self {
            Self::Scalar(s) => s.push_str(text),
            Self::Units(s) => s.extend(text.encode_utf16()),
        }
    }
    fn push(&mut self, value: char) {
        match self {
            Self::Scalar(s) => s.push(value),
            Self::Units(s) => {
                let mut units = [0; 2];
                s.extend_from_slice(value.encode_utf16(&mut units));
            }
        }
    }
}

struct LexicalPolicy<'a> {
    keywords: Option<&'a PunctuationKeywords>,
    terminals: Option<&'a BTreeMap<String, Terminal>>,
    hidden: Option<&'a [String]>,
    prediction: Option<&'a LexerPrediction>,
    syntax: bool,
}

/// Definition syntax uses the pinned STRING escapes and ASCII hidden whitespace.
/// Legacy/model callers keep their explicit existing lexical policy.
fn lex_syntax(source: &str) -> Result<SyntaxTokens, (usize, String)> {
    scan(
        source,
        Some(50_000),
        LexicalPolicy {
            keywords: None,
            terminals: None,
            hidden: None,
            prediction: None,
            syntax: true,
        },
        &mut None,
    )
    .map(|scanned| SyntaxTokens {
        strings: scanned.strings,
        tokens: scanned
            .tokens
            .into_iter()
            .map(|t| Token {
                text: t.text.into_owned(),
                start: t.start,
                end: t.end,
                quoted: t.quoted,
                lexical_alternative: t.lexical_alternative,
            })
            .collect(),
    })
}

/// Compatibility bridge: validate the complete syntax tree, then feed decoded
/// tokens to the existing bounded semantic compiler. Syntax import never enables
/// an unimplemented runtime construct. Older profiles retain their frozen lexer.
fn definition_tokens(
    source: &str,
    profile: Profile,
    declarations: usize,
) -> Result<Vec<Token>, Diagnostic> {
    let error = |(at, message)| Diagnostic::new("xtext", at, at + 1, message);
    if !profile.has_validated_syntax() {
        return lex(source).map_err(error);
    }
    let input = intersect_model::definitions::bundle::DefinitionSource::new(
        "xtext",
        intersect_model::definitions::bundle::SourceKind::Xtext,
        source,
    );
    syntax::parse(
        &input,
        SourceLimits {
            source_bytes: source.len(),
            declarations,
        },
    )?;
    let mut scanned = lex_syntax(source).map_err(error)?;
    for token in &mut scanned.tokens {
        if token.quoted {
            let units = scanned.strings.remove(&token.start).ok_or_else(|| {
                Diagnostic::new(
                    "xtext",
                    token.start,
                    token.end,
                    "Missing syntax string value",
                )
            })?;
            token.text = String::from_utf16(&units).map_err(|_| {
                Diagnostic::new(
                    "xtext",
                    token.start,
                    token.end,
                    "Executable grammar strings cannot contain unpaired UTF-16 surrogates",
                )
            })?;
        }
    }
    Ok(scanned.tokens)
}

fn unicode_escape(
    iter: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    at: usize,
) -> Result<(u16, usize), (usize, String)> {
    let mut value = 0u16;
    let mut end = at;
    for _ in 0..4 {
        let (i, c) = iter
            .next()
            .ok_or((at, "Incomplete Unicode escape".into()))?;
        value = value * 16 + c.to_digit(16).ok_or((i, "Invalid Unicode escape".into()))? as u16;
        end = i + c.len_utf8();
    }
    Ok((value, end))
}

fn scan<'a>(
    source: &'a str,
    token_limit: Option<usize>,
    policy: LexicalPolicy<'_>,
    work: &mut Option<usize>,
) -> Result<Scanned<'a>, (usize, String)> {
    let LexicalPolicy {
        keywords,
        terminals,
        hidden,
        prediction,
        syntax,
    } = policy;
    // Bounded first allocation: large sources otherwise regrow and copy the
    // token vector many times. The token ceiling still decides admission.
    let mut tokens = Vec::with_capacity(
        (source.len() / 4)
            .min(1 << 20)
            .min(token_limit.map_or(usize::MAX, |limit| limit.saturating_add(1))),
    );
    let mut strings = BTreeMap::new();
    let mut iter = source.char_indices().peekable();
    let mut initial = prediction.map(lexer_prediction::ReusableLexicalInitial::new);
    let terminal_initials = terminals::ReusableTerminalPredictions::default();
    let mut lexemes = LexemeMemo::default();
    while let Some((start, ch)) = iter.next() {
        if hidden.is_none()
            && if syntax {
                matches!(ch, ' ' | '\t' | '\r' | '\n')
            } else {
                ch.is_whitespace()
            }
        {
            continue;
        }
        if hidden.is_none() && ch == '/' && iter.peek().is_some_and(|(_, c)| *c == '/') {
            while let Some((offset, c)) = iter.next() {
                if c == '\n' {
                    break;
                }
                if syntax && c == '\r' {
                    if !matches!(iter.next(), Some((_, '\n'))) {
                        return Err((
                            offset,
                            "Syntax line-comment CR must be followed by LF".into(),
                        ));
                    }
                    break;
                }
            }
            continue;
        }
        if hidden.is_none() && ch == '/' && iter.peek().is_some_and(|(_, c)| *c == '*') {
            iter.next();
            let mut closed = false;
            while let Some((_, c)) = iter.next() {
                if c == '*' && iter.peek().is_some_and(|(_, n)| *n == '/') {
                    iter.next();
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err((start, "Unclosed comment".into()));
            }
            continue;
        }
        let mut decoded: Option<DecodedString> = None;
        let mut segment = start + ch.len_utf8();
        let mut end = start + ch.len_utf8();
        let quoted = hidden.is_none() && (ch == '\'' || ch == '"');
        let mut predicted_end = None;
        let predicted = if let Some(initial) = &mut initial {
            let prediction = initial.program();
            let (index, end) = predicted_lexeme(
                source,
                start,
                terminals,
                initial,
                &terminal_initials,
                &mut lexemes,
                work,
            )?
            .ok_or_else(|| (start, "No viable lexical alternative".into()))?;
            predicted_end = Some(end);
            Some((
                index,
                prediction
                    .alternative(index)
                    .map_err(|error| (start, error))?,
            ))
        } else {
            None
        };
        let selected_terminal = match predicted.as_ref().map(|(_, alternative)| *alternative) {
            Some(LexicalAlternative::Terminal(name)) => Some(
                terminals
                    .and_then(|programs| programs.get_key_value(name))
                    .ok_or_else(|| (start, "Missing predicted terminal".into()))?,
            ),
            Some(LexicalAlternative::Keyword(_)) => None,
            None => terminals.and_then(|programs| terminals::selected(programs, &source[start..])),
        };
        let keyword_length = match predicted.as_ref().map(|(_, alternative)| *alternative) {
            Some(LexicalAlternative::Keyword(text)) => {
                if !source[start..].starts_with(text) {
                    return Err((start, format!("Expected keyword {text}")));
                }
                Some(text.len())
            }
            Some(LexicalAlternative::Terminal(_)) => None,
            None => keywords.and_then(|matcher| matcher.match_len(&source[start..])),
        };
        if let Some((name, terminal)) = selected_terminal {
            end = if let Some(end) = predicted_end {
                end
            } else {
                start
                    + terminal
                        .match_len_reusing(&source[start..], &terminal_initials, work)
                        .map_err(|error| (start, error))?
            };
            while iter.peek().is_some_and(|(offset, _)| *offset < end) {
                iter.next();
            }
            if hidden.is_some_and(|hidden| hidden.contains(name)) {
                continue;
            }
        } else if let Some(length) = keyword_length {
            end = start + length;
            while iter.peek().is_some_and(|(offset, _)| *offset < end) {
                iter.next();
            }
        } else if hidden.is_some() {
            return Err((
                start,
                "No declared terminal or keyword accepts this character".into(),
            ));
        } else if quoted {
            if syntax {
                decoded = Some(DecodedString::Units(Vec::new()));
            }
            let mut closed = false;
            while let Some((i, c)) = iter.next() {
                end = i + c.len_utf8();
                if c == ch {
                    if let Some(text) = &mut decoded {
                        text.push_str(&source[segment..i]);
                    }
                    closed = true;
                    break;
                }
                if c == '\\' {
                    let (j, escaped) = iter.next().ok_or((i, "Incomplete escape".into()))?;
                    end = j + escaped.len_utf8();
                    let text = decoded.get_or_insert_with(|| DecodedString::Scalar(String::new()));
                    text.push_str(&source[segment..i]);
                    if syntax && escaped == 'u' {
                        let (value, last) = unicode_escape(&mut iter, j)?;
                        end = last;
                        segment = end;
                        if let DecodedString::Units(units) = text {
                            units.push(value);
                        }
                        continue;
                    }
                    segment = end;
                    text.push(match escaped {
                        'b' if syntax => '\u{0008}',
                        'f' if syntax => '\u{000c}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        _ => return Err((j, "Unsupported escape".into())),
                    });
                }
            }
            if !closed {
                return Err((start, "Unclosed string".into()));
            }
        } else {
            if ch.is_alphanumeric() || ch == '_' {
                while let Some((i, c)) = iter.peek().copied() {
                    if !c.is_alphanumeric() && c != '_' {
                        break;
                    }
                    iter.next();
                    end = i + c.len_utf8();
                }
            }
        }
        let text = decoded.map_or_else(
            || {
                Cow::Borrowed(if quoted {
                    &source[start + ch.len_utf8()..end - ch.len_utf8()]
                } else {
                    &source[start..end]
                })
            },
            |decoded| match decoded {
                DecodedString::Scalar(text) => Cow::Owned(text),
                DecodedString::Units(units) => {
                    strings.insert(start, units);
                    Cow::Borrowed(&source[start + ch.len_utf8()..end - ch.len_utf8()])
                }
            },
        );
        tokens.push(Token {
            text,
            start,
            end,
            quoted,
            lexical_alternative: predicted.map(|(index, _)| index),
        });
        if token_limit.is_some_and(|limit| tokens.len() > limit) {
            return Err((start, "Token budget exceeded".into()));
        }
    }
    Ok(Scanned { tokens, strings })
}

struct SourceHeader {
    name: String,
    parent: Option<String>,
    parent_span: Option<std::ops::Range<usize>>,
}

struct Parser<'policy> {
    capture_origins: bool,
    unordered: Option<Box<unordered::ParserState>>,
    explicit_local_scope: bool,
    package_imports: std::rc::Rc<PackageImports<'policy>>,
    allow_first_predicates: bool,
    allow_constant_guards: bool,
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    aliases: BTreeMap<String, Vec<String>>,
    metamodel_imports: bool,
    hidden: Option<Vec<String>>,
}
impl Parser<'_> {
    fn peek(&self, text: &str) -> bool {
        self.tokens
            .get(self.pos)
            .is_some_and(|t| !t.quoted && t.text == text)
    }
    fn eat(&mut self, text: &str) -> bool {
        if self.peek(text) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn take(&mut self) -> Result<Token, String> {
        let token = self
            .tokens
            .get(self.pos)
            .cloned()
            .ok_or("Unexpected end of grammar")?;
        self.pos += 1;
        Ok(token)
    }
    fn require(&mut self, text: &str) -> Result<(), String> {
        if self.eat(text) {
            Ok(())
        } else {
            Err(format!("Expected '{text}'"))
        }
    }
    /// Decode an escaped definition identifier while retaining its full byte span.
    /// Model lexing remains governed by the language's own terminals/converters.
    fn identifier_token(&mut self, token: Token) -> Result<Token, String> {
        if token.quoted || token.text != "^" {
            return Ok(token);
        }
        let mut identifier = self.take()?;
        let mut chars = identifier.text.chars();
        if identifier.quoted
            || token.end != identifier.start
            || !chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("Escaped identifier requires an adjacent ASCII identifier".into());
        }
        identifier.start = token.start;
        Ok(identifier)
    }
    fn name(&mut self) -> Result<String, String> {
        let token = self.take()?;
        let t = self.identifier_token(token)?;
        if t.quoted
            || !t
                .text
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
        {
            return Err("Expected identifier".into());
        }
        Ok(t.text)
    }
    fn rule_name_tail(&mut self, mut name: String) -> Result<String, String> {
        while self.peek(":")
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| !t.quoted && t.text == ":")
        {
            self.pos += 2;
            name.push_str("::");
            name.push_str(&self.name()?);
        }
        Ok(name)
    }
    fn qualified(&mut self) -> Result<String, String> {
        let mut name = self.name()?;
        while self.eat(".") {
            name.push('.');
            name.push_str(&self.name()?);
        }
        Ok(name)
    }
    fn type_name(&mut self) -> Result<String, String> {
        let name = self.name()?;
        // A single colon belongs to the rule declaration. Qualification starts
        // only with the complete double-colon token pair.
        if self.peek(":")
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| !token.quoted && token.text == ":")
        {
            self.pos += 2;
            if !self.aliases.contains_key(&name) {
                return Err(format!("Unknown metamodel alias {name}"));
            }
            Ok(format!("{name}::{}", self.name()?))
        } else {
            Ok(name)
        }
    }
    fn header(&mut self, schema: &Schema) -> Result<String, String> {
        self.source_header(schema, false).map(|header| header.name)
    }
    fn source_header(
        &mut self,
        schema: &Schema,
        composition: bool,
    ) -> Result<SourceHeader, String> {
        if self.metamodel_imports {
            self.aliases.entry(String::new()).or_default();
        }
        self.require("grammar")?;
        let name = self.qualified()?;
        let inherited = self.eat("with");
        let mut parent_span = None;
        let parent = if inherited {
            let start = self.tokens.get(self.pos).map_or(0, |token| token.start);
            let name = self.qualified()?;
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .ok_or("Missing parent grammar name")?
                .end;
            parent_span = Some(start..end);
            Some(name)
        } else {
            None
        };
        if composition && self.peek(",") {
            return Err("Multiple grammar inheritance is unsupported".into());
        }
        if !composition
            && parent
                .as_deref()
                .is_some_and(|name| name != "org.eclipse.xtext.common.Terminals")
        {
            return Err("Only common.Terminals inheritance is supported".into());
        }
        if self.eat("hidden") {
            if inherited && !composition {
                return Err(
                    "Explicit hidden policy with grammar inheritance is not yet supported".into(),
                );
            }
            self.require("(")?;
            let mut hidden = Vec::new();
            if !self.peek(")") {
                loop {
                    if hidden.len() >= 128 {
                        return Err("Hidden terminal budget exceeded".into());
                    }
                    let token = self.name()?;
                    if hidden.contains(&token) {
                        return Err(format!("Duplicate hidden terminal {token}"));
                    }
                    hidden.push(token);
                    if !self.eat(",") {
                        break;
                    }
                }
            }
            self.require(")")?;
            self.hidden = Some(hidden);
        }
        while self.peek("import")
            || (self.package_imports.inferred_headers && self.peek("generate"))
        {
            if self.eat("generate") {
                self.name()?;
            } else {
                self.require("import")?;
            }
            if self.aliases.values().map(Vec::len).sum::<usize>() >= 128 {
                return Err("Grammar import budget exceeded".into());
            }
            let uri = self.take()?;
            if !uri.quoted
                || (uri.text != schema.uri
                    && uri.text != intersect_model::definitions::ecore::ECORE
                    && !self
                        .package_imports
                        .declarations
                        .packages()
                        .contains_key(&uri.text)
                    && !(self.metamodel_imports
                        && schema
                            .declarations
                            .values()
                            .any(|d| d.namespace == uri.text)))
            {
                return Err(
                    "Grammar import must match the selected EPackage or pinned Ecore nsURI".into(),
                );
            }
            let alias = if self.eat("as") {
                self.name()?
            } else if self.metamodel_imports {
                String::new()
            } else {
                return Err("Expected as".into());
            };
            if self.metamodel_imports {
                if self.aliases.values().flatten().any(|v| v == &uri.text) {
                    return Err(format!("Duplicate metamodel namespace {}", uri.text));
                }
                self.aliases.entry(alias).or_default().push(uri.text);
            } else if self.aliases.insert(alias.clone(), vec![uri.text]).is_some() {
                return Err(format!("Duplicate metamodel alias {alias}"));
            }
        }
        if !self.explicit_local_scope
            && !self
                .aliases
                .values()
                .flatten()
                .any(|namespace| namespace == &schema.uri)
        {
            return Err("Grammar requires an explicit selected EPackage import".into());
        }
        Ok(SourceHeader {
            name,
            parent,
            parent_span,
        })
    }
    // None is structural omission during constant-guard specialization, not an
    // executable epsilon branch. Keep it private to parsing; serialized grammar
    // programs continue to use the existing expression representation.
    fn original_range(&self, start: usize) -> Option<std::ops::Range<usize>> {
        if !self.capture_origins || self.pos <= start {
            return None;
        }
        self.tokens
            .get(start)
            .zip(self.tokens.get(self.pos - 1))
            .map(|(first, last)| first.start..last.end)
    }
    fn expression(&mut self, full_predicates: bool) -> Result<Option<Expr>, String> {
        let start = self.pos;
        let mut expression = self.expression_body(full_predicates)?;
        if let Some(expression) = &mut expression
            && expression.original_range.is_none()
        {
            expression.original_range = self.original_range(start);
        }
        Ok(expression)
    }
    fn expression_body(&mut self, full_predicates: bool) -> Result<Option<Expr>, String> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("Grammar nesting budget exceeded".into());
        }
        let mut alternatives = vec![self.unordered(full_predicates)?];
        while self.eat("|") {
            alternatives.push(self.unordered(full_predicates)?);
        }
        self.depth -= 1;
        if alternatives.len() == 1 {
            return Ok(alternatives.remove(0));
        }
        let alternatives = alternatives.into_iter().flatten().collect::<Vec<_>>();
        Ok(Some(Expr {
            // An existing compound whose children were all removed remains an
            // empty compound. An inactive branch in a mixed choice disappears.
            kind: if alternatives.is_empty() {
                Kind::Sequence(vec![])
            } else {
                Kind::Alternative(alternatives)
            },
            first_set_predicated: false,
            original_range: None,
            prediction_path: None,
            cardinality: '1',
        }))
    }
    fn unordered(&mut self, full_predicates: bool) -> Result<Option<Expr>, String> {
        let start = self.pos;
        let first = self.sequence(full_predicates)?;
        if self.unordered.is_none() || !self.eat("&") {
            return Ok(first);
        }
        let mut members = vec![first];
        loop {
            members.push(self.sequence(full_predicates)?);
            if !self.eat("&") {
                break;
            }
        }
        let members: Vec<_> = members.into_iter().flatten().collect();
        if members.is_empty() {
            return Ok(None);
        }
        let range = self
            .original_range(start)
            .ok_or("Unordered lowering needs exact source ranges")?;
        let unordered = self
            .unordered
            .as_mut()
            .ok_or("Missing unordered parser context")?;
        let pending = unordered::Pending::new(&unordered.namespace, range, members);
        let call = pending.entry()?;
        unordered.pending.push(pending);
        Ok(Some(call))
    }
    fn sequence(&mut self, full_predicates: bool) -> Result<Option<Expr>, String> {
        let start = self.pos;
        let mut expression = self.sequence_body(full_predicates)?;
        if let Some(expression) = &mut expression
            && expression.original_range.is_none()
        {
            expression.original_range = self.original_range(start);
        }
        Ok(expression)
    }
    fn sequence_body(&mut self, full_predicates: bool) -> Result<Option<Expr>, String> {
        let condition = if self.allow_constant_guards && self.eat("<") {
            let token = self.take()?;
            let value = match (token.quoted, token.text.as_str()) {
                (false, "true") => true,
                (false, "false") => false,
                _ => return Err("Expected a specialized boolean guard".into()),
            };
            self.require(">")?;
            Some(value)
        } else {
            None
        };
        let mut items = Vec::new();
        let mut authored_items = 0;
        while self.pos < self.tokens.len()
            && !self.peek("|")
            && !self.peek(")")
            && !self.peek(";")
            && (self.unordered.is_none() || !self.peek("&"))
        {
            // Parse the complete body even when its guard is false. Source
            // validation and language-owned argument binding must not be bypassed.
            let mut atom = self.atom(full_predicates)?;
            authored_items += 1;
            for c in ['?', '*', '+'] {
                if self.eat(&c.to_string()) {
                    if let Some(atom) = &mut atom {
                        atom.cardinality = c;
                        if let Some(range) = &mut atom.original_range
                            && self.capture_origins
                        {
                            range.end = self.tokens[self.pos - 1].end;
                        }
                    }
                    break;
                }
            }
            items.extend(atom);
        }
        if authored_items == 0 {
            return Err("Empty grammar branch is unsupported".into());
        }
        if condition == Some(false)
            || (condition.is_none() && authored_items == 1 && items.is_empty())
        {
            return Ok(None);
        }
        Ok(Some(if items.len() == 1 {
            items.remove(0)
        } else {
            Expr {
                kind: Kind::Sequence(items),
                first_set_predicated: false,
                original_range: None,
                prediction_path: None,
                cardinality: '1',
            }
        }))
    }
    fn atom(&mut self, full_predicates: bool) -> Result<Option<Expr>, String> {
        let start = self.pos;
        let mut expression = self.atom_body(full_predicates)?;
        if let Some(expression) = &mut expression
            && expression.original_range.is_none()
        {
            expression.original_range = self.original_range(start);
        }
        Ok(expression)
    }
    fn atom_body(&mut self, full_predicates: bool) -> Result<Option<Expr>, String> {
        let token = self.take()?;
        let token = self.identifier_token(token)?;
        if !token.quoted && token.text == "=" && full_predicates {
            let arrow = self.take()?;
            if arrow.quoted || arrow.text != ">" || token.end != arrow.start {
                return Err("Syntactic predicate requires contiguous '=>'".into());
            }
            if self.peek("=") || self.peek("-") {
                return Err("Repeated predicate prefixes are unsupported".into());
            }
            let value = self.atom(full_predicates)?;
            return Ok(value.map(|value| Expr {
                kind: Kind::SyntacticPredicate(Box::new(value)),
                first_set_predicated: false,
                original_range: None,
                prediction_path: None,
                cardinality: '1',
            }));
        }
        if !token.quoted && token.text == "-" && self.allow_first_predicates {
            let arrow = self.take()?;
            if arrow.quoted || arrow.text != ">" || token.end != arrow.start {
                return Err("First-token predicate requires contiguous '->'".into());
            }
            if self.peek("-") {
                return Err("Repeated first-token predicate prefixes are unsupported".into());
            }
            let mut value = self.atom(full_predicates)?;
            if let Some(value) = &mut value {
                value.first_set_predicated = true;
            }
            return Ok(value);
        }
        let kind = if token.quoted {
            Kind::Literal(token.text)
        } else if token.text == "(" {
            let value = self.expression(full_predicates)?;
            self.require(")")?;
            return Ok(value.map(|value| Expr {
                kind: Kind::Sequence(vec![value]),
                first_set_predicated: false,
                original_range: None,
                prediction_path: None,
                cardinality: '1',
            }));
        } else if token.text == "{" {
            let class = self.type_name()?;
            if self.eat(".") {
                let feature = self.name()?;
                let many = self.eat("+");
                self.require("=")?;
                if self.name()? != "current" {
                    return Err("Assigned actions require current".into());
                }
                self.require("}")?;
                Kind::Capture {
                    class,
                    feature,
                    many,
                }
            } else {
                self.require("}")?;
                Kind::Action(class)
            }
        } else if token.text == "[" {
            let target = self.type_name()?;
            let spelling = if self.eat("|") {
                let first = self.name()?;
                self.rule_name_tail(first)?
            } else {
                "ID".into()
            };
            self.require("]")?;
            if spelling == "ID" {
                Kind::Reference(target)
            } else {
                Kind::ScalarReference {
                    target,
                    rule: spelling,
                }
            }
        } else if token
            .text
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        {
            let many =
                self.peek("+") && self.tokens.get(self.pos + 1).is_some_and(|t| t.text == "=");
            let boolean = self.peek("?")
                && self.tokens.get(self.pos + 1).is_some_and(|t| {
                    !t.quoted && t.text == "=" && self.tokens[self.pos].end == t.start
                });
            if boolean {
                self.pos += 1;
                self.require("=")?;
                Kind::BooleanAssign {
                    feature: token.text,
                    value: Box::new(self.atom(full_predicates)?.ok_or_else(|| {
                        "Assignment to an omitted guarded group is unsupported".to_string()
                    })?),
                }
            } else if self.peek("=") || many {
                if many {
                    self.pos += 1;
                }
                self.require("=")?;
                Kind::Assign {
                    feature: token.text,
                    many,
                    value: Box::new(self.atom(full_predicates)?.ok_or_else(|| {
                        "Assignment to an omitted guarded group is unsupported".to_string()
                    })?),
                }
            } else {
                Kind::Call(self.rule_name_tail(token.text)?)
            }
        } else {
            return Err(format!("Unsupported Xtext construct '{}'", token.text));
        };
        Ok(Some(Expr {
            kind,
            first_set_predicated: false,
            original_range: None,
            prediction_path: None,
            cardinality: '1',
        }))
    }
}

/// Resolve retained imported identity through the authoritative runtime type path.
fn imported_class_key<'a>(
    schema: &'a Schema,
    namespace: &str,
    name: &str,
) -> Result<&'a str, String> {
    if namespace == schema.uri || namespace == intersect_model::definitions::ecore::ECORE {
        return type_names::rule_class_key(schema, namespace, name);
    }
    let mut declarations = schema
        .declarations
        .values()
        .filter(|d| d.namespace == namespace && d.name == name);
    let declaration = declarations
        .next()
        .ok_or_else(|| format!("Unknown imported classifier {namespace}::{name}"))?;
    if declarations.next().is_some() {
        return Err(format!("Ambiguous imported classifier {namespace}::{name}"));
    }
    let receiver = intersect_model::definitions::generic_types::RuntimeType::new(
        intersect_model::definitions::generic_types::Type::Classifier {
            declaration: intersect_model::definitions::resources::DeclarationId {
                source: declaration.source.clone(),
                path: declaration.path.clone(),
            },
            arguments: vec![],
            raw: false,
        },
    )?;
    schema.class_key_for_type(&receiver)
}

fn nominal_return_key<'a>(
    schema: &'a Schema,
    imports: &PackageImports<'_>,
    namespace: &str,
    name: &str,
    remaining: &mut usize,
    records: &mut usize,
) -> Result<
    Option<(
        &'a str,
        intersect_model::definitions::resources::DeclarationId,
    )>,
    String,
> {
    if !imports.collect_classifiers || imports.validate_authored.is_none() {
        return Ok(None);
    }
    let mut selected = None;
    for (key, datatype) in &schema.datatypes {
        if datatype.name() != name || !datatype.serializable() {
            continue;
        }
        let declaration = datatype.declaration();
        let exact = if namespace == intersect_model::definitions::ecore::ECORE {
            let source = intersect_model::definitions::compiler::standard_datatype_source();
            if declaration.source != source.id {
                false
            } else {
                let bundle = intersect_model::definitions::bundle::DefinitionBundle {
                    version: 1,
                    sources: vec![source.clone()],
                    namespaces: vec![],
                };
                let resources =
                    intersect_model::definitions::resources::ResourceSet::new_with_limits(
                        &bundle,
                        intersect_model::definitions::ecore::ParseLimits {
                            source_bytes: imports
                                .authored_source_limits
                                .unwrap_or_default()
                                .source_bytes,
                        },
                    )
                    .map_err(|error| error.to_string())?;
                resources.resolve(&source.id, &format!("#//{name}"))? == *declaration
            }
        } else {
            imports
                .declarations
                .classifiers()
                .get(namespace)
                .and_then(|items| items.get(name))
                == Some(declaration)
        };
        if exact {
            if selected.is_some() {
                return Err("Ambiguous exact nominal return datatype".into());
            }
            selected = Some((key, declaration));
        }
    }
    let Some((key, declaration)) = selected else {
        return Ok(None);
    };
    if *records == 0 {
        return Err("Nominal return identity declaration budget exceeded".into());
    }
    let bytes = intersect_model::definitions::metadata_limits::serialized_size(
        &(key, declaration),
        *remaining,
    )?;
    *remaining = remaining
        .checked_sub(bytes)
        .ok_or("Nominal return identity byte budget exceeded")?;
    *records -= 1;
    Ok(Some((key.as_str(), declaration.clone())))
}

fn pinned_ecore_classifiers() -> Result<&'static BTreeSet<String>, String> {
    static NAMES: std::sync::OnceLock<Result<BTreeSet<String>, String>> =
        std::sync::OnceLock::new();
    NAMES
        .get_or_init(|| {
            let document = intersect_model::definitions::ecore::Document::parse(
                intersect_model::definitions::compiler::standard_datatype_source(),
            )
            .map_err(|e| e.to_string())?;
            Ok(document
                .root
                .children
                .iter()
                .filter(|n| n.name.local == "eClassifiers")
                .flat_map(|n| {
                    n.attributes
                        .iter()
                        .filter(|a| a.name.namespace.is_none() && a.name.local == "name")
                        .map(|a| a.value.clone())
                })
                .collect())
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn imported_type<'a>(
    name: &'a str,
    aliases: &'a BTreeMap<String, Vec<String>>,
    schema: &'a Schema,
    profile: Profile,
    imports: &PackageImports,
) -> Result<(&'a str, &'a str), String> {
    // Explicit TypeRefs use every local metamodel declaration, irrespective of
    // alias. Inferred return types follow their separate default-package path.
    if matches!(
        profile,
        Profile::NativeV45
            | Profile::NativeV46
            | Profile::NativeV47
            | Profile::NativeV48
            | Profile::NativeV49
            | Profile::NativeV50
            | Profile::NativeV51
            | Profile::NativeV52
            | Profile::NativeV53
            | Profile::NativeV54
            | Profile::NativeV55
            | Profile::NativeV56
            | Profile::NativeV57
            | Profile::NativeV58
            | Profile::NativeV59
            | Profile::NativeV60
            | Profile::NativeV61
            | Profile::NativeV62
            | Profile::NativeV63
            | Profile::NativeV64
            | Profile::NativeV65
            | Profile::NativeV66
            | Profile::NativeV67
            | Profile::NativeV68
            | Profile::NativeV69
            | Profile::NativeV70
            | Profile::NativeV71
            | Profile::NativeV72
            | Profile::NativeV73
            | Profile::NativeV74
            | Profile::NativeV75
            | Profile::NativeV76
            | Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) && !name.contains("::")
    {
        let pinned = if aliases
            .values()
            .flatten()
            .any(|uri| uri == intersect_model::definitions::ecore::ECORE)
        {
            Some(pinned_ecore_classifiers()?)
        } else {
            None
        };
        let mut candidates = aliases.values().flatten().filter(|namespace| {
            (namespace.as_str() == intersect_model::definitions::ecore::ECORE
                && pinned.is_some_and(|names| names.contains(name)))
                || imports
                    .declarations
                    .classifiers()
                    .get(namespace.as_str())
                    .is_some_and(|names| names.contains_key(name))
                || (!imports.collect_classifiers
                    && schema.declarations.values().any(|declaration| {
                        declaration.namespace == **namespace && declaration.name == name
                    }))
        });
        let namespace = candidates
            .next()
            .ok_or_else(|| format!("Unknown unqualified classifier {name}"))?;
        if candidates.next().is_some() {
            return Err(format!("Ambiguous unqualified classifier {name}"));
        }
        return Ok((namespace, name));
    }
    let (alias, local) = match name.split_once("::") {
        Some(parts) => parts,
        None if aliases.contains_key("") => ("", name),
        None => return Ok((&schema.uri, name)),
    };
    let namespaces = aliases
        .get(alias)
        .ok_or_else(|| format!("Unknown metamodel alias {alias}"))?;
    if namespaces.len() == 1 {
        return Ok((&namespaces[0], local));
    }
    let pinned = if namespaces
        .iter()
        .any(|n| n == intersect_model::definitions::ecore::ECORE)
    {
        Some(pinned_ecore_classifiers()?)
    } else {
        None
    };
    let mut found = namespaces.iter().filter(|namespace| {
        (namespace.as_str() == intersect_model::definitions::ecore::ECORE
            && pinned.is_some_and(|names| names.contains(local)))
            || (matches!(
                profile,
                Profile::NativeV45
                    | Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) && imports
                .declarations
                .classifiers()
                .get(namespace.as_str())
                .is_some_and(|names| names.contains_key(local)))
            || (!(matches!(
                profile,
                Profile::NativeV45
                    | Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) && imports.collect_classifiers)
                && schema
                    .declarations
                    .values()
                    .any(|d| d.namespace == **namespace && d.name == local))
    });
    let namespace = found
        .next()
        .ok_or_else(|| format!("Unknown classifier {alias}::{local}"))?;
    if found.next().is_some() {
        return Err(format!("Ambiguous classifier {alias}::{local}"));
    }
    Ok((namespace, local))
}

/// Compile syntax over an explicitly inferred, canonical source world. This
/// binds generated declarations to already materialized definitions; it does
/// not infer a metamodel or authorize serialized schema data. The language
/// frontend owns inference and source replay. Ordinary profile entry points
/// retain their original header contract.
pub fn compile_with_generated_metamodels(
    source: &intersect_model::definitions::bundle::DefinitionSource,
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
) -> Result<Grammar, Diagnostic> {
    let imports =
        generated_package_imports(bundle, schema.schema(), limits, &[source], &BTreeMap::new())?;
    let mut grammar = compile_with_reference_context_and_package_imports(
        &source.text,
        schema,
        profile,
        limits.declarations,
        imports,
    )
    .map_err(|mut error| {
        error.source = source.id.uri.clone();
        error.with_source(&source.id)
    })?;
    for span in grammar.declaration_sources.values_mut() {
        span.source = source.id.clone();
    }
    Ok(grammar)
}
/// Source-bound classifications supplied by the language inference frontend.
/// These inputs bind shared executable composition; they are not a validation
/// certificate for serialized definitions or a replacement inference algorithm.
pub struct GeneratedRuleTypes {
    pub source: intersect_model::definitions::bundle::SourceId,
    pub types: BTreeMap<String, (String, String)>,
}
/// Compile exact generated/imported sources through the ordinary source composer.
pub fn compile_generated_sources(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    bindings: &[GeneratedRuleTypes],
) -> Result<Grammar, Diagnostic> {
    compile_generated_source_programs(
        bundle,
        entry,
        schema,
        profile,
        limits,
        bindings,
        &BTreeMap::new(),
    )
}
/// Compile native-derived programs whose exact authored producer context is
/// retained as a dependency. This checked in-process bridge authorizes no
/// serialized metadata and leaves ordinary source compilation unchanged.
pub fn compile_generated_source_programs(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    bindings: &[GeneratedRuleTypes],
    producers: &BTreeMap<
        intersect_model::definitions::bundle::SourceId,
        intersect_model::definitions::bundle::SourceId,
    >,
) -> Result<Grammar, Diagnostic> {
    compile_source_program(
        bundle,
        entry,
        schema,
        profile,
        limits,
        &[],
        &SourceProgram {
            bindings,
            producers,
        },
    )
}

/// Checked in-process grammar program context. Original producer dependencies
/// remain mandatory; this is never a serialized admission certificate.
pub struct SourceProgram<'a> {
    pub bindings: &'a [GeneratedRuleTypes],
    pub producers: &'a BTreeMap<
        intersect_model::definitions::bundle::SourceId,
        intersect_model::definitions::bundle::SourceId,
    >,
}

/// Consume a language-owned program through the same source composer, retaining
/// explicit computed-setting requirements and exact producer validation.
pub fn compile_source_program(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    program: &SourceProgram<'_>,
) -> Result<Grammar, Diagnostic> {
    compile_source_program_context(
        bundle,
        entry,
        schema,
        profile,
        limits,
        settings,
        SourceProgramContext {
            program,
            validate_authored: None,
            authored_source_limits: limits,
        },
    )
}
/// Trusted in-process language adapter. Native code installs this callback;
/// serialized sources, archives and cached metadata cannot select one.
pub type AuthoredSourceValidation = fn(
    &intersect_model::definitions::bundle::DefinitionBundle,
    &intersect_model::definitions::bundle::SourceId,
    SourceLimits,
) -> Result<(), Diagnostic>;
/// Borrowed trusted native policy. Captured adapters remain caller-owned and
/// cannot enter a compiled package or be reconstructed from serialized data.
pub type AuthoredSourceValidator<'policy> = dyn Fn(
        &intersect_model::definitions::bundle::DefinitionBundle,
        &intersect_model::definitions::bundle::SourceId,
        SourceLimits,
    ) -> Result<(), Diagnostic>
    + 'policy;
pub struct BorrowedAuthoredSourceProgramAdapter<'program, 'definitions, 'policy> {
    pub program: &'program SourceProgram<'definitions>,
    pub validate_authored: &'policy AuthoredSourceValidator<'policy>,
}

pub struct AuthoredSourceProgramAdapter<'a, 'b> {
    pub program: &'a SourceProgram<'b>,
    pub validate_authored: AuthoredSourceValidation,
}
struct SourceProgramContext<'a, 'b> {
    program: &'a SourceProgram<'b>,
    validate_authored: Option<&'a AuthoredSourceValidator<'a>>,
    authored_source_limits: SourceLimits,
}
/// Compile a native-derived program with fresh original-source validation.
/// The callback is executable trusted native policy, not an admission certificate.
pub fn compile_source_program_with_authored_validation(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    adapter: &AuthoredSourceProgramAdapter<'_, '_>,
) -> Result<Grammar, Diagnostic> {
    compile_source_program_with_admitted_authored_limits(
        bundle, entry, schema, profile, limits, settings, adapter, limits,
    )
}
/// Borrow an executable source validator without storing it in grammar metadata.
pub fn compile_source_program_with_borrowed_authored_validation(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    adapter: &BorrowedAuthoredSourceProgramAdapter<'_, '_, '_>,
) -> Result<Grammar, Diagnostic> {
    compile_source_program_with_borrowed_authored_limits(
        bundle, entry, schema, profile, limits, settings, adapter, limits,
    )
}
/// Compiler-owned source policy keeps Rcore and grammar admission independent.
#[allow(clippy::too_many_arguments)] // Preserve the established bridge plus the distinct admitted source-world bound.
pub(crate) fn compile_source_program_with_admitted_authored_limits(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    adapter: &AuthoredSourceProgramAdapter<'_, '_>,
    authored_source_limits: SourceLimits,
) -> Result<Grammar, Diagnostic> {
    compile_source_program_context(
        bundle,
        entry,
        schema,
        profile,
        limits,
        settings,
        SourceProgramContext {
            program: adapter.program,
            validate_authored: Some(&adapter.validate_authored),
            authored_source_limits,
        },
    )
}
#[allow(clippy::too_many_arguments)] // Existing compiler bridge plus its borrowed trusted callback.
pub fn compile_source_program_with_borrowed_authored_limits(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    adapter: &BorrowedAuthoredSourceProgramAdapter<'_, '_, '_>,
    authored_source_limits: SourceLimits,
) -> Result<Grammar, Diagnostic> {
    compile_source_program_context(
        bundle,
        entry,
        schema,
        profile,
        limits,
        settings,
        SourceProgramContext {
            program: adapter.program,
            validate_authored: Some(adapter.validate_authored),
            authored_source_limits,
        },
    )
}
fn compile_source_program_context(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    schema: &mut impl StructureContext,
    profile: Profile,
    limits: SourceLimits,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    context: SourceProgramContext<'_, '_>,
) -> Result<Grammar, Diagnostic> {
    let program = context.program;
    let bindings = program.bindings;
    let producers = program.producers;
    let fail = |message| Diagnostic::new(&entry.uri, 0, 1, message).with_context(entry);
    let sources = bindings
        .iter()
        .map(|binding| bundle.source(&binding.source).map_err(fail))
        .collect::<Result<Vec<_>, _>>()?;
    for program in producers.keys() {
        if !bindings.iter().any(|binding| binding.source == *program) {
            return Err(fail("Unconsumed generated program producer context".into()));
        }
    }
    if profile.has_contextual_first_rule()
        && !profile.has_terminal_cardinality_decisions()
        && !producers.is_empty()
    {
        bundle.validate().map_err(fail)?;
        let original = producers
            .get(entry)
            .ok_or_else(|| fail("Missing exact authored producer for V61 program entry".into()))?;
        bundle.source(original).map_err(fail)?;
        let validate = context.validate_authored.ok_or_else(|| {
            Diagnostic::at_source(
                original,
                0,
                1,
                "V61 source programs require a fresh authored-source validator",
            )
        })?;
        validate(bundle, original, limits)?;
    }
    let mut imports: PackageImports<'_> =
        generated_package_imports(bundle, schema.schema(), limits, &sources, producers)?;
    imports.validate_authored = context.validate_authored;
    imports.authored_source_limits = Some(context.authored_source_limits);
    for (binding, source) in bindings.iter().zip(sources) {
        let syntax = syntax::parse(source, limits)?;
        for name in binding.types.keys() {
            if !syntax
                .rules
                .iter()
                .any(|rule| rule.value.name.value == *name)
            {
                return Err(fail("Inferred return names a missing source rule".into()));
            }
        }
        if imports
            .inferred_rules
            .insert(syntax.name.value, binding.types.clone())
            .is_some()
        {
            return Err(fail("Duplicate inferred grammar identity".into()));
        }
    }
    sources::compile_sources_with_package_imports(
        bundle, entry, schema, profile, limits, settings, imports,
    )
}
fn generated_package_imports<'policy>(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    schema: &Schema,
    limits: SourceLimits,
    sources: &[&intersect_model::definitions::bundle::DefinitionSource],
    producers: &BTreeMap<
        intersect_model::definitions::bundle::SourceId,
        intersect_model::definitions::bundle::SourceId,
    >,
) -> Result<PackageImports<'policy>, Diagnostic> {
    let resources = intersect_model::definitions::resources::ResourceSet::new_with_limits(
        bundle,
        intersect_model::definitions::ecore::ParseLimits {
            source_bytes: limits.source_bytes,
        },
    )
    .map_err(|e| Diagnostic::new("xtext", 0, 1, e.to_string()).with_span(e.span.clone()))?;
    let mut imports = PackageImports {
        inferred_headers: true,
        collect_classifiers: true,
        ..Default::default()
    };
    for source in sources {
        let fail = |message| Diagnostic::at_source(&source.id, 0, source.text.len(), message);
        if bundle.source(&source.id).map_err(fail)? != *source {
            return Err(fail("Missing exact inferred grammar source".into()));
        }
        let syntax = syntax::parse(source, limits)?;
        let producer = producers.get(&source.id).unwrap_or(&source.id);
        if producer != &source.id {
            if !source.dependencies.contains(producer) {
                return Err(fail(
                    "Derived grammar must retain its exact authored producer dependency".into(),
                ));
            }
            let original = bundle.source(producer).map_err(fail)?;
            let original = syntax::parse(original, limits)?;
            let declarations = |syntax: &syntax::Syntax| {
                syntax
                    .metamodels
                    .iter()
                    .map(|entry| {
                        (
                            entry
                                .value
                                .generated_name
                                .as_ref()
                                .map(|name| name.value.clone()),
                            entry.value.uri.value.clone(),
                            entry.value.alias.as_ref().map(|alias| alias.value.clone()),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            if syntax.name.value != original.name.value
                || syntax
                    .parents
                    .iter()
                    .map(|p| &p.value)
                    .ne(original.parents.iter().map(|p| &p.value))
                || declarations(&syntax) != declarations(&original)
            {
                return Err(fail(
                    "Derived grammar changed its authored producer header context".into(),
                ));
            }
            imports
                .setting_sources
                .insert(source.id.clone(), producer.clone());
        }
        for entry in &syntax.metamodels {
            if entry.value.generated_name.is_none() {
                continue;
            }
            let namespace = entry.value.uri.value.to_scalar_string().map_err(fail)?;
            let declaration = resources
                .namespace_binding(&namespace)
                .ok_or_else(|| fail("Unbound generated package".into()))?;
            let generated = bundle.source(&declaration.source).map_err(fail)?;
            if generated.kind != intersect_model::definitions::bundle::SourceKind::Rcore
                || !generated.dependencies.contains(producer)
            {
                return Err(fail(
                    "Generated package must retain its exact grammar dependency".into(),
                ));
            }
        }
    }
    imports.declarations = resources
        .inspect_schema_declarations(
            bundle
                .sources
                .iter()
                .filter(|input| {
                    input.kind == intersect_model::definitions::bundle::SourceKind::Rcore
                })
                .map(|input| &input.id),
            schema,
        )
        .map_err(|error| Diagnostic::at_source(&error.span.source, 0, 1, error.message))?;
    Ok(imports)
}
/// Prepare only explicitly imported pinned reference context. Failed compilation
/// restores the caller's context; repeated compilation never hashes it twice.
pub fn compile_with_reference_context(
    source: &str,
    schema: &mut impl StructureContext,
) -> Result<Grammar, Diagnostic> {
    compile_with_reference_context_and_profile(source, schema, Profile::default())
}

pub fn compile_with_reference_context_and_profile(
    source: &str,
    schema: &mut impl StructureContext,
    profile: Profile,
) -> Result<Grammar, Diagnostic> {
    compile_with_reference_context_and_declaration_limit(source, schema, profile, 128)
}
pub(crate) fn compile_with_reference_context_and_declaration_limit(
    source: &str,
    schema: &mut impl StructureContext,
    profile: Profile,
    declaration_limit: usize,
) -> Result<Grammar, Diagnostic> {
    compile_with_reference_context_and_package_imports(
        source,
        schema,
        profile,
        declaration_limit,
        PackageImports::default(),
    )
}

pub fn compile_with_reference_context_and_package_imports(
    source: &str,
    schema: &mut impl StructureContext,
    profile: Profile,
    declaration_limit: usize,
    package_imports: PackageImports,
) -> Result<Grammar, Diagnostic> {
    if profile.has_terminal_cardinality_decisions() {
        return Err(Diagnostic::new(
            "xtext",
            0,
            1,
            "NativeV63 requires an exact source bundle and language-owned authored-source validation",
        ));
    }
    let tokens = definition_tokens(source, profile, declaration_limit)?;
    let mut parser = Parser {
        capture_origins: false,
        unordered: None,
        explicit_local_scope: matches!(
            profile,
            Profile::NativeV45
                | Profile::NativeV46
                | Profile::NativeV47
                | Profile::NativeV48
                | Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ),
        package_imports: std::rc::Rc::new(package_imports),
        allow_first_predicates: profile.has_parser_prediction(),
        allow_constant_guards: profile.has_constant_guards(),
        tokens,
        pos: 0,
        depth: 0,
        aliases: BTreeMap::new(),
        metamodel_imports: matches!(
            profile,
            Profile::NativeV20
                | Profile::NativeV21
                | Profile::NativeV22
                | Profile::NativeV23
                | Profile::NativeV24
                | Profile::NativeV25
                | Profile::NativeV26
                | Profile::NativeV27
                | Profile::NativeV28
                | Profile::NativeV29
                | Profile::NativeV30
                | Profile::NativeV31
                | Profile::NativeV32
                | Profile::NativeV33
                | Profile::NativeV34
                | Profile::NativeV35
                | Profile::NativeV36
                | Profile::NativeV37
                | Profile::NativeV38
                | Profile::NativeV39
                | Profile::NativeV40
                | Profile::NativeV41
                | Profile::NativeV42
                | Profile::NativeV43
                | Profile::NativeV44
                | Profile::NativeV45
                | Profile::NativeV46
                | Profile::NativeV47
                | Profile::NativeV48
                | Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ),
        hidden: None,
    };
    parser.header(schema.schema()).map_err(|error| {
        let at = parser
            .tokens
            .get(parser.pos.saturating_sub(1))
            .map_or(0, |token| token.start);
        Diagnostic::new("xtext", at, at + 1, error)
    })?;
    let mut context = schema.reference_context();
    let schema = &mut context;
    let result = (|| {
        if parser
            .aliases
            .values()
            .flatten()
            .any(|namespace| namespace == intersect_model::definitions::ecore::ECORE)
        {
            install_reference_top(schema).map_err(|error| Diagnostic::new("xtext", 0, 1, error))?;
        }
        schema
            .validate_reference_top()
            .map_err(|error| Diagnostic::new("xtext", 0, 1, error))?;
        compile_with_setting_declarations(
            source,
            schema,
            profile,
            declaration_limit,
            Vec::new(),
            std::rc::Rc::clone(&parser.package_imports),
        )
    })();
    if result.is_ok() {
        context.commit();
    }
    result
}

/// Prepare the canonical Ecore EObject reference context before binding exact
/// provider contracts for a grammar that imports Ecore. This changes the schema
/// revision once; ordinary grammar compilation uses the same idempotent operation.
/// It neither admits grammar constructs nor installs providers.
pub fn prepare_ecore_reference_context(schema: &mut impl StructureContext) -> Result<(), String> {
    let mut context = schema.reference_context();
    context.enable_eobject_references()?;
    context.commit();
    Ok(())
}

fn install_reference_top(schema: &mut ReferenceContextTransaction<'_>) -> Result<(), String> {
    schema.enable_eobject_references()
}

pub fn compile(source: &str, schema: &Schema) -> Result<Grammar, Diagnostic> {
    compile_with_profile(source, schema, Profile::default())
}

pub fn compile_with_profile(
    source: &str,
    schema: &Schema,
    profile: Profile,
) -> Result<Grammar, Diagnostic> {
    if profile.has_terminal_cardinality_decisions() {
        return Err(Diagnostic::new(
            "xtext",
            0,
            1,
            "NativeV63 requires an exact source bundle and language-owned authored-source validation",
        ));
    }
    compile_with_declaration_limit(source, schema, profile, 128)
}
fn compile_with_declaration_limit(
    source: &str,
    schema: &Schema,
    profile: Profile,
    declaration_limit: usize,
) -> Result<Grammar, Diagnostic> {
    compile_with_setting_declarations(
        source,
        schema,
        profile,
        declaration_limit,
        Vec::new(),
        Default::default(),
    )
}

/// Analyze one exact Xtext source with explicit computed-setting requirements.
/// This admits grammar structure only; a language host must install matching
/// providers and preserve these requirements during source/archive replay.
pub fn compile_with_settings(
    source: &intersect_model::definitions::bundle::DefinitionSource,
    schema: &Schema,
    profile: Profile,
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
) -> Result<Grammar, Diagnostic> {
    if profile.has_terminal_cardinality_decisions() {
        return Err(Diagnostic::at_source(
            &source.id,
            0,
            1,
            "NativeV63 requires language-owned authored-source validation",
        ));
    }
    validate_setting_contracts(source, schema, profile, bundle, contracts)?;
    let mut grammar = compile_with_setting_declarations(
        &source.text,
        schema,
        profile,
        128,
        contracts.to_vec(),
        Default::default(),
    )
    .map_err(|mut diagnostic| {
        diagnostic.source = source.id.uri.clone();
        if profile.has_contextual_first_rule() {
            diagnostic.with_source(&source.id)
        } else {
            diagnostic
        }
    })?;
    for span in grammar.declaration_sources.values_mut() {
        span.source = source.id.clone();
    }
    Ok(grammar)
}
pub(crate) fn validate_setting_contracts(
    source: &intersect_model::definitions::bundle::DefinitionSource,
    schema: &Schema,
    profile: Profile,
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
) -> Result<(), Diagnostic> {
    let error =
        |message: String| Diagnostic::new(&source.id.uri, 0, 1, message).with_context(&source.id);
    if !profile.has_computed_settings() {
        return Err(error(
            "Computed setting admission requires NativeV12".into(),
        ));
    }
    bundle.validate().map_err(error)?;
    if source.kind != intersect_model::definitions::bundle::SourceKind::Xtext
        || !bundle.sources.contains(source)
    {
        return Err(error(
            "Missing exact Xtext source for computed settings".into(),
        ));
    }
    let mut members = BTreeSet::new();
    for contract in contracts {
        contract.validate(schema, bundle).map_err(error)?;
        if !contract.sources.contains(&source.id) {
            return Err(error(
                "Computed setting contract omits the Xtext source".into(),
            ));
        }
        if !members.insert((&contract.class, &contract.feature)) {
            return Err(error(
                "Duplicate computed setting receiver/member contract".into(),
            ));
        }
    }
    Ok(())
}
fn compile_with_setting_declarations(
    source: &str,
    schema: &Schema,
    profile: Profile,
    declaration_limit: usize,
    contracts: Vec<intersect_model::definitions::setting::ComputedSettingContract>,
    package_imports: std::rc::Rc<PackageImports<'_>>,
) -> Result<Grammar, Diagnostic> {
    let (mut grammar, _, spans, at) = parse_declarations(
        source,
        schema,
        profile,
        false,
        None,
        declaration_limit,
        package_imports,
    )?;
    grammar.computed_settings = contracts;
    if profile.has_rule_conversions() {
        let source = intersect_model::definitions::bundle::DefinitionSource::new(
            "xtext",
            intersect_model::definitions::bundle::SourceKind::Xtext,
            source,
        );
        grammar.declaration_sources = spans
            .into_iter()
            .map(|(name, span)| {
                (
                    name,
                    intersect_model::definitions::bundle::SourceSpan {
                        source: source.id.clone(),
                        start: span.start,
                        end: span.end,
                    },
                )
            })
            .collect();
    }
    if profile.has_contextual_first_rule() {
        let authored = intersect_model::definitions::bundle::DefinitionSource::new(
            "xtext",
            intersect_model::definitions::bundle::SourceKind::Xtext,
            source,
        );
        let syntax = syntax::parse(
            &authored,
            SourceLimits {
                source_bytes: source.len(),
                declarations: declaration_limit,
            },
        )?;
        first_rule::validate_compiled(&syntax, &grammar, schema, Default::default())?;
    }
    validate_grammar(&mut grammar, schema).map_err(|e| Diagnostic::new("xtext", at, at + 1, e))?;
    Ok(grammar)
}

type ParsedDeclarations = (
    Grammar,
    Option<String>,
    BTreeMap<String, std::ops::Range<usize>>,
    usize,
);
fn parse_declarations(
    source: &str,
    schema: &Schema,
    profile: Profile,
    composition: bool,
    inherited_hidden: Option<Vec<String>>,
    declaration_limit: usize,
    package_imports: std::rc::Rc<PackageImports<'_>>,
) -> Result<ParsedDeclarations, Diagnostic> {
    let tokens = definition_tokens(source, profile, declaration_limit)?;
    let mut p = Parser {
        capture_origins: profile.has_durable_documents(),
        unordered: profile
            .has_unordered_groups()
            .then(|| Box::new(unordered::ParserState::default())),
        explicit_local_scope: matches!(
            profile,
            Profile::NativeV45
                | Profile::NativeV46
                | Profile::NativeV47
                | Profile::NativeV48
                | Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ),
        package_imports,
        allow_first_predicates: profile.has_parser_prediction(),
        allow_constant_guards: profile.has_constant_guards(),
        tokens,
        pos: 0,
        depth: 0,
        aliases: BTreeMap::new(),
        metamodel_imports: matches!(
            profile,
            Profile::NativeV20
                | Profile::NativeV21
                | Profile::NativeV22
                | Profile::NativeV23
                | Profile::NativeV24
                | Profile::NativeV25
                | Profile::NativeV26
                | Profile::NativeV27
                | Profile::NativeV28
                | Profile::NativeV29
                | Profile::NativeV30
                | Profile::NativeV31
                | Profile::NativeV32
                | Profile::NativeV33
                | Profile::NativeV34
                | Profile::NativeV35
                | Profile::NativeV36
                | Profile::NativeV37
                | Profile::NativeV38
                | Profile::NativeV39
                | Profile::NativeV40
                | Profile::NativeV41
                | Profile::NativeV42
                | Profile::NativeV43
                | Profile::NativeV44
                | Profile::NativeV45
                | Profile::NativeV46
                | Profile::NativeV47
                | Profile::NativeV48
                | Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ),
        hidden: None,
    };
    let result = (|| {
        let SourceHeader { name, parent, .. } = p.source_header(schema, composition)?;
        let grammar_name = name.clone();
        if let Some(unordered) = &mut p.unordered {
            unordered.namespace = unordered::namespace(&name);
        }
        if p.hidden.is_none() {
            p.hidden = inherited_hidden;
        }
        // Xtext flattening gives a parentless grammar an empty policy. Exact
        // composition has already resolved any parent and its effective policy.
        // The single-source common.Terminals shortcut has no retained parent:
        // do not invent its hidden tokens or silently treat it as parentless.
        if matches!(
            profile,
            Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ) && p.hidden.is_none()
            && (composition || parent.is_none())
        {
            p.hidden = Some(Vec::new());
        }
        if profile.has_lexical_prediction() && p.hidden.is_none() {
            return Err("Predictive lexical profile requires an explicit hidden policy".into());
        }
        if p.hidden.is_some() && profile.is_workbench() {
            return Err("Explicit lexical policy requires the native Xtext profile".into());
        }
        let mut nominal_return_bytes = p
            .package_imports
            .authored_source_limits
            .unwrap_or_default()
            .source_bytes;
        let mut nominal_return_records = declaration_limit;
        let mut rules = BTreeMap::new();
        let mut terminals = BTreeMap::new();
        let mut entry = String::new();
        let mut spans = BTreeMap::new();
        let mut final_rules = BTreeSet::new();
        while p.pos < p.tokens.len() {
            let declaration_start = p.tokens[p.pos].start;
            let mut override_declared = false;
            let mut final_declared = false;
            while p.eat("@") {
                if !profile.has_parser_overrides() {
                    return Err("Rule annotations require the contextual native profile".into());
                }
                let annotation = p.name()?;
                match annotation.as_str() {
                    "Override" => {
                        if !composition {
                            return Err(
                                "@Override requires an exact inherited parser declaration".into()
                            );
                        }
                        override_declared = true;
                    }
                    "Final"
                        if matches!(
                            profile,
                            Profile::NativeV19
                                | Profile::NativeV20
                                | Profile::NativeV21
                                | Profile::NativeV22
                                | Profile::NativeV23
                                | Profile::NativeV24
                                | Profile::NativeV25
                                | Profile::NativeV26
                                | Profile::NativeV27
                                | Profile::NativeV28
                                | Profile::NativeV29
                                | Profile::NativeV30
                                | Profile::NativeV31
                                | Profile::NativeV32
                                | Profile::NativeV33
                                | Profile::NativeV34
                                | Profile::NativeV35
                                | Profile::NativeV36
                                | Profile::NativeV37
                                | Profile::NativeV38
                                | Profile::NativeV39
                                | Profile::NativeV40
                                | Profile::NativeV41
                                | Profile::NativeV42
                                | Profile::NativeV43
                                | Profile::NativeV44
                                | Profile::NativeV45
                                | Profile::NativeV46
                                | Profile::NativeV47
                                | Profile::NativeV48
                                | Profile::NativeV49
                                | Profile::NativeV50
                                | Profile::NativeV51
                                | Profile::NativeV52
                                | Profile::NativeV53
                                | Profile::NativeV54
                                | Profile::NativeV55
                                | Profile::NativeV56
                                | Profile::NativeV57
                                | Profile::NativeV58
                                | Profile::NativeV59
                                | Profile::NativeV60
                                | Profile::NativeV61
                                | Profile::NativeV62
                                | Profile::NativeV63
                                | Profile::NativeV64
                                | Profile::NativeV65
                                | Profile::NativeV66
                                | Profile::NativeV67
                                | Profile::NativeV68
                                | Profile::NativeV69
                                | Profile::NativeV70
                                | Profile::NativeV71
                                | Profile::NativeV72
                                | Profile::NativeV73
                                | Profile::NativeV74
                                | Profile::NativeV75
                                | Profile::NativeV76
                                | Profile::NativeV77
                                | Profile::NativeV78
                                | Profile::NativeV79
                                | Profile::NativeV80
                                | Profile::NativeV81
                                | Profile::NativeV82
                        ) =>
                    {
                        final_declared = true
                    }
                    _ => return Err(format!("Unsupported rule annotation {annotation}")),
                }
                if !matches!(
                    profile,
                    Profile::NativeV19
                        | Profile::NativeV20
                        | Profile::NativeV21
                        | Profile::NativeV22
                        | Profile::NativeV23
                        | Profile::NativeV24
                        | Profile::NativeV25
                        | Profile::NativeV26
                        | Profile::NativeV27
                        | Profile::NativeV28
                        | Profile::NativeV29
                        | Profile::NativeV30
                        | Profile::NativeV31
                        | Profile::NativeV32
                        | Profile::NativeV33
                        | Profile::NativeV34
                        | Profile::NativeV35
                        | Profile::NativeV36
                        | Profile::NativeV37
                        | Profile::NativeV38
                        | Profile::NativeV39
                        | Profile::NativeV40
                        | Profile::NativeV41
                        | Profile::NativeV42
                        | Profile::NativeV43
                        | Profile::NativeV44
                        | Profile::NativeV45
                        | Profile::NativeV46
                        | Profile::NativeV47
                        | Profile::NativeV48
                        | Profile::NativeV49
                        | Profile::NativeV50
                        | Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                ) {
                    break;
                }
            }
            if p.eat("terminal") {
                if override_declared && !profile.has_terminal_overrides() {
                    return Err("Terminal overrides are unsupported".into());
                }
                if profile.is_workbench() {
                    return Err("Terminal declarations require the native Xtext profile".into());
                }
                let start = p.tokens[p.pos - 1].start;
                let fragment = p.eat("fragment");
                if fragment && p.hidden.is_none() {
                    return Err("Terminal fragments require explicit lexical policy".into());
                }
                let terminal = p.name()?;
                if final_declared {
                    final_rules.insert(terminal.clone());
                }
                if p.hidden.is_none()
                    && [
                        "ID",
                        "STRING",
                        "INT",
                        "WS",
                        "ML_COMMENT",
                        "SL_COMMENT",
                        "ANY_OTHER",
                    ]
                    .contains(&terminal.as_str())
                {
                    return Err(
                        "Overriding inherited terminals requires unsupported lexical prediction"
                            .into(),
                    );
                }
                let mut datatype = Datatype::EString;
                let mut nominal_return = None;
                let mut enumeration = None;
                if p.eat("returns") {
                    let name = p.type_name()?;
                    let (namespace, local) =
                        imported_type(&name, &p.aliases, schema, profile, &p.package_imports)?;
                    if namespace == intersect_model::definitions::ecore::ECORE
                        && local == "EInt"
                        && profile.has_lexical_prediction()
                    {
                        datatype = Datatype::EInt;
                    } else if profile.has_builtin_scalar_returns()
                        && namespace == intersect_model::definitions::ecore::ECORE
                        && let Some(codec) =
                            intersect_model::definitions::scalars::Builtin::named(local)
                    {
                        datatype = Datatype::Builtin(codec);
                    } else if matches!(
                        profile,
                        Profile::NativeV47
                            | Profile::NativeV48
                            | Profile::NativeV49
                            | Profile::NativeV50
                            | Profile::NativeV51
                            | Profile::NativeV52
                            | Profile::NativeV53
                            | Profile::NativeV54
                            | Profile::NativeV55
                            | Profile::NativeV56
                            | Profile::NativeV57
                            | Profile::NativeV58
                            | Profile::NativeV59
                            | Profile::NativeV60
                            | Profile::NativeV61
                            | Profile::NativeV62
                            | Profile::NativeV63
                            | Profile::NativeV64
                            | Profile::NativeV65
                            | Profile::NativeV66
                            | Profile::NativeV67
                            | Profile::NativeV68
                            | Profile::NativeV69
                            | Profile::NativeV70
                            | Profile::NativeV71
                            | Profile::NativeV72
                            | Profile::NativeV73
                            | Profile::NativeV74
                            | Profile::NativeV75
                            | Profile::NativeV76
                            | Profile::NativeV77
                            | Profile::NativeV78
                            | Profile::NativeV79
                            | Profile::NativeV80
                            | Profile::NativeV81
                            | Profile::NativeV82
                    ) && enum_rules::key(schema, namespace, local).is_ok()
                    {
                        datatype = Datatype::Enum;
                        enumeration = Some(enum_rules::key(schema, namespace, local)?.to_owned());
                    } else if profile.has_durable_documents()
                        && let Some((_, declaration)) = nominal_return_key(
                            schema,
                            &p.package_imports,
                            namespace,
                            local,
                            &mut nominal_return_bytes,
                            &mut nominal_return_records,
                        )?
                    {
                        datatype = Datatype::Nominal;
                        nominal_return = Some(declaration);
                    } else if namespace != intersect_model::definitions::ecore::ECORE
                        || local != "EString"
                    {
                        return Err(
                            "Custom terminals currently require EString return types".into()
                        );
                    }
                }
                p.require(":")?;
                let mut program = terminals::parse(
                    &mut p,
                    format!("{name}::{terminal}"),
                    start,
                    fragment,
                    profile,
                )?;
                spans.insert(terminal.clone(), declaration_start..program.end);
                program.datatype = datatype;
                program.nominal_return = nominal_return;
                program.enumeration = enumeration;
                program.override_declared = override_declared;
                if rules.len() + terminals.len() >= declaration_limit {
                    return Err("Grammar rule budget exceeded".into());
                }
                if rules.contains_key(&terminal)
                    || terminals.insert(terminal.clone(), program).is_some()
                {
                    return Err(format!("Duplicate rule {terminal}"));
                }
                continue;
            }
            if p.eat("enum") {
                if !profile.has_enum_rules() {
                    return Err("Unsupported Xtext declaration enum".into());
                }
                if override_declared && !profile.has_typed_overrides() {
                    return Err("Enum rule annotations are unsupported".into());
                }
                let name = p.name()?;
                if ["ID", "STRING", "INT"].contains(&name.as_str()) {
                    return Err("Enum rules shadowing common terminals are unsupported".into());
                }
                if final_declared {
                    final_rules.insert(name.clone());
                }
                let inferred_name = p.package_imports.inferred_headers.then_some(name.as_str());
                let bound = p
                    .package_imports
                    .inferred_rules
                    .get(&grammar_name)
                    .and_then(|rules| rules.get(&name))
                    .cloned();
                let mut rule =
                    enum_rules::parse(&mut p, schema, profile, inferred_name, bound.as_ref())?;
                rule.override_declared = override_declared;
                if rules.len() + terminals.len() >= declaration_limit {
                    return Err("Grammar rule budget exceeded".into());
                }
                if terminals.contains_key(&name) || rules.contains_key(&name) {
                    return Err(format!("Duplicate rule {name}"));
                }
                spans.insert(name.clone(), declaration_start..p.tokens[p.pos - 1].end);
                rules.insert(name, rule);
                continue;
            }
            let fragment = p.eat("fragment");
            if fragment && profile.is_workbench() {
                return Err("Parser fragments require the native Xtext profile".into());
            }
            let rule = p.name()?;
            if final_declared {
                final_rules.insert(rule.clone());
            }
            if ["terminal", "enum", "fragment", "generate", "import"].contains(&rule.as_str()) {
                return Err(format!("Unsupported Xtext declaration {rule}"));
            }
            let wildcard = fragment && p.eat("*");
            let inferred_source = p.package_imports.inferred_rules.contains_key(&grammar_name);
            if wildcard && !inferred_source {
                return Err("Wildcard fragments require source-bound native inference".into());
            }
            let explicit_return = !wildcard && p.eat("returns");
            let bound = p
                .package_imports
                .inferred_rules
                .get(&grammar_name)
                .and_then(|rules| rules.get(&rule))
                .cloned();
            let inferred_return =
                !explicit_return && bound.is_none() && !fragment && profile.has_return_inference();
            // Relaxing the header requirement for explicit local TypeRefs must
            // not invent a default package for the separate inference path.
            if inferred_return
                && matches!(
                    profile,
                    Profile::NativeV45
                        | Profile::NativeV46
                        | Profile::NativeV47
                        | Profile::NativeV48
                        | Profile::NativeV49
                        | Profile::NativeV50
                        | Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                )
                && !p
                    .aliases
                    .values()
                    .flatten()
                    .any(|namespace| namespace == &schema.uri)
            {
                return Err("Inferred returns require an explicit selected EPackage import".into());
            }
            let class = if wildcard {
                String::new()
            } else if explicit_return {
                p.type_name()?
            } else if fragment && bound.is_none() {
                return Err("Parser fragments currently require explicit returns".into());
            } else {
                rule.clone()
            };
            let (namespace, name) = if wildcard {
                ("", "")
            } else if !explicit_return && bound.is_some() {
                let (namespace, name) = bound.as_ref().ok_or("Missing checked inferred return")?;
                (namespace.as_str(), name.as_str())
            } else if inferred_return {
                (schema.uri.as_str(), class.as_str())
            } else {
                imported_type(&class, &p.aliases, schema, profile, &p.package_imports)?
            };
            let mut nominal_return = None;
            let (class, datatype) = if wildcard {
                (String::new(), None)
            } else if inferred_return {
                (rule.clone(), None)
            } else if namespace == intersect_model::definitions::ecore::ECORE
                && name == "EBoolean"
                && profile.has_rule_conversions()
            {
                if fragment && !inferred_source {
                    return Err("Datatype parser fragments are unsupported".into());
                }
                (String::new(), Some(Datatype::EBoolean))
            } else if namespace == intersect_model::definitions::ecore::ECORE
                && name == "EDouble"
                && profile.has_double_rules()
            {
                if fragment && !inferred_source {
                    return Err("Datatype parser fragments are unsupported".into());
                }
                (String::new(), Some(Datatype::EDouble))
            } else if namespace == intersect_model::definitions::ecore::ECORE && name == "EString" {
                if profile.is_workbench() {
                    return Err("Datatype parser rules require the native Xtext profile".into());
                }
                if fragment && !inferred_source {
                    return Err("Datatype parser fragments are unsupported".into());
                }
                (String::new(), Some(Datatype::EString))
            } else if profile.has_builtin_scalar_returns()
                && namespace == intersect_model::definitions::ecore::ECORE
                && let Some(codec) = intersect_model::definitions::scalars::Builtin::named(name)
            {
                if fragment && !inferred_source {
                    return Err("Datatype parser fragments are unsupported".into());
                }
                (String::new(), Some(Datatype::Builtin(codec)))
            } else if matches!(
                profile,
                Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) && enum_rules::key(schema, namespace, name).is_ok()
            {
                if fragment && !inferred_source {
                    return Err("Datatype parser fragments are unsupported".into());
                }
                (
                    enum_rules::key(schema, namespace, name)?.to_owned(),
                    Some(Datatype::Enum),
                )
            } else if profile.has_durable_documents()
                && let Some((key, declaration)) = nominal_return_key(
                    schema,
                    &p.package_imports,
                    namespace,
                    name,
                    &mut nominal_return_bytes,
                    &mut nominal_return_records,
                )?
            {
                if fragment {
                    return Err("Nominal datatype parser fragments are unsupported".into());
                }
                nominal_return = Some(declaration);
                (key.to_owned(), Some(Datatype::Nominal))
            } else {
                (
                    imported_class_key(schema, namespace, name)?.to_owned(),
                    None,
                )
            };
            if datatype.is_some() && ["ID", "STRING", "INT"].contains(&rule.as_str()) {
                return Err("Datatype rules shadowing common terminals are unsupported".into());
            }
            let hidden = if profile.has_scoped_hidden_tokens() && p.eat("hidden") {
                p.require("(")?;
                let mut hidden = Vec::new();
                if !p.peek(")") {
                    loop {
                        if hidden.len() >= 128 {
                            return Err("Hidden terminal budget exceeded".into());
                        }
                        let name = p.name()?;
                        if hidden.contains(&name) {
                            return Err(format!("Duplicate hidden terminal {name}"));
                        }
                        hidden.push(name);
                        if !p.eat(",") {
                            break;
                        }
                    }
                }
                p.require(")")?;
                Some(hidden)
            } else {
                None
            };
            p.require(":")?;
            let mut body = match p.expression(profile.has_full_syntactic_predicates())? {
                Some(body) => body,
                // A whole rule with no retained body is non-executable. Keep the
                // existing failure primitive for false-only specialization seeds;
                // do not turn it into an epsilon parser. Upstream whole-rule
                // generation obstructions remain separately unqualified evidence.
                None => Expr {
                    kind: Kind::Alternative(vec![]),
                    first_set_predicated: false,
                    original_range: None,
                    prediction_path: None,
                    cardinality: '1',
                },
            };
            bind_reference_types(&mut body, schema, &p.aliases, profile, &p.package_imports)?;
            if let Some(unordered) = &mut p.unordered {
                for pending in &mut unordered.pending {
                    for member in &mut pending.members {
                        bind_reference_types(
                            member,
                            schema,
                            &p.aliases,
                            profile,
                            &p.package_imports,
                        )?;
                    }
                }
            }
            p.require(";")?;
            spans.insert(rule.clone(), declaration_start..p.tokens[p.pos - 1].end);
            if entry.is_empty() && !fragment {
                entry = rule.clone();
            }
            if terminals.contains_key(&rule) {
                return Err(format!("Duplicate rule {rule}"));
            }
            if rules.len() + terminals.len() >= declaration_limit {
                return Err("Grammar rule budget exceeded".into());
            }
            if rules
                .insert(
                    rule.clone(),
                    Rule {
                        hidden,
                        enum_literals: BTreeMap::new(),
                        inferred_return,
                        override_declared,
                        datatype,
                        nominal_return,
                        fragment,
                        wildcard,
                        class,
                        body,
                    },
                )
                .is_some()
            {
                return Err(format!("Duplicate rule {rule}"));
            }
            let parent = rules
                .get(&rule)
                .ok_or("Missing unordered parent declaration")?;
            let lowered = unordered::lower(
                p.unordered
                    .as_mut()
                    .map(|state| std::mem::take(&mut state.pending))
                    .unwrap_or_default(),
                parent,
                declaration_limit.saturating_sub(rules.len() + terminals.len()),
            )?;
            for (name, generated) in lowered {
                spans.insert(name.clone(), declaration_start..p.tokens[p.pos - 1].end);
                if rules.insert(name, generated).is_some() {
                    return Err("Generated unordered rule collision".into());
                }
            }
        }
        if entry.is_empty()
            && (!composition
                || !matches!(
                    profile,
                    Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                )
                || (rules.is_empty() && terminals.is_empty()))
            && !(composition
                && matches!(
                    profile,
                    Profile::NativeV50
                        | Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                )
                && rules.is_empty()
                && !terminals.is_empty())
            && !(composition
                && parent.is_some()
                && matches!(
                    profile,
                    Profile::NativeV43
                        | Profile::NativeV44
                        | Profile::NativeV45
                        | Profile::NativeV46
                        | Profile::NativeV47
                        | Profile::NativeV48
                        | Profile::NativeV49
                        | Profile::NativeV50
                        | Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                ))
        {
            return Err("Grammar requires a parser rule".into());
        }
        let grammar = Grammar {
            call_depth: None,
            analysis_work_budget: p.package_imports.work_budget,
            parameterized_rules: BTreeSet::new(),
            final_rules,
            computed_settings: Vec::new(),
            profile,
            name,
            entry,
            rules,
            terminals,
            hidden: p.hidden.clone(),
            lexical_prediction: None,
            parser_prediction: None,
            declaration_sources: BTreeMap::new(),
            terminal_order: Vec::new(),
            shadowed_declarations: BTreeMap::new(),
        };
        let at = p.tokens.get(p.pos.saturating_sub(1)).map_or(0, |t| t.start);
        Ok((grammar, parent, spans, at))
    })();
    result.map_err(|e| {
        let at = p.tokens.get(p.pos.saturating_sub(1)).map_or(0, |t| t.start);
        Diagnostic::new("xtext", at, at + 1, e)
    })
}
/// Analyze a complete effective grammar after all source declarations are known.
/// Keeping this shared stage separate allows inherited calls to resolve only
/// after composition, without validating a parent as an independent language.
struct AnalysisError {
    declaration: Option<String>,
    // Producer ownership is independent of legacy diagnostic range selection.
    producer_declaration: Option<String>,
    message: String,
}
impl From<String> for AnalysisError {
    fn from(message: String) -> Self {
        Self {
            declaration: None,
            producer_declaration: None,
            message,
        }
    }
}
impl From<&str> for AnalysisError {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}
#[derive(Clone, Copy)]
struct AnalysisObservation<'a> {
    observation: &'a admission_observation::PredictionObservation,
    recursive_work: Option<&'a admission_observation::RecursiveAnalysisCounters>,
    // Stack-only optional sidecar; never retained by Grammar/ParserPrediction.
    adjacent_work: Option<&'a admission_observation::AdjacentWorkObservation>,
    entry: &'a intersect_model::definitions::bundle::SourceId,
    stage: admission_observation::RunStage,
}
fn observe_phase<T, E>(
    result: Result<T, E>,
    grammar: &Grammar,
    context: Option<AnalysisObservation<'_>>,
    phase: admission_observation::Phase,
    owner: Option<&str>,
    decision: Option<&str>,
) -> Result<T, E> {
    if result.is_err()
        && let Some(context) = context
    {
        // Resolve only a real effective declaration. Never infer an origin from
        // byte offsets, an error string or a parser-state number.
        let owner = owner.and_then(|name| {
            grammar
                .declaration_sources
                .get(name)
                .map(|span| (name, span))
        });
        context
            .observation
            .returned_error(context.entry, context.stage, phase, owner, decision);
    }
    result
}
fn validate_grammar(grammar: &mut Grammar, schema: &Schema) -> Result<(), String> {
    analyze_grammar(grammar, schema).map_err(|error| error.message)
}
fn analyze_terminal_parent(grammar: &mut Grammar) -> Result<(), AnalysisError> {
    analyze_terminal_parent_observed(grammar, None)
}
fn analyze_terminal_parent_observed(
    grammar: &mut Grammar,
    observation: Option<AnalysisObservation<'_>>,
) -> Result<(), AnalysisError> {
    let prepared = terminals::prepare(&mut grammar.terminals, grammar.profile);
    observe_phase(
        prepared,
        grammar,
        observation,
        admission_observation::Phase::TerminalPreparation,
        None,
        None,
    )?;
    observe_phase(
        terminals::check(grammar),
        grammar,
        observation,
        admission_observation::Phase::TerminalCheck,
        None,
        None,
    )?;
    analyze_lexical_prediction(grammar, observation)
}
fn analyze_lexical_prediction(
    grammar: &mut Grammar,
    observation: Option<AnalysisObservation<'_>>,
) -> Result<(), AnalysisError> {
    if grammar.profile.has_lexical_prediction() {
        let prediction = observe_phase(
            LexerPrediction::compile(grammar),
            grammar,
            observation,
            admission_observation::Phase::LexicalProgram,
            None,
            None,
        )?;
        if matches!(
            grammar.profile,
            Profile::NativeV48
                | Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ) && let Some(name) = observe_phase(
            prediction.unreachable_terminal(),
            grammar,
            observation,
            admission_observation::Phase::LexicalReachability,
            None,
            None,
        )? {
            return observe_phase(
                Err(AnalysisError {
                    declaration: Some(name.into()),
                    producer_declaration: None,
                    message: format!(
                        "Unreachable terminal {name}: lexical prediction always selects an earlier alternative"
                    ),
                }),
                grammar,
                observation,
                admission_observation::Phase::LexicalReachability,
                Some(name),
                None,
            );
        }
        grammar.lexical_prediction = Some(prediction);
    }
    Ok(())
}
fn analyze_grammar(grammar: &mut Grammar, schema: &Schema) -> Result<(), AnalysisError> {
    analyze_declarations(grammar, schema, true)
}
fn analyze_parent(grammar: &mut Grammar, schema: &Schema) -> Result<(), AnalysisError> {
    analyze_declarations(grammar, schema, false)
}
fn analyze_declarations(
    grammar: &mut Grammar,
    schema: &Schema,
    executable: bool,
) -> Result<(), AnalysisError> {
    analyze_declarations_observed(grammar, schema, executable, None)
}
fn analyze_declarations_observed(
    grammar: &mut Grammar,
    schema: &Schema,
    executable: bool,
    observation: Option<AnalysisObservation<'_>>,
) -> Result<(), AnalysisError> {
    analyze_declarations_prefix_inspected(grammar, schema, executable, observation, None)
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
fn analyze_declarations_prefix_inspected(
    grammar: &mut Grammar,
    schema: &Schema,
    executable: bool,
    observation: Option<AnalysisObservation<'_>>,
    prefix: Option<&SourcePrefixDiagnostic<'_>>,
) -> Result<(), AnalysisError> {
    analyze_declarations_prefix_inspected_frontier_selected(
        grammar,
        schema,
        executable,
        observation,
        prefix,
        RecursiveFrontierOrder::Historical,
    )
}
#[allow(clippy::too_many_arguments)] // Same worker plus explicit transient search order.
fn analyze_declarations_prefix_inspected_frontier_selected(
    grammar: &mut Grammar,
    schema: &Schema,
    executable: bool,
    observation: Option<AnalysisObservation<'_>>,
    prefix: Option<&SourcePrefixDiagnostic<'_>>,
    order: RecursiveFrontierOrder,
) -> Result<(), AnalysisError> {
    analyze_declarations_prefix_inspected_frontier_selected_pool(
        grammar,
        schema,
        executable,
        observation,
        prefix,
        order,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // One transient caller-owned diagnostic; same worker.
fn analyze_declarations_prefix_inspected_frontier_selected_pool(
    grammar: &mut Grammar,
    schema: &Schema,
    executable: bool,
    observation: Option<AnalysisObservation<'_>>,
    prefix: Option<&SourcePrefixDiagnostic<'_>>,
    order: RecursiveFrontierOrder,
    pool: Option<&admission_observation::pool_blocks::CompilerPoolDiagnostic>,
) -> Result<(), AnalysisError> {
    analyze_declarations_prediction_policy(
        grammar,
        schema,
        executable,
        observation,
        prefix,
        order,
        pool,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn analyze_declarations_prediction_policy(
    grammar: &mut Grammar,
    schema: &Schema,
    executable: bool,
    observation: Option<AnalysisObservation<'_>>,
    prefix: Option<&SourcePrefixDiagnostic<'_>>,
    order: RecursiveFrontierOrder,
    pool: Option<&admission_observation::pool_blocks::CompilerPoolDiagnostic>,
    mut runtime: Option<&mut RuntimeGuardedAdmission>,
) -> Result<(), AnalysisError> {
    let work_budget = runtime
        .as_ref()
        .map_or(grammar.analysis_work_budget, |a| a.work_budget());
    grammar.call_depth = runtime.as_ref().map(|a| a.call_depth());
    let inferred = datatype::infer(grammar, schema);
    observe_phase(
        inferred,
        grammar,
        observation,
        admission_observation::Phase::DatatypeInference,
        None,
        None,
    )?;
    let prepared = terminals::prepare(&mut grammar.terminals, grammar.profile);
    observe_phase(
        prepared,
        grammar,
        observation,
        admission_observation::Phase::TerminalPreparation,
        None,
        None,
    )?;
    observe_phase(
        terminals::check(grammar),
        grammar,
        observation,
        admission_observation::Phase::TerminalCheck,
        None,
        None,
    )?;
    let entry_checked = (|| -> Result<(), AnalysisError> {
        if executable
            && grammar
                .rules
                .get(&grammar.entry)
                .ok_or("Missing parser entry")?
                .datatype
                .is_some()
        {
            return Err("Scalar parser entry execution is unsupported".into());
        }
        Ok(())
    })();
    observe_phase(
        entry_checked,
        grammar,
        observation,
        admission_observation::Phase::EntryCategory,
        Some(&grammar.entry),
        None,
    )?;
    if grammar.profile.has_unordered_groups() {
        observe_phase(
            unordered::check(grammar, work_budget.units()),
            grammar,
            observation,
            admission_observation::Phase::FragmentCycles,
            None,
            None,
        )?;
    }
    observe_phase(
        follow::check_fragment_cycles(grammar, work_budget),
        grammar,
        observation,
        admission_observation::Phase::FragmentCycles,
        None,
        None,
    )?;
    let mut deferred_predictions = BTreeSet::new();
    let mut required_calls = None;
    let mut repeat_certificates = None;
    for (name, rule) in &grammar.rules {
        let checked = (|| -> Result<(), String> {
            if grammar.profile.is_workbench() {
                check_legacy_profile(&rule.body)?;
            }
            actions::check(rule, schema, grammar)?;
            if rule.datatype.is_some() {
                datatype::check(&rule.body, schema, grammar)?;
                if first(&rule.body, grammar, 0)?.1
                    && !(grammar.profile.has_unordered_groups()
                        && rule.fragment
                        && unordered::generated(name))
                {
                    return Err("Nullable datatype parser rules are unsupported".into());
                }
            } else if rule.wildcard {
                // No declared classifier exists. Caller contexts below validate
                // every reachable body through the same assignment admission.
            } else if rule.uses_current_flow(grammar) {
                // Current-object flow and assignments were checked together above.
            } else if rule.is_dispatch(grammar) {
                if grammar.profile.is_workbench() {
                    return Err(
                        "Unassigned parser dispatch requires the native Xtext profile".into(),
                    );
                }
                check_dispatch(&rule.body, &rule.class, schema, grammar)?;
            } else {
                check_expr(
                    &rule.body,
                    rule.construction_class(),
                    schema,
                    grammar,
                    false,
                )?;
            }
            let mut seen = BTreeSet::new();
            seen.insert(name.clone());
            observe_phase(
                check_left(&rule.body, grammar, &mut seen, work_budget),
                grammar,
                observation,
                admission_observation::Phase::LeftRecursion,
                Some(name),
                None,
            )?;
            observe_phase(
                check_prediction(
                    &rule.body,
                    grammar,
                    &format!("rule/{name}"),
                    &mut deferred_predictions,
                    work_budget,
                ),
                grammar,
                observation,
                admission_observation::Phase::PredictionCheck,
                Some(name),
                None,
            )?;
            Ok(())
        })();
        observe_phase(
            checked,
            grammar,
            observation,
            admission_observation::Phase::RuleConstruction,
            Some(name),
            None,
        )
        .map_err(|message| AnalysisError {
            declaration: Some(name.clone()),
            producer_declaration: None,
            message,
        })?;
    }
    if grammar.rules.values().any(|rule| rule.fragment) {
        observe_phase(
            check_fragment_callers(grammar, schema, observation, work_budget),
            grammar,
            observation,
            admission_observation::Phase::FragmentCallers,
            None,
            None,
        )?;
    }
    if matches!(
        grammar.profile,
        Profile::NativeV34
            | Profile::NativeV35
            | Profile::NativeV36
            | Profile::NativeV37
            | Profile::NativeV38
            | Profile::NativeV39
            | Profile::NativeV40
            | Profile::NativeV41
            | Profile::NativeV42
            | Profile::NativeV43
            | Profile::NativeV44
            | Profile::NativeV45
            | Profile::NativeV46
            | Profile::NativeV47
            | Profile::NativeV48
            | Profile::NativeV49
            | Profile::NativeV50
            | Profile::NativeV51
            | Profile::NativeV52
            | Profile::NativeV53
            | Profile::NativeV54
            | Profile::NativeV55
            | Profile::NativeV56
            | Profile::NativeV57
            | Profile::NativeV58
            | Profile::NativeV59
            | Profile::NativeV60
            | Profile::NativeV61
            | Profile::NativeV62
            | Profile::NativeV63
            | Profile::NativeV64
            | Profile::NativeV65
            | Profile::NativeV66
            | Profile::NativeV67
            | Profile::NativeV68
            | Profile::NativeV69
            | Profile::NativeV70
            | Profile::NativeV71
            | Profile::NativeV72
            | Profile::NativeV73
            | Profile::NativeV74
            | Profile::NativeV75
            | Profile::NativeV76
            | Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) || grammar
        .rules
        .values()
        .any(|rule| rule.fragment || rule.datatype.is_some())
    {
        observe_phase(
            if matches!(
                grammar.profile,
                Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) {
                (if let Some(runtime) = runtime.as_deref_mut() {
                    follow::check_required_calls_certified_runtime(
                        grammar,
                        &mut deferred_predictions,
                        observation,
                        prefix,
                        Some(runtime),
                    )
                } else if let Some(prefix) = prefix {
                    follow::check_required_calls_certified_prefix_inspected(
                        grammar,
                        &mut deferred_predictions,
                        observation,
                        Some(prefix),
                    )
                } else {
                    follow::check_required_calls_certified(
                        grammar,
                        &mut deferred_predictions,
                        observation,
                    )
                })
                .map(|(calls, remaining, repeats)| {
                    required_calls = Some((calls, remaining));
                    repeat_certificates = Some(repeats);
                })
            } else if grammar.profile.has_required_object_call_certificates() {
                follow::check_required_calls(grammar, &mut deferred_predictions, observation)
                    .map(|pending| required_calls = Some(pending))
            } else if observation.is_some() {
                follow::check_observed(grammar, &mut deferred_predictions, observation)
            } else {
                follow::check(grammar, &mut deferred_predictions)
            },
            grammar,
            observation,
            admission_observation::Phase::Follow,
            None,
            None,
        )?;
    }
    analyze_lexical_prediction(grammar, observation)?;
    if grammar.profile.has_parser_prediction() {
        let mut predicates = BTreeSet::new();
        let capture_recovery = grammar.profile.has_durable_documents();
        for (name, rule) in &mut grammar.rules {
            bind_prediction_paths(
                &mut rule.body,
                &format!("rule/{name}"),
                &mut predicates,
                capture_recovery,
            );
        }
        let prediction = if let Some(admission) = runtime.as_deref_mut() {
            ParserPrediction::compile_runtime_guarded_linear_repeats(
                grammar,
                &predicates,
                observation,
                repeat_certificates.as_mut(),
                admission,
            )
        } else if order != RecursiveFrontierOrder::Historical || pool.is_some() {
            ParserPrediction::compile_with_recursive_frontier_order_pool(
                grammar,
                &predicates,
                grammar.profile.has_recursive_predicates(),
                observation,
                repeat_certificates.as_mut(),
                order,
                pool,
            )
        } else if let Some(certificates) = repeat_certificates.as_mut() {
            ParserPrediction::compile_certified_repeats(
                grammar,
                &predicates,
                grammar.profile.has_recursive_predicates(),
                observation,
                certificates,
            )
        } else if let Some(context) = observation {
            ParserPrediction::compile_observed(
                grammar,
                &predicates,
                grammar.profile.has_recursive_predicates(),
                context,
            )
        } else if grammar.profile.has_recursive_predicates() {
            ParserPrediction::compile_with_recursive_first_predicates(grammar, &predicates)
        } else {
            ParserPrediction::compile_with_first_predicates(grammar, &predicates)
        };
        let mut prediction = observe_phase(
            prediction,
            grammar,
            observation,
            admission_observation::Phase::ParserProgram,
            None,
            None,
        )?;
        // One selected finite pool covers this complete deferred-decision
        // phase, including admitted-record revalidation. It is separate from
        // the earlier parser/static pool and the required_calls pool below.
        // Source qualification and fallback never reset it per decision.
        let mut deferred_remaining = work_budget.units();
        for path in deferred_predictions {
            (if let Some(admission) = runtime
                .as_deref_mut()
                .filter(|admission| admission.contains(&path))
            {
                admission.validate_decision_with_remaining(
                    grammar,
                    &prediction,
                    &path,
                    &mut deferred_remaining,
                )
            } else if matches!(
                grammar.profile,
                Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) {
                if runtime.as_deref().is_some_and(|admission| {
                    admission.policy() == RuntimeGuardedPolicy::InputCheckedCompletionScopes
                }) {
                    runtime
                        .as_deref_mut()
                        .ok_or_else(|| "Missing actual input-checked companion".to_string())
                        .and_then(|admission| {
                            admission.verify_deferred_with_live_choices(
                                grammar,
                                &mut prediction,
                                &path,
                                &mut deferred_remaining,
                            )
                        })
                } else {
                    prediction.verify_nullable_deferred_decision_with_remaining(
                        &path,
                        grammar,
                        &mut deferred_remaining,
                    )
                }
            } else if matches!(
                grammar.profile,
                Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) {
                prediction.verify_disjoint_with_bounded_discovery_remaining(
                    &path,
                    &mut deferred_remaining,
                )
            } else if grammar.profile == Profile::NativeV55 {
                prediction.verify_disjoint_with_filtered_closures(&path)
            } else if grammar.profile == Profile::NativeV54 {
                prediction.verify_disjoint_with_certificates(&path)
            } else if grammar.profile == Profile::NativeV53 {
                prediction.verify_disjoint_with_projection(&path)
            } else if matches!(
                grammar.profile,
                Profile::NativeV42
                    | Profile::NativeV43
                    | Profile::NativeV44
                    | Profile::NativeV45
                    | Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) {
                prediction.verify_disjoint_with_relative_closures(&path)
            } else if grammar.profile == Profile::NativeV41 {
                prediction.verify_disjoint_with_cached_closures(&path)
            } else if grammar.profile == Profile::NativeV40 {
                prediction.verify_disjoint_with_guard_overapproximation(&path)
            } else {
                prediction.verify_disjoint_decision(&path)
            })
            .map_err(|message| {
                if let Some(context) = observation {
                    let owner = path.split('/').nth(1).and_then(|name| {
                        grammar
                            .declaration_sources
                            .get(name)
                            .map(|span| (name, span))
                    });
                    context.observation.returned_error(
                        context.entry,
                        context.stage,
                        admission_observation::Phase::DeferredProof,
                        owner,
                        Some(&path),
                    );
                    context.observation.parser_measurements(
                        admission_observation::Phase::DeferredProof,
                        work_budget.units().checked_sub(deferred_remaining),
                        None,
                    );
                }
                AnalysisError {
                    declaration: path.split('/').nth(1).map(str::to_owned),
                    producer_declaration: None,
                    message,
                }
            })?;
        }
        if let Some((required, mut remaining)) = required_calls {
            for call in required {
                (if matches!(
                    grammar.profile,
                    Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                ) {
                    if let Some(admission) = runtime.as_deref_mut().filter(|admission| {
                        admission.policy() == RuntimeGuardedPolicy::InputCheckedCompletionScopes
                    }) {
                        prediction.verify_required_object_call_with_source_companion(
                            grammar,
                            &call.owner,
                            &call.path,
                            &call.callee,
                            &mut remaining,
                            Some(admission),
                        )
                    } else {
                        prediction.verify_required_object_call_with_nullable_full_source(
                            grammar,
                            &call.owner,
                            &call.path,
                            &call.callee,
                            &mut remaining,
                        )
                    }
                } else {
                    prediction.verify_required_object_call(&call.path, &call.callee, &mut remaining)
                })
                .map_err(|message| {
                    if let Some(context) = observation {
                        let owner = grammar
                            .declaration_sources
                            .get(&call.owner)
                            .map(|span| (call.owner.as_str(), span));
                        context.observation.returned_error(
                            context.entry,
                            context.stage,
                            admission_observation::Phase::DeferredProof,
                            owner,
                            Some(&call.path),
                        );
                    }
                    AnalysisError {
                        declaration: Some(call.owner),
                        producer_declaration: None,
                        message,
                    }
                })?;
            }
        }
        grammar.parser_prediction = Some(prediction);
    }
    Ok(())
}

fn check_legacy_profile(expr: &Expr) -> Result<(), String> {
    match &expr.kind {
        Kind::SyntacticPredicate(_) => Err("Full syntactic predicates require NativeV68".into()),
        Kind::Capture { .. } => Err("Assigned-current actions require NativeV5".into()),
        Kind::Action(_) => Err("Simple actions require the native Xtext profile".into()),
        Kind::BooleanAssign { .. } => {
            Err("Boolean assignment requires the native Xtext profile".into())
        }
        Kind::ScalarReference { .. } => {
            Err("Datatype reference spelling requires the native Xtext profile".into())
        }
        Kind::Assign { value, .. } => check_legacy_profile(value),
        Kind::Sequence(items) | Kind::Alternative(items) => {
            for item in items {
                check_legacy_profile(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

// A fragment's declared return type is a contract, but inherited/generic caller
// features may carry stricter constraints. Reuse assignment admission per actual
// caller class, caching pairs instead of exponentially expanding fragment bodies.
enum FragmentCallMode<'g> {
    Legacy,
    Summaries(follow::RuleCallSummaries<'g>),
}
struct FragmentCallContext<'g, 'o> {
    observation: Option<AnalysisObservation<'o>>,
    initial_work: usize,
    mode: FragmentCallMode<'g>,
}
fn check_fragment_callers(
    grammar: &Grammar,
    schema: &Schema,
    observation: Option<AnalysisObservation<'_>>,
    work_budget: GrammarAnalysisWorkBudget,
) -> Result<(), String> {
    let mut checked = BTreeSet::new();
    let mut budget = work_budget.units();
    let mut context = FragmentCallContext {
        observation,
        initial_work: work_budget.units(),
        mode: if grammar.profile.has_fragment_call_summaries() {
            FragmentCallMode::Summaries(follow::RuleCallSummaries::new(grammar))
        } else {
            FragmentCallMode::Legacy
        },
    };
    for (name, rule) in &grammar.rules {
        if rule.uses_current_flow(grammar) || rule.wildcard || rule.datatype.is_some() {
            continue;
        }
        check_fragment_context_observed(
            &rule.body,
            (Some(name), rule.construction_class()),
            grammar,
            schema,
            &mut checked,
            &mut budget,
            &mut context,
        )?;
    }
    Ok(())
}
fn check_fragment_context(
    body: &Expr,
    class: &str,
    grammar: &Grammar,
    schema: &Schema,
    checked: &mut BTreeSet<(String, String)>,
    budget: &mut usize,
) -> Result<(), String> {
    // This original actions.rs facade never opts into the new global analysis.
    let mut context = FragmentCallContext {
        observation: None,
        initial_work: *budget,
        mode: FragmentCallMode::Legacy,
    };
    check_fragment_context_observed(
        body,
        (None, class),
        grammar,
        schema,
        checked,
        budget,
        &mut context,
    )
}
fn check_fragment_context_observed(
    body: &Expr,
    (owner, class): (Option<&str>, &str),
    grammar: &Grammar,
    schema: &Schema,
    checked: &mut BTreeSet<(String, String)>,
    budget: &mut usize,
    context: &mut FragmentCallContext<'_, '_>,
) -> Result<(), String> {
    let mut pending = BTreeSet::new();
    check_fragment_calls(body, (owner, true), grammar, &mut pending, budget, context)?;
    while let Some(name) = pending.pop_first() {
        let Some((owner, fragment)) = grammar
            .rules
            .get_key_value(&name)
            .filter(|(_, target)| target.fragment)
        else {
            continue;
        };
        // A wildcard body is caller-generic, including any nested typed fragment
        // calls. Actual feature existence and value types are admitted by the
        // shared checked model writes during execution, without inventing caller
        // supertypes or changing sealed inherited schemas.
        if fragment.wildcard {
            continue;
        }
        if !checked.insert((class.to_owned(), name)) {
            continue;
        }
        check_expr(&fragment.body, class, schema, grammar, false)?;
        check_fragment_calls(
            &fragment.body,
            (Some(owner), false),
            grammar,
            &mut pending,
            budget,
            context,
        )?;
    }
    Ok(())
}
fn check_fragment_calls(
    body: &Expr,
    (owner, root): (Option<&str>, bool),
    grammar: &Grammar,
    pending: &mut BTreeSet<String>,
    budget: &mut usize,
    context: &mut FragmentCallContext<'_, '_>,
) -> Result<(), String> {
    let initial_work = context.initial_work;
    let observation = context.observation;
    if let FragmentCallMode::Summaries(cache) = &mut context.mode {
        let mut failure = None;
        let sink = if observation.is_some() {
            Some(&mut failure)
        } else {
            None
        };
        let result = cache.extend(owner, pending, budget, sink);
        if let Some(failure) = failure {
            use follow::{CallWalkLimit, SummaryFailure};
            let (remaining, depth) = match failure {
                SummaryFailure::Body(failure) => {
                    (failure.remaining_work, Some(failure.syntax_depth))
                }
                SummaryFailure::Lookup { remaining_work }
                | SummaryFailure::Edge { remaining_work }
                | SummaryFailure::Retention { remaining_work } => (remaining_work, None),
            };
            let decision = match (root, failure) {
                (
                    true,
                    SummaryFailure::Body(follow::CallWalkFailure {
                        reason: CallWalkLimit::Work,
                        ..
                    }),
                ) => "root-body-calls/work-limit",
                (true, SummaryFailure::Body(_)) => "root-body-calls/syntax-depth-limit",
                (
                    false,
                    SummaryFailure::Body(follow::CallWalkFailure {
                        reason: CallWalkLimit::Work,
                        ..
                    }),
                ) => "fragment-body-calls/work-limit",
                (false, SummaryFailure::Body(_)) => "fragment-body-calls/syntax-depth-limit",
                (true, SummaryFailure::Lookup { .. }) => "root-body-calls/cache-lookup-work-limit",
                (false, SummaryFailure::Lookup { .. }) => {
                    "fragment-body-calls/cache-lookup-work-limit"
                }
                (true, SummaryFailure::Edge { .. }) => "root-body-calls/cache-edge-work-limit",
                (false, SummaryFailure::Edge { .. }) => "fragment-body-calls/cache-edge-work-limit",
                (true, SummaryFailure::Retention { .. }) => {
                    "root-body-calls/cache-retention-work-limit"
                }
                (false, SummaryFailure::Retention { .. }) => {
                    "fragment-body-calls/cache-retention-work-limit"
                }
            };
            let result = observe_phase(
                result,
                grammar,
                observation,
                admission_observation::Phase::FragmentCallScan,
                owner,
                Some(decision),
            );
            if let Some(observation) = observation {
                observation
                    .observation
                    .fragment_call_measurements(initial_work - remaining, depth);
            }
            return result;
        }
        return result;
    }
    let Some(context) = observation else {
        return follow::calls(body, pending, budget);
    };
    let mut failure = None;
    let result = follow::calls_with_failure(body, pending, budget, Some(&mut failure));
    if let Some(failure) = failure {
        use follow::CallWalkLimit::{SyntaxDepth, Work};
        let decision = match (root, failure.reason) {
            (true, Work) => "root-body-calls/work-limit",
            (true, SyntaxDepth) => "root-body-calls/syntax-depth-limit",
            (false, Work) => "fragment-body-calls/work-limit",
            (false, SyntaxDepth) => "fragment-body-calls/syntax-depth-limit",
        };
        let result = observe_phase(
            result,
            grammar,
            observation,
            admission_observation::Phase::FragmentCallScan,
            owner,
            Some(decision),
        );
        context.observation.fragment_call_measurements(
            initial_work - failure.remaining_work,
            Some(failure.syntax_depth),
        );
        return result;
    }
    result
}
fn check_dispatch(
    expr: &Expr,
    class: &str,
    schema: &Schema,
    grammar: &Grammar,
) -> Result<(), String> {
    match &expr.kind {
        Kind::SyntacticPredicate(value) => check_dispatch(value, class, schema, grammar)?,
        Kind::Call(name) => {
            if ["ID", "STRING", "INT"].contains(&name.as_str()) {
                return Err("Unassigned terminal calls are unsupported".into());
            }
            let target = grammar
                .rules
                .get(name)
                .ok_or_else(|| format!("Unknown rule {name}"))?;
            if !schema.conforms(&target.class, class) {
                return Err(format!(
                    "Dispatch return type mismatch: {} does not conform to {class}",
                    target.class
                ));
            }
            if first(&target.body, grammar, 0)?.1 {
                return Err(format!("Nullable dispatch target {name} is unsupported"));
            }
        }
        Kind::Sequence(items) | Kind::Alternative(items) => {
            for item in items {
                check_dispatch(item, class, schema, grammar)?;
            }
        }
        Kind::Literal(_)
            if matches!(
                grammar.profile,
                Profile::NativeV25
                    | Profile::NativeV26
                    | Profile::NativeV27
                    | Profile::NativeV28
                    | Profile::NativeV29
                    | Profile::NativeV30
                    | Profile::NativeV31
                    | Profile::NativeV32
                    | Profile::NativeV33
                    | Profile::NativeV34
                    | Profile::NativeV35
                    | Profile::NativeV36
                    | Profile::NativeV37
                    | Profile::NativeV38
                    | Profile::NativeV39
                    | Profile::NativeV40
                    | Profile::NativeV41
                    | Profile::NativeV42
                    | Profile::NativeV43
                    | Profile::NativeV44
                    | Profile::NativeV45
                    | Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) =>
        {
            check_expr(expr, "", schema, grammar, false)?;
        }
        _ => return Err("Unsupported dispatch shape".into()),
    }
    Ok(())
}
// Keyword choices return the selected string through the existing expression
// evaluator. Sequences, optional RHS values and other conversions stay explicit.
// Every branch produces exactly one object value. Prediction and runtime value
// propagation remain shared with direct calls and single-child grouping.
fn check_object_assignment_rhs(
    expr: &Expr,
    grammar: &Grammar,
    schema: &Schema,
    target: &str,
) -> Result<(), String> {
    if expr.cardinality != '1' {
        return Err("Grouped object assignment requires exactly one value per branch".into());
    }
    match &expr.kind {
        Kind::Call(name) => {
            let rule = grammar
                .rules
                .get(name)
                .filter(|rule| !rule.fragment && rule.datatype.is_none())
                .ok_or("Grouped object assignment requires object parser rules")?;
            if !schema.conforms(&rule.class, target) {
                return Err("Grouped object assignment containment type mismatch".into());
            }
        }
        Kind::Alternative(items) if !items.is_empty() => {
            for item in items {
                check_object_assignment_rhs(item, grammar, schema, target)?;
            }
        }
        Kind::Sequence(items) if items.len() == 1 => {
            check_object_assignment_rhs(&items[0], grammar, schema, target)?;
        }
        _ => {
            return Err(
                "Grouped object assignment requires a single object call in every branch".into(),
            );
        }
    }
    Ok(())
}
fn keyword_assignment(expr: &Expr) -> bool {
    expr.cardinality == '1'
        && match &expr.kind {
            Kind::Literal(_) => true,
            Kind::Alternative(items) => !items.is_empty() && items.iter().all(keyword_assignment),
            Kind::Sequence(items) if items.len() == 1 => keyword_assignment(&items[0]),
            _ => false,
        }
}
fn writable_feature(
    schema: &Schema,
    class: &str,
    feature: &str,
    many: bool,
    grammar: &Grammar,
) -> Result<intersect_model::definitions::schema::Feature, String> {
    let field = schema
        .features(class)?
        .remove(feature)
        .ok_or_else(|| format!("Unknown feature {class}.{feature}"))?;
    if field.derived {
        let operation = if many {
            intersect_model::definitions::setting::SettingOperation::Append
        } else {
            intersect_model::definitions::setting::SettingOperation::Set
        };
        let member = schema.effective_feature(class, feature)?.identity;
        let admitted = grammar.profile.has_computed_settings()
            && grammar.computed_settings.iter().any(|contract| {
                contract.schema_revision == schema.revision
                    && contract.class == class
                    && contract.class_identity == schema.identity(class)
                    && contract.feature == feature
                    && contract.member == member
                    && contract.operations.contains(&operation)
            });
        if !admitted {
            return Err(format!("Cannot assign derived feature {class}.{feature}"));
        }
    }
    if !field.changeable {
        return Err(format!(
            "Cannot assign non-changeable feature {class}.{feature}"
        ));
    }
    if many != (field.upper != 1) {
        return Err(format!(
            "Assignment cardinality differs from {class}.{feature}"
        ));
    }
    Ok(field)
}
// Unassigned EString helpers consume syntax without constructing or replacing an object.
fn is_unassigned_string_call(name: &str, grammar: &Grammar) -> bool {
    if grammar.profile.has_scoped_hidden_tokens()
        && grammar
            .terminals
            .get(name)
            .is_some_and(|terminal| !terminal.fragment && terminal.datatype == Datatype::EString)
    {
        return true;
    }
    matches!(
        grammar.profile,
        Profile::NativeV33
            | Profile::NativeV34
            | Profile::NativeV35
            | Profile::NativeV36
            | Profile::NativeV37
            | Profile::NativeV38
            | Profile::NativeV39
            | Profile::NativeV40
            | Profile::NativeV41
            | Profile::NativeV42
            | Profile::NativeV43
            | Profile::NativeV44
            | Profile::NativeV45
            | Profile::NativeV46
            | Profile::NativeV47
            | Profile::NativeV48
            | Profile::NativeV49
            | Profile::NativeV50
            | Profile::NativeV51
            | Profile::NativeV52
            | Profile::NativeV53
            | Profile::NativeV54
            | Profile::NativeV55
            | Profile::NativeV56
            | Profile::NativeV57
            | Profile::NativeV58
            | Profile::NativeV59
            | Profile::NativeV60
            | Profile::NativeV61
            | Profile::NativeV62
            | Profile::NativeV63
            | Profile::NativeV64
            | Profile::NativeV65
            | Profile::NativeV66
            | Profile::NativeV67
            | Profile::NativeV68
            | Profile::NativeV69
            | Profile::NativeV70
            | Profile::NativeV71
            | Profile::NativeV72
            | Profile::NativeV73
            | Profile::NativeV74
            | Profile::NativeV75
            | Profile::NativeV76
            | Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) && grammar
        .rules
        .get(name)
        .is_some_and(|rule| rule.datatype == Some(Datatype::EString))
}
fn check_expr(
    expr: &Expr,
    class: &str,
    schema: &Schema,
    grammar: &Grammar,
    assigned: bool,
) -> Result<(), String> {
    match &expr.kind {
        Kind::SyntacticPredicate(value) => {
            if !grammar.profile.has_full_syntactic_predicates() {
                return Err("Full syntactic predicates require NativeV68".into());
            }
            check_expr(value, class, schema, grammar, assigned)?;
        }
        Kind::Capture { .. } => {
            return Err("Assigned-current action requires a current-object rule context".into());
        }
        Kind::Action(target) => {
            if assigned || target != class {
                return Err("Action does not match the selected construction class".into());
            }
        }
        Kind::Call(name) => {
            if !["ID", "STRING", "INT"].contains(&name.as_str())
                && !grammar.rules.contains_key(name)
                && !grammar.terminals.contains_key(name)
            {
                return Err(format!("Unknown rule {name}"));
            }
            if let Some(target) = grammar.rules.get(name).filter(|rule| rule.fragment) {
                if assigned {
                    return Err("Parser fragments cannot be assigned as values".into());
                }
                if !target.wildcard
                    && target.datatype.is_none()
                    && !schema.conforms(class, &target.class)
                {
                    return Err(format!(
                        "Fragment caller type mismatch: {class} does not conform to {}",
                        target.class
                    ));
                }
            } else if !assigned && !is_unassigned_string_call(name, grammar) {
                return Err(
                    "Unassigned rule calls are unsupported; use a feature assignment".into(),
                );
            }
        }
        Kind::Reference(target) | Kind::ScalarReference { target, .. } => {
            if !assigned || !schema.is_reference_target(target) {
                return Err(format!("Invalid cross-reference {target}"));
            }
            if let Kind::ScalarReference { rule, .. } = &expr.kind
                && !grammar
                    .rules
                    .get(rule)
                    .is_some_and(|rule| rule.datatype == Some(Datatype::EString))
            {
                return Err(
                    "Cross-reference spelling requires an EString datatype parser rule".into(),
                );
            }
        }
        Kind::Assign {
            feature,
            many,
            value,
        } => {
            let field = writable_feature(schema, class, feature, *many, grammar)?;
            match &value.kind {
                Kind::Literal(_) | Kind::Alternative(_) | Kind::Sequence(_)
                    if matches!(
                        grammar.profile,
                        Profile::NativeV27
                            | Profile::NativeV28
                            | Profile::NativeV29
                            | Profile::NativeV30
                            | Profile::NativeV31
                            | Profile::NativeV32
                            | Profile::NativeV33
                            | Profile::NativeV34
                            | Profile::NativeV35
                            | Profile::NativeV36
                            | Profile::NativeV37
                            | Profile::NativeV38
                            | Profile::NativeV39
                            | Profile::NativeV40
                            | Profile::NativeV41
                            | Profile::NativeV42
                            | Profile::NativeV43
                            | Profile::NativeV44
                            | Profile::NativeV45
                            | Profile::NativeV46
                            | Profile::NativeV47
                            | Profile::NativeV48
                            | Profile::NativeV49
                            | Profile::NativeV50
                            | Profile::NativeV51
                            | Profile::NativeV52
                            | Profile::NativeV53
                            | Profile::NativeV54
                            | Profile::NativeV55
                            | Profile::NativeV56
                            | Profile::NativeV57
                            | Profile::NativeV58
                            | Profile::NativeV59
                            | Profile::NativeV60
                            | Profile::NativeV61
                            | Profile::NativeV62
                            | Profile::NativeV63
                            | Profile::NativeV64
                            | Profile::NativeV65
                            | Profile::NativeV66
                            | Profile::NativeV67
                            | Profile::NativeV68
                            | Profile::NativeV69
                            | Profile::NativeV70
                            | Profile::NativeV71
                            | Profile::NativeV72
                            | Profile::NativeV73
                            | Profile::NativeV74
                            | Profile::NativeV75
                            | Profile::NativeV76
                            | Profile::NativeV77
                            | Profile::NativeV78
                            | Profile::NativeV79
                            | Profile::NativeV80
                            | Profile::NativeV81
                            | Profile::NativeV82
                    ) && keyword_assignment(value) =>
                {
                    if field.reference
                        || field.datatype.is_some()
                        || field.target != "EString"
                        || !schema.feature_domains(class, feature).is_empty()
                    {
                        return Err(format!(
                            "Keyword assignment requires an unconstrained EString attribute: {class}.{feature}"
                        ));
                    }
                }
                Kind::Alternative(_) | Kind::Sequence(_)
                    if matches!(
                        grammar.profile,
                        Profile::NativeV36
                            | Profile::NativeV37
                            | Profile::NativeV38
                            | Profile::NativeV39
                            | Profile::NativeV40
                            | Profile::NativeV41
                            | Profile::NativeV42
                            | Profile::NativeV43
                            | Profile::NativeV44
                            | Profile::NativeV45
                            | Profile::NativeV46
                            | Profile::NativeV47
                            | Profile::NativeV48
                            | Profile::NativeV49
                            | Profile::NativeV50
                            | Profile::NativeV51
                            | Profile::NativeV52
                            | Profile::NativeV53
                            | Profile::NativeV54
                            | Profile::NativeV55
                            | Profile::NativeV56
                            | Profile::NativeV57
                            | Profile::NativeV58
                            | Profile::NativeV59
                            | Profile::NativeV60
                            | Profile::NativeV61
                            | Profile::NativeV62
                            | Profile::NativeV63
                            | Profile::NativeV64
                            | Profile::NativeV65
                            | Profile::NativeV66
                            | Profile::NativeV67
                            | Profile::NativeV68
                            | Profile::NativeV69
                            | Profile::NativeV70
                            | Profile::NativeV71
                            | Profile::NativeV72
                            | Profile::NativeV73
                            | Profile::NativeV74
                            | Profile::NativeV75
                            | Profile::NativeV76
                            | Profile::NativeV77
                            | Profile::NativeV78
                            | Profile::NativeV79
                            | Profile::NativeV80
                            | Profile::NativeV81
                            | Profile::NativeV82
                    ) =>
                {
                    if !field.reference || !field.containment {
                        return Err(format!(
                            "Grouped object assignment requires containment: {class}.{feature}"
                        ));
                    }
                    check_object_assignment_rhs(value, grammar, schema, &field.target)?;
                }
                Kind::Call(n)
                    if grammar
                        .rules
                        .get(n)
                        .is_some_and(|rule| rule.datatype.is_some()) =>
                {
                    let target = &grammar.rules[n];
                    if target.datatype == Some(Datatype::Nominal) {
                        if !grammar.profile.has_durable_documents()
                            || field.reference
                            || target.nominal_return.as_ref() != field.datatype.as_ref()
                            || !schema.datatypes.get(&target.class).is_some_and(|datatype| {
                                Some(datatype.declaration()) == target.nominal_return.as_ref()
                            })
                        {
                            return Err(format!(
                                "Nominal datatype parser assignment identity mismatch for {class}.{feature}"
                            ));
                        }
                    } else if let Some(Datatype::Builtin(codec)) = target.datatype {
                        if !grammar.profile.has_builtin_scalar_returns()
                            || field.reference
                            || field.datatype.is_some()
                            || !schema.feature_domains(class, feature).is_empty()
                            || intersect_model::definitions::scalars::Builtin::named(&field.target)
                                != Some(codec)
                        {
                            return Err(format!(
                                "Builtin datatype rule assignment mismatch for {class}.{feature}: {codec:?}"
                            ));
                        }
                    } else if target.datatype == Some(Datatype::Enum) {
                        enum_rules::check_assignment(schema, class, feature, target, &field)?;
                    } else if target.datatype == Some(Datatype::EBoolean) {
                        if field.reference
                            || field.datatype.is_some()
                            || field.target != "EBoolean"
                            || !schema.feature_domains(class, feature).is_empty()
                        {
                            return Err(format!(
                                "Boolean datatype parser assignment requires an unconstrained EBoolean attribute: {class}.{feature}"
                            ));
                        }
                    } else if target.datatype == Some(Datatype::EDouble) {
                        if field.reference
                            || field.datatype.is_some()
                            || field.target != "EDouble"
                            || !schema.feature_domains(class, feature).is_empty()
                        {
                            return Err(format!(
                                "Double datatype parser assignment requires an unconstrained EDouble attribute: {class}.{feature}"
                            ));
                        }
                    } else if field.reference
                        || field.datatype.is_some()
                        || field.target != "EString"
                        || !schema.feature_domains(class, feature).is_empty()
                    {
                        return Err(format!(
                            "Datatype parser assignment requires an unconstrained EString attribute: {class}.{feature}"
                        ));
                    }
                }
                Kind::Call(n) if grammar.rules.contains_key(n) => {
                    let target = &grammar.rules[n].class;
                    if !field.reference
                        || !field.containment
                        || !schema.conforms(target, &field.target)
                    {
                        return Err(format!("Containment type mismatch for {class}.{feature}"));
                    }
                }
                Kind::Reference(target) | Kind::ScalarReference { target, .. } => {
                    if !field.reference
                        || field.containment
                        || (target != intersect_model::definitions::schema::EOBJECT_REFERENCE_TOP
                            && !schema.conforms(target, &field.target))
                    {
                        return Err(format!("Reference type mismatch for {class}.{feature}"));
                    }
                }
                Kind::Call(n) => {
                    if let Some(terminal) = grammar.terminals.get(n)
                        && terminal.datatype == Datatype::Nominal
                    {
                        if !grammar.profile.has_durable_documents()
                            || field.reference
                            || terminal.nominal_return.as_ref() != field.datatype.as_ref()
                        {
                            return Err(format!(
                                "Nominal terminal assignment identity mismatch for {class}.{feature}"
                            ));
                        }
                        return check_expr(value, class, schema, grammar, true);
                    }
                    if let Some(enumeration) = grammar
                        .terminals
                        .get(n)
                        .and_then(|terminal| terminal.enumeration.as_deref())
                    {
                        if field.reference
                            || schema.runtime_attribute_lexical_target(class, feature)?
                                != Some(enumeration)
                        {
                            return Err(format!(
                                "Enum terminal type mismatch for {class}.{feature}"
                            ));
                        }
                        return check_expr(value, class, schema, grammar, true);
                    }
                    if let Some(Datatype::Builtin(codec)) =
                        grammar.terminals.get(n).map(|terminal| terminal.datatype)
                    {
                        let lexical_target = schema
                            .runtime_attribute_lexical_target(class, feature)?
                            .unwrap_or(&field.target);
                        if !grammar.profile.has_builtin_scalar_returns()
                            || field.reference
                            || intersect_model::definitions::scalars::Builtin::named(lexical_target)
                                != Some(codec)
                        {
                            return Err(format!(
                                "Builtin terminal type mismatch for {class}.{feature}: {codec:?}"
                            ));
                        }
                        return check_expr(value, class, schema, grammar, true);
                    }
                    let expected = if grammar
                        .terminals
                        .get(n)
                        .map_or(n == "INT", |terminal| terminal.datatype == Datatype::EInt)
                    {
                        "EInt"
                    } else {
                        "EString"
                    };
                    let lexical_target = if schema.feature_domains(class, feature).is_empty() {
                        field.target.as_str()
                    } else {
                        schema.runtime_attribute_lexical_target(class, feature)?
                            .ok_or_else(|| format!("No exact lexical codec for constrained feature {class}.{feature}"))?
                    };
                    let lexical_datatype = schema
                        .datatypes
                        .get(lexical_target)
                        .is_some_and(|datatype| datatype.serializable());
                    if field.reference
                        || (!lexical_datatype
                            && (field.datatype.is_some() || lexical_target != expected))
                    {
                        return Err(format!("Terminal type mismatch for {class}.{feature}"));
                    }
                }
                _ => {
                    return Err(
                        "Assignments require a terminal, parser rule or cross-reference".into(),
                    );
                }
            }
            check_expr(value, class, schema, grammar, true)?;
        }
        Kind::BooleanAssign { feature, value } => {
            let fields = schema.features(class)?;
            let field = fields
                .get(feature)
                .ok_or_else(|| format!("Unknown feature {class}.{feature}"))?;
            if field.reference
                || field.upper != 1
                || (field.derived
                    && !(grammar.profile.has_computed_boolean_settings()
                        && grammar.computed_settings.iter().any(|contract| {
                            contract.schema_revision == schema.revision
                                && contract.class == class
                                && contract.class_identity == schema.identity(class)
                                && contract.feature == *feature
                                && schema
                                    .effective_feature(class, feature)
                                    .is_ok_and(|member| contract.member == member.identity)
                                && contract.operations.contains(
                                    &intersect_model::definitions::setting::SettingOperation::Set,
                                )
                        })))
                || !field.changeable
                || field.datatype.is_some()
                || schema.runtime_attribute_lexical_target(class, feature)? != Some("EBoolean")
            {
                return Err(format!(
                    "Boolean assignment requires a writable single EBoolean attribute: {class}.{feature}"
                ));
            }
            if !matches!(value.kind, Kind::Literal(_)) {
                return Err("Boolean assignments currently require a keyword RHS".into());
            }
            check_expr(value, class, schema, grammar, true)?;
        }
        Kind::Sequence(items) | Kind::Alternative(items) => {
            for item in items {
                check_expr(item, class, schema, grammar, assigned)?;
            }
        }
        Kind::Literal(text) => {
            if text.is_empty() {
                return Err("Empty keywords are unsupported".into());
            }
            if !grammar.profile.is_workbench() && keywords::is_punctuation_keyword(text) {
                return Ok(());
            }
            let tokens = lex(text).map_err(|(_, e)| e)?;
            if tokens.len() != 1 || tokens[0].quoted || tokens[0].text != *text {
                return Err(
                    "Keywords must be a single identifier or punctuation character in this profile"
                        .into(),
                );
            }
        }
    }
    Ok(())
}
// Disjoint prediction is required instead of silently substituting PEG choice.
fn first(expr: &Expr, grammar: &Grammar, depth: usize) -> Result<(BTreeSet<String>, bool), String> {
    first_with_work_budget(expr, grammar, depth, grammar.analysis_work_budget)
}
fn first_with_work_budget(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    work_budget: GrammarAnalysisWorkBudget,
) -> Result<(BTreeSet<String>, bool), String> {
    if matches!(
        grammar.profile,
        Profile::NativeV23
            | Profile::NativeV24
            | Profile::NativeV25
            | Profile::NativeV26
            | Profile::NativeV27
            | Profile::NativeV28
            | Profile::NativeV29
            | Profile::NativeV30
            | Profile::NativeV31
            | Profile::NativeV32
            | Profile::NativeV33
            | Profile::NativeV34
            | Profile::NativeV35
            | Profile::NativeV36
            | Profile::NativeV37
            | Profile::NativeV38
            | Profile::NativeV39
            | Profile::NativeV40
            | Profile::NativeV41
            | Profile::NativeV42
            | Profile::NativeV43
            | Profile::NativeV44
            | Profile::NativeV45
            | Profile::NativeV46
            | Profile::NativeV47
            | Profile::NativeV48
            | Profile::NativeV49
            | Profile::NativeV50
            | Profile::NativeV51
            | Profile::NativeV52
            | Profile::NativeV53
            | Profile::NativeV54
            | Profile::NativeV55
            | Profile::NativeV56
            | Profile::NativeV57
            | Profile::NativeV58
            | Profile::NativeV59
            | Profile::NativeV60
            | Profile::NativeV61
            | Profile::NativeV62
            | Profile::NativeV63
            | Profile::NativeV64
            | Profile::NativeV65
            | Profile::NativeV66
            | Profile::NativeV67
            | Profile::NativeV68
            | Profile::NativeV69
            | Profile::NativeV70
            | Profile::NativeV71
            | Profile::NativeV72
            | Profile::NativeV73
            | Profile::NativeV74
            | Profile::NativeV75
            | Profile::NativeV76
            | Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) {
        first_sets::analyze(expr, grammar, depth, work_budget.units())
    } else {
        first_inner(expr, grammar, depth, &mut work_budget.units())
    }
}
fn first_inner(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    budget: &mut usize,
) -> Result<(BTreeSet<String>, bool), String> {
    if depth > 64 || *budget == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    let (set, nullable) = match &expr.kind {
        Kind::Action(_) | Kind::Capture { .. } => (BTreeSet::new(), true),
        Kind::Literal(s) => ([format!("literal:{s}")].into(), false),
        Kind::Reference(_) => (["terminal:ID".into()].into(), false),
        Kind::ScalarReference { rule, .. } => {
            let rule = grammar
                .rules
                .get(rule)
                .ok_or_else(|| "Unknown datatype reference rule".to_string())?;
            first_inner(&rule.body, grammar, depth + 1, budget)?
        }
        Kind::Call(name) => match grammar.rules.get(name) {
            Some(rule) => first_inner(&rule.body, grammar, depth + 1, budget)?,
            None => ([format!("terminal:{name}")].into(), false),
        },
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => first_inner(value, grammar, depth + 1, budget)?,
        Kind::Sequence(items) => {
            let mut set = BTreeSet::new();
            let mut nullable = true;
            for item in items {
                let (next, empty) = first_inner(item, grammar, depth + 1, budget)?;
                set.extend(next);
                if !empty {
                    nullable = false;
                    break;
                }
            }
            (set, nullable)
        }
        Kind::Alternative(items) => {
            let mut set = BTreeSet::new();
            let mut nullable = false;
            for item in items {
                let (next, empty) = first_inner(item, grammar, depth + 1, budget)?;
                set.extend(next);
                nullable |= empty;
            }
            (set, nullable)
        }
    };
    Ok((set, nullable || ['?', '*'].contains(&expr.cardinality)))
}
fn overlaps(left: &BTreeSet<String>, right: &BTreeSet<String>) -> bool {
    if !left.is_disjoint(right) {
        return true;
    }
    let numeric = |set: &BTreeSet<String>| {
        set.iter().any(|s| {
            s.strip_prefix("literal:")
                .is_some_and(|s| s.chars().all(|c| c.is_ascii_digit()))
        })
    };
    (left.contains("terminal:INT") && numeric(right))
        || (right.contains("terminal:INT") && numeric(left))
}
// Only a guard on every entry path can resolve an enclosing decision.
fn visible_entry_guard(expr: &Expr) -> bool {
    visible_entry_guard_expression(expr).is_some()
}
// Same existing source predicate, retaining its exact witness for a private
// native binder. No additional alternative or nullable path is recognized.
fn visible_entry_guard_expression(expr: &Expr) -> Option<&Expr> {
    if expr.first_set_predicated || matches!(expr.kind, Kind::SyntacticPredicate(_)) {
        return Some(expr);
    }
    match &expr.kind {
        Kind::Sequence(items) => items
            .first()
            .filter(|item| item.cardinality == '1')
            .and_then(visible_entry_guard_expression),
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => visible_entry_guard_expression(value),
        _ => None,
    }
}
fn bind_prediction_paths(
    expr: &mut Expr,
    path: &str,
    predicates: &mut BTreeSet<String>,
    all: bool,
) {
    if expr.first_set_predicated {
        predicates.insert(path.into());
    }
    if all
        || matches!(&expr.kind, Kind::Alternative(items) if !items.is_empty())
        || matches!(expr.cardinality, '?' | '*' | '+')
    {
        expr.prediction_path = Some(path.into());
    }
    match &mut expr.kind {
        Kind::Sequence(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                bind_prediction_paths(item, &format!("{path}/s{index}"), predicates, all);
            }
        }
        Kind::Alternative(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                bind_prediction_paths(item, &format!("{path}/a{index}"), predicates, all);
            }
        }
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => {
            bind_prediction_paths(value, &format!("{path}/value"), predicates, all)
        }
        _ => {}
    }
}
// Unquantified nullable wrappers introduce no skip decision of their own.
// The called fragment's FOLLOW and nested decision proofs remain mandatory.
fn is_structural_nullable_continuation(expr: &Expr, grammar: &Grammar) -> bool {
    matches!(
        grammar.profile,
        Profile::NativeV34
            | Profile::NativeV35
            | Profile::NativeV36
            | Profile::NativeV37
            | Profile::NativeV38
            | Profile::NativeV39
            | Profile::NativeV40
            | Profile::NativeV41
            | Profile::NativeV42
            | Profile::NativeV43
            | Profile::NativeV44
            | Profile::NativeV45
            | Profile::NativeV46
            | Profile::NativeV47
            | Profile::NativeV48
            | Profile::NativeV49
            | Profile::NativeV50
            | Profile::NativeV51
            | Profile::NativeV52
            | Profile::NativeV53
            | Profile::NativeV54
            | Profile::NativeV55
            | Profile::NativeV56
            | Profile::NativeV57
            | Profile::NativeV58
            | Profile::NativeV59
            | Profile::NativeV60
            | Profile::NativeV61
            | Profile::NativeV62
            | Profile::NativeV63
            | Profile::NativeV64
            | Profile::NativeV65
            | Profile::NativeV66
            | Profile::NativeV67
            | Profile::NativeV68
            | Profile::NativeV69
            | Profile::NativeV70
            | Profile::NativeV71
            | Profile::NativeV72
            | Profile::NativeV73
            | Profile::NativeV74
            | Profile::NativeV75
            | Profile::NativeV76
            | Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) && expr.cardinality == '1'
        && match &expr.kind {
            Kind::Sequence(_)
            | Kind::Alternative(_)
            | Kind::Assign { .. }
            | Kind::BooleanAssign { .. }
            | Kind::SyntacticPredicate(_) => true,
            Kind::Call(name) => grammar.rules.get(name).is_some_and(|rule| rule.fragment),
            _ => false,
        }
}
fn check_prediction(
    expr: &Expr,
    grammar: &Grammar,
    path: &str,
    deferred: &mut BTreeSet<String>,
    work_budget: GrammarAnalysisWorkBudget,
) -> Result<(), String> {
    if ['*', '+'].contains(&expr.cardinality) {
        let mut once = expr.clone();
        once.cardinality = '1';
        if matches!(
            grammar.profile,
            Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ) {
            // Eligibility consumes the remaining work of THIS original FIRST
            // invocation; it does not initialize another analysis allowance.
            let mut remaining = work_budget.units();
            if first_sets::analyze_with_remaining(&once, grammar, 0, &mut remaining)?.1 {
                let proof = grammar.qualified_nullable_full_repeat_with_work(
                    &format!("{path}/repeat"),
                    &mut remaining,
                )?;
                if proof.is_none()
                    || !std::ptr::eq(
                        proof
                            .as_ref()
                            .ok_or("Missing nullable repeat source proof")?
                            .expression(),
                        expr,
                    )
                {
                    return Err("Nullable repetitions are unsupported".into());
                }
            }
        } else if first_with_work_budget(&once, grammar, 0, work_budget)?.1 {
            return Err("Nullable repetitions are unsupported".into());
        }
    }
    match &expr.kind {
        Kind::Alternative(items) => {
            let mut previous: Vec<(&Expr, BTreeSet<String>)> = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let (next, nullable) = first_with_work_budget(item, grammar, 0, work_budget)?;
                if nullable
                    && !matches!(
                        grammar.profile,
                        Profile::NativeV24
                            | Profile::NativeV25
                            | Profile::NativeV26
                            | Profile::NativeV27
                            | Profile::NativeV28
                            | Profile::NativeV29
                            | Profile::NativeV30
                            | Profile::NativeV31
                            | Profile::NativeV32
                            | Profile::NativeV33
                            | Profile::NativeV34
                            | Profile::NativeV35
                            | Profile::NativeV36
                            | Profile::NativeV37
                            | Profile::NativeV38
                            | Profile::NativeV39
                            | Profile::NativeV40
                            | Profile::NativeV41
                            | Profile::NativeV42
                            | Profile::NativeV43
                            | Profile::NativeV44
                            | Profile::NativeV45
                            | Profile::NativeV46
                            | Profile::NativeV47
                            | Profile::NativeV48
                            | Profile::NativeV49
                            | Profile::NativeV50
                            | Profile::NativeV51
                            | Profile::NativeV52
                            | Profile::NativeV53
                            | Profile::NativeV54
                            | Profile::NativeV55
                            | Profile::NativeV56
                            | Profile::NativeV57
                            | Profile::NativeV58
                            | Profile::NativeV59
                            | Profile::NativeV60
                            | Profile::NativeV61
                            | Profile::NativeV62
                            | Profile::NativeV63
                            | Profile::NativeV64
                            | Profile::NativeV65
                            | Profile::NativeV66
                            | Profile::NativeV67
                            | Profile::NativeV68
                            | Profile::NativeV69
                            | Profile::NativeV70
                            | Profile::NativeV71
                            | Profile::NativeV72
                            | Profile::NativeV73
                            | Profile::NativeV74
                            | Profile::NativeV75
                            | Profile::NativeV76
                            | Profile::NativeV77
                            | Profile::NativeV78
                            | Profile::NativeV79
                            | Profile::NativeV80
                            | Profile::NativeV81
                            | Profile::NativeV82
                    )
                {
                    return Err(
                        "Overlapping or nullable alternatives require unsupported Xtext prediction"
                            .into(),
                    );
                }
                if nullable {
                    deferred.insert(format!("{path}/choice"));
                }
                for (earlier, head) in &previous {
                    if overlaps(head, &next)
                        && !(grammar.profile.has_parser_prediction()
                            && visible_entry_guard(earlier)
                            && earlier.cardinality == '1')
                    {
                        if matches!(
                            grammar.profile,
                            Profile::NativeV24
                                | Profile::NativeV25
                                | Profile::NativeV26
                                | Profile::NativeV27
                                | Profile::NativeV28
                                | Profile::NativeV29
                                | Profile::NativeV30
                                | Profile::NativeV31
                                | Profile::NativeV32
                                | Profile::NativeV33
                                | Profile::NativeV34
                                | Profile::NativeV35
                                | Profile::NativeV36
                                | Profile::NativeV37
                                | Profile::NativeV38
                                | Profile::NativeV39
                                | Profile::NativeV40
                                | Profile::NativeV41
                                | Profile::NativeV42
                                | Profile::NativeV43
                                | Profile::NativeV44
                                | Profile::NativeV45
                                | Profile::NativeV46
                                | Profile::NativeV47
                                | Profile::NativeV48
                                | Profile::NativeV49
                                | Profile::NativeV50
                                | Profile::NativeV51
                                | Profile::NativeV52
                                | Profile::NativeV53
                                | Profile::NativeV54
                                | Profile::NativeV55
                                | Profile::NativeV56
                                | Profile::NativeV57
                                | Profile::NativeV58
                                | Profile::NativeV59
                                | Profile::NativeV60
                                | Profile::NativeV61
                                | Profile::NativeV62
                                | Profile::NativeV63
                                | Profile::NativeV64
                                | Profile::NativeV65
                                | Profile::NativeV66
                                | Profile::NativeV67
                                | Profile::NativeV68
                                | Profile::NativeV69
                                | Profile::NativeV70
                                | Profile::NativeV71
                                | Profile::NativeV72
                                | Profile::NativeV73
                                | Profile::NativeV74
                                | Profile::NativeV75
                                | Profile::NativeV76
                                | Profile::NativeV77
                                | Profile::NativeV78
                                | Profile::NativeV79
                                | Profile::NativeV80
                                | Profile::NativeV81
                                | Profile::NativeV82
                        ) {
                            deferred.insert(format!("{path}/choice"));
                        } else {
                            return Err("Overlapping or nullable alternatives require unsupported Xtext prediction".into());
                        }
                    }
                }
                previous.push((item, next));
                check_prediction(
                    item,
                    grammar,
                    &format!("{path}/a{index}"),
                    deferred,
                    work_budget,
                )?;
            }
        }
        Kind::Sequence(items) => {
            for (index, item) in items.iter().enumerate() {
                check_prediction(
                    item,
                    grammar,
                    &format!("{path}/s{index}"),
                    deferred,
                    work_budget,
                )?;
                if ['?', '*', '+'].contains(&item.cardinality)
                    || first_with_work_budget(item, grammar, 0, work_budget)?.1
                {
                    let (head, _) = first_with_work_budget(item, grammar, 0, work_budget)?;
                    for following in &items[index + 1..] {
                        let (tail, nullable) =
                            first_with_work_budget(following, grammar, 0, work_budget)?;
                        if overlaps(&head, &tail)
                            && !(grammar.profile.has_parser_prediction()
                                && visible_entry_guard(item))
                        {
                            if matches!(
                                grammar.profile,
                                Profile::NativeV30
                                    | Profile::NativeV31
                                    | Profile::NativeV32
                                    | Profile::NativeV33
                                    | Profile::NativeV34
                                    | Profile::NativeV35
                                    | Profile::NativeV36
                                    | Profile::NativeV37
                                    | Profile::NativeV38
                                    | Profile::NativeV39
                                    | Profile::NativeV40
                                    | Profile::NativeV41
                                    | Profile::NativeV42
                                    | Profile::NativeV43
                                    | Profile::NativeV44
                                    | Profile::NativeV45
                                    | Profile::NativeV46
                                    | Profile::NativeV47
                                    | Profile::NativeV48
                                    | Profile::NativeV49
                                    | Profile::NativeV50
                                    | Profile::NativeV51
                                    | Profile::NativeV52
                                    | Profile::NativeV53
                                    | Profile::NativeV54
                                    | Profile::NativeV55
                                    | Profile::NativeV56
                                    | Profile::NativeV57
                                    | Profile::NativeV58
                                    | Profile::NativeV59
                                    | Profile::NativeV60
                                    | Profile::NativeV61
                                    | Profile::NativeV62
                                    | Profile::NativeV63
                                    | Profile::NativeV64
                                    | Profile::NativeV65
                                    | Profile::NativeV66
                                    | Profile::NativeV67
                                    | Profile::NativeV68
                                    | Profile::NativeV69
                                    | Profile::NativeV70
                                    | Profile::NativeV71
                                    | Profile::NativeV72
                                    | Profile::NativeV73
                                    | Profile::NativeV74
                                    | Profile::NativeV75
                                    | Profile::NativeV76
                                    | Profile::NativeV77
                                    | Profile::NativeV78
                                    | Profile::NativeV79
                                    | Profile::NativeV80
                                    | Profile::NativeV81
                                    | Profile::NativeV82
                            ) && ['?', '*', '+'].contains(&item.cardinality)
                            {
                                deferred.insert(format!("{path}/s{index}/repeat"));
                            } else if is_structural_nullable_continuation(item, grammar) {
                                // FOLLOW validates nested repeats; nullable alternatives
                                // already require a shared-graph disjointness proof.
                            } else {
                                return Err("Repetition/optional follow conflict requires unsupported Xtext prediction".into());
                            }
                        }
                        if !nullable {
                            break;
                        }
                    }
                }
            }
        }
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => check_prediction(
            value,
            grammar,
            &format!("{path}/value"),
            deferred,
            work_budget,
        )?,
        _ => {}
    }
    Ok(())
}
fn check_left(
    expr: &Expr,
    grammar: &Grammar,
    seen: &mut BTreeSet<String>,
    work_budget: GrammarAnalysisWorkBudget,
) -> Result<(), String> {
    check_left_inner(expr, grammar, seen, &mut work_budget.units(), work_budget)
}
fn check_left_inner(
    expr: &Expr,
    grammar: &Grammar,
    seen: &mut BTreeSet<String>,
    budget: &mut usize,
    work_budget: GrammarAnalysisWorkBudget,
) -> Result<(), String> {
    if *budget == 0 || seen.len() > 64 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *budget -= 1;
    match &expr.kind {
        Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
            if let Some(rule) = grammar.rules.get(name) {
                if !seen.insert(name.clone()) {
                    return Err(format!("Left recursion through {name}"));
                }
                check_left_inner(&rule.body, grammar, seen, budget, work_budget)?;
                seen.remove(name);
            }
        }
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => {
            check_left_inner(value, grammar, seen, budget, work_budget)?
        }
        Kind::Alternative(items) => {
            for item in items {
                check_left_inner(item, grammar, &mut seen.clone(), budget, work_budget)?;
            }
        }
        Kind::Sequence(items) => {
            for item in items {
                check_left_inner(item, grammar, seen, budget, work_budget)?;
                if !first_with_work_budget(item, grammar, 0, work_budget)?.1 {
                    break;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn bind_reference_types(
    expression: &mut Expr,
    schema: &Schema,
    aliases: &BTreeMap<String, Vec<String>>,
    profile: Profile,
    imports: &PackageImports,
) -> Result<(), String> {
    match &mut expression.kind {
        Kind::Action(name) | Kind::Capture { class: name, .. } => {
            let (namespace, local) = imported_type(name, aliases, schema, profile, imports)?;
            *name = imported_class_key(schema, namespace, local)?.to_owned();
        }
        Kind::Reference(name) | Kind::ScalarReference { target: name, .. } => {
            let (namespace, local) = imported_type(name, aliases, schema, profile, imports)?;
            *name = if namespace == schema.uri
                || namespace == intersect_model::definitions::ecore::ECORE
            {
                type_names::reference_key(schema, namespace, local)?
            } else {
                imported_class_key(schema, namespace, local)?
            }
            .to_owned();
        }
        Kind::Assign { value, .. }
        | Kind::BooleanAssign { value, .. }
        | Kind::SyntacticPredicate(value) => {
            bind_reference_types(value, schema, aliases, profile, imports)?
        }
        Kind::Sequence(items) | Kind::Alternative(items) => {
            for item in items {
                bind_reference_types(item, schema, aliases, profile, imports)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl Grammar {
    /// Call depth for the prediction program compiled from this grammar.
    pub(crate) fn prediction_call_depth(&self) -> usize {
        self.call_depth.unwrap_or(DEFAULT_CALL_DEPTH)
    }
}
impl Grammar {
    /// Shared sorted keyword inventory for prediction, runtime and tooling.
    pub fn keywords(&self) -> Vec<String> {
        fn visit(expr: &Expr, out: &mut Vec<String>) {
            match &expr.kind {
                Kind::Literal(s) => {
                    if !out.contains(s) {
                        out.push(s.clone());
                    }
                }
                Kind::Assign { value, .. }
                | Kind::BooleanAssign { value, .. }
                | Kind::SyntacticPredicate(value) => visit(value, out),
                Kind::Sequence(items) | Kind::Alternative(items) => {
                    for item in items {
                        visit(item, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for rule in self.rules.values() {
            visit(&rule.body, &mut out);
        }
        out.sort();
        out
    }
}

#[cfg(test)]
#[test]
fn authored_context_layout_after_grammar() {
    println!(
        "authored_context_grammar_layout={{\"imports\":{},\"parser\":{},\"source_context\":{}}}",
        std::mem::size_of::<PackageImports>(),
        std::mem::size_of::<Parser>(),
        std::mem::size_of::<SourceProgramContext<'_, '_>>()
    );
}

// Transitional backend publication guard. It stays private to the backend while
// the Xtext owner migrates; no external schema/grammar binding factory exposes it.
pub(crate) struct RuntimeOwnerPublicationPermit {
    _private: (),
}

pub(crate) trait RuntimeGrammarContext {
    fn runtime_grammar(&self) -> Option<&Grammar>;
}
impl RuntimeGrammarContext for Grammar {
    fn runtime_grammar(&self) -> Option<&Grammar> {
        Some(self)
    }
}

/// Requested compiler limits and proof policy; this data carries no admission.
#[derive(Clone, Copy)]
pub struct RuntimeGrammarRequest {
    pub policy: RuntimeGuardedPolicy,
    pub work_budget: GrammarAnalysisWorkBudget,
    pub call_depth: usize,
}

pub enum RuntimeGrammarError {
    Diagnostic(Diagnostic),
    NoObligations,
    Publication(String),
}

/// Immutable result of fresh grammar compilation and private owner binding.
/// There is no constructor from a grammar/admission tuple or approval flag.
///
/// ```compile_fail
/// use intersect_text::grammar::CompiledRuntimeGrammar;
/// fn load(input: &str) -> Result<CompiledRuntimeGrammar, serde_json::Error> {
///     serde_json::from_str(input)
/// }
/// ```
pub struct CompiledRuntimeGrammar {
    grammar: Box<Grammar>,
    admission: RuntimeGuardedAdmission,
}
impl CompiledRuntimeGrammar {
    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }
    pub fn admission(&self) -> &RuntimeGuardedAdmission {
        &self.admission
    }
    /// Consume the owner into ordinary export data, discarding admission.
    pub fn into_metadata(self) -> Grammar {
        *self.grammar
    }
    pub fn save(&self) -> Result<SavedRuntimeGrammar, String> {
        Ok(SavedRuntimeGrammar {
            compiler_identity: crate::compiler_identity().to_owned(),
            grammar: (*self.grammar).clone(),
            admission: self.admission.save()?,
        })
    }
    /// Restore only this native host's previous compiler output, matched by its
    /// compiler identity and section digests. This is the existing trusted host
    /// cache exception, never an untrusted model/network/browser load. Both
    /// modes trust the saved requirement set; recompilation remains the oracle.
    /// Optional model preparation caches are not restored by this operation.
    pub fn load_host_cache(
        saved: SavedRuntimeGrammar,
        mode: RuntimeAdmissionLoadMode,
    ) -> Result<Self, String> {
        let SavedRuntimeGrammar {
            compiler_identity,
            mut grammar,
            admission,
        } = saved;
        if compiler_identity != crate::compiler_identity() {
            return Err("Saved grammar compiler identity differs; rebuild from sources".into());
        }
        if grammar.profile != Profile::NativeV80 {
            return Err("Saved draft package requires NativeV80".into());
        }
        grammar.call_depth = Some(admission.call_depth());
        grammar
            .parser_prediction
            .as_mut()
            .ok_or("Saved draft package has no prediction program")?
            .restore_host_cache_transients(grammar.profile);
        let grammar = Box::new(grammar);
        let permit = RuntimeOwnerPublicationPermit { _private: () };
        let admission =
            RuntimeGuardedAdmission::load_host_cache(admission, &permit, grammar.as_ref(), mode)?;
        Ok(Self { grammar, admission })
    }
}

/// Saved native-host data, not executable authority. Deserialization alone
/// grants nothing; only the explicit trusted-host loader above publishes it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedRuntimeGrammar {
    compiler_identity: String,
    grammar: Grammar,
    admission: SavedRuntimeAdmission,
}

/// Compile and bind one actual grammar using the canonical backend. The model
/// context is checked source output; the private publication guard never escapes.
#[allow(clippy::too_many_arguments)]
pub fn compile_runtime_sources_with_package_imports(
    bundle: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    model: &mut intersect_model::definitions::model_definition::CheckedModelCompilation<'_>,
    profile: Profile,
    limits: SourceLimits,
    settings: &[intersect_model::definitions::setting::ComputedSettingContract],
    mut imports: PackageImports<'_>,
    observation: Option<admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&SourcePrefixDiagnostic<'_>>,
    order: RecursiveFrontierOrder,
    pool: Option<&admission_observation::pool_blocks::CompilerPoolDiagnostic>,
    request: RuntimeGrammarRequest,
) -> Result<CompiledRuntimeGrammar, RuntimeGrammarError> {
    if profile != Profile::NativeV80 {
        return Err(RuntimeGrammarError::Publication(
            "Runtime guarded admission requires explicit NativeV80 ordinary sources".into(),
        ));
    }
    if !std::ptr::eq(bundle, model.sources()) {
        return Err(RuntimeGrammarError::Publication(
            "Runtime grammar requires its actual checked model source world".into(),
        ));
    }
    // Runtime correspondence uses the checked model's actual inventory, never
    // declaration data supplied from another compilation by a native caller.
    imports.declarations = model.take_declarations();
    let mut admission =
        RuntimeGuardedAdmission::new_with_policy_and_budget(request.policy, request.work_budget)
            .with_call_depth(request.call_depth)
            .map_err(RuntimeGrammarError::Publication)?;
    let grammar = compile_sources_with_package_imports_prediction_policy(
        bundle,
        entry,
        model,
        profile,
        limits,
        settings,
        imports,
        observation,
        prefix,
        order,
        pool,
        Some(&mut admission),
    )
    .map_err(RuntimeGrammarError::Diagnostic)?;
    if admission.decision_count() == 0 {
        return Err(RuntimeGrammarError::NoObligations);
    }
    if !admission.owner_allocation_reserved() {
        return Err(RuntimeGrammarError::Publication(
            "Runtime owner allocation was not reserved by successful sealing".into(),
        ));
    }
    // Sealing prepaid the original owner reservation and fixed publication.
    // Its grammar is placed exactly once before any owner address is retained.
    let grammar = Box::new(grammar);
    let permit = RuntimeOwnerPublicationPermit { _private: () };
    admission
        .bind_owner(&permit, grammar.as_ref())
        .map_err(RuntimeGrammarError::Publication)?;
    Ok(CompiledRuntimeGrammar { grammar, admission })
}
