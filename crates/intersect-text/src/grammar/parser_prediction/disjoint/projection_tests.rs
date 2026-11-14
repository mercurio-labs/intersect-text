use super::super::super::{Decision, State};
use super::*;

fn empty() -> ParserPrediction {
    ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states: Vec::new(),
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    }
}
fn words(languages: &[Vec<Vec<&str>>]) -> ParserPrediction {
    let mut result = empty();
    let mut entries = Vec::new();
    for language in languages {
        let root = result.states.len();
        result.states.push(State::default());
        entries.push(root);
        for word in language {
            let mut previous = root;
            for symbol in word {
                let next = result.states.len();
                result.states.push(State::default());
                result.states[previous].edges.push(Edge::Token(
                    PredictionSymbol::Keyword((*symbol).into()),
                    next,
                ));
                previous = next;
            }
            result.states[previous].accepting = true;
        }
    }
    result.decisions.insert(
        "choice".into(),
        Decision {
            entries,
            exit: None,
        },
    );
    result
}
fn project(program: &ParserPrediction, remaining: &mut usize) -> Result<bool, String> {
    prove(program, &program.decisions["choice"].entries, remaining)
}
fn shared(depth: usize, nullable: bool) -> ParserPrediction {
    let mut program = words(&[vec![vec!["a"]], vec![vec!["b"]]]);
    let follows = [0, 2];
    let roots = [program.states.len(), program.states.len() + 1];
    let first = roots[1] + 1;
    for follow in follows {
        program.states.push(State {
            edges: vec![Edge::Call {
                target: first,
                follow,
            }],
            ..State::default()
        });
    }
    for index in 0..depth {
        let start = program.states.len();
        let end = start + 1;
        let edge = if index + 1 == depth {
            Edge::Token(PredictionSymbol::Keyword("x".into()), end)
        } else {
            Edge::Call {
                target: end + 1,
                follow: end,
            }
        };
        program.states.push(State {
            edges: vec![edge],
            ..State::default()
        });
        program.states.push(State {
            rule_stop: true,
            ..State::default()
        });
    }
    if nullable {
        program.states[first].edges.push(Edge::Epsilon(first + 1));
    }
    program.decisions.get_mut("choice").unwrap().entries = roots.to_vec();
    program
}

#[test]
fn projection_proves_the_exact_pinned_connector_with_original_work_and_depth_guards() {
    let program: ParserPrediction = serde_json::from_str(include_str!(
        "../../../../assets/fixtures/xtext/connector-prediction/program.json"
    ))
    .unwrap();
    let roots = &program.decisions["rule/Connector/s2/s0/choice"].entries;
    let mut remaining = 10_000;
    assert!(prove(&program, roots, &mut remaining).unwrap());
    assert_eq!(remaining, 1487);
    println!(
        "Pinned Connector projected proof: {} of 10000 units",
        10_000 - remaining
    );
    program
        .verify_disjoint_with_projection("rule/Connector/s2/s0/choice")
        .unwrap();
    assert_eq!(
        program
            .verify_disjoint_with_relative_closures("rule/Connector/s2/s0/choice")
            .unwrap_err(),
        "Grammar prediction budget exceeded"
    );
    let later = "rule/FeatureElement/choice";
    assert_eq!(program.decisions[later].entries.len(), 10);
    let mut old = 10_000;
    let mut new = 10_000;
    assert_eq!(
        program
            .verify_disjoint_remaining(later, true, super::super::Reuse::Relative, &mut old)
            .unwrap_err(),
        "Grammar prediction budget exceeded"
    );
    assert_eq!(
        program
            .verify_disjoint_with_projection_remaining(later, &mut new)
            .unwrap_err(),
        "Grammar prediction budget exceeded"
    );
    assert_eq!(new, old);
}

