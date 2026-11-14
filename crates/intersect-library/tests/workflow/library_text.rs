//! Public Library model -> text and preserving edit acceptance.
#[path = "library_text/append.rs"]
mod append_tests;
use intersect_library as library;
use intersect_text::{
    EvaluationLimits, Source,
    serialization::{CanonicalSerializer, SerializationLimits},
    source_edit::SourceEditSession,
};
use serde_json::json;

fn serializer() -> CanonicalSerializer {
    CanonicalSerializer::new(
        library::language().unwrap(),
        EvaluationLimits::workbench(),
        SerializationLimits::default(),
    )
}

#[test]
fn library_canonical_model_graph_and_cross_resource_targets_round_trip() {
    let serializer = serializer();
    let model = library::constructed(&serializer.package().schema).unwrap();
    let original = model.clone();
    let output = serializer
        .serialize_with(&model, library::terminal)
        .unwrap();
    assert_eq!(model, original);
    assert_eq!(output.identities.len(), model.elements.len());
    assert_eq!(output.identities["api:dune"], "catalog.library#1");
    let evaluation = output.analysis.valid_model.as_ref().unwrap();
    assert_eq!(evaluation.references.len(), 2);
    assert_eq!(
        evaluation.model.elements[1].values["library"],
        json!("catalog.library#0")
    );
    let loan = library::named(&evaluation.model, "first").unwrap();
    assert_eq!(loan.values["copy"], json!(output.identities["api:dune"]));
    assert_eq!(
        loan.values["member"],
        json!(output.identities["api:member"])
    );
    assert_eq!(
        evaluation.derived[&output.identities["api:dune"]]["available"],
        json!(false)
    );
    assert_eq!(
        evaluation.derived[&output.identities["api:earthsea"]]["available"],
        json!(true)
    );
    for snapshot in &output.analysis.snapshots {
        let resource = &evaluation.model.resources[&snapshot.document().source().uri];
        snapshot.document().check_resource(resource).unwrap();
    }
    // Element storage order does not change root/containment order or bytes.
    let mut reversed = model.clone();
    reversed.elements.reverse();
    let again = serializer
        .serialize_with(&reversed, library::terminal)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&again.sources).unwrap(),
        serde_json::to_value(&output.sources).unwrap()
    );
    assert_eq!(again.identities, output.identities);
}

#[test]
fn library_new_loan_emits_declaration_and_runs_shared_validation() {
    let serializer = serializer();
    let schema = &serializer.package().schema;
    let mut model = library::constructed(schema).unwrap();
    model
        .create(schema, "api:second", "Loan", "loans.library", 0, 0)
        .unwrap();
    model
        .edit(
            schema,
            &[
                ("api:second".into(), "name".into(), json!("second")),
                ("api:second".into(), "copy".into(), json!("api:earthsea")),
                ("api:second".into(), "member".into(), json!("api:member")),
                (
                    "api:circulation".into(),
                    "loans".into(),
                    json!(["api:loan", "api:second"]),
                ),
            ],
        )
        .unwrap();
    let output = serializer
        .serialize_with(&model, library::terminal)
        .unwrap();
    assert_eq!(output.identities.len(), 7);
    assert!(
        output.sources[1]
            .text
            .contains("loan second copy earthsea to ada;")
    );
    assert_eq!(
        output
            .analysis
            .valid_model
            .as_ref()
            .unwrap()
            .references
            .len(),
        4
    );
    // Structural validity alone cannot authorize a second loan for the same copy.
    model
        .edit(
            schema,
            &[("api:second".into(), "copy".into(), json!("api:dune"))],
        )
        .unwrap();
    let error = serializer
        .serialize_with(&model, library::terminal)
        .unwrap_err();
    assert!(error.contains("more than one active loan"), "{error}");
}

#[test]
fn library_canonical_rejects_semantic_loss_and_wrong_but_resolving_spelling() {
    let serializer = serializer();
    let schema = &serializer.package().schema;
    let mut model = library::constructed(schema).unwrap();
    let error = serializer
        .serialize_with(&model, |value| {
            if value.owner.class == "Loan" && value.feature == "copy" {
                Ok("earthsea".into())
            } else {
                library::terminal(value)
            }
        })
        .unwrap_err();
    assert!(error.contains("stored state"), "{error}");
    let worn = schema.enumerations["BookCondition"].parse("WORN").unwrap();
    model
        .edit(
            schema,
            &[(
                "api:dune".into(),
                "condition".into(),
                serde_json::to_value(worn).unwrap(),
            )],
        )
        .unwrap();
    let error = serializer
        .serialize_with(&model, library::terminal)
        .unwrap_err();
    assert!(error.contains("condition"), "{error}");
}

