//! Exact source-bound ancestor programs for explicit parser and terminal calls.
//! Handwritten composition adapter; recognition remains in the shared terminal engine.
use super::{
    Expr, Grammar, Kind, Rule, Terminal,
    terminals::{Expression, Operation},
};
use intersect_model::definitions::bundle::SourceSpan;
use intersect_model::definitions::schema::Diagnostic;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default)]
pub(super) struct Registry {
    contexts: BTreeMap<String, Context>,
}
struct Context {
    parent: Option<String>,
    names: BTreeSet<String>,
    terminals: BTreeMap<String, Terminal>,
    rules: BTreeMap<String, Rule>,
    spans: BTreeMap<String, SourceSpan>,
}
impl Registry {
    pub(super) fn retain(&mut self, grammar: &Grammar, parent: Option<String>) {
        self.contexts.insert(
            grammar.name.clone(),
            Context {
                parent,
                names: grammar
                    .rules
                    .keys()
                    .chain(grammar.terminals.keys())
                    .cloned()
                    .collect(),
                terminals: grammar.terminals.clone(),
                rules: grammar.rules.clone(),
                spans: grammar.declaration_sources.clone(),
            },
        );
    }
    fn has_ordinary_super(&self, context: &str) -> bool {
        let mut cursor = Some(context);
        while let Some(name) = cursor {
            let Some(c) = self.contexts.get(name) else {
                return false;
            };
            if c.names.contains("super") {
                return true;
            }
            cursor = c.parent.as_deref();
        }
        false
    }
    fn target(
        &self,
        context: &str,
        current: &str,
        spelling: &str,
    ) -> Result<(String, String), String> {
        let parts: Vec<_> = spelling.split("::").collect();
        let (qualifier, rule) = if parts.len() == 1 {
            ("super".to_owned(), current)
        } else {
            (parts[..parts.len() - 1].join("."), parts[parts.len() - 1])
        };
        let mut cursor = Some(context.to_owned());
        if qualifier == "super" {
            cursor = self.contexts.get(context).and_then(|c| c.parent.clone());
        }
        while let Some(name) = cursor {
            let c = self
                .contexts
                .get(&name)
                .ok_or_else(|| "Missing grammar source context".to_owned())?;
            let selected = qualifier == "super"
                || qualifier == name
                || name.rsplit('.').next() == Some(qualifier.as_str());
            if selected && c.names.contains(rule) {
                return Ok((name, rule.to_owned()));
            }
            cursor = c.parent.clone();
        }
        Err(format!(
            "Unknown explicit rule call {spelling} in {context}"
        ))
    }
    fn context_for(&self, name: &str, span: &SourceSpan) -> Option<&str> {
        self.contexts.iter().find_map(|(context, c)| {
            c.spans
                .get(name)
                .filter(|s| s.source == span.source && s.start == span.start && s.end == span.end)
                .map(|_| context.as_str())
        })
    }
    pub(super) fn prepare(&self, grammar: &mut Grammar) -> Result<(), Diagnostic> {
        let mut aliases = BTreeMap::new();
        let mut spans = BTreeMap::new();
        let mut pending = Vec::new();
        let mut parser_aliases = BTreeMap::new();
        let mut active = BTreeMap::new();
        for (name, terminal) in &grammar.terminals {
            active.insert(terminal.identity.clone(), name.clone());
        }
        for name in grammar.rules.keys() {
            if let Some(context) = grammar
                .declaration_sources
                .get(name)
                .and_then(|s| self.context_for(name, s))
            {
                active.insert(format!("{context}::{name}"), name.clone());
            }
        }
        if grammar.profile.has_explicit_parser_calls() {
            for (name, rule) in &mut grammar.rules {
                let span = grammar.declaration_sources.get(name).ok_or_else(|| {
                    Diagnostic::new("xtext", 0, 1, "Missing parser declaration span")
                })?;
                let context = self.context_for(name, span).ok_or_else(|| {
                    Diagnostic::new(
                        &span.source.uri,
                        span.start,
                        span.end,
                        "Missing parser declaration context",
                    )
                    .with_span(span.clone())
                })?;
                self.rewrite_parser(&mut rule.body, context, name, &active, &mut pending)
                    .map_err(|e| Diagnostic::from_span(span.clone(), e))?;
            }
        }
        for terminal in grammar.terminals.values_mut() {
            let (context, current) = terminal.identity.rsplit_once("::").ok_or_else(|| {
                Diagnostic::new(
                    "xtext",
                    terminal.start,
                    terminal.end,
                    "Missing terminal declaration identity",
                )
            })?;
            self.rewrite(&mut terminal.body, context, current, &mut pending)
                .map_err(|e| {
                    let span = self
                        .contexts
                        .get(context)
                        .and_then(|c| c.spans.get(current));
                    span.map_or_else(
                        || Diagnostic::new("xtext", terminal.start, terminal.end, &e),
                        |s| Diagnostic::from_span(s.clone(), &e),
                    )
                })?;
        }
        let mut visited = BTreeSet::new();
        while let Some((context, current, parser_use)) = pending.pop() {
            let key = format!("@{context}::{current}");
            if !visited.insert((key.clone(), parser_use)) {
                continue;
            }
            let c = self
                .contexts
                .get(&context)
                .ok_or_else(|| Diagnostic::new("xtext", 0, 1, "Missing retained grammar"))?;
            let source = c.spans.get(&current).ok_or_else(|| {
                Diagnostic::new("xtext", 0, 1, "Missing retained declaration span")
            })?;
            let error = |e: String| Diagnostic::from_span(source.clone(), e);
            if let Some(original) = c.terminals.get(&current) {
                let mut terminal = original.clone();
                let visible =
                    parser_use || aliases.get(&key).is_some_and(|t: &Terminal| !t.fragment);
                terminal.fragment = original.fragment || !visible;
                self.rewrite(&mut terminal.body, &context, &current, &mut pending)
                    .map_err(error)?;
                if visible && !grammar.terminal_order.contains(&key) {
                    grammar.terminal_order.push(key.clone());
                }
                aliases.insert(key.clone(), terminal);
            } else if grammar.profile.has_explicit_parser_calls() && parser_use {
                let mut rule = c
                    .rules
                    .get(&current)
                    .cloned()
                    .ok_or_else(|| error("Missing retained parser program".into()))?;
                self.rewrite_parser(&mut rule.body, &context, &current, &active, &mut pending)
                    .map_err(error)?;
                parser_aliases.insert(key.clone(), rule);
            } else {
                return Err(error(
                    "Terminal call must select a terminal declaration".into(),
                ));
            }
            spans.insert(key, source.clone());
        }
        grammar.terminals.extend(aliases);
        grammar.rules.extend(parser_aliases);
        grammar.declaration_sources.extend(spans);
        if grammar.profile.has_explicit_parser_calls() {
            // Xtext collects local rules in declaration order, then ancestors.
            // Explicitly retained ancestor tokens keep that position; discovery
            // order through parser expressions must not change lexical priority.
            let mut depth = BTreeMap::new();
            let mut cursor = Some(grammar.name.as_str());
            while let Some(name) = cursor {
                depth.insert(name, depth.len());
                cursor = self.contexts.get(name).and_then(|c| c.parent.as_deref());
            }
            let mut ordered: Vec<_> = grammar
                .terminals
                .iter()
                .filter(|(name, t)| !name.starts_with('@') || !t.fragment)
                .collect();
            ordered.sort_by_key(|(name, t)| {
                (
                    t.identity
                        .rsplit_once("::")
                        .and_then(|(c, _)| depth.get(c))
                        .copied()
                        .unwrap_or(usize::MAX),
                    t.start,
                    *name,
                )
            });
            grammar.terminal_order = ordered.into_iter().map(|(name, _)| name.clone()).collect();
        }
        Ok(())
    }
    fn rewrite_parser(
        &self,
        expr: &mut Expr,
        context: &str,
        current: &str,
        active: &BTreeMap<String, String>,
        pending: &mut Vec<(String, String, bool)>,
    ) -> Result<(), String> {
        match &mut expr.kind {
            Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
                if (name == "super" && !self.has_ordinary_super(context)) || name.contains("::") {
                    let target = self.target(context, current, name)?;
                    let identity = format!("{}::{}", target.0, target.1);
                    if let Some(name_in_scope) = active.get(&identity) {
                        *name = name_in_scope.clone();
                    } else {
                        *name = format!("@{identity}");
                        pending.push((target.0, target.1, true));
                    }
                }
            }
            Kind::Sequence(items) | Kind::Alternative(items) => {
                for item in items {
                    self.rewrite_parser(item, context, current, active, pending)?;
                }
            }
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => {
                self.rewrite_parser(value, context, current, active, pending)?
            }
            _ => {}
        }
        Ok(())
    }
    fn rewrite(
        &self,
        expr: &mut Expression,
        context: &str,
        current: &str,
        pending: &mut Vec<(String, String, bool)>,
    ) -> Result<(), String> {
        match &mut expr.kind {
            Operation::Call { name, body } => {
                if (name == "super" && !self.has_ordinary_super(context)) || name.contains("::") {
                    let target = self.target(context, current, name)?;
                    if !self
                        .contexts
                        .get(&target.0)
                        .is_some_and(|c| c.terminals.contains_key(&target.1))
                    {
                        return Err("Terminal call must select a terminal declaration".into());
                    }
                    *name = format!("@{}::{}", target.0, target.1);
                    *body = None;
                    pending.push((target.0, target.1, false));
                }
            }
            Operation::Sequence(items) | Operation::Alternative(items) => {
                for item in items {
                    self.rewrite(item, context, current, pending)?;
                }
            }
            Operation::Negated { operand, .. } | Operation::UntilOperand { operand, .. } => {
                self.rewrite(operand, context, current, pending)?;
            }
            _ => {}
        }
        Ok(())
    }
}
