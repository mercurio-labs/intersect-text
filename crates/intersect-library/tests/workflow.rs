use intersect_library::{self as library, Action};
use intersect_text::{EvaluationLimits, source_edit::SourceEditSession};
use serde_json::json;
fn session() -> SourceEditSession {
    SourceEditSession::new(
        library::language().unwrap(),
        &library::sources(),
        EvaluationLimits::workbench(),
    )
    .unwrap()
}
fn id(s: &SourceEditSession, name: &str) -> String {
    library::named(s.model(), name).unwrap().id.clone()
}
fn apply(s: &mut SourceEditSession, a: Action) {
    let p = library::plan(s, a).unwrap();
    s.adopt(p).unwrap();
}

#[test]
fn rename_updates_incoming_spelling_and_retarget_uses_checked_identity() {
    let mut s = session();
    let dune = id(&s, "dune");
    let first = id(&s, "first");
    let plan = library::plan(
        &s,
        Action::Set {
            id: dune.clone(),
            feature: "name".into(),
            value: json!("arrakis"),
        },
    )
    .unwrap();
    assert_eq!(plan.patches().len(), 2);
    s.adopt(plan).unwrap();
    assert!(
        s.sources()[0]
            .text
            .contains("copy arrakis /* reader note */ \"Dune\"")
    );
    assert!(s.sources()[1].text.contains("copy arrakis to ada"));
    assert_eq!(id(&s, "arrakis"), dune);
    let copy = id(&s, "foundation");
    apply(
        &mut s,
        Action::Set {
            id: first.clone(),
            feature: "copy".into(),
            value: json!(copy),
        },
    );
    let member = id(&s, "grace");
    apply(
        &mut s,
        Action::Set {
            id: first.clone(),
            feature: "member".into(),
            value: json!(member),
        },
    );
    assert!(s.sources()[1].text.contains("copy foundation to grace"));
    assert_eq!(s.evaluation().derived[&dune]["available"], json!(true));
}
#[test]
fn delete_retains_comments_and_rejects_referenced_targets() {
    let mut s = session();
    let original = s.sources()[0].text.clone();
    let dune = id(&s, "dune");
    assert!(library::plan(&s, Action::Delete { id: dune.clone() }).is_err());
    let first = id(&s, "first");
    apply(&mut s, Action::Delete { id: first.clone() });
    assert_eq!(s.sources()[0].text, original);
    assert!(s.sources()[1].text.contains("/* loan note */"));
    assert!(s.sources()[1].text.contains("// keep comment"));
    assert!(!s.sources()[1].text.contains("loan first"));
    assert!(s.model().elements.iter().all(|e| e.id != first));
    assert_eq!(s.evaluation().derived[&dune]["available"], json!(true));
}
#[test]
fn move_preserves_exact_declaration_internal_comment_and_handles() {
    let mut s = session();
    let first = id(&s, "first");
    let copy = id(&s, "earthsea");
    let member = id(&s, "ada");
    apply(
        &mut s,
        Action::AddLoan {
            id: "api:second".into(),
            name: "second".into(),
            copy,
            member,
        },
    );
    apply(
        &mut s,
        Action::Move {
            id: first.clone(),
            position: 1,
        },
    );
    let text = &s.sources()[1].text;
    assert!(text.find("loan second").unwrap() < text.find("loan first").unwrap());
    assert!(text.contains("loan first /* loan note */ copy dune to ada;"));
    assert_eq!(id(&s, "first"), first);
    assert!(text.contains("// keep comment"));
    apply(
        &mut s,
        Action::Move {
            id: first.clone(),
            position: 0,
        },
    );
    assert!(
        s.sources()[1].text.find("loan first").unwrap()
            < s.sources()[1].text.find("loan second").unwrap()
    );
    let p = library::plan(
        &s,
        Action::Set {
            id: first,
            feature: "name".into(),
            value: json!("renamed"),
        },
    )
    .unwrap();
    s.adopt(p).unwrap();
}
#[test]
fn errors_and_stale_plans_leave_the_accepted_pair_unchanged() {
    let mut s = session();
    let before = s.model().clone();
    let texts = serde_json::to_value(s.sources()).unwrap();
    let dune = id(&s, "dune");
    assert!(
        library::plan(
            &s,
            Action::Set {
                id: dune.clone(),
                feature: "name".into(),
                value: json!("earthsea")
            }
        )
        .is_err()
    );
    assert_eq!(s.model(), &before);
    assert_eq!(serde_json::to_value(s.sources()).unwrap(), texts);
    let stale = library::plan(
        &s,
        Action::Delete {
            id: id(&s, "first"),
        },
    )
    .unwrap();
    apply(
        &mut s,
        Action::Set {
            id: dune,
            feature: "title".into(),
            value: json!("Annotated"),
        },
    );
    let current = s.model().clone();
    assert!(s.adopt(stale).is_err());
    assert_eq!(s.model(), &current);
}

#[path = "workflow/frozen_store.rs"]
mod frozen_store;
#[path = "workflow/library_text.rs"]
mod library_text;
