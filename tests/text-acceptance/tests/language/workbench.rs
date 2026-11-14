use intersect_model::core::model::Model;

use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;

const ECORE: &str = include_str!("../../../../fixtures/library/library.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/library/library.xtext");
const RCORE_BEHAVIOR: &str = include_str!("../../../../fixtures/library/library-behavior.rcore");
fn package() -> intersect_text::package::Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .expect("fixture compiles")
}
fn sources() -> Vec<Source> {
    vec![
        Source {
            uri: "catalog.library".into(),
            text: include_str!("../../../../fixtures/library/catalog.library").into(),
        },
        Source {
            uri: "loans.library".into(),
            text: include_str!("../../../../fixtures/library/loans.library").into(),
        },
    ]
}

#[test]
fn namespace_aliases_preserve_fresh_model_and_semantic_results() {
    let baseline = evaluate(&package(), &sources());
    let renamed = ECORE
        .replace("ecore:", "model:")
        .replace("xmlns:ecore", "xmlns:model")
        .replace("xsi:", "instance:")
        .replace("xmlns:xsi", "xmlns:instance");
    let compiled = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &renamed,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let actual = evaluate(&compiled, &sources());
    assert!(actual.diagnostics.is_empty());
    assert_eq!(
        serde_json::to_value(&actual.model.elements).unwrap(),
        serde_json::to_value(&baseline.model.elements).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&actual.derived).unwrap(),
        serde_json::to_value(&baseline.derived).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&actual.references).unwrap(),
        serde_json::to_value(&baseline.references).unwrap()
    );
    assert_eq!(
        compiled.sources["ecore"],
        intersect_text_acceptance::migrate_model_fixture(&renamed).unwrap()
    );
}

#[test]
fn multi_resource_library_text_matches_independent_direct_construction() {
    let input = serde_json::from_str(include_str!(
        "../../../../fixtures/library/multi-resource-canonical.json"
    ))
    .unwrap();
    let p = intersect_text::compiler::compile_bundle(input).unwrap();
    assert_eq!(p.version, 2);
    assert_library_direct(p);
}

#[test]
fn diamond_text_construction_matches_direct_model_services() {
    let ecore = ECORE.replace("name=\"BookCopy\" eSuperTypes=\"#//Named\"", "name=\"BookCopy\" eSuperTypes=\"#//Tracked #//Labeled\"")
        .replace("</ecore:EPackage>", "<eClassifiers xsi:type=\"ecore:EClass\" name=\"Tracked\" eSuperTypes=\"#//Named\"/><eClassifiers xsi:type=\"ecore:EClass\" name=\"Labeled\" interface=\"true\" eSuperTypes=\"#//Named\"><eStructuralFeatures xsi:type=\"ecore:EAttribute\" name=\"label\" defaultValueLiteral=\"secondary\" eType=\"ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString\"/></eClassifiers></ecore:EPackage>");
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &ecore,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    assert_eq!(
        p.schema.feature_owner("BookCopy", "label").unwrap(),
        "Labeled"
    );
    assert_library_direct(p);
    let abstract_ecore = ecore.replace("name=\"BookCopy\"", "name=\"BookCopy\" abstract=\"true\"");
    let abstract_package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &abstract_ecore,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let rejected = evaluate(&abstract_package, &sources());
    assert!(!rejected.diagnostics.is_empty());
    assert!(
        rejected
            .model
            .elements
            .iter()
            .all(|e| e.class != "BookCopy")
    );
}

