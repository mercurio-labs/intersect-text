// Exact B3 field/variant projections. No production layout claim rests on a
// nominal one-byte assumption. These constants are checked by both targets.
use super::*;
use crate::grammar::LexerPrediction;
use crate::grammar::Profile;
use crate::grammar::Rule;
use crate::grammar::Terminal;
use crate::grammar::Token;
use std::mem::{align_of, size_of};
#[allow(dead_code)]
enum LegacyProfile {
    WorkbenchV1,

    NativeV1,

    NativeV2,

    NativeV3,

    NativeV4,

    NativeV5,

    NativeV6,

    NativeV7,

    NativeV8,

    NativeV9,

    NativeV10,

    NativeV11,

    NativeV12,

    NativeV13,

    NativeV14,

    NativeV15,

    NativeV16,

    NativeV17,

    NativeV18,

    NativeV19,

    NativeV20,

    NativeV21,

    NativeV22,

    NativeV23,

    NativeV24,

    NativeV25,

    NativeV26,

    NativeV27,

    NativeV28,

    NativeV29,

    NativeV30,

    NativeV31,

    NativeV32,

    NativeV33,

    NativeV34,

    NativeV35,

    NativeV36,

    NativeV37,

    NativeV38,

    NativeV39,

    NativeV40,

    NativeV41,

    NativeV42,

    NativeV43,

    NativeV44,

    NativeV45,

    NativeV46,

    NativeV47,

    NativeV48,

    NativeV49,

    NativeV50,

    NativeV51,

    NativeV52,

    NativeV53,

    NativeV54,

    NativeV55,

    NativeV56,

    NativeV57,

    NativeV58,

    NativeV59,

    NativeV60,

    NativeV61,

    NativeV62,

    NativeV63,

    NativeV64,

    NativeV65,

    NativeV66,

    NativeV67,
}
#[allow(dead_code)]
enum LegacyKind {
    Action(String),

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
        value: Box<LegacyExpr>,
    },

    BooleanAssign {
        feature: String,
        value: Box<LegacyExpr>,
    },
    Sequence(Vec<LegacyExpr>),
    Alternative(Vec<LegacyExpr>),
}
#[allow(dead_code)]
struct LegacyExpr {
    pub original_range: Option<std::ops::Range<usize>>,

    pub first_set_predicated: bool,

    pub prediction_path: Option<String>,
    pub kind: LegacyKind,
    pub cardinality: char,
}
#[allow(dead_code)]
struct LegacyParser<'policy> {
    capture_origins: bool,
    explicit_local_scope: bool,
    package_imports: std::rc::Rc<crate::grammar::PackageImports<'policy>>,
    allow_first_predicates: bool,
    allow_constant_guards: bool,
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    aliases: BTreeMap<String, Vec<String>>,
    metamodel_imports: bool,
    hidden: Option<Vec<String>>,
}
#[allow(dead_code)]
struct LegacyGrammar {
    pub parameterized_rules: BTreeSet<String>,

    pub final_rules: BTreeSet<String>,

    pub computed_settings: Vec<intersect_model::definitions::setting::ComputedSettingContract>,
    pub profile: LegacyProfile,
    pub name: String,
    pub entry: String,
    pub rules: BTreeMap<String, Rule>,

    pub terminals: BTreeMap<String, Terminal>,

    pub hidden: Option<Vec<String>>,

    pub lexical_prediction: Option<LexerPrediction>,

    pub parser_prediction: Option<LegacyParserPrediction>,

    pub declaration_sources: BTreeMap<String, intersect_model::definitions::bundle::SourceSpan>,

    pub terminal_order: Vec<String>,

    pub shadowed_declarations: BTreeMap<String, Vec<intersect_model::definitions::bundle::SourceSpan>>,
}
#[allow(dead_code)]
enum LegacyEdge {
    Epsilon(usize),
    Action(usize),
    First { path: String, target: usize },
    Token(PredictionSymbol, usize),
    Call { target: usize, follow: usize },
}
#[allow(dead_code)]
struct LegacyConfiguration {
    state: usize,
    returns: Vec<usize>,
    alternative: usize,
    visible: bool,
    guarded: bool,
}
#[allow(dead_code)]
struct LegacyParserPrediction {
    allow_empty_choices: bool,
    states: Vec<State>,
    rules: BTreeMap<String, (usize, usize)>,
    decisions: BTreeMap<String, Decision>,
    first_guards: BTreeMap<String, BTreeSet<PredictionSymbol>>,

