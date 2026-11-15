//! Rollback controls exercise the interpreter independently of prediction checks.
use super::*;
use intersect_model::frozen::FrozenStore;

#[test]
fn unbudgeted_native_reuse_matches_budgeted_parsing_and_resource_rollback() {
    use crate::package::BundleCompilation;
    use intersect_model::core::storage::ModelRead;
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let model = DefinitionSource::new(
        "memory:reuse.rcore",
        SourceKind::Rcore,
        include_str!("../assets/fixtures/library/library-project.rcore"),
    );
    let mut grammar = DefinitionSource::new(
        "memory:reuse.xtext",
        SourceKind::Xtext,
        include_str!("../assets/fixtures/library/library.xtext"),
    );
    let mut common = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        include_str!(
            "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    );
    common.dependencies.push(model.id.clone());
    grammar.dependencies = vec![model.id.clone(), common.id.clone()];
    let package = ordinary::compile(
        BundleCompilation {
            materialized_types: vec![],
            ecore: model.id.clone(),
            namespace: "urn:intersect:example:library:1".into(),
            xtext: Some(grammar.id.clone()),
            rcore_behavior: model.id.clone(),
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![model, grammar, common],
                namespaces: vec![],
            },
        },
        grammar::Profile::NativeV82,
    )
    .unwrap();
    let finite = EvaluationLimits {
        steps_per_resource: Some(1_000_000),
        ..EvaluationLimits::native()
    };
    let repeated = (0..12)
        .map(|index| format!("copy b{index} \"É 😀\" edition 2; member m{index};"))
        .collect::<String>();
    for (body, valid) in [
        ("".to_owned(), true),
        ("member first; member second;".into(), true),
        (repeated.clone(), true),
        (format!("{repeated} loan l copy b0 to m0;"), true),
        (format!("{repeated} loan l copy missing to m0;"), false),
        (format!("{repeated} copy broken \"unterminated"), false),
        (format!("{repeated} member missing_semicolon"), false),
        (format!("{repeated} ?"), false),
    ] {
        let sources = [
            Source {
                uri: "memory:repeated.library".into(),
                text: format!("library Repeated {{ {body} }}"),
            },
            Source {
                uri: "memory:independent.library".into(),
                text: "library Independent { member survivor; }".into(),
            },
        ];
        // Finite budgets disable decision memoization and still perform every
        // canonical debit. Both paths must publish identical successful graphs,
        // diagnostics and reference spans, including after resource rollback.
        let reused = evaluate_stored(&package, &sources, EvaluationLimits::native()).unwrap();
        let budgeted = evaluate_stored(&package, &sources, finite).unwrap();
        assert_eq!(
            reused.evaluation.diagnostics.is_empty(),
            valid,
            "{body}: {:?}",
            reused.evaluation.diagnostics
        );
        assert_eq!(
            reused
                .store
                .query(storage::Query::Value("name", &json!("survivor")))
                .unwrap()
                .len(),
            1,
            "The independent resource must survive: {body}"
        );
        assert_eq!(
            storage::export(&reused.store).unwrap(),
            storage::export(&budgeted.store).unwrap(),
            "{body}"
        );
        assert_eq!(
            serde_json::to_value((
                reused.evaluation.diagnostics,
                reused.evaluation.references,
                reused.evaluation.derived,
            ))
            .unwrap(),
            serde_json::to_value((
                budgeted.evaluation.diagnostics,
                budgeted.evaluation.references,
                budgeted.evaluation.derived,
            ))
            .unwrap(),
            "{body}"
        );
    }
}

fn expr(kind: Kind) -> Expr {
    Expr {
        kind,
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '1',
    }
}
fn assign(feature: &str, many: bool, kind: Kind) -> Expr {
    expr(Kind::Assign {
        feature: feature.into(),
        many,
        value: Box::new(expr(kind)),
    })
}
fn library_store(package: &Package) -> FrozenStore {
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store.configure_schema(&package.schema).unwrap();
    store
        .create_element(&package.schema, "test#0", "Library", "test", 0, 0)
        .unwrap();
    store
}

