//! Acceptance controls for the first bounded source-preserving scalar edit API.
use super::*;
use crate::package::BundleCompilation;
use crate::source_edit::{ScalarSpelling, SourceEditSession};
use intersect_model::definitions::schema::Schema;

const STRUCTURE: &str = r#"
rcore 1;
package sourceEdits version "1";
metamodel "urn:intersect:source-edits" prefix e {
  class Root {
    attr name: EString [1,1];
    ref items: Item [0,*] containment;
  }
  class Item {
    attr name: EString [1,1] unsettable;
    attr title: EString [0,1];
    attr count: EInt [0,1];
    ref target: Item [0,1];
  }
}
"#;
const GRAMMAR: &str = r#"
grammar acceptance.SourceEdits hidden(WS, ML_COMMENT, SL_COMMENT)
import "urn:intersect:source-edits" as e
import "http://www.eclipse.org/emf/2002/Ecore" as ecore
Root returns e::Root: 'root' name=ID '{' items+=Item* '}';
Item returns e::Item:
  'item' name=ID title=STRING ('count' count=INT)? ('to' target=[e::Item|ID])? ';';
terminal ID returns ecore::EString: ('a'..'z'|'A'..'Z'|'_') ('a'..'z'|'A'..'Z'|'0'..'9'|'_')*;
terminal INT returns ecore::EInt: ('0'..'9')+;
terminal STRING returns ecore::EString: '"' ('\\' . | !('\\'|'"'))* '"';
terminal WS returns ecore::EString: (' '|'\t'|'\r'|'\n')+;
terminal ML_COMMENT returns ecore::EString: '/*' ->'*/';
terminal SL_COMMENT returns ecore::EString: '//' !('\n'|'\r')* ('\r'? '\n')?;
"#;

fn language(grammar: &str) -> (document::DocumentLanguage, Schema) {
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let model = DefinitionSource::new("memory:source-edits.rcore", SourceKind::Rcore, STRUCTURE);
    let mut text = DefinitionSource::new("memory:source-edits.xtext", SourceKind::Xtext, grammar);
    text.dependencies.push(model.id.clone());
    let inputs = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:intersect:source-edits".into(),
        xtext: Some(text.id.clone()),
        rcore_behavior: model.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, text],
            namespaces: vec![],
        },
    };
    let package = ordinary::compile(inputs, grammar::Profile::NativeV65).unwrap();
    let schema = package.schema.clone();
    let language = document::DocumentLanguage::prepare(
        &package,
        Default::default(),
        document::AnalysisLimits::workbench(),
    )
    .unwrap();
    (language, schema)
}
fn sources(text: &[&str]) -> Vec<Source> {
    text.iter()
        .enumerate()
        .map(|(i, text)| Source {
            uri: format!("memory:source-edit/{i}.test"),
            text: (*text).into(),
        })
        .collect()
}
fn session_with(grammar: &str, text: &[&str]) -> (SourceEditSession, Schema) {
    let (language, schema) = language(grammar);
    (
        SourceEditSession::new(language, &sources(text), EvaluationLimits::workbench()).unwrap(),
        schema,
    )
}
fn session(text: &[&str]) -> (SourceEditSession, Schema) {
    session_with(GRAMMAR, text)
}
fn item(model: &Model, name: &str) -> String {
    model
        .elements
        .iter()
        .find(|e| e.class == "Item" && e.values["name"] == json!(name))
        .unwrap()
        .id
        .clone()
}
fn spelling(value: ScalarSpelling<'_>) -> Result<String, String> {
    match value.feature {
        "title" => serde_json::to_string(&value.after).map_err(|e| e.to_string()),
        "name" => value
            .after
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "Name spelling needs a string".into()),
        "count" => Ok(value.after.to_string()),
        other => Err(format!("No test spelling policy for {other}")),
    }
}

#[test]
fn source_edit_noop_retains_every_byte_and_never_requests_spelling() {
    let original = "/* 😀 original */\r\nroot Catalog {\r\n\titem first \"é\\nEarthsea\" count 002; // retain\r\n}\r\n";
    let (session, _) = session(&[original]);
    let plan = session
        .plan_with(session.model(), |_| panic!("No-op requested a spelling"))
        .unwrap();
    assert!(plan.patches().is_empty());
    assert_eq!(plan.sources()[0].text, original);
    let output = plan.finish(session.sources(), session.model()).unwrap();
    assert_eq!(output.sources[0].text.as_bytes(), original.as_bytes());
    assert_eq!(output.analysis.valid_model.unwrap().model, *session.model());
}

