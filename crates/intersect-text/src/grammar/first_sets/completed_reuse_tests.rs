//! Pure canonical-worker controls; no new original language request.
use super::{completed_reuse::Completed, *};
use intersect_model::definitions::bundle::SourceId;

fn fixture() -> Result<Grammar, String> {
    let source = SourceId::for_text("memory:completed-first", "x");
    serde_json::from_value(serde_json::json!({
        "profile":"intersect.xtext.native.v80","name":"Completed","entry":"Root",
        "rules":{"Root":{"class":"Root","body":{"cardinality":"1","original_range":{"start":0,"end":1},"kind":{"Literal":"x"}}}},
        "declaration_sources":{"Root":{"source":source,"start":0,"end":1}}
    })).map_err(|e| e.to_string())
}
fn node(grammar: &Grammar, kind: Kind) -> Expr {
    let mut value = grammar.rules["Root"].body.clone();
    value.kind = kind;
    value.cardinality = '1';
    value
}
fn add(grammar: &mut Grammar, name: &str, value: Expr, source: bool) {
    let mut rule = grammar.rules["Root"].clone();
    rule.body = value;
    grammar.rules.insert(name.into(), rule);
    if source {
        let declaration = grammar.declaration_sources["Root"].clone();
        grammar.declaration_sources.insert(name.into(), declaration);
    }
}
fn call(grammar: &Grammar, name: &str) -> Expr {
    node(grammar, Kind::Call(name.into()))
}
fn shared() -> Result<(Grammar, Expr), String> {
    let mut grammar = fixture()?;
    let mut prefix = vec![node(&grammar, Kind::Action("Root".into())); 200];
    prefix.push(node(&grammar, Kind::Literal("x".into())));
    let body = node(&grammar, Kind::Sequence(prefix));
    add(&mut grammar, "Shared", body, true);
    let root = node(
        &grammar,
        Kind::Alternative(vec![call(&grammar, "Shared"); 24]),
    );
    Ok((grammar, root))
}

#[test]
fn completed_shared_prefix_removes_expansions_after_all_paid_overhead() -> Result<(), String> {
    let (grammar, root) = shared()?;
    let original = serde_json::to_value(&grammar).map_err(|e| e.to_string())?;
    let mut plain_work = 10_000;
    let expected = analyze_with_remaining(&root, &grammar, 0, &mut plain_work);
    let mut work = 10_000;
    let mut completed = Completed::new(&grammar, &mut work)?;
    let actual = analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed);
    println!(
        "COMPLETED_FIRST_PAID_WORK {:?}",
        (
            10_000 - plain_work,
            10_000 - work,
            completed.visits,
            completed.hits
        )
    );
    assert_eq!(expected, actual);
    assert!(actual.is_ok());
    assert_eq!(completed.hits, 23);
    // Baseline's only work debit is one per canonical Visit. Enabled work
    // includes actual Visits AND every new header/lookup/retention/copy debit.
    assert!(completed.visits < 10_000 - plain_work);
    assert!(10_000 - work < 10_000 - plain_work);
    assert_eq!(
        serde_json::to_value(&grammar).map_err(|e| e.to_string())?,
        original
    );
    Ok(())
}

#[test]
fn completed_body_and_caller_cardinalities_keep_full_symbol_sets() -> Result<(), String> {
    for body_cardinality in ['1', '?', '*', '+'] {
        let mut grammar = fixture()?;
        let mut body = node(
            &grammar,
            Kind::Alternative(vec![
                node(&grammar, Kind::Literal("x".into())),
                node(&grammar, Kind::Literal("y".into())),
            ]),
        );
        body.cardinality = body_cardinality;
        add(&mut grammar, "Leaf", body, true);
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        for caller in ['?', '1', '*', '+', '1'] {
            let mut entry = call(&grammar, "Leaf");
            entry.cardinality = caller;
            let expected = analyze(&entry, &grammar, 0, 10_000);
            assert_eq!(
                expected,
                analyze_with_completed(&entry, &grammar, 0, &mut work, &mut completed)
            );
            let (tokens, nullable) = expected?;
            assert_eq!(tokens.len(), 2);
            assert_eq!(
                nullable,
                matches!(body_cardinality, '?' | '*') || matches!(caller, '?' | '*')
            );
        }
        let scalar = node(
            &grammar,
            Kind::ScalarReference {
                target: "Root".into(),
                rule: "Leaf".into(),
            },
        );
        assert_eq!(
            analyze(&scalar, &grammar, 0, 10_000),
            analyze_with_completed(&scalar, &grammar, 0, &mut work, &mut completed)
        );
        assert_eq!(completed.hits, 5);
    }
    Ok(())
}