#[test]
fn retained_pair_certificates_prove_the_original_ten_way_feature_element() {
    let program: ParserPrediction = serde_json::from_str(include_str!(
        "../../../../assets/fixtures/xtext/connector-prediction/program.json"
    ))
    .unwrap();
    let path = "rule/FeatureElement/choice";
    let roots = &program.decisions[path].entries;
    let mut remaining = 10_000;
    assert!(prove_certificates(&program, roots, &mut remaining).unwrap());
    assert_eq!(remaining, 3600);
    println!(
        "Pinned FeatureElement certificates: {} of 10000 units",
        10_000 - remaining
    );
    let mut admitted = 10_000;
    program
        .verify_disjoint_with_certificates_remaining(path, &mut admitted)
        .unwrap();
    assert_eq!(admitted, 2600);
    let mut admitted_exact = 7400;
    program
        .verify_disjoint_with_certificates_remaining(path, &mut admitted_exact)
        .unwrap();
    assert_eq!(admitted_exact, 0);
    assert!(
        program
            .verify_disjoint_with_certificates_remaining(path, &mut 7399)
            .unwrap_err()
            .contains("budget")
    );
    let mut exact = 6400;
    assert!(prove_certificates(&program, roots, &mut exact).unwrap());
    assert_eq!(exact, 0);
    assert!(
        prove_certificates(&program, roots, &mut 6399)
            .unwrap_err()
            .contains("budget")
    );
    let mut binary_old = 10_000;
    let mut binary_new = 10_000;
    program
        .verify_disjoint_with_projection_remaining("rule/Connector/s2/s0/choice", &mut binary_old)
        .unwrap();
    program
        .verify_disjoint_with_certificates_remaining("rule/Connector/s2/s0/choice", &mut binary_new)
        .unwrap();
    assert_eq!(binary_new, binary_old);
    let mut accepted = 0;
    for (path, decision) in &program.decisions {
        if decision.entries.len() > 2
            && program.verify_disjoint_with_relative_closures(path).is_ok()
        {
            program
                .verify_disjoint_with_certificates(path)
                .unwrap_or_else(|error| {
                    panic!("Regressed original n-ary decision {path}: {error}")
                });
            accepted += 1;
        }
    }
    println!("Retained {accepted} original accepted n-ary decisions");
    assert!(accepted > 0);
    {
        let path = "rule/FunctionBodyPart/s0/repeat";
        assert_eq!(
            program
                .verify_disjoint_with_relative_closures(path)
                .unwrap_err(),
            "Grammar prediction budget exceeded"
        );
        assert_eq!(
            program.verify_disjoint_with_certificates(path).unwrap_err(),
            "Grammar prediction budget exceeded"
        );
    }
}

#[test]
fn nary_certificates_match_independent_finite_language_prefix_conflicts() {
    let mut all = vec![vec![]];
    for length in 1..=3 {
        for bits in 0..1usize << length {
            all.push(
                (0..length)
                    .map(|offset| if bits & (1 << offset) == 0 { "a" } else { "b" })
                    .collect(),
            );
        }
    }
    let mut languages: Vec<_> = all.iter().map(|word| vec![word.clone()]).collect();
    languages.extend(all.windows(2).map(<[_]>::to_vec));
    let conflict = |a: &Vec<Vec<&str>>, b: &Vec<Vec<&str>>| {
        a.iter().any(|left| {
            b.iter()
                .any(|right| left.starts_with(right) || right.starts_with(left))
        })
    };
    let mut count = 0;
    for a in &languages {
        for b in &languages {
            for c in &languages {
                let expected = !(conflict(a, b) || conflict(a, c) || conflict(b, c));
                let program = words(&[a.clone(), b.clone(), c.clone()]);
                let proved =
                    prove_certificates(&program, &program.decisions["choice"].entries, &mut 10_000)
                        .unwrap();
                assert!(
                    !proved || expected,
                    "Unsound certificate: {a:?} / {b:?} / {c:?}"
                );
                assert_eq!(
                    program.verify_disjoint_with_certificates("choice").is_ok(),
                    expected,
                    "{a:?} / {b:?} / {c:?}"
                );
                count += 1;
            }
        }
    }
    assert_eq!(count, 24_389);
    let mut duplicate = words(&[vec![vec!["a"]], vec![vec!["b"]], vec![vec!["c"]]]);
    duplicate.decisions.get_mut("choice").unwrap().entries[2] = 0;
    assert!(
        duplicate
            .verify_disjoint_with_certificates("choice")
            .is_err()
    );
}

