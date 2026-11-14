use super::*;
use crate::grammar;
use crate::grammar::Profile;
use crate::grammar::backend_test_support::inspect_ecore_schema;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use std::rc::Rc;

fn parsed(body: &str) -> (DefinitionSource, Grammar) {
    let schema = inspect_ecore_schema(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:prefix"><eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="children" upperBound="-1" containment="true" eType="#//Root"/></eClassifiers></e:EPackage>"##).unwrap();
    let source = DefinitionSource::new(
        "memory://prefix/actual.xtext",
        SourceKind::Xtext,
        format!("grammar Prefix hidden() import \"urn:prefix\" as t {body}"),
    );
    // Existing source parser; no manually authored AST, alias or source range.
    // This is inspection setup, not whole language admission/model validity.
    let (mut grammar, _, spans, _) = grammar::parse_declarations(
        &source.text,
        &schema,
        Profile::NativeV80,
        false,
        None,
        128,
        Rc::default(),
    )
    .unwrap();
    grammar.declaration_sources = spans
        .into_iter()
        .map(|(name, range)| {
            (
                name,
                SourceSpan {
                    source: source.id.clone(),
                    start: range.start,
                    end: range.end,
                },
            )
        })
        .collect();
    grammar::terminals::prepare(&mut grammar.terminals, grammar.profile)
        .unwrap_or_else(|error| panic!("Terminal preparation failed: {}", error.message));
    (source, grammar)
}

#[test]
fn source_prefix_capture_keeps_old_none_results_work_and_frame_layout_exact() {
    let (_, grammar) = parsed("Root returns t::Root: 'x';");
    let expr = &grammar.rules["Root"].body;
    let before = serde_json::to_vec(&grammar).unwrap();
    for budget in 0..20 {
        let mut remaining = budget;
        let result = super::super::analyze_with_remaining(expr, &grammar, 0, &mut remaining);
        assert_eq!(result, super::super::analyze(expr, &grammar, 0, budget));
        assert_eq!(remaining, if budget == 0 { 0 } else { budget - 1 });
    }
    let mut exact = 9;
    let observed = grammar
        .inspect_source_prefix_with_work("rule/Root", &mut exact)
        .unwrap();
    assert_eq!(exact, 0);
    assert_eq!(observed.symbols(), &BTreeSet::from(["literal:x".into()]));
    assert_eq!(observed.body_nullable(), Some(false));
    assert_eq!(observed.maximum_call_depth(), 0);
    assert_eq!(observed.calls().count(), 0);
    assert_eq!(serde_json::to_vec(&grammar).unwrap(), before);
    // The opt-in return frame fits the existing largest transient variant.
    #[allow(dead_code)]
    enum LegacyFrame<'a> {
        Visit(&'a Expr, usize, usize),
        Cardinality(char),
        Items {
            items: &'a [Expr],
            next: usize,
            syntax: usize,
            calls: usize,
            alternatives: bool,
            tokens: BTreeSet<String>,
            nullable: bool,
        },
    }
    assert_eq!(
        std::mem::size_of::<super::super::Frame<'_>>(),
        std::mem::size_of::<LegacyFrame<'_>>()
    );
}

#[test]
fn source_prefix_capture_retains_actual_full_assignment_source_and_foreign_negatives() {
    let (source, grammar) = parsed(
        "Root returns t::Root: {t::Root} (=> children+=Target)* 'end'; Target returns t::Root: 'x';",
    );
    let original = serde_json::to_vec(&grammar).unwrap();
    let mut remaining = 10_000;
    let result = grammar
        .inspect_source_prefix_with_work("rule/Root/s1/repeat", &mut remaining)
        .unwrap();
    assert_eq!(result.path(), "rule/Root/s1/repeat");
    assert_eq!(result.owner(), "Root");
    assert_eq!(result.source().source, source.id);
    assert!(result.nullable());
    assert_eq!(result.body_nullable(), Some(false));
    assert_eq!(result.symbols(), &BTreeSet::from(["literal:x".into()]));
    let calls: Vec<_> = result.calls().collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name(), "Target");
    assert!(std::ptr::eq(calls[0].rule(), &grammar.rules["Target"]));
    assert_eq!(calls[0].source(), &grammar.declaration_sources["Target"]);
    assert!(!calls[0].complete_empty_derivation());
    assert!(
        grammar
            .inspect_source_prefix_with_work("rule/Root/s900/repeat", &mut remaining)
            .is_err()
    );
    let (_, foreign) = parsed("Root returns t::Root: 'z';");
    let foreign_result = foreign
        .inspect_source_prefix_with_work("rule/Root", &mut remaining)
        .unwrap();
    assert_eq!(
        result.separation_with_work(&foreign_result, &mut remaining),
        Err("FIRST diagnostics belong to different immutable grammars".into())
    );
    let mut invalid = grammar.clone();
    invalid.rules.get_mut("Target").unwrap().body.original_range = Some(0..usize::MAX);
    assert!(
        matches!(invalid.inspect_source_prefix_with_work("rule/Root/s1/repeat",&mut remaining),Err(error) if error=="Captured FIRST callee escapes its source declaration")
    );
    assert_eq!(serde_json::to_vec(&grammar).unwrap(), original);
}