fn assert_library_direct(p: intersect_text::package::Package) {
    let parsed = evaluate(&p, &sources());
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let rows = [
        ("catalog.library#0", "Library", "Catalog"),
        ("catalog.library#1", "BookCopy", "dune"),
        ("catalog.library#2", "BookCopy", "earthsea"),
        ("catalog.library#3", "Member", "ada"),
        ("loans.library#0", "Library", "Circulation"),
        ("loans.library#1", "Loan", "first"),
    ];
    let mut direct = Model::default();
    let mut edits = Vec::new();
    for (id, class, name) in rows {
        direct
            .create(&p.schema, id, class, id.split('#').next().unwrap(), 0, 0)
            .unwrap();
        edits.push((id.into(), "name".into(), json!(name)));
    }
    for (id, field, value) in [
        (
            "catalog.library#0",
            "copies",
            json!(["catalog.library#1", "catalog.library#2"]),
        ),
        ("catalog.library#0", "members", json!(["catalog.library#3"])),
        ("catalog.library#1", "title", json!("Dune")),
        ("catalog.library#2", "title", json!("A Wizard of Earthsea")),
        ("catalog.library#2", "edition", json!(2)),
        ("loans.library#0", "loans", json!(["loans.library#1"])),
        ("loans.library#1", "copy", json!("catalog.library#1")),
        ("loans.library#1", "member", json!("catalog.library#3")),
    ] {
        edits.push((id.into(), field.into(), value));
    }
    direct.edit(&p.schema, &edits).unwrap();
    direct.validate(&p.schema).unwrap();
    for (actual, expected) in parsed.model.elements.iter().zip(&direct.elements) {
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.class, expected.class);
        assert_eq!(actual.type_identity, expected.type_identity);
        assert_eq!(actual.values, expected.values);
    }
    assert_eq!(parsed.model.elements.len(), direct.elements.len());
    let mut evaluated = intersect_text::Evaluation {
        model: direct,
        diagnostics: vec![],
        references: vec![],
        derived: Default::default(),
    };
    intersect_text::apply_semantics(&p, &mut evaluated);
    assert_eq!(evaluated.derived, parsed.derived);
    assert!(evaluated.diagnostics.is_empty());
}