#[test]
fn certificate_attempts_keep_exact_fallback_work_and_separate_stack_depth_bounds() {
    let mut program = words(&[
        vec![vec!["x", "a"]],
        vec![vec!["x", "x", "a"]],
        vec![vec!["z"]],
    ]);
    let roots = &program.decisions["choice"].entries;
    let mut remaining = 10_000;
    assert!(!prove_certificates(&program, roots, &mut remaining).unwrap());
    let speculative = 10_000 - remaining;
    assert!(speculative > 0);
    program
        .verify_disjoint_remaining(
            "choice",
            true,
            super::super::Reuse::Relative,
            &mut remaining,
        )
        .unwrap();
    let total = 10_000 - remaining;
    let mut exact = total - speculative;
    program
        .verify_disjoint_remaining("choice", true, super::super::Reuse::Relative, &mut exact)
        .unwrap();
    assert_eq!(exact, 0);
    let mut certificate_exact = speculative;
    assert!(!prove_certificates(&program, roots, &mut certificate_exact).unwrap());
    assert_eq!(certificate_exact, 0);
    assert!(
        prove_certificates(&program, roots, &mut (speculative - 1))
            .unwrap_err()
            .contains("budget")
    );
    let mut legacy = 10_000;
    program
        .verify_disjoint_remaining("choice", true, super::super::Reuse::Relative, &mut legacy)
        .unwrap();
    let mut bounded = 10_000;
    program
        .verify_disjoint_with_certificates_remaining("choice", &mut bounded)
        .unwrap();
    assert_eq!(
        legacy, bounded,
        "Cheap legacy proofs keep their exact receipts"
    );
    assert!(total > speculative);
    for depth in [64, 65] {
        program = shared(depth, true);
        let roots = &program.decisions["choice"].entries;
        assert_eq!(
            prove_certificates(&program, roots, &mut 10_000).unwrap(),
            depth == 64
        );
    }
    for length in [127, 128] {
        let mut chain = words(&[vec![vec!["a"]], vec![vec!["b"]], vec![vec!["c"]]]);
        let previous = chain.decisions["choice"].entries[0];
        let start = chain.states.len();
        for offset in 0..length {
            chain.states.push(State {
                edges: vec![Edge::Epsilon(if offset + 1 == length {
                    previous
                } else {
                    start + offset + 1
                })],
                ..State::default()
            });
        }
        chain.decisions.get_mut("choice").unwrap().entries[0] = start;
        let roots = &chain.decisions["choice"].entries;
        assert_eq!(
            prove_certificates(&chain, roots, &mut 10_000).unwrap(),
            length == 127
        );
        chain.verify_disjoint_with_certificates("choice").unwrap();
    }
    let mut broad = words(&[vec![], vec![vec!["b"]], vec![vec!["c"]]]);
    let accepting = broad.states.len();
    broad.states.push(State {
        accepting: true,
        ..State::default()
    });
    for _ in 0..200 {
        let branch = broad.states.len();
        broad.states.push(State {
            edges: vec![Edge::Token(
                PredictionSymbol::Keyword("a".into()),
                accepting,
            )],
            ..State::default()
        });
        broad.states[0].edges.push(Edge::Epsilon(branch));
    }
    let mut allowance = 10_000;
    assert!(
        !prove_certificates(&broad, &broad.decisions["choice"].entries, &mut allowance).unwrap()
    );
    assert_eq!(
        allowance, 8997,
        "Three pair creations plus 1000 discovery units remain spent"
    );
    broad.verify_disjoint_with_certificates("choice").unwrap();
}