#[test]
fn source_prefix_capture_reports_conservative_disjoint_overlap_nullable_and_hidden_facts() {
    let (_, mut grammar) = parsed(
        "Root returns t::Root: 'r'; X returns t::Root: 'x'; Y returns t::Root: 'y'; Same returns t::Root: 'x'; Empty returns t::Root: {t::Root}; Word returns t::Root: ID; terminal ID: ('a'..'z')+;",
    );
    let mut remaining = 10_000;
    {
        let x = grammar
            .inspect_source_prefix_with_work("rule/X", &mut remaining)
            .unwrap();
        let y = grammar
            .inspect_source_prefix_with_work("rule/Y", &mut remaining)
            .unwrap();
        let same = grammar
            .inspect_source_prefix_with_work("rule/Same", &mut remaining)
            .unwrap();
        let empty = grammar
            .inspect_source_prefix_with_work("rule/Empty", &mut remaining)
            .unwrap();
        let word = grammar
            .inspect_source_prefix_with_work("rule/Word", &mut remaining)
            .unwrap();
        assert_eq!(
            x.separation_with_work(&y, &mut remaining).unwrap(),
            PrefixSeparation::DisjointFirstOnly
        );
        assert_eq!(
            x.separation_with_work(&same, &mut remaining).unwrap(),
            PrefixSeparation::Overlap
        );
        assert_eq!(
            x.separation_with_work(&empty, &mut remaining).unwrap(),
            PrefixSeparation::Unavailable
        );
        assert_eq!(
            x.separation_with_work(&word, &mut remaining).unwrap(),
            PrefixSeparation::Unavailable
        );
        assert_eq!(empty.body_nullable(), Some(true));
    }
    // Adversarial explicit policy metadata must withhold the diagnostic result.
    grammar.rules.get_mut("Y").unwrap().hidden = Some(vec![]);
    let x = grammar
        .inspect_source_prefix_with_work("rule/X", &mut remaining)
        .unwrap();
    let hidden = grammar
        .inspect_source_prefix_with_work("rule/Y", &mut remaining)
        .unwrap();
    assert!(hidden.prefix_hidden_policy());
    assert_eq!(
        x.separation_with_work(&hidden, &mut remaining).unwrap(),
        PrefixSeparation::Unavailable
    );
}

#[test]
fn source_prefix_capture_charges_one_original_pool_exact_short_and_combined_depth() {
    let (_, grammar) = parsed("Root returns t::Root: Target; Target returns t::Root: 'x';");
    let mut exact = 18;
    let result = grammar
        .inspect_source_prefix_with_work("rule/Root", &mut exact)
        .unwrap();
    assert_eq!(exact, 0);
    assert_eq!(result.maximum_call_depth(), 1);
    for budget in 0..18 {
        let mut remaining = budget;
        assert!(
            matches!(grammar.inspect_source_prefix_with_work("rule/Root",&mut remaining),Err(error) if error=="Grammar prediction budget exceeded")
        );
        assert_eq!(remaining, 0);
    }
    let mut denied = 0;
    assert_eq!(
        result.combined_call_depth_with_work(127, &mut denied),
        Err("Grammar prediction budget exceeded".into())
    );
    let mut one = 1;
    assert_eq!(
        result.combined_call_depth_with_work(127, &mut one).unwrap(),
        128
    );
    assert_eq!(one, 0);
    one = 1;
    assert_eq!(
        result.combined_call_depth_with_work(128, &mut one),
        Err("Parser prediction call depth exceeded".into())
    );
    assert_eq!(one, 0);
    let mut body = String::from("Root returns t::Root: R0;");
    for index in 0..64 {
        body.push_str(&format!(
            "R{index} returns t::Root: {};",
            if index == 63 {
                "'x'".into()
            } else {
                format!("R{}", index + 1)
            }
        ));
    }
    let (_, grammar) = parsed(&body);
    let mut remaining = 10_000;
    let result = grammar
        .inspect_source_prefix_with_work("rule/Root", &mut remaining)
        .unwrap();
    assert_eq!(result.maximum_call_depth(), 64);
    assert_eq!(
        result
            .combined_call_depth_with_work(64, &mut remaining)
            .unwrap(),
        128
    );
    assert_eq!(
        result.combined_call_depth_with_work(65, &mut remaining),
        Err("Parser prediction call depth exceeded".into())
    );
    assert!(remaining > 0);
}

