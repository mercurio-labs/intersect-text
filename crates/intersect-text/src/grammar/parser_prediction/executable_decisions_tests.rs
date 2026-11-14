// Explicit component allowance over the same recursive-analysis worker.
// This neither compiles a language nor creates a native runtime companion.
fn bounded_recursive_decision(
    program: &ParserPrediction,
    path: &str,
    allowance: usize,
    shared_recursion: bool,
    depth_first: bool,
) -> (Result<bool, String>, usize) {
    let mut remaining = allowance;
    let result = program.multiple_recursive_alternatives(
        &program.decisions[path],
        &mut remaining,
        shared_recursion,
        &mut BTreeMap::new(),
        depth_first,
        (false, None),
    );
    (result, remaining)
}

// V67 keeps continuation metadata while analyzing executable decisions only.
// These controls do not admit source `=>` or prove full-language support.

#[test]
fn recovery_metadata_is_separate_from_choice_and_repeat_selection() {
    for profile in [Profile::NativeV65, Profile::NativeV66, Profile::NativeV67] {
        let mut repeated = keyword("x");
        repeated.cardinality = '*';
        let mut g = grammar(
            sequence(vec![
                choice(vec![
                    sequence(vec![keyword("a"), keyword("b")]),
                    sequence(vec![keyword("a"), keyword("c")]),
                ]),
                repeated,
                keyword("z"),
            ]),
            vec![],
        );
        g.profile = profile;
        let program = ParserPrediction::compile(&g).unwrap();
        let mut corrupted = program.clone();
        let recovery_paths: Vec<_> = program
            .decisions()
            .filter(|path| path.ends_with("/recovery"))
            .map(str::to_owned)
            .collect();
        assert!(recovery_paths.len() > 5);
        for path in &recovery_paths {
            // Keep the keys/marker so this changes only the continuation data
            // used by recovery, not the graph or an executable decision.
            corrupted.decisions.get_mut(path).unwrap().entries.clear();
        }
        for (path, text, expected) in [
            ("rule/Entry/s0/choice", "a b unrelated", Some(0)),
            ("rule/Entry/s0/choice", "a c unrelated", Some(1)),
            ("rule/Entry/s0/choice", "q", None),
            ("rule/Entry/s1/repeat", "x z", Some(0)),
            ("rule/Entry/s1/repeat", "z", None),
        ] {
            let observe = |candidate: &ParserPrediction| {
                let mut input = symbols(text).into_iter();
                let mut reads = 0;
                let mut work = Some(1000);
                let result = candidate.predict(
                    path,
                    || {
                        reads += 1;
                        Ok(input.next())
                    },
                    &mut work,
                );
                (result, reads, work)
            };
            let ordinary = observe(&program);
            assert_eq!(ordinary.0, Ok(expected), "{profile:?} {path} {text}");
            assert_eq!(ordinary, observe(&corrupted), "{profile:?} {path} {text}");
        }
        let continuation = program
            .recovery_symbols_with_work("rule/Entry/s0", &[], &mut Some(1000))
            .unwrap();
        assert_eq!(
            continuation,
            BTreeSet::from([
                PredictionSymbol::Keyword("x".into()),
                PredictionSymbol::Keyword("z".into()),
            ])
        );
        assert_eq!(
            corrupted
                .recovery_symbols_with_work("rule/Entry/s0", &[], &mut Some(1000))
                .unwrap_err(),
            "Invalid recovery continuation"
        );
        assert_eq!(
            serde_json::to_value(&program.states).unwrap(),
            serde_json::to_value(&corrupted.states).unwrap()
        );
        assert_eq!(program.rules, corrupted.rules);
        assert_eq!(program.recursive_decisions, corrupted.recursive_decisions);
    }
}

fn continuation_analysis_grammar(count: usize, profile: Profile) -> Grammar {
    let recursive_body = |name: &str, exit: &str| {
        let mut prefix: Vec<_> = (0..count).map(|_| keyword("a")).collect();
        prefix.push(call(name));
        choice(vec![sequence(prefix), keyword(exit)])
    };
    let mut result = grammar(
        sequence(vec![call("A"), call("B")]),
        vec![
            ("A", recursive_body("A", "b")),
            ("B", recursive_body("B", "c")),
        ],
    );
    result.profile = profile;
    result
}