#[test]
fn nested_hits_propagate_depth_peaks_at_63_64_65_and_syntax_limits() -> Result<(), String> {
    for length in [60, 61, 62] {
        let mut grammar = fixture()?;
        let leaf = node(&grammar, Kind::Literal("x".into()));
        add(&mut grammar, "C", leaf, true);
        let body = call(&grammar, "C");
        add(&mut grammar, "B", body, true);
        let body = call(&grammar, "B");
        add(&mut grammar, "A", body, true);
        for index in (0..length).rev() {
            let next = if index + 1 == length {
                "A".into()
            } else {
                format!("D{}", index + 1)
            };
            let body = call(&grammar, &next);
            // Deliberately ineligible wrappers use the unchanged traversal.
            add(&mut grammar, &format!("D{index}"), body, false);
        }
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        for name in ["C", "B", "A"] {
            let entry = call(&grammar, name);
            assert_eq!(
                analyze(&entry, &grammar, 0, 10_000),
                analyze_with_completed(&entry, &grammar, 0, &mut work, &mut completed)
            );
        }
        assert!(completed.hits >= 2);
        let root = call(&grammar, "D0");
        let expected = analyze(&root, &grammar, 0, 10_000);
        let actual = analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed);
        assert_eq!(actual, expected);
        assert_eq!(actual.is_ok(), length < 62);
        assert!(
            work > 0,
            "independent64 depth check must remain distinct from exhausted work"
        );
    }
    for nesting in [64, 65] {
        let mut grammar = fixture()?;
        let leaf = node(&grammar, Kind::Literal("x".into()));
        add(&mut grammar, "Leaf", leaf, true);
        let entry = call(&grammar, "Leaf");
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        analyze_with_completed(&entry, &grammar, 0, &mut work, &mut completed)?;
        let mut root = call(&grammar, "Leaf");
        for _ in 0..nesting {
            root = node(&grammar, Kind::Sequence(vec![root]));
        }
        assert_eq!(
            analyze(&root, &grammar, 0, 10_000),
            analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed)
        );
    }
    Ok(())
}

#[test]
fn active_cycles_do_not_publish_and_unvisited_productive_tails_remain_legal() -> Result<(), String>
{
    for shape in [0, 1, 2] {
        let mut grammar = fixture()?;
        let body = if shape == 2 {
            node(
                &grammar,
                Kind::Alternative(vec![
                    node(&grammar, Kind::Action("Root".into())),
                    call(&grammar, "Root"),
                ]),
            )
        } else {
            call(&grammar, if shape == 1 { "Other" } else { "Root" })
        };
        grammar.rules.get_mut("Root").ok_or("Missing root")?.body = body;
        if shape == 1 {
            let body = call(&grammar, "Root");
            add(&mut grammar, "Other", body, true);
        }
        let entry = call(&grammar, "Root");
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        let actual = analyze_with_completed(&entry, &grammar, 0, &mut work, &mut completed);
        assert_eq!(actual, analyze(&entry, &grammar, 0, 10_000));
        assert!(actual.is_err());
        assert_eq!(completed.entry_count(), 0);
    }
    let mut grammar = fixture()?;
    let body = node(
        &grammar,
        Kind::Sequence(vec![
            node(&grammar, Kind::Literal("x".into())),
            call(&grammar, "Root"),
        ]),
    );
    grammar.rules.get_mut("Root").ok_or("Missing root")?.body = body;
    let entry = call(&grammar, "Root");
    let mut work = 10_000;
    let mut completed = Completed::new(&grammar, &mut work)?;
    for _ in 0..2 {
        assert_eq!(
            analyze(&entry, &grammar, 0, 10_000),
            analyze_with_completed(&entry, &grammar, 0, &mut work, &mut completed)
        );
    }
    assert_eq!(completed.hits, 1);
    Ok(())
}