#[test]
fn library_canonical_rejects_injected_declarations_and_unrepresentable_null_title() {
    let serializer = serializer();
    let schema = &serializer.package().schema;
    let mut model = library::constructed(schema).unwrap();
    assert!(
        serializer
            .serialize_with(&model, |value| {
                if value.feature == "title" {
                    Ok("\"Dune\"; member injected".into())
                } else {
                    library::terminal(value)
                }
            })
            .is_err()
    );
    model
        .edit(schema, &[("api:dune".into(), "title".into(), json!(null))])
        .unwrap();
    assert!(
        serializer
            .serialize_with(&model, library::terminal)
            .is_err()
    );
}

#[test]
fn library_canonical_has_finite_rendering_budgets_and_supports_simple_actions() {
    for limits in [
        SerializationLimits {
            steps: 0,
            ..Default::default()
        },
        SerializationLimits {
            output_bytes: 1,
            ..Default::default()
        },
        SerializationLimits {
            nesting: 0,
            ..Default::default()
        },
    ] {
        let serializer = CanonicalSerializer::new(
            library::language().unwrap(),
            EvaluationLimits::workbench(),
            limits,
        );
        let model = library::constructed(&serializer.package().schema).unwrap();
        assert!(
            serializer
                .serialize_with(&model, library::terminal)
                .unwrap_err()
                .contains("budget")
        );
    }
    let grammar = include_str!("../../../intersect-text/assets/fixtures/library/library.xtext")
        .replace("'library' name=ID", "{lib::Library} 'library' name=ID");
    let serializer = CanonicalSerializer::new(
        library::language_with_grammar(&grammar).unwrap(),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let model = library::constructed(&serializer.package().schema).unwrap();
    let result = serializer
        .serialize_with(&model, library::terminal)
        .unwrap();
    assert_eq!(result.identities.len(), model.elements.len());
}

fn session() -> SourceEditSession {
    SourceEditSession::new(library::language().unwrap(), &[
        Source { uri: "catalog.library".into(), text: "// 😀 keep\r\nlibrary Catalog { member ada; copy dune /* title */ \"Dune\" edition 001; }\r\n".into() },
        Source { uri: "loans.library".into(), text: "library Circulation { loan first copy dune to ada; }\r\n".into() },
    ], EvaluationLimits::workbench()).unwrap()
}

#[test]
fn library_preserving_successor_edits_keep_sources_and_resource_lifecycle() {
    let mut session = session();
    let schema = library::language().unwrap().package().schema.clone();
    let original = session.sources().to_vec();
    let untouched = session.model().resources["loans.library"].clone();
    let id = library::named(session.model(), "dune").unwrap().id.clone();
    let mut revision = session.model().resources["catalog.library"].content_revision;
    for (field, value) in [("title", json!("Dune: annotated")), ("edition", json!(2))] {
        let mut candidate = session.model().clone();
        candidate
            .edit(&schema, &[(id.clone(), field.into(), value)])
            .unwrap();
        let plan = session.plan_with(&candidate, library::scalar).unwrap();
        assert_eq!(session.adopt(plan).unwrap().len(), 1);
        assert!(session.model().resources["catalog.library"].content_revision > revision);
        revision = session.model().resources["catalog.library"].content_revision;
        assert_eq!(session.model().resources["loans.library"], untouched);
    }
    assert_eq!(
        session.sources()[0].text,
        original[0]
            .text
            .replace("\"Dune\"", "\"Dune: annotated\"")
            .replace("edition 001", "edition 2")
    );
    assert_eq!(session.sources()[1].text, original[1].text);
    let before = session.model().clone();
    let noop = session
        .plan_with(&before, |_| panic!("No-op requested spelling"))
        .unwrap();
    assert!(session.adopt(noop).unwrap().is_empty());
    assert_eq!(session.model(), &before);
}

#[test]
fn library_adoption_rejects_foreign_and_stale_plans_without_changing_session() {
    let mut session = session();
    let foreign = self::session();
    assert_eq!(session.model(), foreign.model());
    let foreign_plan = foreign.plan_with(foreign.model(), library::scalar).unwrap();
    let original = session.model().clone();
    assert!(session.adopt(foreign_plan).is_err());
    assert_eq!(session.model(), &original);
    let schema = library::language().unwrap().package().schema.clone();
    let id = library::named(session.model(), "dune").unwrap().id.clone();
    let stale = session.plan_with(session.model(), library::scalar).unwrap();
    let mut candidate = session.model().clone();
    candidate
        .edit(&schema, &[(id, "title".into(), json!("New"))])
        .unwrap();
    let fresh = session.plan_with(&candidate, library::scalar).unwrap();
    session.adopt(fresh).unwrap();
    let updated = session.model().clone();
    assert!(session.adopt(stale).is_err());
    assert_eq!(session.model(), &updated);
}