#[test]
fn library_cross_resource_defaults_opposites_semantics_and_persistence() {
    let p = package();
    let result = evaluate(&p, &sources());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 6);
    assert_eq!(result.references.len(), 2);
    let dune = &result.model.elements[1];
    assert_eq!(dune.values["edition"], json!(1));
    assert_eq!(dune.values["library"], json!("catalog.library#0"));
    assert_eq!(result.derived[&dune.id]["available"], json!(false));
    assert_eq!(
        result.derived["catalog.library#2"]["available"],
        json!(true)
    );
    let restored: Model =
        serde_json::from_str(&serde_json::to_string(&result.model).unwrap()).unwrap();
    assert_eq!(restored, result.model);
    restored.validate(&p.schema).unwrap();
}
#[test]
fn definition_edits_change_language_and_semantics() {
    let nullable = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        &XTEXT.replace(
            "'library' name=ID",
            "'library' ('optional'? 'marker'?) name=ID",
        ),
        RCORE_BEHAVIOR,
    )
    .unwrap();
    assert!(evaluate(&nullable, &sources()).diagnostics.is_empty());
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        &XTEXT.replace("'copy' name", "'book' name"),
        RCORE_BEHAVIOR,
    )
    .unwrap();
    assert!(!evaluate(&p, &sources()).diagnostics.is_empty());
    let mut changed = sources();
    changed[0].text = changed[0].text.replace("copy ", "book ");
    assert!(evaluate(&p, &changed).diagnostics.is_empty());
    let mut edited = sources();
    edited[1].text = "library Circulation {}".into();
    let fresh = evaluate(&package(), &edited);
    assert_eq!(fresh.derived["catalog.library#1"]["available"], json!(true));
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        &RCORE_BEHAVIOR.replace("<= 1", "<= 0"),
    )
    .unwrap();
    assert!(
        evaluate(&p, &sources())
            .diagnostics
            .iter()
            .any(|d| d.message.contains("active loan"))
    );
}
#[test]
fn direct_api_equivalence_and_atomic_move_rollback() {
    let p = package();
    let result = evaluate(&p, &sources());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let mut direct = Model::default();
    for e in &result.model.elements {
        direct
            .create(&p.schema, &e.id, &e.class, &e.resource, e.start, e.end)
            .unwrap();
    }
    let edits = result
        .model
        .elements
        .iter()
        .flat_map(|e| {
            e.values
                .iter()
                .map(|(k, v)| (e.id.clone(), k.clone(), v.clone()))
        })
        .collect::<Vec<_>>();
    direct.edit(&p.schema, &edits).unwrap();
    // Text evaluation records its source correspondence; direct construction has
    // no source document. Both still produce the same semantic model content.
    assert!(!result.model.resources.is_empty());
    for (uri, parsed) in &result.model.resources {
        assert!(parsed.source.is_some(), "{uri}");
        let constructed = &direct.resources[uri];
        assert!(constructed.source.is_none(), "{uri}");
        assert_ne!(constructed.source, parsed.source, "{uri}");
    }
    let mut direct_content = direct.clone();
    let mut parsed_content = result.model.clone();
    for record in direct_content.resources.values_mut() {
        record.content_revision = 0;
        record.source = None;
    }
    for record in parsed_content.resources.values_mut() {
        record.content_revision = 0;
        record.source = None;
    }
    assert_eq!(direct_content, parsed_content);
    let before = direct.clone();
    assert!(
        direct
            .edit(
                &p.schema,
                &[("catalog.library#1".into(), "edition".into(), json!("bad"))]
            )
            .is_err()
    );
    assert_eq!(direct, before);
    direct
        .edit(
            &p.schema,
            &[
                (
                    "catalog.library#0".into(),
                    "copies".into(),
                    json!(["catalog.library#2"]),
                ),
                (
                    "loans.library#0".into(),
                    "copies".into(),
                    json!(["catalog.library#1"]),
                ),
            ],
        )
        .unwrap();
    assert_eq!(
        direct.elements[1].values["library"],
        json!("loans.library#0")
    );
}
#[test]
fn unsupported_and_malformed_inputs_are_explicit() {
    for text in [
        ECORE.replace("#//Named", "#//Unknown"),
        ECORE.replace("</ecore:EPackage>", ""),
        ECORE.replace("upperBound=\"-1\"", "upperBound=\"-2\""),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
                &text,
                XTEXT,
                RCORE_BEHAVIOR
            )
            .is_err()
        );
    }
    for text in [
        XTEXT.replace("name=ID", "{lib::Library} name=ID"),
        XTEXT.replace("name=ID", "missing=ID"),
        XTEXT.replace("name=ID", "name=INT"),
        XTEXT.replace("'library' name=ID", "copies+=Library 'library' name=ID"),
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
                ECORE,
                &text,
                RCORE_BEHAVIOR
            )
            .is_err()
        );
    }
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            XTEXT,
            &RCORE_BEHAVIOR.replace("noIncoming", "eval")
        )
        .is_err()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            XTEXT,
            &RCORE_BEHAVIOR.replace("rcore 1;", "rcore 99;")
        )
        .is_err()
    );
}
#[test]
fn syntax_reference_and_validation_negatives() {
    let p = package();
    for (text, expected) in [
        ("library Bad { loan x copy missing to ada; }", "Unresolved"),
        ("library Bad { loan x copy ada to ada; }", "Unresolved"),
        ("library Bad { member ; }", "Expected"),
        (
            "library Bad { loan second copy dune to ada; loan third copy dune to ada; }",
            "active loan",
        ),
    ] {
        let mut inputs = sources();
        inputs[1].text = text.into();
        let result = evaluate(&p, &inputs);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains(expected)),
            "{:?}",
            result.diagnostics
        );
    }
}
#[test]
fn same_core_accepts_a_separately_identified_sysml_shaped_schema() {
    // Synthetic structural control, not an import or SysML conformance claim.
    let ecore = ECORE
        .replace(
            "urn:intersect:example:library:1",
            "urn:intersect:test:sysml-shaped",
        )
        .replace("Library", "Package")
        .replace("BookCopy", "PartUsage");
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&ecore)
            .unwrap();
    let mut model = Model::default();
    model.create(&schema, "p", "Package", "test", 0, 1).unwrap();
    model
        .create(&schema, "u", "PartUsage", "test", 0, 1)
        .unwrap();
    model
        .edit(
            &schema,
            &[
                ("p".into(), "name".into(), json!("P")),
                ("u".into(), "name".into(), json!("x")),
                ("p".into(), "copies".into(), json!(["u"])),
            ],
        )
        .unwrap();
    assert_eq!(model.elements[1].values["library"], json!("p"));
    assert_eq!(
        model.elements[1].type_identity,
        schema.identity("PartUsage")
    );
    assert_eq!(
        schema.declarations["PartUsage"].namespace,
        "urn:intersect:test:sysml-shaped"
    );
}

