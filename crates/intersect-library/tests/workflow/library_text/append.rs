use super::*;
use intersect_model::{
    core::model::Model,
    core::text_document::{NodeId, NodeKind},
};
use intersect_text::source_edit::{
    AppendPlacement, AppendTarget, PreparedSourceEdit, SourceInsertion,
};
use serde_json::Value;

const CATALOG: &str = "// catalog 😀 unchanged\r\nlibrary Catalog { copy dune \"Dune\"; copy earthsea \"Earthsea\"; copy foundation \"Foundation\"; member ada; }\r\n";

fn authored(loans: &str) -> SourceEditSession {
    SourceEditSession::new(
        library::language().unwrap(),
        &[
            Source {
                uri: "catalog.library".into(),
                text: CATALOG.into(),
            },
            Source {
                uri: "loans.library".into(),
                text: loans.into(),
            },
        ],
        EvaluationLimits::workbench(),
    )
    .unwrap()
}

fn new_loan(session: &SourceEditSession, id: &str, name: &str, copy: &str) -> Model {
    let schema = &session.package().schema;
    let owner = library::named(session.model(), "Circulation").unwrap();
    let mut children = owner.values["loans"].as_array().unwrap().clone();
    children.push(json!(id));
    let mut candidate = session.model().clone();
    candidate
        .create(schema, id, "Loan", &owner.resource, 0, 0)
        .unwrap();
    candidate
        .edit(
            schema,
            &[
                (id.into(), "name".into(), json!(name)),
                (
                    id.into(),
                    "copy".into(),
                    json!(library::named(session.model(), copy).unwrap().id),
                ),
                (
                    id.into(),
                    "member".into(),
                    json!(library::named(session.model(), "ada").unwrap().id),
                ),
                (owner.id.clone(), "loans".into(), Value::Array(children)),
            ],
        )
        .unwrap();
    candidate
}

fn target(session: &SourceEditSession) -> AppendTarget<'_> {
    AppendTarget {
        owner: &library::named(session.model(), "Circulation").unwrap().id,
        feature: "loans",
        rule: "Loan",
    }
}

fn plan(session: &SourceEditSession, candidate: &Model) -> Result<PreparedSourceEdit, String> {
    session.plan_append_with(
        candidate,
        target(session),
        Default::default(),
        library::terminal,
        library::append,
    )
}

fn keyword(
    value: &AppendPlacement<'_>,
    text: &str,
) -> intersect_model::core::text_document::NodeHandle {
    let document = value.snapshot.document();
    let index = document
        .nodes()
        .iter()
        .enumerate()
        .find(|(_, node)| {
            node.kind == NodeKind::Leaf
                && !node.data.hidden
                && document.text_in(node.range).unwrap() == Some(text)
        })
        .unwrap()
        .0;
    document.handle(NodeId(index)).unwrap()
}