#[test]
fn source_edit_local_unicode_patch_preserves_surroundings_and_rebinds_nodes() {
    let original =
        "/* 😀 */\r\nroot Catalog {\r\n  item first /* title */ \"é😀\" count 2; // tail\r\n}\r\n";
    let (session, schema) = session(&[original]);
    let id = item(session.model(), "first");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[(id.clone(), "title".into(), json!("New \"text\""))],
        )
        .unwrap();
    let plan = session
        .plan_with(&candidate, |value| {
            assert_eq!(value.owner, id);
            assert_eq!(value.feature, "title");
            assert_eq!(value.original, "\"é😀\"");
            assert_eq!(value.before, &json!("é😀"));
            spelling(value)
        })
        .unwrap();
    assert_eq!(plan.patches().len(), 1);
    let patch = &plan.patches()[0];
    assert_eq!(&original[patch.range.start..patch.range.end], "\"é😀\"");
    assert_eq!(patch.original, "\"é😀\"");
    assert_eq!(patch.replacement, "\"New \\\"text\\\"\"");
    assert_eq!(
        plan.sources()[0].text,
        "/* 😀 */\r\nroot Catalog {\r\n  item first /* title */ \"New \\\"text\\\"\" count 2; // tail\r\n}\r\n"
    );
    let output = plan.finish(session.sources(), session.model()).unwrap();
    let model = &output.analysis.valid_model.as_ref().unwrap().model;
    assert_eq!(
        model.elements.iter().find(|e| e.id == id).unwrap().values["title"],
        json!("New \"text\"")
    );
    for snapshot in &output.analysis.snapshots {
        let resource = &model.resources[&snapshot.document().source().uri];
        assert_eq!(
            snapshot.document().resource_revision(),
            resource.content_revision
        );
        for feature in snapshot.feature_nodes() {
            let handle = snapshot.document().handle(feature.node).unwrap();
            let linked = snapshot
                .linked_object(&handle, model, &schema)
                .unwrap()
                .unwrap();
            // Containment assignment nodes can denote the assigned child while
            // their feature owner is the parent. Scalar title nodes denote Item.
            if feature.feature == "title" {
                assert_eq!(linked.id, feature.owner);
            }
        }
    }
}

#[test]
fn source_edit_multiple_patches_preserve_other_resources_and_advance_only_changed_revision() {
    let first = "root A { item first \"Old\" count 1 to second; }\r\n";
    let second = "// untouched 😀\nroot B { item second \"Retained\"; }\n";
    let (session, schema) = session(&[first, second]);
    let id = item(session.model(), "first");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[
                (id.clone(), "title".into(), json!("Much longer")),
                (id, "count".into(), json!(42)),
            ],
        )
        .unwrap();
    let plan = session.plan_with(&candidate, spelling).unwrap();
    assert_eq!(plan.patches().len(), 2);
    assert_eq!(
        plan.sources()[0].text,
        "root A { item first \"Much longer\" count 42 to second; }\r\n"
    );
    assert_eq!(plan.sources()[1].text, second);
    let output = plan.finish(session.sources(), session.model()).unwrap();
    let model = &output.analysis.valid_model.as_ref().unwrap().model;
    let changed = &output.sources[0].uri;
    let untouched = &output.sources[1].uri;
    assert!(
        model.resources[changed].content_revision
            > session.model().resources[changed].content_revision
    );
    assert!(
        model.resources[changed].content_revision > candidate.resources[changed].content_revision
    );
    assert_eq!(
        model.resources[untouched],
        session.model().resources[untouched]
    );
    assert_eq!(
        model.resources[changed].roots,
        session.model().resources[changed].roots
    );
    assert_eq!(
        output
            .analysis
            .valid_model
            .as_ref()
            .unwrap()
            .references
            .len(),
        1
    );
}

#[test]
fn source_edit_callback_parse_and_semantic_failures_keep_baseline_unchanged() {
    let original = "root A { item first \"Old\"; }";
    let (session, schema) = session(&[original]);
    let before = session.model().clone();
    let mut candidate = before.clone();
    candidate
        .edit(
            &schema,
            &[(item(&before, "first"), "title".into(), json!("New"))],
        )
        .unwrap();
    for replacement in [
        "\"Wrong\"",
        "\"unterminated",
        "\"New\"; item extra \"injected\"",
        "/* extra */ \"New\"",
        "\"New\" /* extra */",
        "\"New\" ",
    ] {
        assert!(
            session
                .plan_with(&candidate, |_| Ok(replacement.into()))
                .is_err()
        );
        assert_eq!(session.model(), &before);
        assert_eq!(session.sources()[0].text, original);
    }
    assert!(
        session
            .plan_with(&candidate, |_| Err(
                "Spelling deliberately unavailable".into()
            ))
            .is_err()
    );
    assert_eq!(session.model(), &before);
}

