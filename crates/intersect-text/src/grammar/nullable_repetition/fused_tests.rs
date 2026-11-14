// Authored source-qualification controls. No grammar/model/oracle run is hidden
// here; the existing fixture is an explicit hand-built canonical AST component.
fn fused_signature(proof: Option<QualifiedNullableDecision<'_>>) -> serde_json::Value {
    match proof {
        Some(QualifiedNullableDecision::FullRepeat(p)) => serde_json::json!({
            "kind":"FullRepeat", "owner":p.owner(), "callee":p.callee(),
            "keyword":p.keyword(), "occurrence":p.expression().original_range,
            "call":p.call_expression().original_range,
        }),
        Some(QualifiedNullableDecision::KeywordOptional(p)) => serde_json::json!({
            "kind":"KeywordOptional", "owner":p.owner(), "keyword":p.keyword(),
            "occurrence":p.expression().original_range,
        }),
        None => serde_json::Value::Null,
    }
}

fn old_two_qualifiers(
    grammar: &Grammar,
    path: &str,
    remaining: &mut usize,
) -> Result<serde_json::Value, String> {
    // Exact old compiler order, including its second query after a Full match.
    let full = grammar.qualified_nullable_full_repeat_with_work(path, remaining)?;
    let optional = grammar.qualified_nullable_keyword_optional_with_work(path, remaining)?;
    assert!(full.is_none() || optional.is_none());
    Ok(fused_signature(
        full.map(QualifiedNullableDecision::FullRepeat)
            .or_else(|| optional.map(QualifiedNullableDecision::KeywordOptional)),
    ))
}

fn nonnullable_fixture() -> Grammar {
    let mut grammar = fixture();
    let Kind::Sequence(items) = &mut grammar.rules.get_mut("Target").unwrap().body.kind else {
        panic!("Expected exact target source sequence")
    };
    items[1].cardinality = '1';
    grammar
}

#[test]
fn fused_source_literal_full_optional_and_nonmatching_costs_preserve_capabilities() {
    for (mut grammar, path, old_cost, fused_cost) in [
        (fixture(), "rule/Root/s1/repeat", 15, 10),
        (fixture(), "rule/Target/s1/repeat", 12, 9),
        (nonnullable_fixture(), "rule/Root/s1/repeat", 13, 10),
    ] {
        grammar.profile = Profile::NativeV79;
        let mut old_work = 10000;
        let baseline = old_two_qualifiers(&grammar, path, &mut old_work).unwrap();
        let mut fused_work = 10000;
        let proof = grammar
            .qualified_nullable_decision_with_finite_work(path, &mut fused_work)
            .unwrap();
        if let Some(QualifiedNullableDecision::FullRepeat(p)) = &proof {
            let mut direct_work = 10000;
            let direct = grammar
                .qualified_nullable_full_repeat_with_work(path, &mut direct_work)
                .unwrap().unwrap();
            assert!(std::ptr::eq(p.expression(), direct.expression()));
            assert!(std::ptr::eq(p.call_expression(), direct.call_expression()));
        }
        assert_eq!(fused_signature(proof), baseline);
        assert_eq!(10000 - old_work, old_cost);
        assert_eq!(10000 - fused_work, fused_cost);
        println!("V79_FUSED_SOURCE_COST {}", serde_json::json!({
            "path":path,"old_two_queries":old_cost,"fused":fused_cost,
            "same_capability":baseline,
        }));
    }
}

#[test]
fn fused_source_zero_exact_one_short_and_original_preconsumed_allowance() {
    let mut grammar = fixture();
    grammar.profile = Profile::NativeV79;
    for allowance in 0..=10 {
        let mut remaining = allowance;
        let result = grammar.qualified_nullable_decision_with_finite_work(
            "rule/Root/s1/repeat", &mut remaining,
        );
        if allowance == 10 {
            assert!(matches!(result.unwrap(), Some(QualifiedNullableDecision::FullRepeat(_))));
        } else {
            assert_eq!(result.err().unwrap(), "Grammar prediction budget exceeded");
        }
        assert_eq!(remaining, 0);
    }
    let mut original_remaining = 17;
    for _ in 0..7 { charge(&mut original_remaining).unwrap(); }
    assert!(grammar.qualified_nullable_decision_with_finite_work(
        "rule/Root/s1/repeat", &mut original_remaining,
    ).unwrap().is_some());
    assert_eq!(original_remaining, 0);
    let mut fresh_recovery = 10;
    assert!(grammar.qualified_nullable_decision_with_finite_work(
        "rule/Root/s1/repeat", &mut fresh_recovery,
    ).unwrap().is_some());
    assert_eq!(fresh_recovery, 0);
}