#[test]
fn certificates_retire_only_proven_pairs_and_keep_caller_guard_and_cycle_boundaries() {
    let retired = words(&[
        vec![vec!["a"]],
        vec![vec!["b", "a", "x"]],
        vec![vec!["b", "a", "y"]],
    ]);
    assert!(!project(&retired, &mut 10_000).unwrap());
    assert!(
        prove_certificates(&retired, &retired.decisions["choice"].entries, &mut 10_000).unwrap()
    );
    let mut calls = shared(3, true);
    let other = calls.states.len();
    calls.states.push(State {
        edges: vec![Edge::Token(
            PredictionSymbol::Keyword("z".into()),
            other + 1,
        )],
        ..State::default()
    });
    calls.states.push(State {
        accepting: true,
        ..State::default()
    });
    calls
        .decisions
        .get_mut("choice")
        .unwrap()
        .entries
        .push(other);
    assert!(prove_certificates(&calls, &calls.decisions["choice"].entries, &mut 10_000).unwrap());
    calls.verify_disjoint_with_certificates("choice").unwrap();
    let mut unknown = retired;
    let body = unknown.states.len();
    unknown.states.push(unknown.states[0].clone());
    unknown.states[0].edges = vec![Edge::First {
        path: "missing".into(),
        target: body,
    }];
    assert!(
        !prove_certificates(&unknown, &unknown.decisions["choice"].entries, &mut 10_000).unwrap()
    );
    assert!(
        unknown
            .verify_disjoint_with_certificates("choice")
            .unwrap_err()
            .contains("Guarded")
    );
    unknown.first_guards.insert(
        "missing".into(),
        [PredictionSymbol::Keyword("a".into())].into(),
    );
    assert!(
        prove_certificates(&unknown, &unknown.decisions["choice"].entries, &mut 10_000).unwrap()
    );
    for (count, expected) in [(126, true), (127, false)] {
        let mut wide = words(&[vec![], vec![vec!["other"]], vec![vec!["last"]]]);
        let accept = wide.states.len();
        wide.states.push(State {
            accepting: true,
            ..State::default()
        });
        for index in 0..count {
            wide.states[0].edges.push(Edge::Token(
                PredictionSymbol::Keyword(format!("k{index}")),
                accept,
            ));
        }
        assert_eq!(
            prove_certificates(&wide, &wide.decisions["choice"].entries, &mut 10_000).unwrap(),
            expected
        );
        wide.verify_disjoint_with_certificates("choice").unwrap();
    }
    let entry = calls.decisions["choice"].entries[0];
    calls.states[entry].edges = vec![Edge::Call {
        target: entry,
        follow: 0,
    }];
    assert!(!prove_certificates(&calls, &calls.decisions["choice"].entries, &mut 10_000).unwrap());
    assert!(calls.verify_disjoint_with_certificates("choice").is_err());
}

#[test]
fn shared_nullable_callees_keep_exact_suffixes_and_literal_runtime_choices() {
    let program = shared(3, true);
    assert!(project(&program, &mut 10_000).unwrap());
    for (prefix, suffix, expected) in [
        (false, "a", 0),
        (true, "a", 0),
        (false, "b", 1),
        (true, "b", 1),
    ] {
        let mut tokens = Vec::new();
        if prefix {
            tokens.push(PredictionSymbol::Keyword("x".into()));
        }
        tokens.push(PredictionSymbol::Keyword(suffix.into()));
        let mut tokens = tokens.into_iter();
        assert_eq!(
            program
                .predict("choice", || Ok(tokens.next()), &mut Some(1000))
                .unwrap(),
            Some(expected)
        );
    }
}

#[test]
fn literal_languages_independently_check_projection_soundness_and_fallback() {
    let mut all = vec![Vec::new()];
    for length in 1..=3 {
        for bits in 0..(1usize << length) {
            all.push(
                (0..length)
                    .map(|offset| if bits & (1 << offset) == 0 { "a" } else { "b" })
                    .collect(),
            );
        }
    }
    let mut languages: Vec<_> = all.iter().map(|word| vec![word.clone()]).collect();
    for pair in all.windows(2) {
        languages.push(pair.to_vec());
    }
    for a in &languages {
        for b in &languages {
            let conflict = a.iter().any(|left| {
                b.iter()
                    .any(|right| left.starts_with(right) || right.starts_with(left))
            });
            let program = words(&[a.clone(), b.clone()]);
            if project(&program, &mut 10_000).unwrap() {
                assert!(!conflict, "{a:?} / {b:?}");
            }
            assert_eq!(
                program.verify_disjoint_with_projection("choice").is_ok(),
                !conflict,
                "{a:?} / {b:?}"
            );
        }
    }
    let nary = words(&[
        vec![vec!["x", "a"]],
        vec![vec!["x", "b"]],
        vec![vec!["x", "c"]],
    ]);
    assert!(project(&nary, &mut 10_000).unwrap());
    let mut old = 10_000;
    let mut new = 10_000;
    nary.verify_disjoint_remaining("choice", true, super::super::Reuse::Relative, &mut old)
        .unwrap();
    nary.verify_disjoint_with_projection_remaining("choice", &mut new)
        .unwrap();
    assert_eq!(
        new, old,
        "N-ary decisions must not spend speculative projection work"
    );
}

#[test]
fn erasure_is_inconclusive_without_refunding_fallback_work() {
    let program = words(&[vec![vec!["x", "a"]], vec![vec!["x", "x", "a"]]]);
    let mut remaining = 10_000;
    assert!(!project(&program, &mut remaining).unwrap());
    assert!(remaining < 10_000);
    program
        .verify_disjoint_remaining(
            "choice",
            true,
            super::super::Reuse::Relative,
            &mut remaining,
        )
        .unwrap();
    let total = 10_000 - remaining;
    assert!(
        program
            .verify_disjoint_with_projection_remaining("choice", &mut (total - 1))
            .is_err()
    );
    let mut exact = total;
    program
        .verify_disjoint_with_projection_remaining("choice", &mut exact)
        .unwrap();
    assert_eq!(exact, 0);
}