    recursive_decisions: BTreeSet<String>,
}
#[allow(dead_code)]
struct LegacyState {
    rule_stop: bool,
    accepting: bool,
    edges: Vec<Edge>,
    follow: Vec<usize>,
}
#[allow(dead_code)]
struct LegacyDecision {
    entries: Vec<usize>,
    exit: Option<usize>,
}
const fn layout_pairs() -> [(usize, usize); 10] {
    [
        (size_of::<Profile>(), size_of::<LegacyProfile>()),
        (size_of::<Kind>(), size_of::<LegacyKind>()),
        (size_of::<Expr>(), size_of::<LegacyExpr>()),
        (size_of::<Edge>(), size_of::<LegacyEdge>()),
        (
            size_of::<ParserPrediction>() - size_of::<Option<Box<runtime_guarded::Requirement>>>(),
            size_of::<LegacyParserPrediction>(),
        ),
        (
            size_of::<Grammar>()
                - size_of::<Option<Box<runtime_guarded::Requirement>>>()
                - size_of::<Option<usize>>()
                - size_of::<crate::grammar::GrammarAnalysisWorkBudget>(),
            size_of::<LegacyGrammar>(),
        ),
        (
            // NativeV82 source-only lowering adds one explicit optional owner.
            // Keep the frozen declaration unchanged and account for this growth.
            size_of::<crate::grammar::Parser<'static>>()
                - size_of::<Option<Box<crate::grammar::unordered::ParserState>>>(),
            size_of::<LegacyParser<'static>>(),
        ),
        (size_of::<Configuration>(), size_of::<LegacyConfiguration>()),
        (size_of::<State>(), size_of::<LegacyState>()),
        (size_of::<Decision>(), size_of::<LegacyDecision>()),
    ]
}
const _: () = {
    assert!(size_of::<CompletionGuard>() == size_of::<&str>());
    assert!(align_of::<CompletionGuard>() == align_of::<&str>());
    // Explicit typed runtime requirement is the sole new field. Default None
    // allocates nothing; it is NOT falsely counted as an exact old holder.
    assert!(
        size_of::<ParserPrediction>()
            == size_of::<LegacyParserPrediction>()
                + size_of::<Option<Box<runtime_guarded::Requirement>>>()
    );
    // Grammar embeds Option<ParserPrediction>; account for its transitive
    // holder growth too, rather than reusing the new program in an old type.
    // Grammar's own new field is the prediction call depth its analysis
    // compiles for (`Grammar::call_depth`, not serialized), plus the explicit
    // grammar-analysis work grant (`Grammar::analysis_work_budget`, not serialized).
    assert!(
        size_of::<Grammar>()
            == size_of::<LegacyGrammar>()
                + size_of::<Option<Box<runtime_guarded::Requirement>>>()
                + size_of::<Option<usize>>()
                + size_of::<crate::grammar::GrammarAnalysisWorkBudget>()
    );
    let pairs = layout_pairs();
    let mut i = 0;
    while i < pairs.len() {
        assert!(pairs[i].0 == pairs[i].1);
        i += 1;
    }
    assert!(align_of::<Kind>() == align_of::<LegacyKind>());
    assert!(align_of::<Expr>() == align_of::<LegacyExpr>());
    assert!(align_of::<Edge>() == align_of::<LegacyEdge>());
};
#[test]
fn v68_native_layouts_equal_exact_legacy_field_and_variant_projections() {
    println!(
        "V68_LEGACY_PROJECTIONS_AND_TYPED_RUNTIME_REQUIREMENT {:?} marker_bytes={} grammar_old={} grammar_new={} program_old={} program_new={}",
        layout_pairs(),
        size_of::<Option<Box<runtime_guarded::Requirement>>>(),
        size_of::<LegacyGrammar>(),
        size_of::<Grammar>(),
        size_of::<LegacyParserPrediction>(),
        size_of::<ParserPrediction>()
    );
    for (actual, old) in layout_pairs() {
        assert_eq!(actual, old);
    }
}
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn v68_layout_value(index: usize) -> usize {
    layout_pairs().get(index).map_or(usize::MAX, |pair| pair.0)
}
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn v68_layout_proof() -> i32 {
    i32::from(layout_pairs().iter().any(|(actual, old)| actual != old))
}

// This is a deliberate typed requirement field, not an unchanged-holder claim.
#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(size_of::<LegacyParserPrediction>() == 128);
    assert!(size_of::<ParserPrediction>() == 136);
};
#[cfg(target_pointer_width = "32")]
const _: () = {
    assert!(size_of::<LegacyParserPrediction>() == 64);
    assert!(size_of::<ParserPrediction>() == 68);
};
