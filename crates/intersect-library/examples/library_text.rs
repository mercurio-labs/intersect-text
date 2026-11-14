//! Executable model -> text and source-preserving Library example.
use intersect_library as library;

use intersect_text::{
    EvaluationLimits, Source,
    serialization::{CanonicalSerializer, SerializationLimits},
    source_edit::{AppendTarget, SourceEditSession},
};
use serde_json::json;

fn main() -> Result<(), String> {
    let language = library::language()?;
    let schema = language.package().schema.clone();
    let mut constructed = library::constructed(&schema)?;
    let serializer = CanonicalSerializer::new(
        language,
        EvaluationLimits::workbench(),
        SerializationLimits::default(),
    );
    let first = serializer.serialize_with(&constructed, library::terminal)?;
    assert_eq!(first.identities.len(), 6);
    assert_eq!(first.identities["api:dune"], "catalog.library#1");
    println!("API-created Library -> verified text (6 objects, 2 resources)");
    for source in &first.sources {
        println!("{}:\n{}", source.uri, source.text);
    }

    constructed.create(&schema, "api:second-loan", "Loan", "loans.library", 0, 0)?;
    constructed.edit(
        &schema,
        &[
            ("api:second-loan".into(), "name".into(), json!("second")),
            (
                "api:second-loan".into(),
                "copy".into(),
                json!("api:earthsea"),
            ),
            (
                "api:second-loan".into(),
                "member".into(),
                json!("api:member"),
            ),
            (
                "api:circulation".into(),
                "loans".into(),
                json!(["api:loan", "api:second-loan"]),
            ),
        ],
    )?;
    let expanded = serializer.serialize_with(&constructed, library::terminal)?;
    assert_eq!(expanded.identities.len(), 7);
    let expanded_model = expanded
        .analysis
        .valid_model
        .as_ref()
        .ok_or("Missing generated Library model")?;
    assert_eq!(expanded_model.references.len(), 4);
    assert_eq!(
        expanded_model.derived[&expanded.identities["api:earthsea"]]["available"],
        json!(false)
    );
    println!(
        "New Loan -> verified text (7 objects, 4 cross-resource references)\n{}",
        expanded.sources[1].text
    );

    let original = "// Keep this catalog and its layout. 😀\r\nlibrary Catalog {\r\n  member ada; // original declaration order\r\n  copy earthsea /* edition note */ \"A Wizard of Earthsea\" edition 002;\r\n  copy dune \"Dune\";\r\n}\r\n";
    let sources = vec![
        Source {
            uri: "catalog.library".into(),
            text: original.into(),
        },
        Source {
            uri: "loans.library".into(),
            text: "library Circulation { loan first copy dune to ada; }\r\n".into(),
        },
    ];
    let mut session = SourceEditSession::new(
        library::language()?,
        &sources,
        EvaluationLimits::workbench(),
    )?;
    let noop = session.plan_with(session.model(), |_| {
        Err("No-op must not request spelling".into())
    })?;
    assert!(session.adopt(noop)?.is_empty());
    assert_eq!(session.sources()[0].text, original);
    let id = library::named(session.model(), "earthsea")?.id.clone();
    let revision = session.model().resources["catalog.library"].content_revision;
    let mut candidate = session.model().clone();
    candidate.edit(
        &schema,
        &[(id.clone(), "title".into(), json!("Earthsea: illustrated"))],
    )?;
    let edit = session.plan_with(&candidate, library::scalar)?;
    assert_eq!(session.adopt(edit)?.len(), 1);
    let expected = original.replace("\"A Wizard of Earthsea\"", "\"Earthsea: illustrated\"");
    assert_eq!(session.sources()[0].text, expected);
    assert!(session.model().resources["catalog.library"].content_revision > revision);

    let mut candidate = session.model().clone();
    candidate.edit(&schema, &[(id, "edition".into(), json!(3))])?;
    let edit = session.plan_with(&candidate, library::scalar)?;
    assert_eq!(session.adopt(edit)?.len(), 1);
    assert_eq!(
        session.sources()[0].text,
        expected.replace("edition 002", "edition 3")
    );
    assert_eq!(session.sources()[1].text, sources[1].text);
    println!(
        "Authored Library -> two verified token edits; comments, CRLF, order and other resource preserved\n{}",
        session.sources()[0].text
    );

    // Add a Loan without regenerating either authored resource. IDs survive
    // reparse through checked graph correspondence and refreshed source maps.
    let owner = library::named(session.model(), "Circulation")?.id.clone();
    let earthsea = library::named(session.model(), "earthsea")?.id.clone();
    let member = library::named(session.model(), "ada")?.id.clone();
    let mut children = library::named(session.model(), "Circulation")?.values["loans"]
        .as_array()
        .ok_or("Loan collection is not an array")?
        .clone();
    children.push(json!("api:preserved-second"));
    let mut candidate = session.model().clone();
    candidate.create(
        &schema,
        "api:preserved-second",
        "Loan",
        "loans.library",
        0,
        0,
    )?;
    candidate.edit(
        &schema,
        &[
            (
                "api:preserved-second".into(),
                "name".into(),
                json!("second"),
            ),
            (
                "api:preserved-second".into(),
                "copy".into(),
                json!(earthsea),
            ),
            (
                "api:preserved-second".into(),
                "member".into(),
                json!(member),
            ),
            (owner.clone(), "loans".into(), json!(children)),
        ],
    )?;
    let preserved_catalog = session.sources()[0].text.clone();
    let append = session.plan_append_with(
        &candidate,
        AppendTarget {
            owner: &owner,
            feature: "loans",
            rule: "Loan",
        },
        Default::default(),
        library::terminal,
        library::append,
    )?;
    assert_eq!(append.patches().len(), 1);
    assert_eq!(
        append.patches()[0].range.start,
        append.patches()[0].range.end
    );
    assert_eq!(
        append.parsed_identities()["api:preserved-second"],
        "loans.library#2"
    );
    session.adopt(append)?;
    assert_eq!(session.sources()[0].text, preserved_catalog);
    assert_eq!(
        session.sources()[1].text,
        "library Circulation { loan first copy dune to ada; loan second copy earthsea to ada; }\r\n"
    );
    assert_eq!(
        library::named(session.model(), "second")?.id,
        "api:preserved-second"
    );
    println!(
        "Authored Library -> one insertion-only Loan patch; every original byte and model ID preserved\n{}",
        session.sources()[1].text
    );

    let mut unrepresentable = constructed.clone();
    // The model permits a null title, but the text grammar requires a STRING.
    unrepresentable.edit(&schema, &[("api:dune".into(), "title".into(), json!(null))])?;
    assert!(
        serializer
            .serialize_with(&unrepresentable, library::terminal)
            .is_err()
    );
    println!("Unrepresentable required text value rejected. All Library checks passed.");
    Ok(())
}
