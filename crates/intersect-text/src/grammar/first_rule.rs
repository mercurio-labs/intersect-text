//! Handwritten contextual first-rule admission shared by native source adapters.
//! Categories come from freshly resolved classifiers, never serialized approval.
use super::syntax::{Located, Rule, RuleKind, Syntax};
use intersect_model::definitions::bundle::SourceSpan;
use intersect_model::definitions::schema::Diagnostic;

/// Source-local classification retained while a used-grammar closure is composed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub contains_non_datatype_parser: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    NonParser,
    Parser { datatype: bool, fragment: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Issue {
    ParserRequired,
    DatatypeFirst,
    FragmentFirst,
}
impl Issue {
    pub fn message(self) -> &'static str {
        match self {
            Self::ParserRequired => "The first rule must be a parser rule.",
            Self::DatatypeFirst => "The first rule must be a parser rule, but is a data type rule.",
            Self::FragmentFirst => "The first rule must not be a fragment",
        }
    }
}
/// Pinned default-validator branch order. EDataType is tested before fragment.
pub fn check(first: Option<Category>, contains_non_datatype_parser: bool) -> Result<(), Issue> {
    match first {
        Some(Category::NonParser) if contains_non_datatype_parser => Err(Issue::ParserRequired),
        Some(Category::Parser { datatype: true, .. }) if contains_non_datatype_parser => {
            Err(Issue::DatatypeFirst)
        }
        Some(Category::Parser {
            datatype: false,
            fragment: true,
        }) => Err(Issue::FragmentFirst),
        _ => Ok(()),
    }
}
/// Validate authored order with resolved local categories and exact used closure.
/// Terminal/enum declarations are not ParserRule instances, regardless of their
/// runtime representation. Null/wildcard classifiers are not datatypes.
pub fn validate(
    syntax: &Syntax,
    used: Summary,
    mut is_datatype: impl FnMut(&Located<Rule>) -> Result<bool, Diagnostic>,
) -> Result<Summary, Diagnostic> {
    let mut contains = used.contains_non_datatype_parser;
    let mut first = None;
    for (index, rule) in syntax.rules.iter().enumerate() {
        let category = if matches!(rule.value.kind, RuleKind::Parser | RuleKind::Fragment) {
            let datatype = is_datatype(rule)?;
            contains |= !datatype;
            Category::Parser {
                datatype,
                fragment: rule.value.kind == RuleKind::Fragment,
            }
        } else {
            Category::NonParser
        };
        if index == 0 {
            first = Some(category);
        }
    }
    check(first, contains).map_err(|issue| {
        let (start, end) = syntax
            .rules
            .first()
            .map_or((syntax.start, syntax.end), |r| {
                if issue == Issue::FragmentFirst {
                    (r.start, r.value.name.start)
                } else {
                    (r.value.name.start, r.value.name.end)
                }
            });
        Diagnostic::from_span(
            SourceSpan {
                source: syntax.source.clone(),
                start,
                end,
            },
            issue.message(),
        )
    })?;
    Ok(Summary {
        contains_non_datatype_parser: contains,
    })
}

/// The ordinary adapter consumes the existing resolved return-type analysis.
/// Local source order is retained separately from the effective runtime maps.
pub(super) fn validate_compiled(
    syntax: &Syntax,
    grammar: &super::Grammar,
    schema: &intersect_model::definitions::schema::Schema,
    used: Summary,
) -> Result<Summary, Diagnostic> {
    let mut resolved = grammar.clone();
    super::datatype::infer(&mut resolved, schema).map_err(|failure| {
        Diagnostic::at_source(&syntax.source, syntax.start, syntax.end, failure.message)
    })?;
    validate(syntax, used, |rule| {
        resolved
            .rules
            .get(&rule.value.name.value)
            .map(|resolved| resolved.datatype.is_some())
            .ok_or_else(|| {
                Diagnostic::at_source(
                    &syntax.source,
                    rule.start,
                    rule.end,
                    "Missing resolved authored parser category",
                )
            })
    })
}
