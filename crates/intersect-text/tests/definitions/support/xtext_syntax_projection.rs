// Verification projection only: normalize the native syntax AST to upstream containment shapes.
use intersect_text::grammar::syntax::Element;
use intersect_text::grammar::syntax::ElementKind as K;
use intersect_text::grammar::syntax::RuleKind;
use intersect_text::grammar::syntax::Syntax;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Serialize)]
struct Node {
    class: &'static str,
    attributes: BTreeMap<&'static str, Value>,
    children: BTreeMap<&'static str, Vec<Node>>,
    references: BTreeMap<&'static str, Vec<Value>>,
}
#[derive(Clone, Copy, Default)]
struct Flags {
    cardinality: Option<char>,
    predicated: bool,
    first: bool,
}
fn text(s: &str) -> Value {
    json!({"utf16":s.encode_utf16().collect::<Vec<_>>()})
}
fn boolean(value: bool) -> Value {
    Value::String(value.to_string())
}
impl Node {
    fn new(class: &'static str) -> Self {
        Self {
            class,
            attributes: BTreeMap::new(),
            children: BTreeMap::new(),
            references: BTreeMap::new(),
        }
    }
    fn element(class: &'static str, f: Flags) -> Self {
        Self::new(class)
            .attr(
                "cardinality",
                f.cardinality.map_or(Value::Null, |c| text(&c.to_string())),
            )
            .attr("predicated", boolean(f.predicated))
            .attr("firstSetPredicated", boolean(f.first))
    }
    fn attr(mut self, key: &'static str, value: Value) -> Self {
        self.attributes.insert(key, value);
        self
    }
    fn child(mut self, key: &'static str, value: Node) -> Self {
        self.children.entry(key).or_default().push(value);
        self
    }
    fn reference(mut self, key: &'static str, value: Value) -> Self {
        self.references.entry(key).or_default().push(value);
        self
    }
}
fn ty(name: &str) -> Node {
    let mut n = Node::new("TypeRef");
    let classifier = if let Some((alias, classifier)) = name.split_once("::") {
        n = n.reference("metamodel", text(alias));
        classifier
    } else {
        name
    };
    n.reference("classifier", text(classifier))
}
fn element(e: &Element, f: Flags) -> Node {
    match &e.value {
        K::Group(inner) => element(inner, f),
        K::Cardinality {
            element: inner,
            operator,
        } => element(
            inner,
            Flags {
                cardinality: Some(*operator),
                ..f
            },
        ),
        K::Predicate {
            element: inner,
            first_token,
        } => {
            let flags = Flags {
                predicated: !*first_token,
                first: *first_token,
                ..f
            };
            if let K::Group(group) = &inner.value {
                Node::element("Group", flags).child("elements", element(group, Flags::default()))
            } else {
                element(inner, flags)
            }
        }
        K::Keyword(value) => Node::element("Keyword", f).attr("value", json!({"utf16":value.0})),
        K::Call { rule, arguments } => {
            let mut n = Node::element("RuleCall", f)
                .attr("explicitlyCalled", boolean(false))
                .reference("rule", text(&rule.value));
            if let Some(arguments) = arguments {
                for a in arguments {
                    let mut child = Node::new("NamedArgument")
                        .attr("calledByName", boolean(a.value.parameter.is_some()))
                        .child("value", element(&a.value.value, Flags::default()));
                    if let Some(parameter) = &a.value.parameter {
                        child = child.reference("parameter", text(&parameter.value));
                    }
                    n = n.child("arguments", child);
                }
            }
            n
        }
        K::Sequence(items) | K::Alternatives(items) | K::Unordered(items) => {
            let mut n = Node::element(
                match &e.value {
                    K::Sequence(_) => "Group",
                    K::Alternatives(_) => "Alternatives",
                    _ => "UnorderedGroup",
                },
                f,
            );
            for item in items {
                n = n.child("elements", element(item, Flags::default()));
            }
            n
        }
        K::Guard {
            condition,
            elements,
        } => {
            let mut n = Node::element("Group", f)
                .child("guardCondition", element(condition, Flags::default()));
            for item in elements {
                n = n.child("elements", element(item, Flags::default()));
            }
            n
        }
        K::Assignment {
            feature,
            operator,
            value,
        } => Node::element("Assignment", f)
            .attr("feature", text(&feature.value))
            .attr("operator", text(operator))
            .child("terminal", element(value, Flags::default())),
        K::Action {
            ty: name,
            feature,
            operator,
        } => Node::element("Action", f)
            .attr(
                "feature",
                feature.as_ref().map_or(Value::Null, |n| text(&n.value)),
            )
            .attr(
                "operator",
                operator.as_ref().map_or(Value::Null, |n| text(n)),
            )
            .child("type", ty(&name.value)),
        K::CrossReference { ty: name, spelling } => {
            let mut n = Node::element("CrossReference", f).child("type", ty(&name.value));
            if let Some(s) = spelling {
                n = n.child("terminal", element(s, Flags::default()));
            }
            n
        }
        K::Negated(inner) | K::Until(inner) => Node::element(
            if matches!(e.value, K::Negated(_)) {
                "NegatedToken"
            } else {
                "UntilToken"
            },
            f,
        )
        .child("terminal", element(inner, Flags::default())),
        K::Range { left, right } => Node::element("CharacterRange", f)
            .child("left", element(left, Flags::default()))
            .child("right", element(right, Flags::default())),
        K::Wildcard => Node::element("Wildcard", f),
        K::Eof => Node::element("EOF", f),
        K::EnumLiteral { name, keyword } => {
            let mut n = Node::element("EnumLiteralDeclaration", f)
                .reference("enumLiteral", text(&name.value));
            if let Some(k) = keyword {
                n = n.child("literal", element(k, Flags::default()));
            }
            n
        }
        K::ConditionLiteral(value) => Node::new("LiteralCondition").attr("true", boolean(*value)),
        K::Parameter(name) => Node::new("ParameterReference").reference("parameter", text(name)),
        K::ConditionNot(inner) => {
            Node::new("Negation").child("value", element(inner, Flags::default()))
        }
        K::ConditionAnd(items) | K::ConditionOr(items) => {
            let (first, rest) = items.split_first().expect("Nonempty condition chain");
            let mut n = element(first, Flags::default());
            for item in rest {
                n = Node::new(if matches!(e.value, K::ConditionAnd(_)) {
                    "Conjunction"
                } else {
                    "Disjunction"
                })
                .child("left", n)
                .child("right", element(item, Flags::default()));
            }
            n
        }
    }
}
pub fn project(ast: &Syntax) -> Value {
    let mut root = Node::new("Grammar")
        .attr("name", text(&ast.name.value))
        .attr("definesHiddenTokens", boolean(ast.hidden.is_some()));
    for parent in &ast.parents {
        root = root.reference("usedGrammars", text(&parent.value));
    }
    if let Some(hidden) = &ast.hidden {
        for name in hidden {
            root = root.reference("hiddenTokens", text(&name.value));
        }
    }
    for model in &ast.metamodels {
        let mut n = Node::new(if model.value.generated_name.is_some() {
            "GeneratedMetamodel"
        } else {
            "ReferencedMetamodel"
        })
        .attr(
            "alias",
            model
                .value
                .alias
                .as_ref()
                .map_or(Value::Null, |a| text(&a.value)),
        )
        .reference("ePackage", json!({"utf16":model.value.uri.value.0}));
        if let Some(name) = &model.value.generated_name {
            n = n.attr("name", text(&name.value));
        }
        root = root.child("metamodelDeclarations", n);
    }
    for rule in &ast.rules {
        let rule = &rule.value;
        let mut n = Node::new(match rule.kind {
            RuleKind::Parser | RuleKind::Fragment => "ParserRule",
            RuleKind::Terminal | RuleKind::TerminalFragment => "TerminalRule",
            RuleKind::Enum => "EnumRule",
        })
        .attr("name", text(&rule.name.value));
        if matches!(rule.kind, RuleKind::Parser | RuleKind::Fragment) {
            n = n
                .attr("fragment", boolean(rule.kind == RuleKind::Fragment))
                .attr("wildcard", boolean(rule.wildcard))
                .attr("definesHiddenTokens", boolean(rule.hidden.is_some()));
        }
        if matches!(rule.kind, RuleKind::Terminal | RuleKind::TerminalFragment) {
            n = n.attr("fragment", boolean(rule.kind == RuleKind::TerminalFragment));
        }
        if let Some(return_type) = &rule.return_type {
            n = n.child("type", ty(&return_type.value));
        }
        if let Some(parameters) = &rule.parameters {
            for p in parameters {
                n = n.child(
                    "parameters",
                    Node::new("Parameter").attr("name", text(&p.value)),
                );
            }
        }
        if let Some(hidden) = &rule.hidden {
            for h in hidden {
                n = n.reference("hiddenTokens", text(&h.value));
            }
        }
        for a in &rule.annotations {
            n = n.child(
                "annotations",
                Node::new("Annotation").attr("name", text(&a.value)),
            );
        }
        n = n.child("alternatives", element(&rule.body, Flags::default()));
        root = root.child("rules", n);
    }
    serde_json::to_value(root).unwrap()
}