#[test]
fn resource_budgets_and_namespace_identity_are_enforced() {
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            &XTEXT.replace(
                "copies+=BookCopy | members+=Member",
                "copies+=BookCopy | copies+=BookCopy"
            ),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            &XTEXT.replace("'library' name=ID", "('library'?)* name=ID"),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            &XTEXT.replace("'library' name=ID", "'library'* 'library' name=ID"),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            &XTEXT.replace("lib::", "wrong::"),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            &ECORE.replace("name=\"Loan\"", "xmlns:ecore=\"urn:fake\" name=\"Loan\""),
            XTEXT,
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            &XTEXT.replace("'library'", "'two words'"),
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    let p = package();
    let source = Source {
        uri: "large".into(),
        text: format!("library Big {{ {} }}", "member a; ".repeat(520)),
    };
    assert!(
        evaluate_with_limits(&p, &[source], EvaluationLimits::workbench())
            .diagnostics
            .iter()
            .any(|d| d.message.contains("budget"))
    );
    let source = Source {
        uri: "big".into(),
        text: " ".repeat(256_001),
    };
    assert!(
        evaluate_with_limits(&p, &[source], EvaluationLimits::workbench())
            .diagnostics
            .iter()
            .any(|d| d.message.contains("budget"))
    );
}

#[test]
fn construction_opposite_failure_is_atomic_and_type_identity_is_checked() {
    let p = package();
    let mut result = evaluate(&p, &sources());
    let before = result.model.clone();
    let changed = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &ECORE.replace("defaultValueLiteral=\"1\"", "defaultValueLiteral=\"9\""),
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    assert!(result.model.validate(&changed.schema).is_err());
    assert!(
        result
            .model
            .set_with_opposite(&p.schema, "catalog.library#0", "copies", json!(["missing"]))
            .is_err()
    );
    assert_eq!(before, result.model);
    result.model.elements[1].type_identity = "urn:other#//BookCopy".into();
    assert!(result.model.validate(&p.schema).is_err());
}

#[test]
fn concrete_generic_library_executes_shared_semantics_and_survives_package_reload() {
    let text = include_str!("../../../../fixtures/ecore-generics/library.ecore");
    let compiled = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        text,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let signatures = compiled.schema.generic_signatures.as_ref().unwrap();
    assert!(signatures.parameters.is_empty());
    assert!(!signatures.types.is_empty());
    assert!(
        signatures
            .types
            .iter()
            .all(|t| !t.syntax.source_payload.is_empty())
    );
    let mut baseline = evaluate(&package(), &sources());
    // Equivalent model behavior, with identities updated to the new exact source revision.
    for element in &mut baseline.model.elements {
        element.type_identity = compiled.schema.identity(&element.class);
    }
    let actual = evaluate(&compiled, &sources());
    assert!(actual.diagnostics.is_empty(), "{:?}", actual.diagnostics);
    assert_eq!(
        serde_json::to_value(&actual.model.elements).unwrap(),
        serde_json::to_value(&baseline.model.elements).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&actual.derived).unwrap(),
        serde_json::to_value(&baseline.derived).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&actual.references).unwrap(),
        serde_json::to_value(&baseline.references).unwrap()
    );
    let restored: intersect_text::package::Package =
        serde_json::from_str(&serde_json::to_string(&compiled).unwrap()).unwrap();
    assert_eq!(
        restored.schema.generic_signatures,
        compiled.schema.generic_signatures
    );
    assert_eq!(
        serde_json::to_value(evaluate(&restored, &sources()).model.elements).unwrap(),
        serde_json::to_value(&baseline.model.elements).unwrap()
    );
    assert_library_direct(compiled);
}

#[test]
fn volatile_stored_defaults_remain_live_in_text_evaluation() {
    let compiled = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &ECORE.replace("name=\"edition\"", "volatile=\"true\" name=\"edition\""),
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap();
    let baseline = evaluate(&package(), &sources());
    let actual = evaluate(&compiled, &sources());
    assert!(actual.diagnostics.is_empty());
    assert!(compiled.schema.features("BookCopy").unwrap()["edition"].volatile);
    assert_eq!(
        actual
            .model
            .elements
            .iter()
            .map(|element| &element.values)
            .collect::<Vec<_>>(),
        baseline
            .model
            .elements
            .iter()
            .map(|element| &element.values)
            .collect::<Vec<_>>()
    );
}
