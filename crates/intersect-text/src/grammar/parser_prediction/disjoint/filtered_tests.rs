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
    let mut program = empty();
    let mut entries = Vec::new();
    for language in languages {
        let root = program.states.len();
        entries.push(root);
        program.states.push(State::default());
        for word in language {
            let mut previous = root;
            for text in word {
                let next = program.states.len();
                program.states.push(State::default());
                program.states[previous]
                    .edges
                    .push(Edge::Token(PredictionSymbol::Keyword((*text).into()), next));
                previous = next;
            }
            program.states[previous].accepting = true;
        }
    }
    program.decisions.insert(
        "choice".into(),
        Decision {
            entries,
            exit: None,
        },
    );
    program
}
fn filtered(program: &ParserPrediction, remaining: &mut usize) -> Result<bool, String> {
    prove(program, &program.decisions["choice"].entries, remaining)
}
fn pinned() -> ParserPrediction {
    use sha2::Digest as _;
    let raw = include_str!("../../../../assets/fixtures/xtext/connector-prediction/program.json");
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(raw.as_bytes())),
        "c1206e3c6817f35a891dfa1f19c3ab5f26e436baebdcd79a7f63fb8cc0d87aab"
    );
    serde_json::from_str(raw).unwrap()
}
#[test]
fn original_function_body_proof_and_exact_budget() {
    let program = pinned();
    let path = "rule/FunctionBodyPart/s0/repeat";
    assert_eq!(program.decisions[path].entries, [1061, 1060]);
    let mut remaining = 10_000;
    assert!(prove(&program, &program.decisions[path].entries, &mut remaining).unwrap());
    let units = 10_000 - remaining;
    println!("Filtered original FunctionBodyPart: {units} units");
    assert_eq!(units, 9548);
    let mut exact = units;
    assert!(prove(&program, &program.decisions[path].entries, &mut exact).unwrap());
    assert_eq!(exact, 0);
    assert!(
        prove(&program, &program.decisions[path].entries, &mut (units - 1))
            .unwrap_err()
            .contains("budget")
    );
    let mut admitted = 10_000;
    program
        .verify_disjoint_with_filtered_remaining(path, &mut admitted)
        .unwrap();
    assert_eq!(10_000 - admitted, 9611);
    let mut exact = 9611;
    program
        .verify_disjoint_with_filtered_remaining(path, &mut exact)
        .unwrap();
    assert_eq!(exact, 0);
    assert!(
        program
            .verify_disjoint_with_filtered_remaining(path, &mut 9610)
            .unwrap_err()
            .contains("budget")
    );
    println!(
        "Actual FunctionBodyPart admission: {} units",
        10_000 - admitted
    );
}
#[test]
fn preserves_all_original_v54_acceptances_and_receipts() {
    let program = pinned();
    let mut count = 0;
    for path in program.decisions.keys() {
        if program.verify_disjoint_with_certificates(path).is_ok() {
            program
                .verify_disjoint_with_filtered_closures(path)
                .unwrap_or_else(|error| panic!("Regressed {path}: {error}"));
            count += 1;
        }
    }
    println!("Preserved {count} original accepted V54 decisions");
    assert_eq!(count, 293);
    for (path, units) in [
        ("rule/Connector/s2/s0/choice", 8513),
        ("rule/FeatureElement/choice", 7400),
    ] {
        let mut remaining = 10_000;
        program
            .verify_disjoint_with_filtered_remaining(path, &mut remaining)
            .unwrap();
        assert_eq!(10_000 - remaining, units);
    }
}
#[test]
fn independent_finite_languages_preserve_prefix_conflicts() {
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
    for left in &languages {
        for right in &languages {
            let conflict = left
                .iter()
                .any(|a| right.iter().any(|b| a.starts_with(b) || b.starts_with(a)));
            let program = words(&[left.clone(), right.clone()]);
            assert_eq!(
                filtered(&program, &mut 10_000).unwrap(),
                !conflict,
                "{left:?} / {right:?}"
            );
            assert_eq!(
                program
                    .verify_disjoint_with_bounded_discovery("choice")
                    .is_ok(),
                !conflict,
                "V56 {left:?} / {right:?}"
            );
        }
    }
}
fn shared(depth: usize, nullable: bool) -> ParserPrediction {
    let mut program = words(&[vec![vec!["a"]], vec![vec!["b"]]]);
    let roots = [program.states.len(), program.states.len() + 1];
    let first = roots[1] + 1;
    for follow in [0, 2] {
        program.states.push(State {
            edges: vec![Edge::Call {
                target: first,
                follow,
            }],
            ..State::default()
        });
    }
    for index in 0..depth {
        let state = program.states.len();
        let end = state + 1;
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
fn exact_callers_nullable_returns_and_weighted_depth() {
    for nullable in [false, true] {
        for depth in [1, 8, 64] {
            let program = shared(depth, nullable);
            assert!(
                filtered(&program, &mut 10_000).unwrap(),
                "depth {depth}, nullable {nullable}"
            );
        }
        assert!(!filtered(&shared(65, nullable), &mut 10_000).unwrap());
    }
    let mut ambiguous = shared(8, true);
    ambiguous.states[2].edges = ambiguous.states[0].edges.clone();
    assert!(!filtered(&ambiguous, &mut 10_000).unwrap());
    let mut missing = shared(1, false);
    missing.states[0] = State {
        rule_stop: true,
        ..State::default()
    };
    assert!(
        filtered(&missing, &mut 10_000)
            .unwrap_err()
            .contains("FOLLOW")
    );
}
#[test]
fn unknown_guards_and_summary_cycles_never_grant_admission() {
    let mut guarded = words(&[vec![vec!["a"]], vec![vec!["b"]]]);
    let next = guarded.states.len();
    guarded.states.push(guarded.states[0].clone());
    guarded.states[0].edges = vec![Edge::First {
        path: "guard".into(),
        target: next,
    }];
    assert!(
        filtered(&guarded, &mut 10_000)
            .unwrap_err()
            .contains("Guarded")
    );
    guarded.first_guards.insert(
        "guard".into(),
        [PredictionSymbol::Keyword("a".into())].into(),
    );
    assert!(filtered(&guarded, &mut 10_000).unwrap());
    guarded.states[0].edges = vec![Edge::Epsilon(0)];
    assert!(!filtered(&guarded, &mut 10_000).unwrap());
    guarded.states[0].edges = vec![Edge::Call {
        target: 0,
        follow: next,
    }];
    assert!(!filtered(&guarded, &mut 10_000).unwrap());
}
#[test]
fn signature_saturation_retains_exact_token_identity() {
    for conflict in [false, true] {
        let mut program = words(&[vec![], vec![]]);
        let accept = program.states.len();
        program.states.push(State {
            accepting: true,
            ..State::default()
        });
        for index in 0..150 {
            program.states[1].edges.push(Edge::Token(
                PredictionSymbol::Keyword(format!("right{index}")),
                accept,
            ));
        }
        let name = if conflict { "right149" } else { "left" };
        program.states[0]
            .edges
            .push(Edge::Token(PredictionSymbol::Keyword(name.into()), accept));
        assert_eq!(filtered(&program, &mut 10_000).unwrap(), !conflict);
    }
}

#[test]
fn root_follow_and_summary_evaluator_boundaries_remain_explicit() {
    let mut program = words(&[vec![vec!["x", "a"]], vec![vec!["x", "b"]]]);
    // The left continuation returns with no local caller, then consumes the
    // original global FOLLOW. It must not be treated as acceptance or ignored.
    let follow = program.states.len();
    program.states.push(State {
        edges: vec![Edge::Token(PredictionSymbol::Keyword("a".into()), 2)],
        ..State::default()
    });
    program.states[1] = State {
        rule_stop: true,
        follow: vec![follow],
        ..State::default()
    };
    assert!(filtered(&program, &mut 10_000).unwrap());
    program.states[follow].edges = vec![Edge::Token(PredictionSymbol::Keyword("b".into()), 2)];
    assert!(!filtered(&program, &mut 10_000).unwrap());
    program.states[follow] = State {
        rule_stop: true,
        follow: vec![follow],
        ..State::default()
    };
    assert!(!filtered(&program, &mut 10_000).unwrap());
    for count in [128, 129] {
        let mut program = words(&[vec![vec!["a"]], vec![vec!["b"]]]);
        let mut next = 0;
        for _ in 1..count {
            let state = program.states.len();
            program.states.push(State {
                edges: vec![Edge::Epsilon(next)],
                ..State::default()
            });
            next = state;
        }
        program.decisions.get_mut("choice").unwrap().entries[0] = next;
        assert_eq!(filtered(&program, &mut 10_000).unwrap(), count == 128);
    }
}
#[test]
fn transparent_return_compression_keeps_acceptance_and_branch_boundaries() {
    let mut program = shared(8, false);
    // A continuation that accepts before its local stop is not transparent.
    let first = program.decisions["choice"].entries[0];
    let target = match program.states[first].edges[0] {
        Edge::Call { target, .. } => target,
        _ => panic!("Expected call"),
    };
    program.states[target + 1].accepting = true;
    assert!(!filtered(&program, &mut 10_000).unwrap());
    let mut branch = shared(8, false);
    let alternative = branch.states.len();
    branch.states.push(State {
        edges: vec![Edge::Token(PredictionSymbol::Keyword("c".into()), 1)],
        ..State::default()
    });
    branch.states[target + 1]
        .edges
        .push(Edge::Epsilon(alternative));
    branch.states[target + 1].rule_stop = false;
    assert!(!filtered(&branch, &mut 10_000).unwrap());
}

#[test]
fn shared_variable_length_callee_does_not_cancel_distinct_suffixes() {
    // {a, a b} . {b c} and {a, a b} . {c} overlap at a b c.
    // A shared call cannot simply be removed from both alternatives.
    let mut program = words(&[vec![vec!["b", "c"]], vec![vec!["c"]]]);
    let left = program.states.len();
    let right = left + 1;
    let common = right + 1;
    for follow in [0, 3] {
        program.states.push(State {
            edges: vec![Edge::Call {
                target: common,
                follow,
            }],
            ..State::default()
        });
    }
    program.states.push(State {
        edges: vec![Edge::Token(
            PredictionSymbol::Keyword("a".into()),
            common + 1,
        )],
        ..State::default()
    });
    program.states.push(State {
        edges: vec![
            Edge::Epsilon(common + 2),
            Edge::Token(PredictionSymbol::Keyword("b".into()), common + 2),
        ],
        ..State::default()
    });
    program.states.push(State {
        rule_stop: true,
        ..State::default()
    });
    program.decisions.get_mut("choice").unwrap().entries = vec![left, right];
    assert!(!filtered(&program, &mut 10_000).unwrap());
    program.states[3].edges = vec![Edge::Token(PredictionSymbol::Keyword("right".into()), 4)];
    assert!(filtered(&program, &mut 10_000).unwrap());
}

#[test]
fn namespace_member_is_an_independently_retained_obstruction() {
    let program = pinned();
    let path = "rule/NamespaceMember/choice";
    assert_eq!(program.decisions[path].entries, [1269, 1270]);
    assert_eq!(
        program.verify_disjoint_with_certificates(path).unwrap_err(),
        "Grammar prediction budget exceeded"
    );
    assert!(
        program
            .verify_disjoint_with_filtered_closures(path)
            .unwrap_err()
            .contains("Grammar prediction budget exceeded")
    );
}

#[test]
fn bounded_discovery_preserves_all_v55_acceptances_and_clears_namespace_member() {
    let program = pinned();
    let path = "rule/NamespaceMember/choice";
    let roots = &program.decisions[path].entries;
    let mut original = 10_000;
    assert!(
        super::super::projection::prove(&program, roots, &mut original)
            .unwrap_err()
            .contains("budget")
    );
    assert_eq!(original, 0);
    let mut discovery = 10_000;
    assert_eq!(
        super::super::projection::prove_with_bounded_discovery(
            &program,
            roots,
            &mut discovery,
            2000
        )
        .unwrap(),
        super::super::projection::Discovery::Exhausted
    );
    assert_eq!(discovery, 8000);
    assert!(prove(&program, roots, &mut discovery).unwrap());
    println!(
        "NamespaceMember bounded discovery plus filtered proof: {} units",
        10_000 - discovery
    );
    assert_eq!(discovery, 264);
    let mut actual = 10_000;
    program
        .verify_disjoint_with_discovery_remaining(path, &mut actual, Some(2000))
        .unwrap();
    let namespace_units = 10_000 - actual;
    println!("Native V56 NamespaceMember after certificate repair: {namespace_units} units");
    assert_eq!(namespace_units, 9746);
    for (path, units) in [
        ("rule/Connector/s2/s0/choice", 8513),
        ("rule/FeatureElement/choice", 7400),
        ("rule/FunctionBodyPart/s0/repeat", 9611),
        ("rule/NamespaceMember/choice", namespace_units),
    ] {
        let mut exact = units;
        program
            .verify_disjoint_with_discovery_remaining(path, &mut exact, Some(2000))
            .unwrap();
        assert_eq!(exact, 0);
        assert!(
            program
                .verify_disjoint_with_discovery_remaining(path, &mut (units - 1), Some(2000))
                .unwrap_err()
                .contains("budget")
        );
    }
    let mut accepted = 0;
    for path in program.decisions.keys() {
        if program.verify_disjoint_with_filtered_closures(path).is_ok() {
            program
                .verify_disjoint_with_bounded_discovery(path)
                .unwrap_or_else(|error| panic!("Regressed V55 {path}: {error}"));
            accepted += 1;
        }
    }
    println!("Preserved {accepted} V55 accepted decisions");
    assert_eq!(accepted, 294);
}
#[test]
fn binary_discovery_exact_bound_is_not_an_overall_attempt_bound_or_refund() {
    let program = pinned();
    let roots = &program.decisions["rule/Connector/s2/s0/choice"].entries;
    let mut enough = 10_000;
    assert_eq!(
        super::super::projection::prove_with_bounded_discovery(&program, roots, &mut enough, 1583)
            .unwrap(),
        super::super::projection::Discovery::Proven
    );
    assert_eq!(enough, 1487);
    let mut short = 10_000;
    assert_eq!(
        super::super::projection::prove_with_bounded_discovery(&program, roots, &mut short, 1582)
            .unwrap(),
        super::super::projection::Discovery::Exhausted
    );
    assert_eq!(short, 8418);
    let mut overall = 1582;
    assert!(
        super::super::projection::prove_with_bounded_discovery(&program, roots, &mut overall, 2000)
            .unwrap_err()
            .contains("budget")
    );
    assert_eq!(overall, 0);
    let mut zero = 10_000;
    assert_eq!(
        super::super::projection::prove_with_bounded_discovery(&program, roots, &mut zero, 0)
            .unwrap(),
        super::super::projection::Discovery::Exhausted
    );
    assert_eq!(zero, 10_000);
}

#[test]
fn bounded_search_never_uses_unknown_or_restrictive_guards_as_admission() {
    let mut program = words(&[vec![vec!["a"]], vec![vec!["b"]]]);
    let next = program.states.len();
    program.states.push(program.states[0].clone());
    program.states[0].edges = vec![Edge::First {
        path: "guard".into(),
        target: next,
    }];
    assert!(
        program
            .verify_disjoint_with_bounded_discovery("choice")
            .unwrap_err()
            .contains("Guarded")
    );
    program.first_guards.insert(
        "guard".into(),
        [PredictionSymbol::Keyword("a".into())].into(),
    );
    program
        .verify_disjoint_with_bounded_discovery("choice")
        .unwrap();
    program.states[next].edges = vec![Edge::Token(PredictionSymbol::Keyword("b".into()), 1)];
    assert!(
        program
            .verify_disjoint_with_bounded_discovery("choice")
            .unwrap_err()
            .contains("Ambiguous")
    );
    for depth in [64, 65] {
        let program = shared(depth, true);
        assert_eq!(
            program
                .verify_disjoint_with_bounded_discovery("choice")
                .is_ok(),
            depth == 64
        );
    }
}

fn common_words(callee: &[Vec<&str>], left: &[Vec<&str>], right: &[Vec<&str>]) -> ParserPrediction {
    let mut program = words(&[left.to_vec(), right.to_vec()]);
    let left_follow = program.decisions["choice"].entries[0];
    let right_follow = program.decisions["choice"].entries[1];
    let callee_root = program.states.len();
    program.states.push(State::default());
    for word in callee {
        let mut previous = callee_root;
        for text in word {
            let next = program.states.len();
            program.states.push(State::default());
            program.states[previous]
                .edges
                .push(Edge::Token(PredictionSymbol::Keyword((*text).into()), next));
            previous = next;
        }
        let stop = program.states.len();
        program.states.push(State {
            rule_stop: true,
            ..State::default()
        });
        program.states[previous].edges.push(Edge::Epsilon(stop));
    }
    let mut entries = Vec::new();
    for follow in [left_follow, right_follow] {
        entries.push(program.states.len());
        program.states.push(State {
            edges: vec![Edge::Call {
                target: callee_root,
                follow,
            }],
            ..State::default()
        });
    }
    program.decisions.get_mut("choice").unwrap().entries = entries;
    program
}
fn fence_only(program: &ParserPrediction) -> Result<bool, String> {
    Filtered::new(program, &mut 10_000).callee_fence(&program.decisions["choice"].entries)
}
#[test]
fn callee_fence_matches_independent_finite_concatenation_prefix_conflicts() {
    let mut words = vec![vec![]];
    for length in 1..=3 {
        for bits in 0..1usize << length {
            words.push(
                (0..length)
                    .map(|offset| if bits & (1 << offset) == 0 { "a" } else { "b" })
                    .collect(),
            );
        }
    }
    let mut languages: Vec<_> = words.iter().map(|word| vec![word.clone()]).collect();
    languages.extend(words.windows(2).map(<[_]>::to_vec));
    let mut certificates = 0;
    for language in &languages {
        let callee: Vec<Vec<_>> = language
            .iter()
            .map(|word| {
                word.iter()
                    .map(|s| if *s == "a" { "c" } else { "d" })
                    .collect()
            })
            .collect();
        for left in &languages {
            for right in &languages {
                let program = common_words(&callee, left, right);
                let joined = |suffix: &[Vec<&str>]| {
                    callee
                        .iter()
                        .flat_map(|prefix| {
                            suffix.iter().map(move |tail| {
                                prefix
                                    .iter()
                                    .chain(tail.iter())
                                    .map(|s| (*s).to_owned())
                                    .collect::<Vec<_>>()
                            })
                        })
                        .collect::<Vec<_>>()
                };
                let l = joined(left);
                let r = joined(right);
                let conflict = l
                    .iter()
                    .any(|a| r.iter().any(|b| a.starts_with(b) || b.starts_with(a)));
                if fence_only(&program).unwrap() {
                    assert!(!conflict, "{callee:?} . {left:?} / {right:?}");
                    certificates += 1;
                }
            }
        }
    }
    println!("Checked 24389 independent finite concatenations; {certificates} certificates");
    assert_eq!(certificates, 2842);
    let unsafe_shared = common_words(
        &[vec!["a"], vec!["a", "b"]],
        &[vec!["b", "c"]],
        &[vec!["c"]],
    );
    assert!(!fence_only(&unsafe_shared).unwrap());
    assert!(
        !prove_with_callee_fence(
            &unsafe_shared,
            &unsafe_shared.decisions["choice"].entries,
            &mut 10_000
        )
        .unwrap()
    );
}
#[test]
fn callee_fence_certifies_all_tokens_and_retains_guards_acceptance_and_pending_depth() {
    for depth in [1, 8, 64] {
        assert!(fence_only(&shared(depth, true)).unwrap());
    }
    let deep = shared(65, true);
    let mut remaining = 10_000;
    assert!(
        !Filtered::new(&deep, &mut remaining)
            .callee_fence(&deep.decisions["choice"].entries)
            .unwrap()
    );
    assert!(
        remaining > 8000,
        "must reject semantic depth before exhausting work"
    );
    let mut guarded = common_words(&[vec!["x", "y"]], &[vec!["a"]], &[vec!["b"]]);
    let target = match guarded.states[guarded.decisions["choice"].entries[0]].edges[0] {
        Edge::Call { target, .. } => target,
        _ => panic!("Expected call"),
    };
    let after_first = target + 1;
    let body = guarded.states.len();
    guarded.states.push(guarded.states[after_first].clone());
    guarded.states[after_first].edges = vec![Edge::First {
        path: "unknown-after-token".into(),
        target: body,
    }];
    assert!(!fence_only(&guarded).unwrap());
    guarded.first_guards.insert(
        "unknown-after-token".into(),
        [PredictionSymbol::Keyword("y".into())].into(),
    );
    assert!(fence_only(&guarded).unwrap());
    guarded.states[body].accepting = true;
    assert!(!fence_only(&guarded).unwrap());
    let mut missing = common_words(&[vec!["x"]], &[vec!["a"]], &[vec!["b"]]);
    let target = match missing.states[missing.decisions["choice"].entries[0]].edges[0] {
        Edge::Call { target, .. } => target,
        _ => panic!("Expected call"),
    };
    missing.states[target].edges.clear();
    assert!(!fence_only(&missing).unwrap());
    let mut recursive = common_words(&[vec!["x"]], &[vec!["a"]], &[vec!["b"]]);
    let target = match recursive.states[recursive.decisions["choice"].entries[0]].edges[0] {
        Edge::Call { target, .. } => target,
        _ => panic!("Expected call"),
    };
    recursive.states[target + 1].edges.push(Edge::Call {
        target,
        follow: target + 2,
    });
    assert!(!fence_only(&recursive).unwrap());
}

#[test]
fn grammar_deferred_work_pool_bounded_discovery_legacy_selected_and_shared_exact() {
    let mut left = vec!["a"; 2500];
    left.push("x");
    let mut right = vec!["a"; 2500];
    right.push("y");
    let program = words(&[vec![left], vec![right]]);
    assert!(program.states.len() < 10_000);
    let before = serde_json::to_vec(&program).unwrap();
    assert!(
        program
            .verify_disjoint_with_bounded_discovery("choice")
            .unwrap_err()
            .contains("budget")
    );
    let mut remaining = 1_000_000;
    program
        .verify_disjoint_with_bounded_discovery_remaining("choice", &mut remaining)
        .unwrap();
    let used = 1_000_000 - remaining;
    println!(
        "DEFERRED_SELECTED_DISCOVERY used={used} states={}",
        program.states.len()
    );
    assert!(used > 10_000);
    for grant in [0, used - 1, used] {
        let mut remaining = grant;
        let result =
            program.verify_disjoint_with_bounded_discovery_remaining("choice", &mut remaining);
        if grant == used {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().contains("budget"));
        }
        assert_eq!(remaining, 0);
    }
    let mut shared = used * 2;
    for _ in 0..2 {
        program
            .verify_disjoint_with_bounded_discovery_remaining("choice", &mut shared)
            .unwrap();
    }
    assert_eq!(shared, 0);
    let mut short = used * 2 - 1;
    program
        .verify_disjoint_with_bounded_discovery_remaining("choice", &mut short)
        .unwrap();
    assert!(
        program
            .verify_disjoint_with_bounded_discovery_remaining("choice", &mut short)
            .unwrap_err()
            .contains("budget")
    );
    assert_eq!(short, 0);
    let mut preconsumed = used + 7;
    for _ in 0..7 {
        analysis_charge(&mut preconsumed).unwrap();
    }
    program
        .verify_disjoint_with_bounded_discovery_remaining("choice", &mut preconsumed)
        .unwrap();
    assert_eq!(preconsumed, 0);
    let mut unknown = 17;
    assert_eq!(
        program
            .verify_disjoint_with_bounded_discovery_remaining("foreign", &mut unknown)
            .unwrap_err(),
        "Unknown parser prediction decision at prediction decision foreign"
    );
    assert_eq!(unknown, 17);
    assert_eq!(serde_json::to_vec(&program).unwrap(), before);
}