#[test]
fn append_loan_preserves_every_existing_byte_and_supports_later_scalar_edits() {
    let original = "// loans 😀\r\nlibrary Circulation {\r\n\tloan first copy dune to ada; // keep first\r\n\t// trailing note stays here\r\n}\r\n";
    let mut session = authored(original);
    let candidate = new_loan(&session, "api:new-loan", "second", "earthsea");
    let prepared = plan(&session, &candidate).unwrap();
    assert_eq!(prepared.patches().len(), 1);
    let patch = &prepared.patches()[0];
    assert_eq!(patch.range.start, patch.range.end);
    assert_eq!(patch.original, "");
    assert_eq!(patch.replacement, "\tloan second copy earthsea to ada;\r\n");
    let expected = format!(
        "{}{}{}",
        &original[..patch.range.start],
        patch.replacement,
        &original[patch.range.start..]
    );
    assert_eq!(prepared.sources()[1].text, expected);
    assert_eq!(prepared.sources()[0].text, CATALOG);
    assert_eq!(
        prepared.parsed_identities()["api:new-loan"],
        "loans.library#2"
    );
    let before = session.model().clone();
    session.adopt(prepared).unwrap();
    for old in &before.elements {
        let after = session
            .model()
            .elements
            .iter()
            .find(|e| e.id == old.id)
            .unwrap();
        assert_eq!(after.class, old.class);
    }
    assert_eq!(
        session.model().resources["catalog.library"],
        before.resources["catalog.library"]
    );
    assert!(
        session.model().resources["loans.library"].content_revision
            > candidate.resources["loans.library"].content_revision
    );
    let snapshot = session.snapshot("loans.library").unwrap();
    let occurrence = snapshot
        .feature_nodes()
        .iter()
        .find(|f| f.owner == "api:new-loan" && f.feature == "name")
        .unwrap();
    let handle = snapshot.document().handle(occurrence.node).unwrap();
    let schema = library::language().unwrap().package().schema.clone();
    assert_eq!(
        snapshot
            .linked_object(&handle, session.model(), &schema)
            .unwrap()
            .unwrap()
            .id,
        "api:new-loan"
    );
    // Parse-stage evidence keeps the actual source-derived identity.
    assert!(
        snapshot
            .parse_stage()
            .objects
            .iter()
            .any(|e| e.id == "loans.library#2")
    );
    let mut edited = session.model().clone();
    edited
        .edit(
            &schema,
            &[("api:new-loan".into(), "name".into(), json!("renamed"))],
        )
        .unwrap();
    let scalar = session.plan_with(&edited, library::scalar).unwrap();
    session.adopt(scalar).unwrap();
    assert_eq!(
        session.sources()[1].text,
        expected.replace("loan second ", "loan renamed ")
    );
}

#[test]
fn append_handles_empty_and_inline_lists_with_exact_noops_after_adoption() {
    for original in [
        "library Circulation {\n}\n",
        "library Circulation { } // keep\n",
    ] {
        let mut session = authored(original);
        let candidate = new_loan(&session, "api:new", "first", "earthsea");
        let prepared = plan(&session, &candidate).unwrap();
        let patch = &prepared.patches()[0];
        assert_eq!(
            prepared.sources()[1].text,
            format!(
                "{}{}{}",
                &original[..patch.range.start],
                patch.replacement,
                &original[patch.range.start..]
            )
        );
        session.adopt(prepared).unwrap();
        let model = session.model().clone();
        let sources = serde_json::to_value(session.sources()).unwrap();
        let noop = session
            .plan_with(&model, |_| panic!("No-op requested spelling"))
            .unwrap();
        assert!(session.adopt(noop).unwrap().is_empty());
        assert_eq!(session.model(), &model);
        assert_eq!(serde_json::to_value(session.sources()).unwrap(), sources);
    }
}