#[test]
fn missing_source_and_foreign_equal_grammar_use_canonical_fallback() -> Result<(), String> {
    let mut grammar = fixture()?;
    let body = node(&grammar, Kind::Literal("x".into()));
    add(&mut grammar, "Leaf", body, true);
    let foreign = grammar.clone();
    let entry = call(&grammar, "Leaf");
    let other = call(&foreign, "Leaf");
    let mut work = 10_000;
    let mut completed = Completed::new(&grammar, &mut work)?;
    analyze_with_completed(&entry, &grammar, 0, &mut work, &mut completed)?;
    assert_eq!(
        analyze(&other, &foreign, 0, 10_000),
        analyze_with_completed(&other, &foreign, 0, &mut work, &mut completed)
    );
    assert_eq!(completed.hits, 0);
    for missing in [false, true] {
        let mut grammar = fixture()?;
        let body = node(&grammar, Kind::Literal("z".into()));
        add(&mut grammar, "Leaf", body, true);
        if missing {
            grammar.declaration_sources.remove("Leaf");
        } else {
            grammar
                .rules
                .get_mut("Leaf")
                .ok_or("Missing leaf")?
                .body
                .original_range = None;
        }
        let mut work = 10_000;
        let mut completed = Completed::new(&grammar, &mut work)?;
        for kind in [
            Kind::Call("Leaf".into()),
            Kind::Call("UnboundTerminal".into()),
            Kind::ScalarReference {
                target: "Root".into(),
                rule: "MissingDatatype".into(),
            },
        ] {
            let root = node(&grammar, kind);
            let expected = analyze(&root, &grammar, 0, 10_000);
            let actual = analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed);
            assert_eq!(actual, expected);
        }
        assert_eq!(completed.entry_count(), 0);
    }
    Ok(())
}

#[test]
fn zero_short_exact_preconsumption_and_failed_publication_are_sticky() -> Result<(), String> {
    let grammar = fixture()?;
    let root = call(&grammar, "Root");
    let run = |allowance: usize| -> Result<(Summary, usize), String> {
        let mut work = allowance;
        let mut completed = Completed::new(&grammar, &mut work)?;
        let result = analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed)?;
        Ok((result, work))
    };
    let (expected, left) = run(10_000)?;
    let spent = 10_000 - left;
    assert_eq!(run(spent)?, (expected.clone(), 0));
    assert!(run(0).is_err());
    assert!(run(spent - 1).is_err());
    let mut saw_publication_denial = false;
    for allowance in 3..spent {
        let mut work = allowance;
        let mut completed = Completed::new(&grammar, &mut work)?;
        let result = analyze_with_completed(&root, &grammar, 0, &mut work, &mut completed);
        if completed.completions > 0 && completed.publications == 0 {
            assert!(result.is_err());
            assert_eq!(completed.entry_count(), 0);
            // Replenishing an already failed owner cannot expose partial rows.
            let mut later = 10_000;
            assert!(
                analyze_with_completed(&root, &grammar, 0, &mut later, &mut completed).is_err()
            );
            saw_publication_denial = true;
        }
    }
    assert!(saw_publication_denial);
    // A fresh owner over the same immutable input recovers without credit.
    assert_eq!(run(spent)?, (expected, 0));
    Ok(())
}

#[test]
fn retained_symbol_bound_rejects_before_summary_publication() -> Result<(), String> {
    let small = fixture()?;
    let entry = call(&small, "Root");
    let mut work = 10_000;
    let mut cache = Completed::new(&small, &mut work)?;
    analyze_with_completed(&entry, &small, 0, &mut work, &mut cache)?;
    let exact = cache.retained_bytes();
    for limit in [0, 127, 128, exact - 1, exact] {
        let mut work = 10_000;
        match Completed::with_limit(&small, &mut work, limit) {
            Err(_) => {
                assert!(limit < 128);
            }
            Ok(mut cache) => {
                let value = analyze_with_completed(&entry, &small, 0, &mut work, &mut cache);
                assert_eq!(value.is_ok(), limit == exact);
                assert_eq!(cache.entry_count(), usize::from(limit == exact));
                assert!(cache.retained_bytes() <= limit);
                if limit != exact {
                    assert_eq!(cache.publications, 0);
                }
            }
        }
    }
    let mut grammar = fixture()?;
    let body = node(&grammar, Kind::Literal("x".repeat(1_048_576)));
    add(&mut grammar, "Huge", body, true);
    let entry = call(&grammar, "Huge");
    let mut work = 10_000;
    let mut completed = Completed::new(&grammar, &mut work)?;
    let error = analyze_with_completed(&entry, &grammar, 0, &mut work, &mut completed)
        .err()
        .ok_or("Expected retention denial")?;
    assert!(error.contains("retention bound"));
    assert!(work > 0);
    assert_eq!(completed.entry_count(), 0);
    assert_eq!(completed.publications, 0);
    Ok(())
}
