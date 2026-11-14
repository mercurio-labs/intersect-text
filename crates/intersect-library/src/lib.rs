//! Handwritten Library policy/example inputs over the shared runtime.
use intersect_model::{
    core::model::Model,
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        schema::Schema,
    },
};
use intersect_text::package::BundleCompilation;
use intersect_text::{
    document::{AnalysisLimits, DocumentLanguage},
    grammar::Profile,
    ordinary,
    serialization::TerminalSpelling,
    source_edit::{
        AppendPlacement, AppendTarget, MovePlacement, PreparedSourceEdit, ScalarSpelling,
        SourceEditSession, SourceInsertion,
    },
};
use serde_json::{Value, json};

pub fn language() -> Result<DocumentLanguage, String> {
    language_with_grammar(include_str!(
        "../../intersect-text/assets/fixtures/library/library.xtext"
    ))
}

pub fn language_with_grammar(grammar_text: &str) -> Result<DocumentLanguage, String> {
    let model = DefinitionSource::new(
        "memory:library.rcore",
        SourceKind::Rcore,
        include_str!("../../intersect-text/assets/fixtures/library/library-project.rcore"),
    );
    let mut grammar =
        DefinitionSource::new("memory:Library.xtext", SourceKind::Xtext, grammar_text);
    let mut terminals = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        include_str!(
            "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    );
    terminals.dependencies.push(model.id.clone());
    grammar.dependencies.push(model.id.clone());
    grammar.dependencies.push(terminals.id.clone());
    let inputs = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:intersect:example:library:1".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: model.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, grammar, terminals],
            namespaces: vec![],
        },
    };
    let package = ordinary::compile(inputs, Profile::NativeV65).map_err(|e| format!("{e:?}"))?;
    DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
        .map_err(|e| format!("{e:?}"))
}

/// Arbitrary IDs and intentionally interleaved creation order demonstrate that
/// output order/identity follow resources and containment, not element-vector order.
pub fn constructed(schema: &Schema) -> Result<Model, String> {
    let mut model = Model::default();
    for (id, class, resource) in [
        ("api:loan", "Loan", "loans.library"),
        ("api:earthsea", "BookCopy", "catalog.library"),
        ("api:catalog", "Library", "catalog.library"),
        ("api:member", "Member", "catalog.library"),
        ("api:dune", "BookCopy", "catalog.library"),
        ("api:circulation", "Library", "loans.library"),
    ] {
        model.create(schema, id, class, resource, 0, 0)?;
    }
    let edits = [
        ("api:catalog", "name", json!("Catalog")),
        ("api:circulation", "name", json!("Circulation")),
        ("api:earthsea", "name", json!("earthsea")),
        ("api:earthsea", "title", json!("A Wizard of Earthsea")),
        ("api:earthsea", "edition", json!(2)),
        ("api:dune", "name", json!("dune")),
        ("api:dune", "title", json!("Dune")),
        ("api:member", "name", json!("ada")),
        ("api:loan", "name", json!("first")),
        ("api:loan", "copy", json!("api:dune")),
        ("api:loan", "member", json!("api:member")),
        ("api:catalog", "copies", json!(["api:dune", "api:earthsea"])),
        ("api:catalog", "members", json!(["api:member"])),
        ("api:circulation", "loans", json!(["api:loan"])),
    ]
    .into_iter()
    .map(|(id, field, value)| (id.into(), field.into(), value))
    .collect::<Vec<_>>();
    model.edit(schema, &edits)?;
    model.validate(schema)?;
    Ok(model)
}

pub fn terminal(spelling: TerminalSpelling<'_>) -> Result<String, String> {
    let value = match spelling.target {
        Some(target) => target
            .values
            .get("name")
            .ok_or("Reference target has no Library name")?,
        None => spelling.value,
    };
    match spelling.terminal.rsplit("::").next() {
        Some("ID") => value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "ID requires a string".into()),
        Some("STRING") => serde_json::to_string(value).map_err(|e| e.to_string()),
        Some("INT") if value.is_number() => Ok(value.to_string()),
        _ => Err(format!(
            "Library spelling policy does not support {}",
            spelling.terminal
        )),
    }
}

