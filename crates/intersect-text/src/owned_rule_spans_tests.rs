//! Private exact-fee and layout controls; no production visibility changes.
use super::*;
use intersect_model::frozen::FrozenStore;

fn with_machine(run: impl FnOnce(&mut Machine<'_, '_, FrozenStore>)) {
    with_machine_profile(grammar::Profile::NativeV71, run);
}

fn with_machine_profile(
    profile: grammar::Profile,
    run: impl FnOnce(&mut Machine<'_, '_, FrozenStore>),
) {
    let before: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-owned-rule-spans-v71/native-before-local.json"
    ))
    .unwrap();
    let mut package: Package =
        serde_json::from_value(before["records"][6]["package"].clone()).unwrap();
    package.grammar.as_mut().unwrap().profile = profile;
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store.configure_schema(&package.schema).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let words = BTreeSet::new();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        retained_tokens: true,
        hidden_policy: &[],
        raw_entry: false,
        construction: Some(("Root", 0)),
        settings: None,
        raw_source: "one x",
        utf16_positions: vec![],
        datatype_text: None,
        conversions: None,
        initializers: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        keywords: &words,
        undo: vec![],
        limits: EvaluationLimits::workbench(),
        package: &package,
        uri: "span.model",
        tokens: vec![
            Token {
                text: Cow::Borrowed("one"),
                start: 0,
                end: 3,
                quoted: false,
                lexical_alternative: None,
            },
            Token {
                text: Cow::Borrowed(" "),
                start: 3,
                end: 4,
                quoted: false,
                lexical_alternative: None,
            },
            Token {
                text: Cow::Borrowed("x"),
                start: 4,
                end: 5,
                quoted: false,
                lexical_alternative: None,
            },
        ],
        cursor: TokenCursor {
            position: 2,
            last_matched: Some(0),
            matched: 1,
        },
        datatype_span: None,
        store: &mut store,
        elements: 0,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        diagnostic_position: None,
        expected: String::new(),
    };
    run(&mut machine);
}
fn budget(error: ParseError) {
    assert!(
        matches!(error,ParseError::Budget(ref message) if message=="Execution budget exceeded")
    );
}

#[test]
fn owned_span_helper_exact_one_zero_precedence_and_actual_token_bounds() {
    with_machine(|m| {
        m.limits.steps_per_resource = Some(0);
        budget(m.owned_rule_interval(None).unwrap_err());
        budget(m.construction_offset(Some(999)).unwrap_err());
        assert_eq!(m.steps, 0);
        m.limits.steps_per_resource = Some(1);
        assert!(matches!(
            m.owned_rule_interval(None),
            Err(ParseError::Storage(_))
        ));
        assert_eq!(m.steps, 1);
    });
    with_machine(|m| {
        m.limits.steps_per_resource = Some(1);
        assert_eq!(m.owned_rule_interval(Some(2)).unwrap(), (3, 3));
        assert_eq!(m.steps, 1);
        budget(m.owned_rule_interval(Some(2)).unwrap_err());
    });
    with_machine(|m| {
        m.limits.steps_per_resource = Some(1);
        assert_eq!(m.owned_rule_interval(Some(0)).unwrap(), (0, 3));
        assert_eq!(m.steps, 1);
    });
    with_machine(|m| {
        m.limits.steps_per_resource = Some(1);
        m.cursor.last_matched = None;
        assert_eq!(m.owned_rule_interval(Some(2)).unwrap(), (0, 0));
        assert_eq!(m.steps, 1);
    });
    with_machine(|m| {
        m.limits.steps_per_resource = Some(1);
        assert_eq!(m.construction_offset(Some(2)).unwrap(), 4);
        assert_eq!(m.steps, 1);
    });
    with_machine(|m| {
        m.limits.steps_per_resource = Some(1);
        assert_eq!(m.construction_offset(Some(3)).unwrap(), 0);
        assert_eq!(m.steps, 1);
    });
    for (entry, last) in [(4, Some(0)), (0, Some(3))] {
        with_machine(|m| {
            m.limits.steps_per_resource = Some(1);
            m.cursor.last_matched = last;
            assert!(matches!(
                m.owned_rule_interval(Some(entry)),
                Err(ParseError::Storage(_))
            ));
            assert_eq!(m.steps, 1);
        });
    }
    with_machine(|m| {
        m.limits.steps_per_resource = Some(1);
        m.tokens[0].start = 7;
        assert!(
            matches!(m.owned_rule_interval(Some(0)),Err(ParseError::Storage(ref error)) if error=="Owned rule source interval is inverted")
        );
        assert_eq!(m.steps, 1);
    });
}

