//! Source-bound syntax import. Syntax admission does not confer executable support.
//! Handwritten against the pinned Xtext 2.38 grammar; no generated/Java dependency.
use super::{Parser, SourceLimits, Token, lex_syntax};
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::schema::Diagnostic;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Exact Xtext/EString UTF-16 units, including isolated surrogates.
/// Consumers requiring Unicode scalar strings must explicitly request conversion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SyntaxString(pub Vec<u16>);
impl SyntaxString {
    pub fn to_scalar_string(&self) -> Result<String, String> {
        String::from_utf16(&self.0)
            .map_err(|_| "Syntax string contains an unpaired UTF-16 surrogate".into())
    }
}

/// Decode exactly one complete Xtext string literal using the syntax importer's
/// existing UTF-16 escape authority. This preserves isolated surrogate units;
/// consumers must explicitly choose a compatible value representation.
pub fn decode_string_literal(literal: &str) -> Result<SyntaxString, String> {
    let mut scanned = lex_syntax(literal).map_err(|(_, message)| message)?;
    if scanned.tokens.len() != 1
        || !scanned
            .tokens
            .first()
            .is_some_and(|token| token.quoted && token.start == 0 && token.end == literal.len())
    {
        return Err("Expected one complete STRING literal".into());
    }
    scanned
        .strings
        .remove(&0)
        .map(SyntaxString)
        .ok_or_else(|| "Missing STRING literal value".into())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Located<T> {
    pub start: usize,
    pub end: usize,
    pub value: T,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Syntax {
    /// Grammar node span, excluding leading/trailing hidden trivia.
    pub start: usize,
    pub end: usize,
    /// Full source envelope is 0..source_bytes, bound by the exact source digest.
    pub source_bytes: usize,
    pub source: SourceId,
    pub name: Located<String>,
    pub parents: Vec<Located<String>>,
    pub hidden: Option<Vec<Located<String>>>,
    pub metamodels: Vec<Located<Metamodel>>,
    pub rules: Vec<Located<Rule>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metamodel {
    pub generated_name: Option<Located<String>>,
    pub uri: Located<SyntaxString>,
    pub alias: Option<Located<String>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleKind {
    Parser,
    Fragment,
    Terminal,
    TerminalFragment,
    Enum,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub annotations: Vec<Located<String>>,
    pub kind: RuleKind,
    pub name: Located<String>,
    pub parameters: Option<Vec<Located<String>>>,
    pub return_type: Option<Located<String>>,
    pub wildcard: bool,
    pub hidden: Option<Vec<Located<String>>>,
    pub body: Element,
}
pub type Element = Located<ElementKind>;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    Keyword(SyntaxString),
    Call {
        rule: Located<String>,
        arguments: Option<Vec<Located<Argument>>>,
    },
    Sequence(Vec<Element>),
    Alternatives(Vec<Element>),
    Unordered(Vec<Element>),
    Group(Box<Element>),
    Cardinality {
        element: Box<Element>,
        operator: char,
    },
    Predicate {
        first_token: bool,
        element: Box<Element>,
    },
    Guard {
        condition: Box<Element>,
        elements: Vec<Element>,
    },
    Assignment {
        feature: Located<String>,
        operator: String,
        value: Box<Element>,
    },
    Action {
        ty: Located<String>,
        feature: Option<Located<String>>,
        operator: Option<String>,
    },
    CrossReference {
        ty: Located<String>,
        spelling: Option<Box<Element>>,
    },
    Negated(Box<Element>),
    Until(Box<Element>),
    Range {
        left: Box<Element>,
        right: Box<Element>,
    },
    Wildcard,
    Eof,
    EnumLiteral {
        name: Located<String>,
        keyword: Option<Box<Element>>,
    },
    ConditionLiteral(bool),
    Parameter(String),
    ConditionNot(Box<Element>),
    ConditionAnd(Vec<Element>),
    ConditionOr(Vec<Element>),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Argument {
    pub parameter: Option<Located<String>>,
    pub value: Element,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Parser,
    Assignable,
    Terminal,
    Enum,
}
struct Reader {
    strings: BTreeMap<usize, Vec<u16>>,
    parser: Parser<'static>,
    depth: usize,
}
impl Reader {
    fn at(&self) -> usize {
        self.parser.tokens.get(self.parser.pos).map_or_else(
            || self.parser.tokens.last().map_or(0, |t| t.end),
            |t| t.start,
        )
    }
    fn end(&self) -> usize {
        self.parser
            .tokens
            .get(self.parser.pos.saturating_sub(1))
            .map_or(0, |t| t.end)
    }
    fn located<T>(&self, start: usize, value: T) -> Located<T> {
        Located {
            start,
            end: self.end(),
            value,
        }
    }
    fn peek(&self, s: &str) -> bool {
        self.parser.peek(s)
    }
    fn eat(&mut self, s: &str) -> bool {
        self.parser.eat(s)
    }
    fn require(&mut self, s: &str) -> Result<(), String> {
        self.parser.require(s)
    }
    fn token(&mut self) -> Result<Token, String> {
        self.parser.take()
    }
    fn op(&mut self, s: &str) -> bool {
        let mut end = None;
        for (offset, ch) in s.chars().enumerate() {
            let Some(t) = self.parser.tokens.get(self.parser.pos + offset) else {
                return false;
            };
            if t.quoted || t.text != ch.to_string() || end.is_some_and(|e| e != t.start) {
                return false;
            }
            end = Some(t.end);
        }
        self.parser.pos += s.chars().count();
        true
    }
    fn id(&mut self) -> Result<Located<String>, String> {
        let start = self.at();
        let token = self.token()?;
        let escaped = !token.quoted && token.text == "^";
        let token = self.parser.identifier_token(token)?;
        let mut chars = token.text.chars();
        if token.quoted
            || !chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("Expected syntax identifier".into());
        }
        if !escaped
            && matches!(
                token.text.as_str(),
                "grammar"
                    | "with"
                    | "hidden"
                    | "generate"
                    | "as"
                    | "import"
                    | "fragment"
                    | "returns"
                    | "terminal"
                    | "enum"
                    | "current"
                    | "EOF"
            )
        {
            return Err("Reserved syntax identifier requires a caret escape".into());
        }
        Ok(self.located(start, token.text))
    }
    fn qualified(&mut self, separator: &str) -> Result<Located<String>, String> {
        let mut name = self.id()?;
        while if separator == "::" {
            self.op("::")
        } else {
            self.eat(separator)
        } {
            name.value.push_str(separator);
            name.value.push_str(&self.id()?.value);
            name.end = self.end();
        }
        Ok(name)
    }
    fn plain_id(&mut self) -> Result<Located<String>, String> {
        if self.peek("true") || self.peek("false") {
            return Err("Boolean keyword requires an escaped identifier here".into());
        }
        self.id()
    }
    fn ty(&mut self) -> Result<Located<String>, String> {
        let mut name = self.plain_id()?;
        if self.op("::") {
            name.value.push_str("::");
            name.value.push_str(&self.plain_id()?.value);
            name.end = self.end();
        }
        Ok(name)
    }
    fn string(&mut self) -> Result<Located<SyntaxString>, String> {
        let t = self.token()?;
        if !t.quoted {
            return Err("Expected string literal".into());
        }
        Ok(Located {
            start: t.start,
            end: t.end,
            value: SyntaxString(
                self.strings
                    .remove(&t.start)
                    .ok_or("Missing syntax string value")?,
            ),
        })
    }
    fn hidden(&mut self) -> Result<Option<Vec<Located<String>>>, String> {
        if !self.eat("hidden") {
            return Ok(None);
        }
        self.require("(")?;
        let mut names = Vec::new();
        if !self.peek(")") {
            loop {
                names.push(self.qualified("::")?);
                if !self.eat(",") {
                    break;
                }
            }
        }
        self.require(")")?;
        Ok(Some(names))
    }
    fn expression(&mut self, mode: Mode) -> Result<Element, String> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("Syntax nesting budget exceeded".into());
        }
        let start = self.at();
        let mut items = vec![self.unordered(mode)?];
        while self.eat("|") {
            items.push(self.unordered(mode)?);
        }
        self.depth -= 1;
        if items.len() == 1 {
            Ok(items.remove(0))
        } else {
            Ok(self.located(start, ElementKind::Alternatives(items)))
        }
    }
    fn unordered(&mut self, mode: Mode) -> Result<Element, String> {
        let start = self.at();
        if mode == Mode::Parser && self.eat("<") {
            let condition = self.condition()?;
            self.require(">")?;
            let mut elements = Vec::new();
            while !self.stop() {
                elements.push(self.atom(mode)?);
            }
            if elements.is_empty() {
                return Err("Guard requires elements".into());
            }
            return Ok(self.located(
                start,
                ElementKind::Guard {
                    condition: Box::new(condition),
                    elements,
                },
            ));
        }
        let mut items = vec![self.sequence(mode)?];
        if mode == Mode::Parser {
            while self.eat("&") {
                items.push(self.sequence(mode)?);
            }
        }
        if items.len() == 1 {
            Ok(items.remove(0))
        } else {
            Ok(self.located(start, ElementKind::Unordered(items)))
        }
    }
    fn stop(&self) -> bool {
        self.parser.pos == self.parser.tokens.len()
            || [";", ")", "|", "&"].iter().any(|s| self.peek(s))
    }
    fn sequence(&mut self, mode: Mode) -> Result<Element, String> {
        let start = self.at();
        let mut items = vec![self.atom(mode)?];
        if !matches!(mode, Mode::Assignable | Mode::Enum) {
            while !self.stop() {
                items.push(self.atom(mode)?);
            }
        }
        if items.len() == 1 {
            Ok(items.remove(0))
        } else {
            Ok(self.located(start, ElementKind::Sequence(items)))
        }
    }
    fn atom(&mut self, mode: Mode) -> Result<Element, String> {
        let start = self.at();
        let first_token = self.op("->");
        let predicate = mode == Mode::Parser && (first_token || self.op("=>"));
        let mut element = if predicate {
            let inner = self.base(mode)?;
            if matches!(inner.value, ElementKind::Action { .. }) {
                return Err("Actions cannot carry predicates".into());
            }
            self.located(
                start,
                ElementKind::Predicate {
                    first_token,
                    element: Box::new(inner),
                },
            )
        } else if first_token {
            if mode != Mode::Terminal {
                return Err("Predicate is not assignable".into());
            }
            let inner = self.terminal_element()?;
            self.located(start, ElementKind::Until(Box::new(inner)))
        } else {
            self.base(mode)?
        };
        if !matches!(mode, Mode::Assignable | Mode::Enum)
            && !matches!(element.value, ElementKind::Action { .. })
        {
            for operator in ['?', '*', '+'] {
                if self.eat(&operator.to_string()) {
                    element = self.located(
                        start,
                        ElementKind::Cardinality {
                            element: Box::new(element),
                            operator,
                        },
                    );
                    break;
                }
            }
        }
        Ok(element)
    }
    fn terminal_element(&mut self) -> Result<Element, String> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("Syntax nesting budget exceeded".into());
        }
        let result = if self.op("->") {
            let start = self.parser.tokens[self.parser.pos - 2].start;
            let inner = self.terminal_element()?;
            Ok(self.located(start, ElementKind::Until(Box::new(inner))))
        } else {
            self.base(Mode::Terminal)
        };
        self.depth -= 1;
        result
    }
    fn base(&mut self, mode: Mode) -> Result<Element, String> {
        let start = self.at();
        if mode == Mode::Enum {
            let name = self.id()?;
            let keyword = if self.eat("=") {
                let s = self.string()?;
                Some(Box::new(Located {
                    start: s.start,
                    end: s.end,
                    value: ElementKind::Keyword(s.value),
                }))
            } else {
                None
            };
            return Ok(self.located(start, ElementKind::EnumLiteral { name, keyword }));
        }
        if self.eat("(") {
            let inner = self.expression(mode)?;
            self.require(")")?;
            return Ok(self.located(start, ElementKind::Group(Box::new(inner))));
        }
        if mode == Mode::Terminal {
            if self.eat("!") {
                let inner = self.terminal_element()?;
                return Ok(self.located(start, ElementKind::Negated(Box::new(inner))));
            }
            if self.eat(".") {
                return Ok(self.located(start, ElementKind::Wildcard));
            }
            if self.eat("EOF") {
                return Ok(self.located(start, ElementKind::Eof));
            }
        }
        if self
            .parser
            .tokens
            .get(self.parser.pos)
            .is_some_and(|t| t.quoted)
        {
            let s = self.string()?;
            let left = self.located(start, ElementKind::Keyword(s.value));
            if mode == Mode::Terminal && self.op("..") {
                let s = self.string()?;
                let right = Located {
                    start: s.start,
                    end: s.end,
                    value: ElementKind::Keyword(s.value),
                };
                return Ok(self.located(
                    start,
                    ElementKind::Range {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                ));
            }
            return Ok(left);
        }
        if mode == Mode::Parser && self.eat("{") {
            let ty = self.ty()?;
            let mut feature = None;
            let mut operator = None;
            if self.eat(".") {
                feature = Some(self.id()?);
                operator = Some(
                    if self.op("+=") {
                        "+="
                    } else {
                        self.require("=")?;
                        "="
                    }
                    .into(),
                );
                self.require("current")?;
            }
            self.require("}")?;
            return Ok(self.located(
                start,
                ElementKind::Action {
                    ty,
                    feature,
                    operator,
                },
            ));
        }
        if mode == Mode::Assignable && self.eat("[") {
            let ty = self.ty()?;
            let spelling = if self.eat("|") {
                let element = self.base(Mode::Assignable)?;
                if !matches!(
                    element.value,
                    ElementKind::Keyword(_) | ElementKind::Call { .. }
                ) {
                    return Err("Cross-reference spelling requires keyword or call".into());
                }
                Some(Box::new(element))
            } else {
                None
            };
            self.require("]")?;
            return Ok(self.located(start, ElementKind::CrossReference { ty, spelling }));
        }
        let rule = self.qualified("::")?;
        if mode == Mode::Parser {
            let operator = if self.op("+=") {
                Some("+=")
            } else if self.op("?=") {
                Some("?=")
            } else if self.eat("=") {
                Some("=")
            } else {
                None
            };
            if let Some(operator) = operator {
                if rule.value.contains("::") {
                    return Err("Assignment feature cannot be qualified".into());
                }
                let value = self.atom(Mode::Assignable)?;
                return Ok(self.located(
                    start,
                    ElementKind::Assignment {
                        feature: rule,
                        operator: operator.into(),
                        value: Box::new(value),
                    },
                ));
            }
        }
        let arguments = if mode != Mode::Terminal && self.eat("<") {
            let mut args = Vec::new();
            loop {
                let argument_start = self.at();
                let saved = self.parser.pos;
                let parameter = if let Ok(name) = self.plain_id() {
                    if self.eat("=") {
                        Some(name)
                    } else {
                        self.parser.pos = saved;
                        None
                    }
                } else {
                    self.parser.pos = saved;
                    None
                };
                let value = self.condition()?;
                args.push(self.located(argument_start, Argument { parameter, value }));
                if !self.eat(",") {
                    break;
                }
            }
            self.require(">")?;
            Some(args)
        } else {
            None
        };
        Ok(self.located(start, ElementKind::Call { rule, arguments }))
    }
    fn condition(&mut self) -> Result<Element, String> {
        let start = self.at();
        let mut items = vec![self.conjunction()?];
        while self.eat("|") {
            items.push(self.conjunction()?);
        }
        if items.len() == 1 {
            Ok(items.remove(0))
        } else {
            Ok(self.located(start, ElementKind::ConditionOr(items)))
        }
    }
    fn conjunction(&mut self) -> Result<Element, String> {
        let start = self.at();
        let mut items = vec![self.condition_atom()?];
        while self.eat("&") {
            items.push(self.condition_atom()?);
        }
        if items.len() == 1 {
            Ok(items.remove(0))
        } else {
            Ok(self.located(start, ElementKind::ConditionAnd(items)))
        }
    }
    fn condition_atom(&mut self) -> Result<Element, String> {
        self.depth += 1;
        if self.depth > 64 {
            return Err("Syntax nesting budget exceeded".into());
        }
        let start = self.at();
        let value = if self.eat("!") {
            ElementKind::ConditionNot(Box::new(self.condition_atom()?))
        } else if self.eat("(") {
            let inner = self.condition()?;
            self.require(")")?;
            ElementKind::Group(Box::new(inner))
        } else if self.eat("true") {
            ElementKind::ConditionLiteral(true)
        } else if self.eat("false") {
            ElementKind::ConditionLiteral(false)
        } else {
            ElementKind::Parameter(self.id()?.value)
        };
        self.depth -= 1;
        Ok(self.located(start, value))
    }
    fn rule(&mut self) -> Result<Located<Rule>, String> {
        let start = self.at();
        let mut annotations = Vec::new();
        while self.peek("@") {
            let start = self.at();
            self.require("@")?;
            let name = self.plain_id()?;
            annotations.push(self.located(start, name.value));
        }
        let kind = if self.eat("terminal") {
            if self.eat("fragment") {
                RuleKind::TerminalFragment
            } else {
                RuleKind::Terminal
            }
        } else if self.eat("enum") {
            RuleKind::Enum
        } else if self.eat("fragment") {
            RuleKind::Fragment
        } else {
            RuleKind::Parser
        };
        let name = self.id()?;
        let parser = matches!(kind, RuleKind::Parser | RuleKind::Fragment);
        let parameters = if parser && self.eat("<") {
            let mut names = Vec::new();
            if !self.peek(">") {
                loop {
                    names.push(self.plain_id()?);
                    if !self.eat(",") {
                        break;
                    }
                }
            }
            self.require(">")?;
            Some(names)
        } else {
            None
        };
        let wildcard = kind == RuleKind::Fragment && self.eat("*");
        let return_type = if !wildcard && kind != RuleKind::TerminalFragment && self.eat("returns")
        {
            Some(self.ty()?)
        } else {
            None
        };
        let hidden = if parser { self.hidden()? } else { None };
        self.require(":")?;
        let mode = match kind {
            RuleKind::Terminal | RuleKind::TerminalFragment => Mode::Terminal,
            RuleKind::Enum => Mode::Enum,
            _ => Mode::Parser,
        };
        let body = self.expression(mode)?;
        self.require(";")?;
        Ok(self.located(
            start,
            Rule {
                annotations,
                kind,
                name,
                parameters,
                return_type,
                wildcard,
                hidden,
                body,
            },
        ))
    }
}
// One lexical reader for complete syntax import and header identity selection.
fn reader(source: &DefinitionSource, limits: SourceLimits) -> Result<Reader, Diagnostic> {
    let error = |at, message| Diagnostic::at_source(&source.id, at, at.saturating_add(1), message);
    source.validate().map_err(|e| error(0, e))?;
    if source.kind != SourceKind::Xtext {
        return Err(error(0, "Expected Xtext source".into()));
    }
    if source.text.len() > limits.source_bytes {
        return Err(error(0, "Definition size budget exceeded".into()));
    }
    let tokens = lex_syntax(&source.text).map_err(|(at, e)| error(at, e))?;
    Ok(Reader {
        strings: tokens.strings,
        parser: Parser {
            capture_origins: false,
            unordered: None,
            explicit_local_scope: false,
            package_imports: Default::default(),
            allow_first_predicates: false,
            allow_constant_guards: false,
            tokens: tokens.tokens,
            pos: 0,
            depth: 0,
            aliases: BTreeMap::new(),
            metamodel_imports: false,
            hidden: None,
        },
        depth: 0,
    })
}