#[test]
fn nested_rollback_restores_scalars_lists_elements_pending_and_identity_sequence() {
    nested_rollback_control(false);
}
#[test]
fn nested_rollback_restores_annotated_resource_metadata() {
    nested_rollback_control(true);
}
fn nested_rollback_control(annotated_resource: bool) {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = library_store(&package);
    store
        .set_value("test#0", "name", Some(json!("Original")))
        .unwrap();
    if annotated_resource {
        let mut record = store.resource_record("test").unwrap().unwrap();
        record.content_revision = u64::MAX - 128;
        record.diagnostics = vec!["retained diagnostic".into()];
        record.source = Some(intersect_model::core::resources::SourceCorrespondence {
            uri: "test".into(),
            revision: "before-speculation".into(),
        });
        store.set_resource_record(record).unwrap();
    }
    let before = storage::export(&store).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed("First Second member child; target", None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let checkpoint = machine.checkpoint().unwrap();
    // Repeated writes must unwind in reverse order; append rollback must restore
    // a preexisting list without cloning it. A new feature must disappear.
    let sequence = expr(Kind::Sequence(vec![
        assign("name", false, Kind::Call("ID".into())),
        assign("name", false, Kind::Call("ID".into())),
        assign("members", true, Kind::Call("Member".into())),
        assign("members", true, Kind::Reference("Member".into())),
    ]));
    machine
        .expr(&sequence, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(machine.elements, 2);
    assert_eq!(machine.pending.len(), 1);
    let inner = machine.checkpoint().unwrap();
    machine.rollback(inner).unwrap();
    assert_eq!(
        machine.store.value("test#0", "name").unwrap(),
        Some(json!("Second"))
    );
    machine.rollback(checkpoint).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert!(machine.pending.is_empty());
    assert!(machine.undo.is_empty());
    assert_eq!(machine.cursor.position, 0);
    assert_eq!(machine.depth, 0);
    machine.cursor.position = 2;
    assert_eq!(machine.rule("Member").unwrap(), json!("test#1"));
    assert_eq!(
        machine.store.value("test#1", "name").unwrap(),
        Some(json!("child"))
    );
}

#[test]
fn failed_optional_sequence_rolls_back_successful_children() {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = library_store(&package);
    let before = storage::export(&store).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed("member child; unexpected", None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let optional = Expr {
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '?',
        kind: Kind::Sequence(vec![
            assign("members", true, Kind::Call("Member".into())),
            expr(Kind::Literal("required".into())),
        ]),
    };
    machine
        .expr(&optional, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert_eq!(machine.cursor.position, 0);
    assert_eq!(machine.depth, 0);
    assert!(machine.undo.is_empty());
    assert!(machine.expected.contains("required"));
    assert_eq!(machine.farthest, 3);
    let choice = expr(Kind::Alternative(vec![
        Expr {
            first_set_predicated: false,
            original_range: None,
            prediction_path: None,
            cardinality: '1',
            kind: optional.kind.clone(),
        },
        expr(Kind::Sequence(vec![
            assign("members", true, Kind::Call("Member".into())),
            expr(Kind::Literal("unexpected".into())),
        ])),
    ]));
    machine.expr(&choice, &mut Some(Current::local(0))).unwrap();
    assert_eq!(machine.elements, 2);
    assert_eq!(
        machine.store.value("test#0", "members").unwrap(),
        Some(json!(["test#1"]))
    );
    assert!(machine.store.get("test#1").unwrap().is_some());
    assert_eq!(machine.cursor.position, 4);
}

#[test]
fn repeated_appends_keep_reference_journal_empty_and_outer_rollback_restores_the_list() {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = library_store(&package);
    let input = "member child; ".repeat(2000);
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed(&input, None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let outer = machine.checkpoint().unwrap();
    let mut repeated = assign("members", true, Kind::Call("Member".into()));
    repeated.cardinality = '*';
    machine
        .expr(&repeated, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(machine.elements, 2001);
    assert!(machine.undo.is_empty());
    machine.rollback(outer).unwrap();
    assert_eq!(machine.elements, 1);
    assert_eq!(
        machine.store.value("test#0", "members").unwrap(),
        Some(json!([]))
    );
    assert_eq!(machine.cursor.position, 0);
    assert!(machine.undo.is_empty());
}

#[test]
fn scalar_reference_rollback_restores_unresolved_state_and_spelling() {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store.configure_schema(&package.schema).unwrap();
    store
        .create_element(&package.schema, "test#0", "Loan", "test", 0, 0)
        .unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed("First Second", None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let assignment = assign("copy", false, Kind::Reference("BookCopy".into()));
    machine
        .expr(&assignment, &mut Some(Current::local(0)))
        .unwrap();
    let checkpoint = machine.checkpoint().unwrap();
    let first = storage::export(machine.store).unwrap();
    machine
        .expr(&assignment, &mut Some(Current::local(0)))
        .unwrap();
    assert!(!machine.pending[0].active);
    assert!(machine.pending[1].active);
    machine.rollback(checkpoint).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), first);
    assert_eq!(machine.pending.len(), 1);
    assert_eq!(machine.pending[0].spelling, "First");
    assert!(machine.pending[0].active);
    assert_eq!(machine.scalar_pending[&(0, "copy")], 0);
    let optional = Expr {
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '?',
        kind: Kind::Sequence(vec![
            assignment.clone(),
            expr(Kind::Literal("required".into())),
        ]),
    };
    machine
        .expr(&optional, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), first);
    assert_eq!(machine.cursor.position, 1);
    assert_eq!(machine.pending.len(), 1);
    assert!(machine.pending[0].active);
    assert_eq!(machine.scalar_pending[&(0, "copy")], 0);
}

#[test]
fn speculative_assignments_restore_unsettable_markers_and_pending_reference_state() {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/ecore-unsettable/model.ecore"),
        include_str!("../assets/fixtures/ecore-unsettable/model.xtext"),
        include_str!("../assets/fixtures/ecore-unsettable/model-behavior.rcore"),
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store
        .create_element(&package.schema, "test#0", "Item", "test", 0, 0)
        .unwrap();
    let before = storage::export(&store).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::from([(
        "Item".into(),
        BTreeSet::from(["value".into(), "tags".into(), "link".into()]),
    )]);
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed(r#""seed" "x" target"#, None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let optional = Expr {
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '?',
        kind: Kind::Sequence(vec![
            assign("value", false, Kind::Call("STRING".into())),
            expr(Kind::Literal("missing".into())),
        ]),
    };
    machine
        .expr(&optional, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(machine.cursor.position, 0);
    assert_eq!(storage::export(machine.store).unwrap(), before);
    let checkpoint = machine.checkpoint().unwrap();
    let scalar = assign("value", false, Kind::Call("STRING".into()));
    machine.expr(&scalar, &mut Some(Current::local(0))).unwrap();
    let scalar_state = storage::export(machine.store).unwrap();
    assert!(
        scalar_state.elements[0]
            .assignment_state
            .as_ref()
            .unwrap()
            .explicitly_set("value")
    );
    let inner = machine.checkpoint().unwrap();
    let tail = expr(Kind::Sequence(vec![
        assign("tags", true, Kind::Call("STRING".into())),
        assign("link", false, Kind::Reference("Item".into())),
    ]));
    machine.expr(&tail, &mut Some(Current::local(0))).unwrap();
    assert_eq!(machine.pending.len(), 1);
    machine.rollback(inner).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), scalar_state);
    assert!(machine.pending.is_empty());
    machine.rollback(checkpoint).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert!(machine.undo.is_empty());
}

#[test]
fn boolean_keyword_rollback_restores_values_markers_and_token_position() {
    let package = compile_migrated_fixture_with_profile(
        include_str!("../assets/fixtures/xtext/boolean-keywords/model.ecore"),
        include_str!("../assets/fixtures/xtext/boolean-keywords/model.xtext"),
        include_str!("../assets/fixtures/xtext/boolean-keywords/model-behavior.rcore"),
        grammar::Profile::NativeV1,
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store
        .create_element(&package.schema, "test#0", "Type", "test", 0, 0)
        .unwrap();
    let before = storage::export(&store).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::from([(
        "Type".into(),
        BTreeSet::from(["isAbstract".into(), "isStandard".into()]),
    )]);
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed("abstract standard", None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let abstract_flag = expr(Kind::BooleanAssign {
        feature: "isAbstract".into(),
        value: Box::new(expr(Kind::Literal("abstract".into()))),
    });
    let standard_flag = expr(Kind::BooleanAssign {
        feature: "isStandard".into(),
        value: Box::new(expr(Kind::Literal("standard".into()))),
    });
    let optional = Expr {
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '?',
        kind: Kind::Sequence(vec![
            abstract_flag.clone(),
            standard_flag,
            expr(Kind::Literal("missing".into())),
        ]),
    };
    machine
        .expr(&optional, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(machine.cursor.position, 0);
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert!(machine.undo.is_empty());
    let checkpoint = machine.checkpoint().unwrap();
    machine
        .expr(&abstract_flag, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(
        machine.store.value("test#0", "isAbstract").unwrap(),
        Some(json!(true))
    );
    assert!(
        storage::export(machine.store).unwrap().elements[0]
            .assignment_state
            .as_ref()
            .unwrap()
            .explicitly_set("isAbstract")
    );
    machine.rollback(checkpoint).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert_eq!(machine.cursor.position, 0);
    assert_eq!(machine.elements, 1);
}

#[test]
fn fragment_optional_rollback_restores_caller_fields_pending_and_identity() {
    fragment_rollback(false);
}
#[test]
fn wildcard_rollback_restores_generic_mode_caller_fields_and_identity() {
    fragment_rollback(true);
}
fn fragment_rollback(generic: bool) {
    let mut package = compile_migrated_fixture_with_profile(
        include_str!("../assets/fixtures/xtext/typed-fragments/model.ecore"),
        include_str!("../assets/fixtures/xtext/typed-fragments/model.xtext"),
        include_str!("../assets/fixtures/xtext/typed-fragments/model-behavior.rcore"),
        grammar::Profile::NativeV1,
    )
    .unwrap();
    if generic {
        // Programmatic IR isolates VM call-frame restoration from source
        // compilation. Valid wildcard declarations have no return classifier.
        let grammar = package.grammar.as_mut().unwrap();
        for name in ["Identification", "Target"] {
            let rule = grammar.rules.get_mut(name).unwrap();
            rule.wildcard = true;
            rule.class.clear();
        }
        let mut failure = grammar.rules["Identification"].clone();
        failure.body = expr(Kind::Sequence(vec![
            failure.body.clone(),
            expr(Kind::Literal("missing".into())),
        ]));
        grammar.rules.insert("GenericFailure".into(), failure);
    }
    let words = keywords(&package).into_iter().collect();
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store
        .create_element(&package.schema, "test#0", "Feature", "test", 0, 0)
        .unwrap();
    let before = storage::export(&store).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::from([(
        "Feature".into(),
        BTreeSet::from(["name".into(), "target".into()]),
    )]);
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed("wheel Vehicle", None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let identification = expr(Kind::Call("Identification".into()));
    let target = expr(Kind::Call("Target".into()));
    let optional = Expr {
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '?',
        kind: Kind::Sequence(vec![
            identification.clone(),
            target.clone(),
            expr(Kind::Literal("missing".into())),
        ]),
    };
    machine
        .expr(&optional, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert_eq!(machine.cursor.position, 0);
    assert_eq!(machine.depth, 0);
    assert!(!machine.generic_fragment);
    assert_eq!(machine.elements, 1);
    assert!(machine.pending.is_empty());
    assert!(machine.scalar_pending.is_empty());
    assert!(machine.undo.is_empty());
    let checkpoint = machine.checkpoint().unwrap();
    machine
        .expr(&identification, &mut Some(Current::local(0)))
        .unwrap();
    machine.expr(&target, &mut Some(Current::local(0))).unwrap();
    assert_eq!(
        machine.store.value("test#0", "name").unwrap(),
        Some(json!("wheel"))
    );
    assert_eq!(machine.pending.len(), 1);
    assert_eq!(&*machine.pending[0].owner, "test#0");
    assert_eq!(machine.pending[0].spelling, "Vehicle");
    assert_eq!(machine.elements, 1);
    machine.rollback(checkpoint).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert!(machine.pending.is_empty());
    assert!(machine.scalar_pending.is_empty());
    if generic {
        let mut optional = expr(Kind::Call("GenericFailure".into()));
        optional.cardinality = '?';
        machine
            .expr(&optional, &mut Some(Current::local(0)))
            .unwrap();
        assert_eq!(storage::export(machine.store).unwrap(), before);
        assert_eq!(machine.cursor.position, 0);
        assert_eq!(machine.depth, 0);
        assert!(!machine.generic_fragment);
        assert!(machine.undo.is_empty());
    }
}

#[test]
fn speculative_scalar_group_restores_raw_text_and_utf16_cursor() {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = library_store(&package);
    let before = storage::export(&store).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let raw = "P :: A";
    let punctuation = grammar::PunctuationKeywords::new(["::"]);
    let tokens = grammar::lex_borrowed_with_keywords(raw, None, Some(&punctuation)).unwrap();
    let positions =
        datatype_text::utf16_offsets(raw, tokens.iter().map(|token| token.start)).unwrap();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: raw,
        utf16_positions: positions,
        datatype_text: Some(DatatypeText::default()),
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens,
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let first = expr(Kind::Call("ID".into()));
    machine.expr(&first, &mut Some(Current::local(0))).unwrap();
    let scalar_before = machine.datatype_text.as_ref().unwrap().checkpoint();
    let mut optional = expr(Kind::Sequence(vec![
        expr(Kind::Literal("::".into())),
        expr(Kind::Call("ID".into())),
        expr(Kind::Literal("never".into())),
    ]));
    optional.cardinality = '?';
    machine
        .expr(&optional, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(machine.cursor.position, 1);
    assert_eq!(
        machine.datatype_text.as_ref().unwrap().checkpoint(),
        scalar_before
    );
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert!(machine.pending.is_empty());
    let remaining = expr(Kind::Sequence(vec![
        expr(Kind::Literal("::".into())),
        expr(Kind::Call("ID".into())),
    ]));
    machine
        .expr(&remaining, &mut Some(Current::local(0)))
        .unwrap();
    assert_eq!(
        machine.datatype_text.take().unwrap().into_text().as_deref(),
        Some("P :: A")
    );
    assert_eq!(machine.elements, 1);
}

#[test]
fn speculative_capture_restores_current_wrapper_span_graph_and_ordinals() {
    let package = compile_migrated_fixture_with_profile(
        include_str!("../assets/fixtures/xtext/current-actions/model.ecore"),
        include_str!("../assets/fixtures/xtext/current-actions/model.xtext"),
        include_str!("../assets/fixtures/xtext/current-actions/model-behavior.rcore"),
        grammar::Profile::NativeV5,
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store.configure_schema(&package.schema).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let tokens = grammar::lex_borrowed("1 + 2 - 3", None).unwrap();
    let positions =
        datatype_text::utf16_offsets("1 + 2 - 3", tokens.iter().map(|token| token.start)).unwrap();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "1 + 2 - 3",
        utf16_positions: positions,
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits {
            steps_per_resource: Some(1000),
            ..EvaluationLimits::native()
        },
        package: &package,
        keywords: &words,
        uri: "test",
        tokens,
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 0,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    assert_eq!(machine.rule("Literal").unwrap(), json!("test#0"));
    let mut current = Some(Current::local(0));
    let capture = expr(Kind::Capture {
        class: "OperatorExpression".into(),
        feature: "operand".into(),
        many: true,
    });
    let fold = expr(Kind::Sequence(vec![
        capture.clone(),
        assign("operator", false, Kind::Call("AdditiveOperator".into())),
        assign("operand", true, Kind::Call("Literal".into())),
    ]));
    machine.expr(&fold, &mut current).unwrap();
    assert_eq!(current.map(|current| current.index), Some(1));
    assert_eq!(machine.elements, 3);
    let before = storage::export(machine.store).unwrap();
    let undo = machine.undo.len();
    let steps = machine.steps;
    let mut optional = expr(Kind::Sequence(vec![
        capture,
        expr(Kind::Literal("-".into())),
        expr(Kind::Literal("!".into())),
    ]));
    optional.cardinality = '?';
    machine.expr(&optional, &mut current).unwrap();
    assert_eq!(current.map(|current| current.index), Some(1));
    assert_eq!(machine.elements, 3);
    assert_eq!(machine.cursor.position, 3);
    assert_eq!(machine.depth, 0);
    assert_eq!(machine.undo.len(), undo);
    assert!(machine.steps > steps);
    assert!(machine.pending.is_empty());
    assert!(machine.scalar_pending.is_empty());
    assert_eq!(storage::export(machine.store).unwrap(), before);
    machine.expr(&fold, &mut current).unwrap();
    assert_eq!(current.map(|current| current.index), Some(3));
    assert_eq!(machine.elements, 5);
    assert_eq!(
        machine.store.value("test#3", "operand").unwrap(),
        Some(json!(["test#1", "test#4"]))
    );
    let model = storage::export(machine.store).unwrap();
    let folded = model
        .elements
        .iter()
        .find(|element| element.id == "test#1")
        .unwrap();
    assert_eq!((folded.start, folded.end), (0, 5));
}

#[test]
fn storage_savepoints_restore_provider_side_effects_after_nested_commit() {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let words = keywords(&package).into_iter().collect();
    let mut store = library_store(&package);
    store
        .set_value("test#0", "name", Some(json!("Original")))
        .unwrap();
    let before = storage::export(&store).unwrap();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: grammar::lex_borrowed("First Second member child; target", None).unwrap(),
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 1,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };

    let checkpoint = machine.checkpoint().unwrap();
    machine
        .store
        .set_value("test#0", "name", Some(json!("provider write")))
        .unwrap();
    let inner = machine.checkpoint().unwrap();
    machine
        .store
        .create_element(&package.schema, "provider#aux", "Member", "test", 0, 0)
        .unwrap();
    machine
        .store
        .set_value("provider#aux", "name", Some(json!("helper")))
        .unwrap();
    machine.commit_checkpoint(inner).unwrap();
    machine.cursor.position = 2;
    machine.rollback(checkpoint).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert_eq!(machine.cursor.position, 0);
    assert_eq!(machine.elements, 1);
    // Every successful or rejected expression must close its storage frame.
    let expressions = [
        assign("name", false, Kind::Call("ID".into())),
        expr(Kind::Sequence(vec![
            assign("name", false, Kind::Call("ID".into())),
            expr(Kind::Literal("missing".into())),
        ])),
        Expr {
            cardinality: '*',
            ..expr(Kind::Sequence(vec![]))
        },
    ];
    for expression in &expressions {
        let outer = machine.store.begin().unwrap();
        let _result = machine.expr(expression, &mut Some(Current::local(0)));
        machine.store.rollback(&outer).unwrap();
        assert_eq!(storage::export(machine.store).unwrap(), before);
        machine.cursor.position = 0;
        machine.undo.clear();
    }
}

#[test]
fn resource_records_preserve_source_correspondence_diagnostics_and_empty_attempts() {
    let package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let sources = [
        Source {
            uri: "good.library".into(),
            text: "library Good {}".into(),
        },
        Source {
            uri: "empty.library".into(),
            text: String::new(),
        },
    ];
    let result = evaluate(&package, &sources);
    let good = result.model.resources.get("good.library").unwrap();
    let identity =
        intersect_model::definitions::bundle::SourceId::for_text(&sources[0].uri, &sources[0].text);
    assert_eq!(good.source.as_ref().unwrap().revision, identity.sha256);
    assert!(good.diagnostics.is_empty());
    let empty = result.model.resources.get("empty.library").unwrap();
    assert!(empty.roots.is_empty());
    assert!(!empty.diagnostics.is_empty());
    assert!(empty.source.is_none());
    assert_eq!(
        empty.load_state,
        intersect_model::core::resources::ResourceLoadState::Loaded
    );
    assert!(!result.diagnostics.is_empty());
}

#[test]
fn speculative_default_containment_restores_foreign_metadata_at_revision_limit() {
    use intersect_model::core::resources::{ResourceRecord, SourceCorrespondence};
    let mut package = compile_migrated_fixture(
        include_str!("../assets/fixtures/library/library.ecore"),
        include_str!("../assets/fixtures/library/library.xtext"),
        include_str!("../assets/fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    package
        .schema
        .classes
        .get_mut("Library")
        .unwrap()
        .features
        .get_mut("members")
        .unwrap()
        .default = json!(["foreign#0"]);
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    store.begin().unwrap();
    store.configure_schema(&package.schema).unwrap();
    store
        .create_element(&package.schema, "foreign#0", "Member", "foreign", 0, 0)
        .unwrap();
    store
        .set_value("foreign#0", "name", Some(json!("Retained")))
        .unwrap();
    let mut foreign = store.resource_record("foreign").unwrap().unwrap();
    foreign.content_revision = u64::MAX - 1;
    foreign.diagnostics = vec!["retained foreign diagnostic".into()];
    foreign.source = Some(SourceCorrespondence {
        uri: "foreign".into(),
        revision: "foreign-before-speculation".into(),
    });
    store.set_resource_record(foreign).unwrap();
    let mut empty = ResourceRecord::new("test").unwrap();
    empty.diagnostics = vec!["retained empty resource diagnostic".into()];
    empty.source = Some(SourceCorrespondence {
        uri: "test".into(),
        revision: "empty-before-speculation".into(),
    });
    store.set_resource_record(empty).unwrap();
    let before = storage::export(&store).unwrap();
    let words = keywords(&package).into_iter().collect();
    let lexical_types = BTreeMap::new();
    let unsettable = BTreeMap::new();
    let mut machine = Machine {
        nested_decisions: Default::default(),
        document: None,
        document_origin: None,
        document_feature: None,
        raw_entry: false,
        construction: None,
        settings: None,
        raw_source: "",
        utf16_positions: Vec::new(),
        datatype_text: None,
        initializers: None,
        conversions: None,
        lexical_types: &lexical_types,
        unsettable: &unsettable,
        limits: EvaluationLimits::native(),
        package: &package,
        keywords: &words,
        uri: "test",
        tokens: vec![],
        retained_tokens: false,
        hidden_policy: &[],
        cursor: TokenCursor::default(),
        datatype_span: None,
        diagnostic_position: None,
        store: &mut store,
        elements: 0,
        pending: vec![],
        scalar_pending: BTreeMap::new(),
        undo: vec![],
        steps: 0,
        depth: 0,
        generic_fragment: false,
        farthest: 0,
        expected: String::new(),
    };
    let outer = machine.checkpoint().unwrap();
    let inner = machine.checkpoint().unwrap();
    assert_eq!(machine.create_current("Library", 0).unwrap(), 0);
    let changed = machine.store.resource_record("foreign").unwrap().unwrap();
    assert!(changed.roots.is_empty());
    assert_eq!(changed.content_revision, u64::MAX);
    assert!(changed.source.is_none());
    assert!(changed.diagnostics.is_empty());
    machine.commit_checkpoint(inner).unwrap();
    machine.rollback(outer).unwrap();
    assert_eq!(storage::export(machine.store).unwrap(), before);
    assert_eq!(machine.elements, 0);
    assert!(machine.undo.is_empty());
}