#[test]
fn v67_retains_continuations_and_only_removes_their_recursive_analysis() {
    let guards = BTreeSet::from(["rule/Entry/s0".into()]);
    let before = ParserPrediction::compile_with_recursive_first_predicates(
        &continuation_analysis_grammar(3, Profile::NativeV66),
        &guards,
    )
    .unwrap();
    let after = ParserPrediction::compile_with_recursive_first_predicates(
        &continuation_analysis_grammar(3, Profile::NativeV67),
        &guards,
    )
    .unwrap();
    let mut original = serde_json::to_value(&before).unwrap();
    let mut candidate = serde_json::to_value(&after).unwrap();
    original
        .as_object_mut()
        .unwrap()
        .remove("recursive_decisions");
    candidate
        .as_object_mut()
        .unwrap()
        .remove("recursive_decisions");
    assert_eq!(original, candidate);
    assert!(
        before
            .recursive_decisions
            .iter()
            .all(|path| path.ends_with("/recovery"))
    );
    assert!(after.recursive_decisions.is_empty());
    assert!(after.decisions.contains_key("rule/Entry/s0/recovery"));
    for program in [&before, &after] {
        assert_eq!(
            program
                .recovery_symbols_with_work("rule/Entry/s0", &[], &mut Some(1000))
                .unwrap(),
            BTreeSet::from([
                PredictionSymbol::Keyword("a".into()),
                PredictionSymbol::Keyword("c".into()),
            ])
        );
        assert_eq!(
            selected(program, "rule/A/choice", "b", &mut Some(1000)).unwrap(),
            Some(1)
        );
    }
    let old = continuation_analysis_grammar(2500, Profile::NativeV66);
    let original =
        ParserPrediction::compile_with_recursive_first_predicates(&old, &guards).unwrap();
    let allowance = super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units();
    let (result, remaining) =
        bounded_recursive_decision(&original, "rule/Entry/s0/recovery", allowance, true, true);
    assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
    assert_eq!(remaining, 0);
    let new = continuation_analysis_grammar(2500, Profile::NativeV67);
    let candidate =
        ParserPrediction::compile_with_recursive_first_predicates(&new, &guards).unwrap();
    assert!(candidate.states.len() > 5000 && candidate.states.len() < 10000);
    assert!(candidate.decisions.contains_key("rule/Entry/s0/recovery"));
    assert!(candidate.recursive_decisions.is_empty());
    let mut original = serde_json::to_value(&original).unwrap();
    let mut revised = serde_json::to_value(&candidate).unwrap();
    original
        .as_object_mut()
        .unwrap()
        .remove("recursive_decisions");
    revised
        .as_object_mut()
        .unwrap()
        .remove("recursive_decisions");
    assert_eq!(original, revised);
}

#[test]
fn v67_executable_recursive_choices_keep_explicit_legacy_worker_budget() {
    let mut g = continuation_analysis_grammar(2500, Profile::NativeV67);
    g.rules.get_mut("Entry").unwrap().body = choice(vec![
        sequence(vec![call("A"), keyword("x")]),
        sequence(vec![call("B"), keyword("y")]),
    ]);
    let guards = BTreeSet::from(["rule/Entry/a0/s0".into()]);
    let program = ParserPrediction::compile_with_recursive_first_predicates(&g, &guards).unwrap();
    let (result, remaining) = bounded_recursive_decision(
        &program,
        "rule/Entry/choice",
        super::super::GrammarAnalysisWorkBudget::LEGACY_10K.units(),
        true,
        true,
    );
    assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
    assert_eq!(remaining, 0);
    assert!(
        ParserPrediction::compile_with_recursive_first_predicates(&g, &BTreeSet::new()).is_ok()
    );
}

#[test]
fn v67_profile_is_explicit_and_preserves_old_profile_identifiers_and_default() {
    for version in 0..=66 {
        let identifier = format!("intersect.xtext.native.v{version}");
        let parsed = serde_json::from_value::<Profile>(serde_json::json!(identifier));
        if version == 0 {
            // No version-zero identifier is part of the retained profile set.
            assert!(parsed.is_err());
        } else {
            let profile = parsed.unwrap();
            assert_eq!(profile.as_str(), identifier);
            assert_eq!(serde_json::to_value(profile).unwrap(), identifier);
            assert!(!profile.has_executable_recursive_analysis());
        }
    }
    assert_eq!(
        serde_json::to_value(Profile::default()).unwrap(),
        "intersect.workbench.v1"
    );
    assert_eq!(
        serde_json::to_value(Profile::NativeV67).unwrap(),
        "intersect.xtext.native.v67"
    );
    assert!(Profile::NativeV67.has_executable_recursive_analysis());
}