/// Read only the source-bound grammar identity, using the same lexical reader.
/// This does not admit or validate the grammar body.
pub fn grammar_name(
    source: &DefinitionSource,
    limits: SourceLimits,
) -> Result<Located<String>, Diagnostic> {
    let mut r = reader(source, limits)?;
    let result = (|| {
        r.require("grammar")?;
        r.qualified(".")
    })();
    result.map_err(|e| Diagnostic::at_source(&source.id, r.at(), r.at().saturating_add(1), e))
}

/// Import complete syntax with exact source identity. Runtime admission remains
/// a separate compiler contract; errors return no syntax tree.
pub fn parse(source: &DefinitionSource, limits: SourceLimits) -> Result<Syntax, Diagnostic> {
    let error = |at: usize, message: String| {
        Diagnostic::at_source(&source.id, at, at.saturating_add(1), message)
    };
    let mut r = reader(source, limits)?;
    let result = (|| {
        let grammar_start = r.at();
        r.require("grammar")?;
        let name = r.qualified(".")?;
        let mut parents = Vec::new();
        if r.eat("with") {
            loop {
                parents.push(r.qualified(".")?);
                if !r.eat(",") {
                    break;
                }
            }
        }
        let hidden = r.hidden()?;
        let mut metamodels = Vec::new();
        while r.peek("generate") || r.peek("import") {
            let start = r.at();
            let generated_name = if r.eat("generate") {
                Some(r.id()?)
            } else {
                r.require("import")?;
                None
            };
            let uri = r.string()?;
            let alias = if r.eat("as") { Some(r.id()?) } else { None };
            metamodels.push(r.located(
                start,
                Metamodel {
                    generated_name,
                    uri,
                    alias,
                },
            ));
        }
        let mut rules = Vec::new();
        while r.parser.pos < r.parser.tokens.len() {
            if rules.len() >= limits.declarations {
                return Err("Declaration budget exceeded".into());
            }
            rules.push(r.rule()?);
        }
        if rules.is_empty() {
            return Err("Grammar requires a rule".into());
        }
        Ok(Syntax {
            start: grammar_start,
            end: r.end(),
            source_bytes: source.text.len(),
            source: source.id.clone(),
            name,
            parents,
            hidden,
            metamodels,
            rules,
        })
    })();
    result.map_err(|e| error(r.at(), e))
}