pub fn scalar(spelling: ScalarSpelling<'_>) -> Result<String, String> {
    if let Some(target) = spelling.target {
        if spelling.before == spelling.after
            && spelling
                .before_target
                .is_some_and(|old| old.values.get("name") == target.values.get("name"))
        {
            return Ok(spelling.original.into());
        }
        return target
            .values
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| "Reference target has no Library name".into());
    }
    match spelling.feature {
        "title" => serde_json::to_string(spelling.after).map_err(|e| e.to_string()),
        "edition" => Ok(spelling.after.to_string()),
        "name" => spelling
            .after
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "Name must be a string".into()),
        _ => Err("No Library scalar spelling policy for this feature".into()),
    }
}

pub fn named<'a>(
    model: &'a Model,
    name: &str,
) -> Result<&'a intersect_model::core::model::Element, String> {
    let mut matching = model
        .elements
        .iter()
        .filter(|e| e.values.get("name") == Some(&Value::String(name.into())));
    let object = matching
        .next()
        .ok_or_else(|| format!("No Library object named {name}"))?;
    if matching.next().is_some() {
        return Err(format!("Ambiguous Library name {name}"));
    }
    Ok(object)
}

/// Library-owned placement: append before the actual owner's closing keyword.
/// Existing trailing comments remain with preceding source. Match nearby line
/// endings and declaration indentation without removing any original bytes.
pub fn append(value: AppendPlacement<'_>) -> Result<SourceInsertion, String> {
    use intersect_model::core::text_document::{NodeId, NodeKind};
    let document = value.snapshot.document();
    let mut closing = None;
    let mut owner_start = None;
    for (index, node) in document.nodes().iter().enumerate() {
        if node.kind != NodeKind::Leaf || node.data.hidden {
            continue;
        }
        let Some(owner) = value.snapshot.semantic_object(NodeId(index))? else {
            continue;
        };
        if value
            .snapshot
            .semantic_objects()
            .get(owner)
            .is_some_and(|e| e.id == value.owner.id)
        {
            let start = document
                .utf8_range(node.range)?
                .ok_or("Library token splits a Unicode scalar")?
                .start;
            owner_start = Some(owner_start.map_or(start, |old: usize| old.min(start)));
            if document.text_in(node.range)? == Some("}")
                && closing.replace(NodeId(index)).is_some()
            {
                return Err("Library owner has ambiguous closing keywords".into());
            }
        }
    }
    let before = document.handle(closing.ok_or("Library owner has no closing keyword")?)?;
    let at = document
        .utf8_range(document.node(&before)?.range)?
        .ok_or("Library closing keyword splits a Unicode scalar")?
        .start;
    let prefix = &document.text()[..at];
    let line = prefix.rsplit_once('\n').map_or(prefix, |(_, line)| line);
    let declaration = value.declaration.trim_end_matches('\n');
    let text = if line.chars().all(|c| matches!(c, ' ' | '\t')) {
        let ending = if prefix.ends_with(&format!("\r\n{line}")) {
            "\r\n"
        } else {
            "\n"
        };
        let owner_indent = line;
        // Existing declaration indentation is language style, never a parser.
        let unit = document.text()[owner_start.ok_or("Missing Library owner source")?..at]
            .lines()
            .filter_map(|line| {
                let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
                indent
                    .strip_prefix(owner_indent)
                    .filter(|rest| !rest.is_empty())
            })
            .next()
            .unwrap_or("  ");
        format!("{unit}{declaration}{ending}{owner_indent}")
    } else {
        let separator = if prefix.ends_with(|c: char| c.is_whitespace()) {
            ""
        } else {
            " "
        };
        format!("{separator}{declaration} ")
    };
    Ok(SourceInsertion { before, text })
}

