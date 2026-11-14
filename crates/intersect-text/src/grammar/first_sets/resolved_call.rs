//! A borrowed result of the canonical rule lookup, never serialized authority.
use crate::grammar::Grammar;
use crate::grammar::Rule;

// Fields are private to this module. There is no arbitrary-Rule constructor.
#[derive(Clone, Copy)]
pub(super) struct Resolved<'g> {
    grammar: &'g Grammar,
    name: &'g str,
    rule: &'g Rule,
}

pub(super) fn lookup<'g>(grammar: &'g Grammar, name: &str) -> Option<Resolved<'g>> {
    grammar
        .rules
        .get_key_value(name)
        .map(|(name, rule)| Resolved {
            grammar,
            name: name.as_str(),
            rule,
        })
}
impl<'g> Resolved<'g> {
    pub(super) fn rule(self) -> &'g Rule {
        self.rule
    }
    pub(super) fn for_grammar(self, grammar: &Grammar) -> Option<(&'g str, &'g Rule)> {
        std::ptr::eq(self.grammar, grammar).then_some((self.name, self.rule))
    }
}

#[cfg(test)]
#[path = "resolved_call_tests.rs"]
mod tests;