#[test]
fn source_edit_finish_rejects_changed_source_or_model_baseline() {
    let (session, schema) = session(&["root A { item first \"Old\"; }"]);
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[(item(&candidate, "first"), "title".into(), json!("New"))],
        )
        .unwrap();
    let mut changed_sources = session.sources().to_vec();
    changed_sources[0].text.push(' ');
    assert!(
        session
            .plan_with(&candidate, spelling)
            .unwrap()
            .finish(&changed_sources, session.model())
            .is_err()
    );
    let mut changed_model = session.model().clone();
    changed_model.elements[0].end += 1;
    assert!(
        session
            .plan_with(&candidate, spelling)
            .unwrap()
            .finish(session.sources(), &changed_model)
            .is_err()
    );
    session
        .plan_with(&candidate, spelling)
        .unwrap()
        .finish(session.sources(), session.model())
        .unwrap();
}

#[test]
fn source_edit_rejects_structural_and_unrelated_metadata_changes_before_spelling() {
    let (session, _) = session(&["root A { item first \"One\"; item second \"Two\"; }"]);
    let mut candidates = Vec::new();
    let mut removed = session.model().clone();
    removed.elements.pop();
    candidates.push(removed);
    let mut span = session.model().clone();
    span.elements[0].start += 1;
    candidates.push(span);
    let mut revision_only = session.model().clone();
    revision_only
        .resources
        .values_mut()
        .next()
        .unwrap()
        .content_revision += 1;
    candidates.push(revision_only);
    let mut roots = session.model().clone();
    roots.resources.values_mut().next().unwrap().roots = std::sync::Arc::new(Vec::new());
    candidates.push(roots);
    for candidate in candidates {
        assert!(
            session
                .plan_with(&candidate, |_| panic!(
                    "Unsupported change reached spelling"
                ))
                .is_err()
        );
    }
}

#[test]
fn source_edit_rejects_assignment_only_changes_even_when_effective_value_matches() {
    let (session, _) = session(&["root A { item first \"One\"; }"]);
    let mut candidate = session.model().clone();
    let object = candidate
        .elements
        .iter_mut()
        .find(|e| e.class == "Item")
        .unwrap();
    object.assignment_state = Some(
        object
            .assignment_state
            .as_ref()
            .unwrap()
            .with_assignment("name", false)
            .unwrap(),
    );
    assert!(
        session
            .plan_with(&candidate, |_| panic!(
                "Assignment-only change reached spelling"
            ))
            .is_err()
    );
}

#[test]
fn source_edit_rejects_missing_and_repeated_scalar_occurrences() {
    let (session, schema) = session(&["root A { item first \"One\"; }"]);
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[(item(&candidate, "first"), "count".into(), json!(3))],
        )
        .unwrap();
    assert!(
        session
            .plan_with(&candidate, |_| panic!("Missing syntax reached spelling"))
            .is_err()
    );
    let repeated = GRAMMAR.replace("title=STRING", "title=STRING 'again' title=STRING");
    let (session, schema) =
        session_with(&repeated, &["root A { item first \"One\" again \"Two\"; }"]);
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[(item(&candidate, "first"), "title".into(), json!("Three"))],
        )
        .unwrap();
    assert!(
        session
            .plan_with(&candidate, |_| panic!("Ambiguous syntax reached spelling"))
            .is_err()
    );
}

#[test]
fn source_edit_rejects_multitoken_datatype_and_internal_trivia() {
    let grammar = format!(
        "{}\nPair returns ecore::EString: ID ID;\n",
        GRAMMAR.replace("title=STRING", "title=Pair")
    );
    let (session, schema) = session_with(
        &grammar,
        &["root A { item first one /* retain inside value */ two; }"],
    );
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[(
                item(&candidate, "first"),
                "title".into(),
                json!("three four"),
            )],
        )
        .unwrap();
    assert!(
        session
            .plan_with(&candidate, |_| panic!("Multitoken value reached spelling"))
            .is_err()
    );
}

#[test]
fn source_edit_rejects_recovered_or_invalid_baselines() {
    let (language, _) = language(GRAMMAR);
    assert!(
        SourceEditSession::new(
            language,
            &sources(&["root A { item first \"One\" }"]),
            EvaluationLimits::workbench()
        )
        .is_err()
    );
}

#[test]
fn source_edit_name_change_cannot_silently_break_an_existing_reference() {
    let (session, schema) =
        session(&["root A { item first \"One\" to second; item second \"Two\"; }"]);
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[(item(&candidate, "second"), "name".into(), json!("renamed"))],
        )
        .unwrap();
    assert!(session.plan_with(&candidate, spelling).is_err());
    assert_eq!(
        session.sources()[0].text,
        "root A { item first \"One\" to second; item second \"Two\"; }"
    );
}

#[test]
fn source_edit_revision_exhaustion_rejects_instead_of_regressing_resource_identity() {
    let (session, schema) = session(&["root A { item first \"Old\"; }"]);
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &schema,
            &[(item(&candidate, "first"), "title".into(), json!("New"))],
        )
        .unwrap();
    candidate
        .resources
        .values_mut()
        .next()
        .unwrap()
        .content_revision = u64::MAX;
    assert!(session.plan_with(&candidate, spelling).is_err());
}
