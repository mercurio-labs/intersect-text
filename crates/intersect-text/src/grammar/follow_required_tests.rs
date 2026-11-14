use super::*;
use crate::grammar::Profile;
use crate::grammar::Rule;
use serde_json::json;

fn expression(kind: Kind) -> Expr {
    Expr {
        kind,
        cardinality: '1',
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
    }
}

fn grammar(profile: Profile) -> Grammar {
    let mut grammar: Grammar = serde_json::from_value(
        json!({"profile":profile,"name":"RequiredCall","entry":"Root","rules":{}}),
    )
    .unwrap();
    let mut body = expression(Kind::Literal("x".into()));
    body.cardinality = '?';
    let rule: Rule = serde_json::from_value(json!({"body":body,"class":"Target"})).unwrap();
    grammar.rules.insert("Target".into(), rule);
    grammar
}

fn run(
    grammar: &Grammar,
    expr: &Expr,
    remaining: &mut usize,
    required: &mut Vec<RequiredObjectCall>,
) -> Result<(), String> {
    walk(
        expr,
        &Tokens {
            symbols: ["literal:x".into()].into(),
            caller: false,
        },
        grammar,
        &mut BTreeMap::new(),
        (true, "rule/Root/s1/value", None, Some("Root")),
        remaining,
        (&mut BTreeSet::new(), Some(required)),
    )
}

#[test]
fn required_obligation_retention_has_literal_lookup_and_one_short_precedence() {
    let grammar = grammar(Profile::NativeV71);
    let expr = expression(Kind::Call("Target".into()));
    for limit in 0..3 {
        let mut remaining = limit;
        let mut required = Vec::new();
        assert_eq!(
            run(&grammar, &expr, &mut remaining, &mut required),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(remaining, 0);
        assert!(required.is_empty());
    }
    let mut remaining = 3;
    let mut required = Vec::new();
    assert_eq!(run(&grammar, &expr, &mut remaining, &mut required), Ok(()));
    assert_eq!(remaining, 0);
    assert_eq!(required.len(), 1);
    assert_eq!(
        (
            &*required[0].owner,
            &*required[0].callee,
            &*required[0].path
        ),
        ("Root", "Target", "rule/Root/s1/value")
    );
}

#[test]
fn required_obligations_keep_reverse_validation_order_and_partial_work_without_caching() {
    let grammar = grammar(Profile::NativeV71);
    let expr = expression(Kind::Sequence(vec![
        expression(Kind::Call("Target".into())),
        expression(Kind::Call("Target".into())),
    ]));
    for _ in 0..2 {
        let mut remaining = 6;
        let mut required = Vec::new();
        assert_eq!(
            run(&grammar, &expr, &mut remaining, &mut required),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(remaining, 0);
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].path, "rule/Root/s1/value/s1");
    }
}

#[test]
fn required_obligations_preserve_legacy_conflict_fee_and_reject_datatype_calls() {
    let mut grammar = grammar(Profile::NativeV70);
    let expr = expression(Kind::Call("Target".into()));
    let mut remaining = 3;
    assert_eq!(
        run(&grammar, &expr, &mut remaining, &mut Vec::new()),
        Err("Fragment caller follow conflict requires unsupported Xtext prediction".into())
    );
    assert_eq!(remaining, 2);
    grammar.profile = Profile::NativeV71;
    grammar.rules.get_mut("Target").unwrap().datatype = Some(crate::grammar::Datatype::EString);
    let mut remaining = 2;
    assert_eq!(
        run(&grammar, &expr, &mut remaining, &mut Vec::new()),
        Err("Required nullable call is not a nonfragment object rule".into())
    );
    assert_eq!(remaining, 0);
}