pub fn relocate(value: MovePlacement<'_>) -> Result<SourceInsertion, String> {
    let Some(before) = value.before else {
        return append(AppendPlacement {
            snapshot: value.snapshot,
            owner: value.owner,
            element: value.element,
            feature: "",
            declaration: value.declaration,
        });
    };
    let document = value.snapshot.document();
    let at = document
        .utf8_range(document.node(&before)?.range)?
        .ok_or("Move anchor splits Unicode")?
        .start;
    let prefix = &document.text()[..at];
    let line = prefix.rsplit_once('\n').map_or(prefix, |(_, line)| line);
    let text = if line.chars().all(|c| matches!(c, ' ' | '\t')) {
        let ending = if prefix.ends_with(&format!("\r\n{line}")) {
            "\r\n"
        } else {
            "\n"
        };
        format!("{}{ending}{line}", value.declaration)
    } else {
        format!(
            "{}{} ",
            if prefix.ends_with(char::is_whitespace) {
                ""
            } else {
                " "
            },
            value.declaration
        )
    };
    Ok(SourceInsertion { before, text })
}

pub fn sources() -> Vec<intersect_text::Source> {
    vec![
        intersect_text::Source { uri: "catalog.library".into(), text: "// Authored catalog — keep formatting. 😀\r\nlibrary Catalog {\r\n  copy dune /* reader note */ \"Dune\";\r\n  copy earthsea \"A Wizard of Earthsea\" edition 002;\r\n  copy foundation \"Foundation\";\r\n  member ada;\r\n  member grace;\r\n}\r\n".into() },
        intersect_text::Source { uri: "loans.library".into(), text: "// Authored circulation\r\nlibrary Circulation {\r\n  loan first /* loan note */ copy dune to ada; // keep comment\r\n}\r\n".into() },
    ]
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    AddLoan {
        id: String,
        name: String,
        copy: String,
        member: String,
    },
    Set {
        id: String,
        feature: String,
        value: Value,
    },
    Delete {
        id: String,
    },
    Move {
        id: String,
        position: usize,
    },
}

pub fn plan(session: &SourceEditSession, action: Action) -> Result<PreparedSourceEdit, String> {
    let schema = &session.package().schema;
    match action {
        Action::Set { id, feature, value } => {
            if !["name", "title", "edition", "copy", "member"].contains(&feature.as_str()) {
                return Err("Feature is outside the Library edit profile".into());
            }
            let mut candidate = session.model().clone();
            candidate.edit(schema, &[(id, feature, value)])?;
            session.plan_with(&candidate, scalar)
        }
        Action::AddLoan {
            id,
            name,
            copy,
            member,
        } => {
            let owner = session
                .model()
                .elements
                .iter()
                .find(|e| e.class == "Library" && e.resource == "loans.library")
                .ok_or("Missing circulation Library resource")?;
            let mut values = owner.values["loans"]
                .as_array()
                .ok_or("Missing loan collection")?
                .clone();
            values.push(json!(id));
            let mut candidate = session.model().clone();
            candidate.create(schema, &id, "Loan", &owner.resource, 0, 0)?;
            candidate.edit(
                schema,
                &[
                    (id.clone(), "name".into(), json!(name)),
                    (id.clone(), "copy".into(), json!(copy)),
                    (id, "member".into(), json!(member)),
                    (owner.id.clone(), "loans".into(), json!(values)),
                ],
            )?;
            session.plan_append_with(
                &candidate,
                AppendTarget {
                    owner: &owner.id,
                    feature: "loans",
                    rule: "Loan",
                },
                Default::default(),
                terminal,
                append,
            )
        }
        Action::Delete { id } => session.plan_delete(&id),
        Action::Move { id, position } => {
            let mut destination = None;
            for owner in &session.model().elements {
                for (name, field) in schema.features(&owner.class)? {
                    if field.containment
                        && owner
                            .values
                            .get(&name)
                            .and_then(Value::as_array)
                            .is_some_and(|v| v.iter().any(|v| v.as_str() == Some(&id)))
                    {
                        destination = Some((owner.id.clone(), name));
                    }
                }
            }
            let (owner, feature) =
                destination.ok_or("Object has no movable containment collection")?;
            session.plan_move_with(
                &id,
                intersect_model::core::storage::ContainmentDestination {
                    owner: &owner,
                    feature: &feature,
                    position: Some(position),
                },
                relocate,
            )
        }
    }
}
