//! Syntax import controls. Successful parsing is not runtime qualification.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::SourceLimits;
use intersect_text::grammar::syntax;
use intersect_text::grammar::syntax::ElementKind;
fn parse(text: &str) -> Result<syntax::Syntax, intersect_model::definitions::schema::Diagnostic> {
    syntax::parse(
        &DefinitionSource::new("audit.xtext", SourceKind::Xtext, text),
        SourceLimits::default(),
    )
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn pinned_xtext_grammar_and_terminals_import() {
    for text in [
        include_str!(
            "../../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/Xtext.xtext"
        ),
        include_str!(
            "../../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    ] {
        let ast = parse(text).unwrap();
        assert!(!ast.rules.is_empty());
        for rule in &ast.rules {
            assert!(text[rule.start..rule.end].ends_with(';'));
            assert!(rule.value.body.start >= rule.start);
            assert!(rule.value.body.end < rule.end);
        }
        let json = serde_json::to_string(&ast).unwrap();
        assert_eq!(ast, serde_json::from_str(&json).unwrap());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn complete_headers_and_nonexecuted_constructs_are_retained() {
    let text = r#"grammar sample.^grammar with parent.One, parent.Two hidden(WS, base::COMMENT)
        generate model "urn:model" as m
        import "urn:other"
        @Override @Deprecated Rule<A,B> returns m::Model hidden():
            <A & !B> name=ID | (first?='first' & next+=Other<flag=true>?) | =>('x' 'y') | ->'z';
        fragment Any *: {m::Model} 'a' {m::Model.items+=current};
        terminal fragment END: !('x'|'y') ->('a'|'b');
        enum Choice returns m::Choice: One='one' | Two;
    "#;
    let ast = parse(text).unwrap();
    assert_eq!(ast.parents.len(), 2);
    assert_eq!(ast.metamodels.len(), 2);
    assert_eq!(ast.rules.len(), 4);
    assert_eq!(ast.rules[0].value.annotations.len(), 2);
    assert_eq!(ast.rules[0].value.parameters.as_ref().unwrap().len(), 2);
    assert!(matches!(
        ast.rules[0].value.body.value,
        ElementKind::Alternatives(_)
    ));
    assert!(ast.rules[1].value.wildcard);
    assert_eq!(&text[ast.name.start..ast.name.end], "sample.^grammar");
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn malformed_syntax_and_source_identity_fail_closed() {
    for body in [
        "",
        "Rule:;",
        "Rule: 'x' |;",
        "Rule: ('x';",
        "Rule: name==>ID;",
        "Rule: name=[Model|('x')];",
        "Rule: {Model.x?=current};",
        "terminal T: 'a'..ID;",
        "terminal fragment T returns X: 'x';",
        "Rule<P,>: 'x';",
        "Rule: Other<>;",
        "Rule: <P> ;",
        "enum E: One |;",
        "Rule: 'x'; garbage",
    ] {
        assert!(parse(&format!("grammar test {body}")).is_err(), "{body}");
    }
    let mut source =
        DefinitionSource::new("audit.xtext", SourceKind::Xtext, "grammar test Rule: 'x';");
    source.text.push(' ');
    assert!(syntax::parse(&source, SourceLimits::default()).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn syntax_limits_and_illegal_predicates_reject() {
    for body in [
        "Rule: =>=>ID;",
        "Rule<true>: 'x';",
        "@false Rule: 'x';",
        "Rule: ->{Model};",
        "Rule returns a::b::C: 'x';",
    ] {
        assert!(parse(&format!("grammar test {body}")).is_err(), "{body}");
    }
    let deep = format!(
        "grammar test Rule: {}'x'{};",
        "(".repeat(70),
        ")".repeat(70)
    );
    assert!(parse(&deep).is_err());
    let unary = format!("grammar test terminal T: {}'x';", "!".repeat(70));
    assert!(parse(&unary).is_err());
    let text = "grammar test Rule: 'x';";
    let source = DefinitionSource::new("audit.xtext", SourceKind::Xtext, text);
    assert!(
        syntax::parse(
            &source,
            SourceLimits {
                source_bytes: text.len(),
                declarations: 1
            }
        )
        .is_ok()
    );
    assert!(
        syntax::parse(
            &source,
            SourceLimits {
                source_bytes: text.len() - 1,
                declarations: 1
            }
        )
        .is_err()
    );
    assert!(
        syntax::parse(
            &source,
            SourceLimits {
                source_bytes: text.len(),
                declarations: 0
            }
        )
        .is_err()
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn cached_upstream_syntax_observations() {
    use serde_json::Value;
    fn keywords(value: &Value, out: &mut Vec<(usize, usize, Vec<u16>)>) {
        if let Some(object) = value.as_object() {
            if let Some(text) = object
                .get("value")
                .and_then(|v| v.get("Keyword"))
                .and_then(Value::as_array)
            {
                out.push((
                    object["start"].as_u64().unwrap() as usize,
                    object["end"].as_u64().unwrap() as usize,
                    text.iter().map(|u| u.as_u64().unwrap() as u16).collect(),
                ));
            }
            for value in object.values() {
                keywords(value, out);
            }
        } else if let Some(array) = value.as_array() {
            for value in array {
                keywords(value, out);
            }
        }
    }
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/syntax-ast/cases.json"
    ))
    .unwrap();
    let observations: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/syntax-ast/observations.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 19);
    assert_eq!(cases.len(), observations.len());
    for (case, observation) in cases.iter().zip(&observations) {
        assert_eq!(case["id"], observation["id"]);
        let result = parse(case["source"].as_str().unwrap());
        assert_eq!(
            result.is_err(),
            observation["syntax_errors"].as_bool().unwrap(),
            "{}: {:?}",
            case["id"],
            result
        );
        if let Ok(ast) = result {
            let nodes = observation["nodes"].as_array().unwrap();
            let count = nodes
                .iter()
                .filter(|n| {
                    matches!(
                        n["class"].as_str(),
                        Some("ParserRule" | "TerminalRule" | "EnumRule")
                    )
                })
                .count();
            assert_eq!(ast.rules.len(), count, "{}", case["id"]);
            let mut native = Vec::new();
            keywords(&serde_json::to_value(ast).unwrap(), &mut native);
            let oracle: Vec<_> = nodes
                .iter()
                .filter(|n| n["class"] == "Keyword")
                .map(|n| {
                    (
                        n["start"].as_u64().unwrap() as usize,
                        n["end"].as_u64().unwrap() as usize,
                        n["attributes"]["value"]["utf16"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|u| u.as_u64().unwrap() as u16)
                            .collect::<Vec<_>>(),
                    )
                })
                .collect();
            assert_eq!(native, oracle, "{}", case["id"]);
        }
    }
    // Syntax-only decoding must not silently broaden the inherited model lexer.
    assert!(intersect_text::grammar::lex(r"'\u0041'").is_err());
    assert!(intersect_text::grammar::lex(r"'\b'").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn syntax_strings_preserve_all_utf16_boundaries() {
    for unit in [
        0u16, 8, 12, 0x7f, 0xff, 0xd7ff, 0xd800, 0xdbff, 0xdc00, 0xdfff, 0xe000, 0xffff,
    ] {
        let source = format!("grammar test import \"urn:\\u{unit:04x}\" Rule: '\\u{unit:04x}';");
        let ast = parse(&source).unwrap();
        let ElementKind::Keyword(value) = &ast.rules[0].value.body.value else {
            panic!("Expected keyword")
        };
        assert_eq!(value.0, vec![unit]);
        assert_eq!(
            value.to_scalar_string().is_err(),
            (0xd800..=0xdfff).contains(&unit)
        );
        assert_eq!(ast.metamodels[0].value.uri.value.0.last(), Some(&unit));
        let saved = serde_json::to_string(&ast).unwrap();
        assert_eq!(ast, serde_json::from_str(&saved).unwrap());
    }
    for (source, expected) in [
        (
            r"grammar test Rule: '\uD800x\uDC00';",
            vec![0xd800, 120, 0xdc00],
        ),
        (r"grammar test Rule: '\uD83D\uDE00';", vec![0xd83d, 0xde00]),
        ("grammar test Rule: '😀';", vec![0xd83d, 0xde00]),
    ] {
        let ast = parse(source).unwrap();
        let ElementKind::Keyword(value) = &ast.rules[0].value.body.value else {
            panic!("Expected keyword")
        };
        assert_eq!(value.0, expected);
    }
    for body in [r"'\u'", r"'\u123'", r"'\ug000'", r"'\u000x'"] {
        assert!(parse(&format!("grammar test Rule: {body};")).is_err());
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn annotations_arguments_and_source_envelope_have_spans() {
    let text = include_str!("../../../../fixtures/xtext/syntax-ast/structure.xtext");
    let ast = parse(text).unwrap();
    assert_eq!(ast.source_bytes, text.len());
    assert_eq!(
        &text[ast.rules[0].value.annotations[0].start..ast.rules[0].value.annotations[0].end],
        "@Override"
    );
    assert_eq!(
        &text[ast.rules[0].value.annotations[1].start..ast.rules[0].value.annotations[1].end],
        "@ Deprecated"
    );
    let ElementKind::Call {
        arguments: Some(arguments),
        ..
    } = &ast.rules[0].value.body.value
    else {
        panic!("Expected parameterized call")
    };
    assert_eq!(&text[arguments[0].start..arguments[0].end], "flag=P");
    assert_eq!(&text[arguments[1].start..arguments[1].end], "true");
    assert_eq!(arguments[0].value.parameter.as_ref().unwrap().value, "flag");
    assert!(arguments[1].value.parameter.is_none());
    let observation: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/syntax-ast/structure-observation.json"
    ))
    .unwrap();
    let nodes = observation["nodes"].as_array().unwrap();
    let spans = |class: &str| {
        nodes
            .iter()
            .filter(|n| n["class"] == class)
            .map(|n| {
                (
                    n["start"].as_u64().unwrap() as usize,
                    n["end"].as_u64().unwrap() as usize,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(vec![(ast.start, ast.end)], spans("Grammar"));
    assert_eq!(
        ast.rules[0]
            .value
            .annotations
            .iter()
            .map(|a| (a.start, a.end))
            .collect::<Vec<_>>(),
        spans("Annotation")
    );
    assert_eq!(
        arguments
            .iter()
            .map(|a| (a.start, a.end))
            .collect::<Vec<_>>(),
        spans("NamedArgument")
    );
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn independent_node_classes_and_lexical_boundaries() {
    use std::collections::BTreeMap;
    use syntax::{Element, RuleKind};
    type Counts = BTreeMap<String, usize>;
    fn add(c: &mut Counts, name: &str, n: usize) {
        *c.entry(name.into()).or_default() += n;
    }
    fn walk(e: &Element, c: &mut Counts) {
        match &e.value {
            ElementKind::Keyword(_) => add(c, "Keyword", 1),
            ElementKind::Call { arguments, .. } => {
                add(c, "RuleCall", 1);
                if let Some(args) = arguments {
                    for a in args {
                        add(c, "NamedArgument", 1);
                        walk(&a.value.value, c);
                    }
                }
            }
            ElementKind::Sequence(items)
            | ElementKind::Alternatives(items)
            | ElementKind::Unordered(items) => {
                add(
                    c,
                    match e.value {
                        ElementKind::Sequence(_) => "Group",
                        ElementKind::Alternatives(_) => "Alternatives",
                        _ => "UnorderedGroup",
                    },
                    1,
                );
                for item in items {
                    walk(item, c);
                }
            }
            ElementKind::Group(inner) | ElementKind::Cardinality { element: inner, .. } => {
                walk(inner, c)
            }
            ElementKind::Predicate { element, .. } => {
                if matches!(element.value, ElementKind::Group(_)) {
                    add(c, "Group", 1);
                }
                walk(element, c);
            }
            ElementKind::Guard {
                condition,
                elements,
            } => {
                add(c, "Group", 1);
                walk(condition, c);
                for e in elements {
                    walk(e, c);
                }
            }
            ElementKind::Assignment { value, .. } => {
                add(c, "Assignment", 1);
                walk(value, c);
            }
            ElementKind::Action { .. } => {
                add(c, "Action", 1);
                add(c, "TypeRef", 1);
            }
            ElementKind::CrossReference { spelling, .. } => {
                add(c, "CrossReference", 1);
                add(c, "TypeRef", 1);
                if let Some(s) = spelling {
                    walk(s, c);
                }
            }
            ElementKind::Negated(inner) | ElementKind::Until(inner) => {
                add(
                    c,
                    if matches!(e.value, ElementKind::Negated(_)) {
                        "NegatedToken"
                    } else {
                        "UntilToken"
                    },
                    1,
                );
                walk(inner, c);
            }
            ElementKind::Range { left, right } => {
                add(c, "CharacterRange", 1);
                walk(left, c);
                walk(right, c);
            }
            ElementKind::Wildcard => add(c, "Wildcard", 1),
            ElementKind::Eof => add(c, "EOF", 1),
            ElementKind::EnumLiteral { keyword, .. } => {
                add(c, "EnumLiteralDeclaration", 1);
                if let Some(k) = keyword {
                    walk(k, c);
                }
            }
            ElementKind::ConditionLiteral(_) => add(c, "LiteralCondition", 1),
            ElementKind::Parameter(_) => add(c, "ParameterReference", 1),
            ElementKind::ConditionNot(inner) => {
                add(c, "Negation", 1);
                walk(inner, c);
            }
            ElementKind::ConditionAnd(items) | ElementKind::ConditionOr(items) => {
                add(
                    c,
                    if matches!(e.value, ElementKind::ConditionAnd(_)) {
                        "Conjunction"
                    } else {
                        "Disjunction"
                    },
                    items.len() - 1,
                );
                for item in items {
                    walk(item, c);
                }
            }
        }
    }
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/syntax-ast/coverage-cases.json"
    ))
    .unwrap();
    let observations: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/syntax-ast/coverage-observations.json"
    ))
    .unwrap();
    assert!(intersect_text::grammar::lex("grammar test Rule: 'x'; // trailing\r").is_ok());
    assert_eq!(cases.len(), 11);
    assert_eq!(cases.len(), observations.len());
    for (case, observation) in cases.iter().zip(observations) {
        assert_eq!(case["id"], observation["id"]);
        let parsed = parse(case["source"].as_str().unwrap());
        assert_eq!(
            parsed.is_err(),
            observation["syntax_errors"].as_bool().unwrap(),
            "{}: {:?}",
            case["id"],
            parsed
        );
        if let Ok(ast) = parsed {
            let mut counts = Counts::new();
            add(&mut counts, "Grammar", 1);
            for m in &ast.metamodels {
                add(
                    &mut counts,
                    if m.value.generated_name.is_some() {
                        "GeneratedMetamodel"
                    } else {
                        "ReferencedMetamodel"
                    },
                    1,
                );
            }
            for rule in &ast.rules {
                add(
                    &mut counts,
                    match rule.value.kind {
                        RuleKind::Parser | RuleKind::Fragment => "ParserRule",
                        RuleKind::Terminal | RuleKind::TerminalFragment => "TerminalRule",
                        RuleKind::Enum => "EnumRule",
                    },
                    1,
                );
                if rule.value.return_type.is_some() {
                    add(&mut counts, "TypeRef", 1);
                }
                if let Some(parameters) = &rule.value.parameters
                    && !parameters.is_empty()
                {
                    add(&mut counts, "Parameter", parameters.len());
                }
                if !rule.value.annotations.is_empty() {
                    add(&mut counts, "Annotation", rule.value.annotations.len());
                }
                walk(&rule.value.body, &mut counts);
            }
            let mut expected = Counts::new();
            for node in observation["nodes"].as_array().unwrap() {
                add(&mut expected, node["class"].as_str().unwrap(), 1);
            }
            assert_eq!(counts, expected, "{}", case["id"]);
            if case["id"] == "node-matrix" {
                assert_eq!(counts.len(), 29);
            }
        }
    }
}
#[path = "support/xtext_syntax_projection.rs"]
mod projection;
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn independent_attributes_containment_and_reference_spellings() {
    let mut cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/syntax-ast/cases.json"
    ))
    .unwrap();
    cases.extend(
        serde_json::from_str::<Vec<serde_json::Value>>(include_str!(
            "../../../../fixtures/xtext/syntax-ast/coverage-cases.json"
        ))
        .unwrap(),
    );
    cases.push(serde_json::json!({"id":"structure","source":include_str!("../../../../fixtures/xtext/syntax-ast/structure.xtext")}));
    let observations: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/syntax-ast/tree-observations.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 31);
    assert_eq!(cases.len(), observations.len());
    let mut valid = 0;
    for (case, expected) in cases.iter().zip(observations) {
        assert_eq!(case["id"], expected["id"]);
        let parsed = parse(case["source"].as_str().unwrap());
        assert_eq!(
            parsed.is_err(),
            expected["syntax_errors"].as_bool().unwrap()
        );
        if let Ok(ast) = parsed {
            valid += 1;
            assert_eq!(
                projection::project(&ast),
                expected["tree"],
                "{}",
                case["id"]
            );
        }
    }
    assert_eq!(valid, 14);
}