#[test]
fn source_prefix_capture_never_completes_direct_or_mutual_left_recursive_facts() {
    for extra in [
        "A returns t::Root: A;",
        "A returns t::Root: B; B returns t::Root: A;",
    ] {
        let (_, grammar) = parsed(&format!("Root returns t::Root: A; {extra}"));
        let mut remaining = 10_000;
        assert!(
            matches!(grammar.inspect_source_prefix_with_work("rule/Root",&mut remaining),Err(error) if error=="Grammar prediction budget exceeded")
        );
        assert!(remaining > 0); // Explicit diagnostic frame fees do not hide64.
        let mut plain_remaining = 10_000;
        let plain = super::super::analyze_with_remaining(
            &grammar.rules["Root"].body,
            &grammar,
            0,
            &mut plain_remaining,
        );
        assert_eq!(plain, Err("Grammar prediction budget exceeded".into()));
        assert!(plain_remaining > 9_000); // Old None fees/64 ceiling stay exact.
        assert_eq!(
            super::super::analyze(&grammar.rules["Root"].body, &grammar, 0, 10_000),
            Err("Grammar prediction budget exceeded".into())
        );
    }
}

#[test]
fn source_prefix_capture_prepays_long_symbol_path_and_frame_copies() {
    // literal: is eight UTF-8 bytes. 512 source bytes need three chunks,
    // compared with one for the original literal:x (18 ->20 total units).
    let text = "x".repeat(512);
    let (_, grammar) = parsed(&format!(
        "Root returns t::Root: Target; Target returns t::Root: '{text}';"
    ));
    for budget in [0, 17, 18, 19, 20] {
        let mut remaining = budget;
        let result = grammar.inspect_source_prefix_with_work("rule/Root", &mut remaining);
        if budget < 20 {
            assert!(matches!(result,Err(error) if error=="Grammar prediction budget exceeded"));
        } else {
            let result = result.unwrap();
            assert_eq!(
                result.symbols(),
                &BTreeSet::from([format!("literal:{text}")])
            );
            assert_eq!(result.calls().count(), 1);
        }
        assert_eq!(remaining, 0);
    }
    let owner = format!("R{}", "a".repeat(300));
    let path = format!("rule/{owner}");
    let (_, grammar) = parsed(&format!("{owner} returns t::Root: 'x';"));
    // Both the resolver's extended path and retained report path need two
    // chunks, versus one each for rule/Root (9 ->11 total units).
    for budget in [1, 9, 10, 11] {
        let mut remaining = budget;
        let result = grammar.inspect_source_prefix_with_work(&path, &mut remaining);
        if budget < 11 {
            assert!(matches!(result,Err(error) if error=="Grammar prediction budget exceeded"));
        } else {
            assert_eq!(result.unwrap().owner(), owner);
        }
        assert_eq!(remaining, 0);
    }
    // A denial before path construction must remain that earliest actual fee,
    // rather than inventing source-owner failure after uncharged allocation.
    let mut one = 1;
    assert!(
        matches!(grammar.inspect_source_prefix_with_work(&format!("rule/{}","z".repeat(300)),&mut one),Err(error) if error=="Grammar prediction budget exceeded")
    );
    assert_eq!(one, 0);
    // Prepay frame retention/copy before either new Return or paired Visit.
    let mut frames: Vec<super::super::Frame<'_, std::collections::BTreeSet<String>>> =
        vec![super::super::Frame::Visit(
            &grammar.rules[&owner].body,
            0,
            0,
        )];
    let before_len = frames.len();
    let before_capacity = frames.capacity();
    let mut short = 1;
    assert_eq!(
        retain_source_frame(&mut frames, &mut short),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(short, 0);
    assert_eq!(
        (frames.len(), frames.capacity()),
        (before_len, before_capacity)
    );
    let mut exact = 2;
    retain_source_frame(&mut frames, &mut exact).unwrap();
    assert_eq!(exact, 0);
    assert_eq!(frames.len(), before_len);
    assert!(frames.capacity() >= before_len + 2);
    // Original bounded FIRST still accepts its literal at one unit; no trace
    // storage/copy fee leaks into the default worker.
    let mut plain = 1;
    assert_eq!(
        super::super::analyze_with_remaining(&grammar.rules[&owner].body, &grammar, 0, &mut plain)
            .unwrap(),
        (BTreeSet::from(["literal:x".into()]), false)
    );
    assert_eq!(plain, 0);
}

#[test]
fn source_prefix_capture_accepts_the_default_grant_pool_and_rejects_larger() {
    // Pools come from the caller's per-pool grant; since 1e5cd75b the default
    // grant is GrammarAnalysisWorkBudget::REALISTIC, not the historical 10k.
    let (_, grammar) = parsed("Root returns t::Root: 'x';");
    let grant = grammar::GrammarAnalysisWorkBudget::default().units();
    let mut remaining = grant;
    assert!(
        grammar
            .inspect_source_prefix_with_work("rule/Root", &mut remaining)
            .is_ok()
    );
    assert!(remaining < grant);
    let mut oversized = grant + 1;
    assert_eq!(
        grammar
            .inspect_source_prefix_with_work("rule/Root", &mut oversized)
            .err()
            .as_deref(),
        Some("Source prefix inspection requires the original bounded work pool")
    );
    assert_eq!(oversized, grant + 1);
}