#[test]
fn fused_source_shared_actual_path_depth_saves_only_repeated_resolution() {
    let mut grammar = nonnullable_fixture();
    grammar.profile = Profile::NativeV79;
    let body = grammar.rules["Root"].body.clone();
    let mut alternatives = vec![node(Kind::Literal("z".into()), '1', 0, 1); 6];
    alternatives.push(body);
    grammar.rules.get_mut("Root").unwrap().body =
        node(Kind::Alternative(alternatives), '1', 0, 4);
    let mut old_work = 10000;
    let baseline = old_two_qualifiers(
        &grammar, "rule/Root/a6/s1/repeat", &mut old_work,
    ).unwrap();
    let mut fused_work = 10000;
    let actual = grammar.qualified_nullable_decision_with_finite_work(
        "rule/Root/a6/s1/repeat", &mut fused_work,
    ).unwrap();
    assert_eq!(baseline, fused_signature(actual));
    assert_eq!(baseline, serde_json::Value::Null);
    assert_eq!(10000 - old_work, 14);
    assert_eq!(10000 - fused_work, 10);
    // This is an authored source-path component, not a replay of SysML or a
    // claim that the actual global admission pool saves a measured four units.
}

#[test]
fn fused_source_full_failure_precedes_optional_lookup_with_exact_old_error() {
    for scenario in 0..3 {
        let mut grammar = fixture();
        grammar.profile = Profile::NativeV79;
        match scenario {
            0 => { grammar.declaration_sources.remove("Root"); }
            1 => { grammar.declaration_sources.remove("Target"); }
            _ => { grammar.rules.get_mut("Root").unwrap().hidden = Some(vec![]); }
        }
        let mut old_work = 10000;
        let old_error = old_two_qualifiers(
            &grammar, "rule/Root/s1/repeat", &mut old_work,
        ).err().unwrap();
        let mut new_work = 10000;
        let error = grammar.qualified_nullable_decision_with_finite_work(
            "rule/Root/s1/repeat", &mut new_work,
        ).err().unwrap();
        assert_eq!(error, old_error);
        assert_eq!(new_work, old_work); // no second query after the real Err
    }
}

#[test]
fn fused_source_wrong_path_and_foreign_range_preserve_old_failure_and_recover() {
    let mut grammar = fixture();
    grammar.profile = Profile::NativeV79;
    for path in ["rule/Missing/s1/repeat", "rule/Root/s99/repeat", "wrong/Root/s1/repeat"] {
        let mut old_work = 10000;
        let old_error = old_two_qualifiers(&grammar, path, &mut old_work).err().unwrap();
        let mut work = 10000;
        let error = grammar.qualified_nullable_decision_with_finite_work(path, &mut work)
            .err().unwrap();
        assert_eq!(error, old_error);
        assert_eq!(work, old_work);
    }
    let mut foreign = grammar.clone();
    let Kind::Sequence(items) = &mut foreign.rules.get_mut("Root").unwrap().body.kind else {
        panic!("Expected source sequence")
    };
    items[1].original_range = Some(5..7);
    let mut old_work = 10000;
    let old_error = old_two_qualifiers(&foreign, "rule/Root/s1/repeat", &mut old_work).err().unwrap();
    let mut work = 10000;
    assert_eq!(foreign.qualified_nullable_decision_with_finite_work(
        "rule/Root/s1/repeat", &mut work,
    ).err().unwrap(), old_error);
    assert_eq!(work, old_work);
    assert!(grammar.qualified_nullable_decision_with_finite_work(
        "rule/Root/s1/repeat", &mut 10,
    ).unwrap().is_some());
}

#[test]
fn fused_source_old_profiles_keep_literal_fees_and_unlimited_policy() {
    for profile in [Profile::NativeV77, Profile::NativeV78] {
        let mut grammar = fixture();
        grammar.profile = profile;
        let mut full = 10000;
        assert!(grammar.qualified_nullable_full_repeat_with_work(
            "rule/Root/s1/repeat", &mut full,
        ).unwrap().is_some());
        assert_eq!(10000 - full, 10);
        let mut optional = 10000;
        assert!(grammar.qualified_nullable_keyword_optional_with_work(
            "rule/Target/s1/repeat", &mut optional,
        ).unwrap().is_some());
        assert_eq!(10000 - optional, 8);
        let mut runtime = Some(10000);
        assert!(matches!(grammar.qualified_nullable_decision_with_work(
            "rule/Target/s1/repeat", &mut runtime,
        ).unwrap(), Some(QualifiedNullableDecision::KeywordOptional(_))));
        assert_eq!(runtime, Some(9988));
    }
    let mut grammar = fixture();
    grammar.profile = Profile::NativeV79;
    let mut unlimited = None;
    assert!(grammar.qualified_nullable_decision_with_work(
        "rule/Root/s1/repeat", &mut unlimited,
    ).unwrap().is_some());
    assert_eq!(unlimited, None);
    assert_eq!(std::mem::size_of::<Occurrence<'_>>(), 4 * std::mem::size_of::<usize>());
}