#[test]
fn append_reconciles_shifted_ordinals_and_colliding_parser_ids_simultaneously() {
    let grammar = include_str!("../../../../intersect-text/assets/fixtures/library/library.xtext")
        .replace(
            "(copies+=BookCopy | members+=Member | loans+=Loan)*",
            "(copies+=BookCopy | loans+=Loan)* 'members' members+=Member*",
        );
    let catalog = CATALOG.replace("member ada;", "members member ada;");
    let original = "library Circulation { loan first copy dune to bob; members member bob; }";
    let mut session = SourceEditSession::new(
        library::language_with_grammar(&grammar).unwrap(),
        &[
            Source {
                uri: "catalog.library".into(),
                text: catalog,
            },
            Source {
                uri: "loans.library".into(),
                text: original.into(),
            },
        ],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let bob = library::named(session.model(), "bob").unwrap().id.clone();
    assert_eq!(bob, "loans.library#2");
    for (id, name, copy) in [
        ("loans.library#3", "second", "earthsea"),
        ("api:third", "third", "foundation"),
    ] {
        // The new stable ID can equal a different object's future parser ID.
        let candidate = new_loan(&session, id, name, copy);
        let prepared = session
            .plan_append_with(
                &candidate,
                target(&session),
                Default::default(),
                library::terminal,
                |value| {
                    Ok(SourceInsertion {
                        before: keyword(&value, "members"),
                        text: format!("{} ", value.declaration.trim()),
                    })
                },
            )
            .unwrap();
        assert_ne!(prepared.parsed_identities()[&bob], bob);
        session.adopt(prepared).unwrap();
        assert_eq!(library::named(session.model(), "bob").unwrap().id, bob);
        assert_eq!(library::named(session.model(), name).unwrap().id, id);
        assert_eq!(
            library::named(session.model(), "first").unwrap().values["member"],
            json!(bob)
        );
        let updated = &session.sources()[1].text;
        assert!(updated.contains("members member bob;"));
    }
    assert_eq!(session.parsed_identities()[&bob], "loans.library#4");
    let schema = library::language_with_grammar(&grammar)
        .unwrap()
        .package()
        .schema
        .clone();
    let snapshot = session.snapshot("loans.library").unwrap();
    let field = snapshot
        .feature_nodes()
        .iter()
        .find(|f| f.owner == bob && f.feature == "name")
        .unwrap();
    let handle = snapshot.document().handle(field.node).unwrap();
    assert_eq!(
        snapshot
            .linked_object(&handle, session.model(), &schema)
            .unwrap()
            .unwrap()
            .values["name"],
        json!("bob")
    );
}

#[test]
fn append_rejects_semantic_loss_injection_and_callback_errors_without_changing_baseline() {
    let session = authored("library Circulation { loan first copy dune to ada; }");
    let before = session.model().clone();
    let candidate = new_loan(&session, "api:second", "second", "earthsea");
    for text in [
        "loan second copy foundation to ada; ",
        "loan second copy earthsea to ada; member injected; ",
        "garbage ",
    ] {
        assert!(
            session
                .plan_append_with(
                    &candidate,
                    target(&session),
                    Default::default(),
                    library::terminal,
                    |value| Ok(SourceInsertion {
                        before: keyword(&value, "}"),
                        text: text.into()
                    })
                )
                .is_err()
        );
        assert_eq!(session.model(), &before);
    }
    assert!(
        session
            .plan_append_with(
                &candidate,
                target(&session),
                Default::default(),
                library::terminal,
                |_| Err("No placement policy".into())
            )
            .is_err()
    );
    assert!(
        session
            .plan_append_with(
                &candidate,
                target(&session),
                Default::default(),
                |_| Err("No spelling policy".into()),
                library::append
            )
            .is_err()
    );
    let invalid = new_loan(&session, "api:second", "second", "dune");
    assert!(plan(&session, &invalid).is_err());
    assert_eq!(session.model(), &before);
}

#[test]
fn append_rejects_foreign_or_child_anchors_and_stale_publication() {
    let mut session = authored("library Circulation { loan first copy dune to ada; }");
    let candidate = new_loan(&session, "api:second", "second", "earthsea");
    let foreign = authored("library Circulation { loan first copy dune to ada; }");
    let foreign_anchor = foreign
        .snapshot("loans.library")
        .unwrap()
        .document()
        .handle(NodeId(0))
        .unwrap();
    assert!(
        session
            .plan_append_with(
                &candidate,
                target(&session),
                Default::default(),
                library::terminal,
                |_| Ok(SourceInsertion {
                    before: foreign_anchor,
                    text: "loan second copy earthsea to ada; ".into()
                })
            )
            .is_err()
    );
    assert!(
        session
            .plan_append_with(
                &candidate,
                target(&session),
                Default::default(),
                library::terminal,
                |value| Ok(SourceInsertion {
                    before: keyword(&value, "loan"),
                    text: value.declaration.into()
                })
            )
            .is_err()
    );
    let first = plan(&session, &candidate).unwrap();
    let stale = plan(&session, &candidate).unwrap();
    session.adopt(first).unwrap();
    let updated = session.model().clone();
    assert!(session.adopt(stale).is_err());
    assert_eq!(session.model(), &updated);
}

#[test]
fn append_rejects_other_model_changes_before_requesting_spellings() {
    let session = authored("library Circulation { loan first copy dune to ada; }");
    let schema = library::language().unwrap().package().schema.clone();
    let mut candidate = new_loan(&session, "api:second", "second", "earthsea");
    candidate
        .edit(
            &schema,
            &[(
                library::named(session.model(), "dune").unwrap().id.clone(),
                "title".into(),
                json!("Also changed"),
            )],
        )
        .unwrap();
    assert!(
        session
            .plan_append_with(
                &candidate,
                target(&session),
                Default::default(),
                |_| panic!("Unsupported change reached spelling"),
                library::append
            )
            .is_err()
    );
    let mut candidate = new_loan(&session, "api:second", "second", "earthsea");
    candidate
        .create(&schema, "api:extra", "Member", "loans.library", 0, 0)
        .unwrap();
    assert!(
        session
            .plan_append_with(
                &candidate,
                target(&session),
                Default::default(),
                |_| panic!("Multiple objects reached spelling"),
                library::append
            )
            .is_err()
    );
}

#[test]
fn append_rejects_transfer_of_an_existing_handle_to_an_identical_new_declaration() {
    let grammar = include_str!("../../../../intersect-text/assets/fixtures/library/library.xtext")
        .replace(
            "(copies+=BookCopy | members+=Member | loans+=Loan)*",
            "('members' members+=Member)*",
        );
    let session = SourceEditSession::new(
        library::language_with_grammar(&grammar).unwrap(),
        &[Source {
            uri: "loans.library".into(),
            text: "library Circulation { members member spare; }".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let schema = library::language_with_grammar(&grammar)
        .unwrap()
        .package()
        .schema
        .clone();
    let owner = library::named(session.model(), "Circulation").unwrap();
    let old = library::named(session.model(), "spare").unwrap();
    let mut candidate = session.model().clone();
    candidate
        .create(&schema, "api:spare", "Member", "loans.library", 0, 0)
        .unwrap();
    candidate
        .edit(
            &schema,
            &[
                ("api:spare".into(), "name".into(), json!("spare")),
                (
                    owner.id.clone(),
                    "members".into(),
                    json!([old.id, "api:spare"]),
                ),
            ],
        )
        .unwrap();
    let error = session
        .plan_append_with(
            &candidate,
            AppendTarget {
                owner: &owner.id,
                feature: "members",
                rule: "Member",
            },
            Default::default(),
            library::terminal,
            |value| {
                Ok(SourceInsertion {
                    before: keyword(&value, "members"),
                    text: format!("members {} ", value.declaration.trim()),
                })
            },
        )
        .err()
        .unwrap();
    assert!(error.contains("existing token"), "{error}");
}

#[test]
fn append_obeys_render_source_and_retained_identity_storage_limits() {
    let session = authored("library Circulation { }");
    let candidate = new_loan(&session, "api:second", "second", "earthsea");
    assert!(
        session
            .plan_append_with(
                &candidate,
                target(&session),
                SerializationLimits {
                    steps: 0,
                    ..Default::default()
                },
                library::terminal,
                library::append
            )
            .is_err()
    );
    let sources = session.sources().to_vec();
    let mut limits = EvaluationLimits::workbench();
    limits.input_bytes = Some(sources.iter().map(|s| s.text.len()).sum());
    let tight = SourceEditSession::new(library::language().unwrap(), &sources, limits).unwrap();
    assert!(
        plan(
            &tight,
            &new_loan(&tight, "api:second", "second", "earthsea")
        )
        .is_err()
    );
    let package = library::language().unwrap().package().clone();
    let mut analysis_limits = intersect_text::document::AnalysisLimits::workbench();
    analysis_limits.association_bytes = 50_000;
    let language = intersect_text::document::DocumentLanguage::prepare(
        &package,
        Default::default(),
        analysis_limits,
    )
    .unwrap();
    let tight = SourceEditSession::new(language, &sources, EvaluationLimits::workbench()).unwrap();
    let candidate = new_loan(&tight, &"n".repeat(100_000), "second", "earthsea");
    let error = plan(&tight, &candidate).err().unwrap();
    assert!(error.contains("association budget"), "{error}");
}
