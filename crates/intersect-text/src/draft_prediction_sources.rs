//! One live prediction request of the guarded runtime parser (handwritten):
//! the decision, its expression and the source it is made in.
use super::*;

pub(crate) struct Request<'a> {
    pub(crate) grammar: &'a grammar::Grammar,
    pub(crate) expression: &'a Expr,
    pub(crate) suffix: &'a str,
    pub(crate) decision: &'a str,
    pub(crate) uri: &'a str,
    pub(crate) hidden: &'a [String],
}