#[test]
fn owned_capture_two_fees_deny_before_each_effect_and_rollback_exactly() {
    for limit in 0..=2 {
        with_machine(|m| {
            let owner = m.create_current("Empty", 4).unwrap();
            assert_eq!(owner, 0);
            let before = storage::export(m.store).unwrap();
            let mark = m.checkpoint().unwrap();
            m.limits.steps_per_resource = Some(limit);
            let result = m.capture_current(Current::local(owner), "Root", "previous", false);
            if limit < 2 {
                budget(result.unwrap_err());
                assert_eq!(m.steps, limit);
                assert_eq!(m.elements, 1);
                let mut span = None;
                m.store
                    .inspect("span.model#0", &mut |o| {
                        span = Some((o.start(), o.end()));
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(
                    span,
                    Some(if limit == 0 { (4, 4) } else { (0, 3) }),
                    "first fee precedes sealing; second precedes wrapper"
                );
                m.rollback(mark).unwrap();
                assert_eq!(storage::export(m.store).unwrap(), before);
                assert_eq!(m.steps, limit, "rollback never refunds");
            } else {
                assert_eq!(result.unwrap(), 1);
                assert_eq!(m.steps, 2);
                assert_eq!(m.elements, 2);
                assert_eq!(
                    m.store.value("span.model#1", "previous").unwrap(),
                    Some(json!("span.model#0"))
                );
                m.commit_checkpoint(mark).unwrap();
            }
        });
    }
}

#[test]
fn delegated_capture_one_fee_preserves_final_child_and_current_origin_rollback() {
    for limit in 0..=1 {
        with_machine(|m| {
            m.create_current("Empty", 3).unwrap();
            let before = storage::export(m.store).unwrap();
            let mark = m.checkpoint().unwrap();
            m.limits.steps_per_resource = Some(limit);
            let result = m.capture_current(Current::called(0), "Root", "previous", false);
            if limit == 0 {
                budget(result.unwrap_err());
                assert_eq!(storage::export(m.store).unwrap(), before);
                m.rollback(mark).unwrap();
            } else {
                assert_eq!(result.unwrap(), 1);
                assert_eq!(m.steps, 1);
                let mut span = None;
                m.store
                    .inspect("span.model#0", &mut |o| {
                        span = Some((o.start(), o.end()));
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(span, Some((3, 3)));
                m.commit_checkpoint(mark).unwrap();
            }
        });
    }
}

#[test]
fn implicit_owner_decodes_entry_before_creation_and_restores_on_denial() {
    for limit in 0..=1 {
        with_machine(|m| {
            m.construction = Some(("Root", 2));
            m.limits.steps_per_resource = Some(limit);
            let mut owner = None;
            let result = m.assignment_owner(&mut owner);
            if limit == 0 {
                budget(result.unwrap_err());
                assert!(owner.is_none());
                assert_eq!(m.elements, 0);
            } else {
                assert_eq!(result.unwrap(), 0);
                assert_eq!(m.steps, 1);
                let mut span = None;
                m.store
                    .inspect("span.model#0", &mut |o| {
                        span = Some((o.start(), o.end()));
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(span, Some((4, 4)));
            }
        });
    }
}

// Type-only snapshots copied byte-for-byte from the sealed BEFORE declarations.
// They are never constructed and allocate no resident storage.

#[allow(dead_code)]
struct LegacyMachineLayout<'a, 'store, S: ModelStore> {
    document: Option<Box<document::Capture>>,
    document_origin: Option<&'a str>,
    document_feature: Option<(usize, &'a str)>,
    retained_tokens: bool,
    hidden_policy: &'a [String],
    raw_entry: bool,
    construction: Option<(&'a str, usize)>,
    settings: Option<&'a setting::TextSettings<'a>>,
    raw_source: &'a str,
    utf16_positions: Vec<usize>,
    datatype_text: Option<DatatypeText>,
    conversions: Option<&'a intersect_model::definitions::conversion::ConversionRegistry<'a>>,
    initializers: Option<&'a initialization::PackageInitializers>,
    lexical_types: &'a BTreeMap<String, BTreeMap<String, LexicalType>>,
    unsettable: &'a BTreeMap<String, BTreeSet<String>>,
    keywords: &'a BTreeSet<String>,
    undo: Vec<Undo<'a>>,
    limits: EvaluationLimits,
    package: &'a Package,
    uri: &'a str,
    tokens: Vec<Token<Cow<'a, str>>>,
    cursor: TokenCursor,
    datatype_span: Option<(usize, usize)>,
    store: &'store mut S,
    elements: usize,
    pending: Vec<Pending>,
    scalar_pending: BTreeMap<(usize, &'a str), usize>,
    steps: usize,
    depth: usize,
    // Preserve called-rule values in caller-generic fragment bodies. The model
    // API admits them against the actual owner; field-directed coercion would
    // incorrectly turn a numeric EString into an integer.
    generic_fragment: bool,
    farthest: usize,
    diagnostic_position: Option<usize>,
    expected: String,
    nested_decisions: NestedDecisions,
}
#[allow(dead_code)]
struct LegacyCurrentLayout {
    index: usize,
    delegated: bool,
}
#[allow(dead_code)]
struct LegacyCheckpointLayout {
    document: Option<document::Mark>,
    storage: storage::Checkpoint,
    text: Option<TextCheckpoint>,
    cursor: TokenCursor,
    datatype_span: Option<(usize, usize)>,
    elements: usize,
    undo: usize,
    pending: usize,
    depth: usize,
}
#[test]
fn owned_rule_span_native_layout_matches_exact_before_declarations() {
    use std::mem::{align_of, size_of};
    assert_eq!(
        size_of::<Machine<'_, '_, FrozenStore>>(),
        size_of::<LegacyMachineLayout<'_, '_, FrozenStore>>()
    );
    assert_eq!(
        align_of::<Machine<'_, '_, FrozenStore>>(),
        align_of::<LegacyMachineLayout<'_, '_, FrozenStore>>()
    );
    assert_eq!(size_of::<Current>(), size_of::<LegacyCurrentLayout>());
    assert_eq!(align_of::<Current>(), align_of::<LegacyCurrentLayout>());
    assert_eq!(size_of::<Checkpoint>(), size_of::<LegacyCheckpointLayout>());
    assert_eq!(
        align_of::<Checkpoint>(),
        align_of::<LegacyCheckpointLayout>()
    );
    println!(
        "V71_OWNED_SPAN_LAYOUT Machine={}/{} Current={}/{} Checkpoint={}/{}",
        size_of::<Machine<'_, '_, FrozenStore>>(),
        align_of::<Machine<'_, '_, FrozenStore>>(),
        size_of::<Current>(),
        align_of::<Current>(),
        size_of::<Checkpoint>(),
        align_of::<Checkpoint>()
    );
}

#[test]
fn ordinary_prediction_preserves_store_cursor_construction_and_object_ordinals() {
    with_machine(|m| {
        let source = "one x := y;";
        let grammar = m.package.grammar.as_ref().unwrap();
        let mut work = None;
        m.tokens = grammar::lex_borrowed_with_prediction(
            source,
            None,
            None,
            Some(&grammar.terminals),
            None,
            grammar.lexical_prediction.as_ref(),
            &mut work,
        )
        .unwrap();
        m.hidden_policy = grammar.hidden.as_deref().unwrap();
        m.raw_source = source;
        m.cursor = TokenCursor::default();
        let before = storage::export(m.store).unwrap();
        let cursor = m.cursor;
        let construction = m.construction;
        let choice = &grammar.rules["Root"].body;
        assert_eq!(m.predict(choice, "choice").unwrap(), Some(0));
        assert_eq!(storage::export(m.store).unwrap(), before);
        assert_eq!(m.cursor, cursor);
        assert_eq!(m.construction, construction);
        assert_eq!(m.elements, 0);
        assert!(m.undo.is_empty() && m.pending.is_empty());
    });
}

#[test]
fn legacy_capture_branch_charges_no_new_span_fee_at_zero_remaining_work() {
    // Private branch control only: interpreter profile selection is changed on
    // the same retained fixture IR. Public compilation/package parity is proven
    // separately from full source inputs and sealed whole BEFORE outputs.
    for profile in [
        grammar::Profile::NativeV65,
        grammar::Profile::NativeV66,
        grammar::Profile::NativeV67,
        grammar::Profile::NativeV68,
        grammar::Profile::NativeV69,
        grammar::Profile::NativeV70,
    ] {
        with_machine_profile(profile, |m| {
            m.create_current("Empty", 4).unwrap();
            m.limits.steps_per_resource = Some(0);
            assert_eq!(
                m.capture_current(Current::local(0), "Root", "previous", false)
                    .unwrap(),
                1
            );
            assert_eq!(m.steps, 0);
            let mut span = None;
            m.store
                .inspect("span.model#0", &mut |o| {
                    span = Some((o.start(), o.end()));
                    Ok(())
                })
                .unwrap();
            assert_eq!(
                span,
                Some((4, 3)),
                "old output preserved exactly, including historical inverted span"
            );
        });
    }
}