#[test]
fn projection_keeps_exact_work_depth_alphabet_and_unknown_guard_boundaries() {
    let program = shared(64, false);
    assert!(project(&program, &mut 10_000).unwrap());
    let deep = shared(65, false);
    assert!(!project(&deep, &mut 10_000).unwrap());
    assert!(deep.verify_disjoint_with_projection("choice").is_err());
    let small = shared(2, true);
    let mut remaining = 10_000;
    assert!(project(&small, &mut remaining).unwrap());
    let used = 10_000 - remaining;
    let mut exact = used;
    assert!(project(&small, &mut exact).unwrap());
    assert_eq!(exact, 0);
    assert!(
        project(&small, &mut (used - 1))
            .unwrap_err()
            .contains("budget")
    );
    let mut unknown = words(&[vec![vec!["a"]], vec![vec!["b"]]]);
    let body = unknown.states.len();
    unknown.states.push(unknown.states[0].clone());
    unknown.states[0].edges = vec![Edge::First {
        path: "missing".into(),
        target: body,
    }];
    assert!(!project(&unknown, &mut 10_000).unwrap());
    assert!(
        unknown
            .verify_disjoint_with_projection("choice")
            .unwrap_err()
            .contains("Guarded")
    );
    unknown.first_guards.insert(
        "missing".into(),
        [PredictionSymbol::Keyword("a".into())].into(),
    );
    assert!(project(&unknown, &mut 10_000).unwrap());
    for (count, projected) in [(127, true), (128, false), (129, false)] {
        let mut wide = words(&[vec![], vec![]]);
        let accept = wide.states.len();
        wide.states.push(State {
            accepting: true,
            ..State::default()
        });
        for index in 0..count {
            wide.states[0].edges.push(Edge::Token(
                PredictionSymbol::Keyword(format!("k{index}")),
                accept,
            ));
        }
        wide.states[1].edges.push(Edge::Token(
            PredictionSymbol::Keyword("other".into()),
            accept,
        ));
        assert_eq!(project(&wide, &mut 10_000).unwrap(), projected);
        wide.verify_disjoint_with_projection("choice").unwrap();
    }
    let mut recursive = shared(1, false);
    let entry = recursive.decisions["choice"].entries[0];
    recursive.states[entry].edges = vec![Edge::Call {
        target: entry,
        follow: 0,
    }];
    assert!(!project(&recursive, &mut 10_000).unwrap());
    assert!(recursive.verify_disjoint_with_projection("choice").is_err());
}

#[test]
fn projected_profile_preserves_v52_capabilities_and_all_serialized_identifiers() {
    use crate::grammar::Profile;
    let checks: [fn(Profile) -> bool; 17] = [
        Profile::has_lexical_prediction,
        Profile::has_parser_overrides,
        Profile::has_return_inference,
        Profile::has_current_actions,
        Profile::has_parser_prediction,
        Profile::has_enum_rules,
        Profile::has_rule_conversions,
        Profile::has_recursive_predicates,
        Profile::has_double_rules,
        Profile::has_recursive_fragments,
        Profile::has_computed_settings,
        Profile::has_terminal_decisions,
        Profile::has_validated_syntax,
        Profile::has_terminal_overrides,
        Profile::has_explicit_terminal_calls,
        Profile::has_explicit_parser_calls,
        Profile::has_typed_overrides,
    ];
    for check in checks {
        assert_eq!(check(Profile::NativeV53), check(Profile::NativeV52));
        assert_eq!(check(Profile::NativeV54), check(Profile::NativeV53));
        assert_eq!(check(Profile::NativeV55), check(Profile::NativeV54));
        assert_eq!(check(Profile::NativeV56), check(Profile::NativeV55));
    }
    for version in 1..=56 {
        let name = format!("intersect.xtext.native.v{version}");
        let profile: Profile =
            serde_json::from_value(serde_json::Value::String(name.clone())).unwrap();
        assert_eq!(profile.as_str(), name);
        assert_eq!(serde_json::to_value(profile).unwrap(), name);
    }
}
